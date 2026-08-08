use clap::{Args, Parser, Subcommand, ValueEnum};
use netlang_core::compiler::{
    COMPILE_SCHEMA_VERSION, CompileOptions, CompileReport, Diagnostic, DiagnosticSeverity,
    DiagnosticStage, compile_source,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Output};

#[derive(Parser)]
#[command(name = "netlang", about = "NetLang Circuit Compiler and Simulator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format (human or json)
    #[arg(long, value_enum, default_value_t = Format::Human, global = true)]
    format: Format,
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
    /// Reserved for the Phase 3 SVG schematic renderer
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

#[derive(Serialize)]
struct JsonTestResult {
    metric: String,
    signal: String,
    pass: bool,
    actual: Option<f64>,
    threshold: f64,
}

#[derive(Serialize)]
struct JsonOutput {
    status: String,
    #[serde(flatten)]
    report: CompileReport,
    spice_file: Option<String>,
    tests: Option<Vec<JsonTestResult>>,
}

fn main() {
    let cli = Cli::parse();
    process::exit(run(cli));
}

fn run(cli: Cli) -> i32 {
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
            "error",
            CompileReport::failure(diagnostic),
            None,
            None,
        );
        return 2;
    }

    let source_path = command_path(&cli.command);
    let source = match fs::read_to_string(source_path) {
        Ok(source) => source,
        Err(error) => {
            let diagnostic = diagnostic(
                "NL-I001",
                DiagnosticStage::Io,
                format!("Could not read file '{}': {error}", source_path.display()),
            );
            emit(
                &cli.format,
                "error",
                CompileReport::failure(diagnostic),
                None,
                None,
            );
            return 2;
        }
    };

    let needs_spice = !matches!(cli.command, Commands::Check { .. });
    let options = CompileOptions {
        include_ast: cli.format == Format::Json,
        generate_spice: needs_spice,
        generate_layout: false,
        generate_kicad: false,
    };
    let mut report = compile_source(&source, options);

    if report.has_errors() {
        let exit_code = compile_failure_exit_code(&report);
        emit(&cli.format, "error", report, None, None);
        return exit_code;
    }

    if matches!(cli.command, Commands::Check { .. }) {
        if cli.format == Format::Human {
            emit_human_diagnostics(&report);
            println!("[SUCCESS] Circuit parsed and ERC checks passed.");
        } else {
            emit(&cli.format, "success", report, None, None);
        }
        return 0;
    }

    let output_command = output_command(&cli.command)
        .expect("every non-check, non-render command must define an output policy");
    let spice_path = output_command
        .output
        .clone()
        .unwrap_or_else(|| source_path.with_extension("spice"));
    let spice = report
        .spice_netlist
        .as_deref()
        .expect("successful SPICE-enabled compile must contain a netlist")
        .to_string();

    if let Err(diagnostic) = write_spice(source_path, &spice_path, &spice, output_command.force) {
        report.diagnostics.push(*diagnostic);
        report.spice_netlist = None;
        emit(&cli.format, "error", report, None, None);
        return 2;
    }

    let spice_file = Some(spice_path.to_string_lossy().into_owned());
    if matches!(cli.command, Commands::Compile(_)) {
        if cli.format == Format::Human {
            emit_human_diagnostics(&report);
            println!(
                "[SUCCESS] SPICE netlist generated: {}",
                spice_path.display()
            );
        } else {
            emit(&cli.format, "success", report, spice_file, None);
        }
        return 0;
    }

    if matches!(cli.command, Commands::Simulate(_)) {
        return run_simulate(&cli.format, report, spice_file, &spice_path);
    }

    run_assertions(&cli.format, report, spice_file, &spice)
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
    mut report: CompileReport,
    spice_file: Option<String>,
    spice_path: &Path,
) -> i32 {
    if *format == Format::Human {
        println!("[INFO] Running ngspice simulation...");
    }

    let executable = netlang_core::sim_result::get_ngspice_path();
    let output = match process::Command::new(&executable)
        .arg("-b")
        .arg(spice_path)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            report.diagnostics.push(diagnostic(
                "NL-S001",
                DiagnosticStage::Simulation,
                format!(
                    "Failed to execute ngspice at '{}': {error}",
                    executable.display()
                ),
            ));
            emit(format, "simulation_error", report, spice_file, None);
            return 3;
        }
    };

    if *format == Format::Human {
        print_simulator_logs(&output);
    }

    if simulator_failed(&output) {
        report.diagnostics.push(diagnostic(
            "NL-S002",
            DiagnosticStage::Simulation,
            format!(
                "Ngspice failed with process status {}.",
                output
                    .status
                    .code()
                    .map_or_else(|| "terminated".to_string(), |code| code.to_string())
            ),
        ));
        emit(format, "simulation_error", report, spice_file, None);
        return 3;
    }

    if *format == Format::Json {
        emit(format, "success", report, spice_file, None);
    }
    0
}

