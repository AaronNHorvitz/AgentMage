//! Deterministic presentation generation and full-regeneration editing.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};

use crate::presentation_ooxml::{
    PresentationInspection, PresentationObjectKind, PresentationProfile, inspect_pptx,
};
use crate::word_generation::zip_parts;
use crate::word_ooxml::word_sha256;

const MAX_SLIDES: usize = 256;
const MAX_BLOCKS_PER_SLIDE: usize = 5;
const MAX_TEXT_BYTES: usize = 16 * 1_024;
const MAX_BULLET_ITEMS: usize = 64;
const MAX_SPEAKER_NOTES: usize = 128;
const MAX_TABLE_ROWS: usize = 12;
const MAX_TABLE_COLUMNS: usize = 8;
const MAX_CHART_POINTS: usize = 64;
const MAX_DIAGRAM_NODES: usize = 6;
const MAX_DIAGRAM_EDGES: usize = 32;
const SLIDE_WIDTH: u64 = 12_192_000;
const SLIDE_HEIGHT: u64 = 6_858_000;

type PresentationParts = BTreeMap<String, Vec<u8>>;
type PackagedPresentation = (PresentationParts, Vec<PresentationSlidePreview>);

/// One deterministic table backed by an exact data-source digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationTableSpec {
    /// Exact column headers.
    pub headers: Vec<String>,
    /// Exact ordered rows.
    pub rows: Vec<Vec<String>>,
    /// SHA-256 of canonical `(headers, rows)` data.
    pub data_source_sha256: String,
}

/// One deterministic chart backed by an exact data-source digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationChartSpec {
    /// Chart title.
    pub title: String,
    /// Ordered category labels.
    pub categories: Vec<String>,
    /// Ordered integer values.
    pub values: Vec<i64>,
    /// SHA-256 of canonical `(categories, values)` data.
    pub data_source_sha256: String,
}

/// One node in a deterministic local diagram.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationDiagramNode {
    /// Stable node identity.
    pub node_id: String,
    /// Visible node label.
    pub label: String,
}

/// One directed edge in a deterministic local diagram.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationDiagramEdge {
    /// Existing source node identity.
    pub from: String,
    /// Existing destination node identity.
    pub to: String,
}

/// One deterministic diagram backed by an exact data-source digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationDiagramSpec {
    /// Ordered nodes.
    pub nodes: Vec<PresentationDiagramNode>,
    /// Ordered edges.
    pub edges: Vec<PresentationDiagramEdge>,
    /// SHA-256 of canonical `(nodes, edges)` data.
    pub data_source_sha256: String,
}

/// Closed generated slide block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresentationBlock {
    /// One text paragraph.
    Text {
        /// Visible inert text.
        text: String,
    },
    /// One ordered bullet list.
    Bullets {
        /// Visible inert list items.
        items: Vec<String>,
    },
    /// One comparison or data table.
    Table {
        /// Exact table specification.
        table: PresentationTableSpec,
    },
    /// One local column chart.
    Chart {
        /// Exact chart specification.
        chart: PresentationChartSpec,
    },
    /// One local node-edge diagram.
    Diagram {
        /// Exact diagram specification.
        diagram: PresentationDiagramSpec,
    },
}

/// Exact source specification for one generated slide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSlideSpec {
    /// Stable slide identity.
    pub slide_id: String,
    /// Visible title.
    pub title: String,
    /// Ordered content blocks.
    pub blocks: Vec<PresentationBlock>,
    /// Ordered speaker-note paragraphs.
    pub speaker_notes: Vec<String>,
}

/// Exact source specification for one generated deck.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationDeckSpec {
    /// Stable deck identity.
    pub deck_id: String,
    /// Canonical proposed `.pptx` output path.
    pub output_path: WorkspacePath,
    /// Stable fixed metadata timestamp supplied by the caller.
    pub fixed_timestamp: String,
    /// Ordered slides.
    pub slides: Vec<PresentationSlideSpec>,
}

/// One deterministic structural object preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationObjectPreview {
    /// One-based object order.
    pub object_order: u32,
    /// Closed object class.
    pub kind: PresentationObjectKind,
    /// Exact canonical source-block digest.
    pub content_sha256: String,
    /// Left position in English Metric Units.
    pub x: u64,
    /// Top position in English Metric Units.
    pub y: u64,
    /// Width in English Metric Units.
    pub width: u64,
    /// Height in English Metric Units.
    pub height: u64,
    /// Exact data-source digest for data-backed visuals.
    pub data_source_sha256: Option<String>,
}

