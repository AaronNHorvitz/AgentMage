//! Deterministic rich WordprocessingML document construction without write authority.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

use crate::word_generation::{valid_identifier, xml_escape, zip_parts};
use crate::word_ooxml::word_sha256;
use crate::{
    GeneratedWordPart, WordConversionProfile, WordInspectionReport, WordOoxmlError, inspect_docx,
};
use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};

const RICH_GENERATOR_ID: &str =
    "agentmage-rich-word-v1;zip=8.6.0;quick-xml=0.41.0;external-links=denied;effects=denied";
const MAX_BLOCKS: usize = 16_384;
const MAX_TABLE_CELLS: usize = 65_536;
const MAX_TEXT_BYTES: usize = 1_048_576;

/// Rich Word construction failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordRichGenerationError {
    /// A caller-supplied identity, text value, layout, color, or index was invalid.
    InvalidInput,
    /// A requested block or section identity was absent or duplicated.
    BlockConflict,
    /// The resulting package failed bounded OOXML generation or inspection.
    Package,
}

impl fmt::Display for WordRichGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "invalid rich Word input",
            Self::BlockConflict => "rich Word block conflict",
            Self::Package => "rich Word package failure",
        })
    }
}

impl Error for WordRichGenerationError {}

impl From<WordOoxmlError> for WordRichGenerationError {
    fn from(_: WordOoxmlError) -> Self {
        Self::Package
    }
}

/// Closed style family supported by the rich generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordStyleKind {
    /// Paragraph style.
    Paragraph,
    /// Character style.
    Character,
    /// Table style.
    Table,
}

/// One bounded custom style definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordStyleConfiguration {
    /// Stable OOXML style identity.
    pub style_id: String,
    /// Human-readable style name.
    pub name: String,
    /// Closed style family.
    pub kind: WordStyleKind,
    /// Optional installed-font family request.
    pub font_family: Option<String>,
    /// Optional half-point font size.
    pub half_points: Option<u16>,
    /// Whether the style requests bold text.
    pub bold: bool,
    /// Optional six-digit RGB fill.
    pub shading_rgb: Option<String>,
}

/// Page dimensions and margins in twentieths of a point.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPageConfiguration {
    /// Page width in twips.
    pub width_twips: u32,
    /// Page height in twips.
    pub height_twips: u32,
    /// Top margin in twips.
    pub top_margin_twips: u32,
    /// Right margin in twips.
    pub right_margin_twips: u32,
    /// Bottom margin in twips.
    pub bottom_margin_twips: u32,
    /// Left margin in twips.
    pub left_margin_twips: u32,
    /// Number of page columns.
    pub columns: u8,
    /// Whether landscape orientation is requested.
    pub landscape: bool,
}

impl Default for WordPageConfiguration {
    fn default() -> Self {
        Self {
            width_twips: 12_240,
            height_twips: 15_840,
            top_margin_twips: 1_440,
            right_margin_twips: 1_440,
            bottom_margin_twips: 1_440,
            left_margin_twips: 1_440,
            columns: 1,
            landscape: false,
        }
    }
}

/// Optional first-section header and footer text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordHeaderFooterConfiguration {
    /// Optional header text.
    pub header_text: Option<String>,
    /// Optional footer text.
    pub footer_text: Option<String>,
}

/// Closed table border style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordBorderStyle {
    /// No visible border.
    None,
    /// Single-line border.
    Single,
    /// Double-line border.
    Double,
    /// Dotted border.
    Dotted,
    /// Dashed border.
    Dashed,
}

impl WordBorderStyle {
    fn ooxml(self) -> &'static str {
        match self {
            Self::None => "nil",
            Self::Single => "single",
            Self::Double => "double",
            Self::Dotted => "dotted",
            Self::Dashed => "dashed",
        }
    }
}

/// Deterministic table layout shared by all cells in one table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordTableLayout {
    /// Table width in twips.
    pub width_twips: u32,
    /// Top cell margin in twips.
    pub top_margin_twips: u16,
    /// Right cell margin in twips.
    pub right_margin_twips: u16,
    /// Bottom cell margin in twips.
    pub bottom_margin_twips: u16,
    /// Left cell margin in twips.
    pub left_margin_twips: u16,
    /// Outer and inner table border style.
    pub border_style: WordBorderStyle,
    /// Optional default six-digit RGB cell fill.
    pub default_shading_rgb: Option<String>,
    /// Per-cell six-digit RGB fills keyed by `row:column`.
    pub cell_shading_rgb: BTreeMap<String, String>,
    /// Per-cell border overrides keyed by `row:column`.
    pub cell_border_style: BTreeMap<String, WordBorderStyle>,
}

impl Default for WordTableLayout {
    fn default() -> Self {
        Self {
            width_twips: 9_360,
            top_margin_twips: 72,
            right_margin_twips: 108,
            bottom_margin_twips: 72,
            left_margin_twips: 108,
            border_style: WordBorderStyle::Single,
            default_shading_rgb: None,
            cell_shading_rgb: BTreeMap::new(),
            cell_border_style: BTreeMap::new(),
        }
    }
}

impl WordTableLayout {
    /// Sets the default cell fill.
    pub fn set_cell_shading(&mut self, rgb: Option<&str>) -> Result<(), WordRichGenerationError> {
        self.default_shading_rgb = validate_optional_color(rgb)?;
        Ok(())
    }

