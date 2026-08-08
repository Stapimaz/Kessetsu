use crate::ast::{ComponentType, Connection, Program, Statement};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitIR {
    pub components: Vec<IRComponent>,
    pub connections: Vec<Connection>, // Preserved from AST for graph generation
    pub nets: Vec<String>,            // User-named nets
    pub analyses: Vec<Analysis>,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IRComponent {
    pub id: String,
    pub kind: ComponentKind,
    pub parameters: ComponentParams,
    pub model: Option<ModelRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BJTPolarity {
    NPN,
    PNP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FETPolarity {
    NMOS,
    PMOS,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentParams {
    TwoPinPassive { value: Quantity },
    BJTParams { polarity: BJTPolarity },
    MOSFETParams { polarity: FETPolarity },
    OpAmpParams,
    VoltageSource { value: SourceValue },
    CurrentSource { value: SourceValue },
    Unknown { original_value: String }, // Temporary fallback if we can't parse it
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceValue {
    Dc(Quantity),
    Waveform(Waveform),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Waveform {
    Sine {
        offset: Quantity,
        amplitude: Quantity,
        frequency: Quantity,
    },
    Pulse {
        v1: Quantity,
        v2: Quantity,
        delay: Quantity,
        rise: Quantity,
        fall: Quantity,
        width: Quantity,
        period: Quantity,
    },
    PWL {
        points: Vec<(Quantity, Quantity)>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SIUnit {
    Ohm,
    Farad,
    Henry,
    Volt,
    Ampere,
    Hertz,
    Second,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: f64,
    pub unit: SIUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRef {
    pub name: String,
    pub kind: ComponentKind,
    pub source: ModelSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelSource {
    Builtin,
    UserDefined,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Analysis {
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assertion {
    pub metric: String,
    pub signal: String,
    pub cmp: crate::ast::Cmp,
    pub threshold: Quantity,
}

fn split_number_and_suffix(input: &str) -> Result<(&str, &str), String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("value is missing".to_string());
    }

    let bytes = input.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        index += 1;
    }

    let integer_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let mut has_digits = index > integer_start;

    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let fractional_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        has_digits |= index > fractional_start;
    }

    if !has_digits {
        return Err(format!("invalid numeric value '{input}'"));
    }

    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let exponent_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == exponent_start {
            return Err(format!("invalid exponent in '{input}'"));
        }
    }

    Ok((&input[..index], &input[index..]))
}

fn parse_unit_suffix(suffix: &str) -> Result<(f64, Option<SIUnit>), String> {
    let (factor, unit_text) = if let Some(rest) = suffix.strip_prefix("meg") {
        (1e6, rest)
    } else if let Some(rest) = suffix.strip_prefix('T') {
        (1e12, rest)
    } else if let Some(rest) = suffix.strip_prefix('G') {
        (1e9, rest)
    } else if let Some(rest) = suffix.strip_prefix('M') {
        (1e6, rest)
    } else if let Some(rest) = suffix.strip_prefix(['k', 'K']) {
        (1e3, rest)
    } else if let Some(rest) = suffix.strip_prefix('m') {
        (1e-3, rest)
    } else if let Some(rest) = suffix.strip_prefix(['u', 'µ']) {
        (1e-6, rest)
    } else if let Some(rest) = suffix.strip_prefix('n') {
        (1e-9, rest)
    } else if let Some(rest) = suffix.strip_prefix('p') {
        (1e-12, rest)
    } else {
        (1.0, suffix)
    };

    let unit = match unit_text {
        "" => None,
        "Ohm" | "ohm" | "OHM" | "Ω" => Some(SIUnit::Ohm),
        "F" => Some(SIUnit::Farad),
        "H" => Some(SIUnit::Henry),
        "V" | "v" => Some(SIUnit::Volt),
        "A" | "a" => Some(SIUnit::Ampere),
        "Hz" | "hz" | "HZ" => Some(SIUnit::Hertz),
        "s" | "S" => Some(SIUnit::Second),
        _ => return Err(format!("unsupported unit or trailing text '{suffix}'")),
    };

    Ok((factor, unit))
}

fn parse_value(input: &str) -> Result<(f64, Option<SIUnit>), String> {
    let (number, suffix) = split_number_and_suffix(input)?;
    let parsed =
        f64::from_str(number).map_err(|error| format!("invalid number '{number}': {error}"))?;
    if !parsed.is_finite() {
        return Err(format!("non-finite value '{input}' is not supported"));
    }
    let (factor, unit) = parse_unit_suffix(suffix)?;
    Ok((parsed * factor, unit))
}

pub fn parse_quantity(input: &str, expected_unit: SIUnit) -> Result<Quantity, String> {
    let (value, explicit_unit) = parse_value(input)?;
    if let Some(actual_unit) = explicit_unit
        && actual_unit != expected_unit
    {
        return Err(format!(
            "unit mismatch for '{input}': expected {expected_unit:?}, got {actual_unit:?}"
        ));
    }
    Ok(Quantity {
        value,
        unit: expected_unit,
    })
}

pub fn parse_si_value(input: &str) -> Result<f64, String> {
    parse_value(input).map(|(value, _)| value)
}

pub fn parse_waveform(val: &str, value_unit: SIUnit) -> Result<Option<Waveform>, String> {
    let val_trim = val.trim().trim_matches('"').trim();
    let Some(start) = val_trim.find('(') else {
        return Ok(None);
    };
    if !val_trim.ends_with(')') || start == 0 {
        return Err(format!("malformed waveform '{val}'"));
    }

    let name = &val_trim[..start];
    let inside = &val_trim[start + 1..val_trim.len() - 1];
    let parts: Vec<&str> = inside
        .split([',', ' ', '\t'])
        .filter(|part| !part.is_empty())
        .collect();

    if name.eq_ignore_ascii_case("sine") {
        if parts.len() != 3 {
            return Err(format!(
                "SINE expects exactly 3 parameters, got {}",
                parts.len()
            ));
        }
        return Ok(Some(Waveform::Sine {
            offset: parse_quantity(parts[0], value_unit)?,
            amplitude: parse_quantity(parts[1], value_unit)?,
            frequency: parse_quantity(parts[2], SIUnit::Hertz)?,
        }));
    }

    if name.eq_ignore_ascii_case("pulse") {
        if parts.len() != 7 {
            return Err(format!(
                "PULSE expects exactly 7 parameters, got {}",
                parts.len()
            ));
        }
        return Ok(Some(Waveform::Pulse {
            v1: parse_quantity(parts[0], value_unit)?,
            v2: parse_quantity(parts[1], value_unit)?,
            delay: parse_quantity(parts[2], SIUnit::Second)?,
            rise: parse_quantity(parts[3], SIUnit::Second)?,
            fall: parse_quantity(parts[4], SIUnit::Second)?,
            width: parse_quantity(parts[5], SIUnit::Second)?,
            period: parse_quantity(parts[6], SIUnit::Second)?,
        }));
    }

    Err(format!("unsupported waveform '{name}'"))
}

pub fn resolve_model(name: &str) -> Option<ModelRef> {
    match name.to_uppercase().as_str() {
        "2N3904" | "2N2222" => Some(ModelRef {
            name: name.to_string(),
            kind: ComponentKind::BJT(BJTPolarity::NPN),
            source: ModelSource::Builtin,
        }),
        "2N3906" => Some(ModelRef {
            name: name.to_string(),
            kind: ComponentKind::BJT(BJTPolarity::PNP),
            source: ModelSource::Builtin,
        }),
        "1N4148" | "1N4007" => Some(ModelRef {
            name: name.to_string(),
            kind: ComponentKind::Diode,
            source: ModelSource::Builtin,
        }),
        "IRF540" => Some(ModelRef {
            name: name.to_string(),
            kind: ComponentKind::MOSFET(FETPolarity::NMOS),
            source: ModelSource::Builtin,
        }),
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
                    ComponentType::ModulePort => (
                        ComponentKind::ModulePort,
                        ComponentParams::Unknown {
                            original_value: val_str.to_string(),
                        },
                    ),
                    ComponentType::Resistor => (
                        ComponentKind::Resistor,
                        ComponentParams::TwoPinPassive {
                            value: parse_quantity(val_str, SIUnit::Ohm)
                                .map_err(|error| format!("resistor {}: {error}", decl.name))?,
                        },
                    ),
                    ComponentType::Capacitor => (
                        ComponentKind::Capacitor,
                        ComponentParams::TwoPinPassive {
                            value: parse_quantity(val_str, SIUnit::Farad)
                                .map_err(|error| format!("capacitor {}: {error}", decl.name))?,
                        },
                    ),
                    ComponentType::Inductor => (
                        ComponentKind::Inductor,
                        ComponentParams::TwoPinPassive {
                            value: parse_quantity(val_str, SIUnit::Henry)
                                .map_err(|error| format!("inductor {}: {error}", decl.name))?,
                        },
                    ),
                    ComponentType::Source => {
                        let kind = ComponentKind::VoltageSource;
                        let value = if let Some(waveform) = parse_waveform(val_str, SIUnit::Volt)
                            .map_err(|error| format!("source {}: {error}", decl.name))?
                        {
                            SourceValue::Waveform(waveform)
                        } else {
                            SourceValue::Dc(
                                parse_quantity(val_str, SIUnit::Volt)
                                    .map_err(|error| format!("source {}: {error}", decl.name))?,
                            )
                        };
                        (kind, ComponentParams::VoltageSource { value })
                    }
                    ComponentType::CurrentSource => {
                        let kind = ComponentKind::CurrentSource;
                        let value = if let Some(waveform) = parse_waveform(val_str, SIUnit::Ampere)
                            .map_err(|error| format!("current source {}: {error}", decl.name))?
                        {
                            SourceValue::Waveform(waveform)
                        } else {
                            SourceValue::Dc(parse_quantity(val_str, SIUnit::Ampere).map_err(
                                |error| format!("current source {}: {error}", decl.name),
                            )?)
                        };
                        (kind, ComponentParams::CurrentSource { value })
                    }
                    ComponentType::Transistor => {
                        let mut polarity = BJTPolarity::NPN; // Default

                        if let Some(sub) = &decl.subtype {
                            if sub.to_lowercase() == "pnp" {
                                polarity = BJTPolarity::PNP;
                            }
                        } else if let Some(m) = &model
                            && let ComponentKind::BJT(p) = &m.kind
                        {
                            polarity = *p;
                        }

                        if val_str.is_empty() {
                            let default_model = if polarity == BJTPolarity::NPN {
                                "2N3904"
                            } else {
                                "2N3906"
                            };
                            model = resolve_model(default_model);
                        }

                        (
                            ComponentKind::BJT(polarity),
                            ComponentParams::BJTParams { polarity },
                        )
                    }
                    ComponentType::Mosfet => {
                        let mut polarity = FETPolarity::NMOS;
                        if let Some(m) = &model
                            && let ComponentKind::MOSFET(p) = &m.kind
                        {
                            polarity = *p;
                        }

                        if val_str.is_empty() {
                            model = resolve_model("IRF540");
                        }

                        (
                            ComponentKind::MOSFET(polarity),
                            ComponentParams::MOSFETParams { polarity },
                        )
                    }
                    ComponentType::Diode => {
                        if val_str.is_empty() {
                            model = resolve_model("1N4148");
                        }
                        (
                            ComponentKind::Diode,
                            ComponentParams::Unknown {
                                original_value: val_str.to_string(),
                            },
                        )
                    }
                    ComponentType::OpAmp => (ComponentKind::OpAmp, ComponentParams::OpAmpParams),
                    // _ is not needed since all ComponentTypes are covered
                };

                components.push(IRComponent {
                    id: decl.name.clone(),
                    kind,
                    parameters: params,
                    model,
                });
            }
            Statement::Connect(conn) => {
                connections.push(conn.clone());
            }
            Statement::Net(net) => {
                nets.push(net.name.clone());
            }
            Statement::Assert(assert) => {
                assertions.push(Assertion {
                    metric: assert.metric.clone(),
                    signal: assert.signal.clone(),
                    cmp: assert.cmp.clone(),
                    threshold: parse_quantity(
                        &assert.threshold,
                        if assert.signal.starts_with("V(") {
                            SIUnit::Volt
                        } else if assert.signal.starts_with("I(") {
                            SIUnit::Ampere
                        } else {
                            return Err(format!(
                                "assertion signal '{}' must be a voltage V(...) or current I(...)",
                                assert.signal
                            ));
                        },
                    )
                    .map_err(|error| format!("assertion {}: {error}", assert.signal))?,
                });
            }
            Statement::Simulate(sim) => {
                analyses.push(Analysis {
                    cmd: sim.cmd.clone(),
                    args: sim.args.clone(),
                });
            }
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
