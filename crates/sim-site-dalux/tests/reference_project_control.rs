use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde_json::json;
use sim_codec::{Input, Output, decode_with_codec, encode_with_codec};
use sim_codec_mspdi::{MspdiCodec, doc_to_plan, plan_to_doc};
use sim_kernel::{
    CapabilityName, Cx, DefaultFactory, EncodeOptions, Expr, NoopEvalPolicy, ReadPolicy, Symbol,
};
use sim_lib_construction_office::{
    OfficePackRequest, PackCadence, PackControl, PackSection, project_office_pack,
};
use sim_lib_construction_project::{
    BaselineId, ConstructionProjectLib, ControlId, ProjectId, RoleId,
    construction_project_read_capability, construction_project_write_capability,
    construction_reference_publish_capability,
};
use sim_lib_doc_core::{
    CREDENTIALS_CAPABILITY, DocCodec, DocCodecOptions, ExternalRef, NET_CONNECT_CAPABILITY,
    PROCESS_SPAWN_CAPABILITY,
};
use sim_lib_gantt::{GanttPlan, LinkKind, Task, TaskLink};
use sim_lib_sheet::{CellRef, CellValue};
use sim_site_dalux::{
    DALUX_SITE_ID, DaluxClient, ModeledDalux, StaticDaluxCredentialProvider,
    get_project_items_with_receipt, register_dalux_site,
};
use sim_site_powerproject::{
    ModeledOleReceipt, POWERPROJECT_SITE_ID, import_modeled_ole_receipt, register_powerproject_site,
};
use sim_table_hash::HashTable;
use time::{Date, Month};

#[path = "reference_project_control/control.rs"]
mod control;
#[path = "reference_project_control/domain.rs"]
mod domain;
#[path = "reference_project_control/scenario.rs"]
mod scenario;
#[path = "reference_project_control/support.rs"]
mod support;
#[path = "reference_project_control/timeline.rs"]
mod timeline;

const EXPECTED_REQUIRES: &[&str] = &[
    "codec/lisp",
    "sim-run-core",
    "sim/construction-project",
    "sim-lib-construction-office",
    "table/hash",
    "office/gantt",
    "office/doc-core",
    "office/doc-site",
    "office/doc-surface",
    "codec/mspdi",
    "site/powerproject",
    "site/dalux",
    "ledger/books",
    "construction.project.read",
    "construction.project.write",
    "construction.reference.publish",
];

#[test]
fn reference_project_control_recipe_declares_only_public_components() {
    let root = recipe_root();
    let manifest = fs::read_to_string(root.join("recipe.toml")).expect("recipe manifest");
    let manifest: toml::Value = manifest.parse().expect("valid recipe manifest");
    let requires = manifest["requires"]
        .as_array()
        .expect("requires array")
        .iter()
        .map(|value| value.as_str().expect("string requirement"))
        .collect::<Vec<_>>();
    assert_eq!(requires, EXPECTED_REQUIRES);
    assert_eq!(manifest["setup"].as_str(), Some("setup.siml"));
    assert_eq!(manifest["main"].as_str(), Some("main.siml"));
    assert_eq!(manifest["expected"].as_str(), Some("expected.siml"));

    let setup = fs::read_to_string(root.join("setup.siml")).expect("recipe setup");
    let main = fs::read_to_string(root.join("main.siml")).expect("recipe main");
    for operation in [
        "construction/append",
        "construction/snapshot-as-of",
        "construction/diff-since",
        "construction/explain",
        "construction/gate-report",
        "construction/readiness",
        "construction/exposure",
        "construction/handover-burn-down",
        "construction/reference-admission",
    ] {
        assert!(
            main.contains(operation),
            "recipe omits public operation {operation}"
        );
    }
    for public_surface in [
        "project-office-pack",
        "GanttPlan",
        "DocCodec/decode",
        "DocCodec/encode",
        "import-modeled-ole-receipt",
        "get-project-items-with-receipt",
    ] {
        assert!(
            main.contains(public_surface),
            "recipe omits public surface {public_surface}"
        );
    }
    for stable_id in [
        "project.nordhamn-market-renovation",
        "schedule.critical",
        "evidence.schedule.accepted-C",
        "task.frame-install",
        "receipt.powerproject.accepted-C",
        "items/field-item-001",
        "office.pack.weekly",
        "journal/change-ventilation/seq-25",
    ] {
        assert!(setup.contains(stable_id));
        assert!(main.contains(stable_id));
    }
    for forbidden in ["crate::", "super::", "/src/", "private-rust", "unsafe"] {
        assert!(!setup.contains(forbidden));
        assert!(!main.contains(forbidden));
    }
}

