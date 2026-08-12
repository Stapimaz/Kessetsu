use crate::graph::NetlistGraph;
use crate::ir::{
    Assertion, CircuitIR, ComponentKind, ComponentParams, IRComponent, SIUnit, SourceValue,
    Waveform, parse_quantity,
};
use crate::simulation::{ComplexSeries, Dataset, RealSeriesDataset, SimulationResult};

pub const MEASUREMENT_SCHEMA_VERSION: &str = "netlang.measurement.v1";

pub fn evaluate_assertion_metric(
    assertion: &Assertion,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let metric = assertion.metric.to_ascii_lowercase();
    let arguments = assertion
        .signal
        .split(',')
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .collect::<Vec<_>>();
    match metric.as_str() {
        "value" | "min" | "max" | "peak" | "average" | "avg" | "rms" => {
            evaluate_reduction(&metric, &arguments, circuit, simulation)
        }
        "gain" => evaluate_gain(&arguments, circuit, simulation),
        "bandwidth" | "cutoff" => evaluate_bandwidth(&arguments, simulation),
        "frequency" => evaluate_frequency(&arguments, circuit, simulation),
        "phase" => evaluate_phase(&arguments, simulation),
        "output_power" => evaluate_output_power(&arguments, circuit, simulation),
        "efficiency" => evaluate_efficiency(&arguments, circuit, simulation),
        "thd" => evaluate_thd(&arguments, circuit, simulation),
        "clipping" => evaluate_clipping(&arguments, circuit, simulation),
        "dissipation" => evaluate_dissipation(&arguments, circuit, simulation),
        _ => Err(format!(
            "unsupported assertion metric '{}'; supported engineering metrics are value, min, max, peak, average, rms, gain, bandwidth, cutoff, frequency, phase, output_power, efficiency, thd, clipping and dissipation",
            assertion.metric
        )),
    }
}

#[derive(Debug, Clone)]
enum SignalData {
    Scalar(f64),
    Series { axis: Vec<f64>, values: Vec<f64> },
}

fn evaluate_reduction(
    metric: &str,
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 1 | 3) {
        return Err(format!(
            "metric '{metric}' expects signal or signal,start,stop"
        ));
    }
    let analysis_priority = |dataset: &&crate::simulation::AnalysisDataset| {
        if arguments.len() == 3 {
            match dataset.data {
                Dataset::Transient(_) => 0,
                _ => 3,
            }
        } else if metric == "value" {
            match dataset.data {
                Dataset::OperatingPoint { .. } => 0,
                Dataset::Transient(_) => 1,
                Dataset::DcSweep(_) => 2,
                Dataset::Ac(_) => 3,
            }
        } else {
            match dataset.data {
                Dataset::Transient(_) => 0,
                Dataset::DcSweep(_) => 1,
                Dataset::OperatingPoint { .. } => 2,
                Dataset::Ac(_) => 3,
            }
        }
    };
    let mut datasets = simulation.datasets.iter().collect::<Vec<_>>();
    datasets.sort_by_key(analysis_priority);
    for dataset in datasets {
        if matches!(dataset.data, Dataset::Ac(_))
            || (arguments.len() == 3 && !matches!(dataset.data, Dataset::Transient(_)))
        {
            continue;
        }
        match resolve_signal(&dataset.data, arguments[0], circuit) {
            Ok(Some(SignalData::Scalar(value))) => return reduce_scalar(metric, value),
            Ok(Some(SignalData::Series { axis, values })) => {
                let values = if arguments.len() == 3 {
                    windowed_values(&axis, &values, arguments[1], arguments[2])?
                } else {
                    values
                };
                return reduce_series(metric, &values);
            }
            Ok(None) => continue,
            Err(message) => return Err(message),
        }
    }
    Err(format!(
        "measurement '{}({})' was not produced by the simulation",
        metric, arguments[0]
    ))
}

