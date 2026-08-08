use clap::{Parser, Subcommand, ValueEnum};
use netlang_core::erc::ErcDiagnostic;
use netlang_core::graph::{NetlistGraph, generate_spice};
use netlang_core::parser::parse_program;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "netlang", about = "NetLang Circuit Compiler and Simulator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format (human or json)
    #[arg(long, default_value = "human")]
    format: Format,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and run Electrical Rules Check (ERC)
    Check { file: String },
    /// Parse, ERC, and generate SPICE netlist
    Compile { file: String },
    /// Parse, ERC, generate netlist, and run Ngspice simulation
    Simulate { file: String },
    /// Parse, ERC, generate netlist, simulate and evaluate assertions
    Test { file: String },
    /// Parse, ERC, generate netlist, and render SVG schematic (Phase 3)
    Render { file: String },
}

#[derive(Clone, ValueEnum, PartialEq)]
enum Format {
    Human,
    Json,
}

#[derive(Serialize)]
pub struct JsonTestResult {
    pub metric: String,
    pub signal: String,
    pub pass: bool,
    pub actual: f64,
    pub threshold: f64,
}

#[derive(Serialize)]
struct JsonOutput {
    status: String,
    diagnostics: Vec<ErcDiagnostic>,
    spice_file: Option<String>,
    tests: Option<Vec<JsonTestResult>>,
}

