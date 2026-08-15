//! Bounded direct PowerPoint Open XML inspection without embedded-content execution.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipArchive};

use crate::word_ooxml::word_sha256;

const MAX_SOURCE_BYTES: u64 = 256 * 1_024 * 1_024;
const MAX_ENTRIES: usize = 65_536;
const MAX_ENTRY_BYTES: u64 = 128 * 1_024 * 1_024;
const MAX_TOTAL_BYTES: u64 = 512 * 1_024 * 1_024;
const MAX_SLIDES: usize = 16_384;
const MAX_OBJECTS: usize = 1_000_000;
const MAX_TEXT_BYTES: usize = 16 * 1_024 * 1_024;

/// Closed resource profile for PowerPoint Open XML inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum compressed package bytes.
    pub maximum_source_bytes: u64,
    /// Maximum ZIP entries.
    pub maximum_entries: usize,
    /// Maximum uncompressed bytes in one entry.
    pub maximum_entry_bytes: u64,
    /// Maximum total uncompressed bytes.
    pub maximum_total_uncompressed_bytes: u64,
    /// Maximum slides.
    pub maximum_slides: usize,
    /// Maximum retained slide objects.
    pub maximum_objects: usize,
    /// Maximum retained text bytes per object or note.
    pub maximum_text_bytes: usize,
}

impl Default for PresentationProfile {
    fn default() -> Self {
        Self {
            profile_id: "presentation-ooxml-strict-v1".to_owned(),
            maximum_source_bytes: 64 * 1_024 * 1_024,
            maximum_entries: 8_192,
            maximum_entry_bytes: 32 * 1_024 * 1_024,
            maximum_total_uncompressed_bytes: 128 * 1_024 * 1_024,
            maximum_slides: 4_096,
            maximum_objects: 250_000,
            maximum_text_bytes: 4 * 1_024 * 1_024,
        }
    }
}

impl PresentationProfile {
    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_source_bytes > 0
            && self.maximum_source_bytes <= MAX_SOURCE_BYTES
            && self.maximum_entries > 0
            && self.maximum_entries <= MAX_ENTRIES
            && self.maximum_entry_bytes > 0
            && self.maximum_entry_bytes <= MAX_ENTRY_BYTES
            && self.maximum_total_uncompressed_bytes >= self.maximum_entry_bytes
            && self.maximum_total_uncompressed_bytes <= MAX_TOTAL_BYTES
            && self.maximum_slides > 0
            && self.maximum_slides <= MAX_SLIDES
            && self.maximum_objects > 0
            && self.maximum_objects <= MAX_OBJECTS
            && self.maximum_text_bytes > 0
            && self.maximum_text_bytes <= MAX_TEXT_BYTES
    }
}

/// Closed retained presentation object class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationObjectKind {
    /// Text or generic shape.
    Shape,
    /// Raster or vector image relationship.
    Image,
    /// DrawingML chart relationship.
    Chart,
    /// DrawingML table.
    Table,
    /// Diagram or SmartArt data relationship.
    Diagram,
    /// Unsupported or unclassified graphic frame.
    Unsupported,
}

/// One inert slide hyperlink.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationLink {
    /// Owning slide object identity.
    pub object_id: u32,
    /// Exact relationship target.
    pub target: String,
    /// SHA-256 of the target.
    pub target_sha256: String,
    /// Whether the target is external to the package.
    pub external: bool,
    /// Always false; inspection never follows links.
    pub followed: bool,
}

/// One package-local image used by a slide object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationImageReference {
    /// Owning slide object identity.
    pub object_id: u32,
    /// Exact package part name.
    pub part_name: String,
    /// Exact media bytes digest.
    pub content_sha256: String,
    /// Exact media byte count.
    pub content_bytes: u64,
}

/// One ordered object extracted from a slide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationObject {
    /// Stable object identity from `cNvPr`.
    pub object_id: u32,
    /// One-based object order on the slide.
    pub order: u32,
    /// Declared object name.
    pub name: String,
    /// Closed object class.
    pub kind: PresentationObjectKind,
    /// Ordered decoded text runs.
    pub text: Vec<String>,
    /// Declared alternative text when present.
    pub alternative_text: Option<String>,
    /// Declared caption/title metadata when present.
    pub caption: Option<String>,
    /// Exact digest of the object's canonical retained fields.
    pub object_sha256: String,
}

