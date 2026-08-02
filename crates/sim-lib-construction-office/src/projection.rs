//! Deterministic construction snapshot projection into office values.

use std::collections::BTreeSet;

use sim_kernel::{Cx, Value};
use sim_lib_construction_project::{
    ControlId, EvidenceState, ProjectBook, ProjectFact, ProjectSnapshot,
};
use sim_lib_deck::{Deck, deck_to_doc};
use sim_lib_doc_core::{Doc, DocId, ExternalRef};
use sim_lib_doc_surface::suite_scene;
use sim_lib_sheet::{Sheet, sheet_to_doc};

use crate::{
    OFFICE_PACK_VISIBILITY_POLICY, OfficePackError, OfficePackRequest, PackControl, PackSection,
    visibility::visible_book,
};

mod render;

use render::{report_doc, status_deck, status_sheet};

/// Existing office-family values produced for one role and cadence.
#[derive(Clone, Debug)]
pub struct OfficePack {
    /// Prose report value.
    pub doc: Doc,
    /// Exact spreadsheet value.
    pub sheet: Sheet,
    /// Presentation deck value.
    pub deck: Deck,
    evidence: Vec<ExternalRef>,
}

impl OfficePack {
    /// Converts the Sheet and Deck values into ordinary office documents.
    pub fn documents(&self, cx: &mut Cx) -> Result<Vec<Doc>, OfficePackError> {
        let mut sheet = sheet_to_doc(cx, pack_doc_id(&self.doc.id, "sheet"), &self.sheet)
            .map_err(|error| OfficePackError::Office(error.to_string()))?;
        let mut deck = deck_to_doc(cx, pack_doc_id(&self.doc.id, "deck"), &self.deck)
            .map_err(|error| OfficePackError::Office(error.to_string()))?;
        sheet.origin.clone_from(&self.evidence);
        deck.origin.clone_from(&self.evidence);
        Ok(vec![self.doc.clone(), sheet, deck])
    }

    /// Presents this pack through the installed office suite surface.
    pub fn suite_scene(&self, cx: &mut Cx) -> Result<Value, OfficePackError> {
        let documents = self.documents(cx)?;
        suite_scene(cx, &[], &documents).map_err(|error| OfficePackError::Office(error.to_string()))
    }
}

/// Builds one deterministic, capability-filtered role-cadence pack.
pub fn project_office_pack(
    cx: &mut Cx,
    book: &ProjectBook,
    request: &OfficePackRequest,
) -> Result<OfficePack, OfficePackError> {
    validate_request(book, request)?;
    let visible = visible_book(cx, book, request.as_of_seq)?;
    let snapshot = visible.snapshot_at(request.as_of_seq)?;
    let controls = selected_controls(request);
    let changed = changed_controls(&visible, request)?;
    let mut rows = rows_for(&snapshot, &controls, &changed);
    rows.sort_by(|left, right| {
        left.section
            .cmp(&right.section)
            .then(left.control.cmp(&right.control))
    });
    let metadata = Metadata::new(book, request, &controls, &rows);
    let id = DocId::new(format!(
        "construction/{}/{}/{}",
        book.project(),
        request.role,
        request.cadence.as_str()
    ));
    let evidence = metadata.evidence.clone();
    let doc = report_doc(cx, id, &metadata, &rows)?;
    let sheet = status_sheet(&metadata, &rows);
    let deck = status_deck(&metadata, &rows);
    Ok(OfficePack {
        doc,
        sheet,
        deck,
        evidence,
    })
}

#[derive(Clone)]
struct StatusRow {
    section: PackSection,
    control: ControlId,
    value: String,
    status: EvidenceState,
    explanation: String,
    mandatory: bool,
    changed: bool,
    evidence: Vec<ExternalRef>,
}

struct Metadata {
    project: String,
    role: String,
    cadence: String,
    as_of_seq: u64,
    as_of_date: String,
    generated_at: String,
    baselines: Vec<String>,
    controls: Vec<String>,
    evidence: Vec<ExternalRef>,
    changed_since: Option<u64>,
    aggregate: EvidenceState,
    aggregate_explanation: String,
}

