use clap::{Args, Parser, Subcommand, ValueEnum};
use kessetsu_core::capabilities::{
    CAPABILITIES_SCHEMA_VERSION, CapabilityManifest, capability_manifest,
};
use kessetsu_core::compile_inputs::{CompileInputs, ParameterInput};
use kessetsu_core::compiler::{
    COMPILE_SCHEMA_VERSION, CompileOptions, CompileReport, Diagnostic, DiagnosticSeverity,
    DiagnosticStage, compile_source_with_inputs,
};
use kessetsu_core::exporter::{
    EXPORT_SCHEMA_VERSION, ExportArtifact, ExportCapability, ExportFormat, ExportOptions,
    RenderBackground, export_report,
};
use kessetsu_core::measurement::MEASUREMENT_SCHEMA_VERSION;
use kessetsu_core::models::{
    ExternalModelResources, MAX_EXTERNAL_MODEL_BYTES, validate_external_resource_reference,
};
use kessetsu_core::parse_program;
use kessetsu_core::requirements::{
    REQUIREMENTS_SCHEMA_VERSION, RequirementSet, compile_requirements,
};
use kessetsu_core::sim_result::{
    ASSERTION_SCHEMA_VERSION, AssertionReport, AssertionResult, AssertionStatus, AssertionSummary,
    TolerancePolicy, format_quantity,
};
use kessetsu_core::simulation::{
    CancellationToken, NativeSimulationContext, NgspiceRunner, SIMULATION_SCHEMA_VERSION,
    SimulationRequest, SimulationResult,
};
use kessetsu_core::spice_import::{ImportDiagnostic, ImportSeverity, import_spice};
use kessetsu_core::stress::{
    PART_STRESS_SCHEMA_VERSION, PartStressReport, PartStressStatus, PartStressSummary,
    evaluate_part_stress,
};
use kessetsu_core::tools::{PreferredValues, ToolResult};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

#[path = "kess_support/import.rs"]
mod import;
#[path = "kess_support/mcp.rs"]
mod mcp;
#[path = "kess_support/research.rs"]
mod research;
#[path = "kess_support/tool.rs"]
mod tool;

pub(crate) const CLI_SCHEMA_VERSION: &str = "kessetsu.cli.v1";

#[derive(Parser)]
#[command(
    name = "kess",
    version,
    about = "Kessetsu Circuit Compiler and Simulator",
    after_help = "License: AGPL-3.0-only. Copyright (C) 2026 Stapimaz. No warranty. Source and terms: https://github.com/Stapimaz/Kessetsu"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format (human or json)
    #[arg(long, value_enum, default_value_t = Format::Human, global = true)]
    format: Format,

    /// JSON contract version requested by the caller
    #[arg(long, default_value = CLI_SCHEMA_VERSION, global = true)]
    schema_version: String,

    /// Opt in to verbose JSON fields (comma-separated or repeated)
    #[arg(long, value_enum, value_delimiter = ',', global = true)]
    include: Vec<Include>,

    /// Override a declared root parameter with a numeric literal (repeatable NAME=VALUE)
    #[arg(long = "param", global = true)]
    parameters: Vec<String>,

    /// Circuit source language. `auto` treats .cir/.sp/.spice/.net/.ckt files, and stdin
    /// containing SPICE dot directives, as SPICE netlists.
    #[arg(long = "input", value_enum, default_value_t = InputLanguage::Auto, global = true)]
    input: InputLanguage,
}

#[derive(Clone, Copy, ValueEnum, PartialEq, Eq, Debug)]
enum InputLanguage {
    Auto,
    Kess,
    Spice,
}

#[derive(Subcommand)]
enum Commands {
    /// Describe this installed build for agents and automation
    Capabilities,
    /// Convert a supported SPICE netlist into editable, canonically verified .kess source
    Import(ImportCommand),
    /// Import research CSV and compare unit-mapped local data
    Data(DataCommand),
    /// Evaluate finite study candidates against calibration and validation data
    Fit(FitCommand),
    /// Create, plan, run or export a reproducible local parameter study
    Study(StudyCommand),
    /// Calculate components and generate an editable circuit
    Tool(ToolCommand),
    /// Parse and run Electrical Rules Check (ERC)
    Check { file: PathBuf },
    /// Parse, ERC, and generate a SPICE netlist
    Compile(OutputCommand),
    /// Parse, ERC, generate a netlist, and run Ngspice
    Simulate(OutputCommand),
    /// Parse, ERC, generate a netlist, simulate, and evaluate assertions
    Test(TestCommand),
    /// Render the canonical schematic to SVG, PNG, or PDF
    Render(RenderCommand),
    /// Export a machine-readable or editable circuit artifact
    Export(ExportCommand),
    /// Serve Kessetsu tools to AI agents over the Model Context Protocol (stdio)
    Mcp,
}

#[derive(Args)]
pub(crate) struct ImportCommand {
    /// SPICE netlist path, or - for stdin
    pub(crate) file: PathBuf,
    /// Generated .kess path; defaults beside a file input
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
    /// Allow replacing an existing output file
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Args)]
pub(crate) struct FitCommand {
    #[command(subcommand)]
    pub(crate) action: FitAction,
}