/// One ordered slide inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSlide {
    /// One-based slide number from presentation order.
    pub slide_number: u32,
    /// Exact slide part name.
    pub part_name: String,
    /// Exact slide XML digest.
    pub part_sha256: String,
    /// Package-local layout part when present.
    pub layout_part_name: Option<String>,
    /// Exact layout XML digest when present.
    pub layout_sha256: Option<String>,
    /// Ordered objects.
    pub objects: Vec<PresentationObject>,
    /// Ordered speaker-note text runs.
    pub speaker_notes: Vec<String>,
    /// Inert hyperlinks.
    pub links: Vec<PresentationLink>,
    /// Package-local image references.
    pub images: Vec<PresentationImageReference>,
}

/// Closed presentation inspection finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationFindingKind {
    /// Macro-enabled content type or VBA part.
    MacroContent,
    /// OLE object, embedded package, or executable relationship.
    EmbeddedExecutableContent,
    /// External media or remote document relationship.
    ExternalMedia,
    /// Active action such as macro or program execution.
    ActiveAction,
    /// Unsupported relationship or object retained inertly.
    UnsupportedContent,
}

/// One content-minimized presentation finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationFinding {
    /// Stable finding identity.
    pub finding_id: String,
    /// Closed finding kind.
    pub kind: PresentationFindingKind,
    /// Package part when available.
    pub part_name: Option<String>,
    /// Slide number when available.
    pub slide_number: Option<u32>,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// True when safe extraction or reuse is blocked.
    pub blocks_safe_reuse: bool,
}

/// Complete bounded direct presentation inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationInspection {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Canonical caller-authorized source path.
    pub source_path: WorkspacePath,
    /// Exact source bytes digest.
    pub source_sha256: String,
    /// Exact parser profile.
    pub profile: PresentationProfile,
    /// Ordered slides.
    pub slides: Vec<PresentationSlide>,
    /// Canonically ordered findings.
    pub findings: Vec<PresentationFinding>,
    /// True only when admitted package structures were fully inspected.
    pub inspection_complete: bool,
    /// True only when no blocking finding exists.
    pub safe_for_reuse: bool,
    /// True because input bytes remain authoritative.
    pub original_preserved: bool,
    /// False because inspection is in memory.
    pub filesystem_effect_performed: bool,
    /// False because links and media are never fetched.
    pub network_access_performed: bool,
    /// False because actions, macros, OLE, and embedded content never execute.
    pub execution_performed: bool,
}

/// Stable presentation inspection failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationError {
    /// Profile or source path is invalid.
    InvalidInput,
    /// ZIP, relationship, required part, slide, or XML is malformed.
    MalformedPackage,
    /// ZIP path or package relationship target escapes its authority boundary.
    UnsafePackage,
    /// Encrypted ZIP entries are unsupported.
    EncryptedPackage,
    /// Source, entry, slide, object, text, or expansion limit was exceeded.
    ResourceLimit,
}

impl PresentationError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "presentation.input.invalid",
            Self::MalformedPackage => "presentation.package.malformed",
            Self::UnsafePackage => "presentation.package.unsafe",
            Self::EncryptedPackage => "presentation.package.encrypted",
            Self::ResourceLimit => "presentation.resource.limit",
        }
    }
}

impl std::fmt::Display for PresentationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PresentationError {}

#[derive(Clone)]
struct Relationship {
    target: String,
    relation_type: String,
    external: bool,
}

#[derive(Clone)]
struct ObjectBuilder {
    object_id: Option<u32>,
    name: String,
    kind: PresentationObjectKind,
    text: Vec<String>,
    alternative_text: Option<String>,
    caption: Option<String>,
    link_ids: Vec<String>,
    image_ids: Vec<String>,
}

