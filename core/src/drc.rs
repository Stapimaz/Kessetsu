use crate::ast::*;
use crate::graph::NetlistGraph;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DrcError {
    pub message: String,
}

pub fn check_rules(program: &Program, graph: &NetlistGraph) -> Vec<DrcError> {
    let mut errors = Vec::new();
    let mut declared = HashSet::new();

    // 1. Duplicate declaration check
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if !declared.insert(decl.name.clone()) {
                errors.push(DrcError {
                    message: format!("Duplicate component declaration: {}", decl.name),
                });
            }
        }
    }

    // 2. Undefined component check
    for stmt in &program.statements {
        if let Statement::Connect(conn) = stmt {
            if !declared.contains(&conn.pin1.component) {
                errors.push(DrcError {
                    message: format!("Connection refers to undeclared component: {}", conn.pin1.component),
                });
            }
            if !declared.contains(&conn.pin2.component) {
                errors.push(DrcError {
                    message: format!("Connection refers to undeclared component: {}", conn.pin2.component),
                });
            }
        }
    }

    // 3. Floating Pin Check
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if decl.comp_type == ComponentType::ModulePort {
                continue;
            }
            
            let pins: Vec<&str> = match decl.comp_type {
                ComponentType::Battery => vec!["plus", "minus"],
                ComponentType::Transistor => vec!["c", "b", "e"],
                ComponentType::Mosfet => vec!["d", "g", "s"],
                ComponentType::OpAmp => vec!["in_p", "in_n", "out", "vcc", "vee"],
                _ => vec!["p1", "p2"], // Resistor, Capacitor, Inductor, Diode
            };

            for pin in pins {
                let net = graph.get_net(&decl.name, pin);
                if net == 9999 {
                    errors.push(DrcError {
                        message: format!("Floating Pin: {}.{} is not connected to anything.", decl.name, pin),
                    });
                }
            }
        }
    }

    // 4. Short Circuit Check (Direct short across a power source)
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if decl.comp_type == ComponentType::Battery {
                let net1 = graph.get_net(&decl.name, "plus");
                let net2 = graph.get_net(&decl.name, "minus");
                
                if net1 != 9999 && net2 != 9999 && net1 == net2 {
                    errors.push(DrcError {
                        message: format!("CRITICAL SHORT CIRCUIT: Battery {} plus and minus are connected together!", decl.name),
                    });
                }
            }
        }
    }

    errors
}
