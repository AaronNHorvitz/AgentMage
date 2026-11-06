// SPDX-License-Identifier: Apache-2.0
//! Exact pending edits mediated by the existing kernel and native write driver.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, DataSensitivity, GrantId, GrantNonce,
    GrantOperation, GrantTarget, OperationBinding, PathResolutionIntent, PlatformPathAdapter,
    SessionId, TaskId, ToolId,
};
use agentmage_kernel_engine::grants::{GrantIssuer, SessionReadGrantRequest};
use agentmage_kernel_engine::policy::{
    PolicyDocument, PolicyEngine, ScopeRules, ToolPolicyBinding,
};
use agentmage_kernel_engine::write_approval::{
    ShadowChangeSet, ShadowChangeSetRequest, ShadowWriteDraft, WriteApprovalDecision,
    WriteApprovalPreview, WriteArtifactClass, WriteChangeScope, WriteGrantRequest,
    WriteLineEndings, WriteReviewNarrative, WriteSyntax, build_shadow_change_set,
    issue_write_grant, render_write_preview,
};
use agentmage_kernel_engine::write_transaction::{
    WriteTransactionOutcome, WriteTransactionRequest, execute_write_transaction,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{LinuxCodingReadScope, MAX_BYTES, MAX_FILES};
use crate::{LinuxAtomicWriteDriver, LinuxAtomicWriteDriverLimits};

const LIFETIME_MS: u64 = 120_000;

/// Stable refusal without source, absolute path or operating-system diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingWriteError {
    /// Invalid replacement, resource bound or original digest.
    Input,
    /// Current native target could not be safely observed.
    Boundary,
    /// Expired, mismatched or unavailable exact approval.
    Approval,
    /// The kernel transaction failed; callers must not infer an unchanged disk.
    Transaction,
}

impl LinuxCodingWriteError {
    /// Content-free code for clients and activity records.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Input => "code.write.input",
            Self::Boundary => "code.write.boundary",
            Self::Approval => "code.write.approval",
            Self::Transaction => "code.write.transaction",
        }
    }
}

/// Exact already-checked candidate bytes; this value confers no write authority.
pub struct LinuxCodingReplacement<'a> {
    /// Canonical selected relative path.
    pub path: &'a str,
    /// Digest of the complete original snapshot seen by the editor.
    pub before_sha256: &'a str,
    /// Complete candidate UTF-8 contents.
    pub content: &'a str,
}

/// Non-cloneable, process-only shadow edit. No write grant exists until `apply`.
/// Dropping this object is inert. Applying consumes it even on a refusal.
pub struct LinuxCodingPendingWrite {
    issuer: GrantIssuer,
    policy: PolicyEngine,
    parent: GrantId,
    change_set: ShadowChangeSet,
    preview: WriteApprovalPreview,
    expires_at_epoch_ms: u64,
    deadline: Instant,
    workspace_root_sha256: String,
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
    ScopeRules {
        allowed: values.into_iter().collect(),
        denied: BTreeSet::new(),
    }
}