fn evaluate_gain(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("gain", arguments, 2)?;
    if let Some(dataset) = simulation
        .datasets
        .iter()
        .find(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        let output = resolve_signal(&dataset.data, arguments[0], circuit)?;
        let input = resolve_signal(&dataset.data, arguments[1], circuit)?;
        if let (Some(output), Some(input)) = (output, input) {
            return signal_rms(&output).and_then(|output| {
                let input = signal_rms(&input)?;
                nonzero_divide(output, input, "gain input RMS is zero")
            });
        }
    }
    if let Some(ac) = simulation
        .datasets
        .iter()
        .find_map(|dataset| match &dataset.data {
            Dataset::Ac(ac) => Some(ac),
            _ => None,
        })
    {
        let output = complex_signal(ac, arguments[0])?;
        let input = complex_signal(ac, arguments[1])?;
        return ratio_at(output, input, 0);
    }
    for dataset in &simulation.datasets {
        match &dataset.data {
            Dataset::Ac(_) => continue,
            Dataset::DcSweep(_) | Dataset::OperatingPoint { .. } => {
                let output = resolve_signal(&dataset.data, arguments[0], circuit)?;
                let input = resolve_signal(&dataset.data, arguments[1], circuit)?;
                if let (Some(output), Some(input)) = (output, input) {
                    return signal_rms(&output).and_then(|output| {
                        let input = signal_rms(&input)?;
                        nonzero_divide(output, input, "gain input RMS is zero")
                    });
                }
            }
            Dataset::Transient(_) => continue,
        }
    }
    Err("gain requires both output and input signals in one dataset".to_string())
}

fn evaluate_bandwidth(arguments: &[&str], simulation: &SimulationResult) -> Result<f64, String> {
    require_count("bandwidth", arguments, 2)?;
    let ac = simulation
        .datasets
        .iter()
        .find_map(|dataset| match &dataset.data {
            Dataset::Ac(ac) => Some(ac),
            _ => None,
        })
        .ok_or_else(|| "bandwidth requires an AC analysis dataset".to_string())?;
    let output = complex_signal(ac, arguments[0])?;
    let input = complex_signal(ac, arguments[1])?;
    if ac.frequency_hz.len() < 2 {
        return Err("bandwidth requires at least two AC frequency points".to_string());
    }
    let gains = (0..ac.frequency_hz.len())
        .map(|index| ratio_at(output, input, index))
        .collect::<Result<Vec<_>, _>>()?;
    let reference = gains[0];
    if reference <= 0.0 {
        return Err("bandwidth reference gain must be positive".to_string());
    }
    let threshold = reference / 2.0_f64.sqrt();
    for index in 1..gains.len() {
        if gains[index] <= threshold && gains[index - 1] > threshold {
            let low_frequency = ac.frequency_hz[index - 1];
            let high_frequency = ac.frequency_hz[index];
            let low_gain = gains[index - 1];
            let high_gain = gains[index];
            let fraction = (threshold - low_gain) / (high_gain - low_gain);
            let log_frequency =
                low_frequency.ln() + fraction * (high_frequency.ln() - low_frequency.ln());
            return Ok(log_frequency.exp());
        }
    }
    Err("AC response did not cross the -3 dB bandwidth threshold".to_string())
}

fn evaluate_frequency(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("frequency", arguments, 1)?;
    let (axis, values) = transient_signal(arguments[0], circuit, simulation)?;
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let crossings = values
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let left = pair[0] - mean;
            let right = pair[1] - mean;
            (left <= 0.0 && right > 0.0).then(|| {
                let fraction = -left / (right - left);
                axis[index] + fraction * (axis[index + 1] - axis[index])
            })
        })
        .collect::<Vec<_>>();
    if crossings.len() < 2 {
        return Err("frequency requires at least two rising mean crossings".to_string());
    }
    let periods = crossings
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    let period = periods.iter().sum::<f64>() / periods.len() as f64;
    nonzero_divide(1.0, period, "measured period is zero")
}