#[test]
fn reference_project_control_semantic_golden_is_deterministic() {
    let first = run_modeled_reference_project();
    let second = run_modeled_reference_project();
    assert_eq!(first.summary, second.summary);
    assert_eq!(first.scenario, second.scenario);

    let root = recipe_root();
    let expected = fs::read_to_string(root.join("expected.siml")).expect("semantic golden");
    let mut cx = codec_context();
    let expected = decode_lisp(&mut cx, expected);
    let actual = decode_lisp(&mut cx, first.summary);
    assert!(actual.canonical_eq(&expected));
    assert_eq!(
        encode_lisp(&mut cx, &actual),
        encode_lisp(&mut cx, &expected)
    );

    for source in ["setup.siml", "main.siml"] {
        let text = fs::read_to_string(root.join(source)).expect("Lisp recipe source");
        let first = decode_lisp(&mut cx, text.clone());
        let second = decode_lisp(&mut cx, text);
        assert!(first.canonical_eq(&second));
        assert_eq!(encode_lisp(&mut cx, &first), encode_lisp(&mut cx, &second));
    }
}

#[test]
fn reference_project_control_modeled_mode_denies_network() {
    let outcome = run_modeled_reference_project();
    assert!(outcome.network_denied);
    assert!(outcome.process_denied);
    assert!(outcome.credentials_denied);

    let root = recipe_root();
    for source in ["recipe.toml", "setup.siml", "main.siml", "expected.siml"] {
        let text = fs::read_to_string(root.join(source)).expect("recipe fixture");
        for denied in ["http://", "https://", "Authorization:"] {
            assert!(!text.contains(denied), "{source} contains {denied:?}");
        }
    }
}

#[test]
fn reference_project_control_failure_and_correction_paths_are_explicit() {
    let proof = run_modeled_reference_project().scenario;
    assert!(proof.conflict_visible);
    assert!(proof.domains.late_customer_decision);
    assert!(proof.domains.missing_collaboration_evidence);
    assert!(proof.domains.supplier_expired_then_renewed);
    assert!(proof.domains.non_waivable_safety_blocked);
    assert!(proof.domains.bounded_exception_expired);
    assert_eq!(proof.domains.critical_schedule_effect_days, 5);
    assert!(proof.domains.partial_change_approval);
    assert!(proof.domains.double_count_prevented);
    assert!(proof.handover_defect_corrected);
    assert!(!proof.initial_reference_admitted);
}

#[test]
fn reference_project_control_fixtures_and_generated_evidence_are_synthetic() {
    let root = recipe_root();
    let sources = [
        "recipe.toml",
        "purpose.md",
        "setup.siml",
        "main.siml",
        "expected.siml",
    ]
    .map(|name| fs::read_to_string(root.join(name)).unwrap())
    .join("\n");
    support::assert_synthetic_text("primary recipe", &sources);

    for invented in [
        "North Quay Property Cooperative",
        "Aster Works AB",
        "Copperline Design AB",
        "Blue Arc Installations AB",
        "Pine Grid Controls AB",
        "Fjordglantan 18, 111 28 Nordhamn",
    ] {
        assert!(
            sources.contains(invented),
            "synthetic allowlist entry is missing: {invented}"
        );
    }

    let repository_root = root.parent().unwrap().parent().unwrap();
    let humans = fs::read_to_string(repository_root.join("docs/humans/README.md")).unwrap();
    let generated_section = humans
        .split_once("Specimen `recipe/sim-construction/reference-project-control`")
        .unwrap()
        .1
        .split_once("\nSpecimen `")
        .unwrap()
        .0;
    support::assert_synthetic_text("generated human recipe section", generated_section);

    let contract =
        fs::read_to_string(repository_root.join("docs/generated/repo-contract.json")).unwrap();
    let contract: serde_json::Value = serde_json::from_str(&contract).unwrap();
    let cookbook = contract["recipes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|book| {
            book["card_id"].as_str() == Some("cookbook/construction/reference-project-control")
        })
        .unwrap();
    support::assert_synthetic_text("generated recipe contract", &cookbook.to_string());
}