fn simulator_failed(output: &Output) -> bool {
    !output.status.success()
        || contains_simulator_error(&String::from_utf8_lossy(&output.stdout))
        || contains_simulator_error(&String::from_utf8_lossy(&output.stderr))
}

fn contains_simulator_error(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    lowercase.contains("error") || lowercase.contains("fatal") || lowercase.contains("aborted")
}

fn print_simulator_logs(output: &Output) {
    if !output.stdout.is_empty() {
        println!(
            "\n--- NGSPICE OUTPUT ---\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    if !output.stderr.is_empty() {
        eprintln!(
            "\n--- NGSPICE ERRORS ---\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn run_assertions(
    format: &Format,
    mut report: CompileReport,
    spice_file: Option<String>,
    spice: &str,
) -> i32 {
    if *format == Format::Human {
        println!("[INFO] Running tests and assertions...");
    }

    let simulation = match netlang_core::sim_result::run_simulation(spice) {
        Ok(simulation) => simulation,
        Err(error) => {
            report
                .diagnostics
                .push(diagnostic("NL-S001", DiagnosticStage::Simulation, error));
            emit(format, "simulation_error", report, spice_file, None);
            return 3;
        }
    };

    if !simulation.success {
        let details = if simulation.errors.is_empty() {
            "Ngspice returned an unsuccessful result.".to_string()
        } else {
            format!("Ngspice errors: {}", simulation.errors.join(" | "))
        };
        report
            .diagnostics
            .push(diagnostic("NL-S002", DiagnosticStage::Simulation, details));
        emit(format, "simulation_error", report, spice_file, None);
        return 3;
    }

    let circuit = report
        .ir
        .as_ref()
        .expect("successful compile report must preserve typed IR");
    let results = netlang_core::sim_result::evaluate_assertions(circuit, &simulation);
    let mut all_passed = true;
    let mut json_results = Vec::new();

    for result in results {
        all_passed &= result.pass;
        json_results.push(JsonTestResult {
            metric: result.assertion.metric.clone(),
            signal: result.assertion.signal.clone(),
            pass: result.pass,
            actual: result.actual.is_finite().then_some(result.actual),
            threshold: result.assertion.threshold.value,
        });

        if *format == Format::Human {
            print_assertion_result(&result);
        }
    }

    let status = if all_passed { "success" } else { "test_failed" };
    if *format == Format::Json {
        emit(format, status, report, spice_file, Some(json_results));
    } else if all_passed {
        println!("\n[SUCCESS] All assertions passed.");
    } else {
        eprintln!("\n[ERROR] One or more assertions failed.");
    }

    if all_passed { 0 } else { 4 }
}

fn print_assertion_result(result: &netlang_core::sim_result::TestResult) {
    let status = if result.pass {
        "\x1b[32m[PASS]\x1b[0m"
    } else {
        "\x1b[31m[FAIL]\x1b[0m"
    };
    let comparator = match result.assertion.cmp {
        netlang_core::ast::Cmp::Lt => "<",
        netlang_core::ast::Cmp::Gt => ">",
        netlang_core::ast::Cmp::Le => "<=",
        netlang_core::ast::Cmp::Ge => ">=",
        netlang_core::ast::Cmp::Eq => "==",
    };
    let actual = if result.actual.is_finite() {
        format!("{:.6}", result.actual)
    } else {
        "Not Found".to_string()
    };
    println!(
        "{} {}({}) {} {} (actual: {})",
        status,
        result.assertion.metric,
        result.assertion.signal,
        comparator,
        result.assertion.threshold.value,
        actual
    );
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

fn emit(
    format: &Format,
    status: &str,
    report: CompileReport,
    spice_file: Option<String>,
    tests: Option<Vec<JsonTestResult>>,
) {
    if *format == Format::Json {
        let output = JsonOutput {
            status: status.to_string(),
            report,
            spice_file,
            tests,
        };
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(error) => eprintln!(
                "[ERROR] Could not serialize {COMPILE_SCHEMA_VERSION} JSON output: {error}"
            ),
        }
    } else {
        emit_human_diagnostics(&report);
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

#[cfg(test)]
mod tests {
    use super::contains_simulator_error;

    #[test]
    fn simulator_error_classifier_is_case_insensitive_and_fail_closed() {
        assert!(contains_simulator_error("Fatal error: singular matrix"));
        assert!(contains_simulator_error("run ABORTED"));
        assert!(!contains_simulator_error("No. of Data Rows : 1"));
    }
}