impl ObjectBuilder {
    fn new(kind: PresentationObjectKind) -> Self {
        Self {
            object_id: None,
            name: String::new(),
            kind,
            text: Vec::new(),
            alternative_text: None,
            caption: None,
            link_ids: Vec::new(),
            image_ids: Vec::new(),
        }
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn attribute(
    event: &BytesStart<'_>,
    expected: &[u8],
    reader: &Reader<&[u8]>,
) -> Result<Option<String>, PresentationError> {
    for item in event.attributes().with_checks(true) {
        let item = item.map_err(|_| PresentationError::MalformedPackage)?;
        if local_name(item.key.as_ref()) == expected {
            return item
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|_| PresentationError::MalformedPackage);
        }
    }
    Ok(None)
}

fn relationship_id_attribute(
    event: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
) -> Result<Option<String>, PresentationError> {
    for item in event.attributes().with_checks(true) {
        let item = item.map_err(|_| PresentationError::MalformedPackage)?;
        if item.key.as_ref() == b"r:id" {
            return item
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|_| PresentationError::MalformedPackage);
        }
    }
    Ok(None)
}

fn decoded_text(event: quick_xml::events::BytesText<'_>) -> Result<String, PresentationError> {
    let encoded = event
        .decode()
        .map_err(|_| PresentationError::MalformedPackage)?;
    unescape(&encoded)
        .map(|value| value.into_owned())
        .map_err(|_| PresentationError::MalformedPackage)
}

fn decoded_reference(event: BytesRef<'_>) -> Result<String, PresentationError> {
    let reference = event
        .decode()
        .map_err(|_| PresentationError::MalformedPackage)?;
    let character = match reference.as_ref() {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or(PresentationError::MalformedPackage)?,
        value if value.starts_with('#') => value[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or(PresentationError::MalformedPackage)?,
        _ => return Err(PresentationError::MalformedPackage),
    };
    Ok(character.to_string())
}

fn package_parts(
    source: &[u8],
    profile: &PresentationProfile,
) -> Result<BTreeMap<String, Vec<u8>>, PresentationError> {
    if source.is_empty()
        || u64::try_from(source.len()).unwrap_or(u64::MAX) > profile.maximum_source_bytes
    {
        return Err(PresentationError::ResourceLimit);
    }
    let mut archive =
        ZipArchive::new(Cursor::new(source)).map_err(|_| PresentationError::MalformedPackage)?;
    if archive.is_empty() || archive.len() > profile.maximum_entries {
        return Err(PresentationError::ResourceLimit);
    }
    let mut total = 0_u64;
    let mut parts = BTreeMap::new();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| PresentationError::MalformedPackage)?;
        if file.is_dir() {
            continue;
        }
        if file.encrypted() {
            return Err(PresentationError::EncryptedPackage);
        }
        if !matches!(
            file.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(PresentationError::MalformedPackage);
        }
        let name = file
            .enclosed_name()
            .ok_or(PresentationError::UnsafePackage)?
            .to_string_lossy()
            .replace('\\', "/");
        if name.starts_with('/')
            || name
                .split('/')
                .any(|component| matches!(component, "" | "." | ".."))
            || parts.contains_key(&name)
        {
            return Err(PresentationError::UnsafePackage);
        }
        let expected = file.size();
        if expected > profile.maximum_entry_bytes {
            return Err(PresentationError::ResourceLimit);
        }
        total = total
            .checked_add(expected)
            .ok_or(PresentationError::ResourceLimit)?;
        if total > profile.maximum_total_uncompressed_bytes {
            return Err(PresentationError::ResourceLimit);
        }
        let mut content = Vec::new();
        (&mut file)
            .take(profile.maximum_entry_bytes + 1)
            .read_to_end(&mut content)
            .map_err(|_| PresentationError::MalformedPackage)?;
        if u64::try_from(content.len()).unwrap_or(u64::MAX) != expected {
            return Err(PresentationError::ResourceLimit);
        }
        parts.insert(name, content);
    }
    Ok(parts)
}

