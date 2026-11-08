use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use kessetsu_core::compiler::{CompileOptions, CompileReport, DiagnosticStage};
use kessetsu_core::measurement::MEASUREMENT_SCHEMA_VERSION;
use kessetsu_core::models::{ExternalModelResources, validate_external_resource_reference};
use kessetsu_core::simulation::{
    CancellationToken, NativeSimulationContext, NgspiceRunner, SIMULATION_SCHEMA_VERSION,
    SimulationRequest, SimulationResult,
};
use serde_json::Value;

use super::{
    CLI_SCHEMA_VERSION, DataAction, DataCommand, FitAction, FitCommand, Format, Include,
    StudyAction, StudyCommand, StudyTarget, build_json_output, diagnostic, emit,
    load_external_model_resources, plain_json_artifact,
};

pub(crate) fn emit_study_error(
    format: &Format,
    includes: &BTreeSet<Include>,
    message: impl Into<String>,
) -> i32 {
    emit(
        format,
        "study",
        includes,
        "error",
        CompileReport::failure(diagnostic("KES-X001", DiagnosticStage::Cli, message)),
        None,
        None,
        None,
    );
    2
}

pub(crate) fn emit_study(
    format: &Format,
    includes: &BTreeSet<Include>,
    status: &str,
    body: Value,
    artifact: Option<&Path>,
) {
    let report = kessetsu_core::compiler::compile_source("", CompileOptions::default());
    let mut output = build_json_output("study", includes, status, &report, None, None, None, None);
    output.domain_versions.experiment = Some(kessetsu_core::experiment::EXPERIMENT_SCHEMA);
    output.domain_versions.measurement = Some(MEASUREMENT_SCHEMA_VERSION);
    output.study = Some(body.clone());
    if let Some(path) = artifact {
        output.artifacts.push(plain_json_artifact(
            "study",
            path.to_string_lossy().into_owned(),
        ));
    }
    if *format == Format::Json {
        println!("{}", serde_json::to_string(&output).unwrap());
    } else {
        println!(
            "Study {status}: {}",
            serde_json::to_string_pretty(&body).unwrap()
        );
        if let Some(path) = artifact {
            println!("Saved: {}", path.display());
        }
    }
}

pub(crate) fn study_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > limit {
        return Err("Study file exceeds its size limit".into());
    }
    fs::read(path).map_err(|e| e.to_string())
}

pub(crate) fn study_preflight(input: &Path, output: &Path, force: bool) -> Result<(), String> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let resolved = fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(output.file_name().ok_or("Output must name a file")?);
    let input = fs::canonicalize(input).map_err(|e| e.to_string())?;
    if resolved == input
        || (output.exists() && fs::canonicalize(output).map_err(|e| e.to_string())? == input)
    {
        return Err("Study output cannot overwrite its input".into());
    }
    if output.exists() && !force {
        return Err("Output already exists; use --force or --resume as appropriate".into());
    }
    Ok(())
}

pub(crate) fn study_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let pending = path.with_file_name(format!(".kess-study-{}.tmp", process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&pending, path)
        .map_err(|e| format!("Could not save checkpoint; temporary data is preserved: {e}"))
}