#[derive(Subcommand)]
pub(crate) enum FitAction {
    /// Score a completed study; validation data never influences candidate selection
    Evaluate {
        /// Completed kessetsu.experiment-results.v1 JSON
        file: PathBuf,
        /// kessetsu.fit.v1 mapping and bounds
        #[arg(long)]
        spec: PathBuf,
        /// Observed dataset binding, repeatable: NAME=path.kessdata.json
        #[arg(long = "data", required = true)]
        datasets: Vec<String>,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
pub(crate) struct DataCommand {
    #[command(subcommand)]
    pub(crate) action: DataAction,
}

#[derive(Subcommand)]
pub(crate) enum DataAction {
    /// Preview CSV before selecting an explicit column/unit mapping
    Preview {
        file: PathBuf,
        #[arg(long, default_value = "comma", value_parser = ["comma", "semicolon", "tab"])]
        delimiter: String,
        #[arg(long, default_value = "dot", value_parser = ["dot", "comma"])]
        decimal: String,
        #[arg(long)]
        no_header: bool,
        #[arg(long, default_value_t = 0)]
        skip_records: usize,
    },
    /// Preserve raw CSV and normalized values in versioned research JSON
    Import {
        file: PathBuf,
        #[arg(long)]
        mapping: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Convert one successful typed simulation result into research-data JSON
    FromSimulation {
        /// Raw kessetsu.simulation.v1 JSON, or a CLI envelope containing debug.simulation
        file: PathBuf,
        #[arg(long)]
        mapping: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Compare two imported datasets with an explicit residual/coverage mapping
    Compare {
        file: PathBuf,
        #[arg(long)]
        reference: PathBuf,
        #[arg(long)]
        mapping: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
pub(crate) struct StudyCommand {
    #[command(subcommand)]
    pub(crate) action: StudyAction,
}

#[derive(Subcommand)]
pub(crate) enum StudyAction {
    /// Create an editable experiment specification from a circuit
    Create {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Explicit root parameter list, repeatable: resistance=1kOhm,2kOhm
        #[arg(long)]
        sweep: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    /// Validate a specification and list its deterministic cases, without running Ngspice
    Plan { file: PathBuf },
    /// Run locally; Ctrl+C saves partial results for --resume
    Run {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        resume: bool,
        #[arg(long)]
        force: bool,
    },
    /// Export full results as JSON, summary CSV, full numeric CSV, SVG or HTML
    Export {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, value_enum)]
        target: StudyTarget,
        #[arg(long, default_value = "v(out)")]
        signal: String,
        #[arg(long, default_value_t = 0)]
        analysis: usize,
        /// Restrict overlay to these case IDs (repeatable, at most 12)
        #[arg(long = "case")]
        cases: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    /// Create a new portable folder with results, data, plot, report and allowed model files
    Package {
        /// Original kessetsu.experiment.v1 specification
        file: PathBuf,
        /// Completed kessetsu.experiment-results.v1 JSON
        #[arg(long)]
        results: PathBuf,
        /// New output directory; existing directories are never overwritten
        #[arg(short, long)]
        output: PathBuf,
        /// Signal to plot, for example V(OUT) or I(VSENSE)
        #[arg(long)]
        signal: String,
        #[arg(long, default_value_t = 0)]
        analysis: usize,
        /// Restrict the plot to these case IDs (repeatable, at most 12)
        #[arg(long = "case")]
        cases: Vec<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum StudyTarget {
    Json,
    Csv,
    DataCsv,
    Svg,
    Html,
}

#[derive(Args)]
pub(crate) struct ToolCommand {
    #[command(subcommand)]
    pub(crate) tool: CircuitTool,
    /// Write generated .kess source (no file is written by default)
    #[arg(short, long, global = true)]
    pub(crate) output: Option<PathBuf>,
    /// Allow overwriting an existing output file
    #[arg(long, global = true)]
    pub(crate) force: bool,
    /// Nominal component value series (not a tolerance specification)
    #[arg(long, value_enum, default_value_t = ValueSeries::E24, global = true)]
    pub(crate) values: ValueSeries,
}

#[derive(Subcommand)]
pub(crate) enum CircuitTool {
    /// Size a divider for its actual resistive load
    Divider {
        #[arg(long)]
        vin: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        lower: String,
        /// Omit for an open-circuit load
        #[arg(long)]
        load: Option<String>,
    },
    /// Size a first-order RC filter for a high-impedance output
    RcLowpass {
        #[arg(long)]
        cutoff: String,
        #[arg(long)]
        resistance: String,
    },
    /// Size a non-inverting op-amp stage using KESSETSU_OPAMP_V2
    #[command(visible_alias = "opamp-gain")]
    NonInvertingOpamp {
        /// Target voltage gain V/V (must be > 1)
        #[arg(long, alias = "target-gain")]
        gain: String,
        /// Grounded resistor R1 value (e.g. 10k)
        #[arg(long, aliases = ["input-resistance", "lower", "resistance"])]
        r1: String,
        /// Omit for 0.5V nominal DC/peak input voltage
        #[arg(long, alias = "input-voltage")]
        vin: Option<String>,
        /// Omit for ±15V symmetric rails (must exceed 2V)
        #[arg(long, aliases = ["supply-voltage", "vsupply"])]
        supply: Option<String>,
    },
    /// Size a Zener shunt voltage regulator using a built-in BZX84C diode
    ZenerRegulator {
        /// DC supply voltage (e.g. 12V)
        #[arg(long, alias = "input-voltage")]
        vin: String,
        /// Target Zener voltage (3.3V, 5.1V, or 12V)
        #[arg(long, alias = "zener-voltage")]
        vz: String,
        /// Target load current (e.g. 10mA)
        #[arg(long, alias = "load-current")]
        iload: String,
        /// Omit for 5mA minimum Zener bias current
        #[arg(long, aliases = ["zener-knee-current", "iz"])]
        iz_min: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ValueSeries {
    Exact,
    E12,
    E24,
}

impl From<ValueSeries> for PreferredValues {
    fn from(value: ValueSeries) -> Self {
        match value {
            ValueSeries::Exact => Self::Exact,
            ValueSeries::E12 => Self::E12,
            ValueSeries::E24 => Self::E24,
        }
    }
}

#[derive(Args)]
struct RenderCommand {
    file: PathBuf,

    /// Output path; extension selects svg, png, or pdf
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Raster scale for PNG output (0.25 through 8)
    #[arg(long, default_value_t = 2.0)]
    scale: f32,

    /// PNG/SVG page background
    #[arg(long, value_enum, default_value_t = Background::White)]
    background: Background,

    /// Allow overwriting an existing output file
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct ExportCommand {
    file: PathBuf,

    /// Export target
    #[arg(long, value_enum)]
    target: ExportTarget,

    /// Output path; defaults to the source basename and target extension
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Allow overwriting an existing output file
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct OutputCommand {
    file: PathBuf,

    /// Write the SPICE netlist to this path (default: source path with .spice)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Allow overwriting an existing output file
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct TestCommand {
    #[command(flatten)]
    output: OutputCommand,

    /// Evaluate an external assertion-only .kessreq file instead of inline assertions
    #[arg(long)]
    requirements: Option<PathBuf>,

    /// Require the external requirements file to match this SHA-256 digest
    #[arg(long, requires = "requirements")]
    requirements_sha256: Option<String>,
}

#[derive(Clone, Copy, ValueEnum, PartialEq, Eq)]
pub(crate) enum Format {
    Human,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum ExportTarget {
    SchematicJson,
    Spice,
    Kicad,
    Ltspice,
    BomCsv,
    HandoffJson,
}

impl From<ExportTarget> for ExportFormat {
    fn from(value: ExportTarget) -> Self {
        match value {
            ExportTarget::SchematicJson => Self::SchematicJson,
            ExportTarget::Spice => Self::Spice,
            ExportTarget::Kicad => Self::Kicad,
            ExportTarget::Ltspice => Self::Ltspice,
            ExportTarget::BomCsv => Self::BomCsv,
            ExportTarget::HandoffJson => Self::HandoffJson,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum Background {
    White,
    Transparent,
}

impl From<Background> for RenderBackground {
    fn from(value: Background) -> Self {
        match value {
            Background::White => Self::White,
            Background::Transparent => Self::Transparent,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
#[value(rename_all = "kebab-case")]
pub(crate) enum Include {
    Ast,
    Ir,
    Graph,
    Spice,
    Datasets,
    Simulation,
    Models,
    RawLog,
    EffectiveSource,
}

#[derive(Serialize)]
pub(crate) struct JsonOutput {
    pub(crate) schema_version: &'static str,
    pub(crate) command: &'static str,
    pub(crate) status: String,
    pub(crate) domain_versions: DomainVersions,
    pub(crate) diagnostics: Vec<JsonDiagnostic>,
    pub(crate) summary: JsonSummary,
    pub(crate) measurements: BTreeMap<String, f64>,
    pub(crate) assertions: Option<AssertionReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) part_stress: Option<PartStressReport>,
    pub(crate) requirements: Option<JsonRequirements>,
    pub(crate) artifacts: Vec<JsonArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) calculation: Option<ToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) study: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) fit: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) capabilities: Option<CapabilityManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) debug: Option<BTreeMap<String, Value>>,
}

#[derive(Serialize)]
pub(crate) struct DomainVersions {
    pub(crate) compile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) capabilities: Option<&'static str>,
    pub(crate) simulation: Option<&'static str>,
    pub(crate) measurement: Option<&'static str>,
    pub(crate) assertion: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) part_stress: Option<&'static str>,
    pub(crate) requirements: Option<&'static str>,
    pub(crate) export: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) experiment: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) research_data: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) fit: Option<&'static str>,
}

#[derive(Clone, Serialize)]
pub(crate) struct JsonRequirements {
    pub(crate) schema_version: String,
    pub(crate) sha256: String,
    pub(crate) assertion_count: usize,
    pub(crate) hash_pinned: bool,
}

#[derive(Serialize)]
pub(crate) struct JsonDiagnostic {
    pub(crate) code: String,
    pub(crate) severity: String,
    pub(crate) stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) column: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct JsonSummary {
    pub(crate) errors: usize,
    pub(crate) warnings: usize,
    pub(crate) info: usize,
    pub(crate) analyses: usize,
    pub(crate) measurements: usize,
    pub(crate) assertions: Option<AssertionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) part_stress: Option<PartStressSummary>,
}

#[derive(Clone, Serialize)]
pub(crate) struct JsonArtifact {
    pub(crate) kind: String,
    pub(crate) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exporter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exporter_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) byte_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) connectivity_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) capability: Option<ExportCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) losses: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    process::exit(run(cli));
}

fn run(cli: Cli) -> i32 {
    let command = command_name(&cli.command);
    let includes = cli.include.iter().copied().collect::<BTreeSet<_>>();
    if cli.schema_version != CLI_SCHEMA_VERSION {
        let diagnostic = diagnostic(
            "KES-F002",
            DiagnosticStage::Cli,
            format!(
                "Unsupported CLI schema '{}'; expected '{}'.",
                cli.schema_version, CLI_SCHEMA_VERSION
            ),
        );
        emit(
            &cli.format,
            command,
            &includes,
            "error",
            CompileReport::failure(diagnostic),
            None,
            None,
            None,
        );
        return 2;
    }

    if matches!(cli.command, Commands::Capabilities) {
        if !cli.parameters.is_empty() {
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                CompileReport::failure(diagnostic(
                    "KES-F002",
                    DiagnosticStage::Cli,
                    "--param applies to circuit commands, not capability discovery",
                )),
                None,
                None,
                None,
            );
            return 2;
        }
        return run_capabilities(&cli.format, &includes);
    }

    if matches!(cli.command, Commands::Mcp) {
        return mcp::serve();
    }

    if let Commands::Import(import) = &cli.command {
        if !cli.parameters.is_empty() {
            let mut report = import_spice("");
            report.diagnostics.clear();
            report.diagnostics.push(ImportDiagnostic {
                code: "KES-F002".into(),
                severity: ImportSeverity::Error,
                message: "--param applies after import to editable Kessetsu circuits".into(),
                line: None,
                column: None,
            });
            return import::emit_import(&cli.format, &report, None);
        }
        return import::run_spice_import(import, &cli.format);
    }

    if let Commands::Data(data) = &cli.command {
        if !cli.parameters.is_empty() {
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                CompileReport::failure(diagnostic(
                    "KES-F002",
                    DiagnosticStage::Cli,
                    "--param applies to circuits, not research-data imports/comparisons",
                )),
                None,
                None,
                None,
            );
            return 2;
        }
        return research::run_data(data, &cli.format, &includes);
    }
    if let Commands::Fit(fit) = &cli.command {
        if !cli.parameters.is_empty() {
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                CompileReport::failure(diagnostic(
                    "KES-F002",
                    DiagnosticStage::Cli,
                    "--param applies to circuits, not completed fit evaluation",
                )),
                None,
                None,
                None,
            );
            return 2;
        }
        return research::run_fit(fit, &cli.format, &includes);
    }
    if let Commands::Tool(tool) = &cli.command {
        if !cli.parameters.is_empty() {
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                CompileReport::failure(diagnostic(
                    "KES-F002",
                    DiagnosticStage::Cli,
                    "--param applies to circuit commands, not tool calculations",
                )),
                None,
                None,
                None,
            );
            return 2;
        }
        return tool::run_tool(tool, &cli.format, &includes);
    }
    if let Commands::Study(study) = &cli.command {
        if !cli.parameters.is_empty() {
            return research::emit_study_error(
                &cli.format,
                &includes,
                "Use study axes/revision inputs, not --param",
            );
        }
        return research::run_study(study, &cli.format, &includes);
    }
    let mut inputs = CompileInputs::default();
    for parameter in &cli.parameters {
        let Some((name, value)) = parameter.split_once('=') else {
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                CompileReport::failure(diagnostic(
                    "KES-F002",
                    DiagnosticStage::Cli,
                    "--param requires NAME=VALUE, for example supply=15V",
                )),
                None,
                None,
                None,
            );
            return 2;
        };
        inputs.parameters.push(ParameterInput {
            name: name.into(),
            value: value.into(),
        });
    }
    let source_path = command_path(&cli.command);
    let source_is_stdin = source_path == Path::new("-");
    let source = match read_source(source_path) {
        Ok(source) => source,
        Err(error) => {
            let diagnostic = diagnostic(
                "KES-I001",
                DiagnosticStage::Io,
                if source_is_stdin {
                    format!("Could not read Kessetsu source from stdin: {error}")
                } else {
                    format!("Could not read file '{}': {error}", source_path.display())
                },
            );
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                CompileReport::failure(diagnostic),
                None,
                None,
                None,
            );
            return 2;
        }
    };
    let mut import_notes = Vec::new();
    let source = if resolve_input_language(cli.input, source_path, &source) == InputLanguage::Spice
    {
        let imported = import_spice(&source);
        let mut converted = imported
            .diagnostics
            .iter()
            .map(import_diagnostic)
            .collect::<Vec<_>>();
        match imported.kess_source {
            Some(kess) if !imported.has_errors() => {
                import_notes = converted;
                kess
            }
            _ => {
                if converted.is_empty() {
                    converted.push(diagnostic(
                        "KES-N005",
                        DiagnosticStage::Parse,
                        "SPICE input could not be converted into canonical Kessetsu source",
                    ));
                }
                let mut report = CompileReport::failure(converted.remove(0));
                report.diagnostics.extend(converted);
                emit(
                    &cli.format,
                    command,
                    &includes,
                    "error",
                    report,
                    None,
                    None,
                    None,
                );
                return 2;
            }
        }
    } else {
        source
    };

    let needs_spice = !matches!(cli.command, Commands::Check { .. });
    let needs_schematic = matches!(cli.command, Commands::Render(_) | Commands::Export(_));
    let options = CompileOptions {
        include_ast: includes.contains(&Include::Ast),
        generate_spice: needs_spice,
        generate_layout: needs_schematic,
        generate_kicad: false,
    };
    let external_resources =
        match load_external_model_resources(source_path, source_is_stdin, &source) {
            Ok(resources) => resources,
            Err(diagnostic) => {
                emit(
                    &cli.format,
                    command,
                    &includes,
                    "error",
                    CompileReport::failure(*diagnostic),
                    None,
                    None,
                    None,
                );
                return 2;
            }
        };
    let mut report = compile_source_with_inputs(&source, options, &inputs, &external_resources);
    if !import_notes.is_empty() {
        report.diagnostics.splice(0..0, import_notes);
    }
    if includes.contains(&Include::EffectiveSource)
        && !report.has_errors()
        && report.effective_source.is_none()
    {
        report.effective_source = Some(source.clone());
    }

    if report.has_errors() {
        let exit_code = compile_failure_exit_code(&report);
        emit(
            &cli.format,
            command,
            &includes,
            "error",
            report,
            None,
            None,
            None,
        );
        return exit_code;
    }

    let requirements = if let Commands::Test(test) = &cli.command {
        match attach_external_requirements(test, &mut report) {
            Ok(requirements) => requirements,
            Err(diagnostic) => {
                report.diagnostics.push(*diagnostic);
                let exit_code = compile_failure_exit_code(&report);
                emit(
                    &cli.format,
                    command,
                    &includes,
                    "error",
                    report,
                    None,
                    None,
                    None,
                );
                return exit_code;
            }
        }
    } else {
        None
    };

    if matches!(cli.command, Commands::Check { .. }) {
        if cli.format == Format::Human {
            emit_human_diagnostics(&report);
            println!("[SUCCESS] Circuit parsed and ERC checks passed.");
        } else {
            emit(
                &cli.format,
                command,
                &includes,
                "success",
                report,
                None,
                None,
                None,
            );
        }
        return 0;
    }

    if let Commands::Render(render) = &cli.command {
        let render_format = match render_format(render) {
            Ok(format) => format,
            Err(message) => {
                report
                    .diagnostics
                    .push(diagnostic("KES-X017", DiagnosticStage::Cli, message));
                emit(
                    &cli.format,
                    command,
                    &includes,
                    "error",
                    report,
                    None,
                    None,
                    None,
                );
                return 2;
            }
        };
        return run_artifact_command(
            &cli.format,
            command,
            &includes,
            source_path,
            source_is_stdin,
            report,
            render_format,
            render.output.clone(),
            render.force,
            ExportOptions {
                scale: render.scale,
                background: render.background.into(),
            },
        );
    }
    if let Commands::Export(export) = &cli.command {
        return run_artifact_command(
            &cli.format,
            command,
            &includes,
            source_path,
            source_is_stdin,
            report,
            export.target.into(),
            export.output.clone(),
            export.force,
            ExportOptions::default(),
        );
    }

    let output_command = output_command(&cli.command)
        .expect("every non-check, non-render command must define an output policy");
    let spice_path = output_command
        .output
        .clone()
        .or_else(|| (!source_is_stdin).then(|| source_path.with_extension("spice")));
    let spice = report
        .spice_netlist
        .as_deref()
        .expect("successful SPICE-enabled compile must contain a netlist")
        .to_string();
    let model_lock_path = spice_path.as_ref().and_then(|path| {
        report.model_lock.as_ref().map(|_| {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join("kessetsu.lock")
        })
    });

    if let Some(spice_path) = &spice_path
        && let Err(diagnostic) = validate_external_model_output_location(
            &report,
            source_path,
            spice_path,
            ExportFormat::Spice,
        )
    {
        report.diagnostics.push(*diagnostic);
        emit(
            &cli.format,
            command,
            &includes,
            "error",
            report,
            None,
            None,
            None,
        );
        return 2;
    }

    // Check both destinations before changing either file. A lock collision must
    // not leave a newly written netlist behind.
    let preflight = (|| {
        if let Some(path) = &spice_path {
            validate_spice_destination(source_path, path, output_command.force)?;
        }
        match (&model_lock_path, &spice_path, report.model_lock.as_deref()) {
            (Some(lock), Some(spice), Some(contents)) => {
                should_write_model_lock(source_path, spice, lock, contents, output_command.force)
            }
            _ => Ok(false),
        }
    })();
    let write_lock = match preflight {
        Ok(write_lock) => write_lock,
        Err(diagnostic) => {
            report.diagnostics.push(*diagnostic);
            report.spice_netlist = None;
            emit(
                &cli.format,
                command,
                &includes,
                "error",
                report,
                None,
                None,
                None,
            );
            return 2;
        }
    };

    if let Some(spice_path) = &spice_path
        && let Err(diagnostic) = write_spice(source_path, spice_path, &spice, output_command.force)
    {
        report.diagnostics.push(*diagnostic);
        report.spice_netlist = None;
        emit(
            &cli.format,
            command,
            &includes,
            "error",
            report,
            None,
            None,
            None,
        );
        return 2;
    }

    if write_lock
        && let (Some(lock_path), Some(model_lock)) =
            (&model_lock_path, report.model_lock.as_deref())
        && let Err(error) = fs::write(lock_path, model_lock)
    {
        report.diagnostics.push(diagnostic(
            "KES-I005",
            DiagnosticStage::Io,
            format!(
                "Could not write model lockfile '{}': {error}",
                lock_path.display()
            ),
        ));
        emit(
            &cli.format,
            command,
            &includes,
            "error",
            report,
            None,
            None,
            None,
        );
        return 2;
    }

    let spice_file = spice_path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    if matches!(cli.command, Commands::Compile(_)) {
        if cli.format == Format::Human {
            emit_human_diagnostics(&report);
            if let Some(spice_path) = &spice_path {
                println!(
                    "[SUCCESS] SPICE netlist generated: {}",
                    spice_path.display()
                );
                if let Some(lock_path) = &model_lock_path {
                    println!("[SUCCESS] Model lock generated: {}", lock_path.display());
                }
            } else {
                print!("{spice}");
                println!("[SUCCESS] SPICE netlist generated in memory.");
            }
        } else {
            emit(
                &cli.format,
                command,
                &includes,
                "success",
                report,
                spice_file,
                None,
                None,
            );
        }
        return 0;
    }

    if matches!(cli.command, Commands::Simulate(_)) {
        return run_simulate(
            &cli.format,
            command,
            &includes,
            report,
            spice_file,
            &spice,
            &external_resources,
        );
    }

    run_assertions(
        &cli.format,
        command,
        &includes,
        report,
        spice_file,
        &spice,
        &external_resources,
        requirements,
    )
}

