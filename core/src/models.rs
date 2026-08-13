use crate::ast::{ModelDecl, ModelDeclKind, Program, SubcircuitDecl};
use crate::component::component_definition;
use crate::graph::format_spice_number;
use crate::ir::{
    BJTPolarity, ComponentKind, FETPolarity, IRComponent, ModelDefinition, ModelManifest,
    ModelManifestEntry, ModelProvenance, ModelRef, ModelSource, ResolvedModelPackage,
    SemanticDiagnostic, parse_si_value,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MODEL_MANIFEST_SCHEMA_VERSION: &str = "netlang.models.v1";
pub const MODEL_LOCK_SCHEMA_VERSION: &str = "netlang.lock.v1";
const SIMULATOR_CAPABILITY: &str = "ngspice-35+";

#[derive(Debug, Clone)]
pub struct ModelLibrary {
    models: BTreeMap<String, ModelRef>,
    packages: Vec<ResolvedModelPackage>,
}

impl ModelLibrary {
    pub fn resolve(&self, name: &str) -> Option<ModelRef> {
        self.models.get(&name.to_ascii_uppercase()).cloned()
    }

    pub fn manifest(&self, components: &[IRComponent]) -> ModelManifest {
        let names = components
            .iter()
            .filter_map(|component| component.model.as_ref())
            .map(|model| model.name.to_ascii_uppercase())
            .collect::<BTreeSet<_>>();
        let models = names
            .iter()
            .filter_map(|name| self.models.get(name))
            .map(|model| ModelManifestEntry {
                name: model.name.clone(),
                kind: model.kind.clone(),
                source: model.source.clone(),
                provenance: model.provenance.clone(),
            })
            .collect();
        ModelManifest {
            schema_version: MODEL_MANIFEST_SCHEMA_VERSION.to_string(),
            packages: self.packages.clone(),
            models,
        }
    }
}

pub fn resolve_program_models(program: &Program) -> Result<ModelLibrary, SemanticDiagnostic> {
    let mut library = ModelLibrary {
        models: builtin_models()
            .into_iter()
            .map(|model| (model.name.to_ascii_uppercase(), model))
            .collect(),
        packages: Vec::new(),
    };

    let mut included_packages = BTreeSet::new();
    for include in &program.model_includes {
        let package_key = include.package.to_ascii_lowercase();
        if !included_packages.insert(package_key.clone()) {
            return Err(error(
                "NL-C013",
                format!("duplicate model package include '{}'", include.package),
                None,
                "model_include",
            ));
        }
        let (package, models) = resolve_package(&package_key, &include.version)?;
        for model in models {
            insert_model(&mut library.models, model)?;
        }
        library.packages.push(package);
    }

    for declaration in &program.models {
        let model = compile_user_model(declaration)?;
        insert_model(&mut library.models, model)?;
    }
    for declaration in &program.subcircuits {
        let model = compile_user_subcircuit(declaration)?;
        insert_model(&mut library.models, model)?;
    }
    library
        .packages
        .sort_by(|left, right| left.name.cmp(&right.name));
    Ok(library)
}

pub fn builtin_model(name: &str) -> Option<ModelRef> {
    builtin_models()
        .into_iter()
        .find(|model| model.name.eq_ignore_ascii_case(name))
}

pub fn validate_component_model_pins(
    kind: &ComponentKind,
    model: &ModelRef,
) -> Result<(), SemanticDiagnostic> {
    let ModelDefinition::Subcircuit { pins, .. } = &model.definition else {
        return Ok(());
    };
    let expected = component_definition(kind)
        .pins
        .iter()
        .map(|pin| pin.name.to_string())
        .collect::<Vec<_>>();
    if pins != &expected {
        return Err(error(
            "NL-C012",
            format!(
                "subcircuit '{}' pin order {:?} does not match {:?} catalog order {:?}",
                model.name, pins, kind, expected
            ),
            Some(&model.name),
            "pins",
        ));
    }
    Ok(())
}

pub fn lockfile_json(manifest: &ModelManifest) -> String {
    #[derive(serde::Serialize)]
    struct Lock<'a> {
        schema_version: &'static str,
        manifest_schema_version: &'a str,
        packages: &'a [ResolvedModelPackage],
        models: &'a [ModelManifestEntry],
    }
    serde_json::to_string_pretty(&Lock {
        schema_version: MODEL_LOCK_SCHEMA_VERSION,
        manifest_schema_version: &manifest.schema_version,
        packages: &manifest.packages,
        models: &manifest.models,
    })
    .expect("typed model lock must serialize")
        + "\n"
}

