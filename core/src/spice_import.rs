use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::compiler::{CompileOptions, DiagnosticSeverity, compile_source};

mod component;
mod expr;
mod models;
mod subcircuit;
mod types;

use component::{
    allocate_names, imported_component_line, is_spice_symbol, logical_lines, parse_component,
    parse_coupling, render_component_declaration, resolve_current_controlled_sources, split_fields,
    translate_analysis, unique_identifier,
};
use expr::collect_parameters;
use models::{collect_models, render_inline_model};
use subcircuit::{
    parse_instance, parse_subcircuit, partition_subcircuits, render_subcircuit,
    try_parse_builtin_opamp,
};
use types::{
    ImportedCoupling, ImportedKind, MAX_IMPORT_BYTES, OUTPUT_DIRECTIVES, push_error,
    push_title_info, push_unsupported, push_warning,
};

pub use expr::evaluate_spice_expr;
pub use types::{
    ImportDiagnostic, ImportNameKind, ImportNameMapping, ImportSeverity, ImportSummary,
    SPICE_IMPORT_SCHEMA_VERSION, SpiceImportReport,
};

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

    let mut parameter_units = BTreeMap::<String, types::ImportUnit>::new();
    let mut components = Vec::new();
    let mut instances = Vec::new();
    let mut couplings = Vec::<ImportedCoupling>::new();
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
        } else if text
            .chars()
            .next()
            .is_some_and(|character| character.eq_ignore_ascii_case(&'k'))
        {
            match parse_coupling(text, line.number, &parameter_lookup, &mut parameter_units) {
                Ok(coupling) => couplings.push(coupling),
                Err(_) if is_first_line => push_title_info(&mut report),
                Err(message) => push_error(&mut report, "KES-N002", message, Some(line.number)),
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
    report
        .names
        .extend(couplings.iter().map(|coupling| ImportNameMapping {
            kind: ImportNameKind::Component,
            original: coupling.original_name.clone(),
            kessetsu: coupling.name.clone(),
        }));
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
            | ImportedKind::Jfet(model)
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
    for coupling in &couplings {
        let mapped_inductors = coupling
            .inductors
            .iter()
            .map(|ind| {
                component_names
                    .iter()
                    .find(|(orig, _)| orig.eq_ignore_ascii_case(ind))
                    .map(|(_, mapped)| mapped.clone())
                    .unwrap_or_else(|| ind.clone())
            })
            .collect::<Vec<_>>()
            .join(" ");
        generated.push_str(&format!(
            "couple {} {} {}\n",
            coupling.name, mapped_inductors, coupling.factor
        ));
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
