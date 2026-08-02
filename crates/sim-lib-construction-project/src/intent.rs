//! Customer intent records and requirement-backed coverage reports.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, GatePolicy, GatePolicyReport, ProjectBook, ProjectId,
    ProjectObligation, Result, RoleId,
};

/// Explicitly known or deliberately unknown customer-intent field.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntentField<T> {
    /// Field value, absent when the customer has not stated it.
    pub value: Option<T>,
    /// Reason the value is absent.
    pub unknown_reason: Option<String>,
    /// Reference-only evidence for the stated value or absence.
    pub evidence: Vec<ExternalRef>,
}

impl<T> IntentField<T> {
    /// Records a known customer-stated value.
    #[must_use]
    pub fn known(value: T, evidence: ExternalRef) -> Self {
        Self {
            value: Some(value),
            unknown_reason: None,
            evidence: vec![evidence],
        }
    }

    /// Records that the field is explicitly unknown.
    #[must_use]
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            value: None,
            unknown_reason: Some(reason.into()),
            evidence: Vec::new(),
        }
    }

    /// Returns true when no value is currently stated.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        self.value.is_none()
    }

    pub(crate) fn validate(&self, field: &'static str) -> Result<()> {
        if self.value.is_some() {
            if self.evidence.is_empty() {
                return Err(ConstructionProjectError::EmptyCollection(field));
            }
        } else if self
            .unknown_reason
            .as_ref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            return Err(ConstructionProjectError::EmptyField(field));
        }
        Ok(())
    }
}

/// High-level construction variant named by the customer intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConstructionVariant {
    /// New building work.
    NewBuild,
    /// Renovation, conversion, or tenant improvement work.
    Renovation,
}

/// Customer intent before it becomes accepted construction requirements.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomerIntent {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable control id for this intent record.
    pub control: ControlId,
    /// Intended use.
    pub intended_use: IntentField<String>,
    /// Scope boundary.
    pub scope_boundary: IntentField<String>,
    /// Property constraints.
    pub property_constraints: IntentField<Vec<String>>,
    /// Tenant constraints.
    pub tenant_constraints: IntentField<Vec<String>>,
    /// Success measures.
    pub success_measures: IntentField<Vec<String>>,
    /// Target outcomes.
    pub target_outcomes: IntentField<Vec<String>>,
    /// Delivery form.
    pub delivery_form: IntentField<String>,
    /// Procurement form.
    pub procurement_form: IntentField<String>,
    /// Time frame.
    pub time_frame: IntentField<String>,
    /// Commercial frame.
    pub commercial_frame: IntentField<String>,
    /// Project variant.
    pub variant: IntentField<ConstructionVariant>,
    /// Customer-stated assumptions.
    pub assumptions: Vec<String>,
    /// Customer-stated exclusions.
    pub exclusions: Vec<String>,
    /// Reference-only evidence for the record itself.
    pub evidence: Vec<ExternalRef>,
}

impl CustomerIntent {
    /// Starts a customer intent record with every field explicitly unknown.
    #[must_use]
    pub fn new(project: ProjectId, control: ControlId) -> Self {
        Self {
            project,
            control,
            intended_use: IntentField::unknown("not stated"),
            scope_boundary: IntentField::unknown("not stated"),
            property_constraints: IntentField::unknown("not stated"),
            tenant_constraints: IntentField::unknown("not stated"),
            success_measures: IntentField::unknown("not stated"),
            target_outcomes: IntentField::unknown("not stated"),
            delivery_form: IntentField::unknown("not stated"),
            procurement_form: IntentField::unknown("not stated"),
            time_frame: IntentField::unknown("not stated"),
            commercial_frame: IntentField::unknown("not stated"),
            variant: IntentField::unknown("not stated"),
            assumptions: Vec::new(),
            exclusions: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Adds a customer-stated assumption.
    #[must_use]
    pub fn with_assumption(mut self, assumption: impl Into<String>) -> Self {
        self.assumptions.push(assumption.into());
        self
    }

    /// Adds a customer-stated exclusion.
    #[must_use]
    pub fn with_exclusion(mut self, exclusion: impl Into<String>) -> Self {
        self.exclusions.push(exclusion.into());
        self
    }

    /// Adds reference-only evidence for the intent record.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Returns the names of fields that remain explicitly unknown.
    #[must_use]
    pub fn unknown_fields(&self) -> Vec<String> {
        let mut fields = Vec::new();
        push_unknown(&mut fields, "intended_use", &self.intended_use);
        push_unknown(&mut fields, "scope_boundary", &self.scope_boundary);
        push_unknown(
            &mut fields,
            "property_constraints",
            &self.property_constraints,
        );
        push_unknown(&mut fields, "tenant_constraints", &self.tenant_constraints);
        push_unknown(&mut fields, "success_measures", &self.success_measures);
        push_unknown(&mut fields, "target_outcomes", &self.target_outcomes);
        push_unknown(&mut fields, "delivery_form", &self.delivery_form);
        push_unknown(&mut fields, "procurement_form", &self.procurement_form);
        push_unknown(&mut fields, "time_frame", &self.time_frame);
        push_unknown(&mut fields, "commercial_frame", &self.commercial_frame);
        push_unknown(&mut fields, "variant", &self.variant);
        fields
    }

    /// Validates explicit absence, text lists, and reference-only evidence.
    pub fn validate(&self) -> Result<()> {
        self.intended_use.validate("intent.intended_use")?;
        self.scope_boundary.validate("intent.scope_boundary")?;
        self.property_constraints
            .validate("intent.property_constraints")?;
        self.tenant_constraints
            .validate("intent.tenant_constraints")?;
        self.success_measures.validate("intent.success_measures")?;
        self.target_outcomes.validate("intent.target_outcomes")?;
        self.delivery_form.validate("intent.delivery_form")?;
        self.procurement_form.validate("intent.procurement_form")?;
        self.time_frame.validate("intent.time_frame")?;
        self.commercial_frame.validate("intent.commercial_frame")?;
        self.variant.validate("intent.variant")?;
        validate_texts(&self.assumptions, "intent.assumptions")?;
        validate_texts(&self.exclusions, "intent.exclusions")?;
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection("intent.evidence"));
        }
        Ok(())
    }