fn run_capabilities(format: &Format, includes: &BTreeSet<Include>) -> i32 {
    let manifest = capability_manifest(CLI_SCHEMA_VERSION);
    if *format == Format::Json {
        let report = kessetsu_core::compiler::compile_source("", CompileOptions::default());
        let mut output = build_json_output(
            "capabilities",
            includes,
            "success",
            &report,
            None,
            None,
            None,
            None,
        );
        output.domain_versions.capabilities = Some(CAPABILITIES_SCHEMA_VERSION);
        output.capabilities = Some(manifest);
        println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .expect("static capability manifest must serialize")
        );
    } else {
        println!(
            "Kessetsu {} — machine-readable capability discovery",
            manifest.product.version
        );
        println!("Schema: {}", manifest.schema_version);
        println!(
            "Commands: {}",
            manifest
                .commands
                .iter()
                .filter(|command| command.name != "capabilities")
                .map(|command| command.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("Analyses: {}", manifest.language.analyses.join(", "));
        println!(
            "Exports: {}",
            manifest
                .exports
                .iter()
                .map(|export| export.format.id())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("Docs: {}", manifest.product.documentation);
        println!("Agent JSON: kess capabilities --format json");
    }
    0
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Capabilities => "capabilities",
        Commands::Import(_) => "import",
        Commands::Data(_) => "data",
        Commands::Fit(_) => "fit",
        Commands::Study(_) => "study",
        Commands::Tool(_) => "tool",
        Commands::Check { .. } => "check",
        Commands::Compile(_) => "compile",
        Commands::Simulate(_) => "simulate",
        Commands::Test(_) => "test",
        Commands::Render(_) => "render",
        Commands::Export(_) => "export",
        Commands::Mcp => "mcp",
    }
}

pub(crate) fn read_source(source_path: &Path) -> io::Result<String> {
    if source_path == Path::new("-") {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        Ok(source)
    } else {
        fs::read_to_string(source_path)
    }
}

/// Resolves `--input auto`: SPICE file extensions, or stdin/unknown text containing a line
/// that starts with a SPICE dot directive. Kessetsu source never starts a line with `.`.
fn resolve_input_language(requested: InputLanguage, path: &Path, source: &str) -> InputLanguage {
    if requested != InputLanguage::Auto {
        return requested;
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("kess") => InputLanguage::Kess,
        Some("cir" | "sp" | "spice" | "net" | "ckt" | "cki") => InputLanguage::Spice,
        _ if source
            .lines()
            .any(|line| line.trim_start().starts_with('.')) =>
        {
            InputLanguage::Spice
        }
        _ => InputLanguage::Kess,
    }
}

fn import_diagnostic(diagnostic: &ImportDiagnostic) -> Diagnostic {
    Diagnostic {
        code: diagnostic.code.clone(),
        severity: match diagnostic.severity {
            ImportSeverity::Error => DiagnosticSeverity::Error,
            ImportSeverity::Warning => DiagnosticSeverity::Warning,
            ImportSeverity::Info => DiagnosticSeverity::Info,
        },
        stage: DiagnosticStage::Parse,
        message: format!("SPICE input: {}", diagnostic.message),
        component: None,
        pin: None,
        field: None,
        line: diagnostic.line,
        column: diagnostic.column,
    }
}

pub(crate) fn load_external_model_resources(
    source_path: &Path,
    source_is_stdin: bool,
    source: &str,
) -> Result<ExternalModelResources, Box<Diagnostic>> {
    let Ok(program) = parse_program(source) else {
        return Ok(ExternalModelResources::new());
    };
    if program.external_subcircuits.is_empty() || source_is_stdin {
        return Ok(ExternalModelResources::new());
    }
    let source_directory = source_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .map_err(|error| {
            Box::new(diagnostic(
                "KES-I006",
                DiagnosticStage::Io,
                format!("Could not resolve the Kessetsu source directory: {error}"),
            ))
        })?;
    let mut resources = ExternalModelResources::new();
    for declaration in &program.external_subcircuits {
        let Some(reference) = declaration
            .parameters
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case("file"))
            .map(|field| field.value.as_str())
        else {
            continue;
        };
        if validate_external_resource_reference(reference).is_err()
            || resources.contains_key(reference)
        {
            continue;
        }
        let candidate = reference
            .split('/')
            .fold(source_directory.clone(), |path, segment| path.join(segment));
        let resolved = candidate.canonicalize().map_err(|error| {
            Box::new(diagnostic(
                "KES-I006",
                DiagnosticStage::Io,
                format!("Could not read external model resource '{reference}': {error}"),
            ))
        })?;
        if !resolved.starts_with(&source_directory) || !resolved.is_file() {
            return Err(Box::new(diagnostic(
                "KES-I006",
                DiagnosticStage::Io,
                format!(
                    "External model resource '{reference}' must resolve to a file inside the source directory"
                ),
            )));
        }
        let length = fs::metadata(&resolved)
            .map_err(|error| {
                Box::new(diagnostic(
                    "KES-I006",
                    DiagnosticStage::Io,
                    format!("Could not inspect external model resource '{reference}': {error}"),
                ))
            })?
            .len();
        if length > MAX_EXTERNAL_MODEL_BYTES as u64 {
            return Err(Box::new(diagnostic(
                "KES-I006",
                DiagnosticStage::Io,
                format!(
                    "External model resource '{reference}' exceeds the {MAX_EXTERNAL_MODEL_BYTES} byte limit"
                ),
            )));
        }
        let bytes = fs::read(&resolved).map_err(|error| {
            Box::new(diagnostic(
                "KES-I006",
                DiagnosticStage::Io,
                format!("Could not read external model resource '{reference}': {error}"),
            ))
        })?;
        resources.insert(reference.to_string(), bytes);
    }
    Ok(resources)
}

