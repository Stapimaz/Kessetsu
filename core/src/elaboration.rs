//! Lexical instance elaboration. Bind typed expression identifiers before IR
//! evaluation; retain legacy electrical IDs without merging distinct paths.
use crate::ast::*;
use crate::expression::{MAX_EXPRESSION_WORK, MAX_PARAMETERS};
use crate::ir::SIUnit;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(crate) struct ElaborationError {
    pub message: String,
    pub field: Option<String>,
    pub location: Option<(usize, usize)>,
}

impl From<String> for ElaborationError {
    fn from(message: String) -> Self {
        Self {
            message,
            field: None,
            location: None,
        }
    }
}
impl From<&str> for ElaborationError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}
impl ElaborationError {
    fn at(message: String, field: String, line: usize, column: usize) -> Self {
        Self {
            message,
            field: Some(field),
            location: Some((line, column)),
        }
    }
}

pub(crate) struct ElaboratedProgram {
    pub program: Program,
    pub parameter_locations: BTreeMap<String, ParameterLocation>,
}

pub(crate) struct ParameterLocation {
    pub primary: (usize, usize),
    pub duplicate: Option<(usize, usize)>,
}

fn declarations(statements: &[Statement]) -> BTreeMap<String, SIUnit> {
    statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Param(parameter) => Some((parameter.name.clone(), parameter.unit)),
            _ => None,
        })
        .collect()
}

struct Scope {
    path: Vec<String>,
    units: BTreeMap<String, SIUnit>,
    ports: BTreeSet<String>,
    nets: BTreeSet<String>,
    parameter_locations: BTreeMap<String, (usize, usize)>,
}

impl Scope {
    fn prefix(&self) -> String {
        if self.path.is_empty() {
            String::new()
        } else {
            format!("{}_", self.path.join("_"))
        }
    }
    fn parameter_prefix(&self) -> String {
        if self.path.is_empty() {
            String::new()
        } else {
            format!("{}.", self.path.join("."))
        }
    }
    fn label(&self) -> String {
        if self.path.is_empty() {
            "root".into()
        } else {
            self.path.join(".")
        }
    }
}

struct Elaborator<'a> {
    modules: BTreeMap<&'a str, &'a ModuleDef>,
    output: Vec<Statement>,
    stack: Vec<String>,
    identities: BTreeMap<String, Vec<String>>,
    work: usize,
    parameter_locations: BTreeMap<String, ParameterLocation>,
    /// Maps qualified module parameter name -> (origin parameter prefix, unqualified source).
    module_params: BTreeMap<String, (String, String)>,
}

