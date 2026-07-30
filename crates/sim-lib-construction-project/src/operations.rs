//! Thin loadable constructors and project-control operations.

use std::sync::{Arc, Mutex, MutexGuard};

use serde::de::DeserializeOwned;
use sim_codec::{Input, decode_eval_expr_with_codec};
use sim_kernel::{
    AbiVersion, Args, Callable, Cx, Demand, Error, Export, Expr, Lib, LibManifest, LibTarget,
    Linker, LoadCx, Object, ObjectCompat, PreparedArgs, ReadPolicy, Result, Shape, Symbol, Value,
    Version,
};
use sim_shape::{Bindings, FunctionCase, FunctionObject, ListShape};

mod execute;

use execute::*;

use crate::{
    AcceptedBaseline, CommissioningReadinessReport, CommissioningRequirement, ControlId,
    OutcomeRecord, ProjectBook, ProjectDelta, ProjectFact, ProjectId, ProjectSnapshot, Requirement,
    ScheduleTaskJoinSet, WorkPackage,
    change::ChangeRecord,
    shapes::{any_shape, number_shape, semantic_map_shape, string_shape, type_shape},
};

/// Stable library symbol for construction project-control runtime behavior.
#[must_use]
pub fn construction_project_lib_symbol() -> Symbol {
    Symbol::qualified("sim", "construction-project")
}

/// Summary of one deterministic project snapshot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConstructionStatusReport {
    /// Stable project identity.
    pub project: ProjectId,
    /// Inclusive fact sequence used for this status.
    pub as_of_seq: u64,
    /// Current non-conflicted subjects.
    pub current: usize,
    /// Superseded fact count.
    pub superseded: usize,
    /// Conflicted fact count.
    pub conflicted: usize,
    /// Rejected fact count.
    pub rejected: usize,
    /// Current facts carrying accepted evidence.
    pub accepted: usize,
    /// Current or conflicted subjects requiring accountable action.
    pub blockers: usize,
}

/// Actionable explanation for one project-control subject.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConstructionExplanationReport {
    /// Stable project identity.
    pub project: ProjectId,
    /// Inclusive fact sequence used for this explanation.
    pub as_of_seq: u64,
    /// Subject being explained.
    pub subject: ControlId,
    /// Current fact sequence, when one unambiguous fact exists.
    pub current_sequence: Option<u64>,
    /// Current evidence state, when one unambiguous fact exists.
    pub evidence_state: Option<crate::EvidenceState>,
    /// Stable derivation rows concerning this subject.
    pub rows: Vec<crate::ProjectSnapshotExplanation>,
    /// Bounded next-action diagnostic.
    pub actionable: String,
}

/// One host-registered library for construction Citizens, Shapes, and operations.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConstructionProjectLib;