fn main() {
    let cli = Cli::parse();

    let file_path = match &cli.command {
        Commands::Check { file } => file,
        Commands::Compile { file } => file,
        Commands::Simulate { file } => file,
        Commands::Test { file } => file,
        Commands::Render { file } => file,
    };

    let path = Path::new(file_path);

    // 1. Read file
    let input = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            print_error(
                &cli.format,
                &format!("Could not read file '{}': {}", file_path, e),
            );
            process::exit(2);
        }
    };

    // 2. Parse
    let program = match parse_program(&input) {
        Ok(p) => p,
        Err(e) => {
            print_error(&cli.format, &format!("Syntax Error:\n{}", e));
            process::exit(2);
        }
    };

    // 3. Flatten (Resolve modules)
    let flat_program = match program.flatten() {
        Ok(p) => p,
        Err(e) => {
            print_error(&cli.format, &format!("Flattening Error: {}", e));
            process::exit(2);
        }
    };

    // 4. Graph & ERC
    let circuit = match netlang_core::ir::ast_to_ir(&flat_program) {
        Ok(c) => c,
        Err(diagnostic) => {
            print_semantic_error(&cli.format, diagnostic);
            process::exit(1);
        }
    };

    let graph = NetlistGraph::build(&circuit);
    let errors = netlang_core::erc::check_rules(&circuit, &graph);

    if !errors.is_empty() {
        if cli.format == Format::Json {
            let out = JsonOutput {
                status: "error".to_string(),
                diagnostics: errors,
                spice_file: None,
                tests: None,
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        } else {
            for err in errors {
                eprintln!("[ERC ERROR] {}: {}", err.code, err.message);
            }
        }
        process::exit(1);
    }

    if matches!(cli.command, Commands::Check { .. }) {
        if cli.format == Format::Json {
            let out = JsonOutput {
                status: "success".to_string(),
                diagnostics: vec![],
                spice_file: None,
                tests: None,
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        } else {
            println!("[SUCCESS] Circuit parsed and ERC checks passed.");
        }
        process::exit(0);
    }

    // 5. Generate SPICE
    let spice = generate_spice(&circuit, &graph);
    let spice_path = path.with_extension("spice");

    if let Err(e) = fs::write(&spice_path, &spice) {
        print_error(
            &cli.format,
            &format!(
                "Could not write SPICE file to '{}': {}",
                spice_path.display(),
                e
            ),
        );
        process::exit(2);
    }

    if matches!(cli.command, Commands::Compile { .. }) {
        if cli.format == Format::Json {
            let out = JsonOutput {
                status: "success".to_string(),
                diagnostics: vec![],
                spice_file: Some(spice_path.to_string_lossy().to_string()),
                tests: None,
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        } else {
            println!(
                "[SUCCESS] SPICE netlist generated: {}",
                spice_path.display()
            );
        }
        process::exit(0);
    }

    // 6. Simulate
    if matches!(cli.command, Commands::Simulate { .. }) {
        if cli.format == Format::Human {
            println!("[INFO] Running ngspice simulation...");
        }

        let exe_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/ngspice/bin/ngspice_con.exe");

        if !exe_path.exists() {
            print_error(
                &cli.format,
                &format!("Embedded Ngspice not found at {:?}", exe_path),
            );
            process::exit(3);
        }

        let output = process::Command::new(exe_path)
            .arg("-b")
            .arg(&spice_path)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if cli.format == Format::Json {
                    // For Phase 3, we will parse Ngspice output. For now, just print success JSON.
                    let out = JsonOutput {
                        status: "success".to_string(),
                        diagnostics: vec![],
                        spice_file: Some(spice_path.to_string_lossy().to_string()),
                        tests: None,
                    };
                    println!("{}", serde_json::to_string_pretty(&out).unwrap());
                } else {
                    if !stdout.is_empty() {
                        println!("\n--- NGSPICE OUTPUT ---");
                        println!("{}", stdout);
                    }
                    if !stderr.is_empty() {
                        eprintln!("\n--- NGSPICE ERRORS ---");
                        eprintln!("{}", stderr);
                    }
                }
            }
            Err(e) => {
                print_error(&cli.format, &format!("Failed to execute ngspice: {}", e));
                process::exit(3);
            }
        }
    }

    if matches!(cli.command, Commands::Test { .. }) {
        if cli.format == Format::Human {
            println!("[INFO] Running tests and assertions...");
        }

        let sim_res = netlang_core::sim_result::run_simulation(&spice).unwrap_or_else(|e| {
            print_error(&cli.format, &format!("Failed to run simulation: {}", e));
            process::exit(3);
        });

        if !sim_res.success && cli.format == Format::Human {
            eprintln!("[WARNING] Simulation returned an error (convergence or fatal error).");
            for err in &sim_res.errors {
                eprintln!("  > {}", err);
            }
        }

        let results = netlang_core::sim_result::evaluate_assertions(&circuit, &sim_res);
        let mut all_passed = true;
        let mut json_results = Vec::new();

        for r in &results {
            if !r.pass {
                all_passed = false;
            }
            json_results.push(JsonTestResult {
                metric: r.assertion.metric.clone(),
                signal: r.assertion.signal.clone(),
                pass: r.pass,
                actual: r.actual,
                threshold: r.assertion.threshold.value,
            });
            if cli.format == Format::Human {
                let status = if r.pass {
                    "\x1b[32m[PASS]\x1b[0m"
                } else {
                    "\x1b[31m[FAIL]\x1b[0m"
                };
                let cmp_str = match r.assertion.cmp {
                    netlang_core::ast::Cmp::Lt => "<",
                    netlang_core::ast::Cmp::Gt => ">",
                    netlang_core::ast::Cmp::Le => "<=",
                    netlang_core::ast::Cmp::Ge => ">=",
                    netlang_core::ast::Cmp::Eq => "==",
                };
                if r.actual.is_nan() {
                    println!(
                        "{} {}({}) {} {} (actual: NaN/Not Found)",
                        status,
                        r.assertion.metric,
                        r.assertion.signal,
                        cmp_str,
                        r.assertion.threshold.value
                    );
                } else {
                    println!(
                        "{} {}({}) {} {} (actual: {:.6})",
                        status,
                        r.assertion.metric,
                        r.assertion.signal,
                        cmp_str,
                        r.assertion.threshold.value,
                        r.actual
                    );
                }
            }
        }

        if cli.format == Format::Json {
            let out = JsonOutput {
                status: if all_passed {
                    "success".to_string()
                } else {
                    "test_failed".to_string()
                },
                diagnostics: vec![],
                spice_file: Some(spice_path.to_string_lossy().to_string()),
                tests: Some(json_results),
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }

        if all_passed {
            if cli.format == Format::Human {
                println!("\n[SUCCESS] All assertions passed.");
            }
            process::exit(0);
        } else {
            if cli.format == Format::Human {
                println!("\n[ERROR] One or more assertions failed.");
            }
            process::exit(4);
        }
    }

    if matches!(cli.command, Commands::Render { .. }) {
        print_error(
            &cli.format,
            "Render command is not yet implemented (Phase 3)",
        );
        process::exit(0);
    }
}

fn print_error(format: &Format, message: &str) {
    if *format == Format::Json {
        let out = JsonOutput {
            status: "error".to_string(),
            diagnostics: vec![ErcDiagnostic {
                code: "INTERNAL".to_string(),
                severity: netlang_core::erc::Severity::Error,
                message: message.to_string(),
                component: None,
                pin: None,
            }],
            spice_file: None,
            tests: None,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        eprintln!("[ERROR] {}", message);
    }
}

fn print_semantic_error(format: &Format, diagnostic: netlang_core::ir::SemanticDiagnostic) {
    if *format == Format::Json {
        let out = JsonOutput {
            status: "error".to_string(),
            diagnostics: vec![ErcDiagnostic {
                code: diagnostic.code,
                severity: netlang_core::erc::Severity::Error,
                message: diagnostic.message,
                component: diagnostic.component,
                pin: diagnostic.field,
            }],
            spice_file: None,
            tests: None,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        eprintln!(
            "[SEMANTIC ERROR] {}: {}",
            diagnostic.code, diagnostic.message
        );
    }
}
