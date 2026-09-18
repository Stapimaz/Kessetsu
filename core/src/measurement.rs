use crate::graph::NetlistGraph;
use crate::ir::{
    Assertion, CircuitIR, ComponentKind, ComponentParams, IRComponent, Quantity, ResolvedArgument,
    SIUnit, SourceValue, Waveform, parse_quantity,
};
use crate::simulation::{ComplexSeries, Dataset, RealSeriesDataset, SimulationResult};

pub const MEASUREMENT_SCHEMA_VERSION: &str = "kessetsu.measurement.v3";

pub fn evaluate_assertion_metric(
    assertion: &Assertion,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let metric = assertion.metric.to_ascii_lowercase();
    let arguments = MetricArguments {
        raw: split_arguments(&assertion.signal),
        resolved: &assertion.numeric_arguments,
    };
    match metric.as_str() {
        "value" | "min" | "max" | "peak" | "average" | "avg" | "rms" => {
            evaluate_reduction(&metric, &arguments, circuit, simulation)
        }
        "gain" => evaluate_gain(&arguments, circuit, simulation),
        "gain_at" => evaluate_gain_at(&arguments, simulation),
        "lower_cutoff" | "upper_cutoff" => evaluate_cutoff_edge(&metric, &arguments, simulation),
        "bandwidth" | "cutoff" => evaluate_bandwidth(&arguments, simulation),
        "frequency" => evaluate_frequency(&arguments, circuit, simulation),
        "phase" => evaluate_phase(&arguments, simulation),
        "output_power" => evaluate_output_power(&arguments, circuit, simulation),
        "efficiency" => evaluate_efficiency(&arguments, circuit, simulation),
        "thd" => evaluate_thd(&arguments, circuit, simulation),
        "clipping" => evaluate_clipping(&arguments, circuit, simulation),
        "dissipation" => evaluate_dissipation(&arguments, circuit, simulation),
        "rise_time" | "fall_time" | "settling_time" | "overshoot" | "energy" => {
            evaluate_dynamic(&metric, &arguments, circuit, simulation)
        }
        _ => Err(format!(
            "unsupported assertion metric '{}'; supported engineering metrics are value, min, max, peak, average, rms, gain, gain_at, lower_cutoff, upper_cutoff, bandwidth, cutoff, frequency, phase, output_power, efficiency, thd, clipping and dissipation",
            assertion.metric
        )),
    }
}

fn bounded_transient(
    signal: &str,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
    start: f64,
    stop: f64,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let (axis, values) = transient_signal(signal, circuit, simulation)?;
    interpolate_window(&axis, &values, start, stop)
}

fn interpolate_window(
    axis: &[f64],
    values: &[f64],
    start: f64,
    stop: f64,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    if axis.len() < 2
        || axis.len() != values.len()
        || !start.is_finite()
        || !stop.is_finite()
        || start < 0.0
        || start >= stop
        || axis[0] > start
        || *axis.last().unwrap() < stop
        || axis.iter().chain(values).any(|v| !v.is_finite())
        || axis.windows(2).any(|w| w[0] >= w[1])
    {
        return Err("Dynamic measurement needs finite increasing transient data covering 0 <= start < stop; no extrapolation".into());
    }
    let value_at = |x: f64| {
        let i = axis
            .partition_point(|t| *t <= x)
            .saturating_sub(1)
            .min(axis.len() - 2);
        values[i] + (values[i + 1] - values[i]) * (x - axis[i]) / (axis[i + 1] - axis[i])
    };
    let mut times = vec![start];
    let mut selected = vec![value_at(start)];
    for (t, v) in axis.iter().zip(values) {
        if *t > start && *t < stop {
            times.push(*t);
            selected.push(*v);
        }
    }
    times.push(stop);
    selected.push(value_at(stop));
    Ok((times, selected))
}

