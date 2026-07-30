//! Baseline-aware construction uncertainty exposure derivation.

use std::collections::{BTreeMap, BTreeSet};

use sim_ledger::Amount;

use crate::{
    AcceptedBaseline, BaselineKind, ConstructionProjectError, ControlGraph, ControlId,
    CurrencyCode, EvidenceState, ForecastConsequence, ForecastValue, ProjectSnapshot, Result,
    ScheduleStatusReport, ScheduleTaskJoinSet, UncertaintyKind, UncertaintyRecord,
    UncertaintyState,
};

/// Correlation or hierarchy note retained beside an exact exposure total.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExposureAnnotation {
    /// Current child facts replaced a parent summary to prevent double counting.
    ParentSummaryExcluded {
        /// Excluded parent summary.
        parent: ControlId,
        /// Current children used instead.
        children: Vec<ControlId>,
    },
    /// Consequences share a correlation group that requires expert interpretation.
    Correlated {
        /// Open correlation-group id.
        group: ControlId,
        /// Correlated consequence facts in this bucket.
        consequences: Vec<ControlId>,
    },
}

/// Comparable exact-amount exposure bucket.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExposureBucket {
    /// Risk and opportunity amounts remain separate.
    pub kind: UncertaintyKind,
    /// Scenario remains separate.
    pub scenario: ControlId,
    /// Currency remains separate.
    pub currency: CurrencyCode,
    /// Checked sum of current, comparable leaf consequences.
    #[serde(with = "crate::work_package::amount_serde")]
    pub total: Amount,
    /// Consequence facts included exactly once.
    pub contributors: Vec<ControlId>,
    /// Hierarchy and correlation annotations.
    pub annotations: Vec<ExposureAnnotation>,
}

/// One current uncertainty in the deterministic exposure queue.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExposureQueueItem {
    /// Risk or opportunity control.
    pub uncertainty: ControlId,
    /// Risk or opportunity kind.
    pub kind: UncertaintyKind,
    /// Current lifecycle state.
    pub state: UncertaintyState,
    /// Accepted baseline used by the uncertainty.
    pub baseline: crate::BaselineId,
    /// Whether at least one rating predates the current fact.
    pub stale_rating: bool,
    /// Current forecast-consequence facts.
    pub consequences: Vec<ControlId>,
    /// Downstream controls reached through the canonical control graph.
    pub affected_dependents: Vec<ControlId>,
    /// True when a consequence affects a joined critical-path control.
    pub critical_path: bool,
}

/// Derived current exposure report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExposureReport {
    /// Project sequence used for all current-fact checks.
    pub as_of_seq: u64,
    /// Accepted schedule baseline used for critical-path joins.
    pub schedule_baseline: crate::BaselineId,
    /// Current risk and opportunity queue.
    pub queue: Vec<ExposureQueueItem>,
    /// Exact comparable amount buckets.
    pub amount_buckets: Vec<ExposureBucket>,
}