    /// Sets all cell margins in twips.
    pub fn set_cell_margins(
        &mut self,
        top: u16,
        right: u16,
        bottom: u16,
        left: u16,
    ) -> Result<(), WordRichGenerationError> {
        if [top, right, bottom, left]
            .into_iter()
            .any(|value| value > 2_880)
        {
            return Err(WordRichGenerationError::InvalidInput);
        }
        self.top_margin_twips = top;
        self.right_margin_twips = right;
        self.bottom_margin_twips = bottom;
        self.left_margin_twips = left;
        Ok(())
    }

    /// Sets the exact table width in twips.
    pub fn set_table_width(&mut self, width_twips: u32) -> Result<(), WordRichGenerationError> {
        if !(720..=31_680).contains(&width_twips) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        self.width_twips = width_twips;
        Ok(())
    }

    /// Sets the common table border style.
    pub fn set_table_borders(&mut self, style: WordBorderStyle) {
        self.border_style = style;
    }

    /// Overrides one cell border style.
    pub fn set_single_cell_border(
        &mut self,
        row: u16,
        column: u16,
        style: WordBorderStyle,
    ) -> Result<(), WordRichGenerationError> {
        let key = cell_key(row, column)?;
        self.cell_border_style.insert(key, style);
        Ok(())
    }

    /// Overrides one cell fill.
    pub fn set_single_cell_shading(
        &mut self,
        row: u16,
        column: u16,
        rgb: &str,
    ) -> Result<(), WordRichGenerationError> {
        let key = cell_key(row, column)?;
        self.cell_shading_rgb.insert(key, validate_color(rgb)?);
        Ok(())
    }
}

/// Named numbering definition for list paragraphs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordNumberingDefinition {
    /// Stable definition identity.
    pub numbering_id: String,
    /// True for decimal numbering and false for bullets.
    pub ordered: bool,
    /// Visible level text such as `%1.` or `bullet`.
    pub level_text: String,
}

/// User-visible metadata represented both structurally and as an optional table.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordDocumentMetadata {
    /// Optional document title.
    pub title: Option<String>,
    /// Optional subject.
    pub subject: Option<String>,
    /// Optional creator display value.
    pub creator: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Sorted custom key/value metadata.
    pub custom: BTreeMap<String, String>,
}

/// Decision-card content rendered as a structured two-column table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordDecisionCard {
    /// Stable decision identity.
    pub decision_id: String,
    /// Decision title.
    pub title: String,
    /// Decision state.
    pub status: String,
    /// Decision rationale.
    pub rationale: String,
    /// Optional owner display value.
    pub owner: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum RichBlock {
    Paragraph {
        block_id: String,
        text: String,
        style_id: Option<String>,
        numbering_id: Option<String>,
    },
    Table {
        block_id: String,
        rows: Vec<Vec<String>>,
        layout: WordTableLayout,
    },
    InternalHyperlink {
        block_id: String,
        label: String,
        anchor: String,
    },
}

impl RichBlock {
    fn id(&self) -> &str {
        match self {
            Self::Paragraph { block_id, .. }
            | Self::Table { block_id, .. }
            | Self::InternalHyperlink { block_id, .. } => block_id,
        }
    }
}

/// Mutable, bounded document specification that has no I/O authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordRichDocumentBuilder {
    artifact_id: String,
    page: WordPageConfiguration,
    header_footer: WordHeaderFooterConfiguration,
    metadata: WordDocumentMetadata,
    styles: BTreeMap<String, WordStyleConfiguration>,
    numbering: BTreeMap<String, WordNumberingDefinition>,
    blocks: Vec<RichBlock>,
}

impl WordRichDocumentBuilder {
    /// Creates an empty rich-document specification.
    pub fn new(artifact_id: &str) -> Result<Self, WordRichGenerationError> {
        if !valid_identifier(artifact_id) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        Ok(Self {
            artifact_id: artifact_id.to_owned(),
            page: WordPageConfiguration::default(),
            header_footer: WordHeaderFooterConfiguration::default(),
            metadata: WordDocumentMetadata::default(),
            styles: built_in_styles(),
            numbering: BTreeMap::new(),
            blocks: Vec::new(),
        })
    }

    /// Replaces the complete custom style inventory after validation.
    pub fn configure_styles(
        &mut self,
        styles: impl IntoIterator<Item = WordStyleConfiguration>,
    ) -> Result<(), WordRichGenerationError> {
        let mut configured = built_in_styles();
        for style in styles {
            validate_style(&style)?;
            if configured.insert(style.style_id.clone(), style).is_some() {
                return Err(WordRichGenerationError::BlockConflict);
            }
        }
        self.styles = configured;
        Ok(())
    }

    /// Sets page dimensions, margins, columns, and orientation.
    pub fn configure_page(
        &mut self,
        page: WordPageConfiguration,
    ) -> Result<(), WordRichGenerationError> {
        validate_page(&page)?;
        self.page = page;
        Ok(())
    }

    /// Sets optional first-section header and footer content.
    pub fn configure_header_footer(
        &mut self,
        configuration: WordHeaderFooterConfiguration,
    ) -> Result<(), WordRichGenerationError> {
        validate_optional_text(&configuration.header_text)?;
        validate_optional_text(&configuration.footer_text)?;
        self.header_footer = configuration;
        Ok(())
    }

    /// Sets document metadata without adding a visible table.
    pub fn configure_metadata(
        &mut self,
        metadata: WordDocumentMetadata,
    ) -> Result<(), WordRichGenerationError> {
        validate_metadata(&metadata)?;
        self.metadata = metadata;
        Ok(())
    }

