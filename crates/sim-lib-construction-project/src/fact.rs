//! Append-only facts for construction project-control books.

use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, EvidenceState, ProjectId, Result, RoleId, Visibility,
};

/// Maximum expression nodes accepted in a project fact body.
pub const MAX_FACT_BODY_NODES: usize = 256;

/// Maximum reference-only evidence links accepted on a project fact.
pub const MAX_FACT_EVIDENCE_REFS: usize = 32;

/// Evidence-backed construction project fact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectFact {
    /// Monotone event sequence assigned by the project book writer.
    pub seq: u64,
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable construction-control subject identity.
    pub subject: ControlId,
    /// Open fact kind symbol.
    #[serde(with = "crate::outcome_symbol")]
    pub kind: Symbol,
    /// Calendar date when the fact becomes effective for project control.
    pub effective_on: Date,
    /// Accountable project role that wrote the fact.
    pub actor_role: RoleId,
    /// Prior fact sequence corrected by this fact.
    pub supersedes: Option<u64>,
    /// Disclosure visibility for this fact.
    pub visibility: Visibility,
    /// Shape-ready expression body carried by the fact.
    #[serde(with = "expr_wire")]
    pub body: Expr,
    /// Reference-only external evidence links.
    pub evidence: Vec<ExternalRef>,
    /// Evidence state declared by the fact.
    pub evidence_state: EvidenceState,
}

mod expr_wire {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
    use sim_codec::{DecodeBudget, DecodeLimits};
    use sim_kernel::{CodecId, Expr};

    pub fn serialize<S>(value: &Expr, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        sim_codec_json::expr_to_json(value).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Expr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let mut budget = DecodeBudget::new(DecodeLimits::default());
        sim_codec_json::json_to_expr(CodecId(0), &value, &mut budget, 0).map_err(D::Error::custom)
    }
}

impl ProjectFact {
    /// Builds a project fact with accepted evidence state and no supersession.
    #[must_use]
    pub fn new(
        seq: u64,
        project: ProjectId,
        subject: ControlId,
        kind: Symbol,
        effective_on: Date,
        actor_role: RoleId,
        body: Expr,
    ) -> Self {
        Self {
            seq,
            project,
            subject,
            kind,
            effective_on,
            actor_role,
            supersedes: None,
            visibility: Visibility::Project,
            body,
            evidence: Vec::new(),
            evidence_state: EvidenceState::Accepted,
        }
    }

    /// Sets the superseded sequence.
    #[must_use]
    pub fn supersedes(mut self, sequence: u64) -> Self {
        self.supersedes = Some(sequence);
        self
    }

    /// Sets the disclosure visibility.
    #[must_use]
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Sets the declared evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, state: EvidenceState) -> Self {
        self.evidence_state = state;
        self
    }

    /// Validates local fact bounds and sequence shape.
    pub fn validate_bounds(&self) -> Result<()> {
        if self.seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "fact.seq",
                sequence: self.seq,
            });
        }
        ProjectId::new(self.project.as_str())?;
        ControlId::new(self.subject.as_str())?;
        RoleId::new(self.actor_role.as_str())?;
        validate_kind(&self.kind)?;
        let nodes = expr_node_count(&self.body);
        if nodes > MAX_FACT_BODY_NODES {
            return Err(ConstructionProjectError::FactBodyTooLarge {
                sequence: self.seq,
                nodes,
                max: MAX_FACT_BODY_NODES,
            });
        }
        if self.evidence.len() > MAX_FACT_EVIDENCE_REFS {
            return Err(ConstructionProjectError::EvidenceLimitExceeded {
                sequence: self.seq,
                count: self.evidence.len(),
                max: MAX_FACT_EVIDENCE_REFS,
            });
        }
        for reference in &self.evidence {
            validate_reference(reference)?;
        }
        Ok(())
    }
}

fn validate_kind(kind: &Symbol) -> Result<()> {
    let name = kind.name.as_ref();
    let namespace = kind.namespace.as_deref();
    if name.is_empty()
        || name.contains('/')
        || name.chars().any(char::is_control)
        || namespace.is_some_and(|value| {
            value.is_empty() || value.contains('/') || value.chars().any(char::is_control)
        })
    {
        return Err(ConstructionProjectError::InvalidSymbol {
            value: kind.as_qualified_str(),
            reason: "fact kind must be an unambiguous printable symbol".to_owned(),
        });
    }
    Ok(())
}

fn validate_reference(reference: &ExternalRef) -> Result<()> {
    if reference.backend.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField(
            "fact.evidence.backend",
        ));
    }
    if reference.external_id.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField(
            "fact.evidence.external_id",
        ));
    }
    if reference
        .version
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(ConstructionProjectError::EmptyField(
            "fact.evidence.version",
        ));
    }
    if reference
        .web_url
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(ConstructionProjectError::EmptyField(
            "fact.evidence.web_url",
        ));
    }
    Ok(())
}

/// Counts expression nodes in a fact body.
#[must_use]
pub fn expr_node_count(expr: &Expr) -> usize {
    1 + match expr {
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Number(_)
        | Expr::Symbol(_)
        | Expr::Local(_)
        | Expr::String(_)
        | Expr::Bytes(_) => 0,
        Expr::List(items) | Expr::Vector(items) | Expr::Set(items) | Expr::Block(items) => {
            items.iter().map(expr_node_count).sum()
        }
        Expr::Map(entries) => entries
            .iter()
            .map(|(key, value)| expr_node_count(key) + expr_node_count(value))
            .sum(),
        Expr::Call { operator, args } => {
            expr_node_count(operator) + args.iter().map(expr_node_count).sum::<usize>()
        }
        Expr::Infix { left, right, .. } => expr_node_count(left) + expr_node_count(right),
        Expr::Prefix { arg, .. } | Expr::Postfix { arg, .. } => expr_node_count(arg),
        Expr::Quote { expr, .. } => expr_node_count(expr),
        Expr::Annotated { expr, annotations } => {
            expr_node_count(expr)
                + annotations
                    .iter()
                    .map(|(_, value)| expr_node_count(value))
                    .sum::<usize>()
        }
        Expr::Extension { payload, .. } => expr_node_count(payload),
    }
}
