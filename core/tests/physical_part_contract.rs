use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::ir::PHYSICAL_PART_SCHEMA_VERSION;

#[test]
fn physical_part_metadata_is_typed_separate_and_simulation_neutral() {
    let electrical = "net IN\nnet OUT\nresistor R1 1k\nconnect R1.p1 to IN\nconnect R1.p2 to OUT\n";
    let annotated = format!(
        "{electrical}part R1 manufacturer=\"Yageo\" mpn=\"MFR-25FBF52-1K\" footprint=\"Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P10.16mm_Horizontal\" pin_map=\"p1:1,p2:2\" note=\"bench prototype\"\n"
    );
    let baseline = compile_source(electrical, CompileOptions::default());
    let report = compile_source(&annotated, CompileOptions::default());
    assert!(!baseline.has_errors(), "{:?}", baseline.diagnostics);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(baseline.spice_netlist, report.spice_netlist);

    let manifest = &report.ir.expect("typed IR").physical_parts;
    assert_eq!(manifest.schema_version, PHYSICAL_PART_SCHEMA_VERSION);
    assert_eq!(manifest.assignments.len(), 1);
    let assignment = &manifest.assignments[0];
    assert_eq!(assignment.component, "R1");
    assert_eq!(assignment.manufacturer.as_deref(), Some("Yageo"));
    assert_eq!(assignment.mpn.as_deref(), Some("MFR-25FBF52-1K"));
    assert_eq!(assignment.pin_map["p1"], "1");
    assert_eq!(assignment.pin_map["p2"], "2");
    assert_eq!(assignment.note.as_deref(), Some("bench prototype"));
}

#[test]
fn module_part_assignments_follow_each_flattened_instance() {
    let source = "module Load(a,b) {\nresistor R 1k\npart R manufacturer=\"Yageo\" mpn=\"RC0603FR-071KL\" footprint=\"Resistor_SMD:R_0603_1608Metric\" pin_map=\"p1:1,p2:2\"\nconnect R.p1 to a\nconnect R.p2 to b\n}\nnet A\nnet B\nnet C\nuse Load FIRST\nuse Load SECOND\nconnect FIRST.a to A\nconnect FIRST.b, SECOND.a to B\nconnect SECOND.b to C\n";
    let report = compile_source(source, CompileOptions::default());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let assignments = &report.ir.expect("typed IR").physical_parts.assignments;
    assert_eq!(
        assignments
            .iter()
            .map(|assignment| assignment.component.as_str())
            .collect::<Vec<_>>(),
        ["FIRST_R", "SECOND_R"]
    );
}

#[test]
fn invalid_or_ambiguous_physical_assignments_fail_closed() {
    let cases = [
        (
            "resistor R1 1k\npart MISSING mpn=\"X\"\n",
            "not a declared component",
        ),
        (
            "resistor R1 1k\npart R1 vendor=\"X\"\n",
            "unknown physical-part field",
        ),
        (
            "resistor R1 1k\npart R1 pin_map=\"p1:1,p2:2\"\n",
            "requires an explicit footprint",
        ),
        (
            "resistor R1 1k\npart R1 footprint=\"Lib:Pkg\" pin_map=\"p1:1\"\n",
            "pin_map is incomplete",
        ),
        (
            "resistor R1 1k\npart R1 footprint=\"Lib:Pkg\" pin_map=\"p1:1,p2:1\"\n",
            "physical pad '1' is assigned more than once",
        ),
        (
            "resistor R1 1k\npart R1 mpn=\"A\"\npart R1 mpn=\"B\"\n",
            "assigned more than once",
        ),
        (
            "module M(a,b) {\nresistor R 1k\nconnect R.p1 to a\nconnect R.p2 to b\n}\nuse M X\npart X mpn=\"not-a-real-part\"\n",
            "module interfaces are virtual",
        ),
        (
            "source VIN 1V\npart VIN manufacturer=\"Example\" mpn=\"SUPPLY\"\n",
            "abstract stimuli",
        ),
    ];
    for (source, expected) in cases {
        let report = compile_source(source, CompileOptions::default());
        assert!(report.has_errors(), "source unexpectedly passed:\n{source}");
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "KES-C024")
            .unwrap_or_else(|| panic!("missing KES-C024 for:\n{source}"));
        assert!(
            diagnostic.message.contains(expected),
            "expected '{expected}', got '{}':\n{source}",
            diagnostic.message
        );
        assert!(
            diagnostic.line.is_some(),
            "missing source line for:\n{source}"
        );
    }
}
