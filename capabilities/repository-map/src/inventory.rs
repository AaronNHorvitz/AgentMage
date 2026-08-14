//! Deterministic Git-aware and policy-aware repository inventory construction.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    RepositoryLanguage, StructuralParseResult, grammar_set_sha256, language_for_path,
    parse_structure, verify_structural_parse_result,
};

const MAX_REPOSITORY_FILES: usize = 100_000;
const MAX_REPOSITORY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PARSE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_BRANCH_BYTES: usize = 255;

/// Exact Git tracking state supplied by bounded Git evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitTrackedState {
    /// Tracked and unchanged relative to the index.
    TrackedClean,
    /// Tracked with a worktree or index change.
    TrackedChanged,
    /// Not tracked by Git and not ignored.
    Untracked,
    /// Ignored by applicable Git ignore rules.
    Ignored,
}

/// Visible terminal disposition for one discovered repository path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryEntryDisposition {
    /// Parsed with one pinned grammar.
    Parsed,
    /// Parsed with a pinned grammar that reported syntax errors.
    ParsedWithErrors,
    /// Parsed with a pinned grammar until the structural-item ceiling was reached.
    Truncated,
    /// A supported source path was discovered but its bytes were not authorized for reading.
    ContentNotRead,
    /// Inventoried but no pinned grammar supports the language.
    UnsupportedLanguage,
    /// Inventoried binary content; bytes are not sent to a parser.
    BinaryInventoryOnly,
    /// File exceeds the parser limit but remains in inventory.
    ParseLimitExceeded,
    /// Git ignored the path, so content was not admitted.
    GitIgnored,
    /// Product policy excluded the path, so content was not admitted.
    PolicyExcluded,
    /// Generated content was excluded from v0.1.
    GeneratedExcluded,
    /// Vendored content was excluded from v0.1.
    VendoredExcluded,
    /// An admitted parser failed before returning a tree.
    ParseFailed,
}

/// Complete visible coverage and fixed budgets for one repository map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryCoverage {
    /// Files discovered before exclusions and parser selection.
    pub discovered_files: u64,
    /// Files whose bytes were supplied through the authorized projection.
    pub read_files: u64,
    /// Files for which a pinned parser returned a syntax tree.
    pub parsed_files: u64,
    /// Files lexically searched; zero during base-map construction.
    pub searched_files: u64,
    /// Files skipped because content was unread, binary, or over the parse limit.
    pub skipped_files: u64,
    /// Files excluded by Git or product policy.
    pub excluded_files: u64,
    /// Files with no pinned grammar.
    pub unsupported_files: u64,
    /// Files whose admitted parser failed before returning a tree.
    pub failed_files: u64,
    /// Files whose structural traversal reached its fixed item ceiling.
    pub truncated_files: u64,
    /// Files not fully structurally understood.
    pub uncertain_files: u64,
    /// Sum of discovered file sizes.
    pub discovered_bytes: u64,
    /// Fixed repository file-count ceiling.
    pub repository_file_budget: u64,
    /// Fixed repository byte ceiling.
    pub repository_byte_budget: u64,
    /// Fixed per-file inventory byte ceiling.
    pub file_byte_budget: u64,
    /// Fixed per-file parser byte ceiling.
    pub parse_byte_budget: u64,
    /// Fixed structural-item ceiling per parsed file.
    pub structural_item_budget: u64,
}

/// Immutable metadata and optional authorized bytes for one discovered file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryFileInput {
    /// Canonical path components relative to the approved workspace.
    pub path: Vec<String>,
    /// Exact file byte count from held-object metadata.
    pub size_bytes: u64,
    /// Exact lowercase SHA-256 from held-object evidence.
    pub content_sha256: String,
    /// Optional bytes supplied only after bounded read authorization.
    pub content: Option<Vec<u8>>,
    /// Exact Git tracking state.
    pub git_state: GitTrackedState,
    /// Product policy excluded this path.
    pub policy_excluded: bool,
    /// Deterministic product policy classified this path as generated.
    pub generated: bool,
    /// Deterministic product policy classified this path as vendored.
    pub vendored: bool,
}

