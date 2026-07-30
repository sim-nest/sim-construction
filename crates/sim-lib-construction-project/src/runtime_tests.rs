//! Citizen, codec, Shape, capability, operation, and bootloader conformance.

use std::sync::Arc;

use sim_citizen::values_citizen_eq;
use sim_codec::{Input, Output, decode_with_codec, encode_with_codec};
use sim_kernel::{
    Args, CapabilitySet, Cx, DefaultFactory, EagerPolicy, EncodeOptions, Expr, Lib, Object,
    ReadPolicy, Symbol, TrustLevel, Value, read_construct_capability,
};

use crate::{
    AcceptedBaseline, ChangeExposureReport, ChangeRecord, CommissioningReadinessReport,
    CommissioningRequirement, ConstructionExplanationReport, ConstructionProjectLib,
    ConstructionStatusReport, GateReport, OutcomeRecord, ProductionReadinessSnapshot, ProjectDelta,
    ProjectFact, ProjectId, ProjectSnapshot, ReferenceAdmissionReport, Requirement,
    ScheduleStatusReport, ScheduleTaskJoinSet, WorkPackage, citizen::ConstructionCitizenSpec,
    construction_citizen_registry, construction_constructor_symbols,
    construction_operation_symbols, construction_project_read_capability,
    construction_project_write_capability,
};

#[test]
fn complete_citizen_inventory_is_conformant() {
    let registry = construction_citizen_registry().expect("construction Citizen registry");
    let symbols = crate::construction_citizen_symbols()
        .into_iter()
        .map(|symbol| symbol.to_string())
        .collect::<Vec<_>>();
    let expected = symbols.iter().map(String::as_str).collect::<Vec<_>>();
    registry
        .ensure_contains_symbols(&expected)
        .expect("complete construction Citizen inventory");
    let mut cx = bare_cx();
    sim_citizen::run_registry_conformance_expecting(&mut cx, &registry, &expected)
        .expect("construction Citizen conformance");
}

#[test]
fn lisp_json_and_binary_round_trip_every_durable_domain_category_and_report() {
    let mut cx = codec_cx();
    for value in example_values(&cx) {
        let expr = value.object().as_expr(&mut cx).expect("Citizen expression");
        for codec in [
            Symbol::qualified("codec", "lisp"),
            Symbol::qualified("codec", "json"),
            Symbol::qualified("codec", "binary"),
        ] {
            let encoded = encode_with_codec(
                &mut cx,
                &codec,
                &expr,
                EncodeOptions {
                    position: sim_kernel::EncodePosition::Data,
                    ..EncodeOptions::default()
                },
            )
            .expect("encode Citizen");
            let input = match encoded {
                Output::Text(text) => Input::Text(text),
                Output::Bytes(bytes) => Input::Bytes(bytes),
            };
            let decoded =
                decode_with_codec(&mut cx, &codec, input, trusted_read_construct_policy())
                    .expect("decode Citizen");
            assert!(
                decoded.canonical_eq(&expr),
                "{codec} changed the semantic read-construct"
            );
            let reconstructed =
                reconstruct(&mut cx, &decoded).expect("reconstruct decoded Citizen");
            assert!(
                values_citizen_eq(&mut cx, &value, &reconstructed)
                    .expect("Citizen semantic equality"),
                "{codec} changed the reconstructed object"
            );
        }
    }
}

#[test]
fn loadable_operations_construct_append_snapshot_status_diff_and_explain() {
    let mut cx = runtime_cx();
    let project_name = string_value(&mut cx, "project-demo");
    let project = call(&mut cx, "project-id", vec![project_name]);
    let project_chief = string_value(&mut cx, "project-chief");
    let book = call(&mut cx, "book", vec![project, project_chief]);
    let fact = <ProjectFact as ConstructionCitizenSpec>::example();
    let fact_map = hyphenated_top_level(crate::citizen::encode_semantic(&fact).unwrap());
    let fact_map = sim_citizen::value_from_expr(&mut cx, &fact_map).unwrap();
    let fact = call(&mut cx, "fact", vec![fact_map]);
    assert!(fact.object().downcast_ref::<ProjectFact>().is_some());
    call(&mut cx, "append", vec![book.clone(), fact]);

    let one = integer_value(&mut cx, 1);
    let snapshot = call(&mut cx, "snapshot-as-of", vec![book.clone(), one]);
    let snapshot_record = snapshot
        .object()
        .downcast_ref::<ProjectSnapshot>()
        .expect("snapshot Citizen");
    assert_eq!(snapshot_record.through_seq, 1);

    let status = call(&mut cx, "status", vec![snapshot.clone()]);
    let status = status
        .object()
        .downcast_ref::<ConstructionStatusReport>()
        .expect("status Citizen");
    assert_eq!(
        (status.current, status.accepted, status.blockers),
        (1, 1, 0)
    );

    let scope = string_value(&mut cx, "scope");
    let explanation = call(&mut cx, "explain", vec![snapshot, scope]);
    let explanation = explanation
        .object()
        .downcast_ref::<ConstructionExplanationReport>()
        .expect("explanation Citizen");
    assert_eq!(explanation.current_sequence, Some(1));
    assert!(explanation.actionable.contains("no blocker"));

    let zero = integer_value(&mut cx, 0);
    let delta = call(&mut cx, "diff-since", vec![book, zero]);
    assert_eq!(
        delta
            .object()
            .downcast_ref::<ProjectDelta>()
            .expect("delta Citizen")
            .added,
        vec![crate::ControlId::new("scope").unwrap()]
    );
}

