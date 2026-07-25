//! Design-control records and graph-derived readiness.

use std::collections::{BTreeMap, BTreeSet};

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    AuthorityObligation, AuthorityObligationState, ConstructionProjectError, ControlEdgeKind,
    ControlExplanationPath, ControlGraph, ControlId, ControlNodeKind, DesignRelease,
    DesignReleasePurpose, DesignReview, DesignReviewState, EvidenceState, InspectionRecord,
    InspectionState, PermitRecord, PermitState, ProjectId, Result, RfiRecord, RfiState, RoleId,
};

/// Design deliverable revision with reference-only external artifacts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DesignRevision {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Revision control id.
    pub control: ControlId,
    /// Human-facing revision name.
    pub revision: String,
    /// Role responsible for the deliverable.
    pub responsible_role: RoleId,
    /// Date the revision is needed.
    pub need_date: Date,
    /// Package, task, or control ids affected by the revision.
    pub affected_control_ids: Vec<ControlId>,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// Prior revision superseded by this revision, when any.
    pub supersedes: Option<ControlId>,
    /// Reference-only design artifacts.
    pub external_refs: Vec<ExternalRef>,
}

impl DesignRevision {
    /// Builds a design revision.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        revision: impl Into<String>,
        responsible_role: RoleId,
        need_date: Date,
    ) -> Self {
        Self {
            project,
            control,
            revision: revision.into(),
            responsible_role,
            need_date,
            affected_control_ids: Vec::new(),
            evidence_state: EvidenceState::Reported,
            supersedes: None,
            external_refs: Vec::new(),
        }
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, evidence_state: EvidenceState) -> Self {
        self.evidence_state = evidence_state;
        self
    }

    /// Marks the revision superseded by this revision.
    #[must_use]
    pub fn supersedes(mut self, revision: ControlId) -> Self {
        self.supersedes = Some(revision);
        self
    }

    /// Adds an affected package, task, or control id.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Adds a reference-only design artifact.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}

/// One design readiness blocker.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DesignBlocker {
    /// Blocking control id.
    pub control: ControlId,
    /// Evidence state used by the blocker.
    pub evidence_state: EvidenceState,
    /// Deterministic blocker rule.
    pub rule: String,
    /// Deterministic blocker reason.
    pub reason: String,
    /// True when policy may not waive this blocker for production.
    pub non_waivable: bool,
    /// Stable graph paths from this blocker to the target.
    pub paths: Vec<ControlExplanationPath>,
}

/// Derived design readiness for a package, task, or control.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DesignReadinessReport {
    /// Target package, task, or control.
    pub target: ControlId,
    /// Required release purpose.
    pub required_purpose: DesignReleasePurpose,
    /// Evaluation date.
    pub as_of_date: Date,
    /// True when no design, release, RFI, permit, inspection, or authority blocker remains.
    pub ready: bool,
    /// Current design revisions affecting the target.
    pub current_revisions: Vec<ControlId>,
    /// Purpose-specific releases affecting the target.
    pub releases: Vec<ControlId>,
    /// Blockers in stable control-id order.
    pub blockers: Vec<DesignBlocker>,
}

/// Construction design-control records.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DesignControlSet {
    /// Design deliverable revisions.
    pub revisions: Vec<DesignRevision>,
    /// Design reviews and decisions.
    pub reviews: Vec<DesignReview>,
    /// RFIs.
    pub rfis: Vec<RfiRecord>,
    /// Purpose-specific releases.
    pub releases: Vec<DesignRelease>,
    /// Authority permits.
    pub permits: Vec<PermitRecord>,
    /// Authority inspections.
    pub inspections: Vec<InspectionRecord>,
    /// Authority obligations.
    pub authority_obligations: Vec<AuthorityObligation>,
}

impl DesignControlSet {
    /// Builds an empty design-control set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a design revision.
    #[must_use]
    pub fn with_revision(mut self, revision: DesignRevision) -> Self {
        self.revisions.push(revision);
        self
    }

    /// Adds a design review.
    #[must_use]
    pub fn with_review(mut self, review: DesignReview) -> Self {
        self.reviews.push(review);
        self
    }

    /// Adds an RFI.
    #[must_use]
    pub fn with_rfi(mut self, rfi: RfiRecord) -> Self {
        self.rfis.push(rfi);
        self
    }

    /// Adds a design release.
    #[must_use]
    pub fn with_release(mut self, release: DesignRelease) -> Self {
        self.releases.push(release);
        self
    }

    /// Adds a permit.
    #[must_use]
    pub fn with_permit(mut self, permit: PermitRecord) -> Self {
        self.permits.push(permit);
        self
    }

