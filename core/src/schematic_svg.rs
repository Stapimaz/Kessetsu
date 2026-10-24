use crate::component::{CatalogSymbol, PinSide};
use crate::schematic::{NetKind, Point, Schematic, SchematicComponent, SchematicText, TextAnchor};

const SCALE: i32 = 32;
const TEXT_SUBGRID: i32 = 8;

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn px(point: Point) -> (i32, i32) {
    (point.x * SCALE, point.y * SCALE)
}

fn local(value: f32) -> f32 {
    value * SCALE as f32
}

fn symbol_width(symbol: CatalogSymbol) -> f32 {
    match symbol {
        CatalogSymbol::Resistor
        | CatalogSymbol::Capacitor
        | CatalogSymbol::Inductor
        | CatalogSymbol::Diode
        | CatalogSymbol::VoltageSource
        | CatalogSymbol::CurrentSource
        | CatalogSymbol::ModulePort
        | CatalogSymbol::ExternalTwoTerminal
        | CatalogSymbol::Vcvs
        | CatalogSymbol::Vccs
        | CatalogSymbol::Ccvs
        | CatalogSymbol::Cccs
        | CatalogSymbol::Switch => 2.0,
        CatalogSymbol::Bjt | CatalogSymbol::Mosfet | CatalogSymbol::OpAmp => 3.0,
    }
}

fn bjt_emitter_arrow(pnp: bool) -> [(f32, f32); 3] {
    if pnp {
        // The centroid lies on the collector/emitter branch. The tip points
        // toward the base, as required for a PNP symbol.
        [(1.20, 0.45), (1.47, 0.14), (1.62, 0.39)]
    } else {
        // The centroid lies on the lower branch and the tip points away from
        // the base, as required for an NPN symbol.
        [(1.64, 1.80), (1.31, 1.77), (1.45, 1.53)]
    }
}

