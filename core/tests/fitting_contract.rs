use kessetsu_core::experiment::{
    Axis, CaseStatus, ExperimentResults, ExperimentSpec, Revision, Values, compile_case,
    evaluate_case, failed_case, new_results, plan_experiment, summarize, temperature_netlist,
};
use kessetsu_core::fitting::{
    FIT_SCHEMA, FitObservation, FitParameter, FitSignalScale, FitSpec, ObservationRole,
    evaluate_fit,
};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::research_data::{
    COMPARISON_SCHEMA, ComparisonSpec, Coverage, Interpolation, ResearchData,
    SIMULATION_IMPORT_SCHEMA, SignalPair, SimulationImportSpec, SimulationSignalMapping,
    import_simulation_data,
};
use kessetsu_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use std::collections::BTreeMap;

fn experiment_spec() -> ExperimentSpec {
    ExperimentSpec {
        schema_version: kessetsu_core::experiment::EXPERIMENT_SCHEMA.into(),
        name: "Finite divider calibration".into(),
        source: "param resistance: Ohm = 1k\n\
                 param drive: V = 1V\n\
                 net GND\nnet IN\nnet OUT\n\
                 source VIN sine(0V,{drive},100Hz)\n\
                 resistor RS {resistance}\nresistor RL 1k\n\
                 connect VIN.minus, RL.p2 to GND\n\
                 connect VIN.plus, RS.p1 to IN\n\
                 connect RS.p2, RL.p1 to OUT\n\
                 simulate tran 50us 20ms\n"
            .into(),
        requirements: None,
        axes: vec![Axis {
            parameter: "resistance".into(),
            values: Values::List {
                values: vec!["500Ohm".into(), "1kOhm".into(), "2kOhm".into()],
            },
        }],
        revisions: vec![
            Revision {
                name: "Calibration".into(),
                source: None,
                parameters: BTreeMap::from([("drive".into(), "1V".into())]),
            },
            Revision {
                name: "Validation".into(),
                source: None,
                parameters: BTreeMap::from([("drive".into(), "2V".into())]),
            },
        ],
        tolerances: None,
        temperatures_c: vec![27.0],
        timeout_ms: 30_000,
        measurements: Vec::new(),
        objective: None,
    }
}

fn complete_experiment() -> ExperimentResults {
    let resources = ExternalModelResources::new();
    let plan = plan_experiment(experiment_spec(), &resources).unwrap();
    let runner = NgspiceRunner::discover();
    let mut results = new_results(
        plan,
        runner.info().unwrap(),
        "fitting-contract-runner".into(),
    );
    for (index, case) in results.plan.cases.iter().enumerate() {
        let compiled = compile_case(&results.plan, case, &resources).unwrap();
        let request = SimulationRequest::new(
            temperature_netlist(
                compiled.spice_netlist.as_deref().unwrap(),
                case.temperature_c,
            )
            .unwrap(),
            compiled.ir.as_ref().unwrap().analyses.clone(),
        );
        let simulation = runner.run(&request, &CancellationToken::new()).unwrap();
        results.cases[index] = evaluate_case(&results.plan, case, &compiled, simulation);
    }
    summarize(&mut results);
    results
}

fn run_experiment() -> ExperimentResults {
    let mut results = complete_experiment();
    // Retain one failed calibration candidate to prove failed evidence is not dropped.
    results.cases[0] = failed_case(
        &results.plan.cases[0].id,
        CaseStatus::Error,
        "synthetic candidate failure".into(),
    );
    summarize(&mut results);
    results
}

fn fail_condition(results: &mut ExperimentResults, revision: &str, resistance: &str) {
    let index = results
        .plan
        .cases
        .iter()
        .position(|case| case.name == revision && case.parameters["resistance"] == resistance)
        .unwrap();
    results.cases[index] = failed_case(
        &results.plan.cases[index].id,
        CaseStatus::Error,
        format!("synthetic {revision} failure"),
    );
    summarize(results);
}

