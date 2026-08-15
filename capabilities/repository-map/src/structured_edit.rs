//! Authority-free syntax-aware and exact-text code-change planning.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

use crate::{
    ParseDisposition, RepositoryLanguage, language_for_path, parse_structure,
    verify_structural_parse_result,
};

const STRUCTURED_EDIT_SCHEMA_VERSION: u16 = 1;
const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_EDITS: usize = 256;
const MAX_TEXT_BYTES: usize = 64 * 1024;

/// Closed artifact class aligned with the kernel shadow-change boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredArtifactClass {
    /// Application or library source code.
    Code,
    /// Repository or product configuration.
    Configuration,
    /// Test source, fixture, or golden.
    Test,
    /// Documentation or examples.
    Documentation,
    /// Data, schema, or compatibility migration.
    Migration,
    /// Explicitly authorized generated output.
    GeneratedOutput,
}

/// Closed source language or narrow textual dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredLanguage {
    /// Rust with the pinned Tree-sitter grammar.
    Rust,
    /// Python with the pinned Tree-sitter grammar.
    Python,
    /// TypeScript with the pinned Tree-sitter grammar.
    TypeScript,
    /// TSX with the pinned Tree-sitter grammar.
    Tsx,
    /// JavaScript with the pinned Tree-sitter grammar.
    JavaScript,
    /// Swift with the pinned Tree-sitter grammar.
    Swift,
    /// Go using only unique exact-text fallback in this increment.
    Go,
    /// POSIX-like shell using only unique exact-text fallback in this increment.
    Shell,
    /// SQL using only unique exact-text fallback in this increment.
    Sql,
    /// Markdown, configuration, or another explicitly textual artifact.
    PlainText,
}

impl StructuredLanguage {
    fn repository_language(self) -> Option<RepositoryLanguage> {
        match self {
            Self::Rust => Some(RepositoryLanguage::Rust),
            Self::Python => Some(RepositoryLanguage::Python),
            Self::TypeScript => Some(RepositoryLanguage::TypeScript),
            Self::Tsx => Some(RepositoryLanguage::Tsx),
            Self::JavaScript => Some(RepositoryLanguage::JavaScript),
            Self::Swift => Some(RepositoryLanguage::Swift),
            Self::Go | Self::Shell | Self::Sql | Self::PlainText => None,
        }
    }
}

/// How one edit was structurally validated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredEditMethod {
    /// Exact Tree-sitter node or complete-tree validation.
    SyntaxTree,
    /// Unique exact-text replacement with bounded unchanged-span proof.
    ExactTextFallback,
    /// Deterministic terminal-newline normalization.
    TerminalNewline,
}

/// Closed review hooks selected before a shadow change is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredReviewHook {
    /// Interface and caller review.
    Interface,
    /// Dependency purpose, provenance, and license review.
    Dependency,
    /// Forward, rollback, and partial-failure migration review.
    Migration,
    /// Security boundary review.
    Security,
    /// Measured performance review.
    Performance,
    /// Accessibility review.
    Accessibility,
    /// Format, platform, and caller compatibility review.
    Compatibility,
}

/// One exact proposed edit over immutable preimage bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StructuredEdit {
    /// Rename every exact identifier syntax node with the requested name.
    RenameIdentifier {
        /// Stable edit identity.
        edit_id: String,
        /// Exact old identifier.
        old: String,
        /// Exact replacement identifier.
        replacement: String,
    },
    /// Replace one complete exact named syntax node.
    ReplaceSyntaxNode {
        /// Stable edit identity.
        edit_id: String,
        /// Inclusive zero-based byte offset.
        start_byte: u64,
        /// Exclusive zero-based byte offset.
        end_byte: u64,
        /// SHA-256 of the exact replaced node bytes.
        expected_node_sha256: String,
        /// Replacement syntax bytes represented as UTF-8.
        replacement: String,
    },
    /// Insert one import at the file start or exact end of an existing import node.
    InsertImport {
        /// Stable edit identity.
        edit_id: String,
        /// Exact insertion byte offset in the current preimage.
        at_byte: u64,
        /// Complete import statement, including its line ending.
        statement: String,
    },
    /// Replace one unique exact text fragment in a fallback-only language.
    ReplaceExactText {
        /// Stable edit identity.
        edit_id: String,
        /// Exact unique preimage fragment.
        expected: String,
        /// Exact replacement fragment.
        replacement: String,
    },
    /// Ensure exactly one terminal LF without changing other bytes.
    NormalizeTerminalNewline {
        /// Stable edit identity.
        edit_id: String,
    },
}

