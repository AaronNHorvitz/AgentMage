//! Pure pull-request review, finding-remap, and local shadow-fix contracts.
#![allow(missing_docs)]

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDimension {
    Correctness,
    Security,
    Data,
    Accessibility,
    Performance,
    Dependency,
    Test,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFindingState {
    Current,
    Stale,
    Moved,
    Resolved,
    Superseded,
    Conflicting,
    Uncertain,
    LocallyUnverifiable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PullRequestWorktreeBinding {
    pub binding_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub worktree_sha256: String,
    pub base_revision: String,
    pub head_revision: String,
    pub merge_base_revision: String,
    pub fork_repository_id: Option<String>,
    pub changed_paths_sha256: String,
    pub dependency_state_sha256: String,
    pub instruction_state_sha256: String,
    pub test_state_sha256: String,
    pub commits_since_review: u32,
    pub base_moved: bool,
    pub active_checkout_unchanged: bool,
    pub remote_state_unchanged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PullRequestFinding {
    pub finding_id: String,
    pub dimension: ReviewDimension,
    pub severity: ReviewSeverity,
    pub confidence_basis_points: u16,
    pub state: ReviewFindingState,
    pub path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub commit_revision: String,
    pub evidence_sha256: String,
    pub duplicate_of: Option<String>,
    pub deterministic_check_id: Option<String>,
    pub formatting_only: bool,
    pub displayed: bool,
    pub reason_code: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalShadowFix {
    pub fix_id: String,
    pub finding_id: String,
    pub base_revision: String,
    pub full_diff_sha256: String,
    pub tests_sha256: String,
    pub risks_sha256: String,
    pub rollback_sha256: String,
    pub controlled_write_proposal: bool,
    pub applied: bool,
    pub committed: bool,
    pub pushed: bool,
    pub published: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPullRequestReviewPacket {
    pub schema_version: u16,
    pub packet_id: String,
    pub binding: PullRequestWorktreeBinding,
    pub findings: Vec<PullRequestFinding>,
    pub shadow_fixes: Vec<LocalShadowFix>,
    pub reviewed_epoch_milliseconds: u64,
    pub refreshed_hosted_state_sha256: Option<String>,
    pub precision_basis_points: u16,
    pub duplicate_suppression_basis_points: u16,
    pub publication_enabled: bool,
    pub branch_update_enabled: bool,
    pub commit_enabled: bool,
    pub push_enabled: bool,
    pub merge_enabled: bool,
    pub release_enabled: bool,
    pub packet_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullRequestReviewError {
    InvalidInput,
    IdentityMismatch,
    QualityThreshold,
    PublicationDenied,
}
impl PullRequestReviewError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "pr-review.input.invalid",
            Self::IdentityMismatch => "pr-review.identity.mismatch",
            Self::QualityThreshold => "pr-review.quality.threshold",
            Self::PublicationDenied => "pr-review.publication.denied",
        }
    }
}
impl std::fmt::Display for PullRequestReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for PullRequestReviewError {}
fn sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn path(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 2048
        && !v.starts_with('/')
        && !v.contains(['\\', '?', '#'])
        && !v.split('/').any(|p| p == "..")
}

pub fn build_local_pull_request_review(
    packet_id: &str,
    binding: PullRequestWorktreeBinding,
    mut findings: Vec<PullRequestFinding>,
    shadow_fixes: Vec<LocalShadowFix>,
    reviewed_epoch_milliseconds: u64,
    refreshed_hosted_state_sha256: Option<String>,
    minimum_precision_basis_points: u16,
) -> Result<LocalPullRequestReviewPacket, PullRequestReviewError> {
    if !valid_identifier(packet_id)
        || !valid_identifier(&binding.binding_id)
        || !valid_identifier(&binding.repository_id)
        || !valid_identifier(&binding.worktree_id)
        || ![
            &binding.worktree_sha256,
            &binding.base_revision,
            &binding.head_revision,
            &binding.merge_base_revision,
            &binding.changed_paths_sha256,
            &binding.dependency_state_sha256,
            &binding.instruction_state_sha256,
            &binding.test_state_sha256,
        ]
        .into_iter()
        .all(|v| sha(v))
        || binding.base_revision == binding.head_revision
        || !binding.active_checkout_unchanged
        || !binding.remote_state_unchanged
        || reviewed_epoch_milliseconds == 0
        || minimum_precision_basis_points > 10_000
        || refreshed_hosted_state_sha256
            .as_ref()
            .is_some_and(|v| !sha(v))
    {
        return Err(PullRequestReviewError::InvalidInput);
    }
    let mut suppressed = 0u32;
    for finding in &mut findings {
        if !valid_identifier(&finding.finding_id)
            || !path(&finding.path)
            || finding.line_start == 0
            || finding.line_end < finding.line_start
            || !sha(&finding.commit_revision)
            || !sha(&finding.evidence_sha256)
            || finding.confidence_basis_points > 10_000
            || finding
                .duplicate_of
                .as_ref()
                .is_some_and(|v| !valid_identifier(v))
            || finding
                .deterministic_check_id
                .as_ref()
                .is_some_and(|v| !valid_identifier(v))
        {
            return Err(PullRequestReviewError::InvalidInput);
        }
        if finding.commit_revision != binding.head_revision
            && finding.state == ReviewFindingState::Current
        {
            return Err(PullRequestReviewError::IdentityMismatch);
        }
        let should_suppress = finding.formatting_only
            || finding.duplicate_of.is_some()
            || finding.deterministic_check_id.is_some()
            || finding.confidence_basis_points < minimum_precision_basis_points
            || finding.state != ReviewFindingState::Current;
        finding.displayed = !should_suppress;
        suppressed += u32::from(should_suppress);
    }
    for fix in &shadow_fixes {
        if !valid_identifier(&fix.fix_id)
            || !findings.iter().any(|f| f.finding_id == fix.finding_id)
            || fix.base_revision != binding.head_revision
            || ![
                &fix.full_diff_sha256,
                &fix.tests_sha256,
                &fix.risks_sha256,
                &fix.rollback_sha256,
            ]
            .into_iter()
            .all(|v| sha(v))
            || !fix.controlled_write_proposal
            || fix.applied
            || fix.committed
            || fix.pushed
            || fix.published
        {
            return Err(PullRequestReviewError::PublicationDenied);
        }
    }
    let displayed = findings.iter().filter(|f| f.displayed).count() as u32;
    let precision = (findings
        .iter()
        .filter(|f| f.displayed && f.confidence_basis_points >= minimum_precision_basis_points)
        .count() as u32
        * 10_000)
        .checked_div(displayed)
        .unwrap_or(10_000) as u16;
    let duplicate_suppression = if findings.is_empty() {
        10_000
    } else {
        suppressed * 10_000 / findings.len() as u32
    } as u16;
    if precision < minimum_precision_basis_points {
        return Err(PullRequestReviewError::QualityThreshold);
    }
    let mut packet = LocalPullRequestReviewPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        packet_id: packet_id.into(),
        binding,
        findings,
        shadow_fixes,
        reviewed_epoch_milliseconds,
        refreshed_hosted_state_sha256,
        precision_basis_points: precision,
        duplicate_suppression_basis_points: duplicate_suppression,
        publication_enabled: false,
        branch_update_enabled: false,
        commit_enabled: false,
        push_enabled: false,
        merge_enabled: false,
        release_enabled: false,
        packet_sha256: String::new(),
    };
    packet.packet_sha256 = word_sha256(
        &serde_json::to_vec(&packet).map_err(|_| PullRequestReviewError::InvalidInput)?,
    );
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> PullRequestWorktreeBinding {
        PullRequestWorktreeBinding {
            binding_id: "binding-1".into(),
            repository_id: "repo-1".into(),
            worktree_id: "worktree-1".into(),
            worktree_sha256: "a".repeat(64),
            base_revision: "b".repeat(64),
            head_revision: "c".repeat(64),
            merge_base_revision: "b".repeat(64),
            fork_repository_id: None,
            changed_paths_sha256: "d".repeat(64),
            dependency_state_sha256: "e".repeat(64),
            instruction_state_sha256: "f".repeat(64),
            test_state_sha256: "1".repeat(64),
            commits_since_review: 1,
            base_moved: false,
            active_checkout_unchanged: true,
            remote_state_unchanged: true,
        }
    }
    fn finding(id: &str, state: ReviewFindingState) -> PullRequestFinding {
        PullRequestFinding {
            finding_id: id.into(),
            dimension: ReviewDimension::Correctness,
            severity: ReviewSeverity::High,
            confidence_basis_points: 9000,
            state,
            path: "src/lib.rs".into(),
            line_start: 1,
            line_end: 2,
            commit_revision: "c".repeat(64),
            evidence_sha256: "2".repeat(64),
            duplicate_of: None,
            deterministic_check_id: None,
            formatting_only: false,
            displayed: false,
            reason_code: "verified".into(),
        }
    }
    #[test]
    fn exact_review_and_shadow_fix_are_local_only() {
        let fix = LocalShadowFix {
            fix_id: "fix-1".into(),
            finding_id: "finding-1".into(),
            base_revision: "c".repeat(64),
            full_diff_sha256: "3".repeat(64),
            tests_sha256: "4".repeat(64),
            risks_sha256: "5".repeat(64),
            rollback_sha256: "6".repeat(64),
            controlled_write_proposal: true,
            applied: false,
            committed: false,
            pushed: false,
            published: false,
        };
        let p = build_local_pull_request_review(
            "packet-1",
            binding(),
            vec![finding("finding-1", ReviewFindingState::Current)],
            vec![fix],
            100,
            Some("7".repeat(64)),
            8000,
        )
        .unwrap();
        assert!(p.findings[0].displayed);
        assert!(!p.publication_enabled && !p.push_enabled);
    }
    #[test]
    fn stale_moved_resolved_duplicate_and_low_confidence_are_suppressed() {
        let states = [
            ReviewFindingState::Stale,
            ReviewFindingState::Moved,
            ReviewFindingState::Resolved,
            ReviewFindingState::Superseded,
            ReviewFindingState::Conflicting,
            ReviewFindingState::Uncertain,
            ReviewFindingState::LocallyUnverifiable,
        ];
        let mut values: Vec<_> = states
            .into_iter()
            .enumerate()
            .map(|(n, s)| finding(&format!("finding-{n}"), s))
            .collect();
        let mut duplicate = finding("duplicate-1", ReviewFindingState::Current);
        duplicate.duplicate_of = Some("finding-0".into());
        values.push(duplicate);
        let mut low = finding("low-1", ReviewFindingState::Current);
        low.confidence_basis_points = 100;
        values.push(low);
        let p =
            build_local_pull_request_review("packet-1", binding(), values, vec![], 100, None, 8000)
                .unwrap();
        assert!(p.findings.iter().all(|f| !f.displayed));
    }
    #[test]
    fn moved_head_and_effectful_fix_fail_closed() {
        let mut f = finding("finding-1", ReviewFindingState::Current);
        f.commit_revision = "8".repeat(64);
        assert_eq!(
            build_local_pull_request_review(
                "packet-1",
                binding(),
                vec![f],
                vec![],
                100,
                None,
                8000
            ),
            Err(PullRequestReviewError::IdentityMismatch)
        );
        let mut fix = LocalShadowFix {
            fix_id: "fix-1".into(),
            finding_id: "finding-1".into(),
            base_revision: "c".repeat(64),
            full_diff_sha256: "3".repeat(64),
            tests_sha256: "4".repeat(64),
            risks_sha256: "5".repeat(64),
            rollback_sha256: "6".repeat(64),
            controlled_write_proposal: true,
            applied: false,
            committed: false,
            pushed: true,
            published: false,
        };
        assert!(
            build_local_pull_request_review(
                "packet-1",
                binding(),
                vec![finding("finding-1", ReviewFindingState::Current)],
                vec![fix.clone()],
                100,
                None,
                8000
            )
            .is_err()
        );
        fix.pushed = false;
        fix.published = true;
        assert!(
            build_local_pull_request_review(
                "packet-1",
                binding(),
                vec![finding("finding-1", ReviewFindingState::Current)],
                vec![fix],
                100,
                None,
                8000
            )
            .is_err()
        );
    }
    #[test]
    fn malicious_paths_and_mutated_checkout_are_rejected() {
        let mut f = finding("finding-1", ReviewFindingState::Current);
        f.path = "../escape".into();
        assert!(
            build_local_pull_request_review(
                "packet-1",
                binding(),
                vec![f],
                vec![],
                100,
                None,
                8000
            )
            .is_err()
        );
        let mut b = binding();
        b.active_checkout_unchanged = false;
        assert!(
            build_local_pull_request_review(
                "packet-1",
                b,
                vec![finding("finding-1", ReviewFindingState::Current)],
                vec![],
                100,
                None,
                8000
            )
            .is_err()
        );
    }
}