/// Exact repository and policy context for one map construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryMapInput {
    /// Approved workspace identity.
    pub workspace_id: WorkspaceId,
    /// SHA-256 of stable repository identity, never a remote URL.
    pub repository_sha256: String,
    /// SHA-256 of exact held worktree identity.
    pub worktree_sha256: String,
    /// Current exact branch, or `None` for detached HEAD.
    pub branch: Option<String>,
    /// Current exact 40- or 64-character Git commit object identity.
    pub commit_id: String,
    /// Exact policy revision used for exclusions and classification.
    pub policy_sha256: String,
    /// Host freshness token for the complete input projection.
    pub freshness_sha256: String,
    /// All discovered file metadata in any order.
    pub files: Vec<RepositoryFileInput>,
}

/// One stable repository file inventory and optional parser result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryFileRecord {
    /// Canonical workspace-relative path.
    pub path: WorkspacePath,
    /// Exact Git tracking state.
    pub git_state: GitTrackedState,
    /// Detected pinned language, or no supported grammar.
    pub language: Option<RepositoryLanguage>,
    /// Exact file byte count.
    pub size_bytes: u64,
    /// Exact file content digest.
    pub content_sha256: String,
    /// Whether authorized bytes were supplied to this pure capability.
    pub content_read: bool,
    /// Stable repository identity digest, never a remote URL.
    pub repository_sha256: String,
    /// Exact held worktree identity digest.
    pub worktree_sha256: String,
    /// Exact policy revision used to classify the path.
    pub policy_sha256: String,
    /// Visible terminal inventory or parse state.
    pub disposition: RepositoryEntryDisposition,
    /// Pinned parser facts only when bytes were admitted and parsing returned.
    pub structure: Option<StructuralParseResult>,
    /// Exact branch, or no branch for detached HEAD.
    pub branch: Option<String>,
    /// Exact commit object identity.
    pub commit_id: String,
    /// SHA-256 over every preceding record field.
    pub record_sha256: String,
}

/// Complete deterministic structural repository map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryMap {
    /// Map schema version.
    pub schema_version: u16,
    /// Exact workspace identity.
    pub workspace_id: WorkspaceId,
    /// Stable repository identity digest.
    pub repository_sha256: String,
    /// Exact held worktree identity digest.
    pub worktree_sha256: String,
    /// Exact branch, or detached state.
    pub branch: Option<String>,
    /// Exact current commit object identity.
    pub commit_id: String,
    /// Complete pinned grammar-set identity.
    pub grammar_set_sha256: String,
    /// Exact policy revision.
    pub policy_sha256: String,
    /// Host freshness token for the input projection.
    pub freshness_sha256: String,
    /// Stable path-ordered inventory including all visible exclusions.
    pub files: Vec<RepositoryFileRecord>,
    /// Complete visible base-map coverage and budget ledger.
    pub coverage: RepositoryCoverage,
    /// Total discovered files.
    pub discovered_files: u64,
    /// Total files parsed with a pinned grammar.
    pub parsed_files: u64,
    /// Total files represented without parser facts.
    pub inventory_only_files: u64,
    /// Total excluded or ignored files.
    pub excluded_files: u64,
    /// SHA-256 over every preceding map field.
    pub map_sha256: String,
}

/// Content-free repository map construction failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryMapError {
    /// Repository identity, policy, branch, or freshness is malformed.
    InvalidContext,
    /// File collection, bytes, or path exceeds a fixed bound.
    LimitExceeded,
    /// A file path or identity is malformed or duplicated.
    InvalidFile,
}

impl RepositoryMapError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidContext => "repository.map.context_invalid",
            Self::LimitExceeded => "repository.map.limit_exceeded",
            Self::InvalidFile => "repository.map.file_invalid",
        }
    }
}

