use crate::ast::*;
use pest::Parser;

#[derive(Parser)]
#[grammar = "kessetsu.pest"]
pub struct KessetsuParser;

pub const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SOURCE_STATEMENTS: usize = 100_000;

pub fn parse_program(input: &str) -> Result<Program, pest::error::Error<Rule>> {
    if input.len() > MAX_SOURCE_BYTES {
        // Do not retain/render an oversized input line inside the diagnostic itself.
        return Err(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: format!("Source exceeds the {MAX_SOURCE_BYTES} byte limit"),
            },
            pest::Position::new("", 0).unwrap(),
        ));
    }
    let mut modules = Vec::new();
    let mut model_includes = Vec::new();
    let mut models = Vec::new();
    let mut subcircuits = Vec::new();
    let mut external_subcircuits = Vec::new();
    let mut main_statements = Vec::new();
    let mut expression_work = 0;

    let pairs = KessetsuParser::parse(Rule::program, input)?;

    // Bound aggregate construction before allocating the typed AST. Grammar has
    // no recursive calls; expression depth/node limits run in expression_pair.
    let mut pending = pairs.clone().collect::<Vec<_>>();
    let mut statements = 0;
    while let Some(pair) = pending.pop() {
        if matches!(
            pair.as_rule(),
            Rule::statement
                | Rule::module_decl
                | Rule::model_decl
                | Rule::subcircuit_decl
                | Rule::external_subcircuit_decl
                | Rule::model_include
        ) {
            statements += 1;
            if statements > MAX_SOURCE_STATEMENTS {
                return Err(pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: format!(
                            "Source exceeds the {MAX_SOURCE_STATEMENTS} statement limit"
                        ),
                    },
                    pair.as_span(),
                ));
            }
        }
        pending.extend(pair.into_inner());
    }

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
                                            if let Some(stmt) = parse_bounded_statement(
                                                module_item,
                                                &mut expression_work,
                                            )? {
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
                                if let Some(stmt) =
                                    parse_bounded_statement(inner, &mut expression_work)?
                                {
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
                            Rule::external_subcircuit_decl => {
                                external_subcircuits.push(parse_external_subcircuit_decl(inner));
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
        external_subcircuits,
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

fn parse_external_subcircuit_decl(pair: pest::iterators::Pair<Rule>) -> ExternalSubcircuitDecl {
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
    ExternalSubcircuitDecl {
        kind,
        name,
        pins,
        parameters,
    }
}

fn expression_pair(
    pair: &pest::iterators::Pair<Rule>,
    braced: bool,
) -> Result<crate::expression::Expression, pest::error::Error<Rule>> {
    let text = pair.as_str();
    let text = if braced {
        &text[1..text.len() - 1]
    } else {
        text
    };
    crate::expression::parse_expression(text).map_err(|cause| {
        pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError {
                message: format!("{} (expression byte {})", cause.message, cause.offset + 1),
            },
            pair.as_span(),
        )
    })
}

fn numeric_expression(
    pair: &pest::iterators::Pair<Rule>,
) -> Result<Option<crate::expression::Expression>, pest::error::Error<Rule>> {
    if pair.as_str().starts_with('{') {
        expression_pair(pair, true).map(Some)
    } else {
        Ok(None)
    }
}

fn parse_statement(
    statement_pair: pest::iterators::Pair<Rule>,
) -> Result<Option<Statement>, pest::error::Error<Rule>> {
    let inner = statement_pair.into_inner().next().unwrap();
    Ok(match inner.as_rule() {
        Rule::param_decl => {
            let (line, column) = inner.as_span().start_pos().line_col();
            let mut fields = inner.into_inner();
            let name = fields.next().unwrap().as_str().to_string();
            let type_pair = fields.next().unwrap();
            let unit = crate::expression::parameter_unit(type_pair.as_str()).ok_or_else(|| pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError { message: format!("Unknown parameter type '{}'; use Ohm, F, H, V, A, Hz, s, W, ratio, percent or deg", type_pair.as_str()) },
                type_pair.as_span(),
            ))?;
            let expression = expression_pair(&fields.next().unwrap(), false)?;
            Some(Statement::Param(crate::expression::ParameterDecl {
                name,
                unit,
                expression,
                line,
                column,
                instance_path: Vec::new(),
                default_expression: None,
            }))
        }
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
                    let value_pair = inner_rules.next();
                    let value_expression = value_pair
                        .as_ref()
                        .map(numeric_expression)
                        .transpose()?
                        .flatten();
                    let value = value_pair.map(|v| {
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
                        value_expression,
                        waveform_expression: None,
                        interface_pins: Vec::new(),
                        instance_path: Vec::new(),
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
                        value_expression: None,
                        waveform_expression: None,
                        interface_pins: Vec::new(),
                        instance_path: Vec::new(),
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
                    let value_pair = inner_rules.next().unwrap();
                    let value_expression = numeric_expression(&value_pair)?;
                    let value = Some(value_pair.as_str().to_string());
                    let mut waveform_expression = None;
                    if value_pair.as_rule() == Rule::source_param {
                        let call = value_pair.clone().into_inner().next().unwrap();
                        if call.as_rule() == Rule::func_call {
                            let mut fields = call.into_inner();
                            let name = fields.next().unwrap().as_str().to_string();
                            let mut args = Vec::new();
                            for arg in fields {
                                let expression = numeric_expression(&arg)?;
                                args.push(NumericArgument {
                                    value: arg.as_str().to_string(),
                                    expression,
                                });
                            }
                            // Keep the legacy literal AST shape and parser path.
                            if args.iter().any(|arg| arg.expression.is_some()) {
                                waveform_expression = Some(WaveformCall { name, args });
                            }
                        }
                    }

                    Some(Statement::Decl(ComponentDecl {
                        comp_type,
                        name,
                        subtype: None,
                        value,
                        value_expression,
                        waveform_expression,
                        interface_pins: Vec::new(),
                        instance_path: Vec::new(),
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
            let mut arguments = Vec::new();
            let mut numeric_expressions = Vec::new();
            for argument in call_rules {
                if let Some(expression) = numeric_expression(&argument)? {
                    numeric_expressions.push(IndexedExpression {
                        index: arguments.len(),
                        expression,
                    });
                }
                arguments.push(argument.as_str().to_string());
            }
            let signal = arguments.join(",");
            let cmp_str = inner_rules.next().unwrap().as_str();
            let cmp = match cmp_str {
                "<" => Cmp::Lt,
                ">" => Cmp::Gt,
                "==" => Cmp::Eq,
                "<=" => Cmp::Le,
                ">=" => Cmp::Ge,
                _ => unreachable!(),
            };
            let threshold_pair = inner_rules.next().unwrap();
            let threshold_expression = numeric_expression(&threshold_pair)?;
            let threshold = threshold_pair.as_str().to_string();

            Some(Statement::Assert(AssertStmt {
                metric,
                signal,
                cmp,
                threshold,
                threshold_expression,
                numeric_expressions,
            }))
        }
        Rule::use_stmt => {
            let mut inner_rules = inner.into_inner();
            let module_name = inner_rules.next().unwrap().as_str().to_string();
            let inst_name = inner_rules.next().unwrap().as_str().to_string();
            let mut overrides = Vec::new();
            for pair in inner_rules {
                let (line, column) = pair.as_span().start_pos().line_col();
                let mut fields = pair.into_inner();
                let name = fields.next().unwrap().as_str().to_string();
                let value = fields.next().unwrap().into_inner().next().unwrap();
                if value.as_rule() != Rule::value_expr
                    && crate::ir::parse_value(value.as_str()).is_err()
                {
                    return Err(pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError { message: "A parameter override expects a numeric literal or a braced expression".into() },
                        value.as_span(),
                    ));
                }
                let expression = expression_pair(&value, value.as_rule() == Rule::value_expr)?;
                overrides.push(ParameterOverride {
                    name,
                    expression,
                    line,
                    column,
                });
            }
            Some(Statement::Use(UseStmt {
                module_name,
                inst_name,
                overrides,
            }))
        }
        Rule::sim_cmd => {
            let mut inner_rules = inner.into_inner();
            let cmd = inner_rules.next().unwrap().as_str().to_string();
            let mut args = Vec::new();
            let mut numeric_expressions = Vec::new();
            for arg in inner_rules {
                if let Some(expression) = numeric_expression(&arg)? {
                    numeric_expressions.push(IndexedExpression {
                        index: args.len(),
                        expression,
                    });
                }
                let mut arg_val = arg.as_str().to_string();
                if arg_val.starts_with('"') && arg_val.ends_with('"') {
                    arg_val = arg_val[1..arg_val.len() - 1].to_string();
                }
                args.push(arg_val);
            }
            Some(Statement::Simulate(SimulateStmt {
                cmd,
                args,
                numeric_expressions,
            }))
        }
        _ => None,
    })
}

fn parse_bounded_statement(
    pair: pest::iterators::Pair<Rule>,
    work: &mut usize,
) -> Result<Option<Statement>, pest::error::Error<Rule>> {
    let span = pair.as_span();
    let statement = parse_statement(pair)?;
    if let Some(statement) = &statement {
        *work += statement.expression_nodes();
        if *work > crate::expression::MAX_EXPRESSION_WORK {
            return Err(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "Source expression work limit exceeded".into(),
                },
                span,
            ));
        }
    }
    Ok(statement)
}
