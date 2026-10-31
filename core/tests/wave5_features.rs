use kessetsu_core::{
    compile_source,
    ir::{parse_waveform, SIUnit, SourceValue, Waveform},
    ltspice::generate_ltspice_asc,
    schematic_svg::render_svg,
    CompileOptions,
};

#[test]
fn test_exp_waveform_parsing_and_validation() {
    // Valid voltage source with exp
    let src = r#"
net in
net gnd
source V1 exp(0V, 5V, 1us, 5us, 10us, 20us)
resistor R1 1k
connect V1.plus to in
connect V1.minus to gnd
connect R1.p1 to in
connect R1.p2 to gnd
simulate tran 100ns 50us
"#;
    let compiled = compile_source(src, CompileOptions::all_outputs());
    assert!(!compiled.has_errors(), "Expected valid exp waveform to compile: {:?}", compiled.diagnostics);
    let ir = compiled.ir.expect("IR should exist");
    let v1 = ir.components.iter().find(|c| c.id == "V1").expect("V1 should exist");
    match &v1.parameters {
        kessetsu_core::ir::ComponentParams::VoltageSource { value } => match value {
            SourceValue::Waveform(Waveform::Exp { v1, v2, td1, tau1, td2, tau2 }) => {
                assert_eq!(v1.value, 0.0);
                assert_eq!(v2.value, 5.0);
                assert!((td1.value - 1e-6).abs() < 1e-12);
                assert!((tau1.value - 5e-6).abs() < 1e-12);
                assert!((td2.value - 1e-5).abs() < 1e-12);
                assert!((tau2.value - 2e-5).abs() < 1e-12);
            }
            _ => panic!("Expected Waveform::Exp"),
        },
        _ => panic!("Expected VoltageSource"),
    }

    // SPICE netlist output
    let spice = compiled.spice_netlist.expect("SPICE netlist should be produced");
    assert!(
        spice.contains("V_V1 in 0 EXP(0 5 1e-6 5e-6 1e-5 2e-5)"),
        "SPICE netlist did not contain expected EXP card: {spice}"
    );

    // Schematic label
    let schematic = compiled.schematic.expect("Schematic should exist");
    let comp_v1 = schematic.components.iter().find(|c| c.id == "V1").expect("V1 in schematic");
    assert_eq!(comp_v1.value.as_deref(), Some("exp(0 V, 5 V, 1 µs, 5 µs, 10 µs, 20 µs)"));

    // Invalid: negative delay
    let invalid_delay = r#"
source V1 exp(0V, 5V, -1us, 5us, 10us, 20us)
"#;
    let res = compile_source(invalid_delay, CompileOptions::default());
    assert!(res.has_errors(), "Negative delay must be rejected");

    // Invalid: non-positive tau
    let invalid_tau = r#"
source V1 exp(0V, 5V, 1us, 0s, 10us, 20us)
"#;
    let res = compile_source(invalid_tau, CompileOptions::default());
    assert!(res.has_errors(), "Zero tau must be rejected");

    // Invalid: wrong argument count (e.g. 5 args)
    let invalid_count = r#"
source V1 exp(0V, 5V, 1us, 5us, 10us)
"#;
    let res = compile_source(invalid_count, CompileOptions::default());
    assert!(res.has_errors(), "5 args to exp must be rejected");
}

#[test]
fn test_exp_waveform_evaluation() {
    let wf = parse_waveform("exp(0, 5, 1u, 5u, 10u, 20u)", SIUnit::Volt)
        .expect("should parse")
        .expect("should be Some");

    let val = SourceValue::Waveform(wf);
    let eval = |t: f64| -> f64 {
        match &val {
            SourceValue::Waveform(Waveform::Exp { v1, v2, td1, tau1, td2, tau2 }) => {
                if t <= td1.value {
                    v1.value
                } else if t <= td2.value {
                    v1.value + (v2.value - v1.value) * (1.0 - (-(t - td1.value) / tau1.value).exp())
                } else {
                    v1.value
                        + (v2.value - v1.value) * (1.0 - (-(t - td1.value) / tau1.value).exp())
                        + (v1.value - v2.value) * (1.0 - (-(t - td2.value) / tau2.value).exp())
                }
            }
            _ => unreachable!(),
        }
    };

    // t <= td1 (1us): value is v1 (0.0)
    assert_eq!(eval(0.5e-6), 0.0);
    assert_eq!(eval(1.0e-6), 0.0);

    // td1 < t <= td2: rising towards v2
    let mid_rise = eval(6.0e-6); // t = td1 + tau1 -> 5 * (1 - 1/e) ~= 3.1606
    assert!((mid_rise - 5.0 * (1.0 - (-1.0_f64).exp())).abs() < 1e-6);

    // t > td2: decaying back towards v1
    let t_late = 30.0e-6;
    let val_late = eval(t_late);
    assert!(val_late > 0.0 && val_late < 5.0);
}

