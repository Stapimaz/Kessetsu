use crate::compiler::{CompileOptions, DiagnosticSeverity, compile_source};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SPICE_IMPORT_SCHEMA_VERSION: &str = "kessetsu.spice-import.v1";
const MAX_IMPORT_BYTES: usize = 8 * 1024 * 1024;
const MAX_IMPORT_LINES: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDiagnostic {
    pub code: String,
    pub severity: ImportSeverity,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportNameKind {
    Component,
    Net,
    Parameter,
    Module,
    ModulePort,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportNameMapping {
    pub kind: ImportNameKind,
    pub original: String,
    pub kessetsu: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub components: usize,
    pub nets: usize,
    pub analyses: usize,
    #[serde(default)]
    pub subcircuits: usize,
    #[serde(default)]
    pub instances: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceImportReport {
    pub schema_version: String,
    pub dialect: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub names: Vec<ImportNameMapping>,
    pub summary: ImportSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kess_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_schema_version: Option<String>,
}

impl SpiceImportReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ImportSeverity::Error)
    }
}

#[derive(Debug, Clone)]
enum ImportedKind {
    Resistor(String),
    Capacitor(String),
    Inductor(String),
    VoltageSource(String),
    CurrentSource(String),
    Diode(String),
    Bjt {
        polarity: &'static str,
        model: String,
    },
    Mosfet(String),
    Switch(String),
    OpAmp(String),
    Vcvs(String),
    Vccs(String),
    Ccvs(String),
    Cccs(String),
}

#[derive(Debug, Clone)]
struct ImportedComponent {
    original_name: String,
    nodes: Vec<String>,
    pins: &'static [&'static str],
    kind: ImportedKind,
    line: usize,
}