fn evaluate_phase(arguments: &[&str], simulation: &SimulationResult) -> Result<f64, String> {
    if !matches!(arguments.len(), 2 | 3) {
        return Err("phase expects output,input or output,input,frequency".to_string());
    }
    let ac = simulation
        .datasets
        .iter()
        .find_map(|dataset| match &dataset.data {
            Dataset::Ac(ac) => Some(ac),
            _ => None,
        })
        .ok_or_else(|| "phase requires an AC analysis dataset".to_string())?;
    let output = complex_signal(ac, arguments[0])?;
    let input = complex_signal(ac, arguments[1])?;
    let index = if let Some(target) = arguments.get(2) {
        let target = parse_quantity(target, SIUnit::Hertz)
            .map_err(|error| format!("invalid phase frequency: {error}"))?
            .value;
        ac.frequency_hz
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                (*left - target).abs().total_cmp(&(*right - target).abs())
            })
            .map(|(index, _)| index)
            .ok_or_else(|| "AC frequency axis is empty".to_string())?
    } else {
        0
    };
    let output_phase = output.imaginary[index].atan2(output.real[index]);
    let input_phase = input.imaginary[index].atan2(input.real[index]);
    let mut degrees = (output_phase - input_phase).to_degrees();
    while degrees > 180.0 {
        degrees -= 360.0;
    }
    while degrees <= -180.0 {
        degrees += 360.0;
    }
    Ok(degrees)
}

fn evaluate_output_power(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("output_power", arguments, 2)?;
    let resistance = resistor_value(circuit, arguments[1])?;
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        if let Some(signal) = resolve_signal(&dataset.data, arguments[0], circuit)? {
            let voltage_rms = signal_rms(&signal)?;
            return Ok(voltage_rms * voltage_rms / resistance);
        }
    }
    Err("output_power requires a transient voltage signal".to_string())
}

fn evaluate_efficiency(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 4 | 6) {
        return Err(
            "efficiency expects output,load,supply_voltage,supply_current and optionally a second supply pair"
                .to_string(),
        );
    }
    let output_power = evaluate_output_power(&arguments[..2], circuit, simulation)?;
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        let mut supply_power = 0.0;
        let mut complete = true;
        for pair in arguments[2..].chunks_exact(2) {
            let voltage = resolve_signal(&dataset.data, pair[0], circuit)?;
            let current = resolve_signal(&dataset.data, pair[1], circuit)?;
            if let (Some(voltage), Some(current)) = (voltage, current) {
                supply_power += average_product(&voltage, &current)?.abs();
            } else {
                complete = false;
                break;
            }
        }
        if complete {
            return nonzero_divide(
                output_power * 100.0,
                supply_power,
                "efficiency supply power is zero",
            );
        }
    }
    Err("efficiency requires supply voltage and current in one dataset".to_string())
}

fn evaluate_clipping(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("clipping", arguments, 3)?;
    let lower = parse_quantity(arguments[1], SIUnit::Volt)
        .map_err(|error| format!("invalid clipping lower rail: {error}"))?
        .value;
    let upper = parse_quantity(arguments[2], SIUnit::Volt)
        .map_err(|error| format!("invalid clipping upper rail: {error}"))?
        .value;
    if upper <= lower {
        return Err("clipping requires lower rail < upper rail".to_string());
    }
    let (_, values) = transient_signal(arguments[0], circuit, simulation)?;
    if values.is_empty() {
        return Err("clipping requires transient samples".to_string());
    }
    let margin = (upper - lower).abs() * 1e-3;
    let clipped = values
        .iter()
        .filter(|value| **value <= lower + margin || **value >= upper - margin)
        .count();
    Ok(clipped as f64 / values.len() as f64 * 100.0)
}

