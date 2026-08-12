use crate::component::component_definition;
use crate::ir::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NetId(pub usize);

impl NetId {
    pub const GROUND: Self = Self(0);
}

impl std::fmt::Display for NetId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

pub struct NetlistGraph {
    pub pin_to_net: HashMap<String, NetId>,
    pub net_names: HashMap<NetId, String>,
    pub ground_candidates: Vec<String>,
    pub ground_is_explicit: bool,
    pub net_name_conflicts: Vec<NetNameConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetNameConflict {
    pub net: NetId,
    pub names: Vec<String>,
}

impl NetlistGraph {
    pub fn build(circuit: &CircuitIR) -> Self {
        let mut pin_to_net = HashMap::new();
        let mut next_net_id = 1;

        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_pins = HashSet::new();

        for comp in &circuit.components {
            for pin in component_definition(&comp.kind).pins {
                all_pins.insert(format!("{}.{}", comp.id, pin.name));
            }
        }

        for conn in &circuit.connections {
            if conn.pins.len() < 2 {
                continue;
            }

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
                for j in (i + 1)..pin_ids.len() {
                    adjacency
                        .entry(pin_ids[i].clone())
                        .or_default()
                        .push(pin_ids[j].clone());
                    adjacency
                        .entry(pin_ids[j].clone())
                        .or_default()
                        .push(pin_ids[i].clone());
                }
            }
        }

        let mut ordered_pins: Vec<_> = all_pins.into_iter().collect();
        ordered_pins.sort();

        let mut visited = HashSet::new();
        for pin in &ordered_pins {
            if !visited.contains(pin) {
                let current_net = NetId(next_net_id);
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

        let mut explicit_ground_by_net: HashMap<NetId, String> = HashMap::new();
        for ground_name in circuit
            .nets
            .iter()
            .filter(|name| name.eq_ignore_ascii_case("gnd"))
        {
            if let Some(net) = pin_to_net.get(ground_name) {
                explicit_ground_by_net
                    .entry(*net)
                    .and_modify(|existing| {
                        if ground_name < existing {
                            *existing = ground_name.clone();
                        }
                    })
                    .or_insert_with(|| ground_name.clone());
            }
        }

        let ground_is_explicit = !explicit_ground_by_net.is_empty();
        let mut ground_candidates: Vec<(String, NetId)> = if ground_is_explicit {
            explicit_ground_by_net
                .into_iter()
                .map(|(net, name)| (name, net))
                .collect()
        } else {
            let mut source_minus_by_net: HashMap<NetId, String> = HashMap::new();
            for (pin, net) in &pin_to_net {
                if pin.ends_with(".minus") {
                    source_minus_by_net
                        .entry(*net)
                        .and_modify(|existing| {
                            if pin < existing {
                                *existing = pin.clone();
                            }
                        })
                        .or_insert_with(|| pin.clone());
                }
            }
            source_minus_by_net
                .into_iter()
                .map(|(net, pin)| (pin, net))
                .collect()
        };
        ground_candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let ground_net = ground_candidates.first().map(|(_, net)| *net);
        let ground_candidate_names = ground_candidates
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        if let Some(gnd) = ground_net {
            for net in pin_to_net.values_mut() {
                if *net == gnd {
                    *net = NetId::GROUND;
                }
            }
        }

        let mut net_names = HashMap::new();
        net_names.insert(NetId::GROUND, "0".to_string());

        let mut net_to_pins: HashMap<NetId, Vec<String>> = HashMap::new();
        for (pin, net) in &pin_to_net {
            net_to_pins.entry(*net).or_default().push(pin.clone());
        }

        let mut net_name_conflicts = Vec::new();
        for (net, mut pins) in net_to_pins {
            pins.sort();

            let mut user_names: Vec<_> = pins
                .iter()
                .filter(|pin| circuit.nets.contains(*pin))
                .cloned()
                .collect();
            user_names.sort();
            user_names.dedup();
            if user_names.len() > 1 {
                net_name_conflicts.push(NetNameConflict {
                    net,
                    names: user_names.clone(),
                });
            }

            if net == NetId::GROUND {
                continue;
            }
            if let Some(name) = user_names.first() {
                net_names.insert(net, name.clone());
            } else if let Some(first_pin) = pins.first() {
                let name = format!("N_{}", first_pin.replace(".", "_"));
                net_names.insert(net, name);
            }
        }

        NetlistGraph {
            pin_to_net,
            net_names,
            ground_candidates: ground_candidate_names,
            ground_is_explicit,
            net_name_conflicts,
        }
    }

    pub fn get_net(&self, component: &str, pin: &str) -> Option<NetId> {
        let pin_id = format!("{}.{}", component, pin);
        self.pin_to_net.get(&pin_id).copied()
    }

    pub fn get_net_name(&self, net_id: NetId) -> String {
        self.net_names
            .get(&net_id)
            .cloned()
            .unwrap_or_else(|| format!("{}", net_id))
    }
}

fn get_standard_model(name: &str) -> Option<&'static str> {
    match name.to_uppercase().as_str() {
        "2N3904" => Some(
            ".model 2N3904 NPN (Is=6.734f Xti=3 Eg=1.11 Vaf=74.03 Bf=416.4 Ne=1.259 Ise=6.734f Ikf=66.78m Xtb=1.5 Br=.7371 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=3.638p Mjc=.3085 Vjc=.75 Fc=.5 Cje=4.493p Mje=.2593 Vje=.75 Tr=239.5n Tf=301.2p Itf=.4 Vtf=4 Xtf=2 Rb=10)",
        ),
        "2N3906" => Some(
            ".model 2N3906 PNP (Is=1.41f Xti=3 Eg=1.11 Vaf=18.7 Bf=227.3 Ne=1.5 Ise=0 Ikf=80m Xtb=1.5 Br=4.977 Nc=2 Isc=0 Ikr=0 Rc=2.5 Cjc=9.728p Mjc=.5776 Vjc=.75 Fc=.5 Cje=8.063p Mje=.3677 Vje=.75 Tr=33.42n Tf=179.3p Itf=.4 Vtf=4 Xtf=6 Rb=10)",
        ),
        "2N2222" => Some(
            ".model 2N2222 NPN (Is=14.34f Xti=3 Eg=1.11 Vaf=74.03 Bf=255.9 Ne=1.307 Ise=14.34f Ikf=.2847 Xtb=1.5 Br=6.092 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=7.306p Mjc=.3416 Vjc=.75 Fc=.5 Cje=22.01p Mje=.377 Vje=.75 Tr=46.91n Tf=411.1p Itf=.6 Vtf=1.7 Xtf=3 Rb=10)",
        ),
        "1N4148" => Some(
            ".model 1N4148 D (Is=2.52n Rs=.568 N=1.752 Cjo=4p M=.4 tt=20n Iave=200m Vpk=75 mfg=OnSemi type=silicon)",
        ),
        "1N4007" => Some(
            ".model 1N4007 D (Is=7.02767n Rs=0.0341512 N=1.80803 Cjo=10p M=0.3333 VJ=0.75 Iave=1 Vpk=1000 mfg=Motorola type=silicon)",
        ),
        "IRF540" => Some(
            ".model IRF540 VDMOS (Rg=3 Vto=4.0 Rd=45m Rs=12m Rb=10m Kp=18 Cgdmax=2n Cgdmin=1.3n Cgs=1.7n Cjo=1n Is=2p mfg=IR)",
        ),
        _ => None,
    }
}

fn format_spice_value(comp: &IRComponent) -> String {
    if let Some(m) = &comp.model {
        return m.name.clone();
    }
    match &comp.parameters {
        ComponentParams::TwoPinPassive { value } => format_spice_number(value.value),
        ComponentParams::VoltageSource { value } | ComponentParams::CurrentSource { value } => {
            match value {
                SourceValue::Dc(quantity) => format_spice_number(quantity.value),
                SourceValue::Waveform(waveform) => format_waveform(waveform),
            }
        }
        _ => "".to_string(),
    }
}

pub fn format_spice_number(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }

