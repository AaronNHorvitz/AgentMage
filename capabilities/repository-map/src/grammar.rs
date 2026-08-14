//! Exact v0.1 Tree-sitter grammar inventory and path classification.

use std::fmt::Write;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tree_sitter::Language;

/// Exact supported parser language or dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryLanguage {
    /// Rust source parsed by `tree-sitter-rust`.
    Rust,
    /// Python source parsed by `tree-sitter-python`.
    Python,
    /// TypeScript source parsed by the TypeScript grammar.
    TypeScript,
    /// TSX source parsed by the separate TSX grammar.
    Tsx,
    /// JavaScript or JSX source parsed by `tree-sitter-javascript`.
    JavaScript,
    /// Swift source parsed by `tree-sitter-swift`.
    Swift,
}

impl RepositoryLanguage {
    /// Complete stable language and dialect set for the v0.1 map.
    pub const ALL: [Self; 6] = [
        Self::Rust,
        Self::Python,
        Self::TypeScript,
        Self::Tsx,
        Self::JavaScript,
        Self::Swift,
    ];

    /// Stable language identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::JavaScript => "javascript",
            Self::Swift => "swift",
        }
    }
}

/// Exact parser and grammar provenance for one admitted language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrammarDescriptor {
    /// Stable language or dialect identity.
    pub language: RepositoryLanguage,
    /// Exact grammar crate name.
    pub crate_name: String,
    /// Exact grammar crate version pinned by Cargo.
    pub crate_version: String,
    /// Upstream grammar repository.
    pub repository: String,
    /// Exact Tree-sitter runtime crate version.
    pub parser_version: String,
    /// Tree-sitter grammar ABI version observed at runtime.
    pub abi_version: u32,
    /// SHA-256 of the packaged `node-types.json` grammar metadata.
    pub node_types_sha256: String,
    /// SHA-256 over every preceding descriptor field.
    pub descriptor_sha256: String,
}

/// Returns the exact descriptor for one supported grammar.
#[must_use]
pub fn grammar_descriptor(language: RepositoryLanguage) -> GrammarDescriptor {
    let (crate_name, crate_version, repository, node_types, grammar) = grammar_material(language);
    let mut descriptor = GrammarDescriptor {
        language,
        crate_name: crate_name.to_owned(),
        crate_version: crate_version.to_owned(),
        repository: repository.to_owned(),
        parser_version: "0.26.12".to_owned(),
        abi_version: u32::try_from(grammar.abi_version())
            .expect("Tree-sitter grammar ABI fits the portable descriptor"),
        node_types_sha256: sha256_hex(node_types.as_bytes()),
        descriptor_sha256: String::new(),
    };
    descriptor.descriptor_sha256 = descriptor_digest(&descriptor);
    descriptor
}

/// Verifies one descriptor against the exact compiled grammar material.
#[must_use]
pub fn verify_grammar_descriptor(descriptor: &GrammarDescriptor) -> bool {
    descriptor == &grammar_descriptor(descriptor.language)
}

/// Returns all descriptors in stable language order.
#[must_use]
pub fn supported_grammars() -> Vec<GrammarDescriptor> {
    RepositoryLanguage::ALL
        .into_iter()
        .map(grammar_descriptor)
        .collect()
}

/// Returns a digest over the complete ordered grammar set.
#[must_use]
pub fn grammar_set_sha256() -> String {
    serde_json::to_vec(&supported_grammars())
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"grammar-set-serialization-failed"))
}

/// Detects only the exact v0.1 language set from a final path component.
#[must_use]
pub fn language_for_path(path: &str) -> Option<RepositoryLanguage> {
    let name = path.rsplit('/').next()?;
    let extension = name.rsplit_once('.')?.1;
    match extension {
        "rs" => Some(RepositoryLanguage::Rust),
        "py" | "pyi" => Some(RepositoryLanguage::Python),
        "ts" | "mts" | "cts" => Some(RepositoryLanguage::TypeScript),
        "tsx" => Some(RepositoryLanguage::Tsx),
        "js" | "mjs" | "cjs" | "jsx" => Some(RepositoryLanguage::JavaScript),
        "swift" => Some(RepositoryLanguage::Swift),
        _ => None,
    }
}

pub(crate) fn language(language: RepositoryLanguage) -> Language {
    grammar_material(language).4
}