fn evaluate_thd(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("thd", arguments, 1)?;
    let (_, values) = transient_signal(arguments[0], circuit, simulation)?;
    if values.len() < 16 {
        return Err("THD requires at least 16 transient samples".to_string());
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let centered = values.iter().map(|value| value - mean).collect::<Vec<_>>();
    let half = centered.len() / 2;
    let magnitudes = (1..half)
        .map(|bin| dft_magnitude(&centered, bin))
        .collect::<Vec<_>>();
    let (fundamental_offset, fundamental) = magnitudes
        .iter()
        .copied()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .ok_or_else(|| "THD could not identify a fundamental".to_string())?;
    if fundamental <= f64::EPSILON {
        return Err("THD fundamental magnitude is zero".to_string());
    }
    let fundamental_bin = fundamental_offset + 1;
    let harmonic_squared = (2..=5)
        .filter_map(|harmonic| {
            let bin = fundamental_bin * harmonic;
            (bin < half).then(|| dft_magnitude(&centered, bin).powi(2))
        })
        .sum::<f64>();
    Ok(harmonic_squared.sqrt() / fundamental * 100.0)
}

fn evaluate_dissipation(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("dissipation", arguments, 1)?;
    let signal = format!("P({})", arguments[0]);
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        if let Some(power) = resolve_signal(&dataset.data, &signal, circuit)? {
            return match power {
                SignalData::Scalar(value) => Ok(value.max(0.0)),
                SignalData::Series { values, .. } => Ok(values
                    .iter()
                    .map(|value| value.max(0.0))
                    .sum::<f64>()
                    / values.len() as f64),
            };
        }
    }
    Err(format!(
        "device power for '{}' was not produced",
        arguments[0]
    ))
}

