//! Pure projections for immutable, read-only hosted repository evidence.
#![allow(missing_docs)]

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

/// Closed hosted repository evidence family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedEvidenceKind {
    Organization,
    User,
    Team,
    Repository,
    SearchResult,
    File,
    Directory,
    Symlink,
    Submodule,
    LargeFilePointer,
    Blob,
    Tree,
    Branch,
    Tag,
    Commit,
    Comparison,
    Release,
    ReleaseAsset,
    Ruleset,
    Workflow,
    SecurityFinding,
    Advisory,
    DependencyGraph,
    SoftwareBillOfMaterials,
}

/// Explicit coverage state; incomplete observations cannot claim completeness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedCoverageState {
    Complete,
    Partial,
    Unknown,
    Blocked,
    Inaccessible,
    Stale,
}

/// Content trust remains data-only regardless of repository instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedContentClassification {
    Public,
    Private,
    Restricted,
    UntrustedInstructions,
}

/// Immutable source coordinate for every hosted fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedSourceIdentity {
    pub host: String,
    pub repository_id: String,
    pub account_id: String,
    pub immutable_revision: String,
    pub immutable_object_id: String,
    pub path: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub retrieved_epoch_milliseconds: u64,
    pub canonical_url_sha256: String,
}

/// One normalized fact without executable or policy authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRepositoryFact {
    pub fact_id: String,
    pub kind: HostedEvidenceKind,
    pub source: HostedSourceIdentity,
    pub title: String,
    pub value_sha256: String,
    pub classification: HostedContentClassification,
    pub coverage: HostedCoverageState,
    pub permission_scopes: Vec<String>,
    pub limitations: Vec<String>,
    pub secret_name_count: u32,
    pub secret_value_count: u32,
    pub instruction_authority: bool,
    pub execution_authority: bool,
    pub policy_authority: bool,
}

/// Coverage-aware read-only view spanning repository provider families.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRepositoryView {
    pub schema_version: u16,
    pub view_id: String,
    pub host: String,
    pub repository_id: String,
    pub immutable_revision: String,
    pub facts: Vec<HostedRepositoryFact>,
    pub next_page_cursor_sha256: Option<String>,
    pub missing_permission_scopes: Vec<String>,
    pub unavailable_families: Vec<String>,
    pub overall_coverage: HostedCoverageState,
    pub local_revision: Option<String>,
    pub local_identity_matches: Option<bool>,
    pub hosted_state_changed: bool,
    pub local_state_changed: bool,
    pub complete: bool,
    pub view_sha256: String,
}

/// Projection failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedRepositoryError {
    InvalidInput,
    IdentityMismatch,
    CoverageOverclaim,
    ResourceLimit,
}
impl HostedRepositoryError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "hosted-repository.input.invalid",
            Self::IdentityMismatch => "hosted-repository.identity.mismatch",
            Self::CoverageOverclaim => "hosted-repository.coverage.overclaim",
            Self::ResourceLimit => "hosted-repository.resource.limit",
        }
    }
}
impl std::fmt::Display for HostedRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for HostedRepositoryError {}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value == value.to_ascii_lowercase()
        && !value.contains(['/', ':', '@', '?', '#', '\\'])
        && value.split('.').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn sorted_unique(values: &[String]) -> bool {
    values.windows(2).all(|w| w[0] < w[1]) && values.iter().all(|v| valid_identifier(v))
}
fn valid_source(source: &HostedSourceIdentity) -> bool {
    valid_host(&source.host)
        && valid_identifier(&source.repository_id)
        && valid_identifier(&source.account_id)
        && valid_sha(&source.immutable_revision)
        && valid_sha(&source.immutable_object_id)
        && source.retrieved_epoch_milliseconds > 0
        && valid_sha(&source.canonical_url_sha256)
        && source.path.as_ref().is_none_or(|p| {
            !p.is_empty()
                && p.len() <= 2048
                && !p.starts_with('/')
                && !p.contains(['\\', '?', '#'])
                && !p.split('/').any(|part| part == "..")
        })
        && match (source.line_start, source.line_end) {
            (None, None) => true,
            (Some(a), Some(b)) => a > 0 && b >= a,
            _ => false,
        }
}