impl StructuredEdit {
    fn id(&self) -> &str {
        match self {
            Self::RenameIdentifier { edit_id, .. }
            | Self::ReplaceSyntaxNode { edit_id, .. }
            | Self::InsertImport { edit_id, .. }
            | Self::ReplaceExactText { edit_id, .. }
            | Self::NormalizeTerminalNewline { edit_id } => edit_id,
        }
    }
}

/// Immutable caller input for one in-memory file-change plan.
#[derive(Clone, PartialEq, Eq)]
pub struct StructuredFileChangeRequest {
    /// Stable change identity.
    pub change_id: String,
    /// Exact canonical workspace path.
    pub path: WorkspacePath,
    /// Exact normalized change-intent identity.
    pub intent_sha256: String,
    /// Exact review-ready change-plan identity.
    pub change_plan_sha256: String,
    /// Explicit source language or dialect.
    pub language: StructuredLanguage,
    /// Explicit artifact classification.
    pub artifact_class: StructuredArtifactClass,
    /// Complete immutable preimage bytes.
    pub preimage: Vec<u8>,
    /// Caller-declared complete preimage identity.
    pub expected_preimage_sha256: String,
    /// Stable ordered edit list.
    pub edits: Vec<StructuredEdit>,
    /// Additional evidence-selected review hooks.
    pub additional_review_hooks: Vec<StructuredReviewHook>,
    /// Whether repository evidence classifies this artifact as generated.
    pub generated: bool,
    /// Exact per-change generated-output exception.
    pub allow_generated: bool,
}

impl std::fmt::Debug for StructuredFileChangeRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StructuredFileChangeRequest")
            .field("change_id", &self.change_id)
            .field("path", &self.path)
            .field("intent_sha256", &self.intent_sha256)
            .field("change_plan_sha256", &self.change_plan_sha256)
            .field("language", &self.language)
            .field("artifact_class", &self.artifact_class)
            .field("preimage_sha256", &self.expected_preimage_sha256)
            .field("edits", &self.edits)
            .field("additional_review_hooks", &self.additional_review_hooks)
            .field("generated", &self.generated)
            .field("allow_generated", &self.allow_generated)
            .finish_non_exhaustive()
    }
}

/// One exact changed byte range in preimage coordinates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredChangedRange {
    /// Stable source edit identity.
    pub edit_id: String,
    /// Inclusive preimage byte offset.
    pub start_byte: u64,
    /// Exclusive preimage byte offset.
    pub end_byte: u64,
    /// Exact replaced preimage digest; the empty span uses SHA-256 of empty bytes.
    pub before_sha256: String,
    /// Exact inserted or replacement digest.
    pub after_sha256: String,
    /// Structural validation method.
    pub method: StructuredEditMethod,
}

/// One byte-identical span retained between changed ranges.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredUnchangedSpan {
    /// Inclusive preimage byte offset.
    pub start_byte: u64,
    /// Exclusive preimage byte offset.
    pub end_byte: u64,
    /// SHA-256 of the exact unchanged bytes.
    pub bytes_sha256: String,
}

/// Redacted serializable review view with no source or replacement text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredFileChangeSummary {
    /// Schema version.
    pub schema_version: u16,
    /// Stable change identity.
    pub change_id: String,
    /// Exact canonical workspace path.
    pub path: WorkspacePath,
    /// Exact intent identity.
    pub intent_sha256: String,
    /// Exact change-plan identity.
    pub change_plan_sha256: String,
    /// Explicit language.
    pub language: StructuredLanguage,
    /// Explicit artifact class.
    pub artifact_class: StructuredArtifactClass,
    /// Exact complete preimage digest.
    pub preimage_sha256: String,
    /// Exact complete postimage digest.
    pub postimage_sha256: String,
    /// Changed preimage ranges and content identities.
    pub changed_ranges: Vec<StructuredChangedRange>,
    /// Hashes of every byte-identical preimage span.
    pub unchanged_spans: Vec<StructuredUnchangedSpan>,
    /// Deterministically selected review hooks.
    pub review_hooks: Vec<StructuredReviewHook>,
    /// Whether a pinned syntax tree verified the complete postimage.
    pub postimage_syntax_verified: bool,
    /// Planning never grants mutation authority.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding summary field.
    pub summary_sha256: String,
}