fn resolve_signal(
    dataset: &Dataset,
    signal: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalData>, String> {
    let (function, target) = parse_signal(signal)?;
    match function.to_ascii_uppercase().as_str() {
        "V" => voltage_signal(dataset, target, circuit),
        "I" => current_signal(dataset, target, circuit),
        "P" => power_signal(dataset, target, circuit),
        _ => Err(format!("unsupported measurement primitive '{signal}'")),
    }
}

fn voltage_signal(
    dataset: &Dataset,
    target: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalData>, String> {
    if let Some(component) = circuit
        .components
        .iter()
        .find(|component| component.id.eq_ignore_ascii_case(target))
    {
        let (positive, negative) = device_voltage_pins(component)?;
        return component_voltage(dataset, component, circuit, positive, negative);
    }
    let key = target.to_ascii_lowercase();
    match dataset {
        Dataset::OperatingPoint { values } => {
            if let Some(value) = lookup_scalar(values, &key) {
                Ok(Some(SignalData::Scalar(value)))
            } else if matches!(key.as_str(), "gnd" | "0") {
                Ok(Some(SignalData::Scalar(0.0)))
            } else {
                Ok(None)
            }
        }
        Dataset::Transient(series) | Dataset::DcSweep(series) => {
            if matches!(key.as_str(), "gnd" | "0") {
                return Ok(Some(SignalData::Series {
                    axis: series.axis.values.clone(),
                    values: vec![0.0; series.axis.values.len()],
                }));
            }
            Ok(lookup_series(series, &key).map(|values| SignalData::Series {
                axis: series.axis.values.clone(),
                values: values.clone(),
            }))
        }
        Dataset::Ac(_) => Err(
            "raw voltage reductions over complex AC data are unsupported; use gain, bandwidth or phase"
                .to_string(),
        ),
    }
}

fn current_signal(
    dataset: &Dataset,
    target: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalData>, String> {
    let component = find_component(circuit, target)?;
    let branch_key = format!(
        "{}#branch",
        spice_instance_name(component).to_ascii_lowercase()
    );
    match component.kind {
        ComponentKind::VoltageSource | ComponentKind::Inductor => {
            direct_real_signal(dataset, &branch_key)
        }
        ComponentKind::Resistor => {
            let voltage = component_voltage(dataset, component, circuit, "p1", "p2")?;
            let resistance = passive_value(component)?;
            Ok(voltage.map(|voltage| map_signal(voltage, |value| value / resistance)))
        }
        ComponentKind::Capacitor => {
            let voltage = component_voltage(dataset, component, circuit, "p1", "p2")?;
            let capacitance = passive_value(component)?;
            match voltage {
                Some(SignalData::Series { axis, values }) => {
                    let derivative = derivative(&axis, &values)?;
                    Ok(Some(SignalData::Series {
                        axis,
                        values: derivative
                            .into_iter()
                            .map(|value| value * capacitance)
                            .collect(),
                    }))
                }
                Some(SignalData::Scalar(_)) => Ok(Some(SignalData::Scalar(0.0))),
                None => Ok(None),
            }
        }
        ComponentKind::CurrentSource => source_current(dataset, component),
        ComponentKind::Diode | ComponentKind::BJT(_) | ComponentKind::MOSFET(_) => {
            let key = match component.kind {
                ComponentKind::Diode => format!("@{}[id]", spice_instance_name(component)),
                ComponentKind::BJT(_) => format!("@{}[ic]", spice_instance_name(component)),
                ComponentKind::MOSFET(_) => format!("@{}[id]", spice_instance_name(component)),
                _ => unreachable!(),
            };
            direct_real_signal(dataset, &key.to_ascii_lowercase())
        }
        ComponentKind::OpAmp => Err(
            "op-amp output branch current is not exposed by the safe subcircuit template"
                .to_string(),
        ),
        ComponentKind::ModulePort => Err("module ports have no device current".to_string()),
    }
}

fn power_signal(
    dataset: &Dataset,
    target: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalData>, String> {
    let component = find_component(circuit, target)?;
    let (positive_pin, negative_pin) = device_voltage_pins(component)?;
    let voltage = component_voltage(dataset, component, circuit, positive_pin, negative_pin)?;
    let current = current_signal(dataset, target, circuit)?;
    match (voltage, current) {
        (Some(voltage), Some(current)) => Ok(Some(product_signal(voltage, current)?)),
        _ => Ok(None),
    }
}

fn device_voltage_pins(component: &IRComponent) -> Result<(&'static str, &'static str), String> {
    Ok(match component.kind {
        ComponentKind::Resistor
        | ComponentKind::Capacitor
        | ComponentKind::Inductor
        | ComponentKind::Diode => ("p1", "p2"),
        ComponentKind::VoltageSource | ComponentKind::CurrentSource => ("plus", "minus"),
        ComponentKind::BJT(_) => ("c", "e"),
        ComponentKind::MOSFET(_) => ("d", "s"),
        ComponentKind::OpAmp => ("out", "vee"),
        ComponentKind::ModulePort => return Err("module ports have no device power".to_string()),
    })
}

fn component_voltage(
    dataset: &Dataset,
    component: &IRComponent,
    circuit: &CircuitIR,
    positive_pin: &str,
    negative_pin: &str,
) -> Result<Option<SignalData>, String> {
    let graph = NetlistGraph::build(circuit);
    let positive = graph
        .get_net(&component.id, positive_pin)
        .map(|net| graph.get_net_name(net))
        .ok_or_else(|| format!("{}.{} is not connected", component.id, positive_pin))?;
    let negative = graph
        .get_net(&component.id, negative_pin)
        .map(|net| graph.get_net_name(net))
        .ok_or_else(|| format!("{}.{} is not connected", component.id, negative_pin))?;
    let positive = voltage_signal(dataset, &positive, circuit)?;
    let negative = voltage_signal(dataset, &negative, circuit)?;
    match (positive, negative) {
        (Some(positive), Some(negative)) => Ok(Some(subtract_signal(positive, negative)?)),
        _ => Ok(None),
    }
}

fn source_current(
    dataset: &Dataset,
    component: &IRComponent,
) -> Result<Option<SignalData>, String> {
    let ComponentParams::CurrentSource { value } = &component.parameters else {
        return Err("invalid current-source parameters".to_string());
    };
    match dataset {
        Dataset::OperatingPoint { .. } => match value {
            SourceValue::Dc(quantity) => Ok(Some(SignalData::Scalar(quantity.value))),
            SourceValue::Waveform(_) => Err(
                "waveform current source requires transient data for current measurement"
                    .to_string(),
            ),
        },
        Dataset::Transient(series) | Dataset::DcSweep(series) => Ok(Some(SignalData::Series {
            axis: series.axis.values.clone(),
            values: series
                .axis
                .values
                .iter()
                .map(|time| source_value_at(value, *time))
                .collect::<Result<Vec<_>, _>>()?,
        })),
        Dataset::Ac(_) => Err("current-source AC current reduction is unsupported".to_string()),
    }
}

fn source_value_at(value: &SourceValue, time: f64) -> Result<f64, String> {
    match value {
        SourceValue::Dc(quantity) => Ok(quantity.value),
        SourceValue::Waveform(Waveform::Ac { .. }) => Ok(0.0),
        SourceValue::Waveform(Waveform::Sine {
            offset,
            amplitude,
            frequency,
        }) => {
            Ok(offset.value
                + amplitude.value * (std::f64::consts::TAU * frequency.value * time).sin())
        }
        SourceValue::Waveform(Waveform::SineAc {
            offset,
            amplitude,
            frequency,
            ..
        }) => {
            Ok(offset.value
                + amplitude.value * (std::f64::consts::TAU * frequency.value * time).sin())
        }
        SourceValue::Waveform(Waveform::Pulse { .. } | Waveform::PWL { .. }) => {
            Err("typed current measurement for pulse/PWL source is not implemented".to_string())
        }
    }
}

fn direct_real_signal(dataset: &Dataset, key: &str) -> Result<Option<SignalData>, String> {
    match dataset {
        Dataset::OperatingPoint { values } => {
            Ok(lookup_scalar(values, key).map(SignalData::Scalar))
        }
        Dataset::Transient(series) | Dataset::DcSweep(series) => Ok(lookup_series(series, key)
            .map(|values| SignalData::Series {
                axis: series.axis.values.clone(),
                values: values.clone(),
            })),
        Dataset::Ac(_) => {
            Err("raw current reduction over complex AC data is unsupported".to_string())
        }
    }
}

fn transient_signal(
    signal: &str,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    for dataset in &simulation.datasets {
        if !matches!(dataset.data, Dataset::Transient(_)) {
            continue;
        }
        if let Some(SignalData::Series { axis, values }) =
            resolve_signal(&dataset.data, signal, circuit)?
        {
            return Ok((axis, values));
        }
    }
    Err(format!("'{signal}' requires a transient dataset"))
}

fn complex_signal<'a>(
    dataset: &'a crate::simulation::ComplexSeriesDataset,
    signal: &str,
) -> Result<&'a ComplexSeries, String> {
    let (function, target) = parse_signal(signal)?;
    if !function.eq_ignore_ascii_case("V") {
        return Err(format!(
            "AC engineering metric currently requires voltage signals, got '{signal}'"
        ));
    }
    dataset
        .signals
        .get(&target.to_ascii_lowercase())
        .ok_or_else(|| format!("AC signal '{signal}' was not produced"))
}

fn ratio_at(output: &ComplexSeries, input: &ComplexSeries, index: usize) -> Result<f64, String> {
    let output = output.real[index].hypot(output.imaginary[index]);
    let input = input.real[index].hypot(input.imaginary[index]);
    nonzero_divide(output, input, "AC input magnitude is zero")
}

fn resistor_value(circuit: &CircuitIR, name: &str) -> Result<f64, String> {
    let component = find_component(circuit, name)?;
    if component.kind != ComponentKind::Resistor {
        return Err(format!("output load '{}' is not a resistor", component.id));
    }
    passive_value(component)
}

fn passive_value(component: &IRComponent) -> Result<f64, String> {
    match &component.parameters {
        ComponentParams::TwoPinPassive { value } => Ok(value.value),
        _ => Err(format!("'{}' has no passive scalar value", component.id)),
    }
}

fn find_component<'a>(circuit: &'a CircuitIR, target: &str) -> Result<&'a IRComponent, String> {
    circuit
        .components
        .iter()
        .find(|component| component.id.eq_ignore_ascii_case(target))
        .ok_or_else(|| format!("measurement target '{target}' is not a declared component"))
}

