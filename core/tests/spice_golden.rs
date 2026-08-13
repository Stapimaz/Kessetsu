mod common;

use common::{normalize_text, read_fixture};
use kessetsu_core::erc::check_rules;
use kessetsu_core::graph::{NetlistGraph, generate_spice};
use kessetsu_core::ir::ast_to_ir;
use kessetsu_core::parse_program;
use std::fs;
use std::path::Path;

fn generate(source: &str) -> String {
    let program = parse_program(source)
        .expect("fixture should parse")
        .flatten()
        .expect("fixture should flatten");
    let circuit = ast_to_ir(&program).expect("fixture should convert to IR");
    let graph = NetlistGraph::build(&circuit);
    assert!(check_rules(&circuit, &graph).is_empty());
    normalize_text(&generate_spice(&circuit, &graph))
}

#[test]
fn minimal_circuit_matches_canonical_spice_snapshot() {
    let source = read_fixture("valid/minimal.kess");
    let actual = generate(&source);
    let expected = normalize_text(&read_fixture("golden/minimal.spice"));
    assert_eq!(actual, expected);
}

#[test]
fn valid_repository_examples_match_canonical_spice_snapshots() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples");
    let cases = [
        ("demo_circuit.kess", "golden/demo_circuit.spice"),
        ("wheatstone.kess", "golden/wheatstone.spice"),
        ("test_features.kess", "golden/test_features.spice"),
        ("test_nc.kess", "golden/test_nc.spice"),
    ];

    for (source_name, golden_name) in cases {
        let source_path = examples.join(source_name);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("could not read '{}': {error}", source_path.display()));
        let actual = generate(&source);
        let expected = normalize_text(&read_fixture(golden_name));
        assert_eq!(actual, expected, "golden mismatch for {source_name}");
    }
}

#[test]
fn typed_ac_and_dc_analyses_emit_canonical_spice_commands() {
    let source = "source BIAS 5V\ncurrent_source LOAD 1mA\nresistor R1 1k\nconnect BIAS.plus to R1.p1\nconnect R1.p2 to BIAS.minus\nsimulate ac dec 20 10Hz 1MHz\nsimulate dc BIAS -1V 5V 100mV\nsimulate dc LOAD 2mA -2mA -100uA\n";
    let program = parse_program(source).expect("source should parse");
    let circuit = ast_to_ir(&program).expect("source should reach typed IR");
    let graph = NetlistGraph::build(&circuit);
    let spice = generate_spice(&circuit, &graph);

    assert!(spice.contains("\nac dec 20 10 1e6\n"));
    assert!(spice.contains("\ndc V_BIAS -1 5 0.1\n"));
    assert!(spice.contains("\ndc I_LOAD 0.002 -0.002 -1e-4\n"));
}
