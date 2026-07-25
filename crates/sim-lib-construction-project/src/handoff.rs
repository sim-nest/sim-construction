//! Package handoff readiness from award, supplier, design, material, and acceptance facts.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, DesignReadinessReport, EvidenceState, OrganizationId,
    PackageReadinessReport, ProcurementStatus, QualificationStatus, Result, RoleId,
    SupplierQualificationReport,
};

/// Package handoff joining award, supplier, released design, material, need, and responsibility.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageHandoff {
    /// Stable handoff control id.
    pub control: ControlId,
    /// Work package being handed off.
    pub package: ControlId,
    /// Accountable award decision.
    pub award: ControlId,
    /// Supplier organization accepting responsibility.
    pub supplier: OrganizationId,
    /// Released design control used for production.
    pub released_design: ControlId,
    /// Material readiness control or evidence reference.
    pub material: ControlId,
    /// Supplier lead time in calendar days.
    pub lead_time_days: u16,
    /// Date production needs the package ready.
    pub production_need_on: Date,
    /// Role accepting package responsibility.
    pub accepted_by: RoleId,
    /// Date responsibility was accepted.
    pub accepted_on: Option<Date>,
    /// Explicit responsibility acceptance statement.
    pub responsibility_acceptance: String,
    /// Reference-only handoff evidence.
    pub evidence: Vec<ExternalRef>,
}

impl PackageHandoff {
    /// Builds a package handoff record.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        control: ControlId,
        package: ControlId,
        award: ControlId,
        supplier: OrganizationId,
        released_design: ControlId,
        material: ControlId,
        lead_time_days: u16,
        production_need_on: Date,
        accepted_by: RoleId,
    ) -> Self {
        Self {
            control,
            package,
            award,
            supplier,
            released_design,
            material,
            lead_time_days,
            production_need_on,
            accepted_by,
            accepted_on: None,
            responsibility_acceptance: String::new(),
            evidence: Vec::new(),
        }
    }

    /// Adds explicit supplier responsibility acceptance.
    #[must_use]
    pub fn accepts_responsibility(
        mut self,
        accepted_on: Date,
        statement: impl Into<String>,
    ) -> Self {
        self.accepted_on = Some(accepted_on);
        self.responsibility_acceptance = statement.into();
        self
    }

    /// Replaces the supplier organization reference.
    #[must_use]
    pub fn with_supplier(mut self, supplier: OrganizationId) -> Self {
        self.supplier = supplier;
        self
    }

    /// Adds reference-only handoff evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    fn validate(&self) -> Result<()> {
        if self.lead_time_days == 0 {
            return Err(ConstructionProjectError::EmptyField(
                "package_handoff.lead_time_days",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "package_handoff.evidence",
            ));
        }
        Ok(())
    }
}

/// One package handoff blocker.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageHandoffBlocker {
    /// Blocking control id.
    pub control: ControlId,
    /// Evidence state when applicable.
    pub evidence_state: EvidenceState,
    /// Deterministic blocker rule.
    pub rule: String,
    /// Deterministic explanation.
    pub reason: String,
}

/// Derived package handoff readiness report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandoffReadinessReport {
    /// Handoff control.
    pub handoff: ControlId,
    /// Work package.
    pub package: ControlId,
    /// Evaluation date.
    pub as_of_date: Date,
    /// True only when award, supplier qualification, design, material timing, and acceptance align.
    pub ready: bool,
    /// Supplier named in the handoff.
    pub supplier: OrganizationId,
    /// Blockers in deterministic order.
    pub blockers: Vec<PackageHandoffBlocker>,
}

/// Package handoff records.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageHandoffControlSet {
    /// Handoff records.
    pub handoffs: Vec<PackageHandoff>,
}

impl PackageHandoffControlSet {
    /// Builds an empty package handoff set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a handoff record.
    #[must_use]
    pub fn with_handoff(mut self, handoff: PackageHandoff) -> Self {
        self.handoffs.push(handoff);
        self
    }

