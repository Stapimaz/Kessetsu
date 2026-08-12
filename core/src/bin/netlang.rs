use clap::{Args, Parser, Subcommand, ValueEnum};
use netlang_core::compiler::{
    COMPILE_SCHEMA_VERSION, CompileOptions, CompileReport, Diagnostic, DiagnosticSeverity,
    DiagnosticStage, compile_source,
};
use netlang_core::measurement::MEASUREMENT_SCHEMA_VERSION;
use netlang_core::sim_result::{
    ASSERTION_SCHEMA_VERSION, AssertionReport, AssertionResult, AssertionStatus, AssertionSummary,
    format_quantity,
};
use netlang_core::simulation::{
    CancellationToken, NgspiceRunner, SIMULATION_SCHEMA_VERSION, SimulationRequest,
    SimulationResult, SimulationRunner,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

const CLI_SCHEMA_VERSION: &str = "netlang.cli.v1";

#[derive(Parser)]
#[command(name = "netlang", about = "NetLang Circuit Compiler and Simulator")]
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
    /// Reserved for the Phase 4 SVG schematic renderer
    Render { file: PathBuf },
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
            "NL-F002",
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

    if let Commands::Render { file } = &cli.command {
        let diagnostic = diagnostic(
            "NL-F001",
            DiagnosticStage::Cli,
            format!(
                "Render is not implemented yet; no output was produced for '{}'.",
                file.display()
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
                "NL-I001",
                DiagnosticStage::Io,
                if source_is_stdin {
                    format!("Could not read NetLang source from stdin: {error}")
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
    let options = CompileOptions {
        include_ast: includes.contains(&Include::Ast),
        generate_spice: needs_spice,
        generate_layout: false,
        generate_kicad: false,
    };
    let mut report = compile_source(&source, options);

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
                .join("netlang.lock");
            if let Err(error) = fs::write(&lock_path, model_lock) {
                report.diagnostics.push(diagnostic(
                    "NL-I005",
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
        return run_simulate(&cli.format, command, &includes, report, spice_file, &spice);
    }

    run_assertions(&cli.format, command, &includes, report, spice_file, &spice)
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Check { .. } => "check",
        Commands::Compile(_) => "compile",
        Commands::Simulate(_) => "simulate",
        Commands::Test(_) => "test",
        Commands::Render { .. } => "render",
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

fn command_path(command: &Commands) -> &Path {
    match command {
        Commands::Check { file } | Commands::Render { file } => file,
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
        Commands::Check { .. } | Commands::Render { .. } => None,
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
            "NL-I002",
            DiagnosticStage::Io,
            format!(
                "Refusing to overwrite source file '{}' with generated SPICE.",
                source_path.display()
            ),
        )));
    }

    if output_path.exists() && !force {
        return Err(Box::new(diagnostic(
            "NL-I003",
            DiagnosticStage::Io,
            format!(
                "Output file '{}' already exists; pass --force to overwrite it.",
                output_path.display()
            ),
        )));
    }

    fs::write(output_path, spice).map_err(|error| {
        Box::new(diagnostic(
            "NL-I004",
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

fn run_simulate(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    mut report: CompileReport,
    spice_file: Option<String>,
    spice: &str,
) -> i32 {
    if *format == Format::Human {
        println!("[INFO] Running ngspice simulation...");
    }

    let simulation = match run_simulation(&report, spice) {
        Ok(simulation) => simulation,
        Err(error) => {
            report.diagnostics.push(diagnostic(
                "NL-S001",
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
            .push(diagnostic("NL-S002", DiagnosticStage::Simulation, details));
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
) -> Result<SimulationResult, netlang_core::simulation::SimulationRunError> {
    let analyses = report
        .ir
        .as_ref()
        .expect("successful compile report must preserve typed IR")
        .analyses
        .clone();
    let request = SimulationRequest::new(spice, analyses);
    NgspiceRunner::discover().run(&request, &CancellationToken::new())
}

fn run_assertions(
    format: &Format,
    command: &'static str,
    includes: &BTreeSet<Include>,
    mut report: CompileReport,
    spice_file: Option<String>,
    spice: &str,
) -> i32 {
    if *format == Format::Human {
        println!("[INFO] Running tests and assertions...");
    }

    let simulation = match run_simulation(&report, spice) {
        Ok(simulation) => simulation,
        Err(error) => {
            report.diagnostics.push(diagnostic(
                "NL-S001",
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
            .push(diagnostic("NL-S002", DiagnosticStage::Simulation, details));
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
    let assertion_report = netlang_core::sim_result::evaluate_assertions(circuit, &simulation);
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
                .join("netlang.lock")
                .to_string_lossy()
                .into_owned()
        })
    });
    let mut artifacts = spice_file
        .into_iter()
        .map(|path| JsonArtifact {
            kind: "spice_netlist".to_string(),
            path,
        })
        .collect::<Vec<_>>();
    if let Some(path) = model_lock_file {
        artifacts.push(JsonArtifact {
            kind: "model_lock".to_string(),
            path,
        });
    }
    if let Some(simulation) = simulation {
        artifacts.extend(simulation.artifacts.iter().map(|artifact| JsonArtifact {
            kind: artifact.kind.clone(),
            path: artifact.path.clone(),
        }));
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

    fn from_simulation(diagnostic: &netlang_core::simulation::SimulatorDiagnostic) -> Self {
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