#[derive(Debug, Clone)]
struct LogicalLine {
    number: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct ImportedParameter {
    original_name: String,
    mapped_name: String,
    value: String,
    numeric_value: f64,
    line: usize,
}

#[derive(Debug, Clone)]
struct SubcircuitBlock {
    header: LogicalLine,
    body: Vec<LogicalLine>,
    end: LogicalLine,
}

#[derive(Debug, Clone)]
struct ImportedPort {
    original_name: String,
    mapped_name: String,
}

#[derive(Debug, Clone)]
struct ImportedSubcircuit {
    original_name: String,
    mapped_name: String,
    ports: Vec<ImportedPort>,
    parameters: Vec<ImportedParameter>,
    parameter_units: BTreeMap<String, ImportUnit>,
    components: Vec<ImportedComponent>,
    component_names: BTreeMap<String, String>,
    node_names: BTreeMap<String, String>,
    internal_nets: Vec<String>,
}

#[derive(Debug, Clone)]
struct ImportedInstance {
    original_name: String,
    mapped_name: String,
    module_name: String,
    nodes: Vec<String>,
    overrides: Vec<(String, String)>,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportUnit {
    Ohm,
    Farad,
    Henry,
    Volt,
    Ampere,
    Hertz,
    Second,
    Ratio,
    Siemens,
}

impl ImportUnit {
    const fn kessetsu_name(self) -> &'static str {
        match self {
            Self::Ohm => "Ohm",
            Self::Farad => "F",
            Self::Henry => "H",
            Self::Volt => "V",
            Self::Ampere => "A",
            Self::Hertz => "Hz",
            Self::Second => "s",
            Self::Ratio => "ratio",
            Self::Siemens => "S",
        }
    }
}

type ImportedNames = (BTreeMap<String, String>, BTreeMap<String, String>);
type NameAllocationError = (usize, String);

/// Built-in Kessetsu model names. Inline SPICE models with these names are renamed so the
/// user's parameters are never silently replaced by the built-in card.
const BUILTIN_MODEL_NAMES: &[&str] = &[
    "2N3904",
    "2N3906",
    "2N2222",
    "1N4148",
    "1N4007",
    "1N4733A",
    "BZX84C3V3",
    "BZX84C5V1",
    "BZX84C12",
    "IRF540",
    "KESSETSU_POWER_NPN_V1",
    "KESSETSU_POWER_PNP_V1",
    "KESSETSU_PMOS_V1",
    "KESSETSU_SW_V1",
    "KESSETSU_OPAMP_V1",
    "KESSETSU_OPAMP_V2",
];

/// Vendor/LTspice descriptive fields that carry no Ngspice electrical meaning.
const DESCRIPTIVE_MODEL_FIELDS: &[&str] = &[
    "mfg", "type", "vceo", "icrate", "icrating", "iave", "vpk", "vds", "ron", "qg",
];

/// Output/control directives whose job (printing, plotting, measuring) is owned by Kessetsu
/// measurements and requirements. They are skipped with a warning, never executed.
const OUTPUT_DIRECTIVES: &[&str] = &[
    ".print",
    ".plot",
    ".probe",
    ".save",
    ".meas",
    ".measure",
    ".four",
    ".width",
    ".options",
    ".option",
    ".temp",
    ".backanno",
    ".nodeset",
    ".ic",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineModelKind {
    Diode,
    Bjt(&'static str),
    Mosfet(&'static str),
    Switch,
}

#[derive(Debug, Clone)]
struct InlineModel {
    original: String,
    mapped: String,
    kind: InlineModelKind,
    parameters: Vec<(String, String)>,
}

type InlineModels = BTreeMap<String, InlineModel>;

pub fn import_spice(source: &str) -> SpiceImportReport {
    let mut report = SpiceImportReport {
        schema_version: SPICE_IMPORT_SCHEMA_VERSION.into(),
        dialect: "ngspice-compatible-subset-v1".into(),
        source_sha256: format!("sha256:{:x}", Sha256::digest(source.as_bytes())),
        source_bytes: source.len(),
        diagnostics: Vec::new(),
        names: Vec::new(),
        summary: ImportSummary::default(),
        kess_source: None,
        compile_schema_version: None,
    };
    if source.len() > MAX_IMPORT_BYTES {
        push_error(
            &mut report,
            "KES-N001",
            format!("SPICE input exceeds the {MAX_IMPORT_BYTES} byte limit"),
            None,
        );
        return report;
    }
    if source.contains('\0') {
        push_error(
            &mut report,
            "KES-N002",
            "SPICE input contains a NUL byte",
            None,
        );
        return report;
    }

    let logical = logical_lines(source, &mut report);
    if report.has_errors() {
        return report;
    }
    let (root_lines, blocks) = partition_subcircuits(logical, &mut report);
    if report.has_errors() {
        return report;
    }
    let parameters = collect_parameters(&root_lines, &mut report);
    if report.has_errors() {
        return report;
    }
    let parameter_lookup = parameters
        .iter()
        .map(|parameter| (parameter.original_name.to_ascii_lowercase(), parameter))
        .collect::<BTreeMap<_, _>>();
    let models = collect_models(&root_lines, &mut report);
    if report.has_errors() {
        return report;
    }
    let mut module_identifiers = BTreeSet::new();
    let mut module_names = BTreeMap::<String, (String, usize)>::new();
    let mut user_blocks = Vec::new();
    for block in blocks {
        let fields = split_fields(block.header.text.trim());
        let Some(name) = fields.get(1) else {
            push_error(
                &mut report,
                "KES-N007",
                ".SUBCKT requires a name and at least one interface node",
                Some(block.header.number),
            );
            continue;
        };
        if !is_spice_symbol(name) {
            push_error(
                &mut report,
                "KES-N007",
                format!("invalid SPICE subcircuit name '{name}'"),
                Some(block.header.number),
            );
            continue;
        }
        if (name.eq_ignore_ascii_case("KESSETSU_OPAMP_V1")
            || name.eq_ignore_ascii_case("KESSETSU_OPAMP_V2"))
            && fields.len() == 7
            && !fields[2..]
                .iter()
                .any(|field| field.contains('=') || field.eq_ignore_ascii_case("params:"))
        {
            let canonical_name = if name.eq_ignore_ascii_case("KESSETSU_OPAMP_V2") {
                "KESSETSU_OPAMP_V2"
            } else {
                "KESSETSU_OPAMP_V1"
            };
            report.diagnostics.push(ImportDiagnostic {
                code: "KES-N012".into(),
                severity: ImportSeverity::Info,
                message: format!("recognized built-in op-amp subcircuit '{canonical_name}'"),
                line: Some(block.header.number),
                column: Some(1),
            });
            continue;
        }
        let folded = name.to_ascii_lowercase();
        if let Some((previous, previous_line)) = module_names.get(&folded) {
            push_error(
                &mut report,
                "KES-N007",
                format!(
                    "subcircuit '{name}' duplicates '{previous}' from line {previous_line}; SPICE names are case-insensitive"
                ),
                Some(block.header.number),
            );
            continue;
        }
        module_names.insert(
            folded,
            (
                unique_identifier(name, "Module", &mut module_identifiers),
                block.header.number,
            ),
        );
        user_blocks.push(block);
    }
    if report.has_errors() {
        return report;
    }
    let mut subcircuits = Vec::new();
    for block in &user_blocks {
        match parse_subcircuit(block, &module_names, &models, &mut report.diagnostics) {
            Ok(subcircuit) => subcircuits.push(subcircuit),
            Err((line, message)) => push_error(&mut report, "KES-N007", message, Some(line)),
        }
    }
    if report.has_errors() {
        return report;
    }
    let module_lookup = subcircuits
        .iter()
        .map(|module| (module.original_name.to_ascii_lowercase(), module))
        .collect::<BTreeMap<_, _>>();

    let mut parameter_units = BTreeMap::<String, ImportUnit>::new();
    let mut components = Vec::new();
    let mut instances = Vec::new();
    let mut analyses = Vec::<(usize, Vec<String>)>::new();
    let mut ended = false;
    let mut control_start = None;

    for line in root_lines {
        let text = line.text.trim();
        if text.is_empty() || text.starts_with('*') || text.starts_with(';') {
            continue;
        }
        if let Some(start) = control_start {
            if text
                .split_whitespace()
                .next()
                .is_some_and(|field| field.eq_ignore_ascii_case(".endc"))
            {
                control_start = None;
                push_warning(
                    &mut report,
                    "KES-N011",
                    format!(
                        "skipped simulator .control block (lines {start}-{}); Kessetsu runs only dot analyses and owns measurement/assertions",
                        line.number
                    ),
                    Some(start),
                );
            }
            continue;
        }
        if ended {
            push_error(
                &mut report,
                "KES-N003",
                "content after .end is not part of the imported circuit",
                Some(line.number),
            );
            continue;
        }
        if text.starts_with('.') {
            let fields = split_fields(text);
            let directive = fields[0].to_ascii_lowercase();
            match directive.as_str() {
                ".title" => report.diagnostics.push(ImportDiagnostic {
                    code: "KES-N010".into(),
                    severity: ImportSeverity::Info,
                    message: "SPICE title is metadata and is not copied into executable source"
                        .into(),
                    line: Some(line.number),
                    column: Some(1),
                }),
                ".end" if fields.len() == 1 => ended = true,
                ".op" | ".tran" | ".ac" | ".dc" => {
                    analyses.push((line.number, fields));
                }
                ".param" | ".model" => {}
                ".control" => control_start = Some(line.number),
                ".endc" => push_unsupported(
                    &mut report,
                    line.number,
                    ".endc without a matching .control",
                ),
                ".include" | ".lib" => push_unsupported(
                    &mut report,
                    line.number,
                    "model libraries require an explicit resource binding; arbitrary paths are not imported",
                ),
                ".ends" => push_unsupported(
                    &mut report,
                    line.number,
                    "unmatched .ENDS directive is not importable",
                ),
                output if OUTPUT_DIRECTIVES.contains(&output) => push_warning(
                    &mut report,
                    "KES-N011",
                    format!(
                        "skipped output/simulator-option directive '{}'; use Kessetsu assertions or .kessreq requirements for measurements",
                        fields[0]
                    ),
                    Some(line.number),
                ),
                _ => push_unsupported(
                    &mut report,
                    line.number,
                    &format!("directive '{}' is not supported", fields[0]),
                ),
            }
            continue;
        }

        let is_first_line = line.number == 1;
        if text
            .chars()
            .next()
            .is_some_and(|character| character.eq_ignore_ascii_case(&'x'))
        {
            if let Some(opamp_result) =
                try_parse_builtin_opamp(text, line.number, &module_lookup, Some(&mut report))
            {
                match opamp_result {
                    Ok(component) => components.push(component),
                    Err(_) if is_first_line => push_title_info(&mut report),
                    Err(message) => push_error(&mut report, "KES-N007", message, Some(line.number)),
                }
            } else {
                match parse_instance(
                    text,
                    line.number,
                    &module_lookup,
                    &parameter_lookup,
                    &mut parameter_units,
                ) {
                    Ok(instance) => instances.push(instance),
                    Err(_) if is_first_line => push_title_info(&mut report),
                    Err(message) => push_error(&mut report, "KES-N007", message, Some(line.number)),
                }
            }
        } else {
            match parse_component(
                text,
                line.number,
                &parameter_lookup,
                &mut parameter_units,
                &models,
                &mut report.diagnostics,
            ) {
                Ok(component) => components.push(component),
                Err(_) if is_first_line => push_title_info(&mut report),
                Err(message) => push_error(&mut report, "KES-N002", message, Some(line.number)),
            }
        }
    }

    if let Some(start) = control_start {
        push_unsupported(
            &mut report,
            start,
            ".control block is not terminated by .endc",
        );
    }
    if components.is_empty() && instances.is_empty() {
        push_error(
            &mut report,
            "KES-N002",
            "SPICE input contains no supported circuit components",
            None,
        );
    }
    if report.has_errors() {
        return report;
    }

    let dc_swept_sources = analyses
        .iter()
        .filter_map(|(_, fields)| {
            if fields.len() == 5 && fields[0].eq_ignore_ascii_case(".dc") {
                Some(fields[1].to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();

    if let Err((line, message)) =
        resolve_current_controlled_sources(&mut components, &dc_swept_sources, Some(&mut report))
    {
        push_error(&mut report, "KES-N002", message, Some(line));
        return report;
    }

    let (component_names, net_names) = match allocate_names(&components, &instances) {
        Ok(names) => names,
        Err((line, message)) => {
            push_error(&mut report, "KES-N004", message, Some(line));
            return report;
        }
    };
    for instance in &mut instances {
        instance.mapped_name = component_names[&instance.original_name].clone();
    }
    let mut translated_analyses = Vec::new();
    for (line, fields) in &analyses {
        match translate_analysis(
            fields,
            &component_names,
            &components,
            &parameter_lookup,
            &mut parameter_units,
        ) {
            Ok(analysis) => translated_analyses.push(analysis),
            Err(message) => push_error(&mut report, "KES-N002", message, Some(*line)),
        }
    }
    if report.has_errors() {
        return report;
    }
    report.names.extend(
        component_names
            .iter()
            .map(|(original, mapped)| ImportNameMapping {
                kind: ImportNameKind::Component,
                original: original.clone(),
                kessetsu: mapped.clone(),
            }),
    );
    for module in &subcircuits {
        report.names.push(ImportNameMapping {
            kind: ImportNameKind::Module,
            original: module.original_name.clone(),
            kessetsu: module.mapped_name.clone(),
        });
        report
            .names
            .extend(module.ports.iter().map(|port| ImportNameMapping {
                kind: ImportNameKind::ModulePort,
                original: format!("{}.{}", module.original_name, port.original_name),
                kessetsu: format!("{}.{}", module.mapped_name, port.mapped_name),
            }));
        let port_names = module
            .ports
            .iter()
            .map(|port| port.original_name.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        report.names.extend(
            module
                .node_names
                .iter()
                .filter(|(original, _)| !port_names.contains(&original.to_ascii_lowercase()))
                .map(|(original, mapped)| ImportNameMapping {
                    kind: ImportNameKind::Net,
                    original: format!("{}.{}", module.original_name, original),
                    kessetsu: format!("{}.{}", module.mapped_name, mapped),
                }),
        );
        report
            .names
            .extend(module.parameters.iter().map(|parameter| ImportNameMapping {
                kind: ImportNameKind::Parameter,
                original: format!("{}.{}", module.original_name, parameter.original_name),
                kessetsu: format!("{}.{}", module.mapped_name, parameter.mapped_name),
            }));
        report
            .names
            .extend(
                module
                    .component_names
                    .iter()
                    .map(|(original, mapped)| ImportNameMapping {
                        kind: ImportNameKind::Component,
                        original: format!("{}.{}", module.original_name, original),
                        kessetsu: format!("{}.{}", module.mapped_name, mapped),
                    }),
            );
    }
    report
        .names
        .extend(parameters.iter().map(|parameter| ImportNameMapping {
            kind: ImportNameKind::Parameter,
            original: parameter.original_name.clone(),
            kessetsu: parameter.mapped_name.clone(),
        }));
    report.names.extend(
        net_names
            .iter()
            .map(|(original, mapped)| ImportNameMapping {
                kind: ImportNameKind::Net,
                original: original.clone(),
                kessetsu: mapped.clone(),
            }),
    );

    let mut generated = String::from("// Imported from a supported SPICE subset by Kessetsu.\n");
    generated.push_str(&format!("// Original source: {}\n", report.source_sha256));
    for module in &subcircuits {
        generated.push_str(&render_subcircuit(module));
        generated.push('\n');
    }
    let mut declared_nets = net_names.values().cloned().collect::<Vec<_>>();
    declared_nets.sort();
    declared_nets.dedup();
    for net in &declared_nets {
        generated.push_str(&format!("net {net}\n"));
    }
    if !declared_nets.is_empty() {
        generated.push('\n');
    }

    let referenced_models = components
        .iter()
        .chain(
            subcircuits
                .iter()
                .flat_map(|module| module.components.iter()),
        )
        .filter_map(|component| match &component.kind {
            ImportedKind::Diode(model)
            | ImportedKind::Bjt { model, .. }
            | ImportedKind::Mosfet(model)
            | ImportedKind::Switch(model) => Some(model.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut emitted_models = false;
    for model in models.values() {
        if !referenced_models.contains(&model.mapped) {
            report.diagnostics.push(ImportDiagnostic {
                code: "KES-N012".into(),
                severity: ImportSeverity::Info,
                message: format!(
                    "inline model '{}' is not referenced by an imported device and was not emitted",
                    model.original
                ),
                line: None,
                column: None,
            });
            continue;
        }
        report.names.push(ImportNameMapping {
            kind: ImportNameKind::Model,
            original: model.original.clone(),
            kessetsu: model.mapped.clone(),
        });
        generated.push_str(&render_inline_model(model));
        generated.push('\n');
        emitted_models = true;
    }
    if emitted_models {
        generated.push('\n');
    }

    for parameter in &parameters {
        let Some(unit) = parameter_units.get(&parameter.original_name.to_ascii_lowercase()) else {
            report.diagnostics.push(ImportDiagnostic {
                code: "KES-N012".into(),
                severity: ImportSeverity::Info,
                message: format!(
                    "parameter '{}' is not directly referenced by an imported component and was omitted",
                    parameter.original_name
                ),
                line: Some(parameter.line),
                column: None,
            });
            continue;
        };
        generated.push_str(&format!(
            "param {}: {} = {}\n",
            parameter.mapped_name,
            unit.kessetsu_name(),
            parameter.value
        ));
    }
    if report.has_errors() {
        return report;
    }
    if !parameters.is_empty() {
        generated.push('\n');
    }

    for component in &components {
        let name = &component_names[&component.original_name];
        generated.push_str(&render_component_declaration(component, name));
        generated.push('\n');
    }
    for instance in &instances {
        generated.push_str(&format!(
            "use {} {}",
            instance.module_name, instance.mapped_name
        ));
        if !instance.overrides.is_empty() {
            generated.push('(');
            generated.push_str(
                &instance
                    .overrides
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            generated.push(')');
        }
        generated.push('\n');
    }
    generated.push('\n');
    for component in &components {
        let name = &component_names[&component.original_name];
        for (pin, node) in component.pins.iter().zip(&component.nodes) {
            generated.push_str(&format!("connect {name}.{pin} to {}\n", net_names[node]));
        }
    }
    for instance in &instances {
        let module = subcircuits
            .iter()
            .find(|module| module.mapped_name == instance.module_name)
            .expect("an imported instance references a known module");
        for (port, node) in module.ports.iter().zip(&instance.nodes) {
            generated.push_str(&format!(
                "connect {}.{} to {}\n",
                instance.mapped_name, port.mapped_name, net_names[node]
            ));
        }
    }

    for analysis in translated_analyses {
        generated.push_str(&analysis);
        generated.push('\n');
    }
    if report.has_errors() {
        return report;
    }

    let compile = compile_source(&generated, CompileOptions::default());
    report.compile_schema_version = Some(compile.schema_version.clone());
    for diagnostic in &compile.diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => ImportSeverity::Error,
            DiagnosticSeverity::Warning => ImportSeverity::Warning,
            DiagnosticSeverity::Info => ImportSeverity::Info,
        };
        let source_line = diagnostic.component.as_deref().and_then(|mapped| {
            imported_component_line(
                mapped,
                &components,
                &component_names,
                &instances,
                &subcircuits,
            )
        });
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-N005".into(),
            severity,
            message: format!(
                "canonical Kessetsu recompilation [{}]: {}",
                diagnostic.code, diagnostic.message
            ),
            line: source_line,
            column: None,
        });
    }
    report.summary = ImportSummary {
        components: components.len()
            + subcircuits
                .iter()
                .map(|module| module.components.len())
                .sum::<usize>(),
        nets: declared_nets.len(),
        analyses: analyses.len(),
        subcircuits: subcircuits.len(),
        instances: instances.len(),
    };
    if !report.has_errors() && compile.ir.is_some() && compile.spice_netlist.is_some() {
        report.kess_source = Some(generated);
    } else if !report.has_errors() {
        push_error(
            &mut report,
            "KES-N005",
            "canonical recompilation did not produce typed IR and SPICE output",
            None,
        );
    }
    report
}

fn imported_component_line(
    mapped: &str,
    components: &[ImportedComponent],
    component_names: &BTreeMap<String, String>,
    instances: &[ImportedInstance],
    modules: &[ImportedSubcircuit],
) -> Option<usize> {
    if let Some((original, _)) = component_names
        .iter()
        .find(|(_, candidate)| candidate.as_str() == mapped)
    {
        return components
            .iter()
            .find(|component| &component.original_name == original)
            .map(|component| component.line)
            .or_else(|| {
                instances
                    .iter()
                    .find(|instance| &instance.original_name == original)
                    .map(|instance| instance.line)
            });
    }
    for instance in instances {
        let Some(child) = mapped.strip_prefix(&format!("{}_", instance.mapped_name)) else {
            continue;
        };
        let module = modules
            .iter()
            .find(|module| module.mapped_name == instance.module_name)?;
        let original = module
            .component_names
            .iter()
            .find(|(_, candidate)| candidate.as_str() == child)
            .map(|(original, _)| original)?;
        return module
            .components
            .iter()
            .find(|component| &component.original_name == original)
            .map(|component| component.line);
    }
    None
}

fn partition_subcircuits(
    lines: Vec<LogicalLine>,
    report: &mut SpiceImportReport,
) -> (Vec<LogicalLine>, Vec<SubcircuitBlock>) {
    let mut root = Vec::new();
    let mut blocks = Vec::new();
    let mut current: Option<(LogicalLine, Vec<LogicalLine>)> = None;
    let mut root_ended = false;
    for line in lines {
        if root_ended && current.is_none() {
            root.push(line);
            continue;
        }
        let fields = split_fields(line.text.trim());
        let directive = fields.first().map(|field| field.to_ascii_lowercase());
        match directive.as_deref() {
            Some(".subckt") => {
                if current.is_some() {
                    push_error(
                        report,
                        "KES-N007",
                        "nested .SUBCKT definitions are not supported",
                        Some(line.number),
                    );
                } else {
                    current = Some((line, Vec::new()));
                }
            }
            Some(".ends") => {
                if let Some((header, body)) = current.take() {
                    blocks.push(SubcircuitBlock {
                        header,
                        body,
                        end: line,
                    });
                } else {
                    root.push(line);
                }
            }
            _ => {
                if let Some((_, body)) = &mut current {
                    body.push(line);
                } else {
                    root_ended = directive.as_deref() == Some(".end");
                    root.push(line);
                }
            }
        }
    }
    if let Some((header, _)) = current {
        push_error(
            report,
            "KES-N007",
            "unterminated .SUBCKT; a matching .ENDS is required",
            Some(header.number),
        );
    }
    (root, blocks)
}

fn parse_subcircuit(
    block: &SubcircuitBlock,
    module_names: &BTreeMap<String, (String, usize)>,
    models: &InlineModels,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Result<ImportedSubcircuit, (usize, String)> {
    let normalized_header = normalize_assignments(&block.header.text.replace(',', " "));
    let fields = split_fields(normalized_header.trim());
    let name = fields
        .get(1)
        .ok_or_else(|| {
            (
                block.header.number,
                ".SUBCKT requires a name and at least one interface node".into(),
            )
        })?
        .clone();
    let mapped_name = module_names[&name.to_ascii_lowercase()].0.clone();
    let marker = fields
        .iter()
        .position(|field| matches!(field.to_ascii_lowercase().as_str(), "params:" | "param:"));
    let first_assignment = fields
        .iter()
        .skip(2)
        .position(|field| field.contains('='))
        .map(|index| index + 2);
    let port_end = marker.or(first_assignment).unwrap_or(fields.len());
    if port_end <= 2 {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' requires at least one interface node"),
        ));
    }
    if fields[2..port_end].iter().any(|field| field.contains('=')) {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' parameter defaults must appear after all interface nodes"),
        ));
    }
    let end_fields = split_fields(block.end.text.trim());
    if end_fields.len() > 2 {
        return Err((
            block.end.number,
            ".ENDS accepts only an optional subcircuit name".into(),
        ));
    }
    if let Some(end_name) = end_fields.get(1)
        && !end_name.eq_ignore_ascii_case(&name)
    {
        return Err((
            block.end.number,
            format!(".ENDS name '{end_name}' does not match .SUBCKT '{name}'"),
        ));
    }

    let mut used = BTreeSet::new();
    let mut seen_ports = BTreeMap::<String, String>::new();
    let mut ports = Vec::new();
    for port in &fields[2..port_end] {
        let folded = port.to_ascii_lowercase();
        if let Some(previous) = seen_ports.insert(folded, port.clone()) {
            return Err((
                block.header.number,
                format!("subcircuit '{name}' ports '{previous}' and '{port}' differ only by case"),
            ));
        }
        ports.push(ImportedPort {
            original_name: port.clone(),
            mapped_name: unique_identifier(port, "port", &mut used),
        });
    }

    let assignments = if let Some(index) = marker {
        &fields[index + 1..]
    } else if let Some(index) = first_assignment {
        &fields[index..]
    } else {
        &[]
    };
    if marker.is_some() && assignments.is_empty() {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' has PARAMS: without any defaults"),
        ));
    }
    let mut env = BTreeMap::<String, f64>::new();
    let mut parameters =
        parse_parameter_assignments(assignments, block.header.number, &mut used, &mut env)?;
    for line in &block.body {
        let text = line.text.trim();
        let first_token = text.split_whitespace().next();
        if !first_token.is_some_and(|field| field.eq_ignore_ascii_case(".param")) {
            continue;
        }
        let rest = text[".param".len()..].replace(',', " ");
        let normalized = normalize_assignments(&rest);
        let param_fields = split_fields(normalized.trim());
        if param_fields.is_empty() {
            return Err((
                line.number,
                ".param requires at least one NAME=literal assignment".into(),
            ));
        }
        let extra = parse_parameter_assignments(&param_fields, line.number, &mut used, &mut env)?;
        for param in extra {
            if let Some(existing) = parameters.iter().find(|existing| {
                existing
                    .original_name
                    .eq_ignore_ascii_case(&param.original_name)
            }) {
                return Err((
                    line.number,
                    format!(
                        "parameter '{}' duplicates '{}' in subcircuit '{name}'",
                        param.original_name, existing.original_name
                    ),
                ));
            }
            parameters.push(param);
        }
    }
    let parameter_lookup = parameters
        .iter()
        .map(|parameter| (parameter.original_name.to_ascii_lowercase(), parameter))
        .collect::<BTreeMap<_, _>>();
    let mut parameter_units = BTreeMap::new();
    let mut components = Vec::new();
    for line in &block.body {
        let text = line.text.trim();
        if text.is_empty() || text.starts_with('*') || text.starts_with(';') {
            continue;
        }
        if text.starts_with('.') {
            let directive = split_fields(text)[0].to_ascii_lowercase();
            if directive == ".param" {
                continue;
            }
            return Err((
                line.number,
                format!(
                    "directive '{}' inside subcircuit '{name}' is outside the embedded topology subset",
                    split_fields(text)[0]
                ),
            ));
        }
        if text
            .chars()
            .next()
            .is_some_and(|character| character.eq_ignore_ascii_case(&'x'))
        {
            let empty_modules = BTreeMap::new();
            if let Some(opamp_result) =
                try_parse_builtin_opamp(text, line.number, &empty_modules, None)
            {
                components.push(opamp_result.map_err(|message| (line.number, message))?);
                continue;
            }
            return Err((
                line.number,
                format!(
                    "nested subcircuit instance '{}' is not supported yet; import stops instead of flattening hierarchy",
                    split_fields(text)[0]
                ),
            ));
        }
        components.push(
            parse_component(
                text,
                line.number,
                &parameter_lookup,
                &mut parameter_units,
                models,
                diagnostics,
            )
            .map_err(|message| (line.number, message))?,
        );
    }
    resolve_current_controlled_sources(&mut components, &BTreeSet::new(), None)?;
    if components.is_empty() {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' contains no supported components"),
        ));
    }
    for parameter in &parameters {
        if !parameter_units.contains_key(&parameter.original_name.to_ascii_lowercase()) {
            return Err((
                parameter.line,
                format!(
                    "subcircuit parameter '{}' is unused, so its electrical unit cannot be inferred",
                    parameter.original_name
                ),
            ));
        }
        canonical_number(&parameter.value).map_err(|message| {
            (
                parameter.line,
                format!(
                    "subcircuit parameter '{}' must be a literal value: {message}",
                    parameter.original_name
                ),
            )
        })?;
    }

    let mut component_names = BTreeMap::new();
    let mut component_seen = BTreeMap::<String, String>::new();
    for component in &components {
        let folded = component.original_name.to_ascii_lowercase();
        if let Some(previous) = component_seen.insert(folded, component.original_name.clone()) {
            return Err((
                component.line,
                format!(
                    "subcircuit component names '{previous}' and '{}' differ only by case",
                    component.original_name
                ),
            ));
        }
        component_names.insert(
            component.original_name.clone(),
            unique_identifier(&component.original_name, "C", &mut used),
        );
    }
    let port_lookup = ports
        .iter()
        .map(|port| {
            (
                port.original_name.to_ascii_lowercase(),
                port.mapped_name.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut node_groups = BTreeMap::<String, BTreeSet<String>>::new();
    for node in components.iter().flat_map(|component| &component.nodes) {
        node_groups
            .entry(node.to_ascii_lowercase())
            .or_default()
            .insert(node.clone());
    }
    let mut node_names = BTreeMap::new();
    let mut internal_nets = Vec::new();
    for (folded, spellings) in node_groups {
        let mapped = if let Some(port) = port_lookup.get(&folded) {
            port.clone()
        } else {
            if folded == "0" {
                return Err((
                    block.header.number,
                    format!(
                        "subcircuit '{name}' references global node 0 internally; expose ground as an explicit port"
                    ),
                ));
            }
            let representative = spellings
                .first()
                .expect("a node group always contains a spelling");
            let mapped = unique_identifier(representative, "N", &mut used);
            internal_nets.push(mapped.clone());
            mapped
        };
        for spelling in spellings {
            node_names.insert(spelling, mapped.clone());
        }
    }
    for port in &ports {
        if !node_names
            .keys()
            .any(|node| node.eq_ignore_ascii_case(&port.original_name))
        {
            return Err((
                block.header.number,
                format!(
                    "subcircuit port '{}' is not connected by any supported body component",
                    port.original_name
                ),
            ));
        }
    }
    internal_nets.sort();
    internal_nets.dedup();
    Ok(ImportedSubcircuit {
        original_name: name,
        mapped_name,
        ports,
        parameters,
        parameter_units,
        components,
        component_names,
        node_names,
        internal_nets,
    })
}

fn parse_parameter_assignments(
    assignments: &[String],
    line: usize,
    used: &mut BTreeSet<String>,
    env: &mut BTreeMap<String, f64>,
) -> Result<Vec<ImportedParameter>, (usize, String)> {
    let mut parameters = Vec::new();
    let mut seen = BTreeMap::<String, String>::new();
    for assignment in assignments {
        let Some((name, value)) = assignment.split_once('=') else {
            return Err((
                line,
                format!(
                    "parameter assignment '{assignment}' must use NAME=literal without spaces around '='"
                ),
            ));
        };
        if name.is_empty() || value.is_empty() || !is_spice_name(name) {
            return Err((
                line,
                format!("invalid SPICE parameter assignment '{assignment}'"),
            ));
        }
        let folded = name.to_ascii_lowercase();
        if let Some(previous) = seen.insert(folded.clone(), name.into()) {
            return Err((
                line,
                format!(
                    "parameter '{name}' duplicates '{previous}'; SPICE names are case-insensitive"
                ),
            ));
        }
        let evaluated = evaluate_spice_expr(value, env).map_err(|message| {
            (
                line,
                format!(
                    "parameter '{name}' must be a literal or constant arithmetic expression: {message}"
                ),
            )
        })?;
        env.insert(folded, evaluated);
        let normalized_val = if let Ok(exact) = canonical_number(value) {
            exact
        } else if evaluated == 0.0 {
            "0".into()
        } else {
            stable_scientific(evaluated)
        };
        parameters.push(ImportedParameter {
            original_name: name.into(),
            mapped_name: unique_identifier(name, "P", used),
            value: normalized_val,
            numeric_value: evaluated,
            line,
        });
    }
    Ok(parameters)
}

fn try_parse_builtin_opamp(
    text: &str,
    line: usize,
    modules: &BTreeMap<String, &ImportedSubcircuit>,
    report: Option<&mut SpiceImportReport>,
) -> Option<Result<ImportedComponent, String>> {
    let fields = split_fields(text);
    if fields.len() < 3 {
        return None;
    }
    let assignment_start = fields
        .iter()
        .position(|field| field.contains('=') || field.eq_ignore_ascii_case("params:"))
        .unwrap_or(fields.len());
    let module_index = assignment_start.saturating_sub(1);
    if module_index < 2 {
        return None;
    }
    let requested_module = &fields[module_index];
    if modules.contains_key(&requested_module.to_ascii_lowercase()) {
        return None;
    }
    let upper_module = requested_module.to_ascii_uppercase();
    if !matches!(
        upper_module.as_str(),
        "KESSETSU_OPAMP_V1" | "KESSETSU_OPAMP_V2" | "OPAMP"
    ) {
        return None;
    }
    if assignment_start < fields.len() {
        return Some(Err(format!(
            "built-in op-amp '{requested_module}' does not accept instance parameter overrides"
        )));
    }
    let nodes = fields[1..module_index].to_vec();
    if nodes.len() != 5 {
        return Some(Err(format!(
            "op-amp instance '{}' supplies {} nodes, but '{}' expects 5 terminals (in_p in_n vcc vee out)",
            fields[0],
            nodes.len(),
            requested_module
        )));
    }
    if requested_module.eq_ignore_ascii_case("OPAMP")
        && let Some(report) = report
    {
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-N012".into(),
            severity: ImportSeverity::Info,
            message: format!(
                "mapped generic 5-pin SPICE op-amp '{}' on '{}' to built-in model 'KESSETSU_OPAMP_V1' (pin order: in_p in_n vcc vee out)",
                requested_module, fields[0]
            ),
            line: Some(line),
            column: Some(1),
        });
    }
    let resolved_opamp = if upper_module == "KESSETSU_OPAMP_V2" {
        "KESSETSU_OPAMP_V2"
    } else {
        "KESSETSU_OPAMP_V1"
    };
    Some(Ok(ImportedComponent {
        original_name: fields[0].clone(),
        nodes,
        pins: &["in_p", "in_n", "vcc", "vee", "out"],
        kind: ImportedKind::OpAmp(resolved_opamp.into()),
        line,
    }))
}

fn parse_instance(
    text: &str,
    line: usize,
    modules: &BTreeMap<String, &ImportedSubcircuit>,
    root_parameters: &BTreeMap<String, &ImportedParameter>,
    root_parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<ImportedInstance, String> {
    let normalized = normalize_assignments(&text.replace(',', " "));
    let fields = split_fields(normalized.trim());
    if fields.len() < 4 {
        return Err("X expects an instance name, interface nodes and a subcircuit name".into());
    }
    let assignment_start = fields
        .iter()
        .position(|field| field.contains('=') || field.eq_ignore_ascii_case("params:"))
        .unwrap_or(fields.len());
    let module_index = assignment_start.saturating_sub(1);
    if module_index < 2 {
        return Err("X is missing interface nodes or a subcircuit name".into());
    }
    let requested_module = &fields[module_index];
    let module = modules
        .get(&requested_module.to_ascii_lowercase())
        .copied()
        .ok_or_else(|| {
            if fields[1..module_index].len() == 5 {
                format!(
                    "subcircuit instance '{}' references unknown embedded .SUBCKT '{}'; for a built-in 5-pin op-amp (in_p in_n vcc vee out), use model 'KESSETSU_OPAMP_V1' or 'OPAMP'",
                    fields[0], requested_module
                )
            } else {
                format!(
                    "subcircuit instance '{}' references unknown embedded .SUBCKT '{}'",
                    fields[0], requested_module
                )
            }
        })?;
    let nodes = fields[1..module_index].to_vec();
    if nodes.len() != module.ports.len() {
        return Err(format!(
            "subcircuit instance '{}' supplies {} nodes, but '{}' declares {} ports",
            fields[0],
            nodes.len(),
            module.original_name,
            module.ports.len()
        ));
    }
    let mut override_fields = &fields[assignment_start..];
    if override_fields
        .first()
        .is_some_and(|field| field.eq_ignore_ascii_case("params:"))
    {
        override_fields = &override_fields[1..];
    }
    let parameter_lookup = module
        .parameters
        .iter()
        .map(|parameter| (parameter.original_name.to_ascii_lowercase(), parameter))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut overrides = Vec::new();
    for assignment in override_fields {
        let Some((name, value)) = assignment.split_once('=') else {
            return Err(format!(
                "instance parameter '{assignment}' must use NAME=value without spaces"
            ));
        };
        let folded = name.to_ascii_lowercase();
        let parameter = parameter_lookup.get(&folded).copied().ok_or_else(|| {
            format!(
                "instance '{}' overrides unknown parameter '{}' on subcircuit '{}'",
                fields[0], name, module.original_name
            )
        })?;
        if !seen.insert(folded.clone()) {
            return Err(format!(
                "instance '{}' overrides parameter '{}' more than once",
                fields[0], name
            ));
        }
        let unit = module.parameter_units[&folded];
        overrides.push((
            parameter.mapped_name.clone(),
            translate_numeric(value, unit, root_parameters, root_parameter_units)?,
        ));
    }
    Ok(ImportedInstance {
        original_name: fields[0].clone(),
        mapped_name: String::new(),
        module_name: module.mapped_name.clone(),
        nodes,
        overrides,
        line,
    })
}

fn render_component_declaration(component: &ImportedComponent, name: &str) -> String {
    match &component.kind {
        ImportedKind::Resistor(value) => format!("resistor {name} {value}"),
        ImportedKind::Capacitor(value) => format!("capacitor {name} {value}"),
        ImportedKind::Inductor(value) => format!("inductor {name} {value}"),
        ImportedKind::VoltageSource(value) => format!("source {name} {value}"),
        ImportedKind::CurrentSource(value) => format!("current_source {name} {value}"),
        ImportedKind::Diode(model) => format!("diode {name} {model}"),
        ImportedKind::Bjt { polarity, model } => {
            format!("transistor {name} {polarity} {model}")
        }
        ImportedKind::Mosfet(model) => format!("mosfet {name} {model}"),
        ImportedKind::Switch(model) => format!("switch {name} {model}"),
        ImportedKind::OpAmp(model) => format!("opamp {name} {model}"),
        ImportedKind::Vcvs(value) => format!("vcvs {name} {value}"),
        ImportedKind::Vccs(value) => format!("vccs {name} {value}"),
        ImportedKind::Ccvs(value) => format!("ccvs {name} {value}"),
        ImportedKind::Cccs(value) => format!("cccs {name} {value}"),
    }
}

fn render_subcircuit(module: &ImportedSubcircuit) -> String {
    let mut generated = format!(
        "module {}({}) {{\n",
        module.mapped_name,
        module
            .ports
            .iter()
            .map(|port| port.mapped_name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    for parameter in &module.parameters {
        let unit = module.parameter_units[&parameter.original_name.to_ascii_lowercase()];
        generated.push_str(&format!(
            "  param {}: {} = {}\n",
            parameter.mapped_name,
            unit.kessetsu_name(),
            parameter.value
        ));
    }
    for net in &module.internal_nets {
        generated.push_str(&format!("  net {net}\n"));
    }
    for component in &module.components {
        let name = &module.component_names[&component.original_name];
        generated.push_str("  ");
        generated.push_str(&render_component_declaration(component, name));
        generated.push('\n');
    }
    for component in &module.components {
        let name = &module.component_names[&component.original_name];
        for (pin, node) in component.pins.iter().zip(&component.nodes) {
            generated.push_str(&format!(
                "  connect {name}.{pin} to {}\n",
                module.node_names[node]
            ));
        }
    }
    generated.push_str("}\n");
    generated
}

fn strip_inline_comment(raw: &str) -> &str {
    let trimmed = raw.trim_start();
    if trimmed.starts_with('*') || trimmed.starts_with(';') {
        return raw;
    }
    match raw.find(';') {
        Some(index) => raw[..index].trim_end(),
        None => raw,
    }
}

fn logical_lines(source: &str, report: &mut SpiceImportReport) -> Vec<LogicalLine> {
    let physical = source.lines().collect::<Vec<_>>();
    if physical.len() > MAX_IMPORT_LINES {
        push_error(
            report,
            "KES-N001",
            format!("SPICE input exceeds the {MAX_IMPORT_LINES} line limit"),
            None,
        );
        return Vec::new();
    }
    let mut logical: Vec<LogicalLine> = Vec::new();
    for (index, raw) in physical.into_iter().enumerate() {
        let cleaned = strip_inline_comment(raw);
        let trimmed = cleaned.trim_start();
        if let Some(continuation) = trimmed.strip_prefix('+') {
            let Some(previous) = logical.last_mut() else {
                push_error(
                    report,
                    "KES-N002",
                    "continuation line has no preceding statement",
                    Some(index + 1),
                );
                continue;
            };
            let continuation = continuation.trim();
            if !continuation.is_empty() {
                previous.text.push(' ');
                previous.text.push_str(continuation);
            }
        } else {
            logical.push(LogicalLine {
                number: index + 1,
                text: cleaned.to_string(),
            });
        }
    }
    logical
}

fn collect_parameters(
    lines: &[LogicalLine],
    report: &mut SpiceImportReport,
) -> Vec<ImportedParameter> {
    let mut parameters = Vec::new();
    let mut seen = BTreeMap::<String, (String, usize)>::new();
    let mut used = BTreeSet::new();
    let mut env = BTreeMap::<String, f64>::new();
    for line in lines {
        let trimmed = line.text.trim();
        let first_token = trimmed.split_whitespace().next();
        if first_token.is_none_or(|field| !field.eq_ignore_ascii_case(".param")) {
            continue;
        }
        let rest = trimmed[".param".len()..].replace(',', " ");
        let normalized = normalize_assignments(&rest);
        let fields = split_fields(normalized.trim());
        if fields.is_empty() {
            push_error(
                report,
                "KES-N006",
                ".param requires at least one NAME=literal assignment",
                Some(line.number),
            );
            continue;
        }
        for assignment in &fields {
            let Some((name, value)) = assignment.split_once('=') else {
                push_error(
                    report,
                    "KES-N006",
                    format!(
                        "parameter assignment '{assignment}' must use NAME=literal without spaces around '='"
                    ),
                    Some(line.number),
                );
                continue;
            };
            if name.is_empty() || value.is_empty() || !is_spice_name(name) {
                push_error(
                    report,
                    "KES-N006",
                    format!("invalid SPICE parameter assignment '{assignment}'"),
                    Some(line.number),
                );
                continue;
            }
            let folded = name.to_ascii_lowercase();
            if let Some((previous, previous_line)) = seen.get(&folded) {
                push_error(
                    report,
                    "KES-N006",
                    format!(
                        "parameter '{name}' duplicates '{previous}' from line {previous_line}; SPICE names are case-insensitive"
                    ),
                    Some(line.number),
                );
                continue;
            }
            seen.insert(folded.clone(), (name.into(), line.number));

            let evaluated = match evaluate_spice_expr(value, &env) {
                Ok(val) => val,
                Err(message) => {
                    push_error(
                        report,
                        "KES-N006",
                        format!(
                            "parameter '{name}' must be a literal or constant arithmetic expression: {message}"
                        ),
                        Some(line.number),
                    );
                    continue;
                }
            };
            env.insert(folded, evaluated);
            let normalized_val = if let Ok(exact) = canonical_number(value) {
                exact
            } else if evaluated == 0.0 {
                "0".into()
            } else {
                stable_scientific(evaluated)
            };
            parameters.push(ImportedParameter {
                original_name: name.into(),
                mapped_name: unique_identifier(name, "P", &mut used),
                value: normalized_val,
                numeric_value: evaluated,
                line: line.number,
            });
        }
    }
    parameters
}

fn is_spice_name(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_spice_symbol(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '$')
        })
}

fn parse_component(
    text: &str,
    line: usize,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
    models: &InlineModels,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Result<ImportedComponent, String> {
    let fields = split_fields(text);
    let name = fields
        .first()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "empty SPICE statement".to_string())?;
    let prefix = name.chars().next().unwrap().to_ascii_uppercase();
    let mut two_terminal = |kind: fn(String) -> ImportedKind,
                            unit: ImportUnit|
     -> Result<ImportedComponent, String> {
        let normalized = normalize_assignments(text);
        let passive_fields = split_fields(normalized.trim());
        if passive_fields.len() < 4 {
            return Err(format!(
                "{} expects name, two nodes and one literal value",
                prefix
            ));
        }
        if passive_fields.len() > 4 {
            let known_passive_params = [
                "tc", "tc1", "tc2", "tce", "temp", "dtemp", "m", "scale", "tol", "pwr", "ic",
            ];
            for extra in &passive_fields[4..] {
                let Some((k, v)) = extra.split_once('=') else {
                    return Err(format!(
                        "{} expects name, two nodes and one literal value",
                        prefix
                    ));
                };
                if v.is_empty() || !known_passive_params.contains(&k.to_ascii_lowercase().as_str())
                {
                    return Err(format!(
                        "{} expects name, two nodes and one literal value",
                        prefix
                    ));
                }
            }
            push_warning_diag(
                diagnostics,
                "KES-N011",
                format!(
                    "component '{name}': trailing SPICE instance parameters ({}) were ignored",
                    passive_fields[4..].join(", ")
                ),
                Some(line),
            );
        }
        let raw_val = &passive_fields[3];
        let stripped_val = raw_val
            .split_once('=')
            .filter(|(k, v)| {
                !v.is_empty()
                    && k.len() == 1
                    && k.chars().next().unwrap().eq_ignore_ascii_case(&prefix)
            })
            .map_or(raw_val.as_str(), |(_, v)| v);
        Ok(ImportedComponent {
            original_name: name.clone(),
            nodes: passive_fields[1..3].to_vec(),
            pins: &["p1", "p2"],
            kind: kind(translate_numeric(
                stripped_val,
                unit,
                parameters,
                parameter_units,
            )?),
            line,
        })
    };
    match prefix {
        'R' => two_terminal(ImportedKind::Resistor, ImportUnit::Ohm),
        'C' => two_terminal(ImportedKind::Capacitor, ImportUnit::Farad),
        'L' => two_terminal(ImportedKind::Inductor, ImportUnit::Henry),
        'V' | 'I' => {
            if fields.len() < 4 {
                return Err(format!("{prefix} source is missing its value or waveform"));
            }
            let value = translate_source(
                &fields[3..],
                if prefix == 'V' {
                    ImportUnit::Volt
                } else {
                    ImportUnit::Ampere
                },
                parameters,
                parameter_units,
            )?;
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: fields[1..3].to_vec(),
                pins: &["plus", "minus"],
                kind: if prefix == 'V' {
                    ImportedKind::VoltageSource(value)
                } else {
                    ImportedKind::CurrentSource(value)
                },
                line,
            })
        }
        'D' => {
            if fields.len() != 4 {
                return Err("D expects name, anode, cathode and model".into());
            }
            let model = match models.get(&fields[3].to_ascii_lowercase()) {
                Some(inline) if inline.kind == InlineModelKind::Diode => inline.mapped.clone(),
                Some(inline) => {
                    return Err(format!(
                        "diode '{name}' references non-diode model '{}'",
                        inline.original
                    ));
                }
                None => fields[3].clone(),
            };
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: fields[1..3].to_vec(),
                pins: &["p1", "p2"],
                kind: ImportedKind::Diode(model),
                line,
            })
        }
        'Q' => {
            let (nodes, model_field) = if fields.len() == 6 {
                let nsub = &fields[4];
                let ne = &fields[3];
                if nsub == "0" || nsub.eq_ignore_ascii_case("gnd") || nsub.eq_ignore_ascii_case(ne)
                {
                    push_warning_diag(
                        diagnostics,
                        "KES-N011",
                        format!(
                            "4-terminal BJT '{name}': substrate node '{nsub}' was folded into 3-terminal BJT"
                        ),
                        Some(line),
                    );
                    (fields[1..4].to_vec(), &fields[5])
                } else {
                    return Err(
                        "4-terminal BJT requires substrate node to be grounded (0/GND) or tied to emitter".into(),
                    );
                }
            } else if fields.len() == 5 {
                (fields[1..4].to_vec(), &fields[4])
            } else {
                return Err(
                    "Q expects name, collector, base, emitter and model; substrate nodes are not supported"
                        .into(),
                );
            };
            let (polarity, model) = match models.get(&model_field.to_ascii_lowercase()) {
                Some(InlineModel {
                    kind: InlineModelKind::Bjt(polarity),
                    mapped,
                    ..
                }) => (*polarity, mapped.clone()),
                Some(inline) => {
                    return Err(format!(
                        "transistor '{name}' references non-BJT model '{}'",
                        inline.original
                    ));
                }
                None => {
                    let model = model_field.clone();
                    let polarity = match model.to_ascii_uppercase().as_str() {
                        "2N3906" | "KESSETSU_POWER_PNP_V1" => "pnp",
                        "2N3904" | "2N2222" | "KESSETSU_POWER_NPN_V1" => "npn",
                        _ => {
                            return Err(format!(
                                "BJT model '{model}' is neither built in nor defined by an inline .model NPN/PNP card"
                            ));
                        }
                    };
                    (polarity, model)
                }
            };
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes,
                pins: &["c", "b", "e"],
                kind: ImportedKind::Bjt { polarity, model },
                line,
            })
        }
        'M' => {
            let normalized = normalize_assignments(&text.replace(',', " "));
            let fields = split_fields(normalized.trim());
            let param_start = fields
                .iter()
                .skip(1)
                .position(|f| f.contains('='))
                .map(|i| i + 1)
                .unwrap_or(fields.len());
            let core_fields = &fields[..param_start];
            let (nodes, model) = match core_fields.len() {
                5 => (core_fields[1..4].to_vec(), core_fields[4].clone()),
                6 if core_fields[3].eq_ignore_ascii_case(&core_fields[4]) => {
                    (core_fields[1..4].to_vec(), core_fields[5].clone())
                }
                6 => {
                    return Err(
                        "four-terminal MOSFET import requires bulk to be tied to source".into(),
                    );
                }
                _ => {
                    return Err(
                        "M expects drain, gate, source, optional tied bulk, and model".into(),
                    );
                }
            };
            if param_start < fields.len() {
                let known_mosfet_params = [
                    "w", "l", "ad", "as", "pd", "ps", "nrd", "nrs", "off", "ic", "temp", "dtemp",
                    "m",
                ];
                for extra in &fields[param_start..] {
                    let Some((k, v)) = extra.split_once('=') else {
                        return Err(format!("MOSFET '{name}': invalid parameter '{extra}'"));
                    };
                    if v.is_empty()
                        || !known_mosfet_params.contains(&k.to_ascii_lowercase().as_str())
                    {
                        return Err(format!(
                            "MOSFET '{name}': unsupported instance parameter '{k}'"
                        ));
                    }
                }
                push_warning_diag(
                    diagnostics,
                    "KES-N011",
                    format!(
                        "MOSFET '{name}': instance geometry/multiplier parameters ({}) were ignored",
                        fields[param_start..].join(", ")
                    ),
                    Some(line),
                );
            }
            let model = match models.get(&model.to_ascii_lowercase()) {
                Some(InlineModel {
                    kind: InlineModelKind::Mosfet(_),
                    mapped,
                    ..
                }) => mapped.clone(),
                Some(inline) => {
                    return Err(format!(
                        "MOSFET '{name}' references non-MOSFET model '{}'",
                        inline.original
                    ));
                }
                None if matches!(
                    model.to_ascii_uppercase().as_str(),
                    "IRF540" | "KESSETSU_PMOS_V1"
                ) =>
                {
                    model
                }
                None => {
                    return Err(format!(
                        "MOSFET model '{model}' is neither built in nor defined by an inline .model NMOS/PMOS card"
                    ));
                }
            };
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes,
                pins: &["d", "g", "s"],
                kind: ImportedKind::Mosfet(model),
                line,
            })
        }
        'E' | 'G' => {
            if fields.len() >= 4 {
                let field3_upper = fields[3].to_ascii_uppercase();
                if matches!(
                    field3_upper.as_str(),
                    "VALUE" | "POLY" | "TABLE" | "LAPLACE" | "FREQ"
                ) {
                    return Err(format!(
                        "behavioral source construct '{field3_upper}' is not supported"
                    ));
                }
            }
            if fields.len() != 6 {
                return Err(format!(
                    "{prefix} source expects name, two output nodes, two control nodes, and a gain"
                ));
            }
            let value = translate_numeric(
                &fields[5],
                if prefix == 'E' {
                    ImportUnit::Ratio
                } else {
                    ImportUnit::Siemens
                },
                parameters,
                parameter_units,
            )?;
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: vec![
                    fields[1].clone(),
                    fields[2].clone(),
                    fields[3].clone(),
                    fields[4].clone(),
                ],
                pins: &["p_out", "n_out", "ctrl_p", "ctrl_n"],
                kind: if prefix == 'E' {
                    ImportedKind::Vcvs(value)
                } else {
                    ImportedKind::Vccs(value)
                },
                line,
            })
        }
        'F' | 'H' => {
            if fields.len() >= 4 {
                let field3_upper = fields[3].to_ascii_uppercase();
                if matches!(field3_upper.as_str(), "POLY" | "VALUE") {
                    return Err(format!(
                        "polynomial or behavioral source '{field3_upper}' is not supported"
                    ));
                }
            }
            if fields.len() != 5 {
                return Err(format!(
                    "{prefix} source expects name, two output nodes, controlling voltage source, and a gain"
                ));
            }
            let value = translate_numeric(
                &fields[4],
                if prefix == 'F' {
                    ImportUnit::Ratio
                } else {
                    ImportUnit::Ohm
                },
                parameters,
                parameter_units,
            )?;
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: vec![fields[1].clone(), fields[2].clone(), fields[3].clone()],
                pins: &["p_out", "n_out", "ctrl_p", "ctrl_n"],
                kind: if prefix == 'F' {
                    ImportedKind::Cccs(value)
                } else {
                    ImportedKind::Ccvs(value)
                },
                line,
            })
        }
        'S' => {
            if fields.len() != 6
                && !(fields.len() == 7
                    && matches!(fields[6].to_ascii_uppercase().as_str(), "ON" | "OFF"))
            {
                return Err("S expects name, p1, p2, ctrl_p, ctrl_n and model".into());
            }
            let model = match models.get(&fields[5].to_ascii_lowercase()) {
                Some(InlineModel {
                    kind: InlineModelKind::Switch,
                    mapped,
                    ..
                }) => mapped.clone(),
                Some(inline) => {
                    return Err(format!(
                        "switch '{name}' references non-switch model '{}'",
                        inline.original
                    ));
                }
                None if fields[5].eq_ignore_ascii_case("KESSETSU_SW_V1") => {
                    "KESSETSU_SW_V1".to_string()
                }
                None => {
                    return Err(format!(
                        "switch model '{}' is neither built in nor defined by an inline .model SW card",
                        fields[5]
                    ));
                }
            };
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: fields[1..5].to_vec(),
                pins: &["p1", "p2", "ctrl_p", "ctrl_n"],
                kind: ImportedKind::Switch(model),
                line,
            })
        }
        'B' => Err(format!("behavioral source '{name}' is not supported")),
        'X' => Err(format!(
            "subcircuit instance '{name}' requires a typed model/resource binding"
        )),
        _ => Err(format!(
            "component family '{prefix}' is not supported by the declared import subset"
        )),
    }
}

