use crate::component::CatalogSymbol;
use crate::exporter::ExportError;
use crate::graph::{format_analysis, format_spice_number};
use crate::ir::{Analysis, CircuitIR, ComponentParams, ModelDefinition, SourceValue, Waveform};
use crate::schematic::{Point, Rect, Schematic, SchematicComponent, TextAnchor};
use std::collections::{BTreeMap, BTreeSet};

const GRID: i32 = 40;
const MARGIN: i32 = 160;

struct LtSymbol {
    name: &'static str,
    pins: &'static [(&'static str, i32, i32)],
}

const TWO_PIN_80: &[(&str, i32, i32)] = &[("p1", 16, 16), ("p2", 16, 96)];
const TWO_PIN_64: &[(&str, i32, i32)] = &[("p1", 16, 0), ("p2", 16, 64)];
const VOLTAGE: &[(&str, i32, i32)] = &[("plus", 0, 16), ("minus", 0, 96)];
const CURRENT: &[(&str, i32, i32)] = &[("plus", 0, 0), ("minus", 0, 80)];
const BJT: &[(&str, i32, i32)] = &[("c", 64, 0), ("b", 0, 48), ("e", 64, 96)];
const MOSFET: &[(&str, i32, i32)] = &[("d", 48, 0), ("g", 0, 80), ("s", 48, 96)];
const OPAMP: &[(&str, i32, i32)] = &[
    ("in_p", -32, 80),
    ("in_n", -32, 48),
    ("vcc", 0, 32),
    ("vee", 0, 96),
    ("out", 32, 64),
];

fn symbol(component: &SchematicComponent) -> Result<LtSymbol, ExportError> {
    let symbol = match component.symbol {
        CatalogSymbol::Resistor => LtSymbol {
            name: "res",
            pins: TWO_PIN_80,
        },
        CatalogSymbol::Capacitor => LtSymbol {
            name: "cap",
            pins: TWO_PIN_64,
        },
        CatalogSymbol::Inductor => LtSymbol {
            name: "ind",
            pins: TWO_PIN_80,
        },
        CatalogSymbol::Diode => LtSymbol {
            name: "diode",
            pins: TWO_PIN_64,
        },
        CatalogSymbol::VoltageSource => LtSymbol {
            name: "voltage",
            pins: VOLTAGE,
        },
        CatalogSymbol::CurrentSource => LtSymbol {
            name: "current",
            pins: CURRENT,
        },
        CatalogSymbol::Bjt => LtSymbol {
            name: if component.variant.as_deref() == Some("pnp") {
                "pnp"
            } else {
                "npn"
            },
            pins: BJT,
        },
        CatalogSymbol::Mosfet => LtSymbol {
            name: if component.variant.as_deref() == Some("pmos") {
                "pmos"
            } else {
                "nmos"
            },
            pins: MOSFET,
        },
        CatalogSymbol::OpAmp => LtSymbol {
            name: "OpAmps/opamp2",
            pins: OPAMP,
        },
        CatalogSymbol::ExternalTwoTerminal => LtSymbol {
            // Reuse the native rectangular outline, not its resistor semantics.
            // Prefix X is explicitly emitted below; pins retain subcircuit order.
            name: "Misc/EuropeanResistor",
            pins: TWO_PIN_80,
        },
        CatalogSymbol::ModulePort => {
            return Err(ExportError {
                code: "KES-X013".to_string(),
                message: format!(
                    "LTspice exporter has no lossless symbol for module port '{}'",
                    component.id
                ),
                diagnostics: Vec::new(),
            });
        }
    };
    Ok(symbol)
}

fn schematic_point(schematic: &Schematic, point: Point) -> (i32, i32) {
    (
        (point.x - schematic.bounds.min.x) * GRID + MARGIN,
        (point.y - schematic.bounds.min.y) * GRID + MARGIN,
    )
}

