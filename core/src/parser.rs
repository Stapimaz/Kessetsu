use crate::ast::*;
use pest::Parser;

#[derive(Parser)]
#[grammar = "kessetsu.pest"]
pub struct KessetsuParser;

pub fn parse_program(input: &str) -> Result<Program, pest::error::Error<Rule>> {
    let mut modules = Vec::new();
    let mut model_includes = Vec::new();
    let mut models = Vec::new();
    let mut subcircuits = Vec::new();
    let mut main_statements = Vec::new();

    let pairs = KessetsuParser::parse(Rule::program, input)?;

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
                            Rule::model_include => {
                                let mut fields = inner.into_inner();
                                model_includes.push(ModelInclude {
                                    package: fields.next().unwrap().as_str().to_string(),
                                    version: unquote(fields.next().unwrap().as_str()),
                                });
                            }
                            Rule::model_decl => models.push(parse_model_decl(inner)),
                            Rule::subcircuit_decl => {
                                subcircuits.push(parse_subcircuit_decl(inner));
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
        model_includes,
        models,
        subcircuits,
        statements: main_statements,
    })
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

fn parse_named_value(pair: pest::iterators::Pair<Rule>) -> NamedValue {
    let mut fields = pair.into_inner();
    NamedValue {
        name: fields.next().unwrap().as_str().to_string(),
        value: unquote(fields.next().unwrap().as_str()),
    }
}

fn parse_model_decl(pair: pest::iterators::Pair<Rule>) -> ModelDecl {
    let mut fields = pair.into_inner();
    let kind = match fields.next().unwrap().as_str() {
        "diode" => ModelDeclKind::Diode,
        "bjt" => ModelDeclKind::BJT,
        "mosfet" => ModelDeclKind::MOSFET,
        _ => unreachable!(),
    };
    let name = fields.next().unwrap().as_str().to_string();
    let mut polarity = None;
    let mut parameters = Vec::new();
    for field in fields {
        match field.as_rule() {
            Rule::model_polarity => polarity = Some(field.as_str().to_string()),
            Rule::named_value => parameters.push(parse_named_value(field)),
            _ => unreachable!(),
        }
    }
    ModelDecl {
        kind,
        name,
        polarity,
        parameters,
    }
}

fn parse_subcircuit_decl(pair: pest::iterators::Pair<Rule>) -> SubcircuitDecl {
    let mut fields = pair.into_inner();
    let kind = fields.next().unwrap().as_str().to_string();
    let name = fields.next().unwrap().as_str().to_string();
    let pins = fields
        .next()
        .unwrap()
        .into_inner()
        .map(|pin| pin.as_str().to_string())
        .collect();
    let parameters = fields.map(parse_named_value).collect();
    SubcircuitDecl {
        kind,
        name,
        pins,
        parameters,
    }
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
            let metric_call = inner_rules.next().unwrap();
            let mut call_rules = metric_call.into_inner();
            let metric = call_rules.next().unwrap().as_str().to_string();
            let signal = call_rules
                .map(|argument| argument.as_str().to_string())
                .collect::<Vec<_>>()
                .join(",");
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