fn command_path(command: &Commands) -> &Path {
    match command {
        Commands::Capabilities => {
            unreachable!("capability discovery returns before reading circuit source")
        }
        Commands::Import(_)
        | Commands::Data(_)
        | Commands::Fit(_)
        | Commands::Study(_)
        | Commands::Tool(_)
        | Commands::Mcp => {
            unreachable!("non-circuit commands are handled before reading source")
        }
        Commands::Check { file } => file,
        Commands::Render(command) => &command.file,
        Commands::Export(command) => &command.file,
        Commands::Compile(command) | Commands::Simulate(command) => &command.file,
        Commands::Test(command) => &command.output.file,
    }
}

fn output_command(command: &Commands) -> Option<&OutputCommand> {
    match command {
        Commands::Compile(command) | Commands::Simulate(command) => Some(command),
        Commands::Test(command) => Some(&command.output),
        Commands::Data(_)
        | Commands::Capabilities
        | Commands::Import(_)
        | Commands::Fit(_)
        | Commands::Study(_)
        | Commands::Check { .. }
        | Commands::Render(_)
        | Commands::Export(_)
        | Commands::Tool(_)
        | Commands::Mcp => None,
    }
}

fn attach_external_requirements(
    command: &TestCommand,
    report: &mut CompileReport,
) -> Result<Option<JsonRequirements>, Box<Diagnostic>> {
    let Some(path) = command.requirements.as_deref() else {
        return Ok(None);
    };
    if report
        .ir
        .as_ref()
        .is_some_and(|circuit| !circuit.assertions.is_empty())
    {
        return Err(Box::new(diagnostic(
            "KES-R003",
            DiagnosticStage::Requirements,
            "a test cannot combine inline assertions with --requirements; keep exactly one requirement authority",
        )));
    }

    let bytes = fs::read(path).map_err(|error| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("requirements file");
        Box::new(diagnostic(
            "KES-I008",
            DiagnosticStage::Io,
            format!("Could not read requirements file '{name}': {error}"),
        ))
    })?;
    let RequirementSet {
        schema_version,
        sha256,
        assertions,
    } = compile_requirements(&bytes).map_err(|error| {
        Box::new(Diagnostic {
            code: error.code,
            severity: DiagnosticSeverity::Error,
            stage: DiagnosticStage::Requirements,
            message: error.message,
            component: None,
            pin: None,
            field: None,
            line: error.line,
            column: error.column,
        })
    })?;

    let hash_pinned = command.requirements_sha256.is_some();
    if let Some(expected) = command.requirements_sha256.as_deref() {
        let expected = normalize_sha256(expected).map_err(|message| {
            Box::new(diagnostic(
                "KES-R004",
                DiagnosticStage::Requirements,
                message,
            ))
        })?;
        if expected != sha256 {
            return Err(Box::new(diagnostic(
                "KES-R004",
                DiagnosticStage::Requirements,
                format!("requirements hash mismatch: expected {expected}, got {sha256}"),
            )));
        }
    }

    let assertion_count = assertions.len();
    report
        .ir
        .as_mut()
        .expect("successful compile report must preserve typed IR")
        .assertions = assertions;
    Ok(Some(JsonRequirements {
        schema_version,
        sha256,
        assertion_count,
        hash_pinned,
    }))
}

