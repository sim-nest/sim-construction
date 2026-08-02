use std::sync::Arc;

use sim_kernel::{Cx, DefaultFactory, NoopEvalPolicy, Symbol};
use sim_ledger::Amount;
use sim_lib_construction_office::{
    OfficePackRequest, PackCadence, PackControl, PackSection, project_office_pack,
};
use sim_lib_construction_project::{
    AccountableCloseout, CloseoutControlSet, CloseoutDecision, CloseoutObligation,
    CloseoutObligationKind, CommercialEvidenceSource, DisclosureClearance, DisclosureCondition,
    EvidenceState, FinalEconomyAmountFact, FinalEconomyBasis, FinalEconomyControl,
    FinalEconomyFactKind, FinalEconomyReconciliation, OutcomeControlReport, OutcomeTargetReport,
    OutcomeVariance, ProjectBook, ProjectBookRepository, ProjectObligation,
    ReferenceAdmissionBlocker, ReferenceApproval, ReferenceClaim, ReferenceClaimKind,
    ReferenceDecisionKind, ReferencePackAdmission, Requirement, RequirementLane,
    SnapshotExplanationKind, Visibility, construction_project_read_capability,
};
use sim_lib_doc_core::ExternalRef;

use super::support::{currency, day, id, project, reference as ext, role};
use super::timeline::accepted;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ScenarioProof {
    pub domains: super::domain::DomainProof,
    pub conflict_visible: bool,
    pub handover_defect_corrected: bool,
    pub initial_reference_admitted: bool,
    pub snapshot_bytes: Vec<u8>,
    pub changed_controls: Vec<String>,
    pub superseded_controls: Vec<String>,
    pub final_reference_claims: Vec<String>,
    pub visibility_non_interference: bool,
}