fn projected(results: &ExperimentResults, revision: &str) -> ResearchData {
    let (case, row) = results
        .plan
        .cases
        .iter()
        .zip(&results.cases)
        .find(|(case, _)| case.name == revision && case.parameters["resistance"] == "1000Ohm")
        .unwrap();
    import_simulation_data(
        row.simulation.as_ref().unwrap(),
        SimulationImportSpec {
            schema_version: SIMULATION_IMPORT_SCHEMA.into(),
            name: format!("Observed {revision}"),
            analysis_index: 0,
            signals: vec![SimulationSignalMapping {
                vector: "out".into(),
                name: "output".into(),
            }],
        },
    )
    .unwrap_or_else(|error| panic!("{}: {error}", case.id))
}

fn observation(name: &str, role: ObservationRole, revision: &str, data: &str) -> FitObservation {
    FitObservation {
        name: name.into(),
        role,
        revision: revision.into(),
        temperature_c: 27.0,
        conditions: BTreeMap::new(),
        data: data.into(),
        simulation: SimulationImportSpec {
            schema_version: SIMULATION_IMPORT_SCHEMA.into(),
            name: format!("Candidate {revision}"),
            analysis_index: 0,
            signals: vec![SimulationSignalMapping {
                vector: "out".into(),
                name: "output".into(),
            }],
        },
        comparison: ComparisonSpec {
            schema_version: COMPARISON_SCHEMA.into(),
            name: format!("Residual {revision}"),
            signals: vec![SignalPair {
                data_signal: "output".into(),
                reference_signal: "output".into(),
            }],
            coverage: Coverage::RequireFull,
            interpolation: Interpolation::Linear,
            reference_axis_shift: 0.0,
            window: Some([0.002, 0.018]),
            max_reference_gap: None,
        },
        signals: vec![FitSignalScale {
            data_signal: "output".into(),
            uncertainty: 0.01,
            weight: 1.0,
        }],
        masks: vec![[0.009, 0.010]],
    }
}

fn fit_spec() -> FitSpec {
    FitSpec {
        schema_version: FIT_SCHEMA.into(),
        name: "Divider resistance fit".into(),
        parameters: vec![FitParameter {
            name: "resistance".into(),
            lower: "500Ohm".into(),
            upper: "2kOhm".into(),
        }],
        observations: vec![
            observation(
                "one-volt calibration",
                ObservationRole::Calibration,
                "Calibration",
                "calibration",
            ),
            observation(
                "two-volt holdout",
                ObservationRole::Validation,
                "Validation",
                "validation",
            ),
        ],
        near_equivalent_fraction: 0.01,
    }
}

#[test]
fn finite_fit_selects_only_from_calibration_and_retains_holdout_residuals_and_failures() {
    let experiment = run_experiment();
    let data = BTreeMap::from([
        ("calibration".into(), projected(&experiment, "Calibration")),
        ("validation".into(), projected(&experiment, "Validation")),
    ]);
    let result = evaluate_fit(&experiment, &data, fit_spec()).unwrap();
    assert_eq!(result.schema_version, "kessetsu.fit-result.v1");
    assert_eq!(result.candidates.len(), 3);
    let selected = result
        .candidates
        .iter()
        .find(|candidate| Some(&candidate.id) == result.selected_candidate.as_ref())
        .unwrap();
    assert_eq!(selected.parameters["resistance"], "1000Ohm");
    assert_eq!(selected.calibration_score, Some(0.0));
    assert_eq!(selected.validation_score, Some(0.0));
    assert!(selected.observations.iter().all(|observation| {
        observation.comparison.is_some()
            && observation.scored_points > 0
            && observation
                .comparison
                .as_ref()
                .unwrap()
                .signals
                .iter()
                .all(|signal| signal.points.len() > observation.scored_points)
    }));
    let failed = result
        .candidates
        .iter()
        .find(|candidate| candidate.parameters["resistance"] == "500Ohm")
        .unwrap();
    assert!(!failed.eligible);
    assert!(failed.observations[0].error.is_some());
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("failed"))
    );
}

