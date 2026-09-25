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

fn study_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}
fn decode_study<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|e| study_error(format!("Invalid study data: {e}")))
}

#[wasm_bindgen]
pub fn study_parameters(source: &str, resources: JsValue) -> Result<JsValue, JsValue> {
    let resources = decode_resources(resources)?;
    let report = crate::compiler::compile_source_with_resources(
        source,
        CompileOptions::default(),
        &resources,
    );
    if report.has_errors() {
        return Err(study_error(crate::experiment::diagnostic_text(&report)));
    }
    let circuit = report.ir.ok_or_else(|| study_error("Missing IR"))?;
    to_json_compatible(
        &circuit
            .parameter_manifest
            .parameters
            .into_iter()
            .filter(|p| p.instance_path.is_empty())
            .collect::<Vec<_>>(),
        "study parameters",
    )
}

#[wasm_bindgen]
pub fn plan_study(spec: JsValue, resources: JsValue) -> Result<JsValue, JsValue> {
    let plan =
        crate::experiment::plan_experiment(decode_study(spec)?, &decode_resources(resources)?)
            .map_err(study_error)?;
    to_json_compatible(&plan, "study plan")
}

#[wasm_bindgen]
pub fn prepare_study_case(
    plan: JsValue,
    index: usize,
    resources: JsValue,
) -> Result<JsValue, JsValue> {
    let plan: crate::experiment::ExperimentPlan = decode_study(plan)?;
    let case = plan
        .cases
        .get(index)
        .ok_or_else(|| study_error("Unknown study case"))?;
    let resources = decode_resources(resources)?;
    let report = crate::experiment::compile_case(&plan, case, &resources).map_err(study_error)?;
    let circuit = report.ir.ok_or_else(|| study_error("Missing IR"))?;
    let graph = NetlistGraph::build(&circuit);
    let analyses = circuit
        .analyses
        .iter()
        .enumerate()
        .map(|(index, analysis)| {
            let spice = crate::model_resources::browser_analysis_with_resources(
                &circuit, &graph, analysis, &resources,
            )
            .map_err(study_error)?;
            Ok(BrowserAnalysisPlan {
                index,
                analysis: analysis.clone(),
                netlist: crate::experiment::temperature_netlist(&spice, case.temperature_c)
                    .map_err(study_error)?,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    to_json_compatible(
        &BrowserSimulationPlan {
            schema_version: SIMULATION_SCHEMA_VERSION.into(),
            simulator_adapter: "eecircuit-engine@1.7.0".into(),
            analyses,
        },
        "study simulation",
    )
}

#[wasm_bindgen]
pub fn start_study_results(
    plan: JsValue,
    simulator: JsValue,
    fingerprint: &str,
) -> Result<JsValue, JsValue> {
    let results = crate::experiment::new_results(
        decode_study(plan)?,
        decode_study(simulator)?,
        fingerprint.into(),
    );
    to_json_compatible(&results, "study results")
}

#[wasm_bindgen]
pub fn evaluate_study_case(
    plan: JsValue,
    index: usize,
    simulation: JsValue,
    resources: JsValue,
    error: &str,
    cancelled: bool,
) -> Result<JsValue, JsValue> {
    let plan: crate::experiment::ExperimentPlan = decode_study(plan)?;
    let case = plan
        .cases
        .get(index)
        .ok_or_else(|| study_error("Unknown study case"))?;
    let row = if simulation.is_null() || simulation.is_undefined() {
        crate::experiment::failed_case(
            &case.id,
            if cancelled {
                crate::experiment::CaseStatus::Cancelled
            } else {
                crate::experiment::CaseStatus::Error
            },
            error.into(),
        )
    } else {
        let report = crate::experiment::compile_case(&plan, case, &decode_resources(resources)?)
            .map_err(study_error)?;
        crate::experiment::evaluate_case(&plan, case, &report, decode_study(simulation)?)
    };
    to_json_compatible(&row, "study case")
}

#[wasm_bindgen]
pub fn inspect_study_results(results: JsValue) -> Result<JsValue, JsValue> {
    let mut results: crate::experiment::ExperimentResults = decode_study(results)?;
    crate::experiment::validate_results(
        &results,
        &results.plan,
        &results.simulator,
        &results.solver_fingerprint,
    )
    .map_err(study_error)?;
    crate::experiment::summarize(&mut results);
    to_json_compatible(&results, "study results")
}

#[wasm_bindgen]
pub fn resume_study_results(
    results: JsValue,
    spec: JsValue,
    resources: JsValue,
    simulator: JsValue,
    fingerprint: &str,
) -> Result<JsValue, JsValue> {
    let mut results: crate::experiment::ExperimentResults = decode_study(results)?;
    let plan =
        crate::experiment::plan_experiment(decode_study(spec)?, &decode_resources(resources)?)
            .map_err(study_error)?;
    let simulator = decode_study(simulator)?;
    crate::experiment::validate_results(&results, &plan, &simulator, fingerprint)
        .map_err(study_error)?;
    crate::experiment::summarize(&mut results);
    to_json_compatible(&results, "resumed study")
}

#[wasm_bindgen]
pub fn export_study_results(
    results: JsValue,
    target: &str,
    analysis: usize,
    signal: &str,
    selected: JsValue,
) -> Result<String, JsValue> {
    let mut results: crate::experiment::ExperimentResults = decode_study(results)?;
    crate::experiment::validate_results(
        &results,
        &results.plan,
        &results.simulator,
        &results.solver_fingerprint,
    )
    .map_err(study_error)?;
    crate::experiment::summarize(&mut results);
    match target {
        "json" => serde_json::to_string(&results).map_err(|e| study_error(e.to_string())),
        "csv" => Ok(crate::experiment::results_csv(&results)),
        "data_csv" => Ok(crate::experiment::datasets_csv(&results)),
        "svg" => crate::experiment::plot_svg(
            &results,
            analysis,
            signal,
            &decode_study::<Vec<String>>(selected)?,
        )
        .map_err(study_error),
        "html" => Ok(crate::experiment::report_html(&results)),
        _ => Err(study_error("Unknown study export format")),
    }
}

#[wasm_bindgen]
pub fn study_effective_source(
    plan: JsValue,
    index: usize,
    resources: JsValue,
) -> Result<String, JsValue> {
    let plan: crate::experiment::ExperimentPlan = decode_study(plan)?;
    let case = plan
        .cases
        .get(index)
        .ok_or_else(|| study_error("Unknown study case"))?;
    let report = crate::experiment::compile_case(&plan, case, &decode_resources(resources)?)
        .map_err(study_error)?;
    Ok(report
        .effective_source
        .unwrap_or_else(|| crate::experiment::case_source(&plan, case).to_owned()))
}

#[wasm_bindgen]
pub fn compare_study_plot(
    left: JsValue,
    right: JsValue,
    analysis: usize,
    signal: &str,
) -> Result<String, JsValue> {
    let mut left: crate::experiment::ExperimentResults = decode_study(left)?;
    let right: crate::experiment::ExperimentResults = decode_study(right)?;
    crate::experiment::validate_results(
        &left,
        &left.plan,
        &left.simulator,
        &left.solver_fingerprint,
    )
    .map_err(study_error)?;
    crate::experiment::validate_results(
        &right,
        &right.plan,
        &right.simulator,
        &right.solver_fingerprint,
    )
    .map_err(study_error)?;
    let mut selected = Vec::new();
    for (case, row) in left.plan.cases.iter_mut().zip(&left.cases) {
        case.name = format!("{} / {}", left.plan.spec.name, case.name);
        if row.simulation.is_some() && selected.len() < 6 {
            selected.push(case.id.clone());
        }
    }
    let first_count = selected.len();
    for (mut case, mut row) in right.plan.cases.into_iter().zip(right.cases) {
        case.id = format!("comparison-{}", case.id);
        row.case_id = case.id.clone();
        case.name = format!("{} / {}", right.plan.spec.name, case.name);
        if row.simulation.is_some() && selected.len() < first_count + 6 {
            selected.push(case.id.clone());
        }
        left.plan.cases.push(case);
        left.cases.push(row);
    }
    crate::experiment::plot_svg(&left, analysis, signal, &selected).map_err(study_error)
}

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

/// Converts only the declared SPICE subset and returns editable source after
/// canonical recompilation. Browser file selection and persistence stay in JS.
#[wasm_bindgen]
pub fn import_spice_netlist(input: &str) -> Result<JsValue, JsValue> {
    to_json_compatible(&crate::spice_import::import_spice(input), "SPICE import report")
}

#[wasm_bindgen]
pub fn spice_import_schema_version() -> String {
    crate::spice_import::SPICE_IMPORT_SCHEMA_VERSION.to_string()
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
pub fn preview_research_csv(input: &str, dialect: JsValue) -> Result<JsValue, JsValue> {
    let dialect = if dialect.is_null() || dialect.is_undefined() {
        crate::research_data::CsvDialect::default()
    } else {
        serde_wasm_bindgen::from_value(dialect).map_err(|e| JsValue::from_str(&e.to_string()))?
    };
    let result =
        crate::research_data::preview_csv(input, &dialect).map_err(|e| JsValue::from_str(&e))?;
    to_json_compatible(&result, "CSV preview")
}

#[wasm_bindgen]
pub fn import_research_csv(input: &str, specification: JsValue) -> Result<JsValue, JsValue> {
    let spec = serde_wasm_bindgen::from_value(specification)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result =
        crate::research_data::import_csv(input, spec).map_err(|e| JsValue::from_str(&e))?;
    to_json_compatible(&result, "research data")
}

#[wasm_bindgen]
pub fn import_simulation_research_data(
    simulation: JsValue,
    specification: JsValue,
) -> Result<JsValue, JsValue> {
    let simulation = serde_wasm_bindgen::from_value(simulation)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let spec = serde_wasm_bindgen::from_value(specification)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = crate::research_data::import_simulation_data(&simulation, spec)
        .map_err(|e| JsValue::from_str(&e))?;
    to_json_compatible(&result, "simulation research data")
}

#[wasm_bindgen]
pub fn compare_research_data(
    data: JsValue,
    reference: JsValue,
    specification: JsValue,
) -> Result<JsValue, JsValue> {
    let data =
        serde_wasm_bindgen::from_value(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let reference =
        serde_wasm_bindgen::from_value(reference).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let spec = serde_wasm_bindgen::from_value(specification)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = crate::research_data::compare_data(&data, &reference, spec)
        .map_err(|e| JsValue::from_str(&e))?;
    to_json_compatible(&result, "data comparison")
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