impl LinuxCodingReadScope {
    /// Prepares an effect-free kernel shadow change for selected existing files.
    /// This does not assert Git/worktree ownership or confer command authority.
    /// Only call `apply` after a trusted person approves the complete preview.
    pub fn prepare_write(
        &self,
        replacements: &[LinuxCodingReplacement<'_>],
        intent: &str,
        now_epoch_ms: u64,
    ) -> Result<LinuxCodingPendingWrite, LinuxCodingWriteError> {
        use LinuxCodingWriteError::{Boundary, Input};
        if replacements.is_empty()
            || replacements.len() > MAX_FILES
            || intent.is_empty()
            || intent.len() > 4096
        {
            return Err(Input);
        }
        let expiry = now_epoch_ms.checked_add(LIFETIME_MS).ok_or(Input)?;
        let deadline = Instant::now() + Duration::from_millis(LIFETIME_MS);
        let mut random = [0_u8; 16];
        if rustix::rand::getrandom(&mut random, rustix::rand::GetRandomFlags::empty()) != Ok(16) {
            return Err(Input);
        }
        let id = digest(&random);
        let mut drafts = Vec::new();
        let mut selected = BTreeSet::new();
        let mut total = 0;
        let mut scopes = Vec::new();
        for (index, replacement) in replacements.iter().enumerate() {
            let path = self.paths.get(replacement.path).ok_or(Input)?;
            if !selected.insert(replacement.path)
                || replacement.content.contains('\0')
                || replacement.content.len() > MAX_BYTES
            {
                return Err(Input);
            }
            let held = self
                .adapter
                .resolve(&self.workspace, path, PathResolutionIntent::ReadFile)
                .map_err(|_| Boundary)?;
            let before = held.read_exact_bytes().map_err(|_| Boundary)?;
            if digest(&before) != replacement.before_sha256 {
                return Err(Input);
            }
            total += before.len() + replacement.content.len();
            if total > MAX_BYTES * 2 {
                return Err(Input);
            }
            let after = replacement.content;
            let line_endings = if after.contains('\r') {
                WriteLineEndings::CrLf
            } else if after.contains('\n') {
                WriteLineEndings::Lf
            } else {
                WriteLineEndings::None
            };
            // Read parent scope is exactly each selected leaf, never the root.
            scopes.push(
                GrantTarget::workspace_scope(&self.workspace, path.as_scope())
                    .map_err(|_| Boundary)?,
            );
            drafts.push(ShadowWriteDraft {
                operation_id: format!("edit-{index}"),
                target: GrantTarget::held_object(&held).map_err(|_| Boundary)?,
                observed_bytes: before,
                proposed_bytes: after.as_bytes().to_vec(),
                expected_postimage_sha256: digest(after.as_bytes()),
                artifact_class: WriteArtifactClass::Code,
                syntax: WriteSyntax::Utf8,
                line_endings,
                generated_file: false,
                allow_generated_file: false,
            });
        }
        let targets = drafts.iter().map(|d| d.target.clone()).collect::<Vec<_>>();
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("local-control")]),
            tasks: rules([TaskId::from_raw(&id)]),
            actions: rules([ActionId::from_raw("local-coding-write")]),
            tools: rules([ToolPolicyBinding {
                tool_id: ToolId::from_raw("local-coding-write"),
                tool_version: "1.0.0".into(),
            }]),
            operations: rules([OperationBinding::new(GrantOperation::WorkspaceWrite)]),
            targets: rules(targets),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .map_err(|_| Input)?;
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw(format!("read-{id}")),
                actor_id: ActorId::from_raw("local-control"),
                session_id: SessionId::from_raw(&id),
                task_id: TaskId::from_raw(&id),
                targets: scopes,
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: now_epoch_ms,
                expires_at_epoch_ms: expiry,
                nonce: GrantNonce::from_raw(format!("read-{id}")),
                maximum_derived_operations: 1,
                preview_sha256: digest(self.workspace.root_path_sha256().as_bytes()),
                policy_sha256: policy.policy_sha256().into(),
            })
            .map_err(|_| Input)?;
        let change_set = build_shadow_change_set(
            &parent,
            ShadowChangeSetRequest {
                change_set_id: id,
                observed_at_epoch_ms: now_epoch_ms,
                intent_sha256: digest(intent.as_bytes()),
                plan_sha256: digest(
                    &serde_json::to_vec(&(intent, self.workspace.root_path_sha256()))
                        .map_err(|_| Input)?,
                ),
                scope: WriteChangeScope::Minimal,
                expanded_scope_approval_sha256: None,
                review_hooks: Vec::new(),
                operations: drafts,
                review: WriteReviewNarrative {
                    rationale: "Apply the exact reviewed source replacements".into(),
                    behavior_change:
                        "Only the displayed existing files receive the displayed contents".into(),
                    verification_plan: vec!["Exact postimages checked by the native driver".into()],
                    risks: vec![
                        "These edits have not been compiled or tested".into(),
                        "The chosen workspace is not verified as an isolated Git worktree".into(),
                        "Records are process-only; a crash requires manual inspection".into(),
                    ],
                    rollback:
                        "A new preview and approval are required to restore original contents"
                            .into(),
                    unverified_assumptions: vec![
                        "No command, test, syntax, Git or isolation validation has run".into(),
                    ],
                },
                permitted_verification: Vec::new(),
            },
        )
        .map_err(|_| Input)?;
        let preview = render_write_preview(&change_set).map_err(|_| Input)?;
        Ok(LinuxCodingPendingWrite {
            issuer,
            policy,
            parent: parent.grant_id,
            change_set,
            preview,
            expires_at_epoch_ms: expiry,
            deadline,
            workspace_root_sha256: self.workspace.root_path_sha256().into(),
        })
    }
}

