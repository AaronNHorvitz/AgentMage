//! Bounded parser-backed structural facts with exact source ranges.

use std::fmt::Write;
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, ParseOptions, Parser};

use crate::grammar::{RepositoryLanguage, grammar_descriptor, language};

const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_STRUCTURAL_ITEMS: usize = 10_000;
const MAX_ITEM_TEXT_BYTES: usize = 2_048;

/// Stable structural parse disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseDisposition {
    /// The pinned grammar parsed the source without syntax errors.
    Parsed,
    /// The pinned grammar returned a tree containing syntax errors.
    ParsedWithErrors,
    /// The bounded structural-item ceiling truncated traversal.
    Truncated,
}

/// Closed reliable structural item classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralItemKind {
    /// File-level module identity after successful parser activation.
    Module,
    /// Function or method definition.
    Function,
    /// Class definition.
    Class,
    /// Struct definition.
    Struct,
    /// Enum definition.
    Enum,
    /// Interface or protocol definition.
    Interface,
    /// Trait definition.
    Trait,
    /// Type alias definition.
    TypeAlias,
    /// Constant or static definition.
    Constant,
    /// Exact import or use declaration.
    Import,
}

/// Closed parser-backed relationship classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralRelationshipKind {
    /// One parsed module contains one exact import declaration.
    DeclaresImport,
}

/// Exact zero-based byte and one-based line range in one source revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    /// Inclusive zero-based start byte.
    pub start_byte: u64,
    /// Exclusive zero-based end byte.
    pub end_byte: u64,
    /// Inclusive one-based start line.
    pub start_line: u32,
    /// Inclusive one-based end line.
    pub end_line: u32,
    /// Zero-based start column in bytes.
    pub start_column: u32,
    /// Zero-based end column in bytes.
    pub end_column: u32,
}

/// One parser-backed definition or import fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralItem {
    /// Closed structural kind.
    pub kind: StructuralItemKind,
    /// Bounded control-safe exact name or declaration text.
    pub name: String,
    /// Exact range of the defining syntax node.
    pub range: SourceRange,
    /// SHA-256 of the exact syntax-node bytes.
    pub source_sha256: String,
}

/// One reliable relationship derived directly from a pinned syntax tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralRelationship {
    /// Closed relationship kind.
    pub kind: StructuralRelationshipKind,
    /// Exact bounded module identity supplied to the parser.
    pub source_module: String,
    /// Exact bounded control-safe import declaration.
    pub target_declaration: String,
    /// Exact range of the import declaration.
    pub range: SourceRange,
    /// SHA-256 of the exact import-declaration bytes.
    pub source_sha256: String,
}

/// Complete bounded parser result for one exact source revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralParseResult {
    /// Result schema version.
    pub schema_version: u16,
    /// Exact pinned language or dialect.
    pub language: RepositoryLanguage,
    /// Exact grammar descriptor identity.
    pub grammar_sha256: String,
    /// SHA-256 of complete source bytes.
    pub content_sha256: String,
    /// Exact source byte count.
    pub source_bytes: u64,
    /// Terminal parse state.
    pub disposition: ParseDisposition,
    /// Stable ordered parser-backed items.
    pub items: Vec<StructuralItem>,
    /// Stable reliable relationships; unresolved import targets are never invented.
    pub relationships: Vec<StructuralRelationship>,
    /// Whether the item ceiling truncated traversal.
    pub truncated: bool,
    /// SHA-256 over every preceding result field.
    pub result_sha256: String,
}

/// Content-free structural parser failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryParseError {
    /// Source is empty, oversized, or not valid UTF-8.
    SourceDenied,
    /// The pinned grammar cannot be activated.
    GrammarUnavailable,
    /// Tree-sitter returned no syntax tree.
    ParseFailed,
    /// The caller cancelled parsing before a complete result was available.
    Cancelled,
    /// The guarded parser boundary caught an internal or control-probe panic.
    ParserPanicked,
}

impl RepositoryParseError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SourceDenied => "repository.parse.source_denied",
            Self::GrammarUnavailable => "repository.parse.grammar_unavailable",
            Self::ParseFailed => "repository.parse.failed",
            Self::Cancelled => "repository.parse.cancelled",
            Self::ParserPanicked => "repository.parse.panicked",
        }
    }
}

/// Parses one exact bounded source file with the declared pinned grammar.
pub fn parse_structure(
    language_id: RepositoryLanguage,
    module_name: &str,
    source: &[u8],
) -> Result<StructuralParseResult, RepositoryParseError> {
    parse_structure_with_cancellation(language_id, module_name, source, || false)
}

