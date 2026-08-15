//! Deterministic structured Markdown-to-Word package proposals.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Cursor, Write};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::word_ooxml::word_sha256;
use crate::{
    MarkdownDocument, MarkdownElementKind, MarkdownFidelityWarning, WordConversionProfile,
    WordInspectionReport, WordOoxmlError, inspect_docx,
};

const GENERATOR_ID: &str =
    "agentmage-markdown-to-word-v1;zip=8.6.0;quick-xml=0.41.0;stored=deterministic";
const MAX_GENERATED_PACKAGE_BYTES: usize = 64 * 1_024 * 1_024;

/// Closed generation limitation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordGenerationWarningKind {
    /// Frontmatter remains source metadata and is not rendered into the document body.
    FrontmatterNotRendered,
    /// Markdown links remain visible text and do not become active external relationships.
    LinksRenderedAsText,
    /// Raw HTML remains escaped visible text.
    RawHtmlRenderedAsText,
    /// Reference-style links remain escaped visible text.
    ReferenceStyleLinkRenderedAsText,
    /// Setext-like headings remain ordinary text.
    SetextHeadingRenderedAsText,
    /// Duplicate heading labels remain distinct document paragraphs.
    DuplicateHeadingRetained,
}

/// One stable generation limitation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordGenerationWarning {
    /// Closed warning class.
    pub kind: WordGenerationWarningKind,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Always true; the Markdown source remains authoritative for unsupported semantics.
    pub source_remains_authoritative: bool,
}

/// One generated OOXML part digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedWordPart {
    /// Canonical package-relative name.
    pub part_name: String,
    /// Exact generated content digest.
    pub content_sha256: String,
    /// Exact generated content byte count.
    pub bytes: u64,
}

/// Deterministic Word package proposal with no write authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedWordPackage {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable caller-supplied artifact identity.
    pub artifact_id: String,
    /// Exact Markdown source digest.
    pub source_markdown_sha256: String,
    /// Exact output workspace path proposed by the caller.
    pub output_path: WorkspacePath,
    /// Exact generator identity digest.
    pub generator_identity_sha256: String,
    /// Deterministic `.docx` bytes.
    pub package: Vec<u8>,
    /// Exact package digest.
    pub package_sha256: String,
    /// Generated part ledger in canonical order.
    pub parts: Vec<GeneratedWordPart>,
    /// Explicit Markdown-to-Word limitations.
    pub warnings: Vec<WordGenerationWarning>,
    /// Reopened structural inspection of the generated package.
    pub inspection: WordInspectionReport,
    /// True when heading styles and table structure are emitted semantically.
    pub accessibility_structure_complete: bool,
    /// Always true; package bytes remain an unpersisted proposal.
    pub proposal_only: bool,
    /// Always false; generation does not save the package.
    pub filesystem_effect_performed: bool,
    /// Always false; generation creates no external relationship and uses no network.
    pub network_access_performed: bool,
    /// Always false; generation executes no source content.
    pub execution_performed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WordBlock {
    Heading { level: u8, text: String },
    Paragraph(String),
    ListItem { ordered: bool, text: String },
    Code(String),
    Table(Vec<Vec<String>>),
}

pub(crate) fn valid_identifier(value: &str) -> bool {
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

/// Returns the exact admitted generator identity digest.
#[must_use]
pub fn word_generator_identity_sha256() -> String {
    word_sha256(GENERATOR_ID.as_bytes())
}

fn source_line(document: &MarkdownDocument, start: usize, end: usize) -> String {
    std::str::from_utf8(&document.source_bytes()[start..end])
        .expect("MarkdownDocument guarantees UTF-8")
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}

fn strip_list_marker(value: &str) -> (bool, String) {
    let trimmed = value.trim_start();
    for prefix in ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "+ "] {
        if let Some(text) = trimmed.strip_prefix(prefix) {
            return (false, text.to_owned());
        }
    }
    let marker_end = trimmed
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(trimmed.len());
    if marker_end > 0 {
        let rest = &trimmed[marker_end..];
        if let Some(text) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return (true, text.to_owned());
        }
    }
    (false, trimmed.to_owned())
}

