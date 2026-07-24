//! Deterministic as-of snapshots for construction project books.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ConstructionProjectError, ControlId, EvidenceState, ProjectBook, ProjectFact, ProjectId, Result,
};

/// Explanation category for a fact in a project snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SnapshotExplanationKind {
    /// Fact is the current visible fact for its subject.
    Current,
    /// Fact was superseded by a later correction.
    Superseded,
    /// Fact is rejected and remains provenance only.
    Rejected,
    /// Fact conflicts with another current candidate for the same subject.
    Conflicted,
}

/// Stable explanation row for a snapshot derivation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectSnapshotExplanation {
    /// Subject affected by this row.
    pub subject: ControlId,
    /// Fact sequence explained by this row.
    pub seq: u64,
    /// Fact kind rendered as a stable symbol string.
    pub kind: String,
    /// Evidence state carried by the fact.
    pub evidence_state: EvidenceState,
    /// Explanation category.
    pub explanation: SnapshotExplanationKind,
    /// Related sequence, such as a superseding or conflicting fact.
    pub related_seq: Option<u64>,
}

impl ProjectSnapshotExplanation {
    fn new(
        fact: &ProjectFact,
        explanation: SnapshotExplanationKind,
        related_seq: Option<u64>,
    ) -> Self {
        Self {
            subject: fact.subject.clone(),
            seq: fact.seq,
            kind: fact.kind.as_qualified_str(),
            evidence_state: fact.evidence_state,
            explanation,
            related_seq,
        }
    }
}

/// Deterministic as-of view of a project book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectSnapshot {
    /// Project represented by the snapshot.
    pub project: ProjectId,
    /// Inclusive sequence used to rebuild the snapshot.
    pub through_seq: u64,
    /// Latest non-rejected, non-conflicted fact per subject.
    pub current: BTreeMap<ControlId, ProjectFact>,
    /// Superseded facts retained by subject.
    pub superseded: BTreeMap<ControlId, Vec<ProjectFact>>,
    /// Conflicted facts retained by subject.
    pub conflicted: BTreeMap<ControlId, Vec<ProjectFact>>,
    /// Rejected facts retained by subject.
    pub rejected: BTreeMap<ControlId, Vec<ProjectFact>>,
    /// Byte-stable derivation explanation lane.
    pub explanations: Vec<ProjectSnapshotExplanation>,
}

impl ProjectSnapshot {
    /// Builds a snapshot at `through`.
    pub fn at(book: &ProjectBook, through: u64) -> Result<Self> {
        snapshot_at(book, through)
    }

    /// Returns the current fact for a subject.
    #[must_use]
    pub fn current_fact(&self, subject: &ControlId) -> Option<&ProjectFact> {
        self.current.get(subject)
    }

    /// Returns true when a subject is conflicted.
    #[must_use]
    pub fn is_conflicted(&self, subject: &ControlId) -> bool {
        self.conflicted.contains_key(subject)
    }
}

/// Changed subjects between two deterministic snapshots.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectDelta {
    /// Start sequence.
    pub from_seq: u64,
    /// Inclusive end sequence.
    pub through_seq: u64,
    /// Subjects whose current fact changed or appeared.
    pub added: Vec<ControlId>,
    /// Subjects with newly superseded facts.
    pub superseded: Vec<ControlId>,
    /// Subjects that became conflicted.
    pub conflicted: Vec<ControlId>,
}

