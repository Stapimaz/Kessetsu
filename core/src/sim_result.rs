use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use crate::ir::{Assertion, CircuitIR};
use crate::ast::Cmp;

#[derive(Debug, Clone)]
pub struct SimResult {
    pub success: bool,
    pub meas_results: HashMap<String, f64>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub pass: bool,
    pub assertion: Assertion,
    pub actual: f64,
}

pub fn get_ngspice_path() -> PathBuf {
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("netlang"));
    
    // Option 1: running from workspace root (e.g., via cargo run from /core)
    let cwd_path = PathBuf::from("tools/ngspice/bin/ngspice_con.exe");
    if cwd_path.exists() {
        return cwd_path;
    }
    
    // Option 2: running from /core (where workspace root is ../)
    let cwd_path_alt = PathBuf::from("../core/tools/ngspice/bin/ngspice_con.exe");
    if cwd_path_alt.exists() {
        return cwd_path_alt;
    }
    
    // Option 3: Next to the executable (for release distribution)
    if let Some(parent) = exe_path.parent() {
        let rel_path = parent.join("tools/ngspice/bin/ngspice_con.exe");
        if rel_path.exists() {
            return rel_path;
        }
        
        // Option 4: target/debug/deps
        if let Some(p2) = parent.parent() {
            if let Some(p3) = p2.parent() {
                if let Some(p4) = p3.parent() {
                    let root_path = p4.join("core/tools/ngspice/bin/ngspice_con.exe");
                    if root_path.exists() {
                        return root_path;
                    }
                }
            }
        }
    }
    
    PathBuf::from("ngspice_con.exe") // Fallback to system path
}

pub fn run_simulation(spice_content: &str) -> Result<SimResult, String> {
    let temp_file = env::temp_dir().join("netlang_temp.spice");
    fs::write(&temp_file, spice_content).map_err(|e| format!("Failed to write temp spice file: {}", e))?;

    let ngspice_path = get_ngspice_path();
    
    let output = Command::new(&ngspice_path)
        .arg("-b")
        .arg(&temp_file)
        .output()
        .map_err(|e| format!("Failed to execute ngspice at {:?}: {}", ngspice_path, e))?;
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let mut success = output.status.success();
    let mut meas_results = HashMap::new();
    let mut errors = Vec::new();
    
    for line in stderr.lines() {
        if line.contains("error") || line.contains("Error") {
            errors.push(line.to_string());
            success = false;
        }
    }
    
    for line in stdout.lines() {
        if line.contains("error") || line.contains("Error") || line.contains("fatal") || line.contains("aborted") {
            errors.push(line.to_string());
            success = false;
        }
        
        // .meas prints things like: "max_v = 3.21e-01" or "max_v_my_signal = 6.20e-08 at= 4.0e-08"
        if line.contains("=") && !line.trim().starts_with("Doing analysis") && !line.trim().starts_with("Warning") {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim().to_lowercase();
                let val_str = parts[1].split_whitespace().next().unwrap_or("");
                if let Ok(val) = val_str.parse::<f64>() {
                    meas_results.insert(name, val);
                }
            }
        }
    }
    
    Ok(SimResult {
        success,
        meas_results,
        errors,
    })
}

const EPSILON: f64 = 1e-6;

fn evaluate_cmp(actual: f64, expected: f64, cmp: &Cmp) -> bool {
    match cmp {
        Cmp::Eq => (actual - expected).abs() < EPSILON,
        Cmp::Lt => actual < expected,
        Cmp::Gt => actual > expected,
        Cmp::Le => actual <= expected + EPSILON,
        Cmp::Ge => actual >= expected - EPSILON,
    }
}

pub fn evaluate_assertions(circuit: &CircuitIR, sim_result: &SimResult) -> Vec<TestResult> {
    let mut results = Vec::new();
    
    for assert in &circuit.assertions {
        let raw_name = format!("{}_{}", assert.metric, assert.signal);
        let safe_name = raw_name.replace("(", "_").replace(")", "").to_lowercase();
        
        if let Some(&actual) = sim_result.meas_results.get(&safe_name) {
            let pass = evaluate_cmp(actual, assert.threshold, &assert.cmp);
            results.push(TestResult {
                pass,
                assertion: assert.clone(),
                actual,
            });
        } else {
            results.push(TestResult {
                pass: false,
                assertion: assert.clone(),
                actual: f64::NAN,
            });
        }
    }
    
    results
}
