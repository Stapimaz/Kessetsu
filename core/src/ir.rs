use std::str::FromStr;
use crate::ast::{ComponentType, Connection, Program, Statement};

#[derive(Debug, Clone, PartialEq)]
pub struct CircuitIR {
    pub components: Vec<IRComponent>,
    pub connections: Vec<Connection>, // Preserved from AST for graph generation
    pub nets: Vec<String>, // User-named nets
    pub analyses: Vec<Analysis>,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRComponent {
    pub id: String,
    pub kind: ComponentKind,
    pub parameters: ComponentParams,
    pub model: Option<ModelRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentKind {
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    BJT(BJTPolarity),
    MOSFET(FETPolarity),
    OpAmp,
    VoltageSource,
    CurrentSource,
    ModulePort,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BJTPolarity {
    NPN,
    PNP,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FETPolarity {
    NMOS,
    PMOS,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentParams {
    TwoPinPassive { value: f64, unit: SIUnit },
    BJTParams { polarity: BJTPolarity },
    MOSFETParams { polarity: FETPolarity },
    OpAmpParams,
    DCSource { voltage: f64 },
    ACSource { waveform: Waveform },
    Unknown { original_value: String }, // Temporary fallback if we can't parse it
}

#[derive(Debug, Clone, PartialEq)]
pub enum Waveform {
    Sine { offset: f64, amplitude: f64, frequency: f64 },
    Pulse { v1: f64, v2: f64, delay: f64, rise: f64, fall: f64, width: f64, period: f64 },
    PWL { points: Vec<(f64, f64)> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SIUnit {
    Ohm,
    Farad,
    Henry,
    Volt,
    Ampere,
    Hertz,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRef {
    pub name: String,
    pub kind: ComponentKind,
    pub source: ModelSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelSource {
    Builtin,
    UserDefined,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assertion {
    pub metric: String,
    pub signal: String,
    pub cmp: crate::ast::Cmp,
    pub threshold: f64,
}

pub fn parse_si_value(s: &str) -> Result<f64, String> {
    let mut num_str = String::new();
    let mut suffix = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            num_str.push(c);
        } else {
            suffix.push(c);
        }
    }

    if num_str.is_empty() {
        return Err(format!("Invalid SI value: {}", s));
    }

    let mut val = f64::from_str(&num_str).map_err(|e| format!("Parse float error: {}", e))?;

    let suffix_trim = suffix.trim();
    if suffix_trim.starts_with("p") { val *= 1e-12; }
    else if suffix_trim.starts_with("n") { val *= 1e-9; }
    else if suffix_trim.starts_with("u") || suffix_trim.starts_with("µ") { val *= 1e-6; }
    else if suffix_trim.starts_with("m") && !suffix_trim.starts_with("meg") { val *= 1e-3; }
    else if suffix_trim.starts_with("k") || suffix_trim.starts_with("K") { val *= 1e3; }
    else if suffix_trim.starts_with("meg") || suffix_trim.starts_with("M") { val *= 1e6; }
    else if suffix_trim.starts_with("G") { val *= 1e9; }
    else if suffix_trim.starts_with("T") { val *= 1e12; }

    Ok(val)
}

pub fn parse_waveform(val: &str) -> Option<Waveform> {
    // Basic SINE(offset amplitude frequency) parser
    let val_trim = val.trim();
    if val_trim.to_lowercase().starts_with("sine(") || val_trim.to_lowercase().starts_with("\"sine(") {
        // Find the contents inside the parentheses
        let start = val_trim.find('(')?;
        let end = val_trim.rfind(')')?;
        if start < end {
            let inside = &val_trim[start + 1..end];
            // Split by commas or whitespace
            let parts: Vec<&str> = inside.split(|c| c == ',' || c == ' ' || c == '\t')
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() >= 3 {
                let offset = parse_si_value(parts[0]).unwrap_or(0.0);
                let amp = parse_si_value(parts[1]).unwrap_or(0.0);
                let freq = parse_si_value(parts[2]).unwrap_or(0.0);
                return Some(Waveform::Sine { offset, amplitude: amp, frequency: freq });
            }
        }
    }
    None
}

pub fn resolve_model(name: &str) -> Option<ModelRef> {
    match name.to_uppercase().as_str() {
        "2N3904" | "2N2222" => Some(ModelRef { name: name.to_string(), kind: ComponentKind::BJT(BJTPolarity::NPN), source: ModelSource::Builtin }),
        "2N3906" => Some(ModelRef { name: name.to_string(), kind: ComponentKind::BJT(BJTPolarity::PNP), source: ModelSource::Builtin }),
        "1N4148" | "1N4007" => Some(ModelRef { name: name.to_string(), kind: ComponentKind::Diode, source: ModelSource::Builtin }),
        "IRF540" => Some(ModelRef { name: name.to_string(), kind: ComponentKind::MOSFET(FETPolarity::NMOS), source: ModelSource::Builtin }),
        _ => None,
    }
}

pub fn ast_to_ir(program: &Program) -> Result<CircuitIR, String> {
    let mut components = Vec::new();
    let mut connections = Vec::new();
    let mut nets = Vec::new();
    let mut analyses = Vec::new();
    let mut assertions = Vec::new();

    for stmt in &program.statements {
        match stmt {
            Statement::Decl(decl) => {
                let val_str = decl.value.as_deref().unwrap_or("");
                let mut model = resolve_model(val_str);
                
                let (kind, params) = match decl.comp_type {
                    ComponentType::ModulePort => {
                        (ComponentKind::ModulePort, ComponentParams::Unknown { original_value: val_str.to_string() })
                    },
                    ComponentType::Resistor => (
                        ComponentKind::Resistor,
                        ComponentParams::TwoPinPassive { 
                            value: parse_si_value(val_str).unwrap_or(0.0), 
                            unit: SIUnit::Ohm 
                        }
                    ),
                    ComponentType::Capacitor => (
                        ComponentKind::Capacitor,
                        ComponentParams::TwoPinPassive { 
                            value: parse_si_value(val_str).unwrap_or(0.0), 
                            unit: SIUnit::Farad 
                        }
                    ),
                    ComponentType::Inductor => (
                        ComponentKind::Inductor,
                        ComponentParams::TwoPinPassive { 
                            value: parse_si_value(val_str).unwrap_or(0.0), 
                            unit: SIUnit::Henry 
                        }
                    ),
                    ComponentType::Source => {
                        let kind = ComponentKind::VoltageSource;
                        if let Some(wf) = parse_waveform(val_str) {
                            (kind, ComponentParams::ACSource { waveform: wf })
                        } else {
                            (kind, ComponentParams::DCSource { 
                                voltage: parse_si_value(val_str).unwrap_or(0.0) 
                            })
                        }
                    },
                    ComponentType::CurrentSource => {
                        let kind = ComponentKind::CurrentSource;
                        if let Some(wf) = parse_waveform(val_str) {
                            (kind, ComponentParams::ACSource { waveform: wf })
                        } else {
                            (kind, ComponentParams::DCSource { 
                                voltage: parse_si_value(val_str).unwrap_or(0.0) 
                            })
                        }
                    },
                    ComponentType::Transistor => {
                        let mut polarity = BJTPolarity::NPN; // Default
                        
                        if let Some(sub) = &decl.subtype {
                            if sub.to_lowercase() == "pnp" { polarity = BJTPolarity::PNP; }
                        } else if let Some(m) = &model {
                            if let ComponentKind::BJT(p) = &m.kind { polarity = p.clone(); }
                        }
                        
                        if val_str.is_empty() {
                            let default_model = if polarity == BJTPolarity::NPN { "2N3904" } else { "2N3906" };
                            model = resolve_model(default_model);
                        }
                        
                        (ComponentKind::BJT(polarity.clone()), ComponentParams::BJTParams { polarity })
                    },
                    ComponentType::Mosfet => {
                        let mut polarity = FETPolarity::NMOS;
                        if let Some(m) = &model {
                            if let ComponentKind::MOSFET(p) = &m.kind { polarity = p.clone(); }
                        }
                        
                        if val_str.is_empty() {
                            model = resolve_model("IRF540");
                        }
                        
                        (ComponentKind::MOSFET(polarity.clone()), ComponentParams::MOSFETParams { polarity })
                    },
                    ComponentType::Diode => {
                        if val_str.is_empty() {
                            model = resolve_model("1N4148");
                        }
                        (ComponentKind::Diode, ComponentParams::Unknown { original_value: val_str.to_string() })
                    },
                    ComponentType::OpAmp => {
                        (ComponentKind::OpAmp, ComponentParams::OpAmpParams)
                    },
                    // _ is not needed since all ComponentTypes are covered
                };

                components.push(IRComponent {
                    id: decl.name.clone(),
                    kind,
                    parameters: params,
                    model,
                });
            },
            Statement::Connect(conn) => {
                connections.push(conn.clone());
            },
            Statement::Net(net) => {
                nets.push(net.name.clone());
            },
            Statement::Assert(assert) => {
                assertions.push(Assertion {
                    metric: assert.metric.clone(),
                    signal: assert.signal.clone(),
                    cmp: assert.cmp.clone(),
                    threshold: parse_si_value(&assert.threshold).unwrap_or(0.0),
                });
            },
            Statement::Simulate(sim) => {
                analyses.push(Analysis {
                    cmd: sim.cmd.clone(),
                    args: sim.args.clone(),
                });
            },
            Statement::Use(_) => {
                return Err("Use statements should be flattened before IR conversion".to_string());
            }
        }
    }

    Ok(CircuitIR {
        components,
        connections,
        nets,
        analyses,
        assertions,
    })
}