    /// Adds a visible metadata table from the exact current metadata.
    pub fn add_metadata_table(
        &mut self,
        block_id: &str,
        layout: WordTableLayout,
    ) -> Result<(), WordRichGenerationError> {
        let mut rows = vec![vec!["Field".to_owned(), "Value".to_owned()]];
        for (label, value) in [
            ("Title", self.metadata.title.as_ref()),
            ("Subject", self.metadata.subject.as_ref()),
            ("Creator", self.metadata.creator.as_ref()),
            ("Description", self.metadata.description.as_ref()),
        ] {
            if let Some(value) = value {
                rows.push(vec![label.to_owned(), value.clone()]);
            }
        }
        rows.extend(
            self.metadata
                .custom
                .iter()
                .map(|(key, value)| vec![key.clone(), value.clone()]),
        );
        self.add_table_from_markdown(block_id, rows, layout)
    }

    /// Adds a visibly styled warning paragraph.
    pub fn add_warning(
        &mut self,
        block_id: &str,
        text: &str,
    ) -> Result<(), WordRichGenerationError> {
        self.add_paragraph(block_id, text, Some("AgentMageWarning"), None)
    }

    /// Adds a bounded table from already parsed Markdown cells.
    pub fn add_table_from_markdown(
        &mut self,
        block_id: &str,
        rows: Vec<Vec<String>>,
        layout: WordTableLayout,
    ) -> Result<(), WordRichGenerationError> {
        validate_block_id(self, block_id)?;
        validate_table(&rows, &layout)?;
        self.blocks.push(RichBlock::Table {
            block_id: block_id.to_owned(),
            rows,
            layout,
        });
        self.validate_size()
    }

    /// Adds a decision card as a structured two-column table.
    pub fn add_decision_card(
        &mut self,
        block_id: &str,
        card: WordDecisionCard,
        layout: WordTableLayout,
    ) -> Result<(), WordRichGenerationError> {
        if !valid_identifier(&card.decision_id) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        for value in [&card.title, &card.status, &card.rationale] {
            validate_text(value)?;
        }
        validate_optional_text(&card.owner)?;
        let mut rows = vec![
            vec!["Decision".to_owned(), card.title],
            vec!["Identity".to_owned(), card.decision_id],
            vec!["Status".to_owned(), card.status],
            vec!["Rationale".to_owned(), card.rationale],
        ];
        if let Some(owner) = card.owner {
            rows.push(vec!["Owner".to_owned(), owner]);
        }
        self.add_table_from_markdown(block_id, rows, layout)
    }

    /// Adds an internal-only hyperlink to a validated bookmark anchor.
    pub fn add_hyperlink(
        &mut self,
        block_id: &str,
        label: &str,
        anchor: &str,
    ) -> Result<(), WordRichGenerationError> {
        validate_block_id(self, block_id)?;
        validate_text(label)?;
        if !valid_identifier(anchor) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        self.blocks.push(RichBlock::InternalHyperlink {
            block_id: block_id.to_owned(),
            label: label.to_owned(),
            anchor: anchor.to_owned(),
        });
        self.validate_size()
    }

    /// Adds a paragraph with optional style and numbering identities.
    pub fn add_paragraph(
        &mut self,
        block_id: &str,
        text: &str,
        style_id: Option<&str>,
        numbering_id: Option<&str>,
    ) -> Result<(), WordRichGenerationError> {
        validate_block_id(self, block_id)?;
        validate_text(text)?;
        if style_id.is_some_and(|value| !self.styles.contains_key(value))
            || numbering_id.is_some_and(|value| !self.numbering.contains_key(value))
        {
            return Err(WordRichGenerationError::InvalidInput);
        }
        self.blocks.push(RichBlock::Paragraph {
            block_id: block_id.to_owned(),
            text: text.to_owned(),
            style_id: style_id.map(str::to_owned),
            numbering_id: numbering_id.map(str::to_owned),
        });
        self.validate_size()
    }

    /// Removes one exact block identity.
    pub fn remove_paragraph(&mut self, block_id: &str) -> Result<(), WordRichGenerationError> {
        let index = self.block_index(block_id)?;
        if !matches!(self.blocks[index], RichBlock::Paragraph { .. }) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        self.blocks.remove(index);
        Ok(())
    }

    /// Clones one exact paragraph under a new identity.
    pub fn clone_paragraph(
        &mut self,
        source_block_id: &str,
        new_block_id: &str,
    ) -> Result<(), WordRichGenerationError> {
        validate_block_id(self, new_block_id)?;
        let index = self.block_index(source_block_id)?;
        let RichBlock::Paragraph {
            text,
            style_id,
            numbering_id,
            ..
        } = &self.blocks[index]
        else {
            return Err(WordRichGenerationError::InvalidInput);
        };
        self.blocks.insert(
            index + 1,
            RichBlock::Paragraph {
                block_id: new_block_id.to_owned(),
                text: text.clone(),
                style_id: style_id.clone(),
                numbering_id: numbering_id.clone(),
            },
        );
        self.validate_size()
    }