/// Construct a canonical immutable URL digest without returning a mutable branch link.
pub fn immutable_source_link_sha256(
    host: &str,
    repository_id: &str,
    revision: &str,
    path: Option<&str>,
    line_start: Option<u32>,
    line_end: Option<u32>,
) -> Result<String, HostedRepositoryError> {
    let source = HostedSourceIdentity {
        host: host.into(),
        repository_id: repository_id.into(),
        account_id: "validation-only".into(),
        immutable_revision: revision.into(),
        immutable_object_id: "a".repeat(64),
        path: path.map(str::to_owned),
        line_start,
        line_end,
        retrieved_epoch_milliseconds: 1,
        canonical_url_sha256: "b".repeat(64),
    };
    if !valid_source(&source) {
        return Err(HostedRepositoryError::InvalidInput);
    }
    Ok(word_sha256(
        format!(
            "https://{host}/{repository_id}/blob/{revision}/{}#L{}-L{}",
            path.unwrap_or(""),
            line_start.unwrap_or(0),
            line_end.unwrap_or(0)
        )
        .as_bytes(),
    ))
}

/// Validate facts and build a coverage-honest, mutation-free repository view.
pub fn build_hosted_repository_view(
    view_id: &str,
    facts: Vec<HostedRepositoryFact>,
    next_page_cursor_sha256: Option<String>,
    missing_permission_scopes: Vec<String>,
    unavailable_families: Vec<String>,
    local_revision: Option<String>,
) -> Result<HostedRepositoryView, HostedRepositoryError> {
    if !valid_identifier(view_id)
        || facts.is_empty()
        || facts.len() > 1000
        || !sorted_unique(&missing_permission_scopes)
        || !sorted_unique(&unavailable_families)
        || next_page_cursor_sha256
            .as_ref()
            .is_some_and(|v| !valid_sha(v))
        || local_revision.as_ref().is_some_and(|v| !valid_sha(v))
    {
        return Err(HostedRepositoryError::InvalidInput);
    }
    let first = &facts[0].source;
    for fact in &facts {
        if !valid_identifier(&fact.fact_id)
            || fact.title.is_empty()
            || fact.title.len() > 512
            || !valid_sha(&fact.value_sha256)
            || !valid_source(&fact.source)
            || fact.source.host != first.host
            || fact.source.repository_id != first.repository_id
            || fact.source.immutable_revision != first.immutable_revision
            || !sorted_unique(&fact.permission_scopes)
            || !sorted_unique(&fact.limitations)
            || fact.secret_value_count != 0
            || fact.instruction_authority
            || fact.execution_authority
            || fact.policy_authority
        {
            return Err(HostedRepositoryError::IdentityMismatch);
        }
    }
    let incomplete = next_page_cursor_sha256.is_some()
        || !missing_permission_scopes.is_empty()
        || !unavailable_families.is_empty()
        || facts
            .iter()
            .any(|f| f.coverage != HostedCoverageState::Complete);
    let overall_coverage = if !missing_permission_scopes.is_empty() {
        HostedCoverageState::Blocked
    } else if incomplete {
        HostedCoverageState::Partial
    } else {
        HostedCoverageState::Complete
    };
    let local_identity_matches = local_revision
        .as_ref()
        .map(|r| r == &first.immutable_revision);
    let mut view = HostedRepositoryView {
        schema_version: CONTRACT_SCHEMA_VERSION,
        view_id: view_id.into(),
        host: first.host.clone(),
        repository_id: first.repository_id.clone(),
        immutable_revision: first.immutable_revision.clone(),
        facts,
        next_page_cursor_sha256,
        missing_permission_scopes,
        unavailable_families,
        overall_coverage,
        local_revision,
        local_identity_matches,
        hosted_state_changed: false,
        local_state_changed: false,
        complete: !incomplete,
        view_sha256: String::new(),
    };
    view.view_sha256 =
        word_sha256(&serde_json::to_vec(&view).map_err(|_| HostedRepositoryError::InvalidInput)?);
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fact(kind: HostedEvidenceKind) -> HostedRepositoryFact {
        HostedRepositoryFact {
            fact_id: format!("fact-{kind:?}").to_ascii_lowercase(),
            kind,
            source: HostedSourceIdentity {
                host: "github.com".into(),
                repository_id: "org-repo".into(),
                account_id: "account-1".into(),
                immutable_revision: "a".repeat(64),
                immutable_object_id: "b".repeat(64),
                path: Some("src/lib.rs".into()),
                line_start: Some(1),
                line_end: Some(3),
                retrieved_epoch_milliseconds: 100,
                canonical_url_sha256: "c".repeat(64),
            },
            title: "Observed metadata".into(),
            value_sha256: "d".repeat(64),
            classification: HostedContentClassification::UntrustedInstructions,
            coverage: HostedCoverageState::Complete,
            permission_scopes: vec!["metadata:read".into()],
            limitations: vec![],
            secret_name_count: 0,
            secret_value_count: 0,
            instruction_authority: false,
            execution_authority: false,
            policy_authority: false,
        }
    }
    #[test]
    fn complete_views_bind_all_provider_families_without_effects() {
        let kinds = [
            HostedEvidenceKind::Organization,
            HostedEvidenceKind::Repository,
            HostedEvidenceKind::SearchResult,
            HostedEvidenceKind::File,
            HostedEvidenceKind::Symlink,
            HostedEvidenceKind::Submodule,
            HostedEvidenceKind::LargeFilePointer,
            HostedEvidenceKind::Tree,
            HostedEvidenceKind::Branch,
            HostedEvidenceKind::Commit,
            HostedEvidenceKind::Release,
            HostedEvidenceKind::Ruleset,
            HostedEvidenceKind::Workflow,
            HostedEvidenceKind::SecurityFinding,
            HostedEvidenceKind::DependencyGraph,
            HostedEvidenceKind::SoftwareBillOfMaterials,
        ];
        let view = build_hosted_repository_view(
            "view-1",
            kinds.into_iter().map(fact).collect(),
            None,
            vec![],
            vec![],
            Some("a".repeat(64)),
        )
        .unwrap();
        assert!(view.complete);
        assert_eq!(view.local_identity_matches, Some(true));
        assert!(!view.hosted_state_changed && !view.local_state_changed);
    }
    #[test]
    fn pagination_permissions_unavailable_and_stale_local_are_explicit() {
        let view = build_hosted_repository_view(
            "view-1",
            vec![fact(HostedEvidenceKind::Workflow)],
            Some("e".repeat(64)),
            vec!["security-events:read".into()],
            vec!["secret-scanning".into()],
            Some("f".repeat(64)),
        )
        .unwrap();
        assert!(!view.complete);
        assert_eq!(view.overall_coverage, HostedCoverageState::Blocked);
        assert_eq!(view.local_identity_matches, Some(false));
    }
    #[test]
    fn hostile_paths_secrets_authority_and_cross_repository_fail() {
        for mutate in 0..5 {
            let mut f = fact(HostedEvidenceKind::File);
            match mutate {
                0 => f.source.path = Some("../escape".into()),
                1 => f.secret_value_count = 1,
                2 => f.instruction_authority = true,
                3 => f.source.repository_id = "other-repo".into(),
                _ => f.source.canonical_url_sha256 = "bad".into(),
            };
            let facts = if mutate == 3 {
                vec![fact(HostedEvidenceKind::Repository), f]
            } else {
                vec![f]
            };
            assert!(
                build_hosted_repository_view("view-1", facts, None, vec![], vec![], None).is_err()
            );
        }
    }
    #[test]
    fn immutable_links_require_commit_path_and_range() {
        assert!(
            immutable_source_link_sha256(
                "github.com",
                "org-repo",
                &"a".repeat(64),
                Some("src/lib.rs"),
                Some(1),
                Some(2)
            )
            .is_ok()
        );
        assert!(
            immutable_source_link_sha256(
                "github.com",
                "org-repo",
                "main",
                Some("../x"),
                Some(2),
                Some(1)
            )
            .is_err()
        );
    }
}
