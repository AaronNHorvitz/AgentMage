//! DOCX adapter for the shared structured source extraction contract.

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, DOCX_MEDIA_TYPE, StructuredSourceExtraction,
    StructuredSourceExtractionError, StructuredSourceExtractionRequest, StructuredSourceExtractor,
    StructuredSourceFormat, StructuredSourceProvenance, StructuredSourceSection,
    StructuredSourceSectionKind, StructuredSourceWarning,
};

use crate::word_ooxml::{
    WordConversionProfile, WordOoxmlError, WordPartKind, admitted_docx_parts, word_sha256,
};

const EXTRACTOR_ID: &str = "agentmage-word-structured-source-v1;word-ooxml-v1;network=denied;filesystem=denied;execution=denied";

/// Authority-free DOCX implementation of the shared structured source extractor.
#[derive(Clone, Copy, Debug, Default)]
pub struct WordStructuredSourceExtractor;

struct ProjectionBuilder<'a> {
    request: &'a StructuredSourceExtractionRequest,
    sections: Vec<StructuredSourceSection>,
    warnings: Vec<StructuredSourceWarning>,
    output_bytes: u64,
    next_warning: u32,
}

impl<'a> ProjectionBuilder<'a> {
    fn new(request: &'a StructuredSourceExtractionRequest) -> Self {
        Self {
            request,
            sections: Vec::new(),
            warnings: Vec::new(),
            output_bytes: 0,
            next_warning: 1,
        }
    }

    fn push(
        &mut self,
        kind: StructuredSourceSectionKind,
        content: String,
        provenance: StructuredSourceProvenance,
    ) -> Result<(), StructuredSourceExtractionError> {
        let byte_len = u64::try_from(content.len())
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        self.output_bytes = self
            .output_bytes
            .checked_add(byte_len)
            .ok_or(StructuredSourceExtractionError::ResourceLimit)?;
        if self.sections.len() >= self.request.maximum_sections as usize
            || self.output_bytes > self.request.maximum_output_bytes
        {
            return Err(StructuredSourceExtractionError::ResourceLimit);
        }
        let ordinal = u32::try_from(self.sections.len())
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        let section_id = format!("{}-section-{ordinal:08}", self.request.source_id);
        self.sections.push(StructuredSourceSection {
            parent_section_id: (ordinal != 0)
                .then(|| format!("{}-section-00000000", self.request.source_id)),
            section_id,
            ordinal,
            kind,
            content,
            provenance,
        });
        Ok(())
    }

    fn warn(&mut self, reason_code: &str, source_part: Option<&str>) {
        self.warnings.push(StructuredSourceWarning {
            warning_id: format!(
                "{}-warning-{:08}",
                self.request.source_id, self.next_warning
            ),
            reason_code: reason_code.to_owned(),
            source_part: source_part.map(str::to_owned),
            original_remains_authoritative: true,
        });
        self.next_warning += 1;
    }
}

