use std::env;
use std::fs;
use std::process;
use std::path::Path;

use netlang_core::parser::parse_program;
use netlang_core::graph::NetlistGraph;
use netlang_core::drc::check_rules;
use netlang_core::graph::generate_spice;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: netlang-cli <file.nl>");
        process::exit(1);
    }

    let file_path = &args[1];
    let path = Path::new(file_path);

    let input = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Could not read file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    // 1. Parse
    let program = match parse_program(&input) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[ERROR] Syntax Error:\n{}", e);
            process::exit(1);
        }
    };

    // 2. Flatten (Resolve modules)
    let flat_program = match program.flatten() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[ERROR] Flattening Error: {}", e);
            process::exit(1);
        }
    };

    // 3. Graph & DRC
    let graph = NetlistGraph::build(&flat_program);
    let errors = check_rules(&flat_program, &graph);

    if !errors.is_empty() {
        for err in errors {
            eprintln!("[DRC ERROR] {}", err.message);
        }
        process::exit(1);
    }

    // 4. Generate SPICE
    let spice = generate_spice(&flat_program, &graph);
    
    let spice_path = path.with_extension("spice");
    if let Err(e) = fs::write(&spice_path, &spice) {
        eprintln!("[ERROR] Could not write to '{}': {}", spice_path.display(), e);
        process::exit(1);
    }

    println!("[SUCCESS] SPICE netlist generated: {}", spice_path.display());

    // 5. Run ngspice if requested
    let run_sim = args.contains(&"--run".to_string());
    if run_sim || spice.contains(".control") {
        println!("[INFO] Running ngspice simulation...");
        let output = process::Command::new("ngspice")
            .arg("-b")
            .arg(&spice_path)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stdout.is_empty() {
                    println!("\n--- NGSPICE OUTPUT ---");
                    println!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprintln!("\n--- NGSPICE ERRORS ---");
                    eprintln!("{}", stderr);
                }
            }
            Err(e) => {
                eprintln!("[WARNING] Could not execute ngspice: {}", e);
                eprintln!("[WARNING] Please ensure ngspice is installed and in your PATH to run simulations.");
            }
        }
    }
}
