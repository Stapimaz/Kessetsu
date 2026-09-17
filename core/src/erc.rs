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

    // 1. Duplicate declaration check (KES-E001)
    for comp in &circuit.components {
        if !declared.insert(comp.id.clone()) {
            errors.push(ErcDiagnostic {
                code: "KES-E001".to_string(),
                severity: Severity::Error,
                message: format!("Duplicate component declaration: {}", comp.id),
                component: Some(comp.id.clone()),
                pin: None,
            });
        }
    }

    let mut declared_nets = HashSet::new();
    for net in &circuit.nets {
        if !declared_nets.insert(net) {
            errors.push(ErcDiagnostic {
                code: "KES-E009".to_string(),
                severity: Severity::Error,
                message: format!("Duplicate net declaration: {net}"),
                component: None,
                pin: Some(net.clone()),
            });
        }
        if declared.contains(net) {
            errors.push(ErcDiagnostic {
                code: "KES-E006".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Namespace collision: '{net}' is declared as both a component and a net."
                ),
                component: Some(net.clone()),
                pin: Some(net.clone()),
            });
        }
    }

    // 2. Undefined component check (KES-E002)
    for conn in &circuit.connections {
        for p in &conn.pins {
            if !p.component.is_empty() && !declared.contains(&p.component) {
                errors.push(ErcDiagnostic {
                    code: "KES-E002".to_string(),
                    severity: Severity::Error,
                    message: format!("Connection refers to undeclared component: {}", p.component),
                    component: Some(p.component.clone()),
                    pin: Some(p.pin.clone()),
                });
            }
        }
    }

    // 3. Invalid component pin check (KES-E005), including declared module ports.
    for conn in &circuit.connections {
        for pin in &conn.pins {
            if pin.component.is_empty() {
                continue;
            }
            if let Some(component) = circuit
                .components
                .iter()
                .find(|component| component.id == pin.component)
                && (!is_valid_pin(&component.kind, &pin.pin)
                    || matches!(&component.parameters, crate::ir::ComponentParams::ModulePort { pins, .. } if !pins.contains(&pin.pin)))
            {
                errors.push(ErcDiagnostic {
                    code: "KES-E005".to_string(),
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

    // 4. Floating Pin Check (KES-E003)
    for comp in &circuit.components {
        for pin in component_definition(&comp.kind).pins {
            if graph.get_net(&comp.id, pin.name).is_none() {
                errors.push(ErcDiagnostic {
                    code: "KES-E003".to_string(),
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

    // 5. Short Circuit Check (Direct short across a power source) (KES-E004)
    for comp in &circuit.components {
        if comp.kind == ComponentKind::VoltageSource {
            let net1 = graph.get_net(&comp.id, "plus");
            let net2 = graph.get_net(&comp.id, "minus");

            if net1.is_some() && net1 == net2 {
                errors.push(ErcDiagnostic {
                    code: "KES-E004".to_string(),
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

    // 6. A physical net may have at most one user-facing name (KES-E007).
    for conflict in &graph.net_name_conflicts {
        errors.push(ErcDiagnostic {
            code: "KES-E007".to_string(),
            severity: Severity::Error,
            message: format!(
                "Conflicting user net names refer to the same physical net: {}.",
                conflict.names.join(", ")
            ),
            component: None,
            pin: conflict.names.first().cloned(),
        });
    }

    // 7. Ground selection remains deterministic for diagnostics, but ambiguity
    // is an error until the source explicitly names a single GND net (KES-E008).
    if graph.ground_candidates.len() > 1 {
        errors.push(ErcDiagnostic {
            code: "KES-E008".to_string(),
            severity: Severity::Error,
            message: format!(
                "Ambiguous {} ground candidates: {}. Connect the intended reference to a single explicit 'net GND'.",
                if graph.ground_is_explicit {
                    "explicit"
                } else {
                    "source-minus"
                },
                graph.ground_candidates.join(", ")
            ),
            component: None,
            pin: graph.ground_candidates.first().cloned(),
        });
    }

    errors.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then(left.component.cmp(&right.component))
            .then(left.pin.cmp(&right.pin))
            .then(left.message.cmp(&right.message))
    });

    errors
}
