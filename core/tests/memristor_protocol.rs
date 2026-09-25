//! Reproducible synthetic protocol for the bundled threshold memristor model.
//! This validates model behavior and numerical sensitivity, not a physical device.
use kessetsu_core::experiment::{
    CaseStatus, ExperimentResults, compile_case, decode_spec, evaluate_case, new_results,
    plan_experiment, summarize, temperature_netlist,
};
use kessetsu_core::ir::{SIUnit, parse_quantity};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::simulation::{
    CancellationToken, Dataset, NativeSimulationContext, NgspiceRunner, SimulationRequest,
};

const MODEL: &str = include_str!("../../examples/models/memristor.lib");
const PROTOCOL: &[u8] = include_bytes!("../../examples/memristor_pulse_protocol.kessstudy.json");
const CONVERGENCE: &[u8] = include_bytes!("../../examples/memristor_convergence.kessstudy.json");
const HOLDOUT: &[u8] = include_bytes!("../../examples/memristor_holdout.kessstudy.json");

fn resources() -> ExternalModelResources {
    ExternalModelResources::from([("models/memristor.lib".into(), MODEL.as_bytes().to_vec())])
}

fn run(bytes: &[u8]) -> ExperimentResults {
    let resources = resources();
    let plan = plan_experiment(decode_spec(bytes).unwrap(), &resources).unwrap();
    let runner = NgspiceRunner::discover();
    let mut results = new_results(plan, runner.info().unwrap(), "protocol-test".into());
    for (index, case) in results.plan.cases.iter().enumerate() {
        let report = compile_case(&results.plan, case, &resources).unwrap();
        let circuit = report.ir.as_ref().unwrap();
        let spice =
            temperature_netlist(report.spice_netlist.as_deref().unwrap(), case.temperature_c)
                .unwrap();
        let simulation = runner
            .run_with_context(
                &SimulationRequest::new(spice, circuit.analyses.clone()),
                &NativeSimulationContext {
                    resources: resources.clone(),
                    ..Default::default()
                },
                &CancellationToken::new(),
            )
            .unwrap();
        results.cases[index] = evaluate_case(&results.plan, case, &report, simulation);
    }
    summarize(&mut results);
    assert_eq!(results.summary.errors, 0);
    assert_eq!(results.summary.completed, results.summary.total);
    results
}

fn seconds(text: &str) -> f64 {
    parse_quantity(text, SIUnit::Second).unwrap().value
}

fn ohms(text: &str) -> f64 {
    parse_quantity(text, SIUnit::Ohm).unwrap().value
}

