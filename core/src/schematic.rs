use crate::component::{CatalogSymbol, PinFlow, PinSide, component_definition, through_pin};
use crate::graph::{NetId, NetlistGraph, format_spice_number};
use crate::ir::{
    BJTPolarity, CircuitIR, ComponentKind, ComponentParams, FETPolarity, IRComponent, SIUnit,
    SourceValue, Waveform,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

pub const SCHEMATIC_SCHEMA_VERSION: &str = "kessetsu.schematic.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    fn overlaps(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Right,
    Down,
    Left,
    Up,
}

impl Orientation {
    fn rotate_point(self, point: Point) -> Point {
        match self {
            Self::Right => point,
            Self::Down => Point::new(-point.y, point.x),
            Self::Left => Point::new(-point.x, -point.y),
            Self::Up => Point::new(point.y, -point.x),
        }
    }

    fn rotate_side(self, side: PinSide) -> PinSide {
        let rotate_once = |value| match value {
            PinSide::Left => PinSide::Top,
            PinSide::Top => PinSide::Right,
            PinSide::Right => PinSide::Bottom,
            PinSide::Bottom => PinSide::Left,
        };
        match self {
            Self::Right => side,
            Self::Down => rotate_once(side),
            Self::Left => rotate_once(rotate_once(side)),
            Self::Up => rotate_once(rotate_once(rotate_once(side))),
        }
    }