fn normalize_sha256(value: &str) -> Result<String, String> {
    let digest = value.strip_prefix("sha256:").unwrap_or(value);
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "--requirements-sha256 must contain exactly 64 hexadecimal characters, optionally prefixed by 'sha256:'"
                .to_string(),
        );
    }
    Ok(format!("sha256:{}", digest.to_ascii_lowercase()))
}

fn render_format(command: &RenderCommand) -> Result<ExportFormat, String> {
    let extension = command
        .output
        .as_ref()
        .and_then(|path| path.extension())
        .and_then(|extension| extension.to_str())
        .unwrap_or("svg")
        .to_ascii_lowercase();
    match extension.as_str() {
        "svg" => Ok(ExportFormat::Svg),
        "png" => Ok(ExportFormat::Png),
        "pdf" => Ok(ExportFormat::Pdf),
        _ => Err(format!(
            "render output extension '.{extension}' is unsupported; expected .svg, .png, or .pdf"
        )),
    }
}

fn artifact_output_path(
    source_path: &Path,
    source_is_stdin: bool,
    requested: Option<PathBuf>,
    format: ExportFormat,
) -> Result<PathBuf, String> {
    if let Some(path) = requested {
        return Ok(path);
    }
    if source_is_stdin {
        return Err("--output is required when source is read from stdin".to_string());
    }
    Ok(source_path.with_extension(format.extension()))
}

fn write_artifact(
    source_path: &Path,
    output_path: &Path,
    bytes: &[u8],
    force: bool,
) -> Result<(), Box<Diagnostic>> {
    if paths_refer_to_same_file(source_path, output_path) {
        return Err(Box::new(diagnostic(
            "KES-I002",
            DiagnosticStage::Io,
            format!(
                "Refusing to overwrite source file '{}' with a generated artifact.",
                source_path.display()
            ),
        )));
    }
    if output_path.exists() && !force {
        return Err(Box::new(diagnostic(
            "KES-I003",
            DiagnosticStage::Io,
            format!(
                "Output file '{}' already exists; pass --force to overwrite it.",
                output_path.display()
            ),
        )));
    }
    fs::write(output_path, bytes).map_err(|error| {
        Box::new(diagnostic(
            "KES-I004",
            DiagnosticStage::Io,
            format!(
                "Could not write artifact to '{}': {error}",
                output_path.display()
            ),
        ))
    })
}

