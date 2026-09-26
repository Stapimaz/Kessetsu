use crate::component::component_definition;
use crate::ir::{
    CircuitIR, ComponentKind, PartRatingKind, PhysicalPartAssignment, PhysicalPartManifest,
};
use crate::schematic::Schematic;
use crate::sim_result::format_quantity;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const HANDOFF_SCHEMA_VERSION: &str = "kessetsu.handoff.v2";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BomKey {
    kind: String,
    electrical: String,
    manufacturer: String,
    mpn: String,
    footprint: String,
    pin_map: String,
    selection_status: String,
    eda_status: String,
    provided_ratings: String,
    rating_conditions: String,
    rating_sources: String,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffManifest {
    pub schema_version: String,
    pub generator: String,
    pub generator_version: String,
    pub compile_schema_version: String,
    pub circuit_ir_sha256: String,
    pub counts: HandoffCounts,
    pub physical_parts: PhysicalPartManifest,
    pub footprints: Vec<FootprintDependency>,
    pub model_dependencies: Vec<ModelDependency>,
    pub unresolved_components: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffCounts {
    pub electrical_components: usize,
    pub assigned_parts: usize,
    pub eda_ready_footprints: usize,
    pub external_model_dependencies: usize,
    pub provided_part_ratings: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FootprintDependency {
    pub component: String,
    pub footprint: String,
    pub pin_map: BTreeMap<String, String>,
    pub eda_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelDependency {
    pub model: String,
    pub resource: String,
    pub entry: String,
    pub sha256: String,
    pub simulator: String,
    pub redistribution: String,
}

fn included_in_bom(kind: &ComponentKind) -> bool {
    !matches!(
        kind,
        ComponentKind::ModulePort | ComponentKind::VoltageSource | ComponentKind::CurrentSource
    )
}

fn part_for<'a>(circuit: &'a CircuitIR, component: &str) -> Option<&'a PhysicalPartAssignment> {
    circuit
        .physical_parts
        .assignments
        .iter()
        .find(|part| part.component == component)
}

fn pin_map_text(part: Option<&PhysicalPartAssignment>) -> String {
    part.map(|part| {
        part.pin_map
            .iter()
            .map(|(logical, physical)| format!("{logical}:{physical}"))
            .collect::<Vec<_>>()
            .join(";")
    })
    .unwrap_or_default()
}

fn rating_name(kind: PartRatingKind) -> &'static str {
    match kind {
        PartRatingKind::PeakVoltage => "peak_voltage",
        PartRatingKind::PeakCurrent => "peak_current",
        PartRatingKind::AverageDissipation => "average_dissipation",
    }
}

fn rating_fields(part: Option<&PhysicalPartAssignment>) -> (String, String, String) {
    let Some(part) = part else {
        return (String::new(), String::new(), String::new());
    };
    let ratings = part
        .ratings
        .iter()
        .map(|rating| {
            format!(
                "{}<={}",
                rating_name(rating.kind),
                format_quantity(rating.limit.value, rating.limit.unit)
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let conditions = part
        .ratings
        .iter()
        .map(|rating| format!("{}:{}", rating_name(rating.kind), rating.conditions))
        .collect::<Vec<_>>()
        .join(";");
    let mut sources = part
        .ratings
        .iter()
        .filter_map(|rating| rating.source.clone())
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    (ratings, conditions, sources.join(";"))
}

fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn generate_bom_csv(circuit: &CircuitIR, schematic: &Schematic) -> Vec<u8> {
    let schematic_components = schematic
        .components
        .iter()
        .map(|component| (component.id.as_str(), component))
        .collect::<BTreeMap<_, _>>();
    let mut groups = BTreeMap::<BomKey, Vec<String>>::new();
    for component in circuit
        .components
        .iter()
        .filter(|component| included_in_bom(&component.kind))
    {
        let drawn = schematic_components.get(component.id.as_str());
        let reference = drawn
            .map(|component| component.reference.clone())
            .unwrap_or_else(|| component.id.clone());
        let electrical = drawn
            .and_then(|component| component.value.clone().or(component.model.clone()))
            .or_else(|| component.model.as_ref().map(|model| model.name.clone()))
            .unwrap_or_default();
        let part = part_for(circuit, &component.id);
        let manufacturer = part
            .and_then(|part| part.manufacturer.clone())
            .unwrap_or_default();
        let mpn = part.and_then(|part| part.mpn.clone()).unwrap_or_default();
        let footprint = part
            .and_then(|part| part.footprint.clone())
            .unwrap_or_default();
        let selection_status = if part.is_none() {
            "unassigned"
        } else if manufacturer.is_empty() || mpn.is_empty() {
            "incomplete"
        } else {
            "selected"
        };
        let eda_status = if footprint.is_empty() {
            "no_footprint"
        } else if part.is_some_and(|part| !part.pin_map.is_empty()) {
            "ready"
        } else {
            "pin_map_missing"
        };
        let (provided_ratings, rating_conditions, rating_sources) = rating_fields(part);
        groups
            .entry(BomKey {
                kind: component_definition(&component.kind).display_name.into(),
                electrical,
                manufacturer,
                mpn,
                footprint,
                pin_map: pin_map_text(part),
                selection_status: selection_status.into(),
                eda_status: eda_status.into(),
                provided_ratings,
                rating_conditions,
                rating_sources,
                note: part.and_then(|part| part.note.clone()).unwrap_or_default(),
            })
            .or_default()
            .push(reference);
    }

    let mut output = String::from(
        "item,quantity,references,component_kind,electrical,manufacturer,mpn,footprint,pin_map,selection_status,eda_status,provided_ratings,rating_conditions,rating_sources,note\r\n",
    );
    for (index, (key, mut references)) in groups.into_iter().enumerate() {
        references.sort();
        let fields = [
            (index + 1).to_string(),
            references.len().to_string(),
            references.join(";"),
            key.kind,
            key.electrical,
            key.manufacturer,
            key.mpn,
            key.footprint,
            key.pin_map,
            key.selection_status,
            key.eda_status,
            key.provided_ratings,
            key.rating_conditions,
            key.rating_sources,
            key.note,
        ];
        output.push_str(
            &fields
                .iter()
                .map(|field| csv_field(field))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push_str("\r\n");
    }
    output.into_bytes()
}

pub fn generate_handoff_manifest(
    circuit: &CircuitIR,
    compile_schema_version: &str,
) -> Result<Vec<u8>, serde_json::Error> {
    let circuit_json = serde_json::to_vec(circuit)?;
    let mut footprints = circuit
        .physical_parts
        .assignments
        .iter()
        .filter_map(|part| {
            part.footprint
                .as_ref()
                .map(|footprint| FootprintDependency {
                    component: part.component.clone(),
                    footprint: footprint.clone(),
                    pin_map: part.pin_map.clone(),
                    eda_ready: !part.pin_map.is_empty(),
                })
        })
        .collect::<Vec<_>>();
    footprints.sort_by(|left, right| left.component.cmp(&right.component));

    let mut model_dependencies = circuit
        .model_manifest
        .models
        .iter()
        .filter_map(|model| {
            model.external.as_ref().map(|external| ModelDependency {
                model: model.name.clone(),
                resource: external.resource.clone(),
                entry: external.entry.clone(),
                sha256: model.provenance.content_hash.clone(),
                simulator: model.provenance.simulator.clone(),
                redistribution: format!("{:?}", external.redistribution).to_ascii_lowercase(),
            })
        })
        .collect::<Vec<_>>();
    model_dependencies.sort_by(|left, right| left.model.cmp(&right.model));

    let assigned = circuit
        .physical_parts
        .assignments
        .iter()
        .map(|part| part.component.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let unresolved_components = circuit
        .components
        .iter()
        .filter(|component| included_in_bom(&component.kind))
        .filter(|component| !assigned.contains(component.id.as_str()))
        .map(|component| component.id.clone())
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    if !unresolved_components.is_empty() {
        warnings.push(
            "Unassigned electrical components remain; the BOM is not procurement-complete.".into(),
        );
    }
    if footprints.iter().any(|footprint| !footprint.eda_ready) {
        warnings.push(
            "A footprint without a complete pin map is recorded but not safe for automatic EDA attachment."
                .into(),
        );
    }
    if model_dependencies
        .iter()
        .any(|dependency| dependency.redistribution == "prohibited")
    {
        warnings.push(
            "One or more external model bodies cannot be redistributed; provide the exact hash-bound files separately."
                .into(),
        );
    }
    let provided_part_ratings = circuit
        .physical_parts
        .assignments
        .iter()
        .map(|part| part.ratings.len())
        .sum();
    if provided_part_ratings > 0 {
        warnings.push(crate::stress::PART_STRESS_DISCLAIMER.into());
    }
    let manifest = HandoffManifest {
        schema_version: HANDOFF_SCHEMA_VERSION.into(),
        generator: "kessetsu-handoff".into(),
        generator_version: env!("CARGO_PKG_VERSION").into(),
        compile_schema_version: compile_schema_version.into(),
        circuit_ir_sha256: format!("sha256:{:x}", Sha256::digest(&circuit_json)),
        counts: HandoffCounts {
            electrical_components: circuit
                .components
                .iter()
                .filter(|component| included_in_bom(&component.kind))
                .count(),
            assigned_parts: circuit.physical_parts.assignments.len(),
            eda_ready_footprints: footprints
                .iter()
                .filter(|footprint| footprint.eda_ready)
                .count(),
            external_model_dependencies: model_dependencies.len(),
            provided_part_ratings,
        },
        physical_parts: circuit.physical_parts.clone(),
        footprints,
        model_dependencies,
        unresolved_components,
        warnings,
    };
    let mut json = serde_json::to_vec_pretty(&manifest)?;
    json.push(b'\n');
    Ok(json)
}
