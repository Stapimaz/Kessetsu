use crate::ast::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DrcError {
    pub message: String,
}

pub fn check_rules(program: &Program) -> Vec<DrcError> {
    let mut errors = Vec::new();
    let mut components = HashSet::new();
    
    // Check for duplicate components and collect them
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if !components.insert(decl.name.clone()) {
                errors.push(DrcError {
                    message: format!("Duplicate component declaration: {}", decl.name),
                });
            }
        }
    }

    // Check for connections to undeclared components
    for stmt in &program.statements {
        if let Statement::Connect(conn) = stmt {
            if !components.contains(&conn.pin1.component) {
                errors.push(DrcError {
                    message: format!("Connection refers to undeclared component: {}", conn.pin1.component),
                });
            }
            if !components.contains(&conn.pin2.component) {
                errors.push(DrcError {
                    message: format!("Connection refers to undeclared component: {}", conn.pin2.component),
                });
            }
        }
    }

    errors
}