/// Exact structural slide preview used before native rendering is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSlidePreview {
    /// One-based slide number.
    pub slide_number: u32,
    /// Stable slide identity.
    pub slide_id: String,
    /// Fixed slide width.
    pub width: u64,
    /// Fixed slide height.
    pub height: u64,
    /// Ordered object previews.
    pub objects: Vec<PresentationObjectPreview>,
    /// Exact digest of all preview fields except this digest.
    pub preview_sha256: String,
    /// True because native rendering remains required.
    pub native_render_required: bool,
}

/// Deterministic generated presentation proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedPresentation {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Exact source specification retained for controlled regeneration.
    pub specification: PresentationDeckSpec,
    /// Exact generated PowerPoint Open XML bytes.
    pub pptx: Vec<u8>,
    /// SHA-256 of exact generated bytes.
    pub pptx_sha256: String,
    /// Bounded direct reopen inspection.
    pub inspection: PresentationInspection,
    /// Exact structural previews.
    pub previews: Vec<PresentationSlidePreview>,
    /// True because this is a proposal and no path was written.
    pub proposal_only: bool,
    /// False because generation is in memory.
    pub filesystem_effect_performed: bool,
    /// False because generation uses no network.
    pub network_access_performed: bool,
    /// False because no slide content is executed or rendered.
    pub execution_performed: bool,
}

/// One exact slide replacement in a full-regeneration edit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSlideReplacement {
    /// One-based slide number.
    pub slide_number: u32,
    /// Complete replacement specification.
    pub replacement: PresentationSlideSpec,
}

/// Controlled edit request bound to exact generated bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationEditRequest {
    /// Exact source deck digest.
    pub source_sha256: String,
    /// Strictly ordered unique slide replacements.
    pub replacements: Vec<PresentationSlideReplacement>,
}

/// One exact before-and-after slide change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSlideChange {
    /// One-based slide number.
    pub slide_number: u32,
    /// Exact original structural preview digest.
    pub before_preview_sha256: String,
    /// Exact regenerated structural preview digest.
    pub after_preview_sha256: String,
}

/// Full-regeneration controlled edit result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditedPresentation {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Exact source deck digest.
    pub source_sha256: String,
    /// Exact regenerated proposal.
    pub generated: GeneratedPresentation,
    /// Canonically ordered changed slides.
    pub changes: Vec<PresentationSlideChange>,
    /// Count of untouched slides whose preview hashes remained exact.
    pub unchanged_slide_count: u32,
    /// True because source bytes remain unchanged.
    pub original_preserved: bool,
    /// False because editing is full in-memory regeneration.
    pub filesystem_effect_performed: bool,
    /// False because editing uses no network.
    pub network_access_performed: bool,
    /// False because no slide content executes or renders.
    pub execution_performed: bool,
}

/// Stable presentation generation or editing failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationGenerationError {
    /// Path, identity, timestamp, slide, block, data, or edit request is invalid.
    InvalidInput,
    /// Slide, block, text, table, chart, diagram, or package limit was exceeded.
    ResourceLimit,
    /// The package could not be serialized or safely reopened.
    InvalidGeneratedPackage,
    /// Edit source identity does not match exact generated bytes.
    SourceMismatch,
}

impl PresentationGenerationError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "presentation.generation.input-invalid",
            Self::ResourceLimit => "presentation.generation.resource-limit",
            Self::InvalidGeneratedPackage => "presentation.generation.package-invalid",
            Self::SourceMismatch => "presentation.edit.source-mismatch",
        }
    }
}

impl std::fmt::Display for PresentationGenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PresentationGenerationError {}

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
        && value
            .chars()
            .all(|character| matches!(character, '\t' | '\n' | '\r') || !character.is_control())
}

fn safe_output_path(path: &WorkspacePath) -> bool {
    path.components()
        .last()
        .is_some_and(|item| item.as_str().ends_with(".pptx") && item.as_str().len() > 5)
}

fn valid_timestamp(value: &str) -> bool {
    value.len() == 20
        && value.ends_with('Z')
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            10 => byte == b'T',
            13 | 16 => byte == b':',
            19 => byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
}

fn xml_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
    output
}

fn data_digest<T: Serialize>(value: &T) -> Result<String, PresentationGenerationError> {
    serde_json::to_vec(value)
        .map(|bytes| word_sha256(&bytes))
        .map_err(|_| PresentationGenerationError::InvalidInput)
}