/// Builds the deterministic snapshot at `through`.
pub fn snapshot_at(book: &ProjectBook, through: u64) -> Result<ProjectSnapshot> {
    let mut current = BTreeMap::new();
    let mut superseded = BTreeMap::<ControlId, Vec<ProjectFact>>::new();
    let mut conflicted = BTreeMap::<ControlId, Vec<ProjectFact>>::new();
    let mut rejected = BTreeMap::<ControlId, Vec<ProjectFact>>::new();
    let mut explanations = Vec::new();
    let mut superseded_sequences = BTreeSet::new();

    for fact in book.facts_through(through) {
        if fact.evidence_state == EvidenceState::Rejected {
            rejected
                .entry(fact.subject.clone())
                .or_default()
                .push(fact.clone());
            explanations.push(ProjectSnapshotExplanation::new(
                fact,
                SnapshotExplanationKind::Rejected,
                None,
            ));
            continue;
        }

        if let Some(prior_seq) = fact.supersedes
            && let Some(prior) = book.fact(prior_seq)
        {
            superseded_sequences.insert(prior_seq);
            superseded
                .entry(prior.subject.clone())
                .or_default()
                .push(prior.clone());
            explanations.push(ProjectSnapshotExplanation::new(
                prior,
                SnapshotExplanationKind::Superseded,
                Some(fact.seq),
            ));
            current.remove(&prior.subject);
        }

        if let Some(existing) = current.get(&fact.subject).cloned() {
            move_to_conflicted(&mut conflicted, &mut explanations, existing, Some(fact.seq));
            move_to_conflicted(
                &mut conflicted,
                &mut explanations,
                fact.clone(),
                Some(fact.seq),
            );
            current.remove(&fact.subject);
            continue;
        }

        if conflicted.contains_key(&fact.subject) {
            move_to_conflicted(
                &mut conflicted,
                &mut explanations,
                fact.clone(),
                Some(fact.seq),
            );
            continue;
        }

        current.insert(fact.subject.clone(), fact.clone());
    }

    for fact in current.values() {
        if !superseded_sequences.contains(&fact.seq) {
            explanations.push(ProjectSnapshotExplanation::new(
                fact,
                SnapshotExplanationKind::Current,
                None,
            ));
        }
    }

    sort_fact_lanes(&mut superseded);
    sort_fact_lanes(&mut conflicted);
    sort_fact_lanes(&mut rejected);
    explanations.sort_by(|left, right| {
        left.subject
            .cmp(&right.subject)
            .then(left.seq.cmp(&right.seq))
            .then(left.explanation_label().cmp(right.explanation_label()))
            .then(left.related_seq.cmp(&right.related_seq))
    });

    Ok(ProjectSnapshot {
        project: book.project().clone(),
        through_seq: through,
        current,
        superseded,
        conflicted,
        rejected,
        explanations,
    })
}

/// Builds the changed-subject report between two snapshots.
pub fn snapshot_delta(book: &ProjectBook, from_seq: u64, through_seq: u64) -> Result<ProjectDelta> {
    if from_seq > through_seq {
        return Err(ConstructionProjectError::InvalidSnapshotRange {
            from_seq,
            through_seq,
        });
    }
    let before = snapshot_at(book, from_seq)?;
    let after = snapshot_at(book, through_seq)?;

    let added = sorted_fact_subjects_in_range(book, from_seq, through_seq);
    let superseded = sorted_new_lane_subjects(&before.superseded, &after.superseded);
    let conflicted = sorted_new_lane_subjects(&before.conflicted, &after.conflicted);

    Ok(ProjectDelta {
        from_seq,
        through_seq,
        added,
        superseded,
        conflicted,
    })
}

impl ProjectSnapshotExplanation {
    fn explanation_label(&self) -> &'static str {
        match self.explanation {
            SnapshotExplanationKind::Current => "current",
            SnapshotExplanationKind::Superseded => "superseded",
            SnapshotExplanationKind::Rejected => "rejected",
            SnapshotExplanationKind::Conflicted => "conflicted",
        }
    }
}

fn move_to_conflicted(
    conflicted: &mut BTreeMap<ControlId, Vec<ProjectFact>>,
    explanations: &mut Vec<ProjectSnapshotExplanation>,
    fact: ProjectFact,
    related_seq: Option<u64>,
) {
    let lane = conflicted.entry(fact.subject.clone()).or_default();
    if lane.iter().any(|existing| existing.seq == fact.seq) {
        return;
    }
    explanations.push(ProjectSnapshotExplanation::new(
        &fact,
        SnapshotExplanationKind::Conflicted,
        related_seq,
    ));
    lane.push(fact);
}

fn sort_fact_lanes(lanes: &mut BTreeMap<ControlId, Vec<ProjectFact>>) {
    for facts in lanes.values_mut() {
        facts.sort_by_key(|fact| fact.seq);
    }
}

fn sorted_new_lane_subjects(
    before: &BTreeMap<ControlId, Vec<ProjectFact>>,
    after: &BTreeMap<ControlId, Vec<ProjectFact>>,
) -> Vec<ControlId> {
    after
        .iter()
        .filter_map(|(subject, facts)| {
            let before_len = before.get(subject).map_or(0, Vec::len);
            (facts.len() > before_len).then(|| subject.clone())
        })
        .collect()
}

fn sorted_fact_subjects_in_range(
    book: &ProjectBook,
    from_seq: u64,
    through_seq: u64,
) -> Vec<ControlId> {
    book.facts_through(through_seq)
        .filter(|fact| fact.seq > from_seq)
        .map(|fact| fact.subject.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