/// Complete in-memory plan retaining bytes only for a later explicit shadow proposal.
#[derive(Clone, PartialEq, Eq)]
pub struct StructuredFileChangePlan {
    request: StructuredFileChangeRequest,
    postimage: Vec<u8>,
    summary: StructuredFileChangeSummary,
    plan_sha256: String,
}

impl std::fmt::Debug for StructuredFileChangePlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StructuredFileChangePlan")
            .field("summary", &self.summary)
            .field("plan_sha256", &self.plan_sha256)
            .finish_non_exhaustive()
    }
}

impl StructuredFileChangePlan {
    /// Returns the complete immutable preimage bytes.
    #[must_use]
    pub fn preimage(&self) -> &[u8] {
        &self.request.preimage
    }

    /// Returns the complete proposed postimage bytes.
    #[must_use]
    pub fn postimage(&self) -> &[u8] {
        &self.postimage
    }

    /// Returns the redacted review summary.
    #[must_use]
    pub const fn summary(&self) -> &StructuredFileChangeSummary {
        &self.summary
    }

    /// Returns the exact complete plan identity.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

/// Content-free structured-edit failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuredEditError {
    /// Identifier, hash, path, bound, ordering, or classification is invalid.
    InvalidInput,
    /// Preimage bytes or their exact identity are invalid.
    PreimageInvalid,
    /// Language and path disagree or no safe adapter exists.
    LanguageUnsupported,
    /// Pinned syntax parsing failed or reported malformed syntax.
    SyntaxInvalid,
    /// Requested syntax node or unique exact text was not found exactly.
    EditTargetInvalid,
    /// Changed byte ranges overlap or cannot be represented safely.
    EditCollision,
    /// Generated output lacks an exact exception.
    GeneratedDenied,
}

impl StructuredEditError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "structured-edit.input-invalid",
            Self::PreimageInvalid => "structured-edit.preimage-invalid",
            Self::LanguageUnsupported => "structured-edit.language-unsupported",
            Self::SyntaxInvalid => "structured-edit.syntax-invalid",
            Self::EditTargetInvalid => "structured-edit.target-invalid",
            Self::EditCollision => "structured-edit.collision",
            Self::GeneratedDenied => "structured-edit.generated-denied",
        }
    }
}

#[derive(Clone)]
struct Replacement {
    edit_id: String,
    start: usize,
    end: usize,
    after: Vec<u8>,
    method: StructuredEditMethod,
}

