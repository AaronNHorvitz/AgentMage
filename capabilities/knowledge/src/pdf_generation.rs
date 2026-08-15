//! Deterministic local-only HTML and PDF report proposal generation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use serde::{Deserialize, Serialize};

use crate::pdf_extraction::{PdfExtractionProfile, pdf_extractor_identity_sha256};
use crate::pdf_inspection::{PdfArtifactInspection, inspect_pdf_artifact};
use crate::word_ooxml::word_sha256;
use crate::{MarkdownDocument, MarkdownElementKind, MarkdownFidelityWarning};

const GENERATOR_ID: &str = "agentmage-pdf-report-v1;lopdf=0.44.0;html=csp-local-v1;font=pdf-base14-helvetica;network=denied;filesystem=denied";
const MAX_BLOCKS: usize = 4_096;
const MAX_ROWS: usize = 16_384;
const MAX_CELLS: usize = 64;
const MAX_TEXT_BYTES: usize = 256 * 1_024;
const MAX_HTML_BYTES: usize = 8 * 1_024 * 1_024;
const MAX_PDF_BYTES: usize = 64 * 1_024 * 1_024;
const MAX_PAGES: usize = 10_000;

/// Exact PDF page geometry in points.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfPageSettings {
    /// Page width in points.
    pub width_points: u32,
    /// Page height in points.
    pub height_points: u32,
    /// Top margin in points.
    pub margin_top_points: u32,
    /// Right margin in points.
    pub margin_right_points: u32,
    /// Bottom margin in points.
    pub margin_bottom_points: u32,
    /// Left margin in points.
    pub margin_left_points: u32,
    /// Baseline-to-baseline line height in points.
    pub line_height_points: u32,
}

impl PdfPageSettings {
    /// US Letter portrait with conservative report margins.
    #[must_use]
    pub const fn letter_default() -> Self {
        Self {
            width_points: 612,
            height_points: 792,
            margin_top_points: 54,
            margin_right_points: 54,
            margin_bottom_points: 54,
            margin_left_points: 54,
            line_height_points: 18,
        }
    }

    fn valid(&self) -> bool {
        (216..=2_000).contains(&self.width_points)
            && (216..=2_000).contains(&self.height_points)
            && self.margin_left_points >= 12
            && self.margin_right_points >= 12
            && self.margin_top_points >= 12
            && self.margin_bottom_points >= 12
            && self
                .margin_left_points
                .checked_add(self.margin_right_points)
                .is_some_and(|sum| sum + 72 < self.width_points)
            && self
                .margin_top_points
                .checked_add(self.margin_bottom_points)
                .is_some_and(|sum| sum + 72 < self.height_points)
            && (10..=72).contains(&self.line_height_points)
    }
}

/// Stable PDF report metadata controlled by the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfReportMetadata {
    /// Visible document title.
    pub title: String,
    /// Visible author identity.
    pub author: String,
    /// Optional subject.
    pub subject: Option<String>,
    /// Fixed PDF date string; no current clock is consulted.
    pub creation_date: String,
}

/// One deterministic text form field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfReportFormField {
    /// Stable field name.
    pub name: String,
    /// Visible field label.
    pub label: String,
    /// Optional initial value.
    pub initial_value: Option<String>,
}

/// Closed report block supported by the local PDF generator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PdfReportBlock {
    /// Heading at a level from one through six.
    Heading {
        /// Heading level.
        level: u8,
        /// Visible heading text.
        text: String,
    },
    /// Ordinary paragraph.
    Paragraph {
        /// Visible paragraph text.
        text: String,
    },
    /// One list item.
    ListItem {
        /// Visible list-item text.
        text: String,
    },
    /// One table row; contiguous rows form one table in HTML.
    TableRow {
        /// Ordered visible cells.
        cells: Vec<String>,
        /// Whether this row is a header.
        header: bool,
    },
    /// Internal link to a one-based generated page.
    InternalPageLink {
        /// Visible link label.
        label: String,
        /// One-based target page.
        target_page: u32,
    },
    /// Fillable single-line text form field.
    TextFormField {
        /// Exact field specification.
        field: PdfReportFormField,
    },
    /// Intentional vertical space.
    Spacer,
}