fn validate_block(block: &PresentationBlock) -> Result<(), PresentationGenerationError> {
    match block {
        PresentationBlock::Text { text } => {
            if !valid_text(text, false) {
                return Err(PresentationGenerationError::InvalidInput);
            }
        }
        PresentationBlock::Bullets { items } => {
            if items.is_empty()
                || items.len() > MAX_BULLET_ITEMS
                || items.iter().any(|item| !valid_text(item, false))
            {
                return Err(PresentationGenerationError::InvalidInput);
            }
        }
        PresentationBlock::Table { table } => {
            if table.headers.is_empty()
                || table.headers.len() > MAX_TABLE_COLUMNS
                || table.rows.len() > MAX_TABLE_ROWS
                || table.headers.iter().any(|item| !valid_text(item, false))
                || table.rows.iter().any(|row| {
                    row.len() != table.headers.len()
                        || row.iter().any(|item| !valid_text(item, true))
                })
                || table.data_source_sha256 != data_digest(&(&table.headers, &table.rows))?
            {
                return Err(PresentationGenerationError::InvalidInput);
            }
        }
        PresentationBlock::Chart { chart } => {
            if !valid_text(&chart.title, false)
                || chart.categories.is_empty()
                || chart.categories.len() != chart.values.len()
                || chart.categories.len() > MAX_CHART_POINTS
                || chart.categories.iter().any(|item| !valid_text(item, false))
                || chart.data_source_sha256 != data_digest(&(&chart.categories, &chart.values))?
            {
                return Err(PresentationGenerationError::InvalidInput);
            }
        }
        PresentationBlock::Diagram { diagram } => {
            let nodes = diagram
                .nodes
                .iter()
                .map(|node| node.node_id.as_str())
                .collect::<BTreeSet<_>>();
            if diagram.nodes.is_empty()
                || diagram.nodes.len() > MAX_DIAGRAM_NODES
                || diagram.edges.len() > MAX_DIAGRAM_EDGES
                || nodes.len() != diagram.nodes.len()
                || diagram
                    .nodes
                    .iter()
                    .any(|node| !valid_identifier(&node.node_id) || !valid_text(&node.label, false))
                || diagram.edges.iter().any(|edge| {
                    !nodes.contains(edge.from.as_str()) || !nodes.contains(edge.to.as_str())
                })
                || diagram.data_source_sha256 != data_digest(&(&diagram.nodes, &diagram.edges))?
            {
                return Err(PresentationGenerationError::InvalidInput);
            }
        }
    }
    Ok(())
}

fn validate_spec(spec: &PresentationDeckSpec) -> Result<(), PresentationGenerationError> {
    if !valid_identifier(&spec.deck_id)
        || !safe_output_path(&spec.output_path)
        || !valid_timestamp(&spec.fixed_timestamp)
        || spec.slides.is_empty()
        || spec.slides.len() > MAX_SLIDES
    {
        return Err(PresentationGenerationError::InvalidInput);
    }
    let mut ids = BTreeSet::new();
    for slide in &spec.slides {
        if !valid_identifier(&slide.slide_id)
            || !ids.insert(slide.slide_id.as_str())
            || !valid_text(&slide.title, false)
            || slide.blocks.len() > MAX_BLOCKS_PER_SLIDE
            || slide.speaker_notes.len() > MAX_SPEAKER_NOTES
            || slide
                .speaker_notes
                .iter()
                .any(|note| !valid_text(note, false))
        {
            return Err(PresentationGenerationError::InvalidInput);
        }
        for block in &slide.blocks {
            validate_block(block)?;
        }
    }
    Ok(())
}

fn text_shape(
    id: u32,
    name: &str,
    text: &[String],
    x: u64,
    y: u64,
    width: u64,
    height: u64,
) -> String {
    let paragraphs = text
        .iter()
        .map(|item| format!("<a:p><a:r><a:rPr lang=\"en-US\"/><a:t>{}</a:t></a:r><a:endParaRPr lang=\"en-US\"/></a:p>", xml_escape(item)))
        .collect::<String>();
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{width}\" cy=\"{height}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr wrap=\"square\"/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>",
        xml_escape(name)
    )
}

fn table_frame(
    id: u32,
    table: &PresentationTableSpec,
    x: u64,
    y: u64,
    width: u64,
    height: u64,
) -> String {
    let column_width = width / u64::try_from(table.headers.len()).unwrap_or(1).max(1);
    let grid = (0..table.headers.len())
        .map(|_| format!("<a:gridCol w=\"{column_width}\"/>"))
        .collect::<String>();
    let rows = std::iter::once(&table.headers).chain(table.rows.iter()).map(|row| {
        let cells = row.iter().map(|value| format!("<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>", xml_escape(value))).collect::<String>();
        format!("<a:tr h=\"370000\">{cells}</a:tr>")
    }).collect::<String>();
    format!(
        "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Comparison table\" descr=\"Deterministic comparison table\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{width}\" cy=\"{height}\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr firstRow=\"1\" bandRow=\"1\"/><a:tblGrid>{grid}</a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"
    )
}

