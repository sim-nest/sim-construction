//! Project charter records for construction control.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    BaselineId, ConstructionProjectError, ControlId, ProjectId, Result, RoleId, Visibility,
};

/// Open kind name for a construction project charter.
pub const PROJECT_CHARTER_KIND: &str = "construction/project-charter";

/// Exact ISO 4217 currency used by a construction project.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Validates and builds a project currency code.
    pub fn new(code: impl Into<String>) -> Result<Self> {
        let code = code.into();
        if is_known_project_currency(&code) {
            Ok(Self(code))
        } else {
            Err(ConstructionProjectError::UnknownCurrency(code))
        }
    }

    /// Borrows the currency code text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Reporting cadence for project-control summaries.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReportingCadence {
    /// Human-readable cadence label.
    pub label: String,
    /// Number of calendar days between required reports.
    pub interval_days: u16,
}

impl ReportingCadence {
    /// Builds a reporting cadence.
    pub fn new(label: impl Into<String>, interval_days: u16) -> Result<Self> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "reporting_cadence.label",
            ));
        }
        if interval_days == 0 || interval_days > 366 {
            return Err(ConstructionProjectError::InvalidDueDatePolicy {
                field: "reporting_cadence.interval_days",
                max_days: 366,
            });
        }
        Ok(Self {
            label,
            interval_days,
        })
    }

    /// Weekly project reporting cadence.
    #[must_use]
    pub fn weekly() -> Self {
        Self {
            label: "weekly".to_owned(),
            interval_days: 7,
        }
    }
}

/// Construction project charter.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectCharter {
    /// Stable project identity.
    pub project: ProjectId,
    /// Stable control identity for this charter.
    pub control: ControlId,
    /// Optional accepted baseline identity for this charter.
    pub baseline: Option<BaselineId>,
    /// Human-facing project name for reports and packs.
    pub name: String,
    /// Customer outcome captured for the project.
    pub customer_outcome: String,
    /// Property-side constraints that shape the project.
    pub property_constraints: Vec<String>,
    /// Product or delivery constraints that shape the project.
    pub product_constraints: Vec<String>,
    /// Procurement form or collaboration frame.
    pub procurement_form: String,
    /// Exact project currency.
    pub currency: CurrencyCode,
    /// Project objectives.
    pub objectives: Vec<String>,
    /// Project non-negotiables.
    pub non_negotiables: Vec<String>,
    /// Target outcomes used to judge the project.
    pub target_outcomes: Vec<String>,
    /// Reference criteria for candidate reference projects.
    pub reference_criteria: Vec<String>,
    /// Reporting cadence for control summaries.
    pub reporting_cadence: ReportingCadence,
    /// Role that accepts the charter.
    pub accepted_by: Option<RoleId>,
    /// Calendar date when the charter is accepted.
    pub accepted_on: Option<Date>,
    /// Disclosure visibility for the charter record.
    pub visibility: Visibility,
    /// Reference-only evidence supporting the charter.
    pub source_refs: Vec<ExternalRef>,
}