fn table_cells(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

fn separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let candidate = cell.trim_matches(':');
            candidate.len() >= 3 && candidate.bytes().all(|byte| byte == b'-')
        })
}

fn flush_table(rows: &mut Vec<Vec<String>>, blocks: &mut Vec<WordBlock>) {
    if !rows.is_empty() {
        blocks.push(WordBlock::Table(std::mem::take(rows)));
    }
}

fn markdown_blocks(document: &MarkdownDocument) -> Vec<WordBlock> {
    let mut blocks = Vec::new();
    let mut rows = Vec::new();
    for element in document.elements() {
        let line = source_line(
            document,
            element.source_range.start_byte,
            element.source_range.end_byte,
        );
        match element.kind {
            MarkdownElementKind::TableRow => {
                let cells = table_cells(&line);
                if !separator_row(&cells) {
                    rows.push(cells);
                }
            }
            MarkdownElementKind::Heading => {
                flush_table(&mut rows, &mut blocks);
                blocks.push(WordBlock::Heading {
                    level: element.heading_level.unwrap_or(1),
                    text: element.label.clone(),
                });
            }
            MarkdownElementKind::ListItem | MarkdownElementKind::Task => {
                flush_table(&mut rows, &mut blocks);
                let (ordered, text) = strip_list_marker(&line);
                blocks.push(WordBlock::ListItem { ordered, text });
            }
            MarkdownElementKind::CodeFenceContent => {
                flush_table(&mut rows, &mut blocks);
                blocks.push(WordBlock::Code(line));
            }
            MarkdownElementKind::TextBlock => {
                flush_table(&mut rows, &mut blocks);
                blocks.push(WordBlock::Paragraph(line));
            }
            MarkdownElementKind::CodeFence
            | MarkdownElementKind::MarkdownLink
            | MarkdownElementKind::WikiLink
            | MarkdownElementKind::Blank
            | MarkdownElementKind::Frontmatter => {
                flush_table(&mut rows, &mut blocks);
            }
        }
    }
    flush_table(&mut rows, &mut blocks);
    blocks
}

pub(crate) fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character if character.is_control() && !matches!(character, '\t' | '\n' | '\r') => {
                escaped.push('\u{fffd}');
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn paragraph_xml(text: &str, style: Option<&str>, numbering: Option<u8>) -> String {
    let mut properties = String::new();
    if let Some(style) = style {
        write!(
            &mut properties,
            "<w:pStyle w:val=\"{}\"/>",
            xml_escape(style)
        )
        .expect("String write");
    }
    if let Some(numbering) = numbering {
        write!(
            &mut properties,
            "<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"{numbering}\"/></w:numPr>"
        )
        .expect("String write");
    }
    let ppr = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{properties}</w:pPr>")
    };
    format!(
        "<w:p>{ppr}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_escape(text)
    )
}

fn table_xml(rows: &[Vec<String>]) -> String {
    let mut output = String::from("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/></w:tblPr>");
    for row in rows {
        output.push_str("<w:tr>");
        for cell in row {
            output.push_str("<w:tc>");
            output.push_str(&paragraph_xml(cell, None, None));
            output.push_str("</w:tc>");
        }
        output.push_str("</w:tr>");
    }
    output.push_str("</w:tbl>");
    output
}

fn document_xml(blocks: &[WordBlock]) -> String {
    let mut body = String::new();
    for block in blocks {
        match block {
            WordBlock::Heading { level, text } => body.push_str(&paragraph_xml(
                text,
                Some(&format!("Heading{}", (*level).clamp(1, 6))),
                None,
            )),
            WordBlock::Paragraph(text) => body.push_str(&paragraph_xml(text, None, None)),
            WordBlock::ListItem { ordered, text } => {
                body.push_str(&paragraph_xml(text, None, Some(u8::from(*ordered) + 1)));
            }
            WordBlock::Code(text) => body.push_str(&paragraph_xml(text, Some("Code"), None)),
            WordBlock::Table(rows) => body.push_str(&table_xml(rows)),
        }
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>"
    )
}

