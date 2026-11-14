use std::collections::{BTreeMap, BTreeSet};

use super::expr::canonical_number;
use super::types::{ImportDiagnostic, ImportSeverity, LogicalLine, SpiceImportReport, push_error};

/// Built-in Kessetsu model names. Inline SPICE models with these names are renamed so the
/// user's parameters are never silently replaced by the built-in card.
pub(crate) const BUILTIN_MODEL_NAMES: &[&str] = &[
    "2N3904",
    "2N3906",
    "2N2222",
    "BC547",
    "BC557",
    "1N4148",
    "1N4007",
    "1N5819",
    "1N4733A",
    "BZX84C3V3",
    "BZX84C5V1",
    "BZX84C12",
    "IRF540",
    "2N7000",
    "BSS84",
    "2N5457",
    "J2N5457",
    "2N5460",
    "J201",
    "KESSETSU_POWER_NPN_V1",
    "KESSETSU_POWER_PNP_V1",
    "KESSETSU_PMOS_V1",
    "KESSETSU_SW_V1",
    "KESSETSU_OPAMP_V1",
    "KESSETSU_OPAMP_V2",
];

/// Vendor/LTspice descriptive fields that carry no Ngspice electrical meaning.
pub(crate) const DESCRIPTIVE_MODEL_FIELDS: &[&str] = &[
    "mfg", "type", "vceo", "icrate", "icrating", "iave", "vpk", "vds", "ron", "qg",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InlineModelKind {
    Diode,
    Bjt(&'static str),
    Mosfet(&'static str),
    Jfet(&'static str),
    Switch,
}

#[derive(Debug, Clone)]
pub(crate) struct InlineModel {
    pub(crate) original: String,
    pub(crate) mapped: String,
    pub(crate) kind: InlineModelKind,
    pub(crate) parameters: Vec<(String, String)>,
}

pub(crate) type InlineModels = BTreeMap<String, InlineModel>;

/// Collects root-level `.model` cards into typed inline models. Unsupported model families,
/// non-literal values and parameters outside the typed allowlist fail closed with the
/// original line number.
pub(crate) fn collect_models(
    lines: &[LogicalLine],
    report: &mut SpiceImportReport,
) -> InlineModels {
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

pub(crate) fn parse_inline_model(
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
        "NJF" => InlineModelKind::Jfet("njf"),
        "PJF" => InlineModelKind::Jfet("pjf"),
        "SW" => InlineModelKind::Switch,
        "" => return Err(format!("model '{original}' has no model type")),
        other => {
            return Err(format!(
                "model '{original}' type '{other}' is outside the typed subset (D, NPN, PNP, NMOS, PMOS, NJF, PJF, SW)"
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
        InlineModelKind::Jfet(_) => crate::models::JFET_MODEL_PARAMETERS,
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
pub(crate) fn normalize_assignments(text: &str) -> String {
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

pub(crate) fn render_inline_model(model: &InlineModel) -> String {
    let (kind, polarity) = match model.kind {
        InlineModelKind::Diode => ("diode", String::new()),
        InlineModelKind::Bjt(polarity) => ("bjt", format!(" {polarity}")),
        InlineModelKind::Mosfet(polarity) => ("mosfet", format!(" {polarity}")),
        InlineModelKind::Jfet(polarity) => ("jfet", format!(" {polarity}")),
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
