//! Evaluation logic for construction reference-pack admission.

use std::collections::{BTreeMap, BTreeSet};

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    AccountableCloseout, ConstructionProjectError, ControlId, DisclosureClearance,
    DisclosureCondition, OutcomeControlReport, OutcomeVariance, ProjectBook, ProjectId,
    ReferenceAdmissionBlocker, ReferenceAdmissionReport, ReferenceApproval, ReferenceClaim,
    ReferenceClaimAdmission, ReferenceDecisionKind, ReferenceManifest, ReferenceManifestClaim,
    ReferencePackAdmission, Result, Visibility,
};

impl ReferencePackAdmission {
    /// Builds an empty candidate pack.
    #[must_use]
    pub fn new(
        project: ProjectId,
        as_of_seq: u64,
        as_of_date: Date,
        approving_authority: crate::RoleId,
    ) -> Self {
        Self {
            project,
            as_of_seq,
            as_of_date,
            approving_authority,
            claims: Vec::new(),
            clearances: Vec::new(),
            approvals: Vec::new(),
        }
    }

    /// Adds one proposed claim.
    #[must_use]
    pub fn with_claim(mut self, claim: ReferenceClaim) -> Self {
        self.claims.push(claim);
        self
    }

    /// Adds one claim clearance.
    #[must_use]
    pub fn with_clearance(mut self, clearance: DisclosureClearance) -> Self {
        self.clearances.push(clearance);
        self
    }

    /// Adds one accountable disclosure decision.
    #[must_use]
    pub fn with_approval(mut self, approval: ReferenceApproval) -> Self {
        self.approvals.push(approval);
        self
    }

