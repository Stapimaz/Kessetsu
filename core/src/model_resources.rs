//! Explicit resource discovery and the deliberately narrow browser library profile.
//! Resource bodies never become IR or an ordinary compile/export report.
use crate::graph::{NetlistGraph, generate_browser_analysis_netlist};
use crate::ir::{Analysis, CircuitIR, ModelDefinition, SimulatorCompatibility};
use crate::models::{
    ExternalModelResources, MAX_EXTERNAL_MODEL_BYTES, validate_external_resource_reference,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_BOUND_RESOURCE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_BOUND_RESOURCES: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct ResourceRequirement {
    pub model: String,
    pub resource: String,
    pub sha256: String,
    pub simulator: String,
}

/// Frontend discovery only, not model validation or backend generation.
pub fn resource_requirements(source: &str) -> Result<Vec<ResourceRequirement>, String> {
    let program = crate::parser::parse_program(source).map_err(|e| e.to_string())?;
    program
        .external_subcircuits
        .iter()
        .map(|declaration| {
            let value = |name: &str| {
                declaration
                    .parameters
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                    .map(|p| p.value.clone())
                    .ok_or_else(|| format!("{} requires {name}", declaration.name))
            };
            let resource = value("file")?;
            validate_external_resource_reference(&resource)?;
            Ok(ResourceRequirement {
                model: declaration.name.clone(),
                resource,
                sha256: value("sha256")?,
                simulator: value("simulator")?,
            })
        })
        .collect()
}

pub fn validate_resource_bindings(resources: &ExternalModelResources) -> Result<(), String> {
    if resources.len() > MAX_BOUND_RESOURCES {
        return Err("At most 64 local model resources may be bound".into());
    }
    let mut total = 0usize;
    for (name, bytes) in resources {
        validate_external_resource_reference(name)?;
        if bytes.len() > MAX_EXTERNAL_MODEL_BYTES {
            return Err(format!("'{name}' exceeds the 16 MiB model limit"));
        }
        total = total
            .checked_add(bytes.len())
            .ok_or("Model resource size overflow")?;
        if total > MAX_BOUND_RESOURCE_BYTES {
            return Err("Local model resources exceed the 32 MiB combined limit".into());
        }
    }
    Ok(())
}

/// Join SPICE continuations before examining directives or header terminals.
pub(crate) fn library_statements(text: &str) -> Result<Vec<String>, String> {
    let mut statements: Vec<String> = Vec::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        if line.len() > 32768 || statements.len() >= 100_000 {
            return Err("Model library exceeds statement/line bounds".into());
        }
        if line.chars().any(|c| c.is_control() && c != '\t') {
            return Err("Model library contains control characters".into());
        }
        if let Some(continuation) = line.strip_prefix('+') {
            let previous = statements
                .last_mut()
                .ok_or("Model continuation has no preceding statement")?;
            if previous.len() + continuation.len() + 1 > 32768 {
                return Err("Model logical statement exceeds 32 KiB".into());
            }
            previous.push(' ');
            previous.push_str(continuation.trim());
        } else {
            statements.push(line.to_string());
        }
    }
    Ok(statements)
}

