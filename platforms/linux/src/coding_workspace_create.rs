// SPDX-License-Identifier: Apache-2.0
//! Private, exact absent-file plans for the live worktree owner's original apply.

use super::*;
use crate::{LinuxControlledFilesystemDriver, LinuxFilesystemDriverLimits};
use agentmage_kernel_contracts::{AuthorizedWorkspaceHandle, WorkspaceScopePath};
use agentmage_kernel_engine::filesystem_control::{
    ControlledFilesystemDriver, FileClassification, FilesystemApprovalDecision,
    FilesystemApprovalPreview, FilesystemGrantRequest, FilesystemOperationDraft, FilesystemPlan,
    FilesystemPlanRequest, FilesystemTransactionOutcome, FilesystemTransactionRequest,
    FilesystemTransactionResult, NewDestinationDraft, build_filesystem_plan,
    execute_filesystem_transaction, issue_filesystem_grant, render_filesystem_preview,
};
use agentmage_kernel_engine::filesystem_control::{
    FilesystemApplyAuthorization, FilesystemApplyReport, FilesystemDriverError,
    FilesystemOperationObservation, FilesystemRestoreAuthorization, FilesystemRestoreReport,
};

pub(crate) struct LinuxCodingPendingCreate {
    issuer: GrantIssuer,
    policy: PolicyEngine,
    parent: GrantId,
    plan: FilesystemPlan,
    preview: FilesystemApprovalPreview,
    expiry: u64,
    deadline: Instant,
    root: String,
    sibling_names: Vec<Vec<String>>,
}

struct CreationPreflight<'a, D> {
    native: &'a mut D,
    sibling_names: &'a [Vec<String>],
    applied: bool,
}
impl<D: ControlledFilesystemDriver> ControlledFilesystemDriver for CreationPreflight<'_, D> {
    fn observe(
        &mut self,
        plan: &FilesystemPlan,
    ) -> Result<Vec<FilesystemOperationObservation>, FilesystemDriverError> {
        let observations = self.native.observe(plan)?;
        if !self.applied
            && (observations.len() != self.sibling_names.len()
                || observations
                    .iter()
                    .zip(self.sibling_names)
                    .any(|(o, names)| o.destination_sibling_names != *names))
        {
            return Err(FilesystemDriverError::ObservationUnavailable);
        }
        Ok(observations)
    }
    fn apply(&mut self, authorization: FilesystemApplyAuthorization<'_>) -> FilesystemApplyReport {
        self.applied = true;
        self.native.apply(authorization)
    }
    fn restore(
        &mut self,
        authorization: FilesystemRestoreAuthorization<'_>,
    ) -> FilesystemRestoreReport {
        self.native.restore(authorization)
    }
}

