use clap::{Args, Parser, Subcommand, ValueEnum};
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
use kessetsu_core::tools::{
    PreferredValues, TOOL_SCHEMA_VERSION, ToolRequest, ToolResult, calculate_tool,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

const CLI_SCHEMA_VERSION: &str = "kessetsu.cli.v1";

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
}

#[derive(Subcommand)]
enum Commands {
    /// Import research CSV and compare unit-mapped local data
    Data(DataCommand),
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
}

#[derive(Args)]
struct DataCommand {
    #[command(subcommand)]
    action: DataAction,
}

#[derive(Subcommand)]
enum DataAction {
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
struct StudyCommand {
    #[command(subcommand)]
    action: StudyAction,
}

#[derive(Subcommand)]
enum StudyAction {
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
}

#[derive(Clone, Copy, ValueEnum)]
enum StudyTarget {
    Json,
    Csv,
    DataCsv,
    Svg,
    Html,
}

#[derive(Args)]
struct ToolCommand {
    #[command(subcommand)]
    tool: CircuitTool,
    /// Write generated .kess source (no file is written by default)
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,
    /// Allow overwriting an existing output file
    #[arg(long, global = true)]
    force: bool,
    /// Nominal component value series (not a tolerance specification)
    #[arg(long, value_enum, default_value_t = ValueSeries::E24, global = true)]
    values: ValueSeries,
}

#[derive(Subcommand)]
enum CircuitTool {
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
}

#[derive(Clone, Copy, ValueEnum)]
enum ValueSeries {
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
enum Format {
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
}

impl From<ExportTarget> for ExportFormat {
    fn from(value: ExportTarget) -> Self {
        match value {
            ExportTarget::SchematicJson => Self::SchematicJson,
            ExportTarget::Spice => Self::Spice,
            ExportTarget::Kicad => Self::Kicad,
            ExportTarget::Ltspice => Self::Ltspice,
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
enum Include {
    Ast,
    Ir,
    Graph,
    Spice,
    Datasets,
    Models,
    RawLog,
    EffectiveSource,
}

#[derive(Serialize)]
struct JsonOutput {
    schema_version: &'static str,
    command: &'static str,
    status: String,
    domain_versions: DomainVersions,
    diagnostics: Vec<JsonDiagnostic>,
    summary: JsonSummary,
    measurements: BTreeMap<String, f64>,
    assertions: Option<AssertionReport>,
    requirements: Option<JsonRequirements>,
    artifacts: Vec<JsonArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    calculation: Option<ToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    study: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<BTreeMap<String, Value>>,
}

#[derive(Serialize)]
struct DomainVersions {
    compile: &'static str,
    simulation: Option<&'static str>,
    measurement: Option<&'static str>,
    assertion: Option<&'static str>,
    requirements: Option<&'static str>,
    export: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experiment: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research_data: Option<&'static str>,
}

#[derive(Clone, Serialize)]
struct JsonRequirements {
    schema_version: String,
    sha256: String,
    assertion_count: usize,
    hash_pinned: bool,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    code: String,
    severity: String,
    stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
}

#[derive(Serialize)]
struct JsonSummary {
    errors: usize,
    warnings: usize,
    info: usize,
    analyses: usize,
    measurements: usize,
    assertions: Option<AssertionSummary>,
}

#[derive(Clone, Serialize)]
struct JsonArtifact {
    kind: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exporter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exporter_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    byte_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connectivity_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capability: Option<ExportCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    losses: Vec<String>,
}

fn emit_study_error(
    format: &Format,
    includes: &BTreeSet<Include>,
    message: impl Into<String>,
) -> i32 {
    emit(
        format,
        "study",
        includes,
        "error",
        CompileReport::failure(diagnostic("KES-X001", DiagnosticStage::Cli, message)),
        None,
        None,
        None,
    );
    2
}

fn emit_study(
    format: &Format,
    includes: &BTreeSet<Include>,
    status: &str,
    body: Value,
    artifact: Option<&Path>,
) {
    let report = kessetsu_core::compiler::compile_source("", CompileOptions::default());
    let mut output = build_json_output("study", includes, status, &report, None, None, None, None);
    output.domain_versions.experiment = Some(kessetsu_core::experiment::EXPERIMENT_SCHEMA);
    output.domain_versions.measurement = Some(MEASUREMENT_SCHEMA_VERSION);
    output.study = Some(body.clone());
    if let Some(path) = artifact {
        output.artifacts.push(plain_json_artifact(
            "study",
            path.to_string_lossy().into_owned(),
        ));
    }
    if *format == Format::Json {
        println!("{}", serde_json::to_string(&output).unwrap());
    } else {
        println!(
            "Study {status}: {}",
            serde_json::to_string_pretty(&body).unwrap()
        );
        if let Some(path) = artifact {
            println!("Saved: {}", path.display());
        }
    }
}

fn study_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > limit {
        return Err("Study file exceeds its size limit".into());
    }
    fs::read(path).map_err(|e| e.to_string())
}

fn study_preflight(input: &Path, output: &Path, force: bool) -> Result<(), String> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let resolved = fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(output.file_name().ok_or("Output must name a file")?);
    let input = fs::canonicalize(input).map_err(|e| e.to_string())?;
    if resolved == input
        || (output.exists() && fs::canonicalize(output).map_err(|e| e.to_string())? == input)
    {
        return Err("Study output cannot overwrite its input".into());
    }
    if output.exists() && !force {
        return Err("Output already exists; use --force or --resume as appropriate".into());
    }
    Ok(())
}

fn study_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let pending = path.with_file_name(format!(".kess-study-{}.tmp", process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&pending, path)
        .map_err(|e| format!("Could not save checkpoint; temporary data is preserved: {e}"))
}

fn study_resources(
    file: &Path,
    spec: &kessetsu_core::experiment::ExperimentSpec,
) -> Result<ExternalModelResources, String> {
    let mut resources =
        load_external_model_resources(file, false, &spec.source).map_err(|e| e.message)?;
    for revision in &spec.revisions {
        if let Some(source) = &revision.source {
            resources
                .extend(load_external_model_resources(file, false, source).map_err(|e| e.message)?);
        }
    }
    Ok(resources)
}

fn run_study(command: &StudyCommand, format: &Format, includes: &BTreeSet<Include>) -> i32 {
    use kessetsu_core::experiment::*;
    let execute = || -> Result<i32, String> {
        match &command.action {
            StudyAction::Create {
                file,
                output,
                sweep,
                force,
            } => {
                study_preflight(file, output, *force)?;
                let source = String::from_utf8(study_read(file, MAX_SPEC_BYTES as u64)?)
                    .map_err(|e| e.to_string())?;
                let mut axes = Vec::new();
                for axis in sweep {
                    let (parameter, values) = axis
                        .split_once('=')
                        .ok_or("--sweep requires NAME=value,value")?;
                    axes.push(Axis {
                        parameter: parameter.into(),
                        values: Values::List {
                            values: values.split(',').map(str::to_owned).collect(),
                        },
                    });
                }
                let spec = ExperimentSpec {
                    schema_version: EXPERIMENT_SCHEMA.into(),
                    name: file
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    source,
                    requirements: None,
                    axes,
                    revisions: Vec::new(),
                    tolerances: None,
                    temperatures_c: vec![27.0],
                    timeout_ms: 30_000,
                    measurements: Vec::new(),
                    objective: None,
                };
                let resources = study_resources(file, &spec)?;
                let plan = plan_experiment(spec, &resources)?;
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&plan.spec).map_err(|e| e.to_string())?,
                )?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::json!({"identity":plan.identity,"cases":plan.cases.len()}),
                    Some(output),
                );
                Ok(0)
            }
            StudyAction::Plan { file } => {
                let spec = decode_spec(&study_read(file, MAX_SPEC_BYTES as u64)?)?;
                let resources = study_resources(file, &spec)?;
                let plan = plan_experiment(spec, &resources)?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::json!({"identity":plan.identity,"requirements_sha256":plan.requirements_sha256,"cases":plan.cases}),
                    None,
                );
                Ok(0)
            }
            StudyAction::Run {
                file,
                output,
                resume,
                force,
            } => {
                if *resume && *force {
                    return Err("Choose --resume or --force, not both".into());
                }
                study_preflight(file, output, *resume || *force)?;
                let spec = decode_spec(&study_read(file, MAX_SPEC_BYTES as u64)?)?;
                let resources = study_resources(file, &spec)?;
                let output_absolute = fs::canonicalize(
                    output
                        .parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new(".")),
                )
                .map_err(|e| e.to_string())?
                .join(output.file_name().unwrap());
                let model_root = fs::canonicalize(
                    file.parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new(".")),
                )
                .map_err(|e| e.to_string())?;
                for reference in resources.keys() {
                    let model =
                        fs::canonicalize(model_root.join(reference)).map_err(|e| e.to_string())?;
                    if model == output_absolute
                        || (output.exists()
                            && fs::canonicalize(output).map_err(|e| e.to_string())? == model)
                    {
                        return Err("Results cannot overwrite a bound model file".into());
                    }
                }
                let plan = plan_experiment(spec, &resources)?;
                let runner = NgspiceRunner::discover();
                let simulator = runner.info().map_err(|e| e.to_string())?;
                let solver_fingerprint = hash_bytes(
                    format!(
                        "{}:{}",
                        hash_bytes(&fs::read(&simulator.executable).map_err(|e| e.to_string())?),
                        hash_bytes(
                            &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
                                .map_err(|e| e.to_string())?
                        )
                    )
                    .as_bytes(),
                );
                let mut results = if *resume {
                    let previous: ExperimentResults =
                        serde_json::from_slice(&study_read(output, MAX_RESULT_BYTES as u64)?)
                            .map_err(|e| e.to_string())?;
                    validate_results(&previous, &plan, &simulator, &solver_fingerprint)?;
                    previous
                } else {
                    new_results(plan, simulator, solver_fingerprint)
                };
                let cancellation = CancellationToken::new();
                let signal = cancellation.clone();
                ctrlc::set_handler(move || signal.cancel()).map_err(|e| e.to_string())?;
                let checkpoint = |results: &ExperimentResults| -> Result<(), String> {
                    let bytes = serde_json::to_vec(results).map_err(|e| e.to_string())?;
                    if bytes.len() > MAX_RESULT_BYTES {
                        return Err("Results exceed 64 MiB; previous checkpoint is preserved. Narrow the study.".into());
                    }
                    study_write(output, &bytes)
                };
                summarize(&mut results);
                checkpoint(&results)?;
                for index in 0..results.plan.cases.len() {
                    if cancellation.is_cancelled() {
                        break;
                    }
                    if results.cases[index].status.reusable() {
                        continue;
                    }
                    let case = &results.plan.cases[index];
                    let row = match compile_case(&results.plan, case, &resources) {
                        Err(error) => failed_case(&case.id, CaseStatus::Error, error),
                        Ok(report) => {
                            let circuit = report.ir.as_ref().unwrap();
                            let mut context = NativeSimulationContext::default();
                            for model in &circuit.model_manifest.models {
                                if let Some(external) = &model.external {
                                    if external.simulator
                                        == kessetsu_core::ir::SimulatorCompatibility::NgspicePs
                                    {
                                        context.compatibility = external.simulator;
                                    }
                                    if let Some(bytes) = resources.get(&external.resource) {
                                        context
                                            .resources
                                            .insert(external.resource.clone(), bytes.clone());
                                    }
                                }
                            }
                            let spice = temperature_netlist(
                                report.spice_netlist.as_deref().ok_or("Missing SPICE")?,
                                case.temperature_c,
                            )?;
                            let mut request =
                                SimulationRequest::new(spice, circuit.analyses.clone());
                            request.timeout_ms = results.plan.spec.timeout_ms;
                            match runner.run_with_context(&request, &context, &cancellation) {
                                Ok(simulation) => {
                                    evaluate_case(&results.plan, case, &report, simulation)
                                }
                                Err(error) => {
                                    failed_case(&case.id, CaseStatus::Error, error.to_string())
                                }
                            }
                        }
                    };
                    eprintln!(
                        "Study {}/{}: {} {:?}",
                        index + 1,
                        results.plan.cases.len(),
                        case.id,
                        row.status
                    );
                    results.cases[index] = row;
                    summarize(&mut results);
                    checkpoint(&results)?;
                }
                let exit = if cancellation.is_cancelled() {
                    130
                } else if results.summary.errors > 0 {
                    3
                } else if results.summary.failed > 0 {
                    1
                } else {
                    0
                };
                let body = if includes.contains(&Include::Datasets) {
                    serde_json::to_value(&results).map_err(|e| e.to_string())?
                } else {
                    serde_json::json!({"schema_version":RESULTS_SCHEMA,"identity":results.identity,"summary":results.summary,"cases":results.cases.iter().map(|r| serde_json::json!({"case_id":r.case_id,"status":r.status,"measurements":r.measurements,"assertions":r.assertions,"errors":r.errors})).collect::<Vec<_>>()})
                };
                emit_study(
                    format,
                    includes,
                    if exit == 0 {
                        "success"
                    } else if exit == 130 {
                        "cancelled"
                    } else {
                        "fail"
                    },
                    body,
                    Some(output),
                );
                Ok(exit)
            }
            StudyAction::Export {
                file,
                output,
                target,
                signal,
                analysis,
                cases,
                force,
            } => {
                study_preflight(file, output, *force)?;
                let mut results: ExperimentResults =
                    serde_json::from_slice(&study_read(file, MAX_RESULT_BYTES as u64)?)
                        .map_err(|e| e.to_string())?;
                validate_results(
                    &results,
                    &results.plan,
                    &results.simulator,
                    &results.solver_fingerprint,
                )?;
                summarize(&mut results);
                let content = match target {
                    StudyTarget::Json => {
                        serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?
                    }
                    StudyTarget::Csv => results_csv(&results),
                    StudyTarget::DataCsv => datasets_csv(&results),
                    StudyTarget::Svg => plot_svg(&results, *analysis, signal, cases)?,
                    StudyTarget::Html => report_html(&results),
                };
                study_write(output, content.as_bytes())?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::json!({"identity":results.identity,"summary":results.summary}),
                    Some(output),
                );
                Ok(0)
            }
        }
    };
    match execute() {
        Ok(exit) => exit,
        Err(error) => emit_study_error(format, includes, error),
    }
}

