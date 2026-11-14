use crate::graph::NetlistGraph;
use crate::ir::{
    Assertion, CircuitIR, ComponentKind, ComponentParams, IRComponent, Quantity, ResolvedArgument,
    SIUnit, SourceValue, Waveform, parse_quantity,
};
use crate::simulation::{ComplexSeries, Dataset, RealSeriesDataset, SimulationResult};

pub const MEASUREMENT_SCHEMA_VERSION: &str = "kessetsu.measurement.v3";
pub const SUPPORTED_METRICS: &[&str] = &[
    "value",
    "min",
    "max",
    "peak",
    "pk2pk",
    "average",
    "rms",
    "gain",
    "gain_at",
    "lower_cutoff",
    "upper_cutoff",
    "bandwidth",
    "cutoff",
    "frequency",
    "freq",
    "period",
    "duty_cycle",
    "duty",
    "phase",
    "output_power",
    "efficiency",
    "thd",
    "clipping",
    "dissipation",
    "rise_time",
    "fall_time",
    "settling_time",
    "overshoot",
    "energy",
];

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
        "value" | "min" | "max" | "peak" | "pk2pk" | "p2p" | "vpp" | "average" | "avg" | "rms" => {
            evaluate_reduction(&metric, &arguments, circuit, simulation)
        }
        "gain" => evaluate_gain(&arguments, circuit, simulation),
        "gain_at" => evaluate_gain_at(&arguments, circuit, simulation),
        "lower_cutoff" | "upper_cutoff" => {
            evaluate_cutoff_edge(&metric, &arguments, circuit, simulation)
        }
        "bandwidth" | "cutoff" => evaluate_bandwidth(&arguments, circuit, simulation),
        "frequency" | "freq" => evaluate_frequency(&arguments, circuit, simulation),
        "period" => evaluate_period(&arguments, circuit, simulation),
        "duty_cycle" | "duty" => evaluate_duty_cycle(&arguments, circuit, simulation),
        "phase" => evaluate_phase(&arguments, circuit, simulation),
        "output_power" => evaluate_output_power(&arguments, circuit, simulation),
        "efficiency" => evaluate_efficiency(&arguments, circuit, simulation),
        "thd" => evaluate_thd(&arguments, circuit, simulation),
        "clipping" => evaluate_clipping(&arguments, circuit, simulation),
        "dissipation" => evaluate_dissipation(&arguments, circuit, simulation),
        "rise_time" | "fall_time" | "settling_time" | "overshoot" | "energy" => {
            evaluate_dynamic(&metric, &arguments, circuit, simulation)
        }
        _ => Err(format!(
            "unsupported assertion metric '{}'; supported engineering metrics are {}",
            assertion.metric,
            SUPPORTED_METRICS.join(", ")
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
        let output = complex_signal(ac, arguments[0], circuit)?;
        let input = complex_signal(ac, arguments[1], circuit)?;
        if output.real.is_empty() || input.real.is_empty() {
            return Err("AC frequency axis is empty".to_string());
        }
        return ratio_at(&output, &input, 0);
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

fn evaluate_bandwidth(
    arguments: &[&str],
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("bandwidth", arguments, 2)?;
    let ac = simulation
        .datasets
        .iter()
        .find_map(|dataset| match &dataset.data {
            Dataset::Ac(ac) => Some(ac),
            _ => None,
        })
        .ok_or_else(|| "bandwidth requires an AC analysis dataset".to_string())?;
    let output = complex_signal(ac, arguments[0], circuit)?;
    let input = complex_signal(ac, arguments[1], circuit)?;
    if ac.frequency_hz.len() < 2 {
        return Err("bandwidth requires at least two AC frequency points".to_string());
    }
    let gains = (0..ac.frequency_hz.len())
        .map(|index| ratio_at(&output, &input, index))
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
    circuit: &CircuitIR,
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
    let output = complex_signal(ac, arguments[0], circuit)?;
    let input = complex_signal(ac, arguments[1], circuit)?;
    for series in [&output, &input] {
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
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    require_count("gain_at", arguments, 3)?;
    let target = ac_target_frequency(arguments)?;
    let (frequencies, gains) = ac_response(arguments, circuit, simulation)?;
    gain_at_frequency(frequencies, &gains, target)
}

fn evaluate_cutoff_edge(
    metric: &str,
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 2 | 3) {
        return Err(format!(
            "{metric} expects output,input[,reference_frequency]"
        ));
    }
    let (frequencies, gains) = ac_response(arguments, circuit, simulation)?;
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

fn transient_signal_windowed(
    metric: &str,
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    if !matches!(arguments.len(), 1 | 3) {
        return Err(format!(
            "metric '{metric}' expects signal or signal,start,stop"
        ));
    }
    let (axis, values) = transient_signal(arguments[0], circuit, simulation)?;
    let (axis, values) = if arguments.len() == 3 {
        let [start, stop] = arguments.window(1)?.ok_or("missing measurement window")?;
        windowed_series(&axis, &values, start, stop)?
    } else {
        (axis, values)
    };
    if axis.len() < 2 || axis.len() != values.len() {
        return Err(format!(
            "{metric} requires at least two aligned transient samples"
        ));
    }
    if axis.iter().chain(&values).any(|value| !value.is_finite()) {
        return Err(format!("{metric} requires finite transient samples"));
    }
    if axis.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(format!(
            "{metric} requires strictly increasing transient time samples"
        ));
    }
    Ok((axis, values))
}

fn rising_crossings(axis: &[f64], values: &[f64], threshold: f64) -> Vec<f64> {
    values
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let left = pair[0] - threshold;
            let right = pair[1] - threshold;
            (left <= 0.0 && right > 0.0).then(|| {
                let fraction = -left / (right - left);
                axis[index] + fraction * (axis[index + 1] - axis[index])
            })
        })
        .collect()
}

