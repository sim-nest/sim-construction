//! Citizen read-constructs for durable construction project-control values.

use serde::{Serialize, de::DeserializeOwned};
use sim_citizen::{CitizenField, CitizenRegistry};
use sim_codec_json::{JsonProjectionMode, project_expr_to_json, project_json_to_expr};
use sim_kernel::{Cx, Error, Expr, Result, Symbol, Value};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    AcceptedBaseline, BaselineId, BaselineKind, ChangeControlSet, ChangeDirection,
    ChangeExposureReport, ChangeId, ChangeRecord, CommissioningBurnDown,
    CommissioningItemReadiness, CommissioningReadinessReport, CommissioningRequirement,
    CommissioningRequirementKind, ConstructionExplanationReport, ConstructionStatusReport,
    ContractualBasis, ControlId, CurrencyCode, DomainQuantity, EvidenceState, EvidenceValidity,
    GateReport, OutcomeBoundary, OutcomeMethod, OutcomeRecord, OutcomeRecordKind,
    OutcomeRecordSpec, ProductionReadinessSnapshot, ProjectBook, ProjectDelta, ProjectFact,
    ProjectId, ProjectObligation, ProjectSnapshot, ReferenceAdmissionReport, Requirement,
    RequirementLane, RoleId, ScheduleBaseline, SchedulePlanRevision, ScheduleStatusReport,
    ScheduleTaskJoin, ScheduleTaskJoinSet, WorkPackage,
};

pub(crate) trait ConstructionCitizenSpec:
    Clone + core::fmt::Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    const SYMBOL: &'static str;

    fn example() -> Self;

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

macro_rules! construction_citizen {
    ($ty:ty, $symbol:literal, $example:expr, $validate:expr) => {
        impl $crate::citizen::ConstructionCitizenSpec for $ty {
            const SYMBOL: &'static str = $symbol;

            fn example() -> Self {
                $example
            }

            fn validate(&self) -> ::sim_kernel::Result<()> {
                let result: $crate::Result<()> = ($validate)(self);
                result.map_err(::core::convert::Into::into)
            }
        }

        impl ::sim_citizen::Citizen for $ty {
            fn citizen_symbol() -> ::sim_kernel::Symbol {
                ::sim_citizen::parse_symbol($symbol)
            }

            fn citizen_version() -> u32 {
                1
            }

            fn citizen_arity() -> usize {
                1
            }

            fn citizen_fields() -> &'static [&'static str] {
                &["fields"]
            }
        }

        impl ::sim_kernel::Object for $ty {
            fn display(&self, _cx: &mut ::sim_kernel::Cx) -> ::sim_kernel::Result<String> {
                Ok(format!("#<citizen {}>", $symbol))
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
        }

        impl ::sim_kernel::ObjectCompat for $ty {
            fn class(
                &self,
                cx: &mut ::sim_kernel::Cx,
            ) -> ::sim_kernel::Result<::sim_kernel::ClassRef> {
                let symbol = <Self as ::sim_citizen::Citizen>::citizen_symbol();
                if let Some(value) = cx.registry().class_by_symbol(&symbol) {
                    return Ok(value.clone());
                }
                ::sim_kernel::Factory::class_stub(cx.factory(), ::sim_kernel::ClassId(0), symbol)
            }

            fn as_expr(
                &self,
                cx: &mut ::sim_kernel::Cx,
            ) -> ::sim_kernel::Result<::sim_kernel::Expr> {
                ::sim_citizen::constructor_expr(cx, self)
            }

            fn as_object_encoder(&self) -> Option<&dyn ::sim_kernel::ObjectEncode> {
                Some(self)
            }
        }

        impl ::sim_kernel::ObjectEncode for $ty {
            fn object_encoding(
                &self,
                _cx: &mut ::sim_kernel::Cx,
            ) -> ::sim_kernel::Result<::sim_kernel::ObjectEncoding> {
                <Self as $crate::citizen::ConstructionCitizenSpec>::validate(self)?;
                Ok(::sim_kernel::ObjectEncoding::Constructor {
                    class: <Self as ::sim_citizen::Citizen>::citizen_symbol(),
                    args: vec![
                        ::sim_kernel::Expr::Symbol(::sim_kernel::Symbol::new("v1")),
                        $crate::citizen::encode_semantic(self)?,
                    ],
                })
            }
        }

        impl ::sim_citizen::CitizenRuntime for $ty {
            fn citizen_info() -> ::sim_citizen::CitizenInfo {
                ::sim_citizen::CitizenInfo {
                    symbol: $symbol,
                    version: 1,
                    crate_name: env!("CARGO_PKG_NAME"),
                    arity: 1,
                    install: <Self as ::sim_citizen::CitizenRuntime>::install,
                    conformance: <Self as ::sim_citizen::CitizenRuntime>::conformance,
                }
            }

            fn conformance(cx: &mut ::sim_kernel::Cx) -> ::sim_kernel::Result<()> {
                ::sim_citizen::check_fixture(
                    cx,
                    <Self as $crate::citizen::ConstructionCitizenSpec>::example(),
                )
            }

            fn construct_from_values(
                cx: &mut ::sim_kernel::Cx,
                args: Vec<::sim_kernel::Value>,
            ) -> ::sim_kernel::Result<Self> {
                if args.len() != 2 {
                    return Err(::sim_citizen::arity_error(
                        <Self as ::sim_citizen::Citizen>::citizen_symbol(),
                        2,
                        args.len(),
                    ));
                }
                let mut args = args.into_iter();
                ::sim_citizen::decode_version(
                    cx,
                    args.next().expect("arity checked"),
                    1,
                    <Self as ::sim_citizen::Citizen>::citizen_symbol(),
                )?;
                let fields = ::sim_citizen::value_to_expr(
                    cx,
                    args.next().expect("arity checked"),
                    "fields",
                )?;
                let value = $crate::citizen::decode_semantic(&fields, $symbol)?;
                <Self as $crate::citizen::ConstructionCitizenSpec>::validate(&value)?;
                Ok(value)
            }

            fn example() -> Self {
                <Self as $crate::citizen::ConstructionCitizenSpec>::example()
            }
        }

        const _: () = {
            ::sim_citizen::inventory::submit! {
                ::sim_citizen::CitizenInfo {
                    symbol: $symbol,
                    version: 1,
                    crate_name: env!("CARGO_PKG_NAME"),
                    arity: 1,
                    install: <$ty as ::sim_citizen::CitizenRuntime>::install,
                    conformance: <$ty as ::sim_citizen::CitizenRuntime>::conformance,
                }
            }
        };
    };
}