fn resolve_current_controlled_sources(
    components: &mut Vec<ImportedComponent>,
    dc_swept_sources: &BTreeSet<String>,
    mut report: Option<&mut SpiceImportReport>,
) -> Result<(), (usize, String)> {
    let mut vnam_counts = BTreeMap::<String, usize>::new();
    for c in components.iter() {
        if matches!(c.kind, ImportedKind::Ccvs(_) | ImportedKind::Cccs(_)) && c.nodes.len() == 3 {
            *vnam_counts
                .entry(c.nodes[2].to_ascii_lowercase())
                .or_default() += 1;
        }
    }

    if vnam_counts.is_empty() {
        return Ok(());
    }

    let mut absorbed_voltages = BTreeSet::<String>::new();

    for i in 0..components.len() {
        if !matches!(
            components[i].kind,
            ImportedKind::Ccvs(_) | ImportedKind::Cccs(_)
        ) || components[i].nodes.len() != 3
        {
            continue;
        }

        let fh_line = components[i].line;
        let fh_name = components[i].original_name.clone();
        let vnam = components[i].nodes[2].clone();
        let vnam_lower = vnam.to_ascii_lowercase();

        let v_idx = components
            .iter()
            .position(|c| c.original_name.eq_ignore_ascii_case(&vnam));

        let Some(v_idx) = v_idx else {
            return Err((
                fh_line,
                format!(
                    "controlled source '{fh_name}' references '{vnam}' which is not an independent voltage source in the same scope"
                ),
            ));
        };

        let v_comp = &components[v_idx];
        let ImportedKind::VoltageSource(ref v_val) = v_comp.kind else {
            return Err((
                fh_line,
                format!(
                    "controlled source '{fh_name}' references '{vnam}' which is not an independent voltage source in the same scope"
                ),
            ));
        };

        let v_orig_name = v_comp.original_name.clone();
        let v_nodes = v_comp.nodes.clone();
        let count = *vnam_counts.get(&vnam_lower).unwrap_or(&0);
        let is_swept = dc_swept_sources.contains(&vnam_lower);
        let is_zero = v_val == "0";
        let is_shorted = v_nodes.len() >= 2 && v_nodes[0] == v_nodes[1];

        let p_out = components[i].nodes[0].clone();
        let n_out = components[i].nodes[1].clone();

        if is_zero && count == 1 && !is_swept && !is_shorted {
            components[i].nodes = vec![p_out, n_out, v_nodes[0].clone(), v_nodes[1].clone()];
            absorbed_voltages.insert(v_orig_name.to_ascii_lowercase());
            if let Some(ref mut rep) = report {
                rep.diagnostics.push(ImportDiagnostic {
                    code: "KES-N012".into(),
                    severity: ImportSeverity::Info,
                    message: format!(
                        "folded 0V current-sense source '{v_orig_name}' into controlled source '{fh_name}'"
                    ),
                    line: Some(fh_line),
                    column: Some(1),
                });
            }
        } else {
            let orig_plus = components[v_idx].nodes[0].clone();
            let sense_node = format!("sense_{}", fh_name);
            components[i].nodes = vec![p_out, n_out, orig_plus, sense_node.clone()];
            components[v_idx].nodes[0] = sense_node;
        }
    }

    if !absorbed_voltages.is_empty() {
        components.retain(|c| !absorbed_voltages.contains(&c.original_name.to_ascii_lowercase()));
    }

    Ok(())
}

