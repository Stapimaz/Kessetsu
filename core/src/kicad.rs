use crate::exporter::ExportError;
use crate::ir::{CircuitIR, PartRatingKind, PhysicalPartAssignment};
use crate::schematic::{Point, Schematic, SchematicComponent, TextAnchor, TextRole};
use sha2::{Digest, Sha256};

// Keep canonical layout steps roomy in native EDA tools.  A 100 mil mapping
// made compact circuits electrically correct but crowded their properties and
// pin names; 200 mil preserves the same geometry while matching the visual
// density engineers expect from an editable KiCad schematic.
const GRID_MM: f64 = 5.08;
const PAGE_MARGIN_MM: f64 = 25.4;

fn uuid(seed: &str) -> String {
    let hex = format!("{:x}", Sha256::digest(seed.as_bytes()));
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn quoted(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn coordinate(value: f64) -> String {
    let rendered = format!("{value:.4}");
    rendered
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn point(schematic: &Schematic, value: Point) -> (String, String) {
    (
        coordinate((value.x - schematic.bounds.min.x) as f64 * GRID_MM + PAGE_MARGIN_MM),
        coordinate((value.y - schematic.bounds.min.y) as f64 * GRID_MM + PAGE_MARGIN_MM),
    )
}

fn local_point(component: &SchematicComponent, value: Point) -> (String, String) {
    (
        coordinate((value.x - component.origin.x) as f64 * GRID_MM),
        coordinate(-(value.y - component.origin.y) as f64 * GRID_MM),
    )
}

fn property(name: &str, value: &str, x: &str, y: &str, hidden: bool) -> String {
    format!(
        "      (property {} {} (at {x} {y} 0) (effects (font (size 1.27 1.27)){}))\n",
        quoted(name),
        quoted(value),
        if hidden { " (hide yes)" } else { "" }
    )
}

fn physical_pin_number(
    part: Option<&PhysicalPartAssignment>,
    logical: &str,
    fallback: usize,
) -> String {
    part.and_then(|part| part.pin_map.get(logical))
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn usable_footprint(part: Option<&PhysicalPartAssignment>) -> &str {
    part.filter(|part| !part.pin_map.is_empty())
        .and_then(|part| part.footprint.as_deref())
        .unwrap_or("")
}

fn library_symbol(component: &SchematicComponent, part: Option<&PhysicalPartAssignment>) -> String {
    let name = format!("Kessetsu:NL_{}", component.id);
    let child_name = format!("NL_{}_0_1", component.id);
    let pin_child_name = format!("NL_{}_1_1", component.id);
    let min_x = coordinate((component.bounds.min.x - component.origin.x) as f64 * GRID_MM);
    let min_y = coordinate(-(component.bounds.max.y - component.origin.y) as f64 * GRID_MM);
    let max_x = coordinate((component.bounds.max.x - component.origin.x) as f64 * GRID_MM);
    let max_y = coordinate(-(component.bounds.min.y - component.origin.y) as f64 * GRID_MM);
    let prefix = component
        .reference
        .chars()
        .take_while(|character| character.is_ascii_alphabetic() || *character == '#')
        .collect::<String>();
    let mut out = format!(
        "    (symbol {}\n      (pin_numbers hide)\n      (pin_names (offset 0) hide)\n      (exclude_from_sim no)\n      (in_bom yes)\n      (on_board yes)\n",
        quoted(&name)
    );
    out.push_str(&property("Reference", &prefix, "0", "-2.54", false));
    out.push_str(&property("Value", "Kessetsu", "0", "2.54", false));
    out.push_str(&property(
        "Footprint",
        usable_footprint(part),
        "0",
        "0",
        true,
    ));
    out.push_str(&property("Datasheet", "", "0", "0", true));
    out.push_str(&property(
        "Description",
        "Portable symbol generated from canonical Kessetsu Schematic IR",
        "0",
        "0",
        true,
    ));
    out.push_str(&format!(
        "      (symbol {}\n        (rectangle (start {min_x} {min_y}) (end {max_x} {max_y}) (stroke (width 0.254) (type default)) (fill (type background)))\n      )\n",
        quoted(&child_name)
    ));
    out.push_str(&format!("      (symbol {}\n", quoted(&pin_child_name)));
    for (index, pin) in component.pins.iter().enumerate() {
        let (x, y) = local_point(component, pin.point);
        out.push_str(&format!(
            "        (pin passive line (at {x} {y} 0) (length 0) (name {} (effects (font (size 1.27 1.27)))) (number {} (effects (font (size 1.27 1.27)))))\n",
            quoted(&pin.name),
            quoted(&physical_pin_number(part, &pin.name, index + 1))
        ));
    }
    out.push_str("      )\n      (embedded_fonts no)\n    )\n");
    out
}

fn instance(
    schematic: &Schematic,
    component: &SchematicComponent,
    root_uuid: &str,
    part: Option<&PhysicalPartAssignment>,
) -> String {
    let (x, y) = point(schematic, component.origin);
    let instance_uuid = uuid(&format!("kicad:component:{}", component.id));
    let lib_id = format!("Kessetsu:NL_{}", component.id);
    let value = component
        .value
        .as_deref()
        .or(component.model.as_deref())
        .unwrap_or("Kessetsu");
    let mut out = format!(
        "  (symbol (lib_id {}) (at {x} {y} 0) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) (dnp no) (uuid {})\n",
        quoted(&lib_id),
        quoted(&instance_uuid)
    );
    for (name, content, role) in [
        (
            "Reference",
            component.reference.as_str(),
            TextRole::Reference,
        ),
        ("Value", value, TextRole::Value),
    ] {
        if let Some(text) = schematic.texts.iter().find(|text| {
            text.component == component.id
                && (text.role == role || (role == TextRole::Value && text.role == TextRole::Model))
        }) {
            let tx = coordinate(
                (text.point.x as f64 + text.offset_eighths.x as f64 / 8.0
                    - schematic.bounds.min.x as f64)
                    * GRID_MM
                    + PAGE_MARGIN_MM,
            );
            let ty = coordinate(
                (text.point.y as f64 + text.offset_eighths.y as f64 / 8.0
                    - schematic.bounds.min.y as f64)
                    * GRID_MM
                    + PAGE_MARGIN_MM,
            );
            let justify = match text.anchor {
                TextAnchor::Start => "left bottom",
                TextAnchor::Middle => "bottom",
                TextAnchor::End => "right bottom",
            };
            out.push_str(&format!("    (property {} {} (at {tx} {ty} 0) (effects (font (size 1.27 1.27)) (justify {justify})))\n", quoted(name),quoted(content)));
        } else {
            // Automatically selected generic models are intentionally omitted
            // from visible canonical annotations. Retain the Value property
            // as editable data, without putting an unplanned caption on the
            // symbol body in the native schematic.
            out.push_str(&property(name, content, &x, &y, role == TextRole::Value));
        }
    }
    out.push_str(&property("Footprint", usable_footprint(part), &x, &y, true));
    out.push_str(&property("Datasheet", "", &x, &y, true));
    out.push_str(&property(
        "Description",
        "Generated by Kessetsu",
        &x,
        &y,
        true,
    ));
    if let Some(part) = part {
        if let Some(manufacturer) = &part.manufacturer {
            out.push_str(&property("Manufacturer", manufacturer, &x, &y, true));
        }
        if let Some(mpn) = &part.mpn {
            out.push_str(&property("MPN", mpn, &x, &y, true));
        }
        if !part.pin_map.is_empty() {
            let mapping = part
                .pin_map
                .iter()
                .map(|(logical, physical)| format!("{logical}:{physical}"))
                .collect::<Vec<_>>()
                .join(";");
            out.push_str(&property("Kessetsu_Pin_Map", &mapping, &x, &y, true));
        }
        if let Some(note) = &part.note {
            out.push_str(&property("Kessetsu_Part_Note", note, &x, &y, true));
        }
        if !part.ratings.is_empty() {
            let rating_name = |kind| match kind {
                PartRatingKind::PeakVoltage => "peak_voltage",
                PartRatingKind::PeakCurrent => "peak_current",
                PartRatingKind::AverageDissipation => "average_dissipation",
            };
            let ratings = part
                .ratings
                .iter()
                .map(|rating| {
                    format!(
                        "{}<={}",
                        rating_name(rating.kind),
                        crate::sim_result::format_quantity(rating.limit.value, rating.limit.unit)
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            let conditions = part
                .ratings
                .iter()
                .map(|rating| format!("{}: {}", rating_name(rating.kind), rating.conditions))
                .collect::<Vec<_>>()
                .join("; ");
            let mut sources = part
                .ratings
                .iter()
                .filter_map(|rating| rating.source.clone())
                .collect::<Vec<_>>();
            sources.sort();
            sources.dedup();
            out.push_str(&property("Kessetsu_Ratings", &ratings, &x, &y, true));
            out.push_str(&property(
                "Kessetsu_Rating_Conditions",
                &conditions,
                &x,
                &y,
                true,
            ));
            if !sources.is_empty() {
                out.push_str(&property(
                    "Kessetsu_Rating_Sources",
                    &sources.join("; "),
                    &x,
                    &y,
                    true,
                ));
            }
        }
    }
    if let Some(model) = &component.model {
        out.push_str(&property("Kessetsu_Model", model, &x, &y, true));
    }
    if !component.instance_parameters.is_empty() {
        let parameters = component
            .instance_parameters
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        out.push_str(&property(
            "Kessetsu_Instance_Parameters",
            &parameters,
            &x,
            &y,
            true,
        ));
    }
    if let Some(metadata) = &component.model_metadata
        && metadata.resource.is_some()
    {
        out.push_str(&property(
            "Kessetsu_Model_Hash",
            &metadata.content_hash,
            &x,
            &y,
            true,
        ));
        out.push_str(&property(
            "Kessetsu_Model_Resource",
            metadata.resource.as_deref().unwrap_or(""),
            &x,
            &y,
            true,
        ));
        out.push_str(&property(
            "Kessetsu_Model_Entry",
            metadata.entry.as_deref().unwrap_or(""),
            &x,
            &y,
            true,
        ));
        out.push_str(&property(
            "Kessetsu_Model_Simulator",
            &metadata.simulator,
            &x,
            &y,
            true,
        ));
        out.push_str(&property(
            "Kessetsu_Model_Redistribution",
            metadata.redistribution.as_deref().unwrap_or(""),
            &x,
            &y,
            true,
        ));
    }
    for (index, pin) in component.pins.iter().enumerate() {
        out.push_str(&format!(
            "    (pin {} (uuid {}))\n",
            quoted(&physical_pin_number(part, &pin.name, index + 1)),
            quoted(&uuid(&format!("kicad:pin:{}:{index}", component.id)))
        ));
    }
    out.push_str(&format!(
        "    (instances (project {} (path {} (reference {}) (unit 1))))\n  )\n",
        quoted("Kessetsu"),
        quoted(&format!("/{root_uuid}")),
        quoted(&component.reference)
    ));
    out
}

/// Generates a self-contained KiCad 9/10 s-expression schematic exclusively
/// from verified Schematic IR. Per-instance embedded symbols keep the file
/// portable and put every KiCad pin exactly on its canonical graph anchor.
pub fn generate_kicad_sch(schematic: &Schematic) -> Result<String, ExportError> {
    generate_kicad_sch_with_parts(schematic, None)
}

pub fn generate_kicad_sch_with_parts(
    schematic: &Schematic,
    circuit: Option<&CircuitIR>,
) -> Result<String, ExportError> {
    if !schematic.connectivity.verified
        || !crate::schematic_geometry::geometry_errors(
            &schematic.components,
            &schematic.wires,
            &schematic.junctions,
            &schematic.labels,
            &schematic.crossings,
        )
        .is_empty()
    {
        return Err(ExportError {
            code: "KES-X003".to_string(),
            message: "canonical connectivity proof failed; KiCad export stopped".to_string(),
            diagnostics: Vec::new(),
        });
    }
    if schematic
        .components
        .iter()
        .any(|component| component.pins.is_empty())
    {
        return Err(ExportError {
            code: "KES-X012".to_string(),
            message: "KiCad exporter cannot represent a component without typed pins".to_string(),
            diagnostics: Vec::new(),
        });
    }

    let schematic_json = serde_json::to_vec(schematic).unwrap_or_default();
    let root_uuid = uuid(&format!("kicad:root:{:x}", Sha256::digest(&schematic_json)));
    let mut out = format!(
        "(kicad_sch\n  (version 20241209)\n  (generator {})\n  (generator_version {})\n  (uuid {})\n  (paper {})\n  (title_block (title {}) (comment 1 {}))\n  (lib_symbols\n",
        quoted("kessetsu"),
        quoted(env!("CARGO_PKG_VERSION")),
        quoted(&root_uuid),
        quoted("A4"),
        quoted("Kessetsu Schematic"),
        quoted(&format!(
            "{} / {}",
            crate::schematic::SCHEMATIC_SCHEMA_VERSION,
            crate::exporter::EXPORT_SCHEMA_VERSION
        ))
    );
    for component in &schematic.components {
        let part = circuit.and_then(|circuit| {
            circuit
                .physical_parts
                .assignments
                .iter()
                .find(|part| part.component == component.id)
        });
        out.push_str(&library_symbol(component, part));
    }
    out.push_str("  )\n");

    for junction in &schematic.junctions {
        let (x, y) = point(schematic, junction.point);
        out.push_str(&format!(
            "  (junction (at {x} {y}) (diameter 0) (color 0 0 0 0) (uuid {}))\n",
            quoted(&uuid(&format!("kicad:junction:{}", junction.id)))
        ));
    }
    for wire in &schematic.wires {
        for (segment, pair) in wire.points.windows(2).enumerate() {
            let (x1, y1) = point(schematic, pair[0]);
            let (x2, y2) = point(schematic, pair[1]);
            out.push_str(&format!(
                "  (wire (pts (xy {x1} {y1}) (xy {x2} {y2})) (stroke (width 0) (type default)) (uuid {}))\n",
                quoted(&uuid(&format!("kicad:wire:{}:{segment}", wire.id)))
            ));
        }
    }
    for label in &schematic.labels {
        let (component, attached) = schematic
            .components
            .iter()
            .find(|component| component.id == label.attached_to.component)
            .and_then(|component| {
                component
                    .pins
                    .iter()
                    .find(|pin| pin.name == label.attached_to.pin)
                    .map(|pin| (component, pin))
            })
            .ok_or_else(|| ExportError {
                code: "KES-X018".to_string(),
                message: format!(
                    "KiCad net label '{}' has no canonical attachment {}.{}",
                    label.text, label.attached_to.component, label.attached_to.pin
                ),
                diagnostics: Vec::new(),
            })?;

        // KiCad renders a label away from its anchor according to its
        // justification.  Anchoring every label directly on a pin therefore
        // allowed power/ground names to overlap portable symbol bodies.  Add
        // a short outward stub and place the label at its free end instead.
        let sides = [
            (
                (attached.point.x - component.bounds.min.x).abs(),
                Point {
                    x: attached.point.x - 1,
                    y: attached.point.y,
                },
                "right bottom",
            ),
            (
                (attached.point.x - component.bounds.max.x).abs(),
                Point {
                    x: attached.point.x + 1,
                    y: attached.point.y,
                },
                "left bottom",
            ),
            (
                (attached.point.y - component.bounds.min.y).abs(),
                Point {
                    x: attached.point.x,
                    y: attached.point.y - 1,
                },
                "bottom",
            ),
            (
                (attached.point.y - component.bounds.max.y).abs(),
                Point {
                    x: attached.point.x,
                    y: attached.point.y + 1,
                },
                "top",
            ),
        ];
        let (_, label_point, justify) = sides
            .into_iter()
            .min_by_key(|(distance, _, _)| *distance)
            .expect("component sides are fixed");
        let (pin_x, pin_y) = point(schematic, attached.point);
        let (x, y) = point(schematic, label_point);
        out.push_str(&format!(
            "  (wire (pts (xy {pin_x} {pin_y}) (xy {x} {y})) (stroke (width 0) (type default)) (uuid {}))\n",
            quoted(&uuid(&format!("kicad:label-stub:{}", label.id)))
        ));
        out.push_str(&format!(
            "  (label {} (at {x} {y} 0) (effects (font (size 1.27 1.27)) (justify {justify})) (uuid {}))\n",
            quoted(&label.text),
            quoted(&uuid(&format!("kicad:label:{}", label.id)))
        ));
    }
    for component in &schematic.components {
        let part = circuit.and_then(|circuit| {
            circuit
                .physical_parts
                .assignments
                .iter()
                .find(|part| part.component == component.id)
        });
        out.push_str(&instance(schematic, component, &root_uuid, part));
    }
    out.push_str("  (sheet_instances (path \"/\" (page \"1\")))\n)\n");
    Ok(out)
}
