use crate::ast::*;
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
    pub rotation: i32, // 0: 0deg, 1: 90deg, 2: 180deg, 3: 270deg
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

fn get_comp_def(comp_type: &ComponentType) -> ComponentDef {
    let mut pins = HashMap::new();
    match comp_type {
        ComponentType::Resistor | ComponentType::Capacitor | ComponentType::Inductor | ComponentType::Diode => {
            pins.insert("p1".to_string(), (0, 0));
            pins.insert("p2".to_string(), (2, 0));
            ComponentDef { width: 2, height: 1, pins }
        }
        ComponentType::Source => {
            pins.insert("plus".to_string(), (0, 0));
            pins.insert("minus".to_string(), (2, 0));
            ComponentDef { width: 2, height: 1, pins }
        }
        ComponentType::Transistor => {
            pins.insert("b".to_string(), (0, 1));
            pins.insert("c".to_string(), (2, 0));
            pins.insert("e".to_string(), (2, 2));
            ComponentDef { width: 3, height: 3, pins }
        }
        ComponentType::Mosfet => {
            pins.insert("g".to_string(), (0, 1));
            pins.insert("d".to_string(), (2, 0));
            pins.insert("s".to_string(), (2, 2));
            ComponentDef { width: 3, height: 3, pins }
        }
        ComponentType::OpAmp => {
            pins.insert("in_n".to_string(), (0, 0));
            pins.insert("in_p".to_string(), (0, 2));
            pins.insert("vcc".to_string(), (1, -1));
            pins.insert("vee".to_string(), (1, 3));
            pins.insert("out".to_string(), (3, 1));
            ComponentDef { width: 3, height: 3, pins }
        }
        ComponentType::ModulePort => {
            pins.insert("in".to_string(), (0, 0));
            pins.insert("out".to_string(), (2, 0));
            ComponentDef { width: 2, height: 1, pins }
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

/// Returns the "through-path" exit pin for a given entry pin.
/// 2-pin: p1 <-> p2.  Transistor: c <-> e (b is signal).  MOSFET: d <-> s (g is signal).
fn get_through_pin(comp_type: &str, entry_pin: &str) -> String {
    match comp_type {
        "Resistor" | "Capacitor" | "Inductor" | "Diode" => {
            if entry_pin == "p1" { "p2".to_string() } else { "p1".to_string() }
        }
        "Battery" => {
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

/// Returns true if the pin is a "signal/control" pin (not on the main current path).
/// Transistor: b is signal. MOSFET: g is signal. All other pins are through-path.
fn is_signal_pin(comp_type: &str, pin: &str) -> bool {
    match comp_type {
        "Transistor" => pin == "b",
        "Mosfet" => pin == "g",
        _ => false,
    }
}

// ============================================================
// V5: Chain-Based Schematic Layout Engine
// ============================================================
//
// Core idea: A good schematic = vertical chains between VCC (top)
// and GND (bottom), with horizontal signal wires between them.
//
// 1. Find VCC/GND nets from battery
// 2. Trace series paths (chains) from VCC through components to GND
// 3. Sort: passive chains left, active (transistor) chains right
// 4. Place each chain as a vertical column on the grid
// 5. Route with horizontal rails (VCC/GND) + simple L-shaped wires
//
pub fn generate_layout(program: &Program) -> LayoutResult {
    let mut components = HashMap::new();
    let mut defs = HashMap::new();

    // Build netlist graph
    let graph = NetlistGraph::build(program);
    let mut nets_map: HashMap<usize, Vec<(String, String)>> = HashMap::new();

    for (pin_id, net_id) in &graph.pin_to_net {
        let parts: Vec<&str> = pin_id.split('.').collect();
        if parts.len() == 2 {
            nets_map.entry(*net_id).or_insert_with(Vec::new)
                .push((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Initialize all non-ModulePort components
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if decl.comp_type == ComponentType::ModulePort { continue; }
            let def = get_comp_def(&decl.comp_type);
            defs.insert(decl.name.clone(), def.clone());
            components.insert(decl.name.clone(), ComponentPos {
                x: 0, y: 0,
                comp_type: format!("{:?}", decl.comp_type),
                width: def.width,
                height: def.height,
                rotation: 0,
            });
        }
    }

    if components.is_empty() {
        return LayoutResult { components, wires: Vec::new() };
    }

    // ---- Step 1: Identify VCC / GND nets and battery ----
    let mut vcc_net: Option<usize> = None;
    let mut gnd_net: Option<usize> = None;
    let mut battery_name: Option<String> = None;

    for (net_id, pins) in &nets_map {
        for (comp_name, pin_name) in pins {
            if let Some(pos) = components.get(comp_name) {
                if pos.comp_type == "Battery" {
                    battery_name = Some(comp_name.clone());
                    if pin_name == "plus" { vcc_net = Some(*net_id); }
                    if pin_name == "minus" { gnd_net = Some(*net_id); }
                }
            }
        }
    }

    // ---- Step 2: Find chains (VCC → comp → comp → ... → GND) ----
    // Each chain element is (component_name, entry_pin) for orientation awareness
    let mut chains: Vec<Vec<(String, String)>> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    // Battery is placed separately, not part of any chain
    if let Some(ref bn) = battery_name {
        used.insert(bn.clone());
    }

    if let Some(vcc) = vcc_net {
        if let Some(vcc_pins) = nets_map.get(&vcc) {
            // Collect VCC entry points
            let mut vcc_entries: Vec<(String, String)> = vcc_pins.iter()
                .filter(|(c, _)| components.contains_key(c) && !used.contains(c))
                .cloned()
                .collect();
            // Sort VCC entries: prefer those whose exit net has through-path
            // downstream (e.g., collector connections) over signal-only (e.g., base)
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
                // through-path first (true > false, so reverse)
                b_through.cmp(&a_through).then(a.0.cmp(&b.0))
            });

            for (start_comp, start_pin) in &vcc_entries {
                if used.contains(start_comp) { continue; }

                let mut chain: Vec<(String, String)> = Vec::new();
                let mut current = start_comp.clone();
                let mut entry_pin = start_pin.clone();

                // Follow the through-path: enter comp → exit via paired pin → next comp
                loop {
                    if used.contains(&current) { break; }
                    used.insert(current.clone());
                    chain.push((current.clone(), entry_pin.clone()));

                    let comp_type = components[&current].comp_type.clone();
                    let exit_pin = get_through_pin(&comp_type, &entry_pin);
                    let exit_net = graph.get_net(&current, &exit_pin);

                    if Some(exit_net) == gnd_net { break; } // Reached ground — chain complete
                    if exit_net == 9999 { break; }          // Floating — chain ends

                    // Find next unvisited component on the exit net
                    // Prefer through-path pin entries (c, e, p1, p2) over signal (b, g)
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

    // Remaining unplaced components → singleton chains
    // Determine entry_pin based on VCC/GND connections for correct orientation
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
            "Battery" => vec!["plus", "minus"],
            _ => vec!["p1", "p2"],
        };
        // Find if any pin is on GND → that pin should be at bottom → entry = other pin
        let mut entry = pins[0].to_string();
        for &p in &pins {
            let net = graph.get_net(&name, p);
            if Some(net) == gnd_net {
                entry = get_through_pin(&comp_type, p);
                break;
            }
        }
        // Also check: if a pin is on VCC → that pin is the entry (should be at top)
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

    // ---- Step 3: Sort chains (passive left, active right) ----
    // Signal flows left → right: bias/divider networks first, transistors last
    chains.sort_by_key(|chain| {
        let has_active = chain.iter().any(|(c, _)| {
            components.get(c).map_or(false, |p| {
                p.comp_type == "Transistor" || p.comp_type == "Mosfet"
            })
        });
        if has_active { 1 } else { 0 }
    });

    // ---- Step 4: Place components on grid ----
    let col_spacing = match chains.len() {
        0..=2 => 7,
        3..=4 => 5,
        _     => 4,
    };
    let chain_start_y = 2;  // Leave room above for VCC rail

    // Battery: vertical on the far left
    if let Some(ref bname) = battery_name {
        if let Some(pos) = components.get_mut(bname) {
            pos.rotation = 1; // Vertical: plus at top, minus at bottom
            pos.x = 0;
            pos.y = 3;
        }
    }

    // Place each chain as a vertical column
    for (chain_idx, chain) in chains.iter().enumerate() {
        let axis_x = (chain_idx as i32 + 1) * col_spacing;
        let mut current_y = chain_start_y;

        for (comp_name, entry_pin) in chain {
            let pos = components.get_mut(comp_name).unwrap();

            if pos.comp_type == "Transistor" || pos.comp_type == "Mosfet" {
                // Through-pins c/e (or d/s) at relative (2,0) and (2,2)
                // Shift x by -2 so through-pins land on axis_x
                pos.x = axis_x - 2;
                pos.y = current_y;
                pos.rotation = 0;
            } else {
                // 2-pin vertical placement with orientation awareness:
                // entry_pin should be at TOP (VCC side), exit at BOTTOM (GND side)
                // rotation=1: p1 at top, p2 at bottom
                // rotation=3: p2 at top, p1 at bottom
                pos.x = axis_x;
                pos.y = current_y;
                if entry_pin == "p2" || entry_pin == "minus" {
                    pos.rotation = 3; // flip: entry pin (p2) at top
                } else {
                    pos.rotation = 1; // normal: entry pin (p1/plus) at top
                }
            }

            current_y += 3; // Pin span (2) + gap (1)
        }
    }

    // Normalize coordinates: shift so min position has padding
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

    // ---- Step 5: Route wires ----

    // Pre-compute pin world positions for each net
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
            // VCC rail: horizontal line ABOVE all pins, vertical stubs down
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
            // GND rail: horizontal line BELOW all pins, vertical stubs up
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
            // Signal nets: sequential L-shaped connections (pins sorted by x, then y)
            for i in 1..positions.len() {
                let (ax, ay) = positions[i - 1];
                let (bx, by) = positions[i];

                if ax == bx || ay == by {
                    // Same column or row → straight wire
                    wires.push(Wire { net_id: *net_id, points: vec![(ax, ay), (bx, by)] });
                } else {
                    // L-shaped: horizontal at source y, then vertical to target
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