impl ProjectCharter {
    /// Starts a charter with required identity, name, and currency.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        name: impl Into<String>,
        currency: CurrencyCode,
    ) -> Self {
        Self {
            project,
            control,
            baseline: None,
            name: name.into(),
            customer_outcome: String::new(),
            property_constraints: Vec::new(),
            product_constraints: Vec::new(),
            procurement_form: String::new(),
            currency,
            objectives: Vec::new(),
            non_negotiables: Vec::new(),
            target_outcomes: Vec::new(),
            reference_criteria: Vec::new(),
            reporting_cadence: ReportingCadence::weekly(),
            accepted_by: None,
            accepted_on: None,
            visibility: Visibility::Project,
            source_refs: Vec::new(),
        }
    }

    /// Sets the accepted baseline identity.
    #[must_use]
    pub fn with_baseline(mut self, baseline: BaselineId) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Sets the customer outcome statement.
    #[must_use]
    pub fn with_customer_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.customer_outcome = outcome.into();
        self
    }

    /// Sets the customer outcome statement.
    #[must_use]
    pub fn with_customer_intent(self, intent: impl Into<String>) -> Self {
        self.with_customer_outcome(intent)
    }

    /// Adds a property constraint.
    #[must_use]
    pub fn with_property_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.property_constraints.push(constraint.into());
        self
    }

    /// Adds a product constraint.
    #[must_use]
    pub fn with_product_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.product_constraints.push(constraint.into());
        self
    }

    /// Sets the procurement form.
    #[must_use]
    pub fn with_procurement_form(mut self, form: impl Into<String>) -> Self {
        self.procurement_form = form.into();
        self
    }

    /// Sets the procurement form.
    #[must_use]
    pub fn with_delivery_model(self, model: impl Into<String>) -> Self {
        self.with_procurement_form(model)
    }

    /// Adds a project objective.
    #[must_use]
    pub fn with_objective(mut self, objective: impl Into<String>) -> Self {
        self.objectives.push(objective.into());
        self
    }

    /// Adds a non-negotiable project condition.
    #[must_use]
    pub fn with_non_negotiable(mut self, condition: impl Into<String>) -> Self {
        self.non_negotiables.push(condition.into());
        self
    }

    /// Adds a target outcome.
    #[must_use]
    pub fn with_target_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.target_outcomes.push(outcome.into());
        self
    }

    /// Adds a reference criterion.
    #[must_use]
    pub fn with_reference_criterion(mut self, criterion: impl Into<String>) -> Self {
        self.reference_criteria.push(criterion.into());
        self
    }

    /// Sets the reporting cadence.
    #[must_use]
    pub fn with_reporting_cadence(mut self, cadence: ReportingCadence) -> Self {
        self.reporting_cadence = cadence;
        self
    }

    /// Sets the charter visibility.
    #[must_use]
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Marks the charter accepted by a role on a date.
    #[must_use]
    pub fn accepted_by(mut self, role: RoleId, accepted_on: Date) -> Self {
        self.accepted_by = Some(role);
        self.accepted_on = Some(accepted_on);
        self
    }

    /// Adds a reference-only source link.
    #[must_use]
    pub fn with_source_ref(mut self, source_ref: ExternalRef) -> Self {
        self.source_refs.push(source_ref);
        self
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(self, evidence: ExternalRef) -> Self {
        self.with_source_ref(evidence)
    }

    /// Validates the charter content that must be stable before planning work.
    pub fn validate(&self) -> Result<()> {
        require_text(&self.name, "name")?;
        require_text(&self.customer_outcome, "customer_outcome")?;
        require_text(&self.procurement_form, "procurement_form")?;
        require_texts(&self.property_constraints, "property_constraints")?;
        require_texts(&self.product_constraints, "product_constraints")?;
        require_texts(&self.non_negotiables, "non_negotiables")?;
        require_texts(&self.target_outcomes, "target_outcomes")?;
        require_texts(&self.reference_criteria, "reference_criteria")?;
        if self.objectives.is_empty() {
            return Err(ConstructionProjectError::EmptyObjectives);
        }
        require_texts(&self.objectives, "objectives")?;
        require_refs(&self.source_refs, "source_refs")?;
        Ok(())
    }
}

fn require_text(value: &str, field: &'static str) -> Result<()> {
    if value.trim().is_empty() {
        Err(ConstructionProjectError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn require_texts(values: &[String], field: &'static str) -> Result<()> {
    if values.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(field));
    }
    for value in values {
        require_text(value, field)?;
    }
    Ok(())
}

fn require_refs(values: &[ExternalRef], field: &'static str) -> Result<()> {
    if values.is_empty() {
        Err(ConstructionProjectError::EmptyCollection(field))
    } else {
        Ok(())
    }
}

fn is_known_project_currency(code: &str) -> bool {
    code.len() == 3
        && code.chars().all(|ch| ch.is_ascii_uppercase())
        && KNOWN_PROJECT_CURRENCIES.contains(&code)
}

const KNOWN_PROJECT_CURRENCIES: &[&str] = &[
    "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT",
    "BGN", "BHD", "BIF", "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD",
    "CDF", "CHF", "CLP", "CNY", "COP", "CRC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD",
    "EGP", "ERN", "ETB", "EUR", "FJD", "FKP", "GBP", "GEL", "GHS", "GIP", "GMD", "GNF", "GTQ",
    "GYD", "HKD", "HNL", "HTG", "HUF", "IDR", "ILS", "INR", "IQD", "IRR", "ISK", "JMD", "JOD",
    "JPY", "KES", "KGS", "KHR", "KMF", "KRW", "KWD", "KYD", "KZT", "LAK", "LBP", "LKR", "LRD",
    "LSL", "LYD", "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP", "MRU", "MUR", "MVR", "MWK",
    "MXN", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK", "NPR", "NZD", "OMR", "PAB", "PEN", "PGK",
    "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD", "RUB", "RWF", "SAR", "SBD", "SCR", "SDG",
    "SEK", "SGD", "SHP", "SLE", "SOS", "SRD", "SSP", "STN", "SYP", "SZL", "THB", "TJS", "TMT",
    "TND", "TOP", "TRY", "TTD", "TWD", "TZS", "UAH", "UGX", "USD", "UYU", "UZS", "VES", "VND",
    "VUV", "WST", "XAF", "XCD", "XOF", "XPF", "YER", "ZAR", "ZMW", "ZWL",
];
