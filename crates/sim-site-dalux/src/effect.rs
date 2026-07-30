//! Ledgered `site/dalux` project-item operations.

use std::cell::RefCell;

use serde_json::json;
use sim_kernel::{
    CapabilityName, Cx, Datum, DatumStore, Error, Ref, Symbol,
    effect::{Effect, effect_abort_op_key, effect_resume_op_key, resolve_effect},
};
use sim_lib_doc_core::{CREDENTIALS_CAPABILITY, Doc, ExternalRef, NET_CONNECT_CAPABILITY};
use sim_lib_doc_site::site_symbol;

use crate::client::{DaluxClient, DaluxClientMode, DaluxCredentialProvider};
use crate::model::{item_path, item_references, items_doc, patch_external_ref, project_items_path};
use crate::{DALUX_SITE_ID, DaluxError, DaluxItemReference};

const EFFECT_LEDGER_BACKEND: &str = "effect/ledger";

/// Kernel effect identity and stable effect-ledger reference for a Dalux operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaluxEffectReference {
    /// Per-run kernel effect identity.
    pub effect: Ref,
    /// Stable replay-key reference suitable for a project fact.
    pub ledger_ref: ExternalRef,
}

/// Receipt for a project-item read through `site/dalux`.
#[derive(Clone, Debug, PartialEq)]
pub struct DaluxItemReadReceipt {
    /// Local office projection; vendor payloads remain here rather than in the fact book.
    pub doc: Doc,
    /// URL-free item correlations and bounded vendor state.
    pub items: Vec<DaluxItemReference>,
    /// Recorded kernel effect reference.
    pub effect: DaluxEffectReference,
}

/// Receipt for an allowed note-only patch through `site/dalux`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaluxNotePatchReceipt {
    /// URL-free stable reference to the patched item.
    pub item: ExternalRef,
    /// Recorded kernel effect reference.
    pub effect: DaluxEffectReference,
}

/// Reads Dalux project items and projects them into a local office document.
pub fn get_project_items<C: DaluxCredentialProvider>(
    cx: &mut Cx,
    client: &DaluxClient<C>,
    project_id: &str,
) -> Result<Doc, DaluxError> {
    Ok(get_project_items_with_receipt(cx, client, project_id)?.doc)
}

/// Reads Dalux project items through a ledgered `site/dalux` effect.
pub fn get_project_items_with_receipt<C: DaluxCredentialProvider>(
    cx: &mut Cx,
    client: &DaluxClient<C>,
    project_id: &str,
) -> Result<DaluxItemReadReceipt, DaluxError> {
    let path = project_items_path(project_id)?;
    let input = content_ref(cx, Datum::String(path.clone()))?;
    let mut effect = dalux_effect(
        client,
        "read-project-items",
        input,
        Symbol::qualified("construction", "DaluxItemList"),
    );
    let effect_reference = effect_reference(&mut effect)?;
    let produced = RefCell::new(None);
    let failure = RefCell::new(None);
    let resolved = resolve_effect(cx, effect, |cx, _effect| {
        let body = capture(&failure, client.get_json(cx, &path))?;
        let doc = capture(&failure, items_doc(cx, project_id, &body))?;
        let items = capture(&failure, item_references(&body))?;
        let result = content_ref(
            cx,
            Datum::String(format!("{DALUX_SITE_ID}/projects/{project_id}/items")),
        )?;
        produced.replace(Some((doc, items)));
        Ok(result)
    });
    if let Err(error) = resolved {
        return Err(failure.into_inner().unwrap_or_else(|| error.into()));
    }
    let (doc, items) = produced.into_inner().ok_or_else(|| {
        DaluxError::Effect("Dalux read replay did not carry a projected document".to_owned())
    })?;
    Ok(DaluxItemReadReceipt {
        doc,
        items,
        effect: effect_reference,
    })
}

