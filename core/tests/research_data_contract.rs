mod common;
use common::TestWorkspace;
use kessetsu_core::experiment::hash_bytes;
use kessetsu_core::ir::SIUnit;
use kessetsu_core::research_data::*;
use serde_json::Value;

fn mapping(column: usize, name: &str, unit: SIUnit, source_unit: &str) -> ColumnMapping {
    ColumnMapping {
        column,
        name: name.into(),
        unit,
        source_unit: source_unit.into(),
        gain: 1.0,
        offset: 0.0,
    }
}
fn spec() -> ImportSpec {
    ImportSpec {
        schema_version: IMPORT_SCHEMA.into(),
        name: "Synthetic test fixture".into(),
        file_name: "fixture.csv".into(),
        dialect: CsvDialect::default(),
        metadata: DataMetadata {
            origin: Origin::Synthetic,
            ..Default::default()
        },
        axis: mapping(0, "time", SIUnit::Second, "s"),
        axis_order: AxisOrder::Increasing,
        signals: vec![mapping(1, "out", SIUnit::Volt, "V")],
        missing: MissingPolicy::Error,
        missing_tokens: vec!["".into(), "NA".into()],
    }
}
fn comparison() -> ComparisonSpec {
    ComparisonSpec {
        schema_version: COMPARISON_SCHEMA.into(),
        name: "Explicit comparison".into(),
        signals: vec![SignalPair {
            data_signal: "out".into(),
            reference_signal: "out".into(),
        }],
        coverage: Coverage::RequireFull,
        interpolation: Interpolation::Linear,
        reference_axis_shift: 0.0,
        window: None,
        max_reference_gap: None,
    }
}
fn data(csv: &str) -> ResearchData {
    import_csv(csv, spec()).unwrap()
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}

#[test]
fn raw_utf8_bom_crlf_and_unit_calibration_are_preserved() {
    let csv = "\u{feff}time,voltage,current\r\n0,1000,250\r\n1,2000,500\r\n";
    let mut s = spec();
    s.axis.source_unit = "ms".into();
    s.signals[0].source_unit = "mV".into();
    s.signals[0].gain = 2.0;
    s.signals[0].offset = -0.5;
    s.signals.push(mapping(2, "current", SIUnit::Ampere, "uA"));
    let result = import_csv(csv, s).unwrap();
    assert_eq!(result.raw_csv, csv);
    assert_eq!(result.raw_sha256, hash_bytes(csv.as_bytes()));
    assert_eq!(result.axis.values, [0.0, 0.001]);
    assert_eq!(result.signals[0].values, [1.5, 3.5]);
    near(result.signals[1].values[0], 0.00025);
    assert_eq!(result.source_records, [2, 3]);
    validate_data(&result).unwrap();
    let mut clone = result.clone();
    clone.raw_csv = csv.replace("\r\n", "\n");
    assert!(validate_data(&clone).is_err());
}
#[test]
fn locale_decimal_comma_and_fortran_exponents_are_explicit() {
    let mut s = spec();
    s.dialect.delimiter = Delimiter::Semicolon;
    s.dialect.decimal = Decimal::Comma;
    let result = import_csv("t;v\n0;1,25D-3\n1;2,5e-3\n", s.clone()).unwrap();
    assert_eq!(result.signals[0].values, [0.00125, 0.0025]);
    assert!(import_csv("t;v\n0;1.25\n1;2.5\n", s.clone()).is_err());
    s.dialect.delimiter = Delimiter::Comma;
    assert!(import_csv("t,v\n0,1\n1,2\n", s).is_err());
}
#[test]
fn quoted_multiline_unicode_fields_and_preview_have_exact_record_numbers() {
    let csv = "t,v,note\n0,1,\"Türkçe, \"\"scope\"\"\nsecond line\"\n1,2,done\n";
    let preview = preview_csv(csv, &CsvDialect::default()).unwrap();
    assert_eq!(preview.headers, ["t", "v", "note"]);
    assert_eq!(preview.sample[1].line, 4);
    assert!(preview.sample[0].fields[2].contains("\"scope\"\n"));
    assert_eq!(data(csv).source_records, [2, 3]);
}

