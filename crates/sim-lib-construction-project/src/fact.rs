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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectFact {
    /// Monotone event sequence assigned by the project book writer.
    pub seq: u64,
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable construction-control subject identity.
    pub subject: ControlId,
    /// Open fact kind symbol.
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
    pub body: Expr,
    /// Reference-only external evidence links.
    pub evidence: Vec<ExternalRef>,
    /// Evidence state declared by the fact.
    pub evidence_state: EvidenceState,
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
        Ok(())
    }
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