fn insert_model(
    models: &mut BTreeMap<String, ModelRef>,
    model: ModelRef,
) -> Result<(), SemanticDiagnostic> {
    let key = model.name.to_ascii_uppercase();
    if models.contains_key(&key) {
        return Err(error(
            "NL-C013",
            format!("model/subcircuit name '{}' is already defined", model.name),
            Some(&model.name),
            "name",
        ));
    }
    models.insert(key, model);
    Ok(())
}

fn resolve_package(
    name: &str,
    version: &str,
) -> Result<(ResolvedModelPackage, Vec<ModelRef>), SemanticDiagnostic> {
    if name != "netlang_analog" || version != "1.0.0" {
        return Err(error(
            "NL-C011",
            format!(
                "unavailable model package '{name}@{version}'; supported package is netlang_analog@1.0.0"
            ),
            None,
            "model_include",
        ));
    }
    let model = subcircuit_model(
        "NLANG_PACKAGE_OPAMP",
        ModelSource::Package,
        "NetLang netlang_analog package",
        "Apache-2.0",
        "1.0.0",
        500_000.0,
        2_000_000.0,
    );
    let package_content = definition_text(&model.definition);
    Ok((
        ResolvedModelPackage {
            name: "netlang_analog".to_string(),
            version: "1.0.0".to_string(),
            license: "Apache-2.0".to_string(),
            content_hash: content_hash(package_content),
        },
        vec![model],
    ))
}

fn compile_user_model(declaration: &ModelDecl) -> Result<ModelRef, SemanticDiagnostic> {
    let (metadata, parameters) = split_metadata(&declaration.parameters, &declaration.name)?;
    let (kind, spice_kind, allowed): (ComponentKind, &str, &[&str]) = match declaration.kind {
        ModelDeclKind::Diode => {
            if declaration.polarity.is_some() {
                return Err(error(
                    "NL-C010",
                    "diode models do not accept polarity",
                    Some(&declaration.name),
                    "polarity",
                ));
            }
            (
                ComponentKind::Diode,
                "D",
                &["is", "rs", "n", "cjo", "m", "vj", "tt"],
            )
        }
        ModelDeclKind::BJT => {
            let polarity = declaration.polarity.as_deref().ok_or_else(|| {
                error(
                    "NL-C010",
                    "BJT model requires npn or pnp polarity",
                    Some(&declaration.name),
                    "polarity",
                )
            })?;
            if polarity.eq_ignore_ascii_case("npn") {
                (
                    ComponentKind::BJT(BJTPolarity::NPN),
                    "NPN",
                    &["is", "bf", "vaf", "cje", "cjc", "tf", "tr"],
                )
            } else if polarity.eq_ignore_ascii_case("pnp") {
                (
                    ComponentKind::BJT(BJTPolarity::PNP),
                    "PNP",
                    &["is", "bf", "vaf", "cje", "cjc", "tf", "tr"],
                )
            } else {
                return Err(error(
                    "NL-C010",
                    format!("invalid BJT polarity '{polarity}'"),
                    Some(&declaration.name),
                    "polarity",
                ));
            }
        }
        ModelDeclKind::MOSFET => {
            let polarity = declaration.polarity.as_deref().ok_or_else(|| {
                error(
                    "NL-C010",
                    "MOSFET model requires nmos or pmos polarity",
                    Some(&declaration.name),
                    "polarity",
                )
            })?;
            if polarity.eq_ignore_ascii_case("nmos") {
                (
                    ComponentKind::MOSFET(FETPolarity::NMOS),
                    "NMOS",
                    &["vto", "kp", "lambda", "rd", "rs", "cgd", "cgs"],
                )
            } else if polarity.eq_ignore_ascii_case("pmos") {
                (
                    ComponentKind::MOSFET(FETPolarity::PMOS),
                    "PMOS",
                    &["vto", "kp", "lambda", "rd", "rs", "cgd", "cgs"],
                )
            } else {
                return Err(error(
                    "NL-C010",
                    format!("invalid MOSFET polarity '{polarity}'"),
                    Some(&declaration.name),
                    "polarity",
                ));
            }
        }
    };
    let parameters = canonical_parameters(&parameters, allowed, &declaration.name)?;
    if parameters.is_empty() {
        return Err(error(
            "NL-C010",
            "user model requires at least one typed electrical parameter",
            Some(&declaration.name),
            "parameters",
        ));
    }
    let body = parameters
        .iter()
        .map(|(name, value)| format!("{name}={}", format_spice_number(*value)))
        .collect::<Vec<_>>()
        .join(" ");
    let directive = format!(".model {} {spice_kind} ({body})", declaration.name);
    Ok(model_ref(
        &declaration.name,
        kind,
        ModelSource::UserDefined,
        ModelDefinition::Device { directive },
        metadata.source,
        metadata.license,
        metadata.version,
    ))
}