fn spice_instance_name(component: &IRComponent) -> String {
    let prefix = match component.kind {
        ComponentKind::Resistor => "r",
        ComponentKind::Capacitor => "c",
        ComponentKind::Inductor => "l",
        ComponentKind::Diode => "d",
        ComponentKind::BJT(_) => "q",
        ComponentKind::MOSFET(_) => "m",
        ComponentKind::OpAmp => "x",
        ComponentKind::VoltageSource => "v",
        ComponentKind::CurrentSource => "i",
        ComponentKind::ModulePort => "",
    };
    format!("{prefix}_{}", component.id)
}

fn parse_signal(signal: &str) -> Result<(&str, &str), String> {
    let (function, target) = signal
        .split_once('(')
        .ok_or_else(|| format!("invalid measurement primitive '{signal}'"))?;
    let target = target
        .strip_suffix(')')
        .ok_or_else(|| format!("invalid measurement primitive '{signal}'"))?;
    if target.is_empty() {
        return Err(format!(
            "measurement primitive '{signal}' has an empty target"
        ));
    }
    Ok((function, target))
}

fn lookup_scalar(values: &std::collections::BTreeMap<String, f64>, key: &str) -> Option<f64> {
    values
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| *value)
}

fn lookup_series<'a>(series: &'a RealSeriesDataset, key: &str) -> Option<&'a Vec<f64>> {
    series
        .signals
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, values)| values)
}

