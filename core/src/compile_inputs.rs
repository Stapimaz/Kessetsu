//! Pure root parameter inputs and exact-span source materialization.
use crate::ast::{Program, Statement};
use crate::compiler::{Diagnostic, DiagnosticSeverity, DiagnosticStage};
use crate::expression::{
    MAX_EXPRESSION_BYTES, MAX_EXPRESSION_WORK, MAX_PARAMETERS, parse_expression,
};
use crate::ir::parse_value;
use crate::parser::{KessetsuParser, Rule};
use pest::Parser;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const INPUT_SCHEMA_VERSION: &str = "kessetsu.inputs.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterInput {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileInputs {
    pub schema_version: String,
    #[serde(default)]
    pub parameters: Vec<ParameterInput>,
}

impl Default for CompileInputs {
    fn default() -> Self {
        Self {
            schema_version: INPUT_SCHEMA_VERSION.into(),
            parameters: Vec::new(),
        }
    }
}

fn failure(name: Option<&str>, message: impl Into<String>) -> Box<Diagnostic> {
    Box::new(Diagnostic {
        code: "KES-C022".into(),
        severity: DiagnosticSeverity::Error,
        stage: DiagnosticStage::Semantic,
        message: message.into(),
        component: None,
        pin: None,
        field: name.map(str::to_owned),
        line: None,
        column: None,
    })
}

pub(crate) fn validate_inputs(inputs: &CompileInputs) -> Result<(), Box<Diagnostic>> {
    if inputs.schema_version != INPUT_SCHEMA_VERSION {
        return Err(failure(
            None,
            format!(
                "Unsupported compile input schema '{}'; expected '{INPUT_SCHEMA_VERSION}'",
                inputs.schema_version
            ),
        ));
    }
    if inputs.parameters.len() > MAX_PARAMETERS {
        return Err(failure(None, "Root parameter input count limit exceeded"));
    }
    Ok(())
}

/// Bind typed expression nodes, never compile materialized text as the effective pipeline.
pub(crate) fn apply(
    source: &str,
    program: &mut Program,
    inputs: &CompileInputs,
) -> Result<Option<String>, Box<Diagnostic>> {
    if inputs.parameters.is_empty() {
        return Ok(None);
    }
    let mut units = BTreeMap::new();
    let mut parameters = BTreeMap::new();
    for (index, statement) in program.statements.iter().enumerate() {
        if let Statement::Param(parameter) = statement {
            if units.len() >= MAX_PARAMETERS
                || parameter.name == "pi"
                || units
                    .insert(parameter.name.clone(), parameter.unit)
                    .is_some()
            {
                return Err(failure(
                    Some(&parameter.name),
                    "Duplicate/reserved root parameter or parameter count limit exceeded",
                ));
            }
            parameters.insert(parameter.name.as_str(), index);
        }
    }
    // Even discarded defaults must retain valid lexical names and dimensions.
    let mut work = 0;
    for statement in &program.statements {
        if let Statement::Param(parameter) = statement {
            work += parameter.expression.node_count();
            if work > MAX_EXPRESSION_WORK {
                return Err(failure(
                    Some(&parameter.name),
                    "Root default expression work limit exceeded",
                ));
            }
            parameter
                .expression
                .validate_units(&units, parameter.unit)
                .map_err(|error| {
                    failure(
                        Some(&parameter.name),
                        format!("Root parameter '{}': {error}", parameter.name),
                    )
                })?;
        }
    }
    let mut replacements = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut effective = Vec::new();
    for input in &inputs.parameters {
        let Some(index) = parameters.get(input.name.as_str()).copied() else {
            return Err(failure(
                Some(&input.name),
                format!(
                    "Unknown root parameter '{}'; module parameters require an explicit use override",
                    input.name
                ),
            ));
        };
        if !seen.insert(input.name.as_str()) {
            return Err(failure(
                Some(&input.name),
                format!("Duplicate root parameter input '{}'", input.name),
            ));
        }
        let literal = input.value.trim();
        if literal.len() > MAX_EXPRESSION_BYTES || literal.contains(['\n', '\r']) {
            return Err(failure(
                Some(&input.name),
                "Root parameter inputs require a bounded single numeric literal",
            ));
        }
        parse_value(literal).map_err(|error| {
            failure(
                Some(&input.name),
                format!("Root parameter inputs accept numeric literals only: {error}"),
            )
        })?;
        let expression = parse_expression(literal)
            .map_err(|error| failure(Some(&input.name), error.to_string()))?;
        let Statement::Param(parameter) = &program.statements[index] else {
            unreachable!()
        };
        expression
            .evaluate(&BTreeMap::new(), parameter.unit)
            .map_err(|error| {
                failure(
                    Some(&input.name),
                    format!("Root parameter '{}': {error}", input.name),
                )
            })?;
        replacements.insert(input.name.as_str(), literal);
        effective.push((index, expression));
    }
    // The accepted parser's byte spans preserve all unrelated bytes, including comments,
    // Unicode units, module definitions, line endings and dependent formulas.
    let mut edits = Vec::new();
    let pairs = KessetsuParser::parse(Rule::program, source)
        .map_err(|error| failure(None, error.to_string()))?;
    for pair in pairs {
        for top_level in pair
            .into_inner()
            .filter(|pair| pair.as_rule() == Rule::top_level)
        {
            let inner = top_level.into_inner().next().unwrap();
            if inner.as_rule() != Rule::statement {
                continue;
            }
            let statement = inner.into_inner().next().unwrap();
            if statement.as_rule() != Rule::param_decl {
                continue;
            }
            let mut fields = statement.into_inner();
            let name = fields.next().unwrap();
            fields.next(); // declared type
            let expression = fields.next().unwrap();
            if let Some(value) = replacements.get(name.as_str()) {
                let start = expression.as_span().start();
                let end = start + expression.as_str().trim_end().len();
                edits.push((start, end, *value));
            }
        }
    }
    let mut materialized = source.to_owned();
    for (start, end, value) in edits.into_iter().rev() {
        materialized.replace_range(start..end, value);
    }
    // All validation completes before any AST mutation.
    drop(parameters);
    for (index, expression) in effective {
        let Statement::Param(parameter) = &mut program.statements[index] else {
            unreachable!()
        };
        parameter.default_expression = Some(parameter.expression.source.clone());
        parameter.expression = expression;
    }
    Ok(Some(materialized))
}
