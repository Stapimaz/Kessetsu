mod common;

use common::read_fixture;
use netlang_core::erc::{ErcDiagnostic, check_rules};
use netlang_core::graph::{NetlistGraph, generate_spice};
use netlang_core::ir::{CircuitIR, ast_to_ir};
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

    assert_eq!(graph.get_net("V1", "minus"), 0);
    assert_eq!(graph.get_net("R1", "p2"), 0);
    assert_eq!(graph.get_net_name(0), "0");
}

#[test]
fn user_named_net_takes_precedence_over_generated_name() {
    let source = "net output\nsource V1 5V\nresistor R1 1k\nconnect V1.plus to R1.p1\nconnect R1.p2 to output\nconnect V1.minus to output\n";
    let circuit = circuit_from(source);
    let graph = NetlistGraph::build(&circuit);

    let output_net = graph.get_net("R1", "p2");
    assert_eq!(graph.get_net_name(output_net), "0");

    let named_source = "net output\nsource V1 5V\nresistor R1 1k\nresistor R2 1k\nconnect V1.plus to R1.p1\nconnect R1.p2, R2.p1 to output\nconnect R2.p2 to V1.minus\n";
    let circuit = circuit_from(named_source);
    let graph = NetlistGraph::build(&circuit);
    assert_eq!(graph.get_net_name(graph.get_net("R1", "p2")), "output");
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
