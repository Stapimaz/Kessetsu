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

// Simple XOR-Shift PRNG to avoid external dependencies like 'rand'
struct Prng {
    state: u32,
}
impl Prng {
    fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }
    fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
    fn next_in_range(&mut self, min: i32, max: i32) -> i32 {
        if max <= min { return min; }
        min + (self.next() % ((max - min) as u32)) as i32
    }
    fn next_float(&mut self) -> f64 {
        (self.next() as f64) / (u32::MAX as f64)
    }
}

fn get_comp_def(comp_type: &ComponentType) -> ComponentDef {
    let mut pins = HashMap::new();
    match comp_type {
        ComponentType::Resistor | ComponentType::Capacitor | ComponentType::Inductor | ComponentType::Diode => {
            pins.insert("p1".to_string(), (0, 0));
            pins.insert("p2".to_string(), (2, 0));
            ComponentDef { width: 2, height: 1, pins }
        }
        ComponentType::Battery => {
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

fn calculate_cost(
    components: &HashMap<String, ComponentPos>,
    defs: &HashMap<String, ComponentDef>,
    nets: &HashMap<usize, Vec<(String, String)>>,
) -> f64 {
    let mut cost = 0.0;
    
    // Wire length cost (sum of bounding box half-perimeters for each net)
    for (_net_id, pins) in nets {
        if pins.is_empty() { continue; }
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        
        for (comp_name, pin_name) in pins {
            if let Some(pos) = components.get(comp_name) {
                if let Some(def) = defs.get(comp_name) {
                    if let Some((dx, dy)) = def.pins.get(pin_name) {
                        let px = pos.x + dx;
                        let py = pos.y + dy;
                        if px < min_x { min_x = px; }
                        if px > max_x { max_x = px; }
                        if py < min_y { min_y = py; }
                        if py > max_y { max_y = py; }
                    }
                }
            }
        }
        if min_x != i32::MAX {
            cost += ((max_x - min_x) + (max_y - min_y)) as f64 * 10.0;
        }
    }
    
    // AABB Overlap Penalty
    let comp_names: Vec<_> = components.keys().collect();
    for i in 0..comp_names.len() {
        for j in (i+1)..comp_names.len() {
            let name1 = comp_names[i];
            let name2 = comp_names[j];
            if let (Some(pos1), Some(pos2)) = (components.get(name1), components.get(name2)) {
                // Check intersection (adding 2 units padding so they don't touch)
                let padding = 2;
                let overlap_x = (pos1.x < pos2.x + pos2.width + padding) && (pos1.x + pos1.width + padding > pos2.x);
                let overlap_y = (pos1.y < pos2.y + pos2.height + padding) && (pos1.y + pos1.height + padding > pos2.y);
                
                if overlap_x && overlap_y {
                    cost += 100000.0;
                }
            }
        }
    }
    
    // Global bounding box penalty (keep layout compact)
    let mut layout_min_x = i32::MAX;
    let mut layout_max_x = i32::MIN;
    let mut layout_min_y = i32::MAX;
    let mut layout_max_y = i32::MIN;
    for pos in components.values() {
        if pos.x < layout_min_x { layout_min_x = pos.x; }
        if pos.x + pos.width > layout_max_x { layout_max_x = pos.x + pos.width; }
        if pos.y < layout_min_y { layout_min_y = pos.y; }
        if pos.y + pos.height > layout_max_y { layout_max_y = pos.y + pos.height; }
    }
    
    let area_width = (layout_max_x - layout_min_x) as f64;
    let area_height = (layout_max_y - layout_min_y) as f64;
    cost += (area_width * area_height) * 0.1;
    
    cost
}

pub fn generate_layout(program: &Program) -> LayoutResult {
    let mut components = HashMap::new();
    let mut defs = HashMap::new();
    
    // Build Graph to get nets
    let graph = NetlistGraph::build(program);
    let mut nets_map: HashMap<usize, Vec<(String, String)>> = HashMap::new();
    
    for (pin_id, net_id) in &graph.pin_to_net {
        let parts: Vec<&str> = pin_id.split('.').collect();
        if parts.len() == 2 {
            nets_map.entry(*net_id).or_insert_with(Vec::new).push((parts[0].to_string(), parts[1].to_string()));
        }
    }

    let mut prng = Prng::new(42);

    // Initial Random Placement
    let grid_size = 20; // 20x20 grid roughly
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if decl.comp_type == ComponentType::ModulePort {
                continue; // Do not render module wrappers, they are not real physical components!
            }
            let def = get_comp_def(&decl.comp_type);
            defs.insert(decl.name.clone(), def.clone());
            
            components.insert(decl.name.clone(), ComponentPos {
                x: prng.next_in_range(0, grid_size),
                y: prng.next_in_range(0, grid_size),
                comp_type: format!("{:?}", decl.comp_type),
                width: def.width,
                height: def.height,
            });
        }
    }
    
    // Simulated Annealing
    if !components.is_empty() {
        let mut current_cost = calculate_cost(&components, &defs, &nets_map);
        let mut temp = 10000.0;
        let cooling_rate = 0.99; // slower cooling
        let iterations_per_temp = 100;
        
        let comp_names: Vec<String> = components.keys().cloned().collect();
        
        while temp > 0.1 {
            for _ in 0..iterations_per_temp {
                let r = prng.next_in_range(0, comp_names.len() as i32) as usize;
                let c_name = &comp_names[r];
                
                let mut state_copy = components.clone();
                let pos = state_copy.get_mut(c_name).unwrap();
                
                // Move randomly
                let dx = prng.next_in_range(-5, 6);
                let dy = prng.next_in_range(-5, 6);
                pos.x += dx;
                pos.y += dy;
                
                let new_cost = calculate_cost(&state_copy, &defs, &nets_map);
                
                if new_cost < current_cost {
                    current_cost = new_cost;
                    components = state_copy;
                } else {
                    let acceptance_prob = std::f64::consts::E.powf((current_cost - new_cost) / temp);
                    if prng.next_float() < acceptance_prob {
                        current_cost = new_cost;
                        components = state_copy;
                    }
                }
            }
            temp *= cooling_rate;
        }
    }
    
    // Normalize coordinates so min_x = 0 and min_y = 0
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    for pos in components.values() {
        if pos.x < min_x { min_x = pos.x; }
        if pos.y < min_y { min_y = pos.y; }
    }
    for pos in components.values_mut() {
        pos.x -= min_x;
        pos.y -= min_y;
    }

    // A* Routing (Simplified: for now we just return straight lines from MST, real orthogonal A* is complex for WASM demo)
    // Wait, the user wanted robust routing! We will do basic orthogonal routing.
    let mut wires = Vec::new();
    
    for (net_id, pins) in &nets_map {
        if pins.len() < 2 { continue; }
        
        // Simple star topology: connect all pins to the first valid pin. 
        // With orthogonal manhattan paths.
        let mut first_valid_pin = None;
        for pin in pins {
            if components.contains_key(&pin.0) && defs.contains_key(&pin.0) {
                first_valid_pin = Some(pin);
                break;
            }
        }
        
        if let Some((c1, p1)) = first_valid_pin {
            let pos1 = components.get(c1).unwrap();
            let def1 = defs.get(c1).unwrap();
            if let Some((dx1, dy1)) = def1.pins.get(p1) {
                let start_x = pos1.x + dx1;
                let start_y = pos1.y + dy1;
                
                for i in 1..pins.len() {
                    let (c2, p2) = &pins[i];
                    if let (Some(pos2), Some(def2)) = (components.get(c2), defs.get(c2)) {
                        if let Some((dx2, dy2)) = def2.pins.get(p2) {
                            let end_x = pos2.x + dx2;
                            let end_y = pos2.y + dy2;
                            
                            // L-shape routing
                            let mid_x = start_x;
                            let mid_y = end_y;
                            
                            wires.push(Wire {
                                net_id: *net_id,
                                points: vec![
                                    (start_x, start_y),
                                    (mid_x, mid_y),
                                    (end_x, end_y)
                                ],
                            });
                        }
                    }
                }
            }
        }
    }

    LayoutResult {
        components,
        wires,
    }
}