#[test]
fn shape_failures_are_actionable_and_capability_errors_remain_typed() {
    let mut cx = runtime_cx();
    let malformed = sim_citizen::value_from_expr(
        &mut cx,
        &Expr::Map(vec![(
            Expr::Symbol(Symbol::new("project")),
            Expr::String("missing-required-fields".to_owned()),
        )]),
    )
    .unwrap();
    let error = cx
        .call_function(
            &Symbol::qualified("construction", "fact"),
            Args::new(vec![malformed]),
        )
        .unwrap_err();
    let sim_kernel::Error::NoMatchingOverload { diagnostics, .. } = error else {
        panic!("expected Shape overload rejection, found {error}");
    };
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("required")
                || diagnostic.message.contains("construction/fact")
        }),
        "missing actionable Shape diagnostic: {diagnostics:#?}"
    );

    let mut denied = bare_cx();
    denied.load_lib(&ConstructionProjectLib).unwrap();
    let project = boxed(&denied, <ProjectId as ConstructionCitizenSpec>::example());
    let writer = denied.factory().string("project-chief".to_owned()).unwrap();
    let error = denied
        .call_function(
            &Symbol::qualified("construction", "book"),
            Args::new(vec![project, writer]),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == construction_project_write_capability()
    ));

    let valid = boxed(&denied, <ProjectId as ConstructionCitizenSpec>::example());
    assert_eq!(
        denied
            .call_function(
                &Symbol::qualified("construction", "validate"),
                Args::new(vec![valid]),
            )
            .unwrap()
            .object()
            .as_expr(&mut denied)
            .unwrap(),
        Expr::Bool(true)
    );
}

#[test]
fn manifest_exports_all_constructors_operations_classes_and_shapes() {
    let manifest = ConstructionProjectLib.manifest();
    assert_eq!(construction_constructor_symbols().len(), 10);
    assert_eq!(construction_operation_symbols().len(), 12);
    for symbol in construction_constructor_symbols()
        .into_iter()
        .chain(construction_operation_symbols())
    {
        assert!(
            manifest.exports.iter().any(
                |export| matches!(export, sim_kernel::Export::Function { symbol: found, .. } if found == &symbol)
            ),
            "missing function {symbol}"
        );
    }
    for symbol in crate::construction_shape_symbols() {
        assert!(
            manifest.exports.iter().any(
                |export| matches!(export, sim_kernel::Export::Shape { symbol: found, .. } if found == &symbol)
            ),
            "missing Shape {symbol}"
        );
    }
}

#[test]
fn loadable_lisp_specimen_constructs_appends_snapshots_and_explains() {
    let code = sim_run_core::Bootloader::standard()
        .host_lib("codec/lisp", || {
            Box::new(sim_codec_lisp::LispCodecLib::new(sim_kernel::CodecId(1)).expect("Lisp codec"))
        })
        .host_lib("lib/binding", || Box::new(sim_lib_binding::BindingLib))
        .host_lib("numbers/i64", || {
            Box::new(sim_lib_numbers_i64::I64NumbersLib::new())
        })
        .host_verb("construction", "sim/construction-project", || {
            Box::new(ConstructionProjectLib)
        })
        .with_context(|cx| cx.set_eval_policy(Arc::new(EagerPolicy)))
        .with_capability(construction_project_read_capability())
        .with_capability(construction_project_write_capability())
        .run([
            "sim",
            "--load",
            "host:codec/lisp",
            "--load",
            "host:lib/binding",
            "--load",
            "host:numbers/i64",
            "--load",
            "host:sim/construction-project",
            "construction",
        ])
        .expect("standard bootloader construction specimen");
    assert_eq!(code, 0);
}