/// Builds one deterministic in-memory change plan without writing any file.
pub fn build_structured_file_change(
    request: StructuredFileChangeRequest,
) -> Result<StructuredFileChangePlan, StructuredEditError> {
    validate_request(&request)?;
    let path = display_path(&request.path);
    let parser_language = request.language.repository_language();
    if parser_language.is_some() != language_for_path(&path).is_some()
        || parser_language.is_some_and(|language| language_for_path(&path) != Some(language))
        || (parser_language.is_none() && fallback_language_for_path(&path) != request.language)
    {
        return Err(StructuredEditError::LanguageUnsupported);
    }
    if request.generated && !request.allow_generated {
        return Err(StructuredEditError::GeneratedDenied);
    }
    if (request.artifact_class == StructuredArtifactClass::GeneratedOutput) != request.generated {
        return Err(StructuredEditError::InvalidInput);
    }
    let source =
        std::str::from_utf8(&request.preimage).map_err(|_| StructuredEditError::PreimageInvalid)?;
    if parser_language.is_some_and(|language| !valid_parse(language, &path, &request.preimage)) {
        return Err(StructuredEditError::SyntaxInvalid);
    }

    let mut replacements = Vec::new();
    for edit in &request.edits {
        replacements.extend(resolve_edit(
            edit,
            request.language,
            parser_language,
            source,
        )?);
    }
    replacements.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| left.end.cmp(&right.end))
            .then_with(|| left.edit_id.cmp(&right.edit_id))
    });
    if replacements.is_empty()
        || replacements
            .windows(2)
            .any(|pair| pair[0].end > pair[1].start || pair[0].start == pair[1].start)
    {
        return Err(StructuredEditError::EditCollision);
    }
    let changed_ranges = replacements
        .iter()
        .map(|replacement| StructuredChangedRange {
            edit_id: replacement.edit_id.clone(),
            start_byte: replacement.start as u64,
            end_byte: replacement.end as u64,
            before_sha256: sha256_hex(&request.preimage[replacement.start..replacement.end]),
            after_sha256: sha256_hex(&replacement.after),
            method: replacement.method,
        })
        .collect::<Vec<_>>();
    let unchanged_spans = unchanged_spans(&request.preimage, &replacements);
    let postimage = apply_replacements(&request.preimage, &replacements);
    if postimage == request.preimage
        || postimage.len() > MAX_SOURCE_BYTES
        || std::str::from_utf8(&postimage).is_err()
    {
        return Err(StructuredEditError::EditTargetInvalid);
    }
    let postimage_syntax_verified =
        parser_language.is_some_and(|language| valid_parse(language, &path, &postimage));
    if parser_language.is_some() && !postimage_syntax_verified {
        return Err(StructuredEditError::SyntaxInvalid);
    }
    let review_hooks = derive_review_hooks(&request);
    let mut summary = StructuredFileChangeSummary {
        schema_version: STRUCTURED_EDIT_SCHEMA_VERSION,
        change_id: request.change_id.clone(),
        path: request.path.clone(),
        intent_sha256: request.intent_sha256.clone(),
        change_plan_sha256: request.change_plan_sha256.clone(),
        language: request.language,
        artifact_class: request.artifact_class,
        preimage_sha256: request.expected_preimage_sha256.clone(),
        postimage_sha256: sha256_hex(&postimage),
        changed_ranges,
        unchanged_spans,
        review_hooks,
        postimage_syntax_verified,
        mutation_authority: false,
        summary_sha256: String::new(),
    };
    summary.summary_sha256 = summary_digest(&summary);
    let plan_sha256 = sha256_json(&(
        summary.summary_sha256.as_str(),
        request.edits.as_slice(),
        request.generated,
        request.allow_generated,
    ));
    Ok(StructuredFileChangePlan {
        request,
        postimage,
        summary,
        plan_sha256,
    })
}

/// Verifies one plan by exact deterministic recomputation.
#[must_use]
pub fn verify_structured_file_change(plan: &StructuredFileChangePlan) -> bool {
    !plan.summary.mutation_authority
        && plan.summary.summary_sha256 == summary_digest(&plan.summary)
        && build_structured_file_change(plan.request.clone())
            .is_ok_and(|expected| expected == *plan)
}

fn validate_request(request: &StructuredFileChangeRequest) -> Result<(), StructuredEditError> {
    if !valid_identifier(&request.change_id)
        || !is_sha256(&request.intent_sha256)
        || !is_sha256(&request.change_plan_sha256)
        || request.preimage.is_empty()
        || request.preimage.len() > MAX_SOURCE_BYTES
        || request.expected_preimage_sha256 != sha256_hex(&request.preimage)
        || request.edits.is_empty()
        || request.edits.len() > MAX_EDITS
        || request
            .edits
            .windows(2)
            .any(|pair| pair[0].id() >= pair[1].id())
        || request
            .edits
            .iter()
            .any(|edit| !valid_identifier(edit.id()))
        || request.additional_review_hooks.len() > MAX_EDITS
        || request
            .additional_review_hooks
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(StructuredEditError::InvalidInput);
    }
    Ok(())
}

