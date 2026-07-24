//! In-memory append-only construction project books.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ConstructionProjectError, ProjectFact, ProjectId, Result, RoleId, snapshot_at, snapshot_delta,
};

/// Default maximum facts accepted by an in-memory project book.
pub const DEFAULT_MAX_PROJECT_FACTS: usize = 10_000;

/// Append-only project-control fact book with one authoritative writer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectBook {
    project: ProjectId,
    writer: RoleId,
    max_facts: usize,
    facts: BTreeMap<u64, ProjectFact>,
    superseded_by: BTreeMap<u64, u64>,
}

impl ProjectBook {
    /// Starts an empty project book.
    #[must_use]
    pub fn new(project: ProjectId, writer: RoleId) -> Self {
        Self {
            project,
            writer,
            max_facts: DEFAULT_MAX_PROJECT_FACTS,
            facts: BTreeMap::new(),
            superseded_by: BTreeMap::new(),
        }
    }

    /// Builds a project book with a custom fact count bound.
    #[must_use]
    pub fn with_max_facts(mut self, max_facts: usize) -> Self {
        self.max_facts = max_facts;
        self
    }

    /// Replays facts into a deterministic project book independent of input order.
    pub fn from_facts(
        project: ProjectId,
        writer: RoleId,
        facts: impl IntoIterator<Item = ProjectFact>,
    ) -> Result<Self> {
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::new();
        for fact in facts {
            if !seen.insert(fact.seq) {
                return Err(ConstructionProjectError::DuplicateSequence { sequence: fact.seq });
            }
            ordered.push(fact);
        }
        ordered.sort_by_key(|fact| fact.seq);

        let mut book = Self::new(project, writer);
        for fact in ordered {
            book.append(fact)?;
        }
        Ok(book)
    }

    /// Returns the project owned by this book.
    #[must_use]
    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    /// Returns the authoritative writer role.
    #[must_use]
    pub fn authoritative_writer(&self) -> &RoleId {
        &self.writer
    }

    /// Returns the configured fact count bound.
    #[must_use]
    pub fn max_facts(&self) -> usize {
        self.max_facts
    }

    /// Returns the fact count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// Returns true when the book has no facts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Returns the highest appended sequence.
    #[must_use]
    pub fn last_sequence(&self) -> Option<u64> {
        self.facts.keys().next_back().copied()
    }

    /// Returns facts in sequence order.
    pub fn facts(&self) -> impl Iterator<Item = &ProjectFact> {
        self.facts.values()
    }

    /// Returns one fact by sequence.
    #[must_use]
    pub fn fact(&self, sequence: u64) -> Option<&ProjectFact> {
        self.facts.get(&sequence)
    }

    /// Appends one fact after checking the project, writer, sequence, and supersession.
    pub fn append(&mut self, fact: ProjectFact) -> Result<()> {
        self.validate_append(&fact)?;
        if let Some(supersedes) = fact.supersedes {
            self.superseded_by.insert(supersedes, fact.seq);
        }
        self.facts.insert(fact.seq, fact);
        Ok(())
    }

    /// Builds the deterministic snapshot at `through`.
    pub fn snapshot_at(&self, through: u64) -> Result<crate::ProjectSnapshot> {
        snapshot_at(self, through)
    }

    /// Reports changed subjects between two snapshot sequences.
    pub fn delta(&self, from_seq: u64, through_seq: u64) -> Result<crate::ProjectDelta> {
        snapshot_delta(self, from_seq, through_seq)
    }

    pub(crate) fn facts_through(&self, through: u64) -> impl Iterator<Item = &ProjectFact> {
        self.facts.range(..=through).map(|(_, fact)| fact)
    }

    fn validate_append(&self, fact: &ProjectFact) -> Result<()> {
        if self.facts.len() >= self.max_facts {
            return Err(ConstructionProjectError::FactLimitExceeded {
                max: self.max_facts,
            });
        }
        fact.validate_bounds()?;
        if fact.project != self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: fact.project.clone(),
            });
        }
        if fact.actor_role != self.writer {
            return Err(ConstructionProjectError::WriterMismatch {
                expected: self.writer.clone(),
                actual: fact.actor_role.clone(),
            });
        }
        if self.facts.contains_key(&fact.seq) {
            return Err(ConstructionProjectError::DuplicateSequence { sequence: fact.seq });
        }
        if let Some(last_sequence) = self.last_sequence()
            && fact.seq <= last_sequence
        {
            return Err(ConstructionProjectError::OutOfOrderSequence {
                last_sequence,
                next_sequence: fact.seq,
            });
        }
        self.validate_supersession(fact)?;
        Ok(())
    }

    fn validate_supersession(&self, fact: &ProjectFact) -> Result<()> {
        let Some(supersedes) = fact.supersedes else {
            return Ok(());
        };
        if supersedes >= fact.seq {
            return Err(ConstructionProjectError::InvalidSupersession {
                sequence: fact.seq,
                supersedes,
                reason: "supersession must point backward",
            });
        }
        let Some(prior) = self.facts.get(&supersedes) else {
            return Err(ConstructionProjectError::MissingSupersededFact {
                sequence: fact.seq,
                supersedes,
            });
        };
        if prior.project != fact.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: prior.project.clone(),
                actual: fact.project.clone(),
            });
        }
        if prior.subject != fact.subject {
            return Err(ConstructionProjectError::SupersessionSubjectMismatch {
                sequence: fact.seq,
                supersedes,
                expected: prior.subject.clone(),
                actual: fact.subject.clone(),
            });
        }
        if let Some(existing) = self.superseded_by.get(&supersedes) {
            return Err(ConstructionProjectError::SupersessionFork {
                supersedes,
                existing: *existing,
                attempted: fact.seq,
            });
        }
        Ok(())
    }
}
