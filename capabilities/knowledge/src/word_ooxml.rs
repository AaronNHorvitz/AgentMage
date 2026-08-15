//! Bounded in-memory WordprocessingML inspection and deterministic sidecar extraction.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::{Cursor, Read};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive};

const CONVERTER_ID: &str =
    "agentmage-word-ooxml-v1;zip=8.6.0;quick-xml=0.41.0;network=denied;filesystem=denied";
const MAX_PROFILE_BYTES: u64 = 512 * 1_024 * 1_024;
const MAX_PROFILE_ENTRIES: usize = 16_384;

/// Exact resource and parser profile for one OOXML operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordConversionProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum compressed source bytes.
    pub maximum_source_bytes: u64,
    /// Maximum ZIP entries.
    pub maximum_entries: usize,
    /// Maximum uncompressed bytes in one entry.
    pub maximum_entry_bytes: u64,
    /// Maximum total uncompressed bytes.
    pub maximum_total_uncompressed_bytes: u64,
    /// Maximum integer expansion ratio after accounting for zero-byte storage.
    pub maximum_expansion_ratio: u64,
}

impl WordConversionProfile {
    /// Conservative cross-platform default for ordinary Word documents.
    #[must_use]
    pub fn strict_default() -> Self {
        Self {
            profile_id: "word-ooxml-strict-v1".to_owned(),
            maximum_source_bytes: 64 * 1_024 * 1_024,
            maximum_entries: 4_096,
            maximum_entry_bytes: 32 * 1_024 * 1_024,
            maximum_total_uncompressed_bytes: 128 * 1_024 * 1_024,
            maximum_expansion_ratio: 200,
        }
    }

    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_source_bytes > 0
            && self.maximum_source_bytes <= MAX_PROFILE_BYTES
            && self.maximum_entries > 0
            && self.maximum_entries <= MAX_PROFILE_ENTRIES
            && self.maximum_entry_bytes > 0
            && self.maximum_entry_bytes <= self.maximum_total_uncompressed_bytes
            && self.maximum_total_uncompressed_bytes <= MAX_PROFILE_BYTES
            && self.maximum_expansion_ratio > 0
            && self.maximum_expansion_ratio <= 10_000
    }
}

/// Closed OOXML package part class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordPartKind {
    /// Main WordprocessingML document.
    MainDocument,
    /// Header part.
    Header,
    /// Footer part.
    Footer,
    /// Comment part.
    Comments,
    /// Numbering definitions.
    Numbering,
    /// Style definitions.
    Styles,
    /// Package or part relationships.
    Relationships,
    /// Core, application, or custom properties.
    Properties,
    /// Embedded image or other media.
    Media,
    /// Other XML retained but not semantically interpreted.
    OtherXml,
    /// Other binary retained but not interpreted.
    OtherBinary,
}

/// Closed ZIP compression class admitted by the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordCompressionKind {
    /// Stored without compression.
    Stored,
    /// Deflate compression.
    Deflate,
    /// Compression method is not admitted.
    Unsupported,
}

/// One exact package-part observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPackagePart {
    /// Canonical package-relative part name.
    pub part_name: String,
    /// Closed semantic part class.
    pub kind: WordPartKind,
    /// Closed compression class.
    pub compression: WordCompressionKind,
    /// Compressed byte count from the ZIP directory.
    pub compressed_bytes: u64,
    /// Uncompressed byte count from the ZIP directory.
    pub uncompressed_bytes: u64,
    /// Exact uncompressed digest when the part could be read safely.
    pub content_sha256: Option<String>,
}

/// Closed package quarantine finding class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordPackageFindingKind {
    /// Required OOXML package part is absent.
    MissingRequiredPart,
    /// Entry name is absolute, ambiguous, or traverses parents.
    UnsafeEntryPath,
    /// Entry identity appears more than once.
    DuplicateEntry,
    /// Entry is encrypted.
    EncryptedEntry,
    /// Entry is a symbolic link.
    SymbolicLinkEntry,
    /// Entry uses a compression method outside the admitted set.
    UnsupportedCompression,
    /// Package contains macro, ActiveX, embedded executable, or similar active content.
    ActiveContent,
    /// WordprocessingML requests hidden or web-hidden presentation.
    HiddenContent,
    /// A relationship targets an external resource.
    ExternalRelationship,
    /// XML was malformed or could not be decoded under the closed parser.
    MalformedXml,
}

/// One content-minimized package finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPackageFinding {
    /// Stable finding identity.
    pub finding_id: String,
    /// Closed finding class.
    pub kind: WordPackageFindingKind,
    /// Optional package part identity.
    pub part_name: Option<String>,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Always true for this closed finding inventory.
    pub blocks_extraction: bool,
}

/// Closed Word feature whose fidelity must remain explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordFeatureKind {
    /// Table structure.
    Table,
    /// Comment content or anchor.
    Comment,
    /// Tracked insertion.
    TrackedInsertion,
    /// Tracked deletion.
    TrackedDeletion,
    /// Header content.
    Header,
    /// Footer content.
    Footer,
    /// Numbering or list definition.
    Numbering,
    /// Section, page, margin, or layout property.
    SectionLayout,
    /// Hyperlink.
    Hyperlink,
    /// Drawing, picture, or image relationship.
    Image,
    /// Field code or computed field.
    Field,
    /// Paragraph, character, or table style.
    Style,
    /// Unsupported structured construct preserved in the source package.
    UnsupportedConstruct,
}

/// One feature count in canonical order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordFeatureCount {
    /// Closed feature class.
    pub kind: WordFeatureKind,
    /// Number of observed feature events or parts.
    pub count: u32,
}