/// Builds one deterministic map from already bounded Git, policy, and held-file evidence.
pub fn build_repository_map(
    mut input: RepositoryMapInput,
) -> Result<RepositoryMap, RepositoryMapError> {
    if !is_sha256(&input.repository_sha256)
        || !is_sha256(&input.worktree_sha256)
        || !is_sha256(&input.policy_sha256)
        || !is_sha256(&input.freshness_sha256)
        || !valid_commit(&input.commit_id)
        || input.branch.as_deref().is_some_and(|branch| {
            branch.is_empty()
                || branch.len() > MAX_BRANCH_BYTES
                || branch.starts_with('-')
                || branch.chars().any(char::is_control)
        })
    {
        return Err(RepositoryMapError::InvalidContext);
    }
    if input.files.len() > MAX_REPOSITORY_FILES
        || input
            .files
            .iter()
            .try_fold(0_u64, |total, file| total.checked_add(file.size_bytes))
            .is_none_or(|total| total > MAX_REPOSITORY_BYTES)
    {
        return Err(RepositoryMapError::LimitExceeded);
    }
    input
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
    let mut paths = BTreeSet::new();
    let mut records = Vec::with_capacity(input.files.len());
    for file in input.files {
        let path = WorkspacePath::new(input.workspace_id.clone(), file.path.clone())
            .map_err(|_| RepositoryMapError::InvalidFile)?;
        if !paths.insert(path.clone())
            || file.size_bytes > MAX_FILE_BYTES
            || !is_sha256(&file.content_sha256)
            || file.content.as_ref().is_some_and(|content| {
                content.len() as u64 != file.size_bytes
                    || sha256_hex(content) != file.content_sha256
            })
            || (file.policy_excluded
                || file.generated
                || file.vendored
                || file.git_state == GitTrackedState::Ignored)
                && file.content.is_some()
        {
            return Err(RepositoryMapError::InvalidFile);
        }
        records.push(build_file_record(
            path,
            file,
            input.repository_sha256.clone(),
            input.worktree_sha256.clone(),
            input.policy_sha256.clone(),
            input.branch.clone(),
            input.commit_id.clone(),
        ));
    }
    let parsed_files = records
        .iter()
        .filter(|record| {
            matches!(
                record.disposition,
                RepositoryEntryDisposition::Parsed
                    | RepositoryEntryDisposition::ParsedWithErrors
                    | RepositoryEntryDisposition::Truncated
            )
        })
        .count() as u64;
    let excluded_files = records
        .iter()
        .filter(|record| {
            matches!(
                record.disposition,
                RepositoryEntryDisposition::GitIgnored
                    | RepositoryEntryDisposition::PolicyExcluded
                    | RepositoryEntryDisposition::GeneratedExcluded
                    | RepositoryEntryDisposition::VendoredExcluded
            )
        })
        .count() as u64;
    let coverage = coverage_for_records(&records);
    let mut map = RepositoryMap {
        schema_version: 1,
        workspace_id: input.workspace_id,
        repository_sha256: input.repository_sha256,
        worktree_sha256: input.worktree_sha256,
        branch: input.branch,
        commit_id: input.commit_id,
        grammar_set_sha256: grammar_set_sha256(),
        policy_sha256: input.policy_sha256,
        freshness_sha256: input.freshness_sha256,
        discovered_files: records.len() as u64,
        parsed_files,
        inventory_only_files: records.len() as u64 - parsed_files - excluded_files,
        excluded_files,
        coverage,
        files: records,
        map_sha256: String::new(),
    };
    map.map_sha256 = map_digest(&map);
    Ok(map)
}

