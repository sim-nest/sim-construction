// conformance: baseline-aware exact construction uncertainty exposure

use sim_kernel::{Expr, Symbol};
use sim_ledger::Amount;
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    AcceptedBaseline, BaselineId, BaselineKind, CommercialAmount, ControlEdgeKind, ControlGraph,
    ControlId, ControlNodeKind, CurrencyCode, ExposureAnnotation, ForecastBasis,
    ForecastConsequence, ForecastConsequenceKind, ForecastValue, OpenRating, ProjectBook,
    ProjectFact, ProjectId, ResponseState, RoleId, ScheduleBaseline, ScheduleJoinKind,
    SchedulePlanRevision, ScheduleStatusReport, ScheduleTaskJoin, ScheduleTaskJoinSet,
    UncertaintyRecord, UncertaintyResponse, derive_exposure,
};

#[test]
fn exposure_uses_current_baseline_schedule_dependency_and_comparable_leaf_facts() {
    let book = fact_book(&[
        "risk.switchgear",
        "forecast.amount-parent",
        "forecast.amount-child",
        "forecast.amount-eur",
        "forecast.safety",
    ]);
    let snapshot = book.snapshot_at(5).unwrap();
    let baseline = accepted_baseline();
    let risk = risk();
    let parent = amount_consequence("forecast.amount-parent", 2, "100000.00", "SEK")
        .summarizes(control("forecast.amount-child"));
    let child = amount_consequence("forecast.amount-child", 3, "60000.00", "SEK")
        .with_parent(control("forecast.amount-parent"))
        .correlated_with(control("correlation.switchgear-market"));
    let euro = amount_consequence("forecast.amount-eur", 4, "2000.00", "EUR");
    let safety = ForecastConsequence::new(
        project(),
        control("forecast.safety"),
        control("risk.switchgear"),
        5,
        control("scenario.accepted"),
        ForecastConsequenceKind::Safety,
        ForecastValue::Qualitative("temporary energization overlap".to_owned()),
        basis(5),
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence("safety"));

    let report = derive_exposure(
        &snapshot,
        &[baseline],
        &[risk],
        &[parent, child, euro, safety],
        &joins(),
        &schedule(),
        &graph(),
    )
    .unwrap();

    assert_eq!(report.queue.len(), 1);
    assert!(report.queue[0].critical_path);
    assert_eq!(
        report.queue[0]
            .affected_dependents
            .iter()
            .map(ControlId::as_str)
            .collect::<Vec<_>>(),
        vec!["package.electrical"]
    );
    assert_eq!(report.amount_buckets.len(), 2);
    let sek = report
        .amount_buckets
        .iter()
        .find(|bucket| bucket.currency.as_str() == "SEK")
        .unwrap();
    assert_eq!(sek.total.0, 6_000_000);
    assert_eq!(
        sek.contributors
            .iter()
            .map(ControlId::as_str)
            .collect::<Vec<_>>(),
        vec!["forecast.amount-child"]
    );
    assert!(sek.annotations.iter().any(|annotation| matches!(
        annotation,
        ExposureAnnotation::ParentSummaryExcluded { parent, .. }
            if parent.as_str() == "forecast.amount-parent"
    )));
    assert!(sek.annotations.iter().any(|annotation| matches!(
        annotation,
        ExposureAnnotation::Correlated { group, .. }
            if group.as_str() == "correlation.switchgear-market"
    )));
    let eur = report
        .amount_buckets
        .iter()
        .find(|bucket| bucket.currency.as_str() == "EUR")
        .unwrap();
    assert_eq!(eur.total.0, 200_000);
}

#[test]
fn exposure_amount_overflow_fails_closed() {
    let book = fact_book(&["risk.switchgear", "forecast.max", "forecast.plus-one"]);
    let snapshot = book.snapshot_at(3).unwrap();
    let max = raw_amount_consequence("forecast.max", 2, Amount(i64::MAX));
    let plus_one = raw_amount_consequence("forecast.plus-one", 3, Amount(1));

    let result = derive_exposure(
        &snapshot,
        &[accepted_baseline()],
        &[risk()],
        &[max, plus_one],
        &joins_at(3),
        &schedule_at(3),
        &graph(),
    );

    assert!(matches!(
        result,
        Err(crate::ConstructionProjectError::AmountOverflow {
            field: "forecast.exposure.total"
        })
    ));
}

