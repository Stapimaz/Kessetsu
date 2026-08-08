use crate::ir::*;
use std::collections::{HashMap, HashSet};

pub struct NetlistGraph {
    pub pin_to_net: HashMap<String, usize>,
    pub net_names: HashMap<usize, String>,
}

impl NetlistGraph {
    pub fn build(circuit: &CircuitIR) -> Self {
        let mut pin_to_net = HashMap::new();
        let mut next_net_id = 1;
        
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_pins = HashSet::new();

        for comp in &circuit.components {
            let pins: Vec<&str> = match comp.kind {
                ComponentKind::VoltageSource | ComponentKind::CurrentSource => vec!["plus", "minus"],
                ComponentKind::BJT(_) => vec!["c", "b", "e"],
                ComponentKind::MOSFET(_) => vec!["d", "g", "s"],
                ComponentKind::OpAmp => vec!["in_p", "in_n", "out", "vcc", "vee"],
                _ => vec!["p1", "p2"],
            };
            for pin in pins {
                all_pins.insert(format!("{}.{}", comp.id, pin));
            }
        }

        for conn in &circuit.connections {
            if conn.pins.len() < 2 { continue; }
            
            let mut pin_ids = Vec::new();
            for p in &conn.pins {
                let pid = if p.component.is_empty() {
                    p.pin.clone()
                } else {
                    format!("{}.{}", p.component, p.pin)
                };
                pin_ids.push(pid.clone());
                all_pins.insert(pid);
            }
            
            for i in 0..pin_ids.len() {
                for j in (i+1)..pin_ids.len() {
                    adjacency.entry(pin_ids[i].clone()).or_insert_with(Vec::new).push(pin_ids[j].clone());
                    adjacency.entry(pin_ids[j].clone()).or_insert_with(Vec::new).push(pin_ids[i].clone());
                }
            }
        }

        let mut visited = HashSet::new();
        for pin in &all_pins {
            if !visited.contains(pin) {
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
                
                if connected_count == 1 {
                    pin_to_net.remove(pin);
                }
            }
        }

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
            
            let mut user_name = None;
            for p in &pins {
                if circuit.nets.contains(p) {
                    user_name = Some(p.clone());
                    break;
                }
            }
            
            if let Some(name) = user_name {
                net_names.insert(net, name);
            } else if let Some(first_pin) = pins.first() {
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

fn format_spice_value(comp: &IRComponent) -> String {
    if let Some(m) = &comp.model {
        return m.name.clone();
    }
    match &comp.parameters {
        ComponentParams::TwoPinPassive { value, .. } => format!("{}", value),
        ComponentParams::DCSource { voltage } => format!("{}", voltage),
        ComponentParams::ACSource { waveform } => {
            match waveform {
                Waveform::Sine { offset, amplitude, frequency } => format!("SINE({} {} {})", offset, amplitude, frequency),
                _ => "0".to_string(),
            }
        },
        ComponentParams::Unknown { original_value } => original_value.clone(),
        _ => "".to_string(),
    }
}

pub fn generate_spice(circuit: &CircuitIR, graph: &NetlistGraph) -> String {
    let mut spice = String::from("* NetLang Generated SPICE Netlist\n");
    let mut used_models = HashSet::new();

    for comp in &circuit.components {
        let value_str = format_spice_value(comp);
        match &comp.kind {
            ComponentKind::BJT(_) => {
                let nc = graph.get_net_name(graph.get_net(&comp.id, "c"));
                let nb = graph.get_net_name(graph.get_net(&comp.id, "b"));
                let ne = graph.get_net_name(graph.get_net(&comp.id, "e"));
                spice.push_str(&format!("Q_{} {} {} {} {}\n", comp.id, nc, nb, ne, value_str));
            }
            ComponentKind::MOSFET(_) => {
                let nd = graph.get_net_name(graph.get_net(&comp.id, "d"));
                let ng = graph.get_net_name(graph.get_net(&comp.id, "g"));
                let ns = graph.get_net_name(graph.get_net(&comp.id, "s"));
                spice.push_str(&format!("M_{} {} {} {} {} {}\n", comp.id, nd, ng, ns, ns, value_str));
            }
            ComponentKind::OpAmp => {
                let np = graph.get_net_name(graph.get_net(&comp.id, "in_p"));
                let nn = graph.get_net_name(graph.get_net(&comp.id, "in_n"));
                let vcc = graph.get_net_name(graph.get_net(&comp.id, "vcc"));
                let vee = graph.get_net_name(graph.get_net(&comp.id, "vee"));
                let out = graph.get_net_name(graph.get_net(&comp.id, "out"));
                spice.push_str(&format!("X_{} {} {} {} {} {} {}\n", comp.id, np, nn, vcc, vee, out, value_str));
            }
            ComponentKind::Resistor | ComponentKind::Capacitor | ComponentKind::Inductor | ComponentKind::Diode | ComponentKind::VoltageSource | ComponentKind::CurrentSource => {
                let (p1, p2, prefix) = match &comp.kind {
                    ComponentKind::Resistor => ("p1", "p2", "R"),
                    ComponentKind::VoltageSource => ("plus", "minus", "V"),
                    ComponentKind::CurrentSource => ("plus", "minus", "I"),
                    ComponentKind::Capacitor => ("p1", "p2", "C"),
                    ComponentKind::Inductor => ("p1", "p2", "L"),
                    ComponentKind::Diode => ("p1", "p2", "D"),
                    _ => unreachable!(),
                };
                let net1 = graph.get_net_name(graph.get_net(&comp.id, p1));
                let net2 = graph.get_net_name(graph.get_net(&comp.id, p2));
                spice.push_str(&format!("{}_{} {} {} {}\n", prefix, comp.id, net1, net2, value_str));
            }
            ComponentKind::ModulePort => {}
        }
        if let Some(model_str) = get_standard_model(&value_str) {
            used_models.insert(model_str);
        }
    }
    
    if !used_models.is_empty() {
        spice.push_str("\n* Standard Models\n");
        for model in used_models {
            spice.push_str(model);
            spice.push('\n');
        }
    }
    
    let mut has_sim = false;
    let mut control_block = String::from("\n.control\n");
    for sim in &circuit.analyses {
        has_sim = true;
        let args_str = sim.args.join(" ");
        control_block.push_str(&format!("{} {}\n", sim.cmd, args_str));
    }
    
    let mut dummy_count = 0;
    let mut net_counts: HashMap<usize, usize> = HashMap::new();
    let mut nc_nets: HashSet<usize> = HashSet::new();
    
    for (pin, net) in &graph.pin_to_net {
        *net_counts.entry(*net).or_insert(0) += 1;
        if pin.starts_with("nc.") || pin == "nc" {
            nc_nets.insert(*net);
        }
    }
    
    for (net, count) in net_counts {
        if net != 0 && (count == 1 || nc_nets.contains(&net)) {
            let net_name = graph.get_net_name(net);
            spice.push_str(&format!("R_dummy_{} {} 0 1G\n", dummy_count, net_name));
            dummy_count += 1;
        }
    }

    let mut main_analysis = "tran";
    if has_sim {
        for sim in &circuit.analyses {
            let cmd = sim.cmd.to_lowercase();
            if cmd == "tran" || cmd == "dc" || cmd == "ac" || cmd == "op" {
                main_analysis = Box::leak(cmd.into_boxed_str());
                break;
            }
        }
        
        for assert in &circuit.assertions {
            let raw_name = format!("{}_{}", assert.metric, assert.signal);
            let safe_name = raw_name.replace("(", "_").replace(")", "").to_lowercase();
            let metric = match assert.metric.to_uppercase().as_str() {
                "MAX" => "MAX",
                "MIN" => "MIN",
                "PEAK" => "MAX", // Ngspice MAX is peak positive, PP is peak-to-peak
                "RMS" => "RMS",
                _ => "MAX",
            };
            
            // Note: `op` does not support MAX/MIN/RMS measurements over time.
            // If main_analysis is op, we should use FIND instead or just use DC eval.
            // For now, if metric is MAX/MIN/RMS we assume we need to use it.
            // If it's op, ngspice .meas op expects `FIND v(node) AT=0` or similar, 
            // but for simplicity we'll just output the metric.
            let mut sp_signal = assert.signal.clone();
            if sp_signal.to_uppercase().starts_with("I(") {
                let inside = &sp_signal[2..sp_signal.len()-1];
                let prefix = match inside.chars().next() {
                    Some('R') | Some('r') => "R_",
                    Some('V') | Some('v') => "V_",
                    Some('I') | Some('i') => "I_",
                    Some('C') | Some('c') => "C_",
                    Some('L') | Some('l') => "L_",
                    Some('D') | Some('d') => "D_",
                    Some('Q') | Some('q') => "Q_",
                    Some('M') | Some('m') => "M_",
                    Some('X') | Some('x') => "X_",
                    _ => "",
                };
                sp_signal = format!("I({}{})", prefix, inside);
            }
            
            if main_analysis == "op" {
                control_block.push_str(&format!("meas {} {} FIND {} AT=0\n", main_analysis, safe_name, sp_signal));
            } else {
                control_block.push_str(&format!("meas {} {} {} {}\n", main_analysis, safe_name, metric, sp_signal));
            }
        }

        control_block.push_str("print all\nquit\n.endc\n");
        spice.push_str(&control_block);
    }
    spice
}
