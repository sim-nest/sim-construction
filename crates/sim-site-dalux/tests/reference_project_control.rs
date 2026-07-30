use std::{fs, path::Path, sync::Arc};

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
    BaselineId, ConstructionProjectLib, ControlId, EvidenceState, ProjectFact, ProjectId, RoleId,
    Visibility, construction_project_read_capability, construction_project_write_capability,
    construction_reference_publish_capability,
};
use sim_lib_doc_core::{
    CREDENTIALS_CAPABILITY, DocCodec, DocCodecOptions, ExternalRef, NET_CONNECT_CAPABILITY,
    PROCESS_SPAWN_CAPABILITY,
};
use sim_lib_gantt::{GanttPlan, LinkKind, Task, TaskLink};
use sim_site_dalux::{
    DALUX_SITE_ID, DaluxClient, ModeledDalux, StaticDaluxCredentialProvider,
    get_project_items_with_receipt, register_dalux_site,
};
use sim_site_powerproject::{
    ModeledOleReceipt, POWERPROJECT_SITE_ID, import_modeled_ole_receipt, register_powerproject_site,
};
use sim_table_hash::HashTable;
use time::{Date, Month};

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
        "fact.schedule.task.frame-install",
        "evidence.schedule.rev-a",
        "task.frame-install",
        "receipt.powerproject.rev-a",
        "items/field-item-001",
        "office.pack.weekly",
        "journal/change-001/seq-1",
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
        for denied in [
            "http://",
            "https://",
            "Authorization:",
            "Bearer ",
            "api-key",
            "access-token",
        ] {
            assert!(!text.contains(denied), "{source} contains {denied:?}");
        }
    }
}

struct ModeledOutcome {
    summary: String,
    network_denied: bool,
    process_denied: bool,
    credentials_denied: bool,
}

fn run_modeled_reference_project() -> ModeledOutcome {
    let project = ProjectId::new("reference-project-control").unwrap();
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

    let powerproject_receipt = ModeledOleReceipt::new("receipt.powerproject.rev-a", mspdi.clone());
    let (powerproject_doc, powerproject_report) =
        import_modeled_ole_receipt(&mut cx, &powerproject_receipt).unwrap();
    assert!(powerproject_report.is_lossless());
    let powerproject_plan = doc_to_plan(&mut cx, &powerproject_doc).unwrap();
    assert_eq!(
        powerproject_plan.task("task.frame-install").unwrap().id,
        "task.frame-install"
    );
    assert!(powerproject_doc.origin.iter().any(|source| {
        source.backend == POWERPROJECT_SITE_ID && source.external_id == "receipt.powerproject.rev-a"
    }));

    let dalux = DaluxClient::modeled(
        ModeledDalux::with_json(
            "/projects/reference-project-control/items",
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
        get_project_items_with_receipt(&mut cx, &dalux, "reference-project-control").unwrap();
    assert_eq!(dalux_receipt.items[0].external_ref.backend, DALUX_SITE_ID);
    assert_eq!(
        dalux_receipt.items[0].external_ref.external_id,
        "items/field-item-001"
    );

    repository
        .append_fact(
            &mut cx,
            fact(
                1,
                &project,
                &writer,
                "fact.schedule.task.frame-install",
                "accepted Gantt task join task.frame-install",
                EvidenceState::Accepted,
                vec![
                    ExternalRef::new(
                        "codec/mspdi",
                        "evidence.schedule.rev-a",
                        Some("rev-a".to_owned()),
                        None,
                    ),
                    powerproject_doc.origin.last().unwrap().clone(),
                ],
            ),
        )
        .unwrap();
    repository
        .append_fact(
            &mut cx,
            fact(
                2,
                &project,
                &writer,
                "field.item.001",
                "modeled field item blocks frame installation",
                EvidenceState::Reported,
                vec![
                    dalux_receipt.items[0].external_ref.clone(),
                    dalux_receipt.effect.ledger_ref.clone(),
                ],
            ),
        )
        .unwrap();
    repository
        .append_fact(
            &mut cx,
            fact(
                3,
                &project,
                &writer,
                "change.001",
                "accepted reference-only change exposure",
                EvidenceState::Accepted,
                vec![ExternalRef::new(
                    "ledger/books",
                    "journal/change-001/seq-1",
                    Some("seq-1".to_owned()),
                    None,
                )],
            ),
        )
        .unwrap();

    let historical = repository.read_snapshot(&mut cx, 2).unwrap();
    assert_eq!(historical.through_seq, 2);
    assert!(!historical.current.contains_key(&control("change.001")));
    let book = repository.read_book(&mut cx, 3).unwrap();
    let delta = book.delta(1, 3).unwrap();
    assert_eq!(
        delta.added,
        vec![control("change.001"), control("field.item.001")]
    );

    let request = OfficePackRequest::new(
        PackCadence::Weekly,
        writer,
        3,
        date(30),
        "2026-07-30T06:00:00Z",
    )
    .with_baseline(BaselineId::new("schedule.rev-a").unwrap())
    .with_control(PackControl::mandatory(
        control("fact.schedule.task.frame-install"),
        PackSection::CriticalSchedule,
    ))
    .with_control(PackControl::mandatory(
        control("field.item.001"),
        PackSection::SafetyLegalBlockers,
    ))
    .with_control(PackControl::optional(
        control("change.001"),
        PackSection::RiskChangeEconomy,
    ))
    .changed_since(1);
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
        "construction/reference-project-control/project-chief/weekly"
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
        summary: "(expr:map [project reference-project-control] [table-backend table/hash] [facts 3] [as-of-sequence 2] [changed-controls [change.001 field.item.001]] [schedule-task task.frame-install] [mspdi-round-trip lossless] [powerproject-mode modeled] [powerproject-external receipt.powerproject.rev-a] [dalux-mode modeled] [dalux-external items/field-item-001] [effect-ledger-reference present] [ledger-reference journal/change-001/seq-1] [office-doc-kinds [report sheet deck]] [network denied])".to_owned(),
        network_denied,
        process_denied,
        credentials_denied,
    }
}

fn gantt_plan() -> GanttPlan {
    GanttPlan::new(
        "plan.reference-control",
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

fn fact(
    sequence: u64,
    project: &ProjectId,
    writer: &RoleId,
    subject: &str,
    body: &str,
    state: EvidenceState,
    evidence: Vec<ExternalRef>,
) -> ProjectFact {
    let mut fact = ProjectFact::new(
        sequence,
        project.clone(),
        control(subject),
        Symbol::qualified("reference-project-control", "fact"),
        date(30),
        writer.clone(),
        Expr::String(body.to_owned()),
    )
    .with_evidence_state(state)
    .with_visibility(Visibility::Project);
    for reference in evidence {
        fact = fact.with_evidence(reference);
    }
    fact
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

fn recipe_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at repo/crates/name")
        .join("recipes/reference-project-control")
        .leak()
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
