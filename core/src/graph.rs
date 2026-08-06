use crate::ast::*;
use std::collections::{HashMap, HashSet};

pub struct NetlistGraph {
    pub pin_to_net: HashMap<String, usize>,
    pub net_names: HashMap<usize, String>,
}

impl NetlistGraph {
    pub fn build(program: &Program) -> Self {
        let mut pin_to_net = HashMap::new();
        let mut next_net_id = 1;
        
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_pins = HashSet::new();

        // Ensure all component pins exist in all_pins (even if not connected)
        for stmt in &program.statements {
            if let Statement::Decl(decl) = stmt {
                let pins: Vec<&str> = match decl.comp_type {
                    ComponentType::Source => vec!["plus", "minus"],
                    ComponentType::Transistor => vec!["c", "b", "e"],
                    ComponentType::Mosfet => vec!["d", "g", "s"],
                    ComponentType::OpAmp => vec!["in_p", "in_n", "out", "vcc", "vee"],
                    ComponentType::ModulePort => continue,
                    _ => vec!["p1", "p2"],
                };
                for pin in pins {
                    all_pins.insert(format!("{}.{}", decl.name, pin));
                }
            }
        }

        for stmt in &program.statements {
            if let Statement::Connect(conn) = stmt {
                let pin1_id = format!("{}.{}", conn.pin1.component, conn.pin1.pin);
                let pin2_id = format!("{}.{}", conn.pin2.component, conn.pin2.pin);
                
                adjacency.entry(pin1_id.clone()).or_insert_with(Vec::new).push(pin2_id.clone());
                adjacency.entry(pin2_id.clone()).or_insert_with(Vec::new).push(pin1_id.clone());
                
                all_pins.insert(pin1_id);
                all_pins.insert(pin2_id);
            }
        }

        // Flood fill to assign net_ids
        let mut visited = HashSet::new();
        for pin in &all_pins {
            if !visited.contains(pin) {
                // If the pin has no connections, it doesn't get a real net id (it remains floating)
                // Actually, let's assign a unique net to everything.
                // Or maybe we don't assign a net if it's completely isolated?
                // SPICE needs every pin to be connected to SOMETHING, or it's an error.
                // Let's assign unique nets.
                let current_net = next_net_id;
                next_net_id += 1;
                
                let mut stack = vec![pin.clone()];
                let mut connected_count = 0;
                
                while let Some(curr) = stack.pop() {
                    if !visited.contains(&curr) {
                        visited.insert(curr.clone());
                        pin_to_net.insert(curr.clone(), current_net);
                        connected_count += 1;
                        
                        if let Some(neighbors) = adjacency.get(&curr) {
                            for neighbor in neighbors {
                                if !visited.contains(neighbor) {
                                    stack.push(neighbor.clone());
                                }
                            }
                        }
                    }
                }
                
                // If the pin is isolated (no connections), remove it from pin_to_net
                // so that it returns 9999 and triggers the DRC Floating Pin error.
                if connected_count == 1 {
                    pin_to_net.remove(pin);
                }
            }
        }

        // Force ground net (0) for battery minus
        let mut ground_net = None;
        for (pin, net) in &pin_to_net {
            if pin.ends_with(".minus") {
                ground_net = Some(*net);
                break;
            }
        }

        if let Some(gnd) = ground_net {
            for (_, net) in pin_to_net.iter_mut() {
                if *net == gnd {
                    *net = 0;
                }
            }
        }

        // Generate robust net names
        let mut net_names = HashMap::new();
        net_names.insert(0, "0".to_string());
        
        let mut net_to_pins: HashMap<usize, Vec<String>> = HashMap::new();
        for (pin, net) in &pin_to_net {
            if *net != 0 {
                net_to_pins.entry(*net).or_default().push(pin.clone());
            }
        }
        
        for (net, mut pins) in net_to_pins {
            pins.sort();
            if let Some(first_pin) = pins.first() {
                let name = format!("N_{}", first_pin.replace(".", "_"));
                net_names.insert(net, name);
            }
        }

        NetlistGraph {
            pin_to_net,
            net_names,
        }
    }

    pub fn get_net(&self, component: &str, pin: &str) -> usize {
        let pin_id = format!("{}.{}", component, pin);
        *self.pin_to_net.get(&pin_id).unwrap_or(&9999)
    }
    
    pub fn get_net_name(&self, net_id: usize) -> String {
        self.net_names.get(&net_id).cloned().unwrap_or_else(|| format!("{}", net_id))
    }
}