fn chart_frame(id: u32, relationship_id: &str, x: u64, y: u64, width: u64, height: u64) -> String {
    format!(
        "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Chart\" descr=\"Deterministic local chart\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{width}\" cy=\"{height}\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"><c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" r:id=\"{relationship_id}\"/></a:graphicData></a:graphic></p:graphicFrame>"
    )
}

fn diagram_shapes(id: &mut u32, diagram: &PresentationDiagramSpec, y: u64) -> String {
    let count = u64::try_from(diagram.nodes.len()).unwrap_or(1).max(1);
    let width = 1_600_000_u64;
    let gap = 300_000_u64;
    diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let x = 500_000 + u64::try_from(index).unwrap_or(0) * (width + gap);
            let current = *id;
            *id = id.saturating_add(1);
            let label = format!("{}: {}", node.node_id, node.label);
            text_shape(
                current,
                "Diagram node",
                &[label],
                x.min(SLIDE_WIDTH.saturating_sub(width + 200_000)),
                y,
                width.min((SLIDE_WIDTH - 1_000_000) / count.max(1)),
                900_000,
            )
        })
        .collect()
}

fn chart_xml(chart: &PresentationChartSpec) -> String {
    let categories = chart
        .categories
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "<c:pt idx=\"{index}\"><c:v>{}</c:v></c:pt>",
                xml_escape(item)
            )
        })
        .collect::<String>();
    let values = chart
        .values
        .iter()
        .enumerate()
        .map(|(index, item)| format!("<c:pt idx=\"{index}\"><c:v>{item}</c:v></c:pt>"))
        .collect::<String>();
    let count = chart.categories.len();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:layout/><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:cat><c:strLit><c:ptCount val=\"{count}\"/>{categories}</c:strLit></c:cat><c:val><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val=\"{count}\"/>{values}</c:numLit></c:val></c:ser><c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"b\"/><c:crossAx val=\"2\"/></c:catAx><c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"l\"/><c:crossAx val=\"1\"/></c:valAx></c:plotArea><c:plotVisOnly val=\"1\"/></c:chart></c:chartSpace>",
        xml_escape(&chart.title)
    )
}

struct SlideBuild {
    xml: Vec<u8>,
    relationships: Vec<u8>,
    notes: Vec<u8>,
    charts: Vec<(String, Vec<u8>)>,
    preview: PresentationSlidePreview,
}

fn preview_for_slide(
    slide_number: u32,
    slide: &PresentationSlideSpec,
) -> Result<PresentationSlidePreview, PresentationGenerationError> {
    let mut objects = vec![PresentationObjectPreview {
        object_order: 1,
        kind: PresentationObjectKind::Shape,
        content_sha256: data_digest(&slide.title)?,
        x: 500_000,
        y: 300_000,
        width: 11_000_000,
        height: 800_000,
        data_source_sha256: None,
    }];
    for (index, block) in slide.blocks.iter().enumerate() {
        let (kind, source) = match block {
            PresentationBlock::Text { .. } | PresentationBlock::Bullets { .. } => {
                (PresentationObjectKind::Shape, None)
            }
            PresentationBlock::Table { table } => (
                PresentationObjectKind::Table,
                Some(table.data_source_sha256.clone()),
            ),
            PresentationBlock::Chart { chart } => (
                PresentationObjectKind::Chart,
                Some(chart.data_source_sha256.clone()),
            ),
            PresentationBlock::Diagram { diagram } => (
                PresentationObjectKind::Diagram,
                Some(diagram.data_source_sha256.clone()),
            ),
        };
        objects.push(PresentationObjectPreview {
            object_order: u32::try_from(index + 2).unwrap_or(u32::MAX),
            kind,
            content_sha256: data_digest(block)?,
            x: 500_000,
            y: 1_300_000 + u64::try_from(index).unwrap_or(0) * 1_050_000,
            width: 11_000_000,
            height: 950_000,
            data_source_sha256: source,
        });
    }
    let digest = data_digest(&(
        slide_number,
        &slide.slide_id,
        SLIDE_WIDTH,
        SLIDE_HEIGHT,
        &objects,
    ))?;
    Ok(PresentationSlidePreview {
        slide_number,
        slide_id: slide.slide_id.clone(),
        width: SLIDE_WIDTH,
        height: SLIDE_HEIGHT,
        objects,
        preview_sha256: digest,
        native_render_required: true,
    })
}