impl Lib for ConstructionProjectLib {
    fn manifest(&self) -> LibManifest {
        let mut exports = crate::construction_citizen_symbols()
            .into_iter()
            .map(|symbol| Export::Class {
                symbol,
                class_id: None,
            })
            .collect::<Vec<_>>();
        exports.extend(
            crate::construction_shape_symbols()
                .into_iter()
                .map(|symbol| Export::Shape {
                    symbol,
                    shape_id: None,
                }),
        );
        exports.extend(all_specs().into_iter().map(|spec| Export::Function {
            symbol: spec.symbol(),
            function_id: None,
        }));
        exports.push(Export::Function {
            symbol: construction_cli_entrypoint_symbol(),
            function_id: None,
        });
        LibManifest {
            id: construction_project_lib_symbol(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports,
        }
    }

    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> Result<()> {
        crate::construction_citizen_registry()?.install_all(linker)?;
        crate::shapes::register_type_shapes(linker)?;
        for spec in all_specs() {
            let args_symbol = operation_args_shape_symbol(spec.name);
            let result_symbol = operation_result_shape_symbol(spec.name);
            let args_shape: Arc<dyn Shape> = Arc::new(ListShape::tuple(spec.args.clone()));
            linker.shape_value(
                args_symbol.clone(),
                sim_shape::shape_value(args_symbol, args_shape.clone()),
            )?;
            linker.shape_value(
                result_symbol.clone(),
                sim_shape::shape_value(result_symbol, spec.result.clone()),
            )?;
            let function = FunctionObject::new(
                cx.fresh_function_id(),
                spec.symbol(),
                vec![FunctionCase {
                    id: cx.fresh_case_id(),
                    name: Symbol::qualified(format!("construction/{}", spec.name), "checked"),
                    args: args_shape,
                    result: Some(spec.result.clone()),
                    demand: vec![Demand::Value; spec.args.len()],
                    priority: 10,
                    implementation: spec.implementation,
                }],
            );
            linker.function_value(spec.symbol(), cx.factory().opaque(Arc::new(function))?)?;
        }
        linker.function_value(
            construction_cli_entrypoint_symbol(),
            cx.factory()
                .opaque(Arc::new(ConstructionRecipeEntrypoint))?,
        )?;
        Ok(())
    }
}

/// Installs [`ConstructionProjectLib`] once in a context.
pub fn install_construction_project_lib(cx: &mut Cx) -> Result<()> {
    if cx
        .registry()
        .lib(&construction_project_lib_symbol())
        .is_none()
    {
        cx.load_lib(&ConstructionProjectLib)?;
    }
    Ok(())
}

/// Public constructor symbols, in stable contract order.
#[must_use]
pub fn construction_constructor_symbols() -> Vec<Symbol> {
    constructor_specs()
        .into_iter()
        .map(|spec| spec.symbol())
        .collect()
}

/// Public project-control operation symbols, in stable contract order.
#[must_use]
pub fn construction_operation_symbols() -> Vec<Symbol> {
    operation_specs()
        .into_iter()
        .map(|spec| spec.symbol())
        .collect()
}

pub(crate) fn operation_shape_symbols() -> Vec<Symbol> {
    all_specs()
        .into_iter()
        .flat_map(|spec| {
            [
                operation_args_shape_symbol(spec.name),
                operation_result_shape_symbol(spec.name),
            ]
        })
        .collect()
}

fn operation_args_shape_symbol(name: &str) -> Symbol {
    Symbol::qualified("construction-args", name)
}

fn operation_result_shape_symbol(name: &str) -> Symbol {
    Symbol::qualified("construction-result", name)
}

fn construction_cli_entrypoint_symbol() -> Symbol {
    Symbol::qualified("cli", "main/construction")
}

struct ConstructionRecipeEntrypoint;

impl Object for ConstructionRecipeEntrypoint {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<cli/main/construction>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for ConstructionRecipeEntrypoint {
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for ConstructionRecipeEntrypoint {
    fn call(&self, cx: &mut Cx, _args: Args) -> Result<Value> {
        let expression = decode_eval_expr_with_codec(
            cx,
            &Symbol::qualified("codec", "lisp"),
            Input::Text(
                include_str!("../recipes/01-basics/loadable-project-control/setup.siml").to_owned(),
            ),
            ReadPolicy::default(),
        )?;
        cx.eval_expr(expression)
    }
}

#[derive(Clone)]
struct OperationSpec {
    name: &'static str,
    args: Vec<Arc<dyn Shape>>,
    result: Arc<dyn Shape>,
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
}

impl OperationSpec {
    fn symbol(&self) -> Symbol {
        Symbol::qualified("construction", self.name)
    }
}

fn all_specs() -> Vec<OperationSpec> {
    let mut specs = constructor_specs();
    specs.extend(operation_specs());
    specs
}

fn spec(
    name: &'static str,
    args: Vec<Arc<dyn Shape>>,
    result: Arc<dyn Shape>,
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
) -> OperationSpec {
    OperationSpec {
        name,
        args,
        result,
        implementation,
    }
}

fn constructor_specs() -> Vec<OperationSpec> {
    vec![
        spec(
            "project-id",
            vec![string_shape()],
            type_shape::<ProjectId>(),
            project_id_impl,
        ),
        spec(
            "book",
            vec![type_shape::<ProjectId>(), string_shape()],
            book_shape(),
            book_impl,
        ),
        spec(
            "fact",
            vec![semantic_map_shape(
                &[
                    "seq",
                    "project",
                    "subject",
                    "kind",
                    "effective-on",
                    "actor-role",
                    "visibility",
                    "body",
                    "evidence",
                    "evidence-state",
                ],
                &["supersedes"],
            )],
            type_shape::<ProjectFact>(),
            fact_impl,
        ),
        spec(
            "baseline",
            vec![semantic_map_shape(
                &[
                    "id",
                    "project",
                    "control",
                    "kind",
                    "accepted-by",
                    "accepted-seq",
                    "accepted-on",
                    "evidence",
                ],
                &[],
            )],
            type_shape::<AcceptedBaseline>(),
            baseline_impl,
        ),
        spec(
            "requirement",
            vec![semantic_map_shape(
                &[
                    "id",
                    "lane",
                    "title",
                    "owner",
                    "acceptance-authority",
                    "evidence-required",
                    "evidence-kinds",
                    "source-refs",
                    "dependencies",
                    "non-waivable",
                ],
                &["due-on"],
            )],
            type_shape::<Requirement>(),
            requirement_impl,
        ),
        spec(
            "package",
            vec![semantic_map_shape(
                &[
                    "project",
                    "control",
                    "name",
                    "scope-inclusions",
                    "scope-exclusions",
                    "interfaces",
                    "design-inputs",
                    "inquiry-due-on",
                    "award-due-on",
                    "need-on",
                    "procurement-owner",
                    "award-authority",
                    "target-amount",
                    "supplier-candidates",
                    "evidence",
                ],
                &[],
            )],
            type_shape::<WorkPackage>(),
            package_impl,
        ),
        spec(
            "join",
            vec![semantic_map_shape(&["baseline", "revision", "joins"], &[])],
            type_shape::<ScheduleTaskJoinSet>(),
            join_impl,
        ),
        spec(
            "change",
            vec![semantic_map_shape(
                &[
                    "project",
                    "id",
                    "direction",
                    "contractual-basis",
                    "affected-controls",
                    "affected-tasks",
                    "affected-packages",
                    "responsible-role",
                    "initiated-on",
                    "evidence",
                ],
                &["notice-due-on", "notice-given-on"],
            )],
            type_shape::<ChangeRecord>(),
            change_impl,
        ),
        spec(
            "handover-item",
            vec![semantic_map_shape(
                &[
                    "kind",
                    "obligation",
                    "targets",
                    "critical",
                    "gates",
                    "exception-gates",
                ],
                &[],
            )],
            type_shape::<CommissioningRequirement>(),
            handover_item_impl,
        ),
        spec(
            "outcome",
            vec![semantic_map_shape(
                &[
                    "project",
                    "id",
                    "target",
                    "kind",
                    "quantity",
                    "method",
                    "boundary",
                    "responsible",
                    "reported-on",
                    "evidence-state",
                    "validity",
                    "source-refs",
                    "evidence-refs",
                    "disclosure",
                ],
                &["supersedes"],
            )],
            type_shape::<OutcomeRecord>(),
            outcome_impl,
        ),
    ]
}

fn operation_specs() -> Vec<OperationSpec> {
    vec![
        spec(
            "append",
            vec![book_shape(), type_shape::<ProjectFact>()],
            type_shape::<ProjectFact>(),
            append_impl,
        ),
        spec(
            "snapshot-as-of",
            vec![book_shape(), number_shape()],
            type_shape::<ProjectSnapshot>(),
            snapshot_impl,
        ),
        spec("validate", vec![any_shape()], any_shape(), validate_impl),
        spec(
            "status",
            vec![type_shape::<ProjectSnapshot>()],
            type_shape::<ConstructionStatusReport>(),
            status_impl,
        ),
        spec(
            "explain",
            vec![type_shape::<ProjectSnapshot>(), string_shape()],
            type_shape::<ConstructionExplanationReport>(),
            explain_impl,
        ),
        spec(
            "diff-since",
            vec![book_shape(), number_shape()],
            type_shape::<ProjectDelta>(),
            diff_since_impl,
        ),
        spec(
            "gate-report",
            vec![
                book_shape(),
                semantic_map_shape(&["gate", "as-of-seq", "as-of-date"], &[]),
            ],
            type_shape::<crate::GateReport>(),
            gate_report_impl,
        ),
        spec(
            "schedule-impact",
            vec![semantic_map_shape(
                &["plan", "joins", "graph", "states", "as-of-date"],
                &[],
            )],
            type_shape::<crate::ScheduleStatusReport>(),
            schedule_impact_impl,
        ),
        spec(
            "readiness",
            vec![
                book_shape(),
                semantic_map_shape(&["plan", "schedule", "joins", "as-of-date"], &[]),
            ],
            type_shape::<crate::ProductionReadinessSnapshot>(),
            readiness_impl,
        ),
        spec(
            "exposure",
            vec![semantic_map_shape(
                &["changes", "currency", "as-of-date"],
                &[],
            )],
            type_shape::<crate::ChangeExposureReport>(),
            exposure_impl,
        ),
        spec(
            "handover-burn-down",
            vec![
                book_shape(),
                semantic_map_shape(
                    &["controls", "hierarchy", "target", "as-of-seq", "as-of-date"],
                    &["exceptions", "capabilities"],
                ),
            ],
            type_shape::<CommissioningReadinessReport>(),
            handover_burn_down_impl,
        ),
        spec(
            "reference-admission",
            vec![
                book_shape(),
                semantic_map_shape(&["admission", "closeout", "outcomes"], &[]),
            ],
            type_shape::<crate::ReferenceAdmissionReport>(),
            reference_admission_impl,
        ),
    ]
}

#[derive(Clone)]
struct ProjectBookHandle {
    book: Arc<Mutex<ProjectBook>>,
}

impl sim_kernel::Object for ProjectBookHandle {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        let book = lock_book(self)?;
        Ok(format!(
            "#<construction-book project={} facts={}>",
            book.project(),
            book.len()
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for ProjectBookHandle {
    fn class(&self, cx: &mut Cx) -> Result<sim_kernel::ClassRef> {
        sim_kernel::Factory::class_stub(
            cx.factory(),
            sim_kernel::ClassId(0),
            Symbol::qualified("construction", "ProjectBook"),
        )
    }
}

fn book_shape() -> Arc<dyn Shape> {
    Arc::new(ProjectBookShape)
}

struct ProjectBookShape;

impl Shape for ProjectBookShape {
    fn symbol(&self) -> Option<Symbol> {
        Some(Symbol::qualified("construction-shape", "ProjectBook"))
    }

    fn check_value(&self, _cx: &mut Cx, value: Value) -> Result<sim_kernel::ShapeMatch> {
        Ok(
            if value.object().downcast_ref::<ProjectBookHandle>().is_some() {
                sim_kernel::ShapeMatch::accept(sim_kernel::MatchScore::exact(100))
            } else {
                sim_kernel::ShapeMatch::reject(
                    "construction ProjectBook handle expected; use construction/book",
                )
            },
        )
    }

    fn check_expr(&self, cx: &mut Cx, expr: &Expr) -> Result<sim_kernel::ShapeMatch> {
        let value = cx.eval_expr(expr.clone())?;
        self.check_value(cx, value)
    }

    fn describe(&self, _cx: &mut Cx) -> Result<sim_kernel::ShapeDoc> {
        Ok(sim_kernel::ShapeDoc::new("construction ProjectBook handle")
            .with_detail("live bounded in-memory book; durable facts and snapshots are Citizens"))
    }
}

fn book_handle(value: &Value) -> Result<&ProjectBookHandle> {
    value
        .object()
        .downcast_ref::<ProjectBookHandle>()
        .ok_or_else(|| {
            Error::Eval(
                "construction operation expects a ProjectBook from construction/book".to_owned(),
            )
        })
}

fn lock_book(handle: &ProjectBookHandle) -> Result<MutexGuard<'_, ProjectBook>> {
    handle
        .book
        .lock()
        .map_err(|_| Error::Eval("construction ProjectBook lock poisoned".to_owned()))
}

fn boxed<T>(cx: &Cx, value: T) -> Result<Value>
where
    T: sim_kernel::Object + sim_kernel::ObjectCompat + Send + Sync + 'static,
{
    cx.factory().opaque(Arc::new(value))
}

fn string(cx: &mut Cx, value: &Value, context: &str) -> Result<String> {
    match value.object().as_expr(cx)? {
        Expr::String(value) => Ok(value),
        other => Err(Error::Eval(format!(
            "construction/{context} expects a string, found {other:?}"
        ))),
    }
}

fn plain_value<T: DeserializeOwned>(cx: &mut Cx, value: &Value, context: &str) -> Result<T> {
    let expr = value.object().as_expr(cx)?;
    crate::citizen::decode_semantic(&expr, context)
}

fn arity(name: &str, expected: usize, actual: usize) -> Result<Value> {
    Err(Error::Eval(format!(
        "construction/{name} expects {expected} argument(s), found {actual}"
    )))
}
