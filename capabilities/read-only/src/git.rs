//! Closed command-equivalent plans and bounded evidence for read-only Git inspection.

use std::collections::BTreeMap;
use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel, WorkspaceId, WorkspacePath,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_GIT_RECORDS: u32 = 1_000;
const MAX_GIT_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_GIT_PATHS: usize = 256;
const MAX_GIT_REVISION_BYTES: usize = 256;

/// Stable native tool identity for bounded read-only Git inspection.
pub const GIT_INSPECTION_TOOL_ID: &str = "agentmage.git.inspect";

/// Immutable contract version for bounded read-only Git inspection.
pub const GIT_INSPECTION_TOOL_VERSION: &str = "1.0.0";

/// Closed input-schema identity for bounded read-only Git inspection.
pub const GIT_INSPECTION_INPUT_SCHEMA_ID: &str = "agentmage.git.inspect.input";

/// Closed output-schema identity for bounded read-only Git inspection.
pub const GIT_INSPECTION_OUTPUT_SCHEMA_ID: &str = "agentmage.git.inspect.output";

/// Canonical closed JSON Schema for one bounded Git inspection request.
pub const GIT_INSPECTION_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.git.inspect.input","type":"object","additionalProperties":false,"required":["schema_version","operation","revision","object_id","pathspecs","max_records","max_output_bytes"],"properties":{"schema_version":{"const":1},"operation":{"enum":["status","current_branch","upstream","branch_list","log","diff","staged_diff","show","worktree_list","object","ref","dirty_tree","untracked_files"]},"revision":{"type":["string","null"],"maxLength":256},"object_id":{"type":["string","null"],"pattern":"^(?:[0-9A-Fa-f]{40}|[0-9A-Fa-f]{64})$"},"pathspecs":{"type":"array","maxItems":256,"items":{"type":"array","minItems":1,"maxItems":256,"items":{"type":"string","minLength":1,"maxLength":255}}},"max_records":{"type":"integer","minimum":1,"maximum":1000},"max_output_bytes":{"type":"integer","minimum":1,"maximum":4194304}}}"#;

/// Canonical closed JSON Schema for one bounded Git inspection result.
pub const GIT_INSPECTION_OUTPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.git.inspect.output","type":"object","additionalProperties":false,"required":["schema_version","operation","repository_sha256","worktree_sha256","revision","outcome","records","observed_bytes","truncated","dirty","freshness_sha256","result_sha256"],"properties":{"schema_version":{"const":1},"operation":{"enum":["status","current_branch","upstream","branch_list","log","diff","staged_diff","show","worktree_list","object","ref","dirty_tree","untracked_files"]},"repository_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"worktree_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"revision":{"type":["string","null"]},"outcome":{"enum":["succeeded","no_result","truncated","failed"]},"records":{"type":"array","maxItems":1000,"items":{"type":"object","additionalProperties":false,"required":["sequence","record_kind","escaped_text","raw_sha256"],"properties":{"sequence":{"type":"integer","minimum":1},"record_kind":{"type":"string"},"escaped_text":{"type":"string"},"raw_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}},"observed_bytes":{"type":"integer","minimum":0,"maximum":4194304},"truncated":{"type":"boolean"},"dirty":{"type":["boolean","null"]},"freshness_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"result_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;

/// Returns the declarative definition for bounded read-only Git inspection.
#[must_use]
pub fn git_inspection_tool_definition() -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(GIT_INSPECTION_TOOL_ID),
        tool_version: GIT_INSPECTION_TOOL_VERSION.to_owned(),
        display_name: "Inspect Git repository".to_owned(),
        description: "Runs one fixed bounded read-only Git inspection over an exact held worktree"
            .to_owned(),
        input_schema: git_schema(
            GIT_INSPECTION_INPUT_SCHEMA_ID,
            GIT_INSPECTION_INPUT_SCHEMA_JSON.as_bytes(),
        ),
        output_schema: git_schema(
            GIT_INSPECTION_OUTPUT_SCHEMA_ID,
            GIT_INSPECTION_OUTPUT_SCHEMA_JSON.as_bytes(),
        ),
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "one-exact-held-repository-worktree".to_owned(),
            single_use: true,
        },
        timeout_ms: 15_000,
    }
}