pub(crate) fn study_write_package(
    output: &Path,
    package: &kessetsu_core::research_package::ResearchPackage,
) -> Result<PathBuf, String> {
    if output.exists() {
        return Err("Package output already exists; choose a new directory".into());
    }
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let name = output
        .file_name()
        .ok_or("Package output must name a directory")?;
    let target = parent.join(name);
    let pending = parent.join(format!(".kess-package-{}.tmp", process::id()));
    if pending.exists() {
        return Err(format!(
            "Temporary package directory already exists: {}",
            pending.display()
        ));
    }
    fs::create_dir(&pending).map_err(|e| e.to_string())?;
    let write = || -> Result<(), String> {
        for (relative, bytes) in &package.files {
            validate_external_resource_reference(relative)?;
            let destination = pending.join(relative);
            if let Some(directory) = destination.parent() {
                fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            }
            fs::write(destination, bytes).map_err(|e| e.to_string())?;
        }
        fs::write(
            pending.join("manifest.json"),
            serde_json::to_vec_pretty(&package.manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    };
    if let Err(error) = write() {
        let cleanup = fs::remove_dir_all(&pending);
        return Err(match cleanup {
            Ok(()) => error,
            Err(cleanup) => format!(
                "{error}; incomplete package remains at '{}': {cleanup}",
                pending.display()
            ),
        });
    }
    fs::rename(&pending, &target).map_err(|error| {
        format!(
            "Could not publish package; complete temporary directory remains at '{}': {error}",
            pending.display()
        )
    })?;
    Ok(target.join("manifest.json"))
}

pub(crate) fn study_resources(
    file: &Path,
    spec: &kessetsu_core::experiment::ExperimentSpec,
) -> Result<ExternalModelResources, String> {
    let mut resources =
        load_external_model_resources(file, false, &spec.source).map_err(|e| e.message)?;
    for revision in &spec.revisions {
        if let Some(source) = &revision.source {
            resources
                .extend(load_external_model_resources(file, false, source).map_err(|e| e.message)?);
        }
    }
    Ok(resources)
}

pub(crate) fn run_study(
    command: &StudyCommand,
    format: &Format,
    includes: &BTreeSet<Include>,
) -> i32 {
    use kessetsu_core::experiment::*;
    let execute = || -> Result<i32, String> {
        match &command.action {
            StudyAction::Create {
                file,
                output,
                sweep,
                force,
            } => {
                study_preflight(file, output, *force)?;
                let source = String::from_utf8(study_read(file, MAX_SPEC_BYTES as u64)?)
                    .map_err(|e| e.to_string())?;
                let mut axes = Vec::new();
                for axis in sweep {
                    let (parameter, values) = axis
                        .split_once('=')
                        .ok_or("--sweep requires NAME=value,value")?;
                    axes.push(Axis {
                        parameter: parameter.into(),
                        values: Values::List {
                            values: values.split(',').map(str::to_owned).collect(),
                        },
                    });
                }
                let spec = ExperimentSpec {
                    schema_version: EXPERIMENT_SCHEMA.into(),
                    name: file
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    source,
                    requirements: None,
                    axes,
                    revisions: Vec::new(),
                    tolerances: None,
                    temperatures_c: vec![27.0],
                    timeout_ms: 30_000,
                    measurements: Vec::new(),
                    objective: None,
                };
                let resources = study_resources(file, &spec)?;
                let plan = plan_experiment(spec, &resources)?;
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&plan.spec).map_err(|e| e.to_string())?,
                )?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::json!({"identity":plan.identity,"cases":plan.cases.len()}),
                    Some(output),
                );
                Ok(0)
            }
            StudyAction::Plan { file } => {
                let spec = decode_spec(&study_read(file, MAX_SPEC_BYTES as u64)?)?;
                let resources = study_resources(file, &spec)?;
                let plan = plan_experiment(spec, &resources)?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::json!({"identity":plan.identity,"requirements_sha256":plan.requirements_sha256,"cases":plan.cases}),
                    None,
                );
                Ok(0)
            }
            StudyAction::Run {
                file,
                output,
                resume,
                force,
            } => {
                if *resume && *force {
                    return Err("Choose --resume or --force, not both".into());
                }
                study_preflight(file, output, *resume || *force)?;
                let spec = decode_spec(&study_read(file, MAX_SPEC_BYTES as u64)?)?;
                let resources = study_resources(file, &spec)?;
                let output_absolute = fs::canonicalize(
                    output
                        .parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new(".")),
                )
                .map_err(|e| e.to_string())?
                .join(output.file_name().unwrap());
                let model_root = fs::canonicalize(
                    file.parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new(".")),
                )
                .map_err(|e| e.to_string())?;
                for reference in resources.keys() {
                    let model =
                        fs::canonicalize(model_root.join(reference)).map_err(|e| e.to_string())?;
                    if model == output_absolute
                        || (output.exists()
                            && fs::canonicalize(output).map_err(|e| e.to_string())? == model)
                    {
                        return Err("Results cannot overwrite a bound model file".into());
                    }
                }
                let plan = plan_experiment(spec, &resources)?;
                let runner = NgspiceRunner::discover();
                let simulator = runner.info().map_err(|e| e.to_string())?;
                let solver_fingerprint = hash_bytes(
                    format!(
                        "{}:{}",
                        hash_bytes(&fs::read(&simulator.executable).map_err(|e| e.to_string())?),
                        hash_bytes(
                            &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
                                .map_err(|e| e.to_string())?
                        )
                    )
                    .as_bytes(),
                );
                let mut results = if *resume {
                    let previous: ExperimentResults =
                        serde_json::from_slice(&study_read(output, MAX_RESULT_BYTES as u64)?)
                            .map_err(|e| e.to_string())?;
                    validate_results(&previous, &plan, &simulator, &solver_fingerprint)?;
                    previous
                } else {
                    new_results(plan, simulator, solver_fingerprint)
                };
                let cancellation = CancellationToken::new();
                let signal = cancellation.clone();
                ctrlc::set_handler(move || signal.cancel()).map_err(|e| e.to_string())?;
                let checkpoint = |results: &ExperimentResults| -> Result<(), String> {
                    let bytes = serde_json::to_vec(results).map_err(|e| e.to_string())?;
                    if bytes.len() > MAX_RESULT_BYTES {
                        return Err("Results exceed 64 MiB; previous checkpoint is preserved. Narrow the study.".into());
                    }
                    study_write(output, &bytes)
                };
                summarize(&mut results);
                checkpoint(&results)?;
                for index in 0..results.plan.cases.len() {
                    if cancellation.is_cancelled() {
                        break;
                    }
                    if results.cases[index].status.reusable() {
                        continue;
                    }
                    let case = &results.plan.cases[index];
                    let row = match compile_case(&results.plan, case, &resources) {
                        Err(error) => failed_case(&case.id, CaseStatus::Error, error),
                        Ok(report) => {
                            let circuit = report.ir.as_ref().unwrap();
                            let mut context = NativeSimulationContext::default();
                            for model in &circuit.model_manifest.models {
                                if let Some(external) = &model.external {
                                    if external.simulator
                                        == kessetsu_core::ir::SimulatorCompatibility::NgspicePs
                                    {
                                        context.compatibility = external.simulator;
                                    }
                                    if let Some(bytes) = resources.get(&external.resource) {
                                        context
                                            .resources
                                            .insert(external.resource.clone(), bytes.clone());
                                    }
                                }
                            }
                            let spice = temperature_netlist(
                                report.spice_netlist.as_deref().ok_or("Missing SPICE")?,
                                case.temperature_c,
                            )?;
                            let mut request =
                                SimulationRequest::new(spice, circuit.analyses.clone());
                            request.timeout_ms = results.plan.spec.timeout_ms;
                            match runner.run_with_context(&request, &context, &cancellation) {
                                Ok(simulation) => {
                                    evaluate_case(&results.plan, case, &report, simulation)
                                }
                                Err(error) => {
                                    failed_case(&case.id, CaseStatus::Error, error.to_string())
                                }
                            }
                        }
                    };
                    eprintln!(
                        "Study {}/{}: {} {:?}",
                        index + 1,
                        results.plan.cases.len(),
                        case.id,
                        row.status
                    );
                    results.cases[index] = row;
                    summarize(&mut results);
                    checkpoint(&results)?;
                }
                let exit = if cancellation.is_cancelled() {
                    130
                } else if results.summary.errors > 0 {
                    3
                } else if results.summary.failed > 0 {
                    1
                } else {
                    0
                };
                let body = if includes.contains(&Include::Datasets) {
                    serde_json::to_value(&results).map_err(|e| e.to_string())?
                } else {
                    serde_json::json!({"schema_version":RESULTS_SCHEMA,"identity":results.identity,"summary":results.summary,"cases":results.cases.iter().map(|r| serde_json::json!({"case_id":r.case_id,"status":r.status,"measurements":r.measurements,"assertions":r.assertions,"errors":r.errors})).collect::<Vec<_>>()})
                };
                emit_study(
                    format,
                    includes,
                    if exit == 0 {
                        "success"
                    } else if exit == 130 {
                        "cancelled"
                    } else {
                        "fail"
                    },
                    body,
                    Some(output),
                );
                Ok(exit)
            }
            StudyAction::Export {
                file,
                output,
                target,
                signal,
                analysis,
                cases,
                force,
            } => {
                study_preflight(file, output, *force)?;
                let mut results: ExperimentResults =
                    serde_json::from_slice(&study_read(file, MAX_RESULT_BYTES as u64)?)
                        .map_err(|e| e.to_string())?;
                validate_results(
                    &results,
                    &results.plan,
                    &results.simulator,
                    &results.solver_fingerprint,
                )?;
                summarize(&mut results);
                let content = match target {
                    StudyTarget::Json => {
                        serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?
                    }
                    StudyTarget::Csv => results_csv(&results),
                    StudyTarget::DataCsv => datasets_csv(&results),
                    StudyTarget::Svg => plot_svg(&results, *analysis, signal, cases)?,
                    StudyTarget::Html => report_html(&results),
                };
                study_write(output, content.as_bytes())?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::json!({"identity":results.identity,"summary":results.summary}),
                    Some(output),
                );
                Ok(0)
            }
            StudyAction::Package {
                file,
                results,
                output,
                signal,
                analysis,
                cases,
            } => {
                let spec = decode_spec(&study_read(file, MAX_SPEC_BYTES as u64)?)?;
                let resources = study_resources(file, &spec)?;
                let completed: ExperimentResults =
                    serde_json::from_slice(&study_read(results, MAX_RESULT_BYTES as u64)?)
                        .map_err(|e| e.to_string())?;
                if serde_json::to_value(&spec).map_err(|e| e.to_string())?
                    != serde_json::to_value(&completed.plan.spec).map_err(|e| e.to_string())?
                {
                    return Err("Package specification does not match the completed results".into());
                }
                let package = kessetsu_core::research_package::build_research_package(
                    &completed, &resources, *analysis, signal, cases,
                )?;
                let manifest_path = study_write_package(output, &package)?;
                emit_study(
                    format,
                    includes,
                    "success",
                    serde_json::to_value(&package.manifest).map_err(|e| e.to_string())?,
                    Some(&manifest_path),
                );
                Ok(0)
            }
        }
    };
    match execute() {
        Ok(exit) => exit,
        Err(error) => emit_study_error(format, includes, error),
    }
}