    /// Derives handoff readiness from procurement, supplier, and design reports.
    pub fn readiness_for(
        &self,
        handoff: &ControlId,
        procurement: &PackageReadinessReport,
        supplier: &SupplierQualificationReport,
        design: &DesignReadinessReport,
        as_of_date: Date,
    ) -> Result<HandoffReadinessReport> {
        let handoff = self
            .handoffs
            .iter()
            .find(|candidate| &candidate.control == handoff)
            .ok_or_else(|| ConstructionProjectError::MissingPackageHandoff {
                handoff: handoff.clone(),
            })?;
        handoff.validate()?;

        let mut blockers = Vec::new();
        if handoff.package != procurement.package || handoff.package != design.target {
            return Err(ConstructionProjectError::HandoffPackageMismatch {
                handoff: handoff.control.clone(),
                expected: handoff.package.clone(),
            });
        }

        match &procurement.status {
            ProcurementStatus::Awarded {
                supplier, award, ..
            } => {
                if supplier != handoff.supplier.as_str() {
                    blockers.push(blocker(
                        &handoff.award,
                        "supplier-substitution",
                        "awarded supplier does not match the handoff supplier",
                    ));
                }
                if award != &handoff.award {
                    blockers.push(blocker(
                        &handoff.award,
                        "award-link",
                        "handoff must name the accountable award decision",
                    ));
                }
            }
            _ => blockers.push(blocker(
                &handoff.award,
                "award",
                "ready to award is not ready to produce",
            )),
        }

        if supplier.supplier != handoff.supplier {
            blockers.push(blocker(
                &handoff.control,
                "supplier-qualification",
                "qualification report is for a different supplier",
            ));
        }
        if !matches!(
            supplier.status,
            QualificationStatus::Qualified | QualificationStatus::Expiring
        ) {
            blockers.push(blocker(
                &handoff.control,
                "supplier-qualification",
                "supplier qualification is not accepted and current",
            ));
        }

        if !design.ready {
            blockers.extend(
                design
                    .blockers
                    .iter()
                    .map(|design_blocker| PackageHandoffBlocker {
                        control: design_blocker.control.clone(),
                        evidence_state: design_blocker.evidence_state,
                        rule: format!("design:{}", design_blocker.rule),
                        reason: design_blocker.reason.clone(),
                    }),
            );
        }
        if !design
            .releases
            .iter()
            .any(|release| release == &handoff.released_design)
        {
            blockers.push(blocker(
                &handoff.released_design,
                "released-design",
                "handoff must name a released production design",
            ));
        }

        if as_of_date + time::Duration::days(i64::from(handoff.lead_time_days))
            > handoff.production_need_on
        {
            blockers.push(blocker(
                &handoff.material,
                "lead-time",
                "material lead time does not fit the production need date",
            ));
        }
        if handoff.accepted_on.is_none() || handoff.responsibility_acceptance.trim().is_empty() {
            blockers.push(blocker(
                &handoff.control,
                "responsibility-acceptance",
                "supplier responsibility acceptance is missing",
            ));
        }
        blockers.sort_by(|left, right| {
            left.control
                .cmp(&right.control)
                .then_with(|| left.rule.cmp(&right.rule))
        });

        Ok(HandoffReadinessReport {
            handoff: handoff.control.clone(),
            package: handoff.package.clone(),
            as_of_date,
            ready: blockers.is_empty(),
            supplier: handoff.supplier.clone(),
            blockers,
        })
    }
}

fn blocker(
    control: &ControlId,
    rule: impl Into<String>,
    reason: impl Into<String>,
) -> PackageHandoffBlocker {
    PackageHandoffBlocker {
        control: control.clone(),
        evidence_state: EvidenceState::Reported,
        rule: rule.into(),
        reason: reason.into(),
    }
}
