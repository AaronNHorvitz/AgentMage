//! Bounded PDF metadata, link, form, image, and active-content inspection.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use lopdf::{Dictionary, Document, LoadOptions, Object, ObjectId, decode_text_string};
use serde::{Deserialize, Serialize};

use crate::pdf_extraction::{
    PdfExtractionError, PdfExtractionProfile, PdfExtractionResult, PdfPageIdentity,
    extract_pdf_to_pages, parser_error,
};
use crate::word_ooxml::word_sha256;

/// Closed PDF link or action class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfLinkKind {
    /// Destination inside the same PDF.
    InternalDestination,
    /// External URI retained without resolution.
    ExternalUri,
    /// Destination in another PDF or file.
    RemoteDocument,
    /// Operating-system launch action.
    Launch,
    /// Embedded JavaScript action.
    JavaScript,
    /// Form-submission action.
    SubmitForm,
    /// Other action outside the admitted closed set.
    Unknown,
}

/// One exact inert PDF link or action observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfLinkObservation {
    /// Stable observation identity.
    pub link_id: String,
    /// Exact source page when the action belongs to a page annotation.
    pub page: Option<PdfPageIdentity>,
    /// Indirect-object number when one exists.
    pub object_number: Option<u32>,
    /// Indirect-object generation when one exists.
    pub object_generation: Option<u16>,
    /// Closed action class.
    pub kind: PdfLinkKind,
    /// Decoded target retained as inert data when available.
    pub target: Option<String>,
    /// Digest of the target bytes when a target is available.
    pub target_sha256: Option<String>,
    /// True for actions that could cross a document, process, script, or network boundary.
    pub external_or_active: bool,
    /// Always false; inspection never follows or invokes an action.
    pub followed_or_executed: bool,
}

/// One fillable-form field observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfFormFieldObservation {
    /// Stable field identity.
    pub field_id: String,
    /// Indirect-object number.
    pub object_number: u32,
    /// Indirect-object generation.
    pub object_generation: u16,
    /// PDF field type name, such as `Tx` or `Btn`.
    pub field_type: String,
    /// Decoded field name when available.
    pub field_name: Option<String>,
    /// Digest of the field name when available.
    pub field_name_sha256: Option<String>,
    /// True when a value is present; the value itself is not retained here.
    pub has_value: bool,
}

/// One page-image observation without retained image payload bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfImageObservation {
    /// Exact page identity.
    pub page: PdfPageIdentity,
    /// Indirect-object number of the image stream.
    pub object_number: u32,
    /// Indirect-object generation of the image stream.
    pub object_generation: u16,
    /// Declared image width.
    pub width: u32,
    /// Declared image height.
    pub height: u32,
    /// Declared color space when available.
    pub color_space: Option<String>,
    /// Canonically ordered stream filters.
    pub filters: Vec<String>,
    /// Digest of the retained encoded image stream bytes.
    pub encoded_stream_sha256: String,
}

/// Closed PDF artifact finding class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfArtifactFindingKind {
    /// Embedded JavaScript was observed.
    JavaScript,
    /// Operating-system launch action was observed.
    LaunchAction,
    /// Remote-document action was observed.
    RemoteDocumentAction,
    /// Form submission was observed.
    SubmitFormAction,
    /// Embedded file content was observed.
    EmbeddedFile,
    /// Additional or open actions were observed.
    AutomaticAction,
    /// Link, action, form, image, or metadata structure could not be fully decoded.
    InspectionLimited,
}

/// One content-minimized PDF artifact finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfArtifactFinding {
    /// Stable finding identity.
    pub finding_id: String,
    /// Closed finding class.
    pub kind: PdfArtifactFindingKind,
    /// Optional source object number.
    pub object_number: Option<u32>,
    /// Optional source page identity.
    pub page_id: Option<String>,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Whether this finding prevents safe reuse as generation input.
    pub blocks_safe_reuse: bool,
}

/// Standard PDF metadata fields retained without custom metadata values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfMetadataSummary {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document keywords.
    pub keywords: Option<String>,
    /// Creating application.
    pub creator: Option<String>,
    /// Producing application.
    pub producer: Option<String>,
    /// Raw PDF creation-date string.
    pub creation_date: Option<String>,
    /// Raw PDF modification-date string.
    pub modification_date: Option<String>,
    /// Canonically ordered custom metadata key names; values are not retained.
    pub custom_keys: Vec<String>,
}

