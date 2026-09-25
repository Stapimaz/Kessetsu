mod common;

use common::TestWorkspace;
use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::simulation::{
    CancellationToken, Dataset, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use kessetsu_core::spice_import::{ImportSeverity, SPICE_IMPORT_SCHEMA_VERSION, import_spice};
use serde_json::Value;
use std::fs;

const REPRESENTATIVE: &str = ".title supported mixed-signal subset
VINPUT in 0 DC 0 AC 1
RLOAD in out 10M
CLOAD out 0 1u
DCLAMP out 0 1N4148
QGAIN out in 0 2N3904
MSW out in 0 0 IRF540
.ac dec 20 10 100k
.end
";

#[test]
fn supported_import_preserves_connectivity_values_models_and_analysis() {
    let first = import_spice(REPRESENTATIVE);
    assert!(!first.has_errors(), "{:?}", first.diagnostics);
    assert_eq!(first.schema_version, SPICE_IMPORT_SCHEMA_VERSION);
    assert_eq!(first.summary.components, 6);
    assert_eq!(first.summary.nets, 3);
    assert_eq!(first.summary.analyses, 1);
    let source = first.kess_source.as_deref().expect("verified source");
    let compile = compile_source(source, CompileOptions::default());
    assert!(!compile.has_errors(), "{:?}", compile.diagnostics);
    let spice = compile.spice_netlist.expect("canonical netlist");
    assert!(spice.contains("V_VINPUT in 0 AC 1"), "{spice}");
    // SPICE M means milli, whereas Kessetsu M means mega. The importer must
    // preserve the electrical value rather than the original spelling.
    assert!(spice.contains("R_RLOAD in out 0.01"), "{spice}");
    assert!(spice.contains("D_DCLAMP out 0 1N4148"), "{spice}");
    assert!(spice.contains("Q_QGAIN out in 0 2N3904"), "{spice}");
    assert!(spice.contains("M_MSW out in 0 0 IRF540"), "{spice}");
    assert!(spice.contains("ac dec 20 10 100000"), "{spice}");
    assert_eq!(
        first.kess_source,
        import_spice(REPRESENTATIVE).kess_source,
        "the same bytes must produce deterministic editable source"
    );

    let case_alias = import_spice("V1 IN 0 1\nR1 in OUT 1k\nR2 out 0 1k\n.op\n.end\n");
    assert!(!case_alias.has_errors(), "{:?}", case_alias.diagnostics);
    assert_eq!(case_alias.summary.nets, 3);
    let source = case_alias.kess_source.expect("verified source");
    let compile = compile_source(&source, CompileOptions::default());
    let graph = compile.graph.expect("canonical graph");
    assert!(graph.nets.iter().any(|net| {
        net.pins.iter().any(|pin| pin == "R1.p2") && net.pins.iter().any(|pin| pin == "R2.p1")
    }));
}

#[test]
fn waveform_numeric_nodes_and_dc_sweep_use_the_same_canonical_compiler() {
    let input = "Vdrive 1 0 PULSE(0 5 1u 2u 2u 3m 5m)
R1 1 0 1k
.tran 10u 10m uic
.dc vDRIVE -1 1 .5
.end
";
    let report = import_spice(input);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let source = report.kess_source.expect("verified source");
    assert!(source.contains("net N_1"), "{source}");
    assert!(source.contains("source Vdrive pulse(0,5,1e-6,2e-6,2e-6,3e-3,5e-3)"));
    assert!(source.contains("simulate dc Vdrive -1 1 0.5"));
    let compiled = compile_source(&source, CompileOptions::default());
    assert!(!compiled.has_errors());
    let spice = compiled.spice_netlist.expect("canonical netlist");
    assert!(spice.contains("tran 1e-5 0.01 uic"), "{spice}");
}

#[test]
fn unsupported_or_executable_constructs_fail_closed_at_their_source_lines() {
    let report = import_spice(".include ../secret.lib\nB1 out 0 V=sin(time)\n.end\n");
    assert!(report.has_errors());
    assert!(report.kess_source.is_none());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.line == Some(1)
            && diagnostic.severity == ImportSeverity::Error
            && diagnostic.message.contains("resource binding")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.line == Some(2)
            && diagnostic.severity == ImportSeverity::Error
            && diagnostic.message.contains("behavioral")
    }));
}

