//! Exact child-authority intersection, isolation, ownership, and bounded cancellation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Closed authority atom understood by the shared runtime.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildAuthorityAtom {
    /// Observe one exact declared root.
    ReadRoot(String),
    /// Write one exact owned path.
    WritePath(String),
    /// Invoke one exact shared tool.
    Tool(String),
    /// Use one exact admitted model profile.
    Model(String),
    /// Contact one exact network destination.
    Network(String),
}

/// Four independent ceilings and deny sets for one child.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildAuthorityRequest {
    /// User ceiling.
    pub user: BTreeSet<ChildAuthorityAtom>,
    /// Parent ceiling; it cannot pool sibling grants.
    pub parent: BTreeSet<ChildAuthorityAtom>,
    /// Exact task ceiling.
    pub task: BTreeSet<ChildAuthorityAtom>,
    /// Explicit child ceiling.
    pub child: BTreeSet<ChildAuthorityAtom>,
    /// User denials.
    pub user_denies: BTreeSet<ChildAuthorityAtom>,
    /// Parent denials.
    pub parent_denies: BTreeSet<ChildAuthorityAtom>,
    /// Task denials.
    pub task_denies: BTreeSet<ChildAuthorityAtom>,
    /// Child-profile denials.
    pub child_denies: BTreeSet<ChildAuthorityAtom>,
}

/// Independent resource ceilings for one child.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildResourceLimits {
    /// Maximum children beneath this child.
    pub child_count: u16,
    /// Maximum nesting depth.
    pub nesting_depth: u8,
    /// Maximum turns.
    pub turns: u32,
    /// Maximum tools.
    pub tools: u16,
    /// Maximum processes.
    pub processes: u16,
    /// Maximum memory bytes.
    pub memory_bytes: u64,
    /// Maximum model loads.
    pub model_loads: u16,
    /// Maximum network operations.
    pub network_operations: u16,
    /// Maximum output bytes.
    pub output_bytes: u64,
}

/// Exact isolated child-state identities and owned writable paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildIsolationManifest {
    /// Child identity.
    pub child_id: String,
    /// Parent identity.
    pub parent_id: String,
    /// Memory partition digest.
    pub memory_sha256: String,
    /// Conversation partition digest.
    pub conversation_sha256: String,
    /// Temporary-root digest.
    pub temporary_root_sha256: String,
    /// Receipt partition digest.
    pub receipt_root_sha256: String,
    /// Output partition digest.
    pub output_root_sha256: String,
    /// Exclusive writable worktree digest.
    pub worktree_sha256: String,
    /// Exact owned paths.
    pub owned_paths: BTreeSet<String>,
    /// Independent ceilings.
    pub limits: ChildResourceLimits,
}

/// One child result remains a proposal without authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildProposal {
    /// Attributable child.
    pub child_id: String,
    /// Immutable result digest.
    pub result_sha256: String,
    /// Immutable receipt digest.
    pub receipt_sha256: String,
    /// Parent review is still required.
    pub parent_review_required: bool,
    /// Result carries no authority atoms.
    pub authority: BTreeSet<ChildAuthorityAtom>,
}

/// Closed descendant control state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildControlState {
    /// May continue.
    Active,
    /// Paused by an ancestor.
    Paused,
    /// Cancelled by an ancestor.
    Cancelled,
    /// Grant expired.
    Expired,
    /// Terminated and cleaned.
    Terminated,
}

/// Stable child-admission failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildAuthorityError {
    /// An identity, digest, path, or bound is invalid.
    InvalidInput,
    /// Another child already owns a requested path or worktree.
    OwnershipConflict,
    /// A result attempted to become authority or self-review.
    ProposalRejected,
}

/// Computes exact four-way intersection with deny precedence.
#[must_use]
pub fn effective_child_authority(request: &ChildAuthorityRequest) -> BTreeSet<ChildAuthorityAtom> {
    let denied = request
        .user_denies
        .union(&request.parent_denies)
        .cloned()
        .collect::<BTreeSet<_>>()
        .union(&request.task_denies)
        .cloned()
        .collect::<BTreeSet<_>>()
        .union(&request.child_denies)
        .cloned()
        .collect::<BTreeSet<_>>();
    request
        .user
        .intersection(&request.parent)
        .filter(|atom| request.task.contains(*atom) && request.child.contains(*atom))
        .filter(|atom| !denied.contains(*atom))
        .cloned()
        .collect()
}

/// Validates all isolation identities, limits, and exclusive ownership.
pub fn admit_child_isolation(
    candidate: &ChildIsolationManifest,
    admitted: &[ChildIsolationManifest],
) -> Result<(), ChildAuthorityError> {
    let hashes = [
        &candidate.memory_sha256,
        &candidate.conversation_sha256,
        &candidate.temporary_root_sha256,
        &candidate.receipt_root_sha256,
        &candidate.output_root_sha256,
        &candidate.worktree_sha256,
    ];
    if !valid_id(&candidate.child_id)
        || !valid_id(&candidate.parent_id)
        || candidate.child_id == candidate.parent_id
        || !hashes.into_iter().all(|value| valid_sha256(value))
        || candidate.owned_paths.is_empty()
        || candidate.owned_paths.iter().any(|path| !valid_path(path))
        || !valid_limits(&candidate.limits)
    {
        return Err(ChildAuthorityError::InvalidInput);
    }
    if admitted.iter().any(|other| {
        other.child_id == candidate.child_id
            || other.worktree_sha256 == candidate.worktree_sha256
            || !other.owned_paths.is_disjoint(&candidate.owned_paths)
    }) {
        return Err(ChildAuthorityError::OwnershipConflict);
    }
    Ok(())
}