struct ModeledOutcome {
    summary: String,
    scenario: scenario::ScenarioProof,
    network_denied: bool,
    process_denied: bool,
    credentials_denied: bool,
}

fn run_modeled_reference_project() -> ModeledOutcome {
    let project = ProjectId::new("project.nordhamn-market-renovation").unwrap();
    let writer = RoleId::new("project-chief").unwrap();
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    cx.grant(construction_project_read_capability());
    cx.grant(construction_project_write_capability());
    cx.grant(construction_reference_publish_capability());

    let root = cx.factory().opaque(Arc::new(HashTable::new())).unwrap();
    let project_lib =
        ConstructionProjectLib::with_project_book(root, project.clone(), writer.clone()).unwrap();
    cx.load_lib(&project_lib).unwrap();
    let repository = project_lib
        .project_books()
        .expect("injected Table repository");

    register_powerproject_site(&mut cx, true).unwrap();
    register_dalux_site(&mut cx, true).unwrap();

    let plan = gantt_plan();
    let document = plan_to_doc(&mut cx, &plan).unwrap();
    let options = DocCodecOptions::new(cx.factory().nil().unwrap());
    let (mspdi, encoded) = MspdiCodec.encode(&mut cx, &document, &options).unwrap();
    assert!(encoded.is_lossless());
    let (decoded, decoded_report) = MspdiCodec.decode(&mut cx, &mspdi, &options).unwrap();
    assert!(decoded_report.is_lossless());
    assert_eq!(doc_to_plan(&mut cx, &decoded).unwrap(), plan);

    let powerproject_receipt =
        ModeledOleReceipt::new("receipt.powerproject.accepted-C", mspdi.clone());
    let (powerproject_doc, powerproject_report) =
        import_modeled_ole_receipt(&mut cx, &powerproject_receipt).unwrap();
    assert!(powerproject_report.is_lossless());
    let powerproject_plan = doc_to_plan(&mut cx, &powerproject_doc).unwrap();
    assert_eq!(
        powerproject_plan.task("task.frame-install").unwrap().id,
        "task.frame-install"
    );
    assert!(powerproject_doc.origin.iter().any(|source| {
        source.backend == POWERPROJECT_SITE_ID
            && source.external_id == "receipt.powerproject.accepted-C"
    }));

    let dalux = DaluxClient::modeled(
        ModeledDalux::with_json(
            "/projects/project.nordhamn-market-renovation/items",
            json!({
                "items": [{
                    "id": "field-item-001",
                    "title": "Modeled frame check",
                    "status": "open",
                    "location": "zone-a",
                    "note": "synthetic fixture",
                    "updatedAt": "2026-07-30T05:00:00Z"
                }]
            }),
        ),
        StaticDaluxCredentialProvider::new("modeled-fixture"),
    );
    let dalux_receipt =
        get_project_items_with_receipt(&mut cx, &dalux, "project.nordhamn-market-renovation")
            .unwrap();
    assert_eq!(dalux_receipt.items[0].external_ref.backend, DALUX_SITE_ID);
    assert_eq!(
        dalux_receipt.items[0].external_ref.external_id,
        "items/field-item-001"
    );

    let scenario = scenario::run(
        &mut cx,
        repository,
        vec![
            ExternalRef::new(
                "codec/mspdi",
                "evidence.schedule.accepted-C",
                Some("accepted-C".to_owned()),
                None,
            ),
            powerproject_doc.origin.last().unwrap().clone(),
        ],
        vec![
            dalux_receipt.items[0].external_ref.clone(),
            dalux_receipt.effect.ledger_ref.clone(),
        ],
    );
    let book = repository.read_book(&mut cx, 39).unwrap();

    let request = OfficePackRequest::new(
        PackCadence::Weekly,
        writer,
        39,
        date(30),
        "2026-07-30T06:00:00Z",
    )
    .with_baseline(BaselineId::new("schedule.rev-a").unwrap())
    .with_control(PackControl::mandatory(
        control("schedule.critical"),
        PackSection::CriticalSchedule,
    ))
    .with_control(PackControl::mandatory(
        control("safety.energization"),
        PackSection::SafetyLegalBlockers,
    ))
    .with_control(PackControl::optional(
        control("change.ventilation"),
        PackSection::RiskChangeEconomy,
    ))
    .changed_since(33);
    let pack = project_office_pack(&mut cx, &book, &request).unwrap();
    let doc_kinds = pack
        .documents(&mut cx)
        .unwrap()
        .into_iter()
        .map(|doc| doc.kind.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(doc_kinds, ["report", "sheet", "deck"]);
    assert_eq!(
        pack.doc.id.as_str(),
        "construction/project.nordhamn-market-renovation/project-chief/weekly"
    );
    assert_eq!(sheet_text(&pack, "B12"), "reported");
    assert!(
        sheet_text(&pack, "B13").contains("mandatory control(s) are not accepted"),
        "office pack must explain its non-green aggregate"
    );

    let network_denied = cx
        .require(&CapabilityName::new(NET_CONNECT_CAPABILITY))
        .is_err();
    let process_denied = cx
        .require(&CapabilityName::new(PROCESS_SPAWN_CAPABILITY))
        .is_err();
    let credentials_denied = cx
        .require(&CapabilityName::new(CREDENTIALS_CAPABILITY))
        .is_err();

    ModeledOutcome {
        summary: "(expr:map [project project.nordhamn-market-renovation] [table-backend table/hash] [facts 39] [as-of-sequence 39] [changed-controls [charter.people charter.place lesson.delivery-window outcome.people outcome.place]] [snapshot-boundaries 18] [conflicted-control intent.scope] [superseded-controls [handover.defect.controls measurement.area outcome.climate outcome.people safety.energization supplier.blue-arc]] [late-customer-decision true] [missing-prerequisite prerequisite.workplace-introduction] [supplier-expired-then-renewed true] [non-waivable-safety-blocked true] [bounded-exception-expired true] [critical-schedule-effect-days 5] [change-status partially-approved] [change-supplier-exposure 460000.00] [change-quoted-recovery 460000.00] [change-approved-value 275000.00] [change-unapproved-value 185000.00] [change-net-exposure 0.00] [double-count-prevented true] [handover-defect-corrected true] [initial-reference-admitted false] [final-reference-claims [claim.lesson claim.people claim.place]] [visibility-non-interference true] [schedule-task task.frame-install] [mspdi-round-trip lossless] [powerproject-mode modeled] [powerproject-external receipt.powerproject.accepted-C] [dalux-mode modeled] [dalux-external items/field-item-001] [effect-ledger-reference present] [ledger-reference journal/change-ventilation/seq-25] [office-doc-kinds [report sheet deck]] [network denied])".to_owned(),
        scenario,
        network_denied,
        process_denied,
        credentials_denied,
    }
}

fn gantt_plan() -> GanttPlan {
    GanttPlan::new(
        "plan.nordhamn-renovation",
        vec![
            Task::new(
                "task.design-release",
                "Design release",
                date(1),
                date(3),
                100,
            ),
            Task::new(
                "task.frame-install",
                "Frame installation",
                date(4),
                date(10),
                25,
            ),
            Task::new("task.handover", "Handover", date(11), date(12), 0),
        ],
        vec![
            TaskLink::new(
                "task.design-release",
                "task.frame-install",
                LinkKind::FinishStart,
                0,
            ),
            TaskLink::new(
                "task.frame-install",
                "task.handover",
                LinkKind::FinishStart,
                0,
            ),
        ],
    )
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn sheet_text(pack: &sim_lib_construction_office::OfficePack, cell: &str) -> String {
    match pack.sheet.cell(&CellRef::parse(cell).unwrap()) {
        CellValue::Text(value) => value,
        other => panic!("expected text at {cell}, got {other:?}"),
    }
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

fn recipe_root() -> PathBuf {
    let tests_root = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"))
        .expect("integration-test source directory");
    let repository_root = tests_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("test source lives at repo/crates/name/tests");
    repository_root.join("recipes/reference-project-control")
}

fn codec_context() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    let codec = sim_codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&codec).unwrap();
    cx
}

fn decode_lisp(cx: &mut Cx, source: String) -> Expr {
    decode_with_codec(
        cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(source),
        ReadPolicy::default(),
    )
    .unwrap()
}

fn encode_lisp(cx: &mut Cx, expression: &Expr) -> String {
    match encode_with_codec(
        cx,
        &Symbol::qualified("codec", "lisp"),
        expression,
        EncodeOptions::default(),
    )
    .unwrap()
    {
        Output::Text(text) => text,
        Output::Bytes(_) => panic!("Lisp codec returned bytes"),
    }
}
