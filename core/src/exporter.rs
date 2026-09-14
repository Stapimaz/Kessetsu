use crate::compiler::{CompileReport, Diagnostic};
use crate::schematic::Schematic;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

pub const EXPORT_SCHEMA_VERSION: &str = "kessetsu.export.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Svg,
    Png,
    Pdf,
    SchematicJson,
    Spice,
    Kicad,
    Ltspice,
}

impl ExportFormat {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
            Self::SchematicJson => "schematic-json",
            Self::Spice => "spice",
            Self::Kicad => "kicad",
            Self::Ltspice => "ltspice",
        }
    }

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
            Self::SchematicJson => "schematic.json",
            Self::Spice => "spice",
            Self::Kicad => "kicad_sch",
            Self::Ltspice => "asc",
        }
    }

    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Svg => "image/svg+xml",
            Self::Png => "image/png",
            Self::Pdf => "application/pdf",
            Self::SchematicJson => "application/json",
            Self::Spice | Self::Kicad | Self::Ltspice => "text/plain",
        }
    }

    pub const fn capability(self) -> ExportCapability {
        match self {
            Self::Svg => ExportCapability::visual(true, false),
            Self::Png => ExportCapability::visual(false, false),
            Self::Pdf => ExportCapability::visual(true, false),
            Self::SchematicJson => ExportCapability {
                visual: false,
                machine_readable: true,
                editable: true,
                preserves_connectivity: true,
                preserves_models: true,
                preserves_analysis: false,
            },
            Self::Spice => ExportCapability {
                visual: false,
                machine_readable: true,
                editable: true,
                preserves_connectivity: true,
                preserves_models: true,
                preserves_analysis: true,
            },
            Self::Kicad => ExportCapability {
                visual: true,
                machine_readable: true,
                editable: true,
                preserves_connectivity: true,
                preserves_models: false,
                preserves_analysis: false,
            },
            Self::Ltspice => ExportCapability {
                visual: true,
                machine_readable: true,
                editable: true,
                preserves_connectivity: true,
                preserves_models: true,
                preserves_analysis: true,
            },
        }
    }
}