#[allow(clippy::too_many_arguments)]
fn run_artifact_command(
    output_format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    source_path: &Path,
    source_is_stdin: bool,
    mut report: CompileReport,
    artifact_format: ExportFormat,
    requested_path: Option<PathBuf>,
    force: bool,
    options: ExportOptions,
) -> i32 {
    let output_path = match artifact_output_path(
        source_path,
        source_is_stdin,
        requested_path,
        artifact_format,
    ) {
        Ok(path) => path,
        Err(message) => {
            report
                .diagnostics
                .push(diagnostic("KES-I006", DiagnosticStage::Io, message));
            emit(
                output_format,
                command,
                includes,
                "error",
                report,
                None,
                None,
                None,
            );
            return 2;
        }
    };
    if let Err(diagnostic) =
        validate_external_model_output_location(&report, source_path, &output_path, artifact_format)
    {
        report.diagnostics.push(*diagnostic);
        emit(
            output_format,
            command,
            includes,
            "error",
            report,
            None,
            None,
            None,
        );
        return 2;
    }
    let artifact = match export_report(&report, artifact_format, options) {
        Ok(artifact) => artifact,
        Err(error) => {
            report.diagnostics.extend(error.diagnostics);
            report.diagnostics.push(diagnostic(
                &error.code,
                DiagnosticStage::Schematic,
                error.message,
            ));
            emit(
                output_format,
                command,
                includes,
                "error",
                report,
                None,
                None,
                None,
            );
            return 2;
        }
    };
    if let Err(diagnostic) = write_artifact(source_path, &output_path, &artifact.bytes, force) {
        report.diagnostics.push(*diagnostic);
        emit(
            output_format,
            command,
            includes,
            "error",
            report,
            None,
            None,
            None,
        );
        return 2;
    }

    if *output_format == Format::Human {
        emit_human_diagnostics(&report);
        println!(
            "[SUCCESS] {} artifact generated: {}",
            artifact.format.id(),
            output_path.display()
        );
        for warning in &artifact.warnings {
            println!("[WARNING] {warning}");
        }
        for loss in &artifact.losses {
            println!("[LOSS] {loss}");
        }
    } else {
        let mut output = build_json_output(
            command, includes, "success", &report, None, None, None, None,
        );
        output.domain_versions.export = Some(EXPORT_SCHEMA_VERSION);
        output.artifacts.push(json_export_artifact(
            &artifact,
            output_path.to_string_lossy().into_owned(),
        ));
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("[ERROR] Could not serialize {CLI_SCHEMA_VERSION} JSON output: {error}")
            }
        }
    }
    0
}

fn json_export_artifact(artifact: &ExportArtifact, path: String) -> JsonArtifact {
    JsonArtifact {
        kind: artifact.format.id().to_string(),
        path,
        schema_version: Some(artifact.schema_version.clone()),
        exporter: Some(artifact.exporter.clone()),
        exporter_version: Some(artifact.exporter_version),
        sha256: Some(artifact.sha256.clone()),
        byte_length: Some(artifact.byte_length),
        connectivity_verified: Some(artifact.connectivity_verified),
        capability: Some(artifact.capability),
        warnings: artifact.warnings.clone(),
        losses: artifact.losses.clone(),
    }
}

pub(crate) fn plain_json_artifact(
    kind: impl Into<String>,
    path: impl Into<String>,
) -> JsonArtifact {
    JsonArtifact {
        kind: kind.into(),
        path: path.into(),
        schema_version: None,
        exporter: None,
        exporter_version: None,
        sha256: None,
        byte_length: None,
        connectivity_verified: None,
        capability: None,
        warnings: Vec::new(),
        losses: Vec::new(),
    }
}

fn write_spice(
    source_path: &Path,
    output_path: &Path,
    spice: &str,
    force: bool,
) -> Result<(), Box<Diagnostic>> {
    validate_spice_destination(source_path, output_path, force)?;
    fs::write(output_path, spice).map_err(|error| {
        Box::new(diagnostic(
            "KES-I004",
            DiagnosticStage::Io,
            format!(
                "Could not write SPICE file to '{}': {error}",
                output_path.display()
            ),
        ))
    })
}

fn validate_spice_destination(
    source_path: &Path,
    output_path: &Path,
    force: bool,
) -> Result<(), Box<Diagnostic>> {
    if paths_refer_to_same_file(source_path, output_path) {
        return Err(Box::new(diagnostic(
            "KES-I002",
            DiagnosticStage::Io,
            format!(
                "Refusing to overwrite source file '{}' with generated SPICE.",
                source_path.display()
            ),
        )));
    }

    if output_path.exists() && !force {
        return Err(Box::new(diagnostic(
            "KES-I003",
            DiagnosticStage::Io,
            format!(
                "Output file '{}' already exists; pass --force to overwrite it.",
                output_path.display()
            ),
        )));
    }

    Ok(())
}