impl Metadata {
    fn new(
        book: &ProjectBook,
        request: &OfficePackRequest,
        controls: &[PackControl],
        rows: &[StatusRow],
    ) -> Self {
        let mut baselines = request
            .accepted_baselines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        baselines.sort();
        baselines.dedup();
        let controls = controls
            .iter()
            .map(|control| control.control.to_string())
            .collect();
        let mut evidence = rows
            .iter()
            .flat_map(|row| row.evidence.iter().cloned())
            .collect::<Vec<_>>();
        sort_evidence(&mut evidence);
        evidence.dedup();
        let (aggregate, aggregate_explanation) = aggregate(rows);
        Self {
            project: book.project().to_string(),
            role: request.role.to_string(),
            cadence: request.cadence.as_str().to_owned(),
            as_of_seq: request.as_of_seq,
            as_of_date: request.as_of_date.to_string(),
            generated_at: request.generated_at.clone(),
            baselines,
            controls,
            evidence,
            changed_since: request.changed_since_seq,
            aggregate,
            aggregate_explanation,
        }
    }

    fn fields(&self) -> Vec<(&'static str, String)> {
        vec![
            ("project-id", self.project.clone()),
            ("role", self.role.clone()),
            ("cadence", self.cadence.clone()),
            ("as-of-sequence", self.as_of_seq.to_string()),
            ("as-of-date", self.as_of_date.clone()),
            ("accepted-baseline-ids", self.baselines.join(", ")),
            ("source-control-ids", self.controls.join(", ")),
            ("evidence-refs", evidence_text(&self.evidence)),
            ("generated-at", self.generated_at.clone()),
            (
                "visibility-policy",
                OFFICE_PACK_VISIBILITY_POLICY.to_owned(),
            ),
            (
                "changed-since-sequence",
                self.changed_since
                    .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            ),
            ("status-value", status_label(self.aggregate).to_owned()),
            ("status-explanation", self.aggregate_explanation.clone()),
        ]
    }
}

fn validate_request(
    book: &ProjectBook,
    request: &OfficePackRequest,
) -> Result<(), OfficePackError> {
    if request.generated_at.trim().is_empty() {
        return Err(OfficePackError::InvalidRequest(
            "generated-at marker is empty".to_owned(),
        ));
    }
    if request.as_of_seq > book.last_sequence().unwrap_or(0) {
        return Err(OfficePackError::InvalidRequest(format!(
            "as-of sequence {} exceeds project sequence {}",
            request.as_of_seq,
            book.last_sequence().unwrap_or(0)
        )));
    }
    if request
        .changed_since_seq
        .is_some_and(|sequence| sequence > request.as_of_seq)
    {
        return Err(OfficePackError::InvalidRequest(
            "changed-since sequence exceeds as-of sequence".to_owned(),
        ));
    }
    Ok(())
}

fn selected_controls(request: &OfficePackRequest) -> Vec<PackControl> {
    let mut controls = request
        .controls
        .iter()
        .filter(|control| request.cadence.includes(control.section))
        .cloned()
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| left.control.cmp(&right.control));
    controls.dedup_by(|left, right| left.control == right.control);
    controls
}

fn changed_controls(
    book: &ProjectBook,
    request: &OfficePackRequest,
) -> Result<BTreeSet<ControlId>, OfficePackError> {
    let Some(from) = request.changed_since_seq else {
        return Ok(BTreeSet::new());
    };
    let delta = book.delta(from, request.as_of_seq)?;
    Ok(delta
        .added
        .into_iter()
        .chain(delta.superseded)
        .chain(delta.conflicted)
        .collect())
}

fn rows_for(
    snapshot: &ProjectSnapshot,
    controls: &[PackControl],
    changed: &BTreeSet<ControlId>,
) -> Vec<StatusRow> {
    controls
        .iter()
        .map(|selection| {
            let (value, status, explanation, evidence) = status_for(snapshot, &selection.control);
            let section = if matches!(
                selection.section,
                PackSection::Decisions | PackSection::SafetyLegalBlockers
            ) || status == EvidenceState::Accepted
            {
                selection.section
            } else {
                PackSection::EvidenceExceptions
            };
            StatusRow {
                section,
                control: selection.control.clone(),
                value,
                status,
                explanation,
                mandatory: selection.mandatory,
                changed: changed.contains(&selection.control),
                evidence,
            }
        })
        .collect()
}

