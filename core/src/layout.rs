use crate::component::{ComponentDefinition, component_definition, is_signal_pin, through_pin};
use crate::graph::{NetId, NetlistGraph};
use crate::ir::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LayoutResult {
    pub components: HashMap<String, ComponentPos>,
    pub wires: Vec<Wire>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComponentPos {
    pub x: i32,
    pub y: i32,
    pub comp_type: String,
    pub width: i32,
    pub height: i32,
    pub rotation: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Wire {
    pub net_id: NetId,
    pub points: Vec<(i32, i32)>,
}

fn get_bbox(pos: &ComponentPos) -> (i32, i32, i32, i32) {
    match pos.rotation {
        0 => (0, pos.width, 0, pos.height),
        1 => (-pos.height, 0, 0, pos.width),
        2 => (-pos.width, 0, -pos.height, 0),
        3 => (0, pos.height, -pos.width, 0),
        _ => (0, pos.width, 0, pos.height),
    }
}

pub fn generate_layout(circuit: &CircuitIR) -> LayoutResult {
    let mut components = HashMap::new();
    let mut defs: HashMap<String, ComponentDefinition> = HashMap::new();
    let mut component_kinds: HashMap<String, ComponentKind> = HashMap::new();

    let graph = NetlistGraph::build(circuit);
    let mut nets_map: HashMap<NetId, Vec<(String, String)>> = HashMap::new();

    for (pin_id, net_id) in &graph.pin_to_net {
        let parts: Vec<&str> = pin_id.split('.').collect();
        if parts.len() == 2 {
            nets_map
                .entry(*net_id)
                .or_default()
                .push((parts[0].to_string(), parts[1].to_string()));
        }
    }

    for comp in &circuit.components {
        let def = component_definition(&comp.kind);
        defs.insert(comp.id.clone(), def);
        component_kinds.insert(comp.id.clone(), comp.kind.clone());
        components.insert(
            comp.id.clone(),
            ComponentPos {
                x: 0,
                y: 0,
                comp_type: def.display_name.to_string(),
                width: def.width,
                height: def.height,
                rotation: 0,
            },
        );
    }

    if components.is_empty() {
        return LayoutResult {
            components,
            wires: Vec::new(),
        };
    }

    let mut vcc_net: Option<NetId> = None;
    let mut gnd_net: Option<NetId> = None;
    let mut battery_name: Option<String> = None;

    for (net_id, pins) in &nets_map {
        for (comp_name, pin_name) in pins {
            if component_kinds.get(comp_name) == Some(&ComponentKind::VoltageSource) {
                // fallback match string
                battery_name = Some(comp_name.clone());
                if pin_name == "plus" {
                    vcc_net = Some(*net_id);
                }
                if pin_name == "minus" {
                    gnd_net = Some(*net_id);
                }
            }
        }
    }

    let mut chains: Vec<Vec<(String, String)>> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    if let Some(ref bn) = battery_name {
        used.insert(bn.clone());
    }

    if let Some(vcc) = vcc_net
        && let Some(vcc_pins) = nets_map.get(&vcc)
    {
        let mut vcc_entries: Vec<(String, String)> = vcc_pins
            .iter()
            .filter(|(c, _)| components.contains_key(c) && !used.contains(c))
            .cloned()
            .collect();
        let has_through_connection = |entry: &(String, String)| {
            let kind = &component_kinds[&entry.0];
            let Some(exit) = through_pin(kind, &entry.1) else {
                return false;
            };
            let Some(exit_net) = graph.get_net(&entry.0, exit) else {
                return false;
            };
            nets_map.get(&exit_net).is_some_and(|pins| {
                pins.iter().any(|(next_component, next_pin)| {
                    next_component != &entry.0
                        && components.contains_key(next_component)
                        && !is_signal_pin(&component_kinds[next_component], next_pin)
                })
            })
        };
        vcc_entries.sort_by(|a, b| {
            let a_through = has_through_connection(a);
            let b_through = has_through_connection(b);
            b_through.cmp(&a_through).then(a.0.cmp(&b.0))
        });

        for (start_comp, start_pin) in &vcc_entries {
            if used.contains(start_comp) {
                continue;
            }

            let mut chain: Vec<(String, String)> = Vec::new();
            let mut current = start_comp.clone();
            let mut entry_pin = start_pin.clone();

            loop {
                if used.contains(&current) {
                    break;
                }
                used.insert(current.clone());
                chain.push((current.clone(), entry_pin.clone()));

                let kind = &component_kinds[&current];
                let Some(exit_pin) = through_pin(kind, &entry_pin) else {
                    break;
                };
                let Some(exit_net) = graph.get_net(&current, exit_pin) else {
                    break;
                };

                if Some(exit_net) == gnd_net {
                    break;
                }

                let mut found_next = false;
                if let Some(net_pins) = nets_map.get(&exit_net) {
                    let mut candidates: Vec<&(String, String)> = net_pins
                        .iter()
                        .filter(|(nc, _)| {
                            nc != &current && !used.contains(nc) && components.contains_key(nc)
                        })
                        .collect();
                    candidates.sort_by(|a, b| {
                        let a_sig = is_signal_pin(&component_kinds[&a.0], &a.1);
                        let b_sig = is_signal_pin(&component_kinds[&b.0], &b.1);
                        a_sig.cmp(&b_sig).then(a.0.cmp(&b.0))
                    });

                    if let Some((nc, np)) = candidates.first() {
                        current = (*nc).clone();
                        entry_pin = (*np).clone();
                        found_next = true;
                    }
                }

                if !found_next {
                    break;
                }
            }

            if !chain.is_empty() {
                chains.push(chain);
            }
        }
    }

    let mut remaining: Vec<String> = components
        .keys()
        .filter(|n| !used.contains(*n))
        .cloned()
        .collect();
    remaining.sort();
    for name in remaining {
        let kind = &component_kinds[&name];
        let definition = component_definition(kind);
        let mut entry = definition
            .pins
            .first()
            .map_or_else(String::new, |pin| pin.name.to_string());
        for pin in definition.pins {
            let net = graph.get_net(&name, pin.name);
            if net == gnd_net {
                entry = through_pin(kind, pin.name).unwrap_or(pin.name).to_string();
                break;
            }
        }
        for pin in definition.pins {
            let net = graph.get_net(&name, pin.name);
            if net == vcc_net {
                entry = pin.name.to_string();
                break;
            }
        }
        chains.push(vec![(name.clone(), entry)]);
        used.insert(name);
    }

    chains.sort_by_key(|chain| {
        let has_active = chain.iter().any(|(c, _)| {
            matches!(
                component_kinds.get(c),
                Some(ComponentKind::BJT(_) | ComponentKind::MOSFET(_))
            )
        });
        if has_active { 1 } else { 0 }
    });

    let col_spacing = match chains.len() {
        0..=2 => 7,
        3..=4 => 5,
        _ => 4,
    };
    let chain_start_y = 2;

    if let Some(ref bname) = battery_name
        && let Some(pos) = components.get_mut(bname)
    {
        pos.rotation = 1;
        pos.x = 0;
        pos.y = 3;
    }

    for (chain_idx, chain) in chains.iter().enumerate() {
        let axis_x = (chain_idx as i32 + 1) * col_spacing;
        let mut current_y = chain_start_y;

        for (comp_name, entry_pin) in chain {
            let pos = components.get_mut(comp_name).unwrap();

            if pos.comp_type == "Transistor" || pos.comp_type == "Mosfet" {
                pos.x = axis_x - 2;
                pos.y = current_y;
                pos.rotation = 0;
            } else {
                pos.x = axis_x;
                pos.y = current_y;
                if entry_pin == "p2" || entry_pin == "minus" {
                    pos.rotation = 3;
                } else {
                    pos.rotation = 1;
                }
            }

            current_y += 3;
        }
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    for pos in components.values() {
        let (bx_min, _, by_min, _) = get_bbox(pos);
        if pos.x + bx_min < min_x {
            min_x = pos.x + bx_min;
        }
        if pos.y + by_min < min_y {
            min_y = pos.y + by_min;
        }
    }
    min_x -= 2;
    min_y -= 2;
    for pos in components.values_mut() {
        pos.x -= min_x;
        pos.y -= min_y;
    }

    let mut net_pin_positions: HashMap<NetId, Vec<(i32, i32)>> = HashMap::new();
    for (net_id, pins) in &nets_map {
        let mut positions = Vec::new();
        for (comp_name, pin_name) in pins {
            if let (Some(pos), Some(def)) = (components.get(comp_name), defs.get(comp_name))
                && let Some(pin) = def.pins.iter().find(|pin| pin.name == pin_name)
            {
                let (px, py) = (pin.x, pin.y);
                let (rx, ry) = match pos.rotation {
                    0 => (pos.x + px, pos.y + py),
                    1 => (pos.x - py, pos.y + px),
                    2 => (pos.x - px, pos.y - py),
                    3 => (pos.x + py, pos.y - px),
                    _ => (pos.x + px, pos.y + py),
                };
                positions.push((rx, ry));
            }
        }
        positions.sort();
        positions.dedup();
        if positions.len() >= 2 {
            net_pin_positions.insert(*net_id, positions);
        }
    }

    let mut wires = Vec::new();

    for (net_id, positions) in &net_pin_positions {
        let is_vcc = Some(*net_id) == vcc_net;
        let is_gnd = Some(*net_id) == gnd_net;

        if is_vcc {
            let rail_y = positions.iter().map(|p| p.1).min().unwrap() - 1;
            let min_px = positions.iter().map(|p| p.0).min().unwrap();
            let max_px = positions.iter().map(|p| p.0).max().unwrap();

            wires.push(Wire {
                net_id: *net_id,
                points: vec![(min_px, rail_y), (max_px, rail_y)],
            });
            for &(px, py) in positions {
                if py != rail_y {
                    wires.push(Wire {
                        net_id: *net_id,
                        points: vec![(px, rail_y), (px, py)],
                    });
                }
            }
        } else if is_gnd {
            let rail_y = positions.iter().map(|p| p.1).max().unwrap() + 1;
            let min_px = positions.iter().map(|p| p.0).min().unwrap();
            let max_px = positions.iter().map(|p| p.0).max().unwrap();

            wires.push(Wire {
                net_id: *net_id,
                points: vec![(min_px, rail_y), (max_px, rail_y)],
            });
            for &(px, py) in positions {
                if py != rail_y {
                    wires.push(Wire {
                        net_id: *net_id,
                        points: vec![(px, py), (px, rail_y)],
                    });
                }
            }
        } else {
            for i in 1..positions.len() {
                let (ax, ay) = positions[i - 1];
                let (bx, by) = positions[i];

                if ax == bx || ay == by {
                    wires.push(Wire {
                        net_id: *net_id,
                        points: vec![(ax, ay), (bx, by)],
                    });
                } else {
                    wires.push(Wire {
                        net_id: *net_id,
                        points: vec![(ax, ay), (bx, ay), (bx, by)],
                    });
                }
            }
        }
    }

    LayoutResult { components, wires }
}
