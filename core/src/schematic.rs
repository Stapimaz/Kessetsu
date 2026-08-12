use crate::component::{CatalogSymbol, PinSide, component_definition};
use crate::graph::{NetId, NetlistGraph, format_spice_number};
use crate::ir::{
    BJTPolarity, CircuitIR, ComponentKind, ComponentParams, FETPolarity, IRComponent, SIUnit,
    SourceValue, Waveform,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

pub const SCHEMATIC_SCHEMA_VERSION: &str = "netlang.schematic.v1";

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
    format!("{}{}", format_spice_number(value), unit_suffix(unit))
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
        } => format!(
            "SINE {} {} {}",
            quantity_label(offset.value, offset.unit),
            quantity_label(amplitude.value, amplitude.unit),
            quantity_label(frequency.value, frequency.unit)
        ),
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
    }
    Orientation::Right
}

fn local_bounds(component: &IRComponent, orientation: Orientation) -> Rect {
    let definition = component_definition(&component.kind);
    let mut points = vec![
        orientation.rotate_point(Point::new(0, 0)),
        orientation.rotate_point(Point::new(definition.width, 0)),
        orientation.rotate_point(Point::new(0, definition.height)),
        orientation.rotate_point(Point::new(definition.width, definition.height)),
    ];
    points.extend(
        definition
            .pins
            .iter()
            .map(|pin| orientation.rotate_point(Point::new(pin.x, pin.y))),
    );
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

fn supply_nets(circuit: &CircuitIR, graph: &NetlistGraph) -> BTreeSet<NetId> {
    let mut result = BTreeSet::new();
    for component in &circuit.components {
        if matches!(
            component.parameters,
            ComponentParams::VoltageSource {
                value: SourceValue::Dc(_)
            }
        ) {
            for pin in ["plus", "minus"] {
                if let Some(net) = graph.get_net(&component.id, pin)
                    && net != NetId::GROUND
                {
                    result.insert(net);
                }
            }
        }
    }
    result
}

fn component_ranks(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> BTreeMap<String, usize> {
    let mut net_components: BTreeMap<NetId, BTreeSet<String>> = BTreeMap::new();
    for component in &circuit.components {
        for pin in component_definition(&component.kind).pins {
            if let Some(net) = graph.get_net(&component.id, pin.name) {
                net_components
                    .entry(net)
                    .or_default()
                    .insert(component.id.clone());
            }
        }
    }

    let signal_sources: Vec<_> = circuit
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
    let has_signal_source = !signal_sources.is_empty();
    let traversal_sources = if !has_signal_source {
        all_sources.clone()
    } else {
        signal_sources
    };

    let by_id: BTreeMap<_, _> = circuit
        .components
        .iter()
        .map(|component| (component.id.clone(), component))
        .collect();
    let mut ranks = BTreeMap::new();
    let mut queue = VecDeque::new();
    for source in traversal_sources {
        ranks.insert(source.clone(), 0);
        queue.push_back(source);
    }

    while let Some(component_id) = queue.pop_front() {
        let rank = ranks[&component_id];
        let component = by_id[&component_id];
        for pin in component_definition(&component.kind).pins {
            let Some(net) = graph.get_net(&component.id, pin.name) else {
                continue;
            };
            if net == NetId::GROUND || (has_signal_source && supplies.contains(&net)) {
                continue;
            }
            if let Some(neighbors) = net_components.get(&net) {
                for neighbor in neighbors {
                    if !ranks.contains_key(neighbor) {
                        ranks.insert(neighbor.clone(), rank + 1);
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
    }

    for source in all_sources {
        ranks.entry(source).or_insert(0);
    }
    let fallback_rank = ranks.values().copied().max().unwrap_or(0) + 1;
    for component in &circuit.components {
        ranks.entry(component.id.clone()).or_insert(fallback_rank);
    }
    ranks
}

fn place_components(
    circuit: &CircuitIR,
    graph: &NetlistGraph,
    supplies: &BTreeSet<NetId>,
) -> Vec<SchematicComponent> {
    let ranks = component_ranks(circuit, graph, supplies);
    let mut layers: BTreeMap<usize, Vec<&IRComponent>> = BTreeMap::new();
    for component in &circuit.components {
        layers
            .entry(ranks[&component.id])
            .or_default()
            .push(component);
    }
    for components in layers.values_mut() {
        components.sort_by(|left, right| {
            let priority = |component: &IRComponent| match component.parameters {
                ComponentParams::VoltageSource {
                    value: SourceValue::Waveform(_),
                }
                | ComponentParams::CurrentSource {
                    value: SourceValue::Waveform(_),
                } => 0,
                ComponentParams::VoltageSource { .. } | ComponentParams::CurrentSource { .. } => 1,
                _ => 2,
            };
            priority(left)
                .cmp(&priority(right))
                .then(left.id.cmp(&right.id))
        });
    }

    let mut result = Vec::new();
    let mut base_x = 5;
    for components in layers.into_values() {
        const MAX_ROWS: usize = 4;
        let column_count = components.len().div_ceil(MAX_ROWS);
        for (index, component) in components.into_iter().enumerate() {
            let column = index / MAX_ROWS;
            let row = index % MAX_ROWS;
            let orientation = orientation_for(component, graph);
            let local = local_bounds(component, orientation);
            let desired = Point::new(base_x + column as i32 * 8, 5 + row as i32 * 9);
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
                    let point = orientation
                        .rotate_point(Point::new(pin.x, pin.y))
                        .offset(origin.x, origin.y);
                    let side = orientation.rotate_side(pin.side);
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
            result.push(SchematicComponent {
                id: component.id.clone(),
                symbol: definition.symbol,
                variant,
                reference: component.id.clone(),
                value,
                model,
                orientation,
                origin,
                bounds,
                pins,
            });
        }
        base_x += i32::try_from(column_count.max(1)).unwrap_or(i32::MAX / 8) * 8 + 4;
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
    net.kind != NetKind::Signal || (is_explicit_name(circuit, &net.name) && net.pins.len() >= 3)
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
        for pin in &component.pins {
            blocked.insert(pin.escape);
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
    let component_max_x = components
        .iter()
        .map(|component| component.bounds.max.x)
        .max()
        .unwrap_or(0);
    let mut occupied: BTreeMap<Point, NetId> = BTreeMap::new();
    let mut wires = Vec::new();
    let mut junctions = Vec::new();
    let mut labels = Vec::new();
    let mut wire_counter = 1;
    let mut junction_counter = 1;

    for (net_index, net) in nets.iter().enumerate() {
        if net.pins.len() < 2 {
            continue;
        }
        if label_net(circuit, net) {
            for (label_index, pin) in net.pins.iter().enumerate() {
                let (_, escape) = anchors.get(pin).ok_or_else(|| SchematicError {
                    message: format!("missing anchor for {}", pin.id()),
                })?;
                labels.push(NetLabel {
                    id: format!("L{:04}", labels.len() + 1),
                    net: net.id,
                    text: net.name.clone(),
                    kind: net.kind,
                    point: escape.offset(0, -(label_index as i32 % 2)),
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
            let middle = route_grid(
                start_escape,
                end_escape,
                bounds,
                &blocked,
                &occupied,
                net.id,
            )
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

        let mut ys: Vec<_> = net.pins.iter().map(|pin| anchors[pin].1.y).collect();
        ys.sort();
        let hub = Point::new(component_max_x + 4 + net_index as i32 * 2, ys[ys.len() / 2]);
        let junction_id = format!("J{junction_counter:04}");
        junction_counter += 1;
        junctions.push(Junction {
            id: junction_id.clone(),
            net: net.id,
            point: hub,
        });
        for pin in &net.pins {
            let (pin_point, escape) = anchors[pin];
            let middle =
                route_grid(escape, hub, bounds, &blocked, &occupied, net.id).ok_or_else(|| {
                    SchematicError {
                        message: format!("could not route branch of net '{}'", net.name),
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

fn quality_report(
    components: &[SchematicComponent],
    wires: &[SchematicWire],
    labels: &[NetLabel],
    crossings: &[Crossing],
) -> QualityReport {
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
        issues.push(format!(
            "{} label-symbol collision(s)",
            label_symbol_hits.len()
        ));
    }
    let crossing_limit = (components.len() / 3).min(2);
    if crossings.len() > crossing_limit {
        issues.push(format!(
            "{} geometry crossing(s), limit is {crossing_limit}",
            crossings.len()
        ));
    }
    QualityReport {
        passed: symbol_collisions == 0
            && wire_symbol_hits.is_empty()
            && label_symbol_hits.is_empty()
            && crossings.len() <= crossing_limit,
        symbol_collisions,
        wire_symbol_collisions: wire_symbol_hits.len(),
        label_symbol_collisions: label_symbol_hits.len(),
        crossings: crossings.len(),
        crossing_limit,
        bends,
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
    let quality = quality_report(&components, &wires, &labels, &crossings);
    let bounds = schematic_bounds(&components, &wires, &labels);
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