/// Exact deterministic report request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfReportRequest {
    /// Stable report identity.
    pub report_id: String,
    /// Proposed output path; generation does not write it.
    pub output_path: WorkspacePath,
    /// Controlled document metadata.
    pub metadata: PdfReportMetadata,
    /// Exact page geometry.
    pub page_settings: PdfPageSettings,
    /// Ordered report blocks.
    pub blocks: Vec<PdfReportBlock>,
    /// Optional exact source Markdown digest.
    pub source_markdown_sha256: Option<String>,
}

/// One visible generation fidelity limit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfGenerationLimit {
    /// Stable limit identity.
    pub limit_id: String,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Whether a human must review the generated proposal.
    pub human_review_required: bool,
}

/// Deterministic HTML and PDF proposal with reopened inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedPdfReport {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable report identity.
    pub report_id: String,
    /// Proposed output path.
    pub output_path: WorkspacePath,
    /// Generator identity digest.
    pub generator_identity_sha256: String,
    /// Parser identity used for reopened inspection.
    pub extractor_identity_sha256: String,
    /// Optional exact source Markdown digest.
    pub source_markdown_sha256: Option<String>,
    /// Escaped static HTML bytes.
    pub html: Vec<u8>,
    /// Digest of exact HTML bytes.
    pub html_sha256: String,
    /// Generated PDF bytes.
    pub pdf: Vec<u8>,
    /// Digest of exact PDF bytes.
    pub pdf_sha256: String,
    /// Reopened bounded inspection of the generated PDF.
    pub inspection: PdfArtifactInspection,
    /// Visible fidelity limits in canonical order.
    pub fidelity_limits: Vec<PdfGenerationLimit>,
    /// True because the HTML policy admits no remote resources.
    pub local_assets_only: bool,
    /// False because generation returns a proposal without writing it.
    pub filesystem_effect_performed: bool,
    /// False because no link, font, image, or stylesheet is fetched.
    pub network_access_performed: bool,
    /// False because no browser, script, renderer process, or document action is executed.
    pub execution_performed: bool,
}

/// Stable fail-closed PDF report generation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfGenerationError {
    /// Request identity, metadata, geometry, block, or field is invalid.
    InvalidInput,
    /// Output path is not a safe distinct `.pdf` proposal name.
    UnsafeOutputPath,
    /// Text contains a character unsupported by the pinned base-font profile.
    UnsupportedCharacter,
    /// A block, row, cell, page, HTML, or PDF resource ceiling was exceeded.
    ResourceLimit,
    /// PDF serialization failed.
    Serialization,
    /// Reopened bounded inspection failed or found unexpected active content.
    InspectionFailed,
}

impl PdfGenerationError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "pdf_generation.input.invalid",
            Self::UnsafeOutputPath => "pdf_generation.output_path.unsafe",
            Self::UnsupportedCharacter => "pdf_generation.character.unsupported",
            Self::ResourceLimit => "pdf_generation.resource.limit",
            Self::Serialization => "pdf_generation.serialization.failed",
            Self::InspectionFailed => "pdf_generation.inspection.failed",
        }
    }
}

impl std::fmt::Display for PdfGenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PdfGenerationError {}

#[derive(Clone)]
struct RenderLine {
    text: String,
    font_size: u8,
    internal_page: Option<u32>,
    form: Option<PdfReportFormField>,
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

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_text(value: &str, allow_empty: bool) -> bool {
    (allow_empty || !value.trim().is_empty())
        && value.len() <= MAX_TEXT_BYTES
        && value.bytes().all(|byte| {
            byte == b'\t' || byte == b'\n' || byte == b'\r' || (32..=126).contains(&byte)
        })
}

fn safe_output_path(path: &WorkspacePath) -> bool {
    path.components()
        .last()
        .is_some_and(|item| item.as_str().ends_with(".pdf") && item.as_str().len() > 4)
}

fn html_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
    output
}