impl LinuxCodingReadScope {
    // Only the live owner calls this after checking absent-before lineage.
    // The internal scope contains canonical paths but promises no existing read.
    pub(crate) fn prepare_create(
        &self,
        files: &[(&str, &str, u32)],
        intent: &str,
        now: u64,
    ) -> Result<LinuxCodingPendingCreate, LinuxCodingWriteError> {
        use LinuxCodingWriteError::{Boundary, Input};
        if files.is_empty() || files.len() > MAX_FILES || intent.is_empty() || intent.len() > 4096 {
            return Err(Input);
        }
        let expiry = now.checked_add(LIFETIME_MS).ok_or(Input)?;
        let deadline = Instant::now() + Duration::from_millis(LIFETIME_MS);
        let mut nonce = [0; 16];
        if rustix::rand::getrandom(&mut nonce, rustix::rand::GetRandomFlags::empty()) != Ok(16) {
            return Err(Input);
        }
        let id = digest(&nonce);
        let mut operations = Vec::new();
        let mut sibling_names = Vec::new();
        let mut targets = Vec::new();
        let mut scopes = Vec::new();
        let mut seen = BTreeSet::new();
        let mut total = 0;
        for (i, &(path, content, mode)) in files.iter().enumerate() {
            let canonical = self.paths.get(path).ok_or(Input)?;
            total += content.len();
            if !seen.insert(path)
                || content.is_empty()
                || content.contains('\0')
                || total > MAX_BYTES
            {
                return Err(Input);
            }
            let components = canonical.components();
            let parents = &components[..components.len() - 1];
            let (parent, siblings) = if parents.is_empty() {
                (
                    GrantTarget::held_workspace_root(&self.workspace).map_err(|_| Boundary)?,
                    self.workspace
                        .observe_root_names(4096, 1024 * 1024)
                        .map_err(|_| Boundary)?,
                )
            } else {
                let parent_path = agentmage_kernel_contracts::WorkspacePath::new(
                    self.workspace.workspace_id().clone(),
                    parents.iter().map(|component| component.as_str()),
                )
                .map_err(|_| Input)?;
                let held = self
                    .adapter
                    .resolve(
                        &self.workspace,
                        &parent_path,
                        PathResolutionIntent::ReadDirectory,
                    )
                    .map_err(|_| Boundary)?;
                (
                    GrantTarget::held_object(&held).map_err(|_| Boundary)?,
                    held.observe_directory_names(4096, 1024 * 1024)
                        .map_err(|_| Boundary)?,
                )
            };
            let scope = GrantTarget::workspace_scope(
                &self.workspace,
                WorkspaceScopePath::new(
                    self.workspace.workspace_id().clone(),
                    parents.iter().map(|component| component.as_str()),
                )
                .map_err(|_| Input)?,
            )
            .map_err(|_| Boundary)?;
            if !scopes.contains(&scope) {
                scopes.push(scope);
            }
            if !targets.contains(&parent) {
                targets.push(parent.clone());
            }
            sibling_names.push(siblings.clone());
            operations.push(FilesystemOperationDraft::Create {
                operation_id: format!("create-{i}"),
                destination: NewDestinationDraft {
                    parent,
                    path: canonical.clone(),
                    observed_sibling_names: siblings,
                },
                content: content.as_bytes().to_vec(),
                mode,
                classification: FileClassification::SourceCode,
            });
        }
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("local-control")]),
            tasks: rules([TaskId::from_raw(&id)]),
            actions: rules([ActionId::from_raw("local-coding-create")]),
            tools: rules([ToolPolicyBinding {
                tool_id: ToolId::from_raw("local-coding-create"),
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
                issued_at_epoch_ms: now,
                expires_at_epoch_ms: expiry,
                nonce: GrantNonce::from_raw(format!("read-{id}")),
                maximum_derived_operations: 1,
                preview_sha256: digest(self.workspace.root_path_sha256().as_bytes()),
                policy_sha256: policy.policy_sha256().into(),
            })
            .map_err(|_| Input)?;
        let plan = build_filesystem_plan(
            &parent,
            FilesystemPlanRequest {
                plan_id: id,
                observed_at_epoch_ms: now,
                operations,
                review: WriteReviewNarrative {
                    rationale: intent.into(),
                    behavior_change: "Create only the displayed absent files, contents and modes"
                        .into(),
                    verification_plan: vec!["Native exact postimage and mode verification".into()],
                    risks: vec![
                        "Not compiled or tested; process-only ownership and previews".into(),
                    ],
                    rollback: "Inspect disk after interruption; no automatic replay".into(),
                    unverified_assumptions: vec!["No command, commit or push is authorized".into()],
                },
                permitted_verification: vec!["project-tests".into()],
            },
        )
        .map_err(|_| Input)?;
        let preview = render_filesystem_preview(&plan).map_err(|_| Input)?;
        // Final effect-free native observation checks absence and the held parent.
        let mut driver = LinuxControlledFilesystemDriver::new(&self.workspace, limits());
        let observed =
            agentmage_kernel_engine::filesystem_control::ControlledFilesystemDriver::observe(
                &mut driver,
                &plan,
            )
            .map_err(|_| Boundary)?;
        if observed.iter().any(|o| o.destination.is_some()) {
            return Err(Input);
        }
        Ok(LinuxCodingPendingCreate {
            issuer,
            policy,
            parent: parent.grant_id,
            plan,
            preview,
            expiry,
            deadline,
            root: self.root_sha256().into(),
            sibling_names,
        })
    }
}

fn limits() -> LinuxFilesystemDriverLimits {
    LinuxFilesystemDriverLimits {
        maximum_operations: MAX_FILES,
        maximum_file_bytes: MAX_BYTES as u64,
        maximum_transaction_bytes: (MAX_BYTES * 2) as u64,
        ..LinuxFilesystemDriverLimits::default()
    }
}

