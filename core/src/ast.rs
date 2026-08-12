use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    Resistor,
    Source,
    CurrentSource,
    Capacitor,
    Inductor,
    Diode,
    Transistor,
    Mosfet,
    OpAmp,
    ModulePort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDecl {
    pub comp_type: ComponentType,
    pub name: String,
    pub subtype: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinRef {
    pub component: String,
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub pins: Vec<PinRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetDecl {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cmp {
    Lt,
    Gt,
    Eq,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertStmt {
    pub metric: String,
    pub signal: String,
    pub cmp: Cmp,
    pub threshold: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseStmt {
    pub module_name: String,
    pub inst_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulateStmt {
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelDeclKind {
    Diode,
    BJT,
    MOSFET,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDecl {
    pub kind: ModelDeclKind,
    pub name: String,
    pub polarity: Option<String>,
    pub parameters: Vec<NamedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubcircuitDecl {
    pub kind: String,
    pub name: String,
    pub pins: Vec<String>,
    pub parameters: Vec<NamedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInclude {
    pub package: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Decl(ComponentDecl),
    Connect(Connection),
    Net(NetDecl),
    Assert(AssertStmt),
    Use(UseStmt),
    Simulate(SimulateStmt),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    pub pins: Vec<String>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub modules: Vec<ModuleDef>,
    pub model_includes: Vec<ModelInclude>,
    pub models: Vec<ModelDecl>,
    pub subcircuits: Vec<SubcircuitDecl>,
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn flatten(&self) -> Result<Program, String> {
        let mut flat_statements = Vec::new();
        let mut module_map = std::collections::HashMap::new();

        for md in &self.modules {
            module_map.insert(md.name.clone(), md);
        }

        fn flatten_stmt(
            stmt: &Statement,
            prefix: &str,
            module_map: &std::collections::HashMap<String, &ModuleDef>,
            flat_statements: &mut Vec<Statement>,
        ) -> Result<(), String> {
            match stmt {
                Statement::Decl(decl) => {
                    flat_statements.push(Statement::Decl(ComponentDecl {
                        comp_type: decl.comp_type.clone(),
                        name: format!("{}{}", prefix, decl.name),
                        subtype: decl.subtype.clone(),
                        value: decl.value.clone(),
                    }));
                }
                Statement::Connect(conn) => {
                    let map_pin = |pin: &PinRef| -> PinRef {
                        if pin.component.is_empty() {
                            let prefix_trimmed = prefix.trim_end_matches('_');
                            PinRef {
                                component: prefix_trimmed.to_string(),
                                pin: pin.pin.clone(),
                            }
                        } else {
                            PinRef {
                                component: format!("{}{}", prefix, pin.component),
                                pin: pin.pin.clone(),
                            }
                        }
                    };

                    let new_pins = conn.pins.iter().map(map_pin).collect();
                    flat_statements.push(Statement::Connect(Connection { pins: new_pins }));
                }
                Statement::Use(use_stmt) => {
                    let md = module_map
                        .get(&use_stmt.module_name)
                        .ok_or(format!("Module not found: {}", use_stmt.module_name))?;

                    let inst_name = format!("{}{}", prefix, use_stmt.inst_name);
                    flat_statements.push(Statement::Decl(ComponentDecl {
                        comp_type: ComponentType::ModulePort,
                        name: inst_name,
                        subtype: None,
                        value: Some(use_stmt.module_name.clone()),
                    }));

                    let new_prefix = format!("{}{}_", prefix, use_stmt.inst_name);
                    for s in &md.statements {
                        flatten_stmt(s, &new_prefix, module_map, flat_statements)?;
                    }
                }
                Statement::Net(net) => {
                    flat_statements.push(Statement::Net(NetDecl {
                        name: format!("{}{}", prefix, net.name),
                    }));
                }
                Statement::Assert(assert) => {
                    flat_statements.push(Statement::Assert(AssertStmt {
                        metric: assert.metric.clone(),
                        signal: assert.signal.clone(),
                        cmp: assert.cmp.clone(),
                        threshold: assert.threshold.clone(),
                    }));
                }
                Statement::Simulate(sim) => {
                    flat_statements.push(Statement::Simulate(sim.clone()));
                }
            }
            Ok(())
        }

        for s in &self.statements {
            flatten_stmt(s, "", &module_map, &mut flat_statements)?;
        }

        Ok(Program {
            modules: Vec::new(),
            model_includes: self.model_includes.clone(),
            models: self.models.clone(),
            subcircuits: self.subcircuits.clone(),
            statements: flat_statements,
        })
    }
}
