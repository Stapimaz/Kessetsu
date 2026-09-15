use clap::{Args, Parser, Subcommand, ValueEnum};
use kessetsu_core::compiler::{
    COMPILE_SCHEMA_VERSION, CompileOptions, CompileReport, Diagnostic, DiagnosticSeverity,
    DiagnosticStage, compile_source_with_resources,
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
use kessetsu_core::sim_result::{
    ASSERTION_SCHEMA_VERSION, AssertionReport, AssertionResult, AssertionStatus, AssertionSummary,
    TolerancePolicy, format_quantity,
};
use kessetsu_core::simulation::{
    CancellationToken, NativeSimulationContext, NgspiceRunner, SIMULATION_SCHEMA_VERSION,
    SimulationRequest, SimulationResult,
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
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and run Electrical Rules Check (ERC)
    Check { file: PathBuf },
    /// Parse, ERC, and generate a SPICE netlist
    Compile(OutputCommand),
    /// Parse, ERC, generate a netlist, and run Ngspice
    Simulate(OutputCommand),
    /// Parse, ERC, generate a netlist, simulate, and evaluate assertions
    Test(OutputCommand),
    /// Render the canonical schematic to SVG, PNG, or PDF
    Render(RenderCommand),
    /// Export a machine-readable or editable circuit artifact
    Export(ExportCommand),
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
    artifacts: Vec<JsonArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<BTreeMap<String, Value>>,
}

#[derive(Serialize)]
struct DomainVersions {
    compile: &'static str,
    simulation: Option<&'static str>,
    measurement: Option<&'static str>,
    assertion: Option<&'static str>,
    export: Option<&'static str>,
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
    let mut report = compile_source_with_resources(&source, options, &external_resources);

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

    let model_lock_path =
        if let (Some(spice_path), Some(model_lock)) = (&spice_path, report.model_lock.as_deref()) {
            let lock_path = spice_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("kessetsu.lock");
            if let Err(error) = fs::write(&lock_path, model_lock) {
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
            Some(lock_path)
        } else {
            None
        };

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
    )
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Check { .. } => "check",
        Commands::Compile(_) => "compile",
        Commands::Simulate(_) => "simulate",
        Commands::Test(_) => "test",
        Commands::Render(_) => "render",
        Commands::Export(_) => "export",
    }
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
        Commands::Check { file } => file,
        Commands::Render(command) => &command.file,
        Commands::Export(command) => &command.file,
        Commands::Compile(command) | Commands::Simulate(command) | Commands::Test(command) => {
            &command.file
        }
    }
}

fn output_command(command: &Commands) -> Option<&OutputCommand> {
    match command {
        Commands::Compile(command) | Commands::Simulate(command) | Commands::Test(command) => {
            Some(command)
        }
        Commands::Check { .. } | Commands::Render(_) | Commands::Export(_) => None,
    }
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
        let mut output = build_json_output(command, includes, "success", &report, None, None, None);
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

fn run_assertions(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    mut report: CompileReport,
    spice_file: Option<String>,
    spice: &str,
    external_resources: &ExternalModelResources,
) -> i32 {
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
        emit(
            format,
            command,
            includes,
            "test_failed",
            report,
            spice_file,
            None,
            Some(assertion_report),
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
            "Ngspice returned an unsuccessful result.".to_string()
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
        emit(
            format,
            command,
            includes,
            status,
            report,
            spice_file,
            Some(simulation),
            Some(assertion_report),
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
    if *format == Format::Json {
        let output = build_json_output(
            command,
            includes,
            status,
            &report,
            spice_file,
            simulation.as_ref(),
            assertions,
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
            export: None,
        },
        diagnostics,
        summary,
        measurements,
        assertions,
        artifacts,
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