/// One explicit text-sidecar fidelity limitation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordFidelityWarning {
    /// Stable warning identity.
    pub warning_id: String,
    /// Feature not completely represented by plain text.
    pub feature: WordFeatureKind,
    /// Number of observed instances.
    pub observed_count: u32,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Always true; the original package remains the canonical fidelity source.
    pub original_remains_authoritative: bool,
}

/// Complete in-memory package inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordInspectionReport {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact validated source path.
    pub source_path: WorkspacePath,
    /// Exact source package digest.
    pub source_sha256: String,
    /// Exact conversion identity digest.
    pub conversion_identity_sha256: String,
    /// Exact profile identity.
    pub profile_id: String,
    /// Package parts in canonical name order.
    pub parts: Vec<WordPackagePart>,
    /// Feature counts in enum order.
    pub features: Vec<WordFeatureCount>,
    /// Blocking findings in canonical order.
    pub findings: Vec<WordPackageFinding>,
    /// True when any finding prevents extraction.
    pub quarantined: bool,
    /// True only when every non-directory entry received an exact content digest.
    pub inspection_complete: bool,
    /// Always true; inspection accepts immutable source bytes.
    pub original_preserved: bool,
    /// Always false; no package entry is written to disk.
    pub filesystem_effect_performed: bool,
    /// Always false; relationships and assets are never resolved.
    pub network_access_performed: bool,
    /// Always false; macros, fields, objects, and scripts are never executed.
    pub execution_performed: bool,
}

/// Closed revision state for one exact text fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordRevisionState {
    /// Current non-revision text.
    Current,
    /// Text inside a tracked insertion.
    Inserted,
    /// Text inside a tracked deletion.
    Deleted,
}

/// Exact byte range inside one OOXML part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPartSourceRange {
    /// Exact part identity.
    pub part_name: String,
    /// Inclusive UTF-8 XML start byte.
    pub start_byte: u64,
    /// Exclusive UTF-8 XML end byte.
    pub end_byte: u64,
}

/// One decoded text fragment with exact part provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordTextFragment {
    /// Stable one-based fragment identity.
    pub fragment_id: String,
    /// Exact source range.
    pub source_range: WordPartSourceRange,
    /// Decoded text.
    pub text: String,
    /// Revision state from enclosing tracked-change elements.
    pub revision_state: WordRevisionState,
}

/// Deterministic source-to-sidecar result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordExtractionResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact source package digest.
    pub source_sha256: String,
    /// Exact converter identity digest.
    pub conversion_identity_sha256: String,
    /// Exact profile and source-bound cache key.
    pub cache_key_sha256: String,
    /// Deterministic UTF-8 sidecar bytes.
    pub sidecar: Vec<u8>,
    /// Exact sidecar digest.
    pub sidecar_sha256: String,
    /// Exact ordered fragment ledger.
    pub fragments: Vec<WordTextFragment>,
    /// Explicit fidelity warnings.
    pub fidelity_warnings: Vec<WordFidelityWarning>,
    /// Complete package inspection used for admission.
    pub inspection: WordInspectionReport,
    /// Always true; original source bytes remain authoritative.
    pub original_preserved: bool,
    /// Always false; extraction does not write the sidecar.
    pub filesystem_effect_performed: bool,
    /// Always false; extraction does not resolve external content.
    pub network_access_performed: bool,
    /// Always false; extraction does not execute document content.
    pub execution_performed: bool,
}

/// In-memory cache outcome bound to source, converter, and profile identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSidecarCacheOutcome {
    /// Exact extraction result.
    pub result: WordExtractionResult,
    /// True only when an identical sealed result already existed.
    pub cache_hit: bool,
}

/// Bounded in-memory sidecar cache without persistence authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSidecarCache {
    maximum_entries: usize,
    entries: BTreeMap<String, WordExtractionResult>,
}

impl WordSidecarCache {
    /// Constructs an empty bounded cache.
    pub fn new(maximum_entries: usize) -> Result<Self, WordOoxmlError> {
        if maximum_entries == 0 || maximum_entries > 4_096 {
            return Err(WordOoxmlError::InvalidInput);
        }
        Ok(Self {
            maximum_entries,
            entries: BTreeMap::new(),
        })
    }

    /// Returns a sealed cached result or performs one deterministic extraction.
    pub fn get_or_extract(
        &mut self,
        source_path: &WorkspacePath,
        source: &[u8],
        profile: &WordConversionProfile,
    ) -> Result<WordSidecarCacheOutcome, WordOoxmlError> {
        let key = cache_key(source, profile)?;
        if let Some(result) = self.entries.get(&key) {
            return Ok(WordSidecarCacheOutcome {
                result: result.clone(),
                cache_hit: true,
            });
        }
        if self.entries.len() >= self.maximum_entries {
            return Err(WordOoxmlError::ResourceLimit);
        }
        let result = extract_docx_to_sidecar(source_path, source, profile)?;
        self.entries.insert(key, result.clone());
        Ok(WordSidecarCacheOutcome {
            result,
            cache_hit: false,
        })
    }
}

/// Stable fail-closed WordprocessingML error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordOoxmlError {
    /// Identity, profile, input, or text is invalid.
    InvalidInput,
    /// ZIP container or required package structure is malformed.
    MalformedPackage,
    /// Declared resource ceiling was exceeded.
    ResourceLimit,
    /// Package is structurally readable but quarantined.
    QuarantinedPackage,
    /// XML decoding or structure is malformed.
    MalformedXml,
}

impl WordOoxmlError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "word_ooxml.input.invalid",
            Self::MalformedPackage => "word_ooxml.package.malformed",
            Self::ResourceLimit => "word_ooxml.resource.limit",
            Self::QuarantinedPackage => "word_ooxml.package.quarantined",
            Self::MalformedXml => "word_ooxml.xml.malformed",
        }
    }
}

impl std::fmt::Display for WordOoxmlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WordOoxmlError {}

