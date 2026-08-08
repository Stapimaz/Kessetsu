use crate::ir::ComponentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinDefinition {
    pub name: &'static str,
    pub x: i32,
    pub y: i32,
    pub is_signal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentDefinition {
    pub display_name: &'static str,
    pub spice_prefix: Option<&'static str>,
    pub width: i32,
    pub height: i32,
    pub pins: &'static [PinDefinition],
}

const TWO_PIN: &[PinDefinition] = &[
    PinDefinition {
        name: "p1",
        x: 0,
        y: 0,
        is_signal: false,
    },
    PinDefinition {
        name: "p2",
        x: 2,
        y: 0,
        is_signal: false,
    },
];

const SOURCE_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "plus",
        x: 0,
        y: 0,
        is_signal: false,
    },
    PinDefinition {
        name: "minus",
        x: 2,
        y: 0,
        is_signal: false,
    },
];

const BJT_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "c",
        x: 2,
        y: 0,
        is_signal: false,
    },
    PinDefinition {
        name: "b",
        x: 0,
        y: 1,
        is_signal: true,
    },
    PinDefinition {
        name: "e",
        x: 2,
        y: 2,
        is_signal: false,
    },
];

const MOSFET_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "d",
        x: 2,
        y: 0,
        is_signal: false,
    },
    PinDefinition {
        name: "g",
        x: 0,
        y: 1,
        is_signal: true,
    },
    PinDefinition {
        name: "s",
        x: 2,
        y: 2,
        is_signal: false,
    },
];

const OPAMP_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "in_p",
        x: 0,
        y: 2,
        is_signal: true,
    },
    PinDefinition {
        name: "in_n",
        x: 0,
        y: 0,
        is_signal: true,
    },
    PinDefinition {
        name: "vcc",
        x: 1,
        y: -1,
        is_signal: false,
    },
    PinDefinition {
        name: "vee",
        x: 1,
        y: 3,
        is_signal: false,
    },
    PinDefinition {
        name: "out",
        x: 3,
        y: 1,
        is_signal: false,
    },
];

const MODULE_PORT_PINS: &[PinDefinition] = &[];

pub fn component_definition(kind: &ComponentKind) -> ComponentDefinition {
    match kind {
        ComponentKind::Resistor => definition("Resistor", "R", 2, 1, TWO_PIN),
        ComponentKind::Capacitor => definition("Capacitor", "C", 2, 1, TWO_PIN),
        ComponentKind::Inductor => definition("Inductor", "L", 2, 1, TWO_PIN),
        ComponentKind::Diode => definition("Diode", "D", 2, 1, TWO_PIN),
        ComponentKind::BJT(_) => definition("Transistor", "Q", 3, 3, BJT_PINS),
        ComponentKind::MOSFET(_) => definition("Mosfet", "M", 3, 3, MOSFET_PINS),
        ComponentKind::OpAmp => definition("OpAmp", "X", 3, 3, OPAMP_PINS),
        ComponentKind::VoltageSource => definition("Source", "V", 2, 1, SOURCE_PINS),
        ComponentKind::CurrentSource => definition("CurrentSource", "I", 2, 1, SOURCE_PINS),
        ComponentKind::ModulePort => ComponentDefinition {
            display_name: "ModulePort",
            spice_prefix: None,
            width: 2,
            height: 2,
            pins: MODULE_PORT_PINS,
        },
    }
}

fn definition(
    display_name: &'static str,
    spice_prefix: &'static str,
    width: i32,
    height: i32,
    pins: &'static [PinDefinition],
) -> ComponentDefinition {
    ComponentDefinition {
        display_name,
        spice_prefix: Some(spice_prefix),
        width,
        height,
        pins,
    }
}

pub fn through_pin(kind: &ComponentKind, entry_pin: &str) -> Option<&'static str> {
    match kind {
        ComponentKind::Resistor
        | ComponentKind::Capacitor
        | ComponentKind::Inductor
        | ComponentKind::Diode => match entry_pin {
            "p1" => Some("p2"),
            "p2" => Some("p1"),
            _ => None,
        },
        ComponentKind::VoltageSource | ComponentKind::CurrentSource => match entry_pin {
            "plus" => Some("minus"),
            "minus" => Some("plus"),
            _ => None,
        },
        ComponentKind::BJT(_) => match entry_pin {
            "c" => Some("e"),
            "e" => Some("c"),
            "b" => Some("e"),
            _ => None,
        },
        ComponentKind::MOSFET(_) => match entry_pin {
            "d" => Some("s"),
            "s" => Some("d"),
            "g" => Some("s"),
            _ => None,
        },
        ComponentKind::OpAmp => match entry_pin {
            "vcc" => Some("vee"),
            "vee" => Some("vcc"),
            "in_p" | "in_n" | "out" => Some("out"),
            _ => None,
        },
        ComponentKind::ModulePort => None,
    }
}

pub fn is_valid_pin(kind: &ComponentKind, pin: &str) -> bool {
    kind == &ComponentKind::ModulePort
        || component_definition(kind)
            .pins
            .iter()
            .any(|definition| definition.name == pin)
}

pub fn is_signal_pin(kind: &ComponentKind, pin: &str) -> bool {
    component_definition(kind)
        .pins
        .iter()
        .any(|definition| definition.name == pin && definition.is_signal)
}
