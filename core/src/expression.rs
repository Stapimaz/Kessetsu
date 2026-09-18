//! Bounded, pure engineering expressions. No SPICE, I/O or textual substitution.
use crate::ir::{Quantity, SIUnit, parse_value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXPRESSION_BYTES: usize = 16 * 1024;
pub const MAX_EXPRESSION_NODES: usize = 1024;
pub const MAX_EXPRESSION_DEPTH: usize = 64;
pub const MAX_PARAMETERS: usize = 4096;
pub const MAX_EXPRESSION_WORK: usize = 1_000_000;
pub const PARAMETER_SCHEMA_VERSION: &str = "kessetsu.parameters.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expression {
    pub source: String,
    root: Node,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Node {
    Literal(String),
    Name(String),
    Unary {
        negative: bool,
        operand: Box<Node>,
    },
    Binary {
        operator: Operator,
        left: Box<Node>,
        right: Box<Node>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterDecl {
    pub name: String,
    pub unit: SIUnit,
    pub expression: Expression,
    pub line: usize,
    pub column: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_path: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterRecord {
    pub name: String,
    pub declared_unit: SIUnit,
    pub expression: String,
    pub dependencies: Vec<String>,
    pub resolved: Quantity,
    pub line: usize,
    pub column: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_path: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterManifest {
    pub schema_version: String,
    pub parameters: Vec<ParameterRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<ParameterBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub analysis_bindings: Vec<AnalysisParameterBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertion_bindings: Vec<AssertionParameterBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionParameterBinding {
    pub assertion_index: usize,
    pub field: String,
    pub expression: String,
    pub dependencies: Vec<String>,
    pub resolved: Quantity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisParameterBinding {
    pub analysis_index: usize,
    pub field: String,
    pub expression: String,
    pub dependencies: Vec<String>,
    pub resolved: Quantity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterBinding {
    pub component: String,
    pub field: String,
    pub expression: String,
    pub dependencies: Vec<String>,
    pub resolved: Quantity,
}

impl Default for ParameterManifest {
    fn default() -> Self {
        Self {
            schema_version: PARAMETER_SCHEMA_VERSION.into(),
            parameters: Vec::new(),
            bindings: Vec::new(),
            analysis_bindings: Vec::new(),
            assertion_bindings: Vec::new(),
        }
    }
}

impl ParameterManifest {
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
            && self.bindings.is_empty()
            && self.analysis_bindings.is_empty()
            && self.assertion_bindings.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionError {
    pub message: String,
    /// Zero-based byte offset within the original expression.
    pub offset: usize,
}

impl std::fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExpressionError {}

fn error(message: impl Into<String>, offset: usize) -> ExpressionError {
    ExpressionError {
        message: message.into(),
        offset,
    }
}

pub fn parameter_unit(text: &str) -> Option<SIUnit> {
    Some(match text {
        "Ohm" => SIUnit::Ohm,
        "F" => SIUnit::Farad,
        "H" => SIUnit::Henry,
        "V" => SIUnit::Volt,
        "A" => SIUnit::Ampere,
        "Hz" => SIUnit::Hertz,
        "s" => SIUnit::Second,
        "W" => SIUnit::Watt,
        "J" => SIUnit::Joule,
        "ratio" => SIUnit::Ratio,
        "percent" => SIUnit::Percent,
        "deg" => SIUnit::Degree,
        _ => return None,
    })
}

struct Reader<'a> {
    source: &'a str,
    position: usize,
    nodes: usize,
}

impl Reader<'_> {
    fn spaces(&mut self) {
        while matches!(
            self.source.as_bytes().get(self.position),
            Some(b' ' | b'\t')
        ) {
            self.position += 1;
        }
    }

    fn node(&mut self, node: Node, depth: usize) -> Result<(Node, usize), ExpressionError> {
        self.nodes += 1;
        if self.nodes > MAX_EXPRESSION_NODES || depth > MAX_EXPRESSION_DEPTH {
            return Err(error(
                "Expression exceeds the node or nesting limit",
                self.position,
            ));
        }
        Ok((node, depth))
    }

    fn binary(&mut self, nesting: usize, product: bool) -> Result<(Node, usize), ExpressionError> {
        let (mut left, mut depth) = if product {
            self.atom(nesting)?
        } else {
            self.binary(nesting, true)?
        };
        loop {
            self.spaces();
            let operator = match (product, self.source.as_bytes().get(self.position)) {
                (false, Some(b'+')) => Operator::Add,
                (false, Some(b'-')) => Operator::Subtract,
                (true, Some(b'*')) => Operator::Multiply,
                (true, Some(b'/')) => Operator::Divide,
                _ => break,
            };
            self.position += 1;
            let (right, right_depth) = if product {
                self.atom(nesting)?
            } else {
                self.binary(nesting, true)?
            };
            (left, depth) = self.node(
                Node::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                1 + depth.max(right_depth),
            )?;
        }
        Ok((left, depth))
    }

    fn atom(&mut self, nesting: usize) -> Result<(Node, usize), ExpressionError> {
        if nesting >= MAX_EXPRESSION_DEPTH {
            return Err(error("Expression exceeds the nesting limit", self.position));
        }
        self.spaces();
        let start = self.position;
        match self.source.as_bytes().get(start) {
            Some(b'+' | b'-') => {
                let negative = self.source.as_bytes()[start] == b'-';
                self.position += 1;
                let (operand, depth) = self.atom(nesting + 1)?;
                self.node(
                    Node::Unary {
                        negative,
                        operand: Box::new(operand),
                    },
                    depth + 1,
                )
            }
            Some(b'(') => {
                self.position += 1;
                let value = self.binary(nesting + 1, false)?;
                self.spaces();
                if self.source.as_bytes().get(self.position) != Some(&b')') {
                    return Err(error("Expected closing ')'", self.position));
                }
                self.position += 1;
                Ok(value)
            }
            Some(byte) if byte.is_ascii_alphabetic() => {
                self.position += 1;
                while self
                    .source
                    .as_bytes()
                    .get(self.position)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    self.position += 1;
                }
                self.node(Node::Name(self.source[start..self.position].into()), 1)
            }
            Some(byte) if byte.is_ascii_digit() || *byte == b'.' => {
                while self
                    .source
                    .as_bytes()
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
                if self.source.as_bytes().get(self.position) == Some(&b'.') {
                    self.position += 1;
                    while self
                        .source
                        .as_bytes()
                        .get(self.position)
                        .is_some_and(u8::is_ascii_digit)
                    {
                        self.position += 1;
                    }
                }
                if matches!(self.source.as_bytes().get(self.position), Some(b'e' | b'E')) {
                    self.position += 1;
                    if matches!(self.source.as_bytes().get(self.position), Some(b'+' | b'-')) {
                        self.position += 1;
                    }
                    while self
                        .source
                        .as_bytes()
                        .get(self.position)
                        .is_some_and(u8::is_ascii_digit)
                    {
                        self.position += 1;
                    }
                }
                while let Some(ch) = self.source[self.position..].chars().next() {
                    if !(ch.is_ascii_alphabetic() || matches!(ch, 'µ' | 'Ω' | '%')) {
                        break;
                    }
                    self.position += ch.len_utf8();
                }
                let text = &self.source[start..self.position];
                parse_value(text).map_err(|message| error(message, start))?;
                self.node(Node::Literal(text.into()), 1)
            }
            _ => Err(error("Expected a quantity, parameter name or '('", start)),
        }
    }
}

pub fn parse_expression(source: &str) -> Result<Expression, ExpressionError> {
    if source.len() > MAX_EXPRESSION_BYTES {
        return Err(error("Expression exceeds the byte limit", 0));
    }
    if let Some(offset) = source.find(['\n', '\r']) {
        return Err(error("Expressions must stay on one line", offset));
    }
    let mut reader = Reader {
        source,
        position: 0,
        nodes: 0,
    };
    let (root, _) = reader.binary(0, false)?;
    reader.spaces();
    if reader.position != source.len() {
        return Err(error(
            "Unexpected token; use explicit +, -, * or /",
            reader.position,
        ));
    }
    Ok(Expression {
        source: source.into(),
        root,
    })
}

/// Voltage, current, time and angle exponents; intermediate dimensions need no IR unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Dimension([i16; 4]);

fn dimension(unit: SIUnit) -> Dimension {
    Dimension(match unit {
        SIUnit::Volt => [1, 0, 0, 0],
        SIUnit::Ampere => [0, 1, 0, 0],
        SIUnit::Second => [0, 0, 1, 0],
        SIUnit::Hertz => [0, 0, -1, 0],
        SIUnit::Ohm => [1, -1, 0, 0],
        SIUnit::Farad => [-1, 1, 1, 0],
        SIUnit::Henry => [1, -1, 1, 0],
        SIUnit::Watt => [1, 1, 0, 0],
        SIUnit::Joule => [1, 1, 1, 0],
        SIUnit::Degree => [0, 0, 0, 1],
        SIUnit::Ratio | SIUnit::Percent => [0; 4],
    })
}

fn scale(unit: SIUnit) -> f64 {
    match unit {
        SIUnit::Percent => 0.01,
        SIUnit::Degree => std::f64::consts::PI / 180.0,
        _ => 1.0,
    }
}

fn describe(dimension_value: Dimension) -> String {
    for text in ["Ohm", "F", "H", "V", "A", "Hz", "s", "W", "ratio", "deg"] {
        if dimension(parameter_unit(text).unwrap()) == dimension_value {
            return text.into();
        }
    }
    format!(
        "compound dimension {:?} (V, A, s, angle)",
        dimension_value.0
    )
}

#[derive(Clone, Copy)]
struct Value {
    number: f64,
    dimension: Dimension,
}

fn range(value: f64, nonzero_expected: bool) -> Result<f64, ExpressionError> {
    if !value.is_finite() || (nonzero_expected && value == 0.0) {
        return Err(error(
            "Expression result is outside the supported numeric range",
            0,
        ));
    }
    Ok(value)
}

impl Value {
    fn from_quantity(quantity: &Quantity) -> Result<Self, ExpressionError> {
        Ok(Self {
            number: range(quantity.value * scale(quantity.unit), quantity.value != 0.0)?,
            dimension: dimension(quantity.unit),
        })
    }

    fn into_quantity(self, expected: SIUnit) -> Result<Quantity, ExpressionError> {
        if self.dimension != dimension(expected) {
            return Err(error(
                format!("Expected {expected:?}, got {}", describe(self.dimension)),
                0,
            ));
        }
        Ok(Quantity {
            value: range(self.number / scale(expected), self.number != 0.0)?,
            unit: expected,
        })
    }
}

impl Expression {
    /// Rebind identifiers in the typed tree, retaining the author's expression
    /// for provenance. This is not source substitution or a numeric round-trip.
    pub(crate) fn qualify(&self, prefix: &str) -> Self {
        fn visit(node: &mut Node, prefix: &str) {
            match node {
                Node::Name(name) if name != "pi" => *name = format!("{prefix}{name}"),
                Node::Unary { operand, .. } => visit(operand, prefix),
                Node::Binary { left, right, .. } => {
                    visit(left, prefix);
                    visit(right, prefix);
                }
                _ => {}
            }
        }
        let mut expression = self.clone();
        visit(&mut expression.root, prefix);
        expression
    }

    /// Check even overridden defaults against their lexical scope and declared
    /// unit, without evaluating discarded dependency cycles or dummy numbers.
    pub(crate) fn validate_units(
        &self,
        units: &BTreeMap<String, SIUnit>,
        expected: SIUnit,
    ) -> Result<(), ExpressionError> {
        fn visit(
            node: &Node,
            units: &BTreeMap<String, SIUnit>,
        ) -> Result<Dimension, ExpressionError> {
            match node {
                Node::Literal(text) => {
                    let (_, unit) = parse_value(text).map_err(|message| error(message, 0))?;
                    Ok(dimension(unit.unwrap_or(SIUnit::Ratio)))
                }
                Node::Name(name) if name == "pi" => Ok(dimension(SIUnit::Ratio)),
                Node::Name(name) => units
                    .get(name)
                    .copied()
                    .map(dimension)
                    .ok_or_else(|| error(format!("Unknown parameter '{name}'"), 0)),
                Node::Unary { operand, .. } => visit(operand, units),
                Node::Binary {
                    operator,
                    left,
                    right,
                } => {
                    let mut left = visit(left, units)?;
                    let right = visit(right, units)?;
                    match operator {
                        Operator::Add | Operator::Subtract if left != right => {
                            return Err(error("Cannot add/subtract incompatible dimensions", 0));
                        }
                        Operator::Multiply | Operator::Divide => {
                            for (a, b) in left.0.iter_mut().zip(right.0) {
                                *a = if *operator == Operator::Multiply {
                                    a.checked_add(b)
                                } else {
                                    a.checked_sub(b)
                                }
                                .ok_or_else(|| error("Dimension exponent limit exceeded", 0))?;
                            }
                        }
                        _ => {}
                    }
                    Ok(left)
                }
            }
        }
        fn whole_literal(node: &Node) -> bool {
            match node {
                Node::Literal(_) => true,
                Node::Unary { operand, .. } => whole_literal(operand),
                _ => false,
            }
        }
        if whole_literal(&self.root) {
            self.evaluate(&BTreeMap::new(), expected)?;
            return Ok(());
        }
        let actual = visit(&self.root, units)?;
        if actual != dimension(expected) {
            return Err(error(
                format!("Expected {expected:?}, got {}", describe(actual)),
                0,
            ));
        }
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        let mut count = 0;
        let mut stack = vec![&self.root];
        while let Some(node) = stack.pop() {
            count += 1;
            match node {
                Node::Unary { operand, .. } => stack.push(operand),
                Node::Binary { left, right, .. } => {
                    stack.push(left);
                    stack.push(right);
                }
                _ => {}
            }
        }
        count
    }

    pub fn dependencies(&self) -> BTreeSet<String> {
        let mut dependencies = BTreeSet::new();
        let mut stack = vec![&self.root];
        while let Some(node) = stack.pop() {
            match node {
                Node::Name(name) if name != "pi" => {
                    dependencies.insert(name.clone());
                }
                Node::Unary { operand, .. } => stack.push(operand),
                Node::Binary { left, right, .. } => {
                    stack.push(left);
                    stack.push(right);
                }
                _ => {}
            }
        }
        dependencies
    }

    pub fn evaluate(
        &self,
        values: &BTreeMap<String, Quantity>,
        expected: SIUnit,
    ) -> Result<Quantity, ExpressionError> {
        // Only a whole (possibly signed/parenthesized) literal receives contextual units.
        fn literal(node: &Node) -> Option<(&str, f64)> {
            match node {
                Node::Literal(text) => Some((text, 1.0)),
                Node::Unary { negative, operand } => literal(operand)
                    .map(|(text, sign)| (text, if *negative { -sign } else { sign })),
                _ => None,
            }
        }
        if let Some((text, sign)) = literal(&self.root) {
            let (number, unit) = parse_value(text).map_err(|message| error(message, 0))?;
            return Value::from_quantity(&Quantity {
                value: sign * number,
                unit: unit.unwrap_or(expected),
            })?
            .into_quantity(expected);
        }

        fn evaluate(
            node: &Node,
            values: &BTreeMap<String, Quantity>,
        ) -> Result<Value, ExpressionError> {
            match node {
                Node::Literal(text) => {
                    let (number, unit) = parse_value(text).map_err(|message| error(message, 0))?;
                    Value::from_quantity(&Quantity {
                        value: number,
                        unit: unit.unwrap_or(SIUnit::Ratio),
                    })
                }
                Node::Name(name) if name == "pi" => Value::from_quantity(&Quantity {
                    value: std::f64::consts::PI,
                    unit: SIUnit::Ratio,
                }),
                Node::Name(name) => Value::from_quantity(
                    values
                        .get(name)
                        .ok_or_else(|| error(format!("Unknown parameter '{name}'"), 0))?,
                ),
                Node::Unary { negative, operand } => {
                    let mut value = evaluate(operand, values)?;
                    if *negative {
                        value.number = -value.number;
                    }
                    Ok(value)
                }
                Node::Binary {
                    operator,
                    left,
                    right,
                } => {
                    let left = evaluate(left, values)?;
                    let right = evaluate(right, values)?;
                    let mut result_dimension = left.dimension;
                    let (number, nonzero_expected) = match operator {
                        Operator::Add | Operator::Subtract => {
                            if left.dimension != right.dimension {
                                return Err(error(
                                    format!(
                                        "Cannot add/subtract {} and {}; use explicitly compatible units",
                                        describe(left.dimension),
                                        describe(right.dimension)
                                    ),
                                    0,
                                ));
                            }
                            (
                                if *operator == Operator::Add {
                                    left.number + right.number
                                } else {
                                    left.number - right.number
                                },
                                false,
                            )
                        }
                        Operator::Multiply | Operator::Divide => {
                            if *operator == Operator::Divide && right.number == 0.0 {
                                return Err(error("Division by zero", 0));
                            }
                            for (index, exponent) in result_dimension.0.iter_mut().enumerate() {
                                *exponent = if *operator == Operator::Multiply {
                                    exponent.checked_add(right.dimension.0[index])
                                } else {
                                    exponent.checked_sub(right.dimension.0[index])
                                }
                                .ok_or_else(|| error("Dimension exponent limit exceeded", 0))?;
                            }
                            (
                                if *operator == Operator::Multiply {
                                    left.number * right.number
                                } else {
                                    left.number / right.number
                                },
                                left.number != 0.0 && right.number != 0.0,
                            )
                        }
                    };
                    Ok(Value {
                        number: range(number, nonzero_expected)?,
                        dimension: result_dimension,
                    })
                }
            }
        }
        evaluate(&self.root, values)?.into_quantity(expected)
    }
}

#[derive(Debug)]
pub struct ParameterError {
    pub code: &'static str,
    pub message: String,
    pub name: String,
    pub line: usize,
    pub column: usize,
}

pub fn resolve_parameters(
    declarations: &[&ParameterDecl],
) -> Result<(BTreeMap<String, Quantity>, ParameterManifest), ParameterError> {
    let failure = |decl: &ParameterDecl, code, message: String| ParameterError {
        code,
        message,
        name: decl.name.clone(),
        line: decl.line,
        column: decl.column,
    };
    let mut definitions = BTreeMap::new();
    let mut work = 0;
    for decl in declarations {
        work += decl.expression.node_count();
        if work > MAX_EXPRESSION_WORK {
            return Err(failure(
                decl,
                "KES-C020",
                "Parameter expression work limit exceeded".into(),
            ));
        }
        if definitions.len() >= MAX_PARAMETERS {
            return Err(failure(
                decl,
                "KES-C020",
                format!("Parameter count exceeds {MAX_PARAMETERS}"),
            ));
        }
        if decl.name == "pi" || definitions.insert(decl.name.as_str(), *decl).is_some() {
            return Err(failure(
                decl,
                "KES-C020",
                format!("Duplicate or reserved parameter '{}'", decl.name),
            ));
        }
    }
    let mut incoming = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut ready = BTreeSet::new();
    for (name, decl) in &definitions {
        let dependencies = decl.expression.dependencies();
        for dependency in &dependencies {
            let Some((dependency_name, _)) = definitions.get_key_value(dependency.as_str()) else {
                return Err(failure(
                    decl,
                    "KES-C021",
                    format!("Parameter '{name}' refers to unknown parameter '{dependency}'"),
                ));
            };
            dependents.entry(dependency_name).or_default().push(*name);
        }
        incoming.insert(*name, dependencies.len());
        if dependencies.is_empty() {
            ready.insert(*name);
        }
    }
    let mut values = BTreeMap::new();
    while let Some(name) = ready.pop_first() {
        let decl = definitions[name];
        let value = decl
            .expression
            .evaluate(&values, decl.unit)
            .map_err(|cause| failure(decl, "KES-C022", format!("Parameter '{name}': {cause}")))?;
        values.insert(name.to_string(), value);
        for dependent in dependents.get(name).into_iter().flatten() {
            let count = incoming.get_mut(dependent).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.insert(*dependent);
            }
        }
    }
    if values.len() != definitions.len() {
        let name = *definitions
            .keys()
            .find(|name| !values.contains_key(**name))
            .unwrap();
        let mut path = Vec::new();
        let mut current = name.to_string();
        let mut visited = BTreeMap::new();
        loop {
            if let Some(start) = visited.get(&current) {
                let mut cycle = path[*start..].to_vec();
                cycle.push(current);
                return Err(failure(
                    definitions[name],
                    "KES-C023",
                    format!("Parameter dependency cycle: {}", cycle.join(" -> ")),
                ));
            }
            visited.insert(current.clone(), path.len());
            path.push(current.clone());
            current = definitions[current.as_str()]
                .expression
                .dependencies()
                .into_iter()
                .find(|dependency| !values.contains_key(dependency))
                .unwrap();
        }
    }
    let manifest = ParameterManifest {
        schema_version: PARAMETER_SCHEMA_VERSION.into(),
        parameters: definitions
            .iter()
            .map(|(name, decl)| ParameterRecord {
                name: name.rsplit('.').next().unwrap().to_string(),
                declared_unit: decl.unit,
                expression: decl
                    .default_expression
                    .clone()
                    .unwrap_or_else(|| decl.expression.source.clone()),
                dependencies: decl.expression.dependencies().into_iter().collect(),
                resolved: values[*name].clone(),
                line: decl.line,
                column: decl.column,
                instance_path: decl.instance_path.clone(),
                effective_override: decl
                    .default_expression
                    .as_ref()
                    .map(|_| decl.expression.source.clone()),
            })
            .collect(),
        bindings: Vec::new(),
        analysis_bindings: Vec::new(),
        assertion_bindings: Vec::new(),
    };
    Ok((values, manifest))
}