fn build_slide(
    slide_number: u32,
    slide: &PresentationSlideSpec,
    first_chart: usize,
) -> Result<SlideBuild, PresentationGenerationError> {
    let mut object_id = 2_u32;
    let mut shapes = text_shape(
        object_id,
        "Title",
        std::slice::from_ref(&slide.title),
        500_000,
        300_000,
        11_000_000,
        800_000,
    );
    object_id += 1;
    let mut relationships = vec!["<Relationship Id=\"rIdLayout\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>".to_owned(), format!("<Relationship Id=\"rIdNotes\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"../notesSlides/notesSlide{slide_number}.xml\"/>")];
    let mut charts = Vec::new();
    let mut chart_offset = 0_usize;
    for (index, block) in slide.blocks.iter().enumerate() {
        let y = 1_300_000 + u64::try_from(index).unwrap_or(0) * 1_050_000;
        match block {
            PresentationBlock::Text { text } => shapes.push_str(&text_shape(
                object_id,
                "Text",
                std::slice::from_ref(text),
                500_000,
                y,
                11_000_000,
                950_000,
            )),
            PresentationBlock::Bullets { items } => shapes.push_str(&text_shape(
                object_id, "Bullets", items, 500_000, y, 11_000_000, 950_000,
            )),
            PresentationBlock::Table { table } => shapes.push_str(&table_frame(
                object_id, table, 500_000, y, 11_000_000, 950_000,
            )),
            PresentationBlock::Chart { chart } => {
                let chart_number = first_chart + chart_offset;
                let relationship_id = format!("rIdChart{chart_number}");
                shapes.push_str(&chart_frame(
                    object_id,
                    &relationship_id,
                    500_000,
                    y,
                    11_000_000,
                    950_000,
                ));
                relationships.push(format!("<Relationship Id=\"{relationship_id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"../charts/chart{chart_number}.xml\"/>"));
                charts.push((
                    format!("ppt/charts/chart{chart_number}.xml"),
                    chart_xml(chart).into_bytes(),
                ));
                chart_offset += 1;
            }
            PresentationBlock::Diagram { diagram } => {
                shapes.push_str(&diagram_shapes(&mut object_id, diagram, y))
            }
        }
        object_id = object_id.saturating_add(1);
    }
    let note_text = if slide.speaker_notes.is_empty() {
        vec![String::new()]
    } else {
        slide.speaker_notes.clone()
    };
    let notes_body = note_text
        .iter()
        .map(|item| format!("<a:p><a:r><a:t>{}</a:t></a:r></a:p>", xml_escape(item)))
        .collect::<String>();
    Ok(SlideBuild {
        xml: format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>").into_bytes(),
        relationships: format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{}</Relationships>", relationships.join("")).into_bytes(),
        notes: format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><p:notes xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>{notes_body}</p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>").into_bytes(),
        charts,
        preview: preview_for_slide(slide_number, slide)?,
    })
}