impl LinuxCodingPendingWrite {
    /// Complete escaped before/after diff and its kernel-computed exact hashes.
    #[must_use]
    pub const fn preview(&self) -> &WriteApprovalPreview {
        &self.preview
    }

    /// Preview lifetime, also enforced with a monotonic deadline.
    #[must_use]
    pub const fn expires_at_epoch_ms(&self) -> u64 {
        self.expires_at_epoch_ms
    }

    /// Digest of the held root's selection identity, included in the plan binding.
    #[must_use]
    pub fn workspace_root_sha256(&self) -> &str {
        &self.workspace_root_sha256
    }

    /// Consumes a trusted explicit decision through kernel issuance, policy,
    /// one-use consumption and the existing descriptor-relative atomic driver.
    /// Caller must serialize scope replacement and retain the result before reply.
    /// Once dispatched, the bounded transaction completes/reconciles; it cannot
    /// be cancelled halfway through. No command or syntax success is implied.
    pub fn apply(
        mut self,
        scope: &LinuxCodingReadScope,
        change_set_sha256: &str,
        preview_sha256: &str,
        now_epoch_ms: u64,
    ) -> Result<Value, LinuxCodingWriteError> {
        use LinuxCodingWriteError::{Approval, Transaction};
        if now_epoch_ms >= self.expires_at_epoch_ms
            || Instant::now() >= self.deadline
            || scope.workspace.root_path_sha256() != self.workspace_root_sha256
        {
            return Err(Approval);
        }
        let id = self.change_set.change_set_id();
        let approval = issue_write_grant(
            &mut self.issuer,
            &self.change_set,
            &self.preview,
            &WriteApprovalDecision {
                approval_id: ApprovalId::from_raw(format!("approval-{id}")),
                approved_change_set_sha256: change_set_sha256.into(),
                approved_preview_sha256: preview_sha256.into(),
                approved_at_epoch_ms: now_epoch_ms,
                expires_at_epoch_ms: self.expires_at_epoch_ms,
                permitted_verification: Vec::new(),
                user_confirmed: true,
            },
            WriteGrantRequest {
                parent_grant_id: self.parent,
                grant_id: GrantId::from_raw(format!("write-{id}")),
                action_id: ActionId::from_raw("local-coding-write"),
                action_kind: ActionKind::DeterministicTool,
                tool_id: ToolId::from_raw("local-coding-write"),
                tool_version: "1.0.0".into(),
                nonce: GrantNonce::from_raw(format!("write-{id}")),
                policy_sha256: self.policy.policy_sha256().into(),
            },
        )
        .map_err(|_| Approval)?;
        let mut driver = LinuxAtomicWriteDriver::new(
            &scope.workspace,
            LinuxAtomicWriteDriverLimits {
                maximum_operations: MAX_FILES,
                maximum_file_bytes: MAX_BYTES as u64,
                maximum_transaction_bytes: (MAX_BYTES * 2) as u64,
            },
        );
        let result = execute_write_transaction(
            &mut self.issuer,
            &self.policy,
            &self.change_set,
            &approval,
            WriteTransactionRequest {
                transaction_id: format!("transaction-{id}"),
                now_epoch_ms,
            },
            &mut driver,
        )
        .map_err(|_| Transaction)?;
        Ok(json!({"outcome":match result.outcome {
            WriteTransactionOutcome::Committed => "committed",
            WriteTransactionOutcome::FailedNoChange => "failed_no_change",
            WriteTransactionOutcome::Restored => "restored",
            WriteTransactionOutcome::Uncertain => "uncertain",
        }, "receipts":result.receipts, "consumed_grant_sha256":result.grant_consumption.consumed_grant_sha256,
        "tested":false,"isolation_verified":false,"retention":"process-only"}))
    }
}
