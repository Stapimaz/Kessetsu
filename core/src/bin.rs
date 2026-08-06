use std::env;
use std::fs;
use std::process::Command;
use std::path::PathBuf;
use netlang_core::parser::parse_program;
use netlang_core::graph::{NetlistGraph, generate_spice};
use netlang_core::layout::*; // for future layout usage if needed

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args[1] != "simulate" {
        eprintln!("Usage: netlang simulate <file.nl>");
        std::process::exit(1);
    }

    let filepath = &args[2];
    let code = fs::read_to_string(filepath).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{}'", filepath);
        std::process::exit(1);
    });

    println!("[NetLang] Parsing and building netlist...");
    let program = parse_program(&code).expect("Syntax error in NetLang code");
    let flat_program = program.flatten().expect("Failed to flatten modules");
    let graph = NetlistGraph::build(&flat_program);
    let spice_code = generate_spice(&flat_program, &graph);

    let temp_cir = "temp.cir";
    fs::write(temp_cir, &spice_code).expect("Failed to write SPICE file");

    let exe_path = PathBuf::from("tools/ngspice/bin/ngspice_con.exe");
    if !exe_path.exists() {
        eprintln!("Error: Ngspice not found at {:?}", exe_path);
        eprintln!("Please ensure ngspice_con.exe is bundled in the tools/ngspice/bin folder.");
        std::process::exit(1);
    }

    println!("[NetLang] Simulating with Ngspice...");
    let output = Command::new(exe_path)
        .arg("-b")
        .arg(temp_cir)
        .output()
        .expect("Failed to execute Ngspice");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        println!("\n=== Simulation Results ===");
        println!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprintln!("\n=== Simulation Errors ===");
        eprintln!("{}", stderr);
    }
    
    // Optional: cleanup temp.cir
    let _ = fs::remove_file(temp_cir);
}
