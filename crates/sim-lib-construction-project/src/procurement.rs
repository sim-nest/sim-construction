//! Work-package procurement comparison and award control.

use std::collections::{BTreeMap, BTreeSet};

use sim_ledger::Amount;
use time::Date;

use crate::{
    AwardDecision, AwardDecisionKind, CommercialAmount, ConstructionProjectError, ControlId,
    CurrencyCode, Result, TenderComparison, WorkPackage,
};
/// Derived comparable tender evaluation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TenderEvaluation {
    /// Tender control id.
    pub tender: ControlId,
    /// Supplier candidate.
    pub supplier: String,
    /// Commercial amount.
    pub commercial_amount: CommercialAmount,
    /// Signed variance to the package target amount.
    #[serde(with = "crate::work_package::amount_serde")]
    pub variance_to_target: Amount,
    /// Lead time in calendar days.
    pub lead_time_days: u16,
}

/// Derived procurement comparison.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProcurementComparison {
    /// Work package being compared.
    pub package: ControlId,
    /// Comparable tenders in deterministic supplier order.
    pub comparable: Vec<TenderEvaluation>,
    /// Tender facts preserved but excluded from comparison.
    pub non_comparable: Vec<ControlId>,
    /// Corrected tender facts that remain in the record but not the current comparison.
    pub corrected: Vec<ControlId>,
}

/// Derived package procurement status.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProcurementStatus {
    /// Inquiry basis is not complete enough to send.
    InquiryNotReady,
    /// Inquiry can be issued.
    InquiryReady,
    /// Comparable tenders exist but no award has been made.
    AwardReady,
    /// Package has an accountable award.
    Awarded {
        /// Selected supplier.
        supplier: String,
        /// Selected tender.
        tender: ControlId,
    },
}

/// Derived date consequence report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProcurementDateReport {
    /// True when the inquiry decision is overdue.
    pub inquiry_overdue: bool,
    /// True when the award decision is overdue.
    pub award_overdue: bool,
    /// True when no valid award exists by the package need date.
    pub need_date_exposed: bool,
}

/// Derived schedule consequence for a package.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AwardConsequence {
    /// Design-side consequence summary.
    pub design: String,
    /// Production-side consequence summary.
    pub production: String,
}

/// Exposed interface carried by the package.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InterfaceExposure {
    /// Interface control id.
    pub interface: ControlId,
    /// True when the package has not yet reached a valid award.
    pub exposed: bool,
}

/// Derived package readiness and consequence report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageReadinessReport {
    /// Work package being reported.
    pub package: ControlId,
    /// Evaluation date.
    pub as_of_date: Date,
    /// Procurement status.
    pub status: ProcurementStatus,
    /// Tender comparison.
    pub comparison: ProcurementComparison,
    /// Date consequences.
    pub dates: ProcurementDateReport,
    /// Schedule consequences.
    pub consequence: AwardConsequence,
    /// Exposed package interfaces.
    pub interfaces: Vec<InterfaceExposure>,
}

/// Work-package procurement facts and award decisions.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProcurementControlSet {
    /// Tender comparison facts.
    pub tenders: Vec<TenderComparison>,
    /// Accountable award decisions.
    pub awards: Vec<AwardDecision>,
}

impl ProcurementControlSet {
    /// Builds an empty procurement control set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a tender comparison fact.
    #[must_use]
    pub fn with_tender(mut self, tender: TenderComparison) -> Self {
        self.tenders.push(tender);
        self
    }

    /// Adds an award decision.
    #[must_use]
    pub fn with_award(mut self, award: AwardDecision) -> Self {
        self.awards.push(award);
        self
    }