    /// Replaces an exact inclusive block section with validated new paragraphs.
    pub fn replace_section(
        &mut self,
        first_block_id: &str,
        last_block_id: &str,
        replacements: Vec<(String, String)>,
    ) -> Result<(), WordRichGenerationError> {
        let first = self.block_index(first_block_id)?;
        let last = self.block_index(last_block_id)?;
        if first > last || replacements.is_empty() {
            return Err(WordRichGenerationError::InvalidInput);
        }
        let removed = self.blocks[first..=last]
            .iter()
            .map(RichBlock::id)
            .collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        let mut blocks = Vec::new();
        for (block_id, text) in replacements {
            if !valid_identifier(&block_id)
                || !seen.insert(block_id.clone())
                || (self.blocks.iter().any(|item| item.id() == block_id)
                    && !removed.contains(block_id.as_str()))
            {
                return Err(WordRichGenerationError::BlockConflict);
            }
            validate_text(&text)?;
            blocks.push(RichBlock::Paragraph {
                block_id,
                text,
                style_id: None,
                numbering_id: None,
            });
        }
        self.blocks.splice(first..=last, blocks);
        self.validate_size()
    }

    /// Adds one numbering definition.
    pub fn add_numbering_definition(
        &mut self,
        definition: WordNumberingDefinition,
    ) -> Result<(), WordRichGenerationError> {
        if !valid_identifier(&definition.numbering_id) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        validate_text(&definition.level_text)?;
        if self
            .numbering
            .insert(definition.numbering_id.clone(), definition)
            .is_some()
        {
            return Err(WordRichGenerationError::BlockConflict);
        }
        Ok(())
    }

    /// Applies one existing numbering identity to an existing paragraph.
    pub fn apply_numbering(
        &mut self,
        block_id: &str,
        numbering_id: &str,
    ) -> Result<(), WordRichGenerationError> {
        if !self.numbering.contains_key(numbering_id) {
            return Err(WordRichGenerationError::InvalidInput);
        }
        let index = self.block_index(block_id)?;
        let RichBlock::Paragraph {
            numbering_id: current,
            ..
        } = &mut self.blocks[index]
        else {
            return Err(WordRichGenerationError::InvalidInput);
        };
        *current = Some(numbering_id.to_owned());
        Ok(())
    }

    /// Builds deterministic proposal bytes and reopens them through the bounded inspector.
    pub fn build(
        &self,
        output_path: WorkspacePath,
        profile: &WordConversionProfile,
    ) -> Result<RichWordPackageProposal, WordRichGenerationError> {
        self.validate_size()?;
        if self.blocks.is_empty() {
            return Err(WordRichGenerationError::InvalidInput);
        }
        let specification =
            serde_json::to_vec(self).map_err(|_| WordRichGenerationError::Package)?;
        let specification_sha256 = word_sha256(&specification);
        let parts = package_parts(self)?;
        let part_ledger = parts
            .iter()
            .map(|(part_name, content)| GeneratedWordPart {
                part_name: part_name.clone(),
                content_sha256: word_sha256(content),
                bytes: content.len() as u64,
            })
            .collect();
        let package = zip_parts(&parts)?;
        let inspection = inspect_docx(&output_path, &package, profile)?;
        if inspection.quarantined || !inspection.inspection_complete {
            return Err(WordRichGenerationError::Package);
        }
        Ok(RichWordPackageProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: self.artifact_id.clone(),
            specification_sha256,
            output_path,
            generator_identity_sha256: word_sha256(RICH_GENERATOR_ID.as_bytes()),
            package_sha256: word_sha256(&package),
            package,
            parts: part_ledger,
            helper_capabilities: helper_capabilities(),
            inspection,
            proposal_only: true,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        })
    }

    fn block_index(&self, block_id: &str) -> Result<usize, WordRichGenerationError> {
        self.blocks
            .iter()
            .position(|item| item.id() == block_id)
            .ok_or(WordRichGenerationError::BlockConflict)
    }

    fn validate_size(&self) -> Result<(), WordRichGenerationError> {
        let text_bytes = self
            .blocks
            .iter()
            .map(|block| match block {
                RichBlock::Paragraph { text, .. } => text.len(),
                RichBlock::Table { rows, .. } => rows.iter().flatten().map(String::len).sum(),
                RichBlock::InternalHyperlink { label, anchor, .. } => label.len() + anchor.len(),
            })
            .sum::<usize>();
        if self.blocks.len() > MAX_BLOCKS || text_bytes > MAX_TEXT_BYTES {
            Err(WordRichGenerationError::InvalidInput)
        } else {
            Ok(())
        }
    }
}

/// Deterministic rich Word package proposal and its reopened inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RichWordPackageProposal {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Exact digest of the serialized rich-document specification.
    pub specification_sha256: String,
    /// Validated proposed output path.
    pub output_path: WorkspacePath,
    /// Exact generator identity digest.
    pub generator_identity_sha256: String,
    /// Deterministic package bytes.
    pub package: Vec<u8>,
    /// Exact package digest.
    pub package_sha256: String,
    /// Canonically ordered generated-part ledger.
    pub parts: Vec<GeneratedWordPart>,
    /// Closed helper inventory implemented by the generator.
    pub helper_capabilities: Vec<String>,
    /// Reopened structural inspection.
    pub inspection: WordInspectionReport,
    /// Always true; no file has been saved.
    pub proposal_only: bool,
    /// Always false; the builder has no filesystem authority.
    pub filesystem_effect_performed: bool,
    /// Always false; internal hyperlinks create no network authority.
    pub network_access_performed: bool,
    /// Always false; no document content is executed.
    pub execution_performed: bool,
}