fn placement(
    schematic: &Schematic,
    component: &SchematicComponent,
    symbol: &LtSymbol,
) -> (i32, i32, NativeTransform) {
    // Match semantic pin vectors, including PNP's reversed emitter side. Native
    // symbols differ in dimensions and cannot simply inherit centroid + R0.
    let mut candidates = Vec::new();
    for mirrored in [false, true] {
        for turns in 0..4 {
            let transform = NativeTransform { turns, mirrored };
            let deltas: Vec<_> = symbol
                .pins
                .iter()
                .map(|(name, x, y)| {
                    let pin = component.pins.iter().find(|pin| pin.name == *name).unwrap();
                    let desired = schematic_point(schematic, pin.point);
                    let native = transform.apply(*x, *y);
                    (desired.0 - native.0, desired.1 - native.1)
                })
                .collect();
            let count = deltas.len() as i32;
            let x = ((deltas.iter().map(|p| p.0).sum::<i32>() / count + 4).div_euclid(8)) * 8;
            let y = ((deltas.iter().map(|p| p.1).sum::<i32>() / count + 4).div_euclid(8)) * 8;
            let cost: i64 = deltas
                .iter()
                .map(|p| i64::from(p.0 - x).pow(2) + i64::from(p.1 - y).pow(2))
                .sum();
            candidates.push((cost, mirrored, turns, x, y));
        }
    }
    candidates.sort();
    let (_, mirrored, turns, x, y) = candidates[0];
    (x, y, NativeTransform { turns, mirrored })
}

#[derive(Clone, Copy)]
struct NativeTransform {
    turns: u8,
    mirrored: bool,
}
impl NativeTransform {
    fn apply(self, mut x: i32, mut y: i32) -> (i32, i32) {
        if self.mirrored {
            x = -x;
        }
        for _ in 0..self.turns {
            (x, y) = (-y, x);
        }
        (x, y)
    }
    fn inverse(self, mut x: i32, mut y: i32) -> (i32, i32) {
        for _ in 0..self.turns {
            (x, y) = (y, -x);
        }
        if self.mirrored {
            x = -x;
        }
        (x, y)
    }
    fn name(self) -> String {
        format!(
            "{}{}",
            if self.mirrored { "M" } else { "R" },
            u16::from(self.turns) * 90
        )
    }
}

type NativeAnnotations = BTreeMap<(String, i32), (Point, TextAnchor)>;

fn plan_native_annotations(
    components: &[SchematicComponent],
    circuit: &CircuitIR,
) -> Result<(NativeAnnotations, Vec<Rect>), ExportError> {
    let mut result = BTreeMap::new();
    let mut obstacles: Vec<Rect> = components
        .iter()
        .map(|component| component.bounds)
        .collect();
    let mut text_bounds = Vec::new();
    for component in components {
        let bounds = component.bounds;
        let value = component_value(component, circuit);
        let width = (component.reference.len().max(value.len()) as i32 * 2).max(4);
        let cx = (bounds.min.x + bounds.max.x) / 2;
        let cy = (bounds.min.y + bounds.max.y) / 2;
        let right = (
            Point {
                x: bounds.max.x + 3,
                y: cy - 1,
            },
            TextAnchor::Start,
        );
        let left = (
            Point {
                x: bounds.min.x - 3,
                y: cy - 1,
            },
            TextAnchor::End,
        );
        let above = (
            Point {
                x: cx,
                y: bounds.min.y - 7,
            },
            TextAnchor::Middle,
        );
        let below = (
            Point {
                x: cx,
                y: bounds.max.y + 6,
            },
            TextAnchor::Middle,
        );
        let vertical =
            component.pins.len() == 2 && component.pins[0].point.x == component.pins[1].point.x;
        let candidates = if vertical {
            [right, left, above, below]
        } else {
            [above, below, right, left]
        };
        let selected = candidates
            .into_iter()
            .find_map(|(point, anchor)| {
                let min_x = match anchor {
                    TextAnchor::Start => point.x,
                    TextAnchor::Middle => point.x - width / 2,
                    TextAnchor::End => point.x - width,
                };
                let block = Rect {
                    min: Point {
                        x: min_x,
                        y: point.y - 3,
                    },
                    max: Point {
                        x: min_x + width,
                        y: point.y + 5,
                    },
                };
                if obstacles.iter().any(|other| {
                    block.min.x <= other.max.x
                        && block.max.x >= other.min.x
                        && block.min.y <= other.max.y
                        && block.max.y >= other.min.y
                }) {
                    None
                } else {
                    Some((point, anchor, block))
                }
            })
            .ok_or_else(|| ExportError {
                code: "KES-X017".to_string(),
                message: format!(
                    "LTspice cannot allocate nearby reference/value space for {}",
                    component.id
                ),
                diagnostics: Vec::new(),
            })?;
        let (point, anchor, block) = selected;
        result.insert((component.id.clone(), 0), (point, anchor));
        result.insert(
            (component.id.clone(), 3),
            (
                Point {
                    x: point.x,
                    y: point.y + 4,
                },
                anchor,
            ),
        );
        obstacles.push(block);
        text_bounds.push(Rect {
            min: Point {
                x: block.min.x * 8,
                y: block.min.y * 8,
            },
            max: Point {
                x: block.max.x * 8,
                y: block.max.y * 8,
            },
        });
    }
    Ok((result, text_bounds))
}

