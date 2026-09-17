extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod ast;
pub mod compile_inputs;
pub mod compiler;
pub mod component;
mod elaboration;
pub mod erc;
pub mod exporter;
pub mod expression;
pub mod graph;
pub mod ir;
pub mod kicad;
pub mod layout;
pub mod ltspice;
pub mod measurement;
pub mod models;
pub mod parser;
pub mod requirements;
pub mod schematic;
mod schematic_geometry;
pub mod schematic_svg;
pub mod sim_result;
pub mod simulation;
pub mod simulation_parser;
pub mod tools;
pub mod wasm;

pub use compile_inputs::{CompileInputs, ParameterInput};
pub use compiler::{
    CompileOptions, CompileReport, compile_source, compile_source_with_inputs,
    compile_source_with_resources,
};
pub use parser::parse_program;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erc::check_rules;
    use crate::ir::ast_to_ir;

    #[test]
    fn test_parse_and_erc() {
        let input =
            "resistor R1 10k\nsource B1 5V\nconnect B1.plus to R1.p1\nconnect B1.minus to R1.p2\n";
        let program = parse_program(input).unwrap().flatten().unwrap();
        assert_eq!(program.statements.len(), 4);

        let circuit = ast_to_ir(&program).unwrap();
        let graph = crate::graph::NetlistGraph::build(&circuit);
        let errors = check_rules(&circuit, &graph);
        assert!(errors.is_empty());

        let bad_input = "resistor R1 10k\nconnect B1.plus to R1.p1\n";
        let bad_program = parse_program(bad_input).unwrap().flatten().unwrap();
        let bad_circuit = ast_to_ir(&bad_program).unwrap();
        let bad_graph = crate::graph::NetlistGraph::build(&bad_circuit);
        let errors = check_rules(&bad_circuit, &bad_graph);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].code, "KES-E002");
    }
}
