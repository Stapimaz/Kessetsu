use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::exporter::{
    EXPORT_SCHEMA_VERSION, ExportFormat, ExportOptions, RenderBackground, export_capabilities,
    export_report,
};

fn compile_fixture() -> kessetsu_core::compiler::CompileReport {
    let source = include_str!("fixtures/benchmarks/rc_filter.kess");
    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    report
}

#[test]
fn advertised_formats_are_versioned_deterministic_and_connectivity_safe() {
    let report = compile_fixture();
    let capabilities = export_capabilities();
    assert_eq!(capabilities.len(), 7);
    for descriptor in capabilities {
        assert_eq!(descriptor.schema_version, EXPORT_SCHEMA_VERSION);
        let first = export_report(&report, descriptor.format, ExportOptions::default()).unwrap();
        let second = export_report(&report, descriptor.format, ExportOptions::default()).unwrap();
        assert_eq!(first.bytes, second.bytes, "{:?}", descriptor.format);
        assert_eq!(first.sha256, second.sha256, "{:?}", descriptor.format);
        assert_eq!(first.byte_length, first.bytes.len());
        assert_eq!(first.schema_version, EXPORT_SCHEMA_VERSION);
        assert!(first.connectivity_verified);
        assert_eq!(first.capability, descriptor.capability);
    }
}