fn compile_user_subcircuit(declaration: &SubcircuitDecl) -> Result<ModelRef, SemanticDiagnostic> {
    if !declaration.kind.eq_ignore_ascii_case("opamp") {
        return Err(error(
            "NL-C012",
            format!("unsupported subcircuit kind '{}'", declaration.kind),
            Some(&declaration.name),
            "kind",
        ));
    }
    let expected = component_definition(&ComponentKind::OpAmp)
        .pins
        .iter()
        .map(|pin| pin.name.to_string())
        .collect::<Vec<_>>();
    if declaration.pins != expected {
        return Err(error(
            "NL-C012",
            format!(
                "opamp subcircuit '{}' pins {:?} must exactly match catalog order {:?}",
                declaration.name, declaration.pins, expected
            ),
            Some(&declaration.name),
            "pins",
        ));
    }
    let (metadata, parameters) = split_metadata(&declaration.parameters, &declaration.name)?;
    let values = canonical_parameters(&parameters, &["gain", "bandwidth"], &declaration.name)?;
    let gain = *values.get("gain").ok_or_else(|| {
        error(
            "NL-C012",
            "opamp subcircuit requires gain",
            Some(&declaration.name),
            "gain",
        )
    })?;
    let bandwidth = *values.get("bandwidth").ok_or_else(|| {
        error(
            "NL-C012",
            "opamp subcircuit requires bandwidth",
            Some(&declaration.name),
            "bandwidth",
        )
    })?;
    if gain <= 0.0 || bandwidth <= 0.0 {
        return Err(error(
            "NL-C012",
            "opamp gain and bandwidth must be positive",
            Some(&declaration.name),
            "parameters",
        ));
    }
    Ok(subcircuit_model_with_metadata(
        &declaration.name,
        ModelSource::UserDefined,
        metadata.source,
        metadata.license,
        metadata.version,
        gain,
        bandwidth,
    ))
}

struct Metadata {
    version: String,
    license: String,
    source: String,
}

fn split_metadata(
    values: &[crate::ast::NamedValue],
    model_name: &str,
) -> Result<(Metadata, Vec<crate::ast::NamedValue>), SemanticDiagnostic> {
    let mut metadata = BTreeMap::new();
    let mut parameters = Vec::new();
    for value in values {
        let key = value.name.to_ascii_lowercase();
        if matches!(key.as_str(), "version" | "license" | "source") {
            if metadata.insert(key.clone(), value.value.clone()).is_some() {
                return Err(error(
                    "NL-C010",
                    format!("duplicate metadata field '{}'", value.name),
                    Some(model_name),
                    &value.name,
                ));
            }
        } else {
            parameters.push(value.clone());
        }
    }
    let version = metadata.remove("version").ok_or_else(|| {
        error(
            "NL-C010",
            "user model/subcircuit requires version metadata",
            Some(model_name),
            "version",
        )
    })?;
    let license = metadata.remove("license").ok_or_else(|| {
        error(
            "NL-C010",
            "user model/subcircuit requires license metadata",
            Some(model_name),
            "license",
        )
    })?;
    let source = metadata
        .remove("source")
        .unwrap_or_else(|| "user-defined".to_string());
    for (field, value) in [
        ("version", &version),
        ("license", &license),
        ("source", &source),
    ] {
        if value.is_empty()
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || ". _+-/".contains(character))
        {
            return Err(error(
                "NL-C010",
                format!("unsafe or invalid {field} metadata '{value}'"),
                Some(model_name),
                field,
            ));
        }
    }
    Ok((
        Metadata {
            version,
            license,
            source,
        },
        parameters,
    ))
}