fn helper_capabilities() -> Vec<String> {
    [
        "add_decision_cards",
        "add_hyperlink_internal",
        "add_metadata_table",
        "add_table_from_markdown",
        "add_warning",
        "apply_numbering",
        "build",
        "clone_paragraph",
        "configure_header_footer",
        "configure_page",
        "configure_styles",
        "create_paragraph",
        "remove_paragraph",
        "replace_section",
        "set_cell_margins",
        "set_cell_shading",
        "set_single_cell_border",
        "set_table_borders",
        "set_table_width",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn cell_key(row: u16, column: u16) -> Result<String, WordRichGenerationError> {
    if row > 8_191 || column > 255 {
        return Err(WordRichGenerationError::InvalidInput);
    }
    Ok(format!("{row}:{column}"))
}

fn validate_color(value: &str) -> Result<String, WordRichGenerationError> {
    if value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_ascii_uppercase())
    } else {
        Err(WordRichGenerationError::InvalidInput)
    }
}

fn validate_optional_color(value: Option<&str>) -> Result<Option<String>, WordRichGenerationError> {
    value.map(validate_color).transpose()
}

fn validate_text(value: &str) -> Result<(), WordRichGenerationError> {
    if value.is_empty()
        || value.len() > 262_144
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\t' | '\n' | '\r'))
    {
        Err(WordRichGenerationError::InvalidInput)
    } else {
        Ok(())
    }
}

fn validate_optional_text(value: &Option<String>) -> Result<(), WordRichGenerationError> {
    value.as_ref().map_or(Ok(()), |text| validate_text(text))
}

fn validate_block_id(
    builder: &WordRichDocumentBuilder,
    block_id: &str,
) -> Result<(), WordRichGenerationError> {
    if !valid_identifier(block_id) || builder.blocks.iter().any(|item| item.id() == block_id) {
        Err(WordRichGenerationError::BlockConflict)
    } else {
        Ok(())
    }
}

fn validate_style(style: &WordStyleConfiguration) -> Result<(), WordRichGenerationError> {
    if !valid_identifier(&style.style_id) {
        return Err(WordRichGenerationError::InvalidInput);
    }
    validate_text(&style.name)?;
    validate_optional_text(&style.font_family)?;
    if style
        .half_points
        .is_some_and(|value| !(12..=192).contains(&value))
    {
        return Err(WordRichGenerationError::InvalidInput);
    }
    validate_optional_color(style.shading_rgb.as_deref())?;
    Ok(())
}

fn validate_page(page: &WordPageConfiguration) -> Result<(), WordRichGenerationError> {
    if !(2_880..=31_680).contains(&page.width_twips)
        || !(2_880..=31_680).contains(&page.height_twips)
        || [
            page.top_margin_twips,
            page.right_margin_twips,
            page.bottom_margin_twips,
            page.left_margin_twips,
        ]
        .into_iter()
        .any(|value| value > 7_200)
        || !(1..=8).contains(&page.columns)
    {
        return Err(WordRichGenerationError::InvalidInput);
    }
    Ok(())
}

fn validate_metadata(metadata: &WordDocumentMetadata) -> Result<(), WordRichGenerationError> {
    validate_optional_text(&metadata.title)?;
    validate_optional_text(&metadata.subject)?;
    validate_optional_text(&metadata.creator)?;
    validate_optional_text(&metadata.description)?;
    for (key, value) in &metadata.custom {
        validate_text(key)?;
        validate_text(value)?;
    }
    Ok(())
}

fn validate_table(
    rows: &[Vec<String>],
    layout: &WordTableLayout,
) -> Result<(), WordRichGenerationError> {
    if rows.is_empty()
        || rows.iter().any(Vec::is_empty)
        || rows.iter().map(Vec::len).sum::<usize>() > MAX_TABLE_CELLS
    {
        return Err(WordRichGenerationError::InvalidInput);
    }
    for cell in rows.iter().flatten() {
        validate_text(cell)?;
    }
    let mut checked = layout.clone();
    checked.set_table_width(layout.width_twips)?;
    checked.set_cell_margins(
        layout.top_margin_twips,
        layout.right_margin_twips,
        layout.bottom_margin_twips,
        layout.left_margin_twips,
    )?;
    validate_optional_color(layout.default_shading_rgb.as_deref())?;
    for (key, value) in &layout.cell_shading_rgb {
        validate_cell_key(key)?;
        validate_color(value)?;
    }
    for key in layout.cell_border_style.keys() {
        validate_cell_key(key)?;
    }
    Ok(())
}

fn validate_cell_key(key: &str) -> Result<(), WordRichGenerationError> {
    let Some((row, column)) = key.split_once(':') else {
        return Err(WordRichGenerationError::InvalidInput);
    };
    let expected = cell_key(
        row.parse()
            .map_err(|_| WordRichGenerationError::InvalidInput)?,
        column
            .parse()
            .map_err(|_| WordRichGenerationError::InvalidInput)?,
    )?;
    if expected == key {
        Ok(())
    } else {
        Err(WordRichGenerationError::InvalidInput)
    }
}

fn built_in_styles() -> BTreeMap<String, WordStyleConfiguration> {
    [
        (
            "Normal",
            "Normal",
            WordStyleKind::Paragraph,
            None,
            Some(22),
            false,
            None,
        ),
        (
            "Heading1",
            "Heading 1",
            WordStyleKind::Paragraph,
            None,
            Some(32),
            true,
            None,
        ),
        (
            "Heading2",
            "Heading 2",
            WordStyleKind::Paragraph,
            None,
            Some(28),
            true,
            None,
        ),
        (
            "AgentMageWarning",
            "Warning",
            WordStyleKind::Paragraph,
            None,
            Some(22),
            true,
            Some("FFF2CC"),
        ),
        (
            "TableGrid",
            "Table Grid",
            WordStyleKind::Table,
            None,
            None,
            false,
            None,
        ),
    ]
    .into_iter()
    .map(
        |(style_id, name, kind, font_family, half_points, bold, shading_rgb)| {
            (
                style_id.to_owned(),
                WordStyleConfiguration {
                    style_id: style_id.to_owned(),
                    name: name.to_owned(),
                    kind,
                    font_family: font_family.map(str::to_owned),
                    half_points,
                    bold,
                    shading_rgb: shading_rgb.map(str::to_owned),
                },
            )
        },
    )
    .collect()
}

