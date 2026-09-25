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
        }
    }
}

type ImportedNames = (BTreeMap<String, String>, BTreeMap<String, String>);
type NameAllocationError = (usize, String);

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
    let mut module_identifiers = BTreeSet::new();
    let mut module_names = BTreeMap::<String, (String, usize)>::new();
    for block in &blocks {
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
    }
    if report.has_errors() {
        return report;
    }
    let mut subcircuits = Vec::new();
    for block in &blocks {
        match parse_subcircuit(block, &module_names) {
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

    for line in root_lines {
        let text = line.text.trim();
        if text.is_empty() || text.starts_with('*') || text.starts_with(';') {
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
                ".param" => {}
                ".control" | ".endc" => push_unsupported(
                    &mut report,
                    line.number,
                    "simulator control blocks are not executed or partially imported",
                ),
                ".include" | ".lib" => push_unsupported(
                    &mut report,
                    line.number,
                    "model libraries require an explicit resource binding; arbitrary paths are not imported",
                ),
                ".model" | ".ends" => push_unsupported(
                    &mut report,
                    line.number,
                    "inline model definitions and unmatched .ENDS directives are not importable",
                ),
                _ => push_unsupported(
                    &mut report,
                    line.number,
                    &format!("directive '{}' is not supported", fields[0]),
                ),
            }
            continue;
        }

        if text
            .chars()
            .next()
            .is_some_and(|character| character.eq_ignore_ascii_case(&'x'))
        {
            match parse_instance(
                text,
                line.number,
                &module_lookup,
                &parameter_lookup,
                &mut parameter_units,
            ) {
                Ok(instance) => instances.push(instance),
                Err(message) => push_error(&mut report, "KES-N007", message, Some(line.number)),
            }
        } else {
            match parse_component(text, line.number, &parameter_lookup, &mut parameter_units) {
                Ok(component) => components.push(component),
                Err(message) => push_error(&mut report, "KES-N002", message, Some(line.number)),
            }
        }
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

    for parameter in &parameters {
        let Some(unit) = parameter_units.get(&parameter.original_name.to_ascii_lowercase()) else {
            push_error(
                &mut report,
                "KES-N006",
                format!(
                    "parameter '{}' is unused, so its electrical unit cannot be inferred",
                    parameter.original_name
                ),
                Some(parameter.line),
            );
            continue;
        };
        let value = match canonical_number(&parameter.value) {
            Ok(value) => value,
            Err(message) => {
                push_error(
                    &mut report,
                    "KES-N006",
                    format!(
                        "parameter '{}' must be a literal value in the supported subset: {message}",
                        parameter.original_name
                    ),
                    Some(parameter.line),
                );
                continue;
            }
        };
        generated.push_str(&format!(
            "param {}: {} = {}\n",
            parameter.mapped_name,
            unit.kessetsu_name(),
            value
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
) -> Result<ImportedSubcircuit, (usize, String)> {
    let fields = split_fields(block.header.text.trim());
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
    let port_end = marker.unwrap_or(fields.len());
    if port_end <= 2 {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' requires at least one interface node"),
        ));
    }
    if fields[2..port_end].iter().any(|field| field.contains('=')) {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' parameter defaults require an explicit PARAMS: marker"),
        ));
    }
    if marker.is_none() && fields[2..].iter().any(|field| field.contains('=')) {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' parameter defaults require an explicit PARAMS: marker"),
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

    let assignments = marker.map(|index| &fields[index + 1..]).unwrap_or(&[]);
    if marker.is_some() && assignments.is_empty() {
        return Err((
            block.header.number,
            format!("subcircuit '{name}' has PARAMS: without any defaults"),
        ));
    }
    let parameters = parse_parameter_assignments(assignments, block.header.number, &mut used)?;
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
            return Err((
                line.number,
                format!(
                    "nested subcircuit instance '{}' is not supported yet; import stops instead of flattening hierarchy",
                    split_fields(text)[0]
                ),
            ));
        }
        components.push(
            parse_component(text, line.number, &parameter_lookup, &mut parameter_units)
                .map_err(|message| (line.number, message))?,
        );
    }
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
        if let Some(previous) = seen.insert(folded, name.into()) {
            return Err((
                line,
                format!(
                    "parameter '{name}' duplicates '{previous}'; SPICE names are case-insensitive"
                ),
            ));
        }
        parameters.push(ImportedParameter {
            original_name: name.into(),
            mapped_name: unique_identifier(name, "P", used),
            value: value.into(),
            line,
        });
    }
    Ok(parameters)
}