#[test]
fn test_couple_statement_and_validations() {
    // Valid 2-winding transformer
    let valid_2w = r#"
net in
net out
net gnd
source V1 1V
inductor L1 1m
inductor L2 4m
couple K1 L1 L2 0.99
resistor RL 1k
connect V1.plus to in
connect V1.minus to gnd
connect L1.p1 to in
connect L1.p2 to gnd
connect L2.p1 to out
connect L2.p2 to gnd
connect RL.p1 to out
connect RL.p2 to gnd
simulate op
"#;
    let compiled = compile_source(valid_2w, CompileOptions::all_outputs());
    assert!(!compiled.has_errors(), "Valid 2-winding couple should compile: {:?}", compiled.diagnostics);
    let spice = compiled.spice_netlist.expect("SPICE netlist should be generated");
    assert!(spice.contains("K_K1 L_L1 L_L2 0.99"), "SPICE should contain K_K1 card: {spice}");

    // LTspice export includes directive
    let ltspice = generate_ltspice_asc(compiled.schematic.as_ref().unwrap(), compiled.ir.as_ref().unwrap())
        .expect("LTspice export should succeed");
    assert!(ltspice.contains("!KK1 L1 L2 0.99"), "LTspice ASC should contain !KK1 directive: {ltspice}");

    // Valid 3-winding transformer (multi-winding expansion)
    let valid_3w = r#"
net in
net out1
net out2
net gnd
source V1 1V
inductor L1 1m
inductor L2 2m
inductor L3 4m
couple K1 L1 L2 L3 0.98
resistor R1 1k
resistor R2 1k
connect V1.plus to in
connect V1.minus to gnd
connect L1.p1 to in
connect L1.p2 to gnd
connect L2.p1 to out1
connect L2.p2 to gnd
connect L3.p1 to out2
connect L3.p2 to gnd
connect R1.p1 to out1
connect R1.p2 to gnd
connect R2.p1 to out2
connect R2.p2 to gnd
simulate op
"#;
    let compiled3 = compile_source(valid_3w, CompileOptions::default());
    assert!(!compiled3.has_errors(), "Valid 3-winding couple should compile: {:?}", compiled3.diagnostics);
    let spice3 = compiled3.spice_netlist.expect("SPICE netlist should be generated");
    assert!(spice3.contains("K_K1_1_2 L_L1 L_L2 0.98"), "Should contain K_K1_1_2: {spice3}");
    assert!(spice3.contains("K_K1_1_3 L_L1 L_L3 0.98"), "Should contain K_K1_1_3: {spice3}");
    assert!(spice3.contains("K_K1_2_3 L_L2 L_L3 0.98"), "Should contain K_K1_2_3: {spice3}");

    // Invalid: coefficient > 1.0
    let invalid_coeff_high = r#"
inductor L1 1m
inductor L2 4m
couple K1 L1 L2 1.05
"#;
    let res = compile_source(invalid_coeff_high, CompileOptions::default());
    assert!(res.has_errors(), "Coefficient > 1.0 must be rejected");

    // Invalid: coefficient <= 0.0
    let invalid_coeff_zero = r#"
inductor L1 1m
inductor L2 4m
couple K1 L1 L2 0.0
"#;
    let res = compile_source(invalid_coeff_zero, CompileOptions::default());
    assert!(res.has_errors(), "Coefficient 0.0 must be rejected");

    // Invalid: unknown inductor
    let invalid_unknown = r#"
inductor L1 1m
couple K1 L1 L99 0.95
"#;
    let res = compile_source(invalid_unknown, CompileOptions::default());
    assert!(res.has_errors(), "Unknown inductor must be rejected");

    // Invalid: non-inductor component
    let invalid_non_ind = r#"
inductor L1 1m
resistor R1 1k
couple K1 L1 R1 0.95
"#;
    let res = compile_source(invalid_non_ind, CompileOptions::default());
    assert!(res.has_errors(), "Non-inductor component must be rejected");

    // Invalid: duplicate inductor in same statement
    let invalid_dup = r#"
inductor L1 1m
couple K1 L1 L1 0.95
"#;
    let res = compile_source(invalid_dup, CompileOptions::default());
    assert!(res.has_errors(), "Duplicate inductor in same statement must be rejected");
}

