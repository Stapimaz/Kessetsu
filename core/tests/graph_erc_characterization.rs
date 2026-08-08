mod common;

use common::read_fixture;
use netlang_core::component::component_definition;
use netlang_core::erc::{ErcDiagnostic, check_rules};
use netlang_core::graph::{NetId, NetlistGraph, generate_spice};
use netlang_core::ir::{BJTPolarity, CircuitIR, ComponentKind, FETPolarity, ast_to_ir};
use netlang_core::parse_program;

fn circuit_from(source: &str) -> CircuitIR {
    let program = parse_program(source)
        .expect("fixture should parse")
        .flatten()
        .expect("fixture should flatten");
    ast_to_ir(&program).expect("fixture should convert to IR")
}

fn diagnostics_for(source: &str) -> Vec<ErcDiagnostic> {
    let circuit = circuit_from(source);
    let graph = NetlistGraph::build(&circuit);
    check_rules(&circuit, &graph)
}

#[test]
fn each_existing_erc_code_has_a_regression_fixture() {
    let cases = [
        ("invalid/semantic/duplicate_component.nl", "NL-E001"),
        ("invalid/semantic/undefined_component.nl", "NL-E002"),
        ("invalid/semantic/floating_pin.nl", "NL-E003"),
        ("invalid/semantic/shorted_source.nl", "NL-E004"),
        ("invalid/semantic/invalid_pin.nl", "NL-E005"),
    ];

    for (fixture, expected_code) in cases {
        let diagnostics = diagnostics_for(&read_fixture(fixture));
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == expected_code),
            "{fixture} did not produce {expected_code}: {:?}",
            diagnostics
                .iter()
                .map(|item| &item.code)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn voltage_source_minus_net_is_canonical_ground() {
    let circuit = circuit_from(&read_fixture("valid/minimal.nl"));
    let graph = NetlistGraph::build(&circuit);

    assert_eq!(graph.get_net("V1", "minus"), Some(NetId::GROUND));
    assert_eq!(graph.get_net("R1", "p2"), Some(NetId::GROUND));
    assert_eq!(graph.get_net_name(NetId::GROUND), "0");
}

#[test]
fn user_named_net_takes_precedence_over_generated_name() {
    let source = "net output\nsource V1 5V\nresistor R1 1k\nconnect V1.plus to R1.p1\nconnect R1.p2 to output\nconnect V1.minus to output\n";
    let circuit = circuit_from(source);
    let graph = NetlistGraph::build(&circuit);

    let output_net = graph.get_net("R1", "p2");
    assert_eq!(
        graph.get_net_name(output_net.expect("net should exist")),
        "0"
    );

    let named_source = "net output\nsource V1 5V\nresistor R1 1k\nresistor R2 1k\nconnect V1.plus to R1.p1\nconnect R1.p2, R2.p1 to output\nconnect R2.p2 to V1.minus\n";
    let circuit = circuit_from(named_source);
    let graph = NetlistGraph::build(&circuit);
    assert_eq!(
        graph.get_net_name(graph.get_net("R1", "p2").expect("net should exist")),
        "output"
    );
}

#[test]
fn connection_statement_order_does_not_change_generated_spice() {
    let declarations = "source V1 5V\nresistor R1 1k\nresistor R2 2k\n";
    let source_a = format!(
        "{declarations}connect V1.plus to R1.p1\nconnect R1.p2 to R2.p1\nconnect R2.p2 to V1.minus\nsimulate op\n"
    );
    let source_b = format!(
        "{declarations}connect R2.p2 to V1.minus\nconnect R1.p2 to R2.p1\nconnect V1.plus to R1.p1\nsimulate op\n"
    );

    let circuit_a = circuit_from(&source_a);
    let graph_a = NetlistGraph::build(&circuit_a);
    let circuit_b = circuit_from(&source_b);
    let graph_b = NetlistGraph::build(&circuit_b);

    assert_eq!(
        generate_spice(&circuit_a, &graph_a),
        generate_spice(&circuit_b, &graph_b)
    );
}

#[test]
fn component_declaration_order_does_not_change_generated_spice() {
    let source_a = "source V1 5V\nresistor R1 1k\nresistor R2 2k\nconnect V1.plus to R1.p1\nconnect R1.p2 to R2.p1\nconnect R2.p2 to V1.minus\nsimulate op\n";
    let source_b = "resistor R2 2k\nsource V1 5V\nresistor R1 1k\nconnect V1.plus to R1.p1\nconnect R1.p2 to R2.p1\nconnect R2.p2 to V1.minus\nsimulate op\n";

    let circuit_a = circuit_from(source_a);
    let graph_a = NetlistGraph::build(&circuit_a);
    let circuit_b = circuit_from(source_b);
    let graph_b = NetlistGraph::build(&circuit_b);

    assert_eq!(
        generate_spice(&circuit_a, &graph_a),
        generate_spice(&circuit_b, &graph_b)
    );
}

#[test]
fn disconnected_source_ground_fallback_is_lexicographically_stable() {
    let source = "source Z1 9V\nresistor Rz 1k\nsource A1 5V\nresistor Ra 1k\nconnect Z1.plus to Rz.p1\nconnect Z1.minus to Rz.p2\nconnect A1.plus to Ra.p1\nconnect A1.minus to Ra.p2\n";
    let circuit = circuit_from(source);
    let graph = NetlistGraph::build(&circuit);

    assert_eq!(graph.get_net("A1", "minus"), Some(NetId::GROUND));
    assert_ne!(graph.get_net("Z1", "minus"), Some(NetId::GROUND));
}

#[test]
fn standard_models_and_full_netlist_are_byte_stable_across_rebuilds() {
    let source = include_str!("../../examples/test_features.nl");
    let circuit = circuit_from(source);
    let baseline = generate_spice(&circuit, &NetlistGraph::build(&circuit));

    let model_3904 = baseline
        .find(".model 2N3904")
        .expect("2N3904 model missing");
    let model_3906 = baseline
        .find(".model 2N3906")
        .expect("2N3906 model missing");
    assert!(
        model_3904 < model_3906,
        "models must be sorted by canonical name"
    );

    for _ in 0..100 {
        let actual = generate_spice(&circuit, &NetlistGraph::build(&circuit));
        assert_eq!(actual, baseline);
    }
}

#[test]
fn diagnostic_order_is_repeatable_for_the_same_circuit() {
    let source = "resistor R1 1k\nresistor R1 2k\ntransistor Q1 npn\n";
    let baseline: Vec<_> = diagnostics_for(source)
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.component, diagnostic.pin))
        .collect();

    for _ in 0..50 {
        let actual: Vec<_> = diagnostics_for(source)
            .into_iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.component, diagnostic.pin))
            .collect();
        assert_eq!(actual, baseline);
    }
}

