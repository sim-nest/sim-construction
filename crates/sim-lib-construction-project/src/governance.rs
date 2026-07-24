//! Governance records for construction project control.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use sim_kernel::Symbol;

use crate::{ConstructionProjectError, OrganizationId, ProjectId, Result, RoleId};

/// Disclosure visibility for project-control records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// Visible within the project control group.
    Project,
    /// Visible to customer-side participants.
    Customer,
    /// Visible to supplier-side participants.
    Supplier,
    /// Visible when evaluating reference candidacy.
    ReferenceCandidate,
    /// Visible only under a named restricted policy.
    Restricted(Symbol),
}

impl Visibility {
    /// Returns the stable visibility label.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Project => "project".to_owned(),
            Self::Customer => "customer".to_owned(),
            Self::Supplier => "supplier".to_owned(),
            Self::ReferenceCandidate => "reference-candidate".to_owned(),
            Self::Restricted(symbol) => format!("restricted:{}", symbol.as_qualified_str()),
        }
    }
}

impl Serialize for Visibility {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.label())
    }
}

impl<'de> Deserialize<'de> for Visibility {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "project" => Ok(Self::Project),
            "customer" => Ok(Self::Customer),
            "supplier" => Ok(Self::Supplier),
            "reference-candidate" => Ok(Self::ReferenceCandidate),
            _ => {
                let symbol = value
                    .strip_prefix("restricted:")
                    .ok_or_else(|| D::Error::custom(format!("unknown visibility {value:?}")))
                    .and_then(|raw| symbol_from_text(raw).map_err(D::Error::custom))?;
                Ok(Self::Restricted(symbol))
            }
        }
    }
}

/// Role assignment inside a construction project organization.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoleAssignment {
    /// Stable role identity.
    pub role: RoleId,
    /// Stable organization identity.
    pub organization: OrganizationId,
    /// Responsibilities carried by this role.
    pub responsibilities: Vec<String>,
    /// Decision kinds this role may decide.
    #[serde(with = "symbol_list")]
    pub may_decide: Vec<Symbol>,
    /// Optional escalation role.
    pub escalates_to: Option<RoleId>,
    /// Disclosure visibility for the assignment.
    pub visibility: Visibility,
}

impl RoleAssignment {
    /// Starts a role assignment.
    #[must_use]
    pub fn new(role: RoleId, organization: OrganizationId, visibility: Visibility) -> Self {
        Self {
            role,
            organization,
            responsibilities: Vec::new(),
            may_decide: Vec::new(),
            escalates_to: None,
            visibility,
        }
    }

    /// Adds a responsibility.
    #[must_use]
    pub fn with_responsibility(mut self, responsibility: impl Into<String>) -> Self {
        self.responsibilities.push(responsibility.into());
        self
    }

    /// Adds a decision kind this role may decide.
    #[must_use]
    pub fn may_decide(mut self, decision: Symbol) -> Self {
        self.may_decide.push(decision);
        self
    }

    /// Sets the escalation target.
    #[must_use]
    pub fn escalates_to(mut self, target: RoleId) -> Self {
        self.escalates_to = Some(target);
        self
    }
}

/// Due-date policy for project governance decisions.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DueDatePolicy {
    /// Normal due window in calendar days.
    pub response_days: u16,
    /// Extra days before escalation after a missed due date.
    pub escalation_grace_days: u16,
}

impl DueDatePolicy {
    /// Builds a due-date policy.
    pub fn new(response_days: u16, escalation_grace_days: u16) -> Result<Self> {
        validate_day_count("response_days", response_days)?;
        validate_day_count("escalation_grace_days", escalation_grace_days)?;
        Ok(Self {
            response_days,
            escalation_grace_days,
        })
    }
}

/// Disclosure policy for restricted project-control records.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VisibilityPolicy {
    /// Default disclosure visibility.
    pub default_visibility: Visibility,
    /// Restricted visibility symbols permitted in this project.
    #[serde(with = "symbol_list")]
    pub restricted: Vec<Symbol>,
}

impl VisibilityPolicy {
    /// Builds a project-default visibility policy.
    #[must_use]
    pub fn project() -> Self {
        Self {
            default_visibility: Visibility::Project,
            restricted: Vec::new(),
        }
    }

    /// Adds an allowed restricted visibility symbol.
    #[must_use]
    pub fn with_restricted(mut self, symbol: Symbol) -> Self {
        self.restricted.push(symbol);
        self
    }

    /// Returns whether the visibility is allowed by this policy.
    #[must_use]
    pub fn allows(&self, visibility: &Visibility) -> bool {
        match visibility {
            Visibility::Restricted(symbol) => self.restricted.contains(symbol),
            _ => true,
        }
    }
}