fn paragraph_xml(text: &str, style: Option<&str>, numbering: Option<u32>) -> String {
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

fn bookmarked_paragraph_xml(
    bookmark_id: usize,
    bookmark_name: &str,
    text: &str,
    style: Option<&str>,
    numbering: Option<u32>,
) -> String {
    let paragraph = paragraph_xml(text, style, numbering);
    paragraph.replacen(
        "<w:p>",
        &format!(
            "<w:p><w:bookmarkStart w:id=\"{bookmark_id}\" w:name=\"{}\"/><w:bookmarkEnd w:id=\"{bookmark_id}\"/>",
            xml_escape(bookmark_name)
        ),
        1,
    )
}

fn cell_properties(layout: &WordTableLayout, row: usize, column: usize) -> String {
    let key = format!("{row}:{column}");
    let shading = layout
        .cell_shading_rgb
        .get(&key)
        .or(layout.default_shading_rgb.as_ref())
        .map(|rgb| format!("<w:shd w:fill=\"{}\"/>", xml_escape(rgb)))
        .unwrap_or_default();
    let border = layout.cell_border_style.get(&key).map(|style| {
        format!("<w:tcBorders><w:top w:val=\"{}\"/><w:left w:val=\"{}\"/><w:bottom w:val=\"{}\"/><w:right w:val=\"{}\"/></w:tcBorders>", style.ooxml(), style.ooxml(), style.ooxml(), style.ooxml())
    }).unwrap_or_default();
    format!("<w:tcPr>{shading}{border}</w:tcPr>")
}

fn table_xml(rows: &[Vec<String>], layout: &WordTableLayout) -> String {
    let border = layout.border_style.ooxml();
    let mut output = format!(
        "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"{}\" w:type=\"dxa\"/><w:tblBorders><w:top w:val=\"{border}\"/><w:left w:val=\"{border}\"/><w:bottom w:val=\"{border}\"/><w:right w:val=\"{border}\"/><w:insideH w:val=\"{border}\"/><w:insideV w:val=\"{border}\"/></w:tblBorders><w:tblCellMar><w:top w:w=\"{}\" w:type=\"dxa\"/><w:right w:w=\"{}\" w:type=\"dxa\"/><w:bottom w:w=\"{}\" w:type=\"dxa\"/><w:left w:w=\"{}\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr>",
        layout.width_twips,
        layout.top_margin_twips,
        layout.right_margin_twips,
        layout.bottom_margin_twips,
        layout.left_margin_twips,
    );
    for (row_index, row) in rows.iter().enumerate() {
        output.push_str("<w:tr>");
        for (column_index, cell) in row.iter().enumerate() {
            output.push_str("<w:tc>");
            output.push_str(&cell_properties(layout, row_index, column_index));
            output.push_str(&paragraph_xml(cell, None, None));
            output.push_str("</w:tc>");
        }
        output.push_str("</w:tr>");
    }
    output.push_str("</w:tbl>");
    output
}

fn document_xml(builder: &WordRichDocumentBuilder) -> Result<String, WordRichGenerationError> {
    let numbering_ids = builder
        .numbering
        .keys()
        .enumerate()
        .map(|(index, identity)| (identity.as_str(), index as u32 + 1))
        .collect::<BTreeMap<_, _>>();
    let bookmarks = builder
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.id(), format!("am_{}", index + 1)))
        .collect::<BTreeMap<_, _>>();
    let mut body = String::new();
    for (index, block) in builder.blocks.iter().enumerate() {
        let bookmark = bookmarks
            .get(block.id())
            .expect("bookmark map is built from the same blocks");
        match block {
            RichBlock::Paragraph {
                text,
                style_id,
                numbering_id,
                ..
            } => body.push_str(&bookmarked_paragraph_xml(
                index + 1,
                bookmark,
                text,
                style_id.as_deref(),
                numbering_id
                    .as_deref()
                    .and_then(|identity| numbering_ids.get(identity).copied()),
            )),
            RichBlock::Table { rows, layout, .. } => {
                write!(&mut body, "<w:p><w:bookmarkStart w:id=\"{}\" w:name=\"{}\"/><w:bookmarkEnd w:id=\"{}\"/></w:p>", index + 1, xml_escape(bookmark), index + 1).expect("String write");
                body.push_str(&table_xml(rows, layout));
            }
            RichBlock::InternalHyperlink { label, anchor, .. } => {
                let target = bookmarks
                    .get(anchor.as_str())
                    .ok_or(WordRichGenerationError::BlockConflict)?;
                write!(&mut body, "<w:p><w:bookmarkStart w:id=\"{}\" w:name=\"{}\"/><w:bookmarkEnd w:id=\"{}\"/><w:hyperlink w:anchor=\"{}\"><w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr><w:t>{}</w:t></w:r></w:hyperlink></w:p>", index + 1, xml_escape(bookmark), index + 1, xml_escape(target), xml_escape(label)).expect("String write");
            }
        }
    }
    let mut references = String::new();
    if builder.header_footer.header_text.is_some() {
        references.push_str("<w:headerReference w:type=\"default\" r:id=\"rIdHeader\"/>");
    }
    if builder.header_footer.footer_text.is_some() {
        references.push_str("<w:footerReference w:type=\"default\" r:id=\"rIdFooter\"/>");
    }
    let orientation = if builder.page.landscape {
        " w:orient=\"landscape\""
    } else {
        ""
    };
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{body}<w:sectPr>{references}<w:pgSz w:w=\"{}\" w:h=\"{}\"{orientation}/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\"/><w:cols w:num=\"{}\"/></w:sectPr></w:body></w:document>",
        builder.page.width_twips,
        builder.page.height_twips,
        builder.page.top_margin_twips,
        builder.page.right_margin_twips,
        builder.page.bottom_margin_twips,
        builder.page.left_margin_twips,
        builder.page.columns,
    ))
}