fn run_data(command: &DataCommand, format: &Format, includes: &BTreeSet<Include>) -> i32 {
    use kessetsu_core::research_data::*;
    let read_text = |path: &Path, limit: usize| -> Result<String, String> {
        String::from_utf8(study_read(path, limit as u64)?).map_err(|_| "Input must be UTF-8".into())
    };
    let execute = || -> Result<(Value, Option<&Path>), String> {
        match &command.action {
            DataAction::Preview {
                file,
                delimiter,
                decimal,
                no_header,
                skip_records,
            } => {
                let dialect = CsvDialect {
                    delimiter: match delimiter.as_str() {
                        "semicolon" => Delimiter::Semicolon,
                        "tab" => Delimiter::Tab,
                        _ => Delimiter::Comma,
                    },
                    decimal: if decimal == "comma" {
                        Decimal::Comma
                    } else {
                        Decimal::Dot
                    },
                    header: !no_header,
                    preamble_records: *skip_records,
                };
                Ok((
                    serde_json::to_value(preview_csv(&read_text(file, MAX_CSV_BYTES)?, &dialect)?)
                        .map_err(|e| e.to_string())?,
                    None,
                ))
            }
            DataAction::Import {
                file,
                mapping,
                output,
                force,
            } => {
                for input in [file, mapping] {
                    study_preflight(input, output, *force)?;
                }
                let spec: ImportSpec = serde_json::from_str(&read_text(mapping, 64 * 1024)?)
                    .map_err(|e| e.to_string())?;
                let result = import_csv(&read_text(file, MAX_CSV_BYTES)?, spec)?;
                let mut body = serde_json::json!({ "schema_version": result.schema_version, "identity": result.identity, "raw_sha256": result.raw_sha256, "rows": result.source_records.len(), "skipped": result.skipped.len(), "origin": result.spec.metadata.origin, "axis": {"name": result.axis.name, "unit": result.axis.unit}, "signals": result.signals.iter().map(|c| serde_json::json!({"name":c.name,"unit":c.unit})).collect::<Vec<_>>() });
                if includes.contains(&Include::Datasets) {
                    body["axis"]["values"] =
                        serde_json::to_value(&result.axis.values).map_err(|e| e.to_string())?;
                    body["signals"] =
                        serde_json::to_value(&result.signals).map_err(|e| e.to_string())?;
                }
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
                )?;
                Ok((body, Some(output.as_path())))
            }
            DataAction::Compare {
                file,
                reference,
                mapping,
                output,
                force,
            } => {
                for input in [file, reference, mapping] {
                    study_preflight(input, output, *force)?;
                }
                let data: ResearchData = serde_json::from_str(&read_text(file, 64 * 1024 * 1024)?)
                    .map_err(|e| e.to_string())?;
                let reference: ResearchData =
                    serde_json::from_str(&read_text(reference, 64 * 1024 * 1024)?)
                        .map_err(|e| e.to_string())?;
                let spec: ComparisonSpec = serde_json::from_str(&read_text(mapping, 64 * 1024)?)
                    .map_err(|e| e.to_string())?;
                let result = compare_data(&data, &reference, spec)?;
                let body = if includes.contains(&Include::Datasets) {
                    serde_json::to_value(&result).map_err(|e| e.to_string())?
                } else {
                    serde_json::json!({ "schema_version": result.schema_version, "identity": result.identity, "data_identity": result.data_identity, "reference_identity": result.reference_identity, "axis_unit": result.axis_unit, "signals": result.signals.iter().map(|c| serde_json::json!({"mapping":c.mapping,"unit":c.unit,"metrics":c.metrics})).collect::<Vec<_>>() })
                };
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
                )?;
                Ok((body, Some(output.as_path())))
            }
        }
    };
    match execute() {
        Ok((body, artifact)) => {
            let report = kessetsu_core::compiler::compile_source("", CompileOptions::default());
            let mut output = build_json_output(
                "data",
                includes,
                "completed",
                &report,
                None,
                None,
                None,
                None,
            );
            output.domain_versions.research_data = Some(DATA_SCHEMA);
            output.data = Some(body.clone());
            if let Some(path) = artifact {
                output.artifacts.push(plain_json_artifact(
                    "research_data",
                    path.to_string_lossy().into_owned(),
                ));
            }
            if *format == Format::Json {
                println!("{}", serde_json::to_string(&output).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&body).unwrap());
                if let Some(path) = artifact {
                    println!("Saved: {}", path.display());
                }
            }
            0
        }
        Err(error) => {
            emit(
                format,
                "data",
                includes,
                "error",
                CompileReport::failure(diagnostic("KES-R001", DiagnosticStage::Cli, error)),
                None,
                None,
                None,
            );
            2
        }
    }
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
        return run_data(data, &cli.format, &includes);
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
        return run_tool(tool, &cli.format, &includes);
    }
    if let Commands::Study(study) = &cli.command {
        if !cli.parameters.is_empty() {
            return emit_study_error(
                &cli.format,
                &includes,
                "Use study axes/revision inputs, not --param",
            );
        }
        return run_study(study, &cli.format, &includes);
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

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Data(_) => "data",
        Commands::Study(_) => "study",
        Commands::Tool(_) => "tool",
        Commands::Check { .. } => "check",
        Commands::Compile(_) => "compile",
        Commands::Simulate(_) => "simulate",
        Commands::Test(_) => "test",
        Commands::Render(_) => "render",
        Commands::Export(_) => "export",
    }
}

