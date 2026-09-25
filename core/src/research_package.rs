//! Deterministic, portable folders for completed local studies.
use crate::experiment::{
    ExperimentResults, datasets_csv, hash_bytes, plan_experiment, plot_svg, report_html,
    results_csv, validate_results,
};
use crate::ir::{RedistributionPolicy, SimulatorCompatibility};
use crate::models::ExternalModelResources;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const RESEARCH_PACKAGE_SCHEMA: &str = "kessetsu.research-package.v1";
pub const MAX_RESEARCH_PACKAGE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchPackageFile {
    pub path: String,
    pub role: String,
    pub media_type: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchDependency {
    pub resource: String,
    pub entry: String,
    pub version: String,
    pub license: String,
    pub source: String,
    pub sha256: String,
    pub simulator: SimulatorCompatibility,
    pub redistribution: RedistributionPolicy,
    pub bundled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchPackagePlot {
    pub path: String,
    pub analysis: usize,
    pub signal: String,
    pub case_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchPackageManifest {
    pub schema_version: String,
    pub core_version: String,
    pub study_identity: String,
    pub result_identity: String,
    pub simulator: crate::simulation::SimulatorInfo,
    pub solver_fingerprint: String,
    pub files: Vec<ResearchPackageFile>,
    pub dependencies: Vec<ResearchDependency>,
    pub plot: ResearchPackagePlot,
    pub rerun: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResearchPackage {
    pub manifest: ResearchPackageManifest,
    /// Relative UTF-8 paths to complete file bytes. The manifest itself is added by the adapter.
    pub files: BTreeMap<String, Vec<u8>>,
}

fn media_type(path: &str) -> &'static str {
    if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".csv") {
        "text/csv; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    }
}

fn role(path: &str) -> &'static str {
    match path {
        "study.kessstudy.json" => "study_specification",
        "circuit.kess" => "editable_source",
        "requirements.kessreq" => "requirements",
        "results.json" => "complete_results",
        "summary.csv" => "measurement_table",
        "datasets.csv" => "numeric_datasets",
        "plot.svg" => "publication_plot",
        "report.html" => "human_report",
        "README.txt" => "rerun_instructions",
        _ => "model_resource",
    }
}

fn select_plot_cases(
    results: &ExperimentResults,
    analysis: usize,
    signal: &str,
    requested: &[String],
) -> Result<Vec<String>, String> {
    if requested.len() > 12 {
        return Err("Choose at most 12 cases for the package plot".into());
    }
    if !requested.is_empty() {
        let known = results
            .plan
            .cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<BTreeSet<_>>();
        if let Some(unknown) = requested.iter().find(|id| !known.contains(id.as_str())) {
            return Err(format!("Unknown plot case '{unknown}'"));
        }
        return Ok(requested.to_vec());
    }
    let mut selected = Vec::new();
    for case in &results.plan.cases {
        let candidate = vec![case.id.clone()];
        if plot_svg(results, analysis, signal, &candidate).is_ok() {
            selected.push(case.id.clone());
            if selected.len() == 12 {
                break;
            }
        }
    }
    if selected.is_empty() {
        return Err("No completed series for the requested analysis and signal".into());
    }
    Ok(selected)
}

fn package_readme(manifest: &ResearchPackageManifest) -> String {
    let dependencies = if manifest.dependencies.is_empty() {
        "No external model resources are required.\n".to_owned()
    } else {
        manifest
            .dependencies
            .iter()
            .map(|dependency| {
                format!(
                    "- {} / {}: {} ({}, {}, {})\n",
                    dependency.resource,
                    dependency.entry,
                    if dependency.bundled {
                        "included"
                    } else {
                        "NOT included; acquire it under its stated terms and place it at this exact path"
                    },
                    dependency.license,
                    dependency.version,
                    dependency.sha256
                )
            })
            .collect::<String>()
    };
    format!(
        "Kessetsu portable research package\n\nStudy identity: {}\nResult identity: {}\nCore: {}\nSimulator: {} ({})\nSolver fingerprint: {}\n\nRerun from this directory:\n  {}\n\nRegenerate the human report:\n  {}\n\nExternal dependencies:\n{}\nFiles:\n- study.kessstudy.json is the authoritative specification, including revision sources.\n- circuit.kess is a convenient copy of the root editable source.\n- results.json retains every case, failure, measurement and numeric simulation dataset.\n- summary.csv and datasets.csv are machine-readable projections of that result.\n- plot.svg records analysis {}, signal {}, and the exact plotted case IDs in manifest.json.\n- report.html is a self-contained human-readable summary.\n\nInterpretation limits:\n- Checksums detect accidental corruption; they are not signatures or independent attestations.\n- Simulation evidence is not a physical-hardware guarantee.\n- Reproduction requires a compatible Kessetsu/Ngspice environment and every non-bundled dependency.\n",
        manifest.study_identity,
        manifest.result_identity,
        manifest.core_version,
        manifest.simulator.version,
        manifest.simulator.executable,
        manifest.solver_fingerprint,
        manifest.rerun[0],
        manifest.rerun[1],
        dependencies,
        manifest.plot.analysis,
        manifest.plot.signal
    )
}

pub fn build_research_package(
    results: &ExperimentResults,
    resources: &ExternalModelResources,
    analysis: usize,
    signal: &str,
    requested_cases: &[String],
) -> Result<ResearchPackage, String> {
    let current_plan = plan_experiment(results.plan.spec.clone(), resources)?;
    validate_results(
        results,
        &current_plan,
        &results.simulator,
        &results.solver_fingerprint,
    )?;
    let selected = select_plot_cases(results, analysis, signal, requested_cases)?;
    let mut files = BTreeMap::from([
        (
            "study.kessstudy.json".into(),
            serde_json::to_vec_pretty(&results.plan.spec).map_err(|e| e.to_string())?,
        ),
        (
            "circuit.kess".into(),
            results.plan.spec.source.as_bytes().to_vec(),
        ),
        (
            "results.json".into(),
            serde_json::to_vec_pretty(results).map_err(|e| e.to_string())?,
        ),
        ("summary.csv".into(), results_csv(results).into_bytes()),
        ("datasets.csv".into(), datasets_csv(results).into_bytes()),
        (
            "plot.svg".into(),
            plot_svg(results, analysis, signal, &selected)?.into_bytes(),
        ),
        ("report.html".into(), report_html(results).into_bytes()),
    ]);
    if let Some(requirements) = &results.plan.spec.requirements {
        files.insert(
            "requirements.kessreq".into(),
            requirements.as_bytes().to_vec(),
        );
    }
    let mut reserved_paths = files.keys().cloned().collect::<BTreeSet<_>>();
    reserved_paths.insert("manifest.json".into());

    let mut dependencies = BTreeMap::new();
    for manifest in &results.plan.model_manifests {
        for model in &manifest.models {
            let Some(external) = &model.external else {
                continue;
            };
            let key = (external.resource.clone(), external.entry.clone());
            let dependency = ResearchDependency {
                resource: external.resource.clone(),
                entry: external.entry.clone(),
                version: model.provenance.version.clone(),
                license: model.provenance.license.clone(),
                source: model.provenance.source.clone(),
                sha256: model.provenance.content_hash.clone(),
                simulator: external.simulator,
                redistribution: external.redistribution,
                bundled: external.redistribution == RedistributionPolicy::Permitted,
            };
            if let Some(previous) = dependencies.insert(key, dependency.clone())
                && previous != dependency
            {
                return Err(format!(
                    "Conflicting model metadata for '{} / {}'",
                    dependency.resource, dependency.entry
                ));
            }
            if dependency.bundled {
                let bytes = resources.get(&dependency.resource).ok_or_else(|| {
                    format!("Missing permitted model resource '{}'", dependency.resource)
                })?;
                if reserved_paths.contains(&dependency.resource) {
                    return Err(format!(
                        "Model resource '{}' collides with a reserved package file",
                        dependency.resource
                    ));
                }
                if let Some(previous) = files.insert(dependency.resource.clone(), bytes.clone())
                    && previous != *bytes
                {
                    return Err(format!(
                        "Conflicting model contents for resource '{}'",
                        dependency.resource
                    ));
                }
            }
        }
    }

    let plot_case_arguments = selected
        .iter()
        .map(|case| format!(" --case {case}"))
        .collect::<String>();
    let mut manifest = ResearchPackageManifest {
        schema_version: RESEARCH_PACKAGE_SCHEMA.into(),
        core_version: results.plan.core_version.clone(),
        study_identity: results.plan.identity.clone(),
        result_identity: results.identity.clone(),
        simulator: results.simulator.clone(),
        solver_fingerprint: results.solver_fingerprint.clone(),
        files: Vec::new(),
        dependencies: dependencies.into_values().collect(),
        plot: ResearchPackagePlot {
            path: "plot.svg".into(),
            analysis,
            signal: signal.into(),
            case_ids: selected,
        },
        rerun: vec![
            "kess study run study.kessstudy.json --output rerun-results.json".into(),
            format!(
                "kess study export rerun-results.json --target svg --signal \"{}\" --analysis {}{} --output rerun-plot.svg",
                signal, analysis, plot_case_arguments
            ),
        ],
        caveats: vec![
            "Checksums detect accidental corruption; they are not cryptographic attestations.".into(),
            "Simulation evidence is not a physical-hardware guarantee.".into(),
            "Non-bundled dependencies must be acquired under their stated terms and restored at the exact resource path.".into(),
        ],
    };
    files.insert("README.txt".into(), package_readme(&manifest).into_bytes());
    manifest.files = files
        .iter()
        .map(|(path, bytes)| ResearchPackageFile {
            path: path.clone(),
            role: role(path).into(),
            media_type: media_type(path).into(),
            sha256: hash_bytes(bytes),
            bytes: bytes.len(),
        })
        .collect();
    let total = files
        .values()
        .try_fold(0usize, |total, bytes| total.checked_add(bytes.len()))
        .ok_or("Research package size overflow")?;
    if total > MAX_RESEARCH_PACKAGE_BYTES {
        return Err("Research package exceeds 256 MiB; narrow the study".into());
    }
    Ok(ResearchPackage { manifest, files })
}
