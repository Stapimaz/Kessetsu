use crate::ast::Cmp;
use crate::ir::{Assertion, CircuitIR, SIUnit};
use crate::simulation::SimulationResult;
use serde::{Deserialize, Serialize};

pub const ASSERTION_SCHEMA_VERSION: &str = "kessetsu.assertion.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssertionStatus {
    Pass,
    Fail,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TolerancePolicy {
    pub absolute: f64,
    pub relative: f64,
}

impl Default for TolerancePolicy {
    fn default() -> Self {
        Self {
            absolute: 1e-9,
            relative: 1e-6,
        }
    }
}

impl TolerancePolicy {
    fn tolerance(self, actual: f64, expected: f64) -> f64 {
        self.absolute
            .max(self.relative * actual.abs().max(expected.abs()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionResult {
    pub code: String,
    pub status: AssertionStatus,
    pub metric: String,
    pub signal: String,
    pub comparator: String,
    pub actual: Option<f64>,
    pub threshold: f64,
    pub unit: SIUnit,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AssertionSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionReport {
    pub schema_version: String,
    pub tolerance: TolerancePolicy,
    pub assertions: Vec<AssertionResult>,
    pub summary: AssertionSummary,
}

impl AssertionReport {
    pub fn all_passed(&self) -> bool {
        self.summary.total > 0
            && self.summary.failed == 0
            && self.summary.errors == 0
            && self.summary.skipped == 0
    }
}

pub fn evaluate_assertions(circuit: &CircuitIR, simulation: &SimulationResult) -> AssertionReport {
    evaluate_assertions_with_policy(circuit, simulation, TolerancePolicy::default())
}

pub fn evaluate_assertions_with_policy(
    circuit: &CircuitIR,
    simulation: &SimulationResult,
    tolerance: TolerancePolicy,
) -> AssertionReport {
    let assertions = circuit
        .assertions
        .iter()
        .enumerate()
        .map(|(index, assertion)| evaluate_one(index, assertion, circuit, simulation, tolerance))
        .collect::<Vec<_>>();
    let mut summary = AssertionSummary {
        total: assertions.len(),
        ..AssertionSummary::default()
    };
    for assertion in &assertions {
        match assertion.status {
            AssertionStatus::Pass => summary.passed += 1,
            AssertionStatus::Fail => summary.failed += 1,
            AssertionStatus::Error => summary.errors += 1,
            AssertionStatus::Skipped => summary.skipped += 1,
        }
    }

    AssertionReport {
        schema_version: ASSERTION_SCHEMA_VERSION.to_string(),
        tolerance,
        assertions,
        summary,
    }
}

fn evaluate_one(
    index: usize,
    assertion: &Assertion,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
    tolerance: TolerancePolicy,
) -> AssertionResult {
    let code = format!("KES-T{:03}", index + 1);
    let comparator = comparator_text(&assertion.cmp).to_string();
    let base = |status, actual, message| AssertionResult {
        code: code.clone(),
        status,
        metric: assertion.metric.to_ascii_lowercase(),
        signal: assertion.signal.clone(),
        comparator: comparator.clone(),
        actual,
        threshold: assertion.threshold.value,
        unit: assertion.threshold.unit,
        message,
    };

    if !simulation.succeeded() {
        return base(
            AssertionStatus::Skipped,
            None,
            Some("simulation did not succeed; assertion was not evaluated".to_string()),
        );
    }

    let actual = match crate::measurement::evaluate_assertion_metric(assertion, circuit, simulation)
    {
        Ok(actual) if actual.is_finite() => actual,
        Ok(_) => {
            return base(
                AssertionStatus::Error,
                None,
                Some("measurement produced a non-finite value".to_string()),
            );
        }
        Err(message) => return base(AssertionStatus::Error, None, Some(message)),
    };
    let passed = evaluate_comparator(actual, assertion.threshold.value, &assertion.cmp, tolerance);
    base(
        if passed {
            AssertionStatus::Pass
        } else {
            AssertionStatus::Fail
        },
        Some(actual),
        (!passed).then(|| {
            format!(
                "measured {} {} threshold {}",
                format_quantity(actual, assertion.threshold.unit),
                comparator,
                format_quantity(assertion.threshold.value, assertion.threshold.unit)
            )
        }),
    )
}

fn evaluate_comparator(
    actual: f64,
    expected: f64,
    comparator: &Cmp,
    policy: TolerancePolicy,
) -> bool {
    let tolerance = policy.tolerance(actual, expected);
    match comparator {
        Cmp::Eq => (actual - expected).abs() <= tolerance,
        Cmp::Lt => actual < expected,
        Cmp::Gt => actual > expected,
        Cmp::Le => actual <= expected + tolerance,
        Cmp::Ge => actual >= expected - tolerance,
    }
}

pub fn comparator_text(comparator: &Cmp) -> &'static str {
    match comparator {
        Cmp::Lt => "<",
        Cmp::Gt => ">",
        Cmp::Le => "<=",
        Cmp::Ge => ">=",
        Cmp::Eq => "==",
    }
}

pub fn format_quantity(value: f64, unit: SIUnit) -> String {
    if unit == SIUnit::Ratio {
        return format!("{value:.6}");
    }
    if unit == SIUnit::Percent {
        return format!("{value:.6}%");
    }
    if unit == SIUnit::Degree {
        return format!("{value:.6}deg");
    }
    let (scaled, prefix) = if value != 0.0 && value.abs() < 1e-9 {
        (value * 1e12, "p")
    } else if value != 0.0 && value.abs() < 1e-6 {
        (value * 1e9, "n")
    } else if value != 0.0 && value.abs() < 1e-3 {
        (value * 1e6, "µ")
    } else if value != 0.0 && value.abs() < 1.0 {
        (value * 1e3, "m")
    } else if value.abs() >= 1e9 {
        (value / 1e9, "G")
    } else if value.abs() >= 1e6 {
        (value / 1e6, "M")
    } else if value.abs() >= 1e3 {
        (value / 1e3, "k")
    } else {
        (value, "")
    };
    let suffix = match unit {
        SIUnit::Ohm => "Ω",
        SIUnit::Farad => "F",
        SIUnit::Henry => "H",
        SIUnit::Volt => "V",
        SIUnit::Ampere => "A",
        SIUnit::Hertz => "Hz",
        SIUnit::Second => "s",
        SIUnit::Watt => "W",
        SIUnit::Joule => "J",
        SIUnit::Ratio => "",
        SIUnit::Percent => "%",
        SIUnit::Degree => "deg",
    };
    format!("{scaled:.6}{prefix}{suffix}")
}
