mod common;

use common::TestWorkspace;
use kessetsu_core::experiment::*;
use kessetsu_core::ir::{Quantity, SIUnit};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner, SimulatorInfo,
};
use std::collections::BTreeMap;
use std::fs;

fn spec() -> ExperimentSpec {
    decode_spec(include_bytes!(
        "../../examples/studies/loaded-filter.kessstudy.json"
    ))
    .unwrap()
}
fn resources() -> ExternalModelResources {
    BTreeMap::new()
}
fn solver() -> SimulatorInfo {
    SimulatorInfo {
        executable: "fixture".into(),
        version: "fixture-1".into(),
    }
}
fn run(spec: &ExperimentSpec) -> ExperimentResults {
    let plan = plan_experiment(spec.clone(), &resources()).unwrap();
    let runner = NgspiceRunner::discover();
    let mut result = new_results(plan, runner.info().unwrap(), "test-executable".into());
    for (index, case) in result.plan.cases.iter().enumerate() {
        let compiled = compile_case(&result.plan, case, &resources()).unwrap();
        let request = SimulationRequest::new(
            temperature_netlist(compiled.spice_netlist.as_ref().unwrap(), case.temperature_c)
                .unwrap(),
            compiled.ir.as_ref().unwrap().analyses.clone(),
        );
        let simulation = runner.run(&request, &CancellationToken::new()).unwrap();
        result.cases[index] = evaluate_case(&result.plan, case, &compiled, simulation);
    }
    summarize(&mut result);
    validate_results(
        &result,
        &result.plan,
        &result.simulator,
        &result.solver_fingerprint,
    )
    .unwrap();
    result
}

#[test]
fn portable_examples_match_their_editable_sources() {
    assert_eq!(
        spec().source,
        include_str!("../../examples/loaded_filter.kess").replace("\r\n", "\n")
    );
    let driver = decode_spec(include_bytes!(
        "../../examples/studies/transistor-driver.kessstudy.json"
    ))
    .unwrap();
    assert_eq!(
        driver.source,
        include_str!("../../examples/transistor_driver.kess").replace("\r\n", "\n")
    );
    assert_eq!(
        plan_experiment(driver.clone(), &resources())
            .unwrap()
            .cases
            .len(),
        12
    );
}

