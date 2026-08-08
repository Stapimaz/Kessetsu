use crate::ir::*;
use crate::graph::NetlistGraph;
use std::collections::HashSet;
use serde::{Serialize, Deserialize};

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
            if p.component != "" && !declared.contains(&p.component) {
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

    // 3. Floating Pin Check (NL-E003)
    for comp in &circuit.components {
        let pins: Vec<&str> = match comp.kind {
            ComponentKind::VoltageSource | ComponentKind::CurrentSource => vec!["plus", "minus"],
            ComponentKind::BJT(_) => vec!["c", "b", "e"],
            ComponentKind::MOSFET(_) => vec!["d", "g", "s"],
            ComponentKind::OpAmp => vec!["in_p", "in_n", "out", "vcc", "vee"],
            ComponentKind::ModulePort => continue,
            _ => vec!["p1", "p2"], // Resistor, Capacitor, Inductor, Diode
        };

        for pin in pins {
            let net = graph.get_net(&comp.id, pin);
            if net == 9999 {
                errors.push(ErcDiagnostic {
                    code: "NL-E003".to_string(),
                    severity: Severity::Error,
                    message: format!("Floating Pin: {}.{} is not connected to anything.", comp.id, pin),
                    component: Some(comp.id.clone()),
                    pin: Some(pin.to_string()),
                });
            }
        }
    }

    // 4. Short Circuit Check (Direct short across a power source) (NL-E004)
    for comp in &circuit.components {
        if comp.kind == ComponentKind::VoltageSource {
            let net1 = graph.get_net(&comp.id, "plus");
            let net2 = graph.get_net(&comp.id, "minus");
            
            if net1 != 9999 && net2 != 9999 && net1 == net2 {
                errors.push(ErcDiagnostic {
                    code: "NL-E004".to_string(),
                    severity: Severity::Error,
                    message: format!("CRITICAL SHORT CIRCUIT: Source {} plus and minus are connected together!", comp.id),
                    component: Some(comp.id.clone()),
                    pin: None,
                });
            }
        }
    }

    errors
}
