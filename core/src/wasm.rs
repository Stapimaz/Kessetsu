use crate::compile_inputs::CompileInputs;
use crate::compiler::{COMPILE_SCHEMA_VERSION, CompileOptions};
use crate::exporter::{
    EXPORT_SCHEMA_VERSION, ExportFormat, ExportOptions, RenderBackground, export_capabilities,
    export_report,
};
use crate::graph::NetlistGraph;
use crate::ir::Analysis;
use crate::sim_result::{AssertionReport, evaluate_assertions};
use crate::simulation::{SIMULATION_SCHEMA_VERSION, SimulationResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn calculate_circuit_tool(request: JsValue) -> Result<JsValue, JsValue> {
    let request: crate::tools::ToolRequest = serde_wasm_bindgen::from_value(request)
        .map_err(|error| JsValue::from_str(&format!("Invalid tool inputs: {error}")))?;
    let result = crate::tools::calculate_tool(request)
        .map_err(|error| JsValue::from_str(&format!("{}: {}", error.field, error.message)))?;
    to_json_compatible(&result, "tool calculation")
}

fn to_json_compatible<T: Serialize>(value: &T, context: &str) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| JsValue::from_str(&format!("Could not serialize {context}: {error}")))
}

/// Thin browser adapter over the canonical, side-effect-free compile pipeline.
#[wasm_bindgen]
pub fn compile_kessetsu(input: &str) -> Result<JsValue, JsValue> {
    compile_kessetsu_with_resources(input, JsValue::NULL)
}

fn decode_resources(value: JsValue) -> Result<crate::models::ExternalModelResources, JsValue> {
    let resources = if value.is_null() || value.is_undefined() {
        crate::models::ExternalModelResources::new()
    } else {
        serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Invalid local model bindings: {e}")))?
    };
    crate::model_resources::validate_resource_bindings(&resources)
        .map_err(|e| JsValue::from_str(&e))?;
    Ok(resources)
}

#[wasm_bindgen]
pub fn local_model_requirements(input: &str) -> Result<JsValue, JsValue> {
    let requirements =
        crate::model_resources::resource_requirements(input).map_err(|e| JsValue::from_str(&e))?;
    to_json_compatible(&requirements, "local model requirements")
}

#[wasm_bindgen]
pub fn compile_kessetsu_with_resources(
    input: &str,
    resources: JsValue,
) -> Result<JsValue, JsValue> {
    let resources = decode_resources(resources)?;
    let report = crate::compiler::compile_source_with_resources(
        input,
        CompileOptions::all_outputs(),
        &resources,
    );
    to_json_compatible(&report, "compile report")
}

/// Effective source from this report can use the existing save/share/simulation/export
/// paths without carrying hidden runtime override state.
#[wasm_bindgen]
pub fn compile_kessetsu_with_inputs(input: &str, inputs: JsValue) -> Result<JsValue, JsValue> {
    let inputs: CompileInputs = serde_wasm_bindgen::from_value(inputs)
        .map_err(|error| JsValue::from_str(&format!("Invalid compile inputs: {error}")))?;
    let report = crate::compiler::compile_source_with_inputs(
        input,
        CompileOptions::all_outputs(),
        &inputs,
        &crate::models::ExternalModelResources::new(),
    );
    to_json_compatible(&report, "compile report")
}

/// Returns the compile-report contract implemented by this WASM build.
///
/// Browser clients must compare reports against this value instead of
/// duplicating the version string in TypeScript.
#[wasm_bindgen]
pub fn compile_schema_version() -> String {
    COMPILE_SCHEMA_VERSION.to_string()
}

/// Generates the same versioned artifact contract used by the native CLI.
/// Binary payloads are serialized as byte arrays and should be downloaded as
/// `Uint8Array` by the browser adapter.
#[wasm_bindgen]
pub fn export_kessetsu(
    input: &str,
    format: &str,
    scale: f32,
    transparent: bool,
) -> Result<JsValue, JsValue> {
    export_kessetsu_with_resources(input, format, scale, transparent, JsValue::NULL)
}

#[wasm_bindgen]
pub fn export_kessetsu_with_resources(
    input: &str,
    format: &str,
    scale: f32,
    transparent: bool,
    resources: JsValue,
) -> Result<JsValue, JsValue> {
    let resources = decode_resources(resources)?;
    let format = ExportFormat::from_str(format).map_err(|error| JsValue::from_str(&error))?;
    let report = crate::compiler::compile_source_with_resources(
        input,
        CompileOptions {
            include_ast: false,
            generate_spice: true,
            generate_layout: true,
            generate_kicad: false,
        },
        &resources,
    );
    let artifact = export_report(
        &report,
        format,
        ExportOptions {
            scale,
            background: if transparent {
                RenderBackground::Transparent
            } else {
                RenderBackground::White
            },
        },
    )
    .map_err(|error| JsValue::from_str(&format!("{}: {}", error.code, error.message)))?;
    to_json_compatible(&artifact, "export artifact")
}

#[wasm_bindgen]
pub fn export_schema_version() -> String {
    EXPORT_SCHEMA_VERSION.to_string()
}

#[wasm_bindgen]
pub fn supported_export_capabilities() -> Result<JsValue, JsValue> {
    to_json_compatible(&export_capabilities(), "export capabilities")
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
    prepare_browser_simulation_with_resources(input, JsValue::NULL)
}

#[wasm_bindgen]
pub fn prepare_browser_simulation_with_resources(
    input: &str,
    resources: JsValue,
) -> Result<JsValue, JsValue> {
    let resources = decode_resources(resources)?;
    let report = crate::compiler::compile_source_with_resources(
        input,
        CompileOptions::default(),
        &resources,
    );
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
        .map(|(index, analysis)| {
            Ok(BrowserAnalysisPlan {
                index,
                analysis: analysis.clone(),
                netlist: crate::model_resources::browser_analysis_with_resources(
                    &circuit, &graph, analysis, &resources,
                )
                .map_err(|e| JsValue::from_str(&e))?,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    to_json_compatible(
        &BrowserSimulationPlan {
            schema_version: SIMULATION_SCHEMA_VERSION.to_string(),
            simulator_adapter: "eecircuit-engine@1.7.0".to_string(),
            analyses,
        },
        "simulation plan",
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvaluation {
    pub simulation: SimulationResult,
    pub assertions: AssertionReport,
}

#[wasm_bindgen]
pub fn evaluate_browser_simulation(input: &str, simulation: JsValue) -> Result<JsValue, JsValue> {
    evaluate_browser_simulation_with_resources(input, simulation, JsValue::NULL)
}

#[wasm_bindgen]
pub fn evaluate_browser_simulation_with_resources(
    input: &str,
    simulation: JsValue,
    resources: JsValue,
) -> Result<JsValue, JsValue> {
    let resources = decode_resources(resources)?;
    let report = crate::compiler::compile_source_with_resources(
        input,
        CompileOptions::default(),
        &resources,
    );
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
    to_json_compatible(
        &BrowserEvaluation {
            simulation,
            assertions,
        },
        "browser result",
    )
}
