use pest::Parser;
use crate::ast::*;

#[derive(Parser)]
#[grammar = "netlang.pest"]
pub struct NetlangParser;

pub fn parse_program(input: &str) -> Result<Program, pest::error::Error<Rule>> {
    let mut ast_statements = Vec::new();
    let pairs = NetlangParser::parse(Rule::program, input)?;

    for pair in pairs {
        if pair.as_rule() == Rule::program {
            for statement in pair.into_inner() {
                match statement.as_rule() {
                    Rule::statement => {
                        let inner = statement.into_inner().next().unwrap();
                        match inner.as_rule() {
                            Rule::decl => {
                                let mut inner_rules = inner.into_inner();
                                let comp_str = inner_rules.next().unwrap().as_str();
                                let comp_type = match comp_str {
                                    "resistor" => ComponentType::Resistor,
                                    "battery" => ComponentType::Battery,
                                    "capacitor" => ComponentType::Capacitor,
                                    _ => unreachable!(),
                                };
                                let name = inner_rules.next().unwrap().as_str().to_string();
                                let value = inner_rules.next().unwrap().as_str().to_string();
                                ast_statements.push(Statement::Decl(ComponentDecl {
                                    comp_type,
                                    name,
                                    value,
                                }));
                            }
                            Rule::connect => {
                                let mut inner_rules = inner.into_inner();
                                let p1 = inner_rules.next().unwrap();
                                let mut p1_inner = p1.into_inner();
                                let pin1 = PinRef {
                                    component: p1_inner.next().unwrap().as_str().to_string(),
                                    pin: p1_inner.next().unwrap().as_str().to_string(),
                                };
                                let p2 = inner_rules.next().unwrap();
                                let mut p2_inner = p2.into_inner();
                                let pin2 = PinRef {
                                    component: p2_inner.next().unwrap().as_str().to_string(),
                                    pin: p2_inner.next().unwrap().as_str().to_string(),
                                };
                                ast_statements.push(Statement::Connect(Connection { pin1, pin2 }));
                            }
                            _ => {}
                        }
                    }
                    Rule::EOI => {}
                    _ => unreachable!(),
                }
            }
        }
    }
    Ok(Program { statements: ast_statements })
}
