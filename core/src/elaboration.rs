//! Lexical instance elaboration. Bind typed expression identifiers before IR
//! evaluation; retain legacy electrical IDs without merging distinct paths.
use crate::ast::*;
use crate::expression::{MAX_EXPRESSION_WORK, MAX_PARAMETERS};
use crate::ir::SIUnit;
use std::collections::{BTreeMap, BTreeSet};

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
}

impl Elaborator<'_> {
    fn emit(&mut self, statement: Statement, scope: &Scope) -> Result<(), String> {
        if self.output.len() >= 100_000 {
            return Err("Module expansion exceeds the 100000 statement limit".into());
        }
        let name = match &statement {
            Statement::Decl(decl) => Some(&decl.name),
            Statement::Net(net) => Some(&net.name),
            _ => None,
        };
        if let Some(name) = name {
            if let Some(previous) = self.identities.get(name) {
                if previous != &scope.path {
                    return Err(format!(
                        "Flattened identifier collision '{name}' between instance paths {:?} and {:?}",
                        previous, scope.path
                    ));
                }
            } else {
                self.identities.insert(name.clone(), scope.path.clone());
            }
        }
        self.output.push(statement);
        Ok(())
    }

    fn expand(&mut self, statements: &[Statement], scope: &Scope) -> Result<(), String> {
        let prefix = scope.prefix();
        let parameter_prefix = scope.parameter_prefix();
        for statement in statements {
            match statement {
                Statement::Param(parameter) => {
                    let mut parameter = parameter.clone();
                    parameter.name = format!("{parameter_prefix}{}", parameter.name);
                    parameter.instance_path = scope.path.clone();
                    // Overrides were already qualified in their caller's scope.
                    if parameter.default_expression.is_none() {
                        parameter.expression = parameter.expression.qualify(&parameter_prefix);
                    }
                    self.work += parameter.expression.node_count();
                    if self.work > MAX_EXPRESSION_WORK {
                        return Err("Module expression work limit exceeded".into());
                    }
                    self.emit(Statement::Param(parameter), scope)?;
                }
                Statement::Decl(decl) => {
                    let mut decl = decl.clone();
                    decl.name = format!("{prefix}{}", decl.name);
                    if let Some(expression) = &decl.value_expression {
                        self.work += expression.node_count();
                        if self.work > MAX_EXPRESSION_WORK {
                            return Err("Module expression work limit exceeded".into());
                        }
                        decl.value_expression = Some(expression.qualify(&parameter_prefix));
                    }
                    self.emit(Statement::Decl(decl), scope)?;
                }
                Statement::Net(net) => self.emit(
                    Statement::Net(NetDecl {
                        name: format!("{prefix}{}", net.name),
                    }),
                    scope,
                )?,
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
                            ));
                        });
                    }
                    self.emit(Statement::Connect(Connection { pins }), scope)?;
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
                        ));
                    }
                    let mut path = scope.path.clone();
                    path.push(instance.inst_name.clone());
                    let units = declarations(&module.statements);
                    let child = Scope {
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
                    };
                    let mut overrides = BTreeMap::new();
                    for value in &instance.overrides {
                        let unit = child.units.get(&value.name).ok_or_else(|| {
                            format!(
                                "Instance '{}': unknown override '{}' at {}:{}",
                                child.label(),
                                value.name,
                                value.line,
                                value.column
                            )
                        })?;
                        if overrides
                            .insert(
                                value.name.clone(),
                                value.expression.qualify(&parameter_prefix),
                            )
                            .is_some()
                        {
                            return Err(format!(
                                "Instance '{}': duplicate override '{}'",
                                child.label(),
                                value.name
                            ));
                        }
                        value
                            .expression
                            .validate_units(&scope.units, *unit)
                            .map_err(|error| {
                                format!(
                                    "Instance '{}', override '{}' at {}:{}: {error}",
                                    child.label(),
                                    value.name,
                                    value.line,
                                    value.column
                                )
                            })?;
                    }
                    if self.output.len().saturating_add(module.statements.len()) > 100_000 {
                        return Err("Module expansion exceeds the 100000 statement limit".into());
                    }
                    let mut effective = module.statements.clone();
                    for statement in &mut effective {
                        if let Statement::Param(parameter) = statement
                            && let Some(expression) = overrides.remove(&parameter.name)
                        {
                            parameter.default_expression =
                                Some(parameter.expression.source.clone());
                            parameter.expression = expression;
                        }
                    }
                    self.emit(
                        Statement::Decl(ComponentDecl {
                            comp_type: ComponentType::ModulePort,
                            name: format!("{prefix}{}", instance.inst_name),
                            subtype: None,
                            value: Some(instance.module_name.clone()),
                            value_expression: None,
                            interface_pins: module.pins.clone(),
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
                    ));
                }
                _ => self.emit(statement.clone(), scope)?,
            }
        }
        Ok(())
    }
}

pub(crate) fn flatten(program: &Program) -> Result<Program, String> {
    let mut elaborator = Elaborator {
        modules: BTreeMap::new(),
        output: Vec::new(),
        stack: Vec::new(),
        identities: BTreeMap::new(),
        work: 0,
    };
    for module in &program.modules {
        if elaborator.modules.insert(&module.name, module).is_some() {
            return Err(format!("Duplicate module definition: {}", module.name));
        }
        if module.pins.iter().collect::<BTreeSet<_>>().len() != module.pins.len() {
            return Err(format!(
                "Module '{}': duplicate interface port",
                module.name
            ));
        }
        let units = declarations(&module.statements);
        let count = module
            .statements
            .iter()
            .filter(|s| matches!(s, Statement::Param(_)))
            .count();
        if count != units.len() || units.contains_key("pi") || count > MAX_PARAMETERS {
            return Err(format!(
                "Module '{}': duplicate/reserved parameter or parameter count limit exceeded",
                module.name
            ));
        }
        for statement in &module.statements {
            if let Statement::Param(parameter) = statement {
                parameter
                    .expression
                    .validate_units(&units, parameter.unit)
                    .map_err(|error| {
                        format!(
                            "Module '{}', parameter '{}' at {}:{}: {error}",
                            module.name, parameter.name, parameter.line, parameter.column
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
    };
    elaborator.expand(&program.statements, &root)?;
    Ok(Program {
        modules: Vec::new(),
        model_includes: program.model_includes.clone(),
        models: program.models.clone(),
        subcircuits: program.subcircuits.clone(),
        external_subcircuits: program.external_subcircuits.clone(),
        statements: elaborator.output,
    })
}
