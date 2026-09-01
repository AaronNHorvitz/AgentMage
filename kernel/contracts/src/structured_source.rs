//! Interface-neutral contracts for bounded structured source extraction.

use serde::{Deserialize, Serialize};

use crate::{CONTRACT_SCHEMA_VERSION, WorkspacePath};

/// Exact WordprocessingML media type admitted by the first structured extractor.
pub const DOCX_MEDIA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// Closed structured source family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredSourceFormat {
    /// Office Open XML WordprocessingML package.
    Docx,
    /// Portable Document Format, reserved for its owning story.
    Pdf,
    /// Office Open XML spreadsheet package, reserved for its owning story.
    Xlsx,
}

/// Closed canonical section kind shared by structured-document adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredSourceSectionKind {
    /// Complete document root.
    Document,
    /// Ordinary paragraph content.
    Paragraph,
    /// Styled heading content.
    Heading,
    /// Numbered or bulleted list item.
    ListItem,
    /// Table container.
    Table,
    /// Table row.
    TableRow,
    /// Table cell content.
    TableCell,
    /// Repeating header content.
    Header,
    /// Repeating footer content.
    Footer,
    /// Note-like source content.
    Note,
    /// Word comment content.
    Comment,
    /// Tracked insertion content.
    TrackedInsertion,
    /// Tracked deletion content.
    TrackedDeletion,
    /// Hyperlink content or relationship.
    Link,
    /// Image or drawing observation.
    Image,
    /// Package relationship observation.
    Relationship,
    /// Preserved structure whose semantics are not interpreted.
    Unsupported,
}

/// Exact provenance available for one canonical structured section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredSourceProvenance {
    /// Exact package part, page object, sheet, or equivalent source partition.
    pub source_part: String,
    /// Deterministic source-local structural path.
    pub structural_path: String,
    /// Inclusive source-part UTF-8 byte offset when available.
    pub start_byte: Option<u64>,
    /// Exclusive source-part UTF-8 byte offset when available.
    pub end_byte_exclusive: Option<u64>,
    /// One-based paragraph identity when deterministically available.
    pub paragraph: Option<u32>,
    /// One-based run identity within the paragraph when deterministically available.
    pub run: Option<u32>,
    /// One-based table identity when deterministically available.
    pub table: Option<u32>,
    /// One-based row identity within the table when deterministically available.
    pub row: Option<u32>,
    /// One-based cell identity within the row when deterministically available.
    pub cell: Option<u32>,
    /// Exact relationship identity when present.
    pub relationship_id: Option<String>,
    /// One-based rendered page only when supplied by admitted native evidence.
    pub rendered_page: Option<u32>,
}

/// One canonical, ordered structured-document section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredSourceSection {
    /// Stable source-bound section identity.
    pub section_id: String,
    /// Stable parent identity, absent only for the document root.
    pub parent_section_id: Option<String>,
    /// Stable zero-based order in the canonical projection.
    pub ordinal: u32,
    /// Closed semantic section class.
    pub kind: StructuredSourceSectionKind,
    /// Exact inert text; empty for content-free structural observations.
    pub content: String,
    /// Exact source provenance without path or URI authority.
    pub provenance: StructuredSourceProvenance,
}

/// One visible fidelity, omission, or policy warning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredSourceWarning {
    /// Stable source-bound warning identity.
    pub warning_id: String,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Exact source part when the warning is part-specific.
    pub source_part: Option<String>,
    /// True when the original remains necessary for complete fidelity.
    pub original_remains_authoritative: bool,
}

/// Exact bounded request shared by all structured source extractors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredSourceExtractionRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable caller-owned source identity.
    pub source_id: String,
    /// Exact declared media type.
    pub media_type: String,
    /// Exact source digest.
    pub source_sha256: String,
    /// Validated descriptive workspace path carrying no file descriptor or authority.
    pub source_path: WorkspacePath,
    /// Hard maximum canonical sections.
    pub maximum_sections: u32,
    /// Hard maximum total emitted inert content bytes.
    pub maximum_output_bytes: u64,
}

impl StructuredSourceExtractionRequest {
    /// Returns whether the request uses the only supported contract version.
    #[must_use]
    pub const fn supported_version(&self) -> bool {
        self.schema_version == CONTRACT_SCHEMA_VERSION
    }
}

/// Complete authority-free structured source projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredSourceExtraction {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable source identity.
    pub source_id: String,
    /// Closed source family.
    pub format: StructuredSourceFormat,
    /// Exact source digest.
    pub source_sha256: String,
    /// Exact extractor implementation identity digest.
    pub extractor_sha256: String,
    /// Canonical sections in stable order.
    pub sections: Vec<StructuredSourceSection>,
    /// Visible fidelity and policy warnings in stable order.
    pub warnings: Vec<StructuredSourceWarning>,
    /// Number of canonical text bytes emitted.
    pub output_bytes: u64,
    /// True only when no section or content was omitted by bounds.
    pub extraction_complete: bool,
    /// Always true for structured adapters; captured source bytes remain canonical.
    pub original_preserved: bool,
    /// Always false; extraction never writes source or derivative bytes.
    pub filesystem_effect_performed: bool,
    /// Always false; relationships and assets are never retrieved.
    pub network_access_performed: bool,
    /// Always false; macros, formulas, scripts, fields, and objects are never executed.
    pub execution_performed: bool,
}

/// Closed structured extraction failure family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuredSourceExtractionError {
    /// Request identity, digest, media type, or limits were invalid.
    InvalidInput,
    /// The exact media type is not implemented by this extractor.
    Unsupported,
    /// Cancellation stopped extraction before publication.
    Cancelled,
    /// A declared source or output resource ceiling was exceeded.
    ResourceLimit,
    /// Active, external, encrypted, ambiguous, or otherwise prohibited content was found.
    Quarantined,
    /// Package or structured content was malformed.
    Malformed,
}

/// Interface-neutral extractor over exact already-captured source bytes.
pub trait StructuredSourceExtractor {
    /// Returns the exact implementation identity digest.
    fn extractor_sha256(&self) -> String;

    /// Extracts one bounded source without acquiring filesystem, network, or execution authority.
    fn extract(
        &self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<StructuredSourceExtraction, StructuredSourceExtractionError>;
}