/// Verifies one complete file record without trusting its retained digest.
#[must_use]
pub fn verify_repository_file_record(record: &RepositoryFileRecord) -> bool {
    is_sha256(&record.content_sha256)
        && is_sha256(&record.repository_sha256)
        && is_sha256(&record.worktree_sha256)
        && is_sha256(&record.policy_sha256)
        && valid_commit(&record.commit_id)
        && record.branch.as_deref().is_none_or(|branch| {
            !branch.is_empty()
                && branch.len() <= MAX_BRANCH_BYTES
                && !branch.starts_with('-')
                && !branch.chars().any(char::is_control)
        })
        && record.structure.as_ref().is_none_or(|structure| {
            verify_structural_parse_result(structure)
                && Some(structure.language) == record.language
                && structure.content_sha256 == record.content_sha256
        })
        && matches!(
            (
                record.disposition,
                record.structure.is_some(),
                record.content_read
            ),
            (RepositoryEntryDisposition::Parsed, true, true)
                | (RepositoryEntryDisposition::ParsedWithErrors, true, true)
                | (RepositoryEntryDisposition::Truncated, true, true)
                | (RepositoryEntryDisposition::ContentNotRead, false, false)
                | (
                    RepositoryEntryDisposition::UnsupportedLanguage,
                    false,
                    true | false
                )
                | (RepositoryEntryDisposition::BinaryInventoryOnly, false, true)
                | (RepositoryEntryDisposition::ParseLimitExceeded, false, true)
                | (RepositoryEntryDisposition::GitIgnored, false, false)
                | (RepositoryEntryDisposition::PolicyExcluded, false, false)
                | (RepositoryEntryDisposition::GeneratedExcluded, false, false)
                | (RepositoryEntryDisposition::VendoredExcluded, false, false)
                | (RepositoryEntryDisposition::ParseFailed, false, true)
        )
        && record.record_sha256 == file_digest(record)
}

/// Verifies all nested records, counts, identities, ordering, and complete map hash.
#[must_use]
pub fn verify_repository_map(map: &RepositoryMap) -> bool {
    let parsed = map
        .files
        .iter()
        .filter(|record| {
            matches!(
                record.disposition,
                RepositoryEntryDisposition::Parsed
                    | RepositoryEntryDisposition::ParsedWithErrors
                    | RepositoryEntryDisposition::Truncated
            )
        })
        .count() as u64;
    let excluded = map
        .files
        .iter()
        .filter(|record| {
            matches!(
                record.disposition,
                RepositoryEntryDisposition::GitIgnored
                    | RepositoryEntryDisposition::PolicyExcluded
                    | RepositoryEntryDisposition::GeneratedExcluded
                    | RepositoryEntryDisposition::VendoredExcluded
            )
        })
        .count() as u64;
    let coverage = coverage_for_records(&map.files);
    map.schema_version == 1
        && is_sha256(&map.repository_sha256)
        && is_sha256(&map.worktree_sha256)
        && is_sha256(&map.policy_sha256)
        && is_sha256(&map.freshness_sha256)
        && map.grammar_set_sha256 == grammar_set_sha256()
        && valid_commit(&map.commit_id)
        && map.files.len() <= MAX_REPOSITORY_FILES
        && map.files.windows(2).all(|pair| pair[0].path < pair[1].path)
        && map.files.iter().all(|record| {
            record.path.workspace_id() == &map.workspace_id
                && record.repository_sha256 == map.repository_sha256
                && record.worktree_sha256 == map.worktree_sha256
                && record.policy_sha256 == map.policy_sha256
                && record.branch == map.branch
                && record.commit_id == map.commit_id
                && verify_repository_file_record(record)
        })
        && map.discovered_files == map.files.len() as u64
        && map.parsed_files == parsed
        && map.excluded_files == excluded
        && map.inventory_only_files == map.discovered_files - parsed - excluded
        && map.coverage == coverage
        && map.map_sha256 == map_digest(map)
}

