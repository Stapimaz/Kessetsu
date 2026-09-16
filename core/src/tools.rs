//! Shared nominal calculations and editable circuit templates. Backend artifacts
//! still cross the normal source -> Circuit IR -> backend compilation pipeline.
use crate::graph::format_spice_number;
use crate::ir::{Quantity, SIUnit, parse_quantity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::TAU;

pub const TOOL_SCHEMA_VERSION: &str = "kessetsu.tool.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferredValues {
    Exact,
    E12,
    #[default]
    E24,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolRequest {
    Divider {
        input_voltage: String,
        target_voltage: String,
        lower_resistance: String,
        #[serde(default)]
        load_resistance: Option<String>,
        #[serde(default)]
        preferred_values: PreferredValues,
    },
    RcLowpass {
        cutoff: String,
        resistance: String,
        #[serde(default)]
        preferred_values: PreferredValues,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub schema_version: String,
    pub tool: String,
    pub name: String,
    pub method: String,
    pub preferred_values: PreferredValues,
    pub inputs: BTreeMap<String, Quantity>,
    pub ideal_components: BTreeMap<String, Quantity>,
    pub components: BTreeMap<String, Quantity>,
    pub results: BTreeMap<String, Quantity>,
    pub equations: Vec<String>,
    pub assumptions: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub field: String,
    pub message: String,
}

fn error(field: &str, message: impl Into<String>) -> ToolError {
    ToolError {
        field: field.into(),
        message: message.into(),
    }
}

fn positive(input: &str, unit: SIUnit, field: &str) -> Result<f64, ToolError> {
    let value = parse_quantity(input, unit)
        .map_err(|message| error(field, message))?
        .value;
    representable(value, field)
}

fn representable(value: f64, field: &str) -> Result<f64, ToolError> {
    if !value.is_finite() || value <= 0.0 {
        Err(error(
            field,
            "Must be positive, finite and within the supported numeric range",
        ))
    } else {
        Ok(value)
    }
}

fn quantity(value: f64, unit: SIUnit) -> Quantity {
    Quantity { value, unit }
}

fn parallel(a: f64, b: f64) -> f64 {
    let (small, large) = if a < b { (a, b) } else { (b, a) };
    small / (1.0 + small / large)
}

fn select_value(value: f64, series: PreferredValues) -> Result<f64, ToolError> {
    representable(value, "component_value")?;
    let bases: &[f64] = match series {
        PreferredValues::Exact => return Ok(value),
        PreferredValues::E12 => &[1.0, 1.2, 1.5, 1.8, 2.2, 2.7, 3.3, 3.9, 4.7, 5.6, 6.8, 8.2],
        PreferredValues::E24 => &[
            1.0, 1.1, 1.2, 1.3, 1.5, 1.6, 1.8, 2.0, 2.2, 2.4, 2.7, 3.0, 3.3, 3.6, 3.9, 4.3, 4.7,
            5.1, 5.6, 6.2, 6.8, 7.5, 8.2, 9.1,
        ],
    };
    let decade = value.log10().floor() as i32;
    // Adjacent decades include the 9.1 -> 10 boundary. A decimal parse avoids
    // accumulating binary artifacts when multiplying a mantissa by a power.
    let selected = (decade - 1..=decade + 1)
        .flat_map(|exponent| bases.iter().map(move |base| format!("{base}e{exponent}")))
        .filter_map(|number| number.parse::<f64>().ok())
        .filter(|candidate| candidate.is_finite() && *candidate > 0.0)
        .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
        .ok_or_else(|| error("component_value", "No representable preferred value"))?;
    Ok(selected)
}

pub fn calculate_tool(request: ToolRequest) -> Result<ToolResult, ToolError> {
    let report = match request {
        ToolRequest::Divider {
            input_voltage,
            target_voltage,
            lower_resistance,
            load_resistance,
            preferred_values,
        } => {
            let vin = positive(&input_voltage, SIUnit::Volt, "input_voltage")?;
            let target = positive(&target_voltage, SIUnit::Volt, "target_voltage")?;
            if target >= vin {
                return Err(error(
                    "target_voltage",
                    "Target voltage must be lower than input voltage",
                ));
            }
            let lower = positive(&lower_resistance, SIUnit::Ohm, "lower_resistance")?;
            let load = load_resistance
                .as_deref()
                .map(|input| positive(input, SIUnit::Ohm, "load_resistance"))
                .transpose()?;
            let effective = load.map_or(lower, |load| parallel(lower, load));
            let ideal_upper = representable(effective * (vin / target - 1.0), "upper_resistance")?;
            let r1 = select_value(ideal_upper, preferred_values)?;
            let r2 = select_value(lower, preferred_values)?;
            let effective = load.map_or(r2, |load| parallel(r2, load));
            let vout = representable(vin / (1.0 + r1 / effective), "output_voltage")?;
            let unloaded = representable(vin / (1.0 + r1 / r2), "unloaded_voltage")?;
            let input_current = representable((vin - vout) / r1, "input_current")?;
            let lower_current = representable(vout / r2, "lower_current")?;
            let mut inputs = BTreeMap::from([
                ("input_voltage".into(), quantity(vin, SIUnit::Volt)),
                ("target_voltage".into(), quantity(target, SIUnit::Volt)),
                ("lower_resistance".into(), quantity(lower, SIUnit::Ohm)),
            ]);
            let mut components = BTreeMap::from([
                ("R1".into(), quantity(r1, SIUnit::Ohm)),
                ("R2".into(), quantity(r2, SIUnit::Ohm)),
            ]);
            let mut results = BTreeMap::from([
                ("output_voltage".into(), quantity(vout, SIUnit::Volt)),
                ("unloaded_voltage".into(), quantity(unloaded, SIUnit::Volt)),
                (
                    "target_error".into(),
                    quantity((vout / target - 1.0) * 100.0, SIUnit::Percent),
                ),
                (
                    "loading_error".into(),
                    quantity((vout / unloaded - 1.0) * 100.0, SIUnit::Percent),
                ),
                (
                    "input_current".into(),
                    quantity(input_current, SIUnit::Ampere),
                ),
                (
                    "lower_current".into(),
                    quantity(lower_current, SIUnit::Ampere),
                ),
                (
                    "r1_power".into(),
                    quantity((vin - vout) * input_current, SIUnit::Watt),
                ),
                (
                    "r2_power".into(),
                    quantity(vout * lower_current, SIUnit::Watt),
                ),
                (
                    "output_resistance".into(),
                    quantity(parallel(r1, r2), SIUnit::Ohm),
                ),
            ]);
            let load_source = if let Some(load) = load {
                inputs.insert("load_resistance".into(), quantity(load, SIUnit::Ohm));
                components.insert("RL".into(), quantity(load, SIUnit::Ohm));
                let current = representable(vout / load, "load_current")?;
                results.insert("load_current".into(), quantity(current, SIUnit::Ampere));
                results.insert("load_power".into(), quantity(vout * current, SIUnit::Watt));
                format!(
                    "resistor RL {}\nconnect RL.p1 to OUT\nconnect RL.p2 to GND\n",
                    format_spice_number(load)
                )
            } else {
                String::new()
            };
            ToolResult {
                schema_version: TOOL_SCHEMA_VERSION.into(), tool: "divider".into(), name: "Loaded voltage divider".into(),
                method: "analytical".into(), preferred_values, inputs,
                ideal_components: BTreeMap::from([
                    ("R1".into(), quantity(ideal_upper, SIUnit::Ohm)), ("R2".into(), quantity(lower, SIUnit::Ohm)),
                ]), components, results,
                equations: vec!["Rb = R2 || RL (or R2 for an open load)".into(),
                    "R1 ideal = Rb × (Vin / Vtarget − 1)".into(), "Vout = Vin × Rb / (R1 + Rb)".into(),
                    "P = voltage drop × current".into()],
                assumptions: vec!["Ideal DC voltage source and nominal linear resistors.".into(),
                    "Load is a resistor to ground. An omitted load is open circuit.".into(),
                    "R1 and R2 are rounded independently; RL stays at the entered value.".into(),
                    "Output resistance excludes the load and uses a suppressed ideal source.".into(),
                    "Nominal dissipation is not a component wattage rating or a tolerance analysis.".into()],
                source: format!("// Kessetsu voltage divider: target {} V; nominal, ideal source.\nnet GND\nnet IN\nnet OUT\nsource VIN {}V\nresistor R1 {}\nresistor R2 {}\nconnect VIN.plus, R1.p1 to IN\nconnect R1.p2, R2.p1 to OUT\nconnect VIN.minus, R2.p2 to GND\n{}simulate op\n",
                    format_spice_number(target), format_spice_number(vin), format_spice_number(r1), format_spice_number(r2), load_source),
            }
        }
        ToolRequest::RcLowpass {
            cutoff,
            resistance,
            preferred_values,
        } => {
            let target = positive(&cutoff, SIUnit::Hertz, "cutoff")?;
            let resistance = positive(&resistance, SIUnit::Ohm, "resistance")?;
            let capacitance = representable(1.0 / TAU / resistance / target, "capacitance")?;
            let r = select_value(resistance, preferred_values)?;
            let c = select_value(capacitance, preferred_values)?;
            let tau = representable(r * c, "time_constant")?;
            let achieved = representable(1.0 / TAU / tau, "cutoff")?;
            let start = representable(achieved / 100.0, "analysis_start")?;
            let stop = representable(achieved * 100.0, "analysis_stop")?;
            ToolResult {
                schema_version: TOOL_SCHEMA_VERSION.into(),
                tool: "rc_lowpass".into(),
                name: "RC low-pass filter".into(),
                method: "analytical".into(),
                preferred_values,
                inputs: BTreeMap::from([
                    ("cutoff".into(), quantity(target, SIUnit::Hertz)),
                    ("resistance".into(), quantity(resistance, SIUnit::Ohm)),
                ]),
                ideal_components: BTreeMap::from([
                    ("R1".into(), quantity(resistance, SIUnit::Ohm)),
                    ("C1".into(), quantity(capacitance, SIUnit::Farad)),
                ]),
                components: BTreeMap::from([
                    ("R1".into(), quantity(r, SIUnit::Ohm)),
                    ("C1".into(), quantity(c, SIUnit::Farad)),
                ]),
                results: BTreeMap::from([
                    ("cutoff".into(), quantity(achieved, SIUnit::Hertz)),
                    (
                        "target_error".into(),
                        quantity((achieved / target - 1.0) * 100.0, SIUnit::Percent),
                    ),
                    ("time_constant".into(), quantity(tau, SIUnit::Second)),
                ]),
                equations: vec![
                    "C ideal = 1 / (2π × R × target cutoff)".into(),
                    "Cutoff = 1 / (2π × R × C)".into(),
                    "Time constant = R × C".into(),
                ],
                assumptions: vec![
                    "Ideal voltage source and nominal linear R/C components.".into(),
                    "High-impedance output: no load or extra source resistance.".into(),
                    "R and C are rounded independently; component tolerance is not included."
                        .into(),
                    "Cutoff is the −3.01 dB frequency relative to the DC gain.".into(),
                ],
                source: format!(
                    "// Kessetsu RC low-pass: target {} Hz; high-impedance output.\nnet GND\nnet IN\nnet OUT\nsource VIN ac(1V)\nresistor R1 {}\ncapacitor C1 {}F\nconnect VIN.plus, R1.p1 to IN\nconnect R1.p2, C1.p1 to OUT\nconnect VIN.minus, C1.p2 to GND\nsimulate ac dec 40 {}Hz {}Hz\n",
                    format_spice_number(target),
                    format_spice_number(r),
                    format_spice_number(c),
                    format_spice_number(start),
                    format_spice_number(stop)
                ),
            }
        }
    };
    for (field, quantity) in &report.results {
        if !quantity.value.is_finite()
            || (quantity.value == 0.0 && !matches!(quantity.unit, SIUnit::Percent))
        {
            return Err(error(
                field,
                "Derived result is outside the supported numeric range",
            ));
        }
    }
    Ok(report)
}