    pub const fn degrees(self) -> i32 {
        match self {
            Self::Right => 0,
            Self::Down => 90,
            Self::Left => 180,
            Self::Up => 270,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PinRef {
    pub component: String,
    pub pin: String,
}

impl PinRef {
    fn new(component: impl Into<String>, pin: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            pin: pin.into(),
        }
    }

    fn id(&self) -> String {
        format!("{}.{}", self.component, self.pin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinAnchor {
    pub name: String,
    pub net: Option<NetId>,
    pub point: Point,
    pub escape: Point,
    pub side: PinSide,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicComponent {
    pub id: String,
    pub symbol: CatalogSymbol,
    pub variant: Option<String>,
    pub reference: String,
    pub value: Option<String>,
    pub model: Option<String>,
    pub orientation: Orientation,
    pub mirrored_x: bool,
    pub origin: Point,
    pub bounds: Rect,
    pub pins: Vec<PinAnchor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetKind {
    Ground,
    Supply,
    Signal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicNet {
    pub id: NetId,
    pub name: String,
    pub kind: NetKind,
    pub pins: Vec<PinRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireEndpoint {
    Pin { component: String, pin: String },
    Junction { id: String },
}

impl WireEndpoint {
    fn pin(reference: &PinRef) -> Self {
        Self::Pin {
            component: reference.component.clone(),
            pin: reference.pin.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicWire {
    pub id: String,
    pub net: NetId,
    pub start: WireEndpoint,
    pub end: WireEndpoint,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Junction {
    pub id: String,
    pub net: NetId,
    pub point: Point,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crossing {
    pub id: String,
    pub point: Point,
    pub nets: [NetId; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetLabel {
    pub id: String,
    pub net: NetId,
    pub text: String,
    pub kind: NetKind,
    pub point: Point,
    pub attached_to: PinRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectivityReport {
    pub verified: bool,
    pub expected_connected_pins: usize,
    pub represented_connected_pins: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityReport {
    pub passed: bool,
    pub symbol_collisions: usize,
    pub wire_symbol_collisions: usize,
    pub label_symbol_collisions: usize,
    pub crossings: usize,
    pub crossing_limit: usize,
    pub bends: usize,
    pub wire_length: usize,
    pub local_signal_pins: usize,
    pub wired_local_signal_pins: usize,
    pub explicit_wire_coverage_per_mille: usize,
    pub global_signal_label_pins: usize,
    pub local_signal_label_pins: usize,
    pub flow_inversions: usize,
    pub content_width: usize,
    pub content_height: usize,
    pub aspect_ratio_milli: usize,
    pub occupied_area_per_mille: usize,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schematic {
    pub schema_version: String,
    pub components: Vec<SchematicComponent>,
    pub nets: Vec<SchematicNet>,
    pub wires: Vec<SchematicWire>,
    pub junctions: Vec<Junction>,
    pub crossings: Vec<Crossing>,
    pub labels: Vec<NetLabel>,
    pub bounds: Rect,
    pub connectivity: ConnectivityReport,
    pub quality: QualityReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchematicError {
    pub message: String,
}

impl std::fmt::Display for SchematicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for SchematicError {}

fn unit_suffix(unit: SIUnit) -> &'static str {
    match unit {
        SIUnit::Ohm => "Ω",
        SIUnit::Farad => "F",
        SIUnit::Henry => "H",
        SIUnit::Volt => "V",
        SIUnit::Ampere => "A",
        SIUnit::Hertz => "Hz",
        SIUnit::Second => "s",
        SIUnit::Watt => "W",
        SIUnit::Ratio => "",
        SIUnit::Percent => "%",
        SIUnit::Degree => "°",
    }
}

fn quantity_label(value: f64, unit: SIUnit) -> String {
    if matches!(unit, SIUnit::Ratio | SIUnit::Percent | SIUnit::Degree) {
        return format!("{}{}", format_spice_number(value), unit_suffix(unit));
    }
    let magnitude = value.abs();
    let (scale, prefix) = if magnitude == 0.0 {
        (1.0, "")
    } else if magnitude >= 1e9 {
        (1e9, "G")
    } else if magnitude >= 1e6 {
        (1e6, "M")
    } else if magnitude >= 1e3 {
        (1e3, "k")
    } else if magnitude >= 1.0 {
        (1.0, "")
    } else if magnitude >= 1e-3 {
        (1e-3, "m")
    } else if magnitude >= 1e-6 {
        (1e-6, "µ")
    } else if magnitude >= 1e-9 {
        (1e-9, "n")
    } else {
        (1e-12, "p")
    };
    let scaled = value / scale;
    let precision = if scaled.abs() >= 100.0 {
        0
    } else if scaled.abs() >= 10.0 {
        1
    } else {
        2
    };
    let mut number = format!("{scaled:.precision$}");
    while number.contains('.') && number.ends_with('0') {
        number.pop();
    }
    if number.ends_with('.') {
        number.pop();
    }
    format!("{number} {prefix}{}", unit_suffix(unit))
}

fn waveform_label(waveform: &Waveform) -> String {
    match waveform {
        Waveform::Ac { amplitude } => {
            format!("AC {}", quantity_label(amplitude.value, amplitude.unit))
        }
        Waveform::Sine {
            offset,
            amplitude,
            frequency,
        }
        | Waveform::SineAc {
            offset,
            amplitude,
            frequency,
            ..
        } => {
            let amplitude = quantity_label(amplitude.value, amplitude.unit);
            let frequency = quantity_label(frequency.value, frequency.unit);
            if offset.value.abs() < f64::EPSILON {
                format!("{amplitude} · {frequency} sine")
            } else {
                format!(
                    "{amplitude} · {frequency} sine @ {}",
                    quantity_label(offset.value, offset.unit)
                )
            }
        }
        Waveform::Pulse { v1, v2, period, .. } => format!(
            "PULSE {} {} / {}",
            quantity_label(v1.value, v1.unit),
            quantity_label(v2.value, v2.unit),
            quantity_label(period.value, period.unit)
        ),
        Waveform::PWL { points } => format!("PWL {} points", points.len()),
    }
}

fn component_labels(component: &IRComponent) -> (Option<String>, Option<String>, Option<String>) {
    let value = match &component.parameters {
        ComponentParams::TwoPinPassive { value } => Some(quantity_label(value.value, value.unit)),
        ComponentParams::VoltageSource { value } | ComponentParams::CurrentSource { value } => {
            Some(match value {
                SourceValue::Dc(quantity) => quantity_label(quantity.value, quantity.unit),
                SourceValue::Waveform(waveform) => waveform_label(waveform),
            })
        }
        ComponentParams::ModulePort { module_name } => Some(module_name.clone()),
        _ => None,
    };
    let model = component.model.as_ref().map(|model| model.name.clone());
    let variant = match component.kind {
        ComponentKind::BJT(BJTPolarity::NPN) => Some("npn".to_string()),
        ComponentKind::BJT(BJTPolarity::PNP) => Some("pnp".to_string()),
        ComponentKind::MOSFET(FETPolarity::NMOS) => Some("nmos".to_string()),
        ComponentKind::MOSFET(FETPolarity::PMOS) => Some("pmos".to_string()),
        _ => None,
    };
    (value, model, variant)
}

fn side_vector(side: PinSide) -> (i32, i32) {
    match side {
        PinSide::Left => (-1, 0),
        PinSide::Right => (1, 0),
        PinSide::Top => (0, -1),
        PinSide::Bottom => (0, 1),
    }
}

fn orientation_for(component: &IRComponent, graph: &NetlistGraph) -> Orientation {
    if matches!(
        component.kind,
        ComponentKind::VoltageSource | ComponentKind::CurrentSource
    ) {
        return Orientation::Down;
    }
    if matches!(
        component.kind,
        ComponentKind::Resistor
            | ComponentKind::Capacitor
            | ComponentKind::Inductor
            | ComponentKind::Diode
    ) {
        let p1 = graph.get_net(&component.id, "p1");
        let p2 = graph.get_net(&component.id, "p2");
        if p2 == Some(NetId::GROUND) {
            return Orientation::Down;
        }
        if p1 == Some(NetId::GROUND) {
            return Orientation::Up;
        }
        if p1.is_some_and(|net| is_supply_name(&graph.get_net_name(net))) {
            return Orientation::Down;
        }
        if p2.is_some_and(|net| is_supply_name(&graph.get_net_name(net))) {
            return Orientation::Up;
        }
    }
    Orientation::Right
}

fn transform_local_point(
    point: Point,
    width: i32,
    orientation: Orientation,
    mirrored_x: bool,
) -> Point {
    let point = if mirrored_x {
        Point::new(width - point.x, point.y)
    } else {
        point
    };
    orientation.rotate_point(point)
}

fn transform_local_side(side: PinSide, orientation: Orientation, mirrored_x: bool) -> PinSide {
    let side = if mirrored_x {
        match side {
            PinSide::Left => PinSide::Right,
            PinSide::Right => PinSide::Left,
            other => other,
        }
    } else {
        side
    };
    orientation.rotate_side(side)
}

fn local_bounds(component: &IRComponent, orientation: Orientation, mirrored_x: bool) -> Rect {
    let definition = component_definition(&component.kind);
    let mut points = vec![
        transform_local_point(Point::new(0, 0), definition.width, orientation, mirrored_x),
        transform_local_point(
            Point::new(definition.width, 0),
            definition.width,
            orientation,
            mirrored_x,
        ),
        transform_local_point(
            Point::new(0, definition.height),
            definition.width,
            orientation,
            mirrored_x,
        ),
        transform_local_point(
            Point::new(definition.width, definition.height),
            definition.width,
            orientation,
            mirrored_x,
        ),
    ];
    points.extend(definition.pins.iter().map(|pin| {
        transform_local_point(
            Point::new(pin.x, pin.y),
            definition.width,
            orientation,
            mirrored_x,
        )
    }));
    Rect {
        min: Point::new(
            points.iter().map(|point| point.x).min().unwrap_or(0),
            points.iter().map(|point| point.y).min().unwrap_or(0),
        ),
        max: Point::new(
            points.iter().map(|point| point.x).max().unwrap_or(0),
            points.iter().map(|point| point.y).max().unwrap_or(0),
        ),
    }
}

fn is_supply_name(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "VCC" | "VDD" | "VEE" | "VSS" | "+V" | "-V"
    )
}

fn supply_nets(circuit: &CircuitIR, graph: &NetlistGraph) -> BTreeSet<NetId> {
    let mut result = BTreeSet::new();
    for component in &circuit.components {
        for pin in component_definition(&component.kind).pins {
            if let Some(net) = graph.get_net(&component.id, pin.name)
                && net != NetId::GROUND
                && (pin.flow == PinFlow::Power || is_supply_name(&graph.get_net_name(net)))
            {
                result.insert(net);
            }
        }
    }
    result
}

#[derive(Debug)]
struct FlowAnalysis {
    component_ranks: BTreeMap<String, usize>,
}

fn flow_analysis(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> FlowAnalysis {
    let mut net_pins: BTreeMap<NetId, Vec<PinRef>> = BTreeMap::new();
    for component in &circuit.components {
        for pin in component_definition(&component.kind).pins {
            if let Some(net) = graph.get_net(&component.id, pin.name) {
                net_pins
                    .entry(net)
                    .or_default()
                    .push(PinRef::new(&component.id, pin.name));
            }
        }
    }
    for pins in net_pins.values_mut() {
        pins.sort();
    }

    let waveform_sources: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| {
            matches!(
                component.parameters,
                ComponentParams::VoltageSource {
                    value: SourceValue::Waveform(_)
                } | ComponentParams::CurrentSource {
                    value: SourceValue::Waveform(_)
                }
            )
        })
        .map(|component| component.id.clone())
        .collect();
    let all_sources: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| {
            matches!(
                component.kind,
                ComponentKind::VoltageSource | ComponentKind::CurrentSource
            )
        })
        .map(|component| component.id.clone())
        .collect();
    let non_supply_sources: Vec<_> = all_sources
        .iter()
        .filter(|component_id| {
            graph
                .get_net(component_id, "plus")
                .is_some_and(|net| net != NetId::GROUND && !supplies.contains(&net))
        })
        .cloned()
        .collect();
    let traversal_sources = if !waveform_sources.is_empty() {
        waveform_sources
    } else if !non_supply_sources.is_empty() {
        non_supply_sources
    } else {
        all_sources.clone()
    };

    let by_id: BTreeMap<_, _> = circuit
        .components
        .iter()
        .map(|component| (component.id.clone(), component))
        .collect();
    let mut component_ranks = BTreeMap::new();
    let mut net_ranks = BTreeMap::new();
    let mut queue = VecDeque::new();
    for source in traversal_sources {
        component_ranks.insert(source.clone(), 0);
        if let Some(net) = graph.get_net(&source, "plus")
            && net != NetId::GROUND
            && !supplies.contains(&net)
        {
            net_ranks.entry(net).or_insert(1);
            queue.push_back((net, 1));
        }
    }

    while let Some((net, net_rank)) = queue.pop_front() {
        let Some(pins) = net_pins.get(&net) else {
            continue;
        };
        for pin_ref in pins {
            let component = by_id[&pin_ref.component];
            let definition = component_definition(&component.kind);
            let Some(pin) = definition
                .pins
                .iter()
                .find(|candidate| candidate.name == pin_ref.pin)
            else {
                continue;
            };
            if matches!(
                pin.flow,
                PinFlow::Power | PinFlow::Reference | PinFlow::Output
            ) {
                continue;
            }
            if !component_ranks.contains_key(&component.id) {
                component_ranks.insert(component.id.clone(), net_rank);
            }
            let Some(exit_pin) = through_pin(&component.kind, pin.name) else {
                continue;
            };
            let Some(exit_net) = graph.get_net(&component.id, exit_pin) else {
                continue;
            };
            if exit_net == NetId::GROUND || supplies.contains(&exit_net) {
                continue;
            }
            let next_rank = net_rank + 1;
            if net_ranks
                .get(&exit_net)
                .is_none_or(|existing| next_rank < *existing)
            {
                net_ranks.insert(exit_net, next_rank);
                queue.push_back((exit_net, next_rank));
            }
        }
    }

    for source in all_sources {
        component_ranks.entry(source).or_insert(0);
    }
    let fallback_rank = component_ranks.values().copied().max().unwrap_or(0) + 1;
    for component in &circuit.components {
        if !component_ranks.contains_key(&component.id) {
            let rank = component_definition(&component.kind)
                .pins
                .iter()
                .filter_map(|pin| graph.get_net(&component.id, pin.name))
                .filter_map(|net| net_ranks.get(&net).copied())
                .min()
                .unwrap_or(fallback_rank);
            component_ranks.insert(component.id.clone(), rank);
        }
    }
    FlowAnalysis { component_ranks }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PlacementLane {
    Upper,
    Main,
    Lower,
    PowerBlock,
}

fn is_two_pin_passive(kind: &ComponentKind) -> bool {
    matches!(
        kind,
        ComponentKind::Resistor
            | ComponentKind::Capacitor
            | ComponentKind::Inductor
            | ComponentKind::Diode
    )
}

fn component_nets(component: &IRComponent, graph: &NetlistGraph) -> BTreeSet<NetId> {
    component_definition(&component.kind)
        .pins
        .iter()
        .filter_map(|pin| graph.get_net(&component.id, pin.name))
        .collect()
}

fn is_feedback_passive(component: &IRComponent, circuit: &CircuitIR, graph: &NetlistGraph) -> bool {
    if !is_two_pin_passive(&component.kind) {
        return false;
    }
    let passive_nets = component_nets(component, graph);
    if passive_nets.contains(&NetId::GROUND) {
        return false;
    }
    circuit.components.iter().any(|active| {
        if active.id == component.id {
            return false;
        }
        let definition = component_definition(&active.kind);
        let input_nets: BTreeSet<_> = definition
            .pins
            .iter()
            .filter(|pin| pin.flow == PinFlow::Input)
            .filter_map(|pin| graph.get_net(&active.id, pin.name))
            .collect();
        let output_nets: BTreeSet<_> = definition
            .pins
            .iter()
            .filter(|pin| pin.flow == PinFlow::Output)
            .filter_map(|pin| graph.get_net(&active.id, pin.name))
            .collect();
        passive_nets.iter().any(|net| input_nets.contains(net))
            && passive_nets.iter().any(|net| output_nets.contains(net))
    })
}

fn feedback_owner<'a>(
    component: &IRComponent,
    circuit: &'a CircuitIR,
    graph: &NetlistGraph,
) -> Option<&'a IRComponent> {
    if !is_two_pin_passive(&component.kind) {
        return None;
    }
    let passive_nets = component_nets(component, graph);
    if passive_nets.contains(&NetId::GROUND) {
        return None;
    }
    circuit.components.iter().find(|active| {
        let definition = component_definition(&active.kind);
        let input_nets: BTreeSet<_> = definition
            .pins
            .iter()
            .filter(|pin| pin.flow == PinFlow::Input)
            .filter_map(|pin| graph.get_net(&active.id, pin.name))
            .collect();
        let output_nets: BTreeSet<_> = definition
            .pins
            .iter()
            .filter(|pin| pin.flow == PinFlow::Output)
            .filter_map(|pin| graph.get_net(&active.id, pin.name))
            .collect();
        passive_nets.iter().any(|net| input_nets.contains(net))
            && passive_nets.iter().any(|net| output_nets.contains(net))
    })
}

fn feedback_orientation(
    component: &IRComponent,
    circuit: &CircuitIR,
    graph: &NetlistGraph,
) -> Option<Orientation> {
    let owner = feedback_owner(component, circuit, graph)?;
    let definition = component_definition(&owner.kind);
    let input_nets: BTreeSet<_> = definition
        .pins
        .iter()
        .filter(|pin| pin.flow == PinFlow::Input)
        .filter_map(|pin| graph.get_net(&owner.id, pin.name))
        .collect();
    let output_nets: BTreeSet<_> = definition
        .pins
        .iter()
        .filter(|pin| pin.flow == PinFlow::Output)
        .filter_map(|pin| graph.get_net(&owner.id, pin.name))
        .collect();
    let p1 = graph.get_net(&component.id, "p1")?;
    let p2 = graph.get_net(&component.id, "p2")?;
    if input_nets.contains(&p1) && output_nets.contains(&p2) {
        Some(Orientation::Right)
    } else if output_nets.contains(&p1) && input_nets.contains(&p2) {
        Some(Orientation::Left)
    } else {
        None
    }
}

fn connected_pin_count(circuit: &CircuitIR, graph: &NetlistGraph, net: NetId) -> usize {
    circuit
        .components
        .iter()
        .map(|component| {
            component_definition(&component.kind)
                .pins
                .iter()
                .filter(|pin| graph.get_net(&component.id, pin.name) == Some(net))
                .count()
        })
        .sum()
}

fn effective_placement_rank(
    component: &IRComponent,
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    ranks: &BTreeMap<String, usize>,
) -> usize {
    if let Some(owner) = feedback_owner(component, circuit, graph) {
        return ranks[&owner.id];
    }
    let nets = component_nets(component, graph);
    if is_two_pin_passive(&component.kind)
        && nets.contains(&NetId::GROUND)
        && let Some(signal_net) = nets.into_iter().find(|net| *net != NetId::GROUND)
        && connected_pin_count(circuit, graph, signal_net) <= 3
        && let Some(owner) = circuit.components.iter().find(|candidate| {
            component_definition(&candidate.kind)
                .pins
                .iter()
                .filter(|pin| pin.flow == PinFlow::Input)
                .any(|pin| graph.get_net(&candidate.id, pin.name) == Some(signal_net))
        })
    {
        return ranks[&owner.id];
    }
    ranks[&component.id]
}

fn placement_lane(
    component: &IRComponent,
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> PlacementLane {
    if matches!(
        component.parameters,
        ComponentParams::VoltageSource {
            value: SourceValue::Dc(_)
        } | ComponentParams::CurrentSource {
            value: SourceValue::Dc(_)
        }
    ) && graph
        .get_net(&component.id, "plus")
        .is_some_and(|net| supplies.contains(&net))
    {
        return PlacementLane::PowerBlock;
    }
    if is_feedback_passive(component, circuit, graph) {
        return PlacementLane::Lower;
    }
    let nets = component_nets(component, graph);
    if is_two_pin_passive(&component.kind) && nets.iter().any(|net| supplies.contains(net)) {
        return PlacementLane::Upper;
    }
    if nets.contains(&NetId::GROUND)
        && (is_two_pin_passive(&component.kind)
            || matches!(component.kind, ComponentKind::CurrentSource))
    {
        return PlacementLane::Lower;
    }
    PlacementLane::Main
}

fn placed_component(
    component: &IRComponent,
    graph: &NetlistGraph,
    orientation: Orientation,
    desired: Point,
) -> SchematicComponent {
    placed_component_mirrored(component, graph, orientation, false, desired)
}

fn placed_component_mirrored(
    component: &IRComponent,
    graph: &NetlistGraph,
    orientation: Orientation,
    mirrored_x: bool,
    desired: Point,
) -> SchematicComponent {
    let local = local_bounds(component, orientation, mirrored_x);
    let origin = Point::new(desired.x - local.min.x, desired.y - local.min.y);
    let bounds = Rect {
        min: desired,
        max: Point::new(
            desired.x + local.max.x - local.min.x,
            desired.y + local.max.y - local.min.y,
        ),
    };
    let definition = component_definition(&component.kind);
    let pins = definition
        .pins
        .iter()
        .map(|pin| {
            let point = transform_local_point(
                Point::new(pin.x, pin.y),
                definition.width,
                orientation,
                mirrored_x,
            )
            .offset(origin.x, origin.y);
            let side = transform_local_side(pin.side, orientation, mirrored_x);
            let (dx, dy) = side_vector(side);
            PinAnchor {
                name: pin.name.to_string(),
                net: graph.get_net(&component.id, pin.name),
                point,
                escape: point.offset(dx, dy),
                side,
            }
        })
        .collect();
    let (value, model, variant) = component_labels(component);
    SchematicComponent {
        id: component.id.clone(),
        symbol: definition.symbol,
        variant,
        reference: component.id.clone(),
        value,
        model,
        orientation,
        mirrored_x,
        origin,
        bounds,
        pins,
    }
}

fn oriented_between(
    component: &IRComponent,
    graph: &NetlistGraph,
    first: NetId,
    second: NetId,
    forward: Orientation,
    reverse: Orientation,
) -> Option<Orientation> {
    let p1 = graph.get_net(&component.id, "p1")?;
    let p2 = graph.get_net(&component.id, "p2")?;
    if p1 == first && p2 == second {
        Some(forward)
    } else if p1 == second && p2 == first {
        Some(reverse)
    } else {
        None
    }
}

fn bridge_placement(circuit: &CircuitIR, graph: &NetlistGraph) -> Option<Vec<SchematicComponent>> {
    let sources: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| {
            matches!(
                component.kind,
                ComponentKind::VoltageSource | ComponentKind::CurrentSource
            )
        })
        .collect();
    let passives: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| is_two_pin_passive(&component.kind))
        .collect();
    if sources.len() != 1 || passives.len() != 5 || circuit.components.len() != 6 {
        return None;
    }
    let source = sources[0];
    let top = graph.get_net(&source.id, "plus")?;
    let bottom = graph.get_net(&source.id, "minus")?;
    let mut upper = Vec::new();
    let mut lower = Vec::new();
    for component in &passives {
        let nets = component_nets(component, graph);
        if nets.contains(&top) && !nets.contains(&bottom) {
            let mid = nets.iter().copied().find(|net| *net != top)?;
            upper.push((mid, *component));
        } else if nets.contains(&bottom) && !nets.contains(&top) {
            let mid = nets.iter().copied().find(|net| *net != bottom)?;
            lower.push((mid, *component));
        }
    }
    if upper.len() != 2 || lower.len() != 2 {
        return None;
    }
    upper.sort_by_key(|(mid, component)| (*mid, component.id.as_str()));
    lower.sort_by_key(|(mid, component)| (*mid, component.id.as_str()));
    let mut branches = Vec::new();
    for (mid, upper_component) in upper {
        let lower_component = lower
            .iter()
            .find(|(candidate, _)| *candidate == mid)
            .map(|(_, component)| *component)?;
        branches.push((mid, upper_component, lower_component));
    }
    if branches.len() != 2 || branches[0].0 == branches[1].0 {
        return None;
    }
    let bridge = passives.iter().copied().find(|component| {
        let nets = component_nets(component, graph);
        nets.len() == 2 && nets.contains(&branches[0].0) && nets.contains(&branches[1].0)
    })?;

    let mut placed = Vec::new();
    placed.push(placed_component(
        source,
        graph,
        Orientation::Down,
        Point::new(3, 5),
    ));
    for (index, (mid, upper_component, lower_component)) in branches.iter().enumerate() {
        let x = 9 + i32::try_from(index).ok()? * 8;
        let upper_orientation = oriented_between(
            upper_component,
            graph,
            top,
            *mid,
            Orientation::Down,
            Orientation::Up,
        )?;
        let lower_orientation = oriented_between(
            lower_component,
            graph,
            *mid,
            bottom,
            Orientation::Down,
            Orientation::Up,
        )?;
        placed.push(placed_component(
            upper_component,
            graph,
            upper_orientation,
            Point::new(x, 5),
        ));
        placed.push(placed_component(
            lower_component,
            graph,
            lower_orientation,
            Point::new(x, 12),
        ));
    }
    let bridge_orientation = oriented_between(
        bridge,
        graph,
        branches[0].0,
        branches[1].0,
        Orientation::Right,
        Orientation::Left,
    )?;
    placed.push(placed_component(
        bridge,
        graph,
        bridge_orientation,
        Point::new(12, 9),
    ));
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Some(placed)
}

fn differential_pair_placement(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> Option<Vec<SchematicComponent>> {
    let mut transistors: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| matches!(component.kind, ComponentKind::BJT(_)))
        .collect();
    if transistors.len() != 2 {
        return None;
    }
    transistors.sort_by(|left, right| left.id.cmp(&right.id));
    let tail = graph.get_net(&transistors[0].id, "e")?;
    if graph.get_net(&transistors[1].id, "e") != Some(tail) {
        return None;
    }
    let base_nets = [
        graph.get_net(&transistors[0].id, "b")?,
        graph.get_net(&transistors[1].id, "b")?,
    ];
    let collector_nets = [
        graph.get_net(&transistors[0].id, "c")?,
        graph.get_net(&transistors[1].id, "c")?,
    ];
    if base_nets[0] == base_nets[1] || collector_nets[0] == collector_nets[1] {
        return None;
    }
    let collector_loads: Vec<_> = collector_nets
        .iter()
        .map(|collector_net| {
            circuit.components.iter().find(|component| {
                is_two_pin_passive(&component.kind)
                    && component_nets(component, graph).contains(collector_net)
                    && component_nets(component, graph)
                        .iter()
                        .any(|net| supplies.contains(net))
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let input_sources: Vec<_> = base_nets
        .iter()
        .map(|base_net| {
            circuit.components.iter().find(|component| {
                matches!(
                    component.kind,
                    ComponentKind::VoltageSource | ComponentKind::CurrentSource
                ) && graph.get_net(&component.id, "plus") == Some(*base_net)
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let tail_source = circuit.components.iter().find(|component| {
        matches!(component.kind, ComponentKind::CurrentSource)
            && graph.get_net(&component.id, "plus") == Some(tail)
    })?;
    let supply_source = circuit.components.iter().find(|component| {
        matches!(component.kind, ComponentKind::VoltageSource)
            && graph
                .get_net(&component.id, "plus")
                .is_some_and(|net| supplies.contains(&net))
    })?;
    let recognized: BTreeSet<_> = transistors
        .iter()
        .chain(collector_loads.iter())
        .chain(input_sources.iter())
        .chain([&tail_source, &supply_source])
        .map(|component| component.id.as_str())
        .collect();
    if recognized.len() != circuit.components.len() {
        return None;
    }

    let mut placed = vec![
        placed_component(transistors[0], graph, Orientation::Right, Point::new(13, 8)),
        placed_component_mirrored(
            transistors[1],
            graph,
            Orientation::Right,
            true,
            Point::new(23, 8),
        ),
        placed_component(
            collector_loads[0],
            graph,
            orientation_for(collector_loads[0], graph),
            Point::new(14, 3),
        ),
        placed_component(
            collector_loads[1],
            graph,
            orientation_for(collector_loads[1], graph),
            Point::new(23, 3),
        ),
        placed_component(input_sources[0], graph, Orientation::Down, Point::new(5, 9)),
        placed_component(
            input_sources[1],
            graph,
            Orientation::Down,
            Point::new(29, 9),
        ),
        placed_component(tail_source, graph, Orientation::Down, Point::new(18, 15)),
        placed_component(supply_source, graph, Orientation::Down, Point::new(3, 17)),
    ];
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Some(placed)
}

fn single_transistor_stage_placement(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> Option<Vec<SchematicComponent>> {
    let transistor = circuit.components.iter().find(|component| {
        matches!(
            component.kind,
            ComponentKind::MOSFET(_) | ComponentKind::BJT(_)
        )
    })?;
    if circuit
        .components
        .iter()
        .filter(|component| {
            matches!(
                component.kind,
                ComponentKind::MOSFET(_) | ComponentKind::BJT(_)
            )
        })
        .count()
        != 1
    {
        return None;
    }
    let (input_pin, upper_pin, lower_pin) = match transistor.kind {
        ComponentKind::MOSFET(_) => ("g", "d", "s"),
        ComponentKind::BJT(_) => ("b", "c", "e"),
        _ => return None,
    };
    let gate = graph.get_net(&transistor.id, input_pin)?;
    let drain = graph.get_net(&transistor.id, upper_pin)?;
    let source = graph.get_net(&transistor.id, lower_pin)?;

    let drain_resistor = circuit.components.iter().find(|component| {
        is_two_pin_passive(&component.kind)
            && component_nets(component, graph).contains(&drain)
            && component_nets(component, graph)
                .iter()
                .any(|net| supplies.contains(net))
    })?;
    let source_resistor = circuit.components.iter().find(|component| {
        is_two_pin_passive(&component.kind)
            && component_nets(component, graph).contains(&source)
            && component_nets(component, graph).contains(&NetId::GROUND)
    })?;
    let load = circuit.components.iter().find(|component| {
        component.id != drain_resistor.id
            && is_two_pin_passive(&component.kind)
            && component_nets(component, graph).contains(&drain)
            && component_nets(component, graph).contains(&NetId::GROUND)
    })?;
    let input_source = circuit.components.iter().find(|component| {
        matches!(
            component.kind,
            ComponentKind::VoltageSource | ComponentKind::CurrentSource
        ) && graph.get_net(&component.id, "plus") == Some(gate)
    })?;
    let supply_source = circuit.components.iter().find(|component| {
        matches!(component.kind, ComponentKind::VoltageSource)
            && graph
                .get_net(&component.id, "plus")
                .is_some_and(|net| supplies.contains(&net))
    })?;
    let recognized: BTreeSet<_> = [
        transistor,
        drain_resistor,
        source_resistor,
        load,
        input_source,
        supply_source,
    ]
    .into_iter()
    .map(|component| component.id.as_str())
    .collect();
    if recognized.len() != circuit.components.len() {
        return None;
    }
    let supply = component_nets(drain_resistor, graph)
        .into_iter()
        .find(|net| supplies.contains(net))?;
    let mut placed = vec![
        placed_component(transistor, graph, Orientation::Right, Point::new(13, 7)),
        placed_component(
            drain_resistor,
            graph,
            oriented_between(
                drain_resistor,
                graph,
                supply,
                drain,
                Orientation::Down,
                Orientation::Up,
            )?,
            Point::new(14, 2),
        ),
        placed_component(
            source_resistor,
            graph,
            oriented_between(
                source_resistor,
                graph,
                source,
                NetId::GROUND,
                Orientation::Down,
                Orientation::Up,
            )?,
            Point::new(14, 12),
        ),
        placed_component(
            load,
            graph,
            oriented_between(
                load,
                graph,
                drain,
                NetId::GROUND,
                Orientation::Down,
                Orientation::Up,
            )?,
            Point::new(21, 7),
        ),
        placed_component(input_source, graph, Orientation::Down, Point::new(4, 8)),
        placed_component(supply_source, graph, Orientation::Down, Point::new(2, 15)),
    ];
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Some(placed)
}

fn diode_clamp_placement(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> Option<Vec<SchematicComponent>> {
    let diodes: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| matches!(component.kind, ComponentKind::Diode))
        .collect();
    if diodes.len() != 2 {
        return None;
    }
    let output = graph
        .net_names
        .keys()
        .copied()
        .filter(|net| *net != NetId::GROUND && !supplies.contains(net))
        .find(|net| connected_pin_count(circuit, graph, *net) >= 4)?;
    let upper = diodes.iter().copied().find(|component| {
        let nets = component_nets(component, graph);
        nets.contains(&output) && nets.iter().any(|net| supplies.contains(net))
    })?;
    let lower = diodes.iter().copied().find(|component| {
        let nets = component_nets(component, graph);
        nets.contains(&output) && nets.contains(&NetId::GROUND)
    })?;
    let load = circuit.components.iter().find(|component| {
        is_two_pin_passive(&component.kind)
            && !matches!(component.kind, ComponentKind::Diode)
            && component_nets(component, graph).contains(&output)
            && component_nets(component, graph).contains(&NetId::GROUND)
    })?;
    let input_resistor = circuit.components.iter().find(|component| {
        is_two_pin_passive(&component.kind)
            && !matches!(component.kind, ComponentKind::Diode)
            && component.id != load.id
            && component_nets(component, graph).contains(&output)
    })?;
    let input_net = component_nets(input_resistor, graph)
        .into_iter()
        .find(|net| *net != output)?;
    let input_source = circuit.components.iter().find(|component| {
        matches!(
            component.kind,
            ComponentKind::VoltageSource | ComponentKind::CurrentSource
        ) && graph.get_net(&component.id, "plus") == Some(input_net)
    })?;
    let supply_source = circuit.components.iter().find(|component| {
        matches!(component.kind, ComponentKind::VoltageSource)
            && graph
                .get_net(&component.id, "plus")
                .is_some_and(|net| supplies.contains(&net))
    })?;
    let supply = component_nets(upper, graph)
        .into_iter()
        .find(|net| supplies.contains(net))?;
    let recognized: BTreeSet<_> = [
        upper,
        lower,
        load,
        input_resistor,
        input_source,
        supply_source,
    ]
    .into_iter()
    .map(|component| component.id.as_str())
    .collect();
    if recognized.len() != circuit.components.len() {
        return None;
    }
    let mut placed = vec![
        placed_component(input_source, graph, Orientation::Down, Point::new(3, 8)),
        placed_component(
            input_resistor,
            graph,
            oriented_between(
                input_resistor,
                graph,
                input_net,
                output,
                Orientation::Right,
                Orientation::Left,
            )?,
            Point::new(9, 8),
        ),
        placed_component(
            upper,
            graph,
            oriented_between(
                upper,
                graph,
                supply,
                output,
                Orientation::Down,
                Orientation::Up,
            )?,
            Point::new(14, 3),
        ),
        placed_component(
            lower,
            graph,
            oriented_between(
                lower,
                graph,
                output,
                NetId::GROUND,
                Orientation::Down,
                Orientation::Up,
            )?,
            Point::new(14, 11),
        ),
        placed_component(
            load,
            graph,
            oriented_between(
                load,
                graph,
                output,
                NetId::GROUND,
                Orientation::Down,
                Orientation::Up,
            )?,
            Point::new(20, 8),
        ),
        placed_component(supply_source, graph, Orientation::Down, Point::new(2, 15)),
    ];
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Some(placed)
}

fn place_components(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> Vec<SchematicComponent> {
    if let Some(bridge) = bridge_placement(circuit, graph) {
        return bridge;
    }
    if let Some(pair) = differential_pair_placement(circuit, graph, supplies) {
        return pair;
    }
    if let Some(stage) = single_transistor_stage_placement(circuit, graph, supplies) {
        return stage;
    }
    if let Some(clamp) = diode_clamp_placement(circuit, graph, supplies) {
        return clamp;
    }
    let analysis = flow_analysis(circuit, graph, supplies);
    let mut layers: BTreeMap<(usize, PlacementLane), Vec<&IRComponent>> = BTreeMap::new();
    for component in &circuit.components {
        layers
            .entry((
                effective_placement_rank(component, circuit, graph, &analysis.component_ranks),
                placement_lane(component, circuit, graph, supplies),
            ))
            .or_default()
            .push(component);
    }
    for components in layers.values_mut() {
        components.sort_by(|left, right| left.id.cmp(&right.id));
    }

    const MAIN_Y: i32 = 10;
    let compact_passive_chain = circuit.components.iter().all(|component| {
        is_two_pin_passive(&component.kind)
            || matches!(
                component.kind,
                ComponentKind::VoltageSource | ComponentKind::CurrentSource
            )
    });
    let rank_spacing = if circuit.components.len() <= 4 || compact_passive_chain {
        6
    } else {
        8
    };
    let main_counts: BTreeMap<_, _> = layers
        .iter()
        .filter_map(|((rank, lane), components)| {
            (*lane == PlacementLane::Main).then_some((*rank, components.len()))
        })
        .collect();
    let mut result = Vec::new();
    let mut power_index = 0_i32;
    for ((rank, lane), components) in layers {
        for (index, component) in components.iter().enumerate() {
            let orientation = feedback_orientation(component, circuit, graph)
                .unwrap_or_else(|| orientation_for(component, graph));
            let rank_x = 4 + i32::try_from(rank).unwrap_or(i32::MAX / rank_spacing) * rank_spacing;
            let index = i32::try_from(index).unwrap_or(i32::MAX / 6);
            let desired = match lane {
                PlacementLane::Upper => Point::new(rank_x, 3 + index * 4),
                PlacementLane::Main => {
                    let count = i32::try_from(components.len()).unwrap_or(i32::MAX / 7);
                    let row_offset = if count == 1 {
                        0
                    } else {
                        index * 7 - (count - 1) * 3
                    };
                    let y = match component.kind {
                        ComponentKind::OpAmp => MAIN_Y - 2 + row_offset,
                        ComponentKind::BJT(_) | ComponentKind::MOSFET(_) => MAIN_Y - 1 + row_offset,
                        _ => MAIN_Y + row_offset,
                    };
                    Point::new(rank_x, y)
                }
                PlacementLane::Lower => {
                    let main_count = i32::try_from(main_counts.get(&rank).copied().unwrap_or(0))
                        .unwrap_or(i32::MAX / 7);
                    let contains_feedback = components
                        .iter()
                        .any(|candidate| is_feedback_passive(candidate, circuit, graph));
                    let (column, row) = if contains_feedback {
                        (0, index)
                    } else if components.len() > 4 {
                        (index / 4, index % 4)
                    } else {
                        (index, 0)
                    };
                    Point::new(
                        rank_x + column * 5,
                        MAIN_Y + 5 + (main_count - 1).max(0) * 7 + row * 6,
                    )
                }
                PlacementLane::PowerBlock => {
                    let point = Point::new(1, MAIN_Y + 7 + power_index * 6);
                    power_index += 1;
                    point
                }
            };
            result.push(placed_component(component, graph, orientation, desired));
        }
    }
    result.sort_by(|left, right| left.id.cmp(&right.id));
    result
}

fn build_nets(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    _supplies: &BTreeSet<NetId>,
) -> Vec<SchematicNet> {
    let mut pins: BTreeMap<NetId, Vec<PinRef>> = BTreeMap::new();
    for component in &circuit.components {
        for pin in component_definition(&component.kind).pins {
            if let Some(net) = graph.get_net(&component.id, pin.name) {
                pins.entry(net)
                    .or_default()
                    .push(PinRef::new(&component.id, pin.name));
            }
        }
    }
    pins.into_iter()
        .map(|(id, mut pins)| {
            pins.sort();
            let name = graph.get_net_name(id);
            let named_supply = matches!(
                name.to_ascii_uppercase().as_str(),
                "VCC" | "VDD" | "VEE" | "VSS" | "+V" | "-V"
            );
            let kind = if id == NetId::GROUND {
                NetKind::Ground
            } else if named_supply {
                NetKind::Supply
            } else {
                NetKind::Signal
            };
            SchematicNet {
                id,
                name,
                kind,
                pins,
            }
        })
        .collect()
}

fn is_explicit_name(circuit: &CircuitIR, name: &str) -> bool {
    circuit.nets.iter().any(|candidate| candidate == name)
}

fn label_net(circuit: &CircuitIR, net: &SchematicNet) -> bool {
    net.kind != NetKind::Signal || (is_explicit_name(circuit, &net.name) && net.pins.len() >= 8)
}

fn route_bounds(components: &[SchematicComponent], net_count: usize) -> Rect {
    let min_x = components
        .iter()
        .map(|component| component.bounds.min.x)
        .min()
        .unwrap_or(0);
    let min_y = components
        .iter()
        .map(|component| component.bounds.min.y)
        .min()
        .unwrap_or(0);
    let max_x = components
        .iter()
        .map(|component| component.bounds.max.x)
        .max()
        .unwrap_or(0);
    let max_y = components
        .iter()
        .map(|component| component.bounds.max.y)
        .max()
        .unwrap_or(0);
    let margin = 8 + net_count as i32 * 2;
    Rect {
        min: Point::new(min_x - margin, min_y - margin),
        max: Point::new(max_x + margin, max_y + margin),
    }
}

fn blocked_points(components: &[SchematicComponent]) -> BTreeSet<Point> {
    let mut blocked = BTreeSet::new();
    for component in components {
        for x in component.bounds.min.x..=component.bounds.max.x {
            for y in component.bounds.min.y..=component.bounds.max.y {
                blocked.insert(Point::new(x, y));
            }
        }
    }
    blocked
}

fn compress_path(points: Vec<Point>) -> Vec<Point> {
    let mut result: Vec<Point> = Vec::new();
    for point in points {
        if result.len() >= 2 {
            let a = result[result.len() - 2];
            let b = result[result.len() - 1];
            if (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y) {
                result.pop();
            }
        }
        if result.last() != Some(&point) {
            result.push(point);
        }
    }
    result
}

fn route_grid(
    start: Point,
    goal: Point,
    bounds: Rect,
    blocked: &BTreeSet<Point>,
    occupied: &BTreeMap<Point, NetId>,
    net: NetId,
) -> Option<Vec<Point>> {
    if start == goal {
        return Some(vec![start]);
    }
    const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    let start_state = (start, 4_u8);
    let mut frontier = BinaryHeap::new();
    frontier.push(Reverse((0_u32, start.x, start.y, 4_u8)));
    let mut costs = BTreeMap::new();
    costs.insert(start_state, 0_u32);
    let mut previous: BTreeMap<(Point, u8), (Point, u8)> = BTreeMap::new();
    let mut found = None;

    while let Some(Reverse((cost, x, y, direction))) = frontier.pop() {
        let point = Point::new(x, y);
        if point == goal {
            found = Some((point, direction));
            break;
        }
        if costs.get(&(point, direction)).copied() != Some(cost) {
            continue;
        }
        for (next_direction, (dx, dy)) in DIRECTIONS.iter().copied().enumerate() {
            let next = point.offset(dx, dy);
            if !bounds.contains(next) || (blocked.contains(&next) && next != goal) {
                continue;
            }
            let bend_cost = u32::from(direction != 4 && direction != next_direction as u8) * 4;
            let crossing_cost = occupied
                .get(&next)
                .filter(|occupied_net| **occupied_net != net)
                .map_or(0, |_| 10_000);
            let next_cost = cost + 1 + bend_cost + crossing_cost;
            let next_state = (next, next_direction as u8);
            if costs
                .get(&next_state)
                .is_none_or(|existing| next_cost < *existing)
            {
                costs.insert(next_state, next_cost);
                previous.insert(next_state, (point, direction));
                frontier.push(Reverse((next_cost, next.x, next.y, next_direction as u8)));
            }
        }
    }

    let mut state = found?;
    let mut path = vec![state.0];
    while state != start_state {
        state = previous[&state];
        path.push(state.0);
    }
    path.reverse();
    Some(compress_path(path))
}

fn nearest_open_hub(candidate: Point, bounds: Rect, blocked: &BTreeSet<Point>) -> Option<Point> {
    if bounds.contains(candidate) && !blocked.contains(&candidate) {
        return Some(candidate);
    }
    for radius in 1_i32..=12 {
        for dx in -radius..=radius {
            let dy = radius - dx.abs();
            for signed_dy in [dy, -dy] {
                let point = candidate.offset(dx, signed_dy);
                if bounds.contains(point) && !blocked.contains(&point) {
                    return Some(point);
                }
            }
        }
    }
    None
}

fn orthogonal_route(
    start: Point,
    goal: Point,
    blocked: &BTreeSet<Point>,
    occupied: &BTreeMap<Point, NetId>,
    net: NetId,
) -> Option<Vec<Point>> {
    let candidates = [
        vec![start, Point::new(goal.x, start.y), goal],
        vec![start, Point::new(start.x, goal.y), goal],
    ];
    candidates
        .into_iter()
        .map(compress_path)
        .filter_map(|path| {
            let points = points_on_path(&path);
            if points
                .iter()
                .any(|point| *point != start && *point != goal && blocked.contains(point))
            {
                return None;
            }
            let crossing_cost = points
                .iter()
                .filter(|point| occupied.get(point).is_some_and(|other| *other != net))
                .count()
                * 1_000;
            let length = points.len();
            Some((crossing_cost + length, path))
        })
        .min_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)))
        .map(|(_, path)| path)
}

fn points_on_path(points: &[Point]) -> Vec<Point> {
    let mut result = Vec::new();
    for pair in points.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let dx = (end.x - start.x).signum();
        let dy = (end.y - start.y).signum();
        let mut current = start;
        if result.last() != Some(&current) {
            result.push(current);
        }
        while current != end {
            current = current.offset(dx, dy);
            result.push(current);
        }
    }
    if points.len() == 1 {
        result.push(points[0]);
    }
    result
}

fn pin_maps(
    components: &[SchematicComponent],
) -> (BTreeMap<PinRef, (Point, Point)>, BTreeMap<Point, PinRef>) {
    let mut anchors = BTreeMap::new();
    let mut by_point = BTreeMap::new();
    for component in components {
        for pin in &component.pins {
            let reference = PinRef::new(&component.id, &pin.name);
            anchors.insert(reference.clone(), (pin.point, pin.escape));
            by_point.insert(pin.point, reference);
        }
    }
    (anchors, by_point)
}

fn pin_flow(circuit: &CircuitIR, reference: &PinRef) -> Option<PinFlow> {
    let component = circuit
        .components
        .iter()
        .find(|component| component.id == reference.component)?;
    component_definition(&component.kind)
        .pins
        .iter()
        .find(|pin| pin.name == reference.pin)
        .map(|pin| pin.flow)
}

type RoutedElements = (
    Vec<SchematicWire>,
    Vec<Junction>,
    Vec<Crossing>,
    Vec<NetLabel>,
);

fn route_schematic(
    circuit: &CircuitIR,
    components: &[SchematicComponent],
    nets: &[SchematicNet],
) -> Result<RoutedElements, SchematicError> {
    let (anchors, pin_points) = pin_maps(components);
    let bounds = route_bounds(components, nets.len());
    let blocked = blocked_points(components);
    let mut occupied: BTreeMap<Point, NetId> = BTreeMap::new();
    let mut wires = Vec::new();
    let mut junctions = Vec::new();
    let mut labels = Vec::new();
    let mut wire_counter = 1;
    let mut junction_counter = 1;

    for net in nets {
        if net.pins.len() < 2 {
            continue;
        }
        if label_net(circuit, net) {
            for (label_index, pin) in net.pins.iter().enumerate() {
                let (_point, escape) = anchors.get(pin).ok_or_else(|| SchematicError {
                    message: format!("missing anchor for {}", pin.id()),
                })?;
                let side = components
                    .iter()
                    .find(|component| component.id == pin.component)
                    .and_then(|component| {
                        component.pins.iter().find(|anchor| anchor.name == pin.pin)
                    })
                    .map(|anchor| anchor.side)
                    .unwrap_or(PinSide::Right);
                let offset = i32::try_from(label_index % 2).unwrap_or(0);
                let label_point = match side {
                    PinSide::Left => escape.offset(-1, -offset),
                    PinSide::Right => escape.offset(1, -offset),
                    PinSide::Top => escape.offset(0, -1),
                    PinSide::Bottom => escape.offset(0, 1),
                };
                labels.push(NetLabel {
                    id: format!("L{:04}", labels.len() + 1),
                    net: net.id,
                    text: net.name.clone(),
                    kind: net.kind,
                    point: if matches!(net.kind, NetKind::Ground | NetKind::Supply) {
                        *escape
                    } else {
                        label_point
                    },
                    attached_to: pin.clone(),
                });
            }
            continue;
        }

        if net.pins.len() == 2 {
            let start_ref = &net.pins[0];
            let end_ref = &net.pins[1];
            let (start_point, start_escape) = anchors[start_ref];
            let (end_point, end_escape) = anchors[end_ref];
            let middle = orthogonal_route(start_escape, end_escape, &blocked, &occupied, net.id)
                .or_else(|| {
                    route_grid(
                        start_escape,
                        end_escape,
                        bounds,
                        &blocked,
                        &occupied,
                        net.id,
                    )
                })
                .ok_or_else(|| SchematicError {
                    message: format!("could not route net '{}'", net.name),
                })?;
            let mut points = vec![start_point];
            points.extend(middle);
            points.push(end_point);
            let points = compress_path(points);
            for point in points_on_path(&points) {
                occupied.entry(point).or_insert(net.id);
            }
            wires.push(SchematicWire {
                id: format!("W{wire_counter:04}"),
                net: net.id,
                start: WireEndpoint::pin(start_ref),
                end: WireEndpoint::pin(end_ref),
                points,
            });
            wire_counter += 1;
            continue;
        }

        let mut xs: Vec<_> = net.pins.iter().map(|pin| anchors[pin].1.x).collect();
        xs.sort();
        let mut ys: Vec<_> = net.pins.iter().map(|pin| anchors[pin].1.y).collect();
        ys.sort();
        let driver_escapes: Vec<_> = net
            .pins
            .iter()
            .filter(|pin| pin_flow(circuit, pin) == Some(PinFlow::Output))
            .map(|pin| anchors[pin].1)
            .collect();
        let candidate = if driver_escapes.len() == 1 {
            driver_escapes[0]
        } else {
            Point::new(xs[xs.len() / 2], ys[ys.len() / 2])
        };
        let hub = nearest_open_hub(candidate, bounds, &blocked).ok_or_else(|| SchematicError {
            message: format!("could not place junction for net '{}'", net.name),
        })?;
        let junction_id = format!("J{junction_counter:04}");
        junction_counter += 1;
        junctions.push(Junction {
            id: junction_id.clone(),
            net: net.id,
            point: hub,
        });
        let mut routed_pins = net.pins.clone();
        routed_pins.sort_by_key(|pin| {
            let escape = anchors[pin].1;
            (
                (escape.x - hub.x).abs() + (escape.y - hub.y).abs(),
                pin.clone(),
            )
        });
        for pin in &routed_pins {
            let (pin_point, escape) = anchors[pin];
            let middle = orthogonal_route(escape, hub, &blocked, &occupied, net.id)
                .or_else(|| route_grid(escape, hub, bounds, &blocked, &occupied, net.id))
                .ok_or_else(|| {
                    SchematicError {
                        message: format!(
                            "could not route branch of net '{}' from {} at ({}, {}) to junction ({}, {})",
                            net.name,
                            pin.id(),
                            escape.x,
                            escape.y,
                            hub.x,
                            hub.y
                        ),
                    }
                })?;
            let mut points = vec![pin_point];
            points.extend(middle);
            let points = compress_path(points);
            for point in points_on_path(&points) {
                occupied.entry(point).or_insert(net.id);
            }
            wires.push(SchematicWire {
                id: format!("W{wire_counter:04}"),
                net: net.id,
                start: WireEndpoint::pin(pin),
                end: WireEndpoint::Junction {
                    id: junction_id.clone(),
                },
                points,
            });
            wire_counter += 1;
        }
    }

    let mut point_wires: BTreeMap<Point, Vec<(&str, NetId)>> = BTreeMap::new();
    for wire in &wires {
        for point in points_on_path(&wire.points) {
            point_wires
                .entry(point)
                .or_default()
                .push((&wire.id, wire.net));
        }
    }
    let mut crossings = Vec::new();
    for (point, entries) in point_wires {
        let nets_here: BTreeSet<_> = entries.iter().map(|(_, net)| *net).collect();
        if nets_here.len() > 1 && !pin_points.contains_key(&point) {
            let nets: Vec<_> = nets_here.into_iter().collect();
            for pair in nets.windows(2) {
                crossings.push(Crossing {
                    id: format!("X{:04}", crossings.len() + 1),
                    point,
                    nets: [pair[0], pair[1]],
                });
            }
        }
    }
    junctions.sort_by(|left, right| left.id.cmp(&right.id));
    crossings.sort_by_key(|crossing| (crossing.point, crossing.nets));
    for (index, crossing) in crossings.iter_mut().enumerate() {
        crossing.id = format!("X{:04}", index + 1);
    }
    labels.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((wires, junctions, crossings, labels))
}

fn endpoint_pin(endpoint: &WireEndpoint) -> Option<PinRef> {
    match endpoint {
        WireEndpoint::Pin { component, pin } => Some(PinRef::new(component, pin)),
        WireEndpoint::Junction { .. } => None,
    }
}

fn connectivity_report(
    graph: &NetlistGraph,
    wires: &[SchematicWire],
    labels: &[NetLabel],
) -> ConnectivityReport {
    let mut expected: BTreeMap<NetId, BTreeSet<PinRef>> = BTreeMap::new();
    for (pin_id, net) in &graph.pin_to_net {
        if let Some((component, pin)) = pin_id.split_once('.') {
            expected
                .entry(*net)
                .or_default()
                .insert(PinRef::new(component, pin));
        }
    }
    let mut represented: BTreeMap<NetId, BTreeSet<PinRef>> = BTreeMap::new();
    for wire in wires {
        for endpoint in [&wire.start, &wire.end] {
            if let Some(pin) = endpoint_pin(endpoint) {
                represented.entry(wire.net).or_default().insert(pin);
            }
        }
    }
    for label in labels {
        represented
            .entry(label.net)
            .or_default()
            .insert(label.attached_to.clone());
    }
    let mut errors = Vec::new();
    for (net, expected_pins) in &expected {
        let represented_pins = represented.get(net).cloned().unwrap_or_default();
        if &represented_pins != expected_pins {
            let missing: Vec<_> = expected_pins
                .difference(&represented_pins)
                .map(PinRef::id)
                .collect();
            let extra: Vec<_> = represented_pins
                .difference(expected_pins)
                .map(PinRef::id)
                .collect();
            errors.push(format!(
                "net {net} connectivity mismatch; missing=[{}], extra=[{}]",
                missing.join(","),
                extra.join(",")
            ));
        }
    }
    for net in represented.keys() {
        if !expected.contains_key(net) {
            errors.push(format!("schematic invented net {net}"));
        }
    }
    ConnectivityReport {
        verified: errors.is_empty(),
        expected_connected_pins: expected.values().map(BTreeSet::len).sum(),
        represented_connected_pins: represented.values().map(BTreeSet::len).sum(),
        errors,
    }
}

struct QualityInput<'a> {
    circuit: &'a CircuitIR,
    graph: &'a NetlistGraph,
    supplies: &'a BTreeSet<NetId>,
    components: &'a [SchematicComponent],
    nets: &'a [SchematicNet],
    wires: &'a [SchematicWire],
    labels: &'a [NetLabel],
    crossings: &'a [Crossing],
    bounds: Rect,
}

fn quality_report(input: QualityInput<'_>) -> QualityReport {
    let QualityInput {
        circuit,
        graph,
        supplies,
        components,
        nets,
        wires,
        labels,
        crossings,
        bounds,
    } = input;
    let mut symbol_collisions = 0;
    for (index, left) in components.iter().enumerate() {
        for right in &components[index + 1..] {
            if left.bounds.overlaps(right.bounds) {
                symbol_collisions += 1;
            }
        }
    }

    let mut wire_symbol_hits = BTreeSet::new();
    for wire in wires {
        let endpoint_components: BTreeSet<_> = [&wire.start, &wire.end]
            .into_iter()
            .filter_map(endpoint_pin)
            .map(|pin| pin.component)
            .collect();
        for point in points_on_path(&wire.points) {
            for component in components {
                if !endpoint_components.contains(&component.id) && component.bounds.contains(point)
                {
                    wire_symbol_hits.insert((wire.id.clone(), component.id.clone()));
                }
            }
        }
    }

    let mut label_symbol_hits = BTreeSet::new();
    for label in labels {
        let character_count = i32::try_from(label.text.chars().count()).unwrap_or(i32::MAX - 2);
        let width = (character_count + 2) / 3;
        let label_bounds = Rect {
            min: label.point.offset(-1, -1),
            max: label.point.offset(width.max(1), 1),
        };
        for component in components {
            if component.id != label.attached_to.component
                && label_bounds.overlaps(component.bounds)
            {
                label_symbol_hits.insert((label.id.clone(), component.id.clone()));
            }
        }
    }

    let bends = wires
        .iter()
        .map(|wire| wire.points.len().saturating_sub(2))
        .sum();
    let wire_length = wires
        .iter()
        .flat_map(|wire| wire.points.windows(2))
        .map(|pair| {
            usize::try_from((pair[1].x - pair[0].x).abs() + (pair[1].y - pair[0].y).abs())
                .unwrap_or(usize::MAX)
        })
        .sum();
    let wire_pin_refs: BTreeSet<_> = wires
        .iter()
        .flat_map(|wire| [&wire.start, &wire.end])
        .filter_map(endpoint_pin)
        .collect();
    let label_pin_refs: BTreeSet<_> = labels
        .iter()
        .map(|label| label.attached_to.clone())
        .collect();
    let local_signal_nets: Vec<_> = nets
        .iter()
        .filter(|net| net.kind == NetKind::Signal && net.pins.len() < 8)
        .collect();
    let global_signal_nets: Vec<_> = nets
        .iter()
        .filter(|net| net.kind == NetKind::Signal && net.pins.len() >= 8)
        .collect();
    let local_signal_pins = local_signal_nets.iter().map(|net| net.pins.len()).sum();
    let wired_local_signal_pins = local_signal_nets
        .iter()
        .flat_map(|net| &net.pins)
        .filter(|pin| wire_pin_refs.contains(*pin))
        .count();
    let local_signal_label_pins = local_signal_nets
        .iter()
        .flat_map(|net| &net.pins)
        .filter(|pin| label_pin_refs.contains(*pin))
        .count();
    let global_signal_label_pins = global_signal_nets
        .iter()
        .flat_map(|net| &net.pins)
        .filter(|pin| label_pin_refs.contains(*pin))
        .count();
    let explicit_wire_coverage_per_mille = wired_local_signal_pins
        .saturating_mul(1_000)
        .checked_div(local_signal_pins)
        .unwrap_or(1_000);

    let analysis = flow_analysis(circuit, graph, supplies);
    let positions: BTreeMap<_, _> = components
        .iter()
        .map(|component| {
            (
                component.id.as_str(),
                (component.bounds.min.x + component.bounds.max.x) / 2,
            )
        })
        .collect();
    let main_components: Vec<_> = circuit
        .components
        .iter()
        .filter(|component| {
            placement_lane(component, circuit, graph, supplies) == PlacementLane::Main
        })
        .collect();
    let mut flow_inversions = 0;
    if bridge_placement(circuit, graph).is_none()
        && differential_pair_placement(circuit, graph, supplies).is_none()
    {
        for (index, left) in main_components.iter().enumerate() {
            for right in &main_components[index + 1..] {
                let left_rank = analysis.component_ranks[&left.id];
                let right_rank = analysis.component_ranks[&right.id];
                let left_x = positions[left.id.as_str()];
                let right_x = positions[right.id.as_str()];
                if (left_rank < right_rank && left_x >= right_x)
                    || (right_rank < left_rank && right_x >= left_x)
                {
                    flow_inversions += 1;
                }
            }
        }
    }

    let content_width = usize::try_from((bounds.max.x - bounds.min.x).max(0)).unwrap_or(usize::MAX);
    let content_height =
        usize::try_from((bounds.max.y - bounds.min.y).max(0)).unwrap_or(usize::MAX);
    let aspect_ratio_milli = content_width
        .saturating_mul(1_000)
        .checked_div(content_height)
        .unwrap_or(0);
    let occupied_area: usize = components
        .iter()
        .map(|component| {
            usize::try_from((component.bounds.max.x - component.bounds.min.x + 1).max(1))
                .unwrap_or(usize::MAX)
                .saturating_mul(
                    usize::try_from((component.bounds.max.y - component.bounds.min.y + 1).max(1))
                        .unwrap_or(usize::MAX),
                )
        })
        .sum();
    let bounds_area = content_width.max(1).saturating_mul(content_height.max(1));
    let occupied_area_per_mille = occupied_area.saturating_mul(1_000) / bounds_area;
    let mut issues = Vec::new();
    if symbol_collisions > 0 {
        issues.push(format!("{symbol_collisions} symbol collision(s)"));
    }
    if !wire_symbol_hits.is_empty() {
        issues.push(format!(
            "{} wire-symbol collision(s)",
            wire_symbol_hits.len()
        ));
    }
    if !label_symbol_hits.is_empty() {
        let examples = label_symbol_hits
            .iter()
            .take(4)
            .map(|(label, component)| format!("{label}->{component}"))
            .collect::<Vec<_>>()
            .join(", ");
        issues.push(format!(
            "{} label-symbol collision(s): {examples}",
            label_symbol_hits.len(),
        ));
    }
    let crossing_limit = (components.len() / 3).min(2);
    if crossings.len() > crossing_limit {
        issues.push(format!(
            "{} geometry crossing(s), limit is {crossing_limit}",
            crossings.len()
        ));
    }
    if local_signal_label_pins > 0 {
        issues.push(format!(
            "{local_signal_label_pins} local signal pin(s) hidden behind labels"
        ));
    }
    if explicit_wire_coverage_per_mille < 1_000 {
        issues.push(format!(
            "local signal explicit-wire coverage is {explicit_wire_coverage_per_mille}/1000"
        ));
    }
    if flow_inversions > 0 {
        issues.push(format!("{flow_inversions} primary-flow inversion(s)"));
    }
    QualityReport {
        passed: symbol_collisions == 0
            && wire_symbol_hits.is_empty()
            && label_symbol_hits.is_empty()
            && crossings.len() <= crossing_limit
            && local_signal_label_pins == 0
            && explicit_wire_coverage_per_mille == 1_000
            && flow_inversions == 0,
        symbol_collisions,
        wire_symbol_collisions: wire_symbol_hits.len(),
        label_symbol_collisions: label_symbol_hits.len(),
        crossings: crossings.len(),
        crossing_limit,
        bends,
        wire_length,
        local_signal_pins,
        wired_local_signal_pins,
        explicit_wire_coverage_per_mille,
        global_signal_label_pins,
        local_signal_label_pins,
        flow_inversions,
        content_width,
        content_height,
        aspect_ratio_milli,
        occupied_area_per_mille,
        issues,
    }
}

fn schematic_bounds(
    components: &[SchematicComponent],
    wires: &[SchematicWire],
    labels: &[NetLabel],
) -> Rect {
    let mut points = Vec::new();
    for component in components {
        points.push(component.bounds.min);
        points.push(component.bounds.max);
    }
    for wire in wires {
        points.extend(wire.points.iter().copied());
    }
    points.extend(labels.iter().map(|label| label.point));
    Rect {
        min: Point::new(
            points.iter().map(|point| point.x).min().unwrap_or(0) - 2,
            points.iter().map(|point| point.y).min().unwrap_or(0) - 2,
        ),
        max: Point::new(
            points.iter().map(|point| point.x).max().unwrap_or(0) + 4,
            points.iter().map(|point| point.y).max().unwrap_or(0) + 2,
        ),
    }
}

pub fn generate_schematic(circuit: &CircuitIR) -> Result<Schematic, SchematicError> {
    let graph = NetlistGraph::build(circuit);
    let supplies = supply_nets(circuit, &graph);
    let components = place_components(circuit, &graph, &supplies);
    let nets = build_nets(circuit, &graph, &supplies);
    let (wires, junctions, crossings, labels) = route_schematic(circuit, &components, &nets)?;
    let connectivity = connectivity_report(&graph, &wires, &labels);
    if !connectivity.verified {
        return Err(SchematicError {
            message: format!(
                "schematic connectivity verification failed: {}",
                connectivity.errors.join("; ")
            ),
        });
    }
    let bounds = schematic_bounds(&components, &wires, &labels);
    let quality = quality_report(QualityInput {
        circuit,
        graph: &graph,
        supplies: &supplies,
        components: &components,
        nets: &nets,
        wires: &wires,
        labels: &labels,
        crossings: &crossings,
        bounds,
    });
    Ok(Schematic {
        schema_version: SCHEMATIC_SCHEMA_VERSION.to_string(),
        components,
        nets,
        wires,
        junctions,
        crossings,
        labels,
        bounds,
        connectivity,
        quality,
    })
}