#[test]
fn visual_and_eda_payloads_have_real_format_signatures() {
    let report = compile_fixture();
    let export = |format| export_report(&report, format, ExportOptions::default()).unwrap();
    assert!(export(ExportFormat::Svg).bytes.starts_with(b"<svg"));
    assert_eq!(&export(ExportFormat::Png).bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(export(ExportFormat::Pdf).bytes.starts_with(b"%PDF-"));
    assert!(export(ExportFormat::Kicad).bytes.starts_with(b"(kicad_sch"));
    assert!(
        export(ExportFormat::Ltspice)
            .bytes
            .starts_with(b"Version 4\nSHEET")
    );
    let json: serde_json::Value =
        serde_json::from_slice(&export(ExportFormat::SchematicJson).bytes).unwrap();
    assert_eq!(json["schema_version"], "kessetsu.schematic.v2");
}

#[test]
fn png_options_are_explicit_and_invalid_scale_fails_closed() {
    let report = compile_fixture();
    let white = export_report(
        &report,
        ExportFormat::Png,
        ExportOptions {
            scale: 1.0,
            background: RenderBackground::White,
        },
    )
    .unwrap();
    let transparent = export_report(
        &report,
        ExportFormat::Png,
        ExportOptions {
            scale: 1.0,
            background: RenderBackground::Transparent,
        },
    )
    .unwrap();
    assert_ne!(white.sha256, transparent.sha256);

    let error = export_report(
        &report,
        ExportFormat::Png,
        ExportOptions {
            scale: 100.0,
            background: RenderBackground::White,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "KES-X004");
}

#[test]
fn editable_exports_cover_the_complete_canonical_benchmark_corpus() {
    for (name, source) in [
        ("rc", include_str!("fixtures/benchmarks/rc_filter.kess")),
        ("gain", include_str!("fixtures/benchmarks/gain_stage.kess")),
        (
            "power",
            include_str!("fixtures/benchmarks/power_amplifier.kess"),
        ),
        (
            "dense_bias_network",
            include_str!("fixtures/schematic/dense_bias_network.kess"),
        ),
    ] {
        let report = compile_source(source, CompileOptions::all_outputs());
        assert!(!report.has_errors(), "{name}: {:?}", report.diagnostics);
        let schematic = report.schematic.as_ref().unwrap();
        for format in [ExportFormat::Kicad, ExportFormat::Ltspice] {
            let artifact = export_report(&report, format, ExportOptions::default()).unwrap();
            assert!(artifact.connectivity_verified, "{name} {format:?}");
            let text = String::from_utf8(artifact.bytes).unwrap();
            for component in &schematic.components {
                assert!(
                    text.contains(&component.reference),
                    "{name} {format:?} lost {}",
                    component.reference
                );
            }
        }
    }
}

#[test]
fn kicad_keeps_suppressed_generic_models_as_hidden_editable_values() {
    let report = compile_source(
        include_str!("fixtures/benchmarks/gain_stage.kess"),
        CompileOptions::all_outputs(),
    );
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let artifact = export_report(&report, ExportFormat::Kicad, ExportOptions::default()).unwrap();
    let text = String::from_utf8(artifact.bytes).unwrap();
    let generic_value = text
        .lines()
        .find(|line| line.contains("(property \"Value\" \"KESSETSU_OPAMP_V1\""))
        .expect("generic model remains editable in Value");
    assert!(generic_value.contains("(hide yes)"));
    let explicit_value = text
        .lines()
        .find(|line| line.contains("(property \"Value\" \"10 kΩ\""))
        .expect("explicit component value remains available");
    assert!(!explicit_value.contains("(hide yes)"));
}

#[test]
fn unsupported_symbol_and_unverified_connectivity_fail_closed() {
    let mut report = compile_fixture();
    report.schematic.as_mut().unwrap().components[0].symbol =
        kessetsu_core::component::CatalogSymbol::ModulePort;
    let unsupported =
        export_report(&report, ExportFormat::Ltspice, ExportOptions::default()).unwrap_err();
    assert_eq!(unsupported.code, "KES-X013");

    let mut report = compile_fixture();
    report.schematic.as_mut().unwrap().connectivity.verified = false;
    let unverified =
        export_report(&report, ExportFormat::Kicad, ExportOptions::default()).unwrap_err();
    assert_eq!(unverified.code, "KES-X003");
}

#[test]
fn ltspice_analysis_text_uses_dot_directives() {
    for source in [
        include_str!("fixtures/benchmarks/rc_filter.kess"),
        include_str!("fixtures/benchmarks/power_amplifier.kess"),
    ] {
        let report = compile_source(source, CompileOptions::all_outputs());
        let artifact =
            export_report(&report, ExportFormat::Ltspice, ExportOptions::default()).unwrap();
        let text = String::from_utf8(artifact.bytes).unwrap();
        assert!(text.contains("!.ac dec "));
        assert!(!text.contains("!ac "));
        assert!(!text.contains("!tran "));
        if source.contains("simulate tran") {
            assert!(text.contains(";.tran "));
            assert!(
                artifact
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("one active analysis"))
            );
        }
    }
}

#[test]
fn stale_verified_flag_cannot_export_broken_geometry() {
    let mut report = compile_fixture();
    let schematic = report.schematic.as_mut().unwrap();
    schematic.wires[0].points[0].x += 1;
    assert!(schematic.connectivity.verified);
    for format in [
        ExportFormat::Svg,
        ExportFormat::Png,
        ExportFormat::SchematicJson,
        ExportFormat::Kicad,
        ExportFormat::Ltspice,
    ] {
        assert_eq!(
            export_report(&report, format, ExportOptions::default())
                .unwrap_err()
                .code,
            "KES-X003"
        );
    }
    // Independent circuit semantics still export correctly without a drawing.
    assert!(export_report(&report, ExportFormat::Spice, ExportOptions::default()).is_ok());
}

#[test]
fn ltspice_dc_sweep_targets_the_exported_source_name() {
    for (name, expected) in [("VIN", "VIN"), ("bias", "V_bias")] {
        let source = format!(
            "net GND\nnet OUT\nsource {name} 1V\nresistor R1 1k\nconnect {name}.minus,R1.p2 to GND\nconnect {name}.plus,R1.p1 to OUT\nsimulate dc {name} 0V 5V 1V\n"
        );
        let report = compile_source(&source, CompileOptions::all_outputs());
        let artifact =
            export_report(&report, ExportFormat::Ltspice, ExportOptions::default()).unwrap();
        let text = String::from_utf8(artifact.bytes).unwrap();
        assert!(text.contains(&format!("!.dc {expected} 0 5 1")));
        assert!(text.contains(&format!("SYMATTR InstName {expected}\n")));
    }
}
