mod common;

use common::TestWorkspace;
use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::tools::{PreferredValues, ToolRequest, calculate_tool};

fn divider(series: PreferredValues) -> ToolRequest {
    ToolRequest::Divider {
        input_voltage: "12V".into(),
        target_voltage: "3V".into(),
        lower_resistance: "10k".into(),
        load_resistance: Some("10k".into()),
        preferred_values: series,
    }
}

#[test]
fn loaded_divider_solves_the_actual_load_and_reports_power() {
    let result = calculate_tool(divider(PreferredValues::Exact)).unwrap();
    assert_eq!(result.components["R1"].value, 15_000.0);
    assert_eq!(result.results["output_voltage"].value, 3.0);
    assert_eq!(result.results["unloaded_voltage"].value, 4.8);
    assert!((result.results["loading_error"].value + 37.5).abs() < 1e-10);
    assert!((result.results["r1_power"].value - 0.0054).abs() < 1e-12);
    assert!((result.results["output_resistance"].value - 6000.0).abs() < 1e-10);
    assert!(!compile_source(&result.source, CompileOptions::all_outputs()).has_errors());
}

#[test]
fn preferred_values_recompute_achieved_results_and_cross_decades() {
    let result = calculate_tool(ToolRequest::RcLowpass {
        cutoff: "1kHz".into(),
        resistance: "1k".into(),
        preferred_values: PreferredValues::E12,
    })
    .unwrap();
    assert_eq!(result.components["C1"].value, 150e-9);
    assert!((result.results["cutoff"].value - 1061.032953945969).abs() < 1e-8);
    assert!(result.results["target_error"].value > 6.0);
    let report = compile_source(&result.source, CompileOptions::all_outputs());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let result = calculate_tool(ToolRequest::Divider {
        input_voltage: "2V".into(),
        target_voltage: "1V".into(),
        lower_resistance: "980".into(),
        load_resistance: None,
        preferred_values: PreferredValues::E24,
    })
    .unwrap();
    assert_eq!(result.components["R1"].value, 1000.0);
    assert_eq!(result.components["R2"].value, 1000.0);
    assert!(!result.source.contains("resistor RL"));
}

#[test]
fn invalid_units_domains_and_unrepresentable_results_fail_closed() {
    for (vin, target, lower, load) in [
        ("12V", "12V", "10k", None),
        ("12V", "0V", "10k", None),
        ("-12V", "3V", "10k", None),
        ("12V", "3V", "0", None),
        ("12V", "3V", "1V", None),
        ("12V", "3V", "10k", Some("0")),
        ("1e308V", "1e-308V", "10k", None),
        ("12V", "3V", "1e308T", None),
    ] {
        assert!(
            calculate_tool(ToolRequest::Divider {
                input_voltage: vin.into(),
                target_voltage: target.into(),
                lower_resistance: lower.into(),
                load_resistance: load.map(str::to_string),
                preferred_values: PreferredValues::Exact,
            })
            .is_err()
        );
    }
    assert!(
        calculate_tool(ToolRequest::RcLowpass {
            cutoff: "1e308Hz".into(),
            resistance: "1e308".into(),
            preferred_values: PreferredValues::Exact,
        })
        .is_err()
    );
}

#[test]
fn generated_circuits_match_independent_targets_in_real_ngspice() {
    let workspace = TestWorkspace::new("tool-simulation");
    let divider = calculate_tool(divider(PreferredValues::Exact)).unwrap();
    let divider_source = format!(
        "{}assert value(V(OUT)) > 2.9999V\nassert value(V(OUT)) < 3.0001V\nassert peak(I(VIN)) > 599.9uA\nassert peak(I(VIN)) < 600.1uA\n",
        divider.source
    );
    let rc = calculate_tool(ToolRequest::RcLowpass {
        cutoff: "1kHz".into(),
        resistance: "1k".into(),
        preferred_values: PreferredValues::E12,
    })
    .unwrap();
    // R=1k, C=150nF yields 1061.03 Hz; interpolation gets a 1% window.
    let rc_source = format!(
        "{}assert cutoff(V(OUT),V(IN)) > 1050Hz\nassert cutoff(V(OUT),V(IN)) < 1072Hz\n",
        rc.source
    );
    for source in [divider_source, rc_source] {
        let output = workspace.run_cli_with_stdin(&["test", "-", "--format", "json"], &source);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn cli_tools_use_the_existing_envelope_and_explicit_source_output() {
    let workspace = TestWorkspace::new("tool-cli");
    let args = [
        "tool", "divider", "--vin", "12V", "--target", "3V", "--lower", "10k", "--load", "10k",
        "--values", "exact", "--format", "json",
    ];
    let output = workspace.run_cli(&args);
    assert_eq!(output.status.code(), Some(0), "{:?}", output);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], "kessetsu.cli.v1");
    assert_eq!(json["calculation"]["schema_version"], "kessetsu.tool.v1");
    assert_eq!(
        json["calculation"]["results"]["output_voltage"]["value"],
        3.0
    );
    assert!(json["assertions"].is_null());
    assert_eq!(std::fs::read_dir(workspace.path()).unwrap().count(), 0);
    let mut args = args.to_vec();
    args.extend(["--output", "divider.kess"]);
    assert_eq!(workspace.run_cli(&args).status.code(), Some(0));
    assert_eq!(workspace.run_cli(&args).status.code(), Some(2));
    args.push("--force");
    assert_eq!(workspace.run_cli(&args).status.code(), Some(0));
    let source = std::fs::read_to_string(workspace.path().join("divider.kess")).unwrap();
    assert!(!compile_source(&source, CompileOptions::default()).has_errors());
    let invalid = workspace.run_cli(&[
        "tool", "divider", "--vin", "12V", "--target", "20V", "--lower", "10k", "--format", "json",
    ]);
    assert_eq!(invalid.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(json["diagnostics"][0]["code"], "KES-F003");
}
