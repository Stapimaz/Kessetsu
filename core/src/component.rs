use crate::ir::{BJTPolarity, ComponentKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSymbol {
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    Bjt,
    Mosfet,
    OpAmp,
    VoltageSource,
    CurrentSource,
    ModulePort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinFlow {
    Passive,
    Input,
    Output,
    Power,
    Reference,
    Conduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinDefinition {
    pub name: &'static str,
    pub x: i32,
    pub y: i32,
    pub is_signal: bool,
    pub flow: PinFlow,
    pub side: PinSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentDefinition {
    pub symbol: CatalogSymbol,
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
        flow: PinFlow::Passive,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "p2",
        x: 2,
        y: 0,
        is_signal: false,
        flow: PinFlow::Passive,
        side: PinSide::Right,
    },
];

const SOURCE_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "plus",
        x: 0,
        y: 0,
        is_signal: false,
        flow: PinFlow::Output,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "minus",
        x: 2,
        y: 0,
        is_signal: false,
        flow: PinFlow::Reference,
        side: PinSide::Right,
    },
];

const BJT_NPN_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "c",
        x: 2,
        y: 0,
        is_signal: false,
        flow: PinFlow::Conduction,
        side: PinSide::Top,
    },
    PinDefinition {
        name: "b",
        x: 0,
        y: 1,
        is_signal: true,
        flow: PinFlow::Input,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "e",
        x: 2,
        y: 2,
        is_signal: false,
        flow: PinFlow::Conduction,
        side: PinSide::Bottom,
    },
];

const BJT_PNP_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "c",
        x: 2,
        y: 2,
        is_signal: false,
        flow: PinFlow::Conduction,
        side: PinSide::Bottom,
    },
    PinDefinition {
        name: "b",
        x: 0,
        y: 1,
        is_signal: true,
        flow: PinFlow::Input,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "e",
        x: 2,
        y: 0,
        is_signal: false,
        flow: PinFlow::Conduction,
        side: PinSide::Top,
    },
];

const MOSFET_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "d",
        x: 2,
        y: 0,
        is_signal: false,
        flow: PinFlow::Conduction,
        side: PinSide::Top,
    },
    PinDefinition {
        name: "g",
        x: 0,
        y: 1,
        is_signal: true,
        flow: PinFlow::Input,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "s",
        x: 2,
        y: 2,
        is_signal: false,
        flow: PinFlow::Conduction,
        side: PinSide::Bottom,
    },
];

const OPAMP_PINS: &[PinDefinition] = &[
    PinDefinition {
        name: "in_p",
        x: 0,
        y: 0,
        is_signal: true,
        flow: PinFlow::Input,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "in_n",
        x: 0,
        y: 2,
        is_signal: true,
        flow: PinFlow::Input,
        side: PinSide::Left,
    },
    PinDefinition {
        name: "vcc",
        x: 1,
        y: -1,
        is_signal: false,
        flow: PinFlow::Power,
        side: PinSide::Top,
    },
    PinDefinition {
        name: "vee",
        x: 1,
        y: 3,
        is_signal: false,
        flow: PinFlow::Power,
        side: PinSide::Bottom,
    },
    PinDefinition {
        name: "out",
        x: 3,
        y: 1,
        is_signal: false,
        flow: PinFlow::Output,
        side: PinSide::Right,
    },
];

const MODULE_PORT_PINS: &[PinDefinition] = &[];

pub fn component_definition(kind: &ComponentKind) -> ComponentDefinition {
    match kind {
        ComponentKind::Resistor => {
            definition(CatalogSymbol::Resistor, "Resistor", "R", 2, 1, TWO_PIN)
        }
        ComponentKind::Capacitor => {
            definition(CatalogSymbol::Capacitor, "Capacitor", "C", 2, 1, TWO_PIN)
        }
        ComponentKind::Inductor => {
            definition(CatalogSymbol::Inductor, "Inductor", "L", 2, 1, TWO_PIN)
        }
        ComponentKind::Diode => definition(CatalogSymbol::Diode, "Diode", "D", 2, 1, TWO_PIN),
        ComponentKind::BJT(BJTPolarity::NPN) => {
            definition(CatalogSymbol::Bjt, "Transistor", "Q", 3, 3, BJT_NPN_PINS)
        }
        ComponentKind::BJT(BJTPolarity::PNP) => {
            definition(CatalogSymbol::Bjt, "Transistor", "Q", 3, 3, BJT_PNP_PINS)
        }
        ComponentKind::MOSFET(_) => {
            definition(CatalogSymbol::Mosfet, "Mosfet", "M", 3, 3, MOSFET_PINS)
        }
        ComponentKind::OpAmp => definition(CatalogSymbol::OpAmp, "OpAmp", "X", 3, 3, OPAMP_PINS),
        ComponentKind::VoltageSource => definition(
            CatalogSymbol::VoltageSource,
            "Source",
            "V",
            2,
            1,
            SOURCE_PINS,
        ),
        ComponentKind::CurrentSource => definition(
            CatalogSymbol::CurrentSource,
            "CurrentSource",
            "I",
            2,
            1,
            SOURCE_PINS,
        ),
        ComponentKind::ModulePort => ComponentDefinition {
            symbol: CatalogSymbol::ModulePort,
            display_name: "ModulePort",
            spice_prefix: None,
            width: 2,
            height: 2,
            pins: MODULE_PORT_PINS,
        },
    }
}

fn definition(
    symbol: CatalogSymbol,
    display_name: &'static str,
    spice_prefix: &'static str,
    width: i32,
    height: i32,
    pins: &'static [PinDefinition],
) -> ComponentDefinition {
    ComponentDefinition {
        symbol,
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
