extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod ast;
pub mod parser;
pub mod drc;
pub mod wasm;

pub use parser::parse_program;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drc::check_rules;

    #[test]
    fn test_parse_and_drc() {
        let input = "resistor R1 10k\nbattery B1 5V\nconnect B1.plus R1.p1\n";
        let program = parse_program(input).unwrap();
        assert_eq!(program.statements.len(), 3);
        
        let errors = check_rules(&program);
        assert!(errors.is_empty());
        
        let bad_input = "resistor R1 10k\nconnect B1.plus R1.p1\n";
        let bad_program = parse_program(bad_input).unwrap();
        let errors = check_rules(&bad_program);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Connection refers to undeclared component: B1");
    }
}
