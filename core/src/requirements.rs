use crate::ast::Statement;
use crate::ir::{Assertion, ast_to_ir};
use crate::parser::parse_program;
use pest::error::LineColLocation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const REQUIREMENTS_SCHEMA_VERSION: &str = "kessetsu.requirements.v1";
pub const MAX_REQUIREMENTS_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequirementSet {
    pub schema_version: String,
    pub sha256: String,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementCompileError {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

pub fn compile_requirements(bytes: &[u8]) -> Result<RequirementSet, RequirementCompileError> {
    if bytes.len() > MAX_REQUIREMENTS_BYTES {
        return Err(error(
            "KES-R001",
            format!("requirements input exceeds the {MAX_REQUIREMENTS_BYTES} byte limit"),
            None,
            None,
        ));
    }
    let source = std::str::from_utf8(bytes).map_err(|utf8_error| {
        error(
            "KES-R001",
            format!("requirements input must be UTF-8: {utf8_error}"),
            None,
            None,
        )
    })?;
    let program = parse_program(source).map_err(|parse_error| {
        let (line, column) = match parse_error.line_col {
            LineColLocation::Pos((line, column)) | LineColLocation::Span((line, column), _) => {
                (Some(line), Some(column))
            }
        };
        error("KES-R001", parse_error.to_string(), line, column)
    })?;

    let assertion_only = program.modules.is_empty()
        && program.model_includes.is_empty()
        && program.models.is_empty()
        && program.subcircuits.is_empty()
        && program.external_subcircuits.is_empty()
        && program
            .statements
            .iter()
            .all(|statement| matches!(statement, Statement::Assert(_)));
    if !assertion_only {
        let (line, column) = first_non_assertion_location(source);
        return Err(error(
            "KES-R001",
            "requirements files may contain only comments and assert statements",
            line,
            column,
        ));
    }
    if program.statements.is_empty() {
        return Err(error(
            "KES-R002",
            "requirements file defines no assertions",
            None,
            None,
        ));
    }

    if program.statements.iter().any(|statement| matches!(statement, Statement::Assert(assertion) if assertion.threshold_expression.is_some() || !assertion.numeric_expressions.is_empty())) {
        return Err(error("KES-R001", "independent requirements use literal numeric fields; circuit parameter expressions are not allowed", None, None));
    }

    let circuit = ast_to_ir(&program).map_err(|diagnostic| {
        let (line, column) = locate_semantic_error(source, &diagnostic.message);
        error(&diagnostic.code, diagnostic.message, line, column)
    })?;
    Ok(RequirementSet {
        schema_version: REQUIREMENTS_SCHEMA_VERSION.to_string(),
        sha256: format!("sha256:{:x}", Sha256::digest(bytes)),
        assertions: circuit.assertions,
    })
}

fn first_non_assertion_location(source: &str) -> (Option<usize>, Option<usize>) {
    source
        .lines()
        .enumerate()
        .find_map(|(line_index, line)| {
            let trimmed = line.trim_start();
            (!trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("assert "))
                .then(|| {
                    (
                        Some(line_index + 1),
                        Some(line.len().saturating_sub(trimmed.len()) + 1),
                    )
                })
        })
        .unwrap_or((None, None))
}

fn locate_semantic_error(source: &str, message: &str) -> (Option<usize>, Option<usize>) {
    for needle in message
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '\'' | '"' | ':' | ',' | '.')
        })
        .filter(|needle| !needle.is_empty())
    {
        for (line_index, line) in source.lines().enumerate() {
            if let Some(column) = line.to_ascii_lowercase().find(&needle.to_ascii_lowercase()) {
                return (Some(line_index + 1), Some(column + 1));
            }
        }
    }
    (None, None)
}

fn error(
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
    column: Option<usize>,
) -> RequirementCompileError {
    RequirementCompileError {
        code: code.to_string(),
        message: message.into(),
        line,
        column,
    }
}
