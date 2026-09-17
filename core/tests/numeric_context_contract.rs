use kessetsu_core::ir::{Analysis, ComponentParams, SIUnit, SourceValue, Waveform};
use kessetsu_core::{CompileOptions, compile_source};

const DEFINITIONS: &str = "param level: V = 2V\nparam current: A = 1mA\nparam frequency: Hz = 1kHz\nparam period: s = 1 / frequency\nparam edge: s = 10us\nparam points: ratio = 80\n";

fn circuit(declaration: &str, analyses: &str) -> String {
    format!(
        "{DEFINITIONS}net GND\nnet OUT\n{declaration}\nresistor RL 1k\nconnect VIN.plus, RL.p1 to OUT\nconnect VIN.minus, RL.p2 to GND\n{analyses}"
    )
}

fn equivalent(parametric: &str, literal: &str) {
    let mut actual = compile_source(parametric, CompileOptions::all_outputs());
    let expected = compile_source(literal, CompileOptions::all_outputs());
    assert!(!actual.has_errors(), "{:?}", actual.diagnostics);
    assert!(!expected.has_errors(), "{:?}", expected.diagnostics);
    actual.ir.as_mut().unwrap().parameter_manifest = Default::default();
    assert_eq!(actual.ir, expected.ir);
    assert_eq!(actual.spice_netlist, expected.spice_netlist);
    assert_eq!(actual.schematic, expected.schematic);
    assert_eq!(actual.schematic_svg, expected.schematic_svg);
    assert_eq!(actual.kicad_sch, expected.kicad_sch);
}

#[test]
fn all_supported_waveform_slots_match_literal_ir_and_backends() {
    for (expression, literal) in [
        ("ac({level})", "ac(2V)"),
        ("sine({level / 2},{level},{frequency})", "sine(1V,2V,1kHz)"),
        (
            "sine_ac(0V,{level},{frequency},{level / 2})",
            "sine_ac(0V,2V,1kHz,1V)",
        ),
        (
            "pulse(0V,{level},0s,{edge},{edge},{period},{period * 2})",
            "pulse(0V,2V,0s,10us,10us,1ms,2ms)",
        ),
        (
            "pwl(0s,0V,{period},{level},{period * 2},{level / 2})",
            "pwl(0s,0V,1ms,2V,2ms,1V)",
        ),
    ] {
        let source = circuit(&format!("source VIN {expression}"), "simulate op\n");
        let literal = source.replace(expression, literal).replace(DEFINITIONS, "");
        equivalent(&source, &literal);
        let report = compile_source(&source, CompileOptions::default());
        let bindings = report.ir.unwrap().parameter_manifest.bindings;
        assert!(!bindings.is_empty());
        assert!(bindings.iter().all(|binding| binding.component == "VIN" && binding.field.starts_with("waveform.")));
    }
    let source = circuit(
        "current_source VIN sine({current / 2},{current},{frequency})",
        "simulate op\n",
    );
    equivalent(
        &source,
        &source
            .replace(
                "sine({current / 2},{current},{frequency})",
                "sine(500uA,1mA,1kHz)",
            )
            .replace(DEFINITIONS, ""),
    );
}

#[test]
fn analysis_numeric_fields_match_literals_and_record_typed_provenance() {
    let analyses = "simulate tran {edge} {period * 2}\nsimulate ac dec {points} {frequency / 100} {frequency * 100}\nsimulate dc VIN 0V {level} {level / 4}\n";
    let source = circuit("source VIN 1V", analyses);
    let literal = source
        .replace(
            analyses,
            "simulate tran 10us 2ms\nsimulate ac dec 80 10Hz 100kHz\nsimulate dc VIN 0V 2V 500mV\n",
        )
        .replace(DEFINITIONS, "");
    equivalent(&source, &literal);
    let report = compile_source(&source, CompileOptions::default());
    let manifest = report.ir.unwrap().parameter_manifest;
    assert_eq!(manifest.analysis_bindings.len(), 7);
    assert_eq!(manifest.analysis_bindings[0].analysis_index, 0);
    assert_eq!(manifest.analysis_bindings[0].field, "step");
    assert_eq!(manifest.analysis_bindings[2].field, "points");
    assert_eq!(manifest.analysis_bindings[2].resolved.unit, SIUnit::Ratio);
    let source = circuit(
        "current_source VIN {current}",
        "simulate dc VIN 0A {current} {current / 4}\n",
    );
    equivalent(
        &source,
        &source
            .replace("{current / 4}", "250uA")
            .replace("{current}", "1mA")
            .replace(DEFINITIONS, ""),
    );
}