fn resolve_edit(
    edit: &StructuredEdit,
    language: StructuredLanguage,
    parser_language: Option<RepositoryLanguage>,
    source: &str,
) -> Result<Vec<Replacement>, StructuredEditError> {
    match edit {
        StructuredEdit::RenameIdentifier {
            edit_id,
            old,
            replacement,
        } => {
            let parser_language =
                parser_language.ok_or(StructuredEditError::LanguageUnsupported)?;
            if !valid_identifier(old) || !valid_identifier(replacement) || old == replacement {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            let tree = syntax_tree(parser_language, source.as_bytes())?;
            let mut ranges = identifier_ranges(tree.root_node(), source.as_bytes(), old);
            ranges.sort_unstable();
            ranges.dedup();
            if ranges.is_empty() || ranges.len() > MAX_EDITS {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            Ok(ranges
                .into_iter()
                .map(|(start, end)| Replacement {
                    edit_id: edit_id.clone(),
                    start,
                    end,
                    after: replacement.as_bytes().to_vec(),
                    method: StructuredEditMethod::SyntaxTree,
                })
                .collect())
        }
        StructuredEdit::ReplaceSyntaxNode {
            edit_id,
            start_byte,
            end_byte,
            expected_node_sha256,
            replacement,
        } => {
            let parser_language =
                parser_language.ok_or(StructuredEditError::LanguageUnsupported)?;
            let (start, end) = checked_range(*start_byte, *end_byte, source.len())?;
            if replacement.is_empty()
                || replacement.len() > MAX_TEXT_BYTES
                || sha256_hex(&source.as_bytes()[start..end]) != *expected_node_sha256
            {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            let tree = syntax_tree(parser_language, source.as_bytes())?;
            if !has_exact_named_node(tree.root_node(), start, end) {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            Ok(vec![Replacement {
                edit_id: edit_id.clone(),
                start,
                end,
                after: replacement.as_bytes().to_vec(),
                method: StructuredEditMethod::SyntaxTree,
            }])
        }
        StructuredEdit::InsertImport {
            edit_id,
            at_byte,
            statement,
        } => {
            let parser_language =
                parser_language.ok_or(StructuredEditError::LanguageUnsupported)?;
            let offset = usize::try_from(*at_byte)
                .ok()
                .filter(|offset| *offset <= source.len())
                .ok_or(StructuredEditError::EditTargetInvalid)?;
            if statement.is_empty()
                || statement.len() > MAX_TEXT_BYTES
                || !statement.ends_with('\n')
                || statement.contains('\r')
                || !import_offset_is_valid(parser_language, source.as_bytes(), offset)?
            {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            Ok(vec![Replacement {
                edit_id: edit_id.clone(),
                start: offset,
                end: offset,
                after: statement.as_bytes().to_vec(),
                method: StructuredEditMethod::SyntaxTree,
            }])
        }
        StructuredEdit::ReplaceExactText {
            edit_id,
            expected,
            replacement,
        } => {
            if parser_language.is_some()
                || !matches!(
                    language,
                    StructuredLanguage::Go
                        | StructuredLanguage::Shell
                        | StructuredLanguage::Sql
                        | StructuredLanguage::PlainText
                )
                || expected.is_empty()
                || replacement.is_empty()
                || expected.len() > MAX_TEXT_BYTES
                || replacement.len() > MAX_TEXT_BYTES
                || expected == replacement
                || expected.contains('\r')
                || replacement.contains('\r')
            {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            let matches = source.match_indices(expected).collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            let start = matches[0].0;
            Ok(vec![Replacement {
                edit_id: edit_id.clone(),
                start,
                end: start + expected.len(),
                after: replacement.as_bytes().to_vec(),
                method: StructuredEditMethod::ExactTextFallback,
            }])
        }
        StructuredEdit::NormalizeTerminalNewline { edit_id } => {
            if source.ends_with('\n') || source.contains('\r') {
                return Err(StructuredEditError::EditTargetInvalid);
            }
            Ok(vec![Replacement {
                edit_id: edit_id.clone(),
                start: source.len(),
                end: source.len(),
                after: vec![b'\n'],
                method: StructuredEditMethod::TerminalNewline,
            }])
        }
    }
}

fn valid_parse(language: RepositoryLanguage, path: &str, bytes: &[u8]) -> bool {
    parse_structure(language, path, bytes).is_ok_and(|result| {
        verify_structural_parse_result(&result)
            && result.disposition == ParseDisposition::Parsed
            && !result.truncated
    })
}

fn syntax_tree(
    language: RepositoryLanguage,
    source: &[u8],
) -> Result<tree_sitter::Tree, StructuredEditError> {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::grammar::language(language))
        .map_err(|_| StructuredEditError::SyntaxInvalid)?;
    let tree = parser
        .parse(source, None)
        .ok_or(StructuredEditError::SyntaxInvalid)?;
    if tree.root_node().has_error() {
        return Err(StructuredEditError::SyntaxInvalid);
    }
    Ok(tree)
}

fn identifier_ranges(root: Node<'_>, source: &[u8], expected: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if is_identifier_node(node.kind()) && node.utf8_text(source).ok() == Some(expected) {
            ranges.push((node.start_byte(), node.end_byte()));
        }
        let mut children = node.named_children(&mut node.walk()).collect::<Vec<_>>();
        children.reverse();
        stack.extend(children);
    }
    ranges
}

fn is_identifier_node(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "field_identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "type_identifier"
    )
}

fn has_exact_named_node(root: Node<'_>, start: usize, end: usize) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_named() && node.start_byte() == start && node.end_byte() == end {
            return true;
        }
        let mut children = node.named_children(&mut node.walk()).collect::<Vec<_>>();
        children.reverse();
        stack.extend(children);
    }
    false
}

