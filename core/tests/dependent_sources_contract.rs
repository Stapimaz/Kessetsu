mod common;

use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::exporter::{ExportFormat, ExportOptions, export_report};
use kessetsu_core::ir::{Analysis, ComponentKind, ComponentParams, Quantity, SIUnit};
use kessetsu_core::simulation::{
    CancellationToken, Dataset, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use kessetsu_core::spice_import::{ImportSeverity, import_spice};

#[test]
fn parsing_and_ir_for_all_four_dependent_sources() {
    let source = "
source V1 5V
vcvs E1 2.5
vccs G1 10mS
ccvs H1 500
cccs F1 4

connect E1.ctrl_p to V1.plus
connect G1.ctrl_p to V1.plus
connect H1.ctrl_p to V1.plus
connect F1.ctrl_p to V1.plus

connect E1.ctrl_n to V1.minus
connect G1.ctrl_n to V1.minus
connect H1.ctrl_n to V1.minus
connect F1.ctrl_n to V1.minus

net GND
connect V1.minus to GND

resistor R1 1k
resistor R2 1k
resistor R3 1k
resistor R4 1k

connect E1.p_out to R1.p1
connect E1.n_out, R1.p2 to GND
connect G1.p_out to R2.p1
connect G1.n_out, R2.p2 to GND
connect H1.p_out to R3.p1
connect H1.n_out, R3.p2 to GND
connect F1.p_out to R4.p1
connect F1.n_out, R4.p2 to GND
";

    let compiled = compile_source(source, CompileOptions::all_outputs());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    let ir = compiled.ir.expect("IR generated");
    let e1 = ir.components.iter().find(|c| c.id == "E1").unwrap();
    assert_eq!(e1.kind, ComponentKind::Vcvs);
    assert_eq!(
        e1.parameters,
        ComponentParams::DependentSource {
            value: Quantity {
                value: 2.5,
                unit: SIUnit::Ratio
            }
        }
    );

    let g1 = ir.components.iter().find(|c| c.id == "G1").unwrap();
    assert_eq!(g1.kind, ComponentKind::Vccs);
    assert_eq!(
        g1.parameters,
        ComponentParams::DependentSource {
            value: Quantity {
                value: 0.01,
                unit: SIUnit::Siemens
            }
        }
    );

    let h1 = ir.components.iter().find(|c| c.id == "H1").unwrap();
    assert_eq!(h1.kind, ComponentKind::Ccvs);
    assert_eq!(
        h1.parameters,
        ComponentParams::DependentSource {
            value: Quantity {
                value: 500.0,
                unit: SIUnit::Ohm
            }
        }
    );

    let f1 = ir.components.iter().find(|c| c.id == "F1").unwrap();
    assert_eq!(f1.kind, ComponentKind::Cccs);
    assert_eq!(
        f1.parameters,
        ComponentParams::DependentSource {
            value: Quantity {
                value: 4.0,
                unit: SIUnit::Ratio
            }
        }
    );

    let spice = compiled.spice_netlist.expect("SPICE netlist");
    assert!(spice.contains("E_E1"), "{spice}");
    assert!(spice.contains("G_G1"), "{spice}");
    assert!(spice.contains("V_SENSE_H1"), "{spice}");
    assert!(spice.contains("H_H1"), "{spice}");
    assert!(spice.contains("V_SENSE_F1"), "{spice}");
    assert!(spice.contains("F_F1"), "{spice}");
}

#[test]
fn dependent_source_expression_support() {
    let source = "
param gain : ratio = 3.5
source V1 2V
vcvs E1 {gain * 2}
net in
connect V1.plus, E1.ctrl_p to in
net out
connect E1.p_out to out
net GND
connect V1.minus, E1.ctrl_n, E1.n_out to GND
resistor R1 1k
connect R1.p1 to out
connect R1.p2 to GND
";
    let compiled = compile_source(source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
    let spice = compiled.spice_netlist.expect("SPICE netlist");
    assert!(spice.contains("E_E1 out 0 in 0 7"), "{spice}");
}

#[test]
fn erc_short_circuit_detection_for_dependent_sources() {
    // 1. VCVS shorted output
    let vcvs_short = "
source V1 5V
vcvs E1 2.0
connect E1.ctrl_p to V1.plus
connect E1.ctrl_n to V1.minus
net out
connect E1.p_out, E1.n_out to out
net GND
connect V1.minus to GND
";
    let rep = compile_source(vcvs_short, CompileOptions::default());
    assert!(rep.has_errors());
    assert!(
        rep.diagnostics.iter().any(|d| {
            d.code == "KES-E004"
                && d.message.contains(
                    "Controlled voltage source 'E1' has its output pins (p_out, n_out) shorted together.",
                )
        }),
        "{:?}",
        rep.diagnostics
    );

    // 2. CCVS shorted output
    let ccvs_short_out = "
source V1 5V
ccvs H1 100
connect H1.ctrl_p to V1.plus
connect H1.ctrl_n to V1.minus
net out
connect H1.p_out, H1.n_out to out
net GND
connect V1.minus to GND
";
    let rep = compile_source(ccvs_short_out, CompileOptions::default());
    assert!(rep.has_errors());
    assert!(
        rep.diagnostics.iter().any(|d| {
            d.code == "KES-E004"
                && d.message.contains(
                    "Controlled voltage source 'H1' has its output pins (p_out, n_out) shorted together.",
                )
        }),
        "{:?}",
        rep.diagnostics
    );

    // 3. CCVS shorted current-sense
    let ccvs_short_ctrl = "
source V1 5V
ccvs H1 100
net in
connect H1.ctrl_p, H1.ctrl_n to in
net out
connect H1.p_out to out
net GND
connect H1.n_out, V1.minus to GND
";
    let rep = compile_source(ccvs_short_ctrl, CompileOptions::default());
    assert!(rep.has_errors());
    assert!(
        rep.diagnostics.iter().any(|d| {
            d.code == "KES-E004"
                && d.message.contains(
                    "Current-controlled source 'H1' has its current-sense pins (ctrl_p, ctrl_n) shorted together.",
                )
        }),
        "{:?}",
        rep.diagnostics
    );

    // 4. CCCS shorted current-sense
    let cccs_short_ctrl = "
source V1 5V
cccs F1 2.0
net in
connect F1.ctrl_p, F1.ctrl_n to in
net out
connect F1.p_out to out
net GND
connect F1.n_out, V1.minus to GND
";
    let rep = compile_source(cccs_short_ctrl, CompileOptions::default());
    assert!(rep.has_errors());
    assert!(
        rep.diagnostics.iter().any(|d| {
            d.code == "KES-E004"
                && d.message.contains(
                    "Current-controlled source 'F1' has its current-sense pins (ctrl_p, ctrl_n) shorted together.",
                )
        }),
        "{:?}",
        rep.diagnostics
    );
}

#[test]
fn ngspice_op_simulation_of_all_four_dependent_sources() {
    let source = "
source V1 2V
vcvs E1 3.0
connect E1.ctrl_p to V1.plus
connect E1.ctrl_n to V1.minus

resistor Rload_e 1k
net out_e
connect E1.p_out, Rload_e.p1 to out_e
connect E1.n_out, Rload_e.p2 to GND

vccs G1 5mS
connect G1.ctrl_p to V1.plus
connect G1.ctrl_n to V1.minus
resistor Rload_g 1k
net out_g
connect G1.p_out, Rload_g.p1 to out_g
connect G1.n_out, Rload_g.p2 to GND

source V2 4V
resistor R_sense 2k
connect V2.plus to H1.ctrl_p
connect H1.ctrl_n to F1.ctrl_p
connect F1.ctrl_n to R_sense.p1
connect R_sense.p2, V2.minus to GND

ccvs H1 500
resistor Rload_h 1k
net out_h
connect H1.p_out, Rload_h.p1 to out_h
connect H1.n_out, Rload_h.p2 to GND

cccs F1 2.5
resistor Rload_f 1k
net out_f
connect F1.p_out, Rload_f.p1 to out_f
connect F1.n_out, Rload_f.p2 to GND

net GND
connect V1.minus to GND

simulate op
";

    let compiled = compile_source(source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    let spice = compiled.spice_netlist.expect("netlist");
    let runner = NgspiceRunner::discover();
    let request = SimulationRequest::new(&spice, vec![Analysis::OperatingPoint]);
    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("simulation should run");
    assert!(result.succeeded(), "Simulation failed: {:?}", result.errors);

    let first_dataset = result.datasets.first().expect("at least one dataset");
    if let Dataset::OperatingPoint { values } = &first_dataset.data {
        // V(in) = 2V. E1 gain = 3.0. V(out_e) = 6V
        let v_e = values
            .get("out_e")
            .or_else(|| values.get("v(out_e)"))
            .copied()
            .expect("out_e");
        assert!((v_e - 6.0).abs() < 1e-4, "expected 6V, got {v_e}");

        // VCCS: current from G1 flows out_g -> GND.
        // In SPICE, G n+ n- nc+ nc- value pulls current into n+ (flows n+ to n- internally),
        // so V(out_g) = -5mA/V * 2V * 1k = -10V.
        let v_g = values
            .get("out_g")
            .or_else(|| values.get("v(out_g)"))
            .copied()
            .expect("out_g");
        assert!(
            (v_g.abs() - 10.0).abs() < 1e-4,
            "expected ~10V magnitude, got {v_g}"
        );

        // Sense current for H1: V2 (4V) through R_sense (2k) + 0V sense sources = 2mA.
        // H1 transresistance = 500 Ohm. Output voltage across Rload_h (1k):
        // H1 is a voltage source with V = 500 * 2mA = 1.0V.
        let v_h = values
            .get("out_h")
            .or_else(|| values.get("v(out_h)"))
            .copied()
            .expect("out_h");
        assert!((v_h - 1.0).abs() < 1e-4, "expected 1.0V, got {v_h}");

        // F1 gain = 2.5 on 2mA = 5mA. Magnitude across 1k = 5V.
        let v_f = values
            .get("out_f")
            .or_else(|| values.get("v(out_f)"))
            .copied()
            .expect("out_f");
        assert!(
            (v_f.abs() - 5.0).abs() < 1e-4,
            "expected ~5V magnitude, got {v_f}"
        );
    } else {
        panic!("Expected OperatingPoint dataset");
    }
}

#[test]
fn ngspice_tran_simulation_of_vcvs() {
    let source = "
source V1 sine(0V, 1V, 1kHz)
vcvs E1 2.5
connect E1.ctrl_p to V1.plus
connect E1.ctrl_n to V1.minus

resistor R1 1k
net out
connect E1.p_out, R1.p1 to out
connect E1.n_out, R1.p2 to GND

net GND
connect V1.minus to GND

simulate tran 10us 1ms
";

    let compiled = compile_source(source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    let spice = compiled.spice_netlist.expect("netlist");
    let runner = NgspiceRunner::discover();
    let request = SimulationRequest::new(
        &spice,
        vec![Analysis::Transient {
            step: Quantity {
                value: 10e-6,
                unit: SIUnit::Second,
            },
            stop: Quantity {
                value: 1e-3,
                unit: SIUnit::Second,
            },
            use_initial_conditions: false,
        }],
    );
    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("simulation should run");
    assert!(result.succeeded(), "Simulation failed: {:?}", result.errors);

    let first_dataset = result.datasets.first().expect("at least one dataset");
    if let Dataset::Transient(dataset) = &first_dataset.data {
        let v_out = dataset
            .signals
            .get("out")
            .or_else(|| dataset.signals.get("v(out)"))
            .expect("out signal");
        let max_val = v_out.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_val = v_out.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(
            (max_val - 2.5).abs() < 0.1,
            "expected peak ~2.5V, got {max_val}"
        );
        assert!(
            (min_val - (-2.5)).abs() < 0.1,
            "expected valley ~-2.5V, got {min_val}"
        );
    } else {
        panic!("Expected Transient dataset");
    }
}

#[test]
fn spice_import_e_and_g_sources() {
    let spice_text = "* E and G source test
V1 in 0 2
E1 out_e 0 in 0 3.0
G1 out_g 0 in 0 5m
R1 out_e 0 1k
R2 out_g 0 1k
.op
.end
";

    let report = import_spice(spice_text);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);

    let kess_source = report.kess_source.expect("kess source");
    assert!(kess_source.contains("vcvs E1 3"), "{kess_source}");
    assert!(
        kess_source.contains("vccs G1 5e-3") || kess_source.contains("vccs G1 0.005"),
        "{kess_source}"
    );

    let compiled = compile_source(&kess_source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    let runner = NgspiceRunner::discover();
    let request = SimulationRequest::new(
        compiled.spice_netlist.unwrap(),
        vec![Analysis::OperatingPoint],
    );
    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("simulation");
    assert!(result.succeeded(), "{:?}", result.errors);

    // Test rejection of behavioral keywords
    let behavioral_e = "* behavioral test\nE1 out 0 VALUE = { V(in) * 2 }\n.end\n";
    let bad_report = import_spice(behavioral_e);
    assert!(bad_report.has_errors());
    assert!(bad_report.diagnostics.iter().any(|d| {
        d.message
            .contains("behavioral source construct 'VALUE' is not supported")
    }));
}

#[test]
fn spice_import_f_and_h_sources_with_0v_and_nonzero_sense() {
    // 1. Dedicated 0V sense source (should be absorbed)
    let spice_0v = "* F with 0V sense source
V1 in 0 4
R1 in sense 2k
V_SENSE sense 0 0
F1 out 0 V_SENSE 2.5
Rload out 0 1k
.op
.end
";
    let report_0v = import_spice(spice_0v);
    assert!(!report_0v.has_errors(), "{:?}", report_0v.diagnostics);
    // Info diagnostic KES-N012 should be emitted
    assert!(
        report_0v.diagnostics.iter().any(|d| {
            d.code == "KES-N012"
                && d.severity == ImportSeverity::Info
                && d.message.contains(
                    "folded 0V current-sense source 'V_SENSE' into controlled source 'F1'",
                )
        }),
        "{:?}",
        report_0v.diagnostics
    );
    let kess_0v = report_0v.kess_source.expect("kess source");
    assert!(
        !kess_0v.contains("source V_SENSE"),
        "0V source should be folded away"
    );
    assert!(kess_0v.contains("cccs F1 2.5"), "{kess_0v}");

    let compiled_0v = compile_source(&kess_0v, CompileOptions::default());
    assert!(!compiled_0v.has_errors(), "{:?}", compiled_0v.diagnostics);
    let runner = NgspiceRunner::discover();
    let result_0v = runner
        .run(
            &SimulationRequest::new(
                compiled_0v.spice_netlist.unwrap(),
                vec![Analysis::OperatingPoint],
            ),
            &CancellationToken::new(),
        )
        .expect("sim 0v");
    assert!(result_0v.succeeded());

    // 2. Non-zero voltage source (in-series insertion)
    let spice_nonzero = "* H with non-zero voltage source
V1 in 0 4
R1 in 0 2k
H1 out 0 V1 500
Rload out 0 1k
.op
.end
";
    let report_nonzero = import_spice(spice_nonzero);
    assert!(
        !report_nonzero.has_errors(),
        "{:?}",
        report_nonzero.diagnostics
    );
    let kess_nonzero = report_nonzero.kess_source.expect("kess source");
    // V1 must remain
    assert!(kess_nonzero.contains("source V1"), "{kess_nonzero}");
    assert!(kess_nonzero.contains("ccvs H1 500"), "{kess_nonzero}");

    let compiled_nonzero = compile_source(&kess_nonzero, CompileOptions::default());
    assert!(
        !compiled_nonzero.has_errors(),
        "{:?}",
        compiled_nonzero.diagnostics
    );
    let result_nonzero = runner
        .run(
            &SimulationRequest::new(
                compiled_nonzero.spice_netlist.unwrap(),
                vec![Analysis::OperatingPoint],
            ),
            &CancellationToken::new(),
        )
        .expect("sim nonzero");
    assert!(result_nonzero.succeeded());

    // 3. Error on missing target voltage source
    let spice_missing = "* Missing V source
H1 out 0 V_NONEXISTENT 500
.end
";
    let report_missing = import_spice(spice_missing);
    assert!(report_missing.has_errors());
    assert!(
        report_missing.diagnostics.iter().any(|d| {
            d.message.contains(
                "controlled source 'H1' references 'V_NONEXISTENT' which is not an independent voltage source",
            )
        })
    );
}

#[test]
fn schematic_export_svg_kicad_ltspice_for_dependent_sources() {
    let source = "
source V1 5V
vcvs E1 2.0
vccs G1 10mS
ccvs H1 100
cccs F1 3.0

connect E1.ctrl_p to V1.plus
connect G1.ctrl_p to V1.plus
connect H1.ctrl_p to V1.plus
connect F1.ctrl_p to V1.plus

connect E1.ctrl_n to V1.minus
connect G1.ctrl_n to V1.minus
connect H1.ctrl_n to V1.minus
connect F1.ctrl_n to V1.minus

net GND
connect V1.minus to GND

resistor R1 1k
resistor R2 1k
resistor R3 1k
resistor R4 1k

connect E1.p_out to R1.p1
connect E1.n_out, R1.p2 to GND
connect G1.p_out to R2.p1
connect G1.n_out, R2.p2 to GND
connect H1.p_out to R3.p1
connect H1.n_out, R3.p2 to GND
connect F1.p_out to R4.p1
connect F1.n_out, R4.p2 to GND
";

    let compiled = compile_source(source, CompileOptions::all_outputs());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    // SVG export
    let svg = compiled.schematic_svg.as_deref().expect("SVG output");
    // Verify diamond symbol path is present
    assert!(svg.contains("M 0 0 H"), "control leads present");
    assert!(svg.contains("stroke-width=\"2\""), "stroke styling");

    // KiCad export
    let kicad = compiled.kicad_sch.as_deref().expect("KiCad output");
    assert!(kicad.contains("kicad_sch"));

    // LTspice export
    let ltspice = export_report(&compiled, ExportFormat::Ltspice, ExportOptions::default())
        .expect("LTspice export");
    let ltspice_str = String::from_utf8(ltspice.bytes).expect("UTF-8 LTspice");
    assert!(ltspice_str.contains("SYMBOL e"), "E symbol in LTspice");
    assert!(ltspice_str.contains("SYMBOL g"), "G symbol in LTspice");
    assert!(
        ltspice_str.contains("SYMATTR Prefix X"),
        "Prefix X for CCVS/CCCS in LTspice"
    );
    assert!(
        ltspice_str.contains(".subckt KESSETSU_CCVS_H1"),
        "CCVS subckt definition in LTspice"
    );
    assert!(
        ltspice_str.contains(".subckt KESSETSU_CCCS_F1"),
        "CCCS subckt definition in LTspice"
    );
    assert!(
        ltspice_str.contains("V_SENSE ctrl_p ctrl_n 0"),
        "V_SENSE in subckt"
    );
    assert!(
        ltspice_str.contains("H1 p_out n_out V_SENSE 100"),
        "H1 in subckt"
    );
    assert!(
        ltspice_str.contains("F1 p_out n_out V_SENSE 3"),
        "F1 in subckt"
    );

    // BOM export excludes all 4 dependent sources
    let bom = export_report(&compiled, ExportFormat::BomCsv, ExportOptions::default())
        .expect("BOM export");
    let bom_str = String::from_utf8(bom.bytes).expect("UTF-8 BOM");
    assert!(!bom_str.contains("E1"), "E1 should be excluded from BOM");
    assert!(!bom_str.contains("G1"), "G1 should be excluded from BOM");
    assert!(!bom_str.contains("H1"), "H1 should be excluded from BOM");
    assert!(!bom_str.contains("F1"), "F1 should be excluded from BOM");
}
