mod common;

use common::{normalize_text, read_fixture};
use netlang_core::erc::check_rules;
use netlang_core::graph::{NetlistGraph, generate_spice};
use netlang_core::ir::ast_to_ir;
use netlang_core::parse_program;
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
    let source = read_fixture("valid/minimal.nl");
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
        ("demo_circuit.nl", "golden/demo_circuit.spice"),
        ("wheatstone.nl", "golden/wheatstone.spice"),
        ("test_features.nl", "golden/test_features.spice"),
        ("test_nc.nl", "golden/test_nc.spice"),
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