#[test]
fn test_mosfet_4terminal_and_3terminal_backward_compatibility() {
    // 3-terminal MOSFET (unconnected bulk pin b): backward compatibility proof
    let mosfet_3pin = r#"
net vdd
net vg
net s
net gnd
source VDD 5V
source VG 3V
mosfet M1 IRF540
resistor RS 100
connect VDD.plus to vdd
connect VDD.minus to gnd
connect VG.plus to vg
connect VG.minus to gnd
connect M1.d to vdd
connect M1.g to vg
connect M1.s to s
connect RS.p1 to s
connect RS.p2 to gnd
simulate op
"#;
    let compiled_3p = compile_source(mosfet_3pin, CompileOptions::all_outputs());
    assert!(!compiled_3p.has_errors(), "3-pin MOSFET must compile cleanly: {:?}", compiled_3p.diagnostics);
    
    // ERC: unconnected b must not produce floating pin error
    let erc_3p = compiled_3p.diagnostics.iter().filter(|d| d.code == "KES-E003").count();
    assert_eq!(erc_3p, 0, "Unconnected b pin must not produce floating pin diagnostic");

    // Schematic pins: only d, g, s anchors
    let sch_3p = compiled_3p.schematic.as_ref().unwrap();
    let m1_3p = sch_3p.components.iter().find(|c| c.id == "M1").unwrap();
    assert_eq!(m1_3p.pins.len(), 3, "3-pin MOSFET should have exactly 3 anchors");
    assert!(!m1_3p.pins.iter().any(|p| p.name == "b"));

    // SPICE output: bulk folded to source
    let spice_3p = compiled_3p.spice_netlist.as_ref().unwrap();
    assert!(
        spice_3p.contains("M_M1 vdd vg s s IRF540"),
        "3-pin MOSFET SPICE card should tie bulk to source: {spice_3p}"
    );

    // LTspice export: uses nmos (not nmos4)
    let ltspice_3p = generate_ltspice_asc(sch_3p, compiled_3p.ir.as_ref().unwrap()).unwrap();
    assert!(ltspice_3p.contains("SYMBOL nmos"), "3-pin MOSFET should export as standard nmos: {ltspice_3p}");

    // 4-terminal MOSFET (explicit bulk pin b connected to separate rail / ground)
    let mosfet_4pin = r#"
net vdd
net vg
net s
net gnd
source VDD 5V
source VG 3V
mosfet M1 IRF540
resistor RS 100
connect VDD.plus to vdd
connect VDD.minus to gnd
connect VG.plus to vg
connect VG.minus to gnd
connect M1.d to vdd
connect M1.g to vg
connect M1.s to s
connect M1.b to gnd
connect RS.p1 to s
connect RS.p2 to gnd
simulate op
"#;
    let compiled_4p = compile_source(mosfet_4pin, CompileOptions::all_outputs());
    assert!(!compiled_4p.has_errors(), "4-pin MOSFET must compile cleanly: {:?}", compiled_4p.diagnostics);

    // Schematic pins: all 4 anchors (d, g, s, b)
    let sch_4p = compiled_4p.schematic.as_ref().unwrap();
    let m1_4p = sch_4p.components.iter().find(|c| c.id == "M1").unwrap();
    assert_eq!(m1_4p.pins.len(), 4, "4-pin MOSFET should have exactly 4 anchors");
    assert!(m1_4p.pins.iter().any(|p| p.name == "b"), "4-pin MOSFET must have anchor for b");

    // SVG export includes bulk lead
    let svg = render_svg(sch_4p);
    assert!(!svg.is_empty(), "SVG export should succeed");

    // SPICE output: bulk connected to 0 (gnd)
    let spice_4p = compiled_4p.spice_netlist.as_ref().unwrap();
    assert!(
        spice_4p.contains("M_M1 vdd vg s 0 IRF540"),
        "4-pin MOSFET SPICE card should connect separate bulk net: {spice_4p}"
    );

    // LTspice export: uses nmos4
    let ltspice_4p = generate_ltspice_asc(sch_4p, compiled_4p.ir.as_ref().unwrap()).unwrap();
    assert!(ltspice_4p.contains("SYMBOL nmos4"), "4-pin MOSFET should export as nmos4: {ltspice_4p}");
}