    /// Adds an inspection.
    #[must_use]
    pub fn with_inspection(mut self, inspection: InspectionRecord) -> Self {
        self.inspections.push(inspection);
        self
    }

    /// Adds an authority obligation.
    #[must_use]
    pub fn with_authority_obligation(mut self, obligation: AuthorityObligation) -> Self {
        self.authority_obligations.push(obligation);
        self
    }

    /// Derives design readiness for a package, task, or control through the shared graph.
    pub fn readiness_for(
        &self,
        target: ControlId,
        required_purpose: DesignReleasePurpose,
        as_of_date: Date,
    ) -> Result<DesignReadinessReport> {
        self.validate()?;
        let graph = self.graph_for_target(&target)?;
        let current_revisions = self.current_revisions_for(&target)?;
        if current_revisions.len() > 1 {
            return Err(ConstructionProjectError::ConflictingDesignRevisions {
                affected: target,
                revisions: current_revisions,
            });
        }

        let mut blockers = self.local_blockers(&required_purpose, as_of_date)?;
        let blocker_ids = blockers.keys().cloned().collect::<BTreeSet<_>>();
        let release_ids = self
            .releases
            .iter()
            .filter(|release| affects(&release.affected_control_ids, &target))
            .map(|release| release.control.clone())
            .collect::<Vec<_>>();

        let analysis = graph.analyze_target(
            &target,
            |control| blocker_ids.contains(control),
            |control| {
                blockers
                    .get(control)
                    .map(|blocker| (None, blocker.evidence_state))
                    .unwrap_or((None, EvidenceState::Accepted))
            },
            |_| None,
        )?;
        let paths_by_blocker = analysis
            .explanation_paths
            .into_iter()
            .map(|path| (path.blocker.clone(), path))
            .collect::<BTreeMap<_, _>>();

        let mut blocker_reports = blockers
            .remove_entry(&target)
            .into_iter()
            .map(|(_, blocker)| blocker)
            .chain(blocker_ids.iter().filter_map(|id| blockers.remove(id)))
            .filter(|blocker| {
                paths_by_blocker.contains_key(&blocker.control) || blocker.control == target
            })
            .map(|mut blocker| {
                if let Some(path) = paths_by_blocker.get(&blocker.control) {
                    blocker.paths = vec![path.clone()];
                }
                blocker
            })
            .collect::<Vec<_>>();
        blocker_reports.sort_by(|left, right| left.control.cmp(&right.control));

        if required_purpose == DesignReleasePurpose::Production
            && let Some(blocker) = blocker_reports.iter().find(|blocker| blocker.non_waivable)
        {
            return Err(ConstructionProjectError::NonWaivableProductionBlocker {
                target,
                blocker: blocker.control.clone(),
            });
        }

        Ok(DesignReadinessReport {
            target,
            required_purpose,
            as_of_date,
            ready: blocker_reports.is_empty(),
            current_revisions,
            releases: release_ids,
            blockers: blocker_reports,
        })
    }

    fn validate(&self) -> Result<()> {
        let revisions = self
            .revisions
            .iter()
            .map(|revision| revision.control.clone())
            .collect::<BTreeSet<_>>();
        for revision in &self.revisions {
            validate_common(
                "design_revision",
                &revision.revision,
                &revision.affected_control_ids,
                &revision.external_refs,
            )?;
            if let Some(superseded) = &revision.supersedes
                && !revisions.contains(superseded)
            {
                return Err(ConstructionProjectError::MissingDesignRevision {
                    kind: "design_revision",
                    control: revision.control.clone(),
                    revision: superseded.clone(),
                });
            }
        }
        for review in &self.reviews {
            validate_common(
                "design_review",
                "review",
                &review.affected_control_ids,
                &review.external_refs,
            )?;
            if !revisions.contains(&review.design_revision) {
                return Err(ConstructionProjectError::MissingDesignRevision {
                    kind: "design_review",
                    control: review.control.clone(),
                    revision: review.design_revision.clone(),
                });
            }
        }
        for rfi in &self.rfis {
            validate_common("rfi", "rfi", &rfi.affected_control_ids, &rfi.external_refs)?;
        }
        for release in &self.releases {
            validate_common(
                "design_release",
                &release.revision,
                &release.affected_control_ids,
                &release.external_refs,
            )?;
            if !revisions.contains(&release.design_revision) {
                return Err(ConstructionProjectError::MissingDesignRevision {
                    kind: "design_release",
                    control: release.control.clone(),
                    revision: release.design_revision.clone(),
                });
            }
            if release.released_by != release.release_authority {
                return Err(ConstructionProjectError::DesignReleaseAuthorityMismatch {
                    release: release.control.clone(),
                    expected: release.release_authority.clone(),
                    actual: release.released_by.clone(),
                });
            }
        }
        for permit in &self.permits {
            validate_common(
                "permit",
                "permit",
                &permit.affected_control_ids,
                &permit.external_refs,
            )?;
        }
        for inspection in &self.inspections {
            validate_common(
                "inspection",
                "inspection",
                &inspection.affected_control_ids,
                &inspection.external_refs,
            )?;
        }
        for obligation in &self.authority_obligations {
            validate_common(
                "authority_obligation",
                "authority obligation",
                &obligation.affected_control_ids,
                &obligation.external_refs,
            )?;
        }
        Ok(())
    }

