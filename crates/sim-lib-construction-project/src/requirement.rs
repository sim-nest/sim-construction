//! Shared construction requirements across open project-control lanes.

use sim_kernel::Symbol;
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ConstructionProjectError, ControlId, Result, RoleId};

/// Open construction requirement lane.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequirementLane(pub Symbol);

impl RequirementLane {
    /// Builds an open requirement lane symbol.
    #[must_use]
    pub fn new(symbol: Symbol) -> Self {
        Self(symbol)
    }
}

impl serde::Serialize for RequirementLane {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.as_qualified_str())
    }
}

impl<'de> serde::Deserialize<'de> for RequirementLane {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        symbol_from_text(&value)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

/// Shared requirement shape for customer, design, authority, procurement,
/// supplier, safety, quality, environment, sustainability, production,
/// handover, commercial, people, place, and reference lanes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Requirement {
    /// Stable requirement control id.
    pub id: ControlId,
    /// Open lane symbol.
    pub lane: RequirementLane,
    /// Human-readable requirement title.
    pub title: String,
    /// Accountable owner role.
    pub owner: RoleId,
    /// Role authorized to accept satisfaction.
    pub acceptance_authority: RoleId,
    /// Optional due date.
    pub due_on: Option<Date>,
    /// True when evidence, not a report alone, is mandatory.
    pub evidence_required: bool,
    /// Open evidence kind symbols accepted for this requirement.
    #[serde(with = "symbol_list")]
    pub evidence_kinds: Vec<Symbol>,
    /// Source references that introduced or govern this requirement.
    pub source_refs: Vec<ExternalRef>,
    /// Requirement dependencies that must also be satisfied.
    pub dependencies: Vec<ControlId>,
    /// True when policy may not waive this requirement.
    pub non_waivable: bool,
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
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
    use sim_kernel::Symbol;

    use super::symbol_from_text;

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

impl Requirement {
    /// Builds a shared requirement.
    #[must_use]
    pub fn new(
        id: ControlId,
        lane: RequirementLane,
        title: impl Into<String>,
        owner: RoleId,
        acceptance_authority: RoleId,
    ) -> Self {
        Self {
            id,
            lane,
            title: title.into(),
            owner,
            acceptance_authority,
            due_on: None,
            evidence_required: true,
            evidence_kinds: Vec::new(),
            source_refs: Vec::new(),
            dependencies: Vec::new(),
            non_waivable: false,
        }
    }

    /// Sets the due date.
    #[must_use]
    pub fn due_on(mut self, date: Date) -> Self {
        self.due_on = Some(date);
        self
    }

    /// Marks evidence as optional for this requirement.
    #[must_use]
    pub fn evidence_optional(mut self) -> Self {
        self.evidence_required = false;
        self
    }

    /// Adds an accepted evidence kind symbol.
    #[must_use]
    pub fn with_evidence_kind(mut self, kind: Symbol) -> Self {
        self.evidence_kinds.push(kind);
        self
    }

    /// Adds a source reference.
    #[must_use]
    pub fn with_source_ref(mut self, source_ref: ExternalRef) -> Self {
        self.source_refs.push(source_ref);
        self
    }

    /// Adds a dependency requirement id.
    #[must_use]
    pub fn depends_on(mut self, dependency: ControlId) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Marks the requirement as non-waivable by exception policy.
    #[must_use]
    pub fn non_waivable(mut self) -> Self {
        self.non_waivable = true;
        self
    }

    /// Validates the local requirement shape.
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("requirement.title"));
        }
        if self.source_refs.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "requirement.source_refs",
            ));
        }
        Ok(())
    }
}