#[test]
fn module_source_waveforms_bind_lexically_with_independent_overrides() {
    let source = "param frequency: Hz = 99Hz\nmodule Tone(out,gnd) {\nparam frequency: Hz = 1kHz\nparam amplitude: V = 1V\nsource VIN sine(0V,{amplitude},{frequency})\nconnect VIN.plus to out\nconnect VIN.minus to gnd\n}\nnet GND\nnet A\nnet B\nuse Tone LOW\nuse Tone HIGH(frequency=2kHz,amplitude=2V)\nresistor RA 1k\nresistor RB 1k\nconnect LOW.out, RA.p1 to A\nconnect HIGH.out, RB.p1 to B\nconnect LOW.gnd, HIGH.gnd, RA.p2 to GND\nconnect RB.p2 to GND\nsimulate tran 10us 2ms\n";
    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let ir = report.ir.unwrap();
    for (name, frequency_value, amplitude_value) in
        [("LOW_VIN", 1000.0, 1.0), ("HIGH_VIN", 2000.0, 2.0)]
    {
        let component = ir
            .components
            .iter()
            .find(|component| component.id == name)
            .unwrap();
        let ComponentParams::VoltageSource {
            value:
                SourceValue::Waveform(Waveform::Sine {
                    frequency,
                    amplitude,
                    ..
                }),
        } = &component.parameters
        else {
            panic!("wrong waveform")
        };
        assert_eq!(frequency.value, frequency_value);
        assert_eq!(amplitude.value, amplitude_value);
    }
    assert!(
        ir.parameter_manifest
            .bindings
            .iter()
            .all(|binding| binding.dependencies.iter().all(|name| name.contains('.')))
    );
}

#[test]
fn incorrect_numeric_contexts_fail_closed_before_backend_output() {
    for (declaration, analyses, message) in [
        (
            "source VIN sine(0V,{frequency},1kHz)",
            "simulate op\n",
            "waveform.amplitude",
        ),
        (
            "current_source VIN ac({level})",
            "simulate op\n",
            "Expected Ampere",
        ),
        (
            "source VIN sine(0V,1V,{missing})",
            "simulate op\n",
            "Unknown parameter",
        ),
        (
            "source VIN ac({level / 0})",
            "simulate op\n",
            "Division by zero",
        ),
        ("source VIN sine({level},1V)", "simulate op\n", "exactly 3"),
        (
            "source VIN pwl(0s,0V,{period},1V,{period},2V)",
            "simulate op\n",
            "strictly increasing",
        ),
        (
            "source VIN 1V",
            "simulate tran {level} 1ms\n",
            "transient step",
        ),
        (
            "source VIN 1V",
            "simulate tran {period * 2} {period}\n",
            "step <= stop",
        ),
        (
            "source VIN 1V",
            "simulate ac dec {points / 3} 1Hz 10kHz\n",
            "positive integer",
        ),
        (
            "source VIN 1V",
            "simulate ac dec {0} 1Hz 10kHz\n",
            "positive integer",
        ),
        (
            "source VIN 1V",
            "simulate ac dec {4294967296} 1Hz 10kHz\n",
            "supported range",
        ),
        (
            "source VIN 1V",
            "simulate ac dec {frequency} 1Hz 10kHz\n",
            "invalid AC points",
        ),
        (
            "source VIN 1V",
            "simulate ac {points} 80 1Hz 10kHz\n",
            "AC scale",
        ),
        (
            "source VIN 1V",
            "simulate ac dec 80 {frequency} {frequency / 2}\n",
            "start < stop",
        ),
        (
            "source VIN 1V",
            "simulate dc {level} 0V 1V 1mV\n",
            "source name",
        ),
        (
            "source VIN 1V",
            "simulate dc VIN 0V {current} 1mV\n",
            "DC sweep stop",
        ),
        (
            "source VIN 1V",
            "simulate dc VIN 0V {level} {0V}\n",
            "non-zero",
        ),
        (
            "source VIN 1V",
            "simulate ac dec 80 {frequency} {frequency * 2}\nsimulate ac dec 20 1Hz 100kHz\n",
            "duplicate",
        ),
    ] {
        let report = compile_source(
            &circuit(declaration, analyses),
            CompileOptions::all_outputs(),
        );
        assert!(report.has_errors(), "accepted {declaration}; {analyses}");
        assert!(
            report.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .to_lowercase()
                .contains(&message.to_lowercase())),
            "expected {message}: {:?}",
            report.diagnostics
        );
        assert!(report.spice_netlist.is_none());
        assert!(report.schematic.is_none());
    }
}

#[test]
fn literal_analysis_and_quoted_waveform_syntax_remain_compatible() {
    let source = circuit(
        "source VIN \"SINE(0 1V 1kHz)\"",
        "simulate ac dec 80 10Hz 100kHz\n",
    );
    let report = compile_source(&source.replace(DEFINITIONS, ""), CompileOptions::default());
    assert!(!report.has_errors());
    assert!(report.ir.unwrap().parameter_manifest.is_empty());
    let source = circuit("source VIN \"SINE(0 {level} 1kHz)\"", "simulate op\n");
    assert!(compile_source(&source, CompileOptions::default()).has_errors());
    // Retain literal AC point spelling; expressions opt into engineering arithmetic.
    assert!(
        compile_source(
            &circuit("source VIN 1V", "simulate ac dec 1k 1Hz 100kHz\n"),
            CompileOptions::default()
        )
        .has_errors()
    );
    let report = compile_source(
        &circuit("source VIN 1V", "simulate ac dec {1k} 1Hz 100kHz\n"),
        CompileOptions::default(),
    );
    assert!(matches!(
        report.ir.unwrap().analyses[0],
        Analysis::Ac { points: 1000, .. }
    ));
}
