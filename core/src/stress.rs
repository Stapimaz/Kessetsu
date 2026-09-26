use crate::ast::Cmp;
use crate::ir::{Assertion, CircuitIR, PartRatingKind, ProvidedPartRating, Quantity};
use crate::simulation::{Dataset, SimulationResult};
use serde::{Deserialize, Serialize};

pub const PART_STRESS_SCHEMA_VERSION: &str = "kessetsu.part-stress.v1";
pub const PART_STRESS_DISCLAIMER: &str = "Compares simulated model stress with user-provided limits under the recorded conditions. It does not establish datasheet compliance, safe operating area, thermal safety or hardware reliability.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartStressStatus {
    WithinProvidedLimit,
    ExceedsProvidedLimit,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartStressResult {
    pub component: String,
    pub rating: PartRatingKind,
    pub metric: String,
    pub signal: String,
    pub status: PartStressStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<f64>,
    pub limit: Quantity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utilization_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis: Option<String>,
    pub conditions: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PartStressSummary {
    pub total: usize,
    pub within_provided_limit: usize,
    pub exceeds_provided_limit: usize,
    pub unavailable: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartStressReport {
    pub schema_version: String,
    pub disclaimer: String,
    pub results: Vec<PartStressResult>,
    pub summary: PartStressSummary,
}

fn metric_for(component: &str, rating: &ProvidedPartRating) -> (String, String) {
    match rating.kind {
        PartRatingKind::PeakVoltage => ("peak".into(), format!("V({component})")),
        PartRatingKind::PeakCurrent => ("peak".into(), format!("I({component})")),
        PartRatingKind::AverageDissipation => ("dissipation".into(), component.into()),
    }
}

fn applicable_dataset(rating: PartRatingKind, dataset: &Dataset) -> bool {
    match rating {
        PartRatingKind::PeakVoltage | PartRatingKind::PeakCurrent => {
            !matches!(dataset, Dataset::Ac(_))
        }
        PartRatingKind::AverageDissipation => matches!(dataset, Dataset::Transient(_)),
    }
}

fn evaluate_rating(
    circuit: &CircuitIR,
    simulation: &SimulationResult,
    component: &str,
    rating: &ProvidedPartRating,
) -> PartStressResult {
    let (metric, signal) = metric_for(component, rating);
    let assertion = Assertion {
        metric: metric.clone(),
        signal: signal.clone(),
        cmp: Cmp::Le,
        threshold: rating.limit.clone(),
        numeric_arguments: Vec::new(),
    };
    let mut measurements = Vec::new();
    let mut errors = Vec::new();
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| applicable_dataset(rating.kind, &dataset.data))
    {
        let mut isolated = simulation.clone();
        isolated.analyses = vec![dataset.analysis.clone()];
        isolated.datasets = vec![dataset.clone()];
        match crate::measurement::evaluate_assertion_metric(&assertion, circuit, &isolated) {
            Ok(actual) if actual.is_finite() => {
                measurements.push((actual.abs(), dataset.index, dataset.analysis.kind_name()))
            }
            Ok(_) => errors.push("measurement produced a non-finite value".to_string()),
            Err(error) => errors.push(error),
        }
    }
    let best = measurements
        .into_iter()
        .max_by(|left, right| left.0.total_cmp(&right.0));
    let (status, actual, utilization_percent, dataset_index, analysis, message) =
        if !simulation.succeeded() {
            (
                PartStressStatus::Unavailable,
                None,
                None,
                None,
                None,
                Some("simulation did not succeed; provided limit was not evaluated".into()),
            )
        } else if let Some((actual, dataset_index, analysis)) = best {
            let utilization = actual / rating.limit.value * 100.0;
            (
                if actual <= rating.limit.value {
                    PartStressStatus::WithinProvidedLimit
                } else {
                    PartStressStatus::ExceedsProvidedLimit
                },
                Some(actual),
                Some(utilization),
                Some(dataset_index),
                Some(analysis.to_string()),
                None,
            )
        } else {
            errors.sort();
            errors.dedup();
            (
                PartStressStatus::Unavailable,
                None,
                None,
                None,
                None,
                Some(if errors.is_empty() {
                    "no applicable simulation dataset was produced".into()
                } else {
                    errors.join(" | ")
                }),
            )
        };
    PartStressResult {
        component: component.into(),
        rating: rating.kind,
        metric,
        signal,
        status,
        actual,
        limit: rating.limit.clone(),
        utilization_percent,
        dataset_index,
        analysis,
        conditions: rating.conditions.clone(),
        source: rating.source.clone(),
        message,
    }
}

pub fn evaluate_part_stress(
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> PartStressReport {
    let mut results = Vec::new();
    for part in &circuit.physical_parts.assignments {
        for rating in &part.ratings {
            results.push(evaluate_rating(
                circuit,
                simulation,
                &part.component,
                rating,
            ));
        }
    }
    let mut summary = PartStressSummary {
        total: results.len(),
        ..PartStressSummary::default()
    };
    for result in &results {
        match result.status {
            PartStressStatus::WithinProvidedLimit => summary.within_provided_limit += 1,
            PartStressStatus::ExceedsProvidedLimit => summary.exceeds_provided_limit += 1,
            PartStressStatus::Unavailable => summary.unavailable += 1,
        }
    }
    PartStressReport {
        schema_version: PART_STRESS_SCHEMA_VERSION.into(),
        disclaimer: PART_STRESS_DISCLAIMER.into(),
        results,
        summary,
    }
}
