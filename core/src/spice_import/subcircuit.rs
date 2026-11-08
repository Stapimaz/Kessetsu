use std::collections::{BTreeMap, BTreeSet};

use super::component::{
    parse_component, render_component_declaration, resolve_current_controlled_sources,
    split_fields, unique_identifier,
};
use super::expr::{canonical_number, parse_parameter_assignments, translate_numeric};
use super::models::{InlineModels, normalize_assignments};
use super::types::{
    ImportDiagnostic, ImportSeverity, ImportUnit, ImportedComponent, ImportedInstance,
    ImportedKind, ImportedParameter, ImportedPort, ImportedSubcircuit, LogicalLine,
    SpiceImportReport, SubcircuitBlock, push_error,
};

pub(crate) fn partition_subcircuits(
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

pub(crate) fn parse_subcircuit(
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

pub(crate) fn try_parse_builtin_opamp(
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

pub(crate) fn parse_instance(
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

pub(crate) fn render_subcircuit(module: &ImportedSubcircuit) -> String {
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