fn windowed_values(
    axis: &[f64],
    values: &[f64],
    start: &str,
    stop: &str,
) -> Result<Vec<f64>, String> {
    let start = parse_quantity(start, SIUnit::Second)
        .map_err(|error| format!("invalid window start: {error}"))?
        .value;
    let stop = parse_quantity(stop, SIUnit::Second)
        .map_err(|error| format!("invalid window stop: {error}"))?
        .value;
    if start < 0.0 || stop <= start {
        return Err("measurement window requires 0 <= start < stop".to_string());
    }
    let selected = axis
        .iter()
        .zip(values)
        .filter_map(|(time, value)| (*time >= start && *time <= stop).then_some(*value))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!(
            "measurement window {start}..{stop} contains no samples"
        ));
    }
    Ok(selected)
}

fn reduce_scalar(metric: &str, value: f64) -> Result<f64, String> {
    match metric {
        "value" | "min" | "max" | "average" | "avg" => Ok(value),
        "peak" | "rms" => Ok(value.abs()),
        _ => Err(format!("metric '{metric}' is not supported for OP data")),
    }
}

fn reduce_series(metric: &str, values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("measurement series is empty".to_string());
    }
    match metric {
        "value" => Ok(*values.last().expect("nonempty series")),
        "min" => Ok(values.iter().copied().fold(f64::INFINITY, f64::min)),
        "max" => Ok(values.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        "peak" => Ok(values.iter().copied().map(f64::abs).fold(0.0, f64::max)),
        "average" | "avg" => Ok(values.iter().sum::<f64>() / values.len() as f64),
        "rms" => Ok(
            (values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64).sqrt(),
        ),
        _ => Err(format!(
            "metric '{metric}' is not supported for series data"
        )),
    }
}

fn signal_rms(signal: &SignalData) -> Result<f64, String> {
    match signal {
        SignalData::Scalar(value) => Ok(value.abs()),
        SignalData::Series { values, .. } => reduce_series("rms", values),
    }
}