fn translate_numeric(
    input: &str,
    unit: ImportUnit,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    let (candidate, braced) = input
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .map_or((input, false), |value| (value.trim(), true));
    if is_spice_name(candidate)
        && let Some(parameter) = parameters.get(&candidate.to_ascii_lowercase())
    {
        let key = parameter.original_name.to_ascii_lowercase();
        if let Some(previous) = parameter_units.insert(key.clone(), unit)
            && previous != unit
        {
            parameter_units.insert(key, previous);
            return Err(format!(
                "parameter '{}' is used as both {} and {}",
                parameter.original_name,
                previous.kessetsu_name(),
                unit.kessetsu_name()
            ));
        }
        return Ok(format!("{{{}}}", parameter.mapped_name));
    }
    if braced {
        let env: BTreeMap<String, f64> = parameters
            .iter()
            .map(|(k, p)| (k.clone(), p.numeric_value))
            .collect();
        let val = evaluate_spice_expr(candidate, &env)
            .map_err(|e| format!("SPICE expression '{{{candidate}}}': {e}"))?;
        return Ok(if val == 0.0 {
            "0".into()
        } else {
            stable_scientific(val)
        });
    }
    canonical_number(input)
}

fn normalize_source_fields(fields: &[String]) -> Vec<String> {
    let expanded = fields
        .iter()
        .flat_map(|field| {
            if let Some((prefix, rest)) = field.split_once('=')
                && matches!(prefix.to_ascii_lowercase().as_str(), "dc" | "ac")
                && !rest.is_empty()
            {
                vec![prefix.to_string(), rest.to_string()]
            } else {
                vec![field.clone()]
            }
        })
        .collect::<Vec<_>>();

    let mut normalized = Vec::new();
    let mut index = 0;
    while index < expanded.len() {
        let token = &expanded[index];
        let lower = token.to_ascii_lowercase();
        if matches!(lower.as_str(), "sin" | "sine" | "pulse" | "pwl") {
            if index + 1 < expanded.len() && expanded[index + 1].starts_with('(') {
                normalized.push(format!("{token}{}", expanded[index + 1]));
                index += 2;
                continue;
            }
            let mut args = Vec::new();
            index += 1;
            while index < expanded.len() {
                let candidate = &expanded[index];
                if matches!(candidate.to_ascii_lowercase().as_str(), "ac" | "dc")
                    || candidate.contains('(')
                {
                    break;
                }
                for part in candidate.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        args.push(trimmed.to_string());
                    }
                }
                index += 1;
            }
            normalized.push(format!("{token}({})", args.join(" ")));
            continue;
        }
        normalized.push(token.clone());
        index += 1;
    }
    normalized
}