#[test]
fn large_preview_fields_are_bounded_without_changing_original_evidence() {
    let note = "µ".repeat(400);
    let csv = format!("t,v,note\n0,1,{note}\n1,2,done");
    let preview = preview_csv(&csv, &CsvDialect::default()).unwrap();
    assert_eq!(preview.schema_version, PREVIEW_SCHEMA);
    assert_eq!(preview.sample[0].fields[2].chars().count(), 256);
    assert_eq!(preview.sample[0].truncated_fields, [2]);
    assert_eq!(data(&csv).raw_csv, csv);
}
#[test]
fn headerless_and_preamble_records_are_not_guessed() {
    let mut s = spec();
    s.dialect.header = false;
    s.dialect.preamble_records = 1;
    let csv = "Oscilloscope capture\n0,1\n1,2\n";
    let result = import_csv(csv, s.clone()).unwrap();
    assert_eq!(result.source_records, [2, 3]);
    let preview = preview_csv(csv, &s.dialect).unwrap();
    assert_eq!(preview.headers, ["Column 1", "Column 2"]);
    assert_eq!(preview.data_records, 2);
}
#[test]
fn missing_and_blank_records_are_traceable_without_zero_filling() {
    let csv = "t,v\n0,1\n\n1,NA\n2,3\n";
    assert!(
        import_csv(csv, spec())
            .unwrap_err()
            .contains("Missing mapped value")
    );
    let mut s = spec();
    s.missing = MissingPolicy::SkipRow;
    let result = import_csv(csv, s).unwrap();
    assert_eq!(result.source_records, [2, 5]);
    assert_eq!(result.records_seen, 3);
    assert_eq!(result.skipped.len(), 2);
    assert_eq!(result.skipped[1].reason, "missing_mapped_value");
    assert_eq!(result.axis.values, [0.0, 2.0]);
    assert_eq!(result.signals[0].values, [1.0, 3.0]);
}
#[test]
fn malformed_quotes_ragged_records_and_nonfinite_values_fail() {
    for csv in [
        "t,v\n0,1\n1,\"2",
        "t,v\n0,1\n1,2\"",
        "t,v\n0,1\n1,\"2\"oops",
        "t,v\n0,1\n1,2,3",
        "t,v\n0,1\n1,NaN",
        "t,v\n0,1\n1,inf",
        "t,v\n0,1\n1,1e-999",
        "t,v\n0,1\n1,1k",
    ] {
        assert!(import_csv(csv, spec()).is_err(), "accepted malformed CSV");
    }
    assert!(preview_csv("a,b\n0,\"unterminated", &CsvDialect::default()).is_err());
}
#[test]
fn invalid_units_gain_indices_names_and_order_fail_closed() {
    let csv = "t,v\n0,1\n1,2\n";
    for field in ["unit", "gain", "index", "name", "schema"] {
        let mut s = spec();
        match field {
            "unit" => s.signals[0].source_unit = "k".into(),
            "gain" => s.axis.gain = 0.0,
            "index" => s.axis.column = 4,
            "name" => s.signals[0].name = "TIME".into(),
            _ => s.schema_version = "future".into(),
        }
        assert!(import_csv(csv, s).is_err());
    }
    for csv in ["t,v\n1,1\n0,2", "t,v\n0,1\n0,2"] {
        assert!(import_csv(csv, spec()).is_err());
    }
    let mut s = spec();
    s.axis.unit = SIUnit::Hertz;
    s.axis.source_unit = "Hz".into();
    assert!(import_csv(csv, s).is_err());
}
#[test]
fn input_field_column_record_and_normalized_value_budgets_are_enforced() {
    assert!(preview_csv(&"x".repeat(MAX_CSV_BYTES + 1), &CsvDialect::default()).is_err());
    assert!(
        preview_csv(
            &format!("t,v\n0,{}", "x".repeat(65_537)),
            &CsvDialect::default()
        )
        .is_err()
    );
    assert!(preview_csv(&vec!["x"; 65].join(","), &CsvDialect::default()).is_err());
    assert!(
        preview_csv(
            &format!("t,v\n{}", "0,1\n".repeat(MAX_ROWS + 1)),
            &CsvDialect::default()
        )
        .is_err()
    );
    let mut s = spec();
    s.signals = (1..=10)
        .map(|i| mapping(i, &format!("s{i}"), SIUnit::Volt, "V"))
        .collect();
    let mut csv = format!(
        "{}\n",
        (0..=10)
            .map(|i| format!("c{i}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    for i in 0..91_000 {
        csv.push_str(&format!("{i},1,1,1,1,1,1,1,1,1,1\n"));
    }
    assert!(import_csv(&csv, s).unwrap_err().contains("one million"));
}
#[test]
fn altered_normalized_arrays_or_dropped_rows_cannot_be_resealed() {
    let original = data("t,v\n0,1\n1,2\n");
    for alter in [0, 1, 2] {
        let mut damaged = original.clone();
        match alter {
            0 => damaged.signals[0].values[0] = 99.0,
            1 => {
                damaged.source_records.pop();
            }
            _ => damaged.records_seen = 99,
        }
        damaged.identity.clear();
        damaged.identity = hash_bytes(&serde_json::to_vec(&damaged).unwrap());
        assert!(validate_data(&damaged).is_err());
    }
    let roundtrip: ResearchData =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    validate_data(&roundtrip).unwrap();
}
#[test]
fn residuals_interpolate_on_data_axis_and_have_defined_signed_metrics() {
    let observed = data("t,v\n0,0.1\n0.5,1.1\n1,2.1\n");
    let reference = data("t,v\n0,0\n1,2\n");
    let result = compare_data(&observed, &reference, comparison()).unwrap();
    let signal = &result.signals[0];
    assert_eq!(signal.metrics.matched, 3);
    near(signal.metrics.bias, -0.1);
    near(signal.metrics.mae, 0.1);
    near(signal.metrics.rmse, 0.1);
    near(signal.points[1].predicted.unwrap(), 1.0);
    assert_eq!(result.data_identity, observed.identity);
    assert_eq!(result.reference_origin, Origin::Synthetic);
}
#[test]
fn coverage_windows_and_gaps_keep_every_observation_visible() {
    let observed = data("t,v\n0,0\n0.5,1\n1,2\n1.5,3\n2,4\n");
    let reference = data("t,v\n0.5,1\n1.5,3\n");
    assert!(
        compare_data(&observed, &reference, comparison())
            .unwrap_err()
            .contains("coverage")
    );
    let mut s = comparison();
    s.coverage = Coverage::OverlapOnly;
    let result = compare_data(&observed, &reference, s.clone()).unwrap();
    assert_eq!(result.signals[0].metrics.unmatched, 2);
    assert_eq!(result.signals[0].points.len(), 5);
    assert!(result.signals[0].points[0].predicted.is_none());
    s.coverage = Coverage::RequireFull;
    s.window = Some([0.5, 1.5]);
    let result = compare_data(&observed, &reference, s.clone()).unwrap();
    assert_eq!(result.signals[0].metrics.excluded_by_window, 2);
    s.window = None;
    s.coverage = Coverage::OverlapOnly;
    s.max_reference_gap = Some(0.75);
    let result = compare_data(&observed, &reference, s).unwrap();
    assert_eq!(result.signals[0].metrics.matched, 2);
    assert_eq!(result.signals[0].points[2].status, "reference_gap");
}
#[test]
fn descending_sweeps_are_not_sorted_or_confused_with_time() {
    let mut s = spec();
    s.axis.unit = SIUnit::Volt;
    s.axis.source_unit = "V".into();
    s.axis_order = AxisOrder::Decreasing;
    let observed = import_csv("x,v\n3,6\n2,4\n1,2", s.clone()).unwrap();
    let reference = import_csv("x,v\n3,6\n1,2", s).unwrap();
    let result = compare_data(&observed, &reference, comparison()).unwrap();
    assert_eq!(result.signals[0].points[0].axis, 3.0);
    near(result.signals[0].metrics.rmse, 0.0);
}
#[test]
fn log_frequency_interpolation_and_explicit_axis_shift_are_recorded() {
    let mut s = spec();
    s.axis.unit = SIUnit::Hertz;
    s.axis.source_unit = "Hz".into();
    let observed = import_csv("f,v\n1,0\n10,1\n100,2", s.clone()).unwrap();
    let reference = import_csv("f,v\n1,0\n100,2", s).unwrap();
    let mut c = comparison();
    c.interpolation = Interpolation::LogAxis;
    near(
        compare_data(&observed, &reference, c).unwrap().signals[0]
            .metrics
            .rmse,
        0.0,
    );
    let mut c = comparison();
    c.reference_axis_shift = 1.0;
    let result = compare_data(&data("t,v\n1,0\n2,2"), &data("t,v\n0,0\n1,2"), c).unwrap();
    near(result.signals[0].metrics.rmse, 0.0);
    assert_eq!(result.spec.reference_axis_shift, 1.0);
}
#[test]
fn incompatible_units_duplicate_mappings_empty_overlap_and_extreme_ranges_fail() {
    let left = data("t,v\n0,0\n1,1");
    let mut s = spec();
    s.signals[0].unit = SIUnit::Ampere;
    s.signals[0].source_unit = "A".into();
    assert!(
        compare_data(
            &left,
            &import_csv("t,v\n0,0\n1,1", s).unwrap(),
            comparison()
        )
        .is_err()
    );
    let mut c = comparison();
    c.signals.push(c.signals[0].clone());
    assert!(compare_data(&left, &left, c).is_err());
    assert!(compare_data(&left, &data("t,v\n2,0\n3,1"), comparison()).is_err());
    let mut s = spec();
    s.axis.unit = SIUnit::Volt;
    s.axis.source_unit = "V".into();
    assert!(
        compare_data(
            &import_csv("x,v\n-1e308,0\n0,1\n1e308,2", s.clone()).unwrap(),
            &import_csv("x,v\n-1e308,0\n1e308,2", s).unwrap(),
            comparison()
        )
        .is_err()
    );
    let huge = compare_data(
        &data("t,v\n0,-1e200\n1,-1e200"),
        &data("t,v\n0,1e200\n1,1e200"),
        comparison(),
    )
    .unwrap();
    assert!(huge.signals[0].metrics.rmse.is_finite());
}

#[test]
fn smallest_representable_residuals_do_not_disappear_during_reduction() {
    let report = compare_data(
        &data("t,v\n0,0\n1,0"),
        &data("t,v\n0,5e-324\n1,5e-324"),
        comparison(),
    )
    .unwrap();
    let metrics = &report.signals[0].metrics;
    assert!(metrics.max_absolute > 0.0);
    assert_eq!(metrics.bias, metrics.max_absolute);
    assert_eq!(metrics.mae, metrics.max_absolute);
    assert_eq!(metrics.rmse, metrics.max_absolute);
}

fn cli_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn documented_synthetic_tutorial_uses_normalized_units_and_full_coverage() {
    let observed = import_csv(
        include_str!("../../examples/research/scope-style.csv"),
        serde_json::from_str(include_str!(
            "../../examples/research/scope-style.kessimport.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let reference = import_csv(
        include_str!("../../examples/research/rc-reference.csv"),
        serde_json::from_str(include_str!(
            "../../examples/research/rc-reference.kessimport.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let spec = serde_json::from_str(include_str!(
        "../../examples/research/rc-comparison.kesscompare.json"
    ))
    .unwrap();
    let report = compare_data(&observed, &reference, spec).unwrap();
    assert_eq!(report.data_origin, Origin::Synthetic);
    assert_eq!(report.reference_origin, Origin::Synthetic);
    assert_eq!(report.signals[0].metrics.matched, 6);
    assert_eq!(report.signals[0].metrics.unmatched, 0);
    assert!((0.004..0.006).contains(&report.signals[0].metrics.rmse));
}
#[test]
fn cli_preview_import_compare_are_compact_safe_and_do_not_launch_a_solver() {
    let workspace = TestWorkspace::new("research-data");
    let csv = workspace.write("capture.csv", "t,v\n0,1\n1,2\n");
    let map = workspace.write("mapping.json", &serde_json::to_string(&spec()).unwrap());
    let report = workspace.path().join("capture.json");
    let preview =
        workspace.run_cli(&["data", "preview", csv.to_str().unwrap(), "--format", "json"]);
    assert_eq!(preview.status.code(), Some(0));
    assert_eq!(cli_json(&preview)["data"]["data_records"], 2);
    let args = [
        "data",
        "import",
        csv.to_str().unwrap(),
        "--mapping",
        map.to_str().unwrap(),
        "--output",
        report.to_str().unwrap(),
        "--format",
        "json",
    ];
    let imported = workspace.run_cli(&args);
    assert_eq!(imported.status.code(), Some(0));
    let envelope = cli_json(&imported);
    assert_eq!(envelope["command"], "data");
    assert_eq!(envelope["domain_versions"]["research_data"], DATA_SCHEMA);
    assert!(envelope["data"].get("raw_csv").is_none());
    assert!(envelope["data"]["axis"].get("values").is_none());
    assert_eq!(workspace.run_cli(&args).status.code(), Some(2));
    for input in [&csv, &map] {
        let failed = workspace.run_cli(&[
            "data",
            "import",
            csv.to_str().unwrap(),
            "--mapping",
            map.to_str().unwrap(),
            "--output",
            input.to_str().unwrap(),
            "--force",
            "--format",
            "json",
        ]);
        assert_eq!(failed.status.code(), Some(2));
    }
    let pair = workspace.write(
        "comparison.json",
        &serde_json::to_string(&comparison()).unwrap(),
    );
    let output = workspace.path().join("residuals.json");
    let compared = workspace.run_cli(&[
        "data",
        "compare",
        report.to_str().unwrap(),
        "--reference",
        report.to_str().unwrap(),
        "--mapping",
        pair.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(compared.status.code(), Some(0));
    assert_eq!(
        cli_json(&compared)["data"]["signals"][0]["metrics"]["rmse"],
        0.0
    );
    let artifact: DataComparison = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(artifact.signals[0].points.len(), 2);
    assert!(
        std::fs::read_dir(workspace.path())
            .unwrap()
            .all(|p| !matches!(
                p.unwrap().path().extension().and_then(|s| s.to_str()),
                Some("spice" | "lock")
            ))
    );
    assert_eq!(
        workspace
            .run_cli(&[
                "data",
                "preview",
                csv.to_str().unwrap(),
                "--param",
                "x=1",
                "--format",
                "json"
            ])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        workspace
            .run_cli(&[
                "data",
                "preview",
                csv.to_str().unwrap(),
                "--schema-version",
                "future",
                "--format",
                "json"
            ])
            .status
            .code(),
        Some(2)
    );
}