impl FromStr for ExportFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "svg" => Ok(Self::Svg),
            "png" => Ok(Self::Png),
            "pdf" => Ok(Self::Pdf),
            "schematic-json" | "schematic_json" | "json" => Ok(Self::SchematicJson),
            "spice" | "cir" => Ok(Self::Spice),
            "kicad" | "kicad-sch" | "kicad_sch" => Ok(Self::Kicad),
            "ltspice" | "asc" => Ok(Self::Ltspice),
            _ => Err(format!(
                "unsupported export format '{value}'; expected svg, png, pdf, schematic-json, spice, kicad, or ltspice"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportCapability {
    pub visual: bool,
    pub machine_readable: bool,
    pub editable: bool,
    pub preserves_connectivity: bool,
    pub preserves_models: bool,
    pub preserves_analysis: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportDescriptor {
    pub schema_version: String,
    pub format: ExportFormat,
    pub label: String,
    pub extension: String,
    pub mime_type: String,
    pub capability: ExportCapability,
}

pub fn export_capabilities() -> Vec<ExportDescriptor> {
    [
        (ExportFormat::Svg, "SVG"),
        (ExportFormat::Png, "PNG"),
        (ExportFormat::Pdf, "PDF"),
        (ExportFormat::SchematicJson, "Schematic JSON"),
        (ExportFormat::Spice, "SPICE"),
        (ExportFormat::Kicad, "KiCad"),
        (ExportFormat::Ltspice, "LTspice"),
    ]
    .into_iter()
    .map(|(format, label)| ExportDescriptor {
        schema_version: EXPORT_SCHEMA_VERSION.to_string(),
        format,
        label: label.to_string(),
        extension: format.extension().to_string(),
        mime_type: format.mime_type().to_string(),
        capability: format.capability(),
    })
    .collect()
}

impl ExportCapability {
    const fn visual(editable: bool, preserves_models: bool) -> Self {
        Self {
            visual: true,
            machine_readable: false,
            editable,
            preserves_connectivity: true,
            preserves_models,
            preserves_analysis: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderBackground {
    White,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportOptions {
    pub scale: f32,
    pub background: RenderBackground,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            scale: 2.0,
            background: RenderBackground::White,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtifact {
    pub schema_version: String,
    pub exporter: String,
    pub exporter_version: u32,
    pub format: ExportFormat,
    pub label: String,
    pub extension: String,
    pub mime_type: String,
    pub sha256: String,
    pub byte_length: usize,
    pub connectivity_verified: bool,
    pub capability: ExportCapability,
    pub warnings: Vec<String>,
    pub losses: Vec<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportError {
    pub code: String,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ExportError {}

fn schematic(report: &CompileReport) -> Result<&Schematic, ExportError> {
    report.schematic.as_ref().ok_or_else(|| ExportError {
        code: "KES-X002".to_string(),
        message: "canonical Schematic IR is unavailable; export stopped".to_string(),
        diagnostics: report.diagnostics.clone(),
    })
}

fn render_png(schematic: &Schematic, options: ExportOptions) -> Result<Vec<u8>, ExportError> {
    if !(0.25..=8.0).contains(&options.scale) || !options.scale.is_finite() {
        return Err(ExportError {
            code: "KES-X004".to_string(),
            message: "PNG scale must be a finite value between 0.25 and 8".to_string(),
            diagnostics: Vec::new(),
        });
    }
    let svg = crate::schematic_svg::render_svg_with_background(
        schematic,
        matches!(options.background, RenderBackground::White),
    );
    let mut parser = resvg::usvg::Options::default();
    parser
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/RobotoMono.ttf").to_vec());
    let tree = resvg::usvg::Tree::from_str(&svg, &parser).map_err(|error| ExportError {
        code: "KES-X005".to_string(),
        message: format!("could not parse canonical SVG for PNG rendering: {error}"),
        diagnostics: Vec::new(),
    })?;
    let size = tree.size().to_int_size();
    let width = ((size.width() as f32) * options.scale).round() as u32;
    let height = ((size.height() as f32) * options.scale).round() as u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width.max(1), height.max(1)).ok_or_else(|| ExportError {
            code: "KES-X006".to_string(),
            message: "PNG dimensions exceed the rasterizer limit".to_string(),
            diagnostics: Vec::new(),
        })?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(options.scale, options.scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|error| ExportError {
        code: "KES-X007".to_string(),
        message: format!("could not encode PNG: {error}"),
        diagnostics: Vec::new(),
    })
}

fn render_pdf(schematic: &Schematic) -> Result<Vec<u8>, ExportError> {
    let svg = crate::schematic_svg::render_svg_with_background(schematic, true);
    let mut parser = svg2pdf::usvg::Options::default();
    parser
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/RobotoMono.ttf").to_vec());
    let tree = svg2pdf::usvg::Tree::from_str(&svg, &parser).map_err(|error| ExportError {
        code: "KES-X008".to_string(),
        message: format!("could not parse canonical SVG for PDF rendering: {error}"),
        diagnostics: Vec::new(),
    })?;
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions {
            // Glyph outlines make the vector document deterministic across
            // native and WASM font-database IDs. Semantic/selectable text is
            // retained by the companion SVG artifact.
            embed_text: false,
            ..svg2pdf::ConversionOptions::default()
        },
        svg2pdf::PageOptions { dpi: 96.0 },
    )
    .map_err(|error| ExportError {
        code: "KES-X009".to_string(),
        message: format!("could not encode vector PDF: {error}"),
        diagnostics: Vec::new(),
    })
}

pub fn export_report(
    report: &CompileReport,
    format: ExportFormat,
    options: ExportOptions,
) -> Result<ExportArtifact, ExportError> {
    if report.has_errors() {
        return Err(ExportError {
            code: "KES-X001".to_string(),
            message: "source has compile/ERC errors; no artifact was produced".to_string(),
            diagnostics: report.diagnostics.clone(),
        });
    }

    let mut warnings = Vec::new();
    let mut losses = Vec::new();
    let external_models = report
        .ir
        .as_ref()
        .map(|circuit| {
            circuit
                .model_manifest
                .models
                .iter()
                .filter_map(|model| {
                    model.external.as_ref().map(|external| {
                        format!(
                            "{} requires user-owned '{}' ({})",
                            model.name, external.resource, model.provenance.content_hash
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let (bytes, connectivity_verified) = match format {
        ExportFormat::Svg => {
            let schematic = schematic(report)?;
            (
                crate::schematic_svg::render_svg_with_background(
                    schematic,
                    matches!(options.background, RenderBackground::White),
                )
                .into_bytes(),
                schematic.connectivity.verified,
            )
        }
        ExportFormat::Png => {
            let schematic = schematic(report)?;
            (
                render_png(schematic, options)?,
                schematic.connectivity.verified,
            )
        }
        ExportFormat::Pdf => {
            let schematic = schematic(report)?;
            (render_pdf(schematic)?, schematic.connectivity.verified)
        }
        ExportFormat::SchematicJson => {
            let schematic = schematic(report)?;
            let mut json = serde_json::to_vec_pretty(schematic).map_err(|error| ExportError {
                code: "KES-X010".to_string(),
                message: format!("could not serialize Schematic IR: {error}"),
                diagnostics: Vec::new(),
            })?;
            json.push(b'\n');
            (json, schematic.connectivity.verified)
        }
        ExportFormat::Spice => {
            if !external_models.is_empty() {
                warnings.push(format!(
                    "keep these user-owned model files beside the export: {}",
                    external_models.join("; ")
                ));
            }
            (
                report
                    .spice_netlist
                    .as_ref()
                    .ok_or_else(|| ExportError {
                        code: "KES-X011".to_string(),
                        message: "canonical SPICE is unavailable".to_string(),
                        diagnostics: Vec::new(),
                    })?
                    .as_bytes()
                    .to_vec(),
                true,
            )
        }
        ExportFormat::Kicad => {
            let schematic = schematic(report)?;
            if report.ir.as_ref().is_some_and(|ir| !ir.analyses.is_empty()) {
                losses.push(
                    "simulation commands and assertions remain in the .kess source; KiCad export contains topology, values and model metadata"
                        .to_string(),
                );
            }
            warnings.push(
                "KiCad may report a symbol-table warning for portable embedded Kessetsu symbols; embedded definitions remain editable and connectivity-safe"
                    .to_string(),
            );
            if !external_models.is_empty() {
                losses.push(format!(
                    "external model bodies are not embedded; {}",
                    external_models.join("; ")
                ));
            }
            (
                crate::kicad::generate_kicad_sch(schematic)?.into_bytes(),
                schematic.connectivity.verified,
            )
        }
        ExportFormat::Ltspice => {
            let schematic = schematic(report)?;
            let circuit = report.ir.as_ref().ok_or_else(|| ExportError {
                code: "KES-X016".to_string(),
                message: "typed Circuit IR is unavailable for LTspice export".to_string(),
                diagnostics: Vec::new(),
            })?;
            if report
                .ir
                .as_ref()
                .is_some_and(|ir| !ir.assertions.is_empty())
            {
                losses.push(
                    "engineering assertions remain in the .kess source; LTspice export contains topology, values, model directives and analyses"
                        .to_string(),
                );
            }
            warnings.push(
                "LTspice symbols use the standard bundled symbol library; keep the .kess source as the authoritative design"
                    .to_string(),
            );
            if !external_models.is_empty() {
                warnings.push(format!(
                    "keep these user-owned model files beside the export: {}",
                    external_models.join("; ")
                ));
            }
            (
                crate::ltspice::generate_ltspice_asc(schematic, circuit)?.into_bytes(),
                schematic.connectivity.verified,
            )
        }
    };

    if !connectivity_verified && format.capability().preserves_connectivity {
        return Err(ExportError {
            code: "KES-X003".to_string(),
            message: "canonical connectivity proof failed; export stopped".to_string(),
            diagnostics: Vec::new(),
        });
    }

    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(ExportArtifact {
        schema_version: EXPORT_SCHEMA_VERSION.to_string(),
        exporter: format!("kessetsu-{}", format.id()),
        exporter_version: 1,
        format,
        label: export_capabilities()
            .into_iter()
            .find(|descriptor| descriptor.format == format)
            .map(|descriptor| descriptor.label)
            .unwrap_or_else(|| format.id().to_string()),
        extension: format.extension().to_string(),
        mime_type: format.mime_type().to_string(),
        sha256,
        byte_length: bytes.len(),
        connectivity_verified,
        capability: format.capability(),
        warnings,
        losses,
        bytes,
    })
}
