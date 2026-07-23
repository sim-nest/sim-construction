//! Project identity and charter records.

use sim_lib_doc_core::ExternalRef;

/// Capability name for construction project-control operations.
pub const PROJECT_CONTROL_CAPABILITY: &str = "construction/project";

/// Open kind name for a construction project charter.
pub const PROJECT_CHARTER_KIND: &str = "construction/project-charter";

/// Stable construction project identifier.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ProjectId(pub String);

impl ProjectId {
    /// Builds a project id.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrows the project id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable accountable role identifier.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct RoleId(pub String);

impl RoleId {
    /// Builds a role id.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrows the role id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Minimal accepted project charter.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectCharter {
    /// Stable project identity.
    pub project: ProjectId,
    /// Human-facing project name for reports and packs.
    pub name: String,
    /// Customer intent or outcome statement captured for the project.
    pub customer_intent: String,
    /// Delivery model, procurement form, or collaboration frame.
    pub delivery_model: String,
    /// Project currency code for construction-side commercial summaries.
    pub currency: String,
    /// Role that accepts the charter.
    pub accepted_by: Option<RoleId>,
    /// Calendar date when the charter is accepted.
    pub accepted_on: Option<String>,
    /// Reference-only evidence supporting the charter.
    pub evidence: Vec<ExternalRef>,
}

impl ProjectCharter {
    /// Starts a charter with required identity and name.
    #[must_use]
    pub fn new(project: ProjectId, name: impl Into<String>) -> Self {
        Self {
            project,
            name: name.into(),
            customer_intent: String::new(),
            delivery_model: String::new(),
            currency: String::new(),
            accepted_by: None,
            accepted_on: None,
            evidence: Vec::new(),
        }
    }

    /// Sets the customer intent statement.
    #[must_use]
    pub fn with_customer_intent(mut self, intent: impl Into<String>) -> Self {
        self.customer_intent = intent.into();
        self
    }

    /// Sets the delivery model.
    #[must_use]
    pub fn with_delivery_model(mut self, model: impl Into<String>) -> Self {
        self.delivery_model = model.into();
        self
    }

    /// Sets the project currency code.
    #[must_use]
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Marks the charter accepted by a role on a date string.
    #[must_use]
    pub fn accepted_by(mut self, role: RoleId, accepted_on: impl Into<String>) -> Self {
        self.accepted_by = Some(role);
        self.accepted_on = Some(accepted_on.into());
        self
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }
}