    /// Evaluates current facts, outcomes, disclosure conditions, and authority.
    pub fn evaluate(
        &self,
        book: &ProjectBook,
        closeout: &AccountableCloseout,
        outcomes: &[OutcomeControlReport],
    ) -> Result<ReferenceAdmissionReport> {
        self.validate_inputs(book, closeout)?;
        let snapshot = book.snapshot_at(self.as_of_seq)?;
        let clearances = unique_by_claim(&self.clearances, |value| &value.claim, "clearance")?;
        let approvals = unique_by_claim(&self.approvals, |value| &value.claim, "approval")?;
        let mut claims = self.claims.iter().collect::<Vec<_>>();
        claims.sort_by(|left, right| left.id.cmp(&right.id));
        let mut seen = BTreeSet::new();
        let mut reports = Vec::new();
        let mut rows = Vec::new();
        for claim in claims {
            if !seen.insert(&claim.id) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "reference_claim",
                    id: claim.id.to_string(),
                });
            }
            let mut blockers = self.claim_blockers(
                claim,
                book,
                &snapshot,
                clearances.get(&claim.id).copied(),
                approvals.get(&claim.id).copied(),
                closeout,
                outcomes,
            )?;
            blockers.sort_by_key(blocker_sort_key);
            let admitted = blockers.is_empty();
            if admitted {
                let approval = approvals[&claim.id];
                let mut source_fact_sequences = claim.source_fact_sequences.clone();
                source_fact_sequences.sort_unstable();
                source_fact_sequences.dedup();
                rows.push(ReferenceManifestClaim {
                    claim_id: claim.id.clone(),
                    source_fact_sequences,
                    external_refs: sorted_references(&claim.external_refs),
                    as_of_date: self.as_of_date,
                    visibility: claim.visibility.clone(),
                    approving_decision: approval.decision.clone(),
                });
            }
            reports.push(ReferenceClaimAdmission {
                claim: claim.id.clone(),
                blockers,
                admitted,
            });
        }
        let manifest = reports
            .iter()
            .all(|report| report.admitted)
            .then(|| ReferenceManifest {
                project: self.project.clone(),
                as_of_date: self.as_of_date,
                closeout_decision: closeout.control().clone(),
                claims: rows,
            });
        Ok(ReferenceAdmissionReport {
            claims: reports,
            manifest,
        })
    }

    fn validate_inputs(&self, book: &ProjectBook, closeout: &AccountableCloseout) -> Result<()> {
        if book.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: book.project().clone(),
            });
        }
        if closeout.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: closeout.project().clone(),
            });
        }
        if self.as_of_seq == 0 || closeout.report_seq() > self.as_of_seq {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "reference_admission.as_of_seq",
                sequence: self.as_of_seq,
            });
        }
        if self.claims.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "reference_admission.claims",
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn claim_blockers(
        &self,
        claim: &ReferenceClaim,
        book: &ProjectBook,
        snapshot: &crate::ProjectSnapshot,
        clearance: Option<&DisclosureClearance>,
        approval: Option<&ReferenceApproval>,
        closeout: &AccountableCloseout,
        outcomes: &[OutcomeControlReport],
    ) -> Result<Vec<ReferenceAdmissionBlocker>> {
        validate_claim(claim, &self.project)?;
        let mut blockers = Vec::new();
        let mut restricted_source = false;
        for sequence in &claim.source_fact_sequences {
            let Some(fact) = book.fact(*sequence) else {
                blockers.push(ReferenceAdmissionBlocker::MissingSourceFact(*sequence));
                continue;
            };
            restricted_source |= matches!(fact.visibility, Visibility::Restricted(_));
            if snapshot.is_conflicted(&fact.subject) {
                blockers.push(ReferenceAdmissionBlocker::SourceFactConflicted(*sequence));
            } else {
                let current = snapshot.current_fact(&fact.subject).map(|fact| fact.seq);
                if current != Some(*sequence) {
                    blockers.push(ReferenceAdmissionBlocker::SourceFactNotCurrent {
                        sequence: *sequence,
                        current,
                    });
                }
            }
            if !fact.evidence_state.satisfies_required_evidence() {
                blockers.push(ReferenceAdmissionBlocker::SourceFactNotAccepted {
                    sequence: *sequence,
                    state: fact.evidence_state,
                });
            }
        }
        if let Some(clearance) = clearance {
            validate_condition(&clearance.consent)?;
            validate_condition(&clearance.confidentiality)?;
        }
        let consent = clearance.map(|value| &value.consent);
        if claim.consent_required {
            if consent.is_some_and(DisclosureCondition::is_withdrawn) {
                blockers.push(ReferenceAdmissionBlocker::ConsentWithdrawn);
            } else if !consent.is_some_and(DisclosureCondition::is_satisfied) {
                blockers.push(ReferenceAdmissionBlocker::ConsentUnsatisfied);
            }
        }
        let confidentiality = clearance.map(|value| &value.confidentiality);
        if (claim.confidentiality_required || restricted_source)
            && !confidentiality.is_some_and(DisclosureCondition::is_satisfied)
        {
            blockers.push(ReferenceAdmissionBlocker::ConfidentialityUnsatisfied);
        }
        if let Some(target) = &claim.outcome_target
            && !target_is_admissible(target, &self.project, self.as_of_date, outcomes)
        {
            blockers.push(ReferenceAdmissionBlocker::OutcomeShortfall(target.clone()));
        }
        match approval {
            None => blockers.push(ReferenceAdmissionBlocker::MissingApproval),
            Some(approval) => {
                if approval.outcome != ReferenceDecisionKind::Approve {
                    blockers.push(ReferenceAdmissionBlocker::ApprovalRejected);
                }
                if approval.decided_by != self.approving_authority {
                    blockers.push(ReferenceAdmissionBlocker::ApprovalAuthorityMismatch {
                        expected: self.approving_authority.clone(),
                        actual: approval.decided_by.clone(),
                    });
                }
                if approval.report_seq != self.as_of_seq
                    || approval.decision_seq <= self.as_of_seq
                    || approval.decision_seq <= closeout.decision_seq()
                {
                    blockers.push(ReferenceAdmissionBlocker::ApprovalSequenceMismatch);
                }
                if approval.evidence.is_empty() {
                    blockers.push(ReferenceAdmissionBlocker::MissingApprovalEvidence);
                } else {
                    validate_references(&approval.evidence, "reference_approval.evidence")?;
                }
            }
        }
        Ok(blockers)
    }
}

