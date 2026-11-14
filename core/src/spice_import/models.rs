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
    "LED_RED",
    "LED_GREEN",
    "LED_BLUE",
    "LED_WHITE",
    "IRF540",
    "IRF9540",
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
    "LM358",
    "TL072",
];

/// VDMOS-specific parameters that Ngspice Level 1 `NMOS`/`PMOS` user models do not accept.
const VDMOS_DROPPED_PARAMETERS: &[&str] = &[
    "rg",
    "rb",
    "rc",
    "cgdmax",
    "cgdmin",
    "a",
    "cjo",
    "vj",
    "m",
    "tt",
    "mtriode",
    "ksubthres",
    "theta",
    "bv",
    "ibv",
    "n",
    "tcvth",
    "mu",
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
    let mut is_vdmos = false;
    let mut kind = match model_type.as_str() {
        "D" => InlineModelKind::Diode,
        "NPN" => InlineModelKind::Bjt("npn"),
        "PNP" => InlineModelKind::Bjt("pnp"),
        "NMOS" => InlineModelKind::Mosfet("nmos"),
        "PMOS" => InlineModelKind::Mosfet("pmos"),
        "VDMOS" => {
            is_vdmos = true;
            InlineModelKind::Mosfet("nmos")
        }
        "NJF" => InlineModelKind::Jfet("njf"),
        "PJF" => InlineModelKind::Jfet("pjf"),
        "SW" => InlineModelKind::Switch,
        "" => return Err(format!("model '{original}' has no model type")),
        other => {
            return Err(format!(
                "model '{original}' type '{other}' is outside the typed subset (D, NPN, PNP, NMOS, PMOS, VDMOS, NJF, PJF, SW)"
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
    let mut vdmos_channel: Option<&'static str> = None;
    for assignment in normalize_assignments(&body.replace(',', " ")).split_whitespace() {
        if is_vdmos
            && (assignment.eq_ignore_ascii_case("pchan")
                || assignment.eq_ignore_ascii_case("nchan"))
        {
            let polarity = if assignment.eq_ignore_ascii_case("pchan") {
                "pmos"
            } else {
                "nmos"
            };
            if let Some(previous) = vdmos_channel
                && previous != polarity
            {
                return Err(format!(
                    "VDMOS model '{original}' specifies conflicting channel flags (nchan and pchan)"
                ));
            }
            vdmos_channel = Some(polarity);
            kind = InlineModelKind::Mosfet(polarity);
            continue;
        }
        if is_vdmos && assignment.eq_ignore_ascii_case("noiseless") {
            dropped.push("noiseless".to_string());
            continue;
        }
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
        if is_vdmos
            && !allowed.contains(&key.as_str())
            && VDMOS_DROPPED_PARAMETERS.contains(&key.as_str())
        {
            let _ = canonical_number(value)
                .map_err(|message| format!("model '{original}' parameter '{key}': {message}"))?;
            dropped.push(key);
            continue;
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

#[cfg(test)]
mod tests {
    use crate::spice_import::import_spice;
    use crate::{CompileOptions, compile_source};

    #[test]
    fn spice_import_recognizes_new_builtin_models_and_vdmos_inline_models() {
        let builtin_spice = "\
* Phase 6 Wave 2 built-in models
V1 vcc 0 12
D1 vcc n_led LED_RED
D2 vcc n_led LED_GREEN
D3 vcc n_led LED_BLUE
D4 vcc n_led LED_WHITE
Rled n_led 0 1k
M1 out_p 0 vcc vcc IRF9540
Rp out_p 0 1k
Vin in 0 5
XU1 in out_lm vcc 0 out_lm LM358
XU2 n_led out_tl vcc 0 out_tl TL072
.op
.end
";
        let report = import_spice(builtin_spice);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let kess = report.kess_source.expect("emitted kess source");
        assert!(kess.contains("diode D1 LED_RED"), "{kess}");
        assert!(kess.contains("mosfet M1 IRF9540"), "{kess}");
        assert!(kess.contains("opamp XU1 LM358"), "{kess}");
        assert!(kess.contains("opamp XU2 TL072"), "{kess}");
        let compiled = compile_source(&kess, CompileOptions::default());
        assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);

        let vdmos_spice = "\
* Phase 6 Wave 2 VDMOS inline and builtin collision rename
V1 vcc 0 12
M1 out_p 0 vcc vcc IRF9540
Rp out_p 0 1k
M2 out_u in 0 0 MY_VDMOS_N
Ru vcc out_u 1k
Vin in 0 5
.model MY_VDMOS_N VDMOS (nchan Rg=3 Vto=2.5 Kp=10 Rd=0.05 Rs=0.02 Cgdmax=1n Cgs=1n mfg=Test)
.model IRF9540 VDMOS (pchan Vto=-3.5 Kp=7.5 Lambda=0.015 Rd=0.11 Rs=0.06 Cgdmax=1.2n)
.op
.end
";
        let vdmos_report = import_spice(vdmos_spice);
        assert!(!vdmos_report.has_errors(), "{:?}", vdmos_report.diagnostics);
        let vdmos_kess = vdmos_report.kess_source.expect("emitted vdmos kess source");
        assert!(
            vdmos_kess.contains("model mosfet MY_VDMOS_N nmos")
                && vdmos_kess.contains("model mosfet SPICE_IRF9540 pmos")
                && vdmos_kess.contains("mosfet M1 SPICE_IRF9540"),
            "{vdmos_kess}"
        );
        let vdmos_compiled = compile_source(&vdmos_kess, CompileOptions::default());
        assert!(
            !vdmos_compiled.has_errors(),
            "{:?}",
            vdmos_compiled.diagnostics
        );
    }

    #[test]
    fn spice_import_rejects_conflicting_vdmos_flags_and_unknown_params() {
        let conflicting = "\
V1 vcc 0 5
M1 out 0 0 0 BAD_VDMOS
R1 vcc out 1k
.model BAD_VDMOS VDMOS (nchan pchan Vto=2 Kp=1)
.op
.end
";
        let report = import_spice(conflicting);
        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "KES-N012"));

        let unknown_param = "\
V1 vcc 0 5
M1 out 0 0 0 BAD_PARAM
R1 vcc out 1k
.model BAD_PARAM VDMOS (Vto=2 Kp=1 bogus_param=42)
.op
.end
";
        let report = import_spice(unknown_param);
        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "KES-N012"));
    }
}