fn run_tool(command: &ToolCommand, format: &Format, includes: &BTreeSet<Include>) -> i32 {
    let preferred_values = command.values.into();
    let request = match &command.tool {
        CircuitTool::Divider {
            vin,
            target,
            lower,
            load,
        } => ToolRequest::Divider {
            input_voltage: vin.clone(),
            target_voltage: target.clone(),
            lower_resistance: lower.clone(),
            load_resistance: load.clone(),
            preferred_values,
        },
        CircuitTool::RcLowpass { cutoff, resistance } => ToolRequest::RcLowpass {
            cutoff: cutoff.clone(),
            resistance: resistance.clone(),
            preferred_values,
        },
    };
    let calculation = match calculate_tool(request) {
        Ok(result) => result,
        Err(error) => {
            let mut diagnostic = diagnostic("KES-F003", DiagnosticStage::Cli, error.message);
            diagnostic.field = Some(error.field);
            emit(
                format,
                "tool",
                includes,
                "error",
                CompileReport::failure(diagnostic),
                None,
                None,
                None,
            );
            return 2;
        }
    };
    let mut report = kessetsu_core::compile_source(
        &calculation.source,
        CompileOptions {
            include_ast: includes.contains(&Include::Ast),
            ..CompileOptions::default()
        },
    );
    let mut artifacts = Vec::new();
    if !report.has_errors()
        && let Some(path) = &command.output
    {
        if path.extension().and_then(|value| value.to_str()) != Some("kess") {
            report.diagnostics.push(diagnostic("KES-F003", DiagnosticStage::Cli,
                "Tool --output must have the .kess extension; use render/export on that source for other formats."));
        } else if path.exists() && !command.force {
            report.diagnostics.push(diagnostic(
                "KES-I003",
                DiagnosticStage::Io,
                format!(
                    "Output file '{}' already exists; pass --force to overwrite it.",
                    path.display()
                ),
            ));
        } else if let Err(error) = fs::write(path, &calculation.source) {
            report.diagnostics.push(diagnostic(
                "KES-I004",
                DiagnosticStage::Io,
                format!(
                    "Could not write generated source '{}': {error}",
                    path.display()
                ),
            ));
        } else {
            artifacts.push(plain_json_artifact(
                "circuit_source",
                path.to_string_lossy(),
            ));
        }
    }
    let failed = report.has_errors();
    if *format == Format::Json {
        let mut output = build_json_output(
            "tool",
            includes,
            if failed { "error" } else { "success" },
            &report,
            None,
            None,
            None,
            None,
        );
        output.domain_versions.tool = Some(TOOL_SCHEMA_VERSION);
        output.artifacts = artifacts;
        output.calculation = Some(calculation);
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("finite tool result must serialize")
        );
    } else {
        emit_human_diagnostics(&report);
        if !failed {
            println!(
                "{} — nominal analytical calculation ({:?})",
                calculation.name, calculation.preferred_values
            );
            for (name, value) in &calculation.components {
                println!("{name}: {}", format_quantity(value.value, value.unit));
            }
            for (name, value) in &calculation.results {
                println!("{name}: {}", format_quantity(value.value, value.unit));
            }
            for assumption in &calculation.assumptions {
                println!("Note: {assumption}");
            }
            if let Some(path) = &command.output {
                println!(
                    "Source written to {}. Next: kess simulate {}",
                    path.display(),
                    path.display()
                );
            } else {
                println!(
                    "\nEditable source (use --output circuit.kess to save):\n{}",
                    calculation.source
                );
            }
        }
    }
    if failed { 2 } else { 0 }
}