/// Complete bounded PDF artifact inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfArtifactInspection {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Canonical caller-authorized path.
    pub source_path: WorkspacePath,
    /// Digest of exact input bytes.
    pub source_sha256: String,
    /// Extraction resource profile.
    pub profile: PdfExtractionProfile,
    /// PDF version reported by the parser.
    pub pdf_version: String,
    /// Declared or observed logical page count.
    pub page_count: u32,
    /// True when the source declares encryption.
    pub encrypted: bool,
    /// Standard metadata available without password handling.
    pub metadata: PdfMetadataSummary,
    /// Canonically ordered inert links and actions.
    pub links: Vec<PdfLinkObservation>,
    /// Canonically ordered form fields.
    pub forms: Vec<PdfFormFieldObservation>,
    /// Canonically ordered page images.
    pub images: Vec<PdfImageObservation>,
    /// Canonically ordered active-content and inspection findings.
    pub findings: Vec<PdfArtifactFinding>,
    /// Bounded text extraction when the document is unencrypted.
    pub extraction: Option<PdfExtractionResult>,
    /// True when all requested structures were inspected within declared bounds.
    pub inspection_complete: bool,
    /// True only when no active or structurally limiting finding blocks reuse.
    pub safe_for_generation_input: bool,
    /// Always true; inspection never replaces the original.
    pub original_preserved: bool,
    /// Always false; caller supplied bytes are inspected in memory.
    pub filesystem_effect_performed: bool,
    /// Always false; links and actions remain inert.
    pub network_access_performed: bool,
    /// Always false; scripts, launch actions, and embedded files are never executed.
    pub execution_performed: bool,
}

fn empty_metadata() -> PdfMetadataSummary {
    PdfMetadataSummary {
        title: None,
        author: None,
        subject: None,
        keywords: None,
        creator: None,
        producer: None,
        creation_date: None,
        modification_date: None,
        custom_keys: Vec::new(),
    }
}

fn decoded_string(object: &Object) -> Option<String> {
    decode_text_string(object)
        .ok()
        .filter(|value| value.len() <= 16_384)
}

fn metadata(document: &Document) -> PdfMetadataSummary {
    let Some(info_id) = document
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|item| item.as_reference().ok())
    else {
        return empty_metadata();
    };
    let Ok(dictionary) = document.get_dictionary(info_id) else {
        return empty_metadata();
    };
    let field = |name: &[u8]| dictionary.get(name).ok().and_then(decoded_string);
    let standard = [
        b"Title".as_slice(),
        b"Author",
        b"Subject",
        b"Keywords",
        b"Creator",
        b"Producer",
        b"CreationDate",
        b"ModDate",
        b"Trapped",
    ];
    let mut custom_keys = dictionary
        .iter()
        .filter(|(key, _)| !standard.contains(&key.as_slice()))
        .map(|(key, _)| String::from_utf8_lossy(key).into_owned())
        .filter(|key| !key.is_empty() && key.len() <= 256)
        .collect::<Vec<_>>();
    custom_keys.sort();
    custom_keys.dedup();
    PdfMetadataSummary {
        title: field(b"Title"),
        author: field(b"Author"),
        subject: field(b"Subject"),
        keywords: field(b"Keywords"),
        creator: field(b"Creator"),
        producer: field(b"Producer"),
        creation_date: field(b"CreationDate"),
        modification_date: field(b"ModDate"),
        custom_keys,
    }
}

fn action_kind(dictionary: &Dictionary) -> PdfLinkKind {
    match dictionary
        .get(b"S")
        .ok()
        .and_then(|item| item.as_name().ok())
    {
        Some(b"URI") => PdfLinkKind::ExternalUri,
        Some(b"GoTo") => PdfLinkKind::InternalDestination,
        Some(b"GoToR") => PdfLinkKind::RemoteDocument,
        Some(b"Launch") => PdfLinkKind::Launch,
        Some(b"JavaScript") => PdfLinkKind::JavaScript,
        Some(b"SubmitForm") => PdfLinkKind::SubmitForm,
        _ => PdfLinkKind::Unknown,
    }
}