fn evaluate_frequency(
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let (axis, values) = transient_signal_windowed("frequency", arguments, circuit, simulation)?;
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let crossings = rising_crossings(&axis, &values, mean);
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

fn evaluate_period(
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    if !matches!(arguments.len(), 1 | 3) {
        return Err("period expects signal or signal,start,stop".to_string());
    }
    let frequency = evaluate_frequency(arguments, circuit, simulation)?;
    if !frequency.is_finite() || frequency <= 0.0 {
        return Err(format!(
            "cannot determine period from non-positive or non-finite frequency ({frequency} Hz)"
        ));
    }
    Ok(1.0 / frequency)
}

fn evaluate_duty_cycle(
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
    simulation: &SimulationResult,
) -> Result<f64, String> {
    let (axis, values) = transient_signal_windowed("duty_cycle", arguments, circuit, simulation)?;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() <= 1e-12 {
        return Err("Signal has zero amplitude; cannot determine duty cycle".to_string());
    }
    let mid = 0.5 * (max + min);
    let crossings = rising_crossings(&axis, &values, mid);
    if crossings.len() < 2 {
        return Err("duty_cycle requires at least two rising midpoint crossings".to_string());
    }
    let t_first = crossings[0];
    let t_last = *crossings.last().expect("at least two crossings");
    let total_span = t_last - t_first;
    if !total_span.is_finite() || total_span <= 0.0 {
        return Err("duty_cycle crossing span must be positive".to_string());
    }
    let mut high_time = 0.0;
    for (time_pair, value_pair) in axis.windows(2).zip(values.windows(2)) {
        let (t0, t1) = (time_pair[0], time_pair[1]);
        let (v0, v1) = (value_pair[0], value_pair[1]);
        let seg_start = t0.max(t_first);
        let seg_end = t1.min(t_last);
        if seg_end <= seg_start {
            continue;
        }
        let value_at = |time: f64| -> f64 {
            if time <= t0 {
                v0
            } else if time >= t1 {
                v1
            } else {
                v0 + (v1 - v0) * (time - t0) / (t1 - t0)
            }
        };
        let d_start = value_at(seg_start) - mid;
        let d_end = value_at(seg_end) - mid;
        let dt = seg_end - seg_start;
        if d_start >= 0.0 && d_end >= 0.0 {
            if d_start > 0.0 || d_end > 0.0 {
                high_time += dt;
            }
        } else if d_start > 0.0 && d_end < 0.0 {
            high_time += dt * (d_start / (d_start - d_end));
        } else if d_start < 0.0 && d_end > 0.0 {
            high_time += dt * (d_end / (d_end - d_start));
        }
    }
    Ok(((high_time / total_span) * 100.0).clamp(0.0, 100.0))
}

fn evaluate_phase(
    arguments: &MetricArguments<'_>,
    circuit: &CircuitIR,
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
    let output = complex_signal(ac, arguments[0], circuit)?;
    let input = complex_signal(ac, arguments[1], circuit)?;
    if output.real.is_empty() || input.real.is_empty() {
        return Err("AC frequency axis is empty".to_string());
    }
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
    if !matches!(arguments.len(), 2 | 4 | 5) {
        return Err("thd expects signal,fundamental[,start,stop[,hann]]".to_string());
    }
    if arguments.len() == 5 && !arguments[4].eq_ignore_ascii_case("hann") {
        return Err("THD window policy must be the explicit 'hann' policy".to_string());
    }
    let fundamental_hz = arguments
        .quantity(1, SIUnit::Hertz)
        .map_err(|error| format!("invalid THD fundamental: {error}"))?
        .value;
    if !fundamental_hz.is_finite() || fundamental_hz <= 0.0 {
        return Err("THD fundamental must be positive".to_string());
    }
    let (axis, values) = transient_signal(arguments[0], circuit, simulation)?;
    let (axis, values) = if arguments.len() >= 4 {
        let [start, stop] = arguments.window(2)?.ok_or("missing THD window")?;
        windowed_series(&axis, &values, start, stop)?
    } else {
        (axis, values)
    };
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
    net_voltage_signal(dataset, target)
}

fn net_voltage_signal(dataset: &Dataset, target: &str) -> Result<Option<SignalData>, String> {
    let key = target.to_ascii_lowercase();
    match dataset {
        Dataset::OperatingPoint { values } => {
            if matches!(key.as_str(), "gnd" | "0") {
                Ok(Some(SignalData::Scalar(0.0)))
            } else if let Some(value) = lookup_scalar(values, &key) {
                Ok(Some(SignalData::Scalar(value)))
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

fn resolve_endpoint_net(
    endpoint: &str,
    circuit: &CircuitIR,
    graph: &NetlistGraph,
) -> Result<String, String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err("measurement endpoint cannot be empty".to_string());
    }
    if let Some((component_name, pin)) = endpoint.split_once('.') {
        let component = find_component(circuit, component_name)?;
        let pin_def = crate::component::component_definition(&component.kind)
            .pins
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(pin))
            .ok_or_else(|| format!("'{}' has no pin '{}'", component.id, pin))?;
        return graph
            .get_net(&component.id, pin_def.name)
            .map(|net| graph.get_net_name(net))
            .ok_or_else(|| format!("{}.{} is not connected", component.id, pin_def.name));
    }
    if endpoint.eq_ignore_ascii_case("0") || endpoint.eq_ignore_ascii_case("gnd") {
        return Ok("0".to_string());
    }
    if let Some(net_name) = circuit
        .nets
        .iter()
        .find(|net| net.eq_ignore_ascii_case(endpoint))
    {
        if let Some(net) = graph.pin_to_net.get(net_name) {
            return Ok(graph.get_net_name(*net));
        }
        return Ok(net_name.clone());
    }
    if let Some(net_name) = graph
        .net_names
        .values()
        .find(|name| name.eq_ignore_ascii_case(endpoint))
    {
        return Ok(net_name.clone());
    }
    if let Some((_, net)) = graph
        .pin_to_net
        .iter()
        .find(|(pin, _)| !pin.contains('.') && pin.eq_ignore_ascii_case(endpoint))
    {
        return Ok(graph.get_net_name(*net));
    }
    if endpoint.bytes().all(|byte| byte.is_ascii_digit()) {
        let mapped = format!("n_{endpoint}");
        if let Some(net_name) = circuit
            .nets
            .iter()
            .find(|net| net.eq_ignore_ascii_case(&mapped))
        {
            if let Some(net) = graph.pin_to_net.get(net_name) {
                return Ok(graph.get_net_name(*net));
            }
            return Ok(net_name.clone());
        }
        if let Some(net_name) = graph
            .net_names
            .values()
            .find(|name| name.eq_ignore_ascii_case(&mapped))
        {
            return Ok(net_name.clone());
        }
    }
    Err(format!(
        "measurement endpoint net '{endpoint}' does not exist in circuit"
    ))
}

fn terminal_pair_voltage(
    dataset: &Dataset,
    positive: &str,
    negative: &str,
    circuit: &CircuitIR,
) -> Result<Option<SignalData>, String> {
    if negative.contains(',') {
        return Err(format!(
            "differential voltage V({positive}, {negative}) expects two endpoints"
        ));
    }
    let graph = NetlistGraph::build(circuit);
    let positive_net = resolve_endpoint_net(positive, circuit, &graph)?;
    let negative_net = resolve_endpoint_net(negative, circuit, &graph)?;
    let positive = net_voltage_signal(dataset, &positive_net)?;
    let negative = net_voltage_signal(dataset, &negative_net)?;
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
        ComponentKind::VoltageSource
        | ComponentKind::Inductor
        | ComponentKind::Vcvs
        | ComponentKind::Ccvs => direct_real_signal(dataset, &branch_key),
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
        ComponentKind::Diode
        | ComponentKind::BJT(_)
        | ComponentKind::MOSFET(_)
        | ComponentKind::JFET(_) => {
            let key = match component.kind {
                ComponentKind::Diode => format!("@{}[id]", spice_instance_name(component)),
                ComponentKind::BJT(_) => format!("@{}[ic]", spice_instance_name(component)),
                ComponentKind::MOSFET(_) | ComponentKind::JFET(_) => {
                    format!("@{}[id]", spice_instance_name(component))
                }
                _ => unreachable!(),
            };
            direct_real_signal(dataset, &key.to_ascii_lowercase())
        }
        ComponentKind::OpAmp => Err(
            "op-amp output branch current is not exposed by the safe subcircuit template"
                .to_string(),
        ),
        ComponentKind::Vccs | ComponentKind::Cccs => Err(
            "dependent current source terminal currents require an explicit measurement source; use a series 0 V source"
                .to_string(),
        ),
        ComponentKind::Switch => Err(
            "switch terminal currents require an explicit measurement source; use a series 0 V source"
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
        | ComponentKind::Diode
        | ComponentKind::Switch => ("p1", "p2"),
        ComponentKind::VoltageSource | ComponentKind::CurrentSource => ("plus", "minus"),
        ComponentKind::BJT(_) => ("c", "e"),
        ComponentKind::MOSFET(_) | ComponentKind::JFET(_) => ("d", "s"),
        ComponentKind::OpAmp => ("out", "vee"),
        ComponentKind::Vcvs | ComponentKind::Vccs | ComponentKind::Ccvs | ComponentKind::Cccs => {
            ("p_out", "n_out")
        }
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
    let positive = net_voltage_signal(dataset, &positive)?;
    let negative = net_voltage_signal(dataset, &negative)?;
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
        SourceValue::Waveform(Waveform::Ac { offset, .. }) => {
            Ok(offset.as_ref().map_or(0.0, |q| q.value))
        }
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
        SourceValue::Waveform(Waveform::Exp {
            v1,
            v2,
            td1,
            tau1,
            td2,
            tau2,
        }) => {
            let val = if time <= td1.value {
                v1.value
            } else if time <= td2.value {
                v1.value + (v2.value - v1.value) * (1.0 - (-(time - td1.value) / tau1.value).exp())
            } else {
                v1.value
                    + (v2.value - v1.value) * (1.0 - (-(time - td1.value) / tau1.value).exp())
                    + (v1.value - v2.value) * (1.0 - (-(time - td2.value) / tau2.value).exp())
            };
            Ok(val)
        }
        SourceValue::Waveform(Waveform::PWL { points }) => {
            if points.is_empty() {
                return Ok(0.0);
            }
            if time <= points[0].0.value {
                return Ok(points[0].1.value);
            }
            let last = points.last().expect("non-empty PWL points");
            if time >= last.0.value {
                return Ok(last.1.value);
            }
            for pair in points.windows(2) {
                let (t0, v0) = (pair[0].0.value, pair[0].1.value);
                let (t1, v1) = (pair[1].0.value, pair[1].1.value);
                if time <= t1 {
                    let dt = t1 - t0;
                    if dt <= 0.0 {
                        return Ok(v1);
                    }
                    return Ok(v0 + (v1 - v0) * (time - t0) / dt);
                }
            }
            Ok(last.1.value)
        }
        SourceValue::Waveform(Waveform::Pulse { .. }) => {
            Err("typed current measurement for pulse source is not implemented".to_string())
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

fn lookup_complex_net(
    dataset: &crate::simulation::ComplexSeriesDataset,
    net: &str,
    signal: &str,
) -> Result<ComplexSeries, String> {
    if net.eq_ignore_ascii_case("0") || net.eq_ignore_ascii_case("gnd") {
        return Ok(ComplexSeries {
            real: vec![0.0; dataset.frequency_hz.len()],
            imaginary: vec![0.0; dataset.frequency_hz.len()],
        });
    }
    let key = net.to_ascii_lowercase();
    dataset
        .signals
        .get(&key)
        .or_else(|| {
            dataset
                .signals
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&key))
                .map(|(_, series)| series)
        })
        .or_else(|| {
            key.bytes()
                .all(|byte| byte.is_ascii_digit())
                .then(|| {
                    let mapped = format!("n_{key}");
                    dataset.signals.get(&mapped).or_else(|| {
                        dataset
                            .signals
                            .iter()
                            .find(|(name, _)| name.eq_ignore_ascii_case(&mapped))
                            .map(|(_, series)| series)
                    })
                })
                .flatten()
        })
        .cloned()
        .ok_or_else(|| format!("AC signal '{signal}' was not produced"))
}

fn complex_signal(
    dataset: &crate::simulation::ComplexSeriesDataset,
    signal: &str,
    circuit: &CircuitIR,
) -> Result<ComplexSeries, String> {
    let (function, target) = parse_signal(signal)?;
    if !function.eq_ignore_ascii_case("V") {
        return Err(format!(
            "AC engineering metric currently requires voltage signals, got '{signal}'"
        ));
    }
    let graph = NetlistGraph::build(circuit);
    if let Some((left, right)) = target.split_once(',') {
        let left = left.trim();
        let right = right.trim();
        if right.contains(',') {
            return Err(format!(
                "differential AC voltage '{signal}' expects two endpoints"
            ));
        }
        let left_net = resolve_endpoint_net(left, circuit, &graph)?;
        let right_net = resolve_endpoint_net(right, circuit, &graph)?;
        let left_series = lookup_complex_net(dataset, &left_net, signal)?;
        let right_series = lookup_complex_net(dataset, &right_net, signal)?;
        if left_series.real.len() != right_series.real.len()
            || left_series.imaginary.len() != right_series.imaginary.len()
        {
            return Err(format!(
                "AC differential signal '{signal}' endpoints have mismatched sample counts"
            ));
        }
        return Ok(ComplexSeries {
            real: left_series
                .real
                .iter()
                .zip(&right_series.real)
                .map(|(l, r)| l - r)
                .collect(),
            imaginary: left_series
                .imaginary
                .iter()
                .zip(&right_series.imaginary)
                .map(|(l, r)| l - r)
                .collect(),
        });
    }
    let trimmed = target.trim();
    if trimmed.contains('.') {
        let net = resolve_endpoint_net(trimmed, circuit, &graph)?;
        return lookup_complex_net(dataset, &net, signal);
    }
    match resolve_endpoint_net(trimmed, circuit, &graph) {
        Ok(net) => lookup_complex_net(dataset, &net, signal),
        Err(_) => lookup_complex_net(dataset, trimmed, signal),
    }
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
        ComponentKind::Switch => "s",
        ComponentKind::BJT(_) => "q",
        ComponentKind::MOSFET(_) => "m",
        ComponentKind::JFET(_) => "j",
        ComponentKind::OpAmp | ComponentKind::ExternalDevice(_) => "x",
        ComponentKind::VoltageSource => "v",
        ComponentKind::CurrentSource => "i",
        ComponentKind::Vcvs => "e",
        ComponentKind::Vccs => "g",
        ComponentKind::Ccvs => "h",
        ComponentKind::Cccs => "f",
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
        .or_else(|| {
            key.bytes().all(|byte| byte.is_ascii_digit()).then(|| {
                let mapped = format!("n_{key}");
                values
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&mapped))
                    .map(|(_, value)| *value)
            })?
        })
}

fn lookup_series<'a>(series: &'a RealSeriesDataset, key: &str) -> Option<&'a Vec<f64>> {
    series
        .signals
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, values)| values)
        .or_else(|| {
            key.bytes().all(|byte| byte.is_ascii_digit()).then(|| {
                let mapped = format!("n_{key}");
                series
                    .signals
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&mapped))
                    .map(|(_, values)| values)
            })?
        })
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
        "pk2pk" | "p2p" | "vpp" => {
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            Ok(max - min)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Cmp;
    use crate::ir::{Analysis, ast_to_ir};
    use crate::parser::parse_program;
    use crate::simulation::{
        AnalysisDataset, ComplexSeriesDataset, SeriesAxis, SimulationStatus, SimulatorInfo,
        SimulatorLog, SimulatorProcessStatus,
    };
    use std::collections::BTreeMap;

    fn test_circuit(extra: &str) -> CircuitIR {
        let source = format!(
            "net GND\nnet IN\nnet OUT\nnet OUT_P\nnet OUT_N\n\
             source VIN 1V\nresistor R1 1k\n\
             connect VIN.plus to IN\nconnect VIN.minus to GND\n\
             connect R1.p1 to OUT_P\nconnect R1.p2 to OUT_N\n{extra}"
        );
        let program = parse_program(&source).expect("test circuit should parse");
        ast_to_ir(&program).expect("test circuit should reach IR")
    }

    fn sim_with(analysis: Analysis, data: Dataset) -> SimulationResult {
        SimulationResult {
            schema_version: "kessetsu.simulation.v1".to_string(),
            status: SimulationStatus::Succeeded,
            analyses: vec![analysis.clone()],
            simulator: SimulatorInfo {
                executable: "fixture".to_string(),
                version: "fixture-1".to_string(),
            },
            process: SimulatorProcessStatus {
                exit_code: Some(0),
                success: true,
            },
            measurements: BTreeMap::new(),
            datasets: vec![AnalysisDataset {
                index: 0,
                analysis,
                data,
            }],
            diagnostics: vec![],
            warnings: vec![],
            errors: vec![],
            raw_log: SimulatorLog {
                stdout: String::new(),
                stderr: String::new(),
            },
            artifacts: vec![],
        }
    }

    fn eval(
        metric: &str,
        signal: &str,
        circuit: &CircuitIR,
        sim: &SimulationResult,
    ) -> Result<f64, String> {
        let assertion = Assertion {
            metric: metric.to_string(),
            signal: signal.to_string(),
            cmp: Cmp::Gt,
            threshold: Quantity {
                value: 0.0,
                unit: SIUnit::Volt,
            },
            numeric_arguments: Vec::new(),
        };
        evaluate_assertion_metric(&assertion, circuit, sim)
    }

    fn tran_sim(axis: Vec<f64>, signals: BTreeMap<String, Vec<f64>>) -> SimulationResult {
        sim_with(
            Analysis::Transient {
                step: Quantity {
                    value: 1e-6,
                    unit: SIUnit::Second,
                },
                stop: Quantity {
                    value: *axis.last().unwrap_or(&1e-3),
                    unit: SIUnit::Second,
                },
                use_initial_conditions: false,
            },
            Dataset::Transient(RealSeriesDataset {
                axis: SeriesAxis {
                    name: "time".to_string(),
                    values: axis,
                },
                signals,
            }),
        )
    }

    fn pulse_wave(
        start: f64,
        stop: f64,
        samples: usize,
        period: f64,
        duty: f64,
        phase_offset: f64,
        low: f64,
        high: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let mut axis = Vec::with_capacity(samples);
        let mut values = Vec::with_capacity(samples);
        let edge = period * 0.002;
        for i in 0..samples {
            let t = start + (stop - start) * i as f64 / (samples - 1) as f64;
            let phase = (t + phase_offset).rem_euclid(period) / period;
            let high_frac = duty;
            let edge_frac = edge / period;
            let v = if phase < edge_frac {
                low + (high - low) * (phase / edge_frac)
            } else if phase < high_frac {
                high
            } else if phase < high_frac + edge_frac {
                high - (high - low) * ((phase - high_frac) / edge_frac)
            } else {
                low
            };
            axis.push(t);
            values.push(v);
        }
        (axis, values)
    }

    #[test]
    fn differential_net_voltage_works_across_op_transient_dc_and_ac() {
        let circuit = test_circuit("");

        let op = sim_with(
            Analysis::OperatingPoint,
            Dataset::OperatingPoint {
                values: BTreeMap::from([
                    ("out_p".to_string(), 3.25),
                    ("out_n".to_string(), 1.25),
                    ("in".to_string(), 1.0),
                ]),
            },
        );
        assert!((eval("value", "V(OUT_P, OUT_N)", &circuit, &op).unwrap() - 2.0).abs() < 1e-12);
        assert!((eval("value", "V(out_p, gnd)", &circuit, &op).unwrap() - 3.25).abs() < 1e-12);
        assert!((eval("value", "V(0, OUT_N)", &circuit, &op).unwrap() + 1.25).abs() < 1e-12);
        assert!((eval("value", "V(R1.p1, OUT_N)", &circuit, &op).unwrap() - 2.0).abs() < 1e-12);
        assert!(
            eval("value", "V(NOPE, OUT_N)", &circuit, &op)
                .unwrap_err()
                .contains("NOPE")
        );

        let tran = tran_sim(
            vec![0.0, 1e-3, 2e-3, 3e-3],
            BTreeMap::from([
                ("out_p".to_string(), vec![1.0, 3.0, 1.0, -1.0]),
                ("out_n".to_string(), vec![1.0, 1.0, 2.0, 1.0]),
            ]),
        );
        assert!((eval("max", "V(OUT_P, OUT_N)", &circuit, &tran).unwrap() - 2.0).abs() < 1e-12);
        assert!((eval("min", "V(OUT_P, OUT_N)", &circuit, &tran).unwrap() + 2.0).abs() < 1e-12);
        assert!((eval("pk2pk", "V(R1.p1, OUT_N)", &circuit, &tran).unwrap() - 4.0).abs() < 1e-12);

        let dc = sim_with(
            Analysis::DcSweep {
                source: "VIN".to_string(),
                start: Quantity {
                    value: 0.0,
                    unit: SIUnit::Volt,
                },
                stop: Quantity {
                    value: 2.0,
                    unit: SIUnit::Volt,
                },
                step: Quantity {
                    value: 1.0,
                    unit: SIUnit::Volt,
                },
            },
            Dataset::DcSweep(RealSeriesDataset {
                axis: SeriesAxis {
                    name: "v-sweep".to_string(),
                    values: vec![0.0, 1.0, 2.0],
                },
                signals: BTreeMap::from([
                    ("out_p".to_string(), vec![0.5, 2.5, 4.5]),
                    ("out_n".to_string(), vec![0.5, 0.5, 0.5]),
                ]),
            }),
        );
        assert!((eval("max", "V(OUT_P, OUT_N)", &circuit, &dc).unwrap() - 4.0).abs() < 1e-12);

        let ac = sim_with(
            Analysis::Ac {
                scale: crate::ir::AcScale::Decade,
                points: 10,
                start: Quantity {
                    value: 10.0,
                    unit: SIUnit::Hertz,
                },
                stop: Quantity {
                    value: 10_000.0,
                    unit: SIUnit::Hertz,
                },
            },
            Dataset::Ac(ComplexSeriesDataset {
                frequency_hz: vec![10.0, 100.0, 1_000.0, 10_000.0],
                signals: BTreeMap::from([
                    (
                        "out_p".to_string(),
                        ComplexSeries {
                            real: vec![2.0, 2.0, 1.0, 0.2],
                            imaginary: vec![0.0, 0.0, -1.0, -0.4],
                        },
                    ),
                    (
                        "out_n".to_string(),
                        ComplexSeries {
                            real: vec![-2.0, -2.0, -1.0, -0.2],
                            imaginary: vec![0.0, 0.0, 1.0, 0.4],
                        },
                    ),
                    (
                        "in".to_string(),
                        ComplexSeries {
                            real: vec![1.0; 4],
                            imaginary: vec![0.0; 4],
                        },
                    ),
                ]),
            }),
        );
        assert!(
            (eval("gain", "V(OUT_P, OUT_N), V(IN)", &circuit, &ac).unwrap() - 4.0).abs() < 1e-12
        );
        assert!(
            (eval("gain", "V(out_p, out_n), V(IN)", &circuit, &ac).unwrap() - 4.0).abs() < 1e-12
        );
        assert!(
            (eval("gain", "V(R1.p1, OUT_N), V(IN)", &circuit, &ac).unwrap() - 4.0).abs() < 1e-12
        );
        assert!(
            (eval("gain", "V(OUT_P, 0), V(IN, GND)", &circuit, &ac).unwrap() - 2.0).abs() < 1e-12
        );
        assert!(
            (eval("gain_at", "V(OUT_P, OUT_N), V(IN), 1kHz", &circuit, &ac).unwrap()
                - 8.0_f64.sqrt())
            .abs()
                < 1e-12
        );
        assert!(
            (eval("phase", "V(OUT_P, OUT_N), V(IN), 1kHz", &circuit, &ac).unwrap() + 45.0).abs()
                < 1e-12
        );
        assert!(
            (eval("upper_cutoff", "V(OUT_P, OUT_N), V(IN)", &circuit, &ac).unwrap() - 1_000.0)
                .abs()
                < 1e-9
        );
        assert!(
            (eval("bandwidth", "V(OUT_P, OUT_N), V(IN)", &circuit, &ac).unwrap() - 1_000.0).abs()
                < 1e-9
        );
        assert!(eval("gain", "V(UNKNOWN, OUT_N), V(IN)", &circuit, &ac).is_err());
    }

    #[test]
    fn frequency_freq_and_period_support_full_and_windowed_transient_signals() {
        let circuit = test_circuit("");
        let n = 1001;
        let axis: Vec<f64> = (0..n).map(|i| i as f64 * 1e-5).collect();
        let values: Vec<f64> = axis
            .iter()
            .map(|&t| {
                if t < 4e-3 {
                    (std::f64::consts::TAU * 500.0 * t).sin()
                } else {
                    (std::f64::consts::TAU * 2_000.0 * (t - 4e-3)).sin()
                }
            })
            .collect();
        let sim = tran_sim(axis.clone(), BTreeMap::from([("out".to_string(), values)]));

        let f_early = eval("frequency", "V(OUT), 0ms, 4ms", &circuit, &sim).unwrap();
        let f_late = eval("freq", "V(OUT), 4ms, 10ms", &circuit, &sim).unwrap();
        let p_late = eval("period", "V(OUT), 4ms, 10ms", &circuit, &sim).unwrap();
        assert!((f_early - 500.0).abs() < 2.0, "got {f_early}");
        assert!((f_late - 2_000.0).abs() < 5.0, "got {f_late}");
        assert!((p_late - 0.0005).abs() < 2e-6, "got {p_late}");

        let clean_values: Vec<f64> = axis
            .iter()
            .map(|&t| (std::f64::consts::TAU * 1_000.0 * t).sin())
            .collect();
        let clean_sim = tran_sim(axis, BTreeMap::from([("out".to_string(), clean_values)]));
        assert!((eval("freq", "V(OUT)", &circuit, &clean_sim).unwrap() - 1_000.0).abs() < 1.0);
        assert!((eval("period", "V(OUT)", &circuit, &clean_sim).unwrap() - 1e-3).abs() < 1e-6);

        let dc_tran = tran_sim(
            vec![0.0, 1e-3, 2e-3, 3e-3],
            BTreeMap::from([("out".to_string(), vec![2.5; 4])]),
        );
        assert!(eval("freq", "V(OUT)", &circuit, &dc_tran).is_err());
        assert!(eval("period", "V(OUT)", &circuit, &dc_tran).is_err());
    }

    #[test]
    fn duty_cycle_and_duty_measure_pulses_and_windows() {
        let circuit = test_circuit("");

        let (axis50, vals50) = pulse_wave(0.0, 5e-3, 5001, 1e-3, 0.50, 0.0, 0.0, 5.0);
        let sim50 = tran_sim(axis50, BTreeMap::from([("out".to_string(), vals50)]));
        assert!((eval("duty_cycle", "V(OUT)", &circuit, &sim50).unwrap() - 50.0).abs() < 0.2);
        assert!((eval("duty", "V(OUT)", &circuit, &sim50).unwrap() - 50.0).abs() < 0.2);

        let (axis25, vals25) = pulse_wave(0.0, 5e-3, 5001, 1e-3, 0.25, 0.1e-3, 0.0, 3.3);
        let sim25 = tran_sim(axis25, BTreeMap::from([("out".to_string(), vals25)]));
        assert!((eval("duty_cycle", "V(OUT)", &circuit, &sim25).unwrap() - 25.0).abs() < 0.2);
        assert!((eval("duty", "V(OUT), 1ms, 5ms", &circuit, &sim25).unwrap() - 25.0).abs() < 0.2);

        // Waveform starting mid-high (partial pulse before first rising crossing) should still report 30%.
        let (axis30, vals30) = pulse_wave(0.0, 5e-3, 5001, 1e-3, 0.30, 0.15e-3, 0.0, 5.0);
        assert!(vals30[0] > 4.0);
        let sim30 = tran_sim(axis30, BTreeMap::from([("out".to_string(), vals30)]));
        assert!((eval("duty_cycle", "V(OUT)", &circuit, &sim30).unwrap() - 30.0).abs() < 0.2);

        let (axis_mix, mut vals_mix) = pulse_wave(0.0, 10e-3, 10001, 1e-3, 0.25, 0.0, 0.0, 5.0);
        let (_, vals_late) = pulse_wave(0.0, 10e-3, 10001, 1e-3, 0.75, 0.0, 0.0, 5.0);
        for (i, &t) in axis_mix.iter().enumerate() {
            if t >= 4e-3 {
                vals_mix[i] = vals_late[i];
            }
        }
        let sim_mix = tran_sim(axis_mix, BTreeMap::from([("out".to_string(), vals_mix)]));
        assert!(
            (eval("duty_cycle", "V(OUT), 0ms, 4ms", &circuit, &sim_mix).unwrap() - 25.0).abs()
                < 0.3
        );
        assert!(
            (eval("duty", "V(OUT), 4ms, 10ms", &circuit, &sim_mix).unwrap() - 75.0).abs() < 0.3
        );

        let flat = tran_sim(
            vec![0.0, 1e-3, 2e-3],
            BTreeMap::from([("out".to_string(), vec![1.5; 3])]),
        );
        assert_eq!(
            eval("duty_cycle", "V(OUT)", &circuit, &flat).unwrap_err(),
            "Signal has zero amplitude; cannot determine duty cycle"
        );
    }

    #[test]
    fn thd_supports_two_four_and_five_argument_forms() {
        let circuit = test_circuit("");
        let axis: Vec<f64> = (0..501).map(|i| i as f64 * 1e-5).collect();
        let values: Vec<f64> = axis
            .iter()
            .map(|&t| {
                let w = std::f64::consts::TAU * 1_000.0 * t;
                w.sin() + 0.1 * (2.0 * w).sin()
            })
            .collect();
        let sim = tran_sim(axis, BTreeMap::from([("out".to_string(), values)]));

        let thd2 = eval("thd", "V(OUT), 1kHz", &circuit, &sim).unwrap();
        let thd5_full = eval("thd", "V(OUT), 1kHz, 0ms, 5ms, hann", &circuit, &sim).unwrap();
        assert!((thd2 - thd5_full).abs() < 1e-12);
        assert!((thd2 - 10.0).abs() < 0.1, "got {thd2}");

        let thd4 = eval("thd", "V(OUT), 1kHz, 1ms, 5ms", &circuit, &sim).unwrap();
        let thd5_win = eval("thd", "V(OUT), 1kHz, 1ms, 5ms, hann", &circuit, &sim).unwrap();
        assert!((thd4 - thd5_win).abs() < 1e-12);
        assert!((thd4 - 10.0).abs() < 0.1, "got {thd4}");

        assert!(eval("thd", "V(OUT), 1kHz, 1ms, 5ms, rect", &circuit, &sim).is_err());
        assert!(eval("thd", "V(OUT), 1kHz, 1ms", &circuit, &sim).is_err());
        assert!(eval("freq", "V(OUT), 1ms", &circuit, &sim).is_err());
        assert!(eval("period", "V(OUT), 1ms", &circuit, &sim).is_err());
        assert!(eval("duty_cycle", "V(OUT), 1ms", &circuit, &sim).is_err());
    }

    #[test]
    fn pwl_current_source_interpolates_time_points() {
        let pwl = SourceValue::Waveform(Waveform::PWL {
            points: vec![
                (
                    Quantity {
                        value: 0.0,
                        unit: SIUnit::Second,
                    },
                    Quantity {
                        value: 0.0,
                        unit: SIUnit::Ampere,
                    },
                ),
                (
                    Quantity {
                        value: 1e-3,
                        unit: SIUnit::Second,
                    },
                    Quantity {
                        value: 0.01,
                        unit: SIUnit::Ampere,
                    },
                ),
                (
                    Quantity {
                        value: 2e-3,
                        unit: SIUnit::Second,
                    },
                    Quantity {
                        value: 0.01,
                        unit: SIUnit::Ampere,
                    },
                ),
                (
                    Quantity {
                        value: 3e-3,
                        unit: SIUnit::Second,
                    },
                    Quantity {
                        value: 0.0,
                        unit: SIUnit::Ampere,
                    },
                ),
            ],
        });
        assert!((source_value_at(&pwl, -1e-4).unwrap() - 0.0).abs() < 1e-12);
        assert!((source_value_at(&pwl, 0.0).unwrap() - 0.0).abs() < 1e-12);
        assert!((source_value_at(&pwl, 5e-4).unwrap() - 0.005).abs() < 1e-12);
        assert!((source_value_at(&pwl, 1.5e-3).unwrap() - 0.01).abs() < 1e-12);
        assert!((source_value_at(&pwl, 2.5e-3).unwrap() - 0.005).abs() < 1e-12);
        assert!((source_value_at(&pwl, 5e-3).unwrap() - 0.0).abs() < 1e-12);

        let empty = SourceValue::Waveform(Waveform::PWL { points: vec![] });
        assert!((source_value_at(&empty, 1e-3).unwrap() - 0.0).abs() < 1e-12);

        let circuit = test_circuit(
            "current_source I1 pwl(0s, 0A, 1ms, 10mA, 2ms, 10mA, 3ms, 0A)\n\
             connect I1.plus to OUT\nconnect I1.minus to GND\n",
        );
        let sim = tran_sim(
            vec![0.0, 0.5e-3, 1.0e-3, 2.0e-3, 2.5e-3, 3.0e-3],
            BTreeMap::from([("out".to_string(), vec![0.0; 6])]),
        );
        assert!((eval("peak", "I(I1)", &circuit, &sim).unwrap() - 0.01).abs() < 1e-12);
        assert!((eval("average", "I(I1)", &circuit, &sim).unwrap() - (0.03 / 6.0)).abs() < 1e-12);
    }

    #[test]
    fn compiled_assertions_evaluate_new_periodic_thd_and_differential_metrics() {
        let source = "\
net GND
net IN
net OUT
net OUT_P
net OUT_N
source VIN 1V
resistor R1 1k
connect VIN.plus to IN
connect VIN.minus to GND
connect R1.p1 to OUT_P
connect R1.p2 to OUT_N
simulate tran 10us 5ms
assert freq(V(OUT)) > 990Hz
assert frequency(V(OUT), 1ms, 5ms) < 1010Hz
assert period(V(OUT)) > 0.99ms
assert period(V(OUT), 1ms, 5ms) < 1.01ms
assert duty_cycle(V(OUT)) > 49%
assert duty(V(OUT), 1ms, 5ms) < 51%
assert thd(V(OUT), 1kHz) < 10.5%
assert thd(V(OUT), 1kHz, 1ms, 5ms) < 10.5%
assert max(V(OUT_P, OUT_N)) > 1.9V
";
        let program = parse_program(source).expect("assertions should parse");
        let circuit = ast_to_ir(&program).expect("assertions should compile to IR");
        let axis: Vec<f64> = (0..501).map(|i| i as f64 * 1e-5).collect();
        let out: Vec<f64> = axis
            .iter()
            .map(|&t| {
                let w = std::f64::consts::TAU * 1_000.0 * t;
                w.sin() + 0.1 * (2.0 * w).sin()
            })
            .collect();
        let out_p = vec![2.0; axis.len()];
        let out_n = vec![0.0; axis.len()];
        let sim = tran_sim(
            axis,
            BTreeMap::from([
                ("out".to_string(), out),
                ("out_p".to_string(), out_p),
                ("out_n".to_string(), out_n),
            ]),
        );
        let report = crate::sim_result::evaluate_assertions(&circuit, &sim);
        assert!(report.all_passed(), "{:?}", report.assertions);
    }
}