fn parse_relationships(
    content: &[u8],
) -> Result<BTreeMap<String, Relationship>, PresentationError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    let mut relationships = BTreeMap::new();
    loop {
        match reader
            .read_event()
            .map_err(|_| PresentationError::MalformedPackage)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"Relationship" =>
            {
                let id = attribute(&event, b"Id", &reader)?
                    .ok_or(PresentationError::MalformedPackage)?;
                let target = attribute(&event, b"Target", &reader)?
                    .ok_or(PresentationError::MalformedPackage)?;
                let relation_type = attribute(&event, b"Type", &reader)?
                    .ok_or(PresentationError::MalformedPackage)?;
                let external = attribute(&event, b"TargetMode", &reader)?
                    .is_some_and(|value| value.eq_ignore_ascii_case("external"));
                if relationships
                    .insert(
                        id,
                        Relationship {
                            target,
                            relation_type,
                            external,
                        },
                    )
                    .is_some()
                {
                    return Err(PresentationError::MalformedPackage);
                }
            }
            Event::DocType(_) => return Err(PresentationError::MalformedPackage),
            Event::Eof => return Ok(relationships),
            _ => {}
        }
    }
}

fn resolve_part(base: &str, target: &str) -> Result<String, PresentationError> {
    if target.starts_with('/') || target.contains('\\') || target.contains(':') {
        return Err(PresentationError::UnsafePackage);
    }
    let mut components = base
        .rsplit_once('/')
        .map(|item| item.0.split('/').collect::<Vec<_>>())
        .unwrap_or_default();
    for component in target.split('/') {
        match component {
            "" | "." => return Err(PresentationError::UnsafePackage),
            ".." => {
                components.pop().ok_or(PresentationError::UnsafePackage)?;
            }
            value => components.push(value),
        }
    }
    if components.is_empty() {
        return Err(PresentationError::UnsafePackage);
    }
    Ok(components.join("/"))
}

fn relationship_part(part_name: &str) -> Option<String> {
    let (directory, file) = part_name.rsplit_once('/')?;
    Some(format!("{directory}/_rels/{file}.rels"))
}

fn slide_order(content: &[u8]) -> Result<Vec<String>, PresentationError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    let mut ids = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|_| PresentationError::MalformedPackage)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"sldId" =>
            {
                ids.push(
                    relationship_id_attribute(&event, &reader)?
                        .ok_or(PresentationError::MalformedPackage)?,
                );
            }
            Event::DocType(_) => return Err(PresentationError::MalformedPackage),
            Event::Eof => return Ok(ids),
            _ => {}
        }
    }
}

fn all_text(content: &[u8], maximum: usize) -> Result<Vec<String>, PresentationError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    let mut in_text = false;
    let mut current = String::new();
    let mut result = Vec::new();
    let mut total = 0_usize;
    loop {
        match reader
            .read_event()
            .map_err(|_| PresentationError::MalformedPackage)?
        {
            Event::Start(event) if local_name(event.name().as_ref()) == b"t" => {
                in_text = true;
                current.clear();
            }
            Event::Text(event) if in_text => current.push_str(&decoded_text(event)?),
            Event::GeneralRef(event) if in_text => current.push_str(&decoded_reference(event)?),
            Event::End(event) if local_name(event.name().as_ref()) == b"t" => {
                in_text = false;
                total = total
                    .checked_add(current.len())
                    .ok_or(PresentationError::ResourceLimit)?;
                if total > maximum {
                    return Err(PresentationError::ResourceLimit);
                }
                if !current.is_empty() {
                    result.push(current.clone());
                }
            }
            Event::DocType(_) => return Err(PresentationError::MalformedPackage),
            Event::Eof => return Ok(result),
            _ => {}
        }
    }
}

fn finding(
    findings: &mut Vec<PresentationFinding>,
    kind: PresentationFindingKind,
    part_name: Option<&str>,
    slide_number: Option<u32>,
    reason_code: &str,
    blocks: bool,
) {
    let identity = format!(
        "{}:{}:{reason_code}",
        part_name.unwrap_or("package"),
        slide_number.map_or_else(|| "none".to_owned(), |value| value.to_string())
    );
    findings.push(PresentationFinding {
        finding_id: format!(
            "presentation-finding:{}",
            &word_sha256(identity.as_bytes())[..24]
        ),
        kind,
        part_name: part_name.map(str::to_owned),
        slide_number,
        reason_code: reason_code.to_owned(),
        blocks_safe_reuse: blocks,
    });
}

type OwnedRelationship = (u32, String);
type ParsedSlide = (
    Vec<PresentationObject>,
    Vec<OwnedRelationship>,
    Vec<OwnedRelationship>,
    bool,
);