fn canonical_parameters(
    values: &[crate::ast::NamedValue],
    allowed: &[&str],
    model_name: &str,
) -> Result<BTreeMap<String, f64>, SemanticDiagnostic> {
    let mut parameters = BTreeMap::new();
    for value in values {
        let key = value.name.to_ascii_lowercase();
        if !allowed.contains(&key.as_str()) {
            return Err(error(
                "NL-C010",
                format!(
                    "unsupported parameter '{}' for '{}'; allowed parameters: {}",
                    value.name,
                    model_name,
                    allowed.join(", ")
                ),
                Some(model_name),
                &value.name,
            ));
        }
        let parsed = parse_si_value(&value.value).map_err(|reason| {
            error(
                "NL-C010",
                format!("invalid typed parameter '{}': {reason}", value.name),
                Some(model_name),
                &value.name,
            )
        })?;
        if parameters.insert(key, parsed).is_some() {
            return Err(error(
                "NL-C010",
                format!("duplicate parameter '{}'", value.name),
                Some(model_name),
                &value.name,
            ));
        }
    }
    Ok(parameters)
}

fn builtin_models() -> Vec<ModelRef> {
    vec![
        device_model(
            "2N3904",
            ComponentKind::BJT(BJTPolarity::NPN),
            ".model 2N3904 NPN (Is=6.734f Xti=3 Eg=1.11 Vaf=74.03 Bf=416.4 Ne=1.259 Ise=6.734f Ikf=66.78m Xtb=1.5 Br=.7371 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=3.638p Mjc=.3085 Vjc=.75 Fc=.5 Cje=4.493p Mje=.2593 Vje=.75 Tr=239.5n Tf=301.2p Itf=.4 Vtf=4 Xtf=2 Rb=10)",
        ),
        device_model(
            "2N3906",
            ComponentKind::BJT(BJTPolarity::PNP),
            ".model 2N3906 PNP (Is=1.41f Xti=3 Eg=1.11 Vaf=18.7 Bf=227.3 Ne=1.5 Ise=0 Ikf=80m Xtb=1.5 Br=4.977 Nc=2 Isc=0 Ikr=0 Rc=2.5 Cjc=9.728p Mjc=.5776 Vjc=.75 Fc=.5 Cje=8.063p Mje=.3677 Vje=.75 Tr=33.42n Tf=179.3p Itf=.4 Vtf=4 Xtf=6 Rb=10)",
        ),
        device_model(
            "2N2222",
            ComponentKind::BJT(BJTPolarity::NPN),
            ".model 2N2222 NPN (Is=14.34f Xti=3 Eg=1.11 Vaf=74.03 Bf=255.9 Ne=1.307 Ise=14.34f Ikf=.2847 Xtb=1.5 Br=6.092 Nc=2 Isc=0 Ikr=0 Rc=1 Cjc=7.306p Mjc=.3416 Vjc=.75 Fc=.5 Cje=22.01p Mje=.377 Vje=.75 Tr=46.91n Tf=411.1p Itf=.6 Vtf=1.7 Xtf=3 Rb=10)",
        ),
        verified_device_model(
            "NLANG_POWER_NPN_V1",
            ComponentKind::BJT(BJTPolarity::NPN),
            ".model NLANG_POWER_NPN_V1 NPN (Is=1e-12 Bf=80 Vaf=60 Cje=300p Cjc=150p Tf=1u Tr=5u)",
        ),
        verified_device_model(
            "NLANG_POWER_PNP_V1",
            ComponentKind::BJT(BJTPolarity::PNP),
            ".model NLANG_POWER_PNP_V1 PNP (Is=1e-12 Bf=80 Vaf=60 Cje=300p Cjc=150p Tf=1u Tr=5u)",
        ),
        device_model(
            "1N4148",
            ComponentKind::Diode,
            ".model 1N4148 D (Is=2.52n Rs=.568 N=1.752 Cjo=4p M=.4 tt=20n Iave=200m Vpk=75 mfg=OnSemi type=silicon)",
        ),
        device_model(
            "1N4007",
            ComponentKind::Diode,
            ".model 1N4007 D (Is=7.02767n Rs=0.0341512 N=1.80803 Cjo=10p M=0.3333 VJ=0.75 Iave=1 Vpk=1000 mfg=Motorola type=silicon)",
        ),
        device_model(
            "IRF540",
            ComponentKind::MOSFET(FETPolarity::NMOS),
            ".model IRF540 VDMOS (Rg=3 Vto=4.0 Rd=45m Rs=12m Rb=10m Kp=18 Cgdmax=2n Cgdmin=1.3n Cgs=1.7n Cjo=1n Is=2p mfg=IR)",
        ),
        verified_device_model(
            "NLANG_PMOS_V1",
            ComponentKind::MOSFET(FETPolarity::PMOS),
            ".model NLANG_PMOS_V1 PMOS (Vto=-4 Kp=8 Lambda=0.02 Rd=0.2 Rs=0.05 Cgd=1n Cgs=1.5n)",
        ),
        subcircuit_model(
            "NLANG_OPAMP_V1",
            ModelSource::Builtin,
            "NetLang verified generic opamp",
            "Apache-2.0",
            "1.0.0",
            200_000.0,
            1_000_000.0,
        ),
    ]
}