    /// Derives coverage through shared requirements instead of accepting prose assumptions.
    pub fn coverage_report(
        &self,
        obligations: impl IntoIterator<Item = ProjectObligation>,
        book: &ProjectBook,
        as_of_seq: u64,
        as_of_date: Date,
    ) -> Result<IntentCoverageReport> {
        self.validate()?;
        let mut policy = GatePolicy::new();
        for obligation in obligations {
            policy = policy.with_obligation(obligation);
        }
        let requirement_report = policy.evaluate(book, as_of_seq, as_of_date)?;
        Ok(IntentCoverageReport {
            intent: self.control.clone(),
            as_of_seq,
            ready: self.unknown_fields().is_empty() && requirement_report.ready,
            unknown_fields: self.unknown_fields(),
            requirement_report,
        })
    }
}

/// Derived customer-intent coverage.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntentCoverageReport {
    /// Intent control id.
    pub intent: ControlId,
    /// Snapshot sequence used for derivation.
    pub as_of_seq: u64,
    /// True when all intent fields are stated and shared requirements are ready.
    pub ready: bool,
    /// Intent fields still explicitly unknown.
    pub unknown_fields: Vec<String>,
    /// Requirement graph report.
    pub requirement_report: GatePolicyReport,
}

/// Accountable customer decision accepting an intent basis.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomerIntentAcceptance {
    /// Intent control id.
    pub intent: ControlId,
    /// Role authorized to accept the customer intent basis.
    pub authority: RoleId,
    /// Role that made the decision.
    pub decided_by: RoleId,
    /// Date the decision was due.
    pub due_on: Date,
    /// Date the decision was made.
    pub decided_on: Date,
    /// Reference-only evidence for the decision.
    pub evidence: Vec<ExternalRef>,
}

impl CustomerIntentAcceptance {
    /// Builds a customer-intent acceptance decision.
    #[must_use]
    pub fn new(
        intent: ControlId,
        authority: RoleId,
        decided_by: RoleId,
        due_on: Date,
        decided_on: Date,
    ) -> Self {
        Self {
            intent,
            authority,
            decided_by,
            due_on,
            decided_on,
            evidence: Vec::new(),
        }
    }

    /// Adds reference-only evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Returns true when the customer decision missed the agreed due date.
    #[must_use]
    pub fn is_late(&self) -> bool {
        self.decided_on > self.due_on
    }

    /// Validates authority and evidence.
    pub fn validate(&self) -> Result<()> {
        if self.decided_by != self.authority {
            return Err(ConstructionProjectError::DecisionAuthorityMismatch {
                decision: self.intent.clone(),
                expected: self.authority.clone(),
                actual: self.decided_by.clone(),
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "intent_acceptance.evidence",
            ));
        }
        Ok(())
    }
}

fn push_unknown<T>(fields: &mut Vec<String>, name: &str, field: &IntentField<T>) {
    if field.is_unknown() {
        fields.push(name.to_owned());
    }
}

pub(crate) fn validate_texts(values: &[String], field: &'static str) -> Result<()> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(ConstructionProjectError::EmptyField(field));
    }
    Ok(())
}