/// Fixed read-only Git operation identities admitted by the initial pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitInspectionOperation {
    /// Porcelain-v2 status including branch and untracked state.
    Status,
    /// Current symbolic branch or an explicit detached result.
    CurrentBranch,
    /// Configured upstream of the current branch.
    Upstream,
    /// Local and remote branch references.
    BranchList,
    /// Bounded first-parent-independent commit log.
    Log,
    /// Worktree-versus-index diff.
    Diff,
    /// Index-versus-HEAD diff.
    StagedDiff,
    /// One exact commit or object presentation.
    Show,
    /// Registered worktree list.
    WorktreeList,
    /// Exact object type.
    Object,
    /// Exact ref resolution.
    Ref,
    /// Dirty-tree determination through porcelain status.
    DirtyTree,
    /// Untracked path inspection.
    UntrackedFiles,
}

impl GitInspectionOperation {
    /// Complete stable operation inventory.
    pub const ALL: [Self; 13] = [
        Self::Status,
        Self::CurrentBranch,
        Self::Upstream,
        Self::BranchList,
        Self::Log,
        Self::Diff,
        Self::StagedDiff,
        Self::Show,
        Self::WorktreeList,
        Self::Object,
        Self::Ref,
        Self::DirtyTree,
        Self::UntrackedFiles,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::CurrentBranch => "current_branch",
            Self::Upstream => "upstream",
            Self::BranchList => "branch_list",
            Self::Log => "log",
            Self::Diff => "diff",
            Self::StagedDiff => "staged_diff",
            Self::Show => "show",
            Self::WorktreeList => "worktree_list",
            Self::Object => "object",
            Self::Ref => "ref",
            Self::DirtyTree => "dirty_tree",
            Self::UntrackedFiles => "untracked_files",
        }
    }
}

/// Closed request for one fixed Git inspection operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitInspectionRequest {
    /// Request schema version. Only one is accepted.
    pub schema_version: u16,
    /// Exact fixed operation.
    pub operation: GitInspectionOperation,
    /// Optional exact revision or ref for operations that require one.
    pub revision: Option<String>,
    /// Optional exact hexadecimal object identity.
    pub object_id: Option<String>,
    /// Canonical workspace-relative pathspec component lists.
    pub pathspecs: Vec<Vec<String>>,
    /// Maximum parsed records.
    pub max_records: u32,
    /// Maximum worker output bytes.
    pub max_output_bytes: u64,
}

/// Fixed executable arguments and environment for one sandboxed Git worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitCommandPlan {
    /// Exact argument vector; element zero is always `git`.
    pub argv: Vec<String>,
    /// Complete cleared-environment replacement.
    pub environment: BTreeMap<String, String>,
    /// Exact standard-input bytes, used only by object inspection.
    pub stdin: Vec<u8>,
}

/// Stable content-free planning or parsing refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitInspectionError {
    /// Request fields or operation-specific shape are invalid.
    InvalidRequest,
    /// Worker output exceeded the accepted absolute representation bound.
    OutputDenied,
}

impl GitInspectionError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "git.inspection.request.invalid",
            Self::OutputDenied => "git.inspection.output.denied",
        }
    }
}

/// Terminal inspection outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitInspectionOutcome {
    /// The complete bounded output was parsed.
    Succeeded,
    /// The valid inspection found no records.
    NoResult,
    /// A declared record or byte bound truncated otherwise valid output.
    Truncated,
    /// The fixed Git worker returned a non-success status.
    Failed,
}

/// One bounded untrusted Git output record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitRecord {
    /// One-based deterministic output sequence.
    pub sequence: u32,
    /// Stable operation-specific record class.
    pub record_kind: String,
    /// Control-safe rendering; untrusted bytes never become terminal controls.
    pub escaped_text: String,
    /// SHA-256 of the exact original record bytes.
    pub raw_sha256: String,
}

/// Hash-bound typed result from one fixed Git inspection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitInspectionResult {
    /// Result schema version.
    pub schema_version: u16,
    /// Exact operation.
    pub operation: GitInspectionOperation,
    /// Digest of the exact repository identity supplied by the host.
    pub repository_sha256: String,
    /// Digest of the exact worktree identity supplied by the host.
    pub worktree_sha256: String,
    /// Revision requested or observed, when applicable.
    pub revision: Option<String>,
    /// Explicit terminal state.
    pub outcome: GitInspectionOutcome,
    /// Ordered bounded untrusted records.
    pub records: Vec<GitRecord>,
    /// Exact worker-output byte count observed before parsing.
    pub observed_bytes: u64,
    /// Whether a declared bound truncated output.
    pub truncated: bool,
    /// Explicit dirty-tree fact for dirty-tree inspection only.
    pub dirty: Option<bool>,
    /// Trusted host freshness token for the held repository snapshot.
    pub freshness_sha256: String,
    /// SHA-256 over every preceding result field.
    pub result_sha256: String,
}

