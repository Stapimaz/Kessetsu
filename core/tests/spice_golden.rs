mod common;

use common::{normalize_text, read_fixture};
use netlang_core::erc::check_rules;
use netlang_core::graph::{NetlistGraph, generate_spice};
use netlang_core::ir::ast_to_ir;
use netlang_core::parse_program;

#[test]
fn minimal_circuit_matches_canonical_spice_snapshot() {
    let source = read_fixture("valid/minimal.nl");
    let program = parse_program(&source)
        .expect("fixture should parse")
        .flatten()
        .expect("fixture should flatten");
    let circuit = ast_to_ir(&program).expect("fixture should convert to IR");
    let graph = NetlistGraph::build(&circuit);
    assert!(check_rules(&circuit, &graph).is_empty());

    let actual = normalize_text(&generate_spice(&circuit, &graph));
    let expected = normalize_text(&read_fixture("golden/minimal.spice"));
    assert_eq!(actual, expected);
}
