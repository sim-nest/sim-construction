//! Project-scoped supplier qualification and subcontract-chain readiness.

use crate::{
    CONSTRUCTION_SUPPLIER_READ_CAPABILITY, ConstructionProjectError, ControlId, EvidenceState,
    EvidenceValidity, ObligationPolicy, OrganizationId, ProjectId, ProjectObligation, Result,
    RoleId,
};
use sim_lib_doc_core::ExternalRef;
use std::collections::{BTreeMap, BTreeSet};
use time::Date;

/// Project-scoped supplier or subcontractor reference.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierReference {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable organization reference for the supplier.
    pub supplier: OrganizationId,
    /// Role the organization is expected to perform.
    pub role: RoleId,
    /// Parent supplier in the project subcontract chain, when any.
    pub parent_supplier: Option<OrganizationId>,
    /// Subcontract depth from the directly awarded supplier.
    pub subcontract_depth: u8,
    /// Maximum accepted subcontract depth for this supplier path.
    pub max_accepted_depth: u8,
    /// Role authorized to decide this supplier qualification.
    pub decision_authority: RoleId,
    /// Supplier reference validity window.
    pub validity: EvidenceValidity,
    /// Reference-only supplier evidence.
    evidence: Vec<ExternalRef>,
}
impl SupplierReference {
    /// Builds a project-scoped supplier reference.
    #[must_use]
    pub fn new(
        project: ProjectId,
        supplier: OrganizationId,
        role: RoleId,
        decision_authority: RoleId,
    ) -> Self {
        Self {
            project,
            supplier,
            role,
            parent_supplier: None,
            subcontract_depth: 0,
            max_accepted_depth: 0,
            decision_authority,
            validity: EvidenceValidity::unbounded(),
            evidence: Vec::new(),
        }
    }

    /// Names the parent supplier and accepted depth bounds.
    #[must_use]
    pub fn under_parent(
        mut self,
        parent_supplier: OrganizationId,
        subcontract_depth: u8,
        max_accepted_depth: u8,
    ) -> Self {
        self.parent_supplier = Some(parent_supplier);
        self.subcontract_depth = subcontract_depth;
        self.max_accepted_depth = max_accepted_depth;
        self
    }

    /// Sets the supplier reference validity window.
    #[must_use]
    pub fn with_validity(mut self, validity: EvidenceValidity) -> Self {
        self.validity = validity;
        self
    }

    /// Adds reference-only supplier evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Returns supplier evidence only when the caller has the supplier-read capability.
    pub fn evidence<'a>(&'a self, granted_capabilities: &[String]) -> Result<&'a [ExternalRef]> {
        require_supplier_read(granted_capabilities)?;
        Ok(&self.evidence)
    }

    fn validate(&self) -> Result<()> {
        if self.subcontract_depth > self.max_accepted_depth {
            return Err(ConstructionProjectError::SupplierDepthExceeded {
                supplier: self.supplier.clone(),
                depth: self.subcontract_depth,
                max_accepted_depth: self.max_accepted_depth,
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "supplier.evidence",
            ));
        }
        Ok(())
    }
}
/// Supplier qualification evidence lane.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum SupplierQualificationArea {
    /// Economic standing and financial capacity.
    EconomicStanding,
    /// Responsible-business and human-rights evidence.
    ResponsibleBusinessHumanRights,
    /// Collective arrangements required for the work.
    CollectiveArrangements,
    /// Competence and licenses.
    CompetenceLicenses,
    /// Insurance.
    Insurance,
    /// Safety training.
    SafetyTraining,
    /// Workplace introduction.
    WorkplaceIntroduction,
    /// Risk assessment.
    RiskAssessment,
    /// Work preparation.
    WorkPreparation,
    /// Equipment.
    Equipment,
    /// Materials.
    Materials,
    /// Quality and environment.
    QualityEnvironment,
    /// Staffing.
    Staffing,
    /// Logistics.
    Logistics,
    /// Meeting participation.
    MeetingParticipation,
}
/// Qualification requirement using the shared open obligation model.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualificationRequirement {
    /// Requirement area.
    pub area: SupplierQualificationArea,
    /// Shared project obligation.
    pub obligation: ProjectObligation,
}
impl QualificationRequirement {
    /// Builds a qualification requirement from a project obligation.
    #[must_use]
    pub fn new(area: SupplierQualificationArea, obligation: ProjectObligation) -> Self {
        Self { area, obligation }
    }
}
/// Restricted evidence decision for one supplier requirement.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualificationEvidence {
    /// Supplier organization.
    pub supplier: OrganizationId,
    /// Requirement control id.
    pub requirement: ControlId,
    /// Evidence state.
    pub state: EvidenceState,
    /// Evidence validity.
    pub validity: EvidenceValidity,
    /// Role that accepted or rejected the evidence.
    pub decided_by: RoleId,
    /// Date the decision was made.
    pub decided_on: Date,
    /// Explanation shown in derived reports.
    pub explanation: String,
    /// Restricted reference-only evidence.
    evidence: Vec<ExternalRef>,
}
impl QualificationEvidence {
    /// Builds qualification evidence for a supplier requirement.
    #[must_use]
    pub fn new(
        supplier: OrganizationId,
        requirement: ControlId,
        state: EvidenceState,
        decided_by: RoleId,
        decided_on: Date,
        explanation: impl Into<String>,
    ) -> Self {
        Self {
            supplier,
            requirement,
            state,
            validity: EvidenceValidity::unbounded(),
            decided_by,
            decided_on,
            explanation: explanation.into(),
            evidence: Vec::new(),
        }
    }