pub(crate) fn run_data(
    command: &DataCommand,
    format: &Format,
    includes: &BTreeSet<Include>,
) -> i32 {
    use kessetsu_core::research_data::*;
    let read_text = |path: &Path, limit: usize| -> Result<String, String> {
        String::from_utf8(study_read(path, limit as u64)?).map_err(|_| "Input must be UTF-8".into())
    };
    let execute = || -> Result<(Value, Option<&Path>), String> {
        match &command.action {
            DataAction::Preview {
                file,
                delimiter,
                decimal,
                no_header,
                skip_records,
            } => {
                let dialect = CsvDialect {
                    delimiter: match delimiter.as_str() {
                        "semicolon" => Delimiter::Semicolon,
                        "tab" => Delimiter::Tab,
                        _ => Delimiter::Comma,
                    },
                    decimal: if decimal == "comma" {
                        Decimal::Comma
                    } else {
                        Decimal::Dot
                    },
                    header: !no_header,
                    preamble_records: *skip_records,
                };
                Ok((
                    serde_json::to_value(preview_csv(&read_text(file, MAX_CSV_BYTES)?, &dialect)?)
                        .map_err(|e| e.to_string())?,
                    None,
                ))
            }
            DataAction::Import {
                file,
                mapping,
                output,
                force,
            } => {
                for input in [file, mapping] {
                    study_preflight(input, output, *force)?;
                }
                let spec: ImportSpec = serde_json::from_str(&read_text(mapping, 64 * 1024)?)
                    .map_err(|e| e.to_string())?;
                let result = import_csv(&read_text(file, MAX_CSV_BYTES)?, spec)?;
                let mut body = serde_json::json!({ "schema_version": result.schema_version, "identity": result.identity, "raw_sha256": result.raw_sha256, "rows": result.source_records.len(), "skipped": result.skipped.len(), "origin": result.spec.metadata.origin, "axis": {"name": result.axis.name, "unit": result.axis.unit}, "signals": result.signals.iter().map(|c| serde_json::json!({"name":c.name,"unit":c.unit})).collect::<Vec<_>>() });
                if includes.contains(&Include::Datasets) {
                    body["axis"]["values"] =
                        serde_json::to_value(&result.axis.values).map_err(|e| e.to_string())?;
                    body["signals"] =
                        serde_json::to_value(&result.signals).map_err(|e| e.to_string())?;
                }
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
                )?;
                Ok((body, Some(output.as_path())))
            }
            DataAction::FromSimulation {
                file,
                mapping,
                output,
                force,
            } => {
                for input in [file, mapping] {
                    study_preflight(input, output, *force)?;
                }
                let input: Value = serde_json::from_str(&read_text(file, 64 * 1024 * 1024)?)
                    .map_err(|error| format!("Invalid simulation JSON: {error}"))?;
                let simulation_value = if input.get("schema_version").and_then(Value::as_str)
                    == Some(SIMULATION_SCHEMA_VERSION)
                {
                    input
                } else if input.get("schema_version").and_then(Value::as_str)
                    == Some(CLI_SCHEMA_VERSION)
                {
                    input
                        .get("debug")
                        .and_then(|debug| debug.get("simulation"))
                        .cloned()
                        .ok_or(
                            "CLI input does not contain debug.simulation; rerun with --include simulation",
                        )?
                } else {
                    return Err(
                        "Expected kessetsu.simulation.v1 or kessetsu.cli.v1 simulation JSON".into(),
                    );
                };
                let simulation: SimulationResult = serde_json::from_value(simulation_value)
                    .map_err(|error| format!("Invalid typed simulation result: {error}"))?;
                let spec: SimulationImportSpec =
                    serde_json::from_str(&read_text(mapping, 64 * 1024)?)
                        .map_err(|error| error.to_string())?;
                let result = import_simulation_data(&simulation, spec)?;
                let mut body = serde_json::json!({ "schema_version": result.schema_version, "identity": result.identity, "raw_sha256": result.raw_sha256, "rows": result.source_records.len(), "skipped": result.skipped.len(), "origin": result.spec.metadata.origin, "axis": {"name": result.axis.name, "unit": result.axis.unit}, "signals": result.signals.iter().map(|column| serde_json::json!({"name":column.name,"unit":column.unit})).collect::<Vec<_>>() });
                if includes.contains(&Include::Datasets) {
                    body = serde_json::to_value(&result).map_err(|error| error.to_string())?;
                }
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?,
                )?;
                Ok((body, Some(output.as_path())))
            }
            DataAction::Compare {
                file,
                reference,
                mapping,
                output,
                force,
            } => {
                for input in [file, reference, mapping] {
                    study_preflight(input, output, *force)?;
                }
                let data: ResearchData = serde_json::from_str(&read_text(file, 64 * 1024 * 1024)?)
                    .map_err(|e| e.to_string())?;
                let reference: ResearchData =
                    serde_json::from_str(&read_text(reference, 64 * 1024 * 1024)?)
                        .map_err(|e| e.to_string())?;
                let spec: ComparisonSpec = serde_json::from_str(&read_text(mapping, 64 * 1024)?)
                    .map_err(|e| e.to_string())?;
                let result = compare_data(&data, &reference, spec)?;
                let body = if includes.contains(&Include::Datasets) {
                    serde_json::to_value(&result).map_err(|e| e.to_string())?
                } else {
                    serde_json::json!({ "schema_version": result.schema_version, "identity": result.identity, "data_identity": result.data_identity, "reference_identity": result.reference_identity, "axis_unit": result.axis_unit, "signals": result.signals.iter().map(|c| serde_json::json!({"mapping":c.mapping,"unit":c.unit,"metrics":c.metrics})).collect::<Vec<_>>() })
                };
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
                )?;
                Ok((body, Some(output.as_path())))
            }
        }
    };
    match execute() {
        Ok((body, artifact)) => {
            let report = kessetsu_core::compiler::compile_source("", CompileOptions::default());
            let mut output = build_json_output(
                "data",
                includes,
                "completed",
                &report,
                None,
                None,
                None,
                None,
            );
            output.domain_versions.research_data = Some(DATA_SCHEMA);
            output.data = Some(body.clone());
            if let Some(path) = artifact {
                output.artifacts.push(plain_json_artifact(
                    "research_data",
                    path.to_string_lossy().into_owned(),
                ));
            }
            if *format == Format::Json {
                println!("{}", serde_json::to_string(&output).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&body).unwrap());
                if let Some(path) = artifact {
                    println!("Saved: {}", path.display());
                }
            }
            0
        }
        Err(error) => {
            emit(
                format,
                "data",
                includes,
                "error",
                CompileReport::failure(diagnostic("KES-R001", DiagnosticStage::Cli, error)),
                None,
                None,
                None,
            );
            2
        }
    }
}

