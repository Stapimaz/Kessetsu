mod common;
use common::read_fixture;
use kessetsu_core::sim_result::{AssertionReport, AssertionStatus, evaluate_assertions};
use kessetsu_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use kessetsu_core::{CompileOptions, compile_source};

fn evaluate(source: &str) -> (kessetsu_core::ir::CircuitIR, AssertionReport) {
    let compiled = compile_source(source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
    let ir = compiled.ir.unwrap();
    let request = SimulationRequest::new(compiled.spice_netlist.unwrap(), ir.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .unwrap();
    assert!(result.succeeded(), "{:?}", result.errors);
    let assertions = evaluate_assertions(&ir, &result);
    (ir, assertions)
}

#[test]
fn resolved_threshold_frequency_and_window_match_literal_measurements() {
    let source = read_fixture("parameters/assertions.kess");
    let literal = source
        .replace("{frequency})", "1kHz)")
        .replace("{settling},{duration}", "2ms,10ms")
        .replace("< {maximum}", "< 0.55V");
    let (ir, result) = evaluate(&source);
    let (literal_ir, reference) = evaluate(&literal);
    assert!(result.all_passed(), "{result:?}");
    assert!(reference.all_passed());
    for (actual, expected) in result.assertions.iter().zip(reference.assertions) {
        assert_eq!(actual.actual, expected.actual);
        assert_eq!(actual.threshold, expected.threshold);
    }
    assert_eq!(ir.components, literal_ir.components);
    assert_eq!(
        ir.assertions[3].threshold,
        literal_ir.assertions[3].threshold
    );
    assert_eq!(ir.parameter_manifest.assertion_bindings.len(), 7);
    let compiled = compile_source(&source, CompileOptions::all_outputs());
    let reference = compile_source(&literal, CompileOptions::all_outputs());
    assert_eq!(compiled.spice_netlist, reference.spice_netlist);
    assert_eq!(compiled.schematic, reference.schematic);
    assert_eq!(compiled.kicad_sch, reference.kicad_sch);
    let (_, changed) =
        evaluate(&source.replace("param amplitude: V = 1V", "param amplitude: V = 2V"));
    assert_eq!(changed.assertions[3].status, AssertionStatus::Fail);
}

#[test]
fn power_metrics_use_typed_time_rails_and_frequency_without_reparsing_values() {
    let source = read_fixture("benchmarks/power_amplifier.kess");
    let parametric = format!("param start: s = 2ms\nparam stop: s = 10ms\nparam frequency: Hz = 1kHz\nparam lower: V = -10V\nparam upper: V = 10V\nparam limit: percent = 3%\n{source}")
        .replace(",2ms,10ms", ",{start},{stop}")
        .replace("thd(V(OUT),1kHz,", "thd(V(OUT),{frequency},")
        .replace("clipping(V(OUT),-10V,10V)", "clipping(V(OUT),{lower},{upper})")
        .replace("hann) < 3%", "hann) < {limit}");
    let (_, expected) = evaluate(&source);
    let (_, result) = evaluate(&parametric);
    assert!(result.all_passed(), "{result:?}");
    for (actual, expected) in result.assertions.iter().zip(expected.assertions) {
        assert_eq!(actual.actual, expected.actual);
    }
    let source = read_fixture("benchmarks/ac_coupled_amplifier.kess");
    let parametric =
        format!("param reference: Hz = 1kHz\n{source}").replace(",1kHz)", ",{reference})");
    assert!(evaluate(&parametric).1.all_passed());
}

#[test]
fn expression_units_and_non_numeric_roles_fail_before_outputs() {
    let prefix = "param time: s = 1ms\nparam voltage: V = 1V\nparam frequency: Hz = 1kHz\n";
    for assertion in [
        "assert rms(V(OUT),{voltage},{time}) < 1V",
        "assert gain_at(V(OUT),V(IN),{time}) < 1",
        "assert gain_at(V(OUT),V(IN),{0Hz}) < 1",
        "assert rms(V(OUT)) < {time}",
        "assert rms(V(OUT)) < {voltage / 0}",
        "assert rms(V(OUT)) < {missing}",
        "assert rms({voltage}) < 1V",
        "assert output_power(V(OUT),{voltage}) < 1W",
        "assert thd(V(OUT),1kHz,0s,1ms,{frequency}) < 1%",
        "assert efficiency(V(OUT),RL,V(VCC),I(VP),{time},I(VN)) < 80%",
    ] {
        let compiled = compile_source(
            &format!("{prefix}{assertion}\n"),
            CompileOptions::all_outputs(),
        );
        assert!(compiled.has_errors(), "accepted {assertion}");
        assert_eq!(compiled.diagnostics[0].code, "KES-C006");
        assert!(compiled.spice_netlist.is_none());
    }
    let compiled = compile_source(
        "module M() {\nassert rms(V(OUT)) < {1V}\n}\nuse M X\n",
        CompileOptions::default(),
    );
    assert!(compiled.has_errors());
    assert!(compiled.diagnostics[0].message.contains("circuit root"));
}

#[test]
fn requirements_cannot_capture_circuit_parameters_or_accept_expression_syntax() {
    for source in [
        "assert rms(V(OUT)) < {voltage}\n",
        "assert rms(V(OUT),{time},10ms) < 1V\n",
        "assert rms(V(OUT)) < {1V}\n",
    ] {
        let error =
            kessetsu_core::requirements::compile_requirements(source.as_bytes()).unwrap_err();
        assert_eq!(error.code, "KES-R001");
        assert!(error.message.contains("literal numeric"));
    }
    let requirements =
        kessetsu_core::requirements::compile_requirements(b"assert rms(V(OUT),2ms,10ms) < 0.55V\n")
            .unwrap();
    assert!(requirements.assertions[0].numeric_arguments.is_empty());
}
