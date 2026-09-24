use crate::ast::Program;
use crate::compile_inputs::CompileInputs;
use crate::erc::{ErcDiagnostic, Severity as ErcSeverity, check_rules};
use crate::graph::{NetId, NetlistGraph, generate_spice};
use crate::ir::{CircuitIR, SemanticDiagnostic, ast_to_ir_with_resources};
use crate::layout::LayoutResult;
use crate::models::ExternalModelResources;
use crate::parser::{Rule, parse_program};
use crate::schematic::Schematic;
use pest::error::LineColLocation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const COMPILE_SCHEMA_VERSION: &str = "kessetsu.compile.v6";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStage {
    Parse,
    Flatten,
    Semantic,
    Erc,
    Io,
    Cli,
    Simulation,
    Assertion,
    Requirements,
    Schematic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub stage: DiagnosticStage,
    pub message: String,
    pub component: Option<String>,
    pub pin: Option<String>,
    pub field: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompileOptions {
    pub include_ast: bool,
    pub generate_spice: bool,
    pub generate_layout: bool,
    pub generate_kicad: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            include_ast: false,
            generate_spice: true,
            generate_layout: false,
            generate_kicad: false,
        }
    }
}

impl CompileOptions {
    pub const fn all_outputs() -> Self {
        Self {
            include_ast: true,
            generate_spice: true,
            generate_layout: true,
            generate_kicad: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNetSummary {
    pub id: NetId,
    pub name: String,
    pub pins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSummary {
    pub nets: Vec<GraphNetSummary>,
    pub ground_candidates: Vec<String>,
    pub ground_is_explicit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileReport {
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_source: Option<String>,
    pub ast: Option<Program>,
    pub ir: Option<CircuitIR>,
    pub diagnostics: Vec<Diagnostic>,
    pub graph: Option<GraphSummary>,
    pub spice_netlist: Option<String>,
    pub layout: Option<LayoutResult>,
    pub schematic: Option<Schematic>,
    pub schematic_svg: Option<String>,
    pub kicad_sch: Option<String>,
    pub model_lock: Option<String>,
}

impl CompileReport {
    fn empty() -> Self {
        Self {
            schema_version: COMPILE_SCHEMA_VERSION.to_string(),
            effective_source: None,
            ast: None,
            ir: None,
            diagnostics: Vec::new(),
            graph: None,
            spice_netlist: None,
            layout: None,
            schematic: None,
            schematic_svg: None,
            kicad_sch: None,
            model_lock: None,
        }
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    pub fn failure(diagnostic: Diagnostic) -> Self {
        let mut report = Self::empty();
        report.diagnostics.push(diagnostic);
        report
    }
}

fn parser_diagnostic(error: pest::error::Error<Rule>) -> Diagnostic {
    let (line, column) = match error.line_col {
        LineColLocation::Pos((line, column)) | LineColLocation::Span((line, column), _) => {
            (Some(line), Some(column))
        }
    };
    Diagnostic {
        code: "KES-P001".to_string(),
        severity: DiagnosticSeverity::Error,
        stage: DiagnosticStage::Parse,
        message: error.to_string(),
        component: None,
        pin: None,
        field: None,
        line,
        column,
    }
}

fn annotate_source_location(source: &str, diagnostic: &mut Diagnostic) {
    if diagnostic.line.is_some() {
        return;
    }
    let mut needles = Vec::new();
    if let Some(component) = diagnostic.component.as_deref() {
        needles.push(component);
    }
    if let Some(field) = diagnostic.field.as_deref() {
        needles.push(field);
        if field == "analysis" {
            needles.push("simulate");
        }
    }
    needles.extend(diagnostic.message.split(|character: char| {
        character.is_whitespace() || matches!(character, '\'' | '"' | ':' | ',' | '.')
    }));

    for needle in needles.into_iter().filter(|needle| !needle.is_empty()) {
        for (line_index, line) in source.lines().enumerate() {
            if let Some(column) = line.to_ascii_lowercase().find(&needle.to_ascii_lowercase()) {
                diagnostic.line = Some(line_index + 1);
                diagnostic.column = Some(column + 1);
                return;
            }
        }
    }
}

fn annotate_source_locations(source: &str, diagnostics: &mut [Diagnostic]) {
    for diagnostic in diagnostics {
        annotate_source_location(source, diagnostic);
    }
}

fn flatten_diagnostic(error: crate::elaboration::ElaborationError) -> Diagnostic {
    Diagnostic {
        code: "KES-C008".to_string(),
        severity: DiagnosticSeverity::Error,
        stage: DiagnosticStage::Flatten,
        message: error.message,
        component: None,
        pin: None,
        field: error.field,
        line: error.location.map(|location| location.0),
        column: error.location.map(|location| location.1),
    }
}

impl From<SemanticDiagnostic> for Diagnostic {
    fn from(diagnostic: SemanticDiagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: DiagnosticSeverity::Error,
            stage: DiagnosticStage::Semantic,
            message: diagnostic.message,
            component: diagnostic.component,
            pin: None,
            field: diagnostic.field,
            line: None,
            column: None,
        }
    }
}

impl From<ErcDiagnostic> for Diagnostic {
    fn from(diagnostic: ErcDiagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: match diagnostic.severity {
                ErcSeverity::Error => DiagnosticSeverity::Error,
                ErcSeverity::Warning => DiagnosticSeverity::Warning,
                ErcSeverity::Info => DiagnosticSeverity::Info,
            },
            stage: DiagnosticStage::Erc,
            message: diagnostic.message,
            component: diagnostic.component,
            pin: diagnostic.pin,
            field: None,
            line: None,
            column: None,
        }
    }
}

impl GraphSummary {
    fn from_graph(graph: &NetlistGraph) -> Self {
        let mut nets: BTreeMap<NetId, Vec<String>> = BTreeMap::new();
        for (pin, net) in &graph.pin_to_net {
            nets.entry(*net).or_default().push(pin.clone());
        }

        let nets = nets
            .into_iter()
            .map(|(id, mut pins)| {
                pins.sort();
                GraphNetSummary {
                    id,
                    name: graph.get_net_name(id),
                    pins,
                }
            })
            .collect();

        Self {
            nets,
            ground_candidates: graph.ground_candidates.clone(),
            ground_is_explicit: graph.ground_is_explicit,
        }
    }
}

/// Compiles Kessetsu source without performing filesystem or process I/O.
///
/// Every stage reports failures through `CompileReport::diagnostics`. Backend
/// outputs are generated only when no error-severity diagnostic exists.
pub fn compile_source(source: &str, options: CompileOptions) -> CompileReport {
    compile_source_with_resources(source, options, &ExternalModelResources::new())
}

pub fn compile_source_with_resources(
    source: &str,
    options: CompileOptions,
    resources: &ExternalModelResources,
) -> CompileReport {
    compile_source_with_inputs(source, options, &CompileInputs::default(), resources)
}

/// Shared native/WASM input contract; source materialization is a portable result,
/// not a second compilation pipeline or an in-place file edit.
pub fn compile_source_with_inputs(
    source: &str,
    options: CompileOptions,
    inputs: &CompileInputs,
    resources: &ExternalModelResources,
) -> CompileReport {
    let mut report = CompileReport::empty();
    if let Err(diagnostic) = crate::compile_inputs::validate_inputs(inputs) {
        report.diagnostics.push(*diagnostic);
        return report;
    }

    let mut program = match parse_program(source) {
        Ok(program) => program,
        Err(error) => {
            report.diagnostics.push(parser_diagnostic(error));
            return report;
        }
    };

    let effective_source = match crate::compile_inputs::apply(source, &mut program, inputs) {
        Ok(source) => source,
        Err(mut diagnostic) => {
            if let Some(name) = diagnostic.field.as_deref()
                && let Some(parameter) = program.statements.iter().find_map(|statement| {
                    if let crate::ast::Statement::Param(parameter) = statement
                        && parameter.name == name
                    {
                        Some(parameter)
                    } else {
                        None
                    }
                })
            {
                diagnostic.line = Some(parameter.line);
                diagnostic.column = Some(parameter.column);
            }
            report.diagnostics.push(*diagnostic);
            return report;
        }
    };

    let elaborated = match crate::elaboration::flatten(&program) {
        Ok(program) => program,
        Err(message) => {
            report.diagnostics.push(flatten_diagnostic(message));
            annotate_source_locations(source, &mut report.diagnostics);
            return report;
        }
    };
    let flat_program = elaborated.program;

    if options.include_ast {
        report.ast = Some(flat_program.clone());
    }

    let circuit = match ast_to_ir_with_resources(&flat_program, resources) {
        Ok(circuit) => circuit,
        Err(diagnostic) => {
            let mut diagnostic: Diagnostic = diagnostic.into();
            if diagnostic.component.is_none()
                && matches!(
                    diagnostic.code.as_str(),
                    "KES-C020" | "KES-C021" | "KES-C022" | "KES-C023"
                )
                && let Some(name) = diagnostic.field.as_deref()
                && let Some(location) = elaborated.parameter_locations.get(name)
            {
                let position = if diagnostic.code == "KES-C020" {
                    location.duplicate.unwrap_or(location.primary)
                } else {
                    location.primary
                };
                diagnostic.line = Some(position.0);
                diagnostic.column = Some(position.1);
            }
            report.diagnostics.push(diagnostic);
            annotate_source_locations(source, &mut report.diagnostics);
            return report;
        }
    };
    if !circuit.model_manifest.models.is_empty() || !circuit.model_manifest.packages.is_empty() {
        report.model_lock = Some(crate::models::lockfile_json(&circuit.model_manifest));
    }

    let graph = NetlistGraph::build(&circuit);
    report.diagnostics = check_rules(&circuit, &graph)
        .into_iter()
        .map(Diagnostic::from)
        .collect();
    annotate_source_locations(source, &mut report.diagnostics);
    report.graph = Some(GraphSummary::from_graph(&graph));
    report.ir = Some(circuit.clone());

    if report.has_errors() {
        return report;
    }
    report.effective_source = effective_source;

    if options.generate_spice {
        report.spice_netlist = Some(generate_spice(&circuit, &graph));
    }

    if options.generate_layout || options.generate_kicad {
        let schematic = match crate::schematic::generate_schematic(&circuit) {
            Ok(schematic) => schematic,
            Err(error) => {
                report.diagnostics.push(Diagnostic {
                    code: "KES-L001".to_string(),
                    severity: DiagnosticSeverity::Error,
                    stage: DiagnosticStage::Schematic,
                    message: error.to_string(),
                    component: None,
                    pin: None,
                    field: None,
                    line: None,
                    column: None,
                });
                return report;
            }
        };
        report.schematic_svg = Some(crate::schematic_svg::render_svg(&schematic));
        if options.generate_kicad {
            match crate::kicad::generate_kicad_sch(&schematic) {
                Ok(kicad) => report.kicad_sch = Some(kicad),
                Err(error) => {
                    report.diagnostics.push(Diagnostic {
                        code: error.code,
                        severity: DiagnosticSeverity::Error,
                        stage: DiagnosticStage::Schematic,
                        message: error.message,
                        component: None,
                        pin: None,
                        field: None,
                        line: None,
                        column: None,
                    });
                    return report;
                }
            }
        }
        if options.generate_layout {
            // Legacy debug view only; all user-facing exporters consume the
            // canonical Schematic IR above.
            report.layout = Some(crate::layout::generate_layout(&circuit));
        }
        report.schematic = Some(schematic);
    }

    report
}
