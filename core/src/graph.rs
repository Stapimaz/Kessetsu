use crate::component::component_definition;
use crate::ir::*;
use crate::simulation::analysis_data_filename;
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

pub fn format_analysis(analysis: &Analysis, circuit: &CircuitIR) -> String {
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

pub fn generate_browser_analysis_netlist(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    analysis: &Analysis,
) -> String {
    let generated = generate_spice(circuit, graph);
    let mut netlist = generated
        .split_once("\n.control\n")
        .map_or(generated.as_str(), |(base, _)| base)
        .trim_end()
        .to_string();
    let device_currents = circuit
        .components
        .iter()
        .filter_map(|component| match component.kind {
            ComponentKind::Diode => Some(format!("@D_{}[id]", component.id)),
            ComponentKind::BJT(_) => Some(format!("@Q_{}[ic]", component.id)),
            ComponentKind::MOSFET(_) => Some(format!("@M_{}[id]", component.id)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if !device_currents.is_empty() {
        netlist.push_str("\n.save all ");
        netlist.push_str(&device_currents.into_iter().collect::<Vec<_>>().join(" "));
    }
    netlist.push_str("\n.");
    netlist.push_str(&format_analysis(analysis, circuit));
    netlist.push_str("\n.end\n");
    netlist
}

fn format_waveform(waveform: &Waveform) -> String {
    match waveform {
        Waveform::Ac { amplitude } => {
            format!("AC {}", format_spice_number(amplitude.value))
        }
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
        Waveform::SineAc {
            offset,
            amplitude,
            frequency,
            ac_amplitude,
        } => format!(
            "SINE({} {} {}) AC {}",
            format_spice_number(offset.value),
            format_spice_number(amplitude.value),
            format_spice_number(frequency.value),
            format_spice_number(ac_amplitude.value)
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
    let mut spice = String::from("* Kessetsu Generated SPICE Netlist\n");
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
        if let Some(model) = &comp.model {
            let directive = match &model.definition {
                ModelDefinition::Device { directive }
                | ModelDefinition::Subcircuit { directive, .. } => directive,
            };
            used_models.insert(directive.clone());
        }
    }

    if !used_models.is_empty() {
        spice.push_str("\n* Standard Models\n");
        for model in used_models {
            spice.push_str(&model);
            spice.push('\n');
        }
    }

    let mut has_sim = false;
    let mut control_block = String::from("\n.control\n");
    if !circuit.analyses.is_empty() {
        control_block.push_str("set wr_singlescale\nset wr_vecnames\nset numdgt=17\n");
        let device_currents = circuit
            .components
            .iter()
            .filter_map(|component| match component.kind {
                ComponentKind::Diode => Some(format!("@D_{}[id]", component.id)),
                ComponentKind::BJT(_) => Some(format!("@Q_{}[ic]", component.id)),
                ComponentKind::MOSFET(_) => Some(format!("@M_{}[id]", component.id)),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !device_currents.is_empty() {
            control_block.push_str(&format!("save all {}\n", device_currents.join(" ")));
        }
    }
    for (index, analysis) in circuit.analyses.iter().enumerate() {
        has_sim = true;
        control_block.push_str(&format_analysis(analysis, circuit));
        control_block.push('\n');
        control_block.push_str(&format!(
            "wrdata {} all\n",
            analysis_data_filename(index, analysis)
        ));
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
            let arguments = assert.signal.split(',').collect::<Vec<_>>();
            if arguments.len() != 1
                || !(arguments[0].starts_with("V(") || arguments[0].starts_with("I("))
            {
                continue;
            }
            let signal = arguments[0];
            let target = &signal[2..signal.len() - 1];
            let component = circuit
                .components
                .iter()
                .find(|component| component.id.eq_ignore_ascii_case(target));
            if signal.starts_with("V(") && component.is_some() {
                continue;
            }
            if signal.starts_with("I(")
                && !component.is_some_and(|component| {
                    matches!(
                        component.kind,
                        ComponentKind::VoltageSource | ComponentKind::Inductor
                    )
                })
            {
                continue;
            }
            let safe_signal = signal.replace("(", "_").replace(")", "").to_lowercase();
            let safe_name = format!("{}_{}", assert.metric.to_lowercase(), safe_signal);
            let metric = match assert.metric.to_uppercase().as_str() {
                "MAX" => "MAX",
                "MIN" => "MIN",
                "RMS" => "RMS",
                "AVERAGE" | "AVG" => "AVG",
                _ => "",
            };

            // Note: `op` does not support MAX/MIN/RMS measurements over time.
            // If main_analysis is op, we should use FIND instead or just use DC eval.
            // For now, if metric is MAX/MIN/RMS we assume we need to use it.
            // If it's op, ngspice .meas op expects `FIND v(node) AT=0` or similar,
            // but for simplicity we'll just output the metric.
            let mut sp_signal = signal.to_string();
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
                continue;
            }
            if assert.metric.eq_ignore_ascii_case("peak") {
                control_block.push_str(&format!(
                    "meas {main_analysis} peak_pos_{safe_signal} MAX {sp_signal}\n"
                ));
                control_block.push_str(&format!(
                    "meas {main_analysis} peak_neg_{safe_signal} MIN {sp_signal}\n"
                ));
            } else if !metric.is_empty() {
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