fn package_parts(
    spec: &PresentationDeckSpec,
) -> Result<PackagedPresentation, PresentationGenerationError> {
    let mut overrides = vec!["<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>".to_owned(), "<Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>".to_owned(), "<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>".to_owned(), "<Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>".to_owned(), "<Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>".to_owned(), "<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>".to_owned()];
    let mut slide_ids = Vec::new();
    let mut presentation_relationships = vec!["<Relationship Id=\"rIdMaster\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>".to_owned()];
    let mut parts = BTreeMap::new();
    let mut previews = Vec::new();
    let mut chart_number = 1_usize;
    for (index, slide) in spec.slides.iter().enumerate() {
        let slide_number = index + 1;
        let built = build_slide(
            u32::try_from(slide_number).unwrap_or(u32::MAX),
            slide,
            chart_number,
        )?;
        chart_number += built.charts.len();
        slide_ids.push(format!(
            "<p:sldId id=\"{}\" r:id=\"rIdSlide{slide_number}\"/>",
            255 + slide_number
        ));
        presentation_relationships.push(format!("<Relationship Id=\"rIdSlide{slide_number}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{slide_number}.xml\"/>"));
        overrides.push(format!("<Override PartName=\"/ppt/slides/slide{slide_number}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>"));
        overrides.push(format!("<Override PartName=\"/ppt/notesSlides/notesSlide{slide_number}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>"));
        parts.insert(format!("ppt/slides/slide{slide_number}.xml"), built.xml);
        parts.insert(
            format!("ppt/slides/_rels/slide{slide_number}.xml.rels"),
            built.relationships,
        );
        parts.insert(
            format!("ppt/notesSlides/notesSlide{slide_number}.xml"),
            built.notes,
        );
        parts.insert(format!("ppt/notesSlides/_rels/notesSlide{slide_number}.xml.rels"), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdSlide\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"../slides/slide{slide_number}.xml\"/></Relationships>").into_bytes());
        for (name, content) in built.charts {
            overrides.push(format!("<Override PartName=\"/{}\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>", name));
            parts.insert(name, content);
        }
        previews.push(built.preview);
    }
    parts.insert("[Content_Types].xml".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{}</Types>", overrides.join("")).into_bytes());
    parts.insert("_rels/.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties\" Target=\"docProps/app.xml\"/></Relationships>".to_vec());
    parts.insert("docProps/core.xml".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>{}</dc:title><dc:creator>AgentMage</dc:creator><cp:lastModifiedBy>AgentMage</cp:lastModifiedBy><dcterms:created xsi:type=\"dcterms:W3CDTF\">{}</dcterms:created><dcterms:modified xsi:type=\"dcterms:W3CDTF\">{}</dcterms:modified></cp:coreProperties>", xml_escape(&spec.deck_id), spec.fixed_timestamp, spec.fixed_timestamp).into_bytes());
    parts.insert("docProps/app.xml".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\" xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\"><Application>AgentMage</Application><PresentationFormat>On-screen Show (16:9)</PresentationFormat><Slides>{}</Slides><Notes>{}</Notes><HiddenSlides>0</HiddenSlides></Properties>", spec.slides.len(), spec.slides.len()).into_bytes());
    parts.insert("ppt/presentation.xml".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rIdMaster\"/></p:sldMasterIdLst><p:sldIdLst>{}</p:sldIdLst><p:sldSz cx=\"{SLIDE_WIDTH}\" cy=\"{SLIDE_HEIGHT}\" type=\"screen16x9\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>", slide_ids.join("")).into_bytes());
    parts.insert("ppt/_rels/presentation.xml.rels".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{}</Relationships>", presentation_relationships.join("")).into_bytes());
    parts.insert("ppt/slideMasters/slideMaster1.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><p:sldMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" bg1=\"lt1\" bg2=\"lt2\" folHlink=\"folHlink\" hlink=\"hlink\" tx1=\"dk1\" tx2=\"dk2\"/><p:sldLayoutIdLst><p:sldLayoutId id=\"1\" r:id=\"rIdLayout\"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>".to_vec());
    parts.insert("ppt/slideMasters/_rels/slideMaster1.xml.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdLayout\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rIdTheme\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/></Relationships>".to_vec());
    parts.insert("ppt/slideLayouts/slideLayout1.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>".to_vec());
    parts.insert("ppt/slideLayouts/_rels/slideLayout1.xml.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdMaster\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/></Relationships>".to_vec());
    parts.insert("ppt/theme/theme1.xml".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"AgentMage\"><a:themeElements><a:clrScheme name=\"AgentMage\"><a:dk1><a:srgbClr val=\"1F2937\"/></a:dk1><a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"374151\"/></a:dk2><a:lt2><a:srgbClr val=\"F3F4F6\"/></a:lt2><a:accent1><a:srgbClr val=\"0F766E\"/></a:accent1><a:accent2><a:srgbClr val=\"B45309\"/></a:accent2><a:accent3><a:srgbClr val=\"2563EB\"/></a:accent3><a:accent4><a:srgbClr val=\"7C3AED\"/></a:accent4><a:accent5><a:srgbClr val=\"BE123C\"/></a:accent5><a:accent6><a:srgbClr val=\"4D7C0F\"/></a:accent6><a:hlink><a:srgbClr val=\"0563C1\"/></a:hlink><a:folHlink><a:srgbClr val=\"954F72\"/></a:folHlink></a:clrScheme><a:fontScheme name=\"AgentMage\"><a:majorFont><a:latin typeface=\"Arial\"/></a:majorFont><a:minorFont><a:latin typeface=\"Arial\"/></a:minorFont></a:fontScheme><a:fmtScheme name=\"AgentMage\"><a:fillStyleLst/><a:lnStyleLst/><a:effectStyleLst/><a:bgFillStyleLst/></a:fmtScheme></a:themeElements></a:theme>".to_vec());
    Ok((parts, previews))
}

