//! Versioned, machine-readable discovery for external agents and automation.
//! This describes supported entry points; detailed language semantics remain in
//! the public references linked by the manifest.

use crate::compile_inputs::INPUT_SCHEMA_VERSION;
use crate::compiler::COMPILE_SCHEMA_VERSION;
use crate::experiment::{EXPERIMENT_SCHEMA, MAX_CASES, RESULTS_SCHEMA};
use crate::exporter::{ExportDescriptor, export_capabilities};
use crate::expression::PARAMETER_SCHEMA_VERSION;
use crate::handoff::HANDOFF_SCHEMA_VERSION;
use crate::measurement::{MEASUREMENT_SCHEMA_VERSION, SUPPORTED_METRICS};
use crate::models::{
    MAX_EXTERNAL_MODEL_BYTES, MODEL_LOCK_SCHEMA_VERSION, MODEL_MANIFEST_SCHEMA_VERSION,
};
use crate::parser::{MAX_SOURCE_BYTES, MAX_SOURCE_STATEMENTS};
use crate::requirements::{MAX_REQUIREMENTS_BYTES, REQUIREMENTS_SCHEMA_VERSION};
use crate::research_data::{DATA_SCHEMA, MAX_COLUMNS, MAX_CSV_BYTES, MAX_ROWS};
use crate::schematic::SCHEMATIC_SCHEMA_VERSION;
use crate::sim_result::ASSERTION_SCHEMA_VERSION;
use crate::simulation::SIMULATION_SCHEMA_VERSION;
use crate::spice_import::SPICE_IMPORT_SCHEMA_VERSION;
use crate::stress::PART_STRESS_SCHEMA_VERSION;
use crate::tools::TOOL_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CAPABILITIES_SCHEMA_VERSION: &str = "kessetsu.capabilities.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub schema_version: String,
    pub product: ProductCapability,
    pub contracts: BTreeMap<String, String>,
    pub commands: Vec<CommandCapability>,
    pub calculators: Vec<String>,
    pub language: LanguageCapability,
    pub exports: Vec<ExportDescriptor>,
    pub limits: CapabilityLimits,
    pub agent_workflow: Vec<WorkflowStep>,
    pub boundaries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductCapability {
    pub name: String,
    pub version: String,
    pub source_extension: String,
    pub license: String,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandCapability {
    pub name: String,
    pub purpose: String,
    pub accepts_stdin: bool,
    pub may_run_ngspice: bool,
    pub output_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCapability {
    pub component_keywords: Vec<String>,
    pub structural_keywords: Vec<String>,
    pub analyses: Vec<String>,
    pub measurement_metrics: Vec<String>,
    pub source_waveforms: Vec<String>,
    pub references: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityLimits {
    pub source_bytes: usize,
    pub source_statements: usize,
    pub external_model_bytes: usize,
    pub requirements_bytes: usize,
    pub experiment_cases: usize,
    pub research_csv_bytes: usize,
    pub research_rows: usize,
    pub research_columns: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub goal: String,
    pub command: String,
    pub note: String,
}

pub fn capability_manifest(cli_schema_version: &str) -> CapabilityManifest {
    let contracts = BTreeMap::from([
        ("assertions".into(), ASSERTION_SCHEMA_VERSION.into()),
        ("capabilities".into(), CAPABILITIES_SCHEMA_VERSION.into()),
        ("cli".into(), cli_schema_version.into()),
        ("compile".into(), COMPILE_SCHEMA_VERSION.into()),
        ("compile_inputs".into(), INPUT_SCHEMA_VERSION.into()),
        ("experiments".into(), EXPERIMENT_SCHEMA.into()),
        ("experiment_results".into(), RESULTS_SCHEMA.into()),
        ("handoff".into(), HANDOFF_SCHEMA_VERSION.into()),
        ("measurements".into(), MEASUREMENT_SCHEMA_VERSION.into()),
        ("model_lock".into(), MODEL_LOCK_SCHEMA_VERSION.into()),
        ("models".into(), MODEL_MANIFEST_SCHEMA_VERSION.into()),
        ("parameters".into(), PARAMETER_SCHEMA_VERSION.into()),
        ("part_stress".into(), PART_STRESS_SCHEMA_VERSION.into()),
        ("requirements".into(), REQUIREMENTS_SCHEMA_VERSION.into()),
        ("research_data".into(), DATA_SCHEMA.into()),
        ("schematic".into(), SCHEMATIC_SCHEMA_VERSION.into()),
        ("simulation".into(), SIMULATION_SCHEMA_VERSION.into()),
        ("spice_import".into(), SPICE_IMPORT_SCHEMA_VERSION.into()),
        ("tools".into(), TOOL_SCHEMA_VERSION.into()),
        (
            "exports".into(),
            crate::exporter::EXPORT_SCHEMA_VERSION.into(),
        ),
    ]);
    #[cfg(not(target_arch = "wasm32"))]
    let contracts = {
        let mut contracts = contracts;
        contracts.insert("fit".into(), crate::fitting::FIT_SCHEMA.into());
        contracts.insert(
            "fit_results".into(),
            crate::fitting::FIT_RESULT_SCHEMA.into(),
        );
        contracts.insert(
            "research_package".into(),
            crate::research_package::RESEARCH_PACKAGE_SCHEMA.into(),
        );
        contracts
    };

    CapabilityManifest {
        schema_version: CAPABILITIES_SCHEMA_VERSION.into(),
        product: ProductCapability {
            name: "Kessetsu".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            source_extension: ".kess".into(),
            license: "AGPL-3.0-only".into(),
            documentation: "https://kessetsu.com/docs/".into(),
        },
        contracts,
        commands: command_capabilities(),
        calculators: vec!["divider".into(), "rc-lowpass".into()],
        language: LanguageCapability {
            component_keywords: [
                "resistor",
                "capacitor",
                "inductor",
                "diode",
                "transistor",
                "mosfet",
                "opamp",
                "device",
                "source",
                "current_source",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            structural_keywords: [
                "net",
                "param",
                "connect",
                "module",
                "use",
                "part",
                "model",
                "subcircuit",
                "external_subcircuit",
                "model_include",
                "simulate",
                "assert",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            analyses: ["op", "tran", "ac", "dc"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            measurement_metrics: SUPPORTED_METRICS
                .iter()
                .map(|metric| (*metric).to_string())
                .collect(),
            source_waveforms: ["dc_literal", "ac", "sine", "sine_ac", "pulse", "pwl"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            references: BTreeMap::from([
                (
                    "agent_loop".into(),
                    "https://kessetsu.com/docs/reference/cli/#agent-loop".into(),
                ),
                (
                    "exports".into(),
                    "https://kessetsu.com/docs/reference/exports/".into(),
                ),
                (
                    "language".into(),
                    "https://kessetsu.com/docs/reference/language/".into(),
                ),
                (
                    "measurements".into(),
                    "https://kessetsu.com/docs/reference/measurements/".into(),
                ),
                (
                    "supported_domain".into(),
                    "https://kessetsu.com/docs/reference/supported-domain/".into(),
                ),
            ]),
        },
        exports: export_capabilities(),
        limits: CapabilityLimits {
            source_bytes: MAX_SOURCE_BYTES,
            source_statements: MAX_SOURCE_STATEMENTS,
            external_model_bytes: MAX_EXTERNAL_MODEL_BYTES,
            requirements_bytes: MAX_REQUIREMENTS_BYTES,
            experiment_cases: MAX_CASES,
            research_csv_bytes: MAX_CSV_BYTES,
            research_rows: MAX_ROWS,
            research_columns: MAX_COLUMNS,
        },
        agent_workflow: vec![
            WorkflowStep {
                goal: "Discover this installed build".into(),
                command: "kess capabilities --format json".into(),
                note: "Read contract versions and supported surfaces before generating source.".into(),
            },
            WorkflowStep {
                goal: "Validate a complete in-memory candidate".into(),
                command: "kess check - --format json".into(),
                note: "Send the complete .kess source on stdin; no adjacent artifact is created.".into(),
            },
            WorkflowStep {
                goal: "Inspect the canonical netlist when needed".into(),
                command: "kess compile - --format json --include spice".into(),
                note: "Verbose fields are opt-in; ordinary retries stay compact and deterministic.".into(),
            },
            WorkflowStep {
                goal: "Evaluate independently owned requirements".into(),
                command: "kess test - --format json --requirements limits.kessreq".into(),
                note: "Keep acceptance criteria outside the design agent's editable source.".into(),
            },
            WorkflowStep {
                goal: "Persist an engineering artifact".into(),
                command: "kess export design.kess --target kicad --output design.kicad_sch".into(),
                note: "Existing outputs require explicit --force; inspect returned capability and loss fields.".into(),
            },
        ],
        boundaries: vec![
            "Simulation and supplied part-limit comparisons are engineering evidence, not physical validation or a hardware-safety guarantee.".into(),
            "SPICE import accepts a documented fail-closed subset; unsupported constructs never become an apparently complete circuit.".into(),
            "PCB layout/DRC, RF/EM, thermal/reliability and arbitrary vendor-model portability are outside the current supported domain.".into(),
        ],
    }
}

fn command_capabilities() -> Vec<CommandCapability> {
    [
        (
            "capabilities",
            "Describe this installed build for agents and automation",
            false,
            false,
            "stdout_only",
        ),
        (
            "import",
            "Convert supported SPICE into editable, recompiled Kessetsu source",
            true,
            false,
            "writes_source_when_successful",
        ),
        (
            "data",
            "Import and compare explicitly mapped research data",
            false,
            false,
            "subcommand_specific",
        ),
        (
            "fit",
            "Score finite study candidates against calibration and validation data",
            false,
            false,
            "explicit_output",
        ),
        (
            "study",
            "Create, plan, run and package bounded parameter studies",
            false,
            true,
            "subcommand_specific",
        ),
        (
            "tool",
            "Calculate a bounded engineering starting point and emit editable source",
            false,
            false,
            "stdout_or_explicit_output",
        ),
        (
            "check",
            "Parse, elaborate and run ERC",
            true,
            false,
            "stdout_only",
        ),
        (
            "compile",
            "Generate the canonical SPICE netlist",
            true,
            false,
            "path_input_writes_adjacent;stdin_is_memory_only",
        ),
        (
            "simulate",
            "Run declared analyses through Ngspice",
            true,
            true,
            "path_input_writes_adjacent;stdin_is_memory_only",
        ),
        (
            "test",
            "Simulate and evaluate design-owned or external requirements",
            true,
            true,
            "path_input_writes_adjacent;stdin_is_memory_only",
        ),
        (
            "render",
            "Render the verified canonical schematic",
            true,
            false,
            "explicit_or_path_derived_output",
        ),
        (
            "export",
            "Produce a capability-described visual, machine or EDA artifact",
            true,
            false,
            "explicit_or_path_derived_output",
        ),
    ]
    .into_iter()
    .map(
        |(name, purpose, accepts_stdin, may_run_ngspice, output_policy)| CommandCapability {
            name: name.into(),
            purpose: purpose.into(),
            accepts_stdin,
            may_run_ngspice,
            output_policy: output_policy.into(),
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_uses_live_export_and_measurement_catalogs() {
        let manifest = capability_manifest("kessetsu.cli.v1");
        assert_eq!(manifest.schema_version, CAPABILITIES_SCHEMA_VERSION);
        assert_eq!(manifest.exports, export_capabilities());
        assert_eq!(
            manifest.language.measurement_metrics,
            SUPPORTED_METRICS
                .iter()
                .map(|metric| (*metric).to_string())
                .collect::<Vec<_>>()
        );
        assert!(
            manifest
                .commands
                .iter()
                .any(|command| command.name == "test")
        );
        assert_eq!(manifest.limits.experiment_cases, MAX_CASES);
    }
}
