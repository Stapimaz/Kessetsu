//! Finite, evidence-preserving parameter fitting over completed experiment results.
//! This module does not launch simulators or claim a continuous/global optimum.
use crate::experiment::{ExperimentResults, hash_bytes, validate_results};
use crate::ir::{SIUnit, parse_quantity, parse_value};
use crate::research_data::{
    ComparisonSpec, DataComparison, ResearchData, SimulationImportSpec, compare_data,
    import_simulation_data, validate_data,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FIT_SCHEMA: &str = "kessetsu.fit.v1";
pub const FIT_RESULT_SCHEMA: &str = "kessetsu.fit-result.v1";
pub const MAX_FIT_SPEC_BYTES: usize = 1024 * 1024;
pub const MAX_FIT_RESULT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_FIT_POINTS: usize = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitSpec {
    pub schema_version: String,
    pub name: String,
    pub parameters: Vec<FitParameter>,
    pub observations: Vec<FitObservation>,
    #[serde(default = "default_equivalence")]
    pub near_equivalent_fraction: f64,
}

fn default_equivalence() -> f64 {
    0.01
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitParameter {
    pub name: String,
    pub lower: String,
    pub upper: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationRole {
    Calibration,
    Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitSignalScale {
    pub data_signal: String,
    pub uncertainty: f64,
    #[serde(default = "one")]
    pub weight: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitObservation {
    pub name: String,
    pub role: ObservationRole,
    /// Experiment revision name selecting this operating/stimulus condition.
    pub revision: String,
    pub temperature_c: f64,
    /// Optional non-fitted study-axis values selecting one case per candidate.
    #[serde(default)]
    pub conditions: BTreeMap<String, String>,
    /// Key supplied by the caller for the validated observed dataset.
    pub data: String,
    pub simulation: SimulationImportSpec,
    pub comparison: ComparisonSpec,
    pub signals: Vec<FitSignalScale>,
    /// Additional inclusive axis ranges excluded only from the fit score.
    /// The full comparison/residual record remains retained.
    #[serde(default)]
    pub masks: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitObservationResult {
    pub name: String,
    pub role: ObservationRole,
    pub data_identity: String,
    pub case_id: Option<String>,
    pub score: Option<f64>,
    pub scored_points: usize,
    pub comparison: Option<DataComparison>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitCandidateResult {
    pub id: String,
    pub parameters: BTreeMap<String, String>,
    pub eligible: bool,
    pub calibration_score: Option<f64>,
    pub validation_score: Option<f64>,
    pub boundary_hits: Vec<String>,
    pub observations: Vec<FitObservationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitResult {
    pub schema_version: String,
    pub identity: String,
    pub experiment_identity: String,
    pub simulator: crate::simulation::SimulatorInfo,
    pub solver_fingerprint: String,
    pub data_identities: BTreeMap<String, String>,
    pub spec: FitSpec,
    pub selected_candidate: Option<String>,
    pub near_equivalent_candidates: Vec<String>,
    pub candidates: Vec<FitCandidateResult>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Default)]
struct ScoreAccumulator {
    scale: f64,
    scaled_squares: f64,
    weight: f64,
    points: usize,
}

impl ScoreAccumulator {
    fn add(&mut self, normalized: f64, weight: f64) -> Result<(), String> {
        if !normalized.is_finite() || !weight.is_finite() || weight <= 0.0 {
            return Err("Non-finite normalized residual or invalid weight".into());
        }
        let absolute = normalized.abs();
        if absolute > self.scale {
            self.scaled_squares = if self.scale == 0.0 {
                weight
            } else {
                self.scaled_squares * (self.scale / absolute).powi(2) + weight
            };
            self.scale = absolute;
        } else if self.scale != 0.0 {
            self.scaled_squares += weight * (absolute / self.scale).powi(2);
        }
        self.weight += weight;
        self.points += 1;
        if !self.scaled_squares.is_finite() || !self.weight.is_finite() {
            return Err("Residual aggregation exceeds finite numeric range".into());
        }
        Ok(())
    }

    fn merge(&mut self, other: Self) -> Result<(), String> {
        if other.points == 0 {
            return Ok(());
        }
        if self.points == 0 {
            *self = other;
            return Ok(());
        }
        let scale = self.scale.max(other.scale);
        self.scaled_squares = if scale == 0.0 {
            0.0
        } else {
            self.scaled_squares * (self.scale / scale).powi(2)
                + other.scaled_squares * (other.scale / scale).powi(2)
        };
        self.scale = scale;
        self.weight += other.weight;
        self.points += other.points;
        if !self.scaled_squares.is_finite() || !self.weight.is_finite() {
            return Err("Residual aggregation exceeds finite numeric range".into());
        }
        Ok(())
    }

    fn score(self) -> Option<f64> {
        (self.points > 0).then(|| {
            if self.scale == 0.0 {
                0.0
            } else {
                self.scale * (self.scaled_squares / self.weight).sqrt()
            }
        })
    }
}

pub fn decode_fit_spec(bytes: &[u8]) -> Result<FitSpec, String> {
    if bytes.len() > MAX_FIT_SPEC_BYTES {
        return Err("Fit specification exceeds 1 MiB".into());
    }
    serde_json::from_slice(bytes).map_err(|error| format!("Invalid fit specification: {error}"))
}

fn bounded_text(value: &str, label: &str, max: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(format!(
            "{label} must be nonempty, at most {max} bytes and contain no control characters"
        ));
    }
    Ok(())
}

fn inferred_unit(value: &str) -> Result<SIUnit, String> {
    let (_, unit) = parse_value(value)?;
    Ok(unit.unwrap_or(SIUnit::Ratio))
}

fn equivalent_value(left: &str, right: &str) -> bool {
    match (parse_value(left), parse_value(right)) {
        (Ok((a, au)), Ok((b, bu))) => a == b && au == bu,
        _ => left == right,
    }
}

fn masked(axis: f64, masks: &[[f64; 2]]) -> bool {
    masks
        .iter()
        .any(|[start, stop]| axis >= *start && axis <= *stop)
}

fn score_comparison(
    comparison: &DataComparison,
    observation: &FitObservation,
) -> Result<ScoreAccumulator, String> {
    let scales = observation
        .signals
        .iter()
        .map(|signal| (signal.data_signal.as_str(), signal))
        .collect::<BTreeMap<_, _>>();
    let mut result = ScoreAccumulator::default();
    for signal in &comparison.signals {
        let scale = scales
            .get(signal.mapping.data_signal.as_str())
            .ok_or_else(|| {
                format!(
                    "Missing uncertainty/weight for data signal '{}'",
                    signal.mapping.data_signal
                )
            })?;
        for point in &signal.points {
            if masked(point.axis, &observation.masks) {
                continue;
            }
            if let Some(residual) = point.residual {
                result.add(residual / scale.uncertainty, scale.weight)?;
            }
        }
    }
    if result.points == 0 {
        return Err("Observation has no scored residual points after windows/masks".into());
    }
    Ok(result)
}

fn validate_spec(
    spec: &FitSpec,
    experiment: &ExperimentResults,
    data: &BTreeMap<String, ResearchData>,
) -> Result<BTreeMap<String, (SIUnit, f64, f64)>, String> {
    if spec.schema_version != FIT_SCHEMA {
        return Err(format!("Expected {FIT_SCHEMA}"));
    }
    bounded_text(&spec.name, "Fit name", 200)?;
    if serde_json::to_vec(spec)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_FIT_SPEC_BYTES
    {
        return Err("Fit specification exceeds 1 MiB".into());
    }
    if spec.parameters.is_empty() || spec.parameters.len() > 8 {
        return Err("Fit requires 1-8 parameters".into());
    }
    if spec.observations.len() < 2 || spec.observations.len() > 32 {
        return Err("Fit requires 2-32 observations".into());
    }
    if !spec.near_equivalent_fraction.is_finite()
        || !(0.0..=1.0).contains(&spec.near_equivalent_fraction)
    {
        return Err("near_equivalent_fraction must be within 0..1".into());
    }
    let axes = experiment
        .plan
        .spec
        .axes
        .iter()
        .map(|axis| axis.parameter.as_str())
        .collect::<BTreeSet<_>>();
    let mut names = BTreeSet::new();
    let mut bounds = BTreeMap::new();
    for parameter in &spec.parameters {
        bounded_text(&parameter.name, "Fit parameter", 128)?;
        if !names.insert(parameter.name.as_str()) || !axes.contains(parameter.name.as_str()) {
            return Err(format!(
                "Fit parameters must be unique declared study axes; '{}' is invalid",
                parameter.name
            ));
        }
        let first = experiment
            .plan
            .cases
            .first()
            .and_then(|case| case.parameters.get(&parameter.name))
            .ok_or_else(|| format!("Study cases do not contain '{}'", parameter.name))?;
        let unit = inferred_unit(first)?;
        let lower = parse_quantity(&parameter.lower, unit)?.value;
        let upper = parse_quantity(&parameter.upper, unit)?.value;
        if !lower.is_finite() || !upper.is_finite() || lower >= upper {
            return Err(format!(
                "Fit bounds for '{}' require finite lower < upper",
                parameter.name
            ));
        }
        bounds.insert(parameter.name.clone(), (unit, lower, upper));
    }
    let mut observation_names = BTreeSet::new();
    let mut calibration = 0usize;
    let mut validation = 0usize;
    for observation in &spec.observations {
        bounded_text(&observation.name, "Observation name", 128)?;
        bounded_text(&observation.revision, "Observation revision", 120)?;
        bounded_text(&observation.data, "Observation dataset key", 128)?;
        if !observation_names.insert(observation.name.as_str()) {
            return Err("Observation names must be unique".into());
        }
        match observation.role {
            ObservationRole::Calibration => calibration += 1,
            ObservationRole::Validation => validation += 1,
        }
        if !observation.temperature_c.is_finite()
            || observation.masks.len() > 16
            || observation
                .masks
                .iter()
                .any(|[start, stop]| !start.is_finite() || !stop.is_finite() || start >= stop)
        {
            return Err("Observation temperature/masks are invalid or exceed limits".into());
        }
        for parameter in observation.conditions.keys() {
            if names.contains(parameter.as_str()) {
                return Err(format!(
                    "Observation condition '{}' is a fitted parameter",
                    parameter
                ));
            }
        }
        let observed = data
            .get(&observation.data)
            .ok_or_else(|| format!("Dataset '{}' was not supplied", observation.data))?;
        validate_data(observed)?;
        let mapped = observation
            .comparison
            .signals
            .iter()
            .map(|mapping| mapping.data_signal.as_str())
            .collect::<BTreeSet<_>>();
        let mut scaled = BTreeSet::new();
        for signal in &observation.signals {
            if !scaled.insert(signal.data_signal.as_str())
                || !signal.uncertainty.is_finite()
                || signal.uncertainty <= 0.0
                || !signal.weight.is_finite()
                || signal.weight <= 0.0
                || signal.weight > 1e12
            {
                return Err(
                    "Signal scales must be unique with positive finite uncertainty/weight".into(),
                );
            }
        }
        if mapped.is_empty() || mapped != scaled {
            return Err("Every compared data signal needs exactly one uncertainty/weight".into());
        }
    }
    if calibration == 0 || validation == 0 {
        return Err("Fit requires at least one calibration and one validation observation".into());
    }
    Ok(bounds)
}

fn observation_for_candidate(
    observation: &FitObservation,
    candidate_cases: &[usize],
    experiment: &ExperimentResults,
    observed: &ResearchData,
) -> FitObservationResult {
    let matches = candidate_cases
        .iter()
        .copied()
        .filter(|index| {
            let case = &experiment.plan.cases[*index];
            case.name == observation.revision
                && case.temperature_c.to_bits() == observation.temperature_c.to_bits()
                && observation.conditions.iter().all(|(name, value)| {
                    case.parameters
                        .get(name)
                        .is_some_and(|actual| equivalent_value(actual, value))
                })
        })
        .collect::<Vec<_>>();
    let mut output = FitObservationResult {
        name: observation.name.clone(),
        role: observation.role,
        data_identity: observed.identity.clone(),
        case_id: None,
        score: None,
        scored_points: 0,
        comparison: None,
        error: None,
    };
    if matches.len() != 1 {
        output.error = Some(format!(
            "Observation selector matched {} cases; expected exactly one",
            matches.len()
        ));
        return output;
    }
    let index = matches[0];
    let case = &experiment.plan.cases[index];
    let result = &experiment.cases[index];
    output.case_id = Some(case.id.clone());
    let Some(simulation) = result.simulation.as_ref() else {
        output.error = Some(format!(
            "Case status {:?} has no simulation evidence",
            result.status
        ));
        return output;
    };
    let projected = match import_simulation_data(simulation, observation.simulation.clone()) {
        Ok(projected) => projected,
        Err(error) => {
            output.error = Some(error);
            return output;
        }
    };
    let comparison = match compare_data(observed, &projected, observation.comparison.clone()) {
        Ok(comparison) => comparison,
        Err(error) => {
            output.error = Some(error);
            return output;
        }
    };
    match score_comparison(&comparison, observation) {
        Ok(score) => {
            output.score = score.score();
            output.scored_points = score.points;
            output.comparison = Some(comparison);
        }
        Err(error) => output.error = Some(error),
    }
    output
}

pub fn evaluate_fit(
    experiment: &ExperimentResults,
    data: &BTreeMap<String, ResearchData>,
    spec: FitSpec,
) -> Result<FitResult, String> {
    validate_results(
        experiment,
        &experiment.plan,
        &experiment.simulator,
        &experiment.solver_fingerprint,
    )?;
    if experiment.cases.iter().any(|result| {
        matches!(
            result.status,
            crate::experiment::CaseStatus::Pending | crate::experiment::CaseStatus::Cancelled
        )
    }) {
        return Err("Fit requires a completed experiment with no pending/cancelled cases".into());
    }
    let bounds = validate_spec(&spec, experiment, data)?;
    let parameter_names = spec
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<Vec<_>>();
    let mut groups = BTreeMap::<Vec<String>, Vec<usize>>::new();
    for (index, case) in experiment.plan.cases.iter().enumerate() {
        let key = parameter_names
            .iter()
            .map(|name| {
                case.parameters
                    .get(*name)
                    .cloned()
                    .ok_or_else(|| format!("Case '{}' lacks fit parameter '{name}'", case.id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (name, value) in parameter_names.iter().zip(&key) {
            let (unit, lower, upper) = bounds[*name];
            let numeric = parse_quantity(value, unit)?.value;
            if numeric < lower || numeric > upper {
                return Err(format!(
                    "Study candidate {name}={value} is outside declared fit bounds"
                ));
            }
        }
        groups.entry(key).or_default().push(index);
    }
    let data_identities = data
        .iter()
        .map(|(name, data)| (name.clone(), data.identity.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut candidates = Vec::new();
    let mut total_points = 0usize;
    for (key, case_indices) in groups {
        let parameters = parameter_names
            .iter()
            .zip(&key)
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut boundary_hits = Vec::new();
        for (name, value) in &parameters {
            let (unit, lower, upper) = bounds[name];
            let numeric = parse_quantity(value, unit)?.value;
            let tolerance = (upper - lower).abs().max(1.0) * 1e-12;
            if (numeric - lower).abs() <= tolerance || (numeric - upper).abs() <= tolerance {
                boundary_hits.push(name.clone());
            }
        }
        let mut observations = Vec::new();
        let mut calibration = ScoreAccumulator::default();
        let mut validation = ScoreAccumulator::default();
        let mut calibration_ok = true;
        let mut validation_ok = true;
        for observation in &spec.observations {
            let observed = &data[&observation.data];
            let result =
                observation_for_candidate(observation, &case_indices, experiment, observed);
            total_points =
                total_points.saturating_add(result.comparison.as_ref().map_or(0, |comparison| {
                    comparison
                        .signals
                        .iter()
                        .map(|signal| signal.points.len())
                        .sum()
                }));
            if total_points > MAX_FIT_POINTS {
                return Err("Fit result exceeds two million retained residual points".into());
            }
            if result.error.is_some() {
                match observation.role {
                    ObservationRole::Calibration => calibration_ok = false,
                    ObservationRole::Validation => validation_ok = false,
                }
            } else if let Some(comparison) = &result.comparison {
                let score = score_comparison(comparison, observation)?;
                match observation.role {
                    ObservationRole::Calibration => calibration.merge(score)?,
                    ObservationRole::Validation => validation.merge(score)?,
                }
            }
            observations.push(result);
        }
        let calibration_score = calibration_ok.then(|| calibration.score()).flatten();
        let id = format!(
            "candidate-{}",
            &hash_bytes(&serde_json::to_vec(&parameters).map_err(|error| error.to_string())?)
                [7..19]
        );
        candidates.push(FitCandidateResult {
            id,
            parameters,
            eligible: calibration_score.is_some(),
            calibration_score,
            validation_score: validation_ok.then(|| validation.score()).flatten(),
            boundary_hits,
            observations,
        });
    }
    candidates.sort_by(|left, right| {
        for parameter in &spec.parameters {
            let unit = bounds[&parameter.name].0;
            let left_value = parse_quantity(&left.parameters[&parameter.name], unit);
            let right_value = parse_quantity(&right.parameters[&parameter.name], unit);
            let order = match (left_value, right_value) {
                (Ok(left), Ok(right)) => left.value.total_cmp(&right.value),
                _ => left.parameters[&parameter.name].cmp(&right.parameters[&parameter.name]),
            };
            if order != std::cmp::Ordering::Equal {
                return order;
            }
        }
        left.id.cmp(&right.id)
    });
    let selected = candidates
        .iter()
        .filter_map(|candidate| candidate.calibration_score.map(|score| (candidate, score)))
        .min_by(|(left, left_score), (right, right_score)| {
            left_score
                .total_cmp(right_score)
                .then_with(|| left.id.cmp(&right.id))
        });
    let selected_candidate = selected.map(|(candidate, _)| candidate.id.clone());
    let near_equivalent_candidates = selected
        .map(|(_, best)| {
            let allowance = spec.near_equivalent_fraction * best.max(1e-12);
            candidates
                .iter()
                .filter(|candidate| {
                    candidate
                        .calibration_score
                        .is_some_and(|score| score <= best + allowance)
                })
                .map(|candidate| candidate.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut warnings = vec![
        "Selection is limited to the finite evaluated study grid; it is not evidence of a continuous or global optimum.".into(),
        "Curve agreement does not establish a physical mechanism or validity outside the explicit observations and bounds.".into(),
    ];
    if !data
        .values()
        .any(|dataset| dataset.spec.metadata.origin == crate::research_data::Origin::Measured)
    {
        warnings.push(
            "No supplied dataset is marked measured; this fit is workflow/simulation evidence, not physical-device calibration."
                .into(),
        );
    }
    if near_equivalent_candidates.len() > 1 {
        warnings.push(format!(
            "{} candidates are near-equivalent at the declared tolerance; parameters may be non-identifiable on these observations.",
            near_equivalent_candidates.len()
        ));
    }
    if let Some(selected) = selected_candidate
        .as_ref()
        .and_then(|id| candidates.iter().find(|candidate| &candidate.id == id))
        && !selected.boundary_hits.is_empty()
    {
        warnings.push(format!(
            "Selected candidate touches declared bounds for: {}.",
            selected.boundary_hits.join(", ")
        ));
    }
    let failures = candidates
        .iter()
        .flat_map(|candidate| &candidate.observations)
        .filter(|observation| observation.error.is_some())
        .count();
    if failures > 0 {
        warnings.push(format!(
            "{failures} candidate observation(s) failed and remain recorded in the result."
        ));
    }
    let mut result = FitResult {
        schema_version: FIT_RESULT_SCHEMA.into(),
        identity: String::new(),
        experiment_identity: experiment.identity.clone(),
        simulator: experiment.simulator.clone(),
        solver_fingerprint: experiment.solver_fingerprint.clone(),
        data_identities,
        spec,
        selected_candidate,
        near_equivalent_candidates,
        candidates,
        warnings,
    };
    result.identity = hash_bytes(
        &serde_json::to_vec(&result)
            .map_err(|error| format!("Could not seal fit result: {error}"))?,
    );
    if serde_json::to_vec(&result)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_FIT_RESULT_BYTES
    {
        return Err("Fit result exceeds 64 MiB".into());
    }
    Ok(result)
}