fn styles_xml(styles: &BTreeMap<String, WordStyleConfiguration>) -> String {
    let mut output = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">",
    );
    for style in styles.values() {
        let style_type = match style.kind {
            WordStyleKind::Paragraph => "paragraph",
            WordStyleKind::Character => "character",
            WordStyleKind::Table => "table",
        };
        write!(
            &mut output,
            "<w:style w:type=\"{style_type}\" w:styleId=\"{}\"><w:name w:val=\"{}\"/>",
            xml_escape(&style.style_id),
            xml_escape(&style.name)
        )
        .expect("String write");
        if style.kind != WordStyleKind::Table {
            output.push_str("<w:rPr>");
            if let Some(font) = &style.font_family {
                write!(
                    &mut output,
                    "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>",
                    xml_escape(font),
                    xml_escape(font)
                )
                .expect("String write");
            }
            if let Some(size) = style.half_points {
                write!(&mut output, "<w:sz w:val=\"{size}\"/>").expect("String write");
            }
            if style.bold {
                output.push_str("<w:b/>");
            }
            if let Some(fill) = &style.shading_rgb {
                write!(&mut output, "<w:shd w:fill=\"{}\"/>", xml_escape(fill))
                    .expect("String write");
            }
            output.push_str("</w:rPr>");
        }
        output.push_str("</w:style>");
    }
    output.push_str("</w:styles>");
    output
}

fn numbering_xml(numbering: &BTreeMap<String, WordNumberingDefinition>) -> String {
    let mut output = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">",
    );
    for (index, definition) in numbering.values().enumerate() {
        let id = index + 1;
        let format = if definition.ordered {
            "decimal"
        } else {
            "bullet"
        };
        write!(&mut output, "<w:abstractNum w:abstractNumId=\"{id}\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"{format}\"/><w:lvlText w:val=\"{}\"/></w:lvl></w:abstractNum><w:num w:numId=\"{id}\"><w:abstractNumId w:val=\"{id}\"/></w:num>", xml_escape(&definition.level_text)).expect("String write");
    }
    output.push_str("</w:numbering>");
    output
}

fn simple_text_part(root: &str, text: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:{root} xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{}</w:{root}>", paragraph_xml(text, None, None)).into_bytes()
}

fn core_properties_xml(metadata: &WordDocumentMetadata) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:subject>{}</dc:subject><dc:creator>{}</dc:creator><dc:description>{}</dc:description></cp:coreProperties>", xml_escape(metadata.title.as_deref().unwrap_or("")), xml_escape(metadata.subject.as_deref().unwrap_or("")), xml_escape(metadata.creator.as_deref().unwrap_or("")), xml_escape(metadata.description.as_deref().unwrap_or(""))).into_bytes()
}