fn build_file_record(
    path: WorkspacePath,
    file: RepositoryFileInput,
    repository_sha256: String,
    worktree_sha256: String,
    policy_sha256: String,
    branch: Option<String>,
    commit_id: String,
) -> RepositoryFileRecord {
    let path_text = path
        .components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/");
    let language = language_for_path(&path_text);
    let content_read = file.content.is_some();
    let (disposition, structure) = if file.policy_excluded {
        (RepositoryEntryDisposition::PolicyExcluded, None)
    } else if file.generated {
        (RepositoryEntryDisposition::GeneratedExcluded, None)
    } else if file.vendored {
        (RepositoryEntryDisposition::VendoredExcluded, None)
    } else if file.git_state == GitTrackedState::Ignored {
        (RepositoryEntryDisposition::GitIgnored, None)
    } else {
        match (file.content.as_deref(), language) {
            (Some(_), Some(_)) if file.size_bytes > MAX_PARSE_BYTES => {
                (RepositoryEntryDisposition::ParseLimitExceeded, None)
            }
            (Some(content), Some(_)) if content.contains(&0) => {
                (RepositoryEntryDisposition::BinaryInventoryOnly, None)
            }
            (Some(content), Some(language_id)) => match parse_structure(
                language_id,
                path.components()
                    .last()
                    .expect("workspace path is non-empty")
                    .as_str(),
                content,
            ) {
                Ok(structure) => {
                    let disposition = match structure.disposition {
                        crate::ParseDisposition::Parsed => RepositoryEntryDisposition::Parsed,
                        crate::ParseDisposition::ParsedWithErrors => {
                            RepositoryEntryDisposition::ParsedWithErrors
                        }
                        crate::ParseDisposition::Truncated => RepositoryEntryDisposition::Truncated,
                    };
                    (disposition, Some(structure))
                }
                Err(_) => (RepositoryEntryDisposition::ParseFailed, None),
            },
            (None, Some(_)) => (RepositoryEntryDisposition::ContentNotRead, None),
            (_, None) => (RepositoryEntryDisposition::UnsupportedLanguage, None),
        }
    };
    let mut record = RepositoryFileRecord {
        path,
        git_state: file.git_state,
        language,
        size_bytes: file.size_bytes,
        content_sha256: file.content_sha256,
        content_read,
        repository_sha256,
        worktree_sha256,
        policy_sha256,
        disposition,
        structure,
        branch,
        commit_id,
        record_sha256: String::new(),
    };
    record.record_sha256 = file_digest(&record);
    record
}

fn file_digest(record: &RepositoryFileRecord) -> String {
    digest(&(
        &record.path,
        record.git_state,
        record.language,
        record.size_bytes,
        &record.content_sha256,
        record.content_read,
        &record.repository_sha256,
        &record.worktree_sha256,
        &record.policy_sha256,
        record.disposition,
        &record.structure,
        &record.branch,
        &record.commit_id,
    ))
}

fn map_digest(map: &RepositoryMap) -> String {
    digest(&(
        map.schema_version,
        &map.workspace_id,
        &map.repository_sha256,
        &map.worktree_sha256,
        &map.branch,
        &map.commit_id,
        &map.grammar_set_sha256,
        &map.policy_sha256,
        &map.freshness_sha256,
        &map.files,
        &map.coverage,
        map.discovered_files,
        map.parsed_files,
        map.inventory_only_files,
        map.excluded_files,
    ))
}

fn coverage_for_records(records: &[RepositoryFileRecord]) -> RepositoryCoverage {
    let count = |predicate: fn(&RepositoryFileRecord) -> bool| {
        records.iter().filter(|record| predicate(record)).count() as u64
    };
    RepositoryCoverage {
        discovered_files: records.len() as u64,
        read_files: count(|record| record.content_read),
        parsed_files: count(|record| record.structure.is_some()),
        searched_files: 0,
        skipped_files: count(|record| {
            matches!(
                record.disposition,
                RepositoryEntryDisposition::ContentNotRead
                    | RepositoryEntryDisposition::BinaryInventoryOnly
                    | RepositoryEntryDisposition::ParseLimitExceeded
            )
        }),
        excluded_files: count(|record| {
            matches!(
                record.disposition,
                RepositoryEntryDisposition::GitIgnored
                    | RepositoryEntryDisposition::PolicyExcluded
                    | RepositoryEntryDisposition::GeneratedExcluded
                    | RepositoryEntryDisposition::VendoredExcluded
            )
        }),
        unsupported_files: count(|record| {
            record.disposition == RepositoryEntryDisposition::UnsupportedLanguage
        }),
        failed_files: count(|record| record.disposition == RepositoryEntryDisposition::ParseFailed),
        truncated_files: count(|record| {
            record.disposition == RepositoryEntryDisposition::Truncated
        }),
        uncertain_files: count(|record| {
            !matches!(
                record.disposition,
                RepositoryEntryDisposition::Parsed
                    | RepositoryEntryDisposition::GitIgnored
                    | RepositoryEntryDisposition::PolicyExcluded
                    | RepositoryEntryDisposition::GeneratedExcluded
                    | RepositoryEntryDisposition::VendoredExcluded
            )
        }),
        discovered_bytes: records.iter().map(|record| record.size_bytes).sum(),
        repository_file_budget: MAX_REPOSITORY_FILES as u64,
        repository_byte_budget: MAX_REPOSITORY_BYTES,
        file_byte_budget: MAX_FILE_BYTES,
        parse_byte_budget: MAX_PARSE_BYTES,
        structural_item_budget: 10_000,
    }
}