fn example_values(cx: &Cx) -> Vec<Value> {
    vec![
        boxed(cx, <ProjectId as ConstructionCitizenSpec>::example()),
        boxed(cx, <ProjectFact as ConstructionCitizenSpec>::example()),
        boxed(cx, <AcceptedBaseline as ConstructionCitizenSpec>::example()),
        boxed(cx, <Requirement as ConstructionCitizenSpec>::example()),
        boxed(cx, <WorkPackage as ConstructionCitizenSpec>::example()),
        boxed(
            cx,
            <ScheduleTaskJoinSet as ConstructionCitizenSpec>::example(),
        ),
        boxed(cx, <ChangeRecord as ConstructionCitizenSpec>::example()),
        boxed(
            cx,
            <CommissioningRequirement as ConstructionCitizenSpec>::example(),
        ),
        boxed(cx, <OutcomeRecord as ConstructionCitizenSpec>::example()),
        boxed(cx, <ProjectSnapshot as ConstructionCitizenSpec>::example()),
        boxed(cx, <ProjectDelta as ConstructionCitizenSpec>::example()),
        boxed(cx, <GateReport as ConstructionCitizenSpec>::example()),
        boxed(
            cx,
            <ScheduleStatusReport as ConstructionCitizenSpec>::example(),
        ),
        boxed(
            cx,
            <ProductionReadinessSnapshot as ConstructionCitizenSpec>::example(),
        ),
        boxed(
            cx,
            <ChangeExposureReport as ConstructionCitizenSpec>::example(),
        ),
        boxed(
            cx,
            <CommissioningReadinessReport as ConstructionCitizenSpec>::example(),
        ),
        boxed(
            cx,
            <ReferenceAdmissionReport as ConstructionCitizenSpec>::example(),
        ),
        boxed(
            cx,
            <ConstructionStatusReport as ConstructionCitizenSpec>::example(),
        ),
        boxed(
            cx,
            <ConstructionExplanationReport as ConstructionCitizenSpec>::example(),
        ),
    ]
}

fn runtime_cx() -> Cx {
    let mut cx = bare_cx();
    cx.load_lib(&ConstructionProjectLib).unwrap();
    cx.grant(construction_project_read_capability());
    cx.grant(construction_project_write_capability());
    cx
}

fn codec_cx() -> Cx {
    let mut cx = bare_cx();
    cx.grant(read_construct_capability());
    cx.load_lib(&ConstructionProjectLib).unwrap();
    let lisp = sim_codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = sim_codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let binary = sim_codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary).unwrap();
    cx
}

fn bare_cx() -> Cx {
    Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory))
}

fn call(cx: &mut Cx, name: &str, values: Vec<Value>) -> Value {
    cx.call_function(&Symbol::qualified("construction", name), Args::new(values))
        .unwrap_or_else(|error| panic!("construction/{name}: {error}"))
}

fn reconstruct(cx: &mut Cx, expr: &Expr) -> sim_kernel::Result<Value> {
    let Expr::Extension { tag, payload } = expr else {
        return Err(sim_kernel::Error::Eval(
            "Citizen expression must be a read-construct".to_owned(),
        ));
    };
    if *tag != Symbol::qualified("citizen", "read-construct") {
        return Err(sim_kernel::Error::Eval(format!(
            "unexpected Citizen extension {tag}"
        )));
    }
    let Expr::Vector(items) = payload.as_ref() else {
        return Err(sim_kernel::Error::Eval(
            "Citizen read-construct payload must be a vector".to_owned(),
        ));
    };
    let Some((Expr::Symbol(class), args)) = items.split_first() else {
        return Err(sim_kernel::Error::Eval(
            "Citizen read-construct must name a class".to_owned(),
        ));
    };
    let values = args
        .iter()
        .map(|arg| sim_citizen::value_from_expr(cx, arg))
        .collect::<sim_kernel::Result<Vec<_>>>()?;
    cx.read_construct(class, values)
}

fn trusted_read_construct_policy() -> ReadPolicy {
    ReadPolicy {
        trust: TrustLevel::TrustedSource,
        capabilities: CapabilitySet::new().grant(read_construct_capability()),
    }
}

fn hyphenated_top_level(expr: Expr) -> Expr {
    let Expr::Map(entries) = expr else {
        return expr;
    };
    Expr::Map(
        entries
            .into_iter()
            .map(|(key, value)| {
                let key = match key {
                    Expr::Symbol(symbol) => {
                        Expr::Symbol(Symbol::new(symbol.as_qualified_str().replace('_', "-")))
                    }
                    other => other,
                };
                (key, value)
            })
            .collect(),
    )
}

fn string_value(cx: &mut Cx, value: &str) -> Value {
    cx.factory().string(value.to_owned()).unwrap()
}

fn integer_value(cx: &mut Cx, value: u64) -> Value {
    sim_citizen::value_from_expr(
        cx,
        &<u64 as sim_citizen::CitizenField>::encode_field(&value),
    )
    .unwrap()
}

fn boxed<T>(cx: &Cx, value: T) -> Value
where
    T: Object + sim_kernel::ObjectCompat + Send + Sync + 'static,
{
    cx.factory().opaque(Arc::new(value)).unwrap()
}