fn package_parts(
    builder: &WordRichDocumentBuilder,
) -> Result<BTreeMap<String, Vec<u8>>, WordRichGenerationError> {
    let mut overrides = String::from(
        "<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/><Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>",
    );
    let mut relationships = String::from(
        "<Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rIdNumbering\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>",
    );
    let mut parts = BTreeMap::new();
    if let Some(header) = &builder.header_footer.header_text {
        overrides.push_str("<Override PartName=\"/word/header1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>");
        relationships.push_str("<Relationship Id=\"rIdHeader\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header1.xml\"/>");
        parts.insert(
            "word/header1.xml".to_owned(),
            simple_text_part("hdr", header),
        );
    }
    if let Some(footer) = &builder.header_footer.footer_text {
        overrides.push_str("<Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>");
        relationships.push_str("<Relationship Id=\"rIdFooter\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer1.xml\"/>");
        parts.insert(
            "word/footer1.xml".to_owned(),
            simple_text_part("ftr", footer),
        );
    }
    parts.insert("[Content_Types].xml".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{overrides}</Types>").into_bytes());
    parts.insert("_rels/.rels".to_owned(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>".to_vec());
    parts.insert(
        "docProps/core.xml".to_owned(),
        core_properties_xml(&builder.metadata),
    );
    parts.insert(
        "word/document.xml".to_owned(),
        document_xml(builder)?.into_bytes(),
    );
    parts.insert("word/_rels/document.xml.rels".to_owned(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{relationships}</Relationships>").into_bytes());
    parts.insert(
        "word/numbering.xml".to_owned(),
        numbering_xml(&builder.numbering).into_bytes(),
    );
    parts.insert(
        "word/styles.xml".to_owned(),
        styles_xml(&builder.styles).into_bytes(),
    );
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;
    use crate::{WordFeatureKind, extract_docx_to_sidecar};

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-rich-word"),
            ["out", "report.docx"],
        )
        .expect("path")
    }

    #[test]
    fn rich_builder_exercises_helpers_and_reopens_without_effects() {
        let mut builder = WordRichDocumentBuilder::new("rich-report").expect("builder");
        builder
            .configure_styles([WordStyleConfiguration {
                style_id: "CustomBody".to_owned(),
                name: "Custom Body".to_owned(),
                kind: WordStyleKind::Paragraph,
                font_family: Some("Liberation Sans".to_owned()),
                half_points: Some(24),
                bold: false,
                shading_rgb: None,
            }])
            .expect("styles");
        builder
            .configure_page(WordPageConfiguration {
                columns: 2,
                ..WordPageConfiguration::default()
            })
            .expect("page");
        builder
            .configure_header_footer(WordHeaderFooterConfiguration {
                header_text: Some("Header".to_owned()),
                footer_text: Some("Footer".to_owned()),
            })
            .expect("header/footer");
        builder
            .configure_metadata(WordDocumentMetadata {
                title: Some("Report".to_owned()),
                creator: Some("AgentMage".to_owned()),
                custom: BTreeMap::from([("Status".to_owned(), "Draft".to_owned())]),
                ..WordDocumentMetadata::default()
            })
            .expect("metadata");
        builder
            .add_numbering_definition(WordNumberingDefinition {
                numbering_id: "decimal-1".to_owned(),
                ordered: true,
                level_text: "%1.".to_owned(),
            })
            .expect("numbering");
        builder
            .add_paragraph("p-1", "Heading", Some("CustomBody"), None)
            .expect("paragraph");
        builder.clone_paragraph("p-1", "p-2").expect("clone");
        builder.apply_numbering("p-2", "decimal-1").expect("apply");
        builder
            .replace_section(
                "p-1",
                "p-1",
                vec![("p-3".to_owned(), "Replacement".to_owned())],
            )
            .expect("replace");
        builder
            .add_paragraph("temporary", "Remove this", None, None)
            .expect("temporary");
        builder.remove_paragraph("temporary").expect("remove");
        builder
            .add_warning("warning-1", "Review required")
            .expect("warning");
        builder
            .add_metadata_table("metadata-1", WordTableLayout::default())
            .expect("metadata table");
        let mut layout = WordTableLayout::default();
        layout.set_cell_shading(Some("D9EAF7")).expect("shading");
        layout.set_cell_margins(80, 120, 80, 120).expect("margins");
        layout.set_table_width(8_000).expect("width");
        layout.set_table_borders(WordBorderStyle::Double);
        layout
            .set_single_cell_border(0, 0, WordBorderStyle::Dotted)
            .expect("cell border");
        layout
            .set_single_cell_shading(0, 1, "E2F0D9")
            .expect("cell shading");
        builder
            .add_decision_card(
                "decision-1",
                WordDecisionCard {
                    decision_id: "D-1".to_owned(),
                    title: "Proceed".to_owned(),
                    status: "Proposed".to_owned(),
                    rationale: "Evidence reviewed".to_owned(),
                    owner: None,
                },
                layout,
            )
            .expect("card");
        builder
            .add_hyperlink("link-1", "Jump", "decision-1")
            .expect("link");
        let first = builder
            .build(path(), &WordConversionProfile::strict_default())
            .expect("build");
        let second = builder
            .build(path(), &WordConversionProfile::strict_default())
            .expect("build");
        assert_eq!(first, second);
        assert!(first.proposal_only);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);
        for feature in [
            WordFeatureKind::Header,
            WordFeatureKind::Footer,
            WordFeatureKind::Table,
            WordFeatureKind::Hyperlink,
            WordFeatureKind::Numbering,
            WordFeatureKind::SectionLayout,
            WordFeatureKind::Style,
        ] {
            assert!(
                first
                    .inspection
                    .features
                    .iter()
                    .any(|item| item.kind == feature)
            );
        }
        let extracted = extract_docx_to_sidecar(
            &path(),
            &first.package,
            &WordConversionProfile::strict_default(),
        )
        .expect("extract");
        let sidecar = String::from_utf8(extracted.sidecar).expect("UTF-8");
        for expected in [
            "Replacement",
            "Review required",
            "Proceed",
            "Header",
            "Footer",
        ] {
            assert!(sidecar.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn invalid_layout_identity_and_external_hyperlinks_fail_closed() {
        let mut builder = WordRichDocumentBuilder::new("rich-report").expect("builder");
        assert_eq!(
            builder.add_hyperlink("link", "External", "https://example.invalid"),
            Err(WordRichGenerationError::InvalidInput)
        );
        builder
            .add_hyperlink("unresolved", "Missing", "missing-target")
            .expect("syntactically valid unresolved link");
        assert_eq!(
            builder.build(path(), &WordConversionProfile::strict_default()),
            Err(WordRichGenerationError::BlockConflict)
        );
        builder.blocks.clear();
        assert_eq!(
            builder.add_paragraph("p", "text", Some("missing"), None),
            Err(WordRichGenerationError::InvalidInput)
        );
        let mut layout = WordTableLayout::default();
        assert_eq!(
            layout.set_table_width(1),
            Err(WordRichGenerationError::InvalidInput)
        );
        assert_eq!(
            layout.set_cell_shading(Some("not-rgb")),
            Err(WordRichGenerationError::InvalidInput)
        );
        builder
            .add_paragraph("p", "text", None, None)
            .expect("paragraph");
        assert_eq!(
            builder.add_paragraph("p", "duplicate", None, None),
            Err(WordRichGenerationError::BlockConflict)
        );
    }
}