fn symbol_markup(component: &SchematicComponent) -> String {
    let stroke = "stroke=\"#172033\" stroke-width=\"2\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\"";
    match component.symbol {
        CatalogSymbol::Resistor => format!(
            "<path {stroke} d=\"M 0 0 H {} L {} {} L {} {} L {} {} L {} {} L {} {} L {} {} H {}\"/>",
            local(0.3),
            local(0.5),
            local(-0.28),
            local(0.75),
            local(0.28),
            local(1.0),
            local(-0.28),
            local(1.25),
            local(0.28),
            local(1.5),
            local(-0.28),
            local(1.7),
            0,
            local(2.0)
        ),
        CatalogSymbol::Capacitor => format!(
            "<path {stroke} d=\"M 0 0 H {a} M {a} {top} V {bottom} M {b} {top} V {bottom} M {b} 0 H {end}\"/>",
            a = local(0.9),
            b = local(1.1),
            top = local(-0.45),
            bottom = local(0.45),
            end = local(2.0)
        ),
        CatalogSymbol::Inductor => format!(
            "<path {stroke} d=\"M 0 0 C {a} {up} {b} {up} {c} 0 C {d} {up} {e} {up} {f} 0 C {g} {up} {h} {up} {i} 0 C {j} {up} {k} {up} {end} 0\"/>",
            a = local(0.12),
            b = local(0.38),
            c = local(0.5),
            d = local(0.62),
            e = local(0.88),
            f = local(1.0),
            g = local(1.12),
            h = local(1.38),
            i = local(1.5),
            j = local(1.62),
            k = local(1.88),
            end = local(2.0),
            up = local(-0.42)
        ),
        CatalogSymbol::Diode => format!(
            "<path {stroke} d=\"M 0 0 H {a} M {b} {top} V {bottom} M {b} 0 H {end}\"/><path d=\"M {a} {top} L {b} 0 L {a} {bottom} Z\" fill=\"#ffffff\" stroke=\"#172033\" stroke-width=\"2\"/>",
            a = local(0.72),
            b = local(1.28),
            top = local(-0.42),
            bottom = local(0.42),
            end = local(2.0)
        ),
        CatalogSymbol::VoltageSource => format!(
            "<path {stroke} d=\"M 0 0 H {left} M {right} 0 H {end}\"/><circle {stroke} cx=\"{center}\" cy=\"0\" r=\"{radius}\"/><path {stroke} d=\"M {center} {p1} V {p2} M {m1} {minus} H {m2}\"/>",
            left = local(0.45),
            right = local(1.55),
            end = local(2.0),
            center = local(1.0),
            radius = local(0.55),
            p1 = local(-0.28),
            p2 = local(-0.02),
            m1 = local(0.86),
            m2 = local(1.14),
            minus = local(0.24)
        ),
        CatalogSymbol::CurrentSource => format!(
            "<path {stroke} d=\"M 0 0 H {left} M {right} 0 H {end}\"/><circle {stroke} cx=\"{center}\" cy=\"0\" r=\"{radius}\"/><path {stroke} d=\"M {a} 0 H {b} M {b} 0 L {tip} {up} M {b} 0 L {tip} {down}\"/>",
            left = local(0.45),
            right = local(1.55),
            end = local(2.0),
            center = local(1.0),
            radius = local(0.55),
            a = local(0.72),
            b = local(1.28),
            tip = local(1.1),
            up = local(-0.16),
            down = local(0.16)
        ),
        CatalogSymbol::Bjt => {
            let pnp = component.variant.as_deref() == Some("pnp");
            let (branches, arrow_points) = if pnp {
                (
                    format!(
                        "M {base} {upper} L {two} 0 M {base} {lower} L {two} {two}",
                        base = local(0.72),
                        upper = local(0.72),
                        lower = local(1.28),
                        two = local(2.0)
                    ),
                    bjt_emitter_arrow(true),
                )
            } else {
                (
                    format!(
                        "M {base} {upper} L {two} 0 M {base} {lower} L {two} {two}",
                        base = local(0.72),
                        upper = local(0.72),
                        lower = local(1.28),
                        two = local(2.0)
                    ),
                    bjt_emitter_arrow(false),
                )
            };
            let arrow = format!(
                "M {} {} L {} {} L {} {} Z",
                local(arrow_points[0].0),
                local(arrow_points[0].1),
                local(arrow_points[1].0),
                local(arrow_points[1].1),
                local(arrow_points[2].0),
                local(arrow_points[2].1)
            );
            format!(
                "<path {stroke} d=\"M 0 {one} H {base} M {base} {top} V {bottom} {branches}\"/><path class=\"emitter-arrow\" d=\"{arrow}\" fill=\"#172033\" stroke=\"none\"/>",
                one = local(1.0),
                base = local(0.72),
                top = local(0.45),
                bottom = local(1.55),
            )
        }
        CatalogSymbol::Mosfet => format!(
            "<path {stroke} d=\"M 0 {one} H {gate} M {gate} {top} V {bottom} M {channel} {upper} V {lower} M {channel} {upper} L {two} 0 M {channel} {lower} L {two} {two}\"/>{gate_mark}",
            one = local(1.0),
            gate = local(0.65),
            top = local(0.35),
            bottom = local(1.65),
            channel = local(1.05),
            upper = local(0.48),
            lower = local(1.52),
            two = local(2.0),
            gate_mark = if component.variant.as_deref() == Some("pmos") {
                format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#fff\" stroke=\"#172033\" stroke-width=\"2\"/>",
                    local(0.82),
                    local(1.0),
                    local(0.12)
                )
            } else {
                String::new()
            }
        ),
        CatalogSymbol::OpAmp => format!(
            "<path {stroke} d=\"M 0 0 H {lead} M 0 {two} H {lead} M {one} -{one} V -{tip} M {one} {three} V {bottom} M {right} {one} H {three}\"/><path d=\"M {lead} -{half} L {lead} {mid} L {right} {one} Z\" fill=\"#ffffff\" stroke=\"#172033\" stroke-width=\"2\"/><path d=\"M {sign_left} {plus_y} H {sign_right} M {sign_mid} {plus_top} V {plus_bottom} M {sign_left} {minus_y} H {sign_right}\" fill=\"none\" stroke=\"#172033\" stroke-width=\"1.25\"/>",
            lead = local(0.4),
            one = local(1.0),
            tip = local(0.5),
            three = local(3.0),
            bottom = local(2.5),
            right = local(2.6),
            two = local(2.0),
            half = local(0.5),
            mid = local(2.5),
            sign_left = local(0.58),
            sign_right = local(0.92),
            sign_mid = local(0.75),
            plus_y = local(0.28),
            plus_top = local(0.11),
            plus_bottom = local(0.45),
            minus_y = local(1.72)
        ),
        CatalogSymbol::ExternalTwoTerminal => format!(
            "<path {stroke} d=\"M 0 0 L {} 0 M {} 0 L {} 0\"/><rect {stroke} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/><path {stroke} d=\"M {} {} L {} {} M {} {} L {} {}\"/>",
            local(0.3),
            local(1.7),
            local(2.0),
            local(0.3),
            local(-0.4),
            local(1.4),
            local(0.8),
            local(0.8),
            local(-0.2),
            local(1.2),
            local(0.2),
            local(0.8),
            local(0.2),
            local(1.2),
            local(-0.2)
        ),
        CatalogSymbol::ModulePort => format!(
            "<circle cx=\"{}\" cy=\"0\" r=\"{}\" fill=\"#ffffff\" stroke=\"#6d28d9\" stroke-width=\"2\"/>",
            local(1.0),
            local(0.3)
        ),
        CatalogSymbol::Vcvs | CatalogSymbol::Vccs | CatalogSymbol::Ccvs | CatalogSymbol::Cccs => {
            let two = local(2.0);
            let c_x = local(0.35);
            let c_top = local(0.55);
            let c_bot = local(1.45);
            let is_voltage_controlled =
                matches!(component.symbol, CatalogSymbol::Vcvs | CatalogSymbol::Vccs);
            let control_markup = if is_voltage_controlled {
                let plus_y = local(0.72);
                let minus_y = local(1.28);
                let delta = local(0.08);
                format!(
                    "<path {stroke} d=\"M 0 0 H {c_x} V {c_top} M 0 {two} H {c_x} V {c_bot} M {} {plus_y} H {} M {c_x} {} V {} M {} {minus_y} H {}\"/>",
                    c_x - delta,
                    c_x + delta,
                    plus_y - delta,
                    plus_y + delta,
                    c_x - delta,
                    c_x + delta
                )
            } else {
                let mid_y = local(1.0);
                let arr_w = local(0.10);
                let arr_h = local(0.12);
                format!(
                    "<path {stroke} d=\"M 0 0 H {c_x} V {two} H 0 M {} {} L {c_x} {mid_y} L {} {}\"/>",
                    c_x - arr_w,
                    mid_y - arr_h,
                    c_x + arr_w,
                    mid_y - arr_h
                )
            };

            let d_cx = local(1.35);
            let d_cy = local(1.0);
            let d_top = local(0.45);
            let d_bot = local(1.55);
            let d_lx = local(0.85);
            let d_rx = local(1.85);

            let diamond = format!(
                "<path {stroke} d=\"M {d_cx} 0 H {two} M {d_cx} 0 V {d_top} M {d_cx} {d_bot} V {two} H {two}\"/><path d=\"M {d_cx} {d_top} L {d_rx} {d_cy} L {d_cx} {d_bot} L {d_lx} {d_cy} Z\" fill=\"#ffffff\" stroke=\"#172033\" stroke-width=\"2\"/>"
            );

            let is_voltage_output =
                matches!(component.symbol, CatalogSymbol::Vcvs | CatalogSymbol::Ccvs);
            let inside_markup = if is_voltage_output {
                let plus_y = local(0.75);
                let minus_y = local(1.25);
                let delta = local(0.08);
                format!(
                    "<path {stroke} d=\"M {} {plus_y} H {} M {d_cx} {} V {} M {} {minus_y} H {}\"/>",
                    d_cx - delta,
                    d_cx + delta,
                    plus_y - delta,
                    plus_y + delta,
                    d_cx - delta,
                    d_cx + delta
                )
            } else {
                let top_y = local(0.72);
                let bot_y = local(1.28);
                let arr_w = local(0.10);
                let arr_h = local(0.12);
                format!(
                    "<path {stroke} d=\"M {d_cx} {top_y} V {bot_y} M {} {} L {d_cx} {bot_y} L {} {}\"/>",
                    d_cx - arr_w,
                    bot_y - arr_h,
                    d_cx + arr_w,
                    bot_y - arr_h
                )
            };

            format!("{control_markup}{diamond}{inside_markup}")
        }
        CatalogSymbol::Switch => {
            let two = local(2.0);
            let c_x = local(0.35);
            let c_top = local(0.55);
            let c_bot = local(1.45);
            let plus_y = local(0.72);
            let minus_y = local(1.28);
            let delta = local(0.08);
            let control_path = format!(
                "<path {stroke} d=\"M 0 0 H {c_x} V {c_top} M 0 {two} H {c_x} V {c_bot} M {} {plus_y} H {} M {c_x} {} V {} M {} {minus_y} H {}\"/>",
                c_x - delta,
                c_x + delta,
                plus_y - delta,
                plus_y + delta,
                c_x - delta,
                c_x + delta
            );
            let mid_y = local(1.0);
            let blade_pivot_x = local(1.45);
            let blade_pivot_y = local(1.45);
            let blade_tip_x = local(1.18);
            let blade_tip_y = local(0.62);
            let sw_top = local(0.55);
            let r_c = local(0.06);

            let dash_link = format!(
                "<path stroke=\"#172033\" stroke-width=\"1.5\" stroke-dasharray=\"2,2\" fill=\"none\" d=\"M {c_x} {mid_y} H {blade_tip_x}\"/>"
            );
            let contacts_and_blade = format!(
                "<path {stroke} d=\"M {two} 0 H {blade_pivot_x} V {sw_top} M {two} {two} H {blade_pivot_x} V {blade_pivot_y} M {blade_pivot_x} {blade_pivot_y} L {blade_tip_x} {blade_tip_y}\"/><circle cx=\"{blade_pivot_x}\" cy=\"{sw_top}\" r=\"{r_c}\" fill=\"#ffffff\" stroke=\"#172033\" stroke-width=\"2\"/><circle cx=\"{blade_pivot_x}\" cy=\"{blade_pivot_y}\" r=\"{r_c}\" fill=\"#ffffff\" stroke=\"#172033\" stroke-width=\"2\"/>"
            );
            format!("{control_path}{dash_link}{contacts_and_blade}")
        }
    }
}

