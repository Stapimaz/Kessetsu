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
    #[serde(
        rename = "non_inverting_opamp",
        alias = "non-inverting-opamp",
        alias = "opamp_gain",
        alias = "opamp-gain"
    )]
    NonInvertingOpamp {
        target_gain: String,
        #[serde(alias = "lower_resistance", alias = "resistance")]
        input_resistance: String,
        #[serde(default)]
        input_voltage: Option<String>,
        #[serde(default)]
        supply_voltage: Option<String>,
        #[serde(default)]
        preferred_values: PreferredValues,
    },
    #[serde(rename = "zener_regulator", alias = "zener-regulator")]
    ZenerRegulator {
        input_voltage: String,
        zener_voltage: String,
        load_current: String,
        #[serde(default)]
        zener_knee_current: Option<String>,
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

fn parse_gain(input: &str, field: &str) -> Result<f64, ToolError> {
    let trimmed = input.trim();
    let normalized = trimmed
        .strip_suffix("V/V")
        .or_else(|| trimmed.strip_suffix("v/v"))
        .or_else(|| trimmed.strip_suffix("V/v"))
        .or_else(|| trimmed.strip_suffix("v/V"))
        .map(str::trim_end)
        .unwrap_or(trimmed);
    let value = parse_quantity(normalized, SIUnit::Ratio)
        .map_err(|message| error(field, message))?
        .value;
    if !value.is_finite() || value <= 1.0 {
        return Err(error(
            field,
            "Target gain must be finite and greater than 1.0 V/V for a non-inverting op-amp",
        ));
    }
    Ok(value)
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
        ToolRequest::NonInvertingOpamp {
            target_gain,
            input_resistance,
            input_voltage,
            supply_voltage,
            preferred_values,
        } => {
            let gain = parse_gain(&target_gain, "target_gain")?;
            let r1_input = positive(&input_resistance, SIUnit::Ohm, "input_resistance")?;
            let vin_str = input_voltage.as_deref().unwrap_or("0.5V");
            let vin = parse_quantity(vin_str, SIUnit::Volt)
                .map_err(|message| error("input_voltage", message))?
                .value;
            if !vin.is_finite() || vin == 0.0 {
                return Err(error(
                    "input_voltage",
                    "Input voltage must be finite and non-zero",
                ));
            }
            let supply_str = supply_voltage.as_deref().unwrap_or("15V");
            let v_supply = positive(supply_str, SIUnit::Volt, "supply_voltage")?;
            if v_supply <= 2.0 {
                return Err(error(
                    "supply_voltage",
                    "Supply voltage magnitude must be greater than 2V for KESSETSU_OPAMP_V2",
                ));
            }
            let r1 = select_value(r1_input, preferred_values)?;
            let r2_ideal = representable(r1 * (gain - 1.0), "feedback_resistance")?;
            let r2 = select_value(r2_ideal, preferred_values)?;
            let achieved_gain = representable(1.0 + r2 / r1, "achieved_gain")?;
            let target_error = (achieved_gain / gain - 1.0) * 100.0;
            let vout = vin * achieved_gain;
            if !vout.is_finite() || vout == 0.0 {
                return Err(error(
                    "output_voltage",
                    "Derived output voltage is outside the supported numeric range",
                ));
            }
            let max_swing = v_supply - 1.0;
            if vout.abs() >= max_swing {
                return Err(error(
                    "target_gain",
                    format!(
                        "Target output voltage exceeds op-amp supply rail headroom (|{} V| >= {} V with 1 V drop per rail)",
                        format_spice_number(vout),
                        format_spice_number(max_swing)
                    ),
                ));
            }
            let rail_headroom = representable(max_swing - vout.abs(), "rail_headroom")?;
            let bandwidth_estimate =
                representable(1_000_000.0 / achieved_gain, "bandwidth_estimate")?;
            let v_tol = (vout.abs() * 0.02).max(1e-3);
            let v_low = vout - v_tol;
            let v_high = vout + v_tol;
            let g_tol = (achieved_gain * 0.02).max(0.01);
            let g_low = achieved_gain - g_tol;
            let g_high = achieved_gain + g_tol;
            ToolResult {
                schema_version: TOOL_SCHEMA_VERSION.into(),
                tool: "non_inverting_opamp".into(),
                name: "Non-inverting op-amp".into(),
                method: "analytical".into(),
                preferred_values,
                inputs: BTreeMap::from([
                    ("target_gain".into(), quantity(gain, SIUnit::Ratio)),
                    ("input_resistance".into(), quantity(r1_input, SIUnit::Ohm)),
                    ("input_voltage".into(), quantity(vin, SIUnit::Volt)),
                    ("supply_voltage".into(), quantity(v_supply, SIUnit::Volt)),
                ]),
                ideal_components: BTreeMap::from([
                    ("R1".into(), quantity(r1_input, SIUnit::Ohm)),
                    ("R2".into(), quantity(r2_ideal, SIUnit::Ohm)),
                ]),
                components: BTreeMap::from([
                    ("R1".into(), quantity(r1, SIUnit::Ohm)),
                    ("R2".into(), quantity(r2, SIUnit::Ohm)),
                ]),
                results: BTreeMap::from([
                    ("achieved_gain".into(), quantity(achieved_gain, SIUnit::Ratio)),
                    ("target_error".into(), quantity(target_error, SIUnit::Percent)),
                    ("output_voltage".into(), quantity(vout, SIUnit::Volt)),
                    ("rail_headroom".into(), quantity(rail_headroom, SIUnit::Volt)),
                    (
                        "bandwidth_estimate".into(),
                        quantity(bandwidth_estimate, SIUnit::Hertz),
                    ),
                ]),
                equations: vec![
                    "R2 ideal = R1 × (target gain − 1)".into(),
                    "Achieved gain = 1 + R2 / R1".into(),
                    "Vout = Vin × achieved gain".into(),
                    "Rail headroom = (Vsupply − 1 V) − |Vout|".into(),
                    "Bandwidth estimate = 1 MHz / achieved gain".into(),
                ],
                assumptions: vec![
                    "Uses built-in KESSETSU_OPAMP_V2 macromodel (A_OL = 200k, GBW = 1 MHz, Rout = 50 Ω, 1 V rail drop).".into(),
                    "Symmetric ±Vsupply rails and zero input offset/bias currents.".into(),
                    "R1 and R2 are rounded independently; component tolerance is not included.".into(),
                    "Bandwidth estimate assumes single-pole 1 MHz gain-bandwidth product and light loading.".into(),
                ],
                source: format!(
                    "// Kessetsu non-inverting op-amp: target gain {} V/V; ±{}V rails.\nnet GND\nnet VCC\nnet VEE\nnet IN\nnet FB\nnet OUT\nsource VP {}V\nsource VN {}V\nsource VIN ac(1V, {}V)\nopamp U1 KESSETSU_OPAMP_V2\nresistor R1 {}\nresistor R2 {}\nconnect VP.minus to GND\nconnect VP.plus to VCC\nconnect VN.plus to GND\nconnect VN.minus to VEE\nconnect VIN.minus to GND\nconnect VIN.plus to IN\nconnect U1.in_p to IN\nconnect U1.in_n to FB\nconnect U1.vcc to VCC\nconnect U1.vee to VEE\nconnect U1.out to OUT\nconnect R1.p1 to FB\nconnect R1.p2 to GND\nconnect R2.p1 to OUT\nconnect R2.p2 to FB\nsimulate op\nsimulate ac dec 40 10Hz 1MHz\nassert value(V(OUT)) > {}V\nassert value(V(OUT)) < {}V\nassert gain(V(OUT),V(IN)) > {}\nassert gain(V(OUT),V(IN)) < {}\n",
                    format_spice_number(gain),
                    format_spice_number(v_supply),
                    format_spice_number(v_supply),
                    format_spice_number(v_supply),
                    format_spice_number(vin),
                    format_spice_number(r1),
                    format_spice_number(r2),
                    format_spice_number(v_low),
                    format_spice_number(v_high),
                    format_spice_number(g_low),
                    format_spice_number(g_high),
                ),
            }
        }
        ToolRequest::ZenerRegulator {
            input_voltage,
            zener_voltage,
            load_current,
            zener_knee_current,
            preferred_values,
        } => {
            const ZENER_MODELS: [(&str, f64); 3] =
                [("BZX84C3V3", 3.3), ("BZX84C5V1", 5.1), ("BZX84C12", 12.0)];
            let vin = positive(&input_voltage, SIUnit::Volt, "input_voltage")?;
            let vz_requested = positive(&zener_voltage, SIUnit::Volt, "zener_voltage")?;
            let (zener_model, vz) = ZENER_MODELS
                .into_iter()
                .min_by(|(_, a), (_, b)| {
                    (a - vz_requested)
                        .abs()
                        .total_cmp(&(b - vz_requested).abs())
                })
                .expect("Zener catalog must be non-empty");
            if (vz_requested - vz).abs() / vz > 0.05 + 1e-12 {
                return Err(error(
                    "zener_voltage",
                    "Unsupported Zener voltage; built-in Zener models support 3.3V, 5.1V, or 12V (within 5%)",
                ));
            }
            let iload_target = positive(&load_current, SIUnit::Ampere, "load_current")?;
            let iz_min = positive(
                zener_knee_current.as_deref().unwrap_or("5mA"),
                SIUnit::Ampere,
                "zener_knee_current",
            )?;
            if vin <= vz + 0.5 {
                return Err(error(
                    "input_voltage",
                    format!(
                        "Input voltage must exceed Zener voltage ({} V) by more than 0.5 V",
                        format_spice_number(vz)
                    ),
                ));
            }
            let rs_ideal =
                representable((vin - vz) / (iload_target + iz_min), "series_resistance")?;
            let rl_ideal = representable(vz / iload_target, "load_resistance")?;
            let rs = select_value(rs_ideal, preferred_values)?;
            let rl = select_value(rl_ideal, preferred_values)?;
            let series_current = representable((vin - vz) / rs, "series_current")?;
            let actual_load_current = representable(vz / rl, "load_current")?;
            let zener_current = series_current - actual_load_current;
            if !zener_current.is_finite() || zener_current <= 0.0 {
                return Err(error(
                    "zener_knee_current",
                    "Rounded series and load resistors leave non-positive Zener bias current; increase input_voltage or zener_knee_current, or use exact values",
                ));
            }
            let rs_power = representable((vin - vz) * series_current, "rs_power")?;
            let zener_power = representable(vz * zener_current, "zener_power")?;
            let load_power = representable(vz * actual_load_current, "load_power")?;
            let v_low = (vz * 0.94).min(vz - 0.15).max(0.1);
            let v_high = vz * 1.06 + zener_current * 2.5;
            ToolResult {
                schema_version: TOOL_SCHEMA_VERSION.into(),
                tool: "zener_regulator".into(),
                name: "Zener shunt regulator".into(),
                method: "analytical".into(),
                preferred_values,
                inputs: BTreeMap::from([
                    ("input_voltage".into(), quantity(vin, SIUnit::Volt)),
                    ("zener_voltage".into(), quantity(vz_requested, SIUnit::Volt)),
                    ("load_current".into(), quantity(iload_target, SIUnit::Ampere)),
                    ("zener_knee_current".into(), quantity(iz_min, SIUnit::Ampere)),
                ]),
                ideal_components: BTreeMap::from([
                    ("RS".into(), quantity(rs_ideal, SIUnit::Ohm)),
                    ("RL".into(), quantity(rl_ideal, SIUnit::Ohm)),
                ]),
                components: BTreeMap::from([
                    ("RS".into(), quantity(rs, SIUnit::Ohm)),
                    ("RL".into(), quantity(rl, SIUnit::Ohm)),
                ]),
                results: BTreeMap::from([
                    ("output_voltage".into(), quantity(vz, SIUnit::Volt)),
                    ("series_current".into(), quantity(series_current, SIUnit::Ampere)),
                    ("zener_current".into(), quantity(zener_current, SIUnit::Ampere)),
                    ("load_current".into(), quantity(actual_load_current, SIUnit::Ampere)),
                    ("rs_power".into(), quantity(rs_power, SIUnit::Watt)),
                    ("zener_power".into(), quantity(zener_power, SIUnit::Watt)),
                    ("load_power".into(), quantity(load_power, SIUnit::Watt)),
                ]),
                equations: vec![
                    "RS ideal = (Vin − Vz) / (Iload + Iz_min)".into(),
                    "RL ideal = Vz / Iload".into(),
                    "Is = (Vin − Vz) / RS, Iload = Vz / RL, Iz = Is − Iload".into(),
                    "P = voltage drop × current".into(),
                ],
                assumptions: vec![
                    "Ideal DC input source and constant nominal Zener breakdown voltage Vz.".into(),
                    "Uses built-in BZX84C3V3 (3.3 V), BZX84C5V1 (5.1 V), or BZX84C12 (12 V) diode model.".into(),
                    "RS and RL are rounded independently; verify Zener bias stays above the knee current.".into(),
                    "Nominal dissipation is not a package wattage rating or thermal analysis.".into(),
                ],
                source: format!(
                    "// Kessetsu Zener shunt regulator: target {} V ({zener_model}); nominal, ideal source.\nnet GND\nnet IN\nnet OUT\nsource VIN {}V\nresistor RS {}\ndiode DZ {zener_model}\nresistor RL {}\nconnect VIN.minus to GND\nconnect VIN.plus to IN\nconnect RS.p1 to IN\nconnect RS.p2 to OUT\nconnect DZ.p1 to GND\nconnect DZ.p2 to OUT\nconnect RL.p1 to OUT\nconnect RL.p2 to GND\nsimulate op\nassert value(V(OUT)) > {}V\nassert value(V(OUT)) < {}V\n",
                    format_spice_number(vz),
                    format_spice_number(vin),
                    format_spice_number(rs),
                    format_spice_number(rl),
                    format_spice_number(v_low),
                    format_spice_number(v_high),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{CompileOptions, compile_source};

    #[test]
    fn non_inverting_opamp_calculates_e24_and_compiles() {
        let result = calculate_tool(ToolRequest::NonInvertingOpamp {
            target_gain: "10".into(),
            input_resistance: "10k".into(),
            input_voltage: None,
            supply_voltage: None,
            preferred_values: PreferredValues::E24,
        })
        .unwrap();
        assert_eq!(result.tool, "non_inverting_opamp");
        assert_eq!(result.components["R1"].value, 10_000.0);
        assert_eq!(result.components["R2"].value, 91_000.0);
        assert!((result.results["achieved_gain"].value - 10.1).abs() < 1e-12);
        assert!((result.results["target_error"].value - 1.0).abs() < 1e-10);
        assert!((result.results["output_voltage"].value - 5.05).abs() < 1e-12);
        assert!((result.results["rail_headroom"].value - 8.95).abs() < 1e-12);
        assert!((result.results["bandwidth_estimate"].value - 1_000_000.0 / 10.1).abs() < 1e-6);
        let report = compile_source(&result.source, CompileOptions::all_outputs());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert!(result.source.contains("KESSETSU_OPAMP_V2"));
        assert!(result.source.contains("simulate ac dec 40 10Hz 1MHz"));
        assert!(result.source.contains("gain(V(OUT),V(IN))"));
    }

    #[test]
    fn non_inverting_opamp_accepts_serde_aliases_and_rejects_headroom_overflow() {
        for alias in [
            "non_inverting_opamp",
            "non-inverting-opamp",
            "opamp_gain",
            "opamp-gain",
        ] {
            let req: ToolRequest = serde_json::from_value(serde_json::json!({
                "tool": alias,
                "target_gain": "10V/V",
                "input_resistance": "10k",
                "preferred_values": "exact"
            }))
            .unwrap();
            let res = calculate_tool(req).unwrap();
            assert_eq!(res.tool, "non_inverting_opamp");
            assert_eq!(res.components["R2"].value, 90_000.0);
        }

        let err = calculate_tool(ToolRequest::NonInvertingOpamp {
            target_gain: "30".into(),
            input_resistance: "10k".into(),
            input_voltage: Some("0.5V".into()),
            supply_voltage: Some("15V".into()),
            preferred_values: PreferredValues::Exact,
        })
        .unwrap_err();
        assert!(
            err.message
                .starts_with("Target output voltage exceeds op-amp supply rail headroom")
        );

        for bad_gain in ["1", "0.5", "-2", "10Hz", "10V"] {
            assert!(
                calculate_tool(ToolRequest::NonInvertingOpamp {
                    target_gain: bad_gain.into(),
                    input_resistance: "10k".into(),
                    input_voltage: None,
                    supply_voltage: None,
                    preferred_values: PreferredValues::Exact,
                })
                .is_err()
            );
        }
        assert!(
            calculate_tool(ToolRequest::NonInvertingOpamp {
                target_gain: "2".into(),
                input_resistance: "10k".into(),
                input_voltage: Some("0.2V".into()),
                supply_voltage: Some("2V".into()),
                preferred_values: PreferredValues::Exact,
            })
            .is_err()
        );
    }

    #[test]
    fn zener_regulator_selects_builtin_models_and_compiles() {
        let req: ToolRequest = serde_json::from_value(serde_json::json!({
            "tool": "zener-regulator",
            "input_voltage": "12V",
            "zener_voltage": "5.1V",
            "load_current": "10mA"
        }))
        .unwrap();
        let res = calculate_tool(req).unwrap();
        assert_eq!(res.tool, "zener_regulator");
        assert_eq!(res.components["RS"].value, 470.0);
        assert_eq!(res.components["RL"].value, 510.0);
        assert!((res.results["output_voltage"].value - 5.1).abs() < 1e-12);
        assert!((res.results["load_current"].value - 0.01).abs() < 1e-12);
        assert!(res.source.contains("diode DZ BZX84C5V1"));
        let report = compile_source(&res.source, CompileOptions::all_outputs());
        assert!(!report.has_errors(), "{:?}", report.diagnostics);

        for (vz, expected_model) in [
            ("3.3V", "BZX84C3V3"),
            ("5.0V", "BZX84C5V1"),
            ("12V", "BZX84C12"),
        ] {
            let res = calculate_tool(ToolRequest::ZenerRegulator {
                input_voltage: "24V".into(),
                zener_voltage: vz.into(),
                load_current: "10mA".into(),
                zener_knee_current: None,
                preferred_values: PreferredValues::Exact,
            })
            .unwrap();
            assert!(res.source.contains(expected_model));
        }

        let unsupported = calculate_tool(ToolRequest::ZenerRegulator {
            input_voltage: "24V".into(),
            zener_voltage: "9V".into(),
            load_current: "10mA".into(),
            zener_knee_current: None,
            preferred_values: PreferredValues::Exact,
        })
        .unwrap_err();
        assert!(unsupported.message.contains("3.3V, 5.1V, or 12V"));

        assert!(
            calculate_tool(ToolRequest::ZenerRegulator {
                input_voltage: "5.5V".into(),
                zener_voltage: "5.1V".into(),
                load_current: "10mA".into(),
                zener_knee_current: None,
                preferred_values: PreferredValues::Exact,
            })
            .is_err()
        );
    }
}