    /// Sets evidence validity.
    #[must_use]
    pub fn with_validity(mut self, validity: EvidenceValidity) -> Self {
        self.validity = validity;
        self
    }

    /// Adds restricted reference-only evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Returns evidence refs only when the caller has supplier-read.
    pub fn evidence<'a>(&'a self, granted_capabilities: &[String]) -> Result<&'a [ExternalRef]> {
        require_supplier_read(granted_capabilities)?;
        Ok(&self.evidence)
    }

    fn validate(&self) -> Result<()> {
        if self.explanation.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "qualification_evidence.explanation",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "qualification_evidence.evidence",
            ));
        }
        Ok(())
    }
}
/// Derived supplier qualification status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QualificationStatus {
    /// Mandatory evidence is accepted and current.
    Qualified,
    /// Mandatory evidence is currently accepted but will expire within the caller's warning date.
    Expiring,
    /// Evidence was rejected by the accountable authority.
    Rejected,
    /// Accepted evidence conflicts with another accepted decision.
    Conflicted,
    /// Required evidence is missing, incomplete, or expired.
    NotQualified,
}
/// Derived supplier qualification report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierQualificationReport {
    /// Supplier organization.
    pub supplier: OrganizationId,
    /// Evaluation date.
    pub as_of_date: Date,
    /// Derived status.
    pub status: QualificationStatus,
    /// Requirement states in requirement-id order.
    pub requirement_states: Vec<(ControlId, EvidenceState)>,
    /// Explanation path in stable order.
    pub explanations: Vec<String>,
}
/// Project-scoped supplier qualification control set.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierQualificationSet {
    /// Supplier and subcontract-chain references.
    pub suppliers: Vec<SupplierReference>,
    /// Qualification requirements.
    pub requirements: Vec<QualificationRequirement>,
    /// Restricted qualification decisions and evidence.
    pub evidence: Vec<QualificationEvidence>,
}
impl SupplierQualificationSet {
    /// Builds an empty supplier qualification set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a supplier reference.
    #[must_use]
    pub fn with_supplier(mut self, supplier: SupplierReference) -> Self {
        self.suppliers.push(supplier);
        self
    }