impl GitInspectionResult {
    /// Verifies identities, bounds, control-safe records, and complete result digest.
    #[must_use]
    pub fn verify(&self) -> bool {
        self.schema_version == 1
            && self.records.len() <= MAX_GIT_RECORDS as usize
            && self.observed_bytes <= MAX_GIT_OUTPUT_BYTES
            && is_sha256(&self.repository_sha256)
            && is_sha256(&self.worktree_sha256)
            && is_sha256(&self.freshness_sha256)
            && (self.operation == GitInspectionOperation::DirtyTree || self.dirty.is_none())
            && (self.operation != GitInspectionOperation::DirtyTree || self.dirty.is_some())
            && self.records.iter().enumerate().all(|(index, record)| {
                record.sequence == index as u32 + 1
                    && is_sha256(&record.raw_sha256)
                    && !record.escaped_text.chars().any(char::is_control)
            })
            && self.result_sha256 == result_sha256(self)
    }
}

/// Builds one fixed Git invocation without a shell, alias, hook, pager, or network operation.
pub fn plan_git_inspection(
    request: &GitInspectionRequest,
) -> Result<GitCommandPlan, GitInspectionError> {
    validate_request(request)?;
    let mut argv = vec![
        "git".to_owned(),
        "--no-pager".to_owned(),
        "-c".to_owned(),
        "core.hooksPath=/dev/null".to_owned(),
        "-c".to_owned(),
        "core.fsmonitor=false".to_owned(),
        "-c".to_owned(),
        "diff.external=".to_owned(),
        "-c".to_owned(),
        "diff.trustExitCode=false".to_owned(),
    ];
    let stdin = Vec::new();
    match request.operation {
        GitInspectionOperation::Status | GitInspectionOperation::DirtyTree => argv.extend(
            [
                "status",
                "--porcelain=v2",
                "--branch",
                "-z",
                "--untracked-files=all",
            ]
            .map(str::to_owned),
        ),
        GitInspectionOperation::CurrentBranch => {
            argv.extend(["symbolic-ref", "--quiet", "--short", "HEAD"].map(str::to_owned))
        }
        GitInspectionOperation::Upstream => argv.extend(
            [
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ]
            .map(str::to_owned),
        ),
        GitInspectionOperation::BranchList => argv.extend(
            [
                "for-each-ref",
                "--sort=refname",
                "--format=%(refname)%00%(objectname)%00%(upstream)%00",
                "refs/heads",
                "refs/remotes",
            ]
            .map(str::to_owned),
        ),
        GitInspectionOperation::Log => {
            argv.extend([
                "log".to_owned(),
                "--no-decorate".to_owned(),
                "--no-show-signature".to_owned(),
                format!("--max-count={}", request.max_records),
                "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%s%x00".to_owned(),
            ]);
            append_revision(&mut argv, request.revision.as_deref());
        }
        GitInspectionOperation::Diff | GitInspectionOperation::StagedDiff => {
            argv.push("diff".to_owned());
            argv.extend(
                ["--no-ext-diff", "--no-textconv", "--binary", "--no-renames"].map(str::to_owned),
            );
            if request.operation == GitInspectionOperation::StagedDiff {
                argv.push("--cached".to_owned());
            }
            append_pathspecs(&mut argv, &request.pathspecs);
        }
        GitInspectionOperation::Show => {
            argv.extend(
                [
                    "show",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--binary",
                    "--no-renames",
                    "--no-show-signature",
                    "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%s%x00",
                ]
                .map(str::to_owned),
            );
            append_revision(&mut argv, request.revision.as_deref());
            append_pathspecs(&mut argv, &request.pathspecs);
        }
        GitInspectionOperation::WorktreeList => {
            argv.extend(["worktree", "list", "--porcelain", "-z"].map(str::to_owned))
        }
        GitInspectionOperation::Object => {
            argv.extend(["cat-file", "-t"].map(str::to_owned));
            argv.push(
                request
                    .object_id
                    .as_deref()
                    .expect("validated object")
                    .to_owned(),
            );
        }
        GitInspectionOperation::Ref => {
            argv.extend(["rev-parse", "--verify", "--end-of-options"].map(str::to_owned));
            argv.push(format!(
                "{}^{{object}}",
                request.revision.as_deref().expect("validated revision")
            ));
        }
        GitInspectionOperation::UntrackedFiles => {
            argv.extend(["ls-files", "--others", "--exclude-standard", "-z"].map(str::to_owned))
        }
    }
    Ok(GitCommandPlan {
        argv,
        environment: BTreeMap::from([
            ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
            ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
            ("GIT_NO_REPLACE_OBJECTS".to_owned(), "1".to_owned()),
            ("GIT_OPTIONAL_LOCKS".to_owned(), "0".to_owned()),
            ("GIT_PAGER".to_owned(), "cat".to_owned()),
            ("GIT_PROTOCOL_FROM_USER".to_owned(), "0".to_owned()),
            ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
            ("HOME".to_owned(), "/nonexistent".to_owned()),
            ("LANG".to_owned(), "C".to_owned()),
            ("LC_ALL".to_owned(), "C".to_owned()),
            ("PAGER".to_owned(), "cat".to_owned()),
        ]),
        stdin,
    })
}

