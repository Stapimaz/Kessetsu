use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SPICE_IMPORT_SCHEMA_VERSION: &str = "kessetsu.spice-import.v1";
pub(crate) const MAX_IMPORT_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_IMPORT_LINES: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDiagnostic {
    pub code: String,
    pub severity: ImportSeverity,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportNameKind {
    Component,
    Net,
    Parameter,
    Module,
    ModulePort,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportNameMapping {
    pub kind: ImportNameKind,
    pub original: String,
    pub kessetsu: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub components: usize,
    pub nets: usize,
    pub analyses: usize,
    #[serde(default)]
    pub subcircuits: usize,
    #[serde(default)]
    pub instances: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceImportReport {
    pub schema_version: String,
    pub dialect: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub names: Vec<ImportNameMapping>,
    pub summary: ImportSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kess_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_schema_version: Option<String>,
}

impl SpiceImportReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ImportSeverity::Error)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ImportedKind {
    Resistor(String),
    Capacitor(String),
    Inductor(String),
    VoltageSource(String),
    CurrentSource(String),
    Diode(String),
    Bjt {
        polarity: &'static str,
        model: String,
    },
    Mosfet(String),
    Jfet(String),
    Switch(String),
    OpAmp(String),
    Vcvs(String),
    Vccs(String),
    Ccvs(String),
    Cccs(String),
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedComponent {
    pub(crate) original_name: String,
    pub(crate) nodes: Vec<String>,
    pub(crate) pins: &'static [&'static str],
    pub(crate) kind: ImportedKind,
    pub(crate) line: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct LogicalLine {
    pub(crate) number: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedParameter {
    pub(crate) original_name: String,
    pub(crate) mapped_name: String,
    pub(crate) value: String,
    pub(crate) numeric_value: f64,
    pub(crate) line: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct SubcircuitBlock {
    pub(crate) header: LogicalLine,
    pub(crate) body: Vec<LogicalLine>,
    pub(crate) end: LogicalLine,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedPort {
    pub(crate) original_name: String,
    pub(crate) mapped_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedSubcircuit {
    pub(crate) original_name: String,
    pub(crate) mapped_name: String,
    pub(crate) ports: Vec<ImportedPort>,
    pub(crate) parameters: Vec<ImportedParameter>,
    pub(crate) parameter_units: BTreeMap<String, ImportUnit>,
    pub(crate) components: Vec<ImportedComponent>,
    pub(crate) component_names: BTreeMap<String, String>,
    pub(crate) node_names: BTreeMap<String, String>,
    pub(crate) internal_nets: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedInstance {
    pub(crate) original_name: String,
    pub(crate) mapped_name: String,
    pub(crate) module_name: String,
    pub(crate) nodes: Vec<String>,
    pub(crate) overrides: Vec<(String, String)>,
    pub(crate) line: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedCoupling {
    pub(crate) original_name: String,
    pub(crate) name: String,
    pub(crate) inductors: Vec<String>,
    pub(crate) factor: String,
    #[allow(dead_code)]
    pub(crate) line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportUnit {
    Ohm,
    Farad,
    Henry,
    Volt,
    Ampere,
    Hertz,
    Second,
    Ratio,
    Siemens,
}

impl ImportUnit {
    pub(crate) const fn kessetsu_name(self) -> &'static str {
        match self {
            Self::Ohm => "Ohm",
            Self::Farad => "F",
            Self::Henry => "H",
            Self::Volt => "V",
            Self::Ampere => "A",
            Self::Hertz => "Hz",
            Self::Second => "s",
            Self::Ratio => "ratio",
            Self::Siemens => "S",
        }
    }
}

pub(crate) type ImportedNames = (BTreeMap<String, String>, BTreeMap<String, String>);
pub(crate) type NameAllocationError = (usize, String);

/// Output/control directives whose job (printing, plotting, measuring) is owned by Kessetsu
/// measurements and requirements. They are skipped with a warning, never executed.
pub(crate) const OUTPUT_DIRECTIVES: &[&str] = &[
    ".print",
    ".plot",
    ".probe",
    ".save",
    ".meas",
    ".measure",
    ".four",
    ".width",
    ".options",
    ".option",
    ".temp",
    ".backanno",
    ".nodeset",
    ".ic",
];

pub(crate) fn push_unsupported(report: &mut SpiceImportReport, line: usize, message: &str) {
    push_error(
        report,
        "KES-N003",
        format!("Unsupported SPICE construct: {message}"),
        Some(line),
    );
}

pub(crate) fn push_title_info(report: &mut SpiceImportReport) {
    report.diagnostics.push(ImportDiagnostic {
        code: "KES-N010".into(),
        severity: ImportSeverity::Info,
        message: "first line is not a supported card and was treated as the SPICE title".into(),
        line: Some(1),
        column: Some(1),
    });
}

pub(crate) fn push_warning_diag(
    diagnostics: &mut Vec<ImportDiagnostic>,
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
) {
    diagnostics.push(ImportDiagnostic {
        code: code.into(),
        severity: ImportSeverity::Warning,
        message: message.into(),
        line,
        column: line.map(|_| 1),
    });
}

pub(crate) fn push_warning(
    report: &mut SpiceImportReport,
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
) {
    push_warning_diag(&mut report.diagnostics, code, message, line);
}

pub(crate) fn push_error(
    report: &mut SpiceImportReport,
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
) {
    report.diagnostics.push(ImportDiagnostic {
        code: code.into(),
        severity: ImportSeverity::Error,
        message: message.into(),
        line,
        column: line.map(|_| 1),
    });
}
