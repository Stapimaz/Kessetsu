use crate::component::{component_definition, is_valid_pin};
use crate::graph::NetlistGraph;
use crate::ir::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ErcDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub component: Option<String>,
    pub pin: Option<String>,
}

pub fn check_rules(circuit: &CircuitIR, graph: &NetlistGraph) -> Vec<ErcDiagnostic> {
    let mut errors = Vec::new();
    let mut declared = HashSet::new();

    // 1. Duplicate declaration check (NL-E001)
    for comp in &circuit.components {
        if !declared.insert(comp.id.clone()) {
            errors.push(ErcDiagnostic {
                code: "NL-E001".to_string(),
                severity: Severity::Error,
                message: format!("Duplicate component declaration: {}", comp.id),
                component: Some(comp.id.clone()),
                pin: None,
            });
        }
    }

    // 2. Undefined component check (NL-E002)
    for conn in &circuit.connections {
        for p in &conn.pins {
            if !p.component.is_empty() && !declared.contains(&p.component) {
                errors.push(ErcDiagnostic {
                    code: "NL-E002".to_string(),
                    severity: Severity::Error,
                    message: format!("Connection refers to undeclared component: {}", p.component),
                    component: Some(p.component.clone()),
                    pin: Some(p.pin.clone()),
                });
            }
        }
    }

    // 3. Invalid component pin check (NL-E005). Module ports are dynamic until
    // module interface pins are carried into IR.
    for conn in &circuit.connections {
        for pin in &conn.pins {
            if pin.component.is_empty() {
                continue;
            }
            if let Some(component) = circuit
                .components
                .iter()
                .find(|component| component.id == pin.component)
                && !is_valid_pin(&component.kind, &pin.pin)
            {
                errors.push(ErcDiagnostic {
                    code: "NL-E005".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Invalid pin reference: {}.{} does not exist.",
                        pin.component, pin.pin
                    ),
                    component: Some(pin.component.clone()),
                    pin: Some(pin.pin.clone()),
                });
            }
        }
    }

    // 4. Floating Pin Check (NL-E003)
    for comp in &circuit.components {
        for pin in component_definition(&comp.kind).pins {
            if graph.get_net(&comp.id, pin.name).is_none() {
                errors.push(ErcDiagnostic {
                    code: "NL-E003".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Floating Pin: {}.{} is not connected to anything.",
                        comp.id, pin.name
                    ),
                    component: Some(comp.id.clone()),
                    pin: Some(pin.name.to_string()),
                });
            }
        }
    }

    // 5. Short Circuit Check (Direct short across a power source) (NL-E004)
    for comp in &circuit.components {
        if comp.kind == ComponentKind::VoltageSource {
            let net1 = graph.get_net(&comp.id, "plus");
            let net2 = graph.get_net(&comp.id, "minus");

            if net1.is_some() && net1 == net2 {
                errors.push(ErcDiagnostic {
                    code: "NL-E004".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "CRITICAL SHORT CIRCUIT: Source {} plus and minus are connected together!",
                        comp.id
                    ),
                    component: Some(comp.id.clone()),
                    pin: None,
                });
            }
        }
    }

    errors
}
