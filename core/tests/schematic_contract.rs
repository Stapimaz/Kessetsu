use netlang_core::compiler::{CompileOptions, compile_source};
use netlang_core::component::component_definition;
use netlang_core::schematic::{SCHEMATIC_SCHEMA_VERSION, generate_schematic};
use netlang_core::{parse_program, schematic_svg};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CORPUS: &[(&str, &str)] = &[
    ("minimal", include_str!("fixtures/valid/minimal.nl")),
    (
        "rc_filter",
        include_str!("fixtures/benchmarks/rc_filter.nl"),
    ),
    ("wheatstone", include_str!("../../examples/wheatstone.nl")),
    (
        "gain_stage",
        include_str!("fixtures/benchmarks/gain_stage.nl"),
    ),
    (
        "high_fanout",
        include_str!("fixtures/schematic/high_fanout.nl"),
    ),
    (
        "power_amplifier",
        include_str!("fixtures/benchmarks/power_amplifier.nl"),
    ),
];

fn schematic(source: &str) -> netlang_core::schematic::Schematic {
    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(
        !report.has_errors(),
        "diagnostics: {:?}",
        report.diagnostics
    );
    report.schematic.expect("schematic was requested")
}

fn svg_hash(source: &str) -> String {
    let svg = schematic_svg::render_svg(&schematic(source));
    format!("{:x}", Sha256::digest(svg.as_bytes()))
}

#[test]
fn corpus_is_connectivity_verified_and_passes_quality_gates() {
    for (name, source) in CORPUS {
        let schematic = schematic(source);
        assert_eq!(schematic.schema_version, SCHEMATIC_SCHEMA_VERSION);
        assert!(
            schematic.connectivity.verified,
            "{name}: {:?}",
            schematic.connectivity.errors
        );
        assert_eq!(
            schematic.connectivity.expected_connected_pins,
            schematic.connectivity.represented_connected_pins,
            "{name}"
        );
        assert!(
            schematic.quality.passed,
            "{name}: {:?}; crossings={:?}; nets={:?}",
            schematic.quality, schematic.crossings, schematic.nets
        );
    }
}

#[test]
fn component_pin_anchors_come_from_the_shared_catalog() {
    let source = include_str!("fixtures/benchmarks/power_amplifier.nl");
    let report = compile_source(source, CompileOptions::all_outputs());
    let circuit = report.ir.expect("IR was requested");
    let schematic = report.schematic.expect("schematic was requested");

    for component in &circuit.components {
        let expected: BTreeSet<_> = component_definition(&component.kind)
            .pins
            .iter()
            .map(|pin| pin.name)
            .collect();
        let actual: BTreeSet<_> = schematic
            .components
            .iter()
            .find(|candidate| candidate.id == component.id)
            .expect("every IR component must be placed")
            .pins
            .iter()
            .map(|pin| pin.name.as_str())
            .collect();
        assert_eq!(actual, expected, "{}", component.id);
    }
}

#[test]
fn schematic_json_and_svg_are_byte_stable_across_repeated_compiles() {
    for (name, source) in CORPUS {
        let first = schematic(source);
        let second = schematic(source);
        assert_eq!(
            serde_json::to_vec(&first).expect("schematic should serialize"),
            serde_json::to_vec(&second).expect("schematic should serialize"),
            "{name} JSON changed"
        );
        assert_eq!(
            schematic_svg::render_svg(&first),
            schematic_svg::render_svg(&second),
            "{name} SVG changed"
        );
    }
}

#[test]
fn declaration_and_connection_order_do_not_change_schematic() {
    let first = "net GND\nnet IN\nnet OUT\nsource VIN ac(1V)\nresistor R1 1k\ncapacitor C1 159nF\nconnect VIN.minus, C1.p2 to GND\nconnect VIN.plus, R1.p1 to IN\nconnect R1.p2, C1.p1 to OUT\n";
    let reordered = "capacitor C1 159nF\nresistor R1 1k\nsource VIN ac(1V)\nnet OUT\nnet IN\nnet GND\nconnect C1.p1, R1.p2 to OUT\nconnect R1.p1, VIN.plus to IN\nconnect C1.p2, VIN.minus to GND\n";
    assert_eq!(
        serde_json::to_vec(&schematic(first)).unwrap(),
        serde_json::to_vec(&schematic(reordered)).unwrap()
    );
}

#[test]
fn high_fanout_and_supply_nets_use_semantic_labels() {
    let fanout = schematic(include_str!("fixtures/schematic/high_fanout.nl"));
    assert!(fanout.labels.iter().any(|label| label.text == "BUS"));
    assert!(fanout.labels.iter().any(|label| label.text == "0"));

    let gain = schematic(include_str!("fixtures/benchmarks/gain_stage.nl"));
    assert!(gain.labels.iter().any(|label| label.text == "VCC"));
    assert!(gain.labels.iter().any(|label| label.text == "VEE"));
}

#[test]
fn direct_generator_accepts_only_typed_ir() {
    let source = include_str!("fixtures/valid/minimal.nl");
    let program = parse_program(source).unwrap().flatten().unwrap();
    let circuit = netlang_core::ir::ast_to_ir(&program).unwrap();
    let schematic = generate_schematic(&circuit).unwrap();
    assert!(schematic.connectivity.verified);
}

#[test]
fn svg_visual_golden_hashes_are_cross_platform_stable() {
    for (name, source, expected) in [
        (
            "minimal",
            CORPUS[0].1,
            "85c0aa65dc6336b0a8e87036389093b7529aa3f161d3b006cb20e3fbc255780e",
        ),
        (
            "rc_filter",
            CORPUS[1].1,
            "1fe5d1e6d7980173597766041f80d8222061292858a3c73aa5af3a9b9ee1817f",
        ),
        (
            "wheatstone",
            CORPUS[2].1,
            "4af67149e60bbbeb15b247614b512cdb26702732b413b47151860a5901e839b5",
        ),
        (
            "gain_stage",
            CORPUS[3].1,
            "8f833aea03910bd76d80e1371a9be32931bbc275142ea6f240fcd559706f9d96",
        ),
        (
            "high_fanout",
            CORPUS[4].1,
            "575559d2af8b68a71549c0a7ab47698a638425dcfc4ad356ea595520245cdb44",
        ),
        (
            "power_amplifier",
            CORPUS[5].1,
            "d5372a3ea105bad265ebb94c1ff88ce9f9ad233a606585496da75e6ad4a3bcd3",
        ),
    ] {
        let actual = svg_hash(source);
        assert_eq!(actual, expected, "{name}");
    }
}