#[test]
fn deterministic_cartesian_lists_grids_units_and_limits() {
    let mut s = spec();
    let first = plan_experiment(s.clone(), &resources()).unwrap();
    assert_eq!(first.cases.len(), 6);
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(plan_experiment(s.clone(), &resources()).unwrap()).unwrap()
    );
    s.axes[0].values = Values::Log {
        start: "1kOhm".into(),
        stop: "100kOhm".into(),
        points: 3,
    };
    let p = plan_experiment(s.clone(), &resources()).unwrap();
    let middle =
        kessetsu_core::ir::parse_quantity(&p.cases[2].parameters["resistance"], SIUnit::Ohm)
            .unwrap()
            .value;
    assert!((middle - 10_000.0).abs() < 1e-8);
    s.axes[0].values = Values::Linear {
        start: "1kOhm".into(),
        stop: "3kOhm".into(),
        points: 3,
    };
    assert_eq!(
        plan_experiment(s.clone(), &resources()).unwrap().cases[2].parameters["resistance"],
        "2000Ohm"
    );
    s.axes[0].values = Values::List {
        values: vec!["1V".into()],
    };
    assert!(plan_experiment(s.clone(), &resources()).is_err());
    s.axes[0].values = Values::Linear {
        start: "1kOhm".into(),
        stop: "3kOhm".into(),
        points: 256,
    };
    assert!(plan_experiment(s.clone(), &resources()).is_err());
    s = spec();
    s.axes[0].parameter = "not_declared".into();
    assert!(plan_experiment(s.clone(), &resources()).is_err());
    assert!(decode_spec(br#"{"schema_version":"future"}"#).is_err());
    s = spec();
    s.temperatures_c = vec![f64::NAN];
    assert!(plan_experiment(s.clone(), &resources()).is_err());
}

#[test]
fn seeded_correlated_tolerances_and_corners_are_explicit() {
    let mut s = spec();
    s.axes.clear();
    s.tolerances = Some(ToleranceStudy {
        mode: ToleranceMode::MonteCarlo,
        seed: 42,
        samples: 5,
        parameters: vec![
            ToleranceParameter {
                parameter: "resistance".into(),
                relative: 0.05,
                distribution: Distribution::Normal,
                group: Some("same-batch".into()),
            },
            ToleranceParameter {
                parameter: "load".into(),
                relative: 0.1,
                distribution: Distribution::Normal,
                group: Some("same-batch".into()),
            },
        ],
    });
    let p = plan_experiment(s.clone(), &resources()).unwrap();
    assert_eq!(p.cases.len(), 6);
    assert_eq!(
        p.identity,
        plan_experiment(s.clone(), &resources()).unwrap().identity
    );
    for case in p.cases.iter().skip(1) {
        let r = kessetsu_core::ir::parse_quantity(&case.parameters["resistance"], SIUnit::Ohm)
            .unwrap()
            .value;
        let l = kessetsu_core::ir::parse_quantity(&case.parameters["load"], SIUnit::Ohm)
            .unwrap()
            .value;
        assert!(((r / 1000.0 - 1.0) / 0.05 - (l / 10000.0 - 1.0) / 0.1).abs() < 1e-12);
    }
    s.tolerances.as_mut().unwrap().seed = 43;
    assert_ne!(
        p.identity,
        plan_experiment(s.clone(), &resources()).unwrap().identity
    );
    let t = s.tolerances.as_mut().unwrap();
    t.mode = ToleranceMode::Corners;
    for p in &mut t.parameters {
        p.group = None;
        p.distribution = Distribution::Uniform;
    }
    assert_eq!(
        plan_experiment(s.clone(), &resources())
            .unwrap()
            .cases
            .len(),
        5
    );
}

#[test]
fn loaded_filter_matches_analytical_gain_and_cutoff_and_keeps_failed_cases() {
    let result = run(&spec());
    assert_eq!(result.summary.total, 6);
    assert_eq!(result.summary.passed, 3);
    assert_eq!(result.summary.failed, 3);
    assert_eq!(
        result.summary.errors,
        0,
        "{:?}",
        result
            .cases
            .iter()
            .map(|c| (&c.case_id, &c.errors, &c.measurements))
            .collect::<Vec<_>>()
    );
    let c = 159.154943e-9;
    for (case, row) in result.plan.cases.iter().zip(&result.cases) {
        let r = kessetsu_core::ir::parse_quantity(&case.parameters["resistance"], SIUnit::Ohm)
            .unwrap()
            .value;
        let l = kessetsu_core::ir::parse_quantity(&case.parameters["load"], SIUnit::Ohm)
            .unwrap()
            .value;
        let cutoff = (1.0 / r + 1.0 / l) / (std::f64::consts::TAU * c);
        let gain = l / (r + l) / (1.0 + (100.0 / cutoff).powi(2)).sqrt();
        assert!((row.measurements["gain_100Hz"].value.unwrap() - gain).abs() < 0.0001);
        assert!((row.measurements["cutoff"].value.unwrap() / cutoff - 1.0).abs() < 0.002);
        assert!(row.simulation.as_ref().unwrap().datasets.len() == 1);
    }
    let best = result.summary.best_case.as_ref().unwrap();
    let index = result
        .plan
        .cases
        .iter()
        .position(|c| &c.id == best)
        .unwrap();
    assert_eq!(result.cases[index].status, CaseStatus::Passed);
    assert_eq!(result.plan.cases[index].parameters["resistance"], "1200Ohm");
    assert!(results_csv(&result).contains("failed"));
    assert!(datasets_csv(&result).lines().count() > 1000);
    assert!(
        plot_svg(&result, 0, "V(OUT)", &[])
            .unwrap()
            .contains("<polyline")
    );
    assert!(report_html(&result).contains(&result.identity));
}

#[test]
fn semiconductor_conditions_and_seeded_results_repeat_numerically() {
    let s = decode_spec(include_bytes!(
        "../../examples/studies/transistor-driver.kessstudy.json"
    ))
    .unwrap();
    let result = run(&s);
    assert_eq!(result.summary.total, 12);
    assert_eq!(
        result.summary.errors,
        0,
        "{:?}",
        result
            .cases
            .iter()
            .map(|c| (&c.case_id, &c.errors, &c.measurements))
            .collect::<Vec<_>>()
    );
    assert!(result.summary.passed > 0 && result.summary.failed > 0);
    assert_ne!(
        result.cases[4].measurements["transistor_power"].value,
        result.cases[5].measurements["transistor_power"].value
    );
    let mut s = spec();
    s.axes.clear();
    s.tolerances = Some(ToleranceStudy {
        mode: ToleranceMode::MonteCarlo,
        seed: 42,
        samples: 2,
        parameters: vec![ToleranceParameter {
            parameter: "resistance".into(),
            relative: 0.05,
            distribution: Distribution::Uniform,
            group: None,
        }],
    });
    let a = run(&s);
    let b = run(&s);
    for (left, right) in a.cases.iter().zip(&b.cases) {
        assert_eq!(left.case_id, right.case_id);
        for (name, value) in &left.measurements {
            assert!((value.value.unwrap() - right.measurements[name].value.unwrap()).abs() <= 1e-8);
        }
    }
}

#[test]
fn stale_requirements_models_solver_and_dropped_or_corrupt_rows_refuse_resume() {
    let mut s = spec();
    s.source = s
        .source
        .lines()
        .filter(|l| !l.starts_with("assert "))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    s.requirements = Some("assert gain_at(V(OUT),V(IN),100Hz) > 0.8\n".into());
    let plan = plan_experiment(s.clone(), &resources()).unwrap();
    let result = new_results(plan.clone(), solver(), "runtime-1".into());
    validate_results(&result, &plan, &solver(), "runtime-1").unwrap();
    s.requirements = Some("assert gain_at(V(OUT),V(IN),100Hz) > 0.7\n".into());
    assert!(
        validate_results(
            &result,
            &plan_experiment(s.clone(), &resources()).unwrap(),
            &solver(),
            "runtime-1"
        )
        .is_err()
    );
    assert!(validate_results(&result, &plan, &solver(), "runtime-2").is_err());
    let mut changed = plan.clone();
    changed.model_manifests.clear();
    assert!(validate_results(&result, &changed, &solver(), "runtime-1").is_err());
    let mut dropped = result.clone();
    dropped.cases.pop();
    assert!(validate_results(&dropped, &plan, &solver(), "runtime-1").is_err());
    let mut corrupt = result;
    corrupt.cases[0].status = CaseStatus::Passed;
    assert!(validate_results(&corrupt, &plan, &solver(), "runtime-1").is_err());
    s.source.push_str("assert peak(V(OUT)) < 3V\n");
    assert!(plan_experiment(s.clone(), &resources()).is_err());
}

#[test]
fn partial_errors_cancellation_and_invalid_candidates_remain_visible() {
    let mut s = spec();
    s.source = format!("param inverse: ratio = 1Ohm / resistance\n{}", s.source);
    s.axes = vec![Axis {
        parameter: "resistance".into(),
        values: Values::List {
            values: vec!["0Ohm".into(), "1kOhm".into()],
        },
    }];
    let p = plan_experiment(s.clone(), &resources()).unwrap();
    assert!(compile_case(&p, &p.cases[0], &resources()).is_err());
    let mut result = new_results(p, solver(), "runtime-1".into());
    result.cases[0] = failed_case(
        &result.plan.cases[0].id,
        CaseStatus::Error,
        "invalid candidate".into(),
    );
    result.cases[1] = failed_case(
        &result.plan.cases[1].id,
        CaseStatus::Cancelled,
        "cancelled".into(),
    );
    summarize(&mut result);
    assert_eq!(result.summary.total, 2);
    assert_eq!(result.summary.errors, 1);
    assert_eq!(result.summary.cancelled, 1);
    assert!(CaseStatus::Error.reusable());
    assert!(!CaseStatus::Cancelled.reusable());
    validate_results(&result, &result.plan, &solver(), "runtime-1").unwrap();
}

#[test]
fn exact_external_model_bytes_invalidate_the_plan_and_stay_out_of_exports() {
    let original = b"* study synthetic open fixture\n.SUBCKT StudyAmp INP INN VCC VEE OUT\nE1 OUT 0 INP INN 100000\n.ENDS StudyAmp\n".to_vec();
    let changed = String::from_utf8(original.clone())
        .unwrap()
        .replace("100000", "50000")
        .into_bytes();
    let mut s = spec();
    s.axes.clear();
    s.measurements.clear();
    s.objective = None;
    s.source = format!(
        "external_subcircuit opamp AMP (in_p,in_n,vcc,vee,out) file=\"models/amp.lib\" entry=StudyAmp sha256={} version=1.0.0 license=MIT source=\"synthetic study fixture\" simulator=ngspice redistribution=permitted\nnet GND\nnet VCC\nnet VEE\nnet OUT\nsource VP 6V\nsource VN 6V\nopamp U1 AMP\nresistor RL 10k\nconnect VP.minus, VN.plus, U1.in_p, RL.p2 to GND\nconnect VP.plus, U1.vcc to VCC\nconnect VN.minus, U1.vee to VEE\nconnect U1.in_n, U1.out, RL.p1 to OUT\nsimulate op\n",
        hash_bytes(&original).trim_start_matches("sha256:")
    );
    let resources = BTreeMap::from([("models/amp.lib".into(), original.clone())]);
    let plan = plan_experiment(s.clone(), &resources).unwrap();
    let result = new_results(plan.clone(), solver(), "runtime-1".into());
    let different = BTreeMap::from([("models/amp.lib".into(), changed.clone())]);
    assert!(plan_experiment(s.clone(), &different).is_err());
    s.source = s.source.replace(
        hash_bytes(&original).trim_start_matches("sha256:"),
        hash_bytes(&changed).trim_start_matches("sha256:"),
    );
    let changed_plan = plan_experiment(s, &different).unwrap();
    assert_ne!(plan.identity, changed_plan.identity);
    assert!(validate_results(&result, &changed_plan, &solver(), "runtime-1").is_err());
    assert!(!serde_json::to_string(&result).unwrap().contains("E1 OUT"));
}

#[test]
fn malformed_successful_simulator_data_is_recorded_as_an_error() {
    let result = run(&spec());
    let case = &result.plan.cases[0];
    let compiled = compile_case(&result.plan, case, &resources()).unwrap();
    let mut simulation = result.cases[0].simulation.clone().unwrap();
    if let kessetsu_core::simulation::Dataset::Ac(data) = &mut simulation.datasets[0].data {
        data.signals.get_mut("out").unwrap().real.pop();
    }
    let row = evaluate_case(&result.plan, case, &compiled, simulation);
    assert_eq!(row.status, CaseStatus::Error);
    assert!(row.errors[0].contains("Malformed complex"));
    assert!(row.simulation.is_none());
}

#[test]
fn revisions_and_temperature_preserve_fixed_constraints_and_ir_analyses() {
    let mut s = spec();
    s.axes.clear();
    s.revisions = vec![
        Revision {
            name: "Baseline".into(),
            source: None,
            parameters: BTreeMap::new(),
        },
        Revision {
            name: "Candidate".into(),
            source: None,
            parameters: BTreeMap::from([("resistance".into(), "2kOhm".into())]),
        },
    ];
    s.temperatures_c = vec![27.0, 85.0];
    let p = plan_experiment(s.clone(), &resources()).unwrap();
    assert_eq!(p.cases.len(), 4);
    let a = compile_case(&p, &p.cases[0], &resources()).unwrap();
    let b = compile_case(&p, &p.cases[2], &resources()).unwrap();
    assert_eq!(
        a.ir.as_ref().unwrap().assertions,
        b.ir.as_ref().unwrap().assertions
    );
    let netlist = temperature_netlist(a.spice_netlist.as_ref().unwrap(), 85.0).unwrap();
    assert!(netlist.find(".temp 85").unwrap() < netlist.find(".control").unwrap());
    assert!(temperature_netlist("* test\n.end\n", f64::NAN).is_err());
    let _typed_quantity = Quantity {
        value: 1.0,
        unit: SIUnit::Joule,
    };
}

#[test]
fn cli_creation_checkpoint_resume_and_exports_are_safe_and_compact() {
    let workspace = TestWorkspace::new("study-cli");
    let file = workspace.write("filter.kess", &spec().source);
    let spec_file = workspace.path().join("filter.kessstudy.json");
    let output = workspace.path().join("results.json");
    let arg = |p: &std::path::Path| p.to_string_lossy().into_owned();
    let created = workspace.run_cli(&[
        "study",
        "create",
        &arg(&file),
        "--output",
        &arg(&spec_file),
        "--sweep",
        "resistance=820Ohm,1kOhm",
        "--format",
        "json",
    ]);
    assert_eq!(
        created.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&created.stdout)
    );
    let planned = workspace.run_cli(&["study", "plan", &arg(&spec_file), "--format", "json"]);
    assert_eq!(planned.status.code(), Some(0));
    let ran = workspace.run_cli(&[
        "study",
        "run",
        &arg(&spec_file),
        "--output",
        &arg(&output),
        "--format",
        "json",
    ]);
    assert_eq!(
        ran.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&ran.stdout).unwrap();
    assert!(envelope.get("study").is_some());
    assert!(envelope["study"].get("datasets").is_none());
    let report: ExperimentResults = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report.summary.passed, 2);
    let mut partial = report.clone();
    partial.cases[1] = failed_case(
        &partial.plan.cases[1].id,
        CaseStatus::Cancelled,
        "cancelled".into(),
    );
    summarize(&mut partial);
    fs::write(&output, serde_json::to_vec(&partial).unwrap()).unwrap();
    let resumed = workspace.run_cli(&[
        "study",
        "run",
        &arg(&spec_file),
        "--output",
        &arg(&output),
        "--resume",
        "--format",
        "json",
    ]);
    assert_eq!(
        resumed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&resumed.stdout)
    );
    let resumed: ExperimentResults = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(
        resumed.cases[0].content_sha256,
        report.cases[0].content_sha256
    );
    assert_eq!(resumed.summary.passed, 2);
    for target in ["json", "csv", "data-csv", "svg", "html"] {
        let path = workspace.path().join(format!("export.{target}"));
        let exported = workspace.run_cli(&[
            "study",
            "export",
            &arg(&output),
            "--output",
            &arg(&path),
            "--target",
            target,
            "--format",
            "json",
        ]);
        assert_eq!(
            exported.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&exported.stdout)
        );
        assert!(fs::metadata(path).unwrap().len() > 100);
    }
    let unsafe_write = workspace.run_cli(&[
        "study",
        "run",
        &arg(&spec_file),
        "--output",
        &arg(&spec_file),
        "--force",
        "--format",
        "json",
    ]);
    assert_eq!(unsafe_write.status.code(), Some(2));
    assert!(!file.with_extension("spice").exists());
    assert!(!workspace.path().join("kessetsu.lock").exists());
}