struct InspectedPackage {
    report: WordInspectionReport,
    content: BTreeMap<String, Vec<u8>>,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

pub(crate) fn word_sha256(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

/// Returns the exact admitted converter identity digest.
#[must_use]
pub fn word_conversion_identity_sha256() -> String {
    word_sha256(CONVERTER_ID.as_bytes())
}

fn cache_key(source: &[u8], profile: &WordConversionProfile) -> Result<String, WordOoxmlError> {
    if !profile.valid() || source.is_empty() || source.len() as u64 > profile.maximum_source_bytes {
        return Err(WordOoxmlError::InvalidInput);
    }
    Ok(word_sha256(
        format!(
            "{}\n{}\n{}",
            word_sha256(source),
            word_conversion_identity_sha256(),
            profile.profile_id
        )
        .as_bytes(),
    ))
}

fn safe_part_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains('\\')
        && !value.contains('\0')
        && !value.contains(':')
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn part_kind(name: &str) -> WordPartKind {
    match name {
        "word/document.xml" => WordPartKind::MainDocument,
        "word/comments.xml" => WordPartKind::Comments,
        "word/numbering.xml" => WordPartKind::Numbering,
        "word/styles.xml" => WordPartKind::Styles,
        _ if name.starts_with("word/header") && name.ends_with(".xml") => WordPartKind::Header,
        _ if name.starts_with("word/footer") && name.ends_with(".xml") => WordPartKind::Footer,
        _ if name.ends_with(".rels") => WordPartKind::Relationships,
        _ if name.starts_with("docProps/") && name.ends_with(".xml") => WordPartKind::Properties,
        _ if name.starts_with("word/media/") => WordPartKind::Media,
        _ if name.ends_with(".xml") => WordPartKind::OtherXml,
        _ => WordPartKind::OtherBinary,
    }
}

fn compression_kind(method: CompressionMethod) -> WordCompressionKind {
    match method {
        CompressionMethod::Stored => WordCompressionKind::Stored,
        CompressionMethod::Deflated => WordCompressionKind::Deflate,
        _ => WordCompressionKind::Unsupported,
    }
}

fn active_part(name: &str, content: &[u8]) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("vbaproject.bin")
        || lower.contains("/activex/")
        || lower.contains("/embeddings/")
        || lower.contains("/customui/")
        || lower.ends_with(".exe")
        || lower.ends_with(".dll")
        || lower.ends_with(".js")
        || (name == "[Content_Types].xml"
            && String::from_utf8_lossy(content)
                .to_ascii_lowercase()
                .contains("macroenabled"))
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn attribute_value(
    event: &BytesStart<'_>,
    expected_local_name: &[u8],
    reader: &Reader<&[u8]>,
) -> Option<String> {
    event
        .attributes()
        .with_checks(true)
        .filter_map(Result::ok)
        .find(|attribute| local_name(attribute.key.as_ref()) == expected_local_name)
        .and_then(|attribute| {
            attribute
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                .ok()
                .map(|value| value.into_owned())
        })
}

fn external_relationship(content: &[u8]) -> Result<bool, WordOoxmlError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    loop {
        match reader
            .read_event()
            .map_err(|_| WordOoxmlError::MalformedXml)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"Relationship" =>
            {
                let mode = attribute_value(&event, b"TargetMode", &reader).unwrap_or_default();
                let target = attribute_value(&event, b"Target", &reader).unwrap_or_default();
                let lower = target.to_ascii_lowercase();
                if mode.eq_ignore_ascii_case("external")
                    || lower.starts_with("http:")
                    || lower.starts_with("https:")
                    || lower.starts_with("file:")
                    || lower.starts_with("ftp:")
                {
                    return Ok(true);
                }
            }
            Event::Eof => return Ok(false),
            _ => {}
        }
    }
}

fn hidden_content(content: &[u8]) -> Result<bool, WordOoxmlError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    loop {
        match reader
            .read_event()
            .map_err(|_| WordOoxmlError::MalformedXml)?
        {
            Event::Start(event) | Event::Empty(event)
                if matches!(local_name(event.name().as_ref()), b"vanish" | b"webHidden") =>
            {
                return Ok(true);
            }
            Event::Eof => return Ok(false),
            _ => {}
        }
    }
}

fn package_finding(
    findings: &mut Vec<WordPackageFinding>,
    kind: WordPackageFindingKind,
    part_name: Option<&str>,
    reason_code: &str,
) {
    let identity = part_name.unwrap_or("package");
    findings.push(WordPackageFinding {
        finding_id: format!("finding:{reason_code}:{identity}"),
        kind,
        part_name: part_name.map(str::to_owned),
        reason_code: reason_code.to_owned(),
        blocks_extraction: true,
    });
}

fn feature_kind(name: &[u8]) -> Option<WordFeatureKind> {
    match local_name(name) {
        b"tbl" => Some(WordFeatureKind::Table),
        b"comment" | b"commentRangeStart" | b"commentRangeEnd" | b"commentReference" => {
            Some(WordFeatureKind::Comment)
        }
        b"ins" => Some(WordFeatureKind::TrackedInsertion),
        b"del" => Some(WordFeatureKind::TrackedDeletion),
        b"numPr" | b"abstractNum" | b"num" => Some(WordFeatureKind::Numbering),
        b"sectPr" | b"pgSz" | b"pgMar" | b"cols" => Some(WordFeatureKind::SectionLayout),
        b"hyperlink" => Some(WordFeatureKind::Hyperlink),
        b"drawing" | b"pict" | b"blip" => Some(WordFeatureKind::Image),
        b"fldSimple" | b"instrText" | b"fldChar" => Some(WordFeatureKind::Field),
        b"style" | b"pStyle" | b"rStyle" | b"tblStyle" => Some(WordFeatureKind::Style),
        b"altChunk" | b"object" | b"oleObject" | b"customXml" | b"smartTag" => {
            Some(WordFeatureKind::UnsupportedConstruct)
        }
        _ => None,
    }
}