/// Parses and validates one duplicate-key-free closed Git inspection request.
pub fn validate_git_inspection_request(
    bytes: &[u8],
) -> Result<GitInspectionRequest, GitInspectionError> {
    if bytes.is_empty() || bytes.len() > 64 * 1024 {
        return Err(GitInspectionError::InvalidRequest);
    }
    let request = crate::protocol::parse_closed_json(bytes)
        .map_err(|()| GitInspectionError::InvalidRequest)?;
    validate_request(&request)?;
    Ok(request)
}

/// Parses bounded fixed-worker output into control-safe untrusted evidence.
pub fn parse_git_inspection(
    request: &GitInspectionRequest,
    repository_sha256: &str,
    worktree_sha256: &str,
    freshness_sha256: &str,
    worker_succeeded: bool,
    output: &[u8],
) -> Result<GitInspectionResult, GitInspectionError> {
    validate_request(request)?;
    if ![repository_sha256, worktree_sha256, freshness_sha256]
        .iter()
        .all(|value| is_sha256(value))
        || output.len() as u64 > MAX_GIT_OUTPUT_BYTES
    {
        return Err(GitInspectionError::OutputDenied);
    }
    let admitted = output.len().min(request.max_output_bytes as usize);
    let bounded = &output[..admitted];
    let mut records = split_records(bounded)
        .into_iter()
        .filter(|record| !record.is_empty())
        .take(request.max_records as usize)
        .enumerate()
        .map(|(index, record)| GitRecord {
            sequence: index as u32 + 1,
            record_kind: record_kind(request.operation, record).to_owned(),
            escaped_text: escape_untrusted(record),
            raw_sha256: sha256_hex(record),
        })
        .collect::<Vec<_>>();
    let source_records = split_records(bounded)
        .into_iter()
        .filter(|record| !record.is_empty())
        .count();
    let truncated = admitted < output.len() || source_records > records.len();
    let dirty = (request.operation == GitInspectionOperation::DirtyTree).then(|| {
        records
            .iter()
            .any(|record| record.record_kind != "branch_header")
    });
    if request.operation == GitInspectionOperation::DirtyTree {
        records.retain(|record| record.record_kind != "branch_header");
        records.truncate(1);
        for (index, record) in records.iter_mut().enumerate() {
            record.sequence = index as u32 + 1;
        }
    }
    let outcome = if !worker_succeeded {
        GitInspectionOutcome::Failed
    } else if truncated {
        GitInspectionOutcome::Truncated
    } else if records.is_empty() {
        GitInspectionOutcome::NoResult
    } else {
        GitInspectionOutcome::Succeeded
    };
    let mut result = GitInspectionResult {
        schema_version: 1,
        operation: request.operation,
        repository_sha256: repository_sha256.to_owned(),
        worktree_sha256: worktree_sha256.to_owned(),
        revision: request.revision.clone(),
        outcome,
        records,
        observed_bytes: output.len() as u64,
        truncated,
        dirty,
        freshness_sha256: freshness_sha256.to_owned(),
        result_sha256: String::new(),
    };
    result.result_sha256 = result_sha256(&result);
    Ok(result)
}