impl LinuxCodingPendingCreate {
    pub(crate) fn preview(&self) -> Value {
        json!(self.preview)
    }
    pub(crate) fn expires_at_epoch_ms(&self) -> u64 {
        self.expiry
    }
    pub(crate) fn apply(
        self,
        scope: &LinuxCodingReadScope,
        now: u64,
    ) -> Result<Value, LinuxCodingWriteError> {
        let mut driver = LinuxControlledFilesystemDriver::new(&scope.workspace, limits());
        let result = self.execute(scope, now, &mut driver)?;
        Ok(json!({"outcome": match result.outcome {
            FilesystemTransactionOutcome::Committed => "committed", FilesystemTransactionOutcome::FailedNoChange => "failed_no_change",
            FilesystemTransactionOutcome::Restored => "restored", FilesystemTransactionOutcome::Uncertain => "uncertain",
        }, "receipts": result.receipts, "consumed_grant_sha256": result.grant_consumption.consumed_grant_sha256,
        "verification_requirements":result.verification_requirements,
        "tested":false,"isolation_verified":false,"retention":"process-only"}))
    }
    fn execute<D: ControlledFilesystemDriver>(
        mut self,
        scope: &LinuxCodingReadScope,
        now: u64,
        driver: &mut D,
    ) -> Result<FilesystemTransactionResult, LinuxCodingWriteError> {
        use LinuxCodingWriteError::{Approval, Transaction};
        if now >= self.expiry || Instant::now() >= self.deadline || scope.root_sha256() != self.root
        {
            return Err(Approval);
        }
        let id = self.plan.plan_id();
        let approval = issue_filesystem_grant(
            &mut self.issuer,
            &self.plan,
            &self.preview,
            &FilesystemApprovalDecision {
                approval_id: ApprovalId::from_raw(format!("approval-{id}")),
                approved_plan_sha256: self.plan.plan_sha256().into(),
                approved_preview_sha256: self.preview.preview_sha256.clone(),
                approved_at_epoch_ms: now,
                expires_at_epoch_ms: self.expiry,
                permitted_verification: self.plan.permitted_verification().to_vec(),
                user_confirmed: true,
                high_risk_delete_confirmed: false,
            },
            FilesystemGrantRequest {
                parent_grant_id: self.parent,
                grant_id: GrantId::from_raw(format!("create-{id}")),
                action_id: ActionId::from_raw("local-coding-create"),
                action_kind: ActionKind::DeterministicTool,
                tool_id: ToolId::from_raw("local-coding-create"),
                tool_version: "1.0.0".into(),
                nonce: GrantNonce::from_raw(format!("create-{id}")),
                policy_sha256: self.policy.policy_sha256().into(),
            },
        )
        .map_err(|_| Approval)?;
        let mut checked = CreationPreflight {
            native: driver,
            sibling_names: &self.sibling_names,
            applied: false,
        };
        execute_filesystem_transaction(
            &mut self.issuer,
            &self.policy,
            &self.plan,
            &approval,
            FilesystemTransactionRequest {
                transaction_id: format!("transaction-{id}"),
                now_epoch_ms: now,
                cancelled_before_consume: false,
            },
            &mut checked,
        )
        .map_err(|_| Transaction)
    }

    #[cfg(test)]
    pub(crate) fn apply_with_owned_driver(
        self,
        scope: &LinuxCodingReadScope,
        lease: &crate::LinuxCodingWorktreeLease,
        now: u64,
        publish: bool,
        alter: Option<fn(&mut FilesystemTransactionResult)>,
    ) -> Result<FilesystemTransactionResult, LinuxCodingWriteError> {
        let mut driver = lease
            .create_driver(&scope.workspace, limits())
            .map_err(|_| LinuxCodingWriteError::Boundary)?;
        let mut result = self.execute(scope, now, &mut driver)?;
        if let Some(alter) = alter {
            alter(&mut result);
        }
        if publish {
            driver
                .complete(&result)
                .map_err(|_| LinuxCodingWriteError::Transaction)?;
        }
        Ok(result)
    }
}
