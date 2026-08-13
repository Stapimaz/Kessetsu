use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::component::{PinFlow, component_definition};
use kessetsu_core::ir::{BJTPolarity, ComponentKind, FETPolarity};
use kessetsu_core::schematic::{SCHEMATIC_SCHEMA_VERSION, generate_schematic};
use kessetsu_core::{parse_program, schematic_svg};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CORPUS: &[(&str, &str)] = &[
    ("minimal", include_str!("fixtures/valid/minimal.kess")),
    (
        "rc_filter",
        include_str!("fixtures/benchmarks/rc_filter.kess"),
    ),
    ("wheatstone", include_str!("../../examples/wheatstone.kess")),
    (
        "gain_stage",
        include_str!("fixtures/benchmarks/gain_stage.kess"),
    ),
    (
        "high_fanout",
        include_str!("fixtures/schematic/high_fanout.kess"),
    ),
    (
        "power_amplifier",
        include_str!("fixtures/benchmarks/power_amplifier.kess"),
    ),
    (
        "inverting_amplifier",
        include_str!("fixtures/schematic/inverting_amplifier.kess"),
    ),
    (
        "differential_pair",
        include_str!("fixtures/schematic/differential_pair.kess"),
    ),
    (
        "mosfet_common_source",
        include_str!("fixtures/schematic/mosfet_common_source.kess"),
    ),
    (
        "rlc_ladder",
        include_str!("fixtures/schematic/rlc_ladder.kess"),
    ),
    (
        "diode_clamp",
        include_str!("fixtures/schematic/diode_clamp.kess"),
    ),
    (
        "bjt_common_emitter",
        include_str!("fixtures/schematic/bjt_common_emitter.kess"),
    ),
    (
        "summing_amplifier",
        include_str!("fixtures/schematic/summing_amplifier.kess"),
    ),
];

fn schematic(source: &str) -> kessetsu_core::schematic::Schematic {
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
    let source = include_str!("fixtures/benchmarks/power_amplifier.kess");
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
    let fanout = schematic(include_str!("fixtures/schematic/high_fanout.kess"));
    assert!(fanout.labels.iter().any(|label| label.text == "BUS"));
    assert!(fanout.labels.iter().any(|label| label.text == "0"));

    let gain = schematic(include_str!("fixtures/benchmarks/gain_stage.kess"));
    assert!(gain.labels.iter().any(|label| label.text == "VCC"));
    assert!(gain.labels.iter().any(|label| label.text == "VEE"));
}

#[test]
fn direct_generator_accepts_only_typed_ir() {
    let source = include_str!("fixtures/valid/minimal.kess");
    let program = parse_program(source).unwrap().flatten().unwrap();
    let circuit = kessetsu_core::ir::ast_to_ir(&program).unwrap();
    let schematic = generate_schematic(&circuit).unwrap();
    assert!(schematic.connectivity.verified);
}

#[test]
fn shared_catalog_exposes_semantic_pin_flow_for_layout() {
    for (kind, pin, expected) in [
        (ComponentKind::Resistor, "p1", PinFlow::Passive),
        (ComponentKind::VoltageSource, "plus", PinFlow::Output),
        (ComponentKind::OpAmp, "in_n", PinFlow::Input),
        (ComponentKind::OpAmp, "out", PinFlow::Output),
        (ComponentKind::OpAmp, "vcc", PinFlow::Power),
        (ComponentKind::BJT(BJTPolarity::NPN), "b", PinFlow::Input),
        (
            ComponentKind::MOSFET(FETPolarity::NMOS),
            "d",
            PinFlow::Conduction,
        ),
    ] {
        let actual = component_definition(&kind)
            .pins
            .iter()
            .find(|candidate| candidate.name == pin)
            .map(|candidate| candidate.flow);
        assert_eq!(actual, Some(expected), "{kind:?}.{pin}");
    }
}

#[test]
fn local_signal_nets_remain_explicitly_wired() {
    for (name, source) in CORPUS {
        let schematic = schematic(source);
        assert_eq!(
            schematic.quality.local_signal_label_pins, 0,
            "{name} hid local signal pins behind labels"
        );
        assert_eq!(
            schematic.quality.explicit_wire_coverage_per_mille, 1_000,
            "{name} did not explicitly wire every local signal pin"
        );
    }
}

#[test]
fn topology_patterns_preserve_conventional_stage_geometry() {
    let power = schematic(CORPUS[5].1);
    let center_x = |id: &str| {
        let component = power
            .components
            .iter()
            .find(|component| component.id == id)
            .unwrap();
        component.bounds.min.x + component.bounds.max.x
    };
    assert!(center_x("VIN") < center_x("U1"));
    assert!(center_x("U1") < center_x("U2"));
    assert!(center_x("U2") < center_x("U3"));
    assert!(center_x("U3") < center_x("QN"));
    assert!(center_x("QN") < center_x("RL"));

    let pair = schematic(CORPUS[7].1);
    let q1 = pair
        .components
        .iter()
        .find(|component| component.id == "Q1")
        .unwrap();
    let q2 = pair
        .components
        .iter()
        .find(|component| component.id == "Q2")
        .unwrap();
    assert!(!q1.mirrored_x);
    assert!(q2.mirrored_x);
    assert_eq!(q1.bounds.min.y, q2.bounds.min.y);

    let bridge = schematic(CORPUS[2].1);
    assert_eq!(bridge.crossings.len(), 0);
    let rx = bridge
        .components
        .iter()
        .find(|component| component.id == "Rx")
        .unwrap();
    assert!(matches!(
        rx.orientation,
        kessetsu_core::schematic::Orientation::Right | kessetsu_core::schematic::Orientation::Left
    ));

    for source in [CORPUS[8].1, CORPUS[11].1] {
        let stage = schematic(source);
        let active = stage
            .components
            .iter()
            .find(|component| component.id == "M1" || component.id == "Q1")
            .unwrap();
        let upper = stage
            .components
            .iter()
            .find(|component| component.id == "RD" || component.id == "RC")
            .unwrap();
        let lower = stage
            .components
            .iter()
            .find(|component| component.id == "RS" || component.id == "RE")
            .unwrap();
        assert!(upper.bounds.max.y < active.bounds.min.y);
        assert!(active.bounds.max.y < lower.bounds.min.y);
    }
}

#[test]
fn svg_visual_golden_hashes_are_cross_platform_stable() {
    for (name, source, expected) in [
        (
            "minimal",
            CORPUS[0].1,
            "66904cc17218ad5714e5af6304b8ea9ad15e4a1f7a52d25339701e9f278e2011",
        ),
        (
            "rc_filter",
            CORPUS[1].1,
            "052e96d7edebb6987f7fe2f2e4107db98136574a3148ac9702f9dd03160dcc69",
        ),
        (
            "wheatstone",
            CORPUS[2].1,
            "fff5819becaec0151ac506b209547b8ad75088a5c3be8ab19b9f13f5d027deef",
        ),
        (
            "gain_stage",
            CORPUS[3].1,
            "cbeb215f22d0c760ff59af9e09c1fb71f2bd9a62ea6d35878ff202f15f8d97fa",
        ),
        (
            "high_fanout",
            CORPUS[4].1,
            "b280f9d1473e503bfad37850ef2e4fdcc9147e3570939e389f83d6cf1e236c21",
        ),
        (
            "power_amplifier",
            CORPUS[5].1,
            "7e86ad04ce95f77e79002df4a03b6c8ef5fac2b790563686bdb1c41ceff756c3",
        ),
    ] {
        let actual = svg_hash(source);
        assert_eq!(actual, expected, "{name}");
    }
}
