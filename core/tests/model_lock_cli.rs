mod common;

use common::TestWorkspace;
use serde_json::Value;
use std::fs;

fn source() -> &'static str {
    "net GND\nnet IN\nnet OUT\nsource VIN 1V\nresistor R1 1k\ndiode D1 1N4148\nconnect VIN.plus, R1.p1 to IN\nconnect VIN.minus, D1.p2 to GND\nconnect R1.p2, D1.p1 to OUT\nsimulate op\nassert value(V(OUT)) > 0V\n"
}

fn report(output: &std::process::Output) -> Value {
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn different_lock_requires_force_before_any_netlist_is_written() {
    let workspace = TestWorkspace::new("lock-overwrite");
    workspace.write("circuit.kess", source());
    let lock = workspace.write("kessetsu.lock", "existing evaluator-owned contents\n");
    for command in ["compile", "simulate", "test"] {
        let output = workspace.run_cli(&[command, "circuit.kess", "--format", "json"]);
        assert_eq!(output.status.code(), Some(2), "{command}");
        assert_eq!(report(&output)["diagnostics"][0]["code"], "KES-I003");
        assert!(!workspace.path().join("circuit.spice").exists());
        assert_eq!(
            fs::read_to_string(&lock).unwrap(),
            "existing evaluator-owned contents\n"
        );
    }
    let forced = workspace.run_cli(&["compile", "circuit.kess", "--force", "--format", "json"]);
    assert_eq!(forced.status.code(), Some(0));
    let content = fs::read_to_string(&lock).unwrap();
    assert!(content.contains("kessetsu.lock.v3"));
    assert!(workspace.path().join("circuit.spice").is_file());
    assert!(
        report(&forced)["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["kind"] == "model_lock")
    );
}

#[test]
fn identical_lock_is_reused_without_rewriting_or_requiring_force() {
    let workspace = TestWorkspace::new("lock-reuse");
    workspace.write("first.kess", source());
    workspace.write("second.kess", source());
    assert_eq!(
        workspace.run_cli(&["compile", "first.kess"]).status.code(),
        Some(0)
    );
    let lock = workspace.path().join("kessetsu.lock");
    let original = fs::read(&lock).unwrap();
    let modified = fs::metadata(&lock).unwrap().modified().unwrap();
    // A read-only matching lock must work too: reuse is not an overwrite.
    let original_permissions = fs::metadata(&lock).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&lock, permissions).unwrap();
    for args in [
        vec!["compile", "second.kess", "--format", "json"],
        vec!["compile", "second.kess", "--force", "--format", "json"],
    ] {
        let output = workspace.run_cli(&args);
        assert_eq!(output.status.code(), Some(0), "{:?}", report(&output));
        assert_eq!(fs::read(&lock).unwrap(), original);
        assert_eq!(fs::metadata(&lock).unwrap().modified().unwrap(), modified);
    }
    // Restore the original permissions for normal temporary-workspace cleanup.
    fs::set_permissions(&lock, original_permissions).unwrap();
}

#[test]
fn lock_cannot_overwrite_source_even_with_force() {
    let workspace = TestWorkspace::new("lock-source");
    workspace.write("kessetsu.lock", source());
    workspace.write("kessetsu.spice", "previous netlist\n");
    for force in [false, true] {
        let mut args = vec![
            "compile",
            "kessetsu.lock",
            "--output",
            "new.spice",
            "--format",
            "json",
        ];
        if force {
            args.push("--force");
        }
        let output = workspace.run_cli(&args);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(report(&output)["diagnostics"][0]["code"], "KES-I002");
        assert_eq!(
            fs::read_to_string(workspace.path().join("kessetsu.lock")).unwrap(),
            source()
        );
        assert!(!workspace.path().join("new.spice").exists());
        assert_eq!(
            fs::read_to_string(workspace.path().join("kessetsu.spice")).unwrap(),
            "previous netlist\n"
        );
    }
}

#[test]
fn lock_and_netlist_destinations_cannot_be_the_same() {
    let workspace = TestWorkspace::new("lock-netlist");
    workspace.write("circuit.kess", source());
    for force in [false, true] {
        let mut args = vec![
            "compile",
            "circuit.kess",
            "--output",
            "kessetsu.lock",
            "--format",
            "json",
        ];
        if force {
            args.push("--force");
        }
        let output = workspace.run_cli(&args);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(report(&output)["diagnostics"][0]["code"], "KES-I002");
        assert!(!workspace.path().join("kessetsu.lock").exists());
    }
}

#[test]
fn stdin_stays_in_memory_but_explicit_output_obeys_lock_protection() {
    let workspace = TestWorkspace::new("lock-stdin");
    workspace.write("kessetsu.lock", "keep\n");
    let in_memory = workspace.run_cli_with_stdin(&["compile", "-", "--format", "json"], source());
    assert_eq!(in_memory.status.code(), Some(0));
    assert_eq!(report(&in_memory)["artifacts"], serde_json::json!([]));
    let explicit = workspace.run_cli_with_stdin(
        &[
            "compile",
            "-",
            "--output",
            "output.spice",
            "--format",
            "json",
        ],
        source(),
    );
    assert_eq!(explicit.status.code(), Some(2));
    assert!(!workspace.path().join("output.spice").exists());
    assert_eq!(
        fs::read_to_string(workspace.path().join("kessetsu.lock")).unwrap(),
        "keep\n"
    );
}