pub(super) fn run(
    cx: &mut Cx,
    repository: &ProjectBookRepository,
    schedule_evidence: Vec<ExternalRef>,
    field_evidence: Vec<ExternalRef>,
) -> ScenarioProof {
    for fact in super::timeline::facts(schedule_evidence, field_evidence) {
        repository.append_fact(cx, fact).unwrap();
    }

    let boundary_sequences = [
        2, 3, 7, 8, 10, 11, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 35, 36,
    ];
    let snapshots = boundary_sequences
        .into_iter()
        .map(|sequence| repository.read_snapshot(cx, sequence).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(snapshots.len(), 18);

    let scope = id("intent.scope");
    assert!(snapshots[0].current.contains_key(&scope));
    assert!(snapshots[1].is_conflicted(&scope));
    assert!(snapshots[1].explanations.iter().any(|row| {
        row.subject == scope && row.explanation == SnapshotExplanationKind::Conflicted
    }));
    assert_supersession(&snapshots[3], "measurement.area", 7, 8);
    assert_supersession(&snapshots[5], "supplier.blue-arc", 10, 11);
    assert_supersession(&snapshots[7], "outcome.climate", 16, 17);
    assert_eq!(
        snapshots[8]
            .current_fact(&id("safety.energization"))
            .unwrap()
            .evidence_state,
        EvidenceState::Reported
    );
    assert_eq!(
        snapshots[10]
            .current_fact(&id("exception.delivery-window"))
            .unwrap()
            .evidence_state,
        EvidenceState::Expired
    );
    assert_supersession(&snapshots[11], "safety.energization", 19, 22);
    assert_supersession(&snapshots[13], "change.ventilation", 24, 25);
    assert_supersession(&snapshots[15], "handover.defect.controls", 26, 27);
    assert_eq!(
        snapshots[16]
            .current_fact(&id("outcome.people"))
            .unwrap()
            .evidence_state,
        EvidenceState::Reported
    );
    assert_supersession(&snapshots[17], "outcome.people", 35, 36);

    let book = repository.read_book(cx, 39).unwrap();
    let changed_controls = book
        .delta(33, 39)
        .unwrap()
        .added
        .iter()
        .map(|control| control.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        changed_controls,
        [
            "charter.people",
            "charter.place",
            "lesson.delivery-window",
            "outcome.people",
            "outcome.place",
        ]
    );

    let final_snapshot = book.snapshot_at(39).unwrap();
    let superseded_controls = final_snapshot
        .superseded
        .keys()
        .map(|control| control.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        superseded_controls,
        [
            "change.ventilation",
            "exception.delivery-window",
            "handover.defect.controls",
            "measurement.area",
            "outcome.climate",
            "outcome.people",
            "safety.energization",
            "supplier.blue-arc",
        ]
    );

    let closeout = close_project(&book);
    let (initial_admitted, final_reference_claims) = admit_reference(&book, &closeout);
    assert!(!initial_admitted);
    assert_eq!(
        final_reference_claims,
        ["claim.lesson", "claim.people", "claim.place"]
    );

    let visibility_non_interference = prove_visibility_non_interference();
    assert!(visibility_non_interference);
    let domains = super::domain::run(cx);
    assert!(domains.opportunity_bid_collaboration);
    assert!(domains.late_customer_decision);
    assert!(domains.missing_collaboration_evidence);
    assert!(domains.mobilization);
    assert!(domains.design_permit);
    assert!(domains.procurement_supplier_handoff);
    assert!(domains.supplier_expired_then_renewed);
    assert!(domains.sustainability);
    assert!(domains.lookahead);
    assert!(domains.field_safety);
    assert!(domains.non_waivable_safety_blocked);
    assert!(domains.bounded_exception_expired);
    assert!(domains.risk);
    assert!(domains.change_economy);
    assert!(domains.partial_change_approval);
    assert!(domains.double_count_prevented);
    assert_eq!(domains.critical_schedule_effect_days, 5);

    ScenarioProof {
        domains,
        conflict_visible: snapshots[1].is_conflicted(&scope),
        handover_defect_corrected: snapshots[15]
            .current_fact(&id("handover.defect.controls"))
            .is_some_and(|fact| fact.seq == 27),
        initial_reference_admitted: initial_admitted,
        snapshot_bytes: serde_json::to_vec(&snapshots).unwrap(),
        changed_controls,
        superseded_controls,
        final_reference_claims,
        visibility_non_interference,
    }
}

fn close_project(book: &ProjectBook) -> AccountableCloseout {
    let controls = closeout_controls();
    let economy = final_economy().derive().unwrap();
    let report = controls.report(book, &economy, 32, day(30)).unwrap();
    assert!(report.ready);
    let closeout = CloseoutDecision::new(id("closeout.final"), 32, 33, role("project-director"))
        .with_evidence(ext("closeout/decision", "B"))
        .close(&controls, &report)
        .unwrap();
    assert_eq!(closeout.decision_seq(), 33);
    closeout
}

fn closeout_controls() -> CloseoutControlSet {
    [
        (
            CloseoutObligationKind::WarrantyContactHandoff,
            "closeout.warranty",
        ),
        (
            CloseoutObligationKind::RetentionPolicy,
            "closeout.retention",
        ),
        (
            CloseoutObligationKind::UnresolvedWork,
            "closeout.unresolved",
        ),
        (
            CloseoutObligationKind::EvidenceDisposition,
            "closeout.evidence",
        ),
        (CloseoutObligationKind::Lesson, "closeout.lesson"),
    ]
    .into_iter()
    .fold(
        CloseoutControlSet::new(project(), id("closeout.final"), role("project-director")),
        |controls, (kind, requirement)| {
            controls.with_obligation(CloseoutObligation::new(
                kind,
                ProjectObligation::mandatory(
                    project(),
                    Requirement::new(
                        id(requirement),
                        RequirementLane::new(Symbol::qualified("construction", "closeout")),
                        format!("{kind:?}"),
                        role("project-chief"),
                        role("project-director"),
                    )
                    .with_evidence_kind(Symbol::qualified("construction", "closeout-evidence"))
                    .with_source_ref(ext(&format!("policy/{requirement}"), "A")),
                ),
            ))
        },
    )
}

fn final_economy() -> FinalEconomyControl {
    let basis = FinalEconomyBasis::new(32, day(30), "accepted synthetic closeout cutoff");
    let reconciliation = FinalEconomyReconciliation::new(
        id("economy.final"),
        id("economy.ledger"),
        "final position matches the referenced ledger balance",
    );
    [
        (
            "economy.contract",
            FinalEconomyFactKind::AcceptedContract,
            "28100000.00",
            CommercialEvidenceSource::Document,
        ),
        (
            "economy.forecast",
            FinalEconomyFactKind::CurrentForecast,
            "28285000.00",
            CommercialEvidenceSource::Document,
        ),
        (
            "economy.final",
            FinalEconomyFactKind::FinalPosition,
            "28285000.00",
            CommercialEvidenceSource::Document,
        ),
        (
            "economy.ledger",
            FinalEconomyFactKind::LedgerBalance,
            "28285000.00",
            CommercialEvidenceSource::LedgerBalance,
        ),
    ]
    .into_iter()
    .enumerate()
    .fold(
        FinalEconomyControl::new(project(), currency(), basis, reconciliation),
        |control, (index, (fact_id, kind, value, source))| {
            control.with_fact(
                FinalEconomyAmountFact::new(
                    project(),
                    id(fact_id),
                    kind,
                    Amount::parse(value).unwrap(),
                    currency(),
                    day(30),
                    u64::try_from(index + 1).unwrap(),
                    source,
                    ext(fact_id, "closeout"),
                )
                .with_evidence_state(EvidenceState::Accepted),
            )
        },
    )
}

fn admit_reference(book: &ProjectBook, closeout: &AccountableCloseout) -> (bool, Vec<String>) {
    let initial = reference_claim(
        "claim.people",
        ReferenceClaimKind::PeopleDevelopment,
        34,
        35,
    )
    .requires_consent()
    .asserts_outcome(id("outcome.people"));
    let initial_report =
        ReferencePackAdmission::new(project(), 35, day(30), role("reference-authority"))
            .with_claim(initial)
            .with_clearance(
                DisclosureClearance::new(id("claim.people"))
                    .with_consent(DisclosureCondition::Satisfied(ext("consent/current", "A"))),
            )
            .with_approval(reference_approval("claim.people", 35, 40))
            .evaluate(book, closeout, &[outcome_report("outcome.people")])
            .unwrap();
    assert!(initial_report.manifest.is_none());
    assert!(initial_report.claims[0].blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ReferenceAdmissionBlocker::SourceFactNotAccepted {
                sequence: 35,
                state: EvidenceState::Reported,
            }
        )
    }));

    let people = reference_claim(
        "claim.people",
        ReferenceClaimKind::PeopleDevelopment,
        34,
        36,
    )
    .requires_consent()
    .asserts_outcome(id("outcome.people"));
    let place = reference_claim("claim.place", ReferenceClaimKind::PropertyOutcome, 37, 38)
        .asserts_outcome(id("outcome.place"));
    let lesson = reference_claim("claim.lesson", ReferenceClaimKind::Lesson, 32, 39);
    let report = ReferencePackAdmission::new(project(), 39, day(30), role("reference-authority"))
        .with_claim(people)
        .with_claim(place)
        .with_claim(lesson)
        .with_clearance(
            DisclosureClearance::new(id("claim.people"))
                .with_consent(DisclosureCondition::Satisfied(ext("consent/current", "A"))),
        )
        .with_approval(reference_approval("claim.people", 39, 40))
        .with_approval(reference_approval("claim.place", 39, 41))
        .with_approval(reference_approval("claim.lesson", 39, 42))
        .evaluate(
            book,
            closeout,
            &[
                outcome_report("outcome.people"),
                outcome_report("outcome.place"),
            ],
        )
        .unwrap();
    let manifest = report.manifest.unwrap();
    let claims = manifest
        .claims()
        .iter()
        .map(|claim| claim.claim_id().as_str().to_owned())
        .collect::<Vec<_>>();
    (initial_report.manifest.is_some(), claims)
}

