use crate::ast::*;
use pest::Parser;

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
            let decl_inner = inner.into_inner().next().unwrap();
            match decl_inner.as_rule() {
                Rule::standard_decl => {
                    let mut inner_rules = decl_inner.into_inner();
                    let comp_str = inner_rules.next().unwrap().as_str();
                    let comp_type = match comp_str {
                        "resistor" => ComponentType::Resistor,
                        "capacitor" => ComponentType::Capacitor,
                        "inductor" => ComponentType::Inductor,
                        "diode" => ComponentType::Diode,
                        "mosfet" => ComponentType::Mosfet,
                        "opamp" => ComponentType::OpAmp,
                        _ => unreachable!(),
                    };
                    let name = inner_rules.next().unwrap().as_str().to_string();
                    let value = inner_rules.next().map(|v| {
                        let mut val = v.as_str().to_string();
                        if val.starts_with('"') && val.ends_with('"') {
                            val = val[1..val.len() - 1].to_string();
                        }
                        val
                    });

                    Some(Statement::Decl(ComponentDecl {
                        comp_type,
                        name,
                        subtype: None,
                        value,
                    }))
                }
                Rule::transistor_decl => {
                    let mut inner_rules = decl_inner.into_inner();
                    let name = inner_rules.next().unwrap().as_str().to_string();

                    let mut subtype = None;
                    let mut value = None;

                    for rule in inner_rules {
                        match rule.as_rule() {
                            Rule::polarity => subtype = Some(rule.as_str().to_string()),
                            Rule::comp_value => {
                                let mut val = rule.as_str().to_string();
                                if val.starts_with('"') && val.ends_with('"') {
                                    val = val[1..val.len() - 1].to_string();
                                }
                                value = Some(val);
                            }
                            _ => {}
                        }
                    }
                    Some(Statement::Decl(ComponentDecl {
                        comp_type: ComponentType::Transistor,
                        name,
                        subtype,
                        value,
                    }))
                }
                Rule::source_decl => {
                    let mut inner_rules = decl_inner.into_inner();
                    let source_type_str = inner_rules.next().unwrap().as_str();
                    let comp_type = if source_type_str == "current_source" {
                        ComponentType::CurrentSource
                    } else {
                        ComponentType::Source
                    };
                    let name = inner_rules.next().unwrap().as_str().to_string();
                    let value = Some(inner_rules.next().unwrap().as_str().to_string());

                    Some(Statement::Decl(ComponentDecl {
                        comp_type,
                        name,
                        subtype: None,
                        value,
                    }))
                }
                _ => unreachable!(),
            }
        }
        Rule::net_stmt => {
            let name = inner.into_inner().next().unwrap().as_str().to_string();
            Some(Statement::Net(NetDecl { name }))
        }
        Rule::connect => {
            let mut pins = Vec::new();
            for p in inner.into_inner() {
                let mut p_inner = p.into_inner();
                let first = p_inner.next().unwrap().as_str().to_string();
                let pin = if let Some(second) = p_inner.next() {
                    PinRef {
                        component: first,
                        pin: second.as_str().to_string(),
                    }
                } else {
                    PinRef {
                        component: "".to_string(),
                        pin: first,
                    }
                };
                pins.push(pin);
            }
            Some(Statement::Connect(Connection { pins }))
        }
        Rule::assert_stmt => {
            let mut inner_rules = inner.into_inner();
            let metric = inner_rules.next().unwrap().as_str().to_string();
            let signal = inner_rules.next().unwrap().as_str().to_string();
            let cmp_str = inner_rules.next().unwrap().as_str();
            let cmp = match cmp_str {
                "<" => Cmp::Lt,
                ">" => Cmp::Gt,
                "==" => Cmp::Eq,
                "<=" => Cmp::Le,
                ">=" => Cmp::Ge,
                _ => unreachable!(),
            };
            let threshold = inner_rules.next().unwrap().as_str().to_string();

            Some(Statement::Assert(AssertStmt {
                metric,
                signal,
                cmp,
                threshold,
            }))
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
                    arg_val = arg_val[1..arg_val.len() - 1].to_string();
                }
                args.push(arg_val);
            }
            Some(Statement::Simulate(SimulateStmt { cmd, args }))
        }
        _ => None,
    }
}
