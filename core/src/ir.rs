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
    DiodeParams,
    VoltageSource { value: SourceValue },
    CurrentSource { value: SourceValue },
    ModulePort { module_name: String },
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Analysis {
    OperatingPoint,
    Transient {
        step: Quantity,
        stop: Quantity,
    },
    Ac {
        scale: AcScale,
        points: u32,
        start: Quantity,
        stop: Quantity,
    },
    DcSweep {
        source: String,
        start: Quantity,
        stop: Quantity,
        step: Quantity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcScale {
    Decade,
    Octave,
    Linear,
}

impl Analysis {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::OperatingPoint => "op",
            Self::Transient { .. } => "tran",
            Self::Ac { .. } => "ac",
            Self::DcSweep { .. } => "dc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assertion {
    pub metric: String,
    pub signal: String,
    pub cmp: crate::ast::Cmp,
    pub threshold: Quantity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDiagnostic {
    pub code: String,
    pub message: String,
    pub component: Option<String>,
    pub field: Option<String>,
}

impl std::fmt::Display for SemanticDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SemanticDiagnostic {}

fn semantic_error(
    code: &str,
    message: impl Into<String>,
    component: Option<&str>,
    field: Option<&str>,
) -> SemanticDiagnostic {
    SemanticDiagnostic {
        code: code.to_string(),
        message: message.into(),
        component: component.map(str::to_string),
        field: field.map(str::to_string),
    }
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

fn parse_analysis(
    command: &crate::ast::SimulateStmt,
    components: &[IRComponent],
) -> Result<Analysis, SemanticDiagnostic> {
    let invalid = |message: String| semantic_error("NL-C009", message, None, Some("analysis"));
    let cmd = command.cmd.to_ascii_lowercase();

    match cmd.as_str() {
        "op" => {
            if command.args.is_empty() {
                Ok(Analysis::OperatingPoint)
            } else {
                Err(invalid(format!(
                    "simulate op expects no arguments, got {}",
                    command.args.len()
                )))
            }
        }
        "tran" => {
            if command.args.len() != 2 {
                return Err(invalid(format!(
                    "simulate tran expects step and stop, got {} arguments",
                    command.args.len()
                )));
            }
            let step = parse_quantity(&command.args[0], SIUnit::Second)
                .map_err(|error| invalid(format!("invalid transient step: {error}")))?;
            let stop = parse_quantity(&command.args[1], SIUnit::Second)
                .map_err(|error| invalid(format!("invalid transient stop: {error}")))?;
            if step.value <= 0.0 || stop.value <= 0.0 || step.value > stop.value {
                return Err(invalid(
                    "transient step and stop must be positive, with step <= stop".to_string(),
                ));
            }
            Ok(Analysis::Transient { step, stop })
        }
        "ac" => {
            if command.args.len() != 4 {
                return Err(invalid(format!(
                    "simulate ac expects scale, points, start and stop, got {} arguments",
                    command.args.len()
                )));
            }
            let scale = match command.args[0].to_ascii_lowercase().as_str() {
                "dec" => AcScale::Decade,
                "oct" => AcScale::Octave,
                "lin" => AcScale::Linear,
                value => {
                    return Err(invalid(format!(
                        "unsupported AC scale '{value}'; expected dec, oct or lin"
                    )));
                }
            };
            let points = command.args[1]
                .parse::<u32>()
                .map_err(|_| invalid("AC points must be a positive integer".to_string()))?;
            let start = parse_quantity(&command.args[2], SIUnit::Hertz)
                .map_err(|error| invalid(format!("invalid AC start frequency: {error}")))?;
            let stop = parse_quantity(&command.args[3], SIUnit::Hertz)
                .map_err(|error| invalid(format!("invalid AC stop frequency: {error}")))?;
            if points == 0 || start.value <= 0.0 || stop.value <= start.value {
                return Err(invalid(
                    "AC points must be positive and frequencies must satisfy 0 < start < stop"
                        .to_string(),
                ));
            }
            Ok(Analysis::Ac {
                scale,
                points,
                start,
                stop,
            })
        }
        "dc" => {
            if command.args.len() != 4 {
                return Err(invalid(format!(
                    "simulate dc expects source, start, stop and step, got {} arguments",
                    command.args.len()
                )));
            }
            let source = &command.args[0];
            let source_kind = components
                .iter()
                .find(|component| component.id == *source)
                .map(|component| &component.kind)
                .ok_or_else(|| invalid(format!("DC sweep source '{source}' is not declared")))?;
            let unit = match source_kind {
                ComponentKind::VoltageSource => SIUnit::Volt,
                ComponentKind::CurrentSource => SIUnit::Ampere,
                _ => {
                    return Err(invalid(format!(
                        "DC sweep target '{source}' must be a voltage or current source"
                    )));
                }
            };
            let start = parse_quantity(&command.args[1], unit)
                .map_err(|error| invalid(format!("invalid DC sweep start: {error}")))?;
            let stop = parse_quantity(&command.args[2], unit)
                .map_err(|error| invalid(format!("invalid DC sweep stop: {error}")))?;
            let step = parse_quantity(&command.args[3], unit)
                .map_err(|error| invalid(format!("invalid DC sweep step: {error}")))?;
            if step.value == 0.0
                || (stop.value - start.value).is_sign_positive() != step.value.is_sign_positive()
            {
                return Err(invalid(
                    "DC sweep step must be non-zero and move from start toward stop".to_string(),
                ));
            }
            Ok(Analysis::DcSweep {
                source: source.clone(),
                start,
                stop,
                step,
            })
        }
        _ => Err(invalid(format!(
            "unsupported simulation analysis '{}'; expected op, tran, ac or dc",
            command.cmd
        ))),
    }
}

pub fn ast_to_ir(program: &Program) -> Result<CircuitIR, SemanticDiagnostic> {
    let mut components = Vec::new();
    let mut connections = Vec::new();
    let mut nets = Vec::new();
    let mut analysis_statements = Vec::new();
    let mut assertions = Vec::new();

    for stmt in &program.statements {
        match stmt {
            Statement::Decl(decl) => {
                let val_str = decl.value.as_deref().unwrap_or("");
                let mut model = None;

                let (kind, params) = match decl.comp_type {
                    ComponentType::ModulePort => (
                        ComponentKind::ModulePort,
                        ComponentParams::ModulePort {
                            module_name: val_str.to_string(),
                        },
                    ),
                    ComponentType::Resistor => (
                        ComponentKind::Resistor,
                        ComponentParams::TwoPinPassive {
                            value: parse_quantity(val_str, SIUnit::Ohm).map_err(|error| {
                                semantic_error(
                                    "NL-C001",
                                    format!("invalid resistor value: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })?,
                        },
                    ),
                    ComponentType::Capacitor => (
                        ComponentKind::Capacitor,
                        ComponentParams::TwoPinPassive {
                            value: parse_quantity(val_str, SIUnit::Farad).map_err(|error| {
                                semantic_error(
                                    "NL-C001",
                                    format!("invalid capacitor value: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })?,
                        },
                    ),
                    ComponentType::Inductor => (
                        ComponentKind::Inductor,
                        ComponentParams::TwoPinPassive {
                            value: parse_quantity(val_str, SIUnit::Henry).map_err(|error| {
                                semantic_error(
                                    "NL-C001",
                                    format!("invalid inductor value: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })?,
                        },
                    ),
                    ComponentType::Source => {
                        let kind = ComponentKind::VoltageSource;
                        let value = if let Some(waveform) = parse_waveform(val_str, SIUnit::Volt)
                            .map_err(|error| {
                                semantic_error(
                                    "NL-C002",
                                    format!("invalid voltage-source waveform: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })? {
                            SourceValue::Waveform(waveform)
                        } else {
                            SourceValue::Dc(parse_quantity(val_str, SIUnit::Volt).map_err(
                                |error| {
                                    semantic_error(
                                        "NL-C001",
                                        format!("invalid voltage-source value: {error}"),
                                        Some(&decl.name),
                                        Some("value"),
                                    )
                                },
                            )?)
                        };
                        (kind, ComponentParams::VoltageSource { value })
                    }
                    ComponentType::CurrentSource => {
                        let kind = ComponentKind::CurrentSource;
                        let value = if let Some(waveform) = parse_waveform(val_str, SIUnit::Ampere)
                            .map_err(|error| {
                                semantic_error(
                                    "NL-C002",
                                    format!("invalid current-source waveform: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })? {
                            SourceValue::Waveform(waveform)
                        } else {
                            SourceValue::Dc(parse_quantity(val_str, SIUnit::Ampere).map_err(
                                |error| {
                                    semantic_error(
                                        "NL-C001",
                                        format!("invalid current-source value: {error}"),
                                        Some(&decl.name),
                                        Some("value"),
                                    )
                                },
                            )?)
                        };
                        (kind, ComponentParams::CurrentSource { value })
                    }
                    ComponentType::Transistor => {
                        let polarity_hint = decl.subtype.as_deref().map(|subtype| {
                            if subtype.eq_ignore_ascii_case("pnp") {
                                BJTPolarity::PNP
                            } else {
                                BJTPolarity::NPN
                            }
                        });
                        let requested_model = if val_str.is_empty() {
                            if polarity_hint == Some(BJTPolarity::PNP) {
                                "2N3906"
                            } else {
                                "2N3904"
                            }
                        } else {
                            val_str
                        };
                        let resolved = resolve_model(requested_model).ok_or_else(|| {
                            semantic_error(
                                "NL-C003",
                                format!(
                                    "unsupported BJT model '{requested_model}'; user-defined models are not yet declared by the language"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        let ComponentKind::BJT(model_polarity) = &resolved.kind else {
                            return Err(semantic_error(
                                "NL-C004",
                                format!("model '{requested_model}' is not a BJT model"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        };
                        if let Some(hint) = polarity_hint
                            && hint != *model_polarity
                        {
                            return Err(semantic_error(
                                "NL-C004",
                                format!(
                                    "BJT polarity {hint:?} conflicts with model '{requested_model}' ({model_polarity:?})"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        }
                        let polarity = *model_polarity;
                        model = Some(resolved);

                        (
                            ComponentKind::BJT(polarity),
                            ComponentParams::BJTParams { polarity },
                        )
                    }
                    ComponentType::Mosfet => {
                        let requested_model = if val_str.is_empty() {
                            "IRF540"
                        } else {
                            val_str
                        };
                        let resolved = resolve_model(requested_model).ok_or_else(|| {
                            semantic_error(
                                "NL-C003",
                                format!(
                                    "unsupported MOSFET model '{requested_model}'; user-defined models are not yet declared by the language"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        let ComponentKind::MOSFET(polarity) = &resolved.kind else {
                            return Err(semantic_error(
                                "NL-C004",
                                format!("model '{requested_model}' is not a MOSFET model"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        };
                        let polarity = *polarity;
                        model = Some(resolved);

                        (
                            ComponentKind::MOSFET(polarity),
                            ComponentParams::MOSFETParams { polarity },
                        )
                    }
                    ComponentType::Diode => {
                        let requested_model = if val_str.is_empty() {
                            "1N4148"
                        } else {
                            val_str
                        };
                        let resolved = resolve_model(requested_model).ok_or_else(|| {
                            semantic_error(
                                "NL-C003",
                                format!(
                                    "unsupported diode model '{requested_model}'; user-defined models are not yet declared by the language"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        if resolved.kind != ComponentKind::Diode {
                            return Err(semantic_error(
                                "NL-C004",
                                format!("model '{requested_model}' is not a diode model"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        }
                        model = Some(resolved);
                        (ComponentKind::Diode, ComponentParams::DiodeParams)
                    }
                    ComponentType::OpAmp => {
                        return Err(semantic_error(
                            if val_str.is_empty() {
                                "NL-C005"
                            } else {
                                "NL-C003"
                            },
                            if val_str.is_empty() {
                                "op-amp requires a supported model; no builtin op-amp model is available yet".to_string()
                            } else {
                                format!(
                                    "unsupported op-amp model '{val_str}'; user-defined models are not yet declared by the language"
                                )
                            },
                            Some(&decl.name),
                            Some("model"),
                        ));
                    } // _ is not needed since all ComponentTypes are covered
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
                let signal_function = assert.signal.split_once('(').map(|(name, _)| name);
                let signal_unit = match signal_function {
                    Some(name) if name.eq_ignore_ascii_case("V") => SIUnit::Volt,
                    Some(name) if name.eq_ignore_ascii_case("I") => SIUnit::Ampere,
                    _ => {
                        return Err(semantic_error(
                            "NL-C006",
                            format!(
                                "assertion signal '{}' must be a voltage V(...) or current I(...) measurement",
                                assert.signal
                            ),
                            None,
                            Some("signal"),
                        ));
                    }
                };
                assertions.push(Assertion {
                    metric: assert.metric.clone(),
                    signal: assert.signal.clone(),
                    cmp: assert.cmp.clone(),
                    threshold: parse_quantity(&assert.threshold, signal_unit).map_err(|error| {
                        semantic_error(
                            "NL-C006",
                            format!(
                                "invalid assertion threshold for '{}': {error}",
                                assert.signal
                            ),
                            None,
                            Some("threshold"),
                        )
                    })?,
                });
            }
            Statement::Simulate(sim) => {
                analysis_statements.push(sim.clone());
            }
            Statement::Use(_) => {
                return Err(semantic_error(
                    "NL-C007",
                    "use statements must be flattened before IR conversion",
                    None,
                    None,
                ));
            }
        }
    }

    let analyses = analysis_statements
        .iter()
        .map(|analysis| parse_analysis(analysis, &components))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CircuitIR {
        components,
        connections,
        nets,
        analyses,
        assertions,
    })
}