pub(crate) fn encode_semantic<T: Serialize>(value: &T) -> Result<Expr> {
    let json = serde_json::to_value(value)
        .map_err(|error| Error::Eval(format!("construction semantic encode failed: {error}")))?;
    Ok(project_json_to_expr(
        &json,
        JsonProjectionMode::UntaggedInterop,
    ))
}

pub(crate) fn decode_semantic<T: DeserializeOwned>(expr: &Expr, context: &str) -> Result<T> {
    let normalized = normalize_keyword_keys(expr);
    let json = project_expr_to_json(&normalized, JsonProjectionMode::UntaggedInterop);
    serde_json::from_value(json)
        .map_err(|error| Error::Eval(format!("construction {context}: {error}")))
}

pub(crate) fn normalize_keyword_keys(expr: &Expr) -> Expr {
    match expr {
        Expr::Map(entries) => Expr::Map(
            entries
                .iter()
                .map(|(key, value)| {
                    let key = match key {
                        Expr::Symbol(symbol) => {
                            Expr::Symbol(Symbol::new(symbol.as_qualified_str().replace('-', "_")))
                        }
                        other => normalize_keyword_keys(other),
                    };
                    (key, normalize_keyword_keys(value))
                })
                .collect(),
        ),
        Expr::List(items) => Expr::List(items.iter().map(normalize_keyword_keys).collect()),
        Expr::Vector(items) => Expr::Vector(items.iter().map(normalize_keyword_keys).collect()),
        other => other.clone(),
    }
}

pub(crate) fn decode_fact_value(cx: &mut Cx, value: &Value) -> Result<ProjectFact> {
    if let Some(fact) = value.object().downcast_ref::<ProjectFact>() {
        return Ok(fact.clone());
    }
    let mut fields = normalize_keyword_keys(&value.object().as_expr(cx)?);
    let Expr::Map(entries) = &mut fields else {
        return Err(Error::Eval(
            "construction fact constructor expects a keyword map".to_owned(),
        ));
    };
    if let Some((_, body)) = entries
        .iter_mut()
        .find(|(key, _)| matches!(key, Expr::Symbol(symbol) if symbol.as_qualified_str() == "body"))
    {
        let already_tagged = matches!(
            body,
            Expr::Map(body_entries)
                if body_entries.iter().any(|(key, _)| {
                    matches!(key, Expr::Symbol(symbol) if symbol.as_qualified_str() == "$expr")
                })
        );
        if !already_tagged {
            *body = project_json_to_expr(
                &sim_codec_json::expr_to_json(body),
                JsonProjectionMode::UntaggedInterop,
            );
        }
    }
    let fact: ProjectFact = decode_semantic(&fields, "fact")?;
    fact.validate_bounds()?;
    Ok(fact)
}