impl Elaborator<'_> {
    fn emit(&mut self, statement: Statement, scope: &Scope) -> Result<(), ElaborationError> {
        if self.output.len() >= crate::parser::MAX_SOURCE_STATEMENTS {
            return Err("Module expansion exceeds the 100000 statement limit".into());
        }
        self.work += statement.expression_nodes();
        if self.work > MAX_EXPRESSION_WORK {
            return Err("Module expression work limit exceeded".into());
        }
        let name = match &statement {
            Statement::Decl(decl) => Some(&decl.name),
            Statement::Net(net) => Some(&net.name),
            Statement::Couple(couple) => Some(&couple.name),
            _ => None,
        };
        if let Some(name) = name {
            if let Some(previous) = self.identities.get(name) {
                if previous != &scope.path {
                    return Err(format!(
                        "Flattened identifier collision '{name}' between instance paths {:?} and {:?}",
                        previous, scope.path
                    ).into());
                }
            } else {
                self.identities.insert(name.clone(), scope.path.clone());
            }
        }
        self.output.push(statement);
        Ok(())
    }

    fn expand_module_initial_expr(
        &self,
        origin_prefix: &str,
        expr: &str,
        initial: &InitialStmt,
        depth: usize,
        active: &mut BTreeSet<String>,
    ) -> Result<String, ElaborationError> {
        if depth > crate::expression::MAX_EXPRESSION_DEPTH {
            return Err(ElaborationError::at(
                format!(
                    "Initial condition for net '{}' exceeds maximum parameter expansion depth",
                    initial.net
                ),
                initial.net.clone(),
                initial.line,
                initial.column,
            ));
        }
        let mut out = String::with_capacity(expr.len());
        let bytes = expr.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_alphabetic() {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &expr[start..i];
                if !origin_prefix.is_empty() {
                    let qual_key = format!("{origin_prefix}{ident}");
                    if let Some((target_origin, target_src)) =
                        self.module_params.get(&qual_key).cloned()
                    {
                        if !active.insert(qual_key.clone()) {
                            return Err(ElaborationError::at(
                                format!(
                                    "Cycle detected in module parameter '{}' referenced by initial V({})",
                                    ident, initial.net
                                ),
                                initial.net.clone(),
                                initial.line,
                                initial.column,
                            ));
                        }
                        let expanded = self.expand_module_initial_expr(
                            &target_origin,
                            &target_src,
                            initial,
                            depth + 1,
                            active,
                        )?;
                        active.remove(&qual_key);
                        out.push('(');
                        out.push_str(&expanded);
                        out.push(')');
                        if out.len() > crate::expression::MAX_EXPRESSION_BYTES {
                            return Err(ElaborationError::at(
                                format!(
                                    "Expanded initial condition expression for net '{}' exceeds byte limit",
                                    initial.net
                                ),
                                initial.net.clone(),
                                initial.line,
                                initial.column,
                            ));
                        }
                        continue;
                    }
                }
                out.push_str(ident);
            } else if b.is_ascii_digit() || b == b'.' {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'.' {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
                    i += 1;
                    if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
                        i += 1;
                    }
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                while let Some(ch) = expr[i..].chars().next() {
                    if !(ch.is_ascii_alphabetic() || matches!(ch, 'µ' | 'Ω' | '%')) {
                        break;
                    }
                    i += ch.len_utf8();
                }
                out.push_str(&expr[start..i]);
            } else {
                let ch = expr[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
            if out.len() > crate::expression::MAX_EXPRESSION_BYTES {
                return Err(ElaborationError::at(
                    format!(
                        "Expanded initial condition expression for net '{}' exceeds byte limit",
                        initial.net
                    ),
                    initial.net.clone(),
                    initial.line,
                    initial.column,
                ));
            }
        }
        Ok(out)
    }

    fn expand(&mut self, statements: &[Statement], scope: &Scope) -> Result<(), ElaborationError> {
        let prefix = scope.prefix();
        let parameter_prefix = scope.parameter_prefix();
        for statement in statements {
            match statement {
                Statement::Param(parameter) => {
                    let location = scope
                        .parameter_locations
                        .get(&parameter.name)
                        .copied()
                        .unwrap_or((parameter.line, parameter.column));
                    let mut parameter = parameter.clone();
                    parameter.name = format!("{parameter_prefix}{}", parameter.name);
                    parameter.instance_path = scope.path.clone();
                    self.parameter_locations
                        .entry(parameter.name.clone())
                        .and_modify(|origin| {
                            origin.duplicate.get_or_insert(location);
                        })
                        .or_insert(ParameterLocation {
                            primary: location,
                            duplicate: None,
                        });
                    // Overrides were already qualified in their caller's scope.
                    if parameter.default_expression.is_none() {
                        parameter.expression = parameter.expression.qualify(&parameter_prefix);
                    }
                    self.emit(Statement::Param(parameter), scope)?;
                }
                Statement::Decl(decl) => {
                    let mut decl = decl.clone();
                    decl.name = format!("{prefix}{}", decl.name);
                    decl.instance_path = scope.path.clone();
                    if let Some(expression) = &decl.value_expression {
                        decl.value_expression = Some(expression.qualify(&parameter_prefix));
                    }
                    if let Some(call) = &mut decl.waveform_expression {
                        for arg in &mut call.args {
                            if let Some(expression) = &arg.expression {
                                arg.expression = Some(expression.qualify(&parameter_prefix));
                            }
                        }
                    }
                    for parameter in &mut decl.instance_parameters {
                        parameter.expression = parameter.expression.qualify(&parameter_prefix);
                    }
                    self.emit(Statement::Decl(decl), scope)?;
                }
                Statement::Net(net) => self.emit(
                    Statement::Net(NetDecl {
                        name: format!("{prefix}{}", net.name),
                    }),
                    scope,
                )?,
                Statement::Part(part) => {
                    let mut part = part.clone();
                    part.component = format!("{prefix}{}", part.component);
                    self.emit(Statement::Part(part), scope)?;
                }
                Statement::Connect(connection) => {
                    let mut pins = Vec::new();
                    for pin in &connection.pins {
                        pins.push(if !pin.component.is_empty() {
                            PinRef {
                                component: format!("{prefix}{}", pin.component),
                                pin: pin.pin.clone(),
                            }
                        } else if scope.path.is_empty() {
                            pin.clone()
                        } else if scope.ports.contains(&pin.pin) {
                            PinRef {
                                component: scope.path.join("_"),
                                pin: pin.pin.clone(),
                            }
                        } else if scope.nets.contains(&pin.pin) {
                            PinRef {
                                component: String::new(),
                                pin: format!("{prefix}{}", pin.pin),
                            }
                        } else {
                            return Err(format!(
                                "Instance '{}': undeclared module port or net '{}'",
                                scope.label(),
                                pin.pin
                            )
                            .into());
                        });
                    }
                    self.emit(Statement::Connect(Connection { pins }), scope)?;
                }
                Statement::Couple(couple) => {
                    let mut couple = couple.clone();
                    couple.name = format!("{prefix}{}", couple.name);
                    couple.inductors = couple
                        .inductors
                        .into_iter()
                        .map(|ind| format!("{prefix}{ind}"))
                        .collect();
                    if let Some(expression) = &couple.factor_expression {
                        couple.factor_expression = Some(expression.qualify(&parameter_prefix));
                    }
                    self.emit(Statement::Couple(couple), scope)?;
                }
                Statement::Use(instance) => {
                    let module = *self
                        .modules
                        .get(instance.module_name.as_str())
                        .ok_or_else(|| format!("Module not found: {}", instance.module_name))?;
                    if self.stack.contains(&instance.module_name) || self.stack.len() >= 64 {
                        return Err(format!(
                            "Recursive module or module nesting limit exceeded: {}",
                            instance.module_name
                        )
                        .into());
                    }
                    let mut path = scope.path.clone();
                    path.push(instance.inst_name.clone());
                    let units = declarations(&module.statements);
                    let mut child = Scope {
                        path,
                        units,
                        ports: module.pins.iter().cloned().collect(),
                        nets: module
                            .statements
                            .iter()
                            .filter_map(|s| match s {
                                Statement::Net(net) => Some(net.name.clone()),
                                _ => None,
                            })
                            .collect(),
                        parameter_locations: BTreeMap::new(),
                    };
                    let mut overrides = BTreeMap::new();
                    let mut raw_override_sources = BTreeMap::new();
                    for value in &instance.overrides {
                        self.work += value.expression.node_count();
                        if self.work > MAX_EXPRESSION_WORK {
                            return Err(ElaborationError::at(
                                "Module expression work limit exceeded".into(),
                                value.name.clone(),
                                value.line,
                                value.column,
                            ));
                        }
                        let unit = child.units.get(&value.name).ok_or_else(|| {
                            ElaborationError::at(
                                format!(
                                    "Instance '{}': unknown override '{}' at {}:{}",
                                    child.label(),
                                    value.name,
                                    value.line,
                                    value.column
                                ),
                                format!("{}.{}", child.label(), value.name),
                                value.line,
                                value.column,
                            )
                        })?;
                        if overrides
                            .insert(
                                value.name.clone(),
                                value.expression.qualify(&parameter_prefix),
                            )
                            .is_some()
                        {
                            return Err(ElaborationError::at(
                                format!(
                                    "Instance '{}': duplicate override '{}'",
                                    child.label(),
                                    value.name
                                ),
                                format!("{}.{}", child.label(), value.name),
                                value.line,
                                value.column,
                            ));
                        }
                        raw_override_sources
                            .insert(value.name.clone(), value.expression.source.clone());
                        value
                            .expression
                            .validate_units(&scope.units, *unit)
                            .map_err(|error| {
                                ElaborationError::at(
                                    format!(
                                        "Instance '{}', override '{}' at {}:{}: {error}",
                                        child.label(),
                                        value.name,
                                        value.line,
                                        value.column
                                    ),
                                    format!("{}.{}", child.label(), value.name),
                                    value.line,
                                    value.column,
                                )
                            })?;
                        child
                            .parameter_locations
                            .insert(value.name.clone(), (value.line, value.column));
                    }
                    if self.output.len().saturating_add(module.statements.len())
                        > crate::parser::MAX_SOURCE_STATEMENTS
                    {
                        return Err("Module expansion exceeds the 100000 statement limit".into());
                    }
                    let mut effective = module.statements.clone();
                    let child_param_prefix = child.parameter_prefix();
                    for statement in &mut effective {
                        if let Statement::Param(parameter) = statement {
                            if let Some(expression) = overrides.remove(&parameter.name) {
                                let raw_source = raw_override_sources
                                    .remove(&parameter.name)
                                    .unwrap_or_else(|| expression.source.clone());
                                self.module_params.insert(
                                    format!("{child_param_prefix}{}", parameter.name),
                                    (parameter_prefix.clone(), raw_source),
                                );
                                parameter.default_expression =
                                    Some(parameter.expression.source.clone());
                                parameter.expression = expression;
                            } else {
                                self.module_params.insert(
                                    format!("{child_param_prefix}{}", parameter.name),
                                    (
                                        child_param_prefix.clone(),
                                        parameter.expression.source.clone(),
                                    ),
                                );
                            }
                        }
                    }
                    self.emit(
                        Statement::Decl(ComponentDecl {
                            comp_type: ComponentType::ModulePort,
                            name: format!("{prefix}{}", instance.inst_name),
                            subtype: None,
                            value: Some(instance.module_name.clone()),
                            value_expression: None,
                            waveform_expression: None,
                            instance_parameters: Vec::new(),
                            interface_pins: module.pins.clone(),
                            instance_path: child.path.clone(),
                        }),
                        scope,
                    )?;
                    self.stack.push(instance.module_name.clone());
                    self.expand(&effective, &child)?;
                    self.stack.pop();
                }
                // Module numeric assertion/analysis expressions and target
                // qualification are a separate follow-on context slice.
                Statement::Assert(_) | Statement::Simulate(_)
                    if !scope.path.is_empty() && !scope.units.is_empty() =>
                {
                    return Err(format!(
                        "Instance '{}': put analyses and assertions at the circuit root; parameterized module analysis/assertion contexts are not supported yet",
                        scope.label()
                    ).into());
                }
                Statement::Simulate(analysis) => {
                    let mut analysis = analysis.clone();
                    for argument in &mut analysis.numeric_expressions {
                        let expression = &mut argument.expression;
                        *expression = expression.qualify(&parameter_prefix);
                    }
                    self.emit(Statement::Simulate(analysis), scope)?;
                }
                Statement::Assert(assertion)
                    if !scope.path.is_empty()
                        && (assertion.threshold_expression.is_some()
                            || !assertion.numeric_expressions.is_empty()) =>
                {
                    return Err(format!(
                        "Instance '{}': put parameterized assertions at the circuit root; module assertion target qualification is not supported yet",
                        scope.label()
                    ).into());
                }
                Statement::Initial(initial) => {
                    if scope.path.is_empty() {
                        self.emit(Statement::Initial(initial.clone()), scope)?;
                    } else {
                        let net = if initial.net == "0" || initial.net.eq_ignore_ascii_case("gnd") {
                            initial.net.clone()
                        } else if let Some(port) = scope
                            .ports
                            .iter()
                            .find(|port| port.eq_ignore_ascii_case(&initial.net))
                        {
                            return Err(ElaborationError::at(
                                format!(
                                    "Instance '{}': initial condition cannot target module port '{}'; set initial conditions on a declared internal net or connected outer net instead",
                                    scope.label(),
                                    port
                                ),
                                initial.net.clone(),
                                initial.line,
                                initial.column,
                            ));
                        } else if let Some(declared_net) = scope
                            .nets
                            .iter()
                            .find(|declared| declared.eq_ignore_ascii_case(&initial.net))
                        {
                            format!("{prefix}{declared_net}")
                        } else {
                            format!("{prefix}{}", initial.net)
                        };
                        let voltage = if let Some(inner) = initial
                            .voltage
                            .strip_prefix('{')
                            .and_then(|s| s.strip_suffix('}'))
                        {
                            let parsed = crate::expression::parse_expression(inner).map_err(|cause| {
                                ElaborationError::at(
                                    format!(
                                        "Instance '{}', initial V({}) at {}:{}: {} (expression byte {})",
                                        scope.label(),
                                        initial.net,
                                        initial.line,
                                        initial.column,
                                        cause.message,
                                        cause.offset + 1
                                    ),
                                    initial.net.clone(),
                                    initial.line,
                                    initial.column,
                                )
                            })?;
                            parsed
                                .validate_units(&scope.units, SIUnit::Volt)
                                .map_err(|error| {
                                    ElaborationError::at(
                                        format!(
                                            "Instance '{}', initial V({}) at {}:{}: {error}",
                                            scope.label(),
                                            initial.net,
                                            initial.line,
                                            initial.column
                                        ),
                                        initial.net.clone(),
                                        initial.line,
                                        initial.column,
                                    )
                                })?;
                            let expanded = self.expand_module_initial_expr(
                                &parameter_prefix,
                                inner,
                                initial,
                                0,
                                &mut BTreeSet::new(),
                            )?;
                            crate::expression::parse_expression(&expanded).map_err(|cause| {
                                ElaborationError::at(
                                    format!(
                                        "Instance '{}', initial V({}) at {}:{}: {} (expression byte {})",
                                        scope.label(),
                                        initial.net,
                                        initial.line,
                                        initial.column,
                                        cause.message,
                                        cause.offset + 1
                                    ),
                                    initial.net.clone(),
                                    initial.line,
                                    initial.column,
                                )
                            })?;
                            format!("{{{expanded}}}")
                        } else {
                            initial.voltage.clone()
                        };
                        self.emit(
                            Statement::Initial(InitialStmt {
                                net,
                                voltage,
                                line: initial.line,
                                column: initial.column,
                            }),
                            scope,
                        )?;
                    }
                }
                _ => self.emit(statement.clone(), scope)?,
            }
        }
        Ok(())
    }
}

