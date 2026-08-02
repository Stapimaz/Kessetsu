extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod ast;
pub mod parser;
pub mod graph;
pub mod drc;
pub mod wasm;
pub mod layout;

pub use parser::parse_program;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drc::check_rules;

    #[test]
    fn test_parse_and_drc() {
        let input = "resistor R1 10k\nbattery B1 5V\nconnect B1.plus R1.p1\nconnect B1.minus R1.p2\n";
        let program = parse_program(input).unwrap().flatten().unwrap();
        assert_eq!(program.statements.len(), 4);
        
        let graph = crate::graph::NetlistGraph::build(&program);
        let errors = check_rules(&program, &graph);
        assert!(errors.is_empty());
        
        let bad_input = "resistor R1 10k\nconnect B1.plus R1.p1\n";
        let bad_program = parse_program(bad_input).unwrap().flatten().unwrap();
        let bad_graph = crate::graph::NetlistGraph::build(&bad_program);
        let errors = check_rules(&bad_program, &bad_graph);
        assert!(!errors.is_empty());
    }
}
