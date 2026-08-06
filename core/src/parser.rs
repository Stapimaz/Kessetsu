use pest::Parser;
use crate::ast::*;

#[derive(Parser)]
#[grammar = "netlang.pest"]
pub struct NetlangParser;

pub fn parse_program(input: &str) -> Result<Program, pest::error::Error<Rule>> {
    let mut modules = Vec::new();
    let mut main_statements = Vec::new();

    let pairs = NetlangParser::parse(Rule::program, input)?;

    for pair in pairs {
        if pair.as_rule() == Rule::program {
            for top_level in pair.into_inner() {
                match top_level.as_rule() {
                    Rule::top_level => {
                        let inner = top_level.into_inner().next().unwrap();
                        match inner.as_rule() {
                            Rule::module_decl => {
                                let mut inner_rules = inner.into_inner();
                                let name = inner_rules.next().unwrap().as_str().to_string();
                                
                                let mut pins = Vec::new();
                                let mut statements = Vec::new();

                                for module_item in inner_rules {
                                    match module_item.as_rule() {
                                        Rule::pin_list => {
                                            for pin in module_item.into_inner() {
                                                pins.push(pin.as_str().to_string());
                                            }
                                        }
                                        Rule::statement => {
                                            if let Some(stmt) = parse_statement(module_item) {
                                                statements.push(stmt);
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                modules.push(ModuleDef {
                                    name,
                                    pins,
                                    statements,
                                });
                            }
                            Rule::statement => {
                                if let Some(stmt) = parse_statement(inner) {
                                    main_statements.push(stmt);
                                }
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

    Ok(Program {
        modules,
        statements: main_statements,
    })
}

fn parse_statement(statement_pair: pest::iterators::Pair<Rule>) -> Option<Statement> {
    let inner = statement_pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::decl => {
            let mut inner_rules = inner.into_inner();
            let comp_str = inner_rules.next().unwrap().as_str();
            let comp_type = match comp_str {
                "resistor" => ComponentType::Resistor,
                "source" | "battery" => ComponentType::Source,
                "capacitor" => ComponentType::Capacitor,
                "inductor" => ComponentType::Inductor,
                "diode" => ComponentType::Diode,
                "transistor" => ComponentType::Transistor,
                "mosfet" => ComponentType::Mosfet,
                "opamp" => ComponentType::OpAmp,
                _ => unreachable!(),
            };
            let name = inner_rules.next().unwrap().as_str().to_string();
            let mut value = if let Some(val_node) = inner_rules.next() {
                val_node.as_str().to_string()
            } else {
                "".to_string()
            };
            
            if value.starts_with('"') && value.ends_with('"') {
                value = value[1..value.len()-1].to_string();
            }
            
            Some(Statement::Decl(ComponentDecl {
                comp_type,
                name,
                value,
            }))
        }
        Rule::connect => {
            let mut inner_rules = inner.into_inner();
            let p1 = inner_rules.next().unwrap();
            let mut p1_inner = p1.into_inner();
            let first = p1_inner.next().unwrap().as_str().to_string();
            let pin1 = if let Some(second) = p1_inner.next() {
                PinRef { component: first, pin: second.as_str().to_string() }
            } else {
                PinRef { component: "".to_string(), pin: first }
            };

            let p2 = inner_rules.next().unwrap();
            let mut p2_inner = p2.into_inner();
            let first = p2_inner.next().unwrap().as_str().to_string();
            let pin2 = if let Some(second) = p2_inner.next() {
                PinRef { component: first, pin: second.as_str().to_string() }
            } else {
                PinRef { component: "".to_string(), pin: first }
            };
            Some(Statement::Connect(Connection { pin1, pin2 }))
        }
        Rule::use_stmt => {
            let mut inner_rules = inner.into_inner();
            let module_name = inner_rules.next().unwrap().as_str().to_string();
            let inst_name = inner_rules.next().unwrap().as_str().to_string();
            Some(Statement::Use(UseStmt {
                module_name,
                inst_name,
            }))
        }
        Rule::sim_cmd => {
            let mut inner_rules = inner.into_inner();
            let cmd = inner_rules.next().unwrap().as_str().to_string();
            let mut args = Vec::new();
            for arg in inner_rules {
                let mut arg_val = arg.as_str().to_string();
                if arg_val.starts_with('"') && arg_val.ends_with('"') {
                    arg_val = arg_val[1..arg_val.len()-1].to_string();
                }
                args.push(arg_val);
            }
            Some(Statement::Simulate(SimulateStmt { cmd, args }))
        }
        _ => None,
    }
}
