//! Coordinate-based electrical proof shared by schematic and EDA adapters.
//! Wire tags identify intended ownership; actual paths must also realize that
//! ownership without islands, foreign contacts or ambiguous intersections.
use crate::graph::NetId;
use crate::schematic::{
    Crossing, Junction, NetLabel, PinRef, Point, SchematicComponent, SchematicWire, WireEndpoint,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn geometry_errors(
    components: &[SchematicComponent],
    wires: &[SchematicWire],
    junctions: &[Junction],
    labels: &[NetLabel],
    crossings: &[Crossing],
) -> Vec<String> {
    let mut errors = BTreeSet::new();
    let pins: BTreeMap<_, _> = components
        .iter()
        .flat_map(|component| {
            component.pins.iter().map(move |pin| {
                (
                    PinRef {
                        component: component.id.clone(),
                        pin: pin.name.clone(),
                    },
                    pin,
                )
            })
        })
        .collect();
    let hubs: BTreeMap<_, _> = junctions.iter().map(|hub| (hub.id.as_str(), hub)).collect();
    let mut protected = BTreeSet::new();
    protected.extend(pins.values().map(|pin| pin.point));
    protected.extend(junctions.iter().map(|hub| hub.point));
    let mut edges: BTreeMap<(Point, NetId), BTreeSet<Point>> = BTreeMap::new();
    let mut owners: BTreeMap<Point, BTreeSet<NetId>> = BTreeMap::new();
    let mut add_point = |point, net| {
        owners.entry(point).or_default().insert(net);
    };
    for pin in pins.values() {
        if let Some(net) = pin.net {
            add_point(pin.point, net);
        }
    }
    for hub in junctions {
        add_point(hub.point, hub.net);
    }
    for wire in wires {
        let endpoint = |end: &WireEndpoint| match end {
            WireEndpoint::Pin { component, pin } => pins
                .get(&PinRef {
                    component: component.clone(),
                    pin: pin.clone(),
                })
                .and_then(|pin| pin.net.map(|net| (pin.point, net))),
            WireEndpoint::Junction { id } => hubs.get(id.as_str()).map(|hub| (hub.point, hub.net)),
        };
        for (end, point) in [
            (&wire.start, wire.points.first()),
            (&wire.end, wire.points.last()),
        ] {
            if endpoint(end) != point.map(|point| (*point, wire.net)) {
                errors.insert(format!(
                    "wire {} endpoint does not land on its declared pin/junction",
                    wire.id
                ));
            }
        }
        for point in [wire.points.first(), wire.points.last()]
            .into_iter()
            .flatten()
        {
            protected.insert(*point);
        }
        for point in &wire.points {
            add_point(*point, wire.net);
            edges.entry((*point, wire.net)).or_default();
        }
        for pair in wire.points.windows(2) {
            let (mut point, end) = (pair[0], pair[1]);
            if point.x != end.x && point.y != end.y {
                errors.insert(format!(
                    "wire {} contains a non-orthogonal segment",
                    wire.id
                ));
                continue;
            }
            let (dx, dy) = ((end.x - point.x).signum(), (end.y - point.y).signum());
            while point != end {
                let next = Point {
                    x: point.x + dx,
                    y: point.y + dy,
                };
                edges.entry((point, wire.net)).or_default().insert(next);
                edges.entry((next, wire.net)).or_default().insert(point);
                add_point(next, wire.net);
                point = next;
            }
        }
    }
    for label in labels {
        if pins
            .get(&label.attached_to)
            .is_none_or(|pin| pin.point != label.point || pin.net != Some(label.net))
        {
            errors.insert(format!(
                "label {} is not attached to its intended pin/net",
                label.id
            ));
        }
        add_point(label.point, label.net);
    }
    let straight_axis = |point: Point, net: NetId| -> Option<bool> {
        let neighbors = edges.get(&(point, net))?;
        if neighbors.len() != 2 {
            return None;
        }
        let values: Vec<_> = neighbors.iter().collect();
        if values[0].x == point.x
            && values[1].x == point.x
            && values[0].y + values[1].y == point.y * 2
        {
            Some(true)
        } else if values[0].y == point.y
            && values[1].y == point.y
            && values[0].x + values[1].x == point.x * 2
        {
            Some(false)
        } else {
            None
        }
    };
    let mut actual_crossings = BTreeSet::new();
    for (point, nets) in &owners {
        if nets.len() <= 1 {
            continue;
        }
        let nets: Vec<_> = nets.iter().copied().collect();
        let valid = nets.len() == 2
            && !protected.contains(point)
            && straight_axis(*point, nets[0])
                .zip(straight_axis(*point, nets[1]))
                .is_some_and(|(a, b)| a != b);
        if valid {
            actual_crossings.insert((*point, [nets[0], nets[1]]));
        } else {
            errors.insert(format!(
                "unintended/ambiguous net contact at ({}, {}): {:?}",
                point.x, point.y, nets
            ));
        }
    }
    let declared_crossings: BTreeSet<_> = crossings
        .iter()
        .map(|crossing| {
            let mut nets = crossing.nets;
            nets.sort();
            (crossing.point, nets)
        })
        .collect();
    if actual_crossings != declared_crossings {
        errors.insert(
            "crossing declarations do not match isolated transverse intersections".to_string(),
        );
    }
    for ((point, net), neighbors) in &edges {
        if neighbors.len() >= 3 && !protected.contains(point) {
            errors.insert(format!(
                "net {net} has an undeclared conductive join at ({}, {})",
                point.x, point.y
            ));
        }
    }
    // Equal semantic labels join conductive islands; unrelated wire tags do not.
    let mut label_points: BTreeMap<(NetId, String), BTreeSet<Point>> = BTreeMap::new();
    let mut labels_at_point: BTreeMap<(Point, NetId), BTreeSet<String>> = BTreeMap::new();
    let mut label_owners: BTreeMap<&str, NetId> = BTreeMap::new();
    for label in labels {
        if label_owners
            .insert(&label.text, label.net)
            .is_some_and(|other| other != label.net)
        {
            errors.insert(format!(
                "semantic label '{}' merges different intended nets",
                label.text
            ));
        }
        labels_at_point
            .entry((label.point, label.net))
            .or_default()
            .insert(label.text.clone());
        label_points
            .entry((label.net, label.text.clone()))
            .or_default()
            .insert(label.point);
    }
    let mut required: BTreeMap<NetId, BTreeSet<Point>> = BTreeMap::new();
    for pin in pins.values() {
        if let Some(net) = pin.net {
            required.entry(net).or_default().insert(pin.point);
        }
    }
    for (point, net) in edges.keys() {
        required.entry(*net).or_default().insert(*point);
    }
    for (net, points) in required {
        let Some(first) = points.first().copied() else {
            continue;
        };
        let mut seen = BTreeSet::new();
        let mut queue = vec![first];
        while let Some(point) = queue.pop() {
            if !seen.insert(point) {
                continue;
            }
            if let Some(adjacent) = edges.get(&(point, net)) {
                queue.extend(adjacent);
            }
            if let Some(names) = labels_at_point.get(&(point, net)) {
                for name in names {
                    queue.extend(&label_points[&(net, name.clone())]);
                }
            }
        }
        if !points.is_subset(&seen) {
            errors.insert(format!("net {net} contains disconnected geometric islands"));
        }
    }
    errors.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{CatalogSymbol, PinSide};
    use crate::schematic::{Orientation, PinAnchor, Rect};

    fn point(x: i32, y: i32) -> Point {
        Point { x, y }
    }
    fn pin(id: &str, net: NetId, point: Point) -> SchematicComponent {
        SchematicComponent {
            id: id.to_string(),
            symbol: CatalogSymbol::ModulePort,
            variant: None,
            reference: id.to_string(),
            value: None,
            model: None,
            model_metadata: None,
            orientation: Orientation::Right,
            mirrored_x: false,
            origin: point,
            bounds: Rect {
                min: point,
                max: point,
            },
            pins: vec![PinAnchor {
                name: "p".to_string(),
                net: Some(net),
                point,
                escape: point,
                side: PinSide::Right,
            }],
        }
    }
    fn wire(
        id: &str,
        net: NetId,
        a: &SchematicComponent,
        b: &SchematicComponent,
        points: Vec<Point>,
    ) -> SchematicWire {
        SchematicWire {
            id: id.to_string(),
            net,
            start: WireEndpoint::Pin {
                component: a.id.clone(),
                pin: "p".to_string(),
            },
            end: WireEndpoint::Pin {
                component: b.id.clone(),
                pin: "p".to_string(),
            },
            points,
        }
    }
    fn crossing_fixture() -> (Vec<SchematicComponent>, Vec<SchematicWire>, Vec<Crossing>) {
        let components = vec![
            pin("A", NetId(1), point(-2, 0)),
            pin("B", NetId(1), point(2, 0)),
            pin("C", NetId(2), point(0, -2)),
            pin("D", NetId(2), point(0, 2)),
        ];
        let wires = vec![
            wire(
                "H",
                NetId(1),
                &components[0],
                &components[1],
                vec![point(-2, 0), point(2, 0)],
            ),
            wire(
                "V",
                NetId(2),
                &components[2],
                &components[3],
                vec![point(0, -2), point(0, 2)],
            ),
        ];
        (
            components,
            wires,
            vec![Crossing {
                id: "X".to_string(),
                point: point(0, 0),
                nets: [NetId(1), NetId(2)],
            }],
        )
    }
    #[test]
    fn transverse_crossing_is_legal_only_without_conductive_anchor() {
        let (components, wires, crossings) = crossing_fixture();
        assert!(geometry_errors(&components, &wires, &[], &[], &crossings).is_empty());
        let hubs = vec![Junction {
            id: "J".to_string(),
            net: NetId(1),
            point: point(0, 0),
        }];
        assert!(
            geometry_errors(&components, &wires, &hubs, &[], &crossings)
                .iter()
                .any(|error| error.contains("net contact"))
        );
        let mut components = components;
        components.push(pin("E", NetId(1), point(0, 0)));
        assert!(!geometry_errors(&components, &wires, &[], &[], &crossings).is_empty());
    }
    #[test]
    fn declared_crossing_cannot_excuse_endpoint_or_parallel_contact() {
        let (mut components, mut wires, crossings) = crossing_fixture();
        components[2].pins[0].point = point(0, 0);
        wires[1].points[0] = point(0, 0);
        assert!(!geometry_errors(&components, &wires, &[], &[], &crossings).is_empty());
        components[2].pins[0].point = point(-1, 0);
        components[3].pins[0].point = point(1, 0);
        wires[1].points = vec![point(-1, 0), point(1, 0)];
        assert!(!geometry_errors(&components, &wires, &[], &[], &crossings).is_empty());
    }
    #[test]
    fn matching_wire_tags_do_not_join_disconnected_islands() {
        let components = vec![
            pin("A", NetId(1), point(0, 0)),
            pin("B", NetId(1), point(2, 0)),
            pin("C", NetId(1), point(10, 0)),
            pin("D", NetId(1), point(12, 0)),
        ];
        let wires = vec![
            wire(
                "one",
                NetId(1),
                &components[0],
                &components[1],
                vec![point(0, 0), point(2, 0)],
            ),
            wire(
                "two",
                NetId(1),
                &components[2],
                &components[3],
                vec![point(10, 0), point(12, 0)],
            ),
        ];
        assert!(
            geometry_errors(&components, &wires, &[], &[], &[])
                .iter()
                .any(|error| error.contains("disconnected"))
        );
        let labels: Vec<_> = [0, 2]
            .into_iter()
            .map(|index| NetLabel {
                id: format!("L{index}"),
                net: NetId(1),
                text: "BUS".to_string(),
                kind: crate::schematic::NetKind::Signal,
                point: components[index].pins[0].point,
                side: PinSide::Right,
                attached_to: PinRef {
                    component: components[index].id.clone(),
                    pin: "p".to_string(),
                },
            })
            .collect();
        assert!(geometry_errors(&components, &wires, &[], &labels, &[]).is_empty());
        let mut mismatched = labels;
        mismatched[1].text = "OTHER_BUS".to_string();
        assert!(
            geometry_errors(&components, &wires, &[], &mismatched, &[])
                .iter()
                .any(|error| error.contains("disconnected"))
        );
    }
    #[test]
    fn missing_links_wrong_endpoints_and_diagonals_are_rejected() {
        let (components, mut wires, crossings) = crossing_fixture();
        wires[0].points[1] = point(2, 1);
        let errors = geometry_errors(&components, &wires, &[], &[], &crossings);
        assert!(errors.iter().any(|error| error.contains("endpoint")));
        assert!(errors.iter().any(|error| error.contains("non-orthogonal")));
        let components = vec![
            pin("A", NetId(1), point(0, 0)),
            pin("B", NetId(1), point(1, 0)),
        ];
        let wires = vec![wire(
            "missing",
            NetId(1),
            &components[0],
            &components[1],
            vec![point(0, 0)],
        )];
        assert!(!geometry_errors(&components, &wires, &[], &[], &[]).is_empty());
    }
    #[test]
    fn intentional_same_net_intersections_remain_connected() {
        let (mut components, mut wires, _) = crossing_fixture();
        for component in &mut components {
            component.pins[0].net = Some(NetId(1));
        }
        wires[1].net = NetId(1);
        assert!(!geometry_errors(&components, &wires, &[], &[], &[]).is_empty());
        let hubs = vec![Junction {
            id: "J".to_string(),
            net: NetId(1),
            point: point(0, 0),
        }];
        assert!(geometry_errors(&components, &wires, &hubs, &[], &[]).is_empty());
    }
}