/// A capability subset, not a general SPICE parser or a physical-model endorsement.
/// Only self-contained analog libraries are accepted; no execution/output/file cards.
pub fn validate_browser_library(text: &str) -> Result<(), String> {
    if text.len() > MAX_EXTERNAL_MODEL_BYTES {
        return Err("Model exceeds 16 MiB".into());
    }
    let statements = library_statements(text)?;
    let mut scope: Vec<String> = Vec::new();
    let mut definitions = BTreeSet::new();
    let mut references = Vec::new();
    for line in &statements {
        let lower = line.to_ascii_lowercase();
        if [
            "file", "pwl_file", "shell", ".control", ".include", ".lib", "osdi", "pre_osdi",
        ]
        .iter()
        .any(|token| lower.contains(token))
        {
            return Err(
                "Browser models cannot load files, libraries, controls or compiled plug-ins".into(),
            );
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        let head = fields[0].to_ascii_lowercase();
        if head.starts_with('.') {
            match head.as_str() {
                ".subckt" if fields.len() >= 4 && scope.len() < 64 => {
                    let name = fields[1].to_ascii_lowercase();
                    if !definitions.insert(name.clone()) {
                        return Err("Duplicate subcircuit entry in browser library".into());
                    }
                    scope.push(name);
                }
                ".ends" if fields.len() <= 2 => {
                    let expected = scope.pop().ok_or("Unmatched .ENDS in browser library")?;
                    if fields
                        .get(1)
                        .is_some_and(|name| !name.eq_ignore_ascii_case(&expected))
                    {
                        return Err("Mismatched .ENDS in browser library".into());
                    }
                }
                ".model" if fields.len() >= 3 && !scope.is_empty() => {
                    let model_type = fields[2].split('(').next().unwrap().to_ascii_uppercase();
                    if !matches!(
                        model_type.as_str(),
                        "D" | "NPN" | "PNP" | "NMOS" | "PMOS" | "SW" | "CSW"
                    ) {
                        return Err("Browser model type is not supported".into());
                    }
                }
                ".param" | ".func" if fields.len() >= 2 && !scope.is_empty() => {}
                _ => {
                    return Err(
                        "Browser model directive is not supported; use the native CLI".into(),
                    );
                }
            }
        } else {
            if scope.is_empty() {
                return Err("Browser model devices must be inside a subcircuit".into());
            }
            let minimum = match head.as_bytes()[0].to_ascii_uppercase() {
                b'R' | b'C' | b'L' | b'V' | b'I' | b'D' | b'B' => 4,
                b'E' | b'G' => 5,
                b'F' | b'H' | b'Q' => 5,
                b'M' | b'S' => 6,
                b'X' => 4,
                _ => return Err("Browser model element family is unsupported".into()),
            };
            if fields.len() < minimum {
                return Err("Malformed browser model card".into());
            }
            if head.starts_with('x') {
                references.push((
                    scope.last().unwrap().clone(),
                    fields.last().unwrap().to_ascii_lowercase(),
                ));
            }
        }
    }
    if !scope.is_empty() || definitions.is_empty() {
        return Err("Browser library requires complete subcircuit definitions".into());
    }
    if references
        .iter()
        .any(|(_, entry)| !definitions.contains(entry))
    {
        return Err(
            "Browser subcircuit dependency is unresolved or uses unsupported instance parameters"
                .into(),
        );
    }
    let mut edges: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (parent, child) in references {
        edges.entry(parent).or_default().push(child);
    }
    fn visit(
        name: &str,
        depth: usize,
        edges: &std::collections::BTreeMap<String, Vec<String>>,
        active: &mut BTreeSet<String>,
        heights: &mut std::collections::BTreeMap<String, usize>,
    ) -> Result<usize, String> {
        if depth > 64 {
            return Err("Browser subcircuit dependency depth exceeds 64".into());
        }
        if let Some(height) = heights.get(name) {
            if height + depth > 64 {
                return Err("Browser subcircuit dependency depth exceeds 64".into());
            }
            return Ok(*height);
        }
        if !active.insert(name.into()) {
            return Err("Recursive browser subcircuit dependency".into());
        }
        let mut height = 0;
        if let Some(children) = edges.get(name) {
            for child in children {
                height = height.max(1 + visit(child, depth + 1, edges, active, heights)?);
            }
        }
        active.remove(name);
        heights.insert(name.into(), height);
        Ok(height)
    }
    let mut heights = std::collections::BTreeMap::new();
    for name in &definitions {
        visit(name, 0, &edges, &mut BTreeSet::new(), &mut heights)?;
    }
    Ok(())
}

/// Assemble an ephemeral simulation deck from IR and exact caller-bound bytes.
/// The ordinary SPICE exporter still emits dependency references, never model bodies.
pub fn browser_analysis_with_resources(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    analysis: &Analysis,
    resources: &ExternalModelResources,
) -> Result<String, String> {
    validate_resource_bindings(resources)?;
    let mut deck = generate_browser_analysis_netlist(circuit, graph, analysis);
    let mut inserted = BTreeSet::new();
    let mut entries = BTreeSet::new();
    for component in &circuit.components {
        if let Some(model) = &component.model
            && let ModelDefinition::Subcircuit { directive, .. } = &model.definition
        {
            for line in library_statements(directive)? {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields[0].eq_ignore_ascii_case(".subckt") && fields.len() >= 2 {
                    entries.insert(fields[1].to_ascii_lowercase());
                }
            }
        }
    }
    for component in &circuit.components {
        let Some(model) = &component.model else {
            continue;
        };
        let ModelDefinition::ExternalSubcircuit { metadata } = &model.definition else {
            continue;
        };
        if metadata.simulator != SimulatorCompatibility::Ngspice {
            return Err(format!(
                "{} is native-only: browser simulation does not enable ngspice_ps compatibility",
                model.name
            ));
        }
        let bytes = resources.get(&metadata.resource).ok_or_else(|| {
            format!(
                "Select local model file '{}' in View → Circuit details",
                metadata.resource
            )
        })?;
        if format!("sha256:{:x}", Sha256::digest(bytes)) != model.provenance.content_hash {
            return Err(format!(
                "Model '{}' no longer matches its declared hash",
                model.name
            ));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "Model library must be UTF-8")?;
        validate_browser_library(text).map_err(|reason| {
            format!(
                "{} is native-only for this browser profile: {reason}",
                model.name
            )
        })?;
        if inserted.insert(metadata.resource.clone()) {
            for line in library_statements(text)? {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields[0].eq_ignore_ascii_case(".subckt")
                    && fields.len() >= 2
                    && !entries.insert(fields[1].to_ascii_lowercase())
                {
                    return Err("Browser subcircuit entries collide across selected libraries or built-ins; supply unambiguous model libraries".into());
                }
            }
            // Only replace Core-generated include lines selected by typed IR.
            // Library directives cannot include files; validation precedes any substitution.
            let include = format!(".include \"{}\"", metadata.resource);
            deck = deck
                .lines()
                .map(|line| {
                    if line == include {
                        text.trim_start_matches('\u{feff}').to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            deck.push('\n');
        }
    }
    Ok(deck)
}
