use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    BaselineId, CommercialAmount, ConstructionProjectError, ControlId, CurrencyCode, ForecastBasis,
    ForecastConsequence, ForecastConsequenceKind, ForecastValue, OpenRating, ProjectId,
    RatingValue, ResponseState, RoleId, UncertaintyKind, UncertaintyRecord, UncertaintyResponse,
    UncertaintyState,
};
// conformance: construction risk opportunity ratings realization and capture

use sim_ledger::Amount;

#[test]
fn risk_and_opportunity_retain_complete_accountable_uncertainty_records() {
    let risk = uncertainty(UncertaintyKind::Risk);
    risk.validate().unwrap();
    assert!(matches!(risk.likelihood.value, RatingValue::Qualitative(_)));
    assert!(matches!(risk.impact.value, RatingValue::Quantified { .. }));

    let opportunity = uncertainty(UncertaintyKind::Opportunity)
        .with_state(UncertaintyState::OpportunityCaptured { fact_seq: 4 });
    opportunity.validate().unwrap();
    assert_eq!(opportunity.response.authority, Some(role("project-chief")));
}

#[test]
fn risk_realization_and_opportunity_capture_are_kind_checked() {
    uncertainty(UncertaintyKind::Risk)
        .with_state(UncertaintyState::RiskRealized { fact_seq: 4 })
        .validate()
        .unwrap();

    let mismatched = uncertainty(UncertaintyKind::Opportunity)
        .with_state(UncertaintyState::RiskRealized { fact_seq: 4 });
    assert!(matches!(
        mismatched.validate(),
        Err(ConstructionProjectError::UncertaintyStateMismatch { .. })
    ));
}

#[test]
fn stale_rating_remains_visible_after_a_current_fact_change() {
    let mut risk = uncertainty(UncertaintyKind::Risk);
    risk.fact_seq = 5;
    assert!(risk.has_stale_rating());
    risk.validate().unwrap();
}

#[test]
fn forecast_consequences_preserve_every_typed_lane_and_method_basis() {
    let uncertainty = control("risk.switchgear");
    let basis = ForecastBasis::new(baseline(), "discipline forecast", 4, date(20));
    let kinds_and_values = [
        (
            ForecastConsequenceKind::Time,
            ForecastValue::TimeInterval {
                start_on: date(24),
                finish_on: date(28),
            },
        ),
        (
            ForecastConsequenceKind::Amount,
            ForecastValue::Amount(
                CommercialAmount::parse("125000.00", CurrencyCode::new("SEK").unwrap()).unwrap(),
            ),
        ),
        (
            ForecastConsequenceKind::Safety,
            ForecastValue::Qualitative("energization overlap".to_owned()),
        ),
        (
            ForecastConsequenceKind::Quality,
            ForecastValue::Quantified {
                value: 12,
                unit: "inspection-points".to_owned(),
            },
        ),
        (
            ForecastConsequenceKind::Environment,
            ForecastValue::Quantified {
                value: 4,
                unit: "extra-deliveries".to_owned(),
            },
        ),
        (
            ForecastConsequenceKind::Sustainability,
            ForecastValue::Quantified {
                value: 800,
                unit: "kg-co2e".to_owned(),
            },
        ),
        (
            ForecastConsequenceKind::People,
            ForecastValue::Qualitative("night shift pressure".to_owned()),
        ),
        (
            ForecastConsequenceKind::Place,
            ForecastValue::Qualitative("loading-bay congestion".to_owned()),
        ),
        (
            ForecastConsequenceKind::Customer,
            ForecastValue::Qualitative("opening confidence reduced".to_owned()),
        ),
    ];

    for (index, (kind, value)) in kinds_and_values.into_iter().enumerate() {
        ForecastConsequence::new(
            project(),
            control(&format!("forecast.{}", index + 1)),
            uncertainty.clone(),
            4,
            control("scenario.accepted"),
            kind,
            value,
            basis.clone(),
        )
        .affects(control("package.electrical"))
        .with_evidence(evidence("forecast"))
        .validate()
        .unwrap();
    }

    let wrong = ForecastConsequence::new(
        project(),
        control("forecast.wrong"),
        uncertainty,
        4,
        control("scenario.accepted"),
        ForecastConsequenceKind::Amount,
        ForecastValue::Qualitative("about a lot".to_owned()),
        basis,
    )
    .affects(control("package.electrical"))
    .with_evidence(evidence("forecast"));
    assert!(matches!(
        wrong.validate(),
        Err(ConstructionProjectError::ForecastValueMismatch { .. })
    ));
    assert_eq!(Amount::parse("125000.00").unwrap().0, 12_500_000);
}

fn uncertainty(kind: UncertaintyKind) -> UncertaintyRecord {
    let likelihood = OpenRating::qualitative(
        "project/risk-matrix",
        "possible",
        4,
        date(20),
        "facilitated review",
    );
    let impact = OpenRating::quantified(
        "project/impact-score",
        4,
        "five-point-scale",
        4,
        date(20),
        "discipline estimate",
    );
    let response = UncertaintyResponse::new(
        "qualify the alternate supplier",
        "switchgear submittal is rejected",
        date(28),
        date(24),
        5,
    )
    .with_authority(role("project-chief"))
    .trigger_crossed_at(4)
    .with_state(ResponseState::InProgress);
    let record = match kind {
        UncertaintyKind::Risk => UncertaintyRecord::risk(
            project(),
            control("risk.switchgear"),
            4,
            baseline(),
            control("scenario.accepted"),
            "single-source switchgear",
            "the selected supplier misses approval",
            "energization and customer opening move",
            role("package-lead"),
            response,
            likelihood,
            impact,
        ),
        UncertaintyKind::Opportunity => UncertaintyRecord::opportunity(
            project(),
            control("opportunity.prefabrication"),
            4,
            baseline(),
            control("scenario.accepted"),
            "repeatable riser geometry",
            "the supplier accepts off-site assembly",
            "installation duration and site exposure reduce",
            role("package-lead"),
            response,
            likelihood,
            impact,
        ),
    };
    record
        .affects(control("package.electrical"))
        .with_evidence(evidence("uncertainty"))
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn baseline() -> BaselineId {
    BaselineId::new("baseline.control-1").unwrap()
}

fn role(value: &str) -> RoleId {
    RoleId::new(value).unwrap()
}

fn control(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

fn evidence(id: &str) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("risk/reference-center/{id}"),
        Some("rev-a".to_owned()),
        None,
    )
}
