use crate::compiler::COMPILE_SCHEMA_VERSION;
use crate::ir::{ModelManifest, ModelSource};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};

pub const SHARE_SCHEMA_VERSION: &str = "kessetsu.share.v1";
pub const SHARE_LINK_REPORT_SCHEMA_VERSION: &str = "kessetsu.share_link.v1";
pub const SHARE_URL_ORIGIN: &str = "https://kessetsu.com/";
pub const SHARE_FRAGMENT_PREFIX: &str = "#kessetsu=1.";
pub const MAX_SHARE_SOURCE_BYTES: usize = 64 * 1024;
pub const MAX_SHARE_COMPRESSED_BYTES: usize = 64 * 1024;
pub const MAX_SHARE_ENVELOPE_BYTES: usize = 96 * 1024;
pub const MAX_SHARE_NAME_LENGTH: usize = 80;
pub const SHARE_URL_WARNING_CHARS: usize = 8192;

const COMPATIBLE_COMPILE_SCHEMAS: &[&str] = &[
    "kessetsu.compile.v4",
    "kessetsu.compile.v5",
    "kessetsu.compile.v6",
    "kessetsu.compile.v7",
    "kessetsu.compile.v8",
    COMPILE_SCHEMA_VERSION,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharePackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareEnvelope {
    pub schema_version: String,
    pub compile_schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub source: String,
    pub packages: Vec<SharePackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareLinkReport {
    pub schema_version: String,
    pub url: String,
    pub fragment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub source_bytes: usize,
    pub encoded_chars: usize,
    pub local_models_required: bool,
    pub warnings: Vec<String>,
}

fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

fn is_valid_semver(version: &str) -> bool {
    let (core, suffix) = match version.find(['-', '+']) {
        Some(idx) => (&version[..idx], Some(&version[idx + 1..])),
        None => (version, None),
    };
    let mut parts = core.split('.');
    let major = parts.next();
    let minor = parts.next();
    let patch = parts.next();
    if parts.next().is_some() {
        return false;
    }
    let core_ok = [major, minor, patch]
        .into_iter()
        .all(|part| part.is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())));
    if !core_ok {
        return false;
    }
    match suffix {
        Some(rest) => {
            !rest.is_empty()
                && rest
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        }
        None => true,
    }
}

