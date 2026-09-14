mod common;

use common::{TestWorkspace, read_fixture};
use kessetsu_core::ir::{AcScale, Analysis, Quantity, SIUnit, SimulatorCompatibility};
use kessetsu_core::simulation::{
    ArtifactPolicy, CancellationToken, Dataset, NativeSimulationContext, NgspiceRunner,
    SimulationRequest, SimulationRunErrorKind, SimulationRunner, SimulationStatus,
};
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::time::Duration;

#[test]
fn native_runner_returns_structured_status_logs_and_measurements() {
    let workspace = TestWorkspace::new("structured-runner");
    let simulator = workspace.write_fake_simulator(
        "structured-simulator",
        "max_v_out = 3.21e-1\nWarning: sample warning",
        "",
        0,
    );
    let runner = NgspiceRunner::new(&simulator);
    let request = SimulationRequest::new("* fixture\n.end\n", vec![Analysis::OperatingPoint]);

    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("simulation should launch");

    assert_eq!(result.status, SimulationStatus::Succeeded);
    assert!(result.process.success);
    assert_eq!(result.process.exit_code, Some(0));
    assert_eq!(result.simulator.version, "ngspice-test-1");
    assert_eq!(result.measurements["max_v_out"], 0.321);
    assert_eq!(result.warnings, ["Warning: sample warning"]);
    assert!(result.errors.is_empty());
    assert!(result.raw_log.stdout.contains("max_v_out"));
    assert!(result.artifacts.is_empty());
}

#[test]
fn native_runner_distinguishes_launch_and_simulator_failures() {
    let workspace = TestWorkspace::new("runner-failures");
    let missing = workspace.path().join("missing-ngspice");
    let launch_error = NgspiceRunner::new(missing)
        .run(
            &SimulationRequest::new("* fixture\n.end\n", vec![]),
            &CancellationToken::new(),
        )
        .expect_err("missing executable should fail to launch");
    assert_eq!(launch_error.kind, SimulationRunErrorKind::Launch);

    let simulator =
        workspace.write_fake_simulator("failing-simulator", "", "Fatal error: singular matrix", 9);
    let result = NgspiceRunner::new(simulator)
        .run(
            &SimulationRequest::new("* fixture\n.end\n", vec![]),
            &CancellationToken::new(),
        )
        .expect("process failure should remain a structured result");
    assert_eq!(result.status, SimulationStatus::Failed);
    assert_eq!(result.process.exit_code, Some(9));
    assert_eq!(result.errors, ["Fatal error: singular matrix"]);
}

#[cfg(unix)]
#[test]
fn native_runner_retries_a_transient_text_file_busy_launch() {
    let workspace = TestWorkspace::new("runner-text-file-busy");
    let simulator = workspace.write_fake_simulator("busy-simulator", "", "", 0);
    let held_executable = OpenOptions::new()
        .write(true)
        .open(&simulator)
        .expect("fake simulator should be held open for writing");
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(25));
        drop(held_executable);
    });

    let result = NgspiceRunner::new(simulator)
        .run(
            &SimulationRequest::new("* fixture\n.end\n", vec![]),
            &CancellationToken::new(),
        )
        .expect("a transient ETXTBSY launch should be retried");
    release.join().expect("writer release thread should finish");

    assert_eq!(result.status, SimulationStatus::Succeeded);
    assert_eq!(result.simulator.version, "ngspice-test-1");
}

#[test]
fn native_runner_stages_exact_external_bytes_only_inside_the_run_directory() {
    let workspace = TestWorkspace::new("runner-external-resource");
    let simulator = workspace.write_fake_simulator(
        "external-resource-simulator",
        "",
        "Fatal error: retained fixture",
        9,
    );
    let runner = NgspiceRunner::new(simulator);
    let mut request =
        SimulationRequest::new("* fixture\n.include \"models/fixture.lib\"\n.end\n", vec![]);
    request.artifact_policy = ArtifactPolicy::RetainOnFailure;
    let expected = b"private external model fixture".to_vec();
    let context = NativeSimulationContext {
        compatibility: SimulatorCompatibility::Ngspice,
        resources: BTreeMap::from([("models/fixture.lib".to_string(), expected.clone())]),
    };
    let result = runner
        .run_with_context(&request, &context, &CancellationToken::new())
        .expect("simulator failure should remain structured");
    assert_eq!(result.status, SimulationStatus::Failed);
    let run_directory = result
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "run_directory")
        .map(|artifact| std::path::PathBuf::from(&artifact.path))
        .expect("retained run directory should be reported");
    assert!(
        run_directory
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("kessetsu-sim-"))
    );
    assert_eq!(
        fs::read(run_directory.join("models/fixture.lib"))
            .expect("staged resource should be readable"),
        expected
    );
    fs::remove_dir_all(&run_directory).expect("owned retained test directory should be removable");
}

