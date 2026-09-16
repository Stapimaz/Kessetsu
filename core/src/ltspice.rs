use crate::component::CatalogSymbol;
use crate::exporter::ExportError;
use crate::graph::{format_analysis, format_spice_number};
use crate::ir::{Analysis, CircuitIR, ComponentParams, ModelDefinition, SourceValue, Waveform};
use crate::schematic::{Point, Schematic, SchematicComponent, WireEndpoint};
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
) -> (i32, i32) {
    let desired = component
        .pins
        .iter()
        .map(|pin| schematic_point(schematic, pin.point))
        .collect::<Vec<_>>();
    let desired_x = desired.iter().map(|point| point.0).sum::<i32>() / desired.len() as i32;
    let desired_y = desired.iter().map(|point| point.1).sum::<i32>() / desired.len() as i32;
    let base_x = symbol.pins.iter().map(|(_, x, _)| x).sum::<i32>() / symbol.pins.len() as i32;
    let base_y = symbol.pins.iter().map(|(_, _, y)| y).sum::<i32>() / symbol.pins.len() as i32;
    (desired_x - base_x, desired_y - base_y)
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
/// Standard bundled symbols are used, while endpoint coordinates are remapped
/// to their real `.asy` pin locations before canonical routes are emitted.
pub fn generate_ltspice_asc(
    schematic: &Schematic,
    circuit: &CircuitIR,
) -> Result<String, ExportError> {
    if !schematic.connectivity.verified {
        return Err(ExportError {
            code: "KES-X003".to_string(),
            message: "canonical connectivity proof failed; LTspice export stopped".to_string(),
            diagnostics: Vec::new(),
        });
    }

    let mut pin_points = BTreeMap::<(String, String), (i32, i32)>::new();
    let mut placements = Vec::new();
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
        let (x, y) = placement(schematic, component, &lt_symbol);
        for (pin_name, dx, dy) in lt_symbol.pins {
            pin_points.insert(
                (component.id.clone(), (*pin_name).to_string()),
                (x + dx, y + dy),
            );
        }
        placements.push((component, lt_symbol, x, y));
    }

    let endpoint = |value: &WireEndpoint, fallback: Point| -> Result<(i32, i32), ExportError> {
        match value {
            WireEndpoint::Pin { component, pin } => pin_points
                .get(&(component.clone(), pin.clone()))
                .copied()
                .ok_or_else(|| ExportError {
                    code: "KES-X015".to_string(),
                    message: format!("LTspice pin mapping is missing for {component}.{pin}"),
                    diagnostics: Vec::new(),
                }),
            WireEndpoint::Junction { .. } => Ok(schematic_point(schematic, fallback)),
        }
    };

    let width = (schematic.bounds.max.x - schematic.bounds.min.x) * GRID + MARGIN * 2;
    let height = (schematic.bounds.max.y - schematic.bounds.min.y) * GRID + MARGIN * 2;
    let mut out = format!("Version 4\nSHEET 1 {width} {height}\n");
    for wire in &schematic.wires {
        let mut points = wire
            .points
            .iter()
            .map(|point| schematic_point(schematic, *point))
            .collect::<Vec<_>>();
        if let Some(first) = points.first_mut() {
            *first = endpoint(&wire.start, wire.points[0])?;
        }
        if let Some(last) = points.last_mut() {
            *last = endpoint(&wire.end, *wire.points.last().unwrap_or(&wire.points[0]))?;
        }
        for pair in points.windows(2) {
            out.push_str(&format!(
                "WIRE {} {} {} {}\n",
                pair[0].0, pair[0].1, pair[1].0, pair[1].1
            ));
        }
    }
    for label in &schematic.labels {
        let label_point = pin_points
            .get(&(
                label.attached_to.component.clone(),
                label.attached_to.pin.clone(),
            ))
            .copied()
            .unwrap_or_else(|| schematic_point(schematic, label.point));
        out.push_str(&format!(
            "FLAG {} {} {}\n",
            label_point.0, label_point.1, label.text
        ));
    }
    for (component, lt_symbol, x, y) in placements {
        out.push_str(&format!("SYMBOL {} {x} {y} R0\n", lt_symbol.name));
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
        out.push_str(&format!(
            "SYMATTR Value {}\n",
            component_value(component, circuit).replace(['\r', '\n'], " ")
        ));
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