fn package_parts(blocks: &[WordBlock]) -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        (
            "[Content_Types].xml".to_owned(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/><Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/></Types>".to_vec(),
        ),
        (
            "_rels/.rels".to_owned(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>".to_vec(),
        ),
        ("word/document.xml".to_owned(), document_xml(blocks).into_bytes()),
        (
            "word/_rels/document.xml.rels".to_owned(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/></Relationships>".to_vec(),
        ),
        (
            "word/numbering.xml".to_owned(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:abstractNum w:abstractNumId=\"0\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"bullet\"/></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId=\"1\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1.\"/></w:lvl></w:abstractNum><w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num><w:num w:numId=\"2\"><w:abstractNumId w:val=\"1\"/></w:num></w:numbering>".to_vec(),
        ),
        (
            "word/styles.xml".to_owned(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:rPr><w:b/><w:sz w:val=\"32\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:rPr><w:b/><w:sz w:val=\"28\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Code\"><w:name w:val=\"Code\"/><w:rPr><w:rFonts w:ascii=\"Courier New\"/></w:rPr></w:style><w:style w:type=\"table\" w:styleId=\"TableGrid\"><w:name w:val=\"Table Grid\"/><w:tblPr><w:tblBorders><w:top w:val=\"single\"/><w:left w:val=\"single\"/><w:bottom w:val=\"single\"/><w:right w:val=\"single\"/><w:insideH w:val=\"single\"/><w:insideV w:val=\"single\"/></w:tblBorders></w:tblPr></w:style></w:styles>".to_vec(),
        ),
    ])
}

pub(crate) fn zip_parts(parts: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, WordOoxmlError> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o644);
        for (name, content) in parts {
            writer
                .start_file(name, options)
                .map_err(|_| WordOoxmlError::MalformedPackage)?;
            writer
                .write_all(content)
                .map_err(|_| WordOoxmlError::MalformedPackage)?;
        }
        writer
            .finish()
            .map_err(|_| WordOoxmlError::MalformedPackage)?;
    }
    let package = cursor.into_inner();
    if package.len() > MAX_GENERATED_PACKAGE_BYTES {
        return Err(WordOoxmlError::ResourceLimit);
    }
    Ok(package)
}

fn generation_warnings(document: &MarkdownDocument) -> Vec<WordGenerationWarning> {
    let mut kinds = BTreeMap::new();
    if document.source_bytes().starts_with(b"---\n")
        || document.source_bytes().starts_with(b"---\r\n")
    {
        kinds.insert(
            WordGenerationWarningKind::FrontmatterNotRendered,
            "word.generation.frontmatter-not-rendered",
        );
    }
    if document.elements().iter().any(|item| {
        matches!(
            item.kind,
            MarkdownElementKind::MarkdownLink | MarkdownElementKind::WikiLink
        )
    }) {
        kinds.insert(
            WordGenerationWarningKind::LinksRenderedAsText,
            "word.generation.links-rendered-as-text",
        );
    }
    for warning in document.fidelity_warnings() {
        let (kind, reason) = match warning {
            MarkdownFidelityWarning::RawHtml => (
                WordGenerationWarningKind::RawHtmlRenderedAsText,
                "word.generation.raw-html-rendered-as-text",
            ),
            MarkdownFidelityWarning::ReferenceStyleLink => (
                WordGenerationWarningKind::ReferenceStyleLinkRenderedAsText,
                "word.generation.reference-link-rendered-as-text",
            ),
            MarkdownFidelityWarning::SetextHeading => (
                WordGenerationWarningKind::SetextHeadingRenderedAsText,
                "word.generation.setext-rendered-as-text",
            ),
            MarkdownFidelityWarning::DuplicateHeading => (
                WordGenerationWarningKind::DuplicateHeadingRetained,
                "word.generation.duplicate-heading-retained",
            ),
        };
        kinds.insert(kind, reason);
    }
    kinds
        .into_iter()
        .map(|(kind, reason_code)| WordGenerationWarning {
            kind,
            reason_code: reason_code.to_owned(),
            source_remains_authoritative: true,
        })
        .collect()
}