#[test]
fn native_runner_times_out_and_can_retain_failure_artifacts() {
    let workspace = TestWorkspace::new("runner-timeout");
    let simulator = workspace.write_slow_fake_simulator("slow-simulator", 2);
    let runner = NgspiceRunner::new(simulator);
    let mut request = SimulationRequest::new("* timeout fixture\n.end\n", vec![]);
    request.timeout_ms = 50;
    request.artifact_policy = ArtifactPolicy::RetainOnFailure;

    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("timeout should be a structured result");
    assert_eq!(result.status, SimulationStatus::TimedOut);
    assert!(!result.process.success);
    let artifact = result
        .artifacts
        .first()
        .expect("failure artifact should remain");
    let retained = std::path::PathBuf::from(&artifact.path);
    assert!(retained.join("circuit.spice").is_file());
    fs::remove_dir_all(&retained).expect("test-owned retained artifact should be removable");
}

#[test]
fn native_runner_observes_cancellation() {
    let workspace = TestWorkspace::new("runner-cancel");
    let simulator = workspace.write_slow_fake_simulator("cancel-simulator", 2);
    let runner = NgspiceRunner::new(simulator);
    let request = SimulationRequest::new("* cancellation fixture\n.end\n", vec![]);
    let cancellation = CancellationToken::new();
    let signal = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        signal.cancel();
    });

    let result = runner
        .run(&request, &cancellation)
        .expect("cancellation should be a structured result");
    canceller.join().expect("canceller should finish");
    assert_eq!(result.status, SimulationStatus::Cancelled);
    assert!(!result.process.success);
}

#[test]
fn parallel_runs_do_not_share_a_netlist_path() {
    let workspace = TestWorkspace::new("parallel-runner");
    let simulator = workspace.write_fake_simulator("parallel-simulator", "done", "", 0);
    let runner_a = NgspiceRunner::new(&simulator);
    let runner_b = NgspiceRunner::new(&simulator);
    let first = std::thread::spawn(move || {
        runner_a.run(
            &SimulationRequest::new("* first\n.end\n", vec![]),
            &CancellationToken::new(),
        )
    });
    let second = std::thread::spawn(move || {
        runner_b.run(
            &SimulationRequest::new("* second\n.end\n", vec![]),
            &CancellationToken::new(),
        )
    });

    assert!(
        first
            .join()
            .expect("first runner should join")
            .unwrap()
            .succeeded()
    );
    assert!(
        second
            .join()
            .expect("second runner should join")
            .unwrap()
            .succeeded()
    );
}

#[test]
fn real_ngspice_produces_structured_op_transient_and_ac_datasets() {
    let cases = [
        (
            "simulation/netlists/op.spice",
            Analysis::OperatingPoint,
            "operating_point",
        ),
        (
            "simulation/netlists/tran.spice",
            Analysis::Transient {
                step: Quantity {
                    value: 10e-6,
                    unit: SIUnit::Second,
                },
                stop: Quantity {
                    value: 5e-3,
                    unit: SIUnit::Second,
                },
            },
            "transient",
        ),
        (
            "simulation/netlists/ac.spice",
            Analysis::Ac {
                scale: AcScale::Decade,
                points: 10,
                start: Quantity {
                    value: 10.0,
                    unit: SIUnit::Hertz,
                },
                stop: Quantity {
                    value: 100_000.0,
                    unit: SIUnit::Hertz,
                },
            },
            "ac",
        ),
    ];

    let runner = NgspiceRunner::discover();
    for (fixture, analysis, expected_kind) in cases {
        let request = SimulationRequest::new(read_fixture(fixture), vec![analysis]);
        let result = runner
            .run(&request, &CancellationToken::new())
            .unwrap_or_else(|error| panic!("real Ngspice fixture '{fixture}' failed: {error}"));
        assert_eq!(result.status, SimulationStatus::Succeeded, "{fixture}");
        assert_eq!(result.datasets.len(), 1, "{fixture}");
        let serialized = serde_json::to_value(&result).expect("simulation result should serialize");
        assert_eq!(serialized["schema_version"], "kessetsu.simulation.v1");
        let dataset = &result.datasets[0].data;
        match (expected_kind, dataset) {
            ("operating_point", Dataset::OperatingPoint { values }) => {
                assert!((values["out"] - 2.5).abs() < 1e-12);
            }
            ("transient", Dataset::Transient(series)) => {
                assert!(series.axis.values.len() > 100);
                assert_eq!(series.axis.values.len(), series.signals["out"].len());
            }
            ("ac", Dataset::Ac(series)) => {
                assert_eq!(series.frequency_hz.len(), 41);
                assert_eq!(series.frequency_hz.len(), series.signals["out"].real.len());
            }
            _ => panic!("unexpected dataset for {fixture}: {dataset:?}"),
        }
    }
}