fn translate_source(
    fields: &[String],
    value_unit: ImportUnit,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    let normalized = normalize_source_fields(fields);
    if normalized.len() == 1 {
        if let Some((name, args)) = parse_call(&normalized[0])? {
            return translate_waveform(name, args, value_unit, None, parameters, parameter_units);
        }
        return translate_numeric(&normalized[0], value_unit, parameters, parameter_units);
    }

    let mut dc_field: Option<&str> = None;
    let mut ac_mag: Option<&str> = None;
    let mut ac_phase: Option<&str> = None;
    let mut waveform_field: Option<&str> = None;
    let mut index = 0;
    while index < normalized.len() {
        let token = normalized[index].as_str();
        if token.eq_ignore_ascii_case("dc") {
            if dc_field.is_some() || index + 1 >= normalized.len() {
                return Err("source has unsupported DC/AC field combination".into());
            }
            dc_field = Some(normalized[index + 1].as_str());
            index += 2;
        } else if token.eq_ignore_ascii_case("ac") {
            if ac_mag.is_some() || index + 1 >= normalized.len() {
                return Err("AC source expects magnitude and optional zero-degree phase".into());
            }
            ac_mag = Some(normalized[index + 1].as_str());
            index += 2;
            if index < normalized.len() {
                let next = normalized[index].as_str();
                if !next.eq_ignore_ascii_case("dc")
                    && !next.eq_ignore_ascii_case("ac")
                    && !next.contains('(')
                {
                    if index + 1 < normalized.len()
                        && !normalized[index + 1].contains('(')
                        && !normalized[index + 1].eq_ignore_ascii_case("dc")
                    {
                        return Err(if dc_field.is_some() {
                            "AC source has unsupported trailing fields".into()
                        } else if waveform_field.is_some() {
                            "waveform AC suffix has unsupported trailing fields".into()
                        } else {
                            "AC source expects magnitude and optional zero-degree phase".into()
                        });
                    }
                    ac_phase = Some(next);
                    index += 1;
                }
            }
        } else if parse_call(token)?.is_some() {
            if waveform_field.is_some() {
                return Err("source specifies multiple waveforms".into());
            }
            waveform_field = Some(token);
            index += 1;
        } else {
            return Err(if dc_field.is_some() && ac_mag.is_none() {
                "source has unsupported DC/AC field combination".into()
            } else {
                "source value must be a literal, AC, SINE, PULSE or PWL form".into()
            });
        }
    }

    if let Some(phase) = ac_phase
        && canonical_number(phase)? != "0"
    {
        return Err("non-zero AC source phase is not representable in Kessetsu yet".into());
    }

    if let Some(wf) = waveform_field {
        if let Some(dc) = dc_field
            && canonical_number(dc)? != "0"
        {
            return Err("combined non-zero DC bias and waveform is not representable yet".into());
        }
        let (name, args) = parse_call(wf)?.expect("validated waveform call");
        let ac_translated = match ac_mag {
            Some(mag) => Some(translate_numeric(
                mag,
                value_unit,
                parameters,
                parameter_units,
            )?),
            None => None,
        };
        return translate_waveform(
            name,
            args,
            value_unit,
            ac_translated,
            parameters,
            parameter_units,
        );
    }

    if let Some(mag) = ac_mag {
        if let Some(dc) = dc_field
            && canonical_number(dc).ok().as_deref() != Some("0")
        {
            return Ok(format!(
                "ac({},{})",
                translate_numeric(mag, value_unit, parameters, parameter_units)?,
                translate_numeric(dc, value_unit, parameters, parameter_units)?
            ));
        }
        return Ok(format!(
            "ac({})",
            translate_numeric(mag, value_unit, parameters, parameter_units)?
        ));
    }

    if let Some(dc) = dc_field {
        return translate_numeric(dc, value_unit, parameters, parameter_units);
    }

    Err("source value must be a literal, AC, SINE, PULSE or PWL form".into())
}

