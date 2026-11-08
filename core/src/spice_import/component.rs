use std::collections::{BTreeMap, BTreeSet};

use super::expr::{canonical_number, stable_scientific, translate_numeric};
use super::models::{InlineModel, InlineModelKind, InlineModels, normalize_assignments};
use super::types::{
    ImportDiagnostic, ImportSeverity, ImportUnit, ImportedComponent, ImportedCoupling,
    ImportedInstance, ImportedKind, ImportedNames, ImportedParameter, ImportedSubcircuit,
    LogicalLine, MAX_IMPORT_LINES, NameAllocationError, SpiceImportReport, push_error,
    push_warning_diag,
};

pub(crate) fn imported_component_line(
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

pub(crate) fn strip_inline_comment(raw: &str) -> &str {
    let trimmed = raw.trim_start();
    if trimmed.starts_with('*') || trimmed.starts_with(';') {
        return raw;
    }
    match raw.find(';') {
        Some(index) => raw[..index].trim_end(),
        None => raw,
    }
}

pub(crate) fn logical_lines(source: &str, report: &mut SpiceImportReport) -> Vec<LogicalLine> {
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

pub(crate) fn is_spice_name(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

pub(crate) fn is_spice_symbol(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '$')
        })
}

pub(crate) fn parse_component(
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
            let (nodes, model, pins): (Vec<String>, String, &'static [&'static str]) =
                match core_fields.len() {
                    5 => (
                        core_fields[1..4].to_vec(),
                        core_fields[4].clone(),
                        &["d", "g", "s"],
                    ),
                    6 if core_fields[3].eq_ignore_ascii_case(&core_fields[4]) => (
                        core_fields[1..4].to_vec(),
                        core_fields[5].clone(),
                        &["d", "g", "s"],
                    ),
                    6 => (
                        core_fields[1..5].to_vec(),
                        core_fields[5].clone(),
                        &["d", "g", "s", "b"],
                    ),
                    _ => {
                        return Err(
                            "M expects drain, gate, source, optional bulk, and model".into()
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
                pins,
                kind: ImportedKind::Mosfet(model),
                line,
            })
        }
        'J' => {
            let normalized = normalize_assignments(&text.replace(',', " "));
            let fields = split_fields(normalized.trim());
            let param_start = fields
                .iter()
                .skip(1)
                .position(|f| f.contains('='))
                .map(|i| i + 1)
                .unwrap_or(fields.len());
            let core_fields = &fields[..param_start];
            let (nodes, model, pins): (Vec<String>, String, &'static [&'static str]) =
                match core_fields.len() {
                    5 => (
                        core_fields[1..4].to_vec(),
                        core_fields[4].clone(),
                        &["d", "g", "s"],
                    ),
                    _ => {
                        return Err("J expects drain, gate, source, and model".into());
                    }
                };
            if param_start < fields.len() {
                let known_jfet_params = ["area", "off", "ic", "temp", "m"];
                for extra in &fields[param_start..] {
                    let Some((k, v)) = extra.split_once('=') else {
                        return Err(format!("JFET '{name}': invalid parameter '{extra}'"));
                    };
                    if v.is_empty() || !known_jfet_params.contains(&k.to_ascii_lowercase().as_str())
                    {
                        return Err(format!(
                            "JFET '{name}': unsupported instance parameter '{k}'"
                        ));
                    }
                }
                push_warning_diag(
                    diagnostics,
                    "KES-N011",
                    format!(
                        "JFET '{name}': instance parameters ({}) were ignored",
                        fields[param_start..].join(", ")
                    ),
                    Some(line),
                );
            }
            let model = match models.get(&model.to_ascii_lowercase()) {
                Some(InlineModel {
                    kind: InlineModelKind::Jfet(_),
                    mapped,
                    ..
                }) => mapped.clone(),
                Some(inline) => {
                    return Err(format!(
                        "JFET '{name}' references non-JFET model '{}'",
                        inline.original
                    ));
                }
                None if matches!(
                    model.to_ascii_uppercase().as_str(),
                    "2N5457" | "J2N5457" | "2N5460"
                ) =>
                {
                    model
                }
                None => {
                    return Err(format!(
                        "JFET model '{model}' is neither built in nor defined by an inline .model NJF/PJF card"
                    ));
                }
            };
            Ok(ImportedComponent {
                original_name: name.clone(),
                nodes,
                pins,
                kind: ImportedKind::Jfet(model),
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

pub(crate) fn parse_coupling(
    text: &str,
    line: usize,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<ImportedCoupling, String> {
    let fields = split_fields(text);
    if fields.len() < 4 {
        return Err(
            "coupling statement 'K' requires name, at least 2 inductors, and a coupling factor"
                .into(),
        );
    }
    let original_name = fields[0].clone();
    let name = original_name.clone();
    let inductors = fields[1..fields.len() - 1].to_vec();
    for ind in &inductors {
        if !ind.starts_with(['L', 'l']) {
            return Err(format!("coupling element '{ind}' is not an inductor"));
        }
    }
    let factor_raw = &fields[fields.len() - 1];
    let factor = translate_numeric(factor_raw, ImportUnit::Ratio, parameters, parameter_units)?;
    if let Ok(val) = factor.parse::<f64>()
        && !(0.0 < val && val <= 1.0)
    {
        return Err(format!("coupling factor must be in (0, 1], got {val}"));
    }
    Ok(ImportedCoupling {
        original_name,
        name,
        inductors,
        factor,
        line,
    })
}

pub(crate) fn resolve_current_controlled_sources(
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

pub(crate) fn normalize_source_fields(fields: &[String]) -> Vec<String> {
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
        if matches!(lower.as_str(), "sin" | "sine" | "pulse" | "exp" | "pwl") {
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

pub(crate) fn translate_source(
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
                "source value must be a literal, AC, SINE, PULSE, EXP or PWL form".into()
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

    Err("source value must be a literal, AC, SINE, PULSE, EXP or PWL form".into())
}

pub(crate) fn parse_call(field: &str) -> Result<Option<(&str, Vec<&str>)>, String> {
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

pub(crate) fn translate_waveform(
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
        "exp" => 6,
        "pwl" if args.len() >= 4 && args.len().is_multiple_of(2) => args.len(),
        "pwl" => return Err("PWL requires at least two time/value pairs".into()),
        _ => return Err(format!("unsupported source waveform '{name}'")),
    };
    if args.len() != expected {
        if waveform == "exp" {
            return Err("EXP source requires 6 arguments: v1, v2, td1, tau1, td2, tau2".into());
        }
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
                "exp" if index >= 2 => ImportUnit::Second,
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
        "exp" => "exp",
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

pub(crate) fn translate_analysis(
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

pub(crate) fn allocate_names(
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

pub(crate) fn unique_identifier(
    original: &str,
    prefix: &str,
    used: &mut BTreeSet<String>,
) -> String {
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

pub(crate) fn split_fields(text: &str) -> Vec<String> {
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

pub(crate) fn render_component_declaration(component: &ImportedComponent, name: &str) -> String {
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
        ImportedKind::Jfet(model) => format!("jfet {name} {model}"),
        ImportedKind::Switch(model) => format!("switch {name} {model}"),
        ImportedKind::OpAmp(model) => format!("opamp {name} {model}"),
        ImportedKind::Vcvs(value) => format!("vcvs {name} {value}"),
        ImportedKind::Vccs(value) => format!("vccs {name} {value}"),
        ImportedKind::Ccvs(value) => format!("ccvs {name} {value}"),
        ImportedKind::Cccs(value) => format!("cccs {name} {value}"),
    }
}
