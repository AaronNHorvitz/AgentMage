//! Validation of caller-supplied output from the pinned offline Mermaid renderer.

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde::{Deserialize, Serialize};

use crate::word_ooxml::word_sha256;

const MAX_MERMAID_BYTES: usize = 1_024 * 1_024;
const MAX_SVG_BYTES: usize = 8 * 1_024 * 1_024;
const EXPECTED_PACKAGE: &str = "@mermaid-js/mermaid-cli";
const EXPECTED_VERSION: &str = "11.16.0";
const EXPECTED_LICENSE: &str = "MIT";

/// Exact provenance for one local Mermaid renderer package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfOfflineDiagramAdmission {
    /// Package name.
    pub package_name: String,
    /// Exact package version.
    pub package_version: String,
    /// Declared package license.
    pub package_license: String,
    /// SHA-256 of the admitted executable entry point.
    pub renderer_binary_sha256: String,
    /// SHA-256 of the exact lock file governing the renderer closure.
    pub lockfile_sha256: String,
}

/// Exact output from one externally executed local-only Mermaid render.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfOfflineDiagramObservation {
    /// Stable observation identity.
    pub observation_id: String,
    /// Exact renderer admission.
    pub admission: PdfOfflineDiagramAdmission,
    /// SHA-256 of the exact Mermaid source.
    pub source_sha256: String,
    /// Exact generated SVG bytes.
    pub svg: Vec<u8>,
    /// SHA-256 of the exact SVG bytes.
    pub svg_sha256: String,
    /// True only when the external adapter denied network access for the entire render.
    pub network_denied: bool,
    /// True only when the external adapter observed no remote asset request.
    pub remote_assets_absent: bool,
    /// Exit code from the local renderer process.
    pub renderer_exit_code: i32,
}

/// Content-minimized validated offline-diagram projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfOfflineDiagramProjection {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable observation identity.
    pub observation_id: String,
    /// SHA-256 of the exact Mermaid source.
    pub source_sha256: String,
    /// SHA-256 of the exact SVG output.
    pub svg_sha256: String,
    /// SHA-256 of the canonical admission record.
    pub admission_sha256: String,
    /// True only when the SVG passed the closed passive-content parser.
    pub passive_svg_validated: bool,
    /// True only when provenance, execution, network, and SVG checks pass.
    pub safe_for_local_embedding: bool,
    /// False because validation does not launch the renderer.
    pub execution_performed: bool,
    /// False because validation does not access a network.
    pub network_access_performed: bool,
    /// False because validation does not write files.
    pub filesystem_effect_performed: bool,
}

/// Stable offline diagram validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfOfflineDiagramError {
    /// Identity, admission, digest, or execution observation is invalid.
    InvalidObservation,
    /// Source or SVG exceeds the closed resource profile.
    ResourceLimit,
    /// SVG is malformed or does not have an SVG root.
    MalformedSvg,
    /// SVG contains active content, remote references, or unsafe URL-bearing attributes.
    ActiveOrRemoteContent,
}

impl PdfOfflineDiagramError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidObservation => "pdf_diagram.observation.invalid",
            Self::ResourceLimit => "pdf_diagram.resource.limit",
            Self::MalformedSvg => "pdf_diagram.svg.malformed",
            Self::ActiveOrRemoteContent => "pdf_diagram.svg.active_or_remote",
        }
    }
}

impl std::fmt::Display for PdfOfflineDiagramError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PdfOfflineDiagramError {}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn unsafe_uri(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.starts_with("http:")
        || normalized.starts_with("https:")
        || normalized.starts_with("file:")
        || normalized.starts_with("ftp:")
        || normalized.starts_with("javascript:")
        || normalized.starts_with("data:text/html")
        || normalized.starts_with("//")
        || (normalized.contains("url(") && !normalized.contains("url(#"))
}

fn validate_attributes(
    event: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
) -> Result<(), PdfOfflineDiagramError> {
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.map_err(|_| PdfOfflineDiagramError::MalformedSvg)?;
        let name = local_name(attribute.key.as_ref()).to_ascii_lowercase();
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|_| PdfOfflineDiagramError::MalformedSvg)?;
        if name.starts_with(b"on")
            || (matches!(name.as_slice(), b"href" | b"src" | b"style") && unsafe_uri(&value))
        {
            return Err(PdfOfflineDiagramError::ActiveOrRemoteContent);
        }
    }
    Ok(())
}

