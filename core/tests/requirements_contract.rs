use kessetsu_core::ir::SIUnit;
use kessetsu_core::requirements::{
    MAX_REQUIREMENTS_BYTES, REQUIREMENTS_SCHEMA_VERSION, compile_requirements,
};

#[test]
fn assertion_only_source_becomes_a_versioned_hash_bound_requirement_set() {
    let source = b"// Evaluator-owned limits\nassert output_power(V(OUT),RL,2ms,10ms) > 1.9W\nassert thd(V(OUT),1kHz,2ms,10ms,hann) < 2%\n";
    let first = compile_requirements(source).expect("requirements should compile");
    let second = compile_requirements(source).expect("requirements should compile repeatedly");

    assert_eq!(first.schema_version, REQUIREMENTS_SCHEMA_VERSION);
    assert_eq!(first.sha256, second.sha256);
    assert!(first.sha256.starts_with("sha256:"));
    assert_eq!(first.sha256.len(), 71);
    assert_eq!(first.assertions.len(), 2);
    assert_eq!(first.assertions[0].threshold.unit, SIUnit::Watt);
    assert_eq!(first.assertions[1].threshold.unit, SIUnit::Percent);
}

#[test]
fn exact_bytes_define_requirement_identity() {
    let lf = compile_requirements(b"assert value(V(OUT)) > 1V\n").unwrap();
    let crlf = compile_requirements(b"assert value(V(OUT)) > 1V\r\n").unwrap();
    assert_ne!(lf.sha256, crlf.sha256);
}

#[test]
fn ac_requirements_preserve_typed_evaluator_owned_limits() {
    let source = b"assert gain_at(V(OUT),V(IN),1kHz) > 8.8\nassert lower_cutoff(V(OUT),V(IN),1kHz) < 110Hz\nassert upper_cutoff(V(OUT),V(IN),1kHz) > 8kHz\n";
    let set = compile_requirements(source).unwrap();
    assert_eq!(set.schema_version, "kessetsu.requirements.v1");
    assert_eq!(set.assertions[0].threshold.unit, SIUnit::Ratio);
    assert_eq!(set.assertions[1].threshold.unit, SIUnit::Hertz);
    assert_eq!(set.assertions[2].threshold.unit, SIUnit::Hertz);
    assert_eq!(set.assertions[0].threshold.value, 8.8);
    assert!(compile_requirements(b"assert gain_at(V(OUT),V(IN),1ms) > 8\n").is_err());
}

#[test]
fn empty_non_assertion_invalid_metric_and_oversize_inputs_fail_closed() {
    let empty = compile_requirements(b"// no requirements\n").unwrap_err();
    assert_eq!(empty.code, "KES-R002");

    let declaration = compile_requirements(b"resistor R1 1k\n").unwrap_err();
    assert_eq!(declaration.code, "KES-R001");
    assert_eq!(declaration.line, Some(1));
    assert_eq!(declaration.column, Some(1));

    let metric = compile_requirements(b"assert settle(V(OUT)) < 1V\n").unwrap_err();
    assert_eq!(metric.code, "KES-C006");
    assert_eq!(metric.line, Some(1));
    assert_eq!(metric.column, Some(8));

    let oversized = vec![b' '; MAX_REQUIREMENTS_BYTES + 1];
    assert_eq!(
        compile_requirements(&oversized).unwrap_err().code,
        "KES-R001"
    );
}