fn validate_request(request: &GitInspectionRequest) -> Result<(), GitInspectionError> {
    if request.schema_version != 1
        || request.max_records == 0
        || request.max_records > MAX_GIT_RECORDS
        || request.max_output_bytes == 0
        || request.max_output_bytes > MAX_GIT_OUTPUT_BYTES
        || request.pathspecs.len() > MAX_GIT_PATHS
        || request
            .pathspecs
            .iter()
            .any(|path| canonical_path(path).is_none())
        || request
            .revision
            .as_deref()
            .is_some_and(|value| !valid_revision(value))
        || request
            .object_id
            .as_deref()
            .is_some_and(|value| !valid_object_id(value))
    {
        return Err(GitInspectionError::InvalidRequest);
    }
    let revision_shape = match request.operation {
        GitInspectionOperation::Show | GitInspectionOperation::Ref => request.revision.is_some(),
        GitInspectionOperation::Log => true,
        _ => request.revision.is_none(),
    };
    let needs_object = request.operation == GitInspectionOperation::Object;
    let pathspec_shape = matches!(
        request.operation,
        GitInspectionOperation::Diff
            | GitInspectionOperation::StagedDiff
            | GitInspectionOperation::Show
    ) || request.pathspecs.is_empty();
    if !revision_shape || needs_object != request.object_id.is_some() || !pathspec_shape {
        return Err(GitInspectionError::InvalidRequest);
    }
    Ok(())
}

fn append_revision(argv: &mut Vec<String>, revision: Option<&str>) {
    argv.push("--end-of-options".to_owned());
    if let Some(revision) = revision {
        argv.push(revision.to_owned());
    }
}

fn append_pathspecs(argv: &mut Vec<String>, paths: &[Vec<String>]) {
    argv.push("--".to_owned());
    argv.extend(paths.iter().map(|path| path.join("/")));
}

fn split_records(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.contains(&0) {
        bytes.split(|byte| *byte == 0).collect()
    } else {
        bytes.split(|byte| *byte == b'\n').collect()
    }
}

