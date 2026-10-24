mod common;

use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::exporter::{ExportFormat, ExportOptions, export_report};
use kessetsu_core::ir::{Analysis, ComponentKind, ComponentParams, Quantity, SIUnit};
use kessetsu_core::simulation::{
    CancellationToken, Dataset, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use kessetsu_core::spice_import::import_spice;

#[test]
fn default_and_custom_model_compilation_and_ir() {
    // 1. Default switch (KESSETSU_SW_V1)
    let source_default = "
source Vin 10V
source Vctrl 3V
switch S1
resistor Rload 1k

net GND
connect Vin.minus, Vctrl.minus, Rload.p2 to GND
connect Vin.plus to S1.p1
net out
connect S1.p2, Rload.p1 to out
connect S1.ctrl_p to Vctrl.plus
connect S1.ctrl_n to GND
";

    let compiled = compile_source(source_default, CompileOptions::all_outputs());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    let ir = compiled.ir.as_ref().expect("IR generated");
    let s1 = ir.components.iter().find(|c| c.id == "S1").unwrap();
    assert_eq!(s1.kind, ComponentKind::Switch);
    assert_eq!(s1.parameters, ComponentParams::SwitchParams);
    assert_eq!(
        s1.model.as_ref().map(|m| m.name.as_str()),
        Some("KESSETSU_SW_V1")
    );

    let spice = compiled.spice_netlist.as_deref().expect("SPICE netlist");
    assert!(
        spice.contains("S_S1 out GND ctrl_p GND KESSETSU_SW_V1")
            || spice.contains("S_S1 out GND Vctrl_plus GND KESSETSU_SW_V1")
            || spice.contains("KESSETSU_SW_V1"),
        "spice netlist: {spice}"
    );
    assert!(
        spice.contains(".model KESSETSU_SW_V1 SW"),
        "model directive in spice: {spice}"
    );

    // 2. Custom switch model
    let source_custom = "
model switch MySw version=1.0.0 license=MIT vt=2.0 vh=0.2 ron=0.5 roff=10Meg

source Vin 10V
source Vctrl 3V
switch S1 MySw
resistor Rload 1k

net GND
connect Vin.minus, Vctrl.minus, Rload.p2 to GND
connect Vin.plus to S1.p1
net out
connect S1.p2, Rload.p1 to out
connect S1.ctrl_p to Vctrl.plus
connect S1.ctrl_n to GND
";

    let compiled_custom = compile_source(source_custom, CompileOptions::all_outputs());
    assert!(
        !compiled_custom.has_errors(),
        "{:?}",
        compiled_custom.diagnostics
    );

    let ir_custom = compiled_custom.ir.as_ref().expect("IR generated");
    let s1_custom = ir_custom.components.iter().find(|c| c.id == "S1").unwrap();
    assert_eq!(s1_custom.kind, ComponentKind::Switch);
    assert_eq!(
        s1_custom.model.as_ref().map(|m| m.name.as_str()),
        Some("MySw")
    );

    let spice_custom = compiled_custom
        .spice_netlist
        .as_deref()
        .expect("SPICE netlist");
    assert!(
        spice_custom.contains("MySw"),
        "custom model name in spice: {spice_custom}"
    );
    assert!(
        spice_custom.contains(".model MySw SW"),
        "custom model card in spice: {spice_custom}"
    );
    assert!(
        spice_custom.contains("ron=0.5"),
        "ron parameter in spice: {spice_custom}"
    );
}

#[test]
fn validation_errors_for_switch_and_models() {
    // 1. Negative ron
    let bad_ron = "
model switch BadRon version=1 license=MIT vt=1 ron=-1 roff=1Meg
switch S1 BadRon
";
    let report = compile_source(bad_ron, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C010");
    assert!(
        report.diagnostics[0]
            .message
            .contains("switch model ron and roff must be positive"),
        "{:?}",
        report.diagnostics
    );

    // 2. Zero roff
    let bad_roff = "
model switch BadRoff version=1 license=MIT vt=1 ron=1 roff=0
switch S1 BadRoff
";
    let report = compile_source(bad_roff, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C010");
    assert!(
        report.diagnostics[0]
            .message
            .contains("switch model ron and roff must be positive"),
        "{:?}",
        report.diagnostics
    );

    // 3. Polarity on switch model
    let bad_pol = "
model switch BadPol npn version=1 license=MIT vt=1 ron=1
";
    let report = compile_source(bad_pol, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C010");
    assert!(
        report.diagnostics[0]
            .message
            .contains("switch models do not accept polarity"),
        "{:?}",
        report.diagnostics
    );

    // 4. Model kind mismatch: diode model on switch
    let mismatch_switch = "
model diode DMOD version=1 license=MIT Is=1e-12
switch S1 DMOD
";
    let report = compile_source(mismatch_switch, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C004");
    assert!(
        report.diagnostics[0]
            .message
            .contains("model 'DMOD' is not a switch model"),
        "{:?}",
        report.diagnostics
    );

    // 5. Model kind mismatch: switch model on diode
    let mismatch_diode = "
model switch SW1 version=1 license=MIT vt=1 ron=1
diode D1 SW1
";
    let report = compile_source(mismatch_diode, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C004");
    assert!(
        report.diagnostics[0]
            .message
            .contains("model 'SW1' is not a diode model"),
        "{:?}",
        report.diagnostics
    );

    // 6. Unknown switch model
    let unknown_model = "
switch S1 NonExistentModel
";
    let report = compile_source(unknown_model, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C003");
    assert!(
        report.diagnostics[0]
            .message
            .contains("unsupported switch model 'NonExistentModel'"),
        "{:?}",
        report.diagnostics
    );

    // 7. Expressions rejected on switch
    let expr_switch = "
switch S1 {1+2}
";
    let report = compile_source(expr_switch, CompileOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "KES-C022");
}

#[test]
fn ngspice_op_simulation_of_voltage_controlled_switch() {
    let source = "
source Vin 10V
source Vctrl_off 0V
source Vctrl_on 2V

switch Soff
switch Son

resistor Rload_off 1k
resistor Rload_on 1k

net GND
connect Vin.minus, Vctrl_off.minus, Vctrl_on.minus, Rload_off.p2, Rload_on.p2 to GND
connect Soff.p1, Son.p1 to Vin.plus

net out_off
connect Soff.p2, Rload_off.p1 to out_off
connect Soff.ctrl_p to Vctrl_off.plus
connect Soff.ctrl_n to GND

net out_on
connect Son.p2, Rload_on.p1 to out_on
connect Son.ctrl_p to Vctrl_on.plus
connect Son.ctrl_n to GND

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
        // Soff is OFF (Vctrl=0V < Vt=0.5V, Roff=1Meg). V(out_off) ~ 10V * 1k / 1Meg ~ 0.01V
        let v_off = values
            .get("out_off")
            .or_else(|| values.get("v(out_off)"))
            .copied()
            .expect("out_off");
        assert!(v_off < 0.05, "expected Soff to be OFF, got {v_off} V");

        // Son is ON (Vctrl=2V > Vt=0.5V, Ron=1 Ohm). V(out_on) ~ 10V * 1k / (1k + 1) ~ 9.99V
        let v_on = values
            .get("out_on")
            .or_else(|| values.get("v(out_on)"))
            .copied()
            .expect("out_on");
        assert!(v_on > 9.9, "expected Son to be ON, got {v_on} V");
    } else {
        panic!("Expected OperatingPoint dataset");
    }
}

#[test]
fn ngspice_tran_simulation_of_switch_with_hysteresis() {
    let source = "
model switch SwHyst version=1.0.0 license=MIT vt=2.5 vh=0.5 ron=0.5 roff=10Meg

source Vin 10V
source Vctrl pwl(0s, 0V, 5ms, 5V, 10ms, 0V)

switch S1 SwHyst
resistor Rload 1k

net GND
connect Vin.minus, Vctrl.minus, Rload.p2 to GND
connect Vin.plus to S1.p1

net out
connect S1.p2, Rload.p1 to out
connect S1.ctrl_p to Vctrl.plus
connect S1.ctrl_n to GND

simulate tran 50us 10ms
";

    let compiled = compile_source(source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    let spice = compiled.spice_netlist.expect("netlist");
    let runner = NgspiceRunner::discover();
    let request = SimulationRequest::new(
        &spice,
        vec![Analysis::Transient {
            step: Quantity {
                value: 50e-6,
                unit: SIUnit::Second,
            },
            stop: Quantity {
                value: 10e-3,
                unit: SIUnit::Second,
            },
            use_initial_conditions: false,
        }],
    );
    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("simulation should run");
    assert!(result.succeeded(), "Simulation failed: {:?}", result.errors);

    let first_dataset = result.datasets.first().expect("dataset");
    if let Dataset::Transient(dataset) = &first_dataset.data {
        let v_out_series = dataset
            .signals
            .get("out")
            .or_else(|| dataset.signals.get("v(out)"))
            .expect("v(out) series");

        let sample_at = |target_time: f64| -> f64 {
            let mut closest_idx = 0;
            let mut closest_diff = f64::MAX;
            for (idx, &t) in dataset.axis.values.iter().enumerate() {
                let diff = (t - target_time).abs();
                if diff < closest_diff {
                    closest_diff = diff;
                    closest_idx = idx;
                }
            }
            v_out_series[closest_idx]
        };

        // Rising phase: Vt + Vh = 3.0V (occurs at t = 3.0ms)
        // At t = 1.0ms (Vctrl = 1.0V < 3.0V): switch is OFF -> V(out) ~ 0V
        let v_rise_early = sample_at(1.0e-3);
        assert!(
            v_rise_early < 0.1,
            "expected switch OFF at t=1ms, got {v_rise_early} V"
        );

        // At t = 4.0ms (Vctrl = 4.0V > 3.0V): switch is ON -> V(out) ~ 10V
        let v_rise_on = sample_at(4.0e-3);
        assert!(
            v_rise_on > 9.5,
            "expected switch ON at t=4ms, got {v_rise_on} V"
        );

        // Falling phase: Vt - Vh = 2.0V (occurs at t = 8.0ms)
        // At t = 7.0ms (Vctrl = 3.0V > 2.0V): due to hysteresis, switch is STILL ON -> V(out) ~ 10V!
        let v_fall_hyst = sample_at(7.0e-3);
        assert!(
            v_fall_hyst > 9.5,
            "expected switch STILL ON at t=7ms due to hysteresis, got {v_fall_hyst} V"
        );

        // At t = 9.5ms (Vctrl = 0.5V < 2.0V): switch is OFF -> V(out) ~ 0V
        let v_fall_off = sample_at(9.5e-3);
        assert!(
            v_fall_off < 0.1,
            "expected switch OFF at t=9.5ms, got {v_fall_off} V"
        );
    } else {
        panic!("Expected Transient dataset");
    }
}

#[test]
fn spice_import_switch_preserves_ron_and_simulates() {
    let spice_source = "* Voltage-controlled switch import test
V1 in 0 10
Vctrl ctrl 0 5
S1 in out ctrl 0 SWMOD
R1 out 0 1k
.model SWMOD SW(Vt=1.5 Vh=0.1 Ron=2 Roff=1Meg)
.op
.end
";

    let report = import_spice(spice_source);
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| d.severity == kessetsu_core::spice_import::ImportSeverity::Error),
        "import diagnostics: {:?}",
        report.diagnostics
    );

    let kess_src = report.kess_source.expect("imported kess source");
    assert!(
        kess_src.contains("switch S1"),
        "switch statement: {kess_src}"
    );
    assert!(
        kess_src.contains("model switch SWMOD"),
        "model switch card: {kess_src}"
    );
    // Crucial check: ron=2 must NOT have been dropped as a descriptive field!
    assert!(
        kess_src.contains("ron=2"),
        "ron parameter must be preserved: {kess_src}"
    );
    assert!(
        kess_src.contains("vt=1.5"),
        "vt parameter must be preserved: {kess_src}"
    );

    // Recompile the imported source
    let compiled = compile_source(&kess_src, CompileOptions::default());
    assert!(
        !compiled.has_errors(),
        "recompile errors: {:?}",
        compiled.diagnostics
    );

    let netlist = compiled.spice_netlist.expect("recompiled netlist");
    let runner = NgspiceRunner::discover();
    let request = SimulationRequest::new(&netlist, vec![Analysis::OperatingPoint]);
    let result = runner
        .run(&request, &CancellationToken::new())
        .expect("simulation should run");
    assert!(result.succeeded(), "Simulation failed: {:?}", result.errors);

    let first_dataset = result.datasets.first().expect("dataset");
    if let Dataset::OperatingPoint { values } = &first_dataset.data {
        // Vctrl = 5V > Vt = 1.5V -> switch is ON (Ron = 2 Ohm).
        // V(out) = 10V * 1000 / (1000 + 2) ~ 9.98V
        let v_out = values
            .get("out")
            .or_else(|| values.get("v(out)"))
            .copied()
            .expect("out");
        assert!(
            v_out > 9.9,
            "expected switch to be ON in imported circuit, got {v_out} V"
        );
    } else {
        panic!("Expected OperatingPoint dataset");
    }
}

#[test]
fn schematic_export_svg_kicad_ltspice_for_switch() {
    let source = "
source Vin 10V
source Vctrl 2V
switch S1
resistor R1 1k
net out
connect Vin.plus to S1.p1
connect S1.p2, R1.p1 to out
connect S1.ctrl_p to Vctrl.plus
connect S1.ctrl_n, Vctrl.minus, Vin.minus, R1.p2 to GND
net GND
";

    let compiled = compile_source(source, CompileOptions::all_outputs());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

    // 1. SVG export
    let svg = compiled.schematic_svg.as_deref().expect("SVG output");
    assert!(svg.contains("<svg"), "SVG header");
    assert!(
        svg.contains("stroke-width=\"2\""),
        "stroke styling in SVG: {svg}"
    );
    assert!(svg.contains("M 0 0 H"), "switch leads in SVG: {svg}");

    // 2. KiCad export
    let kicad = compiled.kicad_sch.as_deref().expect("KiCad output");
    assert!(kicad.contains("kicad_sch"), "KiCad schematic header");

    // 3. LTspice export
    let ltspice = export_report(&compiled, ExportFormat::Ltspice, ExportOptions::default())
        .expect("LTspice export");
    let ltspice_str = String::from_utf8(ltspice.bytes).expect("UTF-8 LTspice");
    assert!(
        ltspice_str.contains("SYMBOL sw"),
        "LTspice switch symbol: {ltspice_str}"
    );
    assert!(
        ltspice_str.contains("SYMATTR InstName S_S1")
            || ltspice_str.contains("SYMATTR InstName S1"),
        "LTspice switch instance name: {ltspice_str}"
    );
}
