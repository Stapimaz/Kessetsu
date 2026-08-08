use crate::layout::LayoutResult;

/// Generates a KiCad 6.0/7.0/8.0 compatible schematic file (.kicad_sch)
/// This is a simplified S-Expression generator that allows KiCad to open our NetLang schematics!
pub fn generate_kicad_sch(layout: &LayoutResult) -> String {
    let mut out = String::new();
    let uuid = "00000000-0000-0000-0000-000000000000"; // Mock UUID for simplicity

    // Header
    out.push_str("(kicad_sch (version 20211123) (generator netlang)\n");
    out.push_str("  (uuid \"00000000-0000-0000-0000-000000000000\")\n");
    out.push_str("  (paper \"A4\")\n");

    let scale = 2.54; // Convert grid coordinates to millimeters (standard 100mil grid)
    let offset_x = 50.0;
    let offset_y = 50.0;

    // Components
    for (name, comp) in &layout.components {
        let cx = (comp.x as f64) * scale + offset_x;
        let cy = (comp.y as f64) * scale + offset_y;

        let lib_id = match comp.comp_type.as_str() {
            "Resistor" => "Device:R",
            "Capacitor" => "Device:C",
            "Inductor" => "Device:L",
            "Diode" => "Device:D",
            "Source" => "Simulation_SPICE:VDC",
            "CurrentSource" => "Simulation_SPICE:IDC",
            "Transistor" => "Device:Q_NPN_CBE", // Simplified
            "Mosfet" => "Device:Q_NMOS_DGS",
            "OpAmp" => "Amplifier_Operational:LM741",
            _ => "Device:R", // Fallback
        };

        let rot = match comp.rotation {
            0 => 0,
            1 => 90,
            2 => 180,
            3 => 270,
            _ => 0,
        };

        out.push_str(&format!(
            "  (symbol (lib_id \"{}\") (at {} {} {}) (unit 1)\n",
            lib_id, cx, cy, rot
        ));
        out.push_str(&format!("    (uuid \"{}\")\n", uuid));
        out.push_str(&format!(
            "    (property \"Reference\" \"{}\" (at {} {} 0)\n",
            name,
            cx,
            cy - 2.54
        ));
        out.push_str("      (effects (font (size 1.27 1.27)))\n    )\n");
        out.push_str("  )\n");
    }

    // Wires
    for wire in &layout.wires {
        if wire.points.len() < 2 {
            continue;
        }
        for i in 0..(wire.points.len() - 1) {
            let p1 = wire.points[i];
            let p2 = wire.points[i + 1];
            let x1 = (p1.0 as f64) * scale + offset_x;
            let y1 = (p1.1 as f64) * scale + offset_y;
            let x2 = (p2.0 as f64) * scale + offset_x;
            let y2 = (p2.1 as f64) * scale + offset_y;

            out.push_str(&format!(
                "  (wire (pts (xy {} {}) (xy {} {}))\n",
                x1, y1, x2, y2
            ));
            out.push_str("    (stroke (width 0) (type default))\n");
            out.push_str(&format!("    (uuid \"{}\")\n", uuid));
            out.push_str("  )\n");
        }
    }

    // Footer
    out.push_str(")\n");
    out
}