fn parse_slide_objects(
    content: &[u8],
    maximum_text: usize,
) -> Result<ParsedSlide, PresentationError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    let mut builder: Option<ObjectBuilder> = None;
    let mut text = String::new();
    let mut capture_text = false;
    let mut objects = Vec::new();
    let mut links = Vec::new();
    let mut images = Vec::new();
    let mut active_action = false;
    loop {
        match reader
            .read_event()
            .map_err(|_| PresentationError::MalformedPackage)?
        {
            Event::Start(event) if local_name(event.name().as_ref()) == b"sp" => {
                builder = Some(ObjectBuilder::new(PresentationObjectKind::Shape));
            }
            Event::Start(event) if local_name(event.name().as_ref()) == b"pic" => {
                builder = Some(ObjectBuilder::new(PresentationObjectKind::Image));
            }
            Event::Start(event) if local_name(event.name().as_ref()) == b"graphicFrame" => {
                builder = Some(ObjectBuilder::new(PresentationObjectKind::Unsupported));
            }
            Event::Start(event) | Event::Empty(event)
                if builder.is_some() && local_name(event.name().as_ref()) == b"cNvPr" =>
            {
                let item = builder.as_mut().expect("builder");
                item.object_id = Some(
                    attribute(&event, b"id", &reader)?
                        .ok_or(PresentationError::MalformedPackage)?
                        .parse()
                        .map_err(|_| PresentationError::MalformedPackage)?,
                );
                item.name = attribute(&event, b"name", &reader)?.unwrap_or_default();
                item.alternative_text = attribute(&event, b"descr", &reader)?;
                item.caption = attribute(&event, b"title", &reader)?;
            }
            Event::Start(event)
                if builder.is_some() && local_name(event.name().as_ref()) == b"t" =>
            {
                capture_text = true;
                text.clear();
            }
            Event::Text(event) if capture_text => text.push_str(&decoded_text(event)?),
            Event::GeneralRef(event) if capture_text => text.push_str(&decoded_reference(event)?),
            Event::End(event) if local_name(event.name().as_ref()) == b"t" => {
                capture_text = false;
                if text.len() > maximum_text {
                    return Err(PresentationError::ResourceLimit);
                }
                if !text.is_empty() {
                    builder.as_mut().expect("builder").text.push(text.clone());
                }
            }
            Event::Start(event) | Event::Empty(event)
                if builder.is_some()
                    && matches!(
                        local_name(event.name().as_ref()),
                        b"hlinkClick" | b"hlinkMouseOver"
                    ) =>
            {
                if let Some(id) = attribute(&event, b"id", &reader)? {
                    builder.as_mut().expect("builder").link_ids.push(id);
                }
                if attribute(&event, b"action", &reader)?.is_some_and(|action| {
                    let lower = action.to_ascii_lowercase();
                    lower.contains("macro") || lower.contains("program") || lower.contains("ole")
                }) {
                    active_action = true;
                }
            }
            Event::Start(event) | Event::Empty(event)
                if builder.is_some() && local_name(event.name().as_ref()) == b"blip" =>
            {
                if let Some(id) = attribute(&event, b"embed", &reader)? {
                    let item = builder.as_mut().expect("builder");
                    item.kind = PresentationObjectKind::Image;
                    item.image_ids.push(id);
                }
                if let Some(id) = attribute(&event, b"link", &reader)? {
                    builder.as_mut().expect("builder").link_ids.push(id);
                }
            }
            Event::Start(event) | Event::Empty(event)
                if builder.is_some() && local_name(event.name().as_ref()) == b"chart" =>
            {
                builder.as_mut().expect("builder").kind = PresentationObjectKind::Chart;
            }
            Event::Start(event)
                if builder.is_some() && local_name(event.name().as_ref()) == b"tbl" =>
            {
                builder.as_mut().expect("builder").kind = PresentationObjectKind::Table;
            }
            Event::Start(event) | Event::Empty(event)
                if builder.is_some()
                    && matches!(
                        local_name(event.name().as_ref()),
                        b"relIds" | b"dataModelExt"
                    ) =>
            {
                builder.as_mut().expect("builder").kind = PresentationObjectKind::Diagram;
            }
            Event::End(event)
                if matches!(
                    local_name(event.name().as_ref()),
                    b"sp" | b"pic" | b"graphicFrame"
                ) =>
            {
                let item = builder.take().ok_or(PresentationError::MalformedPackage)?;
                let object_id = item.object_id.ok_or(PresentationError::MalformedPackage)?;
                let canonical = serde_json::to_vec(&(
                    object_id,
                    &item.name,
                    item.kind,
                    &item.text,
                    &item.alternative_text,
                    &item.caption,
                ))
                .map_err(|_| PresentationError::MalformedPackage)?;
                objects.push(PresentationObject {
                    object_id,
                    order: u32::try_from(objects.len() + 1).unwrap_or(u32::MAX),
                    name: item.name,
                    kind: item.kind,
                    text: item.text,
                    alternative_text: item.alternative_text,
                    caption: item.caption,
                    object_sha256: word_sha256(&canonical),
                });
                links.extend(
                    item.link_ids
                        .into_iter()
                        .map(|relationship_id| (object_id, relationship_id)),
                );
                images.extend(
                    item.image_ids
                        .into_iter()
                        .map(|relationship_id| (object_id, relationship_id)),
                );
            }
            Event::DocType(_) => return Err(PresentationError::MalformedPackage),
            Event::Eof => {
                if builder.is_some() || capture_text {
                    return Err(PresentationError::MalformedPackage);
                }
                return Ok((objects, links, images, active_action));
            }
            _ => {}
        }
    }
}

