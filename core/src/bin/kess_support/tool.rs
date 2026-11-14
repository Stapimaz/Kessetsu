use std::collections::BTreeSet;
use std::fs;

use kessetsu_core::compiler::{CompileOptions, CompileReport, DiagnosticStage};
use kessetsu_core::sim_result::format_quantity;
use kessetsu_core::tools::{TOOL_SCHEMA_VERSION, ToolRequest, calculate_tool};

use super::{
    CircuitTool, Format, Include, ToolCommand, build_json_output, diagnostic, emit,
    emit_human_diagnostics, plain_json_artifact,
};

pub(crate) fn run_tool(
    command: &ToolCommand,
    format: &Format,
    includes: &BTreeSet<Include>,
) -> i32 {
    let preferred_values = command.values.into();
    let request = match &command.tool {
        CircuitTool::Divider {
            vin,
            target,
            lower,
            load,
        } => ToolRequest::Divider {
            input_voltage: vin.clone(),
            target_voltage: target.clone(),
            lower_resistance: lower.clone(),
            load_resistance: load.clone(),
            preferred_values,
        },
        CircuitTool::RcLowpass { cutoff, resistance } => ToolRequest::RcLowpass {
            cutoff: cutoff.clone(),
            resistance: resistance.clone(),
            preferred_values,
        },
        CircuitTool::NonInvertingOpamp {
            gain,
            r1,
            vin,
            supply,
        } => ToolRequest::NonInvertingOpamp {
            target_gain: gain.clone(),
            input_resistance: r1.clone(),
            input_voltage: vin.clone(),
            supply_voltage: supply.clone(),
            preferred_values,
        },
        CircuitTool::ZenerRegulator {
            vin,
            vz,
            iload,
            iz_min,
        } => ToolRequest::ZenerRegulator {
            input_voltage: vin.clone(),
            zener_voltage: vz.clone(),
            load_current: iload.clone(),
            zener_knee_current: iz_min.clone(),
            preferred_values,
        },
    };
    let calculation = match calculate_tool(request) {
        Ok(result) => result,
        Err(error) => {
            let mut diagnostic = diagnostic("KES-F003", DiagnosticStage::Cli, error.message);
            diagnostic.field = Some(error.field);
            emit(
                format,
                "tool",
                includes,
                "error",
                CompileReport::failure(diagnostic),
                None,
                None,
                None,
            );
            return 2;
        }
    };
    let mut report = kessetsu_core::compile_source(
        &calculation.source,
        CompileOptions {
            include_ast: includes.contains(&Include::Ast),
            ..CompileOptions::default()
        },
    );
    let mut artifacts = Vec::new();
    if !report.has_errors()
        && let Some(path) = &command.output
    {
        if path.extension().and_then(|value| value.to_str()) != Some("kess") {
            report.diagnostics.push(diagnostic("KES-F003", DiagnosticStage::Cli,
                "Tool --output must have the .kess extension; use render/export on that source for other formats."));
        } else if path.exists() && !command.force {
            report.diagnostics.push(diagnostic(
                "KES-I003",
                DiagnosticStage::Io,
                format!(
                    "Output file '{}' already exists; pass --force to overwrite it.",
                    path.display()
                ),
            ));
        } else if let Err(error) = fs::write(path, &calculation.source) {
            report.diagnostics.push(diagnostic(
                "KES-I004",
                DiagnosticStage::Io,
                format!(
                    "Could not write generated source '{}': {error}",
                    path.display()
                ),
            ));
        } else {
            artifacts.push(plain_json_artifact(
                "circuit_source",
                path.to_string_lossy(),
            ));
        }
    }
    let failed = report.has_errors();
    if *format == Format::Json {
        let mut output = build_json_output(
            "tool",
            includes,
            if failed { "error" } else { "success" },
            &report,
            None,
            None,
            None,
            None,
        );
        output.domain_versions.tool = Some(TOOL_SCHEMA_VERSION);
        output.artifacts = artifacts;
        output.calculation = Some(calculation);
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("finite tool result must serialize")
        );
    } else {
        emit_human_diagnostics(&report);
        if !failed {
            println!(
                "{} — nominal analytical calculation ({:?})",
                calculation.name, calculation.preferred_values
            );
            for (name, value) in &calculation.components {
                println!("{name}: {}", format_quantity(value.value, value.unit));
            }
            for (name, value) in &calculation.results {
                println!("{name}: {}", format_quantity(value.value, value.unit));
            }
            for assumption in &calculation.assumptions {
                println!("Note: {assumption}");
            }
            if let Some(path) = &command.output {
                println!(
                    "Source written to {}. Next: kess simulate {}",
                    path.display(),
                    path.display()
                );
            } else {
                println!(
                    "\nEditable source (use --output circuit.kess to save):\n{}",
                    calculation.source
                );
            }
        }
    }
    if failed { 2 } else { 0 }
}