    /// Adds a qualification requirement.
    #[must_use]
    pub fn with_requirement(mut self, requirement: QualificationRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Adds a qualification evidence decision.
    #[must_use]
    pub fn with_evidence(mut self, evidence: QualificationEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Derives qualification status and explanation paths for a supplier.
    pub fn qualification_for(
        &self,
        supplier: &OrganizationId,
        as_of_date: Date,
        expiry_warning_on: Date,
    ) -> Result<SupplierQualificationReport> {
        self.validate()?;
        let supplier_ref = self
            .suppliers
            .iter()
            .find(|candidate| &candidate.supplier == supplier)
            .ok_or_else(|| ConstructionProjectError::UnknownSupplier {
                supplier: supplier.clone(),
            })?;
        let mut requirement_states = Vec::new();
        let mut explanations = Vec::new();
        let mut has_rejected = false;
        let mut has_conflict = false;
        let mut has_expiring = !supplier_ref.validity.contains(expiry_warning_on);
        let mut missing_mandatory = false;

        if !supplier_ref.validity.contains(as_of_date) {
            missing_mandatory = true;
            explanations.push(format!("supplier {} reference is expired", supplier));
        }

        let evidence_by_requirement = self.current_evidence_by_requirement(supplier);
        for requirement in &self.requirements {
            let requirement_id = requirement.obligation.requirement.id.clone();
            let decisions = evidence_by_requirement
                .get(&requirement_id)
                .cloned()
                .unwrap_or_default();
            let state = state_for_decisions(&decisions, as_of_date);
            requirement_states.push((requirement_id.clone(), state));

            if state == EvidenceState::Conflicted {
                has_conflict = true;
                explanations.push(format!(
                    "requirement {requirement_id} has conflicting evidence"
                ));
            } else if state == EvidenceState::Rejected {
                has_rejected = true;
                explanations.push(format!("requirement {requirement_id} was rejected"));
            } else if state == EvidenceState::Expired {
                missing_mandatory = true;
                explanations.push(format!("requirement {requirement_id} is expired"));
            } else if state == EvidenceState::Accepted {
                if decisions
                    .iter()
                    .any(|decision| !decision.validity.contains(expiry_warning_on))
                {
                    has_expiring = true;
                    explanations.push(format!("requirement {requirement_id} is expiring"));
                }
            } else if requirement.obligation.policy == ObligationPolicy::Mandatory {
                missing_mandatory = true;
                explanations.push(format!("requirement {requirement_id} is not accepted"));
            }
        }
        requirement_states.sort_by(|left, right| left.0.cmp(&right.0));
        explanations.sort();
        explanations.dedup();

        let status = if has_conflict {
            QualificationStatus::Conflicted
        } else if has_rejected {
            QualificationStatus::Rejected
        } else if missing_mandatory {
            QualificationStatus::NotQualified
        } else if has_expiring {
            QualificationStatus::Expiring
        } else {
            QualificationStatus::Qualified
        };

        Ok(SupplierQualificationReport {
            supplier: supplier.clone(),
            as_of_date,
            status,
            requirement_states,
            explanations,
        })
    }

    fn validate(&self) -> Result<()> {
        let mut suppliers = BTreeSet::new();
        for supplier in &self.suppliers {
            supplier.validate()?;
            if !suppliers.insert(supplier.supplier.clone()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "supplier",
                    id: supplier.supplier.as_str().to_owned(),
                });
            }
        }
        for requirement in &self.requirements {
            requirement.obligation.requirement.validate()?;
        }
        let requirement_ids = self
            .requirements
            .iter()
            .map(|requirement| requirement.obligation.requirement.id.clone())
            .collect::<BTreeSet<_>>();
        for evidence in &self.evidence {
            evidence.validate()?;
            if !suppliers.contains(&evidence.supplier) {
                return Err(ConstructionProjectError::UnknownSupplier {
                    supplier: evidence.supplier.clone(),
                });
            }
            if !requirement_ids.contains(&evidence.requirement) {
                return Err(ConstructionProjectError::UnknownQualificationRequirement {
                    requirement: evidence.requirement.clone(),
                });
            }
            let supplier_ref = self
                .suppliers
                .iter()
                .find(|supplier| supplier.supplier == evidence.supplier)
                .expect("validated supplier presence");
            if evidence.decided_by != supplier_ref.decision_authority {
                return Err(ConstructionProjectError::QualificationAuthorityMismatch {
                    supplier: evidence.supplier.clone(),
                    expected: supplier_ref.decision_authority.clone(),
                    actual: evidence.decided_by.clone(),
                });
            }
        }
        Ok(())
    }

    fn current_evidence_by_requirement(
        &self,
        supplier: &OrganizationId,
    ) -> BTreeMap<ControlId, Vec<&QualificationEvidence>> {
        let mut by_requirement: BTreeMap<ControlId, Vec<&QualificationEvidence>> = BTreeMap::new();
        for evidence in self
            .evidence
            .iter()
            .filter(|evidence| &evidence.supplier == supplier)
        {
            by_requirement
                .entry(evidence.requirement.clone())
                .or_default()
                .push(evidence);
        }
        by_requirement
    }
}

fn state_for_decisions(decisions: &[&QualificationEvidence], as_of_date: Date) -> EvidenceState {
    if decisions.is_empty() {
        return EvidenceState::Missing;
    }
    if decisions
        .iter()
        .any(|decision| decision.state == EvidenceState::Conflicted)
    {
        return EvidenceState::Conflicted;
    }
    let accepted = decisions
        .iter()
        .filter(|decision| decision.state == EvidenceState::Accepted)
        .collect::<Vec<_>>();
    if accepted.len() > 1 {
        return EvidenceState::Conflicted;
    }
    if let Some(decision) = accepted.first() {
        return if decision.validity.contains(as_of_date) {
            EvidenceState::Accepted
        } else {
            EvidenceState::Expired
        };
    }
    if decisions
        .iter()
        .any(|decision| decision.state == EvidenceState::Rejected)
    {
        return EvidenceState::Rejected;
    }
    decisions
        .iter()
        .map(|decision| decision.state)
        .max()
        .unwrap_or(EvidenceState::Missing)
}

fn require_supplier_read(granted_capabilities: &[String]) -> Result<()> {
    if granted_capabilities
        .iter()
        .any(|capability| capability == CONSTRUCTION_SUPPLIER_READ_CAPABILITY)
    {
        Ok(())
    } else {
        Err(ConstructionProjectError::MissingCapability {
            capability: CONSTRUCTION_SUPPLIER_READ_CAPABILITY,
        })
    }
}