fn component_markup(component: &SchematicComponent) -> String {
    let (x, y) = px(component.origin);
    let mirror = if component.mirrored_x {
        format!(
            " translate({} 0) scale(-1 1)",
            local(symbol_width(component.symbol))
        )
    } else {
        String::new()
    };
    format!(
        "<g class=\"component\" data-component=\"{id}\"><g transform=\"translate({x} {y}) rotate({rotation}){mirror}\">{symbol}</g></g>",
        id = escape_xml(&component.id),
        rotation = component.orientation.degrees(),
        symbol = symbol_markup(component),
    )
}

fn text_markup(text: &SchematicText) -> String {
    let (base_x, base_y) = px(text.point);
    let x = base_x + text.offset_eighths.x * SCALE / TEXT_SUBGRID;
    let y = base_y + text.offset_eighths.y * SCALE / TEXT_SUBGRID;
    let anchor = match text.anchor {
        TextAnchor::Start => "start",
        TextAnchor::Middle => "middle",
        TextAnchor::End => "end",
    };
    let class = match text.role {
        crate::schematic::TextRole::Reference => "reference",
        crate::schematic::TextRole::Value | crate::schematic::TextRole::Model => "value",
    };
    format!(
        "<text id=\"{}\" class=\"{class}\" data-component=\"{}\" x=\"{x}\" y=\"{y}\" text-anchor=\"{anchor}\">{}</text>",
        escape_xml(&text.id),
        escape_xml(&text.component),
        escape_xml(&text.text)
    )
}