fn get_standard_model(name: &str) -> Option<&'static str> {
    match name.to_uppercase().as_str() {
        "2N3904" => Some(".model 2N3904 NPN (Is=6.734f Xti=3 Eg=1.11 Vaf=74.03 Bf=416.4 Ne=1.259 Ise=6.734f Ikf=66.78m Xtb=1.5 Br=.7371 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=3.638p Mjc=.3085 Vjc=.75 Fc=.5 Cje=4.493p Mje=.2593 Vje=.75 Tr=239.5n Tf=301.2p Itf=.4 Vtf=4 Xtf=2 Rb=10)"),
        "2N3906" => Some(".model 2N3906 PNP (Is=1.41f Xti=3 Eg=1.11 Vaf=18.7 Bf=227.3 Ne=1.5 Ise=0 Ikf=80m Xtb=1.5 Br=4.977 Nc=2 Isc=0 Ikr=0 Rc=2.5 Cjc=9.728p Mjc=.5776 Vjc=.75 Fc=.5 Cje=8.063p Mje=.3677 Vje=.75 Tr=33.42n Tf=179.3p Itf=.4 Vtf=4 Xtf=6 Rb=10)"),
        "2N2222" => Some(".model 2N2222 NPN (Is=14.34f Xti=3 Eg=1.11 Vaf=74.03 Bf=255.9 Ne=1.307 Ise=14.34f Ikf=.2847 Xtb=1.5 Br=6.092 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=7.306p Mjc=.3416 Vjc=.75 Fc=.5 Cje=22.01p Mje=.377 Vje=.75 Tr=46.91n Tf=411.1p Itf=.6 Vtf=1.7 Xtf=3 Rb=10)"),
        "1N4148" => Some(".model 1N4148 D (Is=2.52n Rs=.568 N=1.752 Cjo=4p M=.4 tt=20n Iave=200m Vpk=75 mfg=OnSemi type=silicon)"),
        "1N4007" => Some(".model 1N4007 D (Is=7.02767n Rs=0.0341512 N=1.80803 Cjo=10p M=0.3333 VJ=0.75 Iave=1 Vpk=1000 mfg=Motorola type=silicon)"),
        "IRF540" => Some(".model IRF540 VDMOS (Rg=3 Vto=4.0 Rd=45m Rs=12m Rb=10m Kp=18 Cgdmax=2n Cgdmin=1.3n Cgs=1.7n Cjo=1n Is=2p mfg=IR)"),
        _ => None,
    }
}

pub fn generate_spice(program: &Program, graph: &NetlistGraph) -> String {
    let mut spice = String::from("* NetLang Generated SPICE Netlist\n");
    let mut used_models = HashSet::new();

    
    for stmt in &program.statements {
        if let Statement::Decl(decl) = stmt {
            if decl.comp_type == ComponentType::ModulePort {
                continue;
            }

            match decl.comp_type {
                ComponentType::Transistor => {
                    let nc = graph.get_net_name(graph.get_net(&decl.name, "c"));
                    let nb = graph.get_net_name(graph.get_net(&decl.name, "b"));
                    let ne = graph.get_net_name(graph.get_net(&decl.name, "e"));
                    spice.push_str(&format!("Q_{} {} {} {} {}\n", decl.name, nc, nb, ne, decl.value));
                }
                ComponentType::Mosfet => {
                    let nd = graph.get_net_name(graph.get_net(&decl.name, "d"));
                    let ng = graph.get_net_name(graph.get_net(&decl.name, "g"));
                    let ns = graph.get_net_name(graph.get_net(&decl.name, "s"));
                    spice.push_str(&format!("M_{} {} {} {} {} {}\n", decl.name, nd, ng, ns, ns, decl.value));
                }
                ComponentType::OpAmp => {
                    let np = graph.get_net_name(graph.get_net(&decl.name, "in_p"));
                    let nn = graph.get_net_name(graph.get_net(&decl.name, "in_n"));
                    let vcc = graph.get_net_name(graph.get_net(&decl.name, "vcc"));
                    let vee = graph.get_net_name(graph.get_net(&decl.name, "vee"));
                    let out = graph.get_net_name(graph.get_net(&decl.name, "out"));
                    spice.push_str(&format!("X_{} {} {} {} {} {} {}\n", decl.name, np, nn, vcc, vee, out, decl.value));
                }
                _ => {
                    let (p1, p2, prefix) = match decl.comp_type {
                        ComponentType::Resistor => ("p1", "p2", "R"),
                        ComponentType::Source => ("plus", "minus", "V"),
                        ComponentType::Capacitor => ("p1", "p2", "C"),
                        ComponentType::Inductor => ("p1", "p2", "L"),
                        ComponentType::Diode => ("p1", "p2", "D"),
                        _ => unreachable!(),
                    };
                    let net1 = graph.get_net_name(graph.get_net(&decl.name, p1));
                    let net2 = graph.get_net_name(graph.get_net(&decl.name, p2));
                    spice.push_str(&format!("{}_{} {} {} {}\n", prefix, decl.name, net1, net2, decl.value));
                }
            }
            if let Some(model_str) = get_standard_model(&decl.value) {
                used_models.insert(model_str);
            }
        }
    }
    
    // Add models
    if !used_models.is_empty() {
        spice.push_str("\n* Standard Models\n");
        for model in used_models {
            spice.push_str(model);
            spice.push('\n');
        }
    }
    
    // Add simulation control blocks
    let mut has_sim = false;
    let mut control_block = String::from("\n.control\n");
    for stmt in &program.statements {
        if let Statement::Simulate(sim) = stmt {
            has_sim = true;
            let args_str = sim.args.join(" ");
            control_block.push_str(&format!("{} {}\n", sim.cmd, args_str));
        }
    }
    
    if has_sim {
        control_block.push_str("print all\nquit\n.endc\n");
        spice.push_str(&control_block);
    }

    spice
}
