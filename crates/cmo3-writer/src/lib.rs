//! Minimal-to-fidelity CMO3 writer (AGENT.5).
//!
//! The writer performs **serialization only**: it consumes the Live2D IR,
//! the recovered project and the recovered keyform model and emits a `.cmo3`
//! container. It never guesses hierarchy, reconstructs keyforms or invents
//! bindings (work order section 0).
//!
//! Pipeline (work order section 6):
//!
//! ```text
//! semantic inputs -> Cmo3Project -> identity allocation -> XML document
//!                 -> CAFF entries -> CAFF encoder -> .cmo3 bytes
//! ```
//!
//! The crate must never depend on `moc3-ingest`: binary layout details of the
//! runtime format are unknown here.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod caff;
pub mod ids;
pub mod map;
pub mod model;
pub mod png;
pub mod profile;
pub mod serialize;
pub mod textures;
pub mod validate;
pub mod xml;

use caff::CaffEntry;
use map::WriteError;
use textures::TextureAssets;

/// Result of a minimal CMO3 write.
#[derive(Debug, Clone)]
pub struct WrittenCmo3 {
    /// Complete `.cmo3` bytes.
    pub bytes: Vec<u8>,
    /// The serialized `main.xml` text.
    pub xml: String,
    /// Image-pipeline trace (one step list per mesh).
    pub pipeline: Vec<validate::PipelineStep>,
    /// Typed-validation findings (warnings only; fatals abort).
    pub findings: Vec<validate::Finding>,
    /// Object pool size.
    pub pool_size: usize,
    /// GUID atoms allocated.
    pub guid_count: usize,
}

/// Write the minimal MODEL_IMAGE-mode `.cmo3`.
///
/// Pipeline (work order section 77): validate the typed project, serialize
/// `main.xml`, scan the XML for id/reference integrity, encode the CAFF
/// archive with the synthetic PNG entries and verify the archive by
/// decoding it again. Any Fatal finding aborts before bytes are returned.
pub fn write_minimal_cmo3(
    project: &model::Cmo3Project,
    options: &serialize::IdentityOptions,
    textures: &TextureAssets,
) -> Result<WrittenCmo3, WriteError> {
    let findings = validate::validate_typed(project);
    if findings
        .iter()
        .any(|finding| finding.severity == validate::FindingSeverity::Fatal)
    {
        let first = findings
            .iter()
            .find(|finding| finding.severity == validate::FindingSeverity::Fatal)
            .map(|finding| format!("{}: {}", finding.code, finding.detail))
            .unwrap_or_default();
        return Err(WriteError {
            code: "ValidationFailed",
            message: format!("typed project validation failed; first: {first}"),
        });
    }

    let serialized = serialize::serialize(project, options);
    let scan = validate::scan_xml(&serialized.xml);
    if !scan.is_valid() {
        let sample: Vec<String> = scan
            .dangling
            .iter()
            .take(5)
            .map(|(tag, target)| format!("{tag}->#{target}"))
            .collect();
        return Err(WriteError {
            code: "XmlValidationFailed",
            message: format!(
                "serialized XML is inconsistent (duplicate ids: {}, dangling refs: {}, GUID mismatches: {}; sample: {})",
                scan.duplicate_ids.len(),
                scan.dangling.len(),
                scan.guid_type_mismatches.len(),
                sample.join(", ")
            ),
        });
    }

    let mut entries: Vec<CaffEntry> = Vec::new();
    entries.push(CaffEntry::raw(
        caff::MAIN_XML_PATH,
        caff::MAIN_XML_TAG,
        serialized.xml.as_bytes().to_vec(),
    ));
    for resource in &project.image_resources {
        let asset = textures
            .assets
            .iter()
            .find(|asset| TextureAssets::archive_name(asset.page) == resource.archive_name);
        let Some(asset) = asset else {
            return Err(WriteError {
                code: "MissingTextureAsset",
                message: format!(
                    "archive entry '{}' has no supplied asset bytes",
                    resource.archive_name
                ),
            });
        };
        entries.push(CaffEntry::raw(
            resource.archive_name.clone(),
            "",
            asset.bytes.clone(),
        ));
    }
    if entries.len() > caff::MAX_ENTRIES {
        return Err(WriteError {
            code: "TooManyEntries",
            message: "archive entry cap exceeded".to_string(),
        });
    }

    let bytes = caff::encode(options.archive_key, &entries).map_err(|error| WriteError {
        code: "CaffEncodeFailed",
        message: error.to_string(),
    })?;
    let decoded = caff::decode_strict_raw(&bytes).map_err(|error| WriteError {
        code: "CaffVerifyFailed",
        message: error.to_string(),
    })?;
    if decoded.payload(caff::MAIN_XML_PATH) != Some(serialized.xml.as_bytes()) {
        return Err(WriteError {
            code: "CaffVerifyFailed",
            message: "decoded main.xml differs from the serialized document".to_string(),
        });
    }
    let validation = caff::validate_archive(&bytes).map_err(|error| WriteError {
        code: "CaffVerifyFailed",
        message: error.to_string(),
    })?;
    if !validation.is_valid() {
        return Err(WriteError {
            code: "CaffVerifyFailed",
            message: "archive validation reported fatal findings".to_string(),
        });
    }

    let pipeline = validate::pipeline_trace(project);
    Ok(WrittenCmo3 {
        bytes,
        xml: serialized.xml,
        pipeline,
        findings,
        pool_size: serialized.pool_trace.len(),
        guid_count: serialized.guid_count,
    })
}