#[test]
fn cli_writes_verified_source_and_refuses_an_accidental_overwrite() {
    let workspace = TestWorkspace::new("spice-import");
    let input = workspace.write(
        "filter.cir",
        "V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 159.154943n\n.ac dec 40 10 100k\n.end\n",
    );
    let input = input.to_string_lossy().into_owned();
    let output = workspace.path().join("filter.kess");
    let output_arg = output.to_string_lossy().into_owned();

    let imported = workspace.run_cli(&[
        "--format",
        "json",
        "import",
        &input,
        "--output",
        &output_arg,
    ]);
    assert_eq!(imported.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&imported.stdout).expect("one JSON object");
    assert_eq!(json["status"], "success");
    assert_eq!(
        json["domain_versions"]["spice_import"],
        SPICE_IMPORT_SCHEMA_VERSION
    );
    let generated = fs::read_to_string(&output).expect("generated .kess");
    assert!(generated.contains("resistor R1 1e3"));
    assert!(!compile_source(&generated, CompileOptions::default()).has_errors());

    let repeated = workspace.run_cli(&["import", &input, "--output", &output_arg]);
    assert_eq!(repeated.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("KES-I003"));
}

#[test]
fn imported_rc_response_matches_the_original_supported_netlist_in_ngspice() {
    let input = "VINPUT in 0 AC 1\nR1 in out 1k\nC1 out 0 159.154943n\n.ac dec 40 10 100k\n.end\n";
    let imported = import_spice(input);
    assert!(!imported.has_errors(), "{:?}", imported.diagnostics);
    let compiled = compile_source(
        imported.kess_source.as_deref().expect("verified source"),
        CompileOptions::default(),
    );
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
    let analyses = compiled.ir.as_ref().expect("typed IR").analyses.clone();
    let reference = "Imported RC reference
VINPUT in 0 AC 1
R1 in out 1k
C1 out 0 159.154943n
.control
set wr_singlescale
set wr_vecnames
set numdgt=17
ac dec 40 10 100k
wrdata kessetsu-analysis-000-ac.data all
quit
.endc
.end
";
    let runner = NgspiceRunner::discover();
    let reference = runner
        .run(
            &SimulationRequest::new(reference, analyses.clone()),
            &CancellationToken::new(),
        )
        .expect("Ngspice reference should launch");
    let generated = runner
        .run(
            &SimulationRequest::new(compiled.spice_netlist.expect("canonical netlist"), analyses),
            &CancellationToken::new(),
        )
        .expect("Ngspice imported circuit should launch");
    assert!(reference.succeeded(), "{:?}", reference.errors);
    assert!(generated.succeeded(), "{:?}", generated.errors);
    let Dataset::Ac(reference) = &reference.datasets[0].data else {
        panic!("reference AC dataset missing")
    };
    let Dataset::Ac(generated) = &generated.datasets[0].data else {
        panic!("generated AC dataset missing")
    };
    assert_eq!(reference.frequency_hz, generated.frequency_hz);
    let reference = &reference.signals["out"];
    let generated = &generated.signals["out"];
    for ((expected_real, expected_imaginary), (actual_real, actual_imaginary)) in reference
        .real
        .iter()
        .zip(&reference.imaginary)
        .zip(generated.real.iter().zip(&generated.imaginary))
    {
        let scale = expected_real.abs().max(expected_imaginary.abs()).max(1.0);
        assert!((actual_real - expected_real).abs() <= scale * 1e-12);
        assert!((actual_imaginary - expected_imaginary).abs() <= scale * 1e-12);
    }
}
