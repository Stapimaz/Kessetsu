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
    pub model_manifest: ModelManifest,
    #[serde(
        default,
        skip_serializing_if = "crate::expression::ParameterManifest::is_empty"
    )]
    pub parameter_manifest: crate::expression::ParameterManifest,
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
    TwoPinPassive {
        value: Quantity,
    },
    BJTParams {
        polarity: BJTPolarity,
    },
    MOSFETParams {
        polarity: FETPolarity,
    },
    DiodeParams,
    OpAmpParams,
    VoltageSource {
        value: SourceValue,
    },
    CurrentSource {
        value: SourceValue,
    },
    ModulePort {
        module_name: String,
        #[serde(default)]
        pins: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceValue {
    Dc(Quantity),
    Waveform(Waveform),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Waveform {
    Ac {
        amplitude: Quantity,
    },
    Sine {
        offset: Quantity,
        amplitude: Quantity,
        frequency: Quantity,
    },
    SineAc {
        offset: Quantity,
        amplitude: Quantity,
        frequency: Quantity,
        ac_amplitude: Quantity,
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
    Watt,
    Ratio,
    Percent,
    Degree,
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
    pub definition: ModelDefinition,
    pub provenance: ModelProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelSource {
    Builtin,
    UserDefined,
    Package,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorCompatibility {
    Ngspice,
    NgspicePs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedistributionPolicy {
    Permitted,
    Prohibited,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalModelMetadata {
    pub resource: String,
    pub entry: String,
    pub pins: Vec<String>,
    pub simulator: SimulatorCompatibility,
    pub redistribution: RedistributionPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum ModelDefinition {
    Device {
        directive: String,
    },
    Subcircuit {
        directive: String,
        pins: Vec<String>,
    },
    ExternalSubcircuit {
        metadata: ExternalModelMetadata,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelProvenance {
    pub source: String,
    pub license: String,
    pub version: String,
    pub content_hash: String,
    pub simulator: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModelManifest {
    pub schema_version: String,
    pub packages: Vec<ResolvedModelPackage>,
    pub models: Vec<ModelManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedModelPackage {
    pub name: String,
    pub version: String,
    pub license: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelManifestEntry {
    pub name: String,
    pub kind: ComponentKind,
    pub source: ModelSource,
    pub provenance: ModelProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<ExternalModelMetadata>,
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
        "W" | "w" => Some(SIUnit::Watt),
        "%" => Some(SIUnit::Percent),
        "deg" | "degree" | "degrees" => Some(SIUnit::Degree),
        _ => return Err(format!("unsupported unit or trailing text '{suffix}'")),
    };

    Ok((factor, unit))
}

pub(crate) fn parse_value(input: &str) -> Result<(f64, Option<SIUnit>), String> {
    let (number, suffix) = split_number_and_suffix(input)?;
    let parsed =
        f64::from_str(number).map_err(|error| format!("invalid number '{number}': {error}"))?;
    let nonzero_mantissa = number
        .split(['e', 'E'])
        .next()
        .unwrap()
        .bytes()
        .any(|byte| matches!(byte, b'1'..=b'9'));
    if !parsed.is_finite() || (parsed == 0.0 && nonzero_mantissa) {
        return Err(format!("non-finite value '{input}' is not supported"));
    }
    let (factor, unit) = parse_unit_suffix(suffix)?;
    let value = parsed * factor;
    if !value.is_finite() || (parsed != 0.0 && value == 0.0) {
        return Err(format!(
            "value '{input}' is outside the supported numeric range"
        ));
    }
    Ok((value, unit))
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

    if name.eq_ignore_ascii_case("ac") {
        if parts.len() != 1 {
            return Err(format!(
                "AC expects exactly 1 parameter, got {}",
                parts.len()
            ));
        }
        return Ok(Some(Waveform::Ac {
            amplitude: parse_quantity(parts[0], value_unit)?,
        }));
    }

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

    if name.eq_ignore_ascii_case("sine_ac") {
        if parts.len() != 4 {
            return Err(format!(
                "SINE_AC expects exactly 4 parameters, got {}",
                parts.len()
            ));
        }
        return Ok(Some(Waveform::SineAc {
            offset: parse_quantity(parts[0], value_unit)?,
            amplitude: parse_quantity(parts[1], value_unit)?,
            frequency: parse_quantity(parts[2], SIUnit::Hertz)?,
            ac_amplitude: parse_quantity(parts[3], value_unit)?,
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

    if name.eq_ignore_ascii_case("pwl") {
        if parts.len() < 4 || !parts.len().is_multiple_of(2) {
            return Err(format!(
                "PWL expects at least 2 time/value pairs, got {} parameters",
                parts.len()
            ));
        }
        let mut points = Vec::with_capacity(parts.len() / 2);
        for pair in parts.chunks_exact(2) {
            let time = parse_quantity(pair[0], SIUnit::Second)?;
            let value = parse_quantity(pair[1], value_unit)?;
            if time.value < 0.0 {
                return Err("PWL times must be non-negative".to_string());
            }
            if points
                .last()
                .is_some_and(|previous: &(Quantity, Quantity)| previous.0.value >= time.value)
            {
                return Err("PWL times must be strictly increasing".to_string());
            }
            points.push((time, value));
        }
        return Ok(Some(Waveform::PWL { points }));
    }

    Err(format!("unsupported waveform '{name}'"))
}

pub fn resolve_model(name: &str) -> Option<ModelRef> {
    crate::models::builtin_model(name)
}

fn parse_analysis(
    command: &crate::ast::SimulateStmt,
    components: &[IRComponent],
) -> Result<Analysis, SemanticDiagnostic> {
    let invalid = |message: String| semantic_error("KES-C009", message, None, Some("analysis"));
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
    ast_to_ir_with_resources(program, &crate::models::ExternalModelResources::new())
}

pub fn ast_to_ir_with_resources(
    program: &Program,
    resources: &crate::models::ExternalModelResources,
) -> Result<CircuitIR, SemanticDiagnostic> {
    let declarations = program
        .statements
        .iter()
        .filter_map(|statement| {
            if let Statement::Param(parameter) = statement {
                Some(parameter)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let (parameter_values, mut parameter_manifest) =
        crate::expression::resolve_parameters(&declarations)
            .map_err(|cause| semantic_error(cause.code, cause.message, None, Some(&cause.name)))?;
    let mut expression_work: usize = declarations
        .iter()
        .map(|parameter| parameter.expression.node_count())
        .sum();
    let model_library = crate::models::resolve_program_models_with_resources(program, resources)?;
    let mut components = Vec::new();
    let mut connections = Vec::new();
    let mut nets = Vec::new();
    let mut analysis_statements = Vec::new();
    let mut assertions = Vec::new();

    for stmt in &program.statements {
        match stmt {
            Statement::Param(_) => {}
            Statement::Decl(decl) => {
                let val_str = decl.value.as_deref().unwrap_or("");
                if decl.value_expression.is_some()
                    && !matches!(
                        decl.comp_type,
                        ComponentType::Resistor
                            | ComponentType::Capacitor
                            | ComponentType::Inductor
                            | ComponentType::Source
                            | ComponentType::CurrentSource
                    )
                {
                    return Err(semantic_error(
                        "KES-C022",
                        "Expressions are only accepted in numeric component fields, not model names",
                        Some(&decl.name),
                        Some("value"),
                    ));
                }
                let mut numeric_value = |expected: SIUnit| -> Result<Quantity, String> {
                    if let Some(expression) = &decl.value_expression {
                        expression_work += expression.node_count();
                        if expression_work > crate::expression::MAX_EXPRESSION_WORK {
                            return Err("Compile expression work limit exceeded".into());
                        }
                        let resolved = expression
                            .evaluate(&parameter_values, expected)
                            .map_err(|cause| cause.message)?;
                        parameter_manifest
                            .bindings
                            .push(crate::expression::ParameterBinding {
                                component: decl.name.clone(),
                                field: "value".into(),
                                expression: expression.source.clone(),
                                dependencies: expression.dependencies().into_iter().collect(),
                                resolved: resolved.clone(),
                            });
                        Ok(resolved)
                    } else {
                        parse_quantity(val_str, expected)
                    }
                };
                let mut model = None;

                let (kind, params) = match decl.comp_type {
                    ComponentType::ModulePort => (
                        ComponentKind::ModulePort,
                        ComponentParams::ModulePort {
                            module_name: val_str.to_string(),
                            pins: decl.interface_pins.clone(),
                        },
                    ),
                    ComponentType::Resistor => (
                        ComponentKind::Resistor,
                        ComponentParams::TwoPinPassive {
                            value: numeric_value(SIUnit::Ohm).map_err(|error| {
                                semantic_error(
                                    "KES-C001",
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
                            value: numeric_value(SIUnit::Farad).map_err(|error| {
                                semantic_error(
                                    "KES-C001",
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
                            value: numeric_value(SIUnit::Henry).map_err(|error| {
                                semantic_error(
                                    "KES-C001",
                                    format!("invalid inductor value: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })?,
                        },
                    ),
                    ComponentType::Source => {
                        let kind = ComponentKind::VoltageSource;
                        let waveform = if decl.value_expression.is_some() {
                            Ok(None)
                        } else {
                            parse_waveform(val_str, SIUnit::Volt)
                        };
                        let value = if let Some(waveform) = waveform.map_err(|error| {
                            semantic_error(
                                "KES-C002",
                                format!("invalid voltage-source waveform: {error}"),
                                Some(&decl.name),
                                Some("value"),
                            )
                        })? {
                            SourceValue::Waveform(waveform)
                        } else {
                            SourceValue::Dc(numeric_value(SIUnit::Volt).map_err(|error| {
                                semantic_error(
                                    "KES-C001",
                                    format!("invalid voltage-source value: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })?)
                        };
                        (kind, ComponentParams::VoltageSource { value })
                    }
                    ComponentType::CurrentSource => {
                        let kind = ComponentKind::CurrentSource;
                        let waveform = if decl.value_expression.is_some() {
                            Ok(None)
                        } else {
                            parse_waveform(val_str, SIUnit::Ampere)
                        };
                        let value = if let Some(waveform) = waveform.map_err(|error| {
                            semantic_error(
                                "KES-C002",
                                format!("invalid current-source waveform: {error}"),
                                Some(&decl.name),
                                Some("value"),
                            )
                        })? {
                            SourceValue::Waveform(waveform)
                        } else {
                            SourceValue::Dc(numeric_value(SIUnit::Ampere).map_err(|error| {
                                semantic_error(
                                    "KES-C001",
                                    format!("invalid current-source value: {error}"),
                                    Some(&decl.name),
                                    Some("value"),
                                )
                            })?)
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
                        let resolved = model_library.resolve(requested_model).ok_or_else(|| {
                            semantic_error(
                                "KES-C003",
                                format!(
                                    "unsupported BJT model '{requested_model}'; user-defined models are not yet declared by the language"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        let ComponentKind::BJT(model_polarity) = &resolved.kind else {
                            return Err(semantic_error(
                                "KES-C004",
                                format!("model '{requested_model}' is not a BJT model"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        };
                        if let Some(hint) = polarity_hint
                            && hint != *model_polarity
                        {
                            return Err(semantic_error(
                                "KES-C004",
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
                        let resolved = model_library.resolve(requested_model).ok_or_else(|| {
                            semantic_error(
                                "KES-C003",
                                format!(
                                    "unsupported MOSFET model '{requested_model}'; user-defined models are not yet declared by the language"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        let ComponentKind::MOSFET(polarity) = &resolved.kind else {
                            return Err(semantic_error(
                                "KES-C004",
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
                        let resolved = model_library.resolve(requested_model).ok_or_else(|| {
                            semantic_error(
                                "KES-C003",
                                format!(
                                    "unsupported diode model '{requested_model}'; user-defined models are not yet declared by the language"
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        if resolved.kind != ComponentKind::Diode {
                            return Err(semantic_error(
                                "KES-C004",
                                format!("model '{requested_model}' is not a diode model"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        }
                        model = Some(resolved);
                        (ComponentKind::Diode, ComponentParams::DiodeParams)
                    }
                    ComponentType::OpAmp => {
                        let requested_model = if val_str.is_empty() {
                            "KESSETSU_OPAMP_V1"
                        } else {
                            val_str
                        };
                        let resolved = model_library.resolve(requested_model).ok_or_else(|| {
                            semantic_error(
                                "KES-C003",
                                format!("unsupported op-amp model '{requested_model}'"),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        if resolved.kind != ComponentKind::OpAmp {
                            return Err(semantic_error(
                                "KES-C004",
                                format!("model '{requested_model}' is not an op-amp subcircuit"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        }
                        crate::models::validate_component_model_pins(
                            &ComponentKind::OpAmp,
                            &resolved,
                        )?;
                        model = Some(resolved);
                        (ComponentKind::OpAmp, ComponentParams::OpAmpParams)
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
                let metric = assert.metric.to_ascii_lowercase();
                if !is_supported_assertion_metric(&metric) {
                    return Err(semantic_error(
                        "KES-C006",
                        format!(
                            "unsupported assertion metric '{}'; expected value, min, max, peak, average, avg, rms, gain, gain_at, lower_cutoff, upper_cutoff, bandwidth, cutoff, frequency, phase, output_power, dissipation, efficiency, thd or clipping",
                            assert.metric
                        ),
                        None,
                        Some("signal"),
                    ));
                }
                let arguments = split_assertion_arguments(&assert.signal);
                let signal_unit = assertion_result_unit(&metric, &arguments).ok_or_else(|| {
                    let expected = match metric.as_str() {
                        "gain_at" => "gain_at(V(output_net),V(input_net),positive_frequency_in_Hz)",
                        "lower_cutoff" | "upper_cutoff" => {
                            "V(output_net),V(input_net)[,positive_reference_frequency_in_Hz]"
                        }
                        _ => "typed voltage/current/power or engineering metric arguments",
                    };
                    semantic_error(
                        "KES-C006",
                        format!(
                            "assertion '{}' has invalid arguments '{}'; expected {expected}",
                            assert.metric, assert.signal
                        ),
                        None,
                        Some("signal"),
                    )
                })?;
                assertions.push(Assertion {
                    metric: assert.metric.clone(),
                    signal: assert.signal.clone(),
                    cmp: assert.cmp.clone(),
                    threshold: parse_quantity(&assert.threshold, signal_unit).map_err(|error| {
                        semantic_error(
                            "KES-C006",
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
                    "KES-C007",
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
    let mut analysis_keys = std::collections::BTreeSet::new();
    for analysis in &analyses {
        let key = match analysis {
            Analysis::DcSweep { source, .. } => {
                format!("dc:{}", source.to_ascii_lowercase())
            }
            _ => analysis.kind_name().to_string(),
        };
        if !analysis_keys.insert(key) {
            let subject = match analysis {
                Analysis::DcSweep { source, .. } => format!("dc analysis for source '{source}'"),
                _ => format!("'{}' analysis", analysis.kind_name()),
            };
            return Err(semantic_error(
                "KES-C009",
                format!(
                    "duplicate {subject}; ambiguous repeated analyses require named analysis selectors"
                ),
                None,
                Some("analysis"),
            ));
        }
    }

    let model_manifest = model_library.manifest(&components);
    parameter_manifest
        .parameters
        .sort_by(|a, b| (&a.instance_path, &a.name).cmp(&(&b.instance_path, &b.name)));
    parameter_manifest
        .bindings
        .sort_by(|a, b| (&a.component, &a.field).cmp(&(&b.component, &b.field)));
    Ok(CircuitIR {
        components,
        connections,
        nets,
        analyses,
        assertions,
        model_manifest,
        parameter_manifest,
    })
}

fn split_assertion_arguments(arguments: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut result = Vec::new();
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let argument = arguments[start..index].trim();
                if !argument.is_empty() {
                    result.push(argument);
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    let argument = arguments[start..].trim();
    if !argument.is_empty() {
        result.push(argument);
    }
    result
}

fn signal_unit(signal: &str) -> Option<SIUnit> {
    let function = signal.split_once('(')?.0;
    if function.eq_ignore_ascii_case("V") {
        Some(SIUnit::Volt)
    } else if function.eq_ignore_ascii_case("I") {
        Some(SIUnit::Ampere)
    } else if function.eq_ignore_ascii_case("P") {
        Some(SIUnit::Watt)
    } else {
        None
    }
}

fn is_supported_assertion_metric(metric: &str) -> bool {
    matches!(
        metric,
        "value"
            | "min"
            | "max"
            | "peak"
            | "average"
            | "avg"
            | "rms"
            | "gain"
            | "gain_at"
            | "lower_cutoff"
            | "upper_cutoff"
            | "bandwidth"
            | "cutoff"
            | "frequency"
            | "phase"
            | "output_power"
            | "dissipation"
            | "efficiency"
            | "thd"
            | "clipping"
    )
}

fn assertion_result_unit(metric: &str, arguments: &[&str]) -> Option<SIUnit> {
    match metric {
        "value" | "min" | "max" | "peak" | "average" | "avg" | "rms" => {
            if matches!(arguments.len(), 1 | 3) {
                signal_unit(arguments[0])
            } else {
                None
            }
        }
        "gain" => (arguments.len() == 2).then_some(SIUnit::Ratio),
        "gain_at" | "lower_cutoff" | "upper_cutoff" => {
            let count_ok = if metric == "gain_at" {
                arguments.len() == 3
            } else {
                matches!(arguments.len(), 2 | 3)
            };
            if !count_ok
                || arguments[..2].iter().any(|signal| {
                    signal_unit(signal) != Some(SIUnit::Volt)
                        || signal
                            .split_once('(')
                            .and_then(|(_, target)| target.strip_suffix(')'))
                            .is_none_or(|target| {
                                target.is_empty() || target.contains([',', '(', ')'])
                            })
                })
            {
                return None;
            }
            if let Some(frequency) = arguments.get(2) {
                let quantity = parse_quantity(frequency, SIUnit::Hertz).ok()?;
                if !quantity.value.is_finite() || quantity.value <= 0.0 {
                    return None;
                }
            }
            Some(if metric == "gain_at" {
                SIUnit::Ratio
            } else {
                SIUnit::Hertz
            })
        }
        "bandwidth" | "cutoff" => (arguments.len() == 2).then_some(SIUnit::Hertz),
        "frequency" => (arguments.len() == 1).then_some(SIUnit::Hertz),
        "phase" => matches!(arguments.len(), 2 | 3).then_some(SIUnit::Degree),
        "output_power" => matches!(arguments.len(), 2 | 4).then_some(SIUnit::Watt),
        "dissipation" => matches!(arguments.len(), 1 | 3).then_some(SIUnit::Watt),
        "efficiency" => matches!(arguments.len(), 4 | 6 | 8).then_some(SIUnit::Percent),
        "thd" => (arguments.len() == 5).then_some(SIUnit::Percent),
        "clipping" => (arguments.len() == 3).then_some(SIUnit::Percent),
        _ => None,
    }
}