#[test]
fn absent_connections_use_option_instead_of_a_magic_net_id() {
    let circuit = circuit_from("resistor R1 1k\n");
    let graph = NetlistGraph::build(&circuit);

    assert_eq!(graph.get_net("R1", "p1"), None);
    assert_eq!(graph.get_net("R1", "p2"), None);
    assert!(!graph.net_names.contains_key(&NetId(9999)));
}

#[test]
fn shared_component_catalog_defines_canonical_backend_pin_order() {
    let cases = [
        (ComponentKind::Resistor, vec!["p1", "p2"]),
        (ComponentKind::VoltageSource, vec!["plus", "minus"]),
        (ComponentKind::BJT(BJTPolarity::NPN), vec!["c", "b", "e"]),
        (
            ComponentKind::MOSFET(FETPolarity::NMOS),
            vec!["d", "g", "s"],
        ),
        (
            ComponentKind::OpAmp,
            vec!["in_p", "in_n", "vcc", "vee", "out"],
        ),
    ];

    for (kind, expected_pins) in cases {
        let definition = component_definition(&kind);
        let actual: Vec<_> = definition.pins.iter().map(|pin| pin.name).collect();
        assert_eq!(actual, expected_pins, "unexpected catalog for {kind:?}");
        assert!(definition.spice_prefix.is_some());
    }
}