fn read_source(source_path: &Path) -> io::Result<String> {
    if source_path == Path::new("-") {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        Ok(source)
    } else {
        fs::read_to_string(source_path)
    }
}

fn load_external_model_resources(
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
        Commands::Data(_) | Commands::Study(_) | Commands::Tool(_) => {
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
        | Commands::Study(_)
        | Commands::Check { .. }
        | Commands::Render(_)
        | Commands::Export(_)
        | Commands::Tool(_) => None,
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

fn plain_json_artifact(kind: impl Into<String>, path: impl Into<String>) -> JsonArtifact {
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

fn diagnostic(code: &str, stage: DiagnosticStage, message: impl Into<String>) -> Diagnostic {
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
fn emit(
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
fn build_json_output(
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
    let summary = JsonSummary {
        errors,
        warnings,
        info,
        analyses: simulation.map_or(0, |simulation| simulation.analyses.len()),
        measurements: measurements.len(),
        assertions: assertion_summary,
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
            simulation: simulation.map(|_| SIMULATION_SCHEMA_VERSION),
            measurement: simulation.map(|_| MEASUREMENT_SCHEMA_VERSION),
            assertion: assertions.as_ref().map(|_| ASSERTION_SCHEMA_VERSION),
            requirements: requirements.as_ref().map(|_| REQUIREMENTS_SCHEMA_VERSION),
            export: None,
            tool: None,
            experiment: None,
            research_data: None,
        },
        diagnostics,
        summary,
        measurements,
        assertions,
        requirements,
        artifacts,
        calculation: None,
        study: None,
        data: None,
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

fn emit_human_diagnostics(report: &CompileReport) {
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