/// Derives current exposure from accepted facts, baselines, schedule joins, and dependencies.
pub fn derive_exposure(
    snapshot: &ProjectSnapshot,
    baselines: &[AcceptedBaseline],
    uncertainties: &[UncertaintyRecord],
    consequences: &[ForecastConsequence],
    schedule_joins: &ScheduleTaskJoinSet,
    schedule: &ScheduleStatusReport,
    graph: &ControlGraph,
) -> Result<ExposureReport> {
    let baselines = baseline_map(snapshot, baselines)?;
    validate_schedule(snapshot, &baselines, schedule_joins, schedule)?;

    let mut uncertainty_by_id = BTreeMap::new();
    for uncertainty in uncertainties {
        uncertainty.validate()?;
        validate_project(snapshot, &uncertainty.project, &uncertainty.control)?;
        validate_current(snapshot, &uncertainty.control, uncertainty.fact_seq)?;
        validate_baseline(
            &baselines,
            &uncertainty.baseline,
            uncertainty.fact_seq,
            &uncertainty.control,
        )?;
        if uncertainty_by_id
            .insert(uncertainty.control.clone(), uncertainty)
            .is_some()
        {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "uncertainty",
                id: uncertainty.control.as_str().to_owned(),
            });
        }
    }

    let mut consequence_by_id = BTreeMap::new();
    let mut consequence_by_uncertainty = BTreeMap::<ControlId, Vec<&ForecastConsequence>>::new();
    for consequence in consequences {
        consequence.validate()?;
        validate_project(snapshot, &consequence.project, &consequence.control)?;
        validate_current(snapshot, &consequence.control, consequence.fact_seq)?;
        let uncertainty = uncertainty_by_id
            .get(&consequence.uncertainty)
            .ok_or_else(|| ConstructionProjectError::UncertaintyDerivation {
                control: consequence.control.clone(),
                reason: "forecast consequence references a non-current uncertainty",
            })?;
        if consequence.scenario != uncertainty.scenario {
            return Err(ConstructionProjectError::UncertaintyDerivation {
                control: consequence.control.clone(),
                reason: "forecast consequence scenario differs from its uncertainty",
            });
        }
        if consequence.basis.baseline != uncertainty.baseline {
            return Err(ConstructionProjectError::UncertaintyDerivation {
                control: consequence.control.clone(),
                reason: "forecast consequence baseline differs from its uncertainty",
            });
        }
        validate_baseline(
            &baselines,
            &consequence.basis.baseline,
            consequence.basis.as_of_seq,
            &consequence.control,
        )?;
        if consequence_by_id
            .insert(consequence.control.clone(), consequence)
            .is_some()
        {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "forecast_consequence",
                id: consequence.control.as_str().to_owned(),
            });
        }
        consequence_by_uncertainty
            .entry(consequence.uncertainty.clone())
            .or_default()
            .push(consequence);
    }
    validate_hierarchy(&consequence_by_id)?;

    let critical_controls = schedule_joins
        .joins
        .iter()
        .filter(|join| schedule.critical_tasks.contains(&join.task_id))
        .map(|join| join.control.clone())
        .collect::<BTreeSet<_>>();
    let mut queue = Vec::new();
    for uncertainty in uncertainties {
        let mut current_consequences = consequence_by_uncertainty
            .get(&uncertainty.control)
            .cloned()
            .unwrap_or_default();
        current_consequences.sort_by(|left, right| left.control.cmp(&right.control));
        let analysis = graph.analyze_target(
            &uncertainty.control,
            |_| false,
            |control| {
                snapshot
                    .current_fact(control)
                    .map_or((None, EvidenceState::Missing), |fact| {
                        (Some(fact.seq), fact.evidence_state)
                    })
            },
            |_| None,
        )?;
        let critical_path = current_consequences.iter().any(|consequence| {
            consequence
                .affected_control_ids
                .iter()
                .any(|control| critical_controls.contains(control))
        });
        queue.push(ExposureQueueItem {
            uncertainty: uncertainty.control.clone(),
            kind: uncertainty.kind,
            state: uncertainty.state,
            baseline: uncertainty.baseline.clone(),
            stale_rating: uncertainty.has_stale_rating(),
            consequences: current_consequences
                .iter()
                .map(|consequence| consequence.control.clone())
                .collect(),
            affected_dependents: analysis.affected_dependents,
            critical_path,
        });
    }
    queue.sort_by(|left, right| {
        (!left.critical_path)
            .cmp(&(!right.critical_path))
            .then((!left.stale_rating).cmp(&(!right.stale_rating)))
            .then(left.uncertainty.cmp(&right.uncertainty))
    });

    let amount_buckets = amount_buckets(&uncertainty_by_id, &consequence_by_id)?;
    Ok(ExposureReport {
        as_of_seq: snapshot.through_seq,
        schedule_baseline: schedule.baseline.clone(),
        queue,
        amount_buckets,
    })
}