pub(crate) fn flatten(program: &Program) -> Result<ElaboratedProgram, ElaborationError> {
    let mut elaborator = Elaborator {
        modules: BTreeMap::new(),
        output: Vec::new(),
        stack: Vec::new(),
        identities: BTreeMap::new(),
        work: 0,
        parameter_locations: BTreeMap::new(),
        module_params: BTreeMap::new(),
    };
    for module in &program.modules {
        if elaborator.modules.insert(&module.name, module).is_some() {
            return Err(format!("Duplicate module definition: {}", module.name).into());
        }
        if module.pins.iter().collect::<BTreeSet<_>>().len() != module.pins.len() {
            return Err(format!("Module '{}': duplicate interface port", module.name).into());
        }
        let units = declarations(&module.statements);
        let count = module
            .statements
            .iter()
            .filter(|s| matches!(s, Statement::Param(_)))
            .count();
        if count != units.len() || units.contains_key("pi") || count > MAX_PARAMETERS {
            let mut seen = BTreeSet::new();
            let parameter = module
                .statements
                .iter()
                .filter_map(|statement| {
                    if let Statement::Param(parameter) = statement {
                        Some(parameter)
                    } else {
                        None
                    }
                })
                .find(|parameter| {
                    parameter.name == "pi"
                        || !seen.insert(parameter.name.as_str())
                        || seen.len() > MAX_PARAMETERS
                })
                .unwrap();
            return Err(ElaborationError::at(
                format!(
                    "Module '{}': duplicate/reserved parameter or parameter count limit exceeded",
                    module.name
                ),
                parameter.name.clone(),
                parameter.line,
                parameter.column,
            ));
        }
        for statement in &module.statements {
            if let Statement::Param(parameter) = statement {
                parameter
                    .expression
                    .validate_units(&units, parameter.unit)
                    .map_err(|error| {
                        ElaborationError::at(
                            format!(
                                "Module '{}', parameter '{}' at {}:{}: {error}",
                                module.name, parameter.name, parameter.line, parameter.column
                            ),
                            parameter.name.clone(),
                            parameter.line,
                            parameter.column,
                        )
                    })?;
                elaborator.work += parameter.expression.node_count();
                if elaborator.work > MAX_EXPRESSION_WORK {
                    return Err("Module expression work limit exceeded".into());
                }
            }
        }
    }
    let root = Scope {
        path: Vec::new(),
        units: declarations(&program.statements),
        ports: BTreeSet::new(),
        nets: BTreeSet::new(),
        parameter_locations: BTreeMap::new(),
    };
    elaborator.expand(&program.statements, &root)?;
    Ok(ElaboratedProgram {
        program: Program {
            modules: Vec::new(),
            model_includes: program.model_includes.clone(),
            models: program.models.clone(),
            subcircuits: program.subcircuits.clone(),
            external_subcircuits: program.external_subcircuits.clone(),
            statements: elaborator.output,
        },
        parameter_locations: elaborator.parameter_locations,
    })
}