fn record_kind(operation: GitInspectionOperation, record: &[u8]) -> &'static str {
    if record.starts_with(b"# branch.") {
        "branch_header"
    } else if record.starts_with(b"1 ") {
        "ordinary_change"
    } else if record.starts_with(b"2 ") {
        "rename_or_copy"
    } else if record.starts_with(b"u ") {
        "unmerged"
    } else if record.starts_with(b"? ") {
        "untracked"
    } else if record.starts_with(b"! ") {
        "ignored"
    } else {
        operation.id()
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

fn valid_revision(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_GIT_REVISION_BYTES
        && !value.starts_with('-')
        && !value.starts_with('.')
        && !value.ends_with(['/', '.'])
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains("@{")
        && !value.ends_with(".lock")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._@-".contains(&byte))
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn canonical_path(path: &[String]) -> Option<WorkspacePath> {
    WorkspacePath::new(
        WorkspaceId::from_raw("workspace-git-inspection"),
        path.iter().cloned(),
    )
    .ok()
}

fn result_sha256(result: &GitInspectionResult) -> String {
    serde_json::to_vec(&(
        result.schema_version,
        result.operation,
        &result.repository_sha256,
        &result.worktree_sha256,
        &result.revision,
        result.outcome,
        &result.records,
        result.observed_bytes,
        result.truncated,
        result.dirty,
        &result.freshness_sha256,
    ))
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_else(|_| sha256_hex(b"git-result-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn git_schema(id: &str, bytes: &[u8]) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: sha256_hex(bytes),
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::Write as IoWrite;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{GrantOperation, ToolRiskLevel};
    use sha2::{Digest, Sha256};

    use super::{
        GIT_INSPECTION_INPUT_SCHEMA_JSON, GIT_INSPECTION_OUTPUT_SCHEMA_JSON,
        GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION, GitInspectionOperation,
        GitInspectionOutcome, GitInspectionRequest, git_inspection_tool_definition,
        parse_git_inspection, plan_git_inspection, validate_git_inspection_request,
    };

    fn request(operation: GitInspectionOperation) -> GitInspectionRequest {
        GitInspectionRequest {
            schema_version: 1,
            operation,
            revision: matches!(
                operation,
                GitInspectionOperation::Show | GitInspectionOperation::Ref
            )
            .then(|| "HEAD".to_owned()),
            object_id: (operation == GitInspectionOperation::Object).then(|| "a".repeat(40)),
            pathspecs: Vec::new(),
            max_records: 100,
            max_output_bytes: 1024,
        }
    }

    #[test]
    fn native_definition_is_closed_content_bound_and_read_only() {
        let definition = git_inspection_tool_definition();
        assert_eq!(definition.tool_id.as_str(), GIT_INSPECTION_TOOL_ID);
        assert_eq!(definition.tool_version, GIT_INSPECTION_TOOL_VERSION);
        assert_eq!(definition.risk_level, ToolRiskLevel::Low);
        assert_eq!(definition.declared_effects.len(), 1);
        assert_eq!(
            definition.declared_effects[0].operation(),
            GrantOperation::WorkspaceRead
        );
        assert_eq!(
            definition.required_grant.operation.operation(),
            GrantOperation::WorkspaceRead
        );
        assert!(definition.required_grant.single_use);
        for schema in [
            GIT_INSPECTION_INPUT_SCHEMA_JSON,
            GIT_INSPECTION_OUTPUT_SCHEMA_JSON,
        ] {
            let value: serde_json::Value = serde_json::from_str(schema).expect("schema JSON");
            assert_eq!(value["additionalProperties"], false);
        }
        assert_ne!(
            definition.input_schema.schema_sha256,
            definition.output_schema.schema_sha256
        );
    }

    #[test]
    fn native_request_validation_rejects_duplicate_unknown_and_unsafe_fields() {
        let valid = serde_json::to_vec(&request(GitInspectionOperation::Status))
            .expect("valid Git request");
        assert!(validate_git_inspection_request(&valid).is_ok());

        let duplicate = br#"{"schema_version":1,"schema_version":1,"operation":"status","revision":null,"object_id":null,"pathspecs":[],"max_records":100,"max_output_bytes":1024}"#;
        let unknown = br#"{"schema_version":1,"operation":"status","revision":null,"object_id":null,"pathspecs":[],"max_records":100,"max_output_bytes":1024,"command":"reset --hard"}"#;
        let unsafe_revision = br#"{"schema_version":1,"operation":"show","revision":"--exec-path=/tmp","object_id":null,"pathspecs":[],"max_records":100,"max_output_bytes":1024}"#;
        for invalid in [
            duplicate.as_slice(),
            unknown.as_slice(),
            unsafe_revision.as_slice(),
        ] {
            assert!(validate_git_inspection_request(invalid).is_err());
        }
    }

    #[test]
    fn every_operation_has_one_fixed_non_shell_plan_and_closed_environment() {
        for operation in GitInspectionOperation::ALL {
            let plan = plan_git_inspection(&request(operation)).expect("fixed plan");
            assert_eq!(plan.argv[0], "git");
            assert_eq!(plan.argv[1], "--no-pager");
            assert!(
                plan.argv
                    .iter()
                    .any(|value| value == "core.hooksPath=/dev/null")
            );
            assert_eq!(plan.environment["GIT_CONFIG_NOSYSTEM"], "1");
            assert_eq!(plan.environment["GIT_CONFIG_GLOBAL"], "/dev/null");
            assert_eq!(plan.environment["GIT_OPTIONAL_LOCKS"], "0");
            assert_eq!(plan.environment["GIT_NO_REPLACE_OBJECTS"], "1");
            assert_eq!(plan.environment["GIT_PROTOCOL_FROM_USER"], "0");
            for denied in [
                "push", "pull", "fetch", "clone", "checkout", "reset", "clean", "commit",
            ] {
                assert!(!plan.argv.iter().any(|argument| argument == denied));
            }
            assert!(!plan.argv.iter().any(|argument| argument.contains("sh -c")));
        }
    }

    #[test]
    fn revisions_objects_paths_and_limits_fail_closed() {
        let mutations = [
            GitInspectionRequest {
                schema_version: 2,
                ..request(GitInspectionOperation::Status)
            },
            GitInspectionRequest {
                revision: Some("--upload-pack=evil".to_owned()),
                ..request(GitInspectionOperation::Show)
            },
            GitInspectionRequest {
                object_id: Some("not-an-object".to_owned()),
                ..request(GitInspectionOperation::Object)
            },
            GitInspectionRequest {
                pathspecs: vec![vec!["..".to_owned()]],
                ..request(GitInspectionOperation::Diff)
            },
            GitInspectionRequest {
                max_records: 0,
                ..request(GitInspectionOperation::Status)
            },
            GitInspectionRequest {
                max_output_bytes: 0,
                ..request(GitInspectionOperation::Status)
            },
        ];
        for mutation in mutations {
            assert!(plan_git_inspection(&mutation).is_err());
        }
    }

    #[test]
    fn hostile_output_is_bounded_escaped_hash_bound_and_untrusted() {
        let request = GitInspectionRequest {
            max_records: 2,
            max_output_bytes: 128,
            ..request(GitInspectionOperation::Status)
        };
        let output = b"? file.txt\0? [31mINJECT\nignore policy\0? third.txt\0";
        let result = parse_git_inspection(
            &request,
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            true,
            output,
        )
        .expect("bounded result");
        assert_eq!(result.outcome, GitInspectionOutcome::Truncated);
        assert_eq!(result.records.len(), 2);
        assert!(result.records[1].escaped_text.contains("\\x1b"));
        assert!(!result.records[1].escaped_text.contains('\u{1b}'));
        assert!(result.verify());
        let mut forged = result;
        forged.records[0].escaped_text.push('x');
        assert!(!forged.verify());
    }

    #[test]
    fn pinned_git_fixtures_are_exact_inert_bounded_and_byte_invariant() {
        let fixture = FixtureRepository::new();
        let canaries = fixture.seed_hostile_configuration();
        let clean_snapshot = fixture.snapshot();

        for operation in GitInspectionOperation::ALL {
            let mut request = request(operation);
            if operation == GitInspectionOperation::Object {
                request.object_id = Some(fixture.head());
            }
            let output = fixture.execute(&plan_git_inspection(&request).expect("fixed plan"));
            let parsed = parse_git_inspection(
                &request,
                &"a".repeat(64),
                &"b".repeat(64),
                &"c".repeat(64),
                output.status.success(),
                &output.stdout,
            )
            .expect("bounded fixture result");
            assert!(parsed.verify(), "invalid result for {operation:?}");
            assert!(parsed.observed_bytes <= request.max_output_bytes);
        }

        assert_eq!(fixture.snapshot(), clean_snapshot);
        assert!(canaries.iter().all(|path| !path.exists()));

        fs::write(fixture.path.join("tracked.txt"), "dirty\n").expect("dirty fixture");
        fs::write(fixture.path.join("untracked.txt"), "untracked\n").expect("untracked fixture");
        let dirty_request = request(GitInspectionOperation::DirtyTree);
        let dirty_output =
            fixture.execute(&plan_git_inspection(&dirty_request).expect("dirty plan"));
        let dirty = parse_git_inspection(
            &dirty_request,
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            dirty_output.status.success(),
            &dirty_output.stdout,
        )
        .expect("dirty result");
        assert_eq!(dirty.dirty, Some(true));
        assert!(dirty.verify());

        fixture.git(&["add", "tracked.txt"]);
        clear_canaries(&canaries);
        let staged = fixture.execute(
            &plan_git_inspection(&request(GitInspectionOperation::StagedDiff))
                .expect("staged plan"),
        );
        assert!(staged.status.success());
        assert!(!staged.stdout.is_empty());
        assert!(canaries.iter().all(|path| !path.exists()));

        fixture.git(&["reset", "--hard", "HEAD"]);
        fs::remove_file(fixture.path.join("untracked.txt")).expect("remove untracked fixture");
        fixture.git(&["mv", "tracked.txt", "renamed.txt"]);
        clear_canaries(&canaries);
        let renamed = fixture.execute(
            &plan_git_inspection(&request(GitInspectionOperation::Status)).expect("renamed plan"),
        );
        assert!(renamed.status.success());
        assert!(renamed.stdout.windows(2).any(|window| window == b"2 "));
        assert!(canaries.iter().all(|path| !path.exists()));

        fixture.git(&["reset", "--hard", "HEAD"]);
        fixture.git(&["checkout", "--detach", "HEAD"]);
        clear_canaries(&canaries);
        let branch = fixture.execute(
            &plan_git_inspection(&request(GitInspectionOperation::CurrentBranch))
                .expect("branch plan"),
        );
        assert!(!branch.status.success());
        assert!(branch.stdout.is_empty());

        let malformed = parse_git_inspection(
            &GitInspectionRequest {
                max_records: 1,
                max_output_bytes: 2,
                ..request(GitInspectionOperation::Status)
            },
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            true,
            b"\xff\0second\0",
        )
        .expect("malformed bytes are bounded data");
        assert_eq!(malformed.outcome, GitInspectionOutcome::Truncated);
        assert_eq!(malformed.records[0].escaped_text, "\\xff");
        assert!(malformed.verify());
        assert!(canaries.iter().all(|path| !path.exists()));
    }

    struct FixtureRepository {
        path: PathBuf,
    }

    impl FixtureRepository {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(1);
            let path = std::env::temp_dir().join(format!(
                "agentmage-git-fixture-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create fixture");
            let fixture = Self { path };
            fixture.git(&["init", "--initial-branch=main"]);
            fixture.git(&["config", "user.name", "AgentMage Fixture"]);
            fixture.git(&["config", "user.email", "fixture@example.invalid"]);
            fs::write(fixture.path.join("tracked.txt"), "clean\n").expect("write fixture");
            fs::write(fixture.path.join("ignored.txt"), "ignored\n").expect("write ignored");
            fs::write(fixture.path.join(".gitignore"), "ignored.txt\n").expect("write ignore");
            fixture.git(&["add", "tracked.txt", ".gitignore"]);
            fixture.git(&["commit", "-m", "fixture commit"]);
            fixture.git(&["tag", "fixture-tag"]);
            fixture
        }

        fn seed_hostile_configuration(&self) -> Vec<PathBuf> {
            let hook_canary = self.path.join("hook-ran");
            let pager_canary = self.path.join("pager-ran");
            let diff_canary = self.path.join("diff-ran");
            let credential_canary = self.path.join("credential-ran");
            let alias_canary = self.path.join("alias-ran");
            let hook = self.path.join(".git/hooks/post-index-change");
            fs::write(
                &hook,
                format!("#!/bin/sh\ntouch {}\n", hook_canary.display()),
            )
            .expect("write hostile hook");
            let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&hook, permissions).expect("make hook executable");
            fs::write(self.path.join(".gitattributes"), "*.txt diff=hostile\n")
                .expect("write attributes");
            self.git(&[
                "config",
                "diff.hostile.command",
                &format!("touch {}", diff_canary.display()),
            ]);
            self.git(&[
                "config",
                "core.pager",
                &format!("touch {}", pager_canary.display()),
            ]);
            self.git(&[
                "config",
                "credential.helper",
                &format!("!touch {}", credential_canary.display()),
            ]);
            self.git(&[
                "config",
                "alias.inspect",
                &format!("!touch {}", alias_canary.display()),
            ]);
            self.git(&[
                "remote",
                "add",
                "origin",
                "ssh://invalid.example/repository",
            ]);
            vec![
                hook_canary,
                pager_canary,
                diff_canary,
                credential_canary,
                alias_canary,
            ]
        }

        fn head(&self) -> String {
            String::from_utf8(self.git_output(&["rev-parse", "HEAD"]).stdout)
                .expect("head utf8")
                .trim()
                .to_owned()
        }

        fn execute(&self, plan: &super::GitCommandPlan) -> std::process::Output {
            let mut child = Command::new(&plan.argv[0]);
            child
                .args(&plan.argv[1..])
                .current_dir(&self.path)
                .env_clear()
                .envs(&plan.environment)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = child.spawn().expect("spawn fixed git plan");
            child
                .stdin
                .take()
                .expect("git stdin")
                .write_all(&plan.stdin)
                .expect("write fixed stdin");
            child.wait_with_output().expect("wait for git")
        }

        fn git(&self, args: &[&str]) {
            let output = self.git_output(args);
            assert!(
                output.status.success(),
                "fixture git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        fn git_output(&self, args: &[&str]) -> std::process::Output {
            Command::new("git")
                .args(args)
                .current_dir(&self.path)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .expect("run fixture setup git")
        }

        fn snapshot(&self) -> BTreeMap<PathBuf, String> {
            let mut snapshot = BTreeMap::new();
            snapshot_tree(&self.path, &self.path, &mut snapshot);
            snapshot
        }
    }

    impl Drop for FixtureRepository {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn snapshot_tree(root: &Path, path: &Path, snapshot: &mut BTreeMap<PathBuf, String>) {
        let mut entries = fs::read_dir(path)
            .expect("read fixture tree")
            .map(|entry| entry.expect("fixture entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("fixture metadata");
            let relative = path
                .strip_prefix(root)
                .expect("fixture relative")
                .to_owned();
            if metadata.is_dir() {
                snapshot.insert(
                    relative,
                    format!("directory:{:o}", metadata.permissions().mode()),
                );
                snapshot_tree(root, &path, snapshot);
            } else if metadata.file_type().is_symlink() {
                snapshot.insert(
                    relative,
                    format!(
                        "symlink:{:o}:{}",
                        metadata.permissions().mode(),
                        fs::read_link(&path).expect("read fixture link").display()
                    ),
                );
            } else {
                snapshot.insert(
                    relative,
                    format!(
                        "file:{:o}:{}",
                        metadata.permissions().mode(),
                        hex_sha256(&fs::read(&path).expect("read fixture file"))
                    ),
                );
            }
        }
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn clear_canaries(canaries: &[PathBuf]) {
        for canary in canaries {
            if canary.exists() {
                fs::remove_file(canary).expect("clear setup canary");
            }
        }
    }
}