fn risk() -> UncertaintyRecord {
    let rating =
        OpenRating::qualitative("project/risk-matrix", "high", 1, date(20), "project review");
    UncertaintyRecord::risk(
        project(),
        control("risk.switchgear"),
        1,
        baseline_id(),
        control("scenario.accepted"),
        "single-source design",
        "switchgear approval is missed",
        "energization moves",
        role(),
        UncertaintyResponse::new(
            "qualify an alternate",
            "submittal rejected",
            date(28),
            date(24),
            5,
        )
        .with_authority(role())
        .with_state(ResponseState::InProgress),
        rating.clone(),
        rating,
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence("risk"))
}

fn amount_consequence(
    id: &str,
    fact_seq: u64,
    amount: &str,
    currency: &str,
) -> ForecastConsequence {
    ForecastConsequence::new(
        project(),
        control(id),
        control("risk.switchgear"),
        fact_seq,
        control("scenario.accepted"),
        ForecastConsequenceKind::Amount,
        ForecastValue::Amount(
            CommercialAmount::parse(amount, CurrencyCode::new(currency).unwrap()).unwrap(),
        ),
        basis(fact_seq),
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence(id))
}

fn raw_amount_consequence(id: &str, fact_seq: u64, amount: Amount) -> ForecastConsequence {
    ForecastConsequence::new(
        project(),
        control(id),
        control("risk.switchgear"),
        fact_seq,
        control("scenario.accepted"),
        ForecastConsequenceKind::Amount,
        ForecastValue::Amount(
            CommercialAmount::new(amount, CurrencyCode::new("SEK").unwrap()).unwrap(),
        ),
        basis(fact_seq),
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence(id))
}

fn accepted_baseline() -> AcceptedBaseline {
    AcceptedBaseline::new(
        baseline_id(),
        project(),
        control("baseline.schedule"),
        BaselineKind::Time,
        role(),
        1,
        date(1),
    )
    .with_evidence(evidence("baseline"))
}

fn joins() -> ScheduleTaskJoinSet {
    joins_at(5)
}

fn joins_at(as_of_seq: u64) -> ScheduleTaskJoinSet {
    ScheduleTaskJoinSet::new(
        ScheduleBaseline::new(baseline_id(), "plan", "rev-a", 1).unwrap(),
        SchedulePlanRevision::new("plan", "rev-a", as_of_seq).unwrap(),
        vec![ScheduleTaskJoin::new(
            control("package.electrical"),
            "energization",
            ScheduleJoinKind::Package,
        )],
    )
    .unwrap()
}

fn schedule() -> ScheduleStatusReport {
    schedule_at(5)
}

fn schedule_at(as_of_seq: u64) -> ScheduleStatusReport {
    ScheduleStatusReport {
        baseline: baseline_id(),
        accepted_revision: "rev-a".to_owned(),
        as_of_seq,
        critical_tasks: vec!["energization".to_owned()],
        explanations: Vec::new(),
    }
}

fn graph() -> ControlGraph {
    let mut graph = ControlGraph::new();
    graph
        .add_node(control("risk.switchgear"), ControlNodeKind::Risk)
        .unwrap();
    graph
        .add_node(control("package.electrical"), ControlNodeKind::Package)
        .unwrap();
    graph
        .add_edge(
            control("risk.switchgear"),
            control("package.electrical"),
            ControlEdgeKind::Affects,
        )
        .unwrap();
    graph
}

fn fact_book(subjects: &[&str]) -> ProjectBook {
    let mut book = ProjectBook::new(project(), role());
    for (index, subject) in subjects.iter().enumerate() {
        let seq = u64::try_from(index + 1).unwrap();
        book.append(ProjectFact::new(
            seq,
            project(),
            control(subject),
            Symbol::qualified("construction", "uncertainty"),
            date(u8::try_from(index + 1).unwrap()),
            role(),
            Expr::String((*subject).to_owned()),
        ))
        .unwrap();
    }
    book
}

fn basis(as_of_seq: u64) -> ForecastBasis {
    ForecastBasis::new(
        baseline_id(),
        "discipline estimate",
        as_of_seq,
        date(u8::try_from(as_of_seq).unwrap()),
    )
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn baseline_id() -> BaselineId {
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