/// Patches the note field for one Dalux item and returns an external reference.
pub fn patch_item_note<C: DaluxCredentialProvider>(
    cx: &mut Cx,
    client: &DaluxClient<C>,
    item_id: &str,
    note: &str,
) -> Result<ExternalRef, DaluxError> {
    Ok(patch_item_note_with_receipt(cx, client, item_id, note)?.item)
}

/// Patches only the note field through a ledgered `site/dalux` effect.
pub fn patch_item_note_with_receipt<C: DaluxCredentialProvider>(
    cx: &mut Cx,
    client: &DaluxClient<C>,
    item_id: &str,
    note: &str,
) -> Result<DaluxNotePatchReceipt, DaluxError> {
    let path = item_path(item_id)?;
    let body = json!({ "note": note });
    let input = content_ref(
        cx,
        Datum::Node {
            tag: Symbol::qualified("construction", "DaluxNotePatch"),
            fields: vec![
                (Symbol::new("path"), Datum::String(path.clone())),
                (Symbol::new("note"), Datum::String(note.to_owned())),
            ],
        },
    )?;
    let mut effect = dalux_effect(
        client,
        "patch-item-note",
        input,
        Symbol::qualified("construction", "DaluxItemReference"),
    );
    let effect_reference = effect_reference(&mut effect)?;
    let produced = RefCell::new(None);
    let failure = RefCell::new(None);
    let resolved = resolve_effect(cx, effect, |cx, _effect| {
        let response = capture(&failure, client.patch_json(cx, &path, &body))?;
        let mut item = capture(&failure, patch_external_ref(item_id, &response))?;
        item.web_url = None;
        let result = content_ref(
            cx,
            Datum::String(format!(
                "{}:{}",
                item.external_id,
                item.version.as_deref().unwrap_or("")
            )),
        )?;
        produced.replace(Some(item));
        Ok(result)
    });
    if let Err(error) = resolved {
        return Err(failure.into_inner().unwrap_or_else(|| error.into()));
    }
    let item = produced.into_inner().ok_or_else(|| {
        DaluxError::Effect("Dalux note replay did not carry an item reference".to_owned())
    })?;
    Ok(DaluxNotePatchReceipt {
        item,
        effect: effect_reference,
    })
}

fn capture<T>(
    failure: &RefCell<Option<DaluxError>>,
    result: Result<T, DaluxError>,
) -> sim_kernel::Result<T> {
    result.map_err(|error| {
        failure.replace(Some(error.clone()));
        Error::Eval(error.to_string())
    })
}

fn dalux_effect<C>(
    client: &DaluxClient<C>,
    operation: &str,
    input: Ref,
    result_shape: Symbol,
) -> Effect {
    let requirements = match &client.mode {
        DaluxClientMode::Modeled(_) => Vec::new(),
        DaluxClientMode::Live => vec![
            CapabilityName::new(NET_CONNECT_CAPABILITY),
            CapabilityName::new(CREDENTIALS_CAPABILITY),
        ],
    };
    Effect::new(
        Symbol::qualified("construction/dalux", operation),
        Ref::Symbol(site_symbol(DALUX_SITE_ID)),
        input,
        Ref::Symbol(result_shape),
        effect_resume_op_key(),
        effect_abort_op_key(),
    )
    .with_requirements(requirements)
}

fn effect_reference(effect: &mut Effect) -> Result<DaluxEffectReference, DaluxError> {
    let replay_key = effect.ensure_replay_key(None)?;
    let external_id = format!(
        "{}/{}",
        replay_key.algorithm.as_qualified_str(),
        hex(&replay_key.bytes)
    );
    Ok(DaluxEffectReference {
        effect: effect.id.clone(),
        ledger_ref: ExternalRef::new(EFFECT_LEDGER_BACKEND, external_id, None, None),
    })
}

fn content_ref(cx: &mut Cx, datum: Datum) -> sim_kernel::Result<Ref> {
    Ok(Ref::Content(cx.datum_store_mut().intern(datum)?))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
