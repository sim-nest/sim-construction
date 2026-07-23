use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;
use sim_kernel::{
    CapabilityName, Cx, DefaultFactory, ExportKind, ExportState, NoopEvalPolicy, RuntimeId,
};
use sim_lib_doc_core::{CREDENTIALS_CAPABILITY, NET_CONNECT_CAPABILITY};
use sim_lib_doc_site::site_symbol;

use crate::*;

// conformance: office site workflows model site placement and document exchange.

fn test_context() -> Cx {
    Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory))
}

fn text_at(sheet: &sim_lib_sheet::Sheet, cell: &str) -> String {
    match sheet.cell(&sim_lib_sheet::CellRef::parse(cell).unwrap()) {
        sim_lib_sheet::CellValue::Text(value) => value,
        other => panic!("expected text at {cell}, got {other:?}"),
    }
}

#[test]
fn live_site_carries_dalux_capabilities() {
    let site = live_dalux_site();
    let caps: Vec<_> = site
        .required_caps
        .iter()
        .map(|capability| capability.as_str().to_owned())
        .collect();

    assert_eq!(site.site_id, DALUX_SITE_ID);
    assert!(!site.default_modeled);
    assert_eq!(caps, vec![NET_CONNECT_CAPABILITY, CREDENTIALS_CAPABILITY]);
}

#[test]
fn site_registers_as_export_site() {
    let mut cx = test_context();

    let record = register_dalux_site(&mut cx, true).unwrap();

    assert_eq!(record.kind, ExportKind::named(ExportKind::SITE));
    assert_eq!(record.symbol, site_symbol(DALUX_SITE_ID));
    assert!(matches!(
        record.state,
        ExportState::Resolved {
            id: RuntimeId::Site(_)
        }
    ));
    assert!(
        cx.registry()
            .site_by_symbol(&site_symbol(DALUX_SITE_ID))
            .is_some()
    );
}

#[test]
fn recipes_are_embedded() {
    let cards = sim_cookbook::recipes_from_embedded(RECIPES).unwrap();

    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("dalux-modeled-items"))
    );
}

#[test]
fn company_api_key_provider_is_rejected() {
    let provider = StaticDaluxCredentialProvider::company_api_key("old-key");

    assert!(matches!(
        provider.access_token(),
        Err(DaluxError::CompanyApiKeyUnsupported)
    ));
}

#[test]
fn live_gate_requires_capabilities_and_construction_enable_value() {
    let mut cx = test_context();

    let denied = client::require_live_gate_for_config(&cx, Some("1")).unwrap_err();
    assert!(denied.to_string().contains(NET_CONNECT_CAPABILITY));

    cx.grant(CapabilityName::new(NET_CONNECT_CAPABILITY));
    let denied = client::require_live_gate_for_config(&cx, Some("1")).unwrap_err();
    assert!(denied.to_string().contains(CREDENTIALS_CAPABILITY));

    cx.grant(CapabilityName::new(CREDENTIALS_CAPABILITY));
    let denied = client::require_live_gate_for_config(&cx, None).unwrap_err();
    assert!(denied.to_string().contains(DALUX_LIVE_ENV));

    let denied = client::require_live_gate_for_config(&cx, Some("0")).unwrap_err();
    assert!(denied.to_string().contains(DALUX_LIVE_ENV));

    client::require_live_gate_for_config(&cx, Some("1")).unwrap();
    assert_eq!(DALUX_LIVE_ENV, "SIM_CONSTRUCTION_LIVE_DALUX");
}