/// Governance record for a construction project.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectGovernance {
    /// Stable project identity.
    pub project: ProjectId,
    /// Organization role assignments.
    pub roles: Vec<RoleAssignment>,
    /// Due-date policy for role-owned decisions.
    pub due_date_policy: DueDatePolicy,
    /// Disclosure visibility policy.
    pub visibility_policy: VisibilityPolicy,
}

impl ProjectGovernance {
    /// Starts a governance record.
    #[must_use]
    pub fn new(project: ProjectId, due_date_policy: DueDatePolicy) -> Self {
        Self {
            project,
            roles: Vec::new(),
            due_date_policy,
            visibility_policy: VisibilityPolicy::project(),
        }
    }

    /// Sets the visibility policy.
    #[must_use]
    pub fn with_visibility_policy(mut self, policy: VisibilityPolicy) -> Self {
        self.visibility_policy = policy;
        self
    }

    /// Adds a role assignment.
    #[must_use]
    pub fn with_role(mut self, role: RoleAssignment) -> Self {
        self.roles.push(role);
        self
    }

    /// Validates role identities, escalation authority, and visibility policy.
    pub fn validate(&self) -> Result<()> {
        if self.roles.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection("roles"));
        }
        validate_visibility(
            &self.visibility_policy,
            &self.visibility_policy.default_visibility,
        )?;

        let mut role_ids = BTreeSet::new();
        for assignment in &self.roles {
            if !role_ids.insert(assignment.role.clone()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "role",
                    id: assignment.role.to_string(),
                });
            }
            validate_texts(&assignment.responsibilities, "responsibilities")?;
            validate_visibility(&self.visibility_policy, &assignment.visibility)?;
        }

        let role_map: BTreeMap<RoleId, Option<RoleId>> = self
            .roles
            .iter()
            .map(|assignment| (assignment.role.clone(), assignment.escalates_to.clone()))
            .collect();
        for assignment in &self.roles {
            if let Some(target) = &assignment.escalates_to
                && !role_map.contains_key(target)
            {
                return Err(ConstructionProjectError::MissingEscalationTarget {
                    role: assignment.role.clone(),
                    target: target.clone(),
                });
            }
        }
        validate_no_cycles(&role_map)?;
        Ok(())
    }
}

fn validate_day_count(field: &'static str, days: u16) -> Result<()> {
    if days == 0 || days > 366 {
        Err(ConstructionProjectError::InvalidDueDatePolicy {
            field,
            max_days: 366,
        })
    } else {
        Ok(())
    }
}

fn validate_texts(values: &[String], field: &'static str) -> Result<()> {
    if values.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(field));
    }
    for value in values {
        if value.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(field));
        }
    }
    Ok(())
}

fn validate_visibility(policy: &VisibilityPolicy, visibility: &Visibility) -> Result<()> {
    if policy.allows(visibility) {
        Ok(())
    } else {
        Err(ConstructionProjectError::RestrictedVisibilityDenied {
            symbol: visibility.label(),
        })
    }
}

fn validate_no_cycles(role_map: &BTreeMap<RoleId, Option<RoleId>>) -> Result<()> {
    for start in role_map.keys() {
        let mut seen = BTreeSet::new();
        let mut cursor = start;
        while let Some(Some(next)) = role_map.get(cursor) {
            if !seen.insert(cursor.clone()) {
                return Err(ConstructionProjectError::AuthorityCycle(start.clone()));
            }
            cursor = next;
        }
    }
    Ok(())
}

fn symbol_from_text(value: &str) -> std::result::Result<Symbol, String> {
    if value.is_empty() {
        return Err("empty symbol".to_owned());
    }
    if let Some((namespace, name)) = value.split_once('/') {
        if name.contains('/') {
            return Err("qualified symbol contains more than one '/'".to_owned());
        }
        validate_symbol_part(namespace)?;
        validate_symbol_part(name)?;
        Ok(Symbol::qualified(namespace.to_owned(), name.to_owned()))
    } else {
        Symbol::checked(value.to_owned()).map_err(|error| error.to_string())
    }
}

fn validate_symbol_part(value: &str) -> std::result::Result<(), String> {
    if value.is_empty() {
        return Err("empty qualified symbol part".to_owned());
    }
    if value.chars().any(char::is_control) {
        return Err("symbol contains a control character".to_owned());
    }
    Ok(())
}

mod symbol_list {
    use super::{Deserialize, Deserializer, Serialize, Serializer, symbol_from_text};
    use serde::de::Error as DeError;
    use sim_kernel::Symbol;

    pub fn serialize<S>(symbols: &[Symbol], serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let values: Vec<String> = symbols.iter().map(Symbol::as_qualified_str).collect();
        values.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Vec<Symbol>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = Vec::<String>::deserialize(deserializer)?;
        values
            .into_iter()
            .map(|value| symbol_from_text(&value).map_err(D::Error::custom))
            .collect()
    }
}
