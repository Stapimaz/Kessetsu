use crate::ast::{ComponentType, Connection, Program, Statement};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

pub const PHYSICAL_PART_SCHEMA_VERSION: &str = "kessetsu.physical-parts.v2";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitIR {
    pub components: Vec<IRComponent>,
    pub connections: Vec<Connection>, // Preserved from AST for graph generation
    pub nets: Vec<String>,            // User-named nets
    pub analyses: Vec<Analysis>,
    pub assertions: Vec<Assertion>,
    pub model_manifest: ModelManifest,
    #[serde(default, skip_serializing_if = "PhysicalPartManifest::is_empty")]
    pub physical_parts: PhysicalPartManifest,
    #[serde(
        default,
        skip_serializing_if = "crate::expression::ParameterManifest::is_empty"
    )]
    pub parameter_manifest: crate::expression::ParameterManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalPartAssignment {
    pub component: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mpn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pin_map: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ratings: Vec<ProvidedPartRating>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartRatingKind {
    PeakVoltage,
    PeakCurrent,
    AverageDissipation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvidedPartRating {
    pub kind: PartRatingKind,
    pub limit: Quantity,
    pub conditions: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalPartManifest {
    pub schema_version: String,
    pub assignments: Vec<PhysicalPartAssignment>,
}

impl Default for PhysicalPartManifest {
    fn default() -> Self {
        Self {
            schema_version: PHYSICAL_PART_SCHEMA_VERSION.into(),
            assignments: Vec::new(),
        }
    }
}

impl PhysicalPartManifest {
    pub fn is_empty(&self) -> bool {
        self.assignments.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IRComponent {
    pub id: String,
    /// Structured reusable-block ancestry. Electrical identifiers remain flat
    /// and canonical; presentation layers may use this path for navigation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_path: Vec<String>,
    pub kind: ComponentKind,
    pub parameters: ComponentParams,
    pub model: Option<ModelRef>,
    /// Resolved per-instance values keyed by the model's exact parameter name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub instance_parameters: BTreeMap<String, Quantity>,
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
    ExternalDevice(ExternalDeviceFamily),
    VoltageSource,
    CurrentSource,
    ModulePort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalDeviceFamily {
    Comparator,
    TwoTerminal,
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
    ExternalDeviceParams,
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
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        instance_path: Vec<String>,
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
    Joule,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_parameters: Vec<ExternalParameterDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalParameterDefinition {
    pub name: String,
    pub unit: SIUnit,
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
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        use_initial_conditions: bool,
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
    /// Resolved expression slots are authoritative; signal retains source text
    /// for display and compatibility with existing literal assertions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numeric_arguments: Vec<ResolvedArgument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedArgument {
    pub index: usize,
    pub quantity: Quantity,
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
        "J" | "j" => Some(SIUnit::Joule),
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

    make_waveform(name, parts.len(), value_unit, |index, unit, _field| {
        parse_quantity(parts[index], unit)
    })
    .map(Some)
}

/// One waveform constructor for legacy literals and typed numeric expressions.
fn make_waveform(
    name: &str,
    count: usize,
    value_unit: SIUnit,
    mut numeric: impl FnMut(usize, SIUnit, &str) -> Result<Quantity, String>,
) -> Result<Waveform, String> {
    if name.eq_ignore_ascii_case("ac") {
        if count != 1 {
            return Err(format!("AC expects exactly 1 parameter, got {}", count));
        }
        return Ok(Waveform::Ac {
            amplitude: numeric(0, value_unit, "waveform.amplitude")?,
        });
    }

    if name.eq_ignore_ascii_case("sine") {
        if count != 3 {
            return Err(format!("SINE expects exactly 3 parameters, got {}", count));
        }
        return Ok(Waveform::Sine {
            offset: numeric(0, value_unit, "waveform.offset")?,
            amplitude: numeric(1, value_unit, "waveform.amplitude")?,
            frequency: numeric(2, SIUnit::Hertz, "waveform.frequency")?,
        });
    }

    if name.eq_ignore_ascii_case("sine_ac") {
        if count != 4 {
            return Err(format!(
                "SINE_AC expects exactly 4 parameters, got {}",
                count
            ));
        }
        return Ok(Waveform::SineAc {
            offset: numeric(0, value_unit, "waveform.offset")?,
            amplitude: numeric(1, value_unit, "waveform.amplitude")?,
            frequency: numeric(2, SIUnit::Hertz, "waveform.frequency")?,
            ac_amplitude: numeric(3, value_unit, "waveform.ac_amplitude")?,
        });
    }

    if name.eq_ignore_ascii_case("pulse") {
        if count != 7 {
            return Err(format!("PULSE expects exactly 7 parameters, got {}", count));
        }
        return Ok(Waveform::Pulse {
            v1: numeric(0, value_unit, "waveform.v1")?,
            v2: numeric(1, value_unit, "waveform.v2")?,
            delay: numeric(2, SIUnit::Second, "waveform.delay")?,
            rise: numeric(3, SIUnit::Second, "waveform.rise")?,
            fall: numeric(4, SIUnit::Second, "waveform.fall")?,
            width: numeric(5, SIUnit::Second, "waveform.width")?,
            period: numeric(6, SIUnit::Second, "waveform.period")?,
        });
    }

    if name.eq_ignore_ascii_case("pwl") {
        if count < 4 || !count.is_multiple_of(2) {
            return Err(format!(
                "PWL expects at least 2 time/value pairs, got {} parameters",
                count
            ));
        }
        let mut points = Vec::with_capacity(count / 2);
        for index in (0..count).step_by(2) {
            let time = numeric(
                index,
                SIUnit::Second,
                &format!("waveform.points[{}].time", index / 2),
            )?;
            let value = numeric(
                index + 1,
                value_unit,
                &format!("waveform.points[{}].value", index / 2),
            )?;
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
        return Ok(Waveform::PWL { points });
    }

    Err(format!("unsupported waveform '{name}'"))
}

fn evaluate_numeric(
    expression: &crate::expression::Expression,
    expected: SIUnit,
    values: &std::collections::BTreeMap<String, Quantity>,
    work: &mut usize,
) -> Result<Quantity, String> {
    *work += expression.node_count();
    if *work > crate::expression::MAX_EXPRESSION_WORK {
        return Err("Compile expression work limit exceeded".into());
    }
    expression
        .evaluate(values, expected)
        .map_err(|cause| cause.message)
}

fn resolve_instance_parameters(
    declaration: &crate::ast::ComponentDecl,
    model: &ModelRef,
    parameter_values: &BTreeMap<String, Quantity>,
    expression_work: &mut usize,
    parameter_manifest: &mut crate::expression::ParameterManifest,
) -> Result<BTreeMap<String, Quantity>, SemanticDiagnostic> {
    if declaration.instance_parameters.is_empty() {
        return Ok(BTreeMap::new());
    }
    let ModelDefinition::ExternalSubcircuit { metadata } = &model.definition else {
        return Err(semantic_error(
            "KES-C022",
            format!(
                "model '{}' does not expose typed per-instance parameters",
                model.name
            ),
            Some(&declaration.name),
            Some("instance_parameters"),
        ));
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut resolved = BTreeMap::new();
    for override_value in &declaration.instance_parameters {
        let Some(definition) = metadata
            .instance_parameters
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(&override_value.name))
        else {
            return Err(semantic_error(
                "KES-C022",
                format!(
                    "model '{}' does not expose an instance parameter named '{}'; available parameters: {}",
                    model.name,
                    override_value.name,
                    if metadata.instance_parameters.is_empty() {
                        "none".to_string()
                    } else {
                        metadata
                            .instance_parameters
                            .iter()
                            .map(|definition| definition.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ),
                Some(&declaration.name),
                Some(&override_value.name),
            ));
        };
        if !seen.insert(definition.name.to_ascii_lowercase()) {
            return Err(semantic_error(
                "KES-C022",
                format!("duplicate instance parameter '{}'", definition.name),
                Some(&declaration.name),
                Some(&definition.name),
            ));
        }
        let quantity = evaluate_numeric(
            &override_value.expression,
            definition.unit,
            parameter_values,
            expression_work,
        )
        .map_err(|cause| {
            semantic_error(
                "KES-C022",
                format!(
                    "invalid value for instance parameter '{}': {cause}",
                    definition.name
                ),
                Some(&declaration.name),
                Some(&definition.name),
            )
        })?;
        parameter_manifest
            .bindings
            .push(crate::expression::ParameterBinding {
                component: declaration.name.clone(),
                field: format!("instance_parameter.{}", definition.name),
                expression: override_value.expression.source.clone(),
                dependencies: override_value
                    .expression
                    .dependencies()
                    .into_iter()
                    .collect(),
                resolved: quantity.clone(),
            });
        resolved.insert(definition.name.clone(), quantity);
    }
    Ok(resolved)
}

pub fn resolve_model(name: &str) -> Option<ModelRef> {
    crate::models::builtin_model(name)
}

fn parse_analysis(
    command: &crate::ast::SimulateStmt,
    components: &[IRComponent],
    mut numeric: impl FnMut(usize, SIUnit, &str) -> Result<Quantity, String>,
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
            if command.args.len() != 2
                && !(command.args.len() == 3 && command.args[2].eq_ignore_ascii_case("uic"))
            {
                return Err(invalid(format!(
                    "simulate tran expects step and stop, optionally followed by uic; got {} arguments",
                    command.args.len()
                )));
            }
            let step = numeric(0, SIUnit::Second, "step")
                .map_err(|error| invalid(format!("invalid transient step: {error}")))?;
            let stop = numeric(1, SIUnit::Second, "stop")
                .map_err(|error| invalid(format!("invalid transient stop: {error}")))?;
            if step.value <= 0.0 || stop.value <= 0.0 || step.value > stop.value {
                return Err(invalid(
                    "transient step and stop must be positive, with step <= stop".to_string(),
                ));
            }
            Ok(Analysis::Transient {
                step,
                stop,
                use_initial_conditions: command.args.len() == 3,
            })
        }
        "ac" => {
            if command.args.len() != 4 {
                return Err(invalid(format!(
                    "simulate ac expects scale, points, start and stop, got {} arguments",
                    command.args.len()
                )));
            }
            if command.numeric_expression(0).is_some() {
                return Err(invalid(
                    "AC scale is a keyword (dec, oct or lin), not a numeric expression".into(),
                ));
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
            let points = if command.numeric_expression(1).is_some() {
                let quantity = numeric(1, SIUnit::Ratio, "points")
                    .map_err(|error| invalid(format!("invalid AC points: {error}")))?;
                if quantity.value < 1.0
                    || quantity.value > u32::MAX as f64
                    || quantity.value.fract() != 0.0
                {
                    return Err(invalid(
                        "AC points must be a positive integer within the supported range".into(),
                    ));
                }
                quantity.value as u32
            } else {
                command.args[1]
                    .parse::<u32>()
                    .map_err(|_| invalid("AC points must be a positive integer".to_string()))?
            };
            let start = numeric(2, SIUnit::Hertz, "start")
                .map_err(|error| invalid(format!("invalid AC start frequency: {error}")))?;
            let stop = numeric(3, SIUnit::Hertz, "stop")
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
            if command.numeric_expression(0).is_some() {
                return Err(invalid(
                    "DC sweep target must be a source name, not a numeric expression".into(),
                ));
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
            let start = numeric(1, unit, "start")
                .map_err(|error| invalid(format!("invalid DC sweep start: {error}")))?;
            let stop = numeric(2, unit, "stop")
                .map_err(|error| invalid(format!("invalid DC sweep stop: {error}")))?;
            let step = numeric(3, unit, "step")
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
    let mut part_assignments = Vec::new();

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
                let mut numeric_value = |input: &str,
                                         expression: Option<&crate::expression::Expression>,
                                         expected: SIUnit,
                                         field: &str|
                 -> Result<Quantity, String> {
                    if let Some(expression) = expression {
                        let resolved = evaluate_numeric(
                            expression,
                            expected,
                            &parameter_values,
                            &mut expression_work,
                        )?;
                        parameter_manifest
                            .bindings
                            .push(crate::expression::ParameterBinding {
                                component: decl.name.clone(),
                                field: field.into(),
                                expression: expression.source.clone(),
                                dependencies: expression.dependencies().into_iter().collect(),
                                resolved: resolved.clone(),
                            });
                        Ok(resolved)
                    } else {
                        parse_quantity(input, expected)
                    }
                };
                let mut model = None;
                let mut instance_parameters = BTreeMap::new();

                let (kind, params) = match decl.comp_type {
                    ComponentType::ModulePort => (
                        ComponentKind::ModulePort,
                        ComponentParams::ModulePort {
                            module_name: val_str.to_string(),
                            pins: decl.interface_pins.clone(),
                            instance_path: decl.instance_path.clone(),
                        },
                    ),
                    ComponentType::Resistor => (
                        ComponentKind::Resistor,
                        ComponentParams::TwoPinPassive {
                            value: numeric_value(
                                val_str,
                                decl.value_expression.as_ref(),
                                SIUnit::Ohm,
                                "value",
                            )
                            .map_err(|error| {
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
                            value: numeric_value(
                                val_str,
                                decl.value_expression.as_ref(),
                                SIUnit::Farad,
                                "value",
                            )
                            .map_err(|error| {
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
                            value: numeric_value(
                                val_str,
                                decl.value_expression.as_ref(),
                                SIUnit::Henry,
                                "value",
                            )
                            .map_err(|error| {
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
                        } else if let Some(call) = &decl.waveform_expression {
                            make_waveform(
                                &call.name,
                                call.args.len(),
                                SIUnit::Volt,
                                |index, unit, field| {
                                    let arg = &call.args[index];
                                    numeric_value(&arg.value, arg.expression.as_ref(), unit, field)
                                        .map_err(|error| format!("{field}: {error}"))
                                },
                            )
                            .map(Some)
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
                            SourceValue::Dc(
                                numeric_value(
                                    val_str,
                                    decl.value_expression.as_ref(),
                                    SIUnit::Volt,
                                    "value",
                                )
                                .map_err(|error| {
                                    semantic_error(
                                        "KES-C001",
                                        format!("invalid voltage-source value: {error}"),
                                        Some(&decl.name),
                                        Some("value"),
                                    )
                                })?,
                            )
                        };
                        (kind, ComponentParams::VoltageSource { value })
                    }
                    ComponentType::CurrentSource => {
                        let kind = ComponentKind::CurrentSource;
                        let waveform = if decl.value_expression.is_some() {
                            Ok(None)
                        } else if let Some(call) = &decl.waveform_expression {
                            make_waveform(
                                &call.name,
                                call.args.len(),
                                SIUnit::Ampere,
                                |index, unit, field| {
                                    let arg = &call.args[index];
                                    numeric_value(&arg.value, arg.expression.as_ref(), unit, field)
                                        .map_err(|error| format!("{field}: {error}"))
                                },
                            )
                            .map(Some)
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
                            SourceValue::Dc(
                                numeric_value(
                                    val_str,
                                    decl.value_expression.as_ref(),
                                    SIUnit::Ampere,
                                    "value",
                                )
                                .map_err(|error| {
                                    semantic_error(
                                        "KES-C001",
                                        format!("invalid current-source value: {error}"),
                                        Some(&decl.name),
                                        Some("value"),
                                    )
                                })?,
                            )
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
                        instance_parameters = resolve_instance_parameters(
                            decl,
                            &resolved,
                            &parameter_values,
                            &mut expression_work,
                            &mut parameter_manifest,
                        )?;
                        model = Some(resolved);
                        (ComponentKind::OpAmp, ComponentParams::OpAmpParams)
                    }
                    ComponentType::ExternalDevice => {
                        let resolved = model_library.resolve(val_str).ok_or_else(|| {
                            semantic_error(
                                "KES-C003",
                                format!(
                                    "device '{}' requires a declared external device model",
                                    decl.name
                                ),
                                Some(&decl.name),
                                Some("model"),
                            )
                        })?;
                        if !matches!(resolved.kind, ComponentKind::ExternalDevice(_)) {
                            return Err(semantic_error(
                                "KES-C004",
                                format!("model '{val_str}' is not an external device interface"),
                                Some(&decl.name),
                                Some("model"),
                            ));
                        }
                        crate::models::validate_component_model_pins(&resolved.kind, &resolved)?;
                        instance_parameters = resolve_instance_parameters(
                            decl,
                            &resolved,
                            &parameter_values,
                            &mut expression_work,
                            &mut parameter_manifest,
                        )?;
                        let kind = resolved.kind.clone();
                        model = Some(resolved);
                        (kind, ComponentParams::ExternalDeviceParams)
                    }
                };

                components.push(IRComponent {
                    id: decl.name.clone(),
                    instance_path: decl.instance_path.clone(),
                    kind,
                    parameters: params,
                    model,
                    instance_parameters,
                });
            }
            Statement::Connect(conn) => {
                connections.push(conn.clone());
            }
            Statement::Net(net) => {
                nets.push(net.name.clone());
            }
            Statement::Part(part) => {
                part_assignments.push(part.clone());
            }
            Statement::Assert(assert) => {
                let metric = assert.metric.to_ascii_lowercase();
                if !is_supported_assertion_metric(&metric) {
                    return Err(semantic_error(
                        "KES-C006",
                        format!(
                            "unsupported assertion metric '{}'; expected value, min, max, peak, average, avg, rms, gain, gain_at, lower_cutoff, upper_cutoff, bandwidth, cutoff, frequency, phase, output_power, dissipation, efficiency, thd, clipping, rise_time, fall_time, settling_time, overshoot or energy",
                            assert.metric
                        ),
                        None,
                        Some("signal"),
                    ));
                }
                let arguments = split_assertion_arguments(&assert.signal);
                let mut numeric_arguments = Vec::new();
                for argument in &assert.numeric_expressions {
                    let unit = assertion_numeric_unit(&metric, &arguments, argument.index).ok_or_else(|| semantic_error("KES-C006", format!("assertion '{}': argument {} is not a supported numeric field; signal, component and policy names must be literal", assert.metric, argument.index + 1), None, Some("signal")))?;
                    let quantity = evaluate_numeric(
                        &argument.expression,
                        unit,
                        &parameter_values,
                        &mut expression_work,
                    )
                    .map_err(|error| {
                        semantic_error(
                            "KES-C006",
                            format!(
                                "assertion '{}', argument {}: {error}",
                                assert.metric,
                                argument.index + 1
                            ),
                            None,
                            Some("signal"),
                        )
                    })?;
                    parameter_manifest.assertion_bindings.push(
                        crate::expression::AssertionParameterBinding {
                            assertion_index: assertions.len(),
                            field: format!("arguments[{}]", argument.index),
                            expression: argument.expression.source.clone(),
                            dependencies: argument.expression.dependencies().into_iter().collect(),
                            resolved: quantity.clone(),
                        },
                    );
                    numeric_arguments.push(ResolvedArgument {
                        index: argument.index,
                        quantity,
                    });
                }
                let signal_unit = assertion_result_unit(&metric, &arguments, &numeric_arguments)
                    .ok_or_else(|| {
                        let expected = match metric.as_str() {
                            "gain_at" => {
                                "gain_at(V(output_net),V(input_net),positive_frequency_in_Hz)"
                            }
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
                let threshold = if let Some(expression) = &assert.threshold_expression {
                    let quantity = evaluate_numeric(
                        expression,
                        signal_unit,
                        &parameter_values,
                        &mut expression_work,
                    )
                    .map_err(|error| {
                        semantic_error(
                            "KES-C006",
                            format!("invalid assertion threshold: {error}"),
                            None,
                            Some("threshold"),
                        )
                    })?;
                    parameter_manifest.assertion_bindings.push(
                        crate::expression::AssertionParameterBinding {
                            assertion_index: assertions.len(),
                            field: "threshold".into(),
                            expression: expression.source.clone(),
                            dependencies: expression.dependencies().into_iter().collect(),
                            resolved: quantity.clone(),
                        },
                    );
                    quantity
                } else {
                    parse_quantity(&assert.threshold, signal_unit).map_err(|error| {
                        semantic_error(
                            "KES-C006",
                            format!(
                                "invalid assertion threshold for '{}': {error}",
                                assert.signal
                            ),
                            None,
                            Some("threshold"),
                        )
                    })?
                };
                assertions.push(Assertion {
                    metric: assert.metric.clone(),
                    signal: assert.signal.clone(),
                    cmp: assert.cmp.clone(),
                    threshold,
                    numeric_arguments,
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
        .enumerate()
        .map(|(analysis_index, analysis)| {
            parse_analysis(analysis, &components, |index, unit, field| {
                if let Some(expression) = analysis.numeric_expression(index) {
                    let resolved = evaluate_numeric(
                        expression,
                        unit,
                        &parameter_values,
                        &mut expression_work,
                    )?;
                    parameter_manifest.analysis_bindings.push(
                        crate::expression::AnalysisParameterBinding {
                            analysis_index,
                            field: field.into(),
                            expression: expression.source.clone(),
                            dependencies: expression.dependencies().into_iter().collect(),
                            resolved: resolved.clone(),
                        },
                    );
                    Ok(resolved)
                } else {
                    parse_quantity(&analysis.args[index], unit)
                }
            })
        })
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
    parameter_manifest
        .analysis_bindings
        .sort_by(|a, b| (a.analysis_index, &a.field).cmp(&(b.analysis_index, &b.field)));
    parameter_manifest
        .assertion_bindings
        .sort_by(|a, b| (a.assertion_index, &a.field).cmp(&(b.assertion_index, &b.field)));
    let physical_parts = compile_physical_parts(&part_assignments, &components)?;
    Ok(CircuitIR {
        components,
        connections,
        nets,
        analyses,
        assertions,
        model_manifest,
        physical_parts,
        parameter_manifest,
    })
}

fn compile_physical_parts(
    declarations: &[crate::ast::PartAssignmentDecl],
    components: &[IRComponent],
) -> Result<PhysicalPartManifest, SemanticDiagnostic> {
    let mut assignments = Vec::new();
    let mut assigned = BTreeMap::<String, &crate::ast::PartAssignmentDecl>::new();
    for declaration in declarations {
        if let Some(previous) = assigned.insert(declaration.component.clone(), declaration) {
            return Err(semantic_error(
                "KES-C024",
                format!(
                    "physical part for '{}' is assigned more than once (first assignment at {}:{})",
                    declaration.component, previous.line, previous.column
                ),
                Some(&declaration.component),
                Some("part"),
            ));
        }
        let component = components
            .iter()
            .find(|component| component.id == declaration.component)
            .ok_or_else(|| {
                semantic_error(
                    "KES-C024",
                    format!(
                        "physical part target '{}' is not a declared component",
                        declaration.component
                    ),
                    Some(&declaration.component),
                    Some("part"),
                )
            })?;
        if component.kind == ComponentKind::ModulePort {
            return Err(semantic_error(
                "KES-C024",
                "module interfaces are virtual and cannot receive a physical part",
                Some(&declaration.component),
                Some("part"),
            ));
        }
        if matches!(
            component.kind,
            ComponentKind::VoltageSource | ComponentKind::CurrentSource
        ) {
            return Err(semantic_error(
                "KES-C024",
                "independent simulation sources are abstract stimuli and cannot receive a physical part",
                Some(&declaration.component),
                Some("part"),
            ));
        }

        let mut fields = BTreeMap::<String, String>::new();
        for field in &declaration.fields {
            let key = field.name.to_ascii_lowercase();
            if !matches!(
                key.as_str(),
                "manufacturer"
                    | "mpn"
                    | "footprint"
                    | "pin_map"
                    | "note"
                    | "peak_voltage_limit"
                    | "peak_voltage_conditions"
                    | "peak_current_limit"
                    | "peak_current_conditions"
                    | "average_dissipation_limit"
                    | "average_dissipation_conditions"
                    | "rating_source"
            ) {
                return Err(semantic_error(
                    "KES-C024",
                    format!(
                        "unknown physical-part field '{}'; use identity/footprint fields or an explicit provided rating",
                        field.name
                    ),
                    Some(&declaration.component),
                    Some(&field.name),
                ));
            }
            if fields.insert(key, field.value.clone()).is_some() {
                return Err(semantic_error(
                    "KES-C024",
                    format!("duplicate physical-part field '{}'", field.name),
                    Some(&declaration.component),
                    Some(&field.name),
                ));
            }
        }
        for (name, value) in &fields {
            let max = if matches!(name.as_str(), "note" | "rating_source") {
                1024
            } else if name.ends_with("_conditions") {
                512
            } else {
                256
            };
            if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
                return Err(semantic_error(
                    "KES-C024",
                    format!(
                        "physical-part field '{name}' must be non-empty, control-free and at most {max} bytes"
                    ),
                    Some(&declaration.component),
                    Some(name),
                ));
            }
        }

        let footprint = fields.get("footprint").cloned();
        let pin_map = if let Some(mapping) = fields.get("pin_map") {
            if footprint.is_none() {
                return Err(semantic_error(
                    "KES-C024",
                    "pin_map requires an explicit footprint",
                    Some(&declaration.component),
                    Some("pin_map"),
                ));
            }
            parse_physical_pin_map(&declaration.component, mapping, component)?
        } else {
            BTreeMap::new()
        };
        let rating_source = fields.get("rating_source").cloned();
        let mut ratings = Vec::new();
        for (kind, limit_field, conditions_field, unit) in [
            (
                PartRatingKind::PeakVoltage,
                "peak_voltage_limit",
                "peak_voltage_conditions",
                SIUnit::Volt,
            ),
            (
                PartRatingKind::PeakCurrent,
                "peak_current_limit",
                "peak_current_conditions",
                SIUnit::Ampere,
            ),
            (
                PartRatingKind::AverageDissipation,
                "average_dissipation_limit",
                "average_dissipation_conditions",
                SIUnit::Watt,
            ),
        ] {
            match (fields.get(limit_field), fields.get(conditions_field)) {
                (Some(limit), Some(conditions)) => {
                    let limit = parse_quantity(limit, unit).map_err(|message| {
                        semantic_error(
                            "KES-C024",
                            format!("invalid {limit_field}: {message}"),
                            Some(&declaration.component),
                            Some(limit_field),
                        )
                    })?;
                    if !limit.value.is_finite() || limit.value <= 0.0 {
                        return Err(semantic_error(
                            "KES-C024",
                            format!("{limit_field} must be finite and greater than zero"),
                            Some(&declaration.component),
                            Some(limit_field),
                        ));
                    }
                    ratings.push(ProvidedPartRating {
                        kind,
                        limit,
                        conditions: conditions.clone(),
                        source: rating_source.clone(),
                    });
                }
                (Some(_), None) => {
                    return Err(semantic_error(
                        "KES-C024",
                        format!("{limit_field} requires {conditions_field}"),
                        Some(&declaration.component),
                        Some(conditions_field),
                    ));
                }
                (None, Some(_)) => {
                    return Err(semantic_error(
                        "KES-C024",
                        format!("{conditions_field} requires {limit_field}"),
                        Some(&declaration.component),
                        Some(limit_field),
                    ));
                }
                (None, None) => {}
            }
        }
        if rating_source.is_some() && ratings.is_empty() {
            return Err(semantic_error(
                "KES-C024",
                "rating_source requires at least one provided rating limit",
                Some(&declaration.component),
                Some("rating_source"),
            ));
        }
        assignments.push(PhysicalPartAssignment {
            component: declaration.component.clone(),
            manufacturer: fields.get("manufacturer").cloned(),
            mpn: fields.get("mpn").cloned(),
            footprint,
            pin_map,
            note: fields.get("note").cloned(),
            ratings,
        });
    }
    assignments.sort_by(|left, right| left.component.cmp(&right.component));
    Ok(PhysicalPartManifest {
        schema_version: PHYSICAL_PART_SCHEMA_VERSION.into(),
        assignments,
    })
}

fn parse_physical_pin_map(
    component_id: &str,
    input: &str,
    component: &IRComponent,
) -> Result<BTreeMap<String, String>, SemanticDiagnostic> {
    let expected = crate::component::component_definition(&component.kind)
        .pins
        .iter()
        .map(|pin| pin.name)
        .collect::<Vec<_>>();
    let mut mapping = BTreeMap::new();
    let mut physical = std::collections::BTreeSet::new();
    for entry in input.split(',') {
        let Some((logical, pad)) = entry.trim().split_once(':') else {
            return Err(semantic_error(
                "KES-C024",
                format!("invalid pin_map entry '{entry}'; expected logical_pin:physical_pad"),
                Some(component_id),
                Some("pin_map"),
            ));
        };
        let logical = logical.trim();
        let pad = pad.trim();
        if !expected.contains(&logical) {
            return Err(semantic_error(
                "KES-C024",
                format!(
                    "pin_map names unknown logical pin '{logical}'; expected {}",
                    expected.join(", ")
                ),
                Some(component_id),
                Some("pin_map"),
            ));
        }
        if pad.is_empty()
            || !pad.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            return Err(semantic_error(
                "KES-C024",
                format!("physical pad '{pad}' must use letters, digits, '_', '-' or '.'"),
                Some(component_id),
                Some("pin_map"),
            ));
        }
        if mapping
            .insert(logical.to_string(), pad.to_string())
            .is_some()
        {
            return Err(semantic_error(
                "KES-C024",
                format!("logical pin '{logical}' is mapped more than once"),
                Some(component_id),
                Some("pin_map"),
            ));
        }
        if !physical.insert(pad.to_string()) {
            return Err(semantic_error(
                "KES-C024",
                format!("physical pad '{pad}' is assigned more than once"),
                Some(component_id),
                Some("pin_map"),
            ));
        }
    }
    let missing = expected
        .iter()
        .filter(|pin| !mapping.contains_key(**pin))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(semantic_error(
            "KES-C024",
            format!(
                "pin_map is incomplete; add mappings for {}",
                missing.join(", ")
            ),
            Some(component_id),
            Some("pin_map"),
        ));
    }
    Ok(mapping)
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
            | "rise_time"
            | "fall_time"
            | "settling_time"
            | "overshoot"
            | "energy"
    )
}

fn assertion_result_unit(
    metric: &str,
    arguments: &[&str],
    numeric: &[ResolvedArgument],
) -> Option<SIUnit> {
    if matches!(
        metric,
        "rise_time" | "fall_time" | "settling_time" | "overshoot" | "energy"
    ) {
        let count = if metric == "energy" { 4 } else { 5 };
        if arguments.len() != count {
            return None;
        }
        let mut values = Vec::new();
        for index in (if metric == "energy" { 2 } else { 1 })..count {
            let unit = assertion_numeric_unit(metric, arguments, index)?;
            let quantity = numeric
                .iter()
                .find(|a| a.index == index)
                .map(|a| Ok(a.quantity.clone()))
                .unwrap_or_else(|| parse_quantity(arguments[index], unit))
                .ok()?;
            values.push(quantity.value);
        }
        let n = values.len();
        if values[n - 2] < 0.0
            || values[n - 2] >= values[n - 1]
            || (matches!(metric, "rise_time" | "fall_time") && values[0] >= values[1])
            || (metric == "settling_time" && values[1] <= 0.0)
            || (metric == "overshoot" && values[0] == values[1])
        {
            return None;
        }
    }
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
                let quantity = numeric
                    .iter()
                    .find(|argument| argument.index == 2)
                    .map(|argument| Ok(argument.quantity.clone()))
                    .unwrap_or_else(|| parse_quantity(frequency, SIUnit::Hertz))
                    .ok()?;
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
        "rise_time" | "fall_time" | "settling_time" => (arguments.len() == 5
            && signal_unit(arguments[0]) == Some(SIUnit::Volt))
        .then_some(SIUnit::Second),
        "overshoot" => (arguments.len() == 5 && signal_unit(arguments[0]) == Some(SIUnit::Volt))
            .then_some(SIUnit::Percent),
        "energy" => (arguments.len() == 4
            && signal_unit(arguments[0]) == Some(SIUnit::Volt)
            && signal_unit(arguments[1]) == Some(SIUnit::Ampere))
        .then_some(SIUnit::Joule),
        _ => None,
    }
}

fn assertion_numeric_unit(metric: &str, arguments: &[&str], index: usize) -> Option<SIUnit> {
    let count = arguments.len();
    match metric {
        "value" | "min" | "max" | "peak" | "average" | "avg" | "rms" | "dissipation"
            if count == 3 && matches!(index, 1 | 2) =>
        {
            Some(SIUnit::Second)
        }
        "gain_at" | "lower_cutoff" | "upper_cutoff" | "phase" if count == 3 && index == 2 => {
            Some(SIUnit::Hertz)
        }
        "output_power" if count == 4 && matches!(index, 2 | 3) => Some(SIUnit::Second),
        "efficiency"
            if matches!(count, 6 | 8)
                && index >= count - 2
                && arguments[count - 2..].iter().all(|arg| {
                    arg.starts_with('{') || parse_quantity(arg, SIUnit::Second).is_ok()
                }) =>
        {
            Some(SIUnit::Second)
        }
        "thd" if count == 5 && index == 1 => Some(SIUnit::Hertz),
        "thd" if count == 5 && matches!(index, 2 | 3) => Some(SIUnit::Second),
        "clipping" if count == 3 && matches!(index, 1 | 2) => Some(SIUnit::Volt),
        "rise_time" | "fall_time" | "settling_time" | "overshoot"
            if count == 5 && matches!(index, 1 | 2) =>
        {
            Some(SIUnit::Volt)
        }
        "rise_time" | "fall_time" | "settling_time" | "overshoot"
            if count == 5 && matches!(index, 3 | 4) =>
        {
            Some(SIUnit::Second)
        }
        "energy" if count == 4 && matches!(index, 2 | 3) => Some(SIUnit::Second),
        _ => None,
    }
}