fn should_write_model_lock(
    source_path: &Path,
    spice_path: &Path,
    lock_path: &Path,
    contents: &str,
    force: bool,
) -> Result<bool, Box<Diagnostic>> {
    if paths_refer_to_same_file(source_path, lock_path)
        || paths_refer_to_same_file(spice_path, lock_path)
    {
        return Err(Box::new(diagnostic(
            "KES-I002",
            DiagnosticStage::Io,
            format!(
                "Model lockfile '{}' must not overwrite the source or share the SPICE destination.",
                lock_path.display()
            ),
        )));
    }
    // Reuse identical bytes without rewriting or requiring --force. Different
    // models in a shared directory must not silently replace an existing lock.
    if fs::read(lock_path).is_ok_and(|existing| existing == contents.as_bytes()) {
        return Ok(false);
    }
    if lock_path.exists() && !force {
        return Err(Box::new(diagnostic(
            "KES-I003",
            DiagnosticStage::Io,
            format!(
                "Model lockfile '{}' already exists with different or unreadable contents; pass --force to overwrite it.",
                lock_path.display()
            ),
        )));
    }
    Ok(true)
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn validate_external_model_output_location(
    report: &CompileReport,
    source_path: &Path,
    output_path: &Path,
    format: ExportFormat,
) -> Result<(), Box<Diagnostic>> {
    let has_external_models = report.ir.as_ref().is_some_and(|circuit| {
        circuit
            .model_manifest
            .models
            .iter()
            .any(|model| model.external.is_some())
    });
    if !has_external_models || !matches!(format, ExportFormat::Spice | ExportFormat::Ltspice) {
        return Ok(());
    }
    let source_directory = source_path.parent().unwrap_or_else(|| Path::new("."));
    let output_directory = output_path.parent().unwrap_or_else(|| Path::new("."));
    let same_directory = match (
        source_directory.canonicalize(),
        output_directory.canonicalize(),
    ) {
        (Ok(source), Ok(output)) => source == output,
        _ => source_directory == output_directory,
    };
    if same_directory {
        Ok(())
    } else {
        Err(Box::new(diagnostic(
            "KES-I007",
            DiagnosticStage::Io,
            "SPICE/LTspice output with an external model dependency must stay beside its .kess source so the validated relative model reference remains usable",
        )))
    }
}

fn run_simulate(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    mut report: CompileReport,
    spice_file: Option<String>,
    spice: &str,
    external_resources: &ExternalModelResources,
) -> i32 {
    if *format == Format::Human {
        println!("[INFO] Running ngspice simulation...");
    }

    let simulation = match run_simulation(&report, spice, external_resources) {
        Ok(simulation) => simulation,
        Err(error) => {
            report.diagnostics.push(diagnostic(
                "KES-S001",
                DiagnosticStage::Simulation,
                error.to_string(),
            ));
            emit(
                format,
                command,
                includes,
                "simulation_error",
                report,
                spice_file,
                None,
                None,
            );
            return 3;
        }
    };

    if *format == Format::Human {
        print_simulation_result(&simulation, includes.contains(&Include::RawLog));
    }

    if !simulation.succeeded() {
        let details = if simulation.errors.is_empty() {
            format!("Ngspice ended with {:?} status.", simulation.status)
        } else {
            format!("Ngspice errors: {}", simulation.errors.join(" | "))
        };
        report
            .diagnostics
            .push(diagnostic("KES-S002", DiagnosticStage::Simulation, details));
        emit(
            format,
            command,
            includes,
            "simulation_error",
            report,
            spice_file,
            Some(simulation),
            None,
        );
        return 3;
    }

    if *format == Format::Human {
        let circuit = report
            .ir
            .as_ref()
            .expect("successful compile report must preserve typed IR");
        print_part_stress_report(&evaluate_part_stress(circuit, &simulation));
    }

    if *format == Format::Json {
        emit(
            format,
            command,
            includes,
            "success",
            report,
            spice_file,
            Some(simulation),
            None,
        );
    } else {
        println!("[SUCCESS] Simulation completed.");
    }
    0
}

fn print_simulation_result(simulation: &SimulationResult, include_raw_log: bool) {
    for diagnostic in &simulation.diagnostics {
        eprintln!(
            "[{:?} Simulation] {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
    println!(
        "[INFO] {} analysis dataset(s), {} measurement(s).",
        simulation.datasets.len(),
        simulation.measurements.len()
    );
    for (name, value) in &simulation.measurements {
        println!("[MEASURE] {name} = {value}");
    }
    if include_raw_log {
        print_simulator_logs(simulation);
    }
}

fn print_part_stress_report(report: &PartStressReport) {
    if report.results.is_empty() {
        return;
    }
    println!("[INFO] Provided part-limit comparison (advisory):");
    for result in &report.results {
        let status = match result.status {
            PartStressStatus::WithinProvidedLimit => "WITHIN",
            PartStressStatus::ExceedsProvidedLimit => "EXCEEDS",
            PartStressStatus::Unavailable => "UNAVAILABLE",
        };
        let actual = result
            .actual
            .map(|actual| format_quantity(actual, result.limit.unit))
            .unwrap_or_else(|| "not available".into());
        let utilization = result
            .utilization_percent
            .map(|value| format!("; {value:.1}% of limit"))
            .unwrap_or_default();
        println!(
            "[PART {status}] {} {} = {} / {}{}",
            result.component,
            result.metric,
            actual,
            format_quantity(result.limit.value, result.limit.unit),
            utilization
        );
        println!("       Conditions: {}", result.conditions);
        if let Some(source) = &result.source {
            println!("       Source: {source}");
        }
        if let Some(message) = &result.message {
            println!("       {message}");
        }
    }
    println!("       {}", report.disclaimer);
}

fn print_simulator_logs(simulation: &SimulationResult) {
    if !simulation.raw_log.stdout.is_empty() {
        println!("\n--- NGSPICE OUTPUT ---\n{}", simulation.raw_log.stdout);
    }
    if !simulation.raw_log.stderr.is_empty() {
        eprintln!("\n--- NGSPICE ERRORS ---\n{}", simulation.raw_log.stderr);
    }
}

fn run_simulation(
    report: &CompileReport,
    spice: &str,
    external_resources: &ExternalModelResources,
) -> Result<SimulationResult, kessetsu_core::simulation::SimulationRunError> {
    let analyses = report
        .ir
        .as_ref()
        .expect("successful compile report must preserve typed IR")
        .analyses
        .clone();
    let request = SimulationRequest::new(spice, analyses);
    let mut context = NativeSimulationContext::default();
    for model in &report
        .ir
        .as_ref()
        .expect("successful compile report must preserve typed IR")
        .model_manifest
        .models
    {
        if let Some(external) = &model.external {
            if external.simulator == kessetsu_core::ir::SimulatorCompatibility::NgspicePs {
                context.compatibility = kessetsu_core::ir::SimulatorCompatibility::NgspicePs;
            }
            if let Some(bytes) = external_resources.get(&external.resource) {
                context
                    .resources
                    .insert(external.resource.clone(), bytes.clone());
            }
        }
    }
    NgspiceRunner::discover().run_with_context(&request, &context, &CancellationToken::new())
}

#[allow(clippy::too_many_arguments)]
fn run_assertions(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    mut report: CompileReport,
    spice_file: Option<String>,
    spice: &str,
    external_resources: &ExternalModelResources,
    requirements: Option<JsonRequirements>,
) -> i32 {
    if *format == Format::Human
        && let Some(requirements) = &requirements
    {
        println!(
            "[INFO] Loaded {} external requirement(s), {}{}.",
            requirements.assertion_count,
            requirements.sha256,
            if requirements.hash_pinned {
                " (hash pinned)"
            } else {
                ""
            }
        );
    }
    if report
        .ir
        .as_ref()
        .is_some_and(|circuit| circuit.assertions.is_empty())
    {
        report.diagnostics.push(diagnostic(
            "KES-T000",
            DiagnosticStage::Assertion,
            "test requires at least one assertion, but this circuit defines no assertions; use 'kess simulate' to run without verification or add an assert statement",
        ));
        let assertion_report = AssertionReport {
            schema_version: ASSERTION_SCHEMA_VERSION.to_string(),
            tolerance: TolerancePolicy::default(),
            assertions: Vec::new(),
            summary: AssertionSummary::default(),
        };
        emit_with_requirements(
            format,
            command,
            includes,
            "test_failed",
            report,
            spice_file,
            None,
            Some(assertion_report),
            requirements,
        );
        return 4;
    }

    if *format == Format::Human {
        println!("[INFO] Running tests and assertions...");
    }

    let simulation = match run_simulation(&report, spice, external_resources) {
        Ok(simulation) => simulation,
        Err(error) => {
            report.diagnostics.push(diagnostic(
                "KES-S001",
                DiagnosticStage::Simulation,
                error.to_string(),
            ));
            emit_with_requirements(
                format,
                command,
                includes,
                "simulation_error",
                report,
                spice_file,
                None,
                None,
                requirements,
            );
            return 3;
        }
    };

    if *format == Format::Human {
        print_simulation_result(&simulation, includes.contains(&Include::RawLog));
    }

    if !simulation.succeeded() {
        let details = if simulation.errors.is_empty() {
            "Ngspice returned an unsuccessful result.".to_string()
        } else {
            format!("Ngspice errors: {}", simulation.errors.join(" | "))
        };
        report
            .diagnostics
            .push(diagnostic("KES-S002", DiagnosticStage::Simulation, details));
        emit_with_requirements(
            format,
            command,
            includes,
            "simulation_error",
            report,
            spice_file,
            Some(simulation),
            None,
            requirements,
        );
        return 3;
    }

    let circuit = report
        .ir
        .as_ref()
        .expect("successful compile report must preserve typed IR");
    let assertion_report = kessetsu_core::sim_result::evaluate_assertions(circuit, &simulation);
    let all_passed = assertion_report.all_passed();
    if *format == Format::Human {
        for result in &assertion_report.assertions {
            print_assertion_result(result);
        }
        print_part_stress_report(&evaluate_part_stress(circuit, &simulation));
    }

    let status = if all_passed { "success" } else { "test_failed" };
    if *format == Format::Json {
        emit_with_requirements(
            format,
            command,
            includes,
            status,
            report,
            spice_file,
            Some(simulation),
            Some(assertion_report),
            requirements,
        );
    } else if all_passed {
        println!("\n[SUCCESS] All assertions passed.");
    } else {
        eprintln!("\n[ERROR] One or more assertions failed.");
    }

    if all_passed { 0 } else { 4 }
}

fn print_assertion_result(result: &AssertionResult) {
    let status = match result.status {
        AssertionStatus::Pass => "\x1b[32m[PASS]\x1b[0m",
        AssertionStatus::Fail => "\x1b[31m[FAIL]\x1b[0m",
        AssertionStatus::Error => "\x1b[31m[ERROR]\x1b[0m",
        AssertionStatus::Skipped => "\x1b[33m[SKIPPED]\x1b[0m",
    };
    let actual = result
        .actual
        .map(|actual| format_quantity(actual, result.unit))
        .unwrap_or_else(|| "not available".to_string());
    println!(
        "{} {} {}({}) {} {} (actual: {})",
        status,
        result.code,
        result.metric,
        result.signal,
        result.comparator,
        format_quantity(result.threshold, result.unit),
        actual
    );
    if let Some(message) = &result.message {
        println!("       {message}");
    }
}

fn compile_failure_exit_code(report: &CompileReport) -> i32 {
    if report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.stage,
            DiagnosticStage::Parse | DiagnosticStage::Io
        )
    }) {
        2
    } else {
        1
    }
}