pub(crate) fn decode_value<T: ConstructionCitizenSpec>(
    cx: &mut Cx,
    value: &Value,
    context: &'static str,
) -> Result<T> {
    if let Some(value) = value.object().downcast_ref::<T>() {
        return Ok(value.clone());
    }
    let expr = value.object().as_expr(cx)?;
    let value: T = decode_semantic(&expr, context)?;
    value.validate()?;
    Ok(value)
}

/// Stable Citizen class symbols installed by the construction library.
#[must_use]
pub fn construction_citizen_symbols() -> Vec<Symbol> {
    CONSTRUCTION_CITIZENS
        .iter()
        .map(|symbol| sim_citizen::parse_symbol(symbol))
        .collect()
}

/// Builds the DCE-safe registry for the durable construction domain.
pub fn construction_citizen_registry() -> Result<CitizenRegistry> {
    let mut registry = CitizenRegistry::new();
    registry
        .register::<ProjectId>()?
        .register::<ProjectFact>()?
        .register::<AcceptedBaseline>()?
        .register::<Requirement>()?
        .register::<WorkPackage>()?
        .register::<ScheduleTaskJoinSet>()?
        .register::<ChangeRecord>()?
        .register::<CommissioningRequirement>()?
        .register::<OutcomeRecord>()?
        .register::<ProjectSnapshot>()?
        .register::<ProjectDelta>()?
        .register::<GateReport>()?
        .register::<ScheduleStatusReport>()?
        .register::<ProductionReadinessSnapshot>()?
        .register::<ChangeExposureReport>()?
        .register::<CommissioningReadinessReport>()?
        .register::<ReferenceAdmissionReport>()?
        .register::<ConstructionStatusReport>()?
        .register::<ConstructionExplanationReport>()?;
    Ok(registry)
}

const CONSTRUCTION_CITIZENS: [&str; 19] = [
    "construction/ProjectId",
    "construction/ProjectFact",
    "construction/AcceptedBaseline",
    "construction/Requirement",
    "construction/WorkPackage",
    "construction/ScheduleJoinSet",
    "construction/Change",
    "construction/HandoverItem",
    "construction/Outcome",
    "construction/ProjectSnapshot",
    "construction/ProjectDelta",
    "construction/GateReport",
    "construction/ScheduleImpactReport",
    "construction/ReadinessReport",
    "construction/ExposureReport",
    "construction/HandoverBurnDownReport",
    "construction/ReferenceAdmissionReport",
    "construction/StatusReport",
    "construction/ExplanationReport",
];