fn supply_points_down(name: &str) -> bool {
    matches!(name.to_ascii_uppercase().as_str(), "VEE" | "VSS" | "-V")
}

fn label_markup(label: &crate::schematic::NetLabel) -> String {
    let (x, y) = px(label.point);
    match label.kind {
        NetKind::Ground => {
            let (anchor_x, anchor_y, rotation, stub) = match label.side {
                PinSide::Top => (x, y, 180, String::new()),
                PinSide::Bottom => (x, y, 0, String::new()),
                PinSide::Left => (
                    x - SCALE,
                    y,
                    0,
                    format!(
                        "<path d=\"M {x} {y} H {}\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/>",
                        x - SCALE
                    ),
                ),
                PinSide::Right => (
                    x + SCALE,
                    y,
                    0,
                    format!(
                        "<path d=\"M {x} {y} H {}\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/>",
                        x + SCALE
                    ),
                ),
            };
            format!(
                "<g class=\"net-label ground\" data-net=\"{}\">{stub}<g transform=\"translate({anchor_x} {anchor_y}) rotate({rotation})\"><path d=\"M 0 0 v 7 m -10 0 h 20 m -7 5 h 14 m -4 5 h 8\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/></g></g>",
                label.net,
            )
        }
        NetKind::Supply => {
            let points_down = supply_points_down(&label.text);
            let (anchor_x, anchor_y) = px(crate::schematic::supply_marker_anchor(label));
            let stub = if (x, y) == (anchor_x, anchor_y) {
                String::new()
            } else {
                format!(
                    "<path d=\"M {x} {y} L {anchor_x} {anchor_y}\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/>"
                )
            };
            let rotation = if points_down { 180 } else { 0 };
            let text_y = if points_down {
                anchor_y + 28
            } else {
                anchor_y - 16
            };
            format!(
                "<g class=\"net-label supply\" data-net=\"{}\">{stub}<g transform=\"translate({anchor_x} {anchor_y}) rotate({rotation})\"><path d=\"M 0 0 v -9 m -5 4 l 5 -5 5 5\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/></g><text x=\"{anchor_x}\" y=\"{text_y}\" text-anchor=\"middle\">{}</text></g>",
                label.net,
                escape_xml(&label.text)
            )
        }
        NetKind::Signal => {
            let tag = |tag_x: i32, tag_y: i32, transform: &str| {
                format!(
                    "<g transform=\"translate({tag_x} {tag_y}) {transform}\"><path d=\"M 0 0 h 10 l 5 -5 h 34 v 10 h -34 Z\" fill=\"#eff6ff\" stroke=\"#2563eb\" stroke-width=\"1.5\"/></g>"
                )
            };
            let (shape, text_x, text_y, anchor) = match label.side {
                PinSide::Right => (tag(x, y, ""), x + 30, y + 5, "middle"),
                PinSide::Left => (tag(x, y, "scale(-1 1)"), x - 30, y + 5, "middle"),
                PinSide::Top => (
                    format!(
                        "<path d=\"M {x} {y} v -10\" stroke=\"#2563eb\" stroke-width=\"1.5\"/>{}",
                        tag(x, y - 10, "")
                    ),
                    x + 30,
                    y - 5,
                    "middle",
                ),
                PinSide::Bottom => (
                    format!(
                        "<path d=\"M {x} {y} v 10\" stroke=\"#2563eb\" stroke-width=\"1.5\"/>{}",
                        tag(x, y + 10, "")
                    ),
                    x + 30,
                    y + 15,
                    "middle",
                ),
            };
            format!(
                "<g class=\"net-label signal\" data-net=\"{}\">{shape}<text x=\"{text_x}\" y=\"{text_y}\" text-anchor=\"{anchor}\">{}</text></g>",
                label.net,
                escape_xml(&label.text)
            )
        }
    }
}