fn reference_claim(
    claim_id: &str,
    kind: ReferenceClaimKind,
    charter_seq: u64,
    outcome_seq: u64,
) -> ReferenceClaim {
    ReferenceClaim::new(
        project(),
        id(claim_id),
        kind,
        format!("invented Nordhamn {kind:?} claim"),
        charter_seq,
        Visibility::ReferenceCandidate,
    )
    .with_source_fact(outcome_seq)
    .with_external_ref(ext(&format!("reference/{claim_id}"), "C"))
}

fn reference_approval(claim_id: &str, report_seq: u64, decision_seq: u64) -> ReferenceApproval {
    ReferenceApproval::new(
        id(claim_id),
        id(&format!("decision.{claim_id}")),
        report_seq,
        decision_seq,
        ReferenceDecisionKind::Approve,
        role("reference-authority"),
    )
    .with_evidence(ext(&format!("approval/{claim_id}"), "C"))
}

fn outcome_report(target: &str) -> OutcomeControlReport {
    OutcomeControlReport {
        project: project(),
        as_of: day(30),
        targets: vec![OutcomeTargetReport {
            target: id(target),
            current_record: Some(id(&format!("record.{target}"))),
            forecasts: Vec::new(),
            covered: true,
            variance: OutcomeVariance::OnTarget,
            blockers: Vec::new(),
            reference_claim_admissible: true,
        }],
        gates_clear: true,
    }
}

fn prove_visibility_non_interference() -> bool {
    fn signature(secret: &str) -> String {
        let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
        cx.grant(construction_project_read_capability());
        let mut book = ProjectBook::new(project(), role("project-chief"));
        book.append(accepted(1, "schedule.public", "critical task delayed"))
            .unwrap();
        book.append(accepted(2, "commercial.private", secret).with_visibility(
            Visibility::Restricted(Symbol::qualified("construction", "commercial")),
        ))
        .unwrap();
        let request = OfficePackRequest::new(
            PackCadence::MonthlyGate,
            role("project-chief"),
            2,
            day(30),
            "2026-07-30T06:00:00Z",
        )
        .with_control(PackControl::mandatory(
            id("schedule.public"),
            PackSection::CriticalSchedule,
        ))
        .with_control(PackControl::optional(
            id("commercial.private"),
            PackSection::RiskChangeEconomy,
        ));
        format!(
            "{:?}",
            project_office_pack(&mut cx, &book, &request).unwrap()
        )
    }

    let left = signature("invented confidential value 111");
    let right = signature("invented confidential value 999");
    left == right && !left.contains("111") && !left.contains("999")
}

fn assert_supersession(
    snapshot: &sim_lib_construction_project::ProjectSnapshot,
    control: &str,
    prior: u64,
    current: u64,
) {
    assert_eq!(snapshot.current_fact(&id(control)).unwrap().seq, current);
    assert_eq!(
        snapshot.superseded[&id(control)]
            .iter()
            .map(|fact| fact.seq)
            .collect::<Vec<_>>(),
        [prior]
    );
}