fn normalize_line(value: &str) -> String {
    value
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn block_texts(block: &PdfReportBlock) -> Vec<&str> {
    match block {
        PdfReportBlock::Heading { text, .. }
        | PdfReportBlock::Paragraph { text }
        | PdfReportBlock::ListItem { text } => vec![text],
        PdfReportBlock::TableRow { cells, .. } => cells.iter().map(String::as_str).collect(),
        PdfReportBlock::InternalPageLink { label, .. } => vec![label],
        PdfReportBlock::TextFormField { field } => {
            let mut values = vec![field.name.as_str(), field.label.as_str()];
            if let Some(value) = field.initial_value.as_deref() {
                values.push(value);
            }
            values
        }
        PdfReportBlock::Spacer => Vec::new(),
    }
}

fn validate_request(request: &PdfReportRequest) -> Result<(), PdfGenerationError> {
    if !safe_output_path(&request.output_path) {
        return Err(PdfGenerationError::UnsafeOutputPath);
    }
    if !valid_identifier(&request.report_id)
        || !request.page_settings.valid()
        || request.blocks.is_empty()
        || request.blocks.len() > MAX_BLOCKS
        || !valid_text(&request.metadata.title, false)
        || !valid_text(&request.metadata.author, false)
        || request
            .metadata
            .subject
            .as_deref()
            .is_some_and(|value| !valid_text(value, false))
        || !valid_text(&request.metadata.creation_date, false)
        || request
            .source_markdown_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
    {
        return Err(PdfGenerationError::InvalidInput);
    }
    let mut form_names = BTreeSet::new();
    let mut row_count = 0_usize;
    for block in &request.blocks {
        if block_texts(block)
            .iter()
            .any(|value| !valid_text(value, false))
        {
            return Err(PdfGenerationError::UnsupportedCharacter);
        }
        match block {
            PdfReportBlock::Heading { level, .. } if !(1..=6).contains(level) => {
                return Err(PdfGenerationError::InvalidInput);
            }
            PdfReportBlock::TableRow { cells, .. }
                if cells.is_empty() || cells.len() > MAX_CELLS =>
            {
                return Err(PdfGenerationError::ResourceLimit);
            }
            PdfReportBlock::TableRow { .. } => {
                row_count += 1;
                if row_count > MAX_ROWS {
                    return Err(PdfGenerationError::ResourceLimit);
                }
            }
            PdfReportBlock::InternalPageLink { target_page, .. } if *target_page == 0 => {
                return Err(PdfGenerationError::InvalidInput);
            }
            PdfReportBlock::TextFormField { field }
                if !valid_identifier(&field.name) || !form_names.insert(field.name.clone()) =>
            {
                return Err(PdfGenerationError::InvalidInput);
            }
            _ => {}
        }
    }
    Ok(())
}

fn render_html(request: &PdfReportRequest) -> Result<Vec<u8>, PdfGenerationError> {
    let mut output = String::new();
    write!(
        &mut output,
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src 'self' data:; style-src 'unsafe-inline'; font-src 'self'\"><title>{}</title><style>body{{font-family:sans-serif;max-width:50rem;margin:2rem auto;line-height:1.5}}table{{border-collapse:collapse;width:100%}}th,td{{border:1px solid #555;padding:.35rem;text-align:left}}code,pre{{white-space:pre-wrap}}label{{display:block;margin:.5rem 0}}</style></head><body>\n<h1>{}</h1>\n",
        html_escape(&request.metadata.title),
        html_escape(&request.metadata.title)
    )
    .map_err(|_| PdfGenerationError::Serialization)?;
    let mut table_open = false;
    for block in &request.blocks {
        if !matches!(block, PdfReportBlock::TableRow { .. }) && table_open {
            output.push_str("</tbody></table>\n");
            table_open = false;
        }
        match block {
            PdfReportBlock::Heading { level, text } => {
                writeln!(&mut output, "<h{level}>{}</h{level}>", html_escape(text))
                    .map_err(|_| PdfGenerationError::Serialization)?;
            }
            PdfReportBlock::Paragraph { text } => {
                writeln!(&mut output, "<p>{}</p>", html_escape(text))
                    .map_err(|_| PdfGenerationError::Serialization)?;
            }
            PdfReportBlock::ListItem { text } => {
                writeln!(&mut output, "<p>&bull; {}</p>", html_escape(text))
                    .map_err(|_| PdfGenerationError::Serialization)?;
            }
            PdfReportBlock::TableRow { cells, header } => {
                if !table_open {
                    output.push_str("<table><tbody>\n");
                    table_open = true;
                }
                output.push_str("<tr>");
                let element = if *header { "th" } else { "td" };
                for cell in cells {
                    write!(&mut output, "<{element}>{}</{element}>", html_escape(cell))
                        .map_err(|_| PdfGenerationError::Serialization)?;
                }
                output.push_str("</tr>\n");
            }
            PdfReportBlock::InternalPageLink { label, target_page } => {
                writeln!(
                    &mut output,
                    "<p><a href=\"#page-{target_page}\">{}</a></p>",
                    html_escape(label)
                )
                .map_err(|_| PdfGenerationError::Serialization)?;
            }
            PdfReportBlock::TextFormField { field } => {
                writeln!(
                    &mut output,
                    "<label>{}<input name=\"{}\" value=\"{}\" autocomplete=\"off\"></label>",
                    html_escape(&field.label),
                    html_escape(&field.name),
                    html_escape(field.initial_value.as_deref().unwrap_or(""))
                )
                .map_err(|_| PdfGenerationError::Serialization)?;
            }
            PdfReportBlock::Spacer => output.push_str("<div aria-hidden=\"true\">&nbsp;</div>\n"),
        }
    }
    if table_open {
        output.push_str("</tbody></table>\n");
    }
    output.push_str("</body></html>\n");
    if output.len() > MAX_HTML_BYTES {
        return Err(PdfGenerationError::ResourceLimit);
    }
    Ok(output.into_bytes())
}

fn render_lines(request: &PdfReportRequest) -> Vec<RenderLine> {
    let mut lines = Vec::new();
    for block in &request.blocks {
        let line = match block {
            PdfReportBlock::Heading { level, text } => RenderLine {
                text: normalize_line(text),
                font_size: match level {
                    1 => 20,
                    2 => 18,
                    3 => 16,
                    _ => 14,
                },
                internal_page: None,
                form: None,
            },
            PdfReportBlock::Paragraph { text } => RenderLine {
                text: normalize_line(text),
                font_size: 11,
                internal_page: None,
                form: None,
            },
            PdfReportBlock::ListItem { text } => RenderLine {
                text: format!("- {}", normalize_line(text)),
                font_size: 11,
                internal_page: None,
                form: None,
            },
            PdfReportBlock::TableRow { cells, .. } => RenderLine {
                text: cells
                    .iter()
                    .map(|item| normalize_line(item))
                    .collect::<Vec<_>>()
                    .join(" | "),
                font_size: 10,
                internal_page: None,
                form: None,
            },
            PdfReportBlock::InternalPageLink { label, target_page } => RenderLine {
                text: normalize_line(label),
                font_size: 11,
                internal_page: Some(*target_page),
                form: None,
            },
            PdfReportBlock::TextFormField { field } => RenderLine {
                text: format!(
                    "{}: {}",
                    normalize_line(&field.label),
                    field.initial_value.as_deref().unwrap_or("")
                ),
                font_size: 11,
                internal_page: None,
                form: Some(field.clone()),
            },
            PdfReportBlock::Spacer => RenderLine {
                text: " ".to_owned(),
                font_size: 11,
                internal_page: None,
                form: None,
            },
        };
        lines.push(line);
    }
    lines
}

fn render_pdf(request: &PdfReportRequest) -> Result<Vec<u8>, PdfGenerationError> {
    let lines = render_lines(request);
    let settings = &request.page_settings;
    let usable_height = settings
        .height_points
        .checked_sub(settings.margin_top_points + settings.margin_bottom_points)
        .ok_or(PdfGenerationError::InvalidInput)?;
    let lines_per_page = usize::try_from(usable_height / settings.line_height_points)
        .map_err(|_| PdfGenerationError::ResourceLimit)?;
    if lines_per_page == 0 {
        return Err(PdfGenerationError::InvalidInput);
    }
    let page_count = lines.len().div_ceil(lines_per_page);
    if page_count == 0 || page_count > MAX_PAGES {
        return Err(PdfGenerationError::ResourceLimit);
    }
    for line in &lines {
        if line.internal_page.is_some_and(|page| {
            usize::try_from(page)
                .ok()
                .is_none_or(|page| page > page_count)
        }) {
            return Err(PdfGenerationError::InvalidInput);
        }
    }

    let mut document = Document::with_version("1.7");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let resources_id = document.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let mut page_ids = Vec::new();
    let mut page_line_ranges = Vec::new();
    for (page_index, page_lines) in lines.chunks(lines_per_page).enumerate() {
        let mut operations = Vec::new();
        for (line_index, line) in page_lines.iter().enumerate() {
            let y = i64::from(settings.height_points - settings.margin_top_points)
                - i64::try_from(line_index).map_err(|_| PdfGenerationError::ResourceLimit)?
                    * i64::from(settings.line_height_points);
            operations.extend([
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), i64::from(line.font_size).into()]),
                Operation::new(
                    "Td",
                    vec![i64::from(settings.margin_left_points).into(), y.into()],
                ),
                Operation::new("Tj", vec![Object::string_literal(line.text.as_bytes())]),
                Operation::new("ET", vec![]),
            ]);
        }
        let content = Content { operations }
            .encode()
            .map_err(|_| PdfGenerationError::Serialization)?;
        let content_id = document.add_object(Stream::new(dictionary! {}, content));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), settings.width_points.into(), settings.height_points.into()],
        });
        page_ids.push(page_id);
        page_line_ranges.push((page_index * lines_per_page, page_lines.len()));
    }

    let mut all_form_ids = Vec::new();
    for (page_index, (start, count)) in page_line_ranges.iter().copied().enumerate() {
        let mut annotation_ids = Vec::new();
        for (local_index, line) in lines[start..start + count].iter().enumerate() {
            let top = i64::from(settings.height_points - settings.margin_top_points)
                - i64::try_from(local_index).map_err(|_| PdfGenerationError::ResourceLimit)?
                    * i64::from(settings.line_height_points)
                + 3;
            let bottom = top - i64::from(settings.line_height_points);
            if let Some(target_page) = line.internal_page {
                let target_index = usize::try_from(target_page - 1)
                    .map_err(|_| PdfGenerationError::ResourceLimit)?;
                let annotation_id = document.add_object(dictionary! {
                    "Type" => "Annot", "Subtype" => "Link",
                    "Rect" => vec![settings.margin_left_points.into(), bottom.into(), (settings.width_points - settings.margin_right_points).into(), top.into()],
                    "Dest" => vec![page_ids[target_index].into(), Object::Name(b"Fit".to_vec())],
                    "F" => 4,
                });
                annotation_ids.push(annotation_id);
            }
            if let Some(field) = &line.form {
                let mut field_dictionary = dictionary! {
                    "Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx",
                    "T" => Object::string_literal(field.name.as_bytes()),
                    "TU" => Object::string_literal(field.label.as_bytes()),
                    "Rect" => vec![(settings.margin_left_points + 120).into(), bottom.into(), (settings.width_points - settings.margin_right_points).into(), top.into()],
                    "F" => 4,
                };
                if let Some(value) = &field.initial_value {
                    field_dictionary.set("V", Object::string_literal(value.as_bytes()));
                    field_dictionary.set("DV", Object::string_literal(value.as_bytes()));
                }
                let field_id = document.add_object(field_dictionary);
                annotation_ids.push(field_id);
                all_form_ids.push(field_id);
            }
        }
        if !annotation_ids.is_empty() {
            document
                .get_dictionary_mut(page_ids[page_index])
                .map_err(|_| PdfGenerationError::Serialization)?
                .set(
                    "Annots",
                    annotation_ids
                        .into_iter()
                        .map(Object::Reference)
                        .collect::<Vec<_>>(),
                );
        }
    }

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => page_ids.iter().copied().map(Object::Reference).collect::<Vec<_>>(),
            "Count" => i64::try_from(page_ids.len()).map_err(|_| PdfGenerationError::ResourceLimit)?,
        }),
    );
    let mut catalog = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
    if !all_form_ids.is_empty() {
        let acroform_id = document.add_object(dictionary! {
            "Fields" => all_form_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
            "NeedAppearances" => true,
        });
        catalog.set("AcroForm", acroform_id);
    }
    let catalog_id = document.add_object(catalog);
    let mut info = dictionary! {
        "Title" => Object::string_literal(request.metadata.title.as_bytes()),
        "Author" => Object::string_literal(request.metadata.author.as_bytes()),
        "Creator" => Object::string_literal("AgentMage"),
        "Producer" => Object::string_literal("AgentMage deterministic PDF report v1"),
        "CreationDate" => Object::string_literal(request.metadata.creation_date.as_bytes()),
    };
    if let Some(subject) = &request.metadata.subject {
        info.set("Subject", Object::string_literal(subject.as_bytes()));
    }
    let info_id = document.add_object(info);
    document.trailer.set("Root", catalog_id);
    document.trailer.set("Info", info_id);
    let request_bytes =
        serde_json::to_vec(request).map_err(|_| PdfGenerationError::Serialization)?;
    let request_sha256 = word_sha256(&request_bytes);
    document.trailer.set(
        "ID",
        Object::Array(vec![
            Object::string_literal(&request_sha256.as_bytes()[..16]),
            Object::string_literal(&request_sha256.as_bytes()[16..32]),
        ]),
    );
    let mut pdf = Vec::new();
    document
        .save_to(&mut pdf)
        .map_err(|_| PdfGenerationError::Serialization)?;
    if pdf.len() > MAX_PDF_BYTES {
        return Err(PdfGenerationError::ResourceLimit);
    }
    Ok(pdf)
}