fn inspect_xml_features(
    name: &str,
    content: &[u8],
    counts: &mut BTreeMap<WordFeatureKind, u32>,
) -> Result<(), WordOoxmlError> {
    match part_kind(name) {
        WordPartKind::Header => *counts.entry(WordFeatureKind::Header).or_default() += 1,
        WordPartKind::Footer => *counts.entry(WordFeatureKind::Footer).or_default() += 1,
        WordPartKind::Styles => *counts.entry(WordFeatureKind::Style).or_default() += 1,
        WordPartKind::Numbering => *counts.entry(WordFeatureKind::Numbering).or_default() += 1,
        _ => {}
    }
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    loop {
        match reader
            .read_event()
            .map_err(|_| WordOoxmlError::MalformedXml)?
        {
            Event::Start(event) | Event::Empty(event) => {
                if let Some(kind) = feature_kind(event.name().as_ref()) {
                    *counts.entry(kind).or_default() += 1;
                }
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn central_directory_duplicates(
    source: &[u8],
    directory_start: u64,
) -> Result<(usize, Vec<Option<String>>), WordOoxmlError> {
    let mut offset =
        usize::try_from(directory_start).map_err(|_| WordOoxmlError::MalformedPackage)?;
    let mut names = BTreeSet::new();
    let mut duplicates = Vec::new();
    let mut count = 0_usize;
    while source.get(offset..offset.saturating_add(4)) == Some(&[0x50, 0x4b, 0x01, 0x02]) {
        let header = source
            .get(offset..offset.saturating_add(46))
            .ok_or(WordOoxmlError::MalformedPackage)?;
        let filename_bytes = usize::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_bytes = usize::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_bytes = usize::from(u16::from_le_bytes([header[32], header[33]]));
        let name_start = offset
            .checked_add(46)
            .ok_or(WordOoxmlError::MalformedPackage)?;
        let name_end = name_start
            .checked_add(filename_bytes)
            .ok_or(WordOoxmlError::MalformedPackage)?;
        let next = name_end
            .checked_add(extra_bytes)
            .and_then(|value| value.checked_add(comment_bytes))
            .ok_or(WordOoxmlError::MalformedPackage)?;
        let raw_name = source
            .get(name_start..name_end)
            .ok_or(WordOoxmlError::MalformedPackage)?;
        source
            .get(offset..next)
            .ok_or(WordOoxmlError::MalformedPackage)?;
        if !names.insert(raw_name.to_vec()) {
            duplicates.push(std::str::from_utf8(raw_name).ok().map(str::to_owned));
        }
        count = count.checked_add(1).ok_or(WordOoxmlError::ResourceLimit)?;
        offset = next;
    }
    if count == 0 {
        return Err(WordOoxmlError::MalformedPackage);
    }
    Ok((count, duplicates))
}

fn inspect_package(
    source_path: &WorkspacePath,
    source: &[u8],
    profile: &WordConversionProfile,
) -> Result<InspectedPackage, WordOoxmlError> {
    if !profile.valid() || source.is_empty() || source.len() as u64 > profile.maximum_source_bytes {
        return Err(WordOoxmlError::InvalidInput);
    }
    let mut archive =
        ZipArchive::new(Cursor::new(source)).map_err(|_| WordOoxmlError::MalformedPackage)?;
    let (raw_entry_count, central_duplicates) =
        central_directory_duplicates(source, archive.central_directory_start())?;
    if archive.is_empty() || raw_entry_count > profile.maximum_entries {
        return Err(WordOoxmlError::ResourceLimit);
    }
    let mut names = BTreeSet::new();
    let mut content = BTreeMap::new();
    let mut parts = Vec::new();
    let mut findings = Vec::new();
    for duplicate in central_duplicates {
        package_finding(
            &mut findings,
            WordPackageFindingKind::DuplicateEntry,
            duplicate.as_deref(),
            "word.package.entry-duplicate",
        );
    }
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let (
            name,
            compression,
            compressed_bytes,
            uncompressed_bytes,
            encrypted,
            symlink,
            directory,
        ) = {
            let file = archive
                .by_index_raw(index)
                .map_err(|_| WordOoxmlError::MalformedPackage)?;
            let name = std::str::from_utf8(file.name_raw()).ok().map(str::to_owned);
            (
                name,
                compression_kind(file.compression()),
                file.compressed_size(),
                file.size(),
                file.encrypted(),
                file.is_symlink(),
                file.is_dir(),
            )
        };
        let Some(name) = name else {
            package_finding(
                &mut findings,
                WordPackageFindingKind::UnsafeEntryPath,
                None,
                "word.package.entry-name-invalid-utf8",
            );
            continue;
        };
        if directory {
            continue;
        }
        total_uncompressed = total_uncompressed
            .checked_add(uncompressed_bytes)
            .ok_or(WordOoxmlError::ResourceLimit)?;
        if uncompressed_bytes > profile.maximum_entry_bytes
            || total_uncompressed > profile.maximum_total_uncompressed_bytes
            || uncompressed_bytes
                > compressed_bytes
                    .max(1)
                    .saturating_mul(profile.maximum_expansion_ratio)
        {
            return Err(WordOoxmlError::ResourceLimit);
        }
        if !safe_part_name(&name) {
            package_finding(
                &mut findings,
                WordPackageFindingKind::UnsafeEntryPath,
                Some(&name),
                "word.package.entry-path-unsafe",
            );
        }
        let unique_name = names.insert(name.clone());
        if encrypted {
            package_finding(
                &mut findings,
                WordPackageFindingKind::EncryptedEntry,
                Some(&name),
                "word.package.entry-encrypted",
            );
        }
        if symlink {
            package_finding(
                &mut findings,
                WordPackageFindingKind::SymbolicLinkEntry,
                Some(&name),
                "word.package.entry-symlink",
            );
        }
        if compression == WordCompressionKind::Unsupported {
            package_finding(
                &mut findings,
                WordPackageFindingKind::UnsupportedCompression,
                Some(&name),
                "word.package.compression-unsupported",
            );
        }
        let readable = safe_part_name(&name)
            && unique_name
            && !encrypted
            && !symlink
            && compression != WordCompressionKind::Unsupported;
        let bytes = if readable {
            let mut file = archive
                .by_index(index)
                .map_err(|_| WordOoxmlError::MalformedPackage)?;
            let mut bytes = Vec::with_capacity(uncompressed_bytes as usize);
            file.read_to_end(&mut bytes)
                .map_err(|_| WordOoxmlError::MalformedPackage)?;
            if bytes.len() as u64 != uncompressed_bytes {
                return Err(WordOoxmlError::MalformedPackage);
            }
            Some(bytes)
        } else {
            None
        };
        if bytes
            .as_deref()
            .is_some_and(|value| active_part(&name, value))
            || active_part(&name, &[])
        {
            package_finding(
                &mut findings,
                WordPackageFindingKind::ActiveContent,
                Some(&name),
                "word.package.active-content",
            );
        }
        if let Some(bytes) = &bytes {
            if name.ends_with(".rels") {
                match external_relationship(bytes) {
                    Ok(true) => package_finding(
                        &mut findings,
                        WordPackageFindingKind::ExternalRelationship,
                        Some(&name),
                        "word.package.external-relationship",
                    ),
                    Ok(false) => {}
                    Err(_) => package_finding(
                        &mut findings,
                        WordPackageFindingKind::MalformedXml,
                        Some(&name),
                        "word.package.relationship-xml-malformed",
                    ),
                }
            }
            if name.ends_with(".xml") && hidden_content(bytes).unwrap_or(false) {
                package_finding(
                    &mut findings,
                    WordPackageFindingKind::HiddenContent,
                    Some(&name),
                    "word.package.hidden-content",
                );
            }
            content.insert(name.clone(), bytes.clone());
        }
        let kind = part_kind(&name);
        parts.push(WordPackagePart {
            part_name: name,
            kind,
            compression,
            compressed_bytes,
            uncompressed_bytes,
            content_sha256: bytes.as_deref().map(word_sha256),
        });
    }
    for required in ["[Content_Types].xml", "_rels/.rels", "word/document.xml"] {
        if !names.contains(required) {
            package_finding(
                &mut findings,
                WordPackageFindingKind::MissingRequiredPart,
                Some(required),
                "word.package.required-part-missing",
            );
        }
    }
    let mut feature_counts = BTreeMap::new();
    for (name, bytes) in &content {
        if name.ends_with(".xml") && inspect_xml_features(name, bytes, &mut feature_counts).is_err()
        {
            package_finding(
                &mut findings,
                WordPackageFindingKind::MalformedXml,
                Some(name),
                "word.package.part-xml-malformed",
            );
        }
    }
    parts.sort_by(|left, right| left.part_name.cmp(&right.part_name));
    findings.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    findings.dedup_by(|left, right| left.finding_id == right.finding_id);
    let inspection_complete = parts.iter().all(|part| part.content_sha256.is_some());
    let quarantined = !findings.is_empty();
    Ok(InspectedPackage {
        report: WordInspectionReport {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_path: source_path.clone(),
            source_sha256: word_sha256(source),
            conversion_identity_sha256: word_conversion_identity_sha256(),
            profile_id: profile.profile_id.clone(),
            parts,
            features: feature_counts
                .into_iter()
                .map(|(kind, count)| WordFeatureCount { kind, count })
                .collect(),
            findings,
            quarantined,
            inspection_complete,
            original_preserved: true,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        },
        content,
    })
}

/// Inspects one exact WordprocessingML package without extracting entries to disk.
pub fn inspect_docx(
    source_path: &WorkspacePath,
    source: &[u8],
    profile: &WordConversionProfile,
) -> Result<WordInspectionReport, WordOoxmlError> {
    Ok(inspect_package(source_path, source, profile)?.report)
}

pub(crate) fn admitted_docx_parts(
    source_path: &WorkspacePath,
    source: &[u8],
    profile: &WordConversionProfile,
) -> Result<(WordInspectionReport, BTreeMap<String, Vec<u8>>), WordOoxmlError> {
    let inspected = inspect_package(source_path, source, profile)?;
    if inspected.report.quarantined || !inspected.report.inspection_complete {
        return Err(WordOoxmlError::QuarantinedPackage);
    }
    Ok((inspected.report, inspected.content))
}

fn extract_fragments(
    part_name: &str,
    content: &[u8],
    next_fragment: &mut u32,
) -> Result<Vec<WordTextFragment>, WordOoxmlError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    let mut fragments = Vec::new();
    let mut revision_stack = Vec::new();
    let mut capture_start = None;
    loop {
        let start = reader.buffer_position();
        let event = reader
            .read_event()
            .map_err(|_| WordOoxmlError::MalformedXml)?;
        let end = reader.buffer_position();
        match event {
            Event::Start(event) => match local_name(event.name().as_ref()) {
                b"ins" => revision_stack.push(WordRevisionState::Inserted),
                b"del" => revision_stack.push(WordRevisionState::Deleted),
                b"t" | b"delText" | b"instrText" if capture_start.replace(end).is_some() => {
                    return Err(WordOoxmlError::MalformedXml);
                }
                _ => {}
            },
            Event::End(event) => match local_name(event.name().as_ref()) {
                b"ins" | b"del" => {
                    revision_stack.pop();
                }
                b"t" | b"delText" | b"instrText" => {
                    let content_start = capture_start.take().ok_or(WordOoxmlError::MalformedXml)?;
                    let content_start_usize =
                        usize::try_from(content_start).map_err(|_| WordOoxmlError::MalformedXml)?;
                    let content_end_usize =
                        usize::try_from(start).map_err(|_| WordOoxmlError::MalformedXml)?;
                    let encoded = std::str::from_utf8(
                        content
                            .get(content_start_usize..content_end_usize)
                            .ok_or(WordOoxmlError::MalformedXml)?,
                    )
                    .map_err(|_| WordOoxmlError::MalformedXml)?;
                    if encoded.contains('<') {
                        return Err(WordOoxmlError::MalformedXml);
                    }
                    let decoded = unescape(encoded)
                        .map_err(|_| WordOoxmlError::MalformedXml)?
                        .into_owned();
                    if !decoded.is_empty() {
                        let revision_state = revision_stack
                            .last()
                            .copied()
                            .unwrap_or(WordRevisionState::Current);
                        fragments.push(WordTextFragment {
                            fragment_id: format!("fragment-{next_fragment:08}"),
                            source_range: WordPartSourceRange {
                                part_name: part_name.to_owned(),
                                start_byte: content_start,
                                end_byte: start,
                            },
                            text: decoded,
                            revision_state,
                        });
                        *next_fragment += 1;
                    }
                }
                _ => {}
            },
            Event::Eof => return Ok(fragments),
            _ => {}
        }
    }
}

fn fidelity_warning(feature: &WordFeatureCount) -> WordFidelityWarning {
    let reason = match feature.kind {
        WordFeatureKind::Table => "word.fidelity.table-layout-not-in-text",
        WordFeatureKind::Comment => "word.fidelity.comment-anchor-not-in-text",
        WordFeatureKind::TrackedInsertion | WordFeatureKind::TrackedDeletion => {
            "word.fidelity.revision-presentation-not-in-text"
        }
        WordFeatureKind::Header | WordFeatureKind::Footer => {
            "word.fidelity.page-placement-not-in-text"
        }
        WordFeatureKind::Numbering => "word.fidelity.numbering-definition-not-in-text",
        WordFeatureKind::SectionLayout => "word.fidelity.page-layout-not-in-text",
        WordFeatureKind::Hyperlink => "word.fidelity.hyperlink-target-not-in-text",
        WordFeatureKind::Image => "word.fidelity.image-not-in-text",
        WordFeatureKind::Field => "word.fidelity.field-behavior-not-in-text",
        WordFeatureKind::Style => "word.fidelity.style-not-in-text",
        WordFeatureKind::UnsupportedConstruct => "word.fidelity.unsupported-construct",
    };
    WordFidelityWarning {
        warning_id: format!("warning:{reason}"),
        feature: feature.kind,
        observed_count: feature.count,
        reason_code: reason.to_owned(),
        original_remains_authoritative: true,
    }
}

/// Extracts deterministic text and exact part-range provenance from an admitted package.
pub fn extract_docx_to_sidecar(
    source_path: &WorkspacePath,
    source: &[u8],
    profile: &WordConversionProfile,
) -> Result<WordExtractionResult, WordOoxmlError> {
    let inspected = inspect_package(source_path, source, profile)?;
    if inspected.report.quarantined || !inspected.report.inspection_complete {
        return Err(WordOoxmlError::QuarantinedPackage);
    }
    let mut fragments = Vec::new();
    let mut next_fragment = 1_u32;
    for (name, content) in &inspected.content {
        if matches!(
            part_kind(name),
            WordPartKind::MainDocument
                | WordPartKind::Header
                | WordPartKind::Footer
                | WordPartKind::Comments
        ) {
            fragments.extend(extract_fragments(name, content, &mut next_fragment)?);
        }
    }
    let source_sha256 = inspected.report.source_sha256.clone();
    let conversion_identity_sha256 = inspected.report.conversion_identity_sha256.clone();
    let cache_key_sha256 = cache_key(source, profile)?;
    let mut sidecar = String::new();
    writeln!(&mut sidecar, "# Word Text Sidecar\n").expect("String write");
    writeln!(&mut sidecar, "source_sha256: {source_sha256}").expect("String write");
    writeln!(
        &mut sidecar,
        "conversion_identity_sha256: {conversion_identity_sha256}"
    )
    .expect("String write");
    writeln!(&mut sidecar, "profile_id: {}\n", profile.profile_id).expect("String write");
    let mut active_part = "";
    for fragment in &fragments {
        if fragment.source_range.part_name != active_part {
            active_part = &fragment.source_range.part_name;
            writeln!(&mut sidecar, "## {active_part}\n").expect("String write");
        }
        let revision = match fragment.revision_state {
            WordRevisionState::Current => "current",
            WordRevisionState::Inserted => "inserted",
            WordRevisionState::Deleted => "deleted",
        };
        writeln!(
            &mut sidecar,
            "- [{}:{}-{}; {revision}] {}",
            fragment.source_range.part_name,
            fragment.source_range.start_byte,
            fragment.source_range.end_byte,
            fragment.text.replace('\n', " ")
        )
        .expect("String write");
    }
    let sidecar = sidecar.into_bytes();
    let fidelity_warnings = inspected
        .report
        .features
        .iter()
        .map(fidelity_warning)
        .collect();
    Ok(WordExtractionResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_sha256,
        conversion_identity_sha256,
        cache_key_sha256,
        sidecar_sha256: word_sha256(&sidecar),
        sidecar,
        fragments,
        fidelity_warnings,
        inspection: inspected.report,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-word"),
            ["docs", "report.docx"],
        )
        .expect("path")
    }

    fn package(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o644);
            for (name, content) in entries {
                writer.start_file(*name, options).expect("start");
                writer.write_all(content.as_bytes()).expect("write");
            }
            writer.finish().expect("finish");
        }
        cursor.into_inner()
    }

    fn deflated_package(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o644);
            for (name, content) in entries {
                writer.start_file(*name, options).expect("start");
                writer.write_all(content.as_bytes()).expect("write");
            }
            writer.finish().expect("finish");
        }
        cursor.into_inner()
    }

    fn patch_first_entry(source: &mut [u8], encrypted: bool, compression: Option<u16>) {
        assert_eq!(&source[0..4], b"PK\x03\x04");
        let central = source
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .expect("central directory");
        if encrypted {
            let local_flags = u16::from_le_bytes([source[6], source[7]]) | 1;
            source[6..8].copy_from_slice(&local_flags.to_le_bytes());
            let central_flags = u16::from_le_bytes([source[central + 8], source[central + 9]]) | 1;
            source[central + 8..central + 10].copy_from_slice(&central_flags.to_le_bytes());
        }
        if let Some(method) = compression {
            source[8..10].copy_from_slice(&method.to_le_bytes());
            source[central + 10..central + 12].copy_from_slice(&method.to_le_bytes());
        }
    }

    fn base_entries(document: &str) -> Vec<(&'static str, String)> {
        vec![
            (
                "[Content_Types].xml",
                "<Types><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>".to_owned(),
            ),
            (
                "_rels/.rels",
                "<Relationships><Relationship Id=\"rId1\" Target=\"word/document.xml\"/></Relationships>".to_owned(),
            ),
            ("word/document.xml", document.to_owned()),
        ]
    }

    fn build(entries: Vec<(&str, String)>) -> Vec<u8> {
        let refs = entries
            .iter()
            .map(|(name, content)| (*name, content.as_str()))
            .collect::<Vec<_>>();
        package(&refs)
    }

    #[test]
    fn inspection_and_extraction_are_deterministic_and_effect_free() {
        let source = build(base_entries(
            "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>Report</w:t></w:r></w:p><w:sectPr/></w:body></w:document>",
        ));
        let profile = WordConversionProfile::strict_default();
        let first = extract_docx_to_sidecar(&path(), &source, &profile).expect("extract");
        let second = extract_docx_to_sidecar(&path(), &source, &profile).expect("extract");
        assert_eq!(first, second);
        assert_eq!(first.fragments.len(), 1);
        assert_eq!(first.fragments[0].text, "Report");
        assert!(first.original_preserved);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
    }

    #[test]
    fn feature_inventory_preserves_revision_header_footer_comment_and_layout_limits() {
        let mut entries = base_entries(
            "<w:document xmlns:w=\"w\"><w:body><w:tbl/><w:p><w:ins><w:r><w:t>new</w:t></w:r></w:ins><w:del><w:r><w:delText>old</w:delText></w:r></w:del><w:hyperlink><w:r><w:t>link</w:t></w:r></w:hyperlink></w:p><w:sectPr><w:pgSz/></w:sectPr></w:body></w:document>",
        );
        entries.push((
            "word/header1.xml",
            "<w:hdr xmlns:w=\"w\"><w:p><w:r><w:t>Header</w:t></w:r></w:p></w:hdr>".to_owned(),
        ));
        entries.push((
            "word/footer1.xml",
            "<w:ftr xmlns:w=\"w\"><w:p><w:r><w:t>Footer</w:t></w:r></w:p></w:ftr>".to_owned(),
        ));
        entries.push(("word/comments.xml", "<w:comments xmlns:w=\"w\"><w:comment><w:p><w:r><w:t>Comment</w:t></w:r></w:p></w:comment></w:comments>".to_owned()));
        entries.push((
            "word/numbering.xml",
            "<w:numbering xmlns:w=\"w\"><w:abstractNum/></w:numbering>".to_owned(),
        ));
        let result = extract_docx_to_sidecar(
            &path(),
            &build(entries),
            &WordConversionProfile::strict_default(),
        )
        .expect("extract");
        for feature in [
            WordFeatureKind::Table,
            WordFeatureKind::Comment,
            WordFeatureKind::TrackedInsertion,
            WordFeatureKind::TrackedDeletion,
            WordFeatureKind::Header,
            WordFeatureKind::Footer,
            WordFeatureKind::Numbering,
            WordFeatureKind::SectionLayout,
            WordFeatureKind::Hyperlink,
        ] {
            assert!(
                result
                    .fidelity_warnings
                    .iter()
                    .any(|item| item.feature == feature)
            );
        }
        assert!(
            result
                .fragments
                .iter()
                .any(|item| item.revision_state == WordRevisionState::Inserted)
        );
        assert!(
            result
                .fragments
                .iter()
                .any(|item| item.revision_state == WordRevisionState::Deleted)
        );
    }

    #[test]
    fn cache_identity_reuses_only_exact_source_converter_and_profile() {
        let source = build(base_entries(
            "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>Cached</w:t></w:r></w:p></w:body></w:document>",
        ));
        let profile = WordConversionProfile::strict_default();
        let mut cache = WordSidecarCache::new(2).expect("cache");
        assert!(
            !cache
                .get_or_extract(&path(), &source, &profile)
                .expect("first")
                .cache_hit
        );
        assert!(
            cache
                .get_or_extract(&path(), &source, &profile)
                .expect("second")
                .cache_hit
        );
        let mut changed = source.clone();
        changed.push(0);
        let changed_outcome = cache
            .get_or_extract(&path(), &changed, &profile)
            .expect("changed source");
        assert!(!changed_outcome.cache_hit);
        assert_ne!(
            changed_outcome.result.cache_key_sha256,
            cache_key(&source, &profile).expect("original key")
        );
    }

    #[test]
    fn external_relationship_and_active_content_are_quarantined_without_resolution() {
        let mut entries = base_entries(
            "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>Safe text</w:t></w:r></w:p></w:body></w:document>",
        );
        entries.push(("word/vbaProject.bin", "inert".to_owned()));
        entries.push(("word/_rels/document.xml.rels", "<Relationships><Relationship Id=\"rId9\" TargetMode=\"External\" Target=\"https://example.invalid/x\"/></Relationships>".to_owned()));
        let source = build(entries);
        let report = inspect_docx(&path(), &source, &WordConversionProfile::strict_default())
            .expect("inspection");
        assert!(report.quarantined);
        assert!(
            report
                .findings
                .iter()
                .any(|item| item.kind == WordPackageFindingKind::ActiveContent)
        );
        assert!(
            report
                .findings
                .iter()
                .any(|item| item.kind == WordPackageFindingKind::ExternalRelationship)
        );
        assert_eq!(
            extract_docx_to_sidecar(&path(), &source, &WordConversionProfile::strict_default())
                .expect_err("quarantine"),
            WordOoxmlError::QuarantinedPackage
        );
        assert!(!report.network_access_performed);
        assert!(!report.execution_performed);
    }

    #[test]
    fn duplicate_package_entries_are_quarantined_and_never_extracted() {
        let mut source = package(&[
            ("[Content_Types].xml", "<Types/>"),
            ("_rels/.rels", "<Relationships/>"),
            (
                "word/document.xml",
                "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>first</w:t></w:r></w:p></w:body></w:document>",
            ),
            (
                "word/documenx.xml",
                "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>second</w:t></w:r></w:p></w:body></w:document>",
            ),
        ]);
        let from = b"word/documenx.xml";
        let to = b"word/document.xml";
        let mut replacements = 0;
        for index in 0..=source.len() - from.len() {
            if &source[index..index + from.len()] == from {
                source[index..index + to.len()].copy_from_slice(to);
                replacements += 1;
            }
        }
        assert_eq!(replacements, 2, "local and central ZIP names must change");
        let profile = WordConversionProfile::strict_default();
        let report = inspect_docx(&path(), &source, &profile).expect("inspection");
        assert!(report.quarantined);
        assert!(
            report
                .findings
                .iter()
                .any(|item| item.kind == WordPackageFindingKind::DuplicateEntry)
        );
        assert_eq!(
            extract_docx_to_sidecar(&path(), &source, &profile).expect_err("quarantine"),
            WordOoxmlError::QuarantinedPackage
        );
    }

    #[test]
    fn resource_and_required_part_failures_are_explicit() {
        let source = package(&[("word/document.xml", "<w:document xmlns:w=\"w\"/>")]);
        let report = inspect_docx(&path(), &source, &WordConversionProfile::strict_default())
            .expect("inspection");
        assert!(report.quarantined);
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|item| item.kind == WordPackageFindingKind::MissingRequiredPart)
                .count(),
            2
        );
        let mut profile = WordConversionProfile::strict_default();
        profile.maximum_source_bytes = 1;
        assert_eq!(
            inspect_docx(&path(), &source, &profile).expect_err("bounded"),
            WordOoxmlError::InvalidInput
        );
    }

    #[test]
    fn hostile_path_hidden_xml_encryption_and_compression_fail_closed() {
        let mut entries = base_entries(
            "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:rPr><w:vanish/></w:rPr><w:t>Hidden</w:t></w:r></w:p></w:body></w:document>",
        );
        entries.push(("../outside.xml", "<outside/>".to_owned()));
        entries.push((
            "word/_rels/document.xml.rels",
            "<Relationships><Relationship".to_owned(),
        ));
        let report = inspect_docx(
            &path(),
            &build(entries),
            &WordConversionProfile::strict_default(),
        )
        .expect("inspection");
        for finding in [
            WordPackageFindingKind::UnsafeEntryPath,
            WordPackageFindingKind::HiddenContent,
            WordPackageFindingKind::MalformedXml,
        ] {
            assert!(report.findings.iter().any(|item| item.kind == finding));
        }
        assert!(report.quarantined);
        assert!(!report.network_access_performed);
        assert!(!report.execution_performed);

        let baseline = build(base_entries(
            "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>Text</w:t></w:r></w:p></w:body></w:document>",
        ));
        let mut encrypted = baseline.clone();
        patch_first_entry(&mut encrypted, true, None);
        let encrypted_report = inspect_docx(
            &path(),
            &encrypted,
            &WordConversionProfile::strict_default(),
        )
        .expect("encrypted inspection");
        assert!(
            encrypted_report
                .findings
                .iter()
                .any(|item| item.kind == WordPackageFindingKind::EncryptedEntry)
        );

        let mut unsupported = baseline;
        patch_first_entry(&mut unsupported, false, Some(12));
        let unsupported_report = inspect_docx(
            &path(),
            &unsupported,
            &WordConversionProfile::strict_default(),
        )
        .expect("unsupported inspection");
        assert!(
            unsupported_report
                .findings
                .iter()
                .any(|item| item.kind == WordPackageFindingKind::UnsupportedCompression)
        );
    }

    #[test]
    fn expansion_bomb_and_parser_crash_inputs_are_bounded() {
        let large = "A".repeat(200_000);
        let entries = [
            ("[Content_Types].xml", "<Types/>"),
            ("_rels/.rels", "<Relationships/>"),
            ("word/document.xml", large.as_str()),
        ];
        let mut profile = WordConversionProfile::strict_default();
        profile.maximum_expansion_ratio = 2;
        assert_eq!(
            inspect_docx(&path(), &deflated_package(&entries), &profile).expect_err("bomb"),
            WordOoxmlError::ResourceLimit
        );
        assert_eq!(
            inspect_docx(
                &path(),
                b"not-a-zip",
                &WordConversionProfile::strict_default()
            )
            .expect_err("malformed"),
            WordOoxmlError::MalformedPackage
        );
    }
}