/// Generates deterministic `.docx` proposal bytes from supported Markdown structures.
pub fn generate_docx_from_markdown(
    artifact_id: &str,
    output_path: WorkspacePath,
    document: &MarkdownDocument,
    profile: &WordConversionProfile,
) -> Result<GeneratedWordPackage, WordOoxmlError> {
    if !valid_identifier(artifact_id) || output_path == *document.path() {
        return Err(WordOoxmlError::InvalidInput);
    }
    let blocks = markdown_blocks(document);
    if blocks.is_empty() {
        return Err(WordOoxmlError::InvalidInput);
    }
    let parts = package_parts(&blocks);
    let part_ledger = parts
        .iter()
        .map(|(part_name, content)| GeneratedWordPart {
            part_name: part_name.clone(),
            content_sha256: word_sha256(content),
            bytes: content.len() as u64,
        })
        .collect::<Vec<_>>();
    let package = zip_parts(&parts)?;
    let inspection = inspect_docx(&output_path, &package, profile)?;
    if inspection.quarantined || !inspection.inspection_complete {
        return Err(WordOoxmlError::QuarantinedPackage);
    }
    Ok(GeneratedWordPackage {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: artifact_id.to_owned(),
        source_markdown_sha256: document.source_sha256().to_owned(),
        output_path,
        generator_identity_sha256: word_generator_identity_sha256(),
        package_sha256: word_sha256(&package),
        package,
        parts: part_ledger,
        warnings: generation_warnings(document),
        inspection,
        accessibility_structure_complete: true,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::{MarkdownDocument, extract_docx_to_sidecar};

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-word-generation"),
            ["docs", name],
        )
        .expect("path")
    }

    fn markdown(source: &str) -> MarkdownDocument {
        MarkdownDocument::parse(path("source.md"), source.as_bytes().to_vec()).expect("Markdown")
    }

    #[test]
    fn generated_package_is_deterministic_reopenable_and_effect_free() {
        let source =
            markdown("# Report\n\nParagraph.\n\n- item\n\n| A | B |\n|---|---|\n| 1 | 2 |\n");
        let profile = WordConversionProfile::strict_default();
        let first =
            generate_docx_from_markdown("artifact-1", path("report.docx"), &source, &profile)
                .expect("generate");
        let second =
            generate_docx_from_markdown("artifact-1", path("report.docx"), &source, &profile)
                .expect("generate");
        assert_eq!(first, second);
        assert_eq!(first.parts.len(), 6);
        assert!(!first.inspection.quarantined);
        assert!(first.accessibility_structure_complete);
        assert!(first.proposal_only);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
        let extracted = extract_docx_to_sidecar(&path("report.docx"), &first.package, &profile)
            .expect("reopen");
        let sidecar = String::from_utf8(extracted.sidecar).expect("UTF-8");
        for expected in ["Report", "Paragraph.", "item", "A", "2"] {
            assert!(sidecar.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn code_frontmatter_and_external_link_source_remain_safe_and_visible() {
        let source = markdown(
            "---\ntitle: Safe Report\n---\n# Report\n\n[visible](https://example.invalid)\n\n<script>never()</script>\n\n```text\nraw <value>\n```\n",
        );
        let generated = generate_docx_from_markdown(
            "artifact-safe",
            path("safe.docx"),
            &source,
            &WordConversionProfile::strict_default(),
        )
        .expect("generate");
        for kind in [
            WordGenerationWarningKind::FrontmatterNotRendered,
            WordGenerationWarningKind::LinksRenderedAsText,
            WordGenerationWarningKind::RawHtmlRenderedAsText,
        ] {
            assert!(generated.warnings.iter().any(|item| item.kind == kind));
        }
        assert!(!generated.inspection.quarantined);
        assert!(!generated.inspection.network_access_performed);
        assert!(!generated.inspection.execution_performed);
        let document_part = generated
            .inspection
            .parts
            .iter()
            .find(|item| item.part_name == "word/document.xml")
            .expect("document part");
        assert!(document_part.content_sha256.is_some());
    }

    #[test]
    fn generation_rejects_source_overwrite_identity() {
        let source = markdown("# Report\n");
        assert_eq!(
            generate_docx_from_markdown(
                "artifact-1",
                path("source.md"),
                &source,
                &WordConversionProfile::strict_default(),
            )
            .expect_err("same path"),
            WordOoxmlError::InvalidInput
        );
    }
}