/// Returns the exact deterministic report generator identity digest.
#[must_use]
pub fn pdf_generator_identity_sha256() -> String {
    word_sha256(GENERATOR_ID.as_bytes())
}

/// Produces deterministic local-only HTML and PDF bytes without writing the proposed artifact.
pub fn generate_pdf_report(
    request: &PdfReportRequest,
    inspection_profile: &PdfExtractionProfile,
) -> Result<GeneratedPdfReport, PdfGenerationError> {
    validate_request(request)?;
    let html = render_html(request)?;
    let pdf = render_pdf(request)?;
    let inspection = inspect_pdf_artifact(&request.output_path, &pdf, inspection_profile)
        .map_err(|_| PdfGenerationError::InspectionFailed)?;
    if inspection.encrypted
        || !inspection.inspection_complete
        || !inspection.safe_for_generation_input
        || inspection.extraction.is_none()
    {
        return Err(PdfGenerationError::InspectionFailed);
    }
    let fidelity_limits = vec![
        PdfGenerationLimit {
            limit_id: "pdf-generation.base14-ascii".to_owned(),
            reason_code: "pdf.generation.base14-ascii-only".to_owned(),
            human_review_required: true,
        },
        PdfGenerationLimit {
            limit_id: "pdf-generation.native-visual-review".to_owned(),
            reason_code: "pdf.generation.native-visual-review-required".to_owned(),
            human_review_required: true,
        },
    ];
    Ok(GeneratedPdfReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        report_id: request.report_id.clone(),
        output_path: request.output_path.clone(),
        generator_identity_sha256: pdf_generator_identity_sha256(),
        extractor_identity_sha256: pdf_extractor_identity_sha256(),
        source_markdown_sha256: request.source_markdown_sha256.clone(),
        html_sha256: word_sha256(&html),
        html,
        pdf_sha256: word_sha256(&pdf),
        pdf,
        inspection,
        fidelity_limits,
        local_assets_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn primary_markdown_lines(
    document: &MarkdownDocument,
) -> BTreeMap<u32, (MarkdownElementKind, String, Option<u8>)> {
    let mut lines = BTreeMap::new();
    for element in document.elements() {
        if matches!(
            element.kind,
            MarkdownElementKind::MarkdownLink
                | MarkdownElementKind::WikiLink
                | MarkdownElementKind::Frontmatter
        ) {
            continue;
        }
        let range = element.source_range;
        let Some(bytes) = document
            .source_bytes()
            .get(range.start_byte..range.end_byte)
        else {
            continue;
        };
        let source = String::from_utf8_lossy(bytes)
            .trim_end_matches(['\r', '\n'])
            .to_owned();
        lines
            .entry(range.start_line)
            .or_insert((element.kind, source, element.heading_level));
    }
    lines
}

fn table_cells(source: &str) -> Vec<String> {
    source
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|item| item.trim().to_owned())
        .collect()
}

fn strip_list_marker(source: &str) -> &str {
    let trimmed = source.trim();
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        return rest;
    }
    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
    trimmed
        .get(digits..)
        .and_then(|rest| rest.strip_prefix(". "))
        .unwrap_or(trimmed)
}