construction_citizen!(
    ProjectId,
    "construction/ProjectId",
    project_id(),
    |_: &ProjectId| Ok(())
);
construction_citizen!(
    ProjectFact,
    "construction/ProjectFact",
    fact(),
    ProjectFact::validate_bounds
);
construction_citizen!(
    AcceptedBaseline,
    "construction/AcceptedBaseline",
    baseline(),
    AcceptedBaseline::validate
);
construction_citizen!(
    Requirement,
    "construction/Requirement",
    requirement(),
    Requirement::validate
);
construction_citizen!(
    WorkPackage,
    "construction/WorkPackage",
    package(),
    |value: &WorkPackage| value.validate(&currency())
);
construction_citizen!(
    ScheduleTaskJoinSet,
    "construction/ScheduleJoinSet",
    joins(),
    |value: &ScheduleTaskJoinSet| {
        value.baseline.validate()?;
        value.revision.validate()
    }
);
construction_citizen!(
    ChangeRecord,
    "construction/Change",
    change(),
    ChangeRecord::validate
);
construction_citizen!(
    CommissioningRequirement,
    "construction/HandoverItem",
    handover_item(),
    |value: &CommissioningRequirement| value.obligation.requirement.validate()
);
construction_citizen!(
    OutcomeRecord,
    "construction/Outcome",
    outcome(),
    OutcomeRecord::validate
);
construction_citizen!(
    ProjectSnapshot,
    "construction/ProjectSnapshot",
    snapshot(),
    |_: &ProjectSnapshot| Ok(())
);
construction_citizen!(
    ProjectDelta,
    "construction/ProjectDelta",
    ProjectDelta {
        from_seq: 0,
        through_seq: 1,
        added: vec![control("scope")],
        superseded: Vec::new(),
        conflicted: Vec::new(),
    },
    |_: &ProjectDelta| Ok(())
);
construction_citizen!(
    GateReport,
    "construction/GateReport",
    GateReport {
        gate: control("mobilization"),
        as_of_seq: 1,
        ready: true,
        unmet: Vec::new(),
        conflicted: Vec::new(),
        expired: Vec::new(),
        applied_exceptions: Vec::new(),
    },
    |_: &GateReport| Ok(())
);
construction_citizen!(
    ScheduleStatusReport,
    "construction/ScheduleImpactReport",
    ScheduleStatusReport {
        baseline: baseline_id(),
        accepted_revision: "rev-1".to_owned(),
        as_of_seq: 1,
        critical_tasks: vec!["task-1".to_owned()],
        explanations: Vec::new(),
    },
    |_: &ScheduleStatusReport| Ok(())
);
construction_citizen!(
    ProductionReadinessSnapshot,
    "construction/ReadinessReport",
    ProductionReadinessSnapshot {
        baseline: baseline_id(),
        accepted_revision: "rev-1".to_owned(),
        imported_revision: "rev-1".to_owned(),
        as_of_seq: 1,
        as_of_date: date(),
        six_week_demand: Vec::new(),
        three_week_commitment: Vec::new(),
    },
    |_: &ProductionReadinessSnapshot| Ok(())
);
construction_citizen!(
    ChangeExposureReport,
    "construction/ExposureReport",
    ChangeControlSet::new()
        .derive(&currency(), date())
        .expect("empty exposure is valid"),
    |_: &ChangeExposureReport| Ok(())
);
construction_citizen!(
    CommissioningReadinessReport,
    "construction/HandoverBurnDownReport",
    CommissioningReadinessReport {
        target: control("system-control"),
        as_of_seq: 1,
        ready: false,
        burn_down: CommissioningBurnDown {
            total: 1,
            missing: 1,
            ..CommissioningBurnDown::default()
        },
        items: Vec::<CommissioningItemReadiness>::new(),
    },
    |_: &CommissioningReadinessReport| Ok(())
);
construction_citizen!(
    ReferenceAdmissionReport,
    "construction/ReferenceAdmissionReport",
    ReferenceAdmissionReport {
        claims: Vec::new(),
        manifest: None,
    },
    |_: &ReferenceAdmissionReport| Ok(())
);
construction_citizen!(
    ConstructionStatusReport,
    "construction/StatusReport",
    ConstructionStatusReport {
        project: project_id(),
        as_of_seq: 1,
        current: 1,
        superseded: 0,
        conflicted: 0,
        rejected: 0,
        accepted: 1,
        blockers: 0,
    },
    |_: &ConstructionStatusReport| Ok(())
);
construction_citizen!(
    ConstructionExplanationReport,
    "construction/ExplanationReport",
    ConstructionExplanationReport {
        project: project_id(),
        as_of_seq: 1,
        subject: control("scope"),
        current_sequence: Some(1),
        evidence_state: Some(EvidenceState::Accepted),
        rows: Vec::new(),
        actionable: "current accepted fact".to_owned(),
    },
    |_: &ConstructionExplanationReport| Ok(())
);

fn project_id() -> ProjectId {
    ProjectId::new("project-demo").expect("valid fixture project id")
}

fn control(value: &str) -> ControlId {
    ControlId::new(value).expect("valid fixture control id")
}

fn role(value: &str) -> RoleId {
    RoleId::new(value).expect("valid fixture role id")
}

fn baseline_id() -> BaselineId {
    BaselineId::new("baseline-1").expect("valid fixture baseline id")
}

fn date() -> Date {
    Date::from_calendar_date(2026, Month::January, 15).expect("valid fixture date")
}

fn reference(id: &str) -> ExternalRef {
    ExternalRef::new("project-fixture", id, Some("rev-1".to_owned()), None)
}