pub fn normalize_name(name: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = name else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("share circuit name cannot be blank".to_string());
    }
    if trimmed
        .chars()
        .any(|ch| (ch as u32) < 32 || (ch as u32) == 127)
    {
        return Err("share circuit name cannot contain control characters".to_string());
    }
    if trimmed.chars().count() > MAX_SHARE_NAME_LENGTH {
        return Err(format!(
            "share circuit name exceeds maximum length of {MAX_SHARE_NAME_LENGTH} characters"
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn normalize_packages(packages: &[SharePackage]) -> Result<Vec<SharePackage>, String> {
    let mut normalized = Vec::with_capacity(packages.len());
    let mut seen = BTreeSet::new();
    for package in packages {
        let name = package.name.trim();
        let version = package.version.trim();
        if !is_valid_package_name(name) {
            return Err(format!("invalid share package name '{}'", package.name));
        }
        if !is_valid_semver(version) {
            return Err(format!(
                "invalid share package version '{}' for '{}'",
                package.version, name
            ));
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("duplicate share package '{name}'"));
        }
        normalized.push(SharePackage {
            name: name.to_string(),
            version: version.to_string(),
        });
    }
    normalized.sort_by(|left, right| {
        let left_key = format!("{}@{}", left.name, left.version);
        let right_key = format!("{}@{}", right.name, right.version);
        left_key.cmp(&right_key)
    });
    Ok(normalized)
}

fn packages_from_manifest(manifest: Option<&ModelManifest>) -> Result<Vec<SharePackage>, String> {
    let Some(manifest) = manifest else {
        return Ok(Vec::new());
    };
    let raw = manifest
        .packages
        .iter()
        .map(|package| SharePackage {
            name: package.name.clone(),
            version: package.version.clone(),
        })
        .collect::<Vec<_>>();
    normalize_packages(&raw)
}

fn build_envelope(
    source: &str,
    compile_schema_version: &str,
    manifest: Option<&ModelManifest>,
    name: Option<&str>,
) -> Result<(ShareEnvelope, Option<String>, usize), String> {
    if source.trim().is_empty() {
        return Err("cannot create share link for empty circuit source".to_string());
    }
    let source_bytes = source.len();
    if source_bytes > MAX_SHARE_SOURCE_BYTES {
        return Err(format!(
            "circuit source exceeds maximum share size of {MAX_SHARE_SOURCE_BYTES} bytes (got {source_bytes})"
        ));
    }
    if compile_schema_version.trim().is_empty() {
        return Err("compile schema version must not be empty".to_string());
    }
    let normalized_name = normalize_name(name)?;
    let packages = packages_from_manifest(manifest)?;
    let envelope = ShareEnvelope {
        schema_version: SHARE_SCHEMA_VERSION.to_string(),
        compile_schema_version: compile_schema_version.to_string(),
        name: normalized_name.clone(),
        source: source.to_string(),
        packages,
    };
    Ok((envelope, normalized_name, source_bytes))
}

fn encode_envelope_fragment(envelope: &ShareEnvelope) -> Result<String, String> {
    let envelope_json = serde_json::to_vec(envelope)
        .map_err(|err| format!("failed to serialize share envelope: {err}"))?;
    if envelope_json.len() > MAX_SHARE_ENVELOPE_BYTES {
        return Err(format!(
            "serialized share envelope exceeds maximum size of {MAX_SHARE_ENVELOPE_BYTES} bytes"
        ));
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&envelope_json)
        .map_err(|err| format!("failed to gzip-compress share envelope: {err}"))?;
    let compressed = encoder
        .finish()
        .map_err(|err| format!("failed to finalize gzip share stream: {err}"))?;
    if compressed.len() > MAX_SHARE_COMPRESSED_BYTES {
        return Err(format!(
            "compressed share payload exceeds maximum size of {MAX_SHARE_COMPRESSED_BYTES} bytes"
        ));
    }
    Ok(format!(
        "{SHARE_FRAGMENT_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(compressed)
    ))
}

pub fn encode_share_fragment(
    source: &str,
    compile_schema_version: &str,
    manifest: Option<&ModelManifest>,
    name: Option<&str>,
) -> Result<String, String> {
    let (envelope, _, _) = build_envelope(source, compile_schema_version, manifest, name)?;
    encode_envelope_fragment(&envelope)
}

pub fn encode_share_url(
    source: &str,
    compile_schema_version: &str,
    manifest: Option<&ModelManifest>,
    name: Option<&str>,
) -> Result<ShareLinkReport, String> {
    let (envelope, normalized_name, source_bytes) =
        build_envelope(source, compile_schema_version, manifest, name)?;
    let fragment = encode_envelope_fragment(&envelope)?;
    let url = format!("{SHARE_URL_ORIGIN}{fragment}");
    let encoded_chars = url.chars().count();

    let external_models: Vec<&str> = manifest
        .map(|manifest| {
            manifest
                .models
                .iter()
                .filter(|entry| {
                    entry.external.is_some() || matches!(entry.source, ModelSource::External)
                })
                .map(|entry| entry.name.as_str())
                .collect()
        })
        .unwrap_or_default();
    let local_models_required = !external_models.is_empty();

    let mut warnings = Vec::new();
    if local_models_required {
        warnings.push(format!(
            "Circuit references external local model(s) ({}); recipients opening this share link in a browser will need local model files supplied.",
            external_models.join(", ")
        ));
    }
    if encoded_chars > SHARE_URL_WARNING_CHARS {
        warnings.push(format!(
            "Share URL is {encoded_chars} characters (over {SHARE_URL_WARNING_CHARS}); long URLs may be truncated by some terminals or chat clients."
        ));
    }

    Ok(ShareLinkReport {
        schema_version: SHARE_LINK_REPORT_SCHEMA_VERSION.to_string(),
        url,
        fragment,
        name: normalized_name,
        source_bytes,
        encoded_chars,
        local_models_required,
        warnings,
    })
}

pub fn decode_share_fragment(
    fragment: &str,
    expected_compile_schema: &str,
) -> Result<Option<ShareEnvelope>, String> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized_fragment = if let Some(hash_idx) = trimmed.find('#') {
        &trimmed[hash_idx..]
    } else {
        trimmed
    };
    if !normalized_fragment.starts_with("#kessetsu=") {
        return Ok(None);
    }
    let Some(payload) = normalized_fragment.strip_prefix(SHARE_FRAGMENT_PREFIX) else {
        return Err("unsupported kessetsu share fragment version".to_string());
    };
    if payload.is_empty() {
        return Err("share fragment payload is empty".to_string());
    }

    let compressed = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|err| format!("invalid base64url share payload: {err}"))?;
    if compressed.len() > MAX_SHARE_COMPRESSED_BYTES {
        return Err(format!(
            "compressed share payload exceeds maximum size of {MAX_SHARE_COMPRESSED_BYTES} bytes"
        ));
    }

    let decoder = GzDecoder::new(compressed.as_slice());
    let mut bounded = decoder.take((MAX_SHARE_ENVELOPE_BYTES + 1) as u64);
    let mut decompressed = Vec::new();
    bounded
        .read_to_end(&mut decompressed)
        .map_err(|err| format!("failed to decompress gzip share payload: {err}"))?;
    if decompressed.len() > MAX_SHARE_ENVELOPE_BYTES {
        return Err(format!(
            "decompressed share payload exceeds maximum size of {MAX_SHARE_ENVELOPE_BYTES} bytes"
        ));
    }

    let envelope: ShareEnvelope = serde_json::from_slice(&decompressed)
        .map_err(|err| format!("invalid share envelope JSON: {err}"))?;
    if envelope.schema_version != SHARE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported share schema version '{}'",
            envelope.schema_version
        ));
    }
    let schema_compatible = envelope.compile_schema_version == expected_compile_schema
        || (expected_compile_schema == COMPILE_SCHEMA_VERSION
            && COMPATIBLE_COMPILE_SCHEMAS.contains(&envelope.compile_schema_version.as_str()));
    if !schema_compatible {
        return Err(format!(
            "incompatible share compile schema version '{}' (expected '{expected_compile_schema}')",
            envelope.compile_schema_version
        ));
    }
    if envelope.source.trim().is_empty() {
        return Err("share envelope source cannot be empty".to_string());
    }
    if envelope.source.len() > MAX_SHARE_SOURCE_BYTES {
        return Err(format!(
            "share envelope source exceeds maximum size of {MAX_SHARE_SOURCE_BYTES} bytes"
        ));
    }
    let normalized_name = match envelope.name.as_deref() {
        Some(raw) => {
            if raw != raw.trim() {
                return Err(
                    "share circuit name must not have leading or trailing whitespace".to_string(),
                );
            }
            normalize_name(Some(raw))?
        }
        None => None,
    };
    let normalized_packages = normalize_packages(&envelope.packages)?;

    Ok(Some(ShareEnvelope {
        schema_version: envelope.schema_version,
        compile_schema_version: envelope.compile_schema_version,
        name: normalized_name,
        source: envelope.source,
        packages: normalized_packages,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        ComponentKind, ExternalModelMetadata, ModelManifestEntry, ModelProvenance,
        RedistributionPolicy, ResolvedModelPackage, SimulatorCompatibility,
    };

    #[test]
    fn round_trips_utf8_source_name_and_sorted_packages() {
        let manifest = ModelManifest {
            schema_version: crate::models::MODEL_MANIFEST_SCHEMA_VERSION.to_string(),
            packages: vec![
                ResolvedModelPackage {
                    name: "zeta_models".to_string(),
                    version: "2.1.0".to_string(),
                    license: "MIT".to_string(),
                    content_hash: "hash-z".to_string(),
                },
                ResolvedModelPackage {
                    name: "alpha_pack".to_string(),
                    version: "1.0.0-rc.1".to_string(),
                    license: "Apache-2.0".to_string(),
                    content_hash: "hash-a".to_string(),
                },
            ],
            models: Vec::new(),
        };
        let source = "// Mühlberger αβγ filter\nV1 IN GND DC 5V\nR1 IN OUT 1k\nC1 OUT GND 100n\n";
        let report = encode_share_url(
            source,
            COMPILE_SCHEMA_VERSION,
            Some(&manifest),
            Some("  RC Filter Ω  "),
        )
        .expect("share URL should encode");

        assert_eq!(report.schema_version, SHARE_LINK_REPORT_SCHEMA_VERSION);
        assert!(report.url.starts_with("https://kessetsu.com/#kessetsu=1."));
        assert_eq!(
            report.url,
            format!("https://kessetsu.com/{}", report.fragment)
        );
        assert_eq!(report.name.as_deref(), Some("RC Filter Ω"));
        assert_eq!(report.source_bytes, source.len());
        assert_eq!(report.encoded_chars, report.url.chars().count());
        assert!(!report.local_models_required);
        assert!(report.warnings.is_empty());

        let decoded = decode_share_fragment(&report.url, COMPILE_SCHEMA_VERSION)
            .expect("share URL should decode")
            .expect("share fragment should yield envelope");
        assert_eq!(decoded.schema_version, SHARE_SCHEMA_VERSION);
        assert_eq!(decoded.compile_schema_version, COMPILE_SCHEMA_VERSION);
        assert_eq!(decoded.name.as_deref(), Some("RC Filter Ω"));
        assert_eq!(decoded.source, source);
        assert_eq!(
            decoded.packages,
            vec![
                SharePackage {
                    name: "alpha_pack".to_string(),
                    version: "1.0.0-rc.1".to_string(),
                },
                SharePackage {
                    name: "zeta_models".to_string(),
                    version: "2.1.0".to_string(),
                },
            ]
        );
    }

    #[test]
    fn rejects_invalid_names_and_empty_or_oversized_source() {
        let valid = "V1 IN GND DC 5V\nR1 IN GND 1k\n";
        assert!(encode_share_fragment(valid, COMPILE_SCHEMA_VERSION, None, Some("   ")).is_err());
        assert!(
            encode_share_fragment(valid, COMPILE_SCHEMA_VERSION, None, Some("bad\nname")).is_err()
        );
        assert!(
            encode_share_fragment(valid, COMPILE_SCHEMA_VERSION, None, Some("bad\x7fname"))
                .is_err()
        );
        let over_80 = "x".repeat(MAX_SHARE_NAME_LENGTH + 1);
        assert!(
            encode_share_fragment(valid, COMPILE_SCHEMA_VERSION, None, Some(&over_80)).is_err()
        );
        assert!(encode_share_fragment("  \n ", COMPILE_SCHEMA_VERSION, None, None).is_err());

        let oversized_source = "a".repeat(MAX_SHARE_SOURCE_BYTES + 1);
        assert!(
            encode_share_fragment(&oversized_source, COMPILE_SCHEMA_VERSION, None, None).is_err()
        );
    }

    #[test]
    fn accepts_legacy_compile_schemas_v4_through_v9() {
        let source = "V1 IN GND DC 5V\nR1 IN GND 1k\n";
        for legacy in [
            "kessetsu.compile.v4",
            "kessetsu.compile.v5",
            "kessetsu.compile.v6",
            "kessetsu.compile.v7",
            "kessetsu.compile.v8",
            COMPILE_SCHEMA_VERSION,
        ] {
            let fragment = encode_share_fragment(source, legacy, None, None)
                .expect("legacy compile schema should encode");
            let decoded = decode_share_fragment(&fragment, COMPILE_SCHEMA_VERSION)
                .expect("legacy compile schema should decode against v9")
                .expect("envelope present");
            assert_eq!(decoded.compile_schema_version, legacy);
        }

        let too_old = encode_share_fragment(source, "kessetsu.compile.v3", None, None)
            .expect("encode v3 for negative test");
        assert!(decode_share_fragment(&too_old, COMPILE_SCHEMA_VERSION).is_err());
    }

    #[test]
    fn guards_against_decompression_bombs_and_unrelated_fragments() {
        assert_eq!(
            decode_share_fragment("", COMPILE_SCHEMA_VERSION).unwrap(),
            None
        );
        assert_eq!(
            decode_share_fragment("#section-1", COMPILE_SCHEMA_VERSION).unwrap(),
            None
        );
        assert!(decode_share_fragment("#kessetsu=2.abc", COMPILE_SCHEMA_VERSION).is_err());

        // Construct an envelope-sized compressed payload that decompresses beyond 96 KiB.
        let bomb_payload = vec![b'A'; MAX_SHARE_ENVELOPE_BYTES + 512];
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&bomb_payload).unwrap();
        let compressed = encoder.finish().unwrap();
        assert!(compressed.len() <= MAX_SHARE_COMPRESSED_BYTES);
        let fragment = format!(
            "{SHARE_FRAGMENT_PREFIX}{}",
            URL_SAFE_NO_PAD.encode(compressed)
        );
        let err = decode_share_fragment(&fragment, COMPILE_SCHEMA_VERSION)
            .expect_err("bomb payload should be rejected");
        assert!(err.contains("exceeds maximum size"));
    }

    #[test]
    fn warns_when_external_local_models_are_referenced() {
        let manifest = ModelManifest {
            schema_version: crate::models::MODEL_MANIFEST_SCHEMA_VERSION.to_string(),
            packages: Vec::new(),
            models: vec![ModelManifestEntry {
                name: "MY_EXT_OPAMP".to_string(),
                kind: ComponentKind::OpAmp,
                source: ModelSource::External,
                provenance: ModelProvenance {
                    source: "local".to_string(),
                    license: "Proprietary".to_string(),
                    version: "1.0.0".to_string(),
                    content_hash: "abc".to_string(),
                    simulator: "ngspice".to_string(),
                },
                external: Some(ExternalModelMetadata {
                    resource: "models/opamp.lib".to_string(),
                    entry: "MY_EXT_OPAMP".to_string(),
                    pins: vec![
                        "INP".into(),
                        "INN".into(),
                        "VCC".into(),
                        "VEE".into(),
                        "OUT".into(),
                    ],
                    simulator: SimulatorCompatibility::Ngspice,
                    redistribution: RedistributionPolicy::Prohibited,
                    instance_parameters: Vec::new(),
                }),
            }],
        };
        let source = "V1 VCC GND DC 15V\nR1 VCC GND 1k\n";
        let report = encode_share_url(source, COMPILE_SCHEMA_VERSION, Some(&manifest), None)
            .expect("should still create share link with warning");
        assert!(report.local_models_required);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("MY_EXT_OPAMP"));
    }
}