    /// Derives readiness, comparison, schedule consequence, and interface exposure.
    pub fn readiness_for(
        &self,
        package: &WorkPackage,
        charter_currency: &CurrencyCode,
        as_of_date: Date,
    ) -> Result<PackageReadinessReport> {
        package.validate(charter_currency)?;
        let comparison = self.compare(package, charter_currency)?;
        let award = self.valid_award(package, &comparison)?;

        let status = if let Some((award, tender)) = &award {
            ProcurementStatus::Awarded {
                supplier: tender.supplier.clone(),
                tender: award
                    .selected_tender
                    .clone()
                    .expect("validated selected tender"),
            }
        } else if !comparison.comparable.is_empty() {
            ProcurementStatus::AwardReady
        } else if package.evidence.is_empty() || package.design_inputs.is_empty() {
            ProcurementStatus::InquiryNotReady
        } else {
            ProcurementStatus::InquiryReady
        };

        let dates = ProcurementDateReport {
            inquiry_overdue: as_of_date > package.inquiry_due_on
                && matches!(status, ProcurementStatus::InquiryNotReady),
            award_overdue: as_of_date > package.award_due_on
                && !matches!(status, ProcurementStatus::Awarded { .. }),
            need_date_exposed: as_of_date >= package.need_on
                && !matches!(status, ProcurementStatus::Awarded { .. }),
        };
        let consequence = consequence_for(package, &status, &dates);
        let interfaces = package
            .interfaces
            .iter()
            .cloned()
            .map(|interface| InterfaceExposure {
                interface,
                exposed: !matches!(status, ProcurementStatus::Awarded { .. }),
            })
            .collect();

        Ok(PackageReadinessReport {
            package: package.control.clone(),
            as_of_date,
            status,
            comparison,
            dates,
            consequence,
            interfaces,
        })
    }

    /// Builds the current tender comparison while preserving rejected and corrected facts.
    pub fn compare(
        &self,
        package: &WorkPackage,
        charter_currency: &CurrencyCode,
    ) -> Result<ProcurementComparison> {
        package.validate(charter_currency)?;
        let mut tender_by_id = BTreeMap::new();
        for tender in &self.tenders {
            validate_tender(tender, package, charter_currency)?;
            if tender_by_id
                .insert(tender.control.clone(), tender)
                .is_some()
            {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "tender",
                    id: tender.control.as_str().to_owned(),
                });
            }
        }

        let mut corrected = BTreeSet::new();
        for tender in &self.tenders {
            if let Some(supersedes) = &tender.supersedes {
                let prior = tender_by_id.get(supersedes).ok_or_else(|| {
                    ConstructionProjectError::MissingSupersededTender {
                        tender: tender.control.clone(),
                        supersedes: supersedes.clone(),
                    }
                })?;
                if prior.supplier != tender.supplier {
                    return Err(
                        ConstructionProjectError::TenderSupersessionSupplierMismatch {
                            tender: tender.control.clone(),
                            supersedes: supersedes.clone(),
                        },
                    );
                }
                corrected.insert(supersedes.clone());
            }
        }

        let mut comparable = Vec::new();
        let mut non_comparable = Vec::new();
        for tender in &self.tenders {
            if corrected.contains(&tender.control) {
                continue;
            }
            if tender.is_comparable() {
                comparable.push(TenderEvaluation {
                    tender: tender.control.clone(),
                    supplier: tender.supplier.clone(),
                    commercial_amount: tender.commercial_amount.clone(),
                    variance_to_target: tender.commercial_amount.checked_difference(
                        &package.target_amount,
                        "tender.commercial_amount",
                        charter_currency,
                    )?,
                    lead_time_days: tender.lead_time_days,
                });
            } else {
                non_comparable.push(tender.control.clone());
            }
        }
        comparable.sort_by(|left, right| {
            left.commercial_amount
                .amount
                .cmp(&right.commercial_amount.amount)
                .then_with(|| left.supplier.cmp(&right.supplier))
                .then_with(|| left.tender.cmp(&right.tender))
        });
        non_comparable.sort();
        let mut corrected = corrected.into_iter().collect::<Vec<_>>();
        corrected.sort();

        Ok(ProcurementComparison {
            package: package.control.clone(),
            comparable,
            non_comparable,
            corrected,
        })
    }

    fn valid_award<'a>(
        &'a self,
        package: &WorkPackage,
        comparison: &'a ProcurementComparison,
    ) -> Result<Option<(&'a AwardDecision, &'a TenderComparison)>> {
        let comparable = comparison
            .comparable
            .iter()
            .map(|evaluation| evaluation.tender.clone())
            .collect::<BTreeSet<_>>();
        let tender_by_id = self
            .tenders
            .iter()
            .map(|tender| (tender.control.clone(), tender))
            .collect::<BTreeMap<_, _>>();
        let mut latest_award = None;
        for award in &self.awards {
            validate_award(award, package, &tender_by_id, &comparable)?;
            if award.kind == AwardDecisionKind::Award {
                let tender = tender_by_id
                    .get(
                        award
                            .selected_tender
                            .as_ref()
                            .expect("validated selected tender"),
                    )
                    .expect("validated tender presence");
                latest_award = Some((award, *tender));
            }
        }
        Ok(latest_award)
    }
}