#[test]
fn modeled_project_items_become_dalux_doc() {
    let mut cx = test_context();
    let client = DaluxClient::modeled(
        ModeledDalux::with_json(
            "/projects/synthetic-project-1/items",
            json!({
                "items": [
                    {
                        "id": "item-1",
                        "title": "Door review",
                        "status": "open",
                        "location": "Level 2",
                        "note": "Check frame",
                        "updatedAt": "2026-07-13T10:00:00Z",
                        "webUrl": "https://example.com/dalux/items/item-1"
                    }
                ]
            }),
        ),
        StaticDaluxCredentialProvider::new("token-1"),
    );

    let doc = get_project_items(&mut cx, &client, "synthetic-project-1").unwrap();
    let sheet = sim_lib_sheet::doc_to_sheet(&mut cx, &doc).unwrap();

    assert_eq!(
        doc.id.as_str(),
        "site/dalux/projects/synthetic-project-1/items"
    );
    assert_eq!(text_at(&sheet, "A2"), "item-1");
    assert_eq!(text_at(&sheet, "B2"), "Door review");
}

#[test]
fn modeled_patch_sends_only_note_field() {
    let mut cx = test_context();
    let client = DaluxClient::modeled(
        ModeledDalux::new().with_patch(
            "/items/item-1",
            json!({ "note": "Reviewed in SIM" }),
            ModeledResponse::ok(json!({
                "id": "item-1",
                "updatedAt": "2026-07-13T11:00:00Z",
                "webUrl": "https://example.com/dalux/items/item-1"
            })),
        ),
        StaticDaluxCredentialProvider::new("token-1"),
    );

    let external = patch_item_note(&mut cx, &client, "item-1", "Reviewed in SIM").unwrap();

    assert_eq!(external.backend, DALUX_SITE_ID);
    assert_eq!(external.external_id, "items/item-1");
    assert_eq!(external.version.as_deref(), Some("2026-07-13T11:00:00Z"));
}

#[test]
fn errors_redact_tokens_and_long_project_names() {
    let mut cx = test_context();
    let token = "redacted-value";
    let long_name = format!("project-{}", "x".repeat(140));
    let client = DaluxClient::modeled(
        ModeledDalux::with_status(
            "/projects/project-1/items",
            403,
            json!({
                "token": token,
                "projectName": long_name,
                "message": "denied"
            }),
        ),
        StaticDaluxCredentialProvider::new(token),
    );

    let error = get_project_items(&mut cx, &client, "project-1")
        .unwrap_err()
        .to_string();

    assert!(!error.contains(token));
    assert!(!error.contains(&long_name));
    assert!(error.contains("[redacted-token]"));
    assert!(error.contains("[redacted-long-field]"));
}

#[test]
fn public_fixtures_do_not_carry_live_dalux_inputs() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate lives under repo/crates/name");
    let roots = [manifest_dir.join("recipes"), repo_root.join("openapi")];
    let mut files = Vec::new();
    for root in roots {
        collect_fixture_files(&root, &mut files);
    }

    let mut failures = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).unwrap_or_else(|err| {
            panic!("read fixture {}: {err}", file.display());
        });
        let rel = file.strip_prefix(repo_root).unwrap_or(&file).display();
        let live_host = ["api", "dalux", "com"].join(".");
        for denied in ["Authorization:", "Authorization\"", "Bearer ", "eyJ"] {
            if content.contains(denied) {
                failures.push(format!("{rel} contains denied pattern {denied:?}"));
            }
        }
        if content.contains(&live_host) {
            failures.push(format!("{rel} contains denied live Dalux host"));
        }
        for line in content.lines() {
            if line.contains("[project ") && !line.contains("synthetic") {
                failures.push(format!(
                    "{rel} contains non-synthetic project line {line:?}"
                ));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn collect_fixture_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.is_dir() {
        return;
    }
    for entry in fs::read_dir(root).unwrap_or_else(|err| {
        panic!("read fixture dir {}: {err}", root.display());
    }) {
        let entry = entry.unwrap_or_else(|err| panic!("read fixture entry: {err}"));
        let path = entry.path();
        if path.is_dir() {
            collect_fixture_files(&path, files);
        } else if is_fixture_file(&path) {
            files.push(path);
        }
    }
}

fn is_fixture_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "md" | "siml" | "toml")
    )
}