pub fn render_svg(schematic: &Schematic) -> String {
    render_svg_with_background(schematic, true)
}

pub fn render_svg_with_background(schematic: &Schematic, white_background: bool) -> String {
    let min_x = schematic.bounds.min.x * SCALE;
    let min_y = schematic.bounds.min.y * SCALE;
    let width = (schematic.bounds.max.x - schematic.bounds.min.x) * SCALE;
    let height = (schematic.bounds.max.y - schematic.bounds.min.y) * SCALE;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-label=\"Kessetsu schematic\" data-schema=\"{}\" width=\"{width}\" height=\"{height}\" viewBox=\"{min_x} {min_y} {width} {height}\"><style>.sheet{{fill:#fff}}.wire{{fill:none;stroke:#315f8a;stroke-width:1.75;stroke-linecap:round;stroke-linejoin:round}}.junction{{fill:#315f8a}}.reference{{font:600 14px 'Roboto Mono',ui-monospace,SFMono-Regular,Consolas,monospace;fill:#111827}}.value{{font:12px 'Roboto Mono',ui-monospace,SFMono-Regular,Consolas,monospace;fill:#475569}}.net-label{{font:600 14px 'Roboto Mono',ui-monospace,SFMono-Regular,Consolas,monospace;fill:#274c77}}</style>",
        escape_xml(&schematic.schema_version)
    );
    if white_background {
        svg.push_str(&format!(
            "<rect class=\"sheet\" x=\"{min_x}\" y=\"{min_y}\" width=\"{width}\" height=\"{height}\"/>"
        ));
    }

    for wire in &schematic.wires {
        let points = wire
            .points
            .iter()
            .map(|point| {
                let (x, y) = px(*point);
                format!("{x},{y}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            "<polyline class=\"wire\" data-wire=\"{}\" data-net=\"{}\" points=\"{points}\"/>",
            escape_xml(&wire.id),
            wire.net
        ));
    }
    for crossing in &schematic.crossings {
        let (x, y) = px(crossing.point);
        svg.push_str(&format!(
            "<g class=\"crossing\" data-crossing=\"{}\"><circle cx=\"{x}\" cy=\"{y}\" r=\"5\" fill=\"#fff\"/><path d=\"M {} {y} Q {x} {} {} {y}\" fill=\"none\" stroke=\"#2563eb\" stroke-width=\"2\"/></g>",
            escape_xml(&crossing.id), x - 7, y - 8, x + 7
        ));
    }
    for junction in &schematic.junctions {
        let (x, y) = px(junction.point);
        svg.push_str(&format!(
            "<circle class=\"junction\" data-junction=\"{}\" data-net=\"{}\" cx=\"{x}\" cy=\"{y}\" r=\"4\"/>",
            escape_xml(&junction.id),
            junction.net
        ));
    }
    for component in &schematic.components {
        svg.push_str(&component_markup(component));
    }
    for text in &schematic.texts {
        svg.push_str(&text_markup(text));
    }
    for label in &schematic.labels {
        svg.push_str(&label_markup(label));
    }
    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schematic::Orientation;

    #[test]
    fn xml_text_is_escaped() {
        assert_eq!(escape_xml("A<&\"'>"), "A&lt;&amp;&quot;&apos;&gt;");
    }

    #[test]
    fn orientation_degrees_are_quarter_turns() {
        assert_eq!(Orientation::Right.degrees(), 0);
        assert_eq!(Orientation::Down.degrees(), 90);
        assert_eq!(Orientation::Left.degrees(), 180);
        assert_eq!(Orientation::Up.degrees(), 270);
    }

    #[test]
    fn supply_direction_follows_rail_semantics_not_symbol_orientation() {
        assert!(!supply_points_down("VCC"));
        assert!(!supply_points_down("VDD"));
        assert!(supply_points_down("VEE"));
        assert!(supply_points_down("VSS"));
    }

    #[test]
    fn bjt_arrow_centroids_sit_on_the_emitter_branches() {
        for (pnp, start, end) in [
            (true, (0.72_f32, 0.72_f32), (2.0_f32, 0.0_f32)),
            (false, (0.72_f32, 1.28_f32), (2.0_f32, 2.0_f32)),
        ] {
            let points = bjt_emitter_arrow(pnp);
            let centroid = (
                points.iter().map(|point| point.0).sum::<f32>() / 3.0,
                points.iter().map(|point| point.1).sum::<f32>() / 3.0,
            );
            let cross = (end.0 - start.0) * (centroid.1 - start.1)
                - (end.1 - start.1) * (centroid.0 - start.0);
            assert!(
                cross.abs() < 0.02,
                "arrow centroid is off the emitter branch"
            );
        }
    }
}