/// Converts one already parsed Markdown document into the closed PDF report request model.
pub fn pdf_report_request_from_markdown(
    document: &MarkdownDocument,
    report_id: String,
    output_path: WorkspacePath,
    metadata: PdfReportMetadata,
    page_settings: PdfPageSettings,
) -> Result<(PdfReportRequest, Vec<PdfGenerationLimit>), PdfGenerationError> {
    let mut blocks = Vec::new();
    for (_line, (kind, source, level)) in primary_markdown_lines(document) {
        let block = match kind {
            MarkdownElementKind::Heading => PdfReportBlock::Heading {
                level: level.ok_or(PdfGenerationError::InvalidInput)?,
                text: source.trim_start_matches('#').trim().to_owned(),
            },
            MarkdownElementKind::ListItem | MarkdownElementKind::Task => PdfReportBlock::ListItem {
                text: strip_list_marker(&source).to_owned(),
            },
            MarkdownElementKind::TableRow => PdfReportBlock::TableRow {
                cells: table_cells(&source),
                header: false,
            },
            MarkdownElementKind::Blank => PdfReportBlock::Spacer,
            MarkdownElementKind::CodeFence
            | MarkdownElementKind::CodeFenceContent
            | MarkdownElementKind::TextBlock => PdfReportBlock::Paragraph { text: source },
            MarkdownElementKind::MarkdownLink
            | MarkdownElementKind::WikiLink
            | MarkdownElementKind::Frontmatter => continue,
        };
        blocks.push(block);
    }
    let mut limits = document
        .fidelity_warnings()
        .iter()
        .map(|warning| {
            let reason = match warning {
                MarkdownFidelityWarning::DuplicateHeading => "pdf.markdown.duplicate-heading",
                MarkdownFidelityWarning::RawHtml => "pdf.markdown.raw-html-escaped",
                MarkdownFidelityWarning::ReferenceStyleLink => {
                    "pdf.markdown.reference-link-rendered-inert"
                }
                MarkdownFidelityWarning::SetextHeading => "pdf.markdown.setext-rendered-inert",
            };
            PdfGenerationLimit {
                limit_id: reason.to_owned(),
                reason_code: reason.to_owned(),
                human_review_required: true,
            }
        })
        .collect::<Vec<_>>();
    limits.sort_by(|left, right| left.limit_id.cmp(&right.limit_id));
    limits.dedup_by(|left, right| left.limit_id == right.limit_id);
    let request = PdfReportRequest {
        report_id,
        output_path,
        metadata,
        page_settings,
        blocks,
        source_markdown_sha256: Some(document.source_sha256().to_owned()),
    };
    validate_request(&request)?;
    Ok((request, limits))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-pdf-generation"),
            ["reports", name],
        )
        .expect("path")
    }

    fn request() -> PdfReportRequest {
        PdfReportRequest {
            report_id: "report-001".to_owned(),
            output_path: path("report.pdf"),
            metadata: PdfReportMetadata {
                title: "Review Report".to_owned(),
                author: "AgentMage".to_owned(),
                subject: Some("Local verification".to_owned()),
                creation_date: "D:20260815000000Z".to_owned(),
            },
            page_settings: PdfPageSettings::letter_default(),
            blocks: vec![
                PdfReportBlock::Heading {
                    level: 2,
                    text: "Findings".to_owned(),
                },
                PdfReportBlock::Paragraph {
                    text: "Literal <script> is inert text.".to_owned(),
                },
                PdfReportBlock::TableRow {
                    cells: vec!["Item".to_owned(), "State".to_owned()],
                    header: true,
                },
                PdfReportBlock::TableRow {
                    cells: vec!["Parser".to_owned(), "Pass".to_owned()],
                    header: false,
                },
                PdfReportBlock::InternalPageLink {
                    label: "Return to page one".to_owned(),
                    target_page: 1,
                },
                PdfReportBlock::TextFormField {
                    field: PdfReportFormField {
                        name: "reviewer".to_owned(),
                        label: "Reviewer".to_owned(),
                        initial_value: Some("Unassigned".to_owned()),
                    },
                },
            ],
            source_markdown_sha256: Some("a".repeat(64)),
        }
    }

    #[test]
    fn generates_deterministic_escaped_html_and_reopened_pdf_proposals() {
        let first = generate_pdf_report(&request(), &PdfExtractionProfile::strict_default())
            .expect("generate");
        let second = generate_pdf_report(&request(), &PdfExtractionProfile::strict_default())
            .expect("generate");
        assert_eq!(first, second);
        let html = String::from_utf8(first.html.clone()).expect("html");
        assert!(html.contains("default-src 'none'"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
        assert_eq!(
            first.inspection.metadata.title.as_deref(),
            Some("Review Report")
        );
        assert_eq!(first.inspection.forms.len(), 1);
        assert!(
            first
                .inspection
                .links
                .iter()
                .any(|item| item.kind == crate::PdfLinkKind::InternalDestination)
        );
        assert!(first.inspection.safe_for_generation_input);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
    }

    #[test]
    fn rejects_unsafe_names_remote_or_unicode_text_and_invalid_page_links() {
        let mut changed = request();
        changed.output_path = path("report.txt");
        assert_eq!(
            generate_pdf_report(&changed, &PdfExtractionProfile::strict_default())
                .expect_err("path"),
            PdfGenerationError::UnsafeOutputPath
        );
        let mut changed = request();
        changed.blocks[0] = PdfReportBlock::Paragraph {
            text: "Unicode snowman \u{2603}".to_owned(),
        };
        assert_eq!(
            generate_pdf_report(&changed, &PdfExtractionProfile::strict_default())
                .expect_err("character"),
            PdfGenerationError::UnsupportedCharacter
        );
        let mut changed = request();
        changed.blocks.push(PdfReportBlock::InternalPageLink {
            label: "missing".to_owned(),
            target_page: 99,
        });
        assert_eq!(
            generate_pdf_report(&changed, &PdfExtractionProfile::strict_default())
                .expect_err("target"),
            PdfGenerationError::InvalidInput
        );
    }

    #[test]
    fn parsed_markdown_becomes_closed_blocks_and_escaped_raw_html() {
        let markdown = MarkdownDocument::parse(
            path("source.md"),
            b"# Status\n\n- Ready\n\n| Item | State |\n| --- | --- |\n| Test | Pass |\n\n<script>ignored()</script>\n".to_vec(),
        )
        .expect("markdown");
        let (request, limits) = pdf_report_request_from_markdown(
            &markdown,
            "markdown-report".to_owned(),
            path("markdown.pdf"),
            PdfReportMetadata {
                title: "Status".to_owned(),
                author: "AgentMage".to_owned(),
                subject: None,
                creation_date: "D:20260815000000Z".to_owned(),
            },
            PdfPageSettings::letter_default(),
        )
        .expect("request");
        assert!(
            limits
                .iter()
                .any(|item| item.reason_code == "pdf.markdown.raw-html-escaped")
        );
        let generated = generate_pdf_report(&request, &PdfExtractionProfile::strict_default())
            .expect("generate");
        let html = String::from_utf8(generated.html).expect("html");
        assert!(html.contains("&lt;script&gt;ignored()&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