fn parse_instance(
    text: &str,
    line: usize,
    modules: &BTreeMap<String, &ImportedSubcircuit>,
    root_parameters: &BTreeMap<String, &ImportedParameter>,
    root_parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<ImportedInstance, String> {
    let fields = split_fields(text);
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
            format!(
                "subcircuit instance '{}' references unknown embedded .SUBCKT '{}'",
                fields[0], requested_module
            )
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
        let value = canonical_number(&parameter.value)
            .expect("subcircuit parameter literals are validated before rendering");
        generated.push_str(&format!(
            "  param {}: {} = {}\n",
            parameter.mapped_name,
            unit.kessetsu_name(),
            value
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
        let trimmed = raw.trim_start();
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
            previous.text.push(' ');
            previous.text.push_str(continuation.trim());
        } else {
            logical.push(LogicalLine {
                number: index + 1,
                text: raw.to_string(),
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
    for line in lines {
        let fields = split_fields(line.text.trim());
        if fields
            .first()
            .is_none_or(|field| !field.eq_ignore_ascii_case(".param"))
        {
            continue;
        }
        if fields.len() < 2 {
            push_error(
                report,
                "KES-N006",
                ".param requires at least one NAME=literal assignment",
                Some(line.number),
            );
            continue;
        }
        for assignment in &fields[1..] {
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
            seen.insert(folded, (name.into(), line.number));
            parameters.push(ImportedParameter {
                original_name: name.into(),
                mapped_name: unique_identifier(name, "P", &mut used),
                value: value.into(),
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
) -> Result<ImportedComponent, String> {
    let fields = split_fields(text);
    let name = fields
        .first()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "empty SPICE statement".to_string())?;
    let prefix = name.chars().next().unwrap().to_ascii_uppercase();
    let mut two_terminal =
        |kind: fn(String) -> ImportedKind, unit: ImportUnit| -> Result<ImportedComponent, String> {
            if fields.len() != 4 {
                return Err(format!(
                    "{} expects name, two nodes and one literal value",
                    prefix
                ));
            }
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: fields[1..3].to_vec(),
                pins: &["p1", "p2"],
                kind: kind(translate_numeric(
                    &fields[3],
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
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: fields[1..3].to_vec(),
                pins: &["p1", "p2"],
                kind: ImportedKind::Diode(fields[3].clone()),
                line,
            })
        }
        'Q' => {
            if fields.len() != 5 {
                return Err("Q expects name, collector, base, emitter and model; substrate nodes are not supported".into());
            }
            let model = fields[4].clone();
            let polarity = match model.to_ascii_uppercase().as_str() {
                "2N3906" | "KESSETSU_POWER_PNP_V1" => "pnp",
                "2N3904" | "2N2222" | "KESSETSU_POWER_NPN_V1" => "npn",
                _ => {
                    return Err(format!(
                        "BJT model '{model}' has no typed polarity in the supported import subset"
                    ));
                }
            };
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes: fields[1..4].to_vec(),
                pins: &["c", "b", "e"],
                kind: ImportedKind::Bjt { polarity, model },
                line,
            })
        }
        'M' => {
            let (nodes, model) = match fields.len() {
                5 => (fields[1..4].to_vec(), fields[4].clone()),
                6 if fields[3].eq_ignore_ascii_case(&fields[4]) => {
                    (fields[1..4].to_vec(), fields[5].clone())
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
            if !matches!(
                model.to_ascii_uppercase().as_str(),
                "IRF540" | "KESSETSU_PMOS_V1"
            ) {
                return Err(format!(
                    "MOSFET model '{model}' is outside the supported typed import subset"
                ));
            }
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes,
                pins: &["d", "g", "s"],
                kind: ImportedKind::Mosfet(model),
                line,
            })
        }
        'E' | 'F' | 'G' | 'H' | 'B' => Err(format!(
            "controlled/behavioral source '{name}' is not supported"
        )),
        'X' => Err(format!(
            "subcircuit instance '{name}' requires a typed model/resource binding"
        )),
        _ => Err(format!(
            "component family '{prefix}' is not supported by the declared import subset"
        )),
    }
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
        return Err(format!(
            "SPICE expression '{{{candidate}}}' is outside the literal parameter subset"
        ));
    }
    canonical_number(input)
}

fn translate_source(
    fields: &[String],
    value_unit: ImportUnit,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    if fields.len() == 1 {
        if let Some((name, args)) = parse_call(&fields[0])? {
            return translate_waveform(name, args, value_unit, None, parameters, parameter_units);
        }
        return translate_numeric(&fields[0], value_unit, parameters, parameter_units);
    }
    if fields[0].eq_ignore_ascii_case("ac") {
        if fields.len() > 3 {
            return Err("AC source expects magnitude and optional zero-degree phase".into());
        }
        if fields.len() == 3 && canonical_number(&fields[2])? != "0" {
            return Err("non-zero AC source phase is not representable in Kessetsu yet".into());
        }
        return Ok(format!(
            "ac({})",
            translate_numeric(&fields[1], value_unit, parameters, parameter_units)?
        ));
    }
    if fields[0].eq_ignore_ascii_case("dc") {
        if fields.len() == 2 {
            return translate_numeric(&fields[1], value_unit, parameters, parameter_units);
        }
        if fields.len() >= 4 && fields[2].eq_ignore_ascii_case("ac") {
            if canonical_number(&fields[1])? != "0" {
                return Err(
                    "combined non-zero DC bias and AC magnitude is not representable yet".into(),
                );
            }
            if fields.len() > 5 {
                return Err("AC source has unsupported trailing fields".into());
            }
            if fields.len() == 5 && canonical_number(&fields[4])? != "0" {
                return Err("non-zero AC source phase is not representable in Kessetsu yet".into());
            }
            return Ok(format!(
                "ac({})",
                translate_numeric(&fields[3], value_unit, parameters, parameter_units)?
            ));
        }
        return Err("source has unsupported DC/AC field combination".into());
    }
    if let Some((name, args)) = parse_call(&fields[0])? {
        if fields.len() == 1 {
            return translate_waveform(name, args, value_unit, None, parameters, parameter_units);
        }
        if fields.len() >= 3 && fields[1].eq_ignore_ascii_case("ac") {
            if fields.len() > 4 {
                return Err("waveform AC suffix has unsupported trailing fields".into());
            }
            if fields.len() == 4 && canonical_number(&fields[3])? != "0" {
                return Err("non-zero AC source phase is not representable in Kessetsu yet".into());
            }
            return translate_waveform(
                name,
                args,
                value_unit,
                Some(translate_numeric(
                    &fields[2],
                    value_unit,
                    parameters,
                    parameter_units,
                )?),
                parameters,
                parameter_units,
            );
        }
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
    args: Vec<&str>,
    value_unit: ImportUnit,
    ac: Option<String>,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    let expected = match name.to_ascii_lowercase().as_str() {
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
    let waveform = name.to_ascii_lowercase();
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
    let normalized = match name.to_ascii_lowercase().as_str() {
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
        ".tran"
            if matches!(fields.len(), 3 | 4)
                && (fields.len() == 3 || fields[3].eq_ignore_ascii_case("uic")) =>
        {
            Ok(format!(
                "simulate tran {} {}{}",
                translate_numeric(&fields[1], ImportUnit::Second, parameters, parameter_units,)?,
                translate_numeric(&fields[2], ImportUnit::Second, parameters, parameter_units,)?,
                if fields.len() == 4 { " uic" } else { "" }
            ))
        }
        ".tran" => Err("TRAN import supports tstep, tstop and optional UIC only".into()),
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
        let mapped = if folded == "0" {
            if used.contains("GND") {
                return Err((
                    1,
                    "the imported namespace reserves GND for SPICE node 0".into(),
                ));
            }
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
    let mut depth = 0usize;
    for character in text.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ' ' | '\t' if depth == 0 => {
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

fn canonical_number(input: &str) -> Result<String, String> {
    let input = input.trim();
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

fn push_unsupported(report: &mut SpiceImportReport, line: usize, message: &str) {
    push_error(
        report,
        "KES-N003",
        format!("Unsupported SPICE construct: {message}"),
        Some(line),
    );
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