pub(crate) fn diagnostic(
    code: &str,
    stage: DiagnosticStage,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: DiagnosticSeverity::Error,
        stage,
        message: message.into(),
        component: None,
        pin: None,
        field: None,
        line: None,
        column: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    status: &str,
    report: CompileReport,
    spice_file: Option<String>,
    simulation: Option<SimulationResult>,
    assertions: Option<AssertionReport>,
) {
    emit_with_requirements(
        format, command, includes, status, report, spice_file, simulation, assertions, None,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_with_requirements(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    status: &str,
    report: CompileReport,
    spice_file: Option<String>,
    simulation: Option<SimulationResult>,
    assertions: Option<AssertionReport>,
    requirements: Option<JsonRequirements>,
) {
    if *format == Format::Json {
        let output = build_json_output(
            command,
            includes,
            status,
            &report,
            spice_file,
            simulation.as_ref(),
            assertions,
            requirements,
        );
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("[ERROR] Could not serialize {CLI_SCHEMA_VERSION} JSON output: {error}")
            }
        }
    } else {
        emit_human_diagnostics(&report);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_json_output(
    command: &'static str,
    includes: &BTreeSet<Include>,
    status: &str,
    report: &CompileReport,
    spice_file: Option<String>,
    simulation: Option<&SimulationResult>,
    assertions: Option<AssertionReport>,
    requirements: Option<JsonRequirements>,
) -> JsonOutput {
    let mut diagnostics = report
        .diagnostics
        .iter()
        .map(JsonDiagnostic::from_compile)
        .collect::<Vec<_>>();
    if let Some(simulation) = simulation {
        diagnostics.extend(
            simulation
                .diagnostics
                .iter()
                .map(JsonDiagnostic::from_simulation),
        );
    }

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "error")
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "warning")
        .count();
    let info = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "info")
        .count();
    let measurements = simulation
        .map(|simulation| simulation.measurements.clone())
        .unwrap_or_default();
    let assertion_summary = assertions.as_ref().map(|report| report.summary);
    let part_stress = report
        .ir
        .as_ref()
        .zip(simulation)
        .map(|(circuit, simulation)| evaluate_part_stress(circuit, simulation))
        .filter(|stress| stress.summary.total > 0);
    let part_stress_summary = part_stress.as_ref().map(|report| report.summary);
    let summary = JsonSummary {
        errors,
        warnings,
        info,
        analyses: simulation.map_or(0, |simulation| simulation.analyses.len()),
        measurements: measurements.len(),
        assertions: assertion_summary,
        part_stress: part_stress_summary,
    };

    let model_lock_file = spice_file.as_ref().and_then(|path| {
        report.model_lock.as_ref().map(|_| {
            Path::new(path)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("kessetsu.lock")
                .to_string_lossy()
                .into_owned()
        })
    });
    let mut artifacts = spice_file
        .into_iter()
        .map(|path| plain_json_artifact("spice_netlist", path))
        .collect::<Vec<_>>();
    if let Some(path) = model_lock_file {
        artifacts.push(plain_json_artifact("model_lock", path));
    }
    if let Some(simulation) = simulation {
        artifacts.extend(
            simulation
                .artifacts
                .iter()
                .map(|artifact| plain_json_artifact(&artifact.kind, &artifact.path)),
        );
    }

    JsonOutput {
        schema_version: CLI_SCHEMA_VERSION,
        command,
        status: status.to_string(),
        domain_versions: DomainVersions {
            compile: COMPILE_SCHEMA_VERSION,
            capabilities: None,
            simulation: simulation.map(|_| SIMULATION_SCHEMA_VERSION),
            measurement: simulation.map(|_| MEASUREMENT_SCHEMA_VERSION),
            assertion: assertions.as_ref().map(|_| ASSERTION_SCHEMA_VERSION),
            part_stress: part_stress.as_ref().map(|_| PART_STRESS_SCHEMA_VERSION),
            requirements: requirements.as_ref().map(|_| REQUIREMENTS_SCHEMA_VERSION),
            export: None,
            tool: None,
            experiment: None,
            research_data: None,
            fit: None,
        },
        diagnostics,
        summary,
        measurements,
        assertions,
        part_stress,
        requirements,
        artifacts,
        calculation: None,
        study: None,
        data: None,
        fit: None,
        capabilities: None,
        debug: build_debug(includes, report, simulation),
    }
}

fn build_debug(
    includes: &BTreeSet<Include>,
    report: &CompileReport,
    simulation: Option<&SimulationResult>,
) -> Option<BTreeMap<String, Value>> {
    let mut debug = BTreeMap::new();
    for include in includes {
        let (name, value) = match include {
            Include::Ast => ("ast", to_json_value(&report.ast)),
            Include::Ir => ("ir", to_json_value(&report.ir)),
            Include::Graph => ("graph", to_json_value(&report.graph)),
            Include::Spice => ("spice_netlist", to_json_value(&report.spice_netlist)),
            Include::Datasets => (
                "datasets",
                to_json_value(&simulation.map(|simulation| &simulation.datasets)),
            ),
            Include::Simulation => ("simulation", to_json_value(&simulation)),
            Include::Models => (
                "models",
                to_json_value(&serde_json::json!({
                    "manifest": report.ir.as_ref().map(|ir| &ir.model_manifest),
                    "lock": report.model_lock,
                })),
            ),
            Include::RawLog => (
                "raw_log",
                to_json_value(&simulation.map(|simulation| &simulation.raw_log)),
            ),
            Include::EffectiveSource => {
                ("effective_source", to_json_value(&report.effective_source))
            }
        };
        debug.insert(name.to_string(), value);
    }
    (!debug.is_empty()).then_some(debug)
}

fn to_json_value(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

impl JsonDiagnostic {
    fn from_compile(diagnostic: &Diagnostic) -> Self {
        Self {
            code: diagnostic.code.clone(),
            severity: format!("{:?}", diagnostic.severity).to_ascii_lowercase(),
            stage: format!("{:?}", diagnostic.stage).to_ascii_lowercase(),
            kind: None,
            message: diagnostic.message.clone(),
            component: diagnostic.component.clone(),
            pin: diagnostic.pin.clone(),
            field: diagnostic.field.clone(),
            line: diagnostic.line,
            column: diagnostic.column,
        }
    }

    fn from_simulation(diagnostic: &kessetsu_core::simulation::SimulatorDiagnostic) -> Self {
        Self {
            code: diagnostic.code.clone(),
            severity: format!("{:?}", diagnostic.severity).to_ascii_lowercase(),
            stage: "simulation".to_string(),
            kind: Some(format!("{:?}", diagnostic.kind).to_ascii_lowercase()),
            message: diagnostic.message.clone(),
            component: None,
            pin: None,
            field: None,
            line: None,
            column: None,
        }
    }
}

pub(crate) fn emit_human_diagnostics(report: &CompileReport) {
    if let Some(ir) = &report.ir {
        for parameter in &ir.parameter_manifest.parameters {
            if parameter.instance_path.is_empty() && parameter.effective_override.is_some() {
                eprintln!(
                    "[PARAM] {} = {}",
                    parameter.name,
                    format_quantity(parameter.resolved.value, parameter.resolved.unit)
                );
            }
        }
    }
    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Warning => "WARNING",
            DiagnosticSeverity::Info => "INFO",
        };
        eprintln!(
            "[{} {:?}] {}: {}",
            severity, diagnostic.stage, diagnostic.code, diagnostic.message
        );
    }
}
