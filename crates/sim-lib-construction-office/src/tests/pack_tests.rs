use sim_lib_construction_project::{BaselineId, ProjectFact};
use sim_lib_deck::SlideBlock;
use sim_lib_sheet::{CellRef, CellValue};

use super::*;
use crate::{
    OFFICE_PACK_VISIBILITY_POLICY, OfficePack, OfficePackRequest, PackCadence, PackControl,
    PackSection, office_restricted_read_capability, project_office_pack,
};

// conformance: construction snapshots project into visibility-safe office packs.

#[test]
fn weekly_pack_is_byte_stable_sorted_and_safety_first() {
    let mut fixture = PackFixture::new();
    fixture.append(pack_fact(
        1,
        "schedule.main",
        "schedule",
        "late by two days",
        EvidenceState::Reported,
        Visibility::Project,
    ));
    fixture.append(
        pack_fact(
            2,
            "decision.crane",
            "decision",
            "approve lift plan",
            EvidenceState::Accepted,
            Visibility::Project,
        )
        .with_evidence(external("z-evidence")),
    );
    fixture.append(
        pack_fact(
            3,
            "safety.lift",
            "safety",
            "wind hold",
            EvidenceState::Rejected,
            Visibility::Project,
        )
        .with_evidence(external("a-evidence")),
    );
    let request = request(PackCadence::Weekly, 3)
        .with_baseline(baseline("schedule-b"))
        .with_baseline(baseline("schedule-a"))
        .with_control(control(
            "schedule.main",
            PackSection::CriticalSchedule,
            true,
        ))
        .with_control(control(
            "safety.lift",
            PackSection::SafetyLegalBlockers,
            true,
        ))
        .with_control(control("decision.crane", PackSection::Decisions, true));

    let first = fixture.project(&request);
    let second = fixture.project(&request);
    assert_eq!(
        signature(&mut fixture.cx, &first),
        signature(&mut fixture.cx, &second)
    );
    assert_eq!(
        first
            .deck
            .slides
            .iter()
            .map(|slide| slide.id.as_str())
            .collect::<Vec<_>>(),
        [
            "provenance",
            "decisions",
            "safety-legal-blockers",
            "evidence-exceptions"
        ]
    );
    assert_eq!(sheet_text(&first, "B6"), "schedule-a, schedule-b");
    assert_eq!(
        sheet_text(&first, "B7"),
        "decision.crane, safety.lift, schedule.main"
    );
    assert_eq!(sheet_text(&first, "B10"), OFFICE_PACK_VISIBILITY_POLICY);
    assert_eq!(sheet_text(&first, "B12"), "rejected");
    assert!(sheet_text(&first, "B13").contains("mandatory control(s) are not accepted"));
    assert_eq!(
        first.deck.slides[1].blocks,
        [SlideBlock::BulletList(vec![
            "decision.crane | String(\"approve lift plan\") | accepted: current accepted fact at sequence 2 by project-chief effective 2026-07-30 | changed=false".to_owned()
        ])]
    );
}

#[test]
fn denied_supplier_commercial_people_and_incident_facts_do_not_interfere() {
    let request = request(PackCadence::MonthlyGate, 5)
        .with_control(control("readiness.public", PackSection::Readiness, true))
        .with_control(control(
            "commercial.private",
            PackSection::RiskChangeEconomy,
            false,
        ))
        .with_control(control("people.private", PackSection::Outcomes, false))
        .with_control(control(
            "incident.private",
            PackSection::SafetyLegalBlockers,
            false,
        ))
        .with_control(control("supplier.private", PackSection::Procurement, false));
    let mut left = secret_fixture("left");
    let mut right = secret_fixture("right");

    let left_pack = left.project(&request);
    let right_pack = right.project(&request);
    let left_signature = signature(&mut left.cx, &left_pack);
    assert_eq!(left_signature, signature(&mut right.cx, &right_pack));
    for denied in [
        "left-commercial",
        "left-people",
        "left-incident",
        "left-supplier",
        "https://secret.invalid",
    ] {
        assert!(!left_signature.contains(denied));
    }

    left.cx
        .grant(sim_lib_construction_project::construction_supplier_read_capability());
    for lane in ["commercial", "people", "incident"] {
        left.cx
            .grant(office_restricted_read_capability(&Symbol::qualified(
                "office", lane,
            )));
    }
    let disclosed_pack = left.project(&request);
    let disclosed = signature(&mut left.cx, &disclosed_pack);
    assert!(disclosed.contains("left-commercial"));
    assert!(disclosed.contains("left-people"));
    assert!(disclosed.contains("left-incident"));
    assert!(disclosed.contains("left-supplier"));
}

