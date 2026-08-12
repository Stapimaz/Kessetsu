use crate::compiler::{COMPILE_SCHEMA_VERSION, CompileOptions, compile_source};
use crate::graph::{NetlistGraph, generate_browser_analysis_netlist};
use crate::ir::Analysis;
use crate::sim_result::{AssertionReport, evaluate_assertions};
use crate::simulation::{SIMULATION_SCHEMA_VERSION, SimulationResult};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Thin browser adapter over the canonical, side-effect-free compile pipeline.
#[wasm_bindgen]
pub fn compile_netlang(input: &str) -> Result<JsValue, JsValue> {
    let report = compile_source(input, CompileOptions::all_outputs());
    serde_wasm_bindgen::to_value(&report)
        .map_err(|error| JsValue::from_str(&format!("Could not serialize compile report: {error}")))
}

/// Returns the compile-report contract implemented by this WASM build.
///
/// Browser clients must compare reports against this value instead of
/// duplicating the version string in TypeScript.
#[wasm_bindgen]
pub fn compile_schema_version() -> String {
    COMPILE_SCHEMA_VERSION.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAnalysisPlan {
    pub index: usize,
    pub analysis: Analysis,
    pub netlist: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSimulationPlan {
    pub schema_version: String,
    pub simulator_adapter: String,
    pub analyses: Vec<BrowserAnalysisPlan>,
}

#[wasm_bindgen]
pub fn prepare_browser_simulation(input: &str) -> Result<JsValue, JsValue> {
    let report = compile_source(input, CompileOptions::default());
    if report.has_errors() {
        return Err(JsValue::from_str(
            "Cannot prepare browser simulation for a source with compile/ERC errors",
        ));
    }
    let circuit = report
        .ir
        .ok_or_else(|| JsValue::from_str("Typed IR is missing"))?;
    let graph = NetlistGraph::build(&circuit);
    let analyses = circuit
        .analyses
        .iter()
        .enumerate()
        .map(|(index, analysis)| BrowserAnalysisPlan {
            index,
            analysis: analysis.clone(),
            netlist: generate_browser_analysis_netlist(&circuit, &graph, analysis),
        })
        .collect();
    serde_wasm_bindgen::to_value(&BrowserSimulationPlan {
        schema_version: SIMULATION_SCHEMA_VERSION.to_string(),
        simulator_adapter: "eecircuit-engine@1.7.0".to_string(),
        analyses,
    })
    .map_err(|error| JsValue::from_str(&format!("Could not serialize simulation plan: {error}")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvaluation {
    pub simulation: SimulationResult,
    pub assertions: AssertionReport,
}

#[wasm_bindgen]
pub fn evaluate_browser_simulation(input: &str, simulation: JsValue) -> Result<JsValue, JsValue> {
    let report = compile_source(input, CompileOptions::default());
    if report.has_errors() {
        return Err(JsValue::from_str(
            "Cannot evaluate browser simulation for a source with compile/ERC errors",
        ));
    }
    let circuit = report
        .ir
        .ok_or_else(|| JsValue::from_str("Typed IR is missing"))?;
    let simulation: SimulationResult =
        serde_wasm_bindgen::from_value(simulation).map_err(|error| {
            JsValue::from_str(&format!("Invalid browser simulation result: {error}"))
        })?;
    if simulation.schema_version != SIMULATION_SCHEMA_VERSION {
        return Err(JsValue::from_str(&format!(
            "Unsupported simulation schema: {}",
            simulation.schema_version
        )));
    }
    let assertions = evaluate_assertions(&circuit, &simulation);
    serde_wasm_bindgen::to_value(&BrowserEvaluation {
        simulation,
        assertions,
    })
    .map_err(|error| JsValue::from_str(&format!("Could not serialize browser result: {error}")))
}