fn validate_tender(
    tender: &TenderComparison,
    package: &WorkPackage,
    charter_currency: &CurrencyCode,
) -> Result<()> {
    if tender.package != package.control {
        return Err(ConstructionProjectError::TenderPackageMismatch {
            expected: package.control.clone(),
            actual: tender.package.clone(),
            tender: tender.control.clone(),
        });
    }
    if package.candidate(&tender.supplier).is_none() {
        return Err(ConstructionProjectError::UnknownTenderSupplier {
            tender: tender.control.clone(),
            supplier: tender.supplier.clone(),
        });
    }
    tender
        .commercial_amount
        .validate_currency("tender.commercial_amount", charter_currency)?;
    if tender.evidence.is_empty() {
        return Err(ConstructionProjectError::NonComparableTender {
            tender: tender.control.clone(),
            reason: "missing evaluation evidence",
        });
    }
    Ok(())
}

fn validate_award(
    award: &AwardDecision,
    package: &WorkPackage,
    tender_by_id: &BTreeMap<ControlId, &TenderComparison>,
    comparable: &BTreeSet<ControlId>,
) -> Result<()> {
    if award.package != package.control {
        return Err(ConstructionProjectError::TenderPackageMismatch {
            expected: package.control.clone(),
            actual: award.package.clone(),
            tender: award.control.clone(),
        });
    }
    if award.decided_by != package.award_authority {
        return Err(ConstructionProjectError::AwardAuthorityMismatch {
            award: award.control.clone(),
            expected: package.award_authority.clone(),
            actual: award.decided_by.clone(),
        });
    }
    if award.decided_on > package.need_on {
        return Err(ConstructionProjectError::AwardAfterNeedDate {
            award: award.control.clone(),
            decided_on: award.decided_on,
            need_date: package.need_on,
        });
    }
    if award.rationale.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField("award.rationale"));
    }
    if award.evidence.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection("award.evidence"));
    }
    if award.kind == AwardDecisionKind::Award {
        let selected =
            award
                .selected_tender
                .as_ref()
                .ok_or(ConstructionProjectError::EmptyField(
                    "award.selected_tender",
                ))?;
        let tender = tender_by_id.get(selected).ok_or_else(|| {
            ConstructionProjectError::MissingAwardTender {
                award: award.control.clone(),
                tender: selected.clone(),
            }
        })?;
        if !comparable.contains(selected) {
            return Err(ConstructionProjectError::AwardTenderNotComparable {
                award: award.control.clone(),
                tender: selected.clone(),
            });
        }
        let candidate = package
            .candidate(&tender.supplier)
            .expect("validated tender supplier candidate");
        if !candidate.awardable {
            return Err(ConstructionProjectError::RejectedSupplierAward {
                award: award.control.clone(),
                supplier: tender.supplier.clone(),
            });
        }
    }
    Ok(())
}

fn consequence_for(
    package: &WorkPackage,
    status: &ProcurementStatus,
    dates: &ProcurementDateReport,
) -> AwardConsequence {
    if matches!(status, ProcurementStatus::Awarded { .. }) {
        return AwardConsequence {
            design: "award closes procurement basis for design coordination".to_owned(),
            production: "production can plan against the awarded supplier commitment".to_owned(),
        };
    }
    let design = if dates.inquiry_overdue {
        format!(
            "inquiry basis for {} is late and design clarifications remain exposed",
            package.control
        )
    } else {
        format!(
            "design inputs for {} must remain current until award",
            package.control
        )
    };
    let production = if dates.need_date_exposed {
        format!(
            "need date {} is exposed because no accountable award exists",
            package.need_on
        )
    } else if dates.award_overdue {
        format!(
            "award date {} is overdue and production float is consumed",
            package.award_due_on
        )
    } else {
        "production remains dependent on a future award decision".to_owned()
    };
    AwardConsequence { design, production }
}