#[test]
fn empty_project_produces_complete_pack_and_suite_scene() {
    let mut fixture = PackFixture::new();
    let pack = fixture.project(&request(PackCadence::Daily, 0));

    assert_eq!(sheet_text(&pack, "B4"), "0");
    assert_eq!(sheet_text(&pack, "B7"), "");
    assert_eq!(sheet_text(&pack, "B8"), "");
    assert_eq!(sheet_text(&pack, "B12"), "missing");
    assert!(sheet_text(&pack, "B13").contains("no mandatory source controls"));
    let documents = pack.documents(&mut fixture.cx).unwrap();
    assert_eq!(
        documents
            .iter()
            .map(|doc| doc.kind.as_str())
            .collect::<Vec<_>>(),
        ["report", "sheet", "deck"]
    );
    let scene = pack.suite_scene(&mut fixture.cx).unwrap();
    let scene = scene.object().as_expr(&mut fixture.cx).unwrap();
    assert!(format!("{scene:?}").contains("office-suite"));
}

#[test]
fn historical_pack_does_not_read_future_corrections() {
    let mut fixture = PackFixture::new();
    fixture.append(
        pack_fact(
            1,
            "decision.access",
            "decision",
            "original accepted decision",
            EvidenceState::Accepted,
            Visibility::Project,
        )
        .with_evidence(external("original")),
    );
    fixture.append(
        pack_fact(
            2,
            "decision.access",
            "decision",
            "future reported correction",
            EvidenceState::Reported,
            Visibility::Project,
        )
        .supersedes(1)
        .with_evidence(external("future")),
    );
    let historical = request(PackCadence::MonthlyGate, 1).with_control(control(
        "decision.access",
        PackSection::Decisions,
        true,
    ));
    let current = request(PackCadence::MonthlyGate, 2).with_control(control(
        "decision.access",
        PackSection::Decisions,
        true,
    ));

    let past_pack = fixture.project(&historical);
    let past = signature(&mut fixture.cx, &past_pack);
    let current_pack = fixture.project(&current);
    let now = signature(&mut fixture.cx, &current_pack);
    assert!(past.contains("original accepted decision"));
    assert!(!past.contains("future reported correction"));
    assert!(now.contains("future reported correction"));
    assert!(now.contains("reported"));
}

#[test]
fn changed_since_meeting_is_derived_from_the_filtered_book() {
    let mut fixture = PackFixture::new();
    fixture.append(pack_fact(
        1,
        "schedule.main",
        "schedule",
        "baseline",
        EvidenceState::Accepted,
        Visibility::Project,
    ));
    fixture.append(pack_fact(
        2,
        "readiness.zone-a",
        "readiness",
        "not ready",
        EvidenceState::Reported,
        Visibility::Project,
    ));
    fixture.append(
        pack_fact(
            3,
            "schedule.main",
            "schedule",
            "recovered",
            EvidenceState::Accepted,
            Visibility::Project,
        )
        .supersedes(1),
    );
    let pack = fixture.project(
        &request(PackCadence::Weekly, 3)
            .changed_since(1)
            .with_control(control(
                "schedule.main",
                PackSection::CriticalSchedule,
                true,
            ))
            .with_control(control("readiness.zone-a", PackSection::Readiness, true)),
    );

    let rendered = signature(&mut fixture.cx, &pack);
    assert!(rendered.contains("changed-since-sequence"));
    assert!(rendered.contains("changed=true"));
    assert_eq!(sheet_text(&pack, "B11"), "1");
}