    let absolute = value.abs();
    if (1e-3..1e6).contains(&absolute) {
        return format!("{value:.15}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
    }

    let scientific = format!("{value:.12e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("Rust scientific formatting must include an exponent");
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    let exponent = exponent
        .parse::<i32>()
        .expect("Rust scientific exponent must be an integer");
    format!("{mantissa}e{exponent}")
}

fn format_analysis(analysis: &Analysis, circuit: &CircuitIR) -> String {
    match analysis {
        Analysis::OperatingPoint => "op".to_string(),
        Analysis::Transient { step, stop } => format!(
            "tran {} {}",
            format_spice_number(step.value),
            format_spice_number(stop.value)
        ),
        Analysis::Ac {
            scale,
            points,
            start,
            stop,
        } => {
            let scale = match scale {
                AcScale::Decade => "dec",
                AcScale::Octave => "oct",
                AcScale::Linear => "lin",
            };
            format!(
                "ac {scale} {points} {} {}",
                format_spice_number(start.value),
                format_spice_number(stop.value)
            )
        }
        Analysis::DcSweep {
            source,
            start,
            stop,
            step,
        } => {
            let source_kind = circuit
                .components
                .iter()
                .find(|component| component.id == *source)
                .map(|component| &component.kind)
                .expect("typed DC sweep source must exist in Circuit IR");
            let prefix = match source_kind {
                ComponentKind::VoltageSource => "V_",
                ComponentKind::CurrentSource => "I_",
                _ => unreachable!("typed DC sweep source must be an independent source"),
            };
            format!(
                "dc {prefix}{source} {} {} {}",
                format_spice_number(start.value),
                format_spice_number(stop.value),
                format_spice_number(step.value)
            )
        }
    }
}

fn format_waveform(waveform: &Waveform) -> String {
    match waveform {
        Waveform::Sine {
            offset,
            amplitude,
            frequency,
        } => format!(
            "SINE({} {} {})",
            format_spice_number(offset.value),
            format_spice_number(amplitude.value),
            format_spice_number(frequency.value)
        ),
        Waveform::Pulse {
            v1,
            v2,
            delay,
            rise,
            fall,
            width,
            period,
        } => format!(
            "PULSE({} {} {} {} {} {} {})",
            format_spice_number(v1.value),
            format_spice_number(v2.value),
            format_spice_number(delay.value),
            format_spice_number(rise.value),
            format_spice_number(fall.value),
            format_spice_number(width.value),
            format_spice_number(period.value)
        ),
        Waveform::PWL { points } => {
            let values = points
                .iter()
                .map(|(time, value)| {
                    format!(
                        "{} {}",
                        format_spice_number(time.value),
                        format_spice_number(value.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("PWL({values})")
        }
    }
}

fn component_net_names(graph: &NetlistGraph, component: &IRComponent) -> Vec<String> {
    component_definition(&component.kind)
        .pins
        .iter()
        .map(|pin| {
            graph
                .get_net(&component.id, pin.name)
                .map(|net| graph.get_net_name(net))
                .unwrap_or_else(|| format!("NC_{}_{}", component.id, pin.name))
        })
        .collect()
}

pub fn generate_spice(circuit: &CircuitIR, graph: &NetlistGraph) -> String {
    let mut spice = String::from("* NetLang Generated SPICE Netlist\n");
    let mut used_models = BTreeSet::new();

    let mut components: Vec<_> = circuit.components.iter().collect();
    components.sort_by(|left, right| left.id.cmp(&right.id));

    for comp in components {
        let value_str = format_spice_value(comp);
        let definition = component_definition(&comp.kind);
        let nets = component_net_names(graph, comp);
        match &comp.kind {
            ComponentKind::BJT(_) => {
                spice.push_str(&format!(
                    "Q_{} {} {} {} {}\n",
                    comp.id, nets[0], nets[1], nets[2], value_str
                ));
            }
            ComponentKind::MOSFET(_) => {
                spice.push_str(&format!(
                    "M_{} {} {} {} {} {}\n",
                    comp.id, nets[0], nets[1], nets[2], nets[2], value_str
                ));
            }
            ComponentKind::OpAmp => {
                spice.push_str(&format!(
                    "X_{} {} {} {} {} {} {}\n",
                    comp.id, nets[0], nets[1], nets[2], nets[3], nets[4], value_str
                ));
            }
            ComponentKind::Resistor
            | ComponentKind::Capacitor
            | ComponentKind::Inductor
            | ComponentKind::Diode
            | ComponentKind::VoltageSource
            | ComponentKind::CurrentSource => {
                let prefix = definition
                    .spice_prefix
                    .expect("emitted components must define a SPICE prefix");
                spice.push_str(&format!(
                    "{}_{} {} {} {}\n",
                    prefix, comp.id, nets[0], nets[1], value_str
                ));
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
    for analysis in &circuit.analyses {
        has_sim = true;
        control_block.push_str(&format_analysis(analysis, circuit));
        control_block.push('\n');
    }

    let mut net_counts: HashMap<NetId, usize> = HashMap::new();
    let mut nc_nets: HashSet<NetId> = HashSet::new();

    for (pin, net) in &graph.pin_to_net {
        *net_counts.entry(*net).or_insert(0) += 1;
        if pin.starts_with("nc.") || pin == "nc" {
            nc_nets.insert(*net);
        }
    }

    let mut dangling_net_names: Vec<_> = net_counts
        .into_iter()
        .filter(|(net, count)| *net != NetId::GROUND && (*count == 1 || nc_nets.contains(net)))
        .map(|(net, _)| graph.get_net_name(net))
        .collect();
    dangling_net_names.sort();

    for (dummy_count, net_name) in dangling_net_names.into_iter().enumerate() {
        spice.push_str(&format!("R_dummy_{} {} 0 1G\n", dummy_count, net_name));
    }

    let mut main_analysis = "tran";
    if has_sim {
        if let Some(analysis) = circuit.analyses.first() {
            main_analysis = analysis.kind_name();
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
                let inside = &sp_signal[2..sp_signal.len() - 1];
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
                control_block.push_str(&format!(
                    "meas {} {} FIND {} AT=0\n",
                    main_analysis, safe_name, sp_signal
                ));
            } else {
                control_block.push_str(&format!(
                    "meas {} {} {} {}\n",
                    main_analysis, safe_name, metric, sp_signal
                ));
            }
        }

        control_block.push_str("print all\nquit\n.endc\n");
        spice.push_str(&control_block);
    }
    spice
}
