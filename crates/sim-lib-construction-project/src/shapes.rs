//! Shape contracts for construction Citizens, constructors, and operations.

use std::{marker::PhantomData, sync::Arc};

use sim_kernel::{Cx, Expr, MatchScore, Result, Shape, ShapeDoc, ShapeMatch, Symbol, Value};
use sim_shape::{AnyShape, ExprKind, ExprKindShape, TableExtraPolicy, TableFieldSpec, TableShape};

use crate::{
    AcceptedBaseline, ChangeExposureReport, ChangeRecord, CommissioningReadinessReport,
    CommissioningRequirement, ConstructionExplanationReport, ConstructionStatusReport, GateReport,
    OutcomeRecord, ProductionReadinessSnapshot, ProjectDelta, ProjectFact, ProjectId,
    ProjectSnapshot, ReferenceAdmissionReport, Requirement, ScheduleStatusReport,
    ScheduleTaskJoinSet, WorkPackage, citizen::ConstructionCitizenSpec,
};

/// Returns the registered Shape symbol for one construction Citizen class.
#[must_use]
pub fn construction_type_shape_symbol(class: &Symbol) -> Symbol {
    Symbol::qualified("construction-shape", class.name.to_string())
}

/// Returns every public construction type and operation Shape symbol.
#[must_use]
pub fn construction_shape_symbols() -> Vec<Symbol> {
    let mut symbols = crate::construction_citizen_symbols()
        .iter()
        .map(construction_type_shape_symbol)
        .collect::<Vec<_>>();
    symbols.extend(crate::operations::operation_shape_symbols());
    symbols
}

pub(crate) fn register_type_shapes(linker: &mut sim_kernel::Linker<'_>) -> Result<()> {
    register::<ProjectId>(linker)?;
    register::<ProjectFact>(linker)?;
    register::<AcceptedBaseline>(linker)?;
    register::<Requirement>(linker)?;
    register::<WorkPackage>(linker)?;
    register::<ScheduleTaskJoinSet>(linker)?;
    register::<ChangeRecord>(linker)?;
    register::<CommissioningRequirement>(linker)?;
    register::<OutcomeRecord>(linker)?;
    register::<ProjectSnapshot>(linker)?;
    register::<ProjectDelta>(linker)?;
    register::<GateReport>(linker)?;
    register::<ScheduleStatusReport>(linker)?;
    register::<ProductionReadinessSnapshot>(linker)?;
    register::<ChangeExposureReport>(linker)?;
    register::<CommissioningReadinessReport>(linker)?;
    register::<ReferenceAdmissionReport>(linker)?;
    register::<ConstructionStatusReport>(linker)?;
    register::<ConstructionExplanationReport>(linker)
}

fn register<T: ConstructionCitizenSpec>(linker: &mut sim_kernel::Linker<'_>) -> Result<()> {
    let class = sim_citizen::parse_symbol(T::SYMBOL);
    let symbol = construction_type_shape_symbol(&class);
    linker.shape_value(
        symbol.clone(),
        sim_shape::shape_value(symbol.clone(), type_shape::<T>()),
    )?;
    Ok(())
}

pub(crate) fn type_shape<T: ConstructionCitizenSpec>() -> Arc<dyn Shape> {
    Arc::new(ConstructionTypeShape::<T> {
        symbol: construction_type_shape_symbol(&sim_citizen::parse_symbol(T::SYMBOL)),
        marker: PhantomData,
    })
}

struct ConstructionTypeShape<T> {
    symbol: Symbol,
    marker: PhantomData<T>,
}

impl<T: ConstructionCitizenSpec> Shape for ConstructionTypeShape<T> {
    fn symbol(&self) -> Option<Symbol> {
        Some(self.symbol.clone())
    }

    fn check_value(&self, _cx: &mut Cx, value: Value) -> Result<ShapeMatch> {
        let Some(record) = value.object().downcast_ref::<T>() else {
            return Ok(ShapeMatch::reject(format!(
                "{} Citizen expected; construct it with the matching construction/* constructor",
                T::SYMBOL
            )));
        };
        Ok(match record.validate() {
            Ok(()) => ShapeMatch::accept(MatchScore::exact(100)),
            Err(error) => ShapeMatch::reject(format!("malformed {}: {error}", T::SYMBOL)),
        })
    }

    fn check_expr(&self, cx: &mut Cx, expr: &Expr) -> Result<ShapeMatch> {
        let value = cx.eval_expr(expr.clone())?;
        self.check_value(cx, value)
    }

    fn describe(&self, _cx: &mut Cx) -> Result<ShapeDoc> {
        Ok(ShapeDoc::new(T::SYMBOL).with_detail(
            "durable semantic Citizen; eval reconstructs the object while quote, data, pattern, and surface positions retain its read-construct expression",
        ))
    }
}

pub(crate) fn semantic_map_shape(required: &[&str], optional: &[&str]) -> Arc<dyn Shape> {
    let any: Arc<dyn Shape> = Arc::new(AnyShape);
    let mut fields = required
        .iter()
        .map(|key| TableFieldSpec {
            key: Symbol::new(*key),
            shape: any.clone(),
            required: true,
        })
        .collect::<Vec<_>>();
    fields.extend(optional.iter().map(|key| TableFieldSpec {
        key: Symbol::new(*key),
        shape: any.clone(),
        required: false,
    }));
    Arc::new(TableShape::new(fields, TableExtraPolicy::Reject))
}

pub(crate) fn string_shape() -> Arc<dyn Shape> {
    Arc::new(ExprKindShape::new(ExprKind::String))
}

pub(crate) fn number_shape() -> Arc<dyn Shape> {
    Arc::new(ExprKindShape::new(ExprKind::Number))
}

pub(crate) fn any_shape() -> Arc<dyn Shape> {
    Arc::new(AnyShape)
}