    fn graph_for_target(&self, target: &ControlId) -> Result<ControlGraph> {
        let mut graph = ControlGraph::new();
        graph.add_node(target.clone(), ControlNodeKind::Package)?;
        for revision in &self.revisions {
            self.add_record_node(
                &mut graph,
                &revision.control,
                ControlNodeKind::DesignRevision,
                &revision.affected_control_ids,
            )?;
            if let Some(superseded) = &revision.supersedes {
                graph.add_edge(
                    revision.control.clone(),
                    superseded.clone(),
                    ControlEdgeKind::Changes,
                )?;
            }
        }
        for review in &self.reviews {
            self.add_record_node(
                &mut graph,
                &review.control,
                ControlNodeKind::DesignReview,
                &review.affected_control_ids,
            )?;
            graph.add_edge(
                review.control.clone(),
                review.design_revision.clone(),
                ControlEdgeKind::Decides,
            )?;
        }
        for rfi in &self.rfis {
            self.add_record_node(
                &mut graph,
                &rfi.control,
                ControlNodeKind::Rfi,
                &rfi.affected_control_ids,
            )?;
        }
        for release in &self.releases {
            self.add_record_node(
                &mut graph,
                &release.control,
                ControlNodeKind::DesignRelease,
                &release.affected_control_ids,
            )?;
            graph.add_edge(
                release.control.clone(),
                release.design_revision.clone(),
                ControlEdgeKind::Decides,
            )?;
        }
        for permit in &self.permits {
            self.add_record_node(
                &mut graph,
                &permit.control,
                ControlNodeKind::Permit,
                &permit.affected_control_ids,
            )?;
        }
        for inspection in &self.inspections {
            self.add_record_node(
                &mut graph,
                &inspection.control,
                ControlNodeKind::Inspection,
                &inspection.affected_control_ids,
            )?;
        }
        for obligation in &self.authority_obligations {
            self.add_record_node(
                &mut graph,
                &obligation.control,
                ControlNodeKind::AuthorityObligation,
                &obligation.affected_control_ids,
            )?;
        }
        Ok(graph)
    }

    fn add_record_node(
        &self,
        graph: &mut ControlGraph,
        control: &ControlId,
        kind: ControlNodeKind,
        affected: &[ControlId],
    ) -> Result<()> {
        graph.add_node(control.clone(), kind)?;
        for target in affected {
            if !graph.nodes.contains_key(target) {
                graph.add_node(target.clone(), ControlNodeKind::Package)?;
            }
            graph.add_edge(
                control.clone(),
                target.clone(),
                ControlEdgeKind::Prerequisite,
            )?;
        }
        Ok(())
    }

    fn current_revisions_for(&self, target: &ControlId) -> Result<Vec<ControlId>> {
        let superseded = self
            .revisions
            .iter()
            .filter_map(|revision| revision.supersedes.clone())
            .collect::<BTreeSet<_>>();
        let revisions = self
            .revisions
            .iter()
            .filter(|revision| {
                affects(&revision.affected_control_ids, target)
                    && !superseded.contains(&revision.control)
            })
            .map(|revision| revision.control.clone())
            .collect::<Vec<_>>();
        Ok(revisions)
    }