#[derive(Default)]
struct WordPosition {
    paragraph: u32,
    run: u32,
    table: u32,
    row: u32,
    cell: u32,
    in_paragraph: bool,
    in_table: bool,
    in_row: bool,
    in_cell: bool,
    heading: bool,
    list_item: bool,
    hyperlink_id: Option<String>,
    revision: Option<StructuredSourceSectionKind>,
    stack: Vec<String>,
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn attribute_value(
    event: &BytesStart<'_>,
    expected: &[u8],
    reader: &Reader<&[u8]>,
) -> Option<String> {
    event
        .attributes()
        .with_checks(true)
        .filter_map(Result::ok)
        .find(|attribute| local_name(attribute.key.as_ref()) == expected)
        .and_then(|attribute| {
            attribute
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                .ok()
                .map(|value| value.into_owned())
        })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn provenance(
    part: &str,
    position: &WordPosition,
    start: Option<u64>,
    end: Option<u64>,
    relationship_id: Option<String>,
) -> StructuredSourceProvenance {
    StructuredSourceProvenance {
        source_part: part.to_owned(),
        structural_path: format!("/{part}/{}", position.stack.join("/")),
        start_byte: start,
        end_byte_exclusive: end,
        paragraph: position.in_paragraph.then_some(position.paragraph),
        run: position
            .in_paragraph
            .then_some(position.run)
            .filter(|value| *value > 0),
        table: position.in_table.then_some(position.table),
        row: position.in_row.then_some(position.row),
        cell: position.in_cell.then_some(position.cell),
        relationship_id,
        rendered_page: None,
    }
}

fn text_kind(part: &str, position: &WordPosition) -> StructuredSourceSectionKind {
    if let Some(kind) = position.revision {
        return kind;
    }
    if part.starts_with("word/header") {
        StructuredSourceSectionKind::Header
    } else if part.starts_with("word/footer") {
        StructuredSourceSectionKind::Footer
    } else if part == "word/comments.xml" {
        StructuredSourceSectionKind::Comment
    } else if part.contains("footnotes") || part.contains("endnotes") {
        StructuredSourceSectionKind::Note
    } else if position.in_cell {
        StructuredSourceSectionKind::TableCell
    } else if position.hyperlink_id.is_some() {
        StructuredSourceSectionKind::Link
    } else if position.heading {
        StructuredSourceSectionKind::Heading
    } else if position.list_item {
        StructuredSourceSectionKind::ListItem
    } else {
        StructuredSourceSectionKind::Paragraph
    }
}

fn start_element(
    part: &str,
    event: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
    position: &mut WordPosition,
    builder: &mut ProjectionBuilder<'_>,
) -> Result<(), StructuredSourceExtractionError> {
    let name = String::from_utf8_lossy(local_name(event.name().as_ref())).into_owned();
    position.stack.push(name.clone());
    match name.as_str() {
        "p" => {
            position.paragraph = position.paragraph.saturating_add(1);
            position.run = 0;
            position.in_paragraph = true;
            position.heading = false;
            position.list_item = false;
        }
        "r" => position.run = position.run.saturating_add(1),
        "tbl" => {
            position.table = position.table.saturating_add(1);
            position.row = 0;
            position.cell = 0;
            position.in_table = true;
            builder.push(
                StructuredSourceSectionKind::Table,
                String::new(),
                provenance(part, position, None, None, None),
            )?;
        }
        "tr" => {
            position.row = position.row.saturating_add(1);
            position.cell = 0;
            position.in_row = true;
            builder.push(
                StructuredSourceSectionKind::TableRow,
                String::new(),
                provenance(part, position, None, None, None),
            )?;
        }
        "tc" => {
            position.cell = position.cell.saturating_add(1);
            position.in_cell = true;
        }
        "pStyle" => {
            position.heading = attribute_value(event, b"val", reader)
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("heading"));
        }
        "numPr" => position.list_item = true,
        "hyperlink" => position.hyperlink_id = attribute_value(event, b"id", reader),
        "ins" => position.revision = Some(StructuredSourceSectionKind::TrackedInsertion),
        "del" => position.revision = Some(StructuredSourceSectionKind::TrackedDeletion),
        "drawing" | "pict" => builder.push(
            StructuredSourceSectionKind::Image,
            String::new(),
            provenance(part, position, None, None, None),
        )?,
        "object" | "altChunk" | "smartTag" => {
            builder.push(
                StructuredSourceSectionKind::Unsupported,
                String::new(),
                provenance(part, position, None, None, None),
            )?;
            builder.warn("word.structured.unsupported-construct", Some(part));
        }
        _ => {}
    }
    Ok(())
}

fn end_element(name: &[u8], position: &mut WordPosition) {
    match local_name(name) {
        b"p" => {
            position.in_paragraph = false;
            position.heading = false;
            position.list_item = false;
            position.run = 0;
        }
        b"tbl" => position.in_table = false,
        b"tr" => position.in_row = false,
        b"tc" => position.in_cell = false,
        b"hyperlink" => position.hyperlink_id = None,
        b"ins" | b"del" => position.revision = None,
        _ => {}
    }
    position.stack.pop();
}

fn parse_word_part(
    part: &str,
    content: &[u8],
    builder: &mut ProjectionBuilder<'_>,
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<(), StructuredSourceExtractionError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    let mut position = WordPosition::default();
    let mut capture_start: Option<u64> = None;
    loop {
        if cancelled() {
            return Err(StructuredSourceExtractionError::Cancelled);
        }
        let start = reader.buffer_position();
        let event = reader
            .read_event()
            .map_err(|_| StructuredSourceExtractionError::Malformed)?;
        match event {
            Event::Start(event) => {
                start_element(part, &event, &reader, &mut position, builder)?;
                if matches!(
                    local_name(event.name().as_ref()),
                    b"t" | b"delText" | b"instrText"
                ) {
                    capture_start = Some(reader.buffer_position());
                }
            }
            Event::Empty(event) => {
                start_element(part, &event, &reader, &mut position, builder)?;
                end_element(event.name().as_ref(), &mut position);
            }
            Event::End(event) => {
                if matches!(
                    local_name(event.name().as_ref()),
                    b"t" | b"delText" | b"instrText"
                ) {
                    let content_start = capture_start
                        .take()
                        .ok_or(StructuredSourceExtractionError::Malformed)?;
                    let start_usize = usize::try_from(content_start)
                        .map_err(|_| StructuredSourceExtractionError::Malformed)?;
                    let end_usize = usize::try_from(start)
                        .map_err(|_| StructuredSourceExtractionError::Malformed)?;
                    let encoded = std::str::from_utf8(
                        content
                            .get(start_usize..end_usize)
                            .ok_or(StructuredSourceExtractionError::Malformed)?,
                    )
                    .map_err(|_| StructuredSourceExtractionError::Malformed)?;
                    let decoded = quick_xml::escape::unescape(encoded)
                        .map_err(|_| StructuredSourceExtractionError::Malformed)?
                        .into_owned();
                    if !decoded.is_empty() {
                        let kind = text_kind(part, &position);
                        let relationship_id = position.hyperlink_id.clone();
                        builder.push(
                            kind,
                            decoded,
                            provenance(
                                part,
                                &position,
                                Some(content_start),
                                Some(start),
                                relationship_id,
                            ),
                        )?;
                    }
                }
                end_element(event.name().as_ref(), &mut position);
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn parse_relationships(
    part: &str,
    content: &[u8],
    builder: &mut ProjectionBuilder<'_>,
) -> Result<(), StructuredSourceExtractionError> {
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    let mut ordinal = 0_u32;
    loop {
        let start = reader.buffer_position();
        match reader
            .read_event()
            .map_err(|_| StructuredSourceExtractionError::Malformed)?
        {
            Event::Start(event) | Event::Empty(event)
                if local_name(event.name().as_ref()) == b"Relationship" =>
            {
                ordinal = ordinal.saturating_add(1);
                let relationship_id = attribute_value(&event, b"Id", &reader)
                    .ok_or(StructuredSourceExtractionError::Malformed)?;
                if !valid_id(&relationship_id) {
                    return Err(StructuredSourceExtractionError::Malformed);
                }
                let target = attribute_value(&event, b"Target", &reader).unwrap_or_default();
                let relation_type = attribute_value(&event, b"Type", &reader).unwrap_or_default();
                let image = relation_type.to_ascii_lowercase().contains("/image")
                    || target.to_ascii_lowercase().contains("media/");
                builder.push(
                    if image {
                        StructuredSourceSectionKind::Image
                    } else {
                        StructuredSourceSectionKind::Relationship
                    },
                    String::new(),
                    StructuredSourceProvenance {
                        source_part: part.to_owned(),
                        structural_path: format!("/{part}/Relationship[{ordinal}]"),
                        start_byte: Some(start),
                        end_byte_exclusive: Some(reader.buffer_position()),
                        paragraph: None,
                        run: None,
                        table: None,
                        row: None,
                        cell: None,
                        relationship_id: Some(relationship_id),
                        rendered_page: None,
                    },
                )?;
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn map_error(error: WordOoxmlError) -> StructuredSourceExtractionError {
    match error {
        WordOoxmlError::InvalidInput => StructuredSourceExtractionError::InvalidInput,
        WordOoxmlError::ResourceLimit => StructuredSourceExtractionError::ResourceLimit,
        WordOoxmlError::QuarantinedPackage => StructuredSourceExtractionError::Quarantined,
        WordOoxmlError::MalformedPackage | WordOoxmlError::MalformedXml => {
            StructuredSourceExtractionError::Malformed
        }
    }
}

impl StructuredSourceExtractor for WordStructuredSourceExtractor {
    fn extractor_sha256(&self) -> String {
        word_sha256(EXTRACTOR_ID.as_bytes())
    }

    fn extract(
        &self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<StructuredSourceExtraction, StructuredSourceExtractionError> {
        if !request.supported_version()
            || !valid_id(&request.source_id)
            || request.media_type != DOCX_MEDIA_TYPE
            || !valid_sha256(&request.source_sha256)
            || request.source_sha256 != word_sha256(source)
            || source.is_empty()
            || request.maximum_sections == 0
            || request.maximum_sections > 100_000
            || request.maximum_output_bytes == 0
            || request.maximum_output_bytes > 128 * 1_024 * 1_024
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        if cancelled() {
            return Err(StructuredSourceExtractionError::Cancelled);
        }
        let (inspection, parts) = admitted_docx_parts(
            &request.source_path,
            source,
            &WordConversionProfile::strict_default(),
        )
        .map_err(map_error)?;
        let mut builder = ProjectionBuilder::new(request);
        builder.push(
            StructuredSourceSectionKind::Document,
            String::new(),
            StructuredSourceProvenance {
                source_part: "word/document.xml".to_owned(),
                structural_path: "/word/document.xml".to_owned(),
                start_byte: None,
                end_byte_exclusive: None,
                paragraph: None,
                run: None,
                table: None,
                row: None,
                cell: None,
                relationship_id: None,
                rendered_page: None,
            },
        )?;
        for (part, content) in &parts {
            if cancelled() {
                return Err(StructuredSourceExtractionError::Cancelled);
            }
            let kind = inspection
                .parts
                .iter()
                .find(|item| item.part_name == *part)
                .map(|item| item.kind)
                .unwrap_or(WordPartKind::OtherBinary);
            match kind {
                WordPartKind::MainDocument
                | WordPartKind::Header
                | WordPartKind::Footer
                | WordPartKind::Comments => {
                    parse_word_part(part, content, &mut builder, cancelled)?;
                }
                WordPartKind::Relationships => {
                    parse_relationships(part, content, &mut builder)?;
                }
                WordPartKind::Media => builder.push(
                    StructuredSourceSectionKind::Image,
                    String::new(),
                    StructuredSourceProvenance {
                        source_part: part.clone(),
                        structural_path: format!("/{part}"),
                        start_byte: Some(0),
                        end_byte_exclusive: Some(content.len() as u64),
                        paragraph: None,
                        run: None,
                        table: None,
                        row: None,
                        cell: None,
                        relationship_id: None,
                        rendered_page: None,
                    },
                )?,
                WordPartKind::OtherXml | WordPartKind::OtherBinary => {
                    builder.push(
                        StructuredSourceSectionKind::Unsupported,
                        String::new(),
                        StructuredSourceProvenance {
                            source_part: part.clone(),
                            structural_path: format!("/{part}"),
                            start_byte: Some(0),
                            end_byte_exclusive: Some(content.len() as u64),
                            paragraph: None,
                            run: None,
                            table: None,
                            row: None,
                            cell: None,
                            relationship_id: None,
                            rendered_page: None,
                        },
                    )?;
                    builder.warn("word.structured.part-preserved-not-interpreted", Some(part));
                }
                WordPartKind::Numbering | WordPartKind::Styles | WordPartKind::Properties => {}
            }
        }
        for warning in inspection
            .features
            .iter()
            .filter_map(|feature| match feature.kind {
                crate::WordFeatureKind::SectionLayout => {
                    Some("word.structured.rendered-page-unavailable")
                }
                crate::WordFeatureKind::Field => Some("word.structured.field-not-executed"),
                crate::WordFeatureKind::UnsupportedConstruct => {
                    Some("word.structured.unsupported-construct")
                }
                _ => None,
            })
        {
            builder.warn(warning, None);
        }
        builder
            .warnings
            .sort_by(|left, right| left.warning_id.cmp(&right.warning_id));
        Ok(StructuredSourceExtraction {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: request.source_id.clone(),
            format: StructuredSourceFormat::Docx,
            source_sha256: request.source_sha256.clone(),
            extractor_sha256: self.extractor_sha256(),
            sections: builder.sections,
            warnings: builder.warnings,
            output_bytes: builder.output_bytes,
            extraction_complete: true,
            original_preserved: true,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

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

    fn source() -> Vec<u8> {
        package(&[
            (
                "[Content_Types].xml",
                "<Types><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>",
            ),
            (
                "_rels/.rels",
                "<Relationships><Relationship Id=\"rId1\" Target=\"word/document.xml\"/></Relationships>",
            ),
            (
                "word/_rels/document.xml.rels",
                "<Relationships><Relationship Id=\"rImg\" Type=\"image\" Target=\"media/image1.png\"/></Relationships>",
            ),
            ("word/media/image1.png", "inert-image"),
            (
                "word/header1.xml",
                "<w:hdr xmlns:w=\"w\"><w:p><w:r><w:t>Header</w:t></w:r></w:p></w:hdr>",
            ),
            (
                "word/comments.xml",
                "<w:comments xmlns:w=\"w\"><w:comment><w:p><w:r><w:t>Comment</w:t></w:r></w:p></w:comment></w:comments>",
            ),
            (
                "word/document.xml",
                "<w:document xmlns:w=\"w\" xmlns:r=\"r\"><w:body><w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:hyperlink r:id=\"rLink\"><w:r><w:t>Link</w:t></w:r></w:hyperlink><w:ins><w:r><w:t>New</w:t></w:r></w:ins><w:del><w:r><w:delText>Old</w:delText></w:r></w:del></w:p></w:body></w:document>",
            ),
        ])
    }

    fn request(source: &[u8]) -> StructuredSourceExtractionRequest {
        StructuredSourceExtractionRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: "word-source".to_owned(),
            media_type: DOCX_MEDIA_TYPE.to_owned(),
            source_sha256: word_sha256(source),
            source_path: WorkspacePath::new(
                WorkspaceId::from_raw("word-workspace"),
                ["docs", "source.docx"],
            )
            .expect("path"),
            maximum_sections: 100,
            maximum_output_bytes: 1_024 * 1_024,
        }
    }

    #[test]
    fn maps_structure_and_exact_provenance_without_effects() {
        let source = source();
        let result = WordStructuredSourceExtractor
            .extract(&request(&source), &source, &mut || false)
            .expect("extract");
        for kind in [
            StructuredSourceSectionKind::Document,
            StructuredSourceSectionKind::Heading,
            StructuredSourceSectionKind::Table,
            StructuredSourceSectionKind::TableRow,
            StructuredSourceSectionKind::TableCell,
            StructuredSourceSectionKind::Header,
            StructuredSourceSectionKind::Comment,
            StructuredSourceSectionKind::Link,
            StructuredSourceSectionKind::Image,
            StructuredSourceSectionKind::Relationship,
            StructuredSourceSectionKind::TrackedInsertion,
            StructuredSourceSectionKind::TrackedDeletion,
        ] {
            assert!(
                result.sections.iter().any(|section| section.kind == kind),
                "{kind:?}"
            );
        }
        let cell = result
            .sections
            .iter()
            .find(|section| section.content == "Cell")
            .expect("cell");
        assert_eq!(cell.provenance.table, Some(1));
        assert_eq!(cell.provenance.row, Some(1));
        assert_eq!(cell.provenance.cell, Some(1));
        assert!(cell.provenance.start_byte.is_some());
        assert!(result.original_preserved);
        assert!(!result.filesystem_effect_performed);
        assert!(!result.network_access_performed);
        assert!(!result.execution_performed);
        assert_eq!(
            word_sha256(&serde_json::to_vec(&result).expect("golden projection")),
            "7d26751511195387cc28873e9a1955e6cd02bd9ea322aaf675487112d41fc3c2"
        );
    }

    #[test]
    fn cancellation_bounds_and_active_content_fail_closed() {
        let source = source();
        assert_eq!(
            WordStructuredSourceExtractor.extract(&request(&source), &source, &mut || true),
            Err(StructuredSourceExtractionError::Cancelled)
        );
        let mut bounded = request(&source);
        bounded.maximum_sections = 1;
        assert_eq!(
            WordStructuredSourceExtractor.extract(&bounded, &source, &mut || false),
            Err(StructuredSourceExtractionError::ResourceLimit)
        );
        let active = package(&[
            ("[Content_Types].xml", "<Types/>"),
            ("_rels/.rels", "<Relationships/>"),
            (
                "word/document.xml",
                "<w:document xmlns:w=\"w\"><w:body/></w:document>",
            ),
            ("word/vbaProject.bin", "inert"),
        ]);
        assert_eq!(
            WordStructuredSourceExtractor.extract(&request(&active), &active, &mut || false),
            Err(StructuredSourceExtractionError::Quarantined)
        );
    }
}