pub(crate) fn run_fit(command: &FitCommand, format: &Format, includes: &BTreeSet<Include>) -> i32 {
    use kessetsu_core::fitting::*;
    use kessetsu_core::research_data::ResearchData;
    let read_text = |path: &Path, limit: usize| -> Result<String, String> {
        String::from_utf8(study_read(path, limit as u64)?).map_err(|_| "Input must be UTF-8".into())
    };
    let execute = || -> Result<(Value, &Path), String> {
        match &command.action {
            FitAction::Evaluate {
                file,
                spec,
                datasets,
                output,
                force,
            } => {
                for input in [file, spec] {
                    study_preflight(input, output, *force)?;
                }
                if datasets.len() > 32 {
                    return Err("At most 32 observed datasets can be bound".into());
                }
                let mut bound = BTreeMap::new();
                for binding in datasets {
                    let (name, path) = binding
                        .split_once('=')
                        .ok_or("--data requires NAME=path.kessdata.json")?;
                    if name.trim().is_empty() || name.len() > 128 {
                        return Err("Dataset binding names must be nonempty and bounded".into());
                    }
                    let path = PathBuf::from(path);
                    study_preflight(&path, output, *force)?;
                    let data: ResearchData =
                        serde_json::from_str(&read_text(&path, MAX_FIT_RESULT_BYTES)?)
                            .map_err(|error| format!("Invalid dataset '{name}': {error}"))?;
                    if bound.insert(name.to_string(), data).is_some() {
                        return Err(format!("Duplicate dataset binding '{name}'"));
                    }
                }
                let experiment: kessetsu_core::experiment::ExperimentResults =
                    serde_json::from_str(&read_text(
                        file,
                        kessetsu_core::experiment::MAX_RESULT_BYTES,
                    )?)
                    .map_err(|error| format!("Invalid experiment results: {error}"))?;
                let spec = decode_fit_spec(&study_read(spec, MAX_FIT_SPEC_BYTES as u64)?)?;
                let result = evaluate_fit(&experiment, &bound, spec)?;
                study_write(
                    output,
                    &serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?,
                )?;
                let body = if includes.contains(&Include::Datasets) {
                    serde_json::to_value(&result).map_err(|error| error.to_string())?
                } else {
                    serde_json::json!({
                        "schema_version": result.schema_version,
                        "identity": result.identity,
                        "experiment_identity": result.experiment_identity,
                        "selected_candidate": result.selected_candidate,
                        "near_equivalent_candidates": result.near_equivalent_candidates,
                        "warnings": result.warnings,
                        "candidates": result.candidates.iter().map(|candidate| serde_json::json!({
                            "id": candidate.id,
                            "parameters": candidate.parameters,
                            "eligible": candidate.eligible,
                            "calibration_score": candidate.calibration_score,
                            "validation_score": candidate.validation_score,
                            "boundary_hits": candidate.boundary_hits,
                            "observation_failures": candidate.observations.iter().filter(|observation| observation.error.is_some()).count(),
                        })).collect::<Vec<_>>()
                    })
                };
                Ok((body, output))
            }
        }
    };
    match execute() {
        Ok((body, artifact)) => {
            let report = kessetsu_core::compiler::compile_source("", CompileOptions::default());
            let mut output = build_json_output(
                "fit",
                includes,
                "completed",
                &report,
                None,
                None,
                None,
                None,
            );
            output.domain_versions.experiment = Some(kessetsu_core::experiment::EXPERIMENT_SCHEMA);
            output.domain_versions.research_data = Some(kessetsu_core::research_data::DATA_SCHEMA);
            output.domain_versions.fit = Some(FIT_SCHEMA);
            output.fit = Some(body.clone());
            output.artifacts.push(plain_json_artifact(
                "fit_result",
                artifact.to_string_lossy().into_owned(),
            ));
            if *format == Format::Json {
                println!("{}", serde_json::to_string(&output).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&body).unwrap());
                println!("Saved: {}", artifact.display());
            }
            0
        }
        Err(error) => {
            emit(
                format,
                "fit",
                includes,
                "error",
                CompileReport::failure(diagnostic("KES-R002", DiagnosticStage::Cli, error)),
                None,
                None,
                None,
            );
            2
        }
    }
}
