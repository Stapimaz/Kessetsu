use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::component::{PinFlow, component_definition};
use kessetsu_core::ir::{BJTPolarity, ComponentKind, FETPolarity};
use kessetsu_core::schematic::{SCHEMATIC_SCHEMA_VERSION, TextRole, generate_schematic};
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
fn semantic_labels_are_anchored_to_the_attached_pin() {
    for (name, source) in CORPUS {
        let schematic = schematic(source);
        for label in &schematic.labels {
            let component = schematic
                .components
                .iter()
                .find(|component| component.id == label.attached_to.component)
                .unwrap_or_else(|| panic!("{name}: missing component for {}", label.id));
            let pin = component
                .pins
                .iter()
                .find(|pin| pin.name == label.attached_to.pin)
                .unwrap_or_else(|| panic!("{name}: missing pin for {}", label.id));
            assert_eq!(
                label.point, pin.point,
                "{name}: {} is visually detached",
                label.id
            );
            assert_eq!(
                label.side, pin.side,
                "{name}: {} faces the wrong way",
                label.id
            );
        }
        assert_eq!(
            schematic.quality.detached_semantic_labels, 0,
            "{name}: quality gate missed a detached semantic label"
        );
    }
}

#[test]
fn component_text_is_owned_by_the_schematic_contract_and_stays_collision_free() {
    for (name, source) in CORPUS {
        let schematic = schematic(source);
        for component in &schematic.components {
            let references = schematic
                .texts
                .iter()
                .filter(|text| text.component == component.id && text.role == TextRole::Reference)
                .count();
            assert_eq!(references, 1, "{name}: {} reference", component.id);
            for text in schematic
                .texts
                .iter()
                .filter(|text| text.component == component.id)
            {
                assert!(
                    (0..8).contains(&text.offset_eighths.x)
                        && (0..8).contains(&text.offset_eighths.y),
                    "{name}: {} has a non-canonical fine text offset {:?}",
                    text.id,
                    text.offset_eighths
                );
            }
            if component
                .value
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            {
                assert!(
                    schematic.texts.iter().any(|text| {
                        text.component == component.id && text.role == TextRole::Value
                    }),
                    "{name}: {} value is missing",
                    component.id
                );
            }
            if matches!(
                component.symbol,
                kessetsu_core::component::CatalogSymbol::Resistor
                    | kessetsu_core::component::CatalogSymbol::Capacitor
                    | kessetsu_core::component::CatalogSymbol::Inductor
                    | kessetsu_core::component::CatalogSymbol::Diode
            ) && matches!(
                component.orientation,
                kessetsu_core::schematic::Orientation::Right
                    | kessetsu_core::schematic::Orientation::Left
            ) && let (Some(reference), Some(value)) = (
                schematic.texts.iter().find(|text| {
                    text.component == component.id && text.role == TextRole::Reference
                }),
                schematic
                    .texts
                    .iter()
                    .find(|text| text.component == component.id && text.role == TextRole::Value),
            ) {
                assert_ne!(
                    reference.point.y * 8 + reference.offset_eighths.y,
                    value.point.y * 8 + value.offset_eighths.y,
                    "{name}: {} reference/value share a text row",
                    component.id
                );
            }
        }
        assert_eq!(schematic.quality.text_symbol_collisions, 0, "{name}");
        assert_eq!(schematic.quality.text_wire_collisions, 0, "{name}");
        assert_eq!(schematic.quality.text_text_collisions, 0, "{name}");
        assert_eq!(schematic.quality.text_label_collisions, 0, "{name}");
        assert_eq!(schematic.quality.detached_component_texts, 0, "{name}");
        assert_eq!(
            schematic.quality.component_text_pair_violations, 0,
            "{name}"
        );
    }
}

#[test]
fn bjt_emitter_arrows_render_as_crisp_filled_markers() {
    for (name, source, expected_arrows) in [
        ("power_amplifier", CORPUS[5].1, 2),
        ("differential_pair", CORPUS[7].1, 2),
        ("bjt_common_emitter", CORPUS[11].1, 1),
    ] {
        let svg = schematic_svg::render_svg(&schematic(source));
        assert_eq!(
            svg.matches("class=\"emitter-arrow\"").count(),
            expected_arrows,
            "{name}"
        );
        assert_eq!(
            svg.matches("fill=\"#172033\" stroke=\"none\"/>").count(),
            expected_arrows,
            "{name}"
        );
    }
}

