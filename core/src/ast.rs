use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    Resistor,
    Battery,
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
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinRef {
    pub component: String,
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub pin1: PinRef,
    pub pin2: PinRef,
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
pub enum Statement {
    Decl(ComponentDecl),
    Connect(Connection),
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
                        value: decl.value.clone(),
                    }));
                }
                Statement::Connect(conn) => {
                    let map_pin = |pin: &PinRef| -> PinRef {
                        if pin.component.is_empty() {
                            // It's a module pin (e.g., `in`), it maps to `prefix.in`
                            let prefix_trimmed = prefix.trim_end_matches('_');
                            PinRef { component: prefix_trimmed.to_string(), pin: pin.pin.clone() }
                        } else {
                            // It's an internal component, prefix it
                            PinRef { component: format!("{}{}", prefix, pin.component), pin: pin.pin.clone() }
                        }
                    };
                    flat_statements.push(Statement::Connect(Connection {
                        pin1: map_pin(&conn.pin1),
                        pin2: map_pin(&conn.pin2),
                    }));
                }
                Statement::Use(use_stmt) => {
                    let md = module_map.get(&use_stmt.module_name)
                        .ok_or(format!("Module not found: {}", use_stmt.module_name))?;
                    
                    let inst_name = format!("{}{}", prefix, use_stmt.inst_name);
                    flat_statements.push(Statement::Decl(ComponentDecl {
                        comp_type: ComponentType::ModulePort,
                        name: inst_name,
                        value: use_stmt.module_name.clone(),
                    }));

                    let new_prefix = format!("{}{}_", prefix, use_stmt.inst_name);
                    for s in &md.statements {
                        flatten_stmt(s, &new_prefix, module_map, flat_statements)?;
                    }
                }
                Statement::Simulate(sim) => {
                    // Sim statements usually only exist in the top level. 
                    // If they are in a module, we just push them.
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
            statements: flat_statements,
        })
    }
}