fn grammar_material(
    language: RepositoryLanguage,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Language,
) {
    match language {
        RepositoryLanguage::Rust => (
            "tree-sitter-rust",
            "0.24.2",
            "https://github.com/tree-sitter/tree-sitter-rust",
            tree_sitter_rust::NODE_TYPES,
            tree_sitter_rust::LANGUAGE.into(),
        ),
        RepositoryLanguage::Python => (
            "tree-sitter-python",
            "0.25.0",
            "https://github.com/tree-sitter/tree-sitter-python",
            tree_sitter_python::NODE_TYPES,
            tree_sitter_python::LANGUAGE.into(),
        ),
        RepositoryLanguage::TypeScript => (
            "tree-sitter-typescript",
            "0.23.2",
            "https://github.com/tree-sitter/tree-sitter-typescript",
            tree_sitter_typescript::TYPESCRIPT_NODE_TYPES,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        ),
        RepositoryLanguage::Tsx => (
            "tree-sitter-typescript",
            "0.23.2",
            "https://github.com/tree-sitter/tree-sitter-typescript",
            tree_sitter_typescript::TSX_NODE_TYPES,
            tree_sitter_typescript::LANGUAGE_TSX.into(),
        ),
        RepositoryLanguage::JavaScript => (
            "tree-sitter-javascript",
            "0.25.0",
            "https://github.com/tree-sitter/tree-sitter-javascript",
            tree_sitter_javascript::NODE_TYPES,
            tree_sitter_javascript::LANGUAGE.into(),
        ),
        RepositoryLanguage::Swift => (
            "tree-sitter-swift",
            "0.7.3",
            "https://github.com/alex-pinkus/tree-sitter-swift",
            tree_sitter_swift::NODE_TYPES,
            tree_sitter_swift::LANGUAGE.into(),
        ),
    }
}

fn descriptor_digest(descriptor: &GrammarDescriptor) -> String {
    serde_json::to_vec(&(
        descriptor.language,
        &descriptor.crate_name,
        &descriptor.crate_version,
        &descriptor.repository,
        &descriptor.parser_version,
        descriptor.abi_version,
        &descriptor.node_types_sha256,
    ))
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_else(|_| sha256_hex(b"grammar-descriptor-serialization-failed"))
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
    use std::collections::BTreeSet;

    use super::{
        RepositoryLanguage, grammar_descriptor, grammar_set_sha256, language_for_path,
        supported_grammars, verify_grammar_descriptor,
    };

    #[test]
    fn grammar_bom_is_exact_unique_hash_bound_and_runtime_compatible() {
        let descriptors = supported_grammars();
        assert_eq!(descriptors.len(), RepositoryLanguage::ALL.len());
        assert_eq!(descriptors[0].language, RepositoryLanguage::Rust);
        assert_eq!(descriptors[5].language, RepositoryLanguage::Swift);
        let identities = descriptors
            .iter()
            .map(|descriptor| descriptor.descriptor_sha256.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(identities.len(), descriptors.len());
        for descriptor in descriptors {
            assert!(verify_grammar_descriptor(&descriptor));
            assert_eq!(descriptor, grammar_descriptor(descriptor.language));
            assert_eq!(descriptor.parser_version, "0.26.12");
            assert_eq!(descriptor.descriptor_sha256.len(), 64);
            assert_eq!(descriptor.node_types_sha256.len(), 64);
            assert!((13..=15).contains(&descriptor.abi_version));
        }
        assert_eq!(grammar_set_sha256().len(), 64);

        let mut forged = grammar_descriptor(RepositoryLanguage::Rust);
        forged.repository.push_str("/forged");
        assert!(!verify_grammar_descriptor(&forged));
    }

    #[test]
    fn path_detection_is_closed_and_unsupported_languages_remain_visible() {
        let cases = [
            ("src/lib.rs", Some(RepositoryLanguage::Rust)),
            ("tool.py", Some(RepositoryLanguage::Python)),
            ("ui.ts", Some(RepositoryLanguage::TypeScript)),
            ("ui.tsx", Some(RepositoryLanguage::Tsx)),
            ("app.mjs", Some(RepositoryLanguage::JavaScript)),
            ("App.swift", Some(RepositoryLanguage::Swift)),
            ("README.md", None),
            ("script.sh", None),
            ("source.RS", None),
            ("Makefile", None),
        ];
        for (path, expected) in cases {
            assert_eq!(language_for_path(path), expected, "{path}");
        }
    }
}
