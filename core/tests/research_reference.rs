//! Developer reproduction, not physical-device measurements or independent lab validation.
//! Reference: https://github.com/imr/ngspice/blob/master/examples/memristor/memristor.sp
//! Model: https://arxiv.org/abs/1204.2600 ; notice in examples/models/MEMRISTOR_NOTICE.md.
use kessetsu_core::compiler::{CompileOptions, compile_source_with_resources};
use kessetsu_core::experiment::hash_bytes;
use kessetsu_core::ir::{Analysis, Quantity, SIUnit};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::simulation::{
    CancellationToken, Dataset, NativeSimulationContext, NgspiceRunner, RealSeriesDataset,
    SimulationRequest,
};

const MODEL: &str = include_str!("../../examples/models/memristor.lib");

fn interpolate(axis: &[f64], values: &[f64], x: f64) -> f64 {
    let right = axis.partition_point(|t| *t < x);
    if right == 0 {
        return values[0];
    }
    if right == axis.len() {
        return *values.last().unwrap();
    }
    let left = right - 1;
    values[left] + (values[right] - values[left]) * (x - axis[left]) / (axis[right] - axis[left])
}

fn transient(request: SimulationRequest, resources: ExternalModelResources) -> RealSeriesDataset {
    let result = NgspiceRunner::discover()
        .run_with_context(
            &request,
            &NativeSimulationContext {
                resources,
                ..Default::default()
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert!(result.succeeded(), "{:?}", result.errors);
    let Dataset::Transient(data) = result.datasets.into_iter().next().unwrap().data else {
        panic!("transient missing")
    };
    data
}

#[test]
fn threshold_reference_matches_ir_generated_trajectories_and_timestep_refinement() {
    assert_eq!(
        hash_bytes(MODEL.replace("\r\n", "\n").as_bytes()),
        "sha256:cd4bac38581fb00c1b5667e9d5ec440cf954efb5e26105ae2775853cc90301f6"
    );
    // Upstream drives V1 from ground to the device: retain that negative node polarity.
    // Repeat its 100/110/140 MHz cases independently with uic and the same artificial model.
    for ratio in [1.0, 1.1, 1.4] {
        let frequency = 1e8 * ratio;
        let stop = 1.0 / frequency;
        let step = stop / 100.0;
        let analysis = Analysis::Transient {
            step: Quantity {
                value: step,
                unit: SIUnit::Second,
            },
            stop: Quantity {
                value: stop,
                unit: SIUnit::Second,
            },
            use_initial_conditions: true,
        };
        let reference_deck = format!(
            "Published threshold reference (negative node polarity)\nV1 0 top SIN(0 3 {frequency})\nXmem top 0 memristor\n{MODEL}\n.control\nset wr_singlescale\nset wr_vecnames\nset numdgt=17\ntran {step} {stop} uic\nwrdata kessetsu-analysis-000-tran.data all\n.endc\n.end\n"
        );
        let reference = transient(
            SimulationRequest::new(reference_deck, vec![analysis.clone()]),
            Default::default(),
        );
        let source = include_str!("../../examples/external_memristor.kess")
            .replace(
                "connect VIN.minus, XM.p2 to GND",
                "connect VIN.plus, XM.p2 to GND",
            )
            .replace(
                "connect VIN.plus, VSENSE.plus to IN",
                "connect VIN.minus, VSENSE.plus to IN",
            )
            .replace("100MHz", &format!("{frequency}Hz"))
            .replace("100ps 10ns", &format!("{step}s {stop}s"));
        let resources = ExternalModelResources::from([(
            "models/memristor.lib".into(),
            MODEL.as_bytes().to_vec(),
        )]);
        let report = compile_source_with_resources(&source, CompileOptions::default(), &resources);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let generated = transient(
            SimulationRequest::new(report.spice_netlist.unwrap(), report.ir.unwrap().analyses),
            resources.clone(),
        );
        for (reference_signal, candidate_signal, absolute) in
            [("top", "top", 1e-6), ("v1#branch", "v_vsense#branch", 1e-9)]
        {
            let expected = reference
                .signals
                .get(reference_signal)
                .unwrap_or_else(|| panic!("{reference_signal}: {:?}", reference.signals.keys()));
            let actual = generated
                .signals
                .get(candidate_signal)
                .unwrap_or_else(|| panic!("{candidate_signal}: {:?}", generated.signals.keys()));
            let peak = expected.iter().map(|v| v.abs()).fold(0.0, f64::max);
            let mut maximum: f64 = 0.0;
            for (&t, &value) in reference.axis.values.iter().zip(expected) {
                if t >= generated.axis.values[0] && t <= *generated.axis.values.last().unwrap() {
                    maximum =
                        maximum.max((interpolate(&generated.axis.values, actual, t) - value).abs());
                }
            }
            assert!(
                maximum <= absolute + peak * 0.003,
                "{ratio}: {reference_signal} residual {maximum}, peak {peak}"
            );
        }
        let fine_source = source.replace(
            &format!("simulate tran {step}s"),
            &format!("simulate tran {}s", step / 2.0),
        );
        let fine =
            compile_source_with_resources(&fine_source, CompileOptions::default(), &resources);
        assert!(!fine.has_errors(), "{:?}", fine.diagnostics);
        let refined = transient(
            SimulationRequest::new(fine.spice_netlist.unwrap(), fine.ir.unwrap().analyses),
            resources,
        );
        let currents = &generated.signals["v_vsense#branch"];
        let resistances = generated.signals["top"]
            .iter()
            .zip(currents)
            .filter(|(v, i)| v.abs() > 1e-6 && i.abs() > 1e-12)
            .map(|(v, i)| v / i)
            .collect::<Vec<_>>();
        assert!(
            (resistances[0] - 7000.0).abs() < 70.0,
            "uic starting state missing"
        );
        assert!(
            resistances.iter().all(|r| (950.0..=10500.0).contains(r)),
            "model state outside numerical boundary tolerance"
        );
        let peak = currents.iter().map(|v| v.abs()).fold(0.0, f64::max);
        let delta = generated
            .axis
            .values
            .iter()
            .zip(currents)
            .filter(|(t, _)| {
                **t >= refined.axis.values[0] && **t <= *refined.axis.values.last().unwrap()
            })
            .map(|(&t, &i)| {
                (i - interpolate(&refined.axis.values, &refined.signals["v_vsense#branch"], t))
                    .abs()
            })
            .fold(0.0, f64::max);
        assert!(
            delta / peak < 0.03,
            "{ratio}: timestep sensitivity {}",
            delta / peak
        );
        println!(
            "{frequency} Hz: peak current {peak} A, half-step max residual {:.3}% of peak",
            delta / peak * 100.0
        );
    }
}