fn currency() -> CurrencyCode {
    CurrencyCode::new("SEK").expect("valid fixture currency")
}

fn fact() -> ProjectFact {
    ProjectFact::new(
        1,
        project_id(),
        control("scope"),
        Symbol::qualified("construction", "scope"),
        date(),
        role("project-chief"),
        Expr::Map(vec![(
            Expr::Symbol(Symbol::new("state")),
            Expr::Symbol(Symbol::new("accepted")),
        )]),
    )
    .with_evidence(reference("scope-1"))
}

fn baseline() -> AcceptedBaseline {
    AcceptedBaseline::new(
        baseline_id(),
        project_id(),
        control("baseline-control"),
        BaselineKind::Time,
        role("project-chief"),
        1,
        date(),
    )
    .with_evidence(reference("baseline-1"))
}

fn requirement() -> Requirement {
    Requirement::new(
        control("permit"),
        RequirementLane::new(Symbol::qualified("construction", "authority")),
        "Current building permit",
        role("design-manager"),
        role("project-chief"),
    )
    .with_evidence_kind(Symbol::qualified("evidence", "permit"))
    .with_source_ref(reference("permit-source"))
}

fn package() -> WorkPackage {
    WorkPackage::new(
        project_id(),
        control("package-ventilation"),
        "Ventilation",
        role("procurement-manager"),
        role("project-chief"),
        date(),
        date().next_day().expect("fixture date"),
        date()
            .next_day()
            .and_then(Date::next_day)
            .expect("fixture date"),
        crate::CommercialAmount::parse("1000.00", currency()).expect("valid fixture amount"),
    )
    .includes("Air-handling unit")
    .requires_design_input(control("design-ventilation"))
    .with_supplier(
        crate::SupplierCandidate::new("supplier-demo", "qualified")
            .with_evidence(reference("supplier-1")),
    )
    .with_evidence(reference("package-1"))
}

fn joins() -> ScheduleTaskJoinSet {
    ScheduleTaskJoinSet::new(
        ScheduleBaseline::new(baseline_id(), "plan-1", "rev-1", 1).expect("valid fixture baseline"),
        SchedulePlanRevision::new("plan-1", "rev-1", 1).expect("valid fixture revision"),
        vec![ScheduleTaskJoin::new(
            control("scope"),
            "task-1",
            crate::ScheduleJoinKind::Control,
        )],
    )
    .expect("valid fixture joins")
}

fn change() -> ChangeRecord {
    ChangeRecord::new(
        project_id(),
        ChangeId::new("change-1").expect("valid fixture change id"),
        ChangeDirection::CustomerInstruction,
        ContractualBasis::new("variation", "AB04 chapter 2", reference("contract-ab04")),
        role("project-chief"),
        date(),
        None,
    )
    .affects_control(control("scope"))
    .with_evidence(reference("change-1"))
}

fn handover_item() -> CommissioningRequirement {
    let requirement = requirement();
    CommissioningRequirement::new(
        CommissioningRequirementKind::Certification,
        ProjectObligation::mandatory(project_id(), requirement),
        control("system-control"),
    )
}

fn outcome() -> OutcomeRecord {
    let source = reference("outcome-source");
    OutcomeRecord::new(OutcomeRecordSpec {
        project: project_id(),
        id: control("outcome-1"),
        target: control("target-1"),
        kind: OutcomeRecordKind::Measurement,
        quantity: DomainQuantity::new("12.5", Symbol::qualified("unit", "kg-co2e")),
        method: OutcomeMethod::new(
            Symbol::qualified("method", "epd"),
            "1",
            Symbol::qualified("shape", "epd"),
            source.clone(),
        ),
        boundary: OutcomeBoundary::new(
            Symbol::qualified("boundary", "a1-a3"),
            "product stage",
            source.clone(),
        ),
        responsible: role("sustainability-manager"),
        reported_on: date(),
    })
    .with_evidence_state(EvidenceState::Accepted)
    .with_validity(EvidenceValidity::unbounded())
    .with_source_ref(source.clone())
    .with_evidence_ref(source)
}

fn snapshot() -> ProjectSnapshot {
    let mut book = ProjectBook::new(project_id(), role("project-chief"));
    book.append(fact()).expect("valid fixture fact");
    book.snapshot_at(1).expect("valid fixture snapshot")
}

#[allow(dead_code)]
fn _assert_field_support<T: CitizenField>() {}