fn parse_call(field: &str) -> Result<Option<(&str, Vec<&str>)>, String> {
    let Some(open) = field.find('(') else {
        return Ok(None);
    };
    if !field.ends_with(')') || open == 0 {
        return Err(format!("malformed source waveform '{field}'"));
    }
    let args = field[open + 1..field.len() - 1]
        .split([',', ' ', '\t'])
        .filter(|part| !part.is_empty())
        .collect();
    Ok(Some((&field[..open], args)))
}

fn translate_waveform(
    name: &str,
    mut args: Vec<&str>,
    value_unit: ImportUnit,
    ac: Option<String>,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    let waveform = name.to_ascii_lowercase();
    if matches!(waveform.as_str(), "sin" | "sine") && (4..=6).contains(&args.len()) {
        for extra in &args[3..] {
            if canonical_number(extra)? != "0" {
                return Err(
                    "non-zero SIN delay, damping or phase is not representable in Kessetsu yet"
                        .into(),
                );
            }
        }
        args.truncate(3);
    } else if waveform == "pulse" && args.len() == 8 {
        if canonical_number(args[7])? != "0" {
            return Err(
                "finite PULSE cycle count (Ncycles) is not representable in Kessetsu yet".into(),
            );
        }
        args.truncate(7);
    }
    let expected = match waveform.as_str() {
        "sin" | "sine" => 3,
        "pulse" => 7,
        "pwl" if args.len() >= 4 && args.len().is_multiple_of(2) => args.len(),
        "pwl" => return Err("PWL requires at least two time/value pairs".into()),
        _ => return Err(format!("unsupported source waveform '{name}'")),
    };
    if args.len() != expected {
        return Err(format!(
            "{} requires {expected} numeric fields, found {}",
            name.to_ascii_uppercase(),
            args.len()
        ));
    }
    let values = args
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let unit = match waveform.as_str() {
                "sin" | "sine" if index == 2 => ImportUnit::Hertz,
                "pulse" if index >= 2 => ImportUnit::Second,
                "pwl" if index.is_multiple_of(2) => ImportUnit::Second,
                _ => value_unit,
            };
            translate_numeric(value, unit, parameters, parameter_units)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let normalized = match waveform.as_str() {
        "sin" | "sine" if ac.is_some() => "sine_ac",
        "sin" | "sine" => "sine",
        "pulse" => "pulse",
        "pwl" => "pwl",
        _ => unreachable!(),
    };
    let mut joined = values.join(",");
    if let Some(ac) = ac {
        joined.push(',');
        joined.push_str(&ac);
    }
    Ok(format!("{normalized}({joined})"))
}

