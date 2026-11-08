use std::fs;
use std::path::Path;

use kessetsu_core::compiler::COMPILE_SCHEMA_VERSION;
use kessetsu_core::spice_import::{
    ImportDiagnostic, ImportSeverity, SPICE_IMPORT_SCHEMA_VERSION, SpiceImportReport, import_spice,
};
use serde::Serialize;

use super::{
    CLI_SCHEMA_VERSION, Format, ImportCommand, JsonArtifact, plain_json_artifact, read_source,
};

#[derive(Serialize)]
pub(crate) struct ImportJsonOutput<'a> {
    pub(crate) schema_version: &'static str,
    pub(crate) command: &'static str,
    pub(crate) status: &'static str,
    pub(crate) domain_versions: ImportDomainVersions,
    pub(crate) import: &'a SpiceImportReport,
    pub(crate) artifacts: Vec<JsonArtifact>,
}

#[derive(Serialize)]
pub(crate) struct ImportDomainVersions {
    pub(crate) compile: &'static str,
    pub(crate) spice_import: &'static str,
}

pub(crate) fn run_spice_import(command: &ImportCommand, format: &Format) -> i32 {
    let source_is_stdin = command.file == Path::new("-");
    let source = match read_source(&command.file) {
        Ok(source) => source,
        Err(error) => {
            let mut report = import_spice("");
            report.diagnostics.clear();
            report.diagnostics.push(ImportDiagnostic {
                code: "KES-I001".into(),
                severity: ImportSeverity::Error,
                message: format!(
                    "Could not read SPICE input '{}': {error}",
                    command.file.display()
                ),
                line: None,
                column: None,
            });
            return emit_import(format, &report, None);
        }
    };
    let mut report = import_spice(&source);
    if report.has_errors() {
        return emit_import(format, &report, None);
    }

    let output = command
        .output
        .clone()
        .or_else(|| (!source_is_stdin).then(|| command.file.with_extension("kess")));
    let Some(output) = output else {
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-I002".into(),
            severity: ImportSeverity::Error,
            message: "stdin import requires --output PATH.kess".into(),
            line: None,
            column: None,
        });
        return emit_import(format, &report, None);
    };
    if output.extension().and_then(|extension| extension.to_str()) != Some("kess") {
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-I002".into(),
            severity: ImportSeverity::Error,
            message: "SPICE import output must use the .kess extension".into(),
            line: None,
            column: None,
        });
        return emit_import(format, &report, None);
    }
    let output_parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let resolved_output = match fs::canonicalize(output_parent) {
        Ok(parent) => parent.join(output.file_name().expect("output has a filename")),
        Err(error) => {
            report.diagnostics.push(ImportDiagnostic {
                code: "KES-I001".into(),
                severity: ImportSeverity::Error,
                message: format!("Could not resolve output directory: {error}"),
                line: None,
                column: None,
            });
            return emit_import(format, &report, None);
        }
    };
    let aliases_input = !source_is_stdin
        && fs::canonicalize(&command.file).is_ok_and(|input| {
            input == resolved_output
                || (output.exists()
                    && fs::canonicalize(&output).is_ok_and(|existing| existing == input))
        });
    if aliases_input {
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-I002".into(),
            severity: ImportSeverity::Error,
            message: "Import output cannot overwrite the original SPICE file".into(),
            line: None,
            column: None,
        });
        return emit_import(format, &report, None);
    }
    if output.exists() && !command.force {
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-I003".into(),
            severity: ImportSeverity::Error,
            message: format!(
                "Output '{}' already exists; use --force to replace it",
                output.display()
            ),
            line: None,
            column: None,
        });
        return emit_import(format, &report, None);
    }
    let source = report
        .kess_source
        .as_deref()
        .expect("successful import has canonical source");
    if let Err(error) = fs::write(&output, source) {
        report.diagnostics.push(ImportDiagnostic {
            code: "KES-I004".into(),
            severity: ImportSeverity::Error,
            message: format!("Could not write '{}': {error}", output.display()),
            line: None,
            column: None,
        });
        return emit_import(format, &report, None);
    }
    emit_import(format, &report, Some(&output))
}

pub(crate) fn emit_import(
    format: &Format,
    report: &SpiceImportReport,
    output: Option<&Path>,
) -> i32 {
    let failed = report.has_errors();
    let status = if failed { "error" } else { "success" };
    if *format == Format::Json {
        let artifacts = output
            .map(|path| plain_json_artifact("kessetsu_source", path.to_string_lossy()))
            .into_iter()
            .collect();
        let body = ImportJsonOutput {
            schema_version: CLI_SCHEMA_VERSION,
            command: "import",
            status,
            domain_versions: ImportDomainVersions {
                compile: COMPILE_SCHEMA_VERSION,
                spice_import: SPICE_IMPORT_SCHEMA_VERSION,
            },
            import: report,
            artifacts,
        };
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
    } else {
        for diagnostic in &report.diagnostics {
            let location = diagnostic
                .line
                .map(|line| format!(" at line {line}"))
                .unwrap_or_default();
            eprintln!(
                "[{:?}] {}{}: {}",
                diagnostic.severity, diagnostic.code, location, diagnostic.message
            );
        }
        if let Some(path) = output {
            println!(
                "[SUCCESS] Imported {} components, {} nets and {} analyses to {}",
                report.summary.components,
                report.summary.nets,
                report.summary.analyses,
                path.display()
            );
        }
    }
    if failed { 2 } else { 0 }
}