fn digest<T: Serialize>(value: &T) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"repository-map-serialization-failed"))
}

fn valid_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

    use super::{
        GitTrackedState, RepositoryEntryDisposition, RepositoryFileInput, RepositoryMapInput,
        build_repository_map, sha256_hex, verify_repository_map,
    };

    fn file(path: &[&str], content: Option<&[u8]>) -> RepositoryFileInput {
        let bytes = content.unwrap_or(b"metadata-only");
        RepositoryFileInput {
            path: path.iter().map(|part| (*part).to_owned()).collect(),
            size_bytes: bytes.len() as u64,
            content_sha256: sha256_hex(bytes),
            content: content.map(<[u8]>::to_vec),
            git_state: GitTrackedState::TrackedClean,
            policy_excluded: false,
            generated: false,
            vendored: false,
        }
    }

    fn input(files: Vec<RepositoryFileInput>) -> RepositoryMapInput {
        RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-map"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("main".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files,
        }
    }

    #[test]
    fn inventory_is_stable_git_aware_policy_aware_and_parser_backed() {
        let mut ignored = file(&["ignored.rs"], None);
        ignored.git_state = GitTrackedState::Ignored;
        let mut generated = file(&["dist", "bundle.js"], None);
        generated.generated = true;
        let mut vendored = file(&["vendor", "lib.py"], None);
        vendored.vendored = true;
        let mut excluded = file(&["private", "secret.rs"], None);
        excluded.policy_excluded = true;
        let files = vec![
            file(
                &["src", "lib.rs"],
                Some(b"pub struct Item;\npub fn run() {}\n"),
            ),
            file(&["README.md"], Some(b"# Visible inventory\n")),
            ignored,
            generated,
            vendored,
            excluded,
        ];
        let first = build_repository_map(input(files.clone())).expect("map");
        let mut reversed = files;
        reversed.reverse();
        let second = build_repository_map(input(reversed)).expect("map");
        assert_eq!(first, second);
        assert_eq!(first.discovered_files, 6);
        assert_eq!(first.parsed_files, 1);
        assert_eq!(first.inventory_only_files, 1);
        assert_eq!(first.excluded_files, 4);
        assert_eq!(
            first.files[0].disposition,
            RepositoryEntryDisposition::UnsupportedLanguage
        );
        assert!(
            first
                .files
                .iter()
                .all(|record| record.branch.as_deref() == Some("main"))
        );
        assert_eq!(first.map_sha256.len(), 64);
    }

    #[test]
    fn duplicate_traversal_forged_hash_context_and_limits_fail_closed() {
        let exact = file(&["src", "lib.rs"], Some(b"fn run() {}\n"));
        assert!(build_repository_map(input(vec![exact.clone(), exact])).is_err());
        assert!(build_repository_map(input(vec![file(&["..", "secret"], None)])).is_err());
        let mut forged = file(&["src", "lib.rs"], Some(b"fn run() {}\n"));
        forged.content_sha256 = "f".repeat(64);
        assert!(build_repository_map(input(vec![forged])).is_err());
        let mut bad_context = input(Vec::new());
        bad_context.repository_sha256 = "bad".to_owned();
        assert!(build_repository_map(bad_context).is_err());
        let mut oversized = file(&["large.rs"], None);
        oversized.size_bytes = 16 * 1024 * 1024 + 1;
        assert!(build_repository_map(input(vec![oversized])).is_err());

        let mut excluded_with_content = file(&["private", "secret.rs"], Some(b"secret"));
        excluded_with_content.policy_excluded = true;
        assert!(build_repository_map(input(vec![excluded_with_content])).is_err());

        let not_read = build_repository_map(input(vec![file(&["src", "held.rs"], None)]))
            .expect("metadata-only map");
        assert_eq!(
            not_read.files[0].disposition,
            RepositoryEntryDisposition::ContentNotRead
        );
    }

    #[test]
    fn coverage_ledger_exposes_every_base_map_outcome_and_fixed_budget() {
        let empty = build_repository_map(input(Vec::new())).expect("empty map");
        assert_eq!(empty.coverage.discovered_files, 0);
        assert!(verify_repository_map(&empty));

        let mut ignored = file(&["ignored.rs"], None);
        ignored.git_state = GitTrackedState::Ignored;
        let mut generated = file(&["dist", "bundle.js"], None);
        generated.generated = true;
        let mut vendored = file(&["vendor", "lib.py"], None);
        vendored.vendored = true;
        let mut excluded = file(&["private", "secret.rs"], None);
        excluded.policy_excluded = true;
        let oversized = vec![b'a'; 4 * 1024 * 1024 + 1];
        let mut many = String::new();
        for index in 0..10_050 {
            use std::fmt::Write;
            writeln!(&mut many, "fn item_{index}() {{}}").expect("fixture");
        }
        let files = vec![
            file(&["src", "lib.rs"], Some(b"fn run() {}\n")),
            file(&["src", "malformed.rs"], Some(b"fn {")),
            file(&["src", "truncated.rs"], Some(many.as_bytes())),
            file(&["src", "binary.rs"], Some(b"fn run() {}\0")),
            file(&["legacy", "tool.rb"], Some(b"puts 'visible'\n")),
            file(&["src", "not-read.rs"], None),
            file(&["src", "invalid.rs"], Some(b"\xff")),
            file(&["src", "oversized.rs"], Some(&oversized)),
            ignored,
            generated,
            vendored,
            excluded,
        ];
        let first = build_repository_map(input(files.clone())).expect("coverage map");
        let second = build_repository_map(input(files)).expect("coverage map");
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert!(verify_repository_map(&first));
        assert_eq!(
            first.map_sha256,
            "296fd3fb3768778f17af5bd6d00879cd922c5c74cbc9a467cc0c290db7bc8106"
        );
        assert_eq!(first.coverage.discovered_files, 12);
        assert_eq!(first.coverage.read_files, 7);
        assert_eq!(first.coverage.parsed_files, 3);
        assert_eq!(first.coverage.searched_files, 0);
        assert_eq!(first.coverage.skipped_files, 3);
        assert_eq!(first.coverage.excluded_files, 4);
        assert_eq!(first.coverage.unsupported_files, 1);
        assert_eq!(first.coverage.failed_files, 1);
        assert_eq!(first.coverage.truncated_files, 1);
        assert_eq!(first.coverage.uncertain_files, 7);
        assert_eq!(first.coverage.repository_file_budget, 100_000);
        assert_eq!(first.coverage.repository_byte_budget, 256 * 1024 * 1024);
        assert_eq!(first.coverage.file_byte_budget, 16 * 1024 * 1024);
        assert_eq!(first.coverage.parse_byte_budget, 4 * 1024 * 1024);
        assert_eq!(first.coverage.structural_item_budget, 10_000);
        assert!(
            first
                .files
                .iter()
                .any(|record| { record.disposition == RepositoryEntryDisposition::Truncated })
        );
    }
}