fn translate_analysis(
    fields: &[String],
    component_names: &BTreeMap<String, String>,
    imported_components: &[ImportedComponent],
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    match fields[0].to_ascii_lowercase().as_str() {
        ".op" if fields.len() == 1 => Ok("simulate op".into()),
        ".tran" => {
            let has_uic = fields
                .last()
                .is_some_and(|field| field.eq_ignore_ascii_case("uic"));
            let time_fields = if has_uic {
                &fields[1..fields.len() - 1]
            } else {
                &fields[1..]
            };
            let uic_suffix = if has_uic { " uic" } else { "" };
            match time_fields.len() {
                1 => {
                    let tstop = translate_numeric(
                        &time_fields[0],
                        ImportUnit::Second,
                        parameters,
                        parameter_units,
                    )?;
                    let tstop_val: f64 = tstop.parse().map_err(|_| {
                        "single-argument .tran requires a literal stop time so tstep can be inferred"
                            .to_string()
                    })?;
                    if tstop_val <= 0.0 {
                        return Err("TRAN stop time must be positive".into());
                    }
                    let tstep = stable_scientific(tstop_val / 200.0);
                    Ok(format!("simulate tran {tstep} {tstop}{uic_suffix}"))
                }
                2 => Ok(format!(
                    "simulate tran {} {}{uic_suffix}",
                    translate_numeric(
                        &time_fields[0],
                        ImportUnit::Second,
                        parameters,
                        parameter_units,
                    )?,
                    translate_numeric(
                        &time_fields[1],
                        ImportUnit::Second,
                        parameters,
                        parameter_units,
                    )?,
                )),
                3 | 4 => {
                    if canonical_number(&time_fields[2])? != "0" {
                        return Err(
                            "TRAN import supports tstep, tstop and optional UIC only (tstart must be 0)"
                                .into(),
                        );
                    }
                    let tstop = translate_numeric(
                        &time_fields[1],
                        ImportUnit::Second,
                        parameters,
                        parameter_units,
                    )?;
                    let raw_step = if canonical_number(&time_fields[0]).ok().as_deref() == Some("0")
                    {
                        if let Some(tmax) = time_fields.get(3)
                            && canonical_number(tmax).ok().as_deref() != Some("0")
                        {
                            translate_numeric(
                                tmax,
                                ImportUnit::Second,
                                parameters,
                                parameter_units,
                            )?
                        } else {
                            let tstop_val: f64 = tstop
                                .parse()
                                .map_err(|_| "TRAN stop time must be positive".to_string())?;
                            stable_scientific(tstop_val / 200.0)
                        }
                    } else {
                        translate_numeric(
                            &time_fields[0],
                            ImportUnit::Second,
                            parameters,
                            parameter_units,
                        )?
                    };
                    Ok(format!("simulate tran {raw_step} {tstop}{uic_suffix}"))
                }
                _ => Err("TRAN import supports tstep, tstop and optional UIC only".into()),
            }
        }
        ".ac" if fields.len() == 5 => {
            let scale = fields[1].to_ascii_lowercase();
            if !matches!(scale.as_str(), "dec" | "oct" | "lin") {
                return Err("AC scale must be DEC, OCT or LIN".into());
            }
            let points =
                translate_numeric(&fields[2], ImportUnit::Ratio, parameters, parameter_units)?;
            if !points.starts_with('{') && points.contains(['.', 'e', 'E']) {
                return Err("AC points must be a positive integer".into());
            }
            Ok(format!(
                "simulate ac {scale} {points} {} {}",
                translate_numeric(&fields[3], ImportUnit::Hertz, parameters, parameter_units,)?,
                translate_numeric(&fields[4], ImportUnit::Hertz, parameters, parameter_units,)?
            ))
        }
        ".ac" => Err("AC expects scale, points, start and stop".into()),
        ".dc" if fields.len() == 5 => {
            let mapped = component_names
                .get(&fields[1])
                .or_else(|| {
                    component_names
                        .iter()
                        .find(|(name, _)| name.eq_ignore_ascii_case(&fields[1]))
                        .map(|(_, mapped)| mapped)
                })
                .ok_or_else(|| format!("DC sweep source '{}' does not exist", fields[1]))?;
            let source_unit = imported_components
                .iter()
                .find(|component| component.original_name.eq_ignore_ascii_case(&fields[1]))
                .and_then(|component| match component.kind {
                    ImportedKind::VoltageSource(_) => Some(ImportUnit::Volt),
                    ImportedKind::CurrentSource(_) => Some(ImportUnit::Ampere),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!(
                        "DC sweep target '{}' is not an independent source",
                        fields[1]
                    )
                })?;
            Ok(format!(
                "simulate dc {mapped} {} {} {}",
                translate_numeric(&fields[2], source_unit, parameters, parameter_units)?,
                translate_numeric(&fields[3], source_unit, parameters, parameter_units)?,
                translate_numeric(&fields[4], source_unit, parameters, parameter_units)?
            ))
        }
        ".dc" => Err("DC expects source, start, stop and step".into()),
        _ => Err(format!(
            "unsupported analysis '{}'; import stopped",
            fields[0]
        )),
    }
}

fn allocate_names(
    components: &[ImportedComponent],
    instances: &[ImportedInstance],
) -> Result<ImportedNames, NameAllocationError> {
    let mut component_names = BTreeMap::new();
    let mut casefolded = BTreeMap::<String, (String, usize)>::new();
    let mut used = BTreeSet::new();
    for (original_name, line) in components
        .iter()
        .map(|component| (&component.original_name, component.line))
        .chain(
            instances
                .iter()
                .map(|instance| (&instance.original_name, instance.line)),
        )
    {
        let folded = original_name.to_ascii_lowercase();
        if let Some((previous, _)) = casefolded.get(&folded) {
            return Err((
                line,
                format!(
                    "SPICE component names '{}' and '{}' differ only by case",
                    previous, original_name
                ),
            ));
        }
        casefolded.insert(folded, (original_name.clone(), line));
        let mapped = unique_identifier(original_name, "C", &mut used);
        component_names.insert(original_name.clone(), mapped);
    }
    let mut node_groups = BTreeMap::<String, BTreeSet<String>>::new();
    for node in components
        .iter()
        .flat_map(|component| component.nodes.iter())
        .chain(instances.iter().flat_map(|instance| instance.nodes.iter()))
    {
        node_groups
            .entry(node.to_ascii_lowercase())
            .or_default()
            .insert(node.clone());
    }
    let mut net_names = BTreeMap::new();
    for (folded, spellings) in node_groups {
        let representative = spellings
            .first()
            .expect("a node group always contains a spelling");
        let mapped = if folded == "0" || folded == "gnd" {
            used.insert("GND".into());
            "GND".into()
        } else {
            unique_identifier(representative, "N", &mut used)
        };
        for spelling in spellings {
            net_names.insert(spelling, mapped.clone());
        }
    }
    Ok((component_names, net_names))
}

fn unique_identifier(original: &str, prefix: &str, used: &mut BTreeSet<String>) -> String {
    let mut candidate = String::new();
    for character in original.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            candidate.push(character);
        } else {
            candidate.push('_');
        }
    }
    if candidate.is_empty() || !candidate.starts_with(|c: char| c.is_ascii_alphabetic()) {
        candidate = format!("{prefix}_{candidate}");
    }
    if candidate.eq_ignore_ascii_case("GND") {
        candidate = format!("{prefix}_{candidate}");
    }
    let base = candidate.clone();
    let mut suffix = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn split_fields(text: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    for character in text.chars() {
        match character {
            '(' => {
                paren_depth += 1;
                current.push(character);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(character);
            }
            '{' => {
                brace_depth += 1;
                current.push(character);
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(character);
            }
            ' ' | '\t' if paren_depth == 0 && brace_depth == 0 => {
                if !current.is_empty() {
                    fields.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }
    if !current.is_empty() {
        fields.push(current);
    }
    fields
}

fn convert_rkm_to_standard(input: &str) -> Option<String> {
    let input = input.trim();
    let bytes = input.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let sign_end = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let int_end = i;
    if int_end == sign_end {
        return None;
    }

    let rem = &input[int_end..];
    let (multiplier_str, mult_len) = if rem.len() >= 3 && rem[..3].eq_ignore_ascii_case("meg") {
        ("meg", 3)
    } else if let Some(&c) = bytes.get(int_end) {
        let lower = c.to_ascii_lowercase();
        if lower == b'r' || lower == b'v' {
            ("", 1)
        } else if matches!(lower, b'k' | b'm' | b'u' | b'n' | b'p' | b'f' | b'g' | b't') {
            (&input[int_end..int_end + 1], 1)
        } else {
            return None;
        }
    } else {
        return None;
    };

    let fraction_start = int_end + mult_len;
    i = fraction_start;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let fraction_end = i;
    if fraction_end == fraction_start {
        return None;
    }

    let trailing = &input[fraction_end..];
    if !trailing.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    let sign = &input[..sign_end];
    let integer = &input[sign_end..int_end];
    let fraction = &input[fraction_start..fraction_end];
    Some(format!(
        "{sign}{integer}.{fraction}{multiplier_str}{trailing}"
    ))
}

fn canonical_number(input: &str) -> Result<String, String> {
    let input = input.trim();
    let rkm_converted;
    let input = if let Some(standard) = convert_rkm_to_standard(input) {
        rkm_converted = standard;
        rkm_converted.as_str()
    } else {
        input
    };
    let bytes = input.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let integer = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let mut digits = index > integer;
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let fraction = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        digits |= index > fraction;
    }
    if !digits {
        return Err(format!("'{input}' is not a literal SPICE number"));
    }
    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let exponent = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if exponent == index {
            return Err(format!("'{input}' has an invalid exponent"));
        }
    }
    let number: f64 = input[..index]
        .parse()
        .map_err(|_| format!("'{input}' is not a finite SPICE number"))?;
    let suffix = input[index..].to_ascii_lowercase();
    if !suffix
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return Err(format!(
            "'{input}' is not a literal SPICE number; expressions are not supported"
        ));
    }
    let factor = if suffix.starts_with("meg") {
        1e6
    } else if suffix.starts_with("mil") {
        25.4e-6
    } else {
        match suffix.chars().next() {
            Some('t') => 1e12,
            Some('g') => 1e9,
            Some('k') => 1e3,
            Some('m') => 1e-3,
            Some('u') => 1e-6,
            Some('n') => 1e-9,
            Some('p') => 1e-12,
            Some('f') => 1e-15,
            Some(character) if character.is_ascii_alphabetic() => 1.0,
            None => 1.0,
            _ => return Err(format!("'{input}' has an unsupported SPICE suffix")),
        }
    };
    let value = number * factor;
    if !value.is_finite() || (number != 0.0 && value == 0.0) {
        return Err(format!("'{input}' is outside the supported numeric range"));
    }
    Ok(if value == 0.0 {
        "0".into()
    } else if factor != 1.0 {
        stable_scientific(value)
    } else {
        value.to_string()
    })
}

fn stable_scientific(value: f64) -> String {
    let formatted = format!("{value:.14e}");
    let (mantissa, exponent) = formatted
        .split_once('e')
        .expect("scientific formatting contains an exponent");
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    let exponent: i32 = exponent.parse().expect("formatted exponent is an integer");
    format!("{mantissa}e{exponent}")
}

#[derive(Debug, Clone, PartialEq)]
enum ExprToken {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    StarStar,
    LParen,
    RParen,
    Comma,
}

fn tokenize_spice_expr(expr: &str) -> Result<Vec<ExprToken>, String> {
    let expr = expr.trim();
    let expr = expr
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(expr)
        .trim();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '+' {
            tokens.push(ExprToken::Plus);
            i += 1;
        } else if c == '-' {
            tokens.push(ExprToken::Minus);
            i += 1;
        } else if c == '*' {
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                tokens.push(ExprToken::StarStar);
                i += 2;
            } else {
                tokens.push(ExprToken::Star);
                i += 1;
            }
        } else if c == '/' {
            tokens.push(ExprToken::Slash);
            i += 1;
        } else if c == '^' {
            tokens.push(ExprToken::Caret);
            i += 1;
        } else if c == '(' {
            tokens.push(ExprToken::LParen);
            i += 1;
        } else if c == ')' {
            tokens.push(ExprToken::RParen);
            i += 1;
        } else if c == ',' {
            tokens.push(ExprToken::Comma);
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_ascii_alphanumeric()
                    || ch == '.'
                    || ((ch == '+' || ch == '-')
                        && i > start
                        && (chars[i - 1] == 'e' || chars[i - 1] == 'E'))
                {
                    i += 1;
                } else {
                    break;
                }
            }
            let token_str: String = chars[start..i].iter().collect();
            let canonical = canonical_number(&token_str)
                .map_err(|e| format!("invalid number '{token_str}': {e}"))?;
            let val: f64 = canonical
                .parse()
                .map_err(|_| format!("cannot parse number '{token_str}'"))?;
            tokens.push(ExprToken::Number(val));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            tokens.push(ExprToken::Ident(ident));
        } else {
            return Err(format!("unexpected character '{c}' in SPICE expression"));
        }
    }
    Ok(tokens)
}