fn volts(text: &str) -> f64 {
    parse_quantity(text, SIUnit::Volt).unwrap().value
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn read_state(results: &ExperimentResults, index: usize) -> (f64, f64) {
    let duration = seconds(&results.plan.cases[index].parameters["pulse_duration"]);
    let simulation = results.cases[index].simulation.as_ref().unwrap();
    let Dataset::Transient(data) = &simulation.datasets[0].data else {
        panic!("transient result missing")
    };
    let voltage = &data.signals["top"];
    let current = &data.signals["v_vsense#branch"];
    let state = |start: f64, stop: f64| {
        median(
            data.axis
                .values
                .iter()
                .zip(voltage)
                .zip(current)
                .filter(|((time, _), amp)| start <= **time && **time <= stop && amp.abs() > 1e-12)
                .map(|((_, volt), amp)| volt / amp)
                .collect(),
        )
    };
    (
        state(0.35e-9, 0.65e-9),
        state(duration + 2.15e-9, duration + 2.45e-9),
    )
}

fn energy(results: &ExperimentResults, index: usize) -> f64 {
    results.cases[index].measurements["protocol_energy"]
        .value
        .unwrap()
}

#[test]
fn pulse_matrix_preserves_threshold_direction_state_bounds_and_duration_effect() {
    let results = run(PROTOCOL);
    assert_eq!(results.summary.total, 24);
    for (index, case) in results.plan.cases.iter().enumerate() {
        assert_eq!(results.cases[index].status, CaseStatus::Completed);
        let amplitude = volts(&case.parameters["pulse_amplitude"]);
        let duration = seconds(&case.parameters["pulse_duration"]);
        let initial = ohms(&case.parameters["initial_resistance"]);
        let (before, after) = read_state(&results, index);
        assert!((before - initial).abs() / initial < 0.01);
        assert!((950.0..=10_500.0).contains(&after));
        assert!(energy(&results, index).is_finite() && energy(&results, index) > 0.0);
        if amplitude.abs() < 1.6 {
            assert!((after - before).abs() / before < 0.001);
        } else if amplitude < 0.0 {
            assert!(after < before);
        } else {
            assert!(after > before);
        }

        // For otherwise identical cases, a longer above-threshold pulse cannot produce less
        // modeled state movement (saturation may make it equal within numerical tolerance).
        if amplitude.abs() > 1.6 && duration < 6e-9 {
            let longer = results
                .plan
                .cases
                .iter()
                .position(|other| {
                    other.parameters["pulse_amplitude"] == case.parameters["pulse_amplitude"]
                        && other.parameters["initial_resistance"]
                            == case.parameters["initial_resistance"]
                        && (seconds(&other.parameters["pulse_duration"]) - 6e-9).abs() < 1e-18
                })
                .unwrap();
            let (long_before, long_after) = read_state(&results, longer);
            assert!(
                (long_after - long_before).abs() + 2.0 >= (after - before).abs(),
                "{} moved less under the longer pulse",
                case.id
            );
        }
    }
}

#[test]
fn timestep_refinement_is_bounded_and_holdout_remains_separate() {
    let results = run(CONVERGENCE);
    assert_eq!(results.summary.total, 16);
    let mut maximum_state_delta: f64 = 0.0;
    let mut maximum_energy_delta: f64 = 0.0;
    for (index, case) in results.plan.cases.iter().enumerate() {
        if (seconds(&case.parameters["timestep"]) - 25e-12).abs() > 1e-18 {
            continue;
        }
        let refined = results
            .plan
            .cases
            .iter()
            .position(|other| {
                other.parameters["pulse_amplitude"] == case.parameters["pulse_amplitude"]
                    && other.parameters["pulse_duration"] == case.parameters["pulse_duration"]
                    && other.parameters["initial_resistance"]
                        == case.parameters["initial_resistance"]
                    && (seconds(&other.parameters["timestep"]) - 12.5e-12).abs() < 1e-18
            })
            .unwrap();
        let (_, coarse_state) = read_state(&results, index);
        let (_, fine_state) = read_state(&results, refined);
        maximum_state_delta = maximum_state_delta
            .max((coarse_state - fine_state).abs() / coarse_state.abs().max(fine_state.abs()));
        let coarse_energy = energy(&results, index);
        let fine_energy = energy(&results, refined);
        maximum_energy_delta = maximum_energy_delta
            .max((coarse_energy - fine_energy).abs() / coarse_energy.abs().max(fine_energy.abs()));
    }
    assert!(
        maximum_state_delta < 0.02,
        "state delta {maximum_state_delta}"
    );
    assert!(
        maximum_energy_delta < 0.02,
        "energy delta {maximum_energy_delta}"
    );

    let heldout = run(HOLDOUT);
    assert_eq!(heldout.summary.total, 1);
    let (before, after) = read_state(&heldout, 0);
    assert!(
        (before - 5000.0).abs() < 50.0,
        "held-out initial readback {before} Ohm"
    );
    assert!((9500.0..=10_500.0).contains(&after));
    assert!(energy(&heldout, 0) > 0.0);
    println!(
        "maximum 25 ps vs 12.5 ps deltas: state {:.3}%, energy {:.3}%; held-out state {:.2} -> {:.2} Ohm",
        maximum_state_delta * 100.0,
        maximum_energy_delta * 100.0,
        before,
        after
    );
}