/// Converts one attributable child result to an authority-free proposal.
pub fn child_proposal(
    child_id: String,
    result_sha256: String,
    receipt_sha256: String,
) -> Result<ChildProposal, ChildAuthorityError> {
    if !valid_id(&child_id)
        || !valid_sha256(&result_sha256)
        || !valid_sha256(&receipt_sha256)
        || result_sha256 == receipt_sha256
    {
        return Err(ChildAuthorityError::ProposalRejected);
    }
    Ok(ChildProposal {
        child_id,
        result_sha256,
        receipt_sha256,
        parent_review_required: true,
        authority: BTreeSet::new(),
    })
}

/// Applies an ancestor control transition to every descendant deterministically.
#[must_use]
pub fn propagate_child_control(
    descendants: &BTreeMap<String, ChildControlState>,
    requested: ChildControlState,
) -> BTreeMap<String, ChildControlState> {
    let terminal = match requested {
        ChildControlState::Active => ChildControlState::Active,
        ChildControlState::Paused => ChildControlState::Paused,
        ChildControlState::Cancelled
        | ChildControlState::Expired
        | ChildControlState::Terminated => ChildControlState::Terminated,
    };
    descendants
        .keys()
        .map(|id| (id.clone(), terminal))
        .collect()
}

fn valid_limits(value: &ChildResourceLimits) -> bool {
    value.child_count <= 5
        && value.nesting_depth <= 3
        && value.turns > 0
        && value.turns <= 1_000
        && value.tools <= 64
        && value.processes <= 32
        && value.memory_bytes > 0
        && value.memory_bytes <= 16 * 1024 * 1024 * 1024
        && value.model_loads <= 8
        && value.network_operations <= 256
        && value.output_bytes > 0
        && value.output_bytes <= 1024 * 1024 * 1024
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.starts_with('/')
        && !value
            .split('/')
            .any(|component| matches!(component, "" | "." | ".."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(name: &str) -> ChildAuthorityAtom {
        ChildAuthorityAtom::Tool(name.to_owned())
    }

    fn manifest(child: &str, worktree: char, path: &str) -> ChildIsolationManifest {
        ChildIsolationManifest {
            child_id: child.to_owned(),
            parent_id: "parent".to_owned(),
            memory_sha256: "1".repeat(64),
            conversation_sha256: "2".repeat(64),
            temporary_root_sha256: "3".repeat(64),
            receipt_root_sha256: "4".repeat(64),
            output_root_sha256: "5".repeat(64),
            worktree_sha256: worktree.to_string().repeat(64),
            owned_paths: [path.to_owned()].into_iter().collect(),
            limits: ChildResourceLimits {
                child_count: 0,
                nesting_depth: 1,
                turns: 8,
                tools: 4,
                processes: 1,
                memory_bytes: 1024,
                model_loads: 1,
                network_operations: 0,
                output_bytes: 4096,
            },
        }
    }

    #[test]
    fn authority_is_exact_intersection_with_deny_precedence() {
        let request = ChildAuthorityRequest {
            user: [atom("read"), atom("write")].into_iter().collect(),
            parent: [atom("read"), atom("write"), atom("extra")]
                .into_iter()
                .collect(),
            task: [atom("read"), atom("write")].into_iter().collect(),
            child: [atom("read"), atom("write")].into_iter().collect(),
            user_denies: BTreeSet::new(),
            parent_denies: [atom("write")].into_iter().collect(),
            task_denies: BTreeSet::new(),
            child_denies: BTreeSet::new(),
        };
        assert_eq!(
            effective_child_authority(&request),
            [atom("read")].into_iter().collect()
        );
    }

    #[test]
    fn siblings_cannot_pool_or_collide() {
        let first = manifest("child-one", 'a', "src/one.rs");
        let same_path = manifest("child-two", 'b', "src/one.rs");
        let same_worktree = manifest("child-three", 'a', "src/three.rs");
        assert_eq!(admit_child_isolation(&first, &[]), Ok(()));
        assert_eq!(
            admit_child_isolation(&same_path, std::slice::from_ref(&first)),
            Err(ChildAuthorityError::OwnershipConflict)
        );
        assert_eq!(
            admit_child_isolation(&same_worktree, &[first]),
            Err(ChildAuthorityError::OwnershipConflict)
        );
    }

    #[test]
    fn invalid_limits_and_paths_fail_closed() {
        let mut candidate = manifest("child", 'a', "src/one.rs");
        candidate.limits.nesting_depth = 4;
        assert_eq!(
            admit_child_isolation(&candidate, &[]),
            Err(ChildAuthorityError::InvalidInput)
        );
        candidate = manifest("child", 'a', "../escape");
        assert_eq!(
            admit_child_isolation(&candidate, &[]),
            Err(ChildAuthorityError::InvalidInput)
        );
    }

    #[test]
    fn results_are_untrusted_authority_free_proposals() {
        let proposal = child_proposal("child".to_owned(), "a".repeat(64), "b".repeat(64)).unwrap();
        assert!(proposal.parent_review_required);
        assert!(proposal.authority.is_empty());
    }

    #[test]
    fn ancestor_controls_reach_every_descendant() {
        let descendants = [
            ("one".to_owned(), ChildControlState::Active),
            ("two".to_owned(), ChildControlState::Paused),
        ]
        .into_iter()
        .collect();
        let stopped = propagate_child_control(&descendants, ChildControlState::Cancelled);
        assert_eq!(stopped.len(), 2);
        assert!(
            stopped
                .values()
                .all(|state| *state == ChildControlState::Terminated)
        );
    }
}
