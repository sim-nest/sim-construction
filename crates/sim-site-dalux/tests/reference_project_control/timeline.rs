use sim_kernel::{Expr, Symbol};
use sim_lib_construction_project::{EvidenceState, ProjectFact, Visibility};
use sim_lib_doc_core::ExternalRef;

use super::support::{day, id, project, reference, role};

pub(super) fn facts(
    schedule_evidence: Vec<ExternalRef>,
    field_evidence: Vec<ExternalRef>,
) -> Vec<ProjectFact> {
    let mut facts = vec![
        accepted(1, "opportunity.renovation", "invented customer invitation"),
        accepted(2, "intent.scope", "occupied market hall remains open"),
        accepted(3, "intent.scope", "market hall closes completely"),
        accepted(
            4,
            "decision.customer-access",
            "late customer access decision",
        ),
        accepted(5, "contract.main", "signed main contract evidence"),
        accepted(6, "gate.mobilization", "accountable mobilization approval"),
        accepted(7, "measurement.area", "gross area 1860 m2"),
        accepted(8, "measurement.area", "corrected gross area 1840 m2").supersedes(7),
        state(
            9,
            "prerequisite.workplace-introduction",
            "workplace introduction not supplied",
            EvidenceState::Missing,
        ),
        state(
            10,
            "supplier.blue-arc",
            "supplier insurance expired",
            EvidenceState::Expired,
        ),
        accepted(11, "supplier.blue-arc", "renewed supplier insurance").supersedes(10),
        accepted(12, "design.fire", "fire design revision C released"),
        accepted(13, "permit.fire", "authority permit granted"),
        accepted(14, "procurement.electrical", "electrical package awarded"),
        accepted(15, "handoff.electrical", "supplier handoff accepted"),
        accepted(16, "outcome.climate", "reported 194500 kg-co2e"),
        accepted(17, "outcome.climate", "corrected 184500 kg-co2e").supersedes(16),
        state(
            18,
            "schedule.critical",
            "critical completion effect five days",
            EvidenceState::Reported,
        ),
        state(
            19,
            "safety.energization",
            "non-waivable energization isolation blocker",
            EvidenceState::Reported,
        ),
        accepted(
            20,
            "exception.delivery-window",
            "bounded delivery-window exception valid through July 20",
        ),
        state(
            21,
            "exception.delivery-window",
            "bounded delivery-window exception expired",
            EvidenceState::Expired,
        )
        .supersedes(20),
        accepted(22, "safety.energization", "isolation inspection accepted").supersedes(19),
        accepted(23, "risk.switchgear", "alternate supplier response active"),
        state(
            24,
            "change.ventilation",
            "customer quotation 460000 SEK",
            EvidenceState::Evidenced,
        ),
        accepted(
            25,
            "change.ventilation",
            "customer partially approved 275000 SEK",
        )
        .supersedes(24),
        state(
            26,
            "handover.defect.controls",
            "controls trend defect remains open",
            EvidenceState::Reported,
        ),
        accepted(
            27,
            "handover.defect.controls",
            "controls trend defect corrected and inspected",
        )
        .supersedes(26),
        accepted(28, "closeout.warranty", "warranty contact handed over"),
        accepted(29, "closeout.retention", "retention policy recorded"),
        accepted(30, "closeout.unresolved", "unresolved work register empty"),
        accepted(31, "closeout.evidence", "evidence disposition accepted"),
        accepted(32, "closeout.lesson", "delivery-window lesson accepted"),
        accepted(33, "closeout.final", "accountable closeout decision"),
        reference_fact(34, "charter.people", "people learning target"),
        state(
            35,
            "outcome.people",
            "people learning evidence awaits acceptance",
            EvidenceState::Reported,
        )
        .with_visibility(Visibility::ReferenceCandidate),
        reference_fact(36, "outcome.people", "people learning evidence accepted").supersedes(35),
        reference_fact(37, "charter.place", "occupied-hall continuity target"),
        reference_fact(38, "outcome.place", "occupied-hall continuity achieved"),
        reference_fact(
            39,
            "lesson.delivery-window",
            "bounded exceptions require explicit renewal",
        ),
    ];
    for evidence in schedule_evidence {
        facts[17] = facts[17].clone().with_evidence(evidence);
    }
    for evidence in field_evidence {
        facts[18] = facts[18].clone().with_evidence(evidence);
    }
    facts
}

pub(super) fn accepted(sequence: u64, subject: &str, body: &str) -> ProjectFact {
    state(sequence, subject, body, EvidenceState::Accepted)
}

fn reference_fact(sequence: u64, subject: &str, body: &str) -> ProjectFact {
    accepted(sequence, subject, body).with_visibility(Visibility::ReferenceCandidate)
}

fn state(sequence: u64, subject: &str, body: &str, state: EvidenceState) -> ProjectFact {
    ProjectFact::new(
        sequence,
        project(),
        id(subject),
        Symbol::qualified("construction-scenario", "control"),
        day(u8::try_from(sequence.min(30)).unwrap()),
        role("project-chief"),
        Expr::String(body.to_owned()),
    )
    .with_evidence_state(state)
    .with_evidence(reference(&format!("evidence/{subject}/{sequence}"), "A"))
}