    fn local_blockers(
        &self,
        required_purpose: &DesignReleasePurpose,
        as_of_date: Date,
    ) -> Result<BTreeMap<ControlId, DesignBlocker>> {
        let superseded_by = self
            .revisions
            .iter()
            .filter_map(|revision| {
                revision
                    .supersedes
                    .as_ref()
                    .map(|superseded| (superseded.clone(), revision.control.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let mut blockers = BTreeMap::new();
        for revision in &self.revisions {
            if !revision.evidence_state.satisfies_required_evidence() {
                blockers.insert(
                    revision.control.clone(),
                    blocker(
                        &revision.control,
                        revision.evidence_state,
                        "design-revision",
                        "design revision evidence is not accepted",
                        false,
                    ),
                );
            }
        }
        for review in &self.reviews {
            if !matches!(review.state, DesignReviewState::Accepted) {
                blockers.insert(
                    review.control.clone(),
                    blocker(
                        &review.control,
                        review.evidence_state,
                        "design-review",
                        "design review is not accepted",
                        false,
                    ),
                );
            }
        }
        for rfi in &self.rfis {
            if rfi.state != RfiState::Accepted {
                let reason = if rfi.state == RfiState::Answered {
                    "RFI is answered but not accepted"
                } else {
                    "RFI is not accepted"
                };
                blockers.insert(
                    rfi.control.clone(),
                    blocker(&rfi.control, rfi.evidence_state, "rfi", reason, false),
                );
            }
        }
        for release in &self.releases {
            if &release.purpose != required_purpose {
                blockers.insert(
                    release.control.clone(),
                    blocker(
                        &release.control,
                        release.evidence_state,
                        "release-purpose",
                        "release purpose does not satisfy the requested readiness purpose",
                        false,
                    ),
                );
                continue;
            }
            if !release.evidence_state.satisfies_required_evidence() {
                blockers.insert(
                    release.control.clone(),
                    blocker(
                        &release.control,
                        release.evidence_state,
                        "design-release",
                        "release evidence is not accepted",
                        false,
                    ),
                );
            }
            if let Some(superseding) = superseded_by.get(&release.design_revision)
                && release.revalidated_against.as_ref() != Some(superseding)
            {
                return Err(ConstructionProjectError::StaleDesignRelease {
                    release: release.control.clone(),
                    revision: release.design_revision.clone(),
                    superseding: superseding.clone(),
                });
            }
        }
        for permit in &self.permits {
            let mut state = permit.evidence_state;
            if state.satisfies_required_evidence() && !permit.validity.contains(as_of_date) {
                state = EvidenceState::Expired;
            }
            if permit.state != PermitState::Granted || !state.satisfies_required_evidence() {
                let reason = if state == EvidenceState::Expired {
                    "permit is expired"
                } else if permit.state == PermitState::Hold {
                    "authority permit is on hold"
                } else {
                    "permit is not granted with accepted evidence"
                };
                blockers.insert(
                    permit.control.clone(),
                    blocker(&permit.control, state, "permit", reason, false),
                );
            }
        }
        for inspection in &self.inspections {
            if inspection.state != InspectionState::Passed
                || !inspection.evidence_state.satisfies_required_evidence()
            {
                let reason = if inspection.state == InspectionState::Hold {
                    "authority inspection is on hold"
                } else {
                    "inspection has not passed with accepted evidence"
                };
                blockers.insert(
                    inspection.control.clone(),
                    blocker(
                        &inspection.control,
                        inspection.evidence_state,
                        "inspection",
                        reason,
                        false,
                    ),
                );
            }
        }
        for obligation in &self.authority_obligations {
            if obligation.state != AuthorityObligationState::Satisfied
                || !obligation.evidence_state.satisfies_required_evidence()
            {
                let reason = if obligation.state == AuthorityObligationState::Hold {
                    "authority obligation is on hold"
                } else {
                    "authority obligation is not satisfied with accepted evidence"
                };
                blockers.insert(
                    obligation.control.clone(),
                    blocker(
                        &obligation.control,
                        obligation.evidence_state,
                        "authority-obligation",
                        reason,
                        obligation.non_waivable,
                    ),
                );
            }
        }
        Ok(blockers)
    }
}

fn validate_common(
    kind: &'static str,
    title: &str,
    affected: &[ControlId],
    refs: &[ExternalRef],
) -> Result<()> {
    if title.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField(kind));
    }
    if affected.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(
            "affected_control_ids",
        ));
    }
    if refs.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection("external_refs"));
    }
    Ok(())
}

fn affects(affected: &[ControlId], target: &ControlId) -> bool {
    affected.iter().any(|candidate| candidate == target)
}

fn blocker(
    control: &ControlId,
    evidence_state: EvidenceState,
    rule: impl Into<String>,
    reason: impl Into<String>,
    non_waivable: bool,
) -> DesignBlocker {
    DesignBlocker {
        control: control.clone(),
        evidence_state,
        rule: rule.into(),
        reason: reason.into(),
        non_waivable,
        paths: Vec::new(),
    }
}