fn import_offset_is_valid(
    language: RepositoryLanguage,
    source: &[u8],
    offset: usize,
) -> Result<bool, StructuredEditError> {
    if offset == 0 {
        return Ok(true);
    }
    let result = parse_structure(language, "import-offset", source)
        .map_err(|_| StructuredEditError::SyntaxInvalid)?;
    Ok(result.items.iter().any(|item| {
        if item.kind != crate::StructuralItemKind::Import {
            return false;
        }
        let end = item.range.end_byte as usize;
        offset == end || (source.get(end) == Some(&b'\n') && offset == end + 1)
    }))
}

fn checked_range(
    start: u64,
    end: u64,
    source_len: usize,
) -> Result<(usize, usize), StructuredEditError> {
    let start = usize::try_from(start).map_err(|_| StructuredEditError::EditTargetInvalid)?;
    let end = usize::try_from(end).map_err(|_| StructuredEditError::EditTargetInvalid)?;
    if start >= end || end > source_len {
        return Err(StructuredEditError::EditTargetInvalid);
    }
    Ok((start, end))
}

fn unchanged_spans(source: &[u8], replacements: &[Replacement]) -> Vec<StructuredUnchangedSpan> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    for replacement in replacements {
        if cursor < replacement.start {
            spans.push(StructuredUnchangedSpan {
                start_byte: cursor as u64,
                end_byte: replacement.start as u64,
                bytes_sha256: sha256_hex(&source[cursor..replacement.start]),
            });
        }
        cursor = replacement.end;
    }
    if cursor < source.len() {
        spans.push(StructuredUnchangedSpan {
            start_byte: cursor as u64,
            end_byte: source.len() as u64,
            bytes_sha256: sha256_hex(&source[cursor..]),
        });
    }
    spans
}

fn apply_replacements(source: &[u8], replacements: &[Replacement]) -> Vec<u8> {
    let removed = replacements
        .iter()
        .map(|replacement| replacement.end - replacement.start)
        .sum::<usize>();
    let added = replacements
        .iter()
        .map(|replacement| replacement.after.len())
        .sum::<usize>();
    let mut output = Vec::with_capacity(source.len() - removed + added);
    let mut cursor = 0;
    for replacement in replacements {
        output.extend_from_slice(&source[cursor..replacement.start]);
        output.extend_from_slice(&replacement.after);
        cursor = replacement.end;
    }
    output.extend_from_slice(&source[cursor..]);
    output
}

fn derive_review_hooks(request: &StructuredFileChangeRequest) -> Vec<StructuredReviewHook> {
    let mut hooks = request
        .additional_review_hooks
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if request
        .edits
        .iter()
        .any(|edit| matches!(edit, StructuredEdit::RenameIdentifier { .. }))
    {
        hooks.insert(StructuredReviewHook::Interface);
        hooks.insert(StructuredReviewHook::Compatibility);
    }
    match request.artifact_class {
        StructuredArtifactClass::Configuration => {
            hooks.insert(StructuredReviewHook::Security);
            hooks.insert(StructuredReviewHook::Compatibility);
        }
        StructuredArtifactClass::Migration => {
            hooks.insert(StructuredReviewHook::Migration);
            hooks.insert(StructuredReviewHook::Compatibility);
        }
        StructuredArtifactClass::Code
        | StructuredArtifactClass::Test
        | StructuredArtifactClass::Documentation
        | StructuredArtifactClass::GeneratedOutput => {}
    }
    hooks.into_iter().collect()
}

fn fallback_language_for_path(path: &str) -> StructuredLanguage {
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name.rsplit_once('.').map(|(_, extension)| extension);
    match extension {
        Some("go") => StructuredLanguage::Go,
        Some("sh" | "bash") => StructuredLanguage::Shell,
        Some("sql") => StructuredLanguage::Sql,
        _ => StructuredLanguage::PlainText,
    }
}