/// Generates a deterministic local PowerPoint Open XML proposal and reopens it through the bounded inspector.
pub fn generate_presentation(
    spec: &PresentationDeckSpec,
) -> Result<GeneratedPresentation, PresentationGenerationError> {
    validate_spec(spec)?;
    let (parts, previews) = package_parts(spec)?;
    let pptx =
        zip_parts(&parts).map_err(|_| PresentationGenerationError::InvalidGeneratedPackage)?;
    let inspection = inspect_pptx(
        spec.output_path.clone(),
        &pptx,
        &PresentationProfile::default(),
    )
    .map_err(|_| PresentationGenerationError::InvalidGeneratedPackage)?;
    if !inspection.safe_for_reuse || inspection.slides.len() != spec.slides.len() {
        return Err(PresentationGenerationError::InvalidGeneratedPackage);
    }
    Ok(GeneratedPresentation {
        schema_version: CONTRACT_SCHEMA_VERSION,
        specification: spec.clone(),
        pptx_sha256: word_sha256(&pptx),
        pptx,
        inspection,
        previews,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Applies exact slide replacements through complete deterministic regeneration.
pub fn edit_generated_presentation(
    source: &GeneratedPresentation,
    request: &PresentationEditRequest,
) -> Result<EditedPresentation, PresentationGenerationError> {
    if request.source_sha256 != source.pptx_sha256
        || !valid_sha256(&request.source_sha256)
        || source.pptx_sha256 != word_sha256(&source.pptx)
        || source.previews.len() != source.specification.slides.len()
        || source.inspection.slides.len() != source.specification.slides.len()
        || request.replacements.is_empty()
        || !request
            .replacements
            .windows(2)
            .all(|items| items[0].slide_number < items[1].slide_number)
    {
        return Err(PresentationGenerationError::SourceMismatch);
    }
    let mut specification = source.specification.clone();
    for edit in &request.replacements {
        if edit.slide_number == 0 {
            return Err(PresentationGenerationError::InvalidInput);
        }
        let index = usize::try_from(edit.slide_number.saturating_sub(1))
            .map_err(|_| PresentationGenerationError::InvalidInput)?;
        let target = specification
            .slides
            .get_mut(index)
            .ok_or(PresentationGenerationError::InvalidInput)?;
        if target.slide_id != edit.replacement.slide_id {
            return Err(PresentationGenerationError::InvalidInput);
        }
        *target = edit.replacement.clone();
    }
    let generated = generate_presentation(&specification)?;
    let edited = request
        .replacements
        .iter()
        .map(|item| item.slide_number)
        .collect::<BTreeSet<_>>();
    let changes = request
        .replacements
        .iter()
        .map(|item| {
            let index = usize::try_from(item.slide_number - 1).unwrap_or(usize::MAX);
            PresentationSlideChange {
                slide_number: item.slide_number,
                before_preview_sha256: source.previews[index].preview_sha256.clone(),
                after_preview_sha256: generated.previews[index].preview_sha256.clone(),
            }
        })
        .collect::<Vec<_>>();
    for (index, (before, after)) in source.previews.iter().zip(&generated.previews).enumerate() {
        let slide_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if !edited.contains(&slide_number) && before != after {
            return Err(PresentationGenerationError::InvalidGeneratedPackage);
        }
    }
    Ok(EditedPresentation {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_sha256: source.pptx_sha256.clone(),
        generated,
        changes,
        unchanged_slide_count: u32::try_from(source.previews.len().saturating_sub(edited.len()))
            .unwrap_or(u32::MAX),
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-deck"),
            ["slides", "report.pptx"],
        )
        .expect("path")
    }

    fn table() -> PresentationTableSpec {
        let headers = vec!["Item".to_owned(), "Value".to_owned()];
        let rows = vec![vec!["A".to_owned(), "1".to_owned()]];
        PresentationTableSpec {
            data_source_sha256: data_digest(&(&headers, &rows)).expect("hash"),
            headers,
            rows,
        }
    }

    fn chart() -> PresentationChartSpec {
        let categories = vec!["A".to_owned(), "B".to_owned()];
        let values = vec![1, 2];
        PresentationChartSpec {
            title: "Counts".to_owned(),
            data_source_sha256: data_digest(&(&categories, &values)).expect("hash"),
            categories,
            values,
        }
    }

    fn diagram() -> PresentationDiagramSpec {
        let nodes = vec![
            PresentationDiagramNode {
                node_id: "a".to_owned(),
                label: "Start".to_owned(),
            },
            PresentationDiagramNode {
                node_id: "b".to_owned(),
                label: "Finish".to_owned(),
            },
        ];
        let edges = vec![PresentationDiagramEdge {
            from: "a".to_owned(),
            to: "b".to_owned(),
        }];
        PresentationDiagramSpec {
            data_source_sha256: data_digest(&(&nodes, &edges)).expect("hash"),
            nodes,
            edges,
        }
    }

    fn spec() -> PresentationDeckSpec {
        PresentationDeckSpec {
            deck_id: "deck-1".to_owned(),
            output_path: path(),
            fixed_timestamp: "2026-08-15T00:00:00Z".to_owned(),
            slides: vec![
                PresentationSlideSpec {
                    slide_id: "overview".to_owned(),
                    title: "Overview & status".to_owned(),
                    blocks: vec![
                        PresentationBlock::Text {
                            text: "=HYPERLINK(\"https://example.invalid\") <inert>".to_owned(),
                        },
                        PresentationBlock::Table { table: table() },
                    ],
                    speaker_notes: vec!["Private speaker context".to_owned()],
                },
                PresentationSlideSpec {
                    slide_id: "visuals".to_owned(),
                    title: "Visuals".to_owned(),
                    blocks: vec![
                        PresentationBlock::Chart { chart: chart() },
                        PresentationBlock::Diagram { diagram: diagram() },
                    ],
                    speaker_notes: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn generates_deterministic_deck_with_text_notes_table_chart_diagram_and_previews() {
        let first = generate_presentation(&spec()).expect("generate");
        let second = generate_presentation(&spec()).expect("generate");
        assert_eq!(first.pptx, second.pptx);
        assert_eq!(first.inspection.slides.len(), 2);
        assert_eq!(first.previews.len(), 2);
        assert!(
            first
                .previews
                .iter()
                .all(|item| item.native_render_required)
        );
        assert_eq!(
            first.inspection.slides[0].speaker_notes,
            ["Private speaker context"]
        );
        let inert = first.inspection.slides[0]
            .objects
            .iter()
            .flat_map(|item| &item.text)
            .any(|text| text.contains("=HYPERLINK"));
        assert!(inert);
        assert!(!first.execution_performed);
        assert!(!first.network_access_performed);
        assert!(!first.filesystem_effect_performed);
    }

    #[test]
    fn rejects_data_hash_shape_edge_path_timestamp_and_resource_drift() {
        let mut changed = spec();
        if let PresentationBlock::Chart { chart } = &mut changed.slides[1].blocks[0] {
            chart.values[0] = 99;
        }
        assert!(generate_presentation(&changed).is_err());
        let mut changed = spec();
        changed.fixed_timestamp = "now".to_owned();
        assert!(generate_presentation(&changed).is_err());
        let mut changed = spec();
        changed.output_path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-deck"),
            ["slides", "report.pdf"],
        )
        .expect("path");
        assert!(generate_presentation(&changed).is_err());
    }

    #[test]
    fn full_regeneration_edit_changes_only_selected_slide_preview() {
        let source = generate_presentation(&spec()).expect("source");
        let mut replacement = source.specification.slides[0].clone();
        replacement.title = "Updated overview".to_owned();
        let edited = edit_generated_presentation(
            &source,
            &PresentationEditRequest {
                source_sha256: source.pptx_sha256.clone(),
                replacements: vec![PresentationSlideReplacement {
                    slide_number: 1,
                    replacement,
                }],
            },
        )
        .expect("edit");
        assert_ne!(
            edited.changes[0].before_preview_sha256,
            edited.changes[0].after_preview_sha256
        );
        assert_eq!(source.previews[1], edited.generated.previews[1]);
        assert_eq!(edited.unchanged_slide_count, 1);
        assert!(edited.original_preserved);
        assert!(!edited.filesystem_effect_performed);
    }

    #[test]
    fn edit_rejects_stale_identity_duplicate_order_and_slide_identity_drift() {
        let source = generate_presentation(&spec()).expect("source");
        let mut replacement = source.specification.slides[0].clone();
        replacement.slide_id = "other".to_owned();
        assert!(
            edit_generated_presentation(
                &source,
                &PresentationEditRequest {
                    source_sha256: source.pptx_sha256.clone(),
                    replacements: vec![PresentationSlideReplacement {
                        slide_number: 1,
                        replacement
                    }]
                }
            )
            .is_err()
        );
        assert_eq!(
            edit_generated_presentation(
                &source,
                &PresentationEditRequest {
                    source_sha256: "f".repeat(64),
                    replacements: Vec::new()
                }
            ),
            Err(PresentationGenerationError::SourceMismatch)
        );
    }
}