/// Parses one exact source while polling a caller-owned cancellation or deadline probe.
///
/// A cancellation or panic returns no partial structure. The probe carries no authority and
/// should return `true` when the caller's existing cancellation or deadline state has fired.
pub fn parse_structure_with_cancellation<F>(
    language_id: RepositoryLanguage,
    module_name: &str,
    source: &[u8],
    mut cancellation_requested: F,
) -> Result<StructuralParseResult, RepositoryParseError>
where
    F: FnMut() -> bool,
{
    guard_parser_boundary(|| {
        parse_structure_inner(
            language_id,
            module_name,
            source,
            &mut cancellation_requested,
        )
    })
}

fn parse_structure_inner<F>(
    language_id: RepositoryLanguage,
    module_name: &str,
    source: &[u8],
    cancellation_requested: &mut F,
) -> Result<StructuralParseResult, RepositoryParseError>
where
    F: FnMut() -> bool,
{
    if source.is_empty()
        || source.len() > MAX_SOURCE_BYTES
        || module_name.is_empty()
        || module_name.len() > MAX_ITEM_TEXT_BYTES
        || module_name.chars().any(char::is_control)
        || std::str::from_utf8(source).is_err()
    {
        return Err(RepositoryParseError::SourceDenied);
    }
    if observe_cancellation(cancellation_requested)? {
        return Err(RepositoryParseError::Cancelled);
    }
    let mut parser = Parser::new();
    parser
        .set_language(&language(language_id))
        .map_err(|_| RepositoryParseError::GrammarUnavailable)?;
    let mut terminal = None;
    let mut progress =
        |_state: &tree_sitter::ParseState| match observe_cancellation(cancellation_requested) {
            Ok(false) => ControlFlow::Continue(()),
            Ok(true) => {
                terminal = Some(RepositoryParseError::Cancelled);
                ControlFlow::Break(())
            }
            Err(failure) => {
                terminal = Some(failure);
                ControlFlow::Break(())
            }
        };
    let options = ParseOptions::new().progress_callback(&mut progress);
    let mut read = |offset: usize, _position| source.get(offset..).unwrap_or_default();
    let tree = parser.parse_with_options(&mut read, None, Some(options));
    if let Some(failure) = terminal {
        return Err(failure);
    }
    let tree = tree.ok_or(RepositoryParseError::ParseFailed)?;
    let root = tree.root_node();
    let mut items = vec![StructuralItem {
        kind: StructuralItemKind::Module,
        name: module_name.to_owned(),
        range: node_range(root),
        source_sha256: sha256_hex(source),
    }];
    let mut stack = root.named_children(&mut root.walk()).collect::<Vec<_>>();
    stack.reverse();
    let mut truncated = false;
    while let Some(node) = stack.pop() {
        if observe_cancellation(cancellation_requested)? {
            return Err(RepositoryParseError::Cancelled);
        }
        if items.len() >= MAX_STRUCTURAL_ITEMS {
            truncated = true;
            break;
        }
        if let Some(item) = structural_item(language_id, node, source) {
            items.push(item);
        }
        let mut children = node.named_children(&mut node.walk()).collect::<Vec<_>>();
        children.reverse();
        stack.extend(children);
    }
    items.sort_by(|left, right| {
        left.range
            .start_byte
            .cmp(&right.range.start_byte)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
    let relationships = items
        .iter()
        .filter(|item| item.kind == StructuralItemKind::Import)
        .map(|item| StructuralRelationship {
            kind: StructuralRelationshipKind::DeclaresImport,
            source_module: module_name.to_owned(),
            target_declaration: item.name.clone(),
            range: item.range,
            source_sha256: item.source_sha256.clone(),
        })
        .collect();
    let disposition = if truncated {
        ParseDisposition::Truncated
    } else if root.has_error() {
        ParseDisposition::ParsedWithErrors
    } else {
        ParseDisposition::Parsed
    };
    let mut result = StructuralParseResult {
        schema_version: 1,
        language: language_id,
        grammar_sha256: grammar_descriptor(language_id).descriptor_sha256,
        content_sha256: sha256_hex(source),
        source_bytes: source.len() as u64,
        disposition,
        items,
        relationships,
        truncated,
        result_sha256: String::new(),
    };
    result.result_sha256 = result_digest(&result);
    Ok(result)
}

fn observe_cancellation<F>(probe: &mut F) -> Result<bool, RepositoryParseError>
where
    F: FnMut() -> bool,
{
    catch_unwind(AssertUnwindSafe(probe)).map_err(|_| RepositoryParseError::ParserPanicked)
}

fn guard_parser_boundary<T, F>(operation: F) -> Result<T, RepositoryParseError>
where
    F: FnOnce() -> Result<T, RepositoryParseError>,
{
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(Err(RepositoryParseError::ParserPanicked))
}

/// Verifies all bounds, hashes, ranges, and ordering in one parser result.
#[must_use]
pub fn verify_structural_parse_result(result: &StructuralParseResult) -> bool {
    result.schema_version == 1
        && result.source_bytes > 0
        && result.source_bytes <= MAX_SOURCE_BYTES as u64
        && result.grammar_sha256 == grammar_descriptor(result.language).descriptor_sha256
        && is_sha256(&result.content_sha256)
        && result.items.len() <= MAX_STRUCTURAL_ITEMS
        && result.truncated == (result.disposition == ParseDisposition::Truncated)
        && result.items.windows(2).all(|pair| {
            (
                pair[0].range.start_byte,
                pair[0].kind,
                pair[0].name.as_str(),
            ) <= (
                pair[1].range.start_byte,
                pair[1].kind,
                pair[1].name.as_str(),
            )
        })
        && result.items.iter().all(|item| {
            !item.name.is_empty()
                && item.name.len() <= MAX_ITEM_TEXT_BYTES
                && !item.name.chars().any(char::is_control)
                && item.range.start_byte <= item.range.end_byte
                && item.range.end_byte <= result.source_bytes
                && item.range.start_line > 0
                && item.range.end_line >= item.range.start_line
                && is_sha256(&item.source_sha256)
        })
        && result.relationships.iter().all(|relationship| {
            relationship.kind == StructuralRelationshipKind::DeclaresImport
                && !relationship.source_module.is_empty()
                && relationship.source_module.len() <= MAX_ITEM_TEXT_BYTES
                && !relationship.source_module.chars().any(char::is_control)
                && !relationship.target_declaration.is_empty()
                && relationship.target_declaration.len() <= MAX_ITEM_TEXT_BYTES
                && !relationship
                    .target_declaration
                    .chars()
                    .any(char::is_control)
                && relationship.range.start_byte <= relationship.range.end_byte
                && relationship.range.end_byte <= result.source_bytes
                && is_sha256(&relationship.source_sha256)
                && result.items.iter().any(|item| {
                    item.kind == StructuralItemKind::Import
                        && item.name == relationship.target_declaration
                        && item.range == relationship.range
                        && item.source_sha256 == relationship.source_sha256
                })
        })
        && result.relationships.len()
            == result
                .items
                .iter()
                .filter(|item| item.kind == StructuralItemKind::Import)
                .count()
        && result.result_sha256 == result_digest(result)
}

fn structural_item(
    language_id: RepositoryLanguage,
    node: Node<'_>,
    source: &[u8],
) -> Option<StructuralItem> {
    let (kind, name_node, use_whole_node) = classify_node(language_id, node)?;
    let selected = if use_whole_node { node } else { name_node? };
    let bytes = selected.utf8_text(source).ok()?.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_ITEM_TEXT_BYTES {
        return None;
    }
    Some(StructuralItem {
        kind,
        name: escape_untrusted(bytes),
        range: node_range(node),
        source_sha256: sha256_hex(&source[node.byte_range()]),
    })
}

fn classify_node(
    language_id: RepositoryLanguage,
    node: Node<'_>,
) -> Option<(StructuralItemKind, Option<Node<'_>>, bool)> {
    let kind = match (language_id, node.kind()) {
        (RepositoryLanguage::Rust, "function_item")
        | (RepositoryLanguage::Rust, "function_signature_item") => StructuralItemKind::Function,
        (RepositoryLanguage::Rust, "struct_item") => StructuralItemKind::Struct,
        (RepositoryLanguage::Rust, "enum_item") => StructuralItemKind::Enum,
        (RepositoryLanguage::Rust, "trait_item") => StructuralItemKind::Trait,
        (RepositoryLanguage::Rust, "type_item") => StructuralItemKind::TypeAlias,
        (RepositoryLanguage::Rust, "const_item" | "static_item") => StructuralItemKind::Constant,
        (RepositoryLanguage::Rust, "mod_item") => StructuralItemKind::Module,
        (RepositoryLanguage::Rust, "use_declaration") => {
            return Some((StructuralItemKind::Import, None, true));
        }
        (RepositoryLanguage::Python, "function_definition") => StructuralItemKind::Function,
        (RepositoryLanguage::Python, "class_definition") => StructuralItemKind::Class,
        (RepositoryLanguage::Python, "import_statement" | "import_from_statement") => {
            return Some((StructuralItemKind::Import, None, true));
        }
        (
            RepositoryLanguage::TypeScript
            | RepositoryLanguage::Tsx
            | RepositoryLanguage::JavaScript,
            "function_declaration" | "method_definition",
        ) => StructuralItemKind::Function,
        (
            RepositoryLanguage::TypeScript
            | RepositoryLanguage::Tsx
            | RepositoryLanguage::JavaScript,
            "class_declaration",
        ) => StructuralItemKind::Class,
        (RepositoryLanguage::TypeScript | RepositoryLanguage::Tsx, "interface_declaration") => {
            StructuralItemKind::Interface
        }
        (RepositoryLanguage::TypeScript | RepositoryLanguage::Tsx, "type_alias_declaration") => {
            StructuralItemKind::TypeAlias
        }
        (
            RepositoryLanguage::TypeScript
            | RepositoryLanguage::Tsx
            | RepositoryLanguage::JavaScript,
            "import_statement",
        ) => return Some((StructuralItemKind::Import, None, true)),
        (RepositoryLanguage::Swift, "function_declaration") => StructuralItemKind::Function,
        (RepositoryLanguage::Swift, "class_declaration") => {
            match node.child_by_field_name("declaration_kind")?.kind() {
                "class" | "actor" => StructuralItemKind::Class,
                "struct" => StructuralItemKind::Struct,
                "enum" => StructuralItemKind::Enum,
                _ => return None,
            }
        }
        (RepositoryLanguage::Swift, "protocol_declaration") => StructuralItemKind::Interface,
        (RepositoryLanguage::Swift, "typealias_declaration") => StructuralItemKind::TypeAlias,
        (RepositoryLanguage::Swift, "import_declaration") => {
            return Some((StructuralItemKind::Import, None, true));
        }
        _ => return None,
    };
    Some((kind, node.child_by_field_name("name"), false))
}

fn node_range(node: Node<'_>) -> SourceRange {
    let start = node.start_position();
    let end = node.end_position();
    SourceRange {
        start_byte: node.start_byte() as u64,
        end_byte: node.end_byte() as u64,
        start_line: start.row as u32 + 1,
        end_line: end.row as u32 + 1,
        start_column: start.column as u32,
        end_column: end.column as u32,
    }
}

fn escape_untrusted(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len());
    for byte in bytes {
        if (b' '..=b'~').contains(byte) && *byte != b'\\' {
            output.push(char::from(*byte));
        } else {
            write!(&mut output, "\\x{byte:02x}").expect("writing to String cannot fail");
        }
    }
    output
}

fn result_digest(result: &StructuralParseResult) -> String {
    serde_json::to_vec(&(
        result.schema_version,
        result.language,
        &result.grammar_sha256,
        &result.content_sha256,
        result.source_bytes,
        result.disposition,
        &result.items,
        &result.relationships,
        result.truncated,
    ))
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_else(|_| sha256_hex(b"repository-parse-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{
        ParseDisposition, RepositoryParseError, StructuralItemKind, StructuralRelationshipKind,
        guard_parser_boundary, parse_structure, parse_structure_with_cancellation,
        verify_structural_parse_result,
    };
    use crate::RepositoryLanguage;

    #[test]
    fn pinned_languages_extract_exact_parser_backed_definitions_and_imports() {
        let cases = [
            (
                RepositoryLanguage::Rust,
                "lib",
                "use std::fmt;\npub struct Item;\npub fn run() {}\n",
                vec![
                    StructuralItemKind::Import,
                    StructuralItemKind::Struct,
                    StructuralItemKind::Function,
                ],
            ),
            (
                RepositoryLanguage::Python,
                "tool",
                "import json\nclass Item:\n    pass\ndef run():\n    return 1\n",
                vec![
                    StructuralItemKind::Import,
                    StructuralItemKind::Class,
                    StructuralItemKind::Function,
                ],
            ),
            (
                RepositoryLanguage::TypeScript,
                "tool",
                "import {x} from './x';\ninterface Item {}\nfunction run() {}\n",
                vec![
                    StructuralItemKind::Import,
                    StructuralItemKind::Interface,
                    StructuralItemKind::Function,
                ],
            ),
            (
                RepositoryLanguage::Tsx,
                "view",
                "import React from 'react';\nclass View {}\nfunction App(){ return <div/>; }\n",
                vec![
                    StructuralItemKind::Import,
                    StructuralItemKind::Class,
                    StructuralItemKind::Function,
                ],
            ),
            (
                RepositoryLanguage::JavaScript,
                "app",
                "import x from './x.js';\nclass Item {}\nfunction run() {}\n",
                vec![
                    StructuralItemKind::Import,
                    StructuralItemKind::Class,
                    StructuralItemKind::Function,
                ],
            ),
            (
                RepositoryLanguage::Swift,
                "App",
                "import Foundation\nstruct Item {}\nfunc run() {}\n",
                vec![
                    StructuralItemKind::Import,
                    StructuralItemKind::Struct,
                    StructuralItemKind::Function,
                ],
            ),
        ];
        for (language, module, source, expected) in cases {
            let result = parse_structure(language, module, source.as_bytes()).expect("parse");
            assert_eq!(result.disposition, ParseDisposition::Parsed, "{language:?}");
            assert_eq!(result.items[0].kind, StructuralItemKind::Module);
            let kinds = result
                .items
                .iter()
                .skip(1)
                .map(|item| item.kind)
                .collect::<Vec<_>>();
            for kind in expected {
                assert!(
                    kinds.contains(&kind),
                    "{language:?} missing {kind:?}: {kinds:?}"
                );
            }
            assert!(result.items.iter().all(|item| item.range.start_line > 0));
            assert_eq!(result.relationships.len(), 1);
            assert_eq!(
                result.relationships[0].kind,
                StructuralRelationshipKind::DeclaresImport
            );
            assert_eq!(result.result_sha256.len(), 64);
            assert!(verify_structural_parse_result(&result));
        }
    }

    #[test]
    fn malformed_and_denied_sources_are_explicit_and_bounded() {
        let malformed = parse_structure(RepositoryLanguage::Rust, "bad", b"fn {")
            .expect("tree with syntax error");
        assert_eq!(malformed.disposition, ParseDisposition::ParsedWithErrors);
        assert!(parse_structure(RepositoryLanguage::Rust, "bad", b"").is_err());
        assert!(parse_structure(RepositoryLanguage::Rust, "bad", b"\xff").is_err());
        assert!(
            parse_structure(
                RepositoryLanguage::Rust,
                "bad",
                &vec![b'x'; 4 * 1024 * 1024 + 1]
            )
            .is_err()
        );
    }

    #[test]
    fn parser_cancellation_returns_no_partial_structure() {
        assert_eq!(
            parse_structure_with_cancellation(
                RepositoryLanguage::Rust,
                "cancelled",
                b"fn run() {}\n",
                || true,
            ),
            Err(RepositoryParseError::Cancelled)
        );

        let source = "fn bounded_work() {}\n".repeat(50_000);
        let mut observations = 0_u32;
        let failure = parse_structure_with_cancellation(
            RepositoryLanguage::Rust,
            "mid-parse",
            source.as_bytes(),
            || {
                observations += 1;
                observations >= 2
            },
        )
        .expect_err("progress cancellation must stop parsing");
        assert_eq!(failure, RepositoryParseError::Cancelled);
        assert!(observations >= 2);
    }

    #[test]
    fn parser_panics_are_content_free_failures() {
        let failure = guard_parser_boundary::<(), _>(|| panic!("synthetic parser panic"))
            .expect_err("panic must remain inside the parser boundary");
        assert_eq!(failure, RepositoryParseError::ParserPanicked);
        assert_eq!(failure.code(), "repository.parse.panicked");

        let probe_failure = parse_structure_with_cancellation(
            RepositoryLanguage::Rust,
            "probe-panic",
            b"fn run() {}\n",
            || panic!("synthetic cancellation probe panic"),
        )
        .expect_err("probe panic must remain inside the parser boundary");
        assert_eq!(probe_failure, RepositoryParseError::ParserPanicked);
    }

    #[test]
    fn forged_hashes_and_orphan_relationships_fail_verification() {
        let exact = parse_structure(
            RepositoryLanguage::Rust,
            "lib",
            b"use std::fmt;\nfn run() {}\n",
        )
        .expect("parse");
        let mut forged_hash = exact.clone();
        forged_hash.content_sha256 = "z".repeat(64);
        assert!(!verify_structural_parse_result(&forged_hash));

        let mut orphan = exact;
        orphan.relationships[0].target_declaration = "use absent::item;".to_owned();
        assert!(!verify_structural_parse_result(&orphan));
    }
}