fn validate_passive_svg(svg: &[u8]) -> Result<(), PdfOfflineDiagramError> {
    let mut reader = Reader::from_reader(svg);
    reader.config_mut().trim_text(false);
    let mut root_seen = false;
    let mut depth = 0_usize;
    loop {
        match reader
            .read_event()
            .map_err(|_| PdfOfflineDiagramError::MalformedSvg)?
        {
            Event::Start(event) => {
                let name = local_name(event.name().as_ref()).to_ascii_lowercase();
                if !root_seen {
                    if name != b"svg" {
                        return Err(PdfOfflineDiagramError::MalformedSvg);
                    }
                    root_seen = true;
                }
                if matches!(
                    name.as_slice(),
                    b"script" | b"foreignobject" | b"iframe" | b"object" | b"embed"
                ) {
                    return Err(PdfOfflineDiagramError::ActiveOrRemoteContent);
                }
                validate_attributes(&event, &reader)?;
                depth = depth
                    .checked_add(1)
                    .ok_or(PdfOfflineDiagramError::ResourceLimit)?;
                if depth > 4_096 {
                    return Err(PdfOfflineDiagramError::ResourceLimit);
                }
            }
            Event::Empty(event) => {
                let name = local_name(event.name().as_ref()).to_ascii_lowercase();
                if !root_seen {
                    if name != b"svg" {
                        return Err(PdfOfflineDiagramError::MalformedSvg);
                    }
                    root_seen = true;
                }
                if matches!(
                    name.as_slice(),
                    b"script" | b"foreignobject" | b"iframe" | b"object" | b"embed"
                ) {
                    return Err(PdfOfflineDiagramError::ActiveOrRemoteContent);
                }
                validate_attributes(&event, &reader)?;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::DocType(_) => return Err(PdfOfflineDiagramError::ActiveOrRemoteContent),
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen {
        return Err(PdfOfflineDiagramError::MalformedSvg);
    }
    Ok(())
}

/// Validates exact output supplied by the pinned local Mermaid adapter without executing it.
pub fn validate_pdf_offline_diagram(
    source: &[u8],
    observation: &PdfOfflineDiagramObservation,
) -> Result<PdfOfflineDiagramProjection, PdfOfflineDiagramError> {
    if source.is_empty()
        || source.len() > MAX_MERMAID_BYTES
        || observation.svg.len() > MAX_SVG_BYTES
    {
        return Err(PdfOfflineDiagramError::ResourceLimit);
    }
    if !valid_identifier(&observation.observation_id)
        || observation.admission.package_name != EXPECTED_PACKAGE
        || observation.admission.package_version != EXPECTED_VERSION
        || observation.admission.package_license != EXPECTED_LICENSE
        || !valid_sha256(&observation.admission.renderer_binary_sha256)
        || !valid_sha256(&observation.admission.lockfile_sha256)
        || !valid_sha256(&observation.source_sha256)
        || !valid_sha256(&observation.svg_sha256)
        || word_sha256(source) != observation.source_sha256
        || word_sha256(&observation.svg) != observation.svg_sha256
        || !observation.network_denied
        || !observation.remote_assets_absent
        || observation.renderer_exit_code != 0
    {
        return Err(PdfOfflineDiagramError::InvalidObservation);
    }
    validate_passive_svg(&observation.svg)?;
    let admission = serde_json::to_vec(&observation.admission)
        .map_err(|_| PdfOfflineDiagramError::InvalidObservation)?;
    Ok(PdfOfflineDiagramProjection {
        schema_version: CONTRACT_SCHEMA_VERSION,
        observation_id: observation.observation_id.clone(),
        source_sha256: observation.source_sha256.clone(),
        svg_sha256: observation.svg_sha256.clone(),
        admission_sha256: word_sha256(&admission),
        passive_svg_validated: true,
        safe_for_local_embedding: true,
        execution_performed: false,
        network_access_performed: false,
        filesystem_effect_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(source: &[u8], svg: Vec<u8>) -> PdfOfflineDiagramObservation {
        PdfOfflineDiagramObservation {
            observation_id: "diagram-001".to_owned(),
            admission: PdfOfflineDiagramAdmission {
                package_name: EXPECTED_PACKAGE.to_owned(),
                package_version: EXPECTED_VERSION.to_owned(),
                package_license: EXPECTED_LICENSE.to_owned(),
                renderer_binary_sha256: "a".repeat(64),
                lockfile_sha256: "b".repeat(64),
            },
            source_sha256: word_sha256(source),
            svg_sha256: word_sha256(&svg),
            svg,
            network_denied: true,
            remote_assets_absent: true,
            renderer_exit_code: 0,
        }
    }

    #[test]
    fn accepts_hash_bound_passive_local_svg_without_effects() {
        let source = b"flowchart LR\n  A --> B\n";
        let observation = observation(
            source,
            br##"<svg xmlns="http://www.w3.org/2000/svg"><defs><style>.x{fill:red}</style></defs><path id="x" d="M0 0"/><use href="#x"/></svg>"##.to_vec(),
        );
        let projection = validate_pdf_offline_diagram(source, &observation).expect("projection");
        assert!(projection.passive_svg_validated);
        assert!(projection.safe_for_local_embedding);
        assert!(!projection.execution_performed);
        assert!(!projection.network_access_performed);
        assert!(!projection.filesystem_effect_performed);
    }

    #[test]
    fn rejects_active_remote_and_doctype_svg() {
        let source = b"flowchart LR\n  A --> B\n";
        for svg in [
            br#"<svg><script>alert(1)</script></svg>"#.as_slice(),
            br#"<svg><image href="https://example.invalid/a.png"/></svg>"#.as_slice(),
            br#"<!DOCTYPE svg><svg/>"#.as_slice(),
            br#"<svg><path onclick="alert(1)"/></svg>"#.as_slice(),
        ] {
            assert_eq!(
                validate_pdf_offline_diagram(source, &observation(source, svg.to_vec())),
                Err(PdfOfflineDiagramError::ActiveOrRemoteContent)
            );
        }
    }

    #[test]
    fn rejects_digest_package_network_and_exit_tampering() {
        let source = b"flowchart LR\n  A --> B\n";
        let mut item = observation(source, br#"<svg/>"#.to_vec());
        item.source_sha256 = "c".repeat(64);
        assert_eq!(
            validate_pdf_offline_diagram(source, &item),
            Err(PdfOfflineDiagramError::InvalidObservation)
        );
        let mut item = observation(source, br#"<svg/>"#.to_vec());
        item.network_denied = false;
        assert_eq!(
            validate_pdf_offline_diagram(source, &item),
            Err(PdfOfflineDiagramError::InvalidObservation)
        );
        let mut item = observation(source, br#"<svg/>"#.to_vec());
        item.renderer_exit_code = 1;
        assert_eq!(
            validate_pdf_offline_diagram(source, &item),
            Err(PdfOfflineDiagramError::InvalidObservation)
        );
    }
}