fn baseline_map<'a>(
    snapshot: &ProjectSnapshot,
    baselines: &'a [AcceptedBaseline],
) -> Result<BTreeMap<crate::BaselineId, &'a AcceptedBaseline>> {
    let mut by_id = BTreeMap::new();
    for baseline in baselines {
        baseline.validate()?;
        validate_project(snapshot, &baseline.project, &baseline.control)?;
        if by_id.insert(baseline.id.clone(), baseline).is_some() {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "accepted_baseline",
                id: baseline.id.as_str().to_owned(),
            });
        }
    }
    Ok(by_id)
}

fn validate_schedule(
    snapshot: &ProjectSnapshot,
    baselines: &BTreeMap<crate::BaselineId, &AcceptedBaseline>,
    joins: &ScheduleTaskJoinSet,
    schedule: &ScheduleStatusReport,
) -> Result<()> {
    let accepted = baselines.get(&schedule.baseline).ok_or_else(|| {
        ConstructionProjectError::UncertaintyDerivation {
            control: ControlId::new(schedule.baseline.as_str())
                .expect("baseline ids are valid control ids"),
            reason: "schedule baseline is not accepted",
        }
    })?;
    if accepted.kind != BaselineKind::Time
        || joins.baseline.baseline != schedule.baseline
        || joins.baseline.accepted_revision != schedule.accepted_revision
        || joins.revision.as_of_seq != schedule.as_of_seq
        || schedule.as_of_seq != snapshot.through_seq
        || joins.baseline.accepted_seq != accepted.accepted_seq
    {
        return Err(ConstructionProjectError::UncertaintyDerivation {
            control: accepted.control.clone(),
            reason: "schedule report and accepted join baseline do not match",
        });
    }
    Ok(())
}

fn validate_project(
    snapshot: &ProjectSnapshot,
    project: &crate::ProjectId,
    control: &ControlId,
) -> Result<()> {
    if project != &snapshot.project {
        return Err(ConstructionProjectError::UncertaintyDerivation {
            control: control.clone(),
            reason: "record belongs to a different project",
        });
    }
    Ok(())
}

fn validate_current(snapshot: &ProjectSnapshot, control: &ControlId, fact_seq: u64) -> Result<()> {
    let fact = snapshot.current_fact(control).ok_or_else(|| {
        ConstructionProjectError::OrphanControlRef {
            control: control.clone(),
            as_of_seq: snapshot.through_seq,
        }
    })?;
    if fact.seq != fact_seq {
        return Err(ConstructionProjectError::UncertaintyDerivation {
            control: control.clone(),
            reason: "structured record does not match the current project fact",
        });
    }
    Ok(())
}

fn validate_baseline(
    baselines: &BTreeMap<crate::BaselineId, &AcceptedBaseline>,
    baseline: &crate::BaselineId,
    as_of_seq: u64,
    control: &ControlId,
) -> Result<()> {
    let accepted =
        baselines
            .get(baseline)
            .ok_or_else(|| ConstructionProjectError::UncertaintyDerivation {
                control: control.clone(),
                reason: "record references a baseline that is not accepted",
            })?;
    if as_of_seq < accepted.accepted_seq {
        return Err(ConstructionProjectError::StaleBaselineComparison {
            baseline: accepted.control.clone(),
            accepted_seq: accepted.accepted_seq,
            as_of_seq,
        });
    }
    Ok(())
}

fn validate_hierarchy(consequences: &BTreeMap<ControlId, &ForecastConsequence>) -> Result<()> {
    for consequence in consequences.values() {
        if let Some(parent) = &consequence.parent {
            let parent = consequences.get(parent).ok_or_else(|| {
                ConstructionProjectError::UncertaintyDerivation {
                    control: consequence.control.clone(),
                    reason: "forecast consequence references a non-current parent",
                }
            })?;
            if !parent.summarizes.contains(&consequence.control) {
                return Err(ConstructionProjectError::UncertaintyDerivation {
                    control: consequence.control.clone(),
                    reason: "forecast parent and child links are not reciprocal",
                });
            }
            validate_parent_child(parent, consequence)?;
        }
        for child in &consequence.summarizes {
            let child = consequences.get(child).ok_or_else(|| {
                ConstructionProjectError::UncertaintyDerivation {
                    control: consequence.control.clone(),
                    reason: "forecast summary references a non-current child",
                }
            })?;
            if child.parent.as_ref() != Some(&consequence.control) {
                return Err(ConstructionProjectError::UncertaintyDerivation {
                    control: consequence.control.clone(),
                    reason: "forecast summary and child links are not reciprocal",
                });
            }
            validate_parent_child(consequence, child)?;
        }
    }
    Ok(())
}