fn device_model(name: &str, kind: ComponentKind, directive: &str) -> ModelRef {
    model_ref(
        name,
        kind,
        ModelSource::Builtin,
        ModelDefinition::Device {
            directive: directive.to_string(),
        },
        "NetLang built-in model registry".to_string(),
        "legacy-provenance".to_string(),
        "1.0.0".to_string(),
    )
}

fn verified_device_model(name: &str, kind: ComponentKind, directive: &str) -> ModelRef {
    model_ref(
        name,
        kind,
        ModelSource::Builtin,
        ModelDefinition::Device {
            directive: directive.to_string(),
        },
        "NetLang verified generic model".to_string(),
        "Apache-2.0".to_string(),
        "1.0.0".to_string(),
    )
}

fn subcircuit_model(
    name: &str,
    source_kind: ModelSource,
    source: &str,
    license: &str,
    version: &str,
    gain: f64,
    bandwidth: f64,
) -> ModelRef {
    subcircuit_model_with_metadata(
        name,
        source_kind,
        source.to_string(),
        license.to_string(),
        version.to_string(),
        gain,
        bandwidth,
    )
}

#[allow(clippy::too_many_arguments)]
fn subcircuit_model_with_metadata(
    name: &str,
    source_kind: ModelSource,
    source: String,
    license: String,
    version: String,
    gain: f64,
    bandwidth: f64,
) -> ModelRef {
    let pole_capacitance = gain / (std::f64::consts::TAU * bandwidth);
    let directive = format!(
        ".subckt {name} in_p in_n vcc vee out\nEGAIN n_int 0 in_p in_n {}\nRPOLE n_int out 1\nCPOLE out 0 {}\nRLOAD out 0 1e9\n.ends {name}",
        format_spice_number(gain),
        format_spice_number(pole_capacitance)
    );
    model_ref(
        name,
        ComponentKind::OpAmp,
        source_kind,
        ModelDefinition::Subcircuit {
            directive,
            pins: vec!["in_p", "in_n", "vcc", "vee", "out"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
        source,
        license,
        version,
    )
}

#[allow(clippy::too_many_arguments)]
fn model_ref(
    name: &str,
    kind: ComponentKind,
    source_kind: ModelSource,
    definition: ModelDefinition,
    source: String,
    license: String,
    version: String,
) -> ModelRef {
    let hash = content_hash(definition_text(&definition));
    ModelRef {
        name: name.to_string(),
        kind,
        source: source_kind,
        definition,
        provenance: ModelProvenance {
            source,
            license,
            version,
            content_hash: hash,
            simulator: SIMULATOR_CAPABILITY.to_string(),
        },
    }
}

fn definition_text(definition: &ModelDefinition) -> &str {
    match definition {
        ModelDefinition::Device { directive } | ModelDefinition::Subcircuit { directive, .. } => {
            directive
        }
    }
}

fn content_hash(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("sha256:{digest:x}")
}

fn error(
    code: &str,
    message: impl Into<String>,
    component: Option<&str>,
    field: &str,
) -> SemanticDiagnostic {
    SemanticDiagnostic {
        code: code.to_string(),
        message: message.into(),
        component: component.map(str::to_string),
        field: Some(field.to_string()),
    }
}
