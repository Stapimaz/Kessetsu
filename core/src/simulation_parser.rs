use crate::ir::Analysis;
use crate::simulation::{
    ComplexSeries, ComplexSeriesDataset, Dataset, RealSeriesDataset, SeriesAxis,
    SimulatorDiagnostic, SimulatorDiagnosticKind, SimulatorDiagnosticSeverity,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationParseError {
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for SimulationParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(formatter, "line {line}: {}", self.message)
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for SimulationParseError {}

pub fn parse_measurements(
    stdout: &str,
) -> Result<BTreeMap<String, f64>, Vec<SimulationParseError>> {
    let mut measurements = BTreeMap::new();
    let mut errors = Vec::new();

    for (index, raw_line) in stdout.lines().enumerate() {
        let line = raw_line.trim();
        let Some((raw_name, raw_value)) = line.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        if !is_measurement_name(name) {
            continue;
        }
        let Some(value_token) = raw_value.split_whitespace().next() else {
            errors.push(parse_error(
                index + 1,
                format!("measurement '{name}' has no value"),
            ));
            continue;
        };
        let value = match parse_ngspice_number(value_token) {
            Ok(value) => value,
            Err(message) => {
                errors.push(parse_error(
                    index + 1,
                    format!("measurement '{name}' has invalid value: {message}"),
                ));
                continue;
            }
        };
        let canonical_name = name.to_ascii_lowercase();
        if measurements.insert(canonical_name.clone(), value).is_some() {
            errors.push(parse_error(
                index + 1,
                format!("duplicate measurement '{canonical_name}'"),
            ));
        }
    }

    if errors.is_empty() {
        Ok(measurements)
    } else {
        Err(errors)
    }
}

pub fn parse_wrdata(analysis: &Analysis, input: &str) -> Result<Dataset, SimulationParseError> {
    let mut lines = input.lines().enumerate().filter_map(|(index, line)| {
        let line = line.trim();
        (!line.is_empty()).then_some((index + 1, line))
    });
    let Some((header_line, header)) = lines.next() else {
        return Err(parse_error_without_line("wrdata output is empty"));
    };
    let headers = header
        .split_whitespace()
        .map(canonical_vector_name)
        .collect::<Vec<_>>();
    if headers.len() < 2 {
        return Err(parse_error(
            header_line,
            "wrdata header must contain a scale and at least one vector".to_string(),
        ));
    }

    let rows = lines
        .map(|(line_number, line)| parse_row(line_number, line, headers.len()))
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Err(parse_error_without_line("wrdata output has no data rows"));
    }

    match analysis {
        Analysis::OperatingPoint => parse_operating_point(&headers, &rows),
        Analysis::Transient { .. } => {
            parse_real_series(&headers, &rows, AxisOrder::Increasing).map(Dataset::Transient)
        }
        Analysis::DcSweep { .. } => {
            parse_real_series(&headers, &rows, AxisOrder::Either).map(Dataset::DcSweep)
        }
        Analysis::Ac { .. } => parse_ac_series(&headers, &rows).map(Dataset::Ac),
    }
}

pub fn classify_simulator_log(stdout: &str, stderr: &str) -> Vec<SimulatorDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();

    for line in stdout.lines().chain(stderr.lines()) {
        let message = line.trim();
        if message.is_empty() {
            continue;
        }
        let lower = message.to_ascii_lowercase();
        let classified = if contains_convergence_failure(&lower) {
            Some((
                "KES-S004",
                SimulatorDiagnosticSeverity::Error,
                SimulatorDiagnosticKind::Convergence,
            ))
        } else if contains_fatal_failure(&lower) {
            Some((
                "KES-S005",
                SimulatorDiagnosticSeverity::Error,
                SimulatorDiagnosticKind::Fatal,
            ))
        } else if lower.contains("warning") {
            Some((
                "KES-S003",
                SimulatorDiagnosticSeverity::Warning,
                SimulatorDiagnosticKind::Warning,
            ))
        } else {
            None
        };

        if let Some((code, severity, kind)) = classified {
            let key = (code.to_string(), message.to_string());
            if seen.insert(key) {
                diagnostics.push(SimulatorDiagnostic {
                    code: code.to_string(),
                    severity,
                    kind,
                    message: message.to_string(),
                });
            }
        }
    }

    diagnostics
}

pub fn result_parse_diagnostic(message: impl Into<String>) -> SimulatorDiagnostic {
    SimulatorDiagnostic {
        code: "KES-S006".to_string(),
        severity: SimulatorDiagnosticSeverity::Error,
        kind: SimulatorDiagnosticKind::ResultParse,
        message: message.into(),
    }
}

fn parse_operating_point(
    headers: &[String],
    rows: &[Vec<f64>],
) -> Result<Dataset, SimulationParseError> {
    if rows.len() != 1 {
        return Err(parse_error_without_line(format!(
            "operating-point wrdata must contain exactly one row, got {}",
            rows.len()
        )));
    }
    let mut values = BTreeMap::new();
    for (name, value) in headers.iter().skip(1).zip(rows[0].iter().skip(1)) {
        if values.insert(name.clone(), *value).is_some() {
            return Err(parse_error_without_line(format!(
                "duplicate operating-point vector '{name}'"
            )));
        }
    }
    Ok(Dataset::OperatingPoint { values })
}