fn average_product(left: &SignalData, right: &SignalData) -> Result<f64, String> {
    match (left, right) {
        (SignalData::Scalar(left), SignalData::Scalar(right)) => Ok(left * right),
        (SignalData::Series { values: left, .. }, SignalData::Series { values: right, .. })
            if left.len() == right.len() && !left.is_empty() =>
        {
            Ok(left
                .iter()
                .zip(right)
                .map(|(left, right)| left * right)
                .sum::<f64>()
                / left.len() as f64)
        }
        _ => Err("power signals have incompatible shapes".to_string()),
    }
}

fn map_signal(signal: SignalData, operation: impl Fn(f64) -> f64) -> SignalData {
    match signal {
        SignalData::Scalar(value) => SignalData::Scalar(operation(value)),
        SignalData::Series { axis, values } => SignalData::Series {
            axis,
            values: values.into_iter().map(operation).collect(),
        },
    }
}

fn subtract_signal(left: SignalData, right: SignalData) -> Result<SignalData, String> {
    combine_signal(left, right, |left, right| left - right)
}

fn product_signal(left: SignalData, right: SignalData) -> Result<SignalData, String> {
    combine_signal(left, right, |left, right| left * right)
}

fn combine_signal(
    left: SignalData,
    right: SignalData,
    operation: impl Fn(f64, f64) -> f64,
) -> Result<SignalData, String> {
    match (left, right) {
        (SignalData::Scalar(left), SignalData::Scalar(right)) => {
            Ok(SignalData::Scalar(operation(left, right)))
        }
        (SignalData::Series { axis, values }, SignalData::Scalar(right)) => {
            Ok(SignalData::Series {
                axis,
                values: values
                    .into_iter()
                    .map(|left| operation(left, right))
                    .collect(),
            })
        }
        (SignalData::Scalar(left), SignalData::Series { axis, values }) => Ok(SignalData::Series {
            axis,
            values: values
                .into_iter()
                .map(|right| operation(left, right))
                .collect(),
        }),
        (SignalData::Series { axis, values: left }, SignalData::Series { values: right, .. })
            if left.len() == right.len() =>
        {
            Ok(SignalData::Series {
                axis,
                values: left
                    .into_iter()
                    .zip(right)
                    .map(|(left, right)| operation(left, right))
                    .collect(),
            })
        }
        _ => Err("measurement signals have incompatible shapes".to_string()),
    }
}

fn derivative(axis: &[f64], values: &[f64]) -> Result<Vec<f64>, String> {
    if axis.len() != values.len() || axis.len() < 2 {
        return Err("derivative requires at least two aligned samples".to_string());
    }
    let mut result = Vec::with_capacity(values.len());
    for index in 0..values.len() {
        let (left, right) = if index == 0 {
            (0, 1)
        } else if index + 1 == values.len() {
            (index - 1, index)
        } else {
            (index - 1, index + 1)
        };
        let delta = axis[right] - axis[left];
        if delta <= 0.0 {
            return Err("derivative axis must be strictly increasing".to_string());
        }
        result.push((values[right] - values[left]) / delta);
    }
    Ok(result)
}

fn dft_magnitude(values: &[f64], bin: usize) -> f64 {
    let (real, imaginary) =
        values
            .iter()
            .enumerate()
            .fold((0.0, 0.0), |(real, imaginary), (index, value)| {
                let angle = std::f64::consts::TAU * bin as f64 * index as f64 / values.len() as f64;
                (real + value * angle.cos(), imaginary - value * angle.sin())
            });
    2.0 * real.hypot(imaginary) / values.len() as f64
}

fn require_count(name: &str, arguments: &[&str], expected: usize) -> Result<(), String> {
    if arguments.len() == expected {
        Ok(())
    } else {
        Err(format!(
            "{name} expects {expected} arguments, got {}",
            arguments.len()
        ))
    }
}

fn nonzero_divide(numerator: f64, denominator: f64, message: &str) -> Result<f64, String> {
    if denominator.abs() <= f64::EPSILON {
        Err(message.to_string())
    } else {
        Ok(numerator / denominator)
    }
}
