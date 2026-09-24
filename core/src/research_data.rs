//! Pure research-data import and comparison. No I/O, simulator launch or physical claims.
use crate::experiment::hash_bytes;
use crate::ir::{SIUnit, parse_value};
use crate::simulation::{Dataset, SIMULATION_SCHEMA_VERSION, SimulationResult, SimulationStatus};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const IMPORT_SCHEMA: &str = "kessetsu.data-import.v1";
pub const PREVIEW_SCHEMA: &str = "kessetsu.csv-preview.v1";
pub const DATA_SCHEMA: &str = "kessetsu.research-data.v1";
pub const COMPARISON_SCHEMA: &str = "kessetsu.data-comparison.v1";
pub const SIMULATION_IMPORT_SCHEMA: &str = "kessetsu.simulation-data-import.v1";
pub const MAX_CSV_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ROWS: usize = 100_000;
pub const MAX_COLUMNS: usize = 64;
pub const MAX_VALUES: usize = 1_000_000;
const MAX_FIELD_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Delimiter {
    #[default]
    Comma,
    Semicolon,
    Tab,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Decimal {
    #[default]
    Dot,
    Comma,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvDialect {
    #[serde(default)]
    pub delimiter: Delimiter,
    #[serde(default)]
    pub decimal: Decimal,
    #[serde(default = "yes")]
    pub header: bool,
    #[serde(default)]
    pub preamble_records: usize,
}
fn yes() -> bool {
    true
}
impl Default for CsvDialect {
    fn default() -> Self {
        Self {
            delimiter: Delimiter::Comma,
            decimal: Decimal::Dot,
            header: true,
            preamble_records: 0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Measured,
    PublishedSimulation,
    Simulation,
    Synthetic,
    #[default]
    Unspecified,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DataMetadata {
    #[serde(default)]
    pub origin: Origin,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub sample: Option<String>,
    #[serde(default)]
    pub citation: Option<String>,
    #[serde(default)]
    pub conditions: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissingPolicy {
    #[default]
    Error,
    SkipRow,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AxisOrder {
    #[default]
    Increasing,
    Decreasing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnMapping {
    pub column: usize,
    pub name: String,
    pub unit: SIUnit,
    pub source_unit: String,
    #[serde(default = "one")]
    pub gain: f64,
    /// Offset in normalized units, applied after source-unit conversion and gain.
    #[serde(default)]
    pub offset: f64,
}
fn one() -> f64 {
    1.0
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSpec {
    pub schema_version: String,
    pub name: String,
    pub file_name: String,
    #[serde(default)]
    pub dialect: CsvDialect,
    #[serde(default)]
    pub metadata: DataMetadata,
    pub axis: ColumnMapping,
    #[serde(default)]
    pub axis_order: AxisOrder,
    pub signals: Vec<ColumnMapping>,
    #[serde(default)]
    pub missing: MissingPolicy,
    #[serde(default = "empty_token")]
    pub missing_tokens: Vec<String>,
}
fn empty_token() -> Vec<String> {
    vec![String::new()]
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataColumn {
    pub name: String,
    pub unit: SIUnit,
    pub values: Vec<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedRow {
    pub record: usize,
    pub line: usize,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchData {
    pub schema_version: String,
    pub identity: String,
    pub raw_sha256: String,
    pub raw_csv: String,
    pub spec: ImportSpec,
    pub axis: DataColumn,
    pub signals: Vec<DataColumn>,
    /// Absolute one-based logical CSV record numbers, including header/preamble.
    pub source_records: Vec<usize>,
    pub skipped: Vec<SkippedRow>,
    pub records_seen: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationSignalMapping {
    pub vector: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationImportSpec {
    pub schema_version: String,
    pub name: String,
    pub analysis_index: usize,
    pub signals: Vec<SimulationSignalMapping>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRow {
    pub record: usize,
    pub line: usize,
    pub fields: Vec<String>,
    pub truncated_fields: Vec<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvPreview {
    pub schema_version: String,
    pub raw_sha256: String,
    pub headers: Vec<String>,
    pub truncated_headers: Vec<usize>,
    pub sample: Vec<PreviewRow>,
    pub data_records: usize,
    pub blank_records: usize,
}

fn text(value: &str, label: &str, max: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(|c| c.is_control()) {
        return Err(format!(
            "{label} must be nonempty, at most {max} UTF-8 bytes, without control characters"
        ));
    }
    Ok(())
}
fn validate_dialect(dialect: &CsvDialect) -> Result<(), String> {
    if dialect.preamble_records > 128 {
        return Err("At most 128 preamble records are allowed".into());
    }
    // Comma decimals with comma delimiter require quoting; reject that ambiguous profile.
    if dialect.decimal == Decimal::Comma && dialect.delimiter == Delimiter::Comma {
        return Err("Decimal comma requires a semicolon or tab delimiter".into());
    }
    Ok(())
}
fn conversion(mapping: &ColumnMapping) -> Result<f64, String> {
    text(&mapping.name, "Column name", 128)?;
    if mapping.column >= MAX_COLUMNS
        || !mapping.gain.is_finite()
        || mapping.gain == 0.0
        || !mapping.offset.is_finite()
    {
        return Err("Column index, gain or offset is invalid".into());
    }
    if mapping.source_unit.len() > 24 {
        return Err("Source unit is too long".into());
    }
    if mapping.unit == SIUnit::Ratio && matches!(mapping.source_unit.as_str(), "" | "1") {
        return Ok(1.0);
    }
    let (factor, explicit) = parse_value(&format!("1{}", mapping.source_unit))?;
    if explicit != Some(mapping.unit) {
        return Err(format!("Source unit does not match {:?}", mapping.unit));
    }
    Ok(factor)
}
fn validate_spec(spec: &ImportSpec) -> Result<(), String> {
    if spec.schema_version != IMPORT_SCHEMA {
        return Err("Unsupported data-import schema".into());
    }
    text(&spec.name, "Dataset name", 200)?;
    text(&spec.file_name, "File name", 256)?;
    validate_dialect(&spec.dialect)?;
    if spec.signals.is_empty()
        || spec.signals.len() > 32
        || spec.missing_tokens.len() > 16
        || spec.missing_tokens.iter().any(|t| t.len() > 64)
    {
        return Err("Choose 1–32 signals and at most 16 bounded missing tokens".into());
    }
    if !matches!(
        spec.axis.unit,
        SIUnit::Second
            | SIUnit::Hertz
            | SIUnit::Volt
            | SIUnit::Ampere
            | SIUnit::Ohm
            | SIUnit::Ratio
    ) {
        return Err("Unsupported comparison-axis quantity".into());
    }
    if matches!(spec.axis.unit, SIUnit::Second | SIUnit::Hertz)
        && spec.axis_order != AxisOrder::Increasing
    {
        return Err(
            "Time/frequency axes must be increasing; use acquisition time for I–V loops".into(),
        );
    }
    let mut names = BTreeSet::new();
    for mapping in std::iter::once(&spec.axis).chain(&spec.signals) {
        conversion(mapping)?;
        if !names.insert(mapping.name.to_ascii_lowercase()) {
            return Err("Duplicate logical column name".into());
        }
    }
    for (label, value, max) in [
        ("Device", &spec.metadata.device, 256),
        ("Sample", &spec.metadata.sample, 256),
        ("Citation", &spec.metadata.citation, 2000),
    ] {
        if let Some(value) = value {
            text(value, label, max)?;
        }
    }
    if spec.metadata.conditions.len() > 32 {
        return Err("At most 32 condition entries are allowed".into());
    }
    for (key, value) in &spec.metadata.conditions {
        text(key, "Condition name", 80)?;
        text(value, "Condition", 2000)?;
    }
    Ok(())
}

struct CsvRow {
    record: usize,
    line: usize,
    fields: Vec<String>,
    blank: bool,
}
/// Strict, bounded, streaming CSV tokenizer. Quoted line breaks retain raw provenance.
struct CsvRecords<'a> {
    input: &'a str,
    at: usize,
    line: usize,
    record: usize,
    delimiter: u8,
}
impl<'a> CsvRecords<'a> {
    fn new(input: &'a str, dialect: &CsvDialect) -> Result<Self, String> {
        validate_dialect(dialect)?;
        if input.is_empty() || input.len() > MAX_CSV_BYTES {
            return Err("CSV must contain 1 byte–8 MiB of UTF-8 text".into());
        }
        let at = if input.starts_with('\u{feff}') { 3 } else { 0 };
        Ok(Self {
            input,
            at,
            line: 1,
            record: 0,
            delimiter: match dialect.delimiter {
                Delimiter::Comma => b',',
                Delimiter::Semicolon => b';',
                Delimiter::Tab => b'\t',
            },
        })
    }
    fn next(&mut self) -> Result<Option<CsvRow>, String> {
        if self.at == self.input.len() {
            return Ok(None);
        }
        if self.record >= MAX_ROWS + 129 {
            return Err("CSV record limit exceeded".into());
        }
        let start = self.at;
        let start_line = self.line;
        let bytes = self.input.as_bytes();
        let mut fields = Vec::new();
        let mut field = Vec::new();
        // 0 start, 1 unquoted, 2 quoted, 3 after closing quote.
        let mut state = 0;
        loop {
            let byte = bytes.get(self.at).copied();
            if byte.is_none() && state == 2 {
                return Err(format!("Unclosed quoted field at line {start_line}"));
            }
            if state != 2 && (byte.is_none() || matches!(byte, Some(b'\n' | b'\r'))) {
                fields.push(String::from_utf8(field).map_err(|_| "Invalid UTF-8 CSV field")?);
                if let Some(newline) = byte {
                    self.at += 1;
                    if newline == b'\r' && bytes.get(self.at) == Some(&b'\n') {
                        self.at += 1;
                    }
                    self.line += 1;
                }
                self.record += 1;
                if fields.len() > MAX_COLUMNS {
                    return Err("CSV exceeds 64 columns".into());
                }
                return Ok(Some(CsvRow {
                    record: self.record,
                    line: start_line,
                    fields,
                    blank: self.input[start..self.at].trim().is_empty(),
                }));
            }
            let byte = byte.unwrap();
            self.at += 1;
            match state {
                2 => {
                    if byte == b'"' {
                        if bytes.get(self.at) == Some(&b'"') {
                            field.push(b'"');
                            self.at += 1;
                        } else {
                            state = 3;
                        }
                    } else {
                        field.push(byte);
                        if byte == b'\n' || (byte == b'\r' && bytes.get(self.at) != Some(&b'\n')) {
                            self.line += 1;
                        }
                    }
                }
                _ if byte == self.delimiter => {
                    fields.push(
                        String::from_utf8(std::mem::take(&mut field))
                            .map_err(|_| "Invalid UTF-8 CSV field")?,
                    );
                    if fields.len() >= MAX_COLUMNS {
                        return Err("CSV exceeds 64 columns".into());
                    }
                    state = 0;
                }
                0 if byte == b'"' => state = 2,
                3 if matches!(byte, b' ' | b'\t') => {}
                3 => {
                    return Err(format!(
                        "Unexpected text after closing quote at line {}",
                        self.line
                    ));
                }
                _ if byte == b'"' => {
                    return Err(format!("Quote inside unquoted field at line {}", self.line));
                }
                _ => {
                    field.push(byte);
                    state = 1;
                }
            }
            if field.len() > MAX_FIELD_BYTES {
                return Err("CSV field exceeds 64 KiB".into());
            }
        }
    }
}
fn headers(row: &CsvRow, dialect: &CsvDialect) -> Vec<String> {
    if dialect.header {
        row.fields.iter().map(|s| s.trim().to_owned()).collect()
    } else {
        (0..row.fields.len())
            .map(|i| format!("Column {}", i + 1))
            .collect()
    }
}
fn preview_fields(fields: Vec<String>) -> (Vec<String>, Vec<usize>) {
    let mut clipped = Vec::new();
    let mut truncated = Vec::new();
    for (index, field) in fields.into_iter().enumerate() {
        let display = field.chars().take(256).collect::<String>();
        if display.len() < field.len() {
            truncated.push(index);
        }
        clipped.push(display);
    }
    (clipped, truncated)
}

pub fn preview_csv(input: &str, dialect: &CsvDialect) -> Result<CsvPreview, String> {
    let mut rows = CsvRecords::new(input, dialect)?;
    let mut result = CsvPreview {
        schema_version: PREVIEW_SCHEMA.into(),
        raw_sha256: hash_bytes(input.as_bytes()),
        headers: Vec::new(),
        truncated_headers: Vec::new(),
        sample: Vec::new(),
        data_records: 0,
        blank_records: 0,
    };
    while let Some(row) = rows.next()? {
        if row.record <= dialect.preamble_records {
            continue;
        }
        if row.blank {
            result.blank_records += 1;
            continue;
        }
        if result.headers.is_empty() {
            (result.headers, result.truncated_headers) = preview_fields(headers(&row, dialect));
            if dialect.header {
                continue;
            }
        }
        if row.fields.len() != result.headers.len() {
            return Err(format!(
                "Record {} has a different column count",
                row.record
            ));
        }
        result.data_records += 1;
        if result.data_records > MAX_ROWS {
            return Err("CSV exceeds 100000 data records".into());
        }
        if result.sample.len() < 8 {
            let (fields, truncated_fields) = preview_fields(row.fields);
            result.sample.push(PreviewRow {
                record: row.record,
                line: row.line,
                fields,
                truncated_fields,
            });
        }
    }
    if result.data_records == 0 {
        return Err("CSV contains no data records".into());
    }
    Ok(result)
}
fn seal(mut data: ResearchData) -> ResearchData {
    data.identity.clear();
    data.identity = hash_bytes(&serde_json::to_vec(&data).expect("finite validated research data"));
    data
}
pub fn import_csv(input: &str, spec: ImportSpec) -> Result<ResearchData, String> {
    validate_spec(&spec)?;
    let mappings = std::iter::once(&spec.axis)
        .chain(&spec.signals)
        .collect::<Vec<_>>();
    let factors = mappings
        .iter()
        .map(|m| conversion(m))
        .collect::<Result<Vec<_>, _>>()?;
    let mut values = vec![Vec::new(); mappings.len()];
    let mut source_records = Vec::new();
    let mut skipped = Vec::new();
    let mut rows = CsvRecords::new(input, &spec.dialect)?;
    let mut width = None;
    let mut records_seen = 0;
    while let Some(row) = rows.next()? {
        if row.record <= spec.dialect.preamble_records {
            continue;
        }
        if row.blank {
            skipped.push(SkippedRow {
                record: row.record,
                line: row.line,
                reason: "blank_record".into(),
            });
            continue;
        }
        if width.is_none() {
            width = Some(row.fields.len());
            if mappings.iter().any(|m| m.column >= row.fields.len()) {
                return Err("Mapped column is outside the CSV width".into());
            }
            if spec.dialect.header {
                continue;
            }
        }
        if Some(row.fields.len()) != width {
            return Err(format!(
                "Record {} has a different column count",
                row.record
            ));
        }
        records_seen += 1;
        if records_seen > MAX_ROWS {
            return Err("CSV exceeds 100000 data records".into());
        }
        let missing = mappings.iter().any(|m| {
            spec.missing_tokens
                .iter()
                .any(|token| token == row.fields[m.column].trim())
        });
        if missing {
            if spec.missing == MissingPolicy::Error {
                return Err(format!(
                    "Missing mapped value at record {} (line {}); choose explicit skip_row to omit the whole row",
                    row.record, row.line
                ));
            }
            skipped.push(SkippedRow {
                record: row.record,
                line: row.line,
                reason: "missing_mapped_value".into(),
            });
            continue;
        }
        if (source_records.len() + 1) * mappings.len() > MAX_VALUES {
            return Err("Dataset exceeds one million normalized values".into());
        }
        let mut parsed = Vec::new();
        for (mapping, factor) in mappings.iter().zip(&factors) {
            let raw = row.fields[mapping.column].trim();
            let numeric = match spec.dialect.decimal {
                Decimal::Dot => {
                    if raw.contains(',') {
                        return Err(format!("Wrong decimal convention at record {}", row.record));
                    }
                    raw.to_owned()
                }
                Decimal::Comma => {
                    if raw.contains('.') {
                        return Err(format!("Wrong decimal convention at record {}", row.record));
                    }
                    raw.replace(',', ".")
                }
            }
            .replace(['D', 'd'], "e");
            numeric.parse::<f64>().map_err(|_| {
                format!(
                    "Invalid number at record {}, column {}",
                    row.record,
                    mapping.column + 1
                )
            })?;
            let numeric = parse_value(&numeric)
                .map_err(|_| {
                    format!(
                        "Out-of-range number at record {}, column {}",
                        row.record,
                        mapping.column + 1
                    )
                })?
                .0;
            let calibrated = (numeric * factor) * mapping.gain;
            let converted = calibrated + mapping.offset;
            if !numeric.is_finite()
                || !converted.is_finite()
                || (numeric != 0.0 && calibrated == 0.0)
            {
                return Err(format!(
                    "Non-finite/out-of-range value at record {}, column {}",
                    row.record,
                    mapping.column + 1
                ));
            }
            parsed.push(converted);
        }
        if spec.axis.unit == SIUnit::Hertz && parsed[0] <= 0.0 {
            return Err("Frequency must be positive".into());
        }
        if values[0].last().is_some_and(|last| match spec.axis_order {
            AxisOrder::Increasing => parsed[0] <= *last,
            AxisOrder::Decreasing => parsed[0] >= *last,
        }) {
            return Err(format!(
                "Axis is not strictly ordered at record {}; no automatic sorting is performed",
                row.record
            ));
        }
        for (column, value) in values.iter_mut().zip(parsed) {
            column.push(value);
        }
        source_records.push(row.record);
    }
    if source_records.len() < 2 {
        return Err("At least two usable data records are required".into());
    }
    let mut columns = mappings
        .iter()
        .zip(values)
        .map(|(m, values)| DataColumn {
            name: m.name.clone(),
            unit: m.unit,
            values,
        })
        .collect::<Vec<_>>();
    let axis = columns.remove(0);
    Ok(seal(ResearchData {
        schema_version: DATA_SCHEMA.into(),
        identity: String::new(),
        raw_sha256: hash_bytes(input.as_bytes()),
        raw_csv: input.into(),
        spec,
        axis,
        signals: columns,
        source_records,
        skipped,
        records_seen,
    }))
}
/// Reconstruct from raw evidence: altered arrays/counts cannot pass just by resealing JSON.
pub fn validate_data(data: &ResearchData) -> Result<(), String> {
    if data.schema_version != DATA_SCHEMA || data.raw_csv.len() > MAX_CSV_BYTES {
        return Err("Unsupported/oversized research-data report".into());
    }
    let expected = import_csv(&data.raw_csv, data.spec.clone())?;
    if serde_json::to_vec(&expected).map_err(|e| e.to_string())?
        != serde_json::to_vec(data).map_err(|e| e.to_string())?
    {
        return Err("Research data differs from its raw evidence/mapping or identity".into());
    }
    Ok(())
}

fn unit_suffix(unit: SIUnit) -> &'static str {
    match unit {
        SIUnit::Ohm => "Ohm",
        SIUnit::Farad => "F",
        SIUnit::Henry => "H",
        SIUnit::Volt => "V",
        SIUnit::Ampere => "A",
        SIUnit::Hertz => "Hz",
        SIUnit::Second => "s",
        SIUnit::Watt => "W",
        SIUnit::Joule => "J",
        SIUnit::Ratio => "",
        SIUnit::Percent => "%",
        SIUnit::Degree => "deg",
    }
}

fn vector_unit(vector: &str) -> Result<SIUnit, String> {
    let upper = vector.trim().to_ascii_uppercase();
    if upper.starts_with("V(") && upper.ends_with(')') {
        Ok(SIUnit::Volt)
    } else if (upper.starts_with("I(") && upper.ends_with(')')) || upper.ends_with("#BRANCH") {
        Ok(SIUnit::Ampere)
    } else if !upper.is_empty()
        && upper
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
    {
        // Ngspice/browser adapters expose node-voltage vectors as bare node names.
        Ok(SIUnit::Volt)
    } else {
        Err(format!(
            "Simulation vector '{vector}' has no supported electrical-unit contract"
        ))
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Project one successful typed simulation dataset into the ordinary research-data contract.
/// The generated CSV is derived evidence; its metadata retains the simulator/result identity.
pub fn import_simulation_data(
    simulation: &SimulationResult,
    spec: SimulationImportSpec,
) -> Result<ResearchData, String> {
    if spec.schema_version != SIMULATION_IMPORT_SCHEMA {
        return Err("Unsupported simulation-data import schema".into());
    }
    text(&spec.name, "Dataset name", 200)?;
    if simulation.schema_version != SIMULATION_SCHEMA_VERSION
        || simulation.status != SimulationStatus::Succeeded
        || !simulation.process.success
    {
        return Err("Only a successful typed simulation result can be imported".into());
    }
    if spec.signals.is_empty() || spec.signals.len() > 16 {
        return Err("Choose 1-16 simulation signals".into());
    }
    let selected = simulation
        .datasets
        .iter()
        .find(|dataset| dataset.index == spec.analysis_index)
        .ok_or("Simulation analysis index not found")?;
    let (axis_name, axis_unit, axis_values, available, ac) = match &selected.data {
        Dataset::OperatingPoint { .. } => {
            return Err("Operating-point data has no comparison axis".into());
        }
        Dataset::Transient(series) => (
            series.axis.name.clone(),
            SIUnit::Second,
            series.axis.values.clone(),
            series.signals.keys().cloned().collect::<Vec<_>>(),
            false,
        ),
        Dataset::DcSweep(series) => {
            let unit = match &selected.analysis {
                crate::ir::Analysis::DcSweep { start, .. } => start.unit,
                _ => return Err("DC dataset/analysis metadata does not agree".into()),
            };
            (
                series.axis.name.clone(),
                unit,
                series.axis.values.clone(),
                series.signals.keys().cloned().collect::<Vec<_>>(),
                false,
            )
        }
        Dataset::Ac(series) => (
            "frequency".into(),
            SIUnit::Hertz,
            series.frequency_hz.clone(),
            series.signals.keys().cloned().collect::<Vec<_>>(),
            true,
        ),
    };
    if axis_values.len() < 2 {
        return Err("Simulation analysis needs at least two ordered samples".into());
    }
    let mut logical_names = BTreeSet::new();
    logical_names.insert(axis_name.to_ascii_lowercase());
    let mut vectors = BTreeSet::new();
    let mut projected = Vec::new();
    for mapping in &spec.signals {
        text(&mapping.vector, "Simulation vector", 256)?;
        text(&mapping.name, "Signal name", 128)?;
        if !available.iter().any(|name| name == &mapping.vector) {
            return Err(format!(
                "Simulation vector '{}' was not found",
                mapping.vector
            ));
        }
        if !vectors.insert(mapping.vector.clone())
            || !logical_names.insert(mapping.name.to_ascii_lowercase())
        {
            return Err("Simulation vectors and logical signal names must be unique".into());
        }
        let unit = vector_unit(&mapping.vector)?;
        let values = match &selected.data {
            Dataset::Transient(series) | Dataset::DcSweep(series) => series
                .signals
                .get(&mapping.vector)
                .cloned()
                .ok_or("Simulation signal not found")?,
            Dataset::Ac(series) => {
                let values = series
                    .signals
                    .get(&mapping.vector)
                    .ok_or("Simulation signal not found")?;
                if values.real.len() != values.imaginary.len() {
                    return Err("Complex simulation vector lengths do not agree".into());
                }
                values
                    .real
                    .iter()
                    .zip(&values.imaginary)
                    .map(|(real, imaginary)| real.hypot(*imaginary))
                    .collect()
            }
            Dataset::OperatingPoint { .. } => unreachable!(),
        };
        if values.len() != axis_values.len() {
            return Err("Simulation axis and signal lengths do not agree".into());
        }
        projected.push((mapping, unit, values));
    }
    let descending = axis_values.first() > axis_values.last();
    if matches!(axis_unit, SIUnit::Second | SIUnit::Hertz) && descending {
        return Err("Time/frequency simulation axes must increase".into());
    }
    let result_sha256 = hash_bytes(
        &serde_json::to_vec(simulation)
            .map_err(|error| format!("Could not hash simulation: {error}"))?,
    );
    let mut conditions = BTreeMap::from([
        ("analysis_index".into(), spec.analysis_index.to_string()),
        ("analysis_kind".into(), selected.analysis.kind_name().into()),
        ("simulation_sha256".into(), result_sha256),
        (
            "simulator".into(),
            format!(
                "{} {}",
                simulation.simulator.executable, simulation.simulator.version
            ),
        ),
        (
            "complex_projection".into(),
            if ac { "magnitude" } else { "real" }.into(),
        ),
    ]);
    for (index, (mapping, _, _)) in projected.iter().enumerate() {
        conditions.insert(
            format!("signal_{}_vector", index + 1),
            mapping.vector.clone(),
        );
    }
    let mut csv = std::iter::once(csv_field(&axis_name))
        .chain(
            projected
                .iter()
                .map(|(mapping, _, _)| csv_field(&mapping.name)),
        )
        .collect::<Vec<_>>()
        .join(",");
    csv.push('\n');
    for (index, axis) in axis_values.iter().enumerate() {
        csv.push_str(&format!("{axis:.17e}"));
        for (_, _, values) in &projected {
            csv.push(',');
            csv.push_str(&format!("{:.17e}", values[index]));
        }
        csv.push('\n');
    }
    let import_spec = ImportSpec {
        schema_version: IMPORT_SCHEMA.into(),
        name: spec.name,
        file_name: format!("simulation-analysis-{}.csv", spec.analysis_index + 1),
        dialect: CsvDialect::default(),
        metadata: DataMetadata {
            origin: Origin::Simulation,
            citation: Some(
                "Derived from a typed Kessetsu simulation result; see metadata conditions".into(),
            ),
            conditions,
            ..DataMetadata::default()
        },
        axis: ColumnMapping {
            column: 0,
            name: axis_name,
            unit: axis_unit,
            source_unit: unit_suffix(axis_unit).into(),
            gain: 1.0,
            offset: 0.0,
        },
        axis_order: if descending {
            AxisOrder::Decreasing
        } else {
            AxisOrder::Increasing
        },
        signals: projected
            .iter()
            .enumerate()
            .map(|(index, (mapping, unit, _))| ColumnMapping {
                column: index + 1,
                name: mapping.name.clone(),
                unit: *unit,
                source_unit: unit_suffix(*unit).into(),
                gain: 1.0,
                offset: 0.0,
            })
            .collect(),
        missing: MissingPolicy::Error,
        missing_tokens: vec![String::new()],
    };
    import_csv(&csv, import_spec)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    #[default]
    RequireFull,
    OverlapOnly,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    #[default]
    Linear,
    LogAxis,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalPair {
    pub data_signal: String,
    pub reference_signal: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonSpec {
    pub schema_version: String,
    pub name: String,
    pub signals: Vec<SignalPair>,
    #[serde(default)]
    pub coverage: Coverage,
    #[serde(default)]
    pub interpolation: Interpolation,
    /// Shift reference axis by this normalized-unit quantity, explicitly recorded.
    #[serde(default)]
    pub reference_axis_shift: f64,
    #[serde(default)]
    pub window: Option<[f64; 2]>,
    #[serde(default)]
    pub max_reference_gap: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidualPoint {
    pub source_record: usize,
    pub axis: f64,
    pub observed: f64,
    pub predicted: Option<f64>,
    /// predicted − observed, in the signal's normalized unit.
    pub residual: Option<f64>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidualMetrics {
    pub total: usize,
    pub matched: usize,
    pub excluded_by_window: usize,
    pub unmatched: usize,
    pub bias: f64,
    pub mae: f64,
    pub rmse: f64,
    pub max_absolute: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalComparison {
    pub mapping: SignalPair,
    pub unit: SIUnit,
    pub metrics: ResidualMetrics,
    pub points: Vec<ResidualPoint>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataComparison {
    pub schema_version: String,
    pub identity: String,
    pub data_identity: String,
    pub reference_identity: String,
    pub data_origin: Origin,
    pub reference_origin: Origin,
    pub axis_unit: SIUnit,
    pub spec: ComparisonSpec,
    pub signals: Vec<SignalComparison>,
}

pub fn compare_data(
    data: &ResearchData,
    reference: &ResearchData,
    spec: ComparisonSpec,
) -> Result<DataComparison, String> {
    validate_data(data)?;
    validate_data(reference)?;
    if spec.schema_version != COMPARISON_SCHEMA {
        return Err("Unsupported data-comparison schema".into());
    }
    text(&spec.name, "Comparison name", 200)?;
    if spec.signals.is_empty() || spec.signals.len() > 16 {
        return Err("Choose 1–16 comparison signals".into());
    }
    if spec.signals.len() * data.axis.values.len() > MAX_VALUES {
        return Err("Comparison exceeds one million residual points".into());
    }
    if data.axis.unit != reference.axis.unit {
        return Err("Comparison axis units do not match".into());
    }
    if !spec.reference_axis_shift.is_finite()
        || spec
            .window
            .is_some_and(|[a, b]| !a.is_finite() || !b.is_finite() || a >= b)
        || spec
            .max_reference_gap
            .is_some_and(|v| !v.is_finite() || v <= 0.0)
    {
        return Err("Invalid axis shift, comparison window or maximum gap".into());
    }
    if spec.interpolation == Interpolation::LogAxis && data.axis.unit != SIUnit::Hertz {
        return Err("Log-axis interpolation requires a frequency axis".into());
    }
    let descending = reference.spec.axis_order == AxisOrder::Decreasing;
    let indices = if descending {
        (0..reference.axis.values.len()).rev().collect::<Vec<_>>()
    } else {
        (0..reference.axis.values.len()).collect::<Vec<_>>()
    };
    let axis = indices
        .iter()
        .map(|i| reference.axis.values[*i] + spec.reference_axis_shift)
        .collect::<Vec<_>>();
    if axis
        .iter()
        .any(|x| !x.is_finite() || (spec.interpolation == Interpolation::LogAxis && *x <= 0.0))
        || axis.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(
            "Shifted reference axis is non-finite, invalid or loses numerical ordering".into(),
        );
    }
    let mut seen = BTreeSet::new();
    let mut compared = Vec::new();
    for mapping in &spec.signals {
        if !seen.insert(mapping.data_signal.clone()) {
            return Err("Duplicate data-signal comparison".into());
        }
        let observed = data
            .signals
            .iter()
            .find(|c| c.name == mapping.data_signal)
            .ok_or("Data signal not found")?;
        let predicted = reference
            .signals
            .iter()
            .find(|c| c.name == mapping.reference_signal)
            .ok_or("Reference signal not found")?;
        if observed.unit != predicted.unit {
            return Err("Compared signal units do not match".into());
        }
        let mut points = Vec::new();
        for (i, &x) in data.axis.values.iter().enumerate() {
            let (value, status) = if spec.window.is_some_and(|[a, b]| x < a || x > b) {
                (None, "excluded_by_window")
            } else if x < axis[0] || x > *axis.last().unwrap() {
                (None, "outside_reference")
            } else {
                let right = axis.partition_point(|v| *v < x);
                if right < axis.len() && axis[right] == x {
                    (Some(predicted.values[indices[right]]), "matched")
                } else {
                    let left = right - 1;
                    if spec
                        .max_reference_gap
                        .is_some_and(|g| axis[right] - axis[left] > g)
                    {
                        (None, "reference_gap")
                    } else {
                        let transform = |x: f64| {
                            if spec.interpolation == Interpolation::LogAxis {
                                x.ln()
                            } else {
                                x
                            }
                        };
                        let a = transform(axis[left]);
                        let b = transform(axis[right]);
                        let delta = b - a;
                        let numerator = transform(x) - a;
                        if !delta.is_finite() || delta <= 0.0 || !numerator.is_finite() {
                            return Err(
                                "Interpolation axis exceeds numerical range/resolution".into()
                            );
                        }
                        let fraction = numerator / delta;
                        let value = predicted.values[indices[left]] * (1.0 - fraction)
                            + predicted.values[indices[right]] * fraction;
                        if !fraction.is_finite() || !value.is_finite() {
                            return Err("Interpolation exceeds numerical range/resolution".into());
                        }
                        (Some(value), "matched")
                    }
                }
            };
            let residual = value.map(|value| value - observed.values[i]);
            if residual.is_some_and(|v| !v.is_finite()) {
                return Err("Residual exceeds finite numeric range".into());
            }
            points.push(ResidualPoint {
                source_record: data.source_records[i],
                axis: x,
                observed: observed.values[i],
                predicted: value,
                residual,
                status: status.into(),
            });
        }
        let residuals = points.iter().filter_map(|p| p.residual).collect::<Vec<_>>();
        let excluded_by_window = points
            .iter()
            .filter(|p| p.status == "excluded_by_window")
            .count();
        let unmatched = points.len() - excluded_by_window - residuals.len();
        if residuals.is_empty() {
            return Err("No matched comparison points".into());
        }
        if unmatched > 0 && spec.coverage == Coverage::RequireFull {
            return Err(format!(
                "{unmatched} points lack reference coverage; no extrapolation is allowed"
            ));
        }
        let n = residuals.len() as f64;
        let maximum = residuals.iter().map(|v| v.abs()).fold(0.0, f64::max);
        // Scale before reduction: avoid both overflow and loss of subnormal residuals.
        let bias = if maximum == 0.0 {
            0.0
        } else {
            maximum * residuals.iter().map(|v| (v / maximum) / n).sum::<f64>()
        };
        let mae = if maximum == 0.0 {
            0.0
        } else {
            maximum
                * residuals
                    .iter()
                    .map(|v| (v.abs() / maximum) / n)
                    .sum::<f64>()
        };
        let rmse = if maximum == 0.0 {
            0.0
        } else {
            maximum
                * residuals
                    .iter()
                    .map(|v| (v / maximum).powi(2) / n)
                    .sum::<f64>()
                    .sqrt()
        };
        if [bias, mae, rmse].iter().any(|v| !v.is_finite()) {
            return Err("Residual aggregation exceeds finite range".into());
        }
        compared.push(SignalComparison {
            mapping: mapping.clone(),
            unit: observed.unit,
            metrics: ResidualMetrics {
                total: points.len(),
                matched: residuals.len(),
                excluded_by_window,
                unmatched,
                bias,
                mae,
                rmse,
                max_absolute: maximum,
            },
            points,
        });
    }
    let mut result = DataComparison {
        schema_version: COMPARISON_SCHEMA.into(),
        identity: String::new(),
        data_identity: data.identity.clone(),
        reference_identity: reference.identity.clone(),
        data_origin: data.spec.metadata.origin,
        reference_origin: reference.spec.metadata.origin,
        axis_unit: data.axis.unit,
        spec,
        signals: compared,
    };
    result.identity =
        hash_bytes(&serde_json::to_vec(&result).expect("finite validated comparison"));
    Ok(result)
}