fn evaluate_dynamic(
    metric: &str,
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let energy = metric == "energy";
    if arguments.len() != if energy { 4 } else { 5 } {
        return Err("Invalid dynamic measurement arguments".into());
    }
    let start = arguments
        .quantity(if energy { 2 } else { 3 }, SIUnit::Second)?
        .value;
    let stop = arguments
        .quantity(if energy { 3 } else { 4 }, SIUnit::Second)?
        .value;
    if energy {
        let (axis, voltage) = transient_signal(arguments[0], circuit, simulation)?;
        let (current_axis, current) = transient_signal(arguments[1], circuit, simulation)?;
        if axis != current_axis || voltage.len() != current.len() {
            return Err("Energy signals must share the same transient samples".into());
        }
        let power = voltage
            .iter()
            .zip(current)
            .map(|(v, i)| v * i)
            .collect::<Vec<_>>();
        let (axis, power) = interpolate_window(&axis, &power, start, stop)?;
        return Ok(axis
            .windows(2)
            .zip(power.windows(2))
            .map(|(t, p)| (t[1] - t[0]) * (p[0] + p[1]) * 0.5)
            .sum());
    }
    let a = arguments.quantity(1, SIUnit::Volt)?.value;
    let b = arguments.quantity(2, SIUnit::Volt)?.value;
    let (axis, values) = bounded_transient(arguments[0], circuit, simulation, start, stop)?;
    match metric {
        "rise_time" | "fall_time" => {
            if a >= b {
                return Err("Transition thresholds require low < high".into());
            }
            let rising = metric == "rise_time";
            let (first, second) = if rising { (a, b) } else { (b, a) };
            let crossing = |threshold: f64, after: f64| {
                axis.windows(2).zip(values.windows(2)).find_map(|(t, v)| {
                    let directed = if rising {
                        v[0] < threshold && v[1] >= threshold
                    } else {
                        v[0] > threshold && v[1] <= threshold
                    };
                    if !directed {
                        return None;
                    }
                    let time = t[0] + (t[1] - t[0]) * (threshold - v[0]) / (v[1] - v[0]);
                    (time >= after).then_some(time)
                })
            };
            let first_time = crossing(first, start)
                .ok_or("First directed threshold crossing not observed in window")?;
            let second_time = crossing(second, first_time)
                .ok_or("Second directed threshold crossing not observed in window")?;
            Ok(second_time - first_time)
        }
        "settling_time" => {
            if b <= 0.0 {
                return Err("Settling tolerance must be positive volts".into());
            }
            if (values.last().unwrap() - a).abs() > b {
                return Err("Signal has not settled by the end of the specified window".into());
            }
            let Some(index) = values.iter().rposition(|v| (*v - a).abs() > b) else {
                return Ok(0.0);
            };
            let boundary = if values[index] > a { a + b } else { a - b };
            let time = axis[index]
                + (axis[index + 1] - axis[index]) * (boundary - values[index])
                    / (values[index + 1] - values[index]);
            Ok(time - start)
        }
        "overshoot" => {
            if a == b {
                return Err("Overshoot needs distinct initial and target voltages".into());
            }
            let direction = (b - a).signum();
            let peak = values
                .iter()
                .map(|v| direction * (v - b))
                .fold(0.0, f64::max);
            Ok(100.0 * peak / (b - a).abs())
        }
        _ => Err("Unsupported dynamic metric".into()),
    }
}

struct MetricArguments<'a> {
    raw: Vec<&'a str>,
    resolved: &'a [ResolvedArgument],
}

