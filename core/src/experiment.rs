//! Portable local experiments. Pure case generation/evidence; adapters own scheduling and I/O.
use crate::compile_inputs::{CompileInputs, ParameterInput};
use crate::compiler::{
    COMPILE_SCHEMA_VERSION, CompileOptions, CompileReport, compile_source_with_inputs,
};
use crate::graph::format_spice_number;
use crate::ir::{CircuitIR, Quantity, SIUnit, parse_quantity};
use crate::measurement::{MEASUREMENT_SCHEMA_VERSION, evaluate_assertion_metric};
use crate::models::ExternalModelResources;
use crate::requirements::compile_requirements;
use crate::sim_result::{AssertionReport, evaluate_assertions};
use crate::simulation::{Dataset, SimulationResult, SimulatorInfo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EXPERIMENT_SCHEMA: &str = "kessetsu.experiment.v1";
pub const RESULTS_SCHEMA: &str = "kessetsu.experiment-results.v1";
pub const MAX_CASES: usize = 256;
pub const MAX_SPEC_BYTES: usize = 1024 * 1024;
pub const MAX_RESULT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CASE_VALUES: usize = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentSpec {
    pub schema_version: String,
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub requirements: Option<String>,
    #[serde(default)]
    pub axes: Vec<Axis>,
    #[serde(default)]
    pub revisions: Vec<Revision>,
    #[serde(default)]
    pub tolerances: Option<ToleranceStudy>,
    #[serde(default = "default_temperature")]
    pub temperatures_c: Vec<f64>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub measurements: Vec<Measurement>,
    #[serde(default)]
    pub objective: Option<Objective>,
}
fn default_temperature() -> Vec<f64> {
    vec![27.0]
}
fn default_timeout() -> u64 {
    30_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub parameter: String,
    pub values: Values,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Values {
    List {
        values: Vec<String>,
    },
    Linear {
        start: String,
        stop: String,
        points: usize,
    },
    Log {
        start: String,
        stop: String,
        points: usize,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub name: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToleranceStudy {
    pub mode: ToleranceMode,
    #[serde(default)]
    pub seed: u32,
    #[serde(default)]
    pub samples: usize,
    pub parameters: Vec<ToleranceParameter>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceMode {
    Corners,
    MonteCarlo,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToleranceParameter {
    pub parameter: String,
    /// Uniform half-width or Gaussian standard deviation, as a fraction of nominal.
    pub relative: f64,
    #[serde(default)]
    pub distribution: Distribution,
    /// Same group means the same signed random deviation (perfect positive correlation).
    #[serde(default)]
    pub group: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    #[default]
    Uniform,
    Normal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub name: String,
    pub expression: String,
    pub unit: SIUnit,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    pub measurement: String,
    pub direction: Direction,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentCase {
    pub id: String,
    pub revision: usize,
    pub name: String,
    pub parameters: BTreeMap<String, String>,
    pub temperature_c: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentPlan {
    pub identity: String,
    pub source_sha256: String,
    pub requirements_sha256: Option<String>,
    pub core_version: String,
    pub compile_schema: String,
    pub measurement_schema: String,
    pub model_manifests: Vec<crate::ir::ModelManifest>,
    pub spec: ExperimentSpec,
    pub cases: Vec<ExperimentCase>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Pending,
    Completed,
    Passed,
    Failed,
    Error,
    Cancelled,
}
impl CaseStatus {
    pub fn reusable(self) -> bool {
        !matches!(self, Self::Pending | Self::Cancelled)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementValue {
    pub value: Option<f64>,
    pub unit: SIUnit,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: CaseStatus,
    pub measurements: BTreeMap<String, MeasurementValue>,
    pub assertions: Option<AssertionReport>,
    pub simulation: Option<SimulationResult>,
    pub errors: Vec<String>,
    /// Accidental-corruption detection, not a cryptographic attestation.
    pub content_sha256: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub total: usize,
    pub pending: usize,
    pub completed: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub cancelled: usize,
    pub best_case: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub schema_version: String,
    pub identity: String,
    pub plan: ExperimentPlan,
    pub simulator: SimulatorInfo,
    pub solver_fingerprint: String,
    pub cases: Vec<CaseResult>,
    pub summary: ExperimentSummary,
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn digest<T: Serialize>(value: &T) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("validated finite experiment data"))
}
pub fn unit_suffix(unit: SIUnit) -> &'static str {
    match unit {
        SIUnit::Ohm => "Ohm",
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
    }
}
fn literal(value: f64, unit: SIUnit) -> String {
    // Full round-trip precision, unlike display-oriented SPICE formatting.
    format!("{value}{}", unit_suffix(unit))
}
fn parameter_units(circuit: &CircuitIR) -> BTreeMap<String, Quantity> {
    circuit
        .parameter_manifest
        .parameters
        .iter()
        .filter(|p| p.instance_path.is_empty())
        .map(|p| (p.name.clone(), p.resolved.clone()))
        .collect()
}
fn assertion_for(measurement: &Measurement) -> Result<crate::ir::Assertion, String> {
    if measurement.expression.len() > 2048
        || measurement.expression.contains(['\n', '\r', '{', '}'])
    {
        return Err("Study measurements require a bounded literal metric expression".into());
    }
    let text = format!(
        "assert {} >= 0{}\n",
        measurement.expression,
        unit_suffix(measurement.unit)
    );
    let set = compile_requirements(text.as_bytes()).map_err(|e| e.message)?;
    if set.assertions.len() != 1 {
        return Err("Expected one study measurement".into());
    }
    Ok(set.assertions.into_iter().next().unwrap())
}
pub fn decode_spec(bytes: &[u8]) -> Result<ExperimentSpec, String> {
    if bytes.len() > MAX_SPEC_BYTES {
        return Err("Experiment specification exceeds 1 MiB".into());
    }
    serde_json::from_slice(bytes).map_err(|e| format!("Invalid experiment specification: {e}"))
}

pub fn plan_experiment(
    spec: ExperimentSpec,
    resources: &ExternalModelResources,
) -> Result<ExperimentPlan, String> {
    if spec.schema_version != EXPERIMENT_SCHEMA {
        return Err(format!("Expected {EXPERIMENT_SCHEMA}"));
    }
    if spec.name.trim().is_empty() || spec.name.len() > 120 {
        return Err("Name must contain 1–120 bytes".into());
    }
    if serde_json::to_vec(&spec).map_err(|e| e.to_string())?.len() > MAX_SPEC_BYTES {
        return Err("Experiment specification exceeds 1 MiB".into());
    }
    if spec.axes.len() > 8 || spec.revisions.len() > 16 || spec.measurements.len() > 32 {
        return Err("Study limit: 8 axes, 16 revisions, 32 measurements".into());
    }
    if !(1..=120_000).contains(&spec.timeout_ms) {
        return Err("Timeout must be 1–120000 ms".into());
    }
    if spec.temperatures_c.is_empty()
        || spec.temperatures_c.len() > 16
        || spec
            .temperatures_c
            .iter()
            .any(|t| !t.is_finite() || !(-200.0..=300.0).contains(t))
    {
        return Err("Supply 1–16 temperatures in -200..300 °C".into());
    }
    let requirements_sha256 = spec
        .requirements
        .as_ref()
        .map(|s| {
            compile_requirements(s.as_bytes())
                .map(|r| r.sha256)
                .map_err(|e| e.message)
        })
        .transpose()?;
    let mut measurement_names = BTreeSet::new();
    for m in &spec.measurements {
        if m.name.is_empty() || m.name.len() > 64 || !measurement_names.insert(m.name.clone()) {
            return Err("Measurement names must be unique and bounded".into());
        }
        assertion_for(m)?;
    }
    if spec
        .objective
        .as_ref()
        .is_some_and(|o| !measurement_names.contains(&o.measurement))
    {
        return Err("Objective must name a declared measurement".into());
    }
    let revisions = if spec.revisions.is_empty() {
        vec![Revision {
            name: "Design".into(),
            source: None,
            parameters: BTreeMap::new(),
        }]
    } else {
        spec.revisions.clone()
    };
    let mut cases = Vec::new();
    let mut model_identities = Vec::new();
    let mut random = Random(spec.tolerances.as_ref().map_or(0, |t| t.seed));
    for (revision_index, revision) in revisions.iter().enumerate() {
        if revision.name.is_empty() || revision.name.len() > 120 {
            return Err("Revision names must be bounded and nonempty".into());
        }
        let source = revision.source.as_deref().unwrap_or(&spec.source);
        let base = compile_source_with_inputs(
            source,
            CompileOptions::default(),
            &inputs(&revision.parameters),
            resources,
        );
        if base.has_errors() {
            return Err(format!(
                "Revision '{}': {}",
                revision.name,
                diagnostic_text(&base)
            ));
        }
        let circuit = base.ir.as_ref().ok_or("Missing circuit IR")?;
        if circuit.analyses.is_empty() {
            return Err("Add at least one simulate analysis to the circuit".into());
        }
        if requirements_sha256.is_some() && !circuit.assertions.is_empty() {
            return Err("Independent requirements cannot be mixed with inline assertions".into());
        }
        model_identities.push(circuit.model_manifest.clone());
        let units = parameter_units(circuit);
        let mut combinations = vec![revision.parameters.clone()];
        let mut axis_names = BTreeSet::new();
        for axis in &spec.axes {
            if !axis_names.insert(&axis.parameter) {
                return Err("Duplicate sweep parameter".into());
            }
            let unit = units
                .get(&axis.parameter)
                .ok_or_else(|| format!("Unknown root parameter '{}'", axis.parameter))?
                .unit;
            let values = axis_values(&axis.values, unit)?;
            combinations = product(combinations, &axis.parameter, &values)?;
        }
        let mut expanded = Vec::new();
        for combination in combinations {
            // Nominals can depend on the current grid point.
            let point = compile_source_with_inputs(
                source,
                CompileOptions::default(),
                &inputs(&combination),
                resources,
            );
            if let Some(tolerance) = &spec.tolerances {
                if point.has_errors() {
                    return Err(format!("Tolerance nominal: {}", diagnostic_text(&point)));
                }
                expanded.extend(tolerance_cases(
                    combination,
                    tolerance,
                    &parameter_units(point.ir.as_ref().unwrap()),
                    &axis_names,
                    &mut random,
                )?);
            } else {
                expanded.push(combination);
            }
            if expanded.len() > MAX_CASES {
                return Err("Study exceeds 256 cases".into());
            }
        }
        for parameters in expanded {
            for temperature_c in &spec.temperatures_c {
                if cases.len() >= MAX_CASES {
                    return Err("Study exceeds 256 cases".into());
                }
                let content = digest(&(source, &parameters, temperature_c));
                cases.push(ExperimentCase {
                    id: format!("case-{:04}-{}", cases.len() + 1, &content[7..19]),
                    revision: revision_index,
                    name: revision.name.clone(),
                    parameters: parameters.clone(),
                    temperature_c: *temperature_c,
                });
            }
        }
    }
    let core_version = env!("CARGO_PKG_VERSION").to_owned();
    let identity = digest(&(
        &spec,
        &cases,
        &model_identities,
        &core_version,
        COMPILE_SCHEMA_VERSION,
        MEASUREMENT_SCHEMA_VERSION,
    ));
    Ok(ExperimentPlan {
        identity,
        source_sha256: hash_bytes(spec.source.as_bytes()),
        requirements_sha256,
        core_version,
        compile_schema: COMPILE_SCHEMA_VERSION.into(),
        measurement_schema: MEASUREMENT_SCHEMA_VERSION.into(),
        model_manifests: model_identities,
        spec,
        cases,
    })
}

fn axis_values(values: &Values, unit: SIUnit) -> Result<Vec<String>, String> {
    match values {
        Values::List { values } => {
            if values.is_empty() || values.len() > MAX_CASES {
                return Err("List needs 1–256 values".into());
            }
            values
                .iter()
                .map(|v| parse_quantity(v, unit).map(|q| literal(q.value, unit)))
                .collect()
        }
        Values::Linear {
            start,
            stop,
            points,
        }
        | Values::Log {
            start,
            stop,
            points,
        } => {
            if !(2..=MAX_CASES).contains(points) {
                return Err("Grid needs 2–256 points".into());
            }
            let a = parse_quantity(start, unit)?.value;
            let b = parse_quantity(stop, unit)?.value;
            let logarithmic = matches!(values, Values::Log { .. });
            if a >= b || (logarithmic && a <= 0.0) {
                return Err("Grid needs start < stop; log endpoints must be positive".into());
            }
            (0..*points)
                .map(|i| {
                    let fraction = i as f64 / (*points - 1) as f64;
                    let value = if i == 0 {
                        a
                    } else if i == *points - 1 {
                        b
                    } else if logarithmic {
                        (a.ln() + fraction * (b.ln() - a.ln())).exp()
                    } else {
                        a * (1.0 - fraction) + b * fraction
                    };
                    if !value.is_finite() {
                        Err("Non-finite grid value".into())
                    } else {
                        Ok(literal(value, unit))
                    }
                })
                .collect()
        }
    }
}
fn product(
    base: Vec<BTreeMap<String, String>>,
    name: &str,
    values: &[String],
) -> Result<Vec<BTreeMap<String, String>>, String> {
    if base.len().saturating_mul(values.len()) > MAX_CASES {
        return Err("Study exceeds 256 cases".into());
    }
    Ok(base
        .into_iter()
        .flat_map(|point| {
            values.iter().map(move |v| {
                let mut point = point.clone();
                point.insert(name.into(), v.clone());
                point
            })
        })
        .collect())
}
struct Random(u32);
impl Random {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b9);
        let mut x = self.0;
        x ^= x >> 16;
        x = x.wrapping_mul(0x21f0aaad);
        x ^= x >> 15;
        x = x.wrapping_mul(0x735a2d97);
        x ^= x >> 15;
        (f64::from(x) + 0.5) / 4294967296.0
    }
    fn deviation(&mut self, distribution: Distribution) -> f64 {
        match distribution {
            Distribution::Uniform => 2.0 * self.uniform() - 1.0,
            Distribution::Normal => {
                (-2.0 * self.uniform().ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()
            }
        }
    }
}
fn tolerance_cases(
    base: BTreeMap<String, String>,
    study: &ToleranceStudy,
    nominal: &BTreeMap<String, Quantity>,
    axes: &BTreeSet<&String>,
    random: &mut Random,
) -> Result<Vec<BTreeMap<String, String>>, String> {
    if study.parameters.is_empty() || study.parameters.len() > 8 {
        return Err("Tolerance study needs 1–8 parameters".into());
    }
    let mut names = BTreeSet::new();
    let mut groups = BTreeMap::new();
    for p in &study.parameters {
        if !p.relative.is_finite()
            || p.relative <= 0.0
            || p.relative >= 1.0
            || !nominal.contains_key(&p.parameter)
            || !names.insert(&p.parameter)
            || axes.contains(&p.parameter)
        {
            return Err(
                "Tolerance needs unique root parameters not on sweep axes, with 0 < relative < 1"
                    .into(),
            );
        }
        if let Some(group) = &p.group {
            if group.is_empty() || group.len() > 64 {
                return Err("Correlation group must be bounded and nonempty".into());
            }
            if groups
                .insert(group.clone(), p.distribution)
                .is_some_and(|d| d != p.distribution)
            {
                return Err("Correlated parameters must use the same distribution".into());
            }
        }
        if study.mode == ToleranceMode::Corners
            && (p.distribution != Distribution::Uniform || p.group.is_some())
        {
            return Err("Corners require ungrouped uniform half-widths".into());
        }
    }
    let count = match study.mode {
        ToleranceMode::Corners => 1 << study.parameters.len(),
        ToleranceMode::MonteCarlo => study.samples,
    };
    if count == 0 || count >= MAX_CASES {
        return Err("Study needs 1–255 non-nominal samples/corners".into());
    }
    let mut output = vec![base.clone()];
    for index in 0..count {
        let mut point = base.clone();
        let mut deviations = BTreeMap::new();
        for (parameter_index, p) in study.parameters.iter().enumerate() {
            let deviation = if study.mode == ToleranceMode::Corners {
                if (index >> parameter_index) & 1 == 0 {
                    -1.0
                } else {
                    1.0
                }
            } else if let Some(group) = &p.group {
                *deviations
                    .entry(group.clone())
                    .or_insert_with(|| random.deviation(p.distribution))
            } else {
                random.deviation(p.distribution)
            };
            let quantity = &nominal[&p.parameter];
            let value = quantity.value * (1.0 + p.relative * deviation);
            if !value.is_finite() {
                return Err("Non-finite tolerance sample".into());
            }
            point.insert(p.parameter.clone(), literal(value, quantity.unit));
        }
        output.push(point);
    }
    Ok(output)
}
fn inputs(parameters: &BTreeMap<String, String>) -> CompileInputs {
    CompileInputs {
        parameters: parameters
            .iter()
            .map(|(name, value)| ParameterInput {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
        ..Default::default()
    }
}
pub fn diagnostic_text(report: &CompileReport) -> String {
    report
        .diagnostics
        .iter()
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect::<Vec<_>>()
        .join("; ")
}
pub fn case_source<'a>(plan: &'a ExperimentPlan, case: &ExperimentCase) -> &'a str {
    plan.spec
        .revisions
        .get(case.revision)
        .and_then(|r| r.source.as_deref())
        .unwrap_or(&plan.spec.source)
}
pub fn compile_case(
    plan: &ExperimentPlan,
    case: &ExperimentCase,
    resources: &ExternalModelResources,
) -> Result<CompileReport, String> {
    let mut report = compile_source_with_inputs(
        case_source(plan, case),
        CompileOptions::default(),
        &inputs(&case.parameters),
        resources,
    );
    if report.has_errors() {
        return Err(diagnostic_text(&report));
    }
    for model in &report
        .ir
        .as_ref()
        .ok_or("Missing circuit IR")?
        .model_manifest
        .models
    {
        if let Some(external) = &model.external
            && let Some(bytes) = resources.get(&external.resource)
        {
            let body = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
            if body.lines().any(|line| {
                let line = line.trim().to_ascii_lowercase();
                line.starts_with(".control")
                    || line.starts_with(".temp")
                    || (line.starts_with(".options")
                        && line
                            .split_whitespace()
                            .any(|word| word.starts_with("temp=")))
            }) {
                return Err("Study model libraries cannot override temperature or define controls; use a compatible library with an explicit updated identity".into());
            }
        }
    }
    if let Some(requirements) = &plan.spec.requirements {
        let set = compile_requirements(requirements.as_bytes()).map_err(|e| e.message)?;
        let circuit = report.ir.as_mut().ok_or("Missing circuit IR")?;
        if !circuit.assertions.is_empty() {
            return Err("Cannot mix inline and independent requirements".into());
        }
        circuit.assertions = set.assertions;
        // Regenerate from IR, never append textual assertions to source.
        report.spice_netlist = Some(crate::graph::generate_spice(
            circuit,
            &crate::graph::NetlistGraph::build(circuit),
        ));
    }
    Ok(report)
}
pub fn temperature_netlist(netlist: &str, temperature_c: f64) -> Result<String, String> {
    if !temperature_c.is_finite() || !(-200.0..=300.0).contains(&temperature_c) {
        return Err("Invalid study temperature".into());
    }
    let newline = netlist
        .find("\n.control")
        .or_else(|| netlist.rfind("\n.end\n"))
        .map(|i| i + 1)
        .unwrap_or(netlist.len());
    let mut output = netlist.to_owned();
    output.insert_str(
        newline,
        &format!(".temp {}\n", format_spice_number(temperature_c)),
    );
    Ok(output)
}
fn seal(mut row: CaseResult) -> CaseResult {
    row.content_sha256.clear();
    row.content_sha256 = digest(&row);
    row
}
pub fn failed_case(case_id: &str, status: CaseStatus, error: String) -> CaseResult {
    seal(CaseResult {
        case_id: case_id.into(),
        status,
        measurements: BTreeMap::new(),
        assertions: None,
        simulation: None,
        errors: vec![error],
        content_sha256: String::new(),
    })
}
fn validate_simulation(simulation: &SimulationResult) -> Result<(), String> {
    if simulation.schema_version != crate::simulation::SIMULATION_SCHEMA_VERSION {
        return Err("Unsupported simulation result schema".into());
    }
    if simulation.succeeded()
        && (!simulation.process.success || simulation.datasets.len() != simulation.analyses.len())
    {
        return Err("Incomplete or unsuccessful simulator evidence".into());
    }
    let mut count = 0usize;
    let mut seen = BTreeSet::new();
    for dataset in &simulation.datasets {
        if !seen.insert(dataset.index) {
            return Err("Duplicate analysis dataset".into());
        }
        if simulation.analyses.get(dataset.index) != Some(&dataset.analysis)
            || !matches!(
                (&dataset.analysis, &dataset.data),
                (
                    crate::ir::Analysis::OperatingPoint,
                    Dataset::OperatingPoint { .. }
                ) | (crate::ir::Analysis::Transient { .. }, Dataset::Transient(_))
                    | (crate::ir::Analysis::DcSweep { .. }, Dataset::DcSweep(_))
                    | (crate::ir::Analysis::Ac { .. }, Dataset::Ac(_))
            )
        {
            return Err("Dataset kind/index does not match its analysis".into());
        }
        match &dataset.data {
            Dataset::OperatingPoint { values } => {
                count += values.len();
                if values.is_empty() || values.values().any(|v| !v.is_finite()) {
                    return Err("Non-finite OP data".into());
                }
            }
            Dataset::Transient(data) | Dataset::DcSweep(data) => {
                let axis = &data.axis.values;
                let descending = matches!(dataset.data, Dataset::DcSweep(_))
                    && axis.first().zip(axis.last()).is_some_and(|(a, b)| a > b);
                if axis.is_empty()
                    || axis.iter().any(|v| !v.is_finite())
                    || axis.windows(2).any(|w| {
                        if descending {
                            w[0] <= w[1]
                        } else {
                            w[0] >= w[1]
                        }
                    })
                {
                    return Err("Malformed result axis".into());
                }
                count += axis.len();
                for values in data.signals.values() {
                    count = count.saturating_add(values.len());
                    if values.len() != axis.len() || values.iter().any(|v| !v.is_finite()) {
                        return Err("Malformed signal data".into());
                    }
                }
            }
            Dataset::Ac(data) => {
                let axis = &data.frequency_hz;
                if axis.is_empty()
                    || axis.iter().any(|v| !v.is_finite() || *v <= 0.0)
                    || axis.windows(2).any(|w| w[0] >= w[1])
                {
                    return Err("Malformed AC axis".into());
                }
                count += axis.len();
                for values in data.signals.values() {
                    count = count
                        .saturating_add(values.real.len())
                        .saturating_add(values.imaginary.len());
                    if values.real.len() != axis.len()
                        || values.imaginary.len() != axis.len()
                        || values
                            .real
                            .iter()
                            .chain(&values.imaginary)
                            .any(|v| !v.is_finite())
                    {
                        return Err("Malformed complex signal data".into());
                    }
                }
            }
        }
    }
    if count > MAX_CASE_VALUES {
        return Err("Case exceeds 2 million retained numeric values; narrow the study".into());
    }
    Ok(())
}
pub fn evaluate_case(
    plan: &ExperimentPlan,
    case: &ExperimentCase,
    report: &CompileReport,
    mut simulation: SimulationResult,
) -> CaseResult {
    if let Err(error) = validate_simulation(&simulation) {
        return failed_case(&case.id, CaseStatus::Error, error);
    }
    let Some(circuit) = &report.ir else {
        return failed_case(&case.id, CaseStatus::Error, "Missing circuit IR".into());
    };
    if simulation.analyses != circuit.analyses
        || simulation
            .datasets
            .iter()
            .any(|d| circuit.analyses.get(d.index) != Some(&d.analysis))
    {
        return failed_case(
            &case.id,
            CaseStatus::Error,
            "Simulation analysis identity does not match this case".into(),
        );
    }
    let assertions = evaluate_assertions(circuit, &simulation);
    let measurements = plan
        .spec
        .measurements
        .iter()
        .map(|measurement| {
            let value = if simulation.succeeded() {
                assertion_for(measurement)
                    .and_then(|a| evaluate_assertion_metric(&a, circuit, &simulation))
            } else {
                Err("Simulation did not succeed".into())
            };
            let value = match value {
                Ok(v) if v.is_finite() => MeasurementValue {
                    value: Some(v),
                    unit: measurement.unit,
                    error: None,
                },
                Ok(_) => MeasurementValue {
                    value: None,
                    unit: measurement.unit,
                    error: Some("Non-finite measurement".into()),
                },
                Err(e) => MeasurementValue {
                    value: None,
                    unit: measurement.unit,
                    error: Some(e),
                },
            };
            (measurement.name.clone(), value)
        })
        .collect::<BTreeMap<_, _>>();
    let status = if simulation.status == crate::simulation::SimulationStatus::Cancelled {
        CaseStatus::Cancelled
    } else if !simulation.succeeded()
        || assertions.summary.errors > 0
        || assertions.summary.skipped > 0
        || measurements.values().any(|v| v.error.is_some())
    {
        CaseStatus::Error
    } else if assertions.summary.failed > 0 {
        CaseStatus::Failed
    } else if assertions.summary.total > 0 {
        CaseStatus::Passed
    } else {
        CaseStatus::Completed
    };
    simulation.raw_log.stdout.clear();
    simulation.raw_log.stderr.clear();
    simulation.artifacts.clear();
    seal(CaseResult {
        case_id: case.id.clone(),
        status,
        measurements,
        assertions: Some(assertions),
        errors: simulation.errors.clone(),
        simulation: Some(simulation),
        content_sha256: String::new(),
    })
}
pub fn new_results(
    plan: ExperimentPlan,
    simulator: SimulatorInfo,
    solver_fingerprint: String,
) -> ExperimentResults {
    let cases = plan
        .cases
        .iter()
        .map(|c| {
            seal(CaseResult {
                case_id: c.id.clone(),
                status: CaseStatus::Pending,
                measurements: BTreeMap::new(),
                assertions: None,
                simulation: None,
                errors: Vec::new(),
                content_sha256: String::new(),
            })
        })
        .collect();
    let mut output = ExperimentResults {
        schema_version: RESULTS_SCHEMA.into(),
        identity: digest(&(&plan.identity, &simulator, &solver_fingerprint)),
        plan,
        simulator,
        solver_fingerprint,
        cases,
        summary: ExperimentSummary::default(),
    };
    summarize(&mut output);
    output
}
pub fn summarize(results: &mut ExperimentResults) {
    let mut summary = ExperimentSummary {
        total: results.plan.cases.len(),
        ..Default::default()
    };
    let mut best = None::<(&str, f64)>;
    for row in &results.cases {
        match row.status {
            CaseStatus::Pending => summary.pending += 1,
            CaseStatus::Completed => summary.completed += 1,
            CaseStatus::Passed => summary.passed += 1,
            CaseStatus::Failed => summary.failed += 1,
            CaseStatus::Error => summary.errors += 1,
            CaseStatus::Cancelled => summary.cancelled += 1,
        }
        if let Some(objective) = &results.plan.spec.objective
            && matches!(row.status, CaseStatus::Passed | CaseStatus::Completed)
            && let Some(value) = row
                .measurements
                .get(&objective.measurement)
                .and_then(|m| m.value)
            && best.is_none_or(|(_, old)| match objective.direction {
                Direction::Minimize => value < old,
                Direction::Maximize => value > old,
            })
        {
            best = Some((&row.case_id, value));
        }
    }
    summary.best_case = best.map(|(id, _)| id.to_owned());
    results.summary = summary;
}
pub fn validate_results(
    results: &ExperimentResults,
    plan: &ExperimentPlan,
    simulator: &SimulatorInfo,
    solver_fingerprint: &str,
) -> Result<(), String> {
    if plan.cases.is_empty()
        || plan.cases.len() > MAX_CASES
        || serde_json::to_vec(&plan.spec)
            .map_err(|e| e.to_string())?
            .len()
            > MAX_SPEC_BYTES
        || results.cases.iter().any(|row| {
            row.measurements
                .values()
                .any(|m| m.value.is_some_and(|v| !v.is_finite()))
        })
    {
        return Err("Invalid experiment size or non-finite measurement".into());
    }
    if results.schema_version != RESULTS_SCHEMA
        || results.plan.identity != plan.identity
        || results.identity != digest(&(&plan.identity, simulator, solver_fingerprint))
        || results.cases.len() != plan.cases.len()
    {
        return Err("Cannot resume: specification, models, requirements, Core or simulator identity changed".into());
    }
    if digest(&results.plan) != digest(plan) || results.simulator != *simulator {
        return Err("Cannot resume: plan/simulator contents changed".into());
    }
    for (row, case) in results.cases.iter().zip(&plan.cases) {
        if row.case_id != case.id || seal(row.clone()).content_sha256 != row.content_sha256 {
            return Err("Invalid result case identity/content checksum".into());
        }
        if let Some(simulation) = &row.simulation {
            validate_simulation(simulation)?;
            if simulation.simulator != *simulator {
                return Err("Case used a different simulator".into());
            }
        }
    }
    Ok(())
}
pub fn results_csv(results: &ExperimentResults) -> String {
    let quote = csv_quote;
    let mut output =
        "case,revision,status,temperature_c,parameters,measurement,value,unit,error\n".to_owned();
    for (case, row) in results.plan.cases.iter().zip(&results.cases) {
        let status = serde_json::to_string(&row.status)
            .unwrap()
            .trim_matches('"')
            .to_owned();
        let parameters = serde_json::to_string(&case.parameters).unwrap();
        let prefix = format!(
            "{},{},{},{},{}",
            quote(&case.id),
            quote(&case.name),
            status,
            case.temperature_c,
            quote(&parameters)
        );
        if row.measurements.is_empty() {
            output.push_str(&format!("{prefix},,,,{}\n", quote(&row.errors.join("; "))));
        }
        for (name, m) in &row.measurements {
            output.push_str(&format!(
                "{prefix},{},{},{},{}\n",
                quote(name),
                m.value.map(|v| v.to_string()).unwrap_or_default(),
                unit_suffix(m.unit),
                quote(m.error.as_deref().unwrap_or(""))
            ));
        }
    }
    output
}
pub fn datasets_csv(results: &ExperimentResults) -> String {
    let mut output = "case,analysis,axis,signal,real,imaginary\n".to_owned();
    let quote = csv_quote;
    for row in &results.cases {
        if let Some(simulation) = &row.simulation {
            for dataset in &simulation.datasets {
                match &dataset.data {
                    Dataset::OperatingPoint { values } => {
                        for (name, value) in values {
                            output.push_str(&format!(
                                "{},{},,{},{},\n",
                                quote(&row.case_id),
                                dataset.index,
                                quote(name),
                                value
                            ));
                        }
                    }
                    Dataset::Transient(data) | Dataset::DcSweep(data) => {
                        for (name, values) in &data.signals {
                            for (x, y) in data.axis.values.iter().zip(values) {
                                output.push_str(&format!(
                                    "{},{},{},{},{},\n",
                                    quote(&row.case_id),
                                    dataset.index,
                                    x,
                                    quote(name),
                                    y
                                ));
                            }
                        }
                    }
                    Dataset::Ac(data) => {
                        for (name, values) in &data.signals {
                            for ((x, real), imaginary) in data
                                .frequency_hz
                                .iter()
                                .zip(&values.real)
                                .zip(&values.imaginary)
                            {
                                output.push_str(&format!(
                                    "{},{},{},{},{},{}\n",
                                    quote(&row.case_id),
                                    dataset.index,
                                    x,
                                    quote(name),
                                    real,
                                    imaginary
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    output
}

fn csv_quote(text: &str) -> String {
    let prefix = if text.trim_start().starts_with(['=', '+', '-', '@']) {
        "'"
    } else {
        ""
    };
    format!("\"{prefix}{}\"", text.replace('"', "\"\""))
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Publication-safe overlay. Full raw samples remain in result JSON/CSV.
pub fn plot_svg(
    results: &ExperimentResults,
    analysis: usize,
    signal: &str,
    selected: &[String],
) -> Result<String, String> {
    let key = signal.to_ascii_lowercase();
    let data_keys = if let Some(name) = key.strip_prefix("v(").and_then(|s| s.strip_suffix(')')) {
        vec![name.to_owned(), key.clone()]
    } else if let Some(name) = key.strip_prefix("i(").and_then(|s| s.strip_suffix(')')) {
        vec![
            format!("{name}#branch"),
            format!("v_{name}#branch"),
            format!("l_{name}#branch"),
            key.clone(),
        ]
    } else {
        vec![key.clone()]
    };
    let mut series = Vec::new();
    let mut logarithmic = None;
    let mut dataset_kind = None;
    let mut axis_label = None;
    for (case, row) in results.plan.cases.iter().zip(&results.cases) {
        if !selected.is_empty() && !selected.contains(&case.id) {
            continue;
        }
        let Some(data) = row
            .simulation
            .as_ref()
            .and_then(|s| s.datasets.iter().find(|d| d.index == analysis))
        else {
            continue;
        };
        let kind = std::mem::discriminant(&data.data);
        if dataset_kind.is_some_and(|old| old != kind) {
            return Err("Cannot overlay different analysis kinds".into());
        }
        dataset_kind = Some(kind);
        let label = match &data.analysis {
            crate::ir::Analysis::Ac { .. } => "Frequency (Hz), logarithmic scale".to_owned(),
            crate::ir::Analysis::Transient { .. } => "Time (s)".to_owned(),
            crate::ir::Analysis::DcSweep { start, .. } => {
                format!("Source sweep ({})", unit_suffix(start.unit))
            }
            _ => continue,
        };
        if axis_label.as_ref().is_some_and(|old| old != &label) {
            return Err("Cannot overlay incompatible axis units".into());
        }
        axis_label = Some(label);
        let (axis, values, log) = match &data.data {
            Dataset::Transient(data) | Dataset::DcSweep(data) => {
                let Some(values) = data_keys.iter().find_map(|key| data.signals.get(key)) else {
                    continue;
                };
                (data.axis.values.clone(), values.clone(), false)
            }
            Dataset::Ac(data) => {
                let Some(values) = data_keys.iter().find_map(|key| data.signals.get(key)) else {
                    continue;
                };
                (
                    data.frequency_hz.clone(),
                    values
                        .real
                        .iter()
                        .zip(&values.imaginary)
                        .map(|(r, i)| r.hypot(*i))
                        .collect(),
                    true,
                )
            }
            _ => continue,
        };
        if logarithmic.is_some_and(|old| old != log) {
            return Err("Cannot overlay different analysis kinds".into());
        }
        logarithmic = Some(log);
        if axis.len() >= 2 {
            series.push((case, axis, values));
        }
    }
    if series.is_empty() {
        return Err("No series for this analysis/signal; choose a signal from full results".into());
    }
    if series.len() > 12 {
        return Err("Choose at most 12 cases for a readable overlay".into());
    }
    let log = logarithmic.unwrap_or(false);
    let transform = |x: f64| if log { x.ln() } else { x };
    let min_x = series
        .iter()
        .flat_map(|(_, x, _)| x.iter().map(|x| transform(*x)))
        .fold(f64::INFINITY, f64::min);
    let max_x = series
        .iter()
        .flat_map(|(_, x, _)| x.iter().map(|x| transform(*x)))
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = series
        .iter()
        .flat_map(|(_, _, y)| y)
        .copied()
        .fold(f64::INFINITY, f64::min);
    let max_y = series
        .iter()
        .flat_map(|(_, _, y)| y)
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    if ![min_x, max_x, min_y, max_y].iter().all(|v| v.is_finite()) || min_x >= max_x {
        return Err("Invalid plot bounds".into());
    }
    let padding = (max_y - min_y).abs().max(max_y.abs() * 0.02).max(1e-12) * 0.08;
    let low = min_y - padding;
    let high = max_y + padding;
    let colors = [
        "#167d8d", "#c46b18", "#7960ac", "#bc426c", "#548428", "#386ccc", "#9a5537", "#497373",
        "#826f12", "#7b4280", "#a13d3d", "#356239",
    ];
    let height = 425 + series.len() * 20;
    let unit = if key.starts_with("v(") {
        "V"
    } else if key.starts_with("i(") {
        "A"
    } else {
        "signal units"
    };
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 900 {height}\" role=\"img\" aria-label=\"{}\"><rect width=\"900\" height=\"{height}\" fill=\"white\"/><g font-family=\"sans-serif\" fill=\"#202b36\"><text x=\"72\" y=\"28\" font-size=\"18\">{}</text><text x=\"72\" y=\"50\" font-size=\"12\">{} · {} · {}</text>",
        escape(&results.plan.spec.name),
        escape(&results.plan.spec.name),
        escape(signal),
        if log { "magnitude" } else { "real samples" },
        unit
    );
    for i in 0..=4 {
        let fraction = i as f64 / 4.0;
        let x = 72.0 + fraction * 790.0;
        let y = 355.0 - fraction * 280.0;
        let x_value = min_x + fraction * (max_x - min_x);
        let x_value = if log { x_value.exp() } else { x_value };
        svg.push_str(&format!("<path d=\"M{x:.2} 75V355M72 {y:.2}H862\" stroke=\"#dae0e5\" fill=\"none\"/><text x=\"{x:.2}\" y=\"375\" font-size=\"11\" text-anchor=\"middle\">{x_value:.2e}</text><text x=\"66\" y=\"{y:.2}\" font-size=\"11\" text-anchor=\"end\">{:.2e}</text>", low + fraction * (high - low)));
    }
    svg.push_str(&format!(
        "<text x=\"460\" y=\"402\" font-size=\"12\" text-anchor=\"middle\">{}</text>",
        escape(axis_label.as_deref().unwrap_or("Analysis axis"))
    ));
    for (index, (case, axis, values)) in series.iter().enumerate() {
        // Min/max buckets preserve visible extrema; never alter persisted arrays.
        let bucket = (axis.len() / 300).max(1);
        let mut indices = BTreeSet::from([0, axis.len() - 1]);
        for start in (0..values.len()).step_by(bucket) {
            let stop = (start + bucket).min(values.len());
            let min = (start..stop)
                .min_by(|a, b| values[*a].total_cmp(&values[*b]))
                .unwrap();
            let max = (start..stop)
                .max_by(|a, b| values[*a].total_cmp(&values[*b]))
                .unwrap();
            indices.insert(min);
            indices.insert(max);
        }
        let points = indices
            .into_iter()
            .map(|i| {
                format!(
                    "{:.2},{:.2}",
                    72.0 + 790.0 * (transform(axis[i]) - min_x) / (max_x - min_x),
                    355.0 - 280.0 * (values[i] - low) / (high - low)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            "<polyline points=\"{points}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.8\"/>",
            colors[index]
        ));
        let caption = format!(
            "{} · {} · {} °C · {}",
            case.id,
            case.name,
            case.temperature_c,
            serde_json::to_string(&case.parameters).unwrap()
        );
        svg.push_str(&format!(
            "<text x=\"72\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
            425 + index * 20,
            colors[index],
            escape(&caption.chars().take(125).collect::<String>())
        ));
    }
    svg.push_str("</g></svg>");
    Ok(svg)
}

pub fn report_html(results: &ExperimentResults) -> String {
    let mut html = format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>{}</title><style>body{{font:15px system-ui;margin:40px;line-height:1.5;color:#202b36}}code{{overflow-wrap:anywhere}}table{{border-collapse:collapse;width:100%}}th,td{{padding:9px;border:1px solid #ccd3d9;text-align:left}}.failed,.error{{color:#a32323}}@media print{{body{{margin:0;font-size:11px}}}}</style><h1>{}</h1><p>Local simulation study; not a physical-hardware guarantee.</p><p>Source <code>{}</code><br>Evidence <code>{}</code><br>Requirements <code>{}</code><br>Core {} · Simulator {} ({})</p><pre>{}</pre><p>All cases remain in the denominator, including pending/cancelled/failed cases. Optimization selects only evaluated feasible cases, not a global optimum. Monte Carlo estimates are conditional on declared distributions, model assumptions and sample count; they are not guaranteed production yield.</p><table><thead><tr><th>Case / conditions</th><th>Status</th><th>Measurements</th><th>Requirements / errors</th></tr></thead><tbody>",
        escape(&results.plan.spec.name),
        escape(&results.plan.spec.name),
        escape(&results.plan.source_sha256),
        escape(&results.identity),
        escape(
            results
                .plan
                .requirements_sha256
                .as_deref()
                .unwrap_or("design-owned inline or none")
        ),
        escape(&results.plan.core_version),
        escape(&results.simulator.version),
        escape(&results.simulator.executable),
        escape(&serde_json::to_string_pretty(&results.summary).unwrap())
    );
    for (case, row) in results.plan.cases.iter().zip(&results.cases) {
        let status = serde_json::to_string(&row.status)
            .unwrap()
            .trim_matches('"')
            .to_owned();
        let values = row
            .measurements
            .iter()
            .map(|(name, m)| {
                format!(
                    "{}: {} {} {}",
                    name,
                    m.value
                        .map(|v| v.to_string())
                        .unwrap_or("unavailable".into()),
                    unit_suffix(m.unit),
                    m.error.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let constraints = row
            .assertions
            .as_ref()
            .map(|a| {
                format!(
                    "{} / {} passed, {} failed, {} errors",
                    a.summary.passed, a.summary.total, a.summary.failed, a.summary.errors
                )
            })
            .unwrap_or_default();
        html.push_str(&format!("<tr><td>{} · {} · {} °C<br><code>{}</code></td><td class=\"{}\">{}</td><td>{}</td><td>{}<br>{}</td></tr>", escape(&case.id), escape(&case.name), case.temperature_c, escape(&serde_json::to_string(&case.parameters).unwrap()), status, status, escape(&values), escape(&constraints), escape(&row.errors.join("; "))));
    }
    html.push_str(&format!("</tbody></table><details><summary>Exact specification and model provenance</summary><pre>{}</pre><pre>{}</pre></details><p>Retain the accompanying full JSON/CSV and matching local model resources for reproducibility. Result checksums detect accidental corruption; they are not independent attestations.</p></html>", escape(&serde_json::to_string_pretty(&results.plan.spec).unwrap()), escape(&serde_json::to_string_pretty(&results.plan.model_manifests).unwrap())));
    html
}