#[test]
fn fit_spec_rejects_missing_holdout_invalid_scales_and_fit_axes_as_conditions() {
    let experiment = run_experiment();
    let data = BTreeMap::from([
        ("calibration".into(), projected(&experiment, "Calibration")),
        ("validation".into(), projected(&experiment, "Validation")),
    ]);
    let mut spec = fit_spec();
    spec.observations.pop();
    assert!(evaluate_fit(&experiment, &data, spec).is_err());

    let mut spec = fit_spec();
    spec.observations[0].signals[0].uncertainty = 0.0;
    assert!(evaluate_fit(&experiment, &data, spec).is_err());

    let mut spec = fit_spec();
    spec.observations[0]
        .conditions
        .insert("resistance".into(), "1kOhm".into());
    assert!(evaluate_fit(&experiment, &data, spec).is_err());
}

#[test]
fn holdout_failure_cannot_change_selection_and_ambiguous_boundary_fits_are_visible() {
    let complete = complete_experiment();
    let data = BTreeMap::from([
        ("calibration".into(), projected(&complete, "Calibration")),
        ("validation".into(), projected(&complete, "Validation")),
    ]);

    let mut failed_holdout = complete.clone();
    fail_condition(&mut failed_holdout, "Validation", "1000Ohm");
    let result = evaluate_fit(&failed_holdout, &data, fit_spec()).unwrap();
    let selected = result
        .candidates
        .iter()
        .find(|candidate| Some(&candidate.id) == result.selected_candidate.as_ref())
        .unwrap();
    assert_eq!(selected.parameters["resistance"], "1000Ohm");
    assert_eq!(selected.calibration_score, Some(0.0));
    assert_eq!(selected.validation_score, None);

    let mut ambiguous = complete;
    fail_condition(&mut ambiguous, "Calibration", "1000Ohm");
    let mut spec = fit_spec();
    spec.near_equivalent_fraction = 1.0;
    let result = evaluate_fit(&ambiguous, &data, spec).unwrap();
    assert_eq!(result.near_equivalent_candidates.len(), 2);
    let selected = result
        .candidates
        .iter()
        .find(|candidate| Some(&candidate.id) == result.selected_candidate.as_ref())
        .unwrap();
    assert_eq!(selected.boundary_hits, vec!["resistance"]);
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("non-identifiable"))
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("touches declared bounds"))
    );
}

#[test]
fn cli_evaluates_bound_datasets_and_writes_one_versioned_fit_artifact() {
    let experiment = run_experiment();
    let calibration = projected(&experiment, "Calibration");
    let validation = projected(&experiment, "Validation");
    let workspace = TestWorkspace::new("fit-cli");
    workspace.write(
        "study-results.json",
        &serde_json::to_string_pretty(&experiment).unwrap(),
    );
    workspace.write(
        "calibration.json",
        &serde_json::to_string_pretty(&calibration).unwrap(),
    );
    workspace.write(
        "validation.json",
        &serde_json::to_string_pretty(&validation).unwrap(),
    );
    workspace.write(
        "fit.json",
        &serde_json::to_string_pretty(&fit_spec()).unwrap(),
    );
    let output = workspace.run_cli(&[
        "fit",
        "evaluate",
        "study-results.json",
        "--spec",
        "fit.json",
        "--data",
        "calibration=calibration.json",
        "--data",
        "validation=validation.json",
        "--output",
        "fit-result.json",
        "--format",
        "json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["command"], "fit");
    assert_eq!(envelope["status"], "completed");
    assert_eq!(envelope["domain_versions"]["fit"], FIT_SCHEMA);
    assert_eq!(envelope["fit"]["schema_version"], "kessetsu.fit-result.v1");
    let saved: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(workspace.path().join("fit-result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["schema_version"], "kessetsu.fit-result.v1");

    let repeated = workspace.run_cli(&[
        "fit",
        "evaluate",
        "study-results.json",
        "--spec",
        "fit.json",
        "--data",
        "calibration=calibration.json",
        "--data",
        "validation=validation.json",
        "--output",
        "fit-result.json",
        "--format",
        "json",
    ]);
    assert!(!repeated.status.success());
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("Output already exists"));
}
mod common;

use common::TestWorkspace;
