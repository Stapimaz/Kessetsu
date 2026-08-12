use crate::ast::Cmp;
use crate::ir::{Assertion, CircuitIR, ComponentKind, SIUnit};
use crate::simulation::{Dataset, SimulationResult};
use serde::{Deserialize, Serialize};

pub const ASSERTION_SCHEMA_VERSION: &str = "netlang.assertion.v1";

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
        self.summary.failed == 0 && self.summary.errors == 0 && self.summary.skipped == 0
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
    let code = format!("NL-T{:03}", index + 1);
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

    let actual = match resolve_metric(assertion, circuit, simulation) {
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

fn resolve_metric(
    assertion: &Assertion,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let metric = assertion.metric.to_ascii_lowercase();
    if !matches!(
        metric.as_str(),
        "value" | "min" | "max" | "peak" | "average" | "avg" | "rms"
    ) {
        return Err(format!(
            "unsupported assertion metric '{}'; supported metrics are value, min, max, peak, average and rms",
            assertion.metric
        ));
    }

    for dataset in &simulation.datasets {
        match resolve_dataset_signal(&dataset.data, &assertion.signal, circuit) {
            Ok(Some(SignalValues::Scalar(value))) => return reduce_scalar(&metric, value),
            Ok(Some(SignalValues::Series(values))) => return reduce_series(&metric, values),
            Ok(None) => continue,
            Err(message) => return Err(message),
        }
    }

    resolve_measurement_fallback(&metric, &assertion.signal, simulation).ok_or_else(|| {
        format!(
            "measurement '{}({})' was not produced by the simulation",
            assertion.metric, assertion.signal
        )
    })
}

enum SignalValues<'a> {
    Scalar(f64),
    Series(&'a [f64]),
}

fn resolve_dataset_signal<'a>(
    dataset: &'a Dataset,
    signal: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalValues<'a>>, String> {
    let (function, target) = parse_signal(signal)?;
    let vector = match function {
        SignalFunction::Voltage => target.to_ascii_lowercase(),
        SignalFunction::Current => current_vector_name(target, circuit)?,
    };

    match dataset {
        Dataset::OperatingPoint { values } => {
            Ok(values.get(&vector).copied().map(SignalValues::Scalar))
        }
        Dataset::Transient(series) | Dataset::DcSweep(series) => Ok(series
            .signals
            .get(&vector)
            .map(|values| SignalValues::Series(values))),
        Dataset::Ac(_) => Err(
            "min/max/peak/average/rms assertions over complex AC data are not supported; use a frequency-domain engineering metric"
                .to_string(),
        ),
    }
}

#[derive(Clone, Copy)]
enum SignalFunction {
    Voltage,
    Current,
}

fn parse_signal(signal: &str) -> Result<(SignalFunction, &str), String> {
    let Some((function, target)) = signal.split_once('(') else {
        return Err(format!("invalid assertion signal '{signal}'"));
    };
    let Some(target) = target.strip_suffix(')') else {
        return Err(format!("invalid assertion signal '{signal}'"));
    };
    let function = if function.eq_ignore_ascii_case("V") {
        SignalFunction::Voltage
    } else if function.eq_ignore_ascii_case("I") {
        SignalFunction::Current
    } else {
        return Err(format!("unsupported assertion signal '{signal}'"));
    };
    Ok((function, target))
}

fn current_vector_name(target: &str, circuit: &CircuitIR) -> Result<String, String> {
    let component = circuit
        .components
        .iter()
        .find(|component| component.id.eq_ignore_ascii_case(target))
        .ok_or_else(|| format!("current target '{target}' is not a declared component"))?;
    let prefix = match component.kind {
        ComponentKind::VoltageSource => "v_",
        ComponentKind::Inductor => "l_",
        _ => {
            return Err(format!(
                "structured branch current for '{}' ({:?}) is not available in this analysis",
                component.id, component.kind
            ));
        }
    };
    Ok(format!(
        "{prefix}{}#branch",
        component.id.to_ascii_lowercase()
    ))
}

fn reduce_scalar(metric: &str, value: f64) -> Result<f64, String> {
    match metric {
        "value" | "min" | "max" | "average" | "avg" => Ok(value),
        "peak" | "rms" => Ok(value.abs()),
        _ => Err(format!(
            "metric '{metric}' is not supported for operating-point data"
        )),
    }
}

fn reduce_series(metric: &str, values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("measurement series is empty".to_string());
    }
    match metric {
        "value" => values
            .last()
            .copied()
            .ok_or_else(|| "measurement series is empty".to_string()),
        "min" => Ok(values.iter().copied().fold(f64::INFINITY, f64::min)),
        "max" => Ok(values.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        "peak" => Ok(values.iter().copied().map(f64::abs).fold(0.0, f64::max)),
        "average" | "avg" => Ok(values.iter().sum::<f64>() / values.len() as f64),
        "rms" => Ok(
            (values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64).sqrt(),
        ),
        _ => Err(format!(
            "metric '{metric}' is not supported for series data"
        )),
    }
}

fn resolve_measurement_fallback(
    metric: &str,
    signal: &str,
    simulation: &SimulationResult,
) -> Option<f64> {
    let safe_signal = signal
        .replace('(', "_")
        .replace(')', "")
        .to_ascii_lowercase();
    if metric == "peak" {
        let positive = simulation
            .measurements
            .get(&format!("peak_pos_{safe_signal}"));
        let negative = simulation
            .measurements
            .get(&format!("peak_neg_{safe_signal}"));
        if let (Some(positive), Some(negative)) = (positive, negative) {
            return Some(positive.abs().max(negative.abs()));
        }
    }
    let key = format!("{metric}_{safe_signal}");
    simulation.measurements.get(&key).copied().map(|value| {
        if matches!(metric, "peak" | "rms") {
            value.abs()
        } else {
            value
        }
    })
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
    };
    format!("{scaled:.6}{prefix}{suffix}")
}
