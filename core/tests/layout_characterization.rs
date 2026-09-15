use kessetsu_core::compiler::{CompileOptions, compile_source};
use std::collections::BTreeSet;

struct Fixture {
    name: &'static str,
    source: &'static str,
    component_count: usize,
}

const CORPUS: &[Fixture] = &[
    Fixture {
        name: "minimal",
        source: include_str!("fixtures/valid/minimal.kess"),
        component_count: 2,
    },
    Fixture {
        name: "rc_filter",
        source: include_str!("fixtures/benchmarks/rc_filter.kess"),
        component_count: 3,
    },
    Fixture {
        name: "wheatstone",
        source: include_str!("../../examples/wheatstone_bridge.kess"),
        component_count: 6,
    },
    Fixture {
        name: "gain_stage",
        source: include_str!("fixtures/benchmarks/gain_stage.kess"),
        component_count: 7,
    },
    Fixture {
        name: "high_fanout",
        source: include_str!("fixtures/schematic/high_fanout.kess"),
        component_count: 9,
    },
    Fixture {
        name: "power_amplifier",
        source: include_str!("fixtures/benchmarks/power_amplifier.kess"),
        component_count: 11,
    },
];

#[test]
fn legacy_layout_corpus_keeps_component_and_net_coverage_visible() {
    for fixture in CORPUS {
        let report = compile_source(fixture.source, CompileOptions::all_outputs());
        assert!(
            !report.has_errors(),
            "{} diagnostics: {:?}",
            fixture.name,
            report.diagnostics
        );

        let graph = report.graph.expect("valid fixture should produce graph");
        let layout = report.layout.expect("layout was requested");
        assert_eq!(
            layout.components.len(),
            fixture.component_count,
            "{} component coverage drifted",
            fixture.name
        );

        let expected_ids: BTreeSet<_> = graph
            .nets
            .iter()
            .filter(|net| net.pins.len() >= 2)
            .map(|net| net.id)
            .collect();
        let rendered_ids: BTreeSet<_> = layout.wires.iter().map(|wire| wire.net_id).collect();
        assert_eq!(
            rendered_ids, expected_ids,
            "{} lost or invented a connected net",
            fixture.name
        );
        assert!(
            layout.wires.iter().all(|wire| wire.points.len() >= 2),
            "{} emitted an empty wire",
            fixture.name
        );
    }
}

#[test]
fn characterization_exposes_legacy_layout_serialization_instability() {
    let source = include_str!("fixtures/benchmarks/power_amplifier.kess");
    let first = compile_source(source, CompileOptions::all_outputs())
        .layout
        .expect("layout was requested");
    let second = compile_source(source, CompileOptions::all_outputs())
        .layout
        .expect("layout was requested");

    // HashMap-backed component serialization is not itself a stable contract.
    // Compare canonical component IDs here; Phase 4.1 replaces this shape with
    // an ordered, versioned Schematic IR and byte-stability golden tests.
    let first_ids: BTreeSet<_> = first.components.keys().collect();
    let second_ids: BTreeSet<_> = second.components.keys().collect();
    assert_eq!(first_ids, second_ids);
}
