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
use expr::{collect_parameters, translate_numeric};
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
    let mut initial_conditions = Vec::<(usize, String)>::new();
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
                ".ic" => {
                    initial_conditions.push((line.number, text.to_string()));
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
    let mut seen_ic_nets = BTreeSet::<String>::new();
    let mut translated_ics = Vec::new();
    for (line, raw_text) in &initial_conditions {
        match parse_ic_entries(raw_text) {
            Ok(entries) => {
                for (node, raw_value) in entries {
                    let Some(mapped_net) = net_names
                        .get(&node)
                        .or_else(|| {
                            net_names
                                .iter()
                                .find(|(orig, _)| orig.eq_ignore_ascii_case(&node))
                                .map(|(_, mapped)| mapped)
                        })
                        .cloned()
                    else {
                        push_error(
                            &mut report,
                            "KES-N002",
                            format!(".ic references unknown node '{node}'"),
                            Some(*line),
                        );
                        continue;
                    };
                    if mapped_net == "GND" {
                        push_error(
                            &mut report,
                            "KES-N002",
                            format!(".ic cannot set initial voltage on ground node '{node}'"),
                            Some(*line),
                        );
                        continue;
                    }
                    if !seen_ic_nets.insert(mapped_net.to_ascii_lowercase()) {
                        push_error(
                            &mut report,
                            "KES-N002",
                            format!("duplicate .ic initial condition for node '{node}'"),
                            Some(*line),
                        );
                        continue;
                    }
                    match translate_numeric(
                        &raw_value,
                        types::ImportUnit::Volt,
                        &parameter_lookup,
                        &mut parameter_units,
                    ) {
                        Ok(value) => {
                            translated_ics.push(format!("initial V({mapped_net}) = {value}"));
                        }
                        Err(message) => {
                            push_error(&mut report, "KES-N002", message, Some(*line));
                        }
                    }
                }
            }
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

    for ic in translated_ics {
        generated.push_str(&ic);
        generated.push('\n');
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

fn parse_ic_entries(text: &str) -> Result<Vec<(String, String)>, String> {
    let trimmed = text.trim();
    let after_directive = trimmed
        .get(..3)
        .filter(|prefix| prefix.eq_ignore_ascii_case(".ic"))
        .map(|_| trimmed[3..].trim())
        .ok_or_else(|| "invalid .ic directive".to_string())?;
    if after_directive.is_empty() {
        return Err(".ic requires at least one V(node)=voltage initial condition".into());
    }
    let mut entries = Vec::new();
    let mut pos = 0usize;
    let len = after_directive.len();
    while pos < len {
        while pos < len {
            let ch = after_directive[pos..].chars().next().unwrap();
            if ch.is_whitespace() || ch == ',' {
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if pos >= len {
            break;
        }
        let target_start = pos;
        let first_ch = after_directive[pos..].chars().next().unwrap();
        if !first_ch.eq_ignore_ascii_case(&'v') {
            if first_ch.eq_ignore_ascii_case(&'i')
                && after_directive[pos + first_ch.len_utf8()..]
                    .trim_start()
                    .starts_with('(')
            {
                return Err(
                    "inductor current initial conditions in .ic are not supported; only node voltages V(node)=value are supported"
                        .into(),
                );
            }
            return Err(format!(
                "invalid .ic entry starting at '{}'; expected V(node)=value",
                &after_directive[target_start..]
            ));
        }
        pos += first_ch.len_utf8();
        while pos < len {
            let ch = after_directive[pos..].chars().next().unwrap();
            if ch.is_whitespace() {
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if !after_directive[pos..].starts_with('(') {
            return Err(format!(
                "invalid .ic entry starting at '{}'; expected V(node)=value",
                &after_directive[target_start..]
            ));
        }
        pos += 1;
        let node_start = pos;
        let Some(close_rel) = after_directive[pos..].find(')') else {
            return Err(format!(
                "unterminated V(node) target in .ic starting at '{}'",
                &after_directive[target_start..]
            ));
        };
        let node = after_directive[node_start..pos + close_rel].trim();
        if node.is_empty()
            || node.contains(['(', ')', '=', ','])
            || node.chars().any(char::is_whitespace)
        {
            return Err(format!(
                "invalid node target in .ic entry '{}'",
                &after_directive[target_start..=pos + close_rel]
            ));
        }
        pos += close_rel + 1;
        while pos < len {
            let ch = after_directive[pos..].chars().next().unwrap();
            if ch.is_whitespace() {
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if !after_directive[pos..].starts_with('=') {
            return Err(format!(
                ".ic target 'V({node})' must be followed by '=value'"
            ));
        }
        pos += 1;
        while pos < len {
            let ch = after_directive[pos..].chars().next().unwrap();
            if ch.is_whitespace() {
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if pos >= len {
            return Err(format!(".ic entry 'V({node})=' is missing a voltage value"));
        }
        let value = if after_directive[pos..].starts_with('{') {
            let val_start = pos;
            let mut brace_depth = 0usize;
            let mut found_end = None;
            for (offset, ch) in after_directive[pos..].char_indices() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            found_end = Some(pos + offset + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(val_end) = found_end else {
                return Err(format!("unterminated expression in .ic entry 'V({node})='"));
            };
            pos = val_end;
            &after_directive[val_start..val_end]
        } else {
            let val_start = pos;
            while pos < len {
                let ch = after_directive[pos..].chars().next().unwrap();
                if ch.is_whitespace() || ch == ',' {
                    break;
                }
                pos += ch.len_utf8();
            }
            let val = &after_directive[val_start..pos];
            if val.is_empty() {
                return Err(format!(".ic entry 'V({node})=' is missing a voltage value"));
            }
            val
        };
        entries.push((node.to_string(), value.to_string()));
    }
    if entries.is_empty() {
        return Err(".ic requires at least one V(node)=voltage initial condition".into());
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exporter::{ExportFormat, ExportOptions, export_report};

    #[test]
    fn imports_classic_phase6_builtin_models_and_renames_inline_collisions() {
        let builtin_spice = "* Phase 6 classic semiconductors
V1 vcc 0 12
D1 vcc n1 1N5819
R1 n1 0 1k
Q1 n2 n1 0 BC547
R2 vcc n2 1k
Q2 n3 n2 vcc BC557
R3 n3 0 1k
M1 n4 n3 0 0 2N7000
R4 vcc n4 1k
M2 n5 n4 vcc vcc BSS84
R5 n5 0 1k
J1 n6 0 n7 J201
R6 n7 0 1k
R7 vcc n6 1k
.op
.end
";
        let report = import_spice(builtin_spice);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let kess = report.kess_source.expect("generated .kess source");
        assert!(kess.contains("diode D1 1N5819"), "{kess}");
        assert!(kess.contains("transistor Q1 npn BC547"), "{kess}");
        assert!(kess.contains("transistor Q2 pnp BC557"), "{kess}");
        assert!(kess.contains("mosfet M1 2N7000"), "{kess}");
        assert!(kess.contains("mosfet M2 BSS84"), "{kess}");
        assert!(kess.contains("jfet J1 J201"), "{kess}");

        let compiled = compile_source(&kess, CompileOptions::default());
        assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
        let spice = compiled.spice_netlist.expect("SPICE netlist");
        for expected in [
            ".model 1N5819 D (",
            ".model BC547 NPN (",
            ".model BC557 PNP (",
            ".model 2N7000 NMOS (",
            ".model BSS84 PMOS (",
            ".model J201 NJF (",
            "@J_J1[id]",
        ] {
            assert!(
                spice.contains(expected),
                "missing '{expected}' in:\n{spice}"
            );
        }

        let collision_spice = "* Inline overrides of newly added built-ins must rename safely
V1 vcc 0 5
D1 vcc n1 1N5819
R1 n1 0 1k
Q1 n2 n1 0 BC547
R2 vcc n2 1k
M1 n3 n2 0 0 2N7000
R3 vcc n3 1k
J1 n4 0 0 J201
R4 vcc n4 1k
.model 1N5819 D (IS=1e-5 N=1.1)
.model BC547 NPN (IS=2e-14 BF=250)
.model 2N7000 NMOS (VTO=1.8 KP=0.2)
.model J201 NJF (VTO=-1.2 BETA=1.5m)
.op
.end
";
        let collision = import_spice(collision_spice);
        assert!(!collision.has_errors(), "{:?}", collision.diagnostics);
        let collision_kess = collision.kess_source.expect("collision .kess");
        assert!(
            collision_kess.contains("model diode SPICE_1N5819"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("diode D1 SPICE_1N5819"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("model bjt SPICE_BC547 npn"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("transistor Q1 npn SPICE_BC547"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("model mosfet SPICE_2N7000 nmos"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("mosfet M1 SPICE_2N7000"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("model jfet SPICE_J201 njf"),
            "{collision_kess}"
        );
        assert!(
            collision_kess.contains("jfet J1 SPICE_J201"),
            "{collision_kess}"
        );
    }

    #[test]
    fn imports_ic_directives_into_kess_spice_and_ltspice() {
        let input = "* RC transient with multiple .ic formats
.param VINIT=1.25
V1 in 0 5
R1 in OUT 1k
C1 OUT 0 100n
R2 OUT n2 1k
C2 n2 0 100n
.ic v(out) = {VINIT}, V( n2 ) = -500mV
.tran 10u 2m uic
.end
";
        let report = import_spice(input);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|d| d.severity == ImportSeverity::Warning && d.message.contains(".ic")),
            "unexpected warning for .ic: {:?}",
            report.diagnostics
        );
        let kess = report.kess_source.expect("generated .kess source");
        assert!(kess.contains("param VINIT: V = 1.25"), "{kess}");
        assert!(kess.contains("initial V(OUT) = {VINIT}"), "{kess}");
        assert!(kess.contains("initial V(n2) = -5e-1"), "{kess}");

        let compiled = compile_source(&kess, CompileOptions::all_outputs());
        assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
        let spice = compiled.spice_netlist.as_ref().expect("SPICE netlist");
        let ic_pos = spice.find(".ic V(out)=1.25").expect("missing .ic V(out)");
        let n2_pos = spice.find(".ic V(n2)=-0.5").expect("missing .ic V(n2)");
        let ctrl_pos = spice.find("\n.control\n").expect("missing .control");
        assert!(ic_pos < ctrl_pos && n2_pos < ctrl_pos, "{spice}");

        let ltspice = export_report(&compiled, ExportFormat::Ltspice, ExportOptions::default())
            .expect("LTspice export");
        let asc = String::from_utf8(ltspice.bytes).expect("UTF-8 .asc");
        assert!(asc.contains("!.ic V(OUT)="), "{asc}");
        assert!(asc.contains("!.ic V(n2)="), "{asc}");

        let space_separated_ic = "* Space-separated .ic pairs
V1 in 0 5
R1 in a 1k
C1 a 0 100n
R2 a b 1k
C2 b 0 100n
.ic V(a)=1.0 V(b)=2.0
.tran 10u 1m uic
.end
";
        let space_report = import_spice(space_separated_ic);
        assert!(!space_report.has_errors(), "{:?}", space_report.diagnostics);
        let space_kess = space_report.kess_source.expect("space-separated .ic .kess");
        assert!(space_kess.contains("initial V(a) = 1\n"), "{space_kess}");
        assert!(space_kess.contains("initial V(b) = 2\n"), "{space_kess}");
    }

    #[test]
    fn rejects_invalid_ic_directives_with_line_numbers() {
        for (ic_line, expected_substr) in [
            (".ic", "at least one"),
            (".ic V(missing)=1.0", "unknown node"),
            (".ic V(0)=0", "ground node"),
            (".ic V(out)=1.0 V(OUT)=2.0", "duplicate"),
            (".ic I(L1)=10m", "inductor current"),
            (".ic V(out)=", "missing a voltage value"),
        ] {
            let input = format!(
                "* Invalid .ic test\nV1 in 0 5\nR1 in out 1k\nL1 out 0 1m\n{ic_line}\n.tran 1u 1m\n.end\n"
            );
            let report = import_spice(&input);
            assert!(
                report.has_errors(),
                "expected error for '{ic_line}', got: {:?}",
                report.diagnostics
            );
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|d| d.severity == ImportSeverity::Error
                        && d.line == Some(5)
                        && d.message.contains(expected_substr)),
                "expected '{expected_substr}' on line 5 for '{ic_line}', got: {:?}",
                report.diagnostics
            );
        }
    }
}