impl<'a> std::ops::Deref for MetricArguments<'a> {
    type Target = [&'a str];
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl MetricArguments<'_> {
    fn quantity(&self, index: usize, unit: SIUnit) -> Result<Quantity, String> {
        if let Some(argument) = self
            .resolved
            .iter()
            .find(|argument| argument.index == index)
        {
            if argument.quantity.unit != unit {
                return Err(format!(
                    "Expected {unit:?}, got {:?}",
                    argument.quantity.unit
                ));
            }
            return Ok(argument.quantity.clone());
        }
        parse_quantity(
            self.raw
                .get(index)
                .ok_or("missing numeric metric argument")?,
            unit,
        )
    }
    fn window(&self, index: usize) -> Result<Option<[f64; 2]>, String> {
        if self.len() <= index {
            return Ok(None);
        }
        let start = self
            .quantity(index, SIUnit::Second)
            .map_err(|error| format!("invalid window start: {error}"))?
            .value;
        let stop = self
            .quantity(index + 1, SIUnit::Second)
            .map_err(|error| format!("invalid window stop: {error}"))?
            .value;
        Ok(Some([start, stop]))
    }
}

#[derive(Debug, Clone)]
enum SignalData {
    Scalar(f64),
    Series { axis: Vec<f64>, values: Vec<f64> },
}

fn evaluate_reduction(
    metric: &str,
    arguments: &MetricArguments<'_>,
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
                    let [start, stop] = arguments.window(1)?.ok_or("missing measurement window")?;
                    windowed_values(&axis, &values, start, stop)?
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
    let maximum = gains.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum > reference * 1.01 {
        return Err(
            "bandwidth/cutoff currently supports low-pass responses whose first AC point is the passband reference"
                .to_string(),
        );
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

/// New AC metrics deliberately validate their complete response without changing
/// the legacy gain/cutoff/phase sampling and analysis-selection semantics.
fn ac_response<'a>(
    arguments: &[&str],
    simulation: &'a SimulationResult,
) -> Result<(&'a [f64], Vec<f64>), String> {
    let ac = simulation
        .datasets
        .iter()
        .find_map(|dataset| match &dataset.data {
            Dataset::Ac(ac) => Some(ac),
            _ => None,
        })
        .ok_or_else(|| {
            "this metric requires an AC analysis dataset; add simulate ac".to_string()
        })?;
    let frequencies = &ac.frequency_hz;
    if frequencies.is_empty()
        || frequencies
            .iter()
            .any(|frequency| !frequency.is_finite() || *frequency <= 0.0)
        || frequencies.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("AC frequencies must be finite, positive and strictly increasing".to_string());
    }
    let output = complex_signal(ac, arguments[0])?;
    let input = complex_signal(ac, arguments[1])?;
    for series in [output, input] {
        if series.real.len() != frequencies.len() || series.imaginary.len() != frequencies.len() {
            return Err("AC signal samples must align with the frequency axis".to_string());
        }
        if series
            .real
            .iter()
            .chain(&series.imaginary)
            .any(|value| !value.is_finite())
        {
            return Err("AC signal samples must be finite".to_string());
        }
    }
    let gains = (0..frequencies.len())
        .map(|index| {
            let denominator = input.real[index].hypot(input.imaginary[index]);
            if denominator == 0.0 {
                return Err(format!(
                    "AC input magnitude is zero at {} Hz",
                    frequencies[index]
                ));
            }
            let gain = output.real[index].hypot(output.imaginary[index]) / denominator;
            if !denominator.is_finite() || !gain.is_finite() {
                return Err(format!(
                    "AC magnitude ratio is non-finite at {} Hz",
                    frequencies[index]
                ));
            }
            Ok(gain)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((frequencies, gains))
}

fn ac_target_frequency(arguments: &MetricArguments<'_>) -> Result<f64, String> {
    let frequency = arguments
        .quantity(2, SIUnit::Hertz)
        .map_err(|error| format!("invalid AC target/reference frequency: {error}"))?
        .value;
    if !frequency.is_finite() || frequency <= 0.0 {
        return Err("AC target/reference frequency must be finite and positive".to_string());
    }
    Ok(frequency)
}

fn gain_at_frequency(frequencies: &[f64], gains: &[f64], target: f64) -> Result<f64, String> {
    if target < frequencies[0] || target > frequencies[frequencies.len() - 1] {
        return Err(format!(
            "frequency {target} Hz is outside the AC sweep {}..{} Hz; extend the sweep",
            frequencies[0],
            frequencies[frequencies.len() - 1]
        ));
    }
    let right = frequencies.partition_point(|frequency| *frequency < target);
    if frequencies[right] == target {
        return Ok(gains[right]);
    }
    let left = right - 1;
    let span = frequencies[right].ln() - frequencies[left].ln();
    if span <= 0.0 {
        return Err(
            "AC frequency samples are too close for log-frequency interpolation".to_string(),
        );
    }
    let fraction = (target.ln() - frequencies[left].ln()) / span;
    // A convex sum avoids overflow when finite gains differ by extreme scales.
    Ok(gains[left] * (1.0 - fraction) + gains[right] * fraction)
}

fn evaluate_gain_at(
    arguments: &MetricArguments<'_>,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("gain_at", arguments, 3)?;
    let target = ac_target_frequency(arguments)?;
    let (frequencies, gains) = ac_response(arguments, simulation)?;
    gain_at_frequency(frequencies, &gains, target)
}

fn evaluate_cutoff_edge(
    metric: &str,
    arguments: &MetricArguments<'_>,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 2 | 3) {
        return Err(format!(
            "{metric} expects output,input[,reference_frequency]"
        ));
    }
    let (frequencies, gains) = ac_response(arguments, simulation)?;
    if frequencies.len() < 2 {
        return Err(format!(
            "{metric} requires at least two AC frequency points"
        ));
    }
    let reference = if arguments.len() == 3 {
        ac_target_frequency(arguments)?
    } else {
        let peak = gains
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        frequencies[peak]
    };
    let reference_gain = gain_at_frequency(frequencies, &gains, reference)?;
    if reference_gain <= 0.0 {
        return Err("AC cutoff reference gain must be positive".to_string());
    }
    let threshold = reference_gain / 2.0_f64.sqrt();
    if threshold <= 0.0 {
        return Err("AC half-power threshold is below numerical resolution".to_string());
    }
    if arguments.len() == 2 {
        let mut bands = 0;
        let mut inside = false;
        for gain in &gains {
            if *gain >= threshold {
                if !inside {
                    bands += 1;
                }
                inside = true;
            } else {
                inside = false;
            }
        }
        if bands > 1 {
            return Err("AC response has multiple disjoint half-power bands; provide a reference_frequency to select one".to_string());
        }
    }
    // Insert an interpolated reference so a frequency between samples selects
    // its own connected half-power band rather than a nearest sample's band.
    let mut points: Vec<_> = frequencies
        .iter()
        .copied()
        .zip(gains.iter().copied())
        .collect();
    let index = points.partition_point(|point| point.0 < reference);
    if points[index].0 != reference {
        points.insert(index, (reference, reference_gain));
    }
    let crossing = |a: (f64, f64), b: (f64, f64)| {
        let fraction = (threshold - a.1) / (b.1 - a.1);
        (a.0.ln() + fraction * (b.0.ln() - a.0.ln())).exp()
    };
    if metric == "lower_cutoff" {
        for left in (0..index).rev() {
            if points[left].1 < threshold {
                return Ok(crossing(points[left], points[left + 1]));
            }
        }
        if index > 0 && points[0].1 == threshold {
            return Ok(points[0].0);
        }
    } else {
        for right in index + 1..points.len() {
            if points[right].1 < threshold {
                return Ok(crossing(points[right - 1], points[right]));
            }
        }
        if index + 1 < points.len() && points.last().unwrap().1 == threshold {
            return Ok(points.last().unwrap().0);
        }
    }
    Err(format!(
        "AC response has no observed {metric} half-power crossing around {reference} Hz; extend the sweep or choose another reference"
    ))
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

fn evaluate_phase(
    arguments: &MetricArguments<'_>,
    simulation: &SimulationResult,
) -> Result<f64, String> {
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
    let index = if arguments.get(2).is_some() {
        let target = arguments
            .quantity(2, SIUnit::Hertz)
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
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 2 | 4) {
        return Err("output_power expects output,load or output,load,start,stop".to_string());
    }
    output_power(
        arguments[0],
        arguments[1],
        arguments.window(2)?,
        circuit,
        simulation,
    )
}

fn output_power(
    output: &str,
    load: &str,
    window: Option<[f64; 2]>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let resistance = resistor_value(circuit, load)?;
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        if let Some(signal) = resolve_signal(&dataset.data, output, circuit)? {
            let signal = maybe_window_signal(signal, window)?;
            let voltage_rms = signal_rms(&signal)?;
            return Ok(voltage_rms * voltage_rms / resistance);
        }
    }
    Err("output_power requires a transient voltage signal".to_string())
}

fn evaluate_efficiency(
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 4 | 6 | 8) {
        return Err(
            "efficiency expects output,load,supply_voltage,supply_current, optional second supply pair, and optional start,stop window"
                .to_string(),
        );
    }
    let has_window = arguments.len() >= 6
        && arguments
            .quantity(arguments.len() - 2, SIUnit::Second)
            .is_ok()
        && arguments
            .quantity(arguments.len() - 1, SIUnit::Second)
            .is_ok();
    let supply_end = if has_window {
        arguments.len() - 2
    } else {
        arguments.len()
    };
    if !matches!(supply_end, 4 | 6) {
        return Err(
            "efficiency requires one or two complete supply voltage/current pairs".to_string(),
        );
    }
    let window = if has_window {
        arguments.window(arguments.len() - 2)?
    } else {
        None
    };
    let output_power = output_power(arguments[0], arguments[1], window, circuit, simulation)?;
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        let mut supply_power = 0.0;
        let mut complete = true;
        for pair in arguments[2..supply_end].chunks_exact(2) {
            let voltage = resolve_signal(&dataset.data, pair[0], circuit)?;
            let current = resolve_signal(&dataset.data, pair[1], circuit)?;
            if let (Some(voltage), Some(current)) = (voltage, current) {
                let voltage = maybe_window_signal(voltage, window)?;
                let current = maybe_window_signal(current, window)?;
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
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("clipping", arguments, 3)?;
    let lower = arguments
        .quantity(1, SIUnit::Volt)
        .map_err(|error| format!("invalid clipping lower rail: {error}"))?
        .value;
    let upper = arguments
        .quantity(2, SIUnit::Volt)
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
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if arguments.len() != 5 {
        return Err("thd expects signal,fundamental,start,stop,hann".to_string());
    }
    if !arguments[4].eq_ignore_ascii_case("hann") {
        return Err("THD window policy must be the explicit 'hann' policy".to_string());
    }
    let fundamental_hz = arguments
        .quantity(1, SIUnit::Hertz)
        .map_err(|error| format!("invalid THD fundamental: {error}"))?
        .value;
    if fundamental_hz <= 0.0 {
        return Err("THD fundamental must be positive".to_string());
    }
    let (axis, values) = transient_signal(arguments[0], circuit, simulation)?;
    let [start, stop] = arguments.window(2)?.ok_or("missing THD window")?;
    let (axis, values) = windowed_series(&axis, &values, start, stop)?;
    if values.len() < 32 {
        return Err(
            "THD requires at least 32 transient samples in its measurement window".to_string(),
        );
    }
    let (axis, values) = uniform_resample(&axis, &values)?;
    let duration = axis.last().unwrap() - axis[0];
    if duration * fundamental_hz < 2.0 {
        return Err(
            "THD measurement window must contain at least two fundamental periods".to_string(),
        );
    }
    let sample_rate = (values.len() - 1) as f64 / duration;
    if fundamental_hz * 5.0 >= sample_rate / 2.0 {
        return Err("THD fifth harmonic exceeds the transient Nyquist limit".to_string());
    }
    let fundamental = windowed_tone_amplitude(&axis, &values, fundamental_hz);
    if fundamental <= f64::EPSILON {
        return Err("THD fundamental magnitude is zero".to_string());
    }
    let harmonic_squared = (2..=5)
        .map(|harmonic| {
            windowed_tone_amplitude(&axis, &values, fundamental_hz * harmonic as f64).powi(2)
        })
        .sum::<f64>();
    Ok(harmonic_squared.sqrt() / fundamental * 100.0)
}

fn evaluate_dissipation(
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 1 | 3) {
        return Err("dissipation expects device or device,start,stop".to_string());
    }
    let signal = format!("P({})", arguments[0]);
    for dataset in simulation
        .datasets
        .iter()
        .filter(|dataset| matches!(dataset.data, Dataset::Transient(_)))
    {
        if let Some(power) = resolve_signal(&dataset.data, &signal, circuit)? {
            let power = maybe_window_signal(power, arguments.window(1)?)?;
            return match power {
                SignalData::Scalar(value) => Ok(value.max(0.0)),
                SignalData::Series { axis, values } => time_weighted_average(
                    &axis,
                    &values
                        .iter()
                        .map(|value| value.max(0.0))
                        .collect::<Vec<_>>(),
                ),
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
    if let Some((positive, negative)) = target.split_once(',') {
        return terminal_pair_voltage(dataset, positive.trim(), negative.trim(), circuit);
    }
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

fn terminal_pair_voltage(
    dataset: &Dataset,
    positive: &str,
    negative: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalData>, String> {
    let graph = NetlistGraph::build(circuit);
    let endpoint_net = |endpoint: &str| -> Result<String, String> {
        let (component_name, pin) = endpoint.split_once('.').ok_or_else(|| {
            format!("terminal-pair voltage endpoint '{endpoint}' must use component.pin")
        })?;
        let component = find_component(circuit, component_name)?;
        let pin_exists = crate::component::component_definition(&component.kind)
            .pins
            .iter()
            .any(|definition| definition.name.eq_ignore_ascii_case(pin));
        if !pin_exists {
            return Err(format!("'{}' has no pin '{}'", component.id, pin));
        }
        graph
            .get_net(&component.id, pin)
            .map(|net| graph.get_net_name(net))
            .ok_or_else(|| format!("{}.{} is not connected", component.id, pin))
    };
    let positive = voltage_signal(dataset, &endpoint_net(positive)?, circuit)?;
    let negative = voltage_signal(dataset, &endpoint_net(negative)?, circuit)?;
    match (positive, negative) {
        (Some(positive), Some(negative)) => Ok(Some(subtract_signal(positive, negative)?)),
        _ => Ok(None),
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
        ComponentKind::ExternalDevice(_) => Err("external subcircuit terminal currents require an explicit measurement source; use a series 0 V source".to_string()),
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
        ComponentKind::ExternalDevice(_) => {
            return Err(
                "external subcircuit power requires explicit terminal measurements".to_string(),
            );
        }
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
        ComponentKind::OpAmp | ComponentKind::ExternalDevice(_) => "x",
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

fn split_arguments(arguments: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut result = Vec::new();
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let argument = arguments[start..index].trim();
                if !argument.is_empty() {
                    result.push(argument);
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    let argument = arguments[start..].trim();
    if !argument.is_empty() {
        result.push(argument);
    }
    result
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
    start: f64,
    stop: f64,
) -> Result<Vec<f64>, String> {
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

fn windowed_series(
    axis: &[f64],
    values: &[f64],
    start_value: f64,
    stop_value: f64,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    if start_value < 0.0 || stop_value <= start_value {
        return Err("measurement window requires 0 <= start < stop".to_string());
    }
    let selected = axis
        .iter()
        .copied()
        .zip(values.iter().copied())
        .filter(|(time, _)| *time >= start_value && *time <= stop_value)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!(
            "measurement window {start_value}..{stop_value} contains no samples"
        ));
    }
    Ok(selected.into_iter().unzip())
}

fn maybe_window_signal(signal: SignalData, window: Option<[f64; 2]>) -> Result<SignalData, String> {
    let Some(window) = window else {
        return Ok(signal);
    };
    match signal {
        SignalData::Scalar(_) => {
            Err("measurement window requires transient series data".to_string())
        }
        SignalData::Series { axis, values } => {
            let (axis, values) = windowed_series(&axis, &values, window[0], window[1])?;
            Ok(SignalData::Series { axis, values })
        }
    }
}

fn uniform_resample(axis: &[f64], values: &[f64]) -> Result<(Vec<f64>, Vec<f64>), String> {
    if axis.len() != values.len() || axis.len() < 2 {
        return Err("THD requires aligned transient samples".to_string());
    }
    if axis.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err("THD time axis must be strictly increasing".to_string());
    }
    let start = axis[0];
    let stop = *axis.last().expect("nonempty axis");
    let step = (stop - start) / (axis.len() - 1) as f64;
    let uniform_axis = (0..axis.len())
        .map(|index| start + step * index as f64)
        .collect::<Vec<_>>();
    let mut source_index = 0usize;
    let mut uniform_values = Vec::with_capacity(values.len());
    for target in &uniform_axis {
        while source_index + 1 < axis.len() && axis[source_index + 1] < *target {
            source_index += 1;
        }
        if source_index + 1 == axis.len() {
            uniform_values.push(values[source_index]);
            continue;
        }
        let left_time = axis[source_index];
        let right_time = axis[source_index + 1];
        let fraction = (*target - left_time) / (right_time - left_time);
        uniform_values.push(
            values[source_index] + fraction * (values[source_index + 1] - values[source_index]),
        );
    }
    Ok((uniform_axis, uniform_values))
}

fn windowed_tone_amplitude(axis: &[f64], values: &[f64], frequency_hz: f64) -> f64 {
    let count = values.len();
    let weights = (0..count)
        .map(|index| 0.5 - 0.5 * (std::f64::consts::TAU * index as f64 / (count - 1) as f64).cos())
        .collect::<Vec<_>>();
    let weight_sum = weights.iter().sum::<f64>();
    let mean = values
        .iter()
        .zip(&weights)
        .map(|(value, weight)| value * weight)
        .sum::<f64>()
        / weight_sum;
    let (real, imaginary) = axis.iter().zip(values).zip(&weights).fold(
        (0.0, 0.0),
        |(real, imaginary), ((time, value), weight)| {
            let phase = std::f64::consts::TAU * frequency_hz * (*time - axis[0]);
            let centered = (value - mean) * weight;
            (
                real + centered * phase.cos(),
                imaginary - centered * phase.sin(),
            )
        },
    );
    2.0 * real.hypot(imaginary) / weight_sum
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
        SignalData::Series { axis, values } => Ok(time_weighted_average(
            axis,
            &values.iter().map(|value| value * value).collect::<Vec<_>>(),
        )?
        .sqrt()),
    }
}

fn average_product(left: &SignalData, right: &SignalData) -> Result<f64, String> {
    match (left, right) {
        (SignalData::Scalar(left), SignalData::Scalar(right)) => Ok(left * right),
        (SignalData::Series { axis, values: left }, SignalData::Series { values: right, .. })
            if left.len() == right.len() && !left.is_empty() =>
        {
            time_weighted_average(
                axis,
                &left
                    .iter()
                    .zip(right)
                    .map(|(left, right)| left * right)
                    .collect::<Vec<_>>(),
            )
        }
        _ => Err("power signals have incompatible shapes".to_string()),
    }
}

fn time_weighted_average(axis: &[f64], values: &[f64]) -> Result<f64, String> {
    if axis.len() != values.len() || axis.len() < 2 {
        return Err("time-weighted measurement requires at least two aligned samples".to_string());
    }
    let mut integral = 0.0;
    for index in 1..axis.len() {
        let delta = axis[index] - axis[index - 1];
        if delta <= 0.0 {
            return Err("time-weighted measurement axis must be strictly increasing".to_string());
        }
        integral += delta * (values[index - 1] + values[index]) * 0.5;
    }
    nonzero_divide(
        integral,
        axis[axis.len() - 1] - axis[0],
        "time-weighted measurement window has zero duration",
    )
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