fn validate_claim(claim: &ReferenceClaim, project: &ProjectId) -> Result<()> {
    if &claim.project != project {
        return Err(ConstructionProjectError::ProjectMismatch {
            expected: project.clone(),
            actual: claim.project.clone(),
        });
    }
    if claim.statement.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField(
            "reference_claim.statement",
        ));
    }
    if claim.charter_fact_seq == 0
        || !claim
            .source_fact_sequences
            .contains(&claim.charter_fact_seq)
    {
        return Err(ConstructionProjectError::InvalidSequence {
            field: "reference_claim.charter_fact_seq",
            sequence: claim.charter_fact_seq,
        });
    }
    if claim.external_refs.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(
            "reference_claim.external_refs",
        ));
    }
    validate_references(&claim.external_refs, "reference_claim.external_ref")?;
    Ok(())
}

fn unique_by_claim<'a, T>(
    values: &'a [T],
    claim: impl Fn(&'a T) -> &'a ControlId,
    kind: &'static str,
) -> Result<BTreeMap<ControlId, &'a T>> {
    let mut result = BTreeMap::new();
    for value in values {
        let id = claim(value);
        if result.insert(id.clone(), value).is_some() {
            return Err(ConstructionProjectError::DuplicateId {
                kind,
                id: id.to_string(),
            });
        }
    }
    Ok(result)
}

fn target_is_admissible(
    target: &ControlId,
    project: &ProjectId,
    as_of: Date,
    outcomes: &[OutcomeControlReport],
) -> bool {
    let mut matching = outcomes
        .iter()
        .filter(|report| &report.project == project && report.as_of == as_of)
        .flat_map(|report| &report.targets)
        .filter(|report| &report.target == target);
    let Some(report) = matching.next() else {
        return false;
    };
    matching.next().is_none()
        && report.covered
        && report.reference_claim_admissible
        && report.variance == OutcomeVariance::OnTarget
}

fn validate_condition(condition: &DisclosureCondition) -> Result<()> {
    match condition {
        DisclosureCondition::NotRequired => Ok(()),
        DisclosureCondition::Satisfied(reference)
        | DisclosureCondition::Denied(reference)
        | DisclosureCondition::Withdrawn(reference) => validate_references(
            std::slice::from_ref(reference),
            "disclosure_condition.evidence",
        ),
    }
}

fn validate_references(references: &[ExternalRef], field: &'static str) -> Result<()> {
    for reference in references {
        if reference.backend.trim().is_empty()
            || reference.external_id.trim().is_empty()
            || reference
                .version
                .as_deref()
                .is_none_or(|version| version.trim().is_empty())
        {
            return Err(ConstructionProjectError::EmptyField(field));
        }
    }
    Ok(())
}

fn sorted_references(references: &[ExternalRef]) -> Vec<ExternalRef> {
    let mut references = references.to_vec();
    references.sort_by(|left, right| {
        left.backend
            .cmp(&right.backend)
            .then(left.external_id.cmp(&right.external_id))
            .then(left.version.cmp(&right.version))
            .then(left.web_url.cmp(&right.web_url))
    });
    references.dedup();
    references
}

fn blocker_sort_key(blocker: &ReferenceAdmissionBlocker) -> String {
    match blocker {
        ReferenceAdmissionBlocker::MissingSourceFact(sequence) => format!("1:{sequence}"),
        ReferenceAdmissionBlocker::SourceFactNotAccepted { sequence, .. } => {
            format!("2:{sequence}")
        }
        ReferenceAdmissionBlocker::SourceFactNotCurrent { sequence, .. } => {
            format!("3:{sequence}")
        }
        ReferenceAdmissionBlocker::SourceFactConflicted(sequence) => format!("4:{sequence}"),
        ReferenceAdmissionBlocker::ConsentUnsatisfied => "5".to_owned(),
        ReferenceAdmissionBlocker::ConsentWithdrawn => "6".to_owned(),
        ReferenceAdmissionBlocker::ConfidentialityUnsatisfied => "7".to_owned(),
        ReferenceAdmissionBlocker::OutcomeShortfall(target) => format!("8:{target}"),
        ReferenceAdmissionBlocker::MissingApproval => "9".to_owned(),
        ReferenceAdmissionBlocker::ApprovalRejected => "a".to_owned(),
        ReferenceAdmissionBlocker::ApprovalAuthorityMismatch { .. } => "b".to_owned(),
        ReferenceAdmissionBlocker::ApprovalSequenceMismatch => "c".to_owned(),
        ReferenceAdmissionBlocker::MissingApprovalEvidence => "d".to_owned(),
    }
}
