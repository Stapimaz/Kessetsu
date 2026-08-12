use crate::component::CatalogSymbol;
use crate::schematic::{NetKind, Point, Schematic, SchematicComponent};

const SCALE: i32 = 32;

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
            let arrow = if component.variant.as_deref() == Some("pnp") {
                format!(
                    "M {} {} L {} {} L {} {}",
                    local(1.15),
                    local(1.3),
                    local(1.48),
                    local(1.58),
                    local(1.48),
                    local(1.18)
                )
            } else {
                format!(
                    "M {} {} L {} {} L {} {}",
                    local(1.48),
                    local(1.58),
                    local(1.15),
                    local(1.3),
                    local(1.55),
                    local(1.3)
                )
            };
            format!(
                "<path {stroke} d=\"M 0 {one} H {base} M {base} {top} V {bottom} M {base} {upper} L {two} 0 M {base} {lower} L {two} {two}\"/><path d=\"{arrow}\" fill=\"none\" stroke=\"#172033\" stroke-width=\"2\"/>",
                one = local(1.0),
                base = local(0.72),
                top = local(0.45),
                bottom = local(1.55),
                upper = local(0.72),
                lower = local(1.28),
                two = local(2.0)
            )
        }
        CatalogSymbol::Mosfet => format!(
            "<path {stroke} d=\"M 0 {one} H {gate} M {gate} {top} V {bottom} M {channel} 0 V {a} M {channel} {b} V {c} M {channel} {d} V {two} M {channel} {half} H {two}\"/>",
            one = local(1.0),
            gate = local(0.65),
            top = local(0.4),
            bottom = local(1.6),
            channel = local(0.95),
            a = local(0.65),
            b = local(0.8),
            c = local(1.2),
            d = local(1.35),
            two = local(2.0),
            half = local(1.0)
        ),
        CatalogSymbol::OpAmp => format!(
            "<path {stroke} d=\"M 0 0 H {lead} M 0 {two} H {lead} M {one} -{one} V -{tip} M {one} {three} V {bottom} M {right} {one} H {three}\"/><path d=\"M {lead} -{half} L {lead} {mid} L {right} {one} Z\" fill=\"#ffffff\" stroke=\"#172033\" stroke-width=\"2\"/><text x=\"{signx}\" y=\"{plusy}\" font-size=\"14\" fill=\"#172033\">−</text><text x=\"{signx}\" y=\"{minusy}\" font-size=\"14\" fill=\"#172033\">+</text>",
            lead = local(0.4),
            one = local(1.0),
            tip = local(0.5),
            three = local(3.0),
            bottom = local(2.5),
            right = local(2.6),
            two = local(2.0),
            half = local(0.5),
            mid = local(2.5),
            signx = local(0.58),
            plusy = local(0.2),
            minusy = local(2.2)
        ),
        CatalogSymbol::ModulePort => format!(
            "<circle cx=\"{}\" cy=\"0\" r=\"{}\" fill=\"#ffffff\" stroke=\"#6d28d9\" stroke-width=\"2\"/>",
            local(1.0),
            local(0.3)
        ),
    }
}

fn component_markup(component: &SchematicComponent) -> String {
    let (x, y) = px(component.origin);
    let reference_x = (component.bounds.min.x + component.bounds.max.x) * SCALE / 2;
    let reference_y = component.bounds.min.y * SCALE - 16;
    let value_y = component.bounds.max.y * SCALE + 20;
    let value = component
        .value
        .as_deref()
        .or(component.model.as_deref())
        .unwrap_or("");
    format!(
        "<g class=\"component\" data-component=\"{id}\"><g transform=\"translate({x} {y}) rotate({rotation})\">{symbol}</g><text class=\"reference\" x=\"{reference_x}\" y=\"{reference_y}\" text-anchor=\"middle\">{reference}</text>{value_markup}</g>",
        id = escape_xml(&component.id),
        rotation = component.orientation.degrees(),
        symbol = symbol_markup(component),
        reference = escape_xml(&component.reference),
        value_markup = if value.is_empty() {
            String::new()
        } else {
            format!(
                "<text class=\"value\" x=\"{reference_x}\" y=\"{value_y}\" text-anchor=\"middle\">{}</text>",
                escape_xml(value)
            )
        }
    )
}

pub fn render_svg(schematic: &Schematic) -> String {
    let min_x = schematic.bounds.min.x * SCALE;
    let min_y = schematic.bounds.min.y * SCALE;
    let width = (schematic.bounds.max.x - schematic.bounds.min.x) * SCALE;
    let height = (schematic.bounds.max.y - schematic.bounds.min.y) * SCALE;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-label=\"NetLang schematic\" data-schema=\"{}\" viewBox=\"{min_x} {min_y} {width} {height}\"><style>.sheet{{fill:#fff}}.wire{{fill:none;stroke:#2563eb;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}}.junction{{fill:#2563eb}}.reference{{font:600 14px ui-monospace,SFMono-Regular,Consolas,monospace;fill:#111827}}.value{{font:12px ui-monospace,SFMono-Regular,Consolas,monospace;fill:#334155}}.net-label{{font:600 15px ui-monospace,SFMono-Regular,Consolas,monospace;fill:#1e3a8a}}</style><rect class=\"sheet\" x=\"{min_x}\" y=\"{min_y}\" width=\"{width}\" height=\"{height}\"/>",
        escape_xml(&schematic.schema_version)
    );

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
    for label in &schematic.labels {
        let (x, y) = px(label.point);
        match label.kind {
            NetKind::Ground => svg.push_str(&format!(
                "<g class=\"net-label ground\" data-net=\"{}\"><path d=\"M {x} {y} v 7 m -10 0 h 20 m -7 5 h 14 m -4 5 h 8\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/></g>",
                label.net
            )),
            NetKind::Supply => svg.push_str(&format!(
                "<g class=\"net-label supply\" data-net=\"{}\"><path d=\"M {x} {y} v -9 m -5 4 l 5 -5 5 5\" stroke=\"#172033\" stroke-width=\"2\" fill=\"none\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text></g>",
                label.net, x, y - 16, escape_xml(&label.text)
            )),
            NetKind::Signal => svg.push_str(&format!(
                "<g class=\"net-label signal\" data-net=\"{}\"><path d=\"M {x} {y} h 10 l 5 -5 h 34 v 10 h -34 Z\" fill=\"#eff6ff\" stroke=\"#2563eb\" stroke-width=\"1.5\"/><text x=\"{}\" y=\"{}\">{}</text></g>",
                label.net, x + 18, y + 5, escape_xml(&label.text)
            )),
        }
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
}