fn display_path(path: &WorkspacePath) -> String {
    path.components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn summary_digest(summary: &StructuredFileChangeSummary) -> String {
    sha256_json(&(
        summary.schema_version,
        &summary.change_id,
        &summary.path,
        &summary.intent_sha256,
        &summary.change_plan_sha256,
        summary.language,
        summary.artifact_class,
        &summary.preimage_sha256,
        &summary.postimage_sha256,
        &summary.changed_ranges,
        &summary.unchanged_spans,
        &summary.review_hooks,
        summary.postimage_syntax_verified,
        summary.mutation_authority,
    ))
}

fn sha256_json(value: &impl Serialize) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"structured-edit-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use super::*;

    fn path(value: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-structured-edit"),
            value.iter().copied(),
        )
        .expect("path")
    }

    fn request(
        path_parts: &[&str],
        language: StructuredLanguage,
        source: &str,
        edit: StructuredEdit,
    ) -> StructuredFileChangeRequest {
        StructuredFileChangeRequest {
            change_id: "change-structured-1".to_owned(),
            path: path(path_parts),
            intent_sha256: "1".repeat(64),
            change_plan_sha256: "2".repeat(64),
            language,
            artifact_class: StructuredArtifactClass::Code,
            preimage: source.as_bytes().to_vec(),
            expected_preimage_sha256: sha256_hex(source.as_bytes()),
            edits: vec![edit],
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        }
    }

    #[test]
    fn syntax_rename_preserves_unrelated_python_typescript_and_javascript_bytes() {
        let cases = [
            (
                &["src", "value.py"][..],
                StructuredLanguage::Python,
                "def old_name(value):\n    return old_name(value - 1) if value else 0\n",
            ),
            (
                &["src", "value.ts"][..],
                StructuredLanguage::TypeScript,
                "export function old_name(value: number): number { return value; }\nold_name(1);\n",
            ),
            (
                &["src", "value.js"][..],
                StructuredLanguage::JavaScript,
                "export function old_name(value) { return value; }\nold_name(1);\n",
            ),
        ];
        for (path_parts, language, source) in cases {
            let plan = build_structured_file_change(request(
                path_parts,
                language,
                source,
                StructuredEdit::RenameIdentifier {
                    edit_id: "edit-rename".to_owned(),
                    old: "old_name".to_owned(),
                    replacement: "new_name".to_owned(),
                },
            ))
            .expect("syntax rename");
            assert!(verify_structured_file_change(&plan));
            assert!(
                std::str::from_utf8(plan.postimage())
                    .expect("utf8")
                    .contains("new_name")
            );
            assert!(
                !std::str::from_utf8(plan.postimage())
                    .expect("utf8")
                    .contains("old_name")
            );
            assert!(plan.summary().postimage_syntax_verified);
            assert_eq!(
                plan.summary().review_hooks,
                [
                    StructuredReviewHook::Interface,
                    StructuredReviewHook::Compatibility
                ]
            );
            assert!(!plan.summary().mutation_authority);
        }
    }

    #[test]
    fn go_shell_sql_and_plain_text_use_only_unique_exact_fallback() {
        let cases = [
            (
                &["main.go"][..],
                StructuredLanguage::Go,
                "package main\nfunc main() { oldValue := 1 }\n",
                "oldValue",
                "newValue",
            ),
            (
                &["script.sh"][..],
                StructuredLanguage::Shell,
                "#!/bin/sh\nold_value=1\nprintf '%s\\n' \"$old_value\"\n",
                "old_value=1",
                "new_value=1",
            ),
            (
                &["query.sql"][..],
                StructuredLanguage::Sql,
                "SELECT old_column FROM sample;\n",
                "old_column",
                "new_column",
            ),
            (
                &["README.md"][..],
                StructuredLanguage::PlainText,
                "Before exact phrase after\n",
                "exact phrase",
                "reviewed phrase",
            ),
        ];
        for (path_parts, language, source, expected, replacement) in cases {
            let plan = build_structured_file_change(request(
                path_parts,
                language,
                source,
                StructuredEdit::ReplaceExactText {
                    edit_id: "edit-exact".to_owned(),
                    expected: expected.to_owned(),
                    replacement: replacement.to_owned(),
                },
            ))
            .expect("exact fallback");
            assert!(verify_structured_file_change(&plan));
            assert_eq!(plan.summary().changed_ranges.len(), 1);
            assert_eq!(
                plan.summary().changed_ranges[0].method,
                StructuredEditMethod::ExactTextFallback
            );
            assert!(!plan.summary().postimage_syntax_verified);
            assert!(!plan.summary().unchanged_spans.is_empty());
        }
    }

    #[test]
    fn import_node_replace_and_terminal_format_are_syntax_checked() {
        let source = "import os\n\ndef value():\n    return 1\n";
        let import = build_structured_file_change(request(
            &["module.py"],
            StructuredLanguage::Python,
            source,
            StructuredEdit::InsertImport {
                edit_id: "edit-import".to_owned(),
                at_byte: 10,
                statement: "import sys\n".to_owned(),
            },
        ))
        .expect("import");
        assert!(verify_structured_file_change(&import));
        assert!(
            std::str::from_utf8(import.postimage())
                .expect("utf8")
                .contains("import sys")
        );

        let node_source = "def value():\n    return 1\n";
        let start = node_source.find("return 1").expect("node");
        let end = start + "return 1".len();
        let replaced = build_structured_file_change(request(
            &["module.py"],
            StructuredLanguage::Python,
            node_source,
            StructuredEdit::ReplaceSyntaxNode {
                edit_id: "edit-node".to_owned(),
                start_byte: start as u64,
                end_byte: end as u64,
                expected_node_sha256: sha256_hex(b"return 1"),
                replacement: "return 2".to_owned(),
            },
        ))
        .expect("node replacement");
        assert!(verify_structured_file_change(&replaced));

        let formatted = build_structured_file_change(request(
            &["module.py"],
            StructuredLanguage::Python,
            "value = 1",
            StructuredEdit::NormalizeTerminalNewline {
                edit_id: "edit-format".to_owned(),
            },
        ))
        .expect("terminal newline");
        assert_eq!(formatted.postimage(), b"value = 1\n");
    }

    #[test]
    fn malformed_stale_mismatched_generated_duplicate_and_forged_plans_fail_closed() {
        let edit = StructuredEdit::RenameIdentifier {
            edit_id: "edit-rename".to_owned(),
            old: "value".to_owned(),
            replacement: "result".to_owned(),
        };
        let malformed = request(
            &["module.py"],
            StructuredLanguage::Python,
            "def value(\n",
            edit.clone(),
        );
        assert_eq!(
            build_structured_file_change(malformed),
            Err(StructuredEditError::SyntaxInvalid)
        );

        let mut stale = request(
            &["module.py"],
            StructuredLanguage::Python,
            "value = 1\n",
            edit.clone(),
        );
        stale.expected_preimage_sha256 = "9".repeat(64);
        assert_eq!(
            build_structured_file_change(stale),
            Err(StructuredEditError::InvalidInput)
        );

        let mismatched = request(
            &["module.go"],
            StructuredLanguage::Python,
            "value = 1\n",
            edit.clone(),
        );
        assert_eq!(
            build_structured_file_change(mismatched),
            Err(StructuredEditError::LanguageUnsupported)
        );

        let mut generated = request(
            &["generated.py"],
            StructuredLanguage::Python,
            "value = 1\n",
            edit.clone(),
        );
        generated.artifact_class = StructuredArtifactClass::GeneratedOutput;
        generated.generated = true;
        assert_eq!(
            build_structured_file_change(generated),
            Err(StructuredEditError::GeneratedDenied)
        );

        let mut duplicate = request(
            &["module.py"],
            StructuredLanguage::Python,
            "value = 1\n",
            edit.clone(),
        );
        duplicate.edits.push(edit);
        assert_eq!(
            build_structured_file_change(duplicate),
            Err(StructuredEditError::InvalidInput)
        );

        let mut plan = build_structured_file_change(request(
            &["module.py"],
            StructuredLanguage::Python,
            "value = 1\n",
            StructuredEdit::RenameIdentifier {
                edit_id: "edit-rename".to_owned(),
                old: "value".to_owned(),
                replacement: "result".to_owned(),
            },
        ))
        .expect("plan");
        plan.summary.mutation_authority = true;
        assert!(!verify_structured_file_change(&plan));
        assert!(!format!("{plan:?}").contains("value = 1"));
    }
}