fn action_target(dictionary: &Dictionary, kind: PdfLinkKind) -> Option<String> {
    let key = match kind {
        PdfLinkKind::ExternalUri => b"URI".as_slice(),
        PdfLinkKind::RemoteDocument | PdfLinkKind::Launch => b"F",
        PdfLinkKind::JavaScript => b"JS",
        PdfLinkKind::SubmitForm => b"F",
        PdfLinkKind::InternalDestination | PdfLinkKind::Unknown => return None,
    };
    dictionary.get(key).ok().and_then(decoded_string)
}

fn finding(
    kind: PdfArtifactFindingKind,
    object_id: Option<ObjectId>,
    page_id: Option<&str>,
    reason_code: &str,
) -> PdfArtifactFinding {
    let object_number = object_id.map(|item| item.0);
    let identity = format!(
        "{}\n{}\n{}",
        object_number.map_or_else(|| "none".to_owned(), |item| item.to_string()),
        page_id.unwrap_or("none"),
        reason_code
    );
    PdfArtifactFinding {
        finding_id: format!("pdf-finding:{}", word_sha256(identity.as_bytes())),
        kind,
        object_number,
        page_id: page_id.map(str::to_owned),
        reason_code: reason_code.to_owned(),
        blocks_safe_reuse: true,
    }
}

fn action_finding(
    kind: PdfLinkKind,
    object_id: Option<ObjectId>,
    page_id: Option<&str>,
) -> Option<PdfArtifactFinding> {
    let (finding_kind, reason) = match kind {
        PdfLinkKind::RemoteDocument => (
            PdfArtifactFindingKind::RemoteDocumentAction,
            "pdf.action.remote-document-inert",
        ),
        PdfLinkKind::Launch => (
            PdfArtifactFindingKind::LaunchAction,
            "pdf.action.launch-inert",
        ),
        PdfLinkKind::JavaScript => (
            PdfArtifactFindingKind::JavaScript,
            "pdf.action.javascript-inert",
        ),
        PdfLinkKind::SubmitForm => (
            PdfArtifactFindingKind::SubmitFormAction,
            "pdf.action.submit-form-inert",
        ),
        _ => return None,
    };
    Some(finding(finding_kind, object_id, page_id, reason))
}

fn page_annotation_ids(document: &Document, page_id: ObjectId) -> Vec<ObjectId> {
    let Ok(page) = document.get_dictionary(page_id) else {
        return Vec::new();
    };
    let Ok(annotations) = page.get(b"Annots") else {
        return Vec::new();
    };
    let Ok((_, annotations)) = document.dereference(annotations) else {
        return Vec::new();
    };
    annotations
        .as_array()
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_reference().ok())
        .collect()
}

fn inspect_action(
    document: &Document,
    action: &Object,
    object_id: Option<ObjectId>,
    page: Option<&PdfPageIdentity>,
) -> Option<(PdfLinkObservation, Option<PdfArtifactFinding>)> {
    let (resolved_id, resolved) = document.dereference(action).ok()?;
    let dictionary = resolved.as_dict().ok()?;
    let kind = action_kind(dictionary);
    let target = action_target(dictionary, kind);
    let actual_id = resolved_id.or(object_id);
    let object_label = actual_id.map_or_else(
        || "inline".to_owned(),
        |item| format!("{}:{}", item.0, item.1),
    );
    let page_label = page.map_or("none", |item| item.page_id.as_str());
    let target_sha256 = target.as_ref().map(|item| word_sha256(item.as_bytes()));
    let link_id = format!(
        "pdf-link:{}",
        word_sha256(
            format!(
                "{object_label}\n{page_label}\n{kind:?}\n{}",
                target_sha256.as_deref().unwrap_or("none")
            )
            .as_bytes()
        )
    );
    let observation = PdfLinkObservation {
        link_id,
        page: page.cloned(),
        object_number: actual_id.map(|item| item.0),
        object_generation: actual_id.map(|item| item.1),
        kind,
        target,
        target_sha256,
        external_or_active: !matches!(kind, PdfLinkKind::InternalDestination),
        followed_or_executed: false,
    };
    let related_finding = action_finding(kind, actual_id, page.map(|item| item.page_id.as_str()));
    Some((observation, related_finding))
}