/// Inspects caller-supplied `.pptx` bytes without resolving links or executing embedded content.
pub fn inspect_pptx(
    source_path: WorkspacePath,
    source: &[u8],
    profile: &PresentationProfile,
) -> Result<PresentationInspection, PresentationError> {
    if !profile.valid() {
        return Err(PresentationError::InvalidInput);
    }
    let parts = package_parts(source, profile)?;
    let presentation = parts
        .get("ppt/presentation.xml")
        .ok_or(PresentationError::MalformedPackage)?;
    let presentation_relationships = parse_relationships(
        parts
            .get("ppt/_rels/presentation.xml.rels")
            .ok_or(PresentationError::MalformedPackage)?,
    )?;
    let ids = slide_order(presentation)?;
    if ids.is_empty() || ids.len() > profile.maximum_slides {
        return Err(PresentationError::ResourceLimit);
    }
    let mut findings = Vec::new();
    if parts.keys().any(|name| name.ends_with("vbaProject.bin"))
        || parts.iter().any(|(name, content)| {
            (name.ends_with(".xml") || name.ends_with(".rels"))
                && String::from_utf8_lossy(content)
                    .to_ascii_lowercase()
                    .contains("macroenabled")
        })
    {
        finding(
            &mut findings,
            PresentationFindingKind::MacroContent,
            None,
            None,
            "presentation.macro-content",
            true,
        );
    }
    let mut remaining_objects = profile.maximum_objects;
    let mut slides = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let relation = presentation_relationships
            .get(id)
            .ok_or(PresentationError::MalformedPackage)?;
        if relation.external || !relation.relation_type.ends_with("/slide") {
            return Err(PresentationError::UnsafePackage);
        }
        let part_name = resolve_part("ppt/presentation.xml", &relation.target)?;
        let content = parts
            .get(&part_name)
            .ok_or(PresentationError::MalformedPackage)?;
        let relationships = relationship_part(&part_name)
            .and_then(|name| parts.get(&name))
            .map(|content| parse_relationships(content))
            .transpose()?
            .unwrap_or_default();
        let slide_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let (objects, link_ids, image_ids, active_action) =
            parse_slide_objects(content, profile.maximum_text_bytes)?;
        let object_id_count = objects
            .iter()
            .map(|item| item.object_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if object_id_count != objects.len() {
            return Err(PresentationError::MalformedPackage);
        }
        if objects.len() > remaining_objects {
            return Err(PresentationError::ResourceLimit);
        }
        remaining_objects -= objects.len();
        if active_action {
            finding(
                &mut findings,
                PresentationFindingKind::ActiveAction,
                Some(&part_name),
                Some(slide_number),
                "presentation.active-action-inert",
                true,
            );
        }
        let mut links = Vec::new();
        for (object_id, id) in &link_ids {
            let relation = relationships
                .get(id)
                .ok_or(PresentationError::MalformedPackage)?;
            links.push(PresentationLink {
                object_id: *object_id,
                target: relation.target.clone(),
                target_sha256: word_sha256(relation.target.as_bytes()),
                external: relation.external,
                followed: false,
            });
            if relation.external && !relation.relation_type.ends_with("/hyperlink") {
                finding(
                    &mut findings,
                    PresentationFindingKind::ExternalMedia,
                    Some(&part_name),
                    Some(slide_number),
                    "presentation.external-media-inert",
                    true,
                );
            }
        }
        let mut images = Vec::new();
        for (object_id, id) in &image_ids {
            let relation = relationships
                .get(id)
                .ok_or(PresentationError::MalformedPackage)?;
            if relation.external || !relation.relation_type.ends_with("/image") {
                finding(
                    &mut findings,
                    PresentationFindingKind::ExternalMedia,
                    Some(&part_name),
                    Some(slide_number),
                    "presentation.image-relationship-unsafe",
                    true,
                );
                continue;
            }
            let image_part = resolve_part(&part_name, &relation.target)?;
            let image = parts
                .get(&image_part)
                .ok_or(PresentationError::MalformedPackage)?;
            images.push(PresentationImageReference {
                object_id: *object_id,
                part_name: image_part,
                content_sha256: word_sha256(image),
                content_bytes: u64::try_from(image.len()).unwrap_or(u64::MAX),
            });
        }
        let layout = relationships
            .values()
            .find(|item| item.relation_type.ends_with("/slideLayout") && !item.external)
            .map(|item| resolve_part(&part_name, &item.target))
            .transpose()?;
        let notes = relationships
            .values()
            .find(|item| item.relation_type.ends_with("/notesSlide") && !item.external)
            .map(|item| resolve_part(&part_name, &item.target))
            .transpose()?;
        for relation in relationships.values() {
            let lower = relation.relation_type.to_ascii_lowercase();
            if lower.contains("oleobject")
                || lower.ends_with("/package")
                || lower.contains("embeddedpackage")
            {
                finding(
                    &mut findings,
                    PresentationFindingKind::EmbeddedExecutableContent,
                    Some(&part_name),
                    Some(slide_number),
                    "presentation.embedded-executable-inert",
                    true,
                );
            }
        }
        let layout_sha256 = layout
            .as_ref()
            .map(|name| {
                parts
                    .get(name)
                    .map(|content| word_sha256(content))
                    .ok_or(PresentationError::MalformedPackage)
            })
            .transpose()?;
        let speaker_notes = notes
            .as_ref()
            .map(|name| {
                parts
                    .get(name)
                    .ok_or(PresentationError::MalformedPackage)
                    .and_then(|content| all_text(content, profile.maximum_text_bytes))
            })
            .transpose()?
            .unwrap_or_default();
        slides.push(PresentationSlide {
            slide_number,
            part_name: part_name.clone(),
            part_sha256: word_sha256(content),
            layout_part_name: layout,
            layout_sha256,
            objects,
            speaker_notes,
            links,
            images,
        });
    }
    findings.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    Ok(PresentationInspection {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_path,
        source_sha256: word_sha256(source),
        profile: profile.clone(),
        slides,
        safe_for_reuse: !findings.iter().any(|item| item.blocks_safe_reuse),
        findings,
        inspection_complete: true,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::word_generation::zip_parts;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-slides"),
            ["slides", "deck.pptx"],
        )
        .expect("path")
    }

    fn fixture(extra_relation: &str, macro_part: bool) -> Vec<u8> {
        let mut parts = BTreeMap::from([
            ("[Content_Types].xml".to_owned(), b"<Types/>".to_vec()),
            ("ppt/presentation.xml".to_owned(), b"<p:presentation xmlns:p=\"p\" xmlns:r=\"r\"><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId1\"/></p:sldIdLst></p:presentation>".to_vec()),
            ("ppt/_rels/presentation.xml.rels".to_owned(), b"<Relationships><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide1.xml\"/></Relationships>".to_vec()),
            ("ppt/slides/slide1.xml".to_owned(), b"<p:sld xmlns:p=\"p\" xmlns:a=\"a\" xmlns:r=\"r\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\" title=\"Caption\" descr=\"Alternative\"><a:hlinkClick r:id=\"rIdLink\"/></p:cNvPr></p:nvSpPr><p:txBody><a:p><a:r><a:t>Hello &amp; welcome</a:t></a:r></a:p></p:txBody></p:sp><p:pic><p:nvPicPr><p:cNvPr id=\"3\" name=\"Image\" descr=\"Chart image\"/></p:nvPicPr><p:blipFill><a:blip r:embed=\"rIdImage\"/></p:blipFill></p:pic></p:spTree></p:cSld></p:sld>".to_vec()),
            ("ppt/slides/_rels/slide1.xml.rels".to_owned(), format!("<Relationships><Relationship Id=\"rIdLink\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"https://example.invalid\" TargetMode=\"External\"/><Relationship Id=\"rIdImage\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"../media/image1.png\"/><Relationship Id=\"rIdLayout\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rIdNotes\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"../notesSlides/notesSlide1.xml\"/>{extra_relation}</Relationships>").into_bytes()),
            ("ppt/media/image1.png".to_owned(), b"fixture image".to_vec()),
            ("ppt/slideLayouts/slideLayout1.xml".to_owned(), b"<p:sldLayout xmlns:p=\"p\"/>".to_vec()),
            ("ppt/notesSlides/notesSlide1.xml".to_owned(), b"<p:notes xmlns:p=\"p\" xmlns:a=\"a\"><a:t>Speaker note</a:t></p:notes>".to_vec()),
        ]);
        if macro_part {
            parts.insert("ppt/vbaProject.bin".to_owned(), b"inert fixture".to_vec());
        }
        zip_parts(&parts).expect("zip")
    }

    #[test]
    fn extracts_order_text_notes_links_images_captions_layouts_and_hashes_without_effects() {
        let report = inspect_pptx(path(), &fixture("", false), &PresentationProfile::default())
            .expect("inspect");
        assert!(report.safe_for_reuse);
        let slide = &report.slides[0];
        assert_eq!(slide.slide_number, 1);
        assert_eq!(slide.objects.len(), 2);
        assert_eq!(slide.objects[0].text, ["Hello & welcome"]);
        assert_eq!(slide.objects[0].caption.as_deref(), Some("Caption"));
        assert_eq!(
            slide.objects[0].alternative_text.as_deref(),
            Some("Alternative")
        );
        assert_eq!(slide.speaker_notes, ["Speaker note"]);
        assert!(slide.links[0].external && !slide.links[0].followed);
        assert_eq!(slide.links[0].object_id, 2);
        assert_eq!(slide.images.len(), 1);
        assert_eq!(slide.images[0].object_id, 3);
        assert!(slide.layout_sha256.is_some());
        assert!(!report.filesystem_effect_performed);
        assert!(!report.network_access_performed);
        assert!(!report.execution_performed);
    }

    #[test]
    fn macro_ole_external_media_and_active_actions_are_quarantined_inertly() {
        let ole = "<Relationship Id=\"rIdOle\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject\" Target=\"../embeddings/object1.bin\"/>";
        let report = inspect_pptx(path(), &fixture(ole, true), &PresentationProfile::default())
            .expect("inspect");
        assert!(!report.safe_for_reuse);
        assert!(
            report
                .findings
                .iter()
                .any(|item| item.kind == PresentationFindingKind::MacroContent)
        );
        assert!(
            report
                .findings
                .iter()
                .any(|item| item.kind == PresentationFindingKind::EmbeddedExecutableContent)
        );
        assert!(!report.execution_performed);
    }

    #[test]
    fn malformed_missing_and_oversized_packages_fail_closed() {
        assert!(inspect_pptx(path(), b"not zip", &PresentationProfile::default()).is_err());
        let tiny = PresentationProfile {
            maximum_source_bytes: 8,
            ..PresentationProfile::default()
        };
        assert_eq!(
            inspect_pptx(path(), &fixture("", false), &tiny),
            Err(PresentationError::ResourceLimit)
        );
        let parts = BTreeMap::from([(
            "ppt/presentation.xml".to_owned(),
            b"<p:presentation/>".to_vec(),
        )]);
        assert!(
            inspect_pptx(
                path(),
                &zip_parts(&parts).expect("zip"),
                &PresentationProfile::default()
            )
            .is_err()
        );
    }
}