#[test]
fn every_cadence_builds_doc_sheet_deck_and_appropriate_sections() {
    let cadences = [
        PackCadence::Daily,
        PackCadence::Weekly,
        PackCadence::MonthlyGate,
        PackCadence::Handover,
        PackCadence::Closeout,
        PackCadence::ReferenceReview,
    ];
    for cadence in cadences {
        let mut fixture = PackFixture::new();
        fixture.append(pack_fact(
            1,
            "decision.one",
            "decision",
            cadence.as_str(),
            EvidenceState::Accepted,
            Visibility::Project,
        ));
        let pack = fixture.project(&request(cadence, 1).with_control(control(
            "decision.one",
            PackSection::Decisions,
            true,
        )));
        assert_eq!(pack.doc.kind.as_str(), "report");
        assert_eq!(
            pack.sheet.name,
            format!("reference-center {}", cadence.as_str())
        );
        assert_eq!(pack.deck.slides[1].id, "decisions");
        assert_eq!(pack.documents(&mut fixture.cx).unwrap().len(), 3);
    }
}

struct PackFixture {
    cx: Cx,
    book: ProjectBook,
}

impl PackFixture {
    fn new() -> Self {
        let mut cx = authorized_context();
        cx.grant(construction_project_read_capability());
        Self {
            cx,
            book: ProjectBook::new(project("reference-center"), role()),
        }
    }

    fn append(&mut self, fact: ProjectFact) {
        self.book.append(fact).unwrap();
    }

    fn project(&mut self, request: &OfficePackRequest) -> OfficePack {
        project_office_pack(&mut self.cx, &self.book, request).unwrap()
    }
}

fn secret_fixture(marker: &str) -> PackFixture {
    let mut fixture = PackFixture::new();
    fixture.append(pack_fact(
        1,
        "readiness.public",
        "readiness",
        "public-ready",
        EvidenceState::Accepted,
        Visibility::Project,
    ));
    fixture.append(
        pack_fact(
            2,
            "readiness.public",
            "supplier",
            format!("{marker}-supplier-conflict"),
            EvidenceState::Accepted,
            Visibility::Supplier,
        )
        .with_evidence(secret_external(marker, "supplier")),
    );
    for (seq, control_id, lane) in [
        (3, "commercial.private", "commercial"),
        (4, "people.private", "people"),
        (5, "incident.private", "incident"),
    ] {
        fixture.append(
            pack_fact(
                seq,
                control_id,
                lane,
                format!("{marker}-{lane}"),
                EvidenceState::Accepted,
                Visibility::Restricted(Symbol::qualified("office", lane)),
            )
            .with_evidence(secret_external(marker, lane)),
        );
    }
    fixture
}

fn pack_fact(
    seq: u64,
    control_id: &str,
    kind: &str,
    body: impl Into<String>,
    status: EvidenceState,
    visibility: Visibility,
) -> ProjectFact {
    ProjectFact::new(
        seq,
        project("reference-center"),
        super::control(control_id),
        Symbol::qualified("construction-pack-test", kind),
        Date::from_calendar_date(2026, Month::July, 30).unwrap(),
        role(),
        Expr::String(body.into()),
    )
    .with_evidence_state(status)
    .with_visibility(visibility)
}

fn request(cadence: PackCadence, as_of_seq: u64) -> OfficePackRequest {
    OfficePackRequest::new(
        cadence,
        role(),
        as_of_seq,
        Date::from_calendar_date(2026, Month::July, 30).unwrap(),
        "2026-07-30T06:00:00Z",
    )
}

fn control(id: &str, section: PackSection, mandatory: bool) -> PackControl {
    if mandatory {
        PackControl::mandatory(super::control(id), section)
    } else {
        PackControl::optional(super::control(id), section)
    }
}

fn baseline(id: &str) -> BaselineId {
    BaselineId::new(id).unwrap()
}

fn secret_external(marker: &str, lane: &str) -> ExternalRef {
    ExternalRef::new(
        "secret",
        format!("{marker}-{lane}"),
        Some("private".to_owned()),
        Some("https://secret.invalid".to_owned()),
    )
}

fn signature(cx: &mut Cx, pack: &OfficePack) -> String {
    format!(
        "{:?}",
        pack.documents(cx)
            .unwrap()
            .into_iter()
            .map(|doc| doc.body.object().as_expr(cx).unwrap())
            .collect::<Vec<_>>()
    )
}

fn sheet_text(pack: &OfficePack, cell: &str) -> String {
    match pack.sheet.cell(&CellRef::parse(cell).unwrap()) {
        CellValue::Text(value) => value,
        other => panic!("expected text at {cell}, got {other:?}"),
    }
}