fn spice_source(value: &SourceValue) -> String {
    match value {
        SourceValue::Dc(value) => format_spice_number(value.value),
        SourceValue::Waveform(Waveform::Ac { amplitude }) => {
            format!("AC {}", format_spice_number(amplitude.value))
        }
        SourceValue::Waveform(Waveform::Sine {
            offset,
            amplitude,
            frequency,
        }) => format!(
            "SINE({} {} {})",
            format_spice_number(offset.value),
            format_spice_number(amplitude.value),
            format_spice_number(frequency.value)
        ),
        SourceValue::Waveform(Waveform::SineAc {
            offset,
            amplitude,
            frequency,
            ac_amplitude,
        }) => format!(
            "SINE({} {} {}) AC {}",
            format_spice_number(offset.value),
            format_spice_number(amplitude.value),
            format_spice_number(frequency.value),
            format_spice_number(ac_amplitude.value)
        ),
        SourceValue::Waveform(Waveform::Pulse {
            v1,
            v2,
            delay,
            rise,
            fall,
            width,
            period,
        }) => format!(
            "PULSE({} {} {} {} {} {} {})",
            format_spice_number(v1.value),
            format_spice_number(v2.value),
            format_spice_number(delay.value),
            format_spice_number(rise.value),
            format_spice_number(fall.value),
            format_spice_number(width.value),
            format_spice_number(period.value)
        ),
        SourceValue::Waveform(Waveform::PWL { points }) => format!(
            "PWL({})",
            points
                .iter()
                .map(|(time, value)| format!(
                    "{} {}",
                    format_spice_number(time.value),
                    format_spice_number(value.value)
                ))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    }
}

fn component_value(component: &SchematicComponent, circuit: &CircuitIR) -> String {
    let Some(ir) = circuit
        .components
        .iter()
        .find(|candidate| candidate.id == component.id)
    else {
        return component
            .model
            .clone()
            .or_else(|| component.value.clone())
            .unwrap_or_else(|| "Kessetsu".to_string());
    };
    if let Some(model) = &ir.model
        && let ModelDefinition::ExternalSubcircuit { metadata } = &model.definition
    {
        return metadata.entry.clone();
    }
    match &ir.parameters {
        ComponentParams::TwoPinPassive { value } => format_spice_number(value.value),
        ComponentParams::VoltageSource { value } | ComponentParams::CurrentSource { value } => {
            spice_source(value)
        }
        _ => component
            .model
            .clone()
            .or_else(|| component.value.clone())
            .unwrap_or_else(|| "Kessetsu".to_string()),
    }
}

/// Generates an LTspice XVII/24 `.asc` schematic from canonical Schematic IR.
/// Standard bundled symbols are transformed and routed in their actual `.asy`
/// geometry. The coordinate proof is repeated after target adaptation.
pub fn generate_ltspice_asc(
    schematic: &Schematic,
    circuit: &CircuitIR,
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
            message: "canonical connectivity proof failed; LTspice export stopped".to_string(),
            diagnostics: Vec::new(),
        });
    }

    let mut pin_points = BTreeMap::<(String, String), (i32, i32)>::new();
    let mut placements = Vec::new();
    let mut native_components = Vec::new();
    for component in &schematic.components {
        let lt_symbol = symbol(component)?;
        if lt_symbol.pins.len() != component.pins.len() {
            return Err(ExportError {
                code: "KES-X014".to_string(),
                message: format!(
                    "LTspice symbol '{}' pin count does not match canonical component '{}'",
                    lt_symbol.name, component.id
                ),
                diagnostics: Vec::new(),
            });
        }
        if lt_symbol
            .pins
            .iter()
            .any(|(name, _, _)| !component.pins.iter().any(|pin| pin.name == *name))
        {
            return Err(ExportError {
                code: "KES-X015".to_string(),
                message: format!(
                    "LTspice semantic pin mapping is missing for {}",
                    component.id
                ),
                diagnostics: Vec::new(),
            });
        }
        let (x, y, transform) = placement(schematic, component, &lt_symbol);
        let mut native = component.clone();
        for (pin_name, dx, dy) in lt_symbol.pins {
            let (dx, dy) = transform.apply(*dx, *dy);
            pin_points.insert(
                (component.id.clone(), (*pin_name).to_string()),
                (x + dx, y + dy),
            );
            let pin = native
                .pins
                .iter_mut()
                .find(|pin| pin.name == *pin_name)
                .unwrap();
            pin.point = Point {
                x: (x + dx) / 8,
                y: (y + dy) / 8,
            };
            let outward = match pin.side {
                crate::component::PinSide::Left => (-1, 0),
                crate::component::PinSide::Right => (1, 0),
                crate::component::PinSide::Top => (0, -1),
                crate::component::PinSide::Bottom => (0, 1),
            };
            pin.escape = Point {
                x: pin.point.x + outward.0,
                y: pin.point.y + outward.1,
            };
        }
        native.bounds = Rect {
            min: Point {
                x: native.pins.iter().map(|pin| pin.point.x).min().unwrap(),
                y: native.pins.iter().map(|pin| pin.point.y).min().unwrap(),
            },
            max: Point {
                x: native.pins.iter().map(|pin| pin.point.x).max().unwrap(),
                y: native.pins.iter().map(|pin| pin.point.y).max().unwrap(),
            },
        };
        if native.bounds.min.x == native.bounds.max.x {
            native.bounds.min.x -= 2;
            native.bounds.max.x += 2;
        }
        if native.bounds.min.y == native.bounds.max.y {
            native.bounds.min.y -= 2;
            native.bounds.max.y += 2;
        }
        native_components.push(native);
        placements.push((component, lt_symbol, x, y, transform));
    }
    let mut native_labels = schematic.labels.clone();
    for label in &mut native_labels {
        let mapped = pin_points[&(
            label.attached_to.component.clone(),
            label.attached_to.pin.clone(),
        )];
        label.point = Point {
            x: mapped.0 / 8,
            y: mapped.1 / 8,
        };
    }
    let (native_annotations, native_text_bounds) =
        plan_native_annotations(&native_components, circuit)?;
    let (native_wires, native_junctions, native_crossings, native_labels) =
        crate::schematic::route_schematic(
            circuit,
            &native_components,
            &schematic.nets,
            native_labels,
            &native_text_bounds,
        )
        .map_err(|error| ExportError {
            code: "KES-X016".to_string(),
            message: format!("LTspice target routing failed: {error}"),
            diagnostics: Vec::new(),
        })?;
    let errors = crate::schematic_geometry::geometry_errors(
        &native_components,
        &native_wires,
        &native_junctions,
        &native_labels,
        &native_crossings,
    );
    if !errors.is_empty() {
        return Err(ExportError {
            code: "KES-X016".to_string(),
            message: format!("LTspice geometry proof failed: {}", errors.join("; ")),
            diagnostics: Vec::new(),
        });
    }

    let width = (schematic.bounds.max.x - schematic.bounds.min.x) * GRID + MARGIN * 2;
    let height = (schematic.bounds.max.y - schematic.bounds.min.y) * GRID + MARGIN * 2;
    let mut out = format!("Version 4\nSHEET 1 {width} {height}\n");
    for wire in &native_wires {
        for pair in wire.points.windows(2) {
            out.push_str(&format!(
                "WIRE {} {} {} {}\n",
                pair[0].x * 8,
                pair[0].y * 8,
                pair[1].x * 8,
                pair[1].y * 8
            ));
        }
    }
    for label in &native_labels {
        out.push_str(&format!(
            "FLAG {} {} {}\n",
            label.point.x * 8,
            label.point.y * 8,
            label.text
        ));
    }
    for (component, lt_symbol, x, y, transform) in placements {
        out.push_str(&format!(
            "SYMBOL {} {x} {y} {}\n",
            lt_symbol.name,
            transform.name()
        ));
        for window in [0, 3] {
            let (point, anchor) = native_annotations[&(component.id.clone(), window)];
            let (wx, wy) = transform.inverse(point.x * 8 - x, point.y * 8 - y);
            let alignment = match (transform.turns, transform.mirrored, anchor) {
                (1 | 3, _, TextAnchor::Middle) => "VCenter",
                (_, _, TextAnchor::Middle) => "Center",
                (0, false, TextAnchor::Start)
                | (2, true, TextAnchor::Start)
                | (0, true, TextAnchor::End)
                | (2, false, TextAnchor::End) => "Left",
                (0, _, _) | (2, _, _) => "Right",
                (1, false, TextAnchor::Start)
                | (3, true, TextAnchor::Start)
                | (1, true, TextAnchor::End)
                | (3, false, TextAnchor::End) => "VBottom",
                _ => "VTop",
            };
            out.push_str(&format!("WINDOW {window} {wx} {wy} {alignment} 2\n"));
        }
        let source_prefix = match component.symbol {
            CatalogSymbol::VoltageSource => Some('V'),
            CatalogSymbol::CurrentSource => Some('I'),
            _ => None,
        };
        let instance = match source_prefix {
            Some(prefix) if !component.reference.to_ascii_uppercase().starts_with(prefix) => {
                format!("{prefix}_{}", component.reference)
            }
            _ => component.reference.clone(),
        };
        out.push_str(&format!("SYMATTR InstName {instance}\n"));
        if component.symbol == CatalogSymbol::ExternalTwoTerminal {
            out.push_str("SYMATTR Prefix X\n");
        }
        out.push_str(&format!(
            "SYMATTR Value {}\n",
            component_value(component, circuit).replace(['\r', '\n'], " ")
        ));
        if let Some(ir) = circuit
            .components
            .iter()
            .find(|candidate| candidate.id == component.id)
            && !ir.instance_parameters.is_empty()
        {
            out.push_str(&format!(
                "SYMATTR SpiceLine {}\n",
                ir.instance_parameters
                    .iter()
                    .map(|(name, value)| { format!("{name}={}", format_spice_number(value.value)) })
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
    }

    let mut directive_y = height + 32;
    let mut emitted_models = BTreeSet::new();
    for component in &circuit.components {
        if let Some(model) = &component.model {
            if !emitted_models.insert(model.name.to_ascii_uppercase()) {
                continue;
            }
            let directive = match &model.definition {
                ModelDefinition::Device { directive }
                | ModelDefinition::Subcircuit { directive, .. } => directive.clone(),
                ModelDefinition::ExternalSubcircuit { metadata } => {
                    format!(".include \"{}\"", metadata.resource)
                }
            };
            for line in directive.lines().filter(|line| !line.trim().is_empty()) {
                out.push_str(&format!("TEXT 32 {directive_y} Left 2 !{line}\n"));
                directive_y += 16;
            }
        }
    }
    for (index, analysis) in circuit.analyses.iter().enumerate() {
        let mut command = format_analysis(analysis, circuit);
        if let Analysis::DcSweep { source, .. } = analysis {
            let canonical_source = command.split_whitespace().nth(1).unwrap().to_string();
            let prefix = canonical_source.chars().next().unwrap();
            let lt_source = if source.to_ascii_uppercase().starts_with(prefix) {
                source.clone()
            } else {
                canonical_source.clone()
            };
            command = command.replacen(&canonical_source, &lt_source, 1);
        }
        // LTspice requires exactly one active analysis; retain the others as
        // visible comments so users can select them in the target application.
        let marker = if index == 0 { '!' } else { ';' };
        out.push_str(&format!(
            "TEXT 32 {directive_y} Left 2 {marker}.{command}\n"
        ));
        directive_y += 16;
    }
    Ok(out)
}
