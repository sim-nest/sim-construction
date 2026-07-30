// conformance: deterministic construction uncertainty attention recommendations

use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    AttentionLevel, BaselineId, ControlId, ExposureQueueItem, ExposureReport, ForecastBasis,
    ForecastConsequence, ForecastConsequenceKind, ForecastValue, OpenRating, ProjectBook,
    ProjectFact, ProjectId, ResponseState, RoleId, UncertaintyKind, UncertaintyRecord,
    UncertaintyResponse, UncertaintyState, derive_escalation_queue,
};

#[test]
fn escalation_rules_recommend_attention_without_making_a_decision() {
    let mut book = ProjectBook::new(project(), role());
    book.append(fact(1, "forecast.switchgear", None)).unwrap();
    book.append(fact(2, "risk.switchgear", None)).unwrap();
    book.append(fact(3, "forecast.switchgear", Some(1)))
        .unwrap();
    let snapshot = book.snapshot_at(3).unwrap();

    let uncertainty = risk();
    let consequence = ForecastConsequence::new(
        project(),
        control("forecast.switchgear"),
        control("risk.switchgear"),
        3,
        control("scenario.accepted"),
        ForecastConsequenceKind::Safety,
        ForecastValue::Qualitative("temporary energization overlap".to_owned()),
        ForecastBasis::new(baseline(), "discipline estimate", 3, date(23)),
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence("forecast"));
    let exposure = ExposureReport {
        as_of_seq: 3,
        schedule_baseline: baseline(),
        queue: vec![ExposureQueueItem {
            uncertainty: uncertainty.control.clone(),
            kind: uncertainty.kind,
            state: uncertainty.state,
            baseline: uncertainty.baseline.clone(),
            stale_rating: false,
            consequences: vec![consequence.control.clone()],
            affected_dependents: vec![control("package.electrical")],
            critical_path: true,
        }],
        amount_buckets: Vec::new(),
    };

    let queue = derive_escalation_queue(
        &snapshot,
        &exposure,
        &[uncertainty],
        &[consequence],
        date(30),
    )
    .unwrap();

    assert_eq!(queue.len(), 1);
    let recommendation = &queue[0];
    assert_eq!(recommendation.attention, AttentionLevel::Immediate);
    assert_eq!(recommendation.recommended_to, None);
    let rendered = format!("{:?}", recommendation.reasons);
    for reason in [
        "OverdueResponse",
        "CrossedTrigger",
        "ChangedConsequence",
        "MissingAuthority",
        "DecisionLeadTime",
        "CriticalPathConsequence",
    ] {
        assert!(rendered.contains(reason), "missing reason {reason}");
    }
    assert_eq!(
        recommendation.recommendation,
        "review current evidence and obtain an accountable decision"
    );
}

fn risk() -> UncertaintyRecord {
    let rating =
        OpenRating::qualitative("project/risk-matrix", "high", 2, date(22), "project review");
    UncertaintyRecord::risk(
        project(),
        control("risk.switchgear"),
        2,
        baseline(),
        control("scenario.accepted"),
        "single-source design",
        "switchgear approval is missed",
        "energization moves",
        role(),
        UncertaintyResponse::new(
            "qualify an alternate",
            "submittal rejected",
            date(25),
            date(24),
            5,
        )
        .trigger_crossed_at(2)
        .with_state(ResponseState::InProgress),
        rating.clone(),
        rating,
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence("risk"))
    .with_state(UncertaintyState::Open)
}

fn fact(seq: u64, subject: &str, supersedes: Option<u64>) -> ProjectFact {
    let fact = ProjectFact::new(
        seq,
        project(),
        control(subject),
        Symbol::qualified("construction", "uncertainty"),
        date(u8::try_from(20 + seq).unwrap()),
        role(),
        Expr::String(subject.to_owned()),
    );
    match supersedes {
        Some(prior) => fact.supersedes(prior),
        None => fact,
    }
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn baseline() -> BaselineId {
    BaselineId::new("baseline.schedule").unwrap()
}

fn role() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn control(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

fn evidence(id: &str) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("risk/reference-center/{id}"),
        Some("rev-a".to_owned()),
        None,
    )
}

#[test]
fn opportunity_capture_stays_an_attention_fact_not_a_decision() {
    let mut opportunity = risk();
    opportunity.kind = UncertaintyKind::Opportunity;
    opportunity.state = UncertaintyState::OpportunityCaptured { fact_seq: 2 };
    opportunity.validate().unwrap();
}

#[test]
fn high_attention_queue_order_is_deterministic() {
    let mut book = ProjectBook::new(project(), role());
    for (seq, id) in [(1, "risk.b"), (2, "risk.c"), (3, "risk.a")] {
        book.append(fact(seq, id, None)).unwrap();
    }
    let snapshot = book.snapshot_at(3).unwrap();
    let b = ordered_risk("risk.b", 1, true);
    let c = ordered_risk("risk.c", 2, false);
    let a = ordered_risk("risk.a", 3, true);
    let exposure = ExposureReport {
        as_of_seq: 3,
        schedule_baseline: baseline(),
        queue: [&b, &c, &a]
            .into_iter()
            .map(|uncertainty| ExposureQueueItem {
                uncertainty: uncertainty.control.clone(),
                kind: uncertainty.kind,
                state: uncertainty.state,
                baseline: uncertainty.baseline.clone(),
                stale_rating: false,
                consequences: Vec::new(),
                affected_dependents: Vec::new(),
                critical_path: false,
            })
            .collect(),
        amount_buckets: Vec::new(),
    };

    let queue = derive_escalation_queue(&snapshot, &exposure, &[b, c, a], &[], date(10)).unwrap();

    assert_eq!(
        queue
            .iter()
            .map(|item| (item.uncertainty.as_str(), item.attention))
            .collect::<Vec<_>>(),
        vec![
            ("risk.a", AttentionLevel::Immediate),
            ("risk.b", AttentionLevel::Immediate),
            ("risk.c", AttentionLevel::High),
        ]
    );
}

fn ordered_risk(id: &str, fact_seq: u64, crossed: bool) -> UncertaintyRecord {
    let rating = OpenRating::qualitative(
        "project/risk-matrix",
        "medium",
        fact_seq,
        date(u8::try_from(fact_seq).unwrap()),
        "project review",
    );
    let (due_on, decision_due_on) = if crossed {
        (date(20), date(19))
    } else {
        (date(5), date(4))
    };
    let mut response = UncertaintyResponse::new(
        "review response",
        "threshold crossed",
        due_on,
        decision_due_on,
        1,
    )
    .with_authority(role());
    if crossed {
        response = response.trigger_crossed_at(fact_seq);
    }
    UncertaintyRecord::risk(
        project(),
        control(id),
        fact_seq,
        baseline(),
        control("scenario.accepted"),
        "current project condition",
        "uncertain event",
        "project consequence",
        role(),
        response,
        rating.clone(),
        rating,
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence(id))
}
