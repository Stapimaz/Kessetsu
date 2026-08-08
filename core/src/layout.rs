use crate::ir::*;
use crate::graph::NetlistGraph;
use serde::{Serialize, Deserialize};
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
    pub net_id: usize,
    pub points: Vec<(i32, i32)>,
}

#[derive(Clone, Debug)]
struct ComponentDef {
    width: i32,
    height: i32,
    pins: HashMap<String, (i32, i32)>,
}

fn kind_to_string(kind: &ComponentKind) -> String {
    match kind {
        ComponentKind::Resistor => "Resistor".to_string(),
        ComponentKind::Capacitor => "Capacitor".to_string(),
        ComponentKind::Inductor => "Inductor".to_string(),
        ComponentKind::Diode => "Diode".to_string(),
        ComponentKind::BJT(_) => "Transistor".to_string(),
        ComponentKind::MOSFET(_) => "Mosfet".to_string(),
        ComponentKind::OpAmp => "OpAmp".to_string(),
        ComponentKind::VoltageSource => "Source".to_string(),
        ComponentKind::CurrentSource => "CurrentSource".to_string(),
        ComponentKind::ModulePort => "ModulePort".to_string(),
    }
}

fn get_comp_def(kind: &ComponentKind) -> ComponentDef {
    let mut pins = HashMap::new();
    match kind {
        ComponentKind::Resistor | ComponentKind::Capacitor | ComponentKind::Inductor | ComponentKind::Diode => {
            pins.insert("p1".to_string(), (0, 0));
            pins.insert("p2".to_string(), (2, 0));
            ComponentDef { width: 2, height: 1, pins }
        }
        ComponentKind::VoltageSource | ComponentKind::CurrentSource => {
            pins.insert("plus".to_string(), (0, 0));
            pins.insert("minus".to_string(), (2, 0));
            ComponentDef { width: 2, height: 1, pins }
        }
        ComponentKind::BJT(_) => {
            pins.insert("b".to_string(), (0, 1));
            pins.insert("c".to_string(), (2, 0));
            pins.insert("e".to_string(), (2, 2));
            ComponentDef { width: 3, height: 3, pins }
        }
        ComponentKind::MOSFET(_) => {
            pins.insert("g".to_string(), (0, 1));
            pins.insert("d".to_string(), (2, 0));
            pins.insert("s".to_string(), (2, 2));
            ComponentDef { width: 3, height: 3, pins }
        }
        ComponentKind::OpAmp => {
            pins.insert("in_n".to_string(), (0, 0));
            pins.insert("in_p".to_string(), (0, 2));
            pins.insert("vcc".to_string(), (1, -1));
            pins.insert("vee".to_string(), (1, 3));
            pins.insert("out".to_string(), (3, 1));
            ComponentDef { width: 3, height: 3, pins }
        }
        ComponentKind::ModulePort => {
            ComponentDef { width: 2, height: 2, pins }
        }
    }
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

fn get_through_pin(comp_type: &str, entry_pin: &str) -> String {
    match comp_type {
        "Resistor" | "Capacitor" | "Inductor" | "Diode" => {
            if entry_pin == "p1" { "p2".to_string() } else { "p1".to_string() }
        }
        "Source" | "CurrentSource" => {
            if entry_pin == "plus" { "minus".to_string() } else { "plus".to_string() }
        }
        "Transistor" => {
            match entry_pin {
                "c" => "e".to_string(),
                "e" => "c".to_string(),
                _ => "e".to_string(),
            }
        }
        "Mosfet" => {
            match entry_pin {
                "d" => "s".to_string(),
                "s" => "d".to_string(),
                _ => "s".to_string(),
            }
        }
        "OpAmp" => {
            match entry_pin {
                "vcc" => "vee".to_string(),
                "vee" => "vcc".to_string(),
                _ => "out".to_string(),
            }
        }
        _ => "p2".to_string(),
    }
}

fn is_signal_pin(comp_type: &str, pin: &str) -> bool {
    match comp_type {
        "Transistor" => pin == "b",
        "Mosfet" => pin == "g",
        _ => false,
    }
}

pub fn generate_layout(circuit: &CircuitIR) -> LayoutResult {
    let mut components = HashMap::new();
    let mut defs = HashMap::new();

    let graph = NetlistGraph::build(circuit);
    let mut nets_map: HashMap<usize, Vec<(String, String)>> = HashMap::new();

    for (pin_id, net_id) in &graph.pin_to_net {
        let parts: Vec<&str> = pin_id.split('.').collect();
        if parts.len() == 2 {
            nets_map.entry(*net_id).or_insert_with(Vec::new)
                .push((parts[0].to_string(), parts[1].to_string()));
        }
    }

    for comp in &circuit.components {
        let def = get_comp_def(&comp.kind);
        defs.insert(comp.id.clone(), def.clone());
        components.insert(comp.id.clone(), ComponentPos {
            x: 0, y: 0,
            comp_type: kind_to_string(&comp.kind),
            width: def.width,
            height: def.height,
            rotation: 0,
        });
    }

    if components.is_empty() {
        return LayoutResult { components, wires: Vec::new() };
    }

    let mut vcc_net: Option<usize> = None;
    let mut gnd_net: Option<usize> = None;
    let mut battery_name: Option<String> = None;

    for (net_id, pins) in &nets_map {
        for (comp_name, pin_name) in pins {
            if let Some(pos) = components.get(comp_name) {
                if pos.comp_type == "Source" || pos.comp_type == "Battery" { // fallback match string
                    battery_name = Some(comp_name.clone());
                    if pin_name == "plus" { vcc_net = Some(*net_id); }
                    if pin_name == "minus" { gnd_net = Some(*net_id); }
                }
            }
        }
    }

    let mut chains: Vec<Vec<(String, String)>> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    if let Some(ref bn) = battery_name {
        used.insert(bn.clone());
    }

    if let Some(vcc) = vcc_net {
        if let Some(vcc_pins) = nets_map.get(&vcc) {
            let mut vcc_entries: Vec<(String, String)> = vcc_pins.iter()
                .filter(|(c, _)| components.contains_key(c) && !used.contains(c))
                .cloned()
                .collect();
            vcc_entries.sort_by(|a, b| {
                let a_through = {
                    let ct = &components[&a.0].comp_type;
                    let exit = get_through_pin(ct, &a.1);
                    let enet = graph.get_net(&a.0, &exit);
                    if enet == 9999 { false }
                    else if let Some(np) = nets_map.get(&enet) {
                        np.iter().any(|(nc, npin)| {
                            nc != &a.0 && components.contains_key(nc)
                            && !is_signal_pin(&components[nc].comp_type, npin)
                        })
                    } else { false }
                };
                let b_through = {
                    let ct = &components[&b.0].comp_type;
                    let exit = get_through_pin(ct, &b.1);
                    let enet = graph.get_net(&b.0, &exit);
                    if enet == 9999 { false }
                    else if let Some(np) = nets_map.get(&enet) {
                        np.iter().any(|(nc, npin)| {
                            nc != &b.0 && components.contains_key(nc)
                            && !is_signal_pin(&components[nc].comp_type, npin)
                        })
                    } else { false }
                };
                b_through.cmp(&a_through).then(a.0.cmp(&b.0))
            });

            for (start_comp, start_pin) in &vcc_entries {
                if used.contains(start_comp) { continue; }

                let mut chain: Vec<(String, String)> = Vec::new();
                let mut current = start_comp.clone();
                let mut entry_pin = start_pin.clone();

                loop {
                    if used.contains(&current) { break; }
                    used.insert(current.clone());
                    chain.push((current.clone(), entry_pin.clone()));

                    let comp_type = components[&current].comp_type.clone();
                    let exit_pin = get_through_pin(&comp_type, &entry_pin);
                    let exit_net = graph.get_net(&current, &exit_pin);

                    if Some(exit_net) == gnd_net { break; } 
                    if exit_net == 9999 { break; }          

                    let mut found_next = false;
                    if let Some(net_pins) = nets_map.get(&exit_net) {
                        let mut candidates: Vec<&(String, String)> = net_pins.iter()
                            .filter(|(nc, _)| {
                                nc != &current && !used.contains(nc) && components.contains_key(nc)
                            })
                            .collect();
                        candidates.sort_by(|a, b| {
                            let a_sig = is_signal_pin(&components[&a.0].comp_type, &a.1);
                            let b_sig = is_signal_pin(&components[&b.0].comp_type, &b.1);
                            a_sig.cmp(&b_sig).then(a.0.cmp(&b.0))
                        });

                        if let Some((nc, np)) = candidates.first() {
                            current = (*nc).clone();
                            entry_pin = (*np).clone();
                            found_next = true;
                        }
                    }

                    if !found_next { break; }
                }

                if !chain.is_empty() {
                    chains.push(chain);
                }
            }
        }
    }

    let mut remaining: Vec<String> = components.keys()
        .filter(|n| !used.contains(*n))
        .cloned()
        .collect();
    remaining.sort();
    for name in remaining {
        let comp_type = components[&name].comp_type.clone();
        let pins: Vec<&str> = match comp_type.as_str() {
            "Transistor" => vec!["c", "b", "e"],
            "Mosfet" => vec!["d", "g", "s"],
            "Source" | "Battery" => vec!["plus", "minus"],
            _ => vec!["p1", "p2"],
        };
        let mut entry = pins[0].to_string();
        for &p in &pins {
            let net = graph.get_net(&name, p);
            if Some(net) == gnd_net {
                entry = get_through_pin(&comp_type, p);
                break;
            }
        }
        for &p in &pins {
            let net = graph.get_net(&name, p);
            if Some(net) == vcc_net {
                entry = p.to_string();
                break;
            }
        }
        chains.push(vec![(name.clone(), entry)]);
        used.insert(name);
    }

    chains.sort_by_key(|chain| {
        let has_active = chain.iter().any(|(c, _)| {
            components.get(c).map_or(false, |p| {
                p.comp_type == "Transistor" || p.comp_type == "Mosfet"
            })
        });
        if has_active { 1 } else { 0 }
    });

    let col_spacing = match chains.len() {
        0..=2 => 7,
        3..=4 => 5,
        _     => 4,
    };
    let chain_start_y = 2;  

    if let Some(ref bname) = battery_name {
        if let Some(pos) = components.get_mut(bname) {
            pos.rotation = 1; 
            pos.x = 0;
            pos.y = 3;
        }
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
        if pos.x + bx_min < min_x { min_x = pos.x + bx_min; }
        if pos.y + by_min < min_y { min_y = pos.y + by_min; }
    }
    min_x -= 2;
    min_y -= 2;
    for pos in components.values_mut() {
        pos.x -= min_x;
        pos.y -= min_y;
    }

    let mut net_pin_positions: HashMap<usize, Vec<(i32, i32)>> = HashMap::new();
    for (net_id, pins) in &nets_map {
        let mut positions = Vec::new();
        for (comp_name, pin_name) in pins {
            if let (Some(pos), Some(def)) = (components.get(comp_name), defs.get(comp_name)) {
                if let Some(&(px, py)) = def.pins.get(pin_name) {
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

            wires.push(Wire { net_id: *net_id, points: vec![(min_px, rail_y), (max_px, rail_y)] });
            for &(px, py) in positions {
                if py != rail_y {
                    wires.push(Wire { net_id: *net_id, points: vec![(px, rail_y), (px, py)] });
                }
            }
        } else if is_gnd {
            let rail_y = positions.iter().map(|p| p.1).max().unwrap() + 1;
            let min_px = positions.iter().map(|p| p.0).min().unwrap();
            let max_px = positions.iter().map(|p| p.0).max().unwrap();

            wires.push(Wire { net_id: *net_id, points: vec![(min_px, rail_y), (max_px, rail_y)] });
            for &(px, py) in positions {
                if py != rail_y {
                    wires.push(Wire { net_id: *net_id, points: vec![(px, py), (px, rail_y)] });
                }
            }
        } else {
            for i in 1..positions.len() {
                let (ax, ay) = positions[i - 1];
                let (bx, by) = positions[i];

                if ax == bx || ay == by {
                    wires.push(Wire { net_id: *net_id, points: vec![(ax, ay), (bx, by)] });
                } else {
                    wires.push(Wire { net_id: *net_id, points: vec![(ax, ay), (bx, ay), (bx, by)] });
                }
            }
        }
    }

    LayoutResult {
        components,
        wires,
    }
}
