//! Bounded construction exception decisions.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, ConstructionProjectError, ControlId, ProjectId, Result,
    RoleId,
};

/// Scope covered by an exception decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExceptionScope {
    /// Project identity.
    pub project: ProjectId,
    /// Requirement ids covered by the exception.
    pub requirements: Vec<ControlId>,
}

impl ExceptionScope {
    /// Builds an exception scope for one project.
    #[must_use]
    pub fn new(project: ProjectId) -> Self {
        Self {
            project,
            requirements: Vec::new(),
        }
    }

    /// Adds one covered requirement.
    #[must_use]
    pub fn covers(mut self, requirement: ControlId) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Returns true when this scope covers the requirement id.
    #[must_use]
    pub fn covers_requirement(&self, requirement: &ControlId) -> bool {
        self.requirements
            .iter()
            .any(|covered| covered == requirement)
    }
}

/// Accountable, bounded exception decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExceptionDecision {
    /// Stable exception control id.
    pub id: ControlId,
    /// Project and requirement scope.
    pub scope: ExceptionScope,
    /// Role that made the exception decision.
    pub decided_by: RoleId,
    /// Role authorized to make the exception decision.
    pub authority: RoleId,
    /// Reason for accepting the exception.
    pub reason: String,
    /// Date on which the decision was made.
    pub decided_on: Date,
    /// Expiry date after which the exception no longer applies.
    pub expires_on: Date,
    /// Reference-only evidence links supporting the exception.
    pub evidence: Vec<ExternalRef>,
}

impl ExceptionDecision {
    /// Builds an exception decision.
    #[must_use]
    pub fn new(
        id: ControlId,
        scope: ExceptionScope,
        decided_by: RoleId,
        authority: RoleId,
        reason: impl Into<String>,
        decided_on: Date,
        expires_on: Date,
    ) -> Self {
        Self {
            id,
            scope,
            decided_by,
            authority,
            reason: reason.into(),
            decided_on,
            expires_on,
            evidence: Vec::new(),
        }
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates authority, capability, reason, evidence, and expiry.
    pub fn validate(&self, granted_capabilities: &[String], as_of_date: Date) -> Result<()> {
        if !granted_capabilities
            .iter()
            .any(|capability| capability == CONSTRUCTION_EXCEPTION_CAPABILITY)
        {
            return Err(ConstructionProjectError::MissingCapability {
                capability: CONSTRUCTION_EXCEPTION_CAPABILITY,
            });
        }
        if self.scope.requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "exception.scope.requirements",
            ));
        }
        if self.decided_by != self.authority {
            return Err(ConstructionProjectError::ExceptionAuthorityMismatch {
                exception: self.id.clone(),
                expected: self.authority.clone(),
                actual: self.decided_by.clone(),
            });
        }
        if self.reason.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("exception.reason"));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "exception.evidence",
            ));
        }
        if as_of_date > self.expires_on {
            return Err(ConstructionProjectError::ExpiredException {
                exception: self.id.clone(),
                expired_on: self.expires_on,
                as_of_date,
            });
        }
        Ok(())
    }
}