struct ExprParser<'a> {
    tokens: &'a [ExprToken],
    pos: usize,
    env: &'a BTreeMap<String, f64>,
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&ExprToken> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<ExprToken> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                ExprToken::Plus => {
                    self.next();
                    let right = self.parse_term()?;
                    left += right;
                }
                ExprToken::Minus => {
                    self.next();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                ExprToken::Star => {
                    self.next();
                    let right = self.parse_power()?;
                    left *= right;
                }
                ExprToken::Slash => {
                    self.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return Err("division by zero in SPICE expression".into());
                    }
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        if let Some(tok) = self.peek()
            && matches!(tok, ExprToken::Caret | ExprToken::StarStar)
        {
            self.next();
            let exp = self.parse_power()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if let Some(tok) = self.peek() {
            if matches!(tok, ExprToken::Plus) {
                self.next();
                return self.parse_unary();
            } else if matches!(tok, ExprToken::Minus) {
                self.next();
                return Ok(-self.parse_unary()?);
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.next() {
            Some(ExprToken::Number(val)) => Ok(val),
            Some(ExprToken::LParen) => {
                let inner = self.parse_expr()?;
                match self.next().as_ref() {
                    Some(ExprToken::RParen) => Ok(inner),
                    _ => Err("unmatched '(' in SPICE expression".into()),
                }
            }
            Some(ExprToken::Ident(ident)) => {
                let ident_lower = ident.to_ascii_lowercase();
                if self.peek() == Some(&ExprToken::LParen) {
                    if ident_lower == "v" || ident_lower == "i" {
                        return Err(
                            "signal/time-dependent expression is not a constant SPICE parameter"
                                .into(),
                        );
                    }
                    self.next();
                    let arg1 = self.parse_expr()?;
                    if self.peek() == Some(&ExprToken::Comma) {
                        self.next();
                        let arg2 = self.parse_expr()?;
                        if self.next().as_ref() != Some(&ExprToken::RParen) {
                            return Err(format!("expected ')' closing call to '{ident}'"));
                        }
                        match ident_lower.as_str() {
                            "min" => Ok(arg1.min(arg2)),
                            "max" => Ok(arg1.max(arg2)),
                            "pow" => Ok(arg1.powf(arg2)),
                            _ => Err(format!(
                                "unknown 2-argument function '{ident}' in SPICE expression"
                            )),
                        }
                    } else {
                        if self.next().as_ref() != Some(&ExprToken::RParen) {
                            return Err(format!("expected ')' closing call to '{ident}'"));
                        }
                        match ident_lower.as_str() {
                            "sqrt" => {
                                if arg1 < 0.0 {
                                    return Err("sqrt of negative number".into());
                                }
                                Ok(arg1.sqrt())
                            }
                            "sin" => Ok(arg1.sin()),
                            "cos" => Ok(arg1.cos()),
                            "tan" => Ok(arg1.tan()),
                            "asin" => Ok(arg1.asin()),
                            "acos" => Ok(arg1.acos()),
                            "atan" => Ok(arg1.atan()),
                            "exp" => Ok(arg1.exp()),
                            "ln" | "log" => {
                                if arg1 <= 0.0 {
                                    return Err("log of non-positive number".into());
                                }
                                Ok(arg1.ln())
                            }
                            "log10" => {
                                if arg1 <= 0.0 {
                                    return Err("log10 of non-positive number".into());
                                }
                                Ok(arg1.log10())
                            }
                            "abs" => Ok(arg1.abs()),
                            _ => Err(format!("unknown function '{ident}' in SPICE expression")),
                        }
                    }
                } else {
                    if ident_lower == "time" {
                        return Err(
                            "signal/time-dependent expression is not a constant SPICE parameter"
                                .into(),
                        );
                    }
                    if ident_lower == "pi" {
                        return Ok(std::f64::consts::PI);
                    }
                    if ident_lower == "e" {
                        return Ok(std::f64::consts::E);
                    }
                    if let Some(&val) = self.env.get(&ident_lower) {
                        return Ok(val);
                    }
                    Err(format!(
                        "unknown parameter or unsupported symbol '{ident}' in SPICE expression"
                    ))
                }
            }
            Some(tok) => Err(format!("unexpected token '{:?}' in SPICE expression", tok)),
            None => Err("unexpected end of SPICE expression".into()),
        }
    }
}

pub fn evaluate_spice_expr(expr: &str, env: &BTreeMap<String, f64>) -> Result<f64, String> {
    let tokens = tokenize_spice_expr(expr)?;
    if tokens.is_empty() {
        return Err("empty SPICE expression".into());
    }
    let mut parser = ExprParser {
        tokens: &tokens,
        pos: 0,
        env,
    };
    let result = parser.parse_expr()?;
    if parser.pos < tokens.len() {
        return Err(format!(
            "trailing tokens in SPICE expression starting at {:?}",
            tokens[parser.pos]
        ));
    }
    if !result.is_finite() {
        return Err("SPICE expression result is not finite".into());
    }
    Ok(result)
}

fn push_unsupported(report: &mut SpiceImportReport, line: usize, message: &str) {
    push_error(
        report,
        "KES-N003",
        format!("Unsupported SPICE construct: {message}"),
        Some(line),
    );
}

fn push_title_info(report: &mut SpiceImportReport) {
    report.diagnostics.push(ImportDiagnostic {
        code: "KES-N010".into(),
        severity: ImportSeverity::Info,
        message: "first line is not a supported card and was treated as the SPICE title".into(),
        line: Some(1),
        column: Some(1),
    });
}

fn push_warning_diag(
    diagnostics: &mut Vec<ImportDiagnostic>,
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
) {
    diagnostics.push(ImportDiagnostic {
        code: code.into(),
        severity: ImportSeverity::Warning,
        message: message.into(),
        line,
        column: line.map(|_| 1),
    });
}

fn push_warning(
    report: &mut SpiceImportReport,
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
) {
    push_warning_diag(&mut report.diagnostics, code, message, line);
}

/// Collects root-level `.model` cards into typed inline models. Unsupported model families,
/// non-literal values and parameters outside the typed allowlist fail closed with the
/// original line number.
fn collect_models(lines: &[LogicalLine], report: &mut SpiceImportReport) -> InlineModels {
    let mut models = InlineModels::new();
    let mut used_names = BTreeSet::new();
    for line in lines {
        let text = line.text.trim();
        let mut tokens = text.split_whitespace();
        if !tokens
            .next()
            .is_some_and(|field| field.eq_ignore_ascii_case(".model"))
        {
            continue;
        }
        match parse_inline_model(text, &mut used_names) {
            Ok((model, dropped)) => {
                if !dropped.is_empty() {
                    report.diagnostics.push(ImportDiagnostic {
                        code: "KES-N012".into(),
                        severity: ImportSeverity::Info,
                        message: format!(
                            "model '{}': descriptive vendor fields without Ngspice electrical meaning were not copied: {}",
                            model.original,
                            dropped.join(", ")
                        ),
                        line: Some(line.number),
                        column: Some(1),
                    });
                }
                let key = model.original.to_ascii_lowercase();
                if models.insert(key, model).is_some() {
                    push_error(
                        report,
                        "KES-N012",
                        "duplicate .model name; SPICE names are case-insensitive",
                        Some(line.number),
                    );
                }
            }
            Err(message) => push_error(report, "KES-N012", message, Some(line.number)),
        }
    }
    models
}

fn parse_inline_model(
    text: &str,
    used_names: &mut BTreeSet<String>,
) -> Result<(InlineModel, Vec<String>), String> {
    let rest = text[".model".len()..].trim_start();
    let name_end = rest
        .find(|character: char| character.is_whitespace() || character == '(')
        .ok_or("'.model' requires a name and a model type")?;
    let original = rest[..name_end].to_string();
    let rest = rest[name_end..].trim_start();
    let type_end = rest
        .find(|character: char| !character.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let model_type = rest[..type_end].to_ascii_uppercase();
    let kind = match model_type.as_str() {
        "D" => InlineModelKind::Diode,
        "NPN" => InlineModelKind::Bjt("npn"),
        "PNP" => InlineModelKind::Bjt("pnp"),
        "NMOS" => InlineModelKind::Mosfet("nmos"),
        "PMOS" => InlineModelKind::Mosfet("pmos"),
        "SW" => InlineModelKind::Switch,
        "" => return Err(format!("model '{original}' has no model type")),
        other => {
            return Err(format!(
                "model '{original}' type '{other}' is outside the typed subset (D, NPN, PNP, NMOS, PMOS, SW)"
            ));
        }
    };
    let mut body = rest[type_end..].trim().to_string();
    if let Some(inner) = body.strip_prefix('(') {
        body = inner
            .strip_suffix(')')
            .ok_or_else(|| format!("model '{original}' has an unterminated parameter list"))?
            .to_string();
    }
    let allowed = match kind {
        InlineModelKind::Diode => crate::models::DIODE_MODEL_PARAMETERS,
        InlineModelKind::Bjt(_) => crate::models::BJT_MODEL_PARAMETERS,
        InlineModelKind::Mosfet(_) => crate::models::MOSFET_MODEL_PARAMETERS,
        InlineModelKind::Switch => crate::models::SWITCH_MODEL_PARAMETERS,
    };
    let mut parameters = Vec::new();
    let mut dropped = Vec::new();
    let mut seen = BTreeSet::new();
    for assignment in normalize_assignments(&body.replace(',', " ")).split_whitespace() {
        let (key, value) = assignment.split_once('=').ok_or_else(|| {
            format!("model '{original}' parameter '{assignment}' is not a NAME=value assignment")
        })?;
        let key = key.to_ascii_lowercase();
        if !allowed.contains(&key.as_str()) && DESCRIPTIVE_MODEL_FIELDS.contains(&key.as_str()) {
            dropped.push(key);
            continue;
        }
        if key == "level" {
            if canonical_number(value).ok().as_deref() == Some("1") {
                continue;
            }
            return Err(format!(
                "model '{original}' uses level={value}; only Ngspice level 1 cards are importable"
            ));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "model '{original}' parameter '{key}' is outside the typed Ngspice allowlist: {}",
                allowed.join(", ")
            ));
        }
        let value = canonical_number(value)
            .map_err(|message| format!("model '{original}' parameter '{key}': {message}"))?;
        if !seen.insert(key.clone()) {
            return Err(format!("model '{original}' repeats parameter '{key}'"));
        }
        parameters.push((key, value));
    }
    if parameters.is_empty() {
        return Err(format!(
            "model '{original}' has no electrical parameters to import"
        ));
    }
    let valid_identifier = original
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
        && original
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    let builtin = BUILTIN_MODEL_NAMES
        .iter()
        .any(|builtin| builtin.eq_ignore_ascii_case(&original));
    let mapped =
        if valid_identifier && !builtin && !used_names.contains(&original.to_ascii_lowercase()) {
            original.clone()
        } else {
            let sanitized = original
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            let mut candidate = format!("SPICE_{sanitized}");
            let mut suffix = 2;
            while used_names.contains(&candidate.to_ascii_lowercase()) {
                candidate = format!("SPICE_{sanitized}_{suffix}");
                suffix += 1;
            }
            candidate
        };
    used_names.insert(mapped.to_ascii_lowercase());
    Ok((
        InlineModel {
            original,
            mapped,
            kind,
            parameters,
        },
        dropped,
    ))
}

/// Removes whitespace around `=` so `IS = 1e-14` and `IS=1e-14` tokenize identically,
/// while preserving expressions inside `{...}`.
fn normalize_assignments(text: &str) -> String {
    let characters = text.chars().collect::<Vec<_>>();
    let mut normalized = String::with_capacity(text.len());
    let mut brace_depth = 0usize;
    for (index, character) in characters.iter().enumerate() {
        if *character == '{' {
            brace_depth += 1;
            normalized.push('{');
        } else if *character == '}' {
            brace_depth = brace_depth.saturating_sub(1);
            normalized.push('}');
        } else if brace_depth > 0 {
            normalized.push(*character);
        } else if character.is_whitespace() {
            let previous = normalized.chars().last();
            let next = characters[index + 1..]
                .iter()
                .find(|candidate| !candidate.is_whitespace());
            if previous.is_none_or(|value| value == '=' || value == ' ') || next == Some(&'=') {
                continue;
            }
            normalized.push(' ');
        } else {
            normalized.push(*character);
        }
    }
    normalized
}

fn render_inline_model(model: &InlineModel) -> String {
    let (kind, polarity) = match model.kind {
        InlineModelKind::Diode => ("diode", String::new()),
        InlineModelKind::Bjt(polarity) => ("bjt", format!(" {polarity}")),
        InlineModelKind::Mosfet(polarity) => ("mosfet", format!(" {polarity}")),
        InlineModelKind::Switch => ("switch", String::new()),
    };
    let parameters = model
        .parameters
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "model {kind} {}{polarity} version=0.0.0 license=unspecified source=spice-import {parameters}",
        model.mapped
    )
}

fn push_error(
    report: &mut SpiceImportReport,
    code: &str,
    message: impl Into<String>,
    line: Option<usize>,
) {
    report.diagnostics.push(ImportDiagnostic {
        code: code.into(),
        severity: ImportSeverity::Error,
        message: message.into(),
        line,
        column: line.map(|_| 1),
    });
}