fn status_for(
    snapshot: &ProjectSnapshot,
    control: &ControlId,
) -> (String, EvidenceState, String, Vec<ExternalRef>) {
    if let Some(fact) = snapshot.current.get(control) {
        return fact_status(fact, "current");
    }
    if let Some(facts) = snapshot.conflicted.get(control) {
        let sequences = facts
            .iter()
            .map(|fact| fact.seq.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return (
            "conflicted".to_owned(),
            EvidenceState::Conflicted,
            format!("visible facts at sequences {sequences} require accountable resolution"),
            evidence_for(facts),
        );
    }
    if let Some(facts) = snapshot.rejected.get(control)
        && let Some(fact) = facts.last()
    {
        return fact_status(fact, "rejected");
    }
    (
        "unknown".to_owned(),
        EvidenceState::Missing,
        format!(
            "no visible current fact for {control} as of sequence {}",
            snapshot.through_seq
        ),
        Vec::new(),
    )
}

fn fact_status(
    fact: &ProjectFact,
    disposition: &str,
) -> (String, EvidenceState, String, Vec<ExternalRef>) {
    (
        format!("{:?}", fact.body),
        fact.evidence_state,
        format!(
            "{disposition} {} fact at sequence {} by {} effective {}",
            status_label(fact.evidence_state),
            fact.seq,
            fact.actor_role,
            fact.effective_on
        ),
        fact.evidence.clone(),
    )
}

fn evidence_for(facts: &[ProjectFact]) -> Vec<ExternalRef> {
    let mut evidence = facts
        .iter()
        .flat_map(|fact| fact.evidence.iter().cloned())
        .collect::<Vec<_>>();
    sort_evidence(&mut evidence);
    evidence.dedup();
    evidence
}

fn aggregate(rows: &[StatusRow]) -> (EvidenceState, String) {
    let mandatory = rows.iter().filter(|row| row.mandatory).collect::<Vec<_>>();
    if mandatory.is_empty() {
        return (
            EvidenceState::Missing,
            "no mandatory source controls were selected for this horizon".to_owned(),
        );
    }
    let blockers = mandatory
        .iter()
        .filter(|row| row.status != EvidenceState::Accepted)
        .copied()
        .collect::<Vec<_>>();
    if blockers.is_empty() {
        return (
            EvidenceState::Accepted,
            format!("all {} mandatory controls are accepted", mandatory.len()),
        );
    }
    let worst = blockers
        .iter()
        .min_by_key(|row| status_priority(row.status))
        .expect("blockers is non-empty");
    (
        worst.status,
        format!(
            "{} mandatory control(s) are not accepted; first is {}: {}",
            blockers.len(),
            worst.control,
            worst.explanation
        ),
    )
}

fn status_label(status: EvidenceState) -> &'static str {
    match status {
        EvidenceState::Missing => "missing",
        EvidenceState::Reported => "reported",
        EvidenceState::Evidenced => "evidenced",
        EvidenceState::Accepted => "accepted",
        EvidenceState::Rejected => "rejected",
        EvidenceState::Expired => "expired",
        EvidenceState::Conflicted => "conflicted",
    }
}

fn status_priority(status: EvidenceState) -> u8 {
    match status {
        EvidenceState::Rejected => 0,
        EvidenceState::Conflicted => 1,
        EvidenceState::Expired => 2,
        EvidenceState::Missing => 3,
        EvidenceState::Reported => 4,
        EvidenceState::Evidenced => 5,
        EvidenceState::Accepted => 6,
    }
}

fn evidence_text(evidence: &[ExternalRef]) -> String {
    evidence
        .iter()
        .map(|reference| {
            format!(
                "{}:{}@{}",
                reference.backend,
                reference.external_id,
                reference.version.as_deref().unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn sort_evidence(evidence: &mut [ExternalRef]) {
    evidence.sort_by(|left, right| {
        left.backend
            .cmp(&right.backend)
            .then(left.external_id.cmp(&right.external_id))
            .then(left.version.cmp(&right.version))
            .then(left.web_url.cmp(&right.web_url))
    });
}

fn pack_doc_id(report: &DocId, kind: &str) -> DocId {
    DocId::new(format!("{}/{kind}", report.as_str()))
}