/// Inspects PDF metadata and artifact structures without following actions or retaining payloads.
pub fn inspect_pdf_artifact(
    source_path: &WorkspacePath,
    source: &[u8],
    profile: &PdfExtractionProfile,
) -> Result<PdfArtifactInspection, PdfExtractionError> {
    if !profile.valid()
        || source.is_empty()
        || source.len() > profile.maximum_source_bytes
        || !source.starts_with(b"%PDF-")
    {
        return Err(PdfExtractionError::InvalidInput);
    }
    let document = Document::load_mem_with_options(
        source,
        LoadOptions {
            password: None,
            filter: None,
            strict: true,
            max_decompressed_size: Some(profile.maximum_decompressed_stream_bytes),
        },
    )
    .map_err(|error| parser_error(&error))?;
    if document.objects.len() > profile.maximum_objects {
        return Err(PdfExtractionError::ResourceLimit);
    }
    let source_sha256 = word_sha256(source);
    let encrypted = document.is_encrypted() || document.was_encrypted();
    if encrypted {
        return Ok(PdfArtifactInspection {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_path: source_path.clone(),
            source_sha256,
            profile: profile.clone(),
            pdf_version: document.version,
            page_count: 0,
            encrypted: true,
            metadata: empty_metadata(),
            links: Vec::new(),
            forms: Vec::new(),
            images: Vec::new(),
            findings: vec![finding(
                PdfArtifactFindingKind::InspectionLimited,
                None,
                None,
                "pdf.document.encrypted-password-boundary",
            )],
            extraction: None,
            inspection_complete: false,
            safe_for_generation_input: false,
            original_preserved: true,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        });
    }

    let extraction = extract_pdf_to_pages(source_path, source, profile)?;
    let page_by_number = extraction
        .pages
        .iter()
        .map(|item| (item.identity.page_number, item.identity.clone()))
        .collect::<BTreeMap<_, _>>();
    let page_objects = document.get_pages();
    let mut links = Vec::new();
    let mut forms = Vec::new();
    let mut images = Vec::new();
    let mut findings = Vec::new();
    let mut observed_action_ids = BTreeSet::new();

    for (page_number, page_object_id) in &page_objects {
        let page = page_by_number
            .get(page_number)
            .ok_or(PdfExtractionError::MalformedDocument)?;
        for annotation_id in page_annotation_ids(&document, *page_object_id) {
            let Ok(annotation) = document.get_dictionary(annotation_id) else {
                findings.push(finding(
                    PdfArtifactFindingKind::InspectionLimited,
                    Some(annotation_id),
                    Some(&page.page_id),
                    "pdf.annotation.decode-limited",
                ));
                continue;
            };
            if annotation
                .get(b"Subtype")
                .ok()
                .and_then(|item| item.as_name().ok())
                == Some(b"Link")
            {
                if let Ok(action) = annotation.get(b"A") {
                    if let Some((link, related)) =
                        inspect_action(&document, action, Some(annotation_id), Some(page))
                    {
                        if let Some(number) = link.object_number {
                            observed_action_ids.insert(number);
                        }
                        links.push(link);
                        findings.extend(related);
                    }
                } else if annotation.get(b"Dest").is_ok() {
                    let target = None;
                    let link_id = format!(
                        "pdf-link:{}",
                        word_sha256(
                            format!(
                                "{}:{}\n{}\ninternal",
                                annotation_id.0, annotation_id.1, page.page_id
                            )
                            .as_bytes()
                        )
                    );
                    links.push(PdfLinkObservation {
                        link_id,
                        page: Some(page.clone()),
                        object_number: Some(annotation_id.0),
                        object_generation: Some(annotation_id.1),
                        kind: PdfLinkKind::InternalDestination,
                        target,
                        target_sha256: None,
                        external_or_active: false,
                        followed_or_executed: false,
                    });
                }
            }
            if annotation.has(b"AA") {
                findings.push(finding(
                    PdfArtifactFindingKind::AutomaticAction,
                    Some(annotation_id),
                    Some(&page.page_id),
                    "pdf.action.annotation-additional-inert",
                ));
            }
        }

        match document.get_page_images(*page_object_id) {
            Ok(page_images) if page_images.len() <= profile.maximum_images_per_page => {
                for image in page_images {
                    let width = u32::try_from(image.width)
                        .map_err(|_| PdfExtractionError::MalformedDocument)?;
                    let height = u32::try_from(image.height)
                        .map_err(|_| PdfExtractionError::MalformedDocument)?;
                    let mut filters = image.filters.unwrap_or_default();
                    filters.sort();
                    filters.dedup();
                    images.push(PdfImageObservation {
                        page: page.clone(),
                        object_number: image.id.0,
                        object_generation: image.id.1,
                        width,
                        height,
                        color_space: image.color_space,
                        filters,
                        encoded_stream_sha256: word_sha256(image.content),
                    });
                }
            }
            Ok(_) => return Err(PdfExtractionError::ResourceLimit),
            Err(_) => findings.push(finding(
                PdfArtifactFindingKind::InspectionLimited,
                Some(*page_object_id),
                Some(&page.page_id),
                "pdf.image.inventory-limited",
            )),
        }
    }

    for (object_id, object) in &document.objects {
        let dictionary = match object {
            Object::Dictionary(value) => Some(value),
            Object::Stream(value) => Some(&value.dict),
            _ => None,
        };
        let Some(dictionary) = dictionary else {
            continue;
        };
        if dictionary
            .get(b"Type")
            .ok()
            .and_then(|item| item.as_name().ok())
            == Some(b"EmbeddedFile")
        {
            findings.push(finding(
                PdfArtifactFindingKind::EmbeddedFile,
                Some(*object_id),
                None,
                "pdf.embedded-file.inert",
            ));
        }
        if let Ok(field_type) = dictionary.get(b"FT").and_then(Object::as_name) {
            let field_type = String::from_utf8_lossy(field_type).into_owned();
            if field_type.len() <= 32 {
                let field_name = dictionary.get(b"T").ok().and_then(decoded_string);
                let field_name_sha256 =
                    field_name.as_ref().map(|item| word_sha256(item.as_bytes()));
                forms.push(PdfFormFieldObservation {
                    field_id: format!(
                        "pdf-form:{}",
                        word_sha256(format!("{}\n{}", object_id.0, object_id.1).as_bytes())
                    ),
                    object_number: object_id.0,
                    object_generation: object_id.1,
                    field_type,
                    field_name,
                    field_name_sha256,
                    has_value: dictionary.has(b"V"),
                });
            }
        }
        if dictionary.has(b"S")
            && !observed_action_ids.contains(&object_id.0)
            && let Some((link, related)) = inspect_action(&document, object, Some(*object_id), None)
            && link.kind != PdfLinkKind::Unknown
        {
            links.push(link);
            findings.extend(related);
        }
    }

    if let Ok(root_id) = document.trailer.get(b"Root").and_then(Object::as_reference)
        && let Ok(root) = document.get_dictionary(root_id)
        && (root.has(b"OpenAction") || root.has(b"AA"))
    {
        findings.push(finding(
            PdfArtifactFindingKind::AutomaticAction,
            Some(root_id),
            None,
            "pdf.action.document-automatic-inert",
        ));
    }

    links.sort_by(|left, right| left.link_id.cmp(&right.link_id));
    links.dedup_by(|left, right| left.link_id == right.link_id);
    forms.sort_by(|left, right| left.field_id.cmp(&right.field_id));
    forms.dedup_by(|left, right| left.field_id == right.field_id);
    images.sort_by(|left, right| {
        (
            left.page.page_number,
            left.object_number,
            left.object_generation,
        )
            .cmp(&(
                right.page.page_number,
                right.object_number,
                right.object_generation,
            ))
    });
    findings.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    findings.dedup_by(|left, right| left.finding_id == right.finding_id);
    let inspection_complete = !findings
        .iter()
        .any(|item| item.kind == PdfArtifactFindingKind::InspectionLimited);
    let safe_for_generation_input = inspection_complete && findings.is_empty();
    Ok(PdfArtifactInspection {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_path: source_path.clone(),
        source_sha256,
        profile: profile.clone(),
        pdf_version: document.version.clone(),
        page_count: u32::try_from(page_objects.len())
            .map_err(|_| PdfExtractionError::ResourceLimit)?,
        encrypted: false,
        metadata: metadata(&document),
        links,
        forms,
        images,
        findings,
        extraction: Some(extraction),
        inspection_complete,
        safe_for_generation_input,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
    use lopdf::content::{Content, Operation};
    use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
    use lopdf::{Document, Object, Stream, dictionary};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-pdf-inspection"),
            ["docs", "inspected.pdf"],
        )
        .expect("path")
    }

    fn fixture(active: bool) -> Vec<u8> {
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        });
        let resources_id = document.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Tj", vec![Object::string_literal("Visible text")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            content.encode().expect("content"),
        ));
        let action_id = document.add_object(if active {
            Object::Dictionary(dictionary! {
                "Type" => "Action", "S" => "JavaScript", "JS" => Object::string_literal("ignored()"),
            })
        } else {
            Object::Dictionary(dictionary! {
                "Type" => "Action", "S" => "URI", "URI" => Object::string_literal("https://example.invalid"),
            })
        });
        let annotation_id = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Link", "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()],
            "A" => action_id,
        });
        let form_id = document.add_object(dictionary! {
            "FT" => "Tx", "T" => Object::string_literal("reviewer"), "V" => Object::string_literal("not-retained"),
        });
        let page_id = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "Resources" => resources_id, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Annots" => vec![annotation_id.into(), form_id.into()],
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog", "Pages" => pages_id,
            "AcroForm" => dictionary! { "Fields" => vec![form_id.into()] },
        });
        let info_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Review"),
            "Author" => Object::string_literal("AgentMage"),
            "CreationDate" => Object::string_literal("D:20260815000000Z"),
            "PrivateLabel" => Object::string_literal("value-not-retained"),
        });
        document.trailer.set("Root", catalog_id);
        document.trailer.set("Info", info_id);
        let mut output = Vec::new();
        document.save_to(&mut output).expect("save");
        output
    }

    #[test]
    fn extracts_metadata_links_forms_and_page_provenance_without_effects() {
        let source = fixture(false);
        let report =
            inspect_pdf_artifact(&path(), &source, &PdfExtractionProfile::strict_default())
                .expect("inspect");
        assert_eq!(report.metadata.title.as_deref(), Some("Review"));
        assert_eq!(report.metadata.author.as_deref(), Some("AgentMage"));
        assert_eq!(report.metadata.custom_keys, ["PrivateLabel"]);
        assert_eq!(report.page_count, 1);
        assert_eq!(report.links.len(), 1);
        assert_eq!(report.links[0].kind, PdfLinkKind::ExternalUri);
        assert!(!report.links[0].followed_or_executed);
        assert_eq!(report.forms.len(), 1);
        assert_eq!(report.forms[0].field_name.as_deref(), Some("reviewer"));
        assert!(report.forms[0].has_value);
        assert!(report.extraction.is_some());
        assert!(!report.network_access_performed);
        assert!(!report.execution_performed);
    }

    #[test]
    fn active_actions_are_inert_and_block_safe_reuse() {
        let source = fixture(true);
        let report =
            inspect_pdf_artifact(&path(), &source, &PdfExtractionProfile::strict_default())
                .expect("inspect");
        assert!(
            report
                .links
                .iter()
                .any(|item| item.kind == PdfLinkKind::JavaScript)
        );
        assert!(
            report
                .findings
                .iter()
                .any(|item| item.kind == PdfArtifactFindingKind::JavaScript)
        );
        assert!(!report.safe_for_generation_input);
        assert!(!report.execution_performed);
    }

    #[test]
    fn encrypted_source_returns_blocked_inventory_without_metadata_or_extraction() {
        let source = fixture(false);
        let mut document = Document::load_mem(&source).expect("load");
        document.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal(vec![1_u8; 16]),
                Object::string_literal(vec![2_u8; 16]),
            ]),
        );
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &document,
            owner_password: "owner",
            user_password: "",
            key_length: 128,
            permissions: Permissions::all(),
        })
        .expect("state");
        document.encrypt(&state).expect("encrypt");
        let mut encrypted = Vec::new();
        document.save_to(&mut encrypted).expect("save");
        let report =
            inspect_pdf_artifact(&path(), &encrypted, &PdfExtractionProfile::strict_default())
                .expect("blocked inventory");
        assert!(report.encrypted);
        assert!(report.extraction.is_none());
        assert!(!report.inspection_complete);
        assert!(!report.safe_for_generation_input);
    }
}