#[test]
fn compact_passive_chains_use_direct_aligned_connections() {
    for (name, source, endpoints) in [
        ("minimal", CORPUS[0].1, ("V1", "R1")),
        ("rc_filter", CORPUS[1].1, ("R1", "C1")),
    ] {
        let schematic = schematic(source);
        let wire = schematic
            .wires
            .iter()
            .find(|wire| {
                let serialized = serde_json::to_string(&[&wire.start, &wire.end]).unwrap();
                serialized.contains(endpoints.0) && serialized.contains(endpoints.1)
            })
            .unwrap_or_else(|| panic!("{name}: missing direct connection"));
        assert_eq!(wire.points.len(), 2, "{name}: {:?}", wire.points);
        let start = wire.points[0];
        let end = wire.points[1];
        assert!(
            start.x == end.x || start.y == end.y,
            "{name}: {:?}",
            wire.points
        );
        assert_eq!(
            schematic.quality.aligned_wire_coverage_per_mille, 1_000,
            "{name}"
        );
    }
}

#[test]
fn dual_supply_sources_form_a_compact_power_block() {
    let schematic = schematic(CORPUS[3].1);
    let vp = schematic
        .components
        .iter()
        .find(|component| component.id == "VP")
        .unwrap();
    let vn = schematic
        .components
        .iter()
        .find(|component| component.id == "VN")
        .unwrap();
    assert_eq!(vp.bounds.min.y, vn.bounds.min.y);
    assert!(vp.bounds.max.x < vn.bounds.min.x || vn.bounds.max.x < vp.bounds.min.x);
}

#[test]
fn small_parallel_networks_use_shared_horizontal_rails() {
    let schematic = schematic(include_str!("fixtures/schematic/parallel_branches.kess"));
    assert!(schematic.connectivity.verified);
    assert!(schematic.quality.passed, "{:?}", schematic.quality.issues);
    let top_pins: Vec<_> = schematic
        .components
        .iter()
        .map(|component| component.pins.iter().map(|pin| pin.point.y).min().unwrap())
        .collect();
    let bottom_pins: Vec<_> = schematic
        .components
        .iter()
        .map(|component| component.pins.iter().map(|pin| pin.point.y).max().unwrap())
        .collect();
    assert!(top_pins.windows(2).all(|pair| pair[0] == pair[1]));
    assert!(bottom_pins.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn grounded_output_loads_align_below_their_active_driver() {
    let schematic = schematic(include_str!("fixtures/schematic/opamp_output_load.kess"));
    let output_x = schematic
        .components
        .iter()
        .find(|component| component.id == "U1")
        .and_then(|component| component.pins.iter().find(|pin| pin.name == "out"))
        .map(|pin| pin.point.x)
        .unwrap();
    let load_x = schematic
        .components
        .iter()
        .find(|component| component.id == "R1")
        .and_then(|component| component.pins.iter().find(|pin| pin.name == "p1"))
        .map(|pin| pin.point.x)
        .unwrap();

    assert_eq!(load_x, output_x);
    assert!(schematic.connectivity.verified);
    assert!(schematic.quality.passed, "{:?}", schematic.quality.issues);
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
            "6f830225fc4a75b3e9f654c1f8398bb1c8b7fcd7c2e0741571dac267abca3a4b",
        ),
        (
            "rc_filter",
            CORPUS[1].1,
            "cd714d7005061d27889f342c10cc589b2340be12b4cdaee31b04f72c6c1e25dd",
        ),
        (
            "wheatstone",
            CORPUS[2].1,
            "31cfa59c8bd758c47578c8eff5bec4eb5875bac679b3bd0877a7484a43ff30c7",
        ),
        (
            "gain_stage",
            CORPUS[3].1,
            "537643881975fd56d4b7b25a5d23052a308afda69b35a5115c019d03736f7668",
        ),
        (
            "high_fanout",
            CORPUS[4].1,
            "bec7c0f842f962e88bd4357b68b56e9bfbfb0a2b31cf25feb0ad4d387393babc",
        ),
        (
            "power_amplifier",
            CORPUS[5].1,
            "dca2efa25df95d1015716dd0b69e7ed1b672a4aa0e35a1481cdd367e3cdd2b27",
        ),
    ] {
        let actual = svg_hash(source);
        assert_eq!(actual, expected, "{name}");
    }
}
