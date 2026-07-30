use sim_kernel::{Expr, Symbol};
use sim_lib_construction_project::{
    BaselineId, ControlId, CurrencyCode, EvidenceState, ProjectBook, ProjectFact, ProjectId,
    ProjectObligation, Requirement, RequirementLane, RoleId, Visibility,
};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

pub(super) fn project() -> ProjectId {
    ProjectId::new("project.nordhamn-market-renovation").unwrap()
}

pub(super) fn id(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

pub(super) fn role(value: &str) -> RoleId {
    RoleId::new(value).unwrap()
}

pub(super) fn currency() -> CurrencyCode {
    CurrencyCode::new("SEK").unwrap()
}

pub(super) fn baseline_id() -> BaselineId {
    BaselineId::new("baseline.schedule.accepted-C").unwrap()
}

pub(super) fn reference(id: &str, version: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some(version.to_owned()), None)
}

pub(super) fn day(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

pub(super) fn august(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::August, day).unwrap()
}

pub(super) fn mandatory(control: &str, lane: &str) -> ProjectObligation {
    ProjectObligation::mandatory(
        project(),
        Requirement::new(
            id(control),
            RequirementLane::new(Symbol::qualified("construction-lane", lane)),
            control,
            role("package-lead"),
            role("project-chief"),
        )
        .with_evidence_kind(Symbol::qualified("construction-evidence", "external-ref"))
        .with_source_ref(reference(&format!("requirement/{control}"), "A")),
    )
}

pub(super) fn control_fact(sequence: u64, subject: &str, state: EvidenceState) -> ProjectFact {
    ProjectFact::new(
        sequence,
        project(),
        id(subject),
        Symbol::qualified("construction-scenario", "control"),
        day(u8::try_from(sequence.min(30)).unwrap()),
        role("project-chief"),
        Expr::String(subject.to_owned()),
    )
    .with_evidence_state(state)
    .with_visibility(Visibility::Project)
    .with_evidence(reference(&format!("evidence/{subject}"), "A"))
}

pub(super) fn accepted_book(subject: &str) -> ProjectBook {
    let mut book = ProjectBook::new(project(), role("project-chief"));
    book.append(control_fact(1, subject, EvidenceState::Accepted))
        .unwrap();
    book
}

pub(super) fn assert_synthetic_text(label: &str, text: &str) {
    let text = text.to_ascii_lowercase();
    let at_sign = char::from(64);
    let forbidden = [
        ["zen", "gun"].concat(),
        ["road", "map"].concat(),
        ["his", "tory"].concat(),
        ["http", "://"].concat(),
        ["https", "://"].concat(),
        ["www", "."].concat(),
        ["bear", "er"].concat(),
        ["api", "-key"].concat(),
        ["access", "-token"].concat(),
        ["pass", "word"].concat(),
        ["sec", "ret"].concat(),
        ["dalux", ".com"].concat(),
    ];
    for denied in forbidden {
        assert!(
            !text.contains(&denied),
            "{label} contains forbidden synthetic-data marker {denied:?}"
        );
    }
    assert!(
        !text.contains(at_sign),
        "{label} contains an email-like identity marker"
    );
}