fn parse_real_series(
    headers: &[String],
    rows: &[Vec<f64>],
    axis_order: AxisOrder,
) -> Result<RealSeriesDataset, SimulationParseError> {
    let axis_name = headers[0].clone();
    let axis_values = rows.iter().map(|row| row[0]).collect::<Vec<_>>();
    ensure_axis_order(&axis_name, &axis_values, axis_order)?;

    let mut signals = BTreeMap::new();
    for column in 1..headers.len() {
        let name = &headers[column];
        if name == &axis_name {
            continue;
        }
        let values = rows.iter().map(|row| row[column]).collect::<Vec<_>>();
        if signals.insert(name.clone(), values).is_some() {
            return Err(parse_error_without_line(format!(
                "duplicate real-series vector '{name}'"
            )));
        }
    }

    Ok(RealSeriesDataset {
        axis: SeriesAxis {
            name: axis_name,
            values: axis_values,
        },
        signals,
    })
}

fn parse_ac_series(
    headers: &[String],
    rows: &[Vec<f64>],
) -> Result<ComplexSeriesDataset, SimulationParseError> {
    if !(headers.len() - 1).is_multiple_of(2) {
        return Err(parse_error_without_line(
            "AC wrdata columns after frequency must be real/imaginary pairs",
        ));
    }
    let frequency_hz = rows.iter().map(|row| row[0]).collect::<Vec<_>>();
    ensure_strictly_increasing("frequency", &frequency_hz)?;

    let mut signals = BTreeMap::new();
    for column in (1..headers.len()).step_by(2) {
        let name = &headers[column];
        if headers[column + 1] != *name {
            return Err(parse_error_without_line(format!(
                "AC vector '{name}' does not have adjacent real/imaginary columns"
            )));
        }
        if name == &headers[0] {
            continue;
        }
        let series = ComplexSeries {
            real: rows.iter().map(|row| row[column]).collect(),
            imaginary: rows.iter().map(|row| row[column + 1]).collect(),
        };
        if signals.insert(name.clone(), series).is_some() {
            return Err(parse_error_without_line(format!(
                "duplicate AC vector '{name}'"
            )));
        }
    }

    Ok(ComplexSeriesDataset {
        frequency_hz,
        signals,
    })
}

fn parse_row(
    line_number: usize,
    line: &str,
    expected_columns: usize,
) -> Result<Vec<f64>, SimulationParseError> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() != expected_columns {
        return Err(parse_error(
            line_number,
            format!(
                "wrdata row has {} columns; expected {expected_columns}",
                tokens.len()
            ),
        ));
    }
    tokens
        .iter()
        .map(|token| {
            parse_ngspice_number(token).map_err(|message| parse_error(line_number, message))
        })
        .collect()
}

fn parse_ngspice_number(token: &str) -> Result<f64, String> {
    let trimmed = token.trim();
    let normalized_exponent = trimmed.replace(['D', 'd'], "e");
    let normalized = if normalized_exponent.contains(',') && !normalized_exponent.contains('.') {
        normalized_exponent.replace(',', ".")
    } else {
        normalized_exponent
    };
    let value = normalized
        .parse::<f64>()
        .map_err(|error| format!("invalid Ngspice number '{token}': {error}"))?;
    if !value.is_finite() {
        return Err(format!("non-finite Ngspice number '{token}'"));
    }
    Ok(value)
}

fn ensure_strictly_increasing(name: &str, values: &[f64]) -> Result<(), SimulationParseError> {
    ensure_axis_order(name, values, AxisOrder::Increasing)
}

#[derive(Clone, Copy)]
enum AxisOrder {
    Increasing,
    Either,
}

fn ensure_axis_order(
    name: &str,
    values: &[f64],
    order: AxisOrder,
) -> Result<(), SimulationParseError> {
    let increasing = values.windows(2).all(|pair| pair[1] > pair[0]);
    let decreasing = values.windows(2).all(|pair| pair[1] < pair[0]);
    let valid = increasing || matches!(order, AxisOrder::Either) && decreasing;
    if !valid {
        return Err(parse_error_without_line(format!(
            "dataset axis '{name}' must be strictly monotonic in the expected direction"
        )));
    }
    Ok(())
}

fn canonical_vector_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn is_measurement_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
}

fn contains_convergence_failure(lower: &str) -> bool {
    [
        "singular matrix",
        "timestep too small",
        "convergence failure",
        "failed to converge",
        "iteration limit reached",
        "gmin stepping failed",
        "source stepping failed",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

fn contains_fatal_failure(lower: &str) -> bool {
    if lower.contains("no error") || lower.contains("0 error") {
        return false;
    }
    lower.contains("fatal")
        || lower.contains("aborted")
        || lower.starts_with("error")
        || lower.contains(" error:")
}

fn parse_error(line: usize, message: impl Into<String>) -> SimulationParseError {
    SimulationParseError {
        line: Some(line),
        message: message.into(),
    }
}

fn parse_error_without_line(message: impl Into<String>) -> SimulationParseError {
    SimulationParseError {
        line: None,
        message: message.into(),
    }
}