fn validate_parent_child(parent: &ForecastConsequence, child: &ForecastConsequence) -> Result<()> {
    let amount_currency_matches = match (&parent.value, &child.value) {
        (ForecastValue::Amount(parent), ForecastValue::Amount(child)) => {
            parent.currency == child.currency
        }
        (ForecastValue::Amount(_), _) | (_, ForecastValue::Amount(_)) => false,
        _ => true,
    };
    if parent.uncertainty != child.uncertainty
        || parent.scenario != child.scenario
        || parent.kind != child.kind
        || !amount_currency_matches
    {
        return Err(ConstructionProjectError::UncertaintyDerivation {
            control: child.control.clone(),
            reason: "forecast parent and child are not comparable",
        });
    }
    Ok(())
}

type BucketKey = (UncertaintyKind, ControlId, CurrencyCode);

fn amount_buckets(
    uncertainties: &BTreeMap<ControlId, &UncertaintyRecord>,
    consequences: &BTreeMap<ControlId, &ForecastConsequence>,
) -> Result<Vec<ExposureBucket>> {
    let excluded_parents = consequences
        .values()
        .filter(|consequence| !consequence.summarizes.is_empty())
        .map(|consequence| consequence.control.clone())
        .collect::<BTreeSet<_>>();
    let mut grouped = BTreeMap::<BucketKey, Vec<&ForecastConsequence>>::new();
    for consequence in consequences.values() {
        if excluded_parents.contains(&consequence.control) {
            continue;
        }
        let ForecastValue::Amount(amount) = &consequence.value else {
            continue;
        };
        let uncertainty = uncertainties[&consequence.uncertainty];
        grouped
            .entry((
                uncertainty.kind,
                consequence.scenario.clone(),
                amount.currency.clone(),
            ))
            .or_default()
            .push(consequence);
    }

    let mut buckets = Vec::new();
    for ((kind, scenario, currency), mut current) in grouped {
        current.sort_by(|left, right| left.control.cmp(&right.control));
        let mut total = 0_i64;
        let mut correlations = BTreeMap::<ControlId, Vec<ControlId>>::new();
        for consequence in &current {
            let ForecastValue::Amount(amount) = &consequence.value else {
                unreachable!("amount bucket contains only amount consequences");
            };
            total = total.checked_add(amount.amount.0).ok_or(
                ConstructionProjectError::AmountOverflow {
                    field: "forecast.exposure.total",
                },
            )?;
            if let Some(group) = &consequence.correlation {
                correlations
                    .entry(group.clone())
                    .or_default()
                    .push(consequence.control.clone());
            }
        }
        let mut annotations = consequences
            .values()
            .filter(|parent| {
                excluded_parents.contains(&parent.control)
                    && parent.scenario == scenario
                    && uncertainties[&parent.uncertainty].kind == kind
                    && matches!(
                        &parent.value,
                        ForecastValue::Amount(amount) if amount.currency == currency
                    )
            })
            .map(|parent| ExposureAnnotation::ParentSummaryExcluded {
                parent: parent.control.clone(),
                children: parent.summarizes.clone(),
            })
            .collect::<Vec<_>>();
        annotations.extend(correlations.into_iter().map(|(group, mut consequences)| {
            consequences.sort();
            ExposureAnnotation::Correlated {
                group,
                consequences,
            }
        }));
        buckets.push(ExposureBucket {
            kind,
            scenario,
            currency,
            total: Amount(total),
            contributors: current
                .iter()
                .map(|consequence| consequence.control.clone())
                .collect(),
            annotations,
        });
    }
    Ok(buckets)
}
