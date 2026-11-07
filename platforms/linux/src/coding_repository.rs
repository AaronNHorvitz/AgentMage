// SPDX-License-Identifier: Apache-2.0
//! Approved worktree creation, held ownership, commands and exact source edits.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, AuthorityTransactionId,
    BoundaryKind, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, DataSensitivity,
    GrantId, GrantNonce, GrantOperation, GrantSideEffect, GrantTarget, OperationAttemptId,
    OperationBinding, PathResolutionIntent, PlatformPathAdapter, RequiredGrantTemplate, SchemaId,
    SchemaReference, SessionId, TaskId, ToolCall, ToolCallId, ToolDefinition, ToolId,
    ToolRiskLevel, ValidationIssue, ValidationSeverity, WorkspaceAuthorizationId, WorkspaceId,
    WorkspacePath,
};
use agentmage_kernel_engine::{
    authority_transaction::AuthorityTransactionRequest,
    grants::{DerivedOperationGrantRequest, SessionReadGrantRequest},
    operational_store::{OperationalStoreKeyError, OperationalStoreKeyProvider},
    policy::{
        PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules, ToolPolicyBinding,
    },
    propagation::CancellationToken,
    repository_safety::{
        PreparedRepositoryOperation, RepositoryEffectDriver, plan_worktree_create,
    },
    tooling::{Tool, ToolRegistry},
};
use hkdf::Hkdf;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{
    LinuxAuthorityRuntime, LinuxAuthorizedWorkspace, LinuxCodingReadScope,
    LinuxDevelopmentPlatformAdapter, LinuxGitArtifact, LinuxHeldObject, LinuxPathAdapter,
    LinuxRepositoryCollector, LinuxRepositoryExecutor, LinuxRepositoryScope,
    LinuxStrictLocalRootInspector, authorize_workspace_root, open_linux_development_authority,
};

#[path = "coding_repository_apply.rs"]
mod apply;
#[path = "coding_repository_command.rs"]
mod command;
#[path = "coding_repository_edit.rs"]
mod edit;
#[path = "coding_repository_validation.rs"]
mod validation;
pub use command::LinuxCodingCommandCancellation;
pub use validation::LinuxCodingTestFormat;

const TOOL: &str = "local-coding-worktree-create";

#[derive(Clone)]
struct AppliedEdit {
    before_sha256: String,
    after_sha256: String,
}

struct CreatedWorktree {
    original_path: PathBuf,
    original: LinuxAuthorizedWorkspace,
    edits: BTreeMap<String, AppliedEdit>,
    path: PathBuf,
    held: LinuxAuthorizedWorkspace,
    owned: LinuxAuthorizedWorkspace,
    creation_receipt_sha256: String,
}
const LIFETIME_MS: u64 = 120_000;

/// Content-free refusal. After dispatch an error never establishes no effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxCodingRepositoryError(&'static str);
impl LinuxCodingRepositoryError {
    /// Stable public code without paths, credentials or Git output.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.0
    }
}
type Result<T> = std::result::Result<T, LinuxCodingRepositoryError>;
fn err(code: &'static str) -> LinuxCodingRepositoryError {
    LinuxCodingRepositoryError(code)
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
    ScopeRules {
        allowed: values.into_iter().collect(),
        denied: BTreeSet::new(),
    }
}

struct Key(Zeroizing<[u8; 32]>);
impl OperationalStoreKeyProvider for Key {
    fn with_key<T>(
        &mut self,
        operation: impl FnOnce(&[u8]) -> T,
    ) -> std::result::Result<T, OperationalStoreKeyError> {
        Ok(operation(&*self.0))
    }
}

struct Pending {
    prepared: PreparedRepositoryOperation,
    before: agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest,
    scope: LinuxRepositoryScope,
    collector: LinuxRepositoryCollector,
    repository: LinuxAuthorizedWorkspace,
    metadata: LinuxHeldObject,
    owned: LinuxAuthorizedWorkspace,
    root: PathBuf,
    destination: PathBuf,
    preview: Value,
    hash: String,
    created_at: u64,
    expires_at: u64,
    deadline: Instant,
}

/// Non-production local coding repository owner. Uses the existing encrypted
/// authority store; previews are process-only and are never resumed or replayed.
/// Exact commands use the existing bounded executor and exact edits the native
/// write transaction. No model loop, commit, push or deletion.
pub struct LinuxCodingRepository {
    authority: LinuxAuthorityRuntime,
    state_root: PathBuf,
    pending: Option<Pending>,
    created: Option<CreatedWorktree>,
    command: Option<command::PendingCommand>,
    validation: Option<validation::ValidationSeries>,
    edit: Option<edit::PendingEdit>,
    edit_result: Option<Value>,
    apply: Option<apply::PendingApply>,
    apply_result: Option<Value>,
}

impl LinuxCodingRepository {
    /// Opens an existing owner-only strict-local directory. The person explicitly
    /// authorizes state writes by starting this service. Key material is a random
    /// control secret (at least 32 bytes), domain-separated with HKDF; it must be
    /// retained unchanged to reopen the encrypted state. No key file is created.
    /// This is the existing coding-development activation, not production admission.
    pub fn open(state_root: &Path, control_secret: &[u8], now_epoch_ms: u64) -> Result<Self> {
        if control_secret.len() < 32 || control_secret.len() > 256 || !state_root.is_absolute() {
            return Err(err("code.repository.state"));
        }
        LinuxStrictLocalRootInspector::inspect(state_root)
            .map_err(|_| err("code.repository.state"))?;
        let mut key = Key(Zeroizing::new([0; 32]));
        Hkdf::<Sha256>::new(Some(b"agentmage-local-coding-state-v1"), control_secret)
            .expand(b"canonical-authority", &mut *key.0)
            .map_err(|_| err("code.repository.state"))?;
        let development = LinuxDevelopmentPlatformAdapter::activate(
            "coding-development-v1",
            hash(b"local-api-coding-development-v1"),
            AdapterInstanceId::from_raw("local-coding-repository"),
        )
        .map_err(|_| err("code.repository.state"))?;
        let authority =
            open_linux_development_authority(&development, state_root, &mut key, now_epoch_ms)
                .map_err(|_| err("code.repository.state"))?;
        Ok(Self {
            authority,
            state_root: state_root.to_path_buf(),
            pending: None,
            created: None,
            command: None,
            validation: None,
            edit: None,
            edit_result: None,
            apply: None,
            apply_result: None,
        })
    }

    /// Shows an inert preview and the canonical content-free authority receipts.
    /// Recovery preserves uncertain outcomes; a receipt never reissues an approval.
    pub fn inspect(&self) -> Result<Value> {
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        let receipts = self.authority.authority().receipts();
        Ok(json!({"version":1,"retention":"canonical-authority",
            "pending_retention":"process-only", "pending":self.pending.as_ref().map(|p| &p.preview),
            "receipts":receipts.iter().rev().take(32).collect::<Vec<_>>(),
            "maximum_receipts":32,"history_truncated":receipts.len()>32,
            "tested":false,"automatic_replay":false}))
    }

    /// Explicit consent to inspect the entire chosen repository through the
    /// native pinned Git collector. This runs hardened local Git observations;
    /// callers must disclose command execution and full-tree reads before calling.
    /// The management directory must exist, be private and empty, outside the
    /// repository and state. Preparation never creates a branch or worktree.
    pub fn prepare(
        &mut self,
        root: &Path,
        owned_root: &Path,
        task: &str,
        base: &str,
        now: u64,
    ) -> Result<Value> {
        validate_input(root, owned_root, task, base)?;
        for path in [root, owned_root] {
            if path.starts_with(&self.state_root) || self.state_root.starts_with(path) {
                return Err(err("code.repository.scope"));
            }
        }
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        let repository = held_root(root)?;
        let owned = held_root(owned_root)?;
        LinuxStrictLocalRootInspector::inspect(owned_root)
            .map_err(|_| err("code.repository.scope"))?;
        if !owned
            .observe_root_names(1, 256)
            .map_err(|_| err("code.repository.scope"))?
            .is_empty()
        {
            return Err(err("code.repository.destination"));
        }
        let adapter = LinuxPathAdapter::new(AdapterInstanceId::from_raw(TOOL), 0);
        let metadata = adapter
            .resolve(
                &repository,
                &WorkspacePath::new(WorkspaceId::from_raw(TOOL), [".git"])
                    .map_err(|_| err("code.repository.scope"))?,
                PathResolutionIntent::Metadata,
            )
            .map_err(|_| err("code.repository.scope"))?;
        let scope = LinuxRepositoryScope::verify(root, root.join(".git"), owned_root)
            .map_err(|_| err("code.repository.scope"))?;
        let git = LinuxGitArtifact::verify("/usr/bin/git").map_err(|e| err(e.kind().code()))?;
        let collector = LinuxRepositoryCollector::new(git);
        let before = collector
            .collect(&scope)
            .map_err(|e| err(e.kind().code()))?;
        let mut random = [0; 16];
        if rustix::rand::getrandom(&mut random, rustix::rand::GetRandomFlags::empty()) != Ok(16) {
            return Err(err("code.repository.state"));
        }
        let transaction = format!("worktree-{}", hash(&random));
        let destination = scope.owned_root().join("worktree");
        let plan = plan_worktree_create(
            &transaction,
            task,
            base,
            scope.repository_path_sha256(),
            &hash(destination.as_os_str().as_bytes()),
            &before,
        )
        .map_err(|e| err(e.code()))?;
        let prepared = PreparedRepositoryOperation::new(plan).map_err(|e| err(e.code()))?;
        let expires_at = now
            .checked_add(LIFETIME_MS)
            .ok_or(err("code.repository.clock"))?;
        let mut preview = json!({"version":1,"operation":"create-worktree","plan":prepared.plan(),
            "repository_root_sha256":repository.root_path_sha256(),"owned_root_sha256":owned.root_path_sha256(),
            "expires_at_epoch_ms":expires_at,"created_at_epoch_ms":now,
            "warning":"Creates an agent task branch and worktree. No tests, commit, push, removal or automatic cleanup. Inspect disk after interruption.",
            "retention":"canonical-authority","preview_retention":"process-only"});
        let digest = hash(&serde_json::to_vec(&preview).map_err(|_| err("code.repository.state"))?);
        preview["preview_sha256"] = json!(digest);
        repository
            .revalidate()
            .map_err(|_| err("code.repository.scope"))?;
        metadata
            .revalidate()
            .map_err(|_| err("code.repository.scope"))?;
        owned
            .revalidate()
            .map_err(|_| err("code.repository.scope"))?;
        self.pending = Some(Pending {
            prepared,
            before,
            scope,
            collector,
            repository,
            metadata,
            owned,
            root: root.to_path_buf(),
            destination,
            preview: preview.clone(),
            hash: digest,
            created_at: now,
            expires_at,
            deadline: Instant::now() + Duration::from_millis(LIFETIME_MS),
        });
        Ok(preview)
    }

    /// Consumes a fresh exact trusted decision. A mismatch preserves the preview;
    /// denial, expiry and all dispatched outcomes consume it. A dispatch error is
    /// potentially effectful and must never be automatically retried.
    pub fn decide(&mut self, preview_sha256: &str, approve: bool, now: u64) -> Result<Value> {
        let pending = self
            .pending
            .as_ref()
            .ok_or(err("code.repository.choice-required"))?;
        if pending.hash != preview_sha256 {
            return Err(err("code.repository.stale"));
        }
        let pending = self
            .pending
            .take()
            .ok_or(err("code.repository.choice-required"))?;
        if !approve {
            return Ok(json!({"outcome":"denied","effect":"none"}));
        }
        if now < pending.created_at
            || now >= pending.expires_at
            || Instant::now() >= pending.deadline
        {
            return Err(err("code.repository.expired"));
        }
        self.execute(pending, now)
    }

    fn execute(&mut self, pending: Pending, now: u64) -> Result<Value> {
        let preflight_started = Instant::now();
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        pending
            .repository
            .revalidate()
            .map_err(|_| err("code.repository.scope"))?;
        pending
            .metadata
            .revalidate()
            .map_err(|_| err("code.repository.scope"))?;
        pending
            .owned
            .revalidate()
            .map_err(|_| err("code.repository.scope"))?;
        LinuxCodingReadScope::verify_root(&pending.root, pending.repository.root_path_sha256())
            .map_err(|_| err("code.repository.scope"))?;
        let owned_path = pending
            .destination
            .parent()
            .ok_or(err("code.repository.scope"))?;
        LinuxCodingReadScope::verify_root(owned_path, pending.owned.root_path_sha256())
            .map_err(|_| err("code.repository.scope"))?;
        let current = pending
            .collector
            .collect(&pending.scope)
            .map_err(|e| err(e.kind().code()))?;
        if current != pending.before
            || !pending
                .owned
                .observe_root_names(1, 256)
                .map_err(|_| err("code.repository.scope"))?
                .is_empty()
        {
            return Err(err("code.repository.stale"));
        }
        // Collection may be slow. Advance the trusted caller's clock by actual
        // monotonic preflight time before consuming the expiring grant.
        let dispatch_now = now
            .checked_add(
                u64::try_from(preflight_started.elapsed().as_millis())
                    .map_err(|_| err("code.repository.clock"))?,
            )
            .ok_or(err("code.repository.clock"))?;
        if Instant::now() >= pending.deadline || dispatch_now >= pending.expires_at {
            return Err(err("code.repository.expired"));
        }
        let (receipt, specialized) = dispatch(
            self.authority.authority_mut(),
            &pending.repository,
            pending.metadata,
            pending.prepared,
            pending.before,
            pending.hash,
            pending.expires_at,
            dispatch_now,
            LinuxRepositoryExecutor::new(pending.collector, pending.scope),
        )?;
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.uncertain"))?;
        pending
            .repository
            .revalidate()
            .map_err(|_| err("code.repository.uncertain"))?;
        pending
            .owned
            .revalidate()
            .map_err(|_| err("code.repository.uncertain"))?;
        LinuxCodingReadScope::verify_root(&pending.root, pending.repository.root_path_sha256())
            .map_err(|_| err("code.repository.uncertain"))?;
        LinuxCodingReadScope::verify_root(owned_path, pending.owned.root_path_sha256())
            .map_err(|_| err("code.repository.uncertain"))?;
        // Receipt proves repository preservation, not permission to read or edit
        // the new worktree. Source selection remains a separate explicit action.
        let repository_root_sha256 = pending.repository.root_path_sha256().to_owned();
        let worktree_root_sha256 =
            if receipt.outcome == agentmage_kernel_contracts::OperationOutcome::Succeeded {
                let held = held_root(&pending.destination)
                    .map_err(|_| err("code.repository.uncertain"))?;
                let identity = held.root_path_sha256().to_owned();
                // Retain a held identity only after observed native success. This is
                // process-local provenance, never reconstructed from a caller path.
                self.command = None;
                self.validation = None;
                self.edit = None;
                self.edit_result = None;
                self.apply = None;
                self.apply_result = None;
                self.created = Some(CreatedWorktree {
                    original_path: pending.root,
                    original: pending.repository,
                    edits: BTreeMap::new(),
                    path: pending.destination,
                    held,
                    owned: pending.owned,
                    creation_receipt_sha256: receipt.receipt_sha256.clone(),
                });
                Some(identity)
            } else {
                None
            };
        Ok(
            json!({"outcome":receipt.outcome,"receipt":receipt,"repository_receipt":specialized,
            "repository_root_sha256":repository_root_sha256,
            "worktree_root_sha256": worktree_root_sha256,
            "source_selected":false,"tested":false,"retention":"canonical-authority"}),
        )
    }
}

fn held_root(path: &Path) -> Result<LinuxAuthorizedWorkspace> {
    authorize_workspace_root(
        path,
        WorkspaceId::from_raw(TOOL),
        WorkspaceAuthorizationId::from_raw(TOOL),
        AdapterInstanceId::from_raw(TOOL),
    )
    .map_err(|_| err("code.repository.scope"))
}
fn validate_input(root: &Path, owned: &Path, task: &str, base: &str) -> Result<()> {
    if [root, owned]
        .iter()
        .any(|p| !p.is_absolute() || p.as_os_str().len() > 4096 || p.components().count() > 256)
        || root.starts_with(owned)
        || owned.starts_with(root)
        || task.is_empty()
        || task.len() > 64
        || !task
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !task
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || ![40, 64].contains(&base.len())
        || !base
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(err("code.repository.input"));
    }
    Ok(())
}

struct RepositoryTool {
    definition: ToolDefinition,
    bytes: Vec<u8>,
}
impl RepositoryTool {
    fn new(bytes: &[u8]) -> Self {
        let schema = |name: &str, bytes: &str| SchemaReference {
            schema_id: SchemaId::from_raw(name),
            schema_version: 1,
            schema_sha256: hash(bytes.as_bytes()),
        };
        let operation = OperationBinding::new(GrantOperation::GitWorktreeCreate);
        Self {
            bytes: bytes.to_vec(),
            definition: ToolDefinition {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_id: ToolId::from_raw(TOOL),
                tool_version: "1.0.0".into(),
                display_name: "Create approved task worktree".into(),
                description: "Execute only the exact prepared local worktree plan".into(),
                input_schema: schema(
                    "local-coding-worktree-plan-v1",
                    include_str!("../../../schemas/runtime/repository-operation-plan.schema.json"),
                ),
                output_schema: schema(
                    "local-coding-worktree-receipt-v1",
                    include_str!(
                        "../../../schemas/runtime/repository-operation-receipt.schema.json"
                    ),
                ),
                risk_level: ToolRiskLevel::High,
                declared_effects: vec![operation],
                required_grant: RequiredGrantTemplate {
                    operation,
                    target_scope: "exact-held-repository-metadata".into(),
                    single_use: true,
                },
                timeout_ms: 300_000,
            },
        }
    }
}
impl Tool for RepositoryTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
    fn validate_arguments(&self, bytes: &[u8]) -> Vec<ValidationIssue> {
        if bytes == self.bytes {
            vec![]
        } else {
            vec![ValidationIssue {
                code: "code.repository.arguments".into(),
                severity: ValidationSeverity::Error,
                field_path: vec!["arguments".into()],
                message: "Exact approved plan required".into(),
            }]
        }
    }
}

// The generic boundary permits component tests with synthetic platform results;
// the public path above always instantiates the existing native Git executor.
#[allow(clippy::too_many_arguments)]
fn dispatch<E: agentmage_kernel_engine::repository_safety::BoundedRepositoryExecutor>(
    runtime: &mut agentmage_kernel_engine::operational_store::DurableAuthorityRuntime,
    repository: &LinuxAuthorizedWorkspace,
    metadata: LinuxHeldObject,
    prepared: PreparedRepositoryOperation,
    before: agentmage_kernel_engine::repository_safety::RepositoryPreservationManifest,
    preview_hash: String,
    expires_at: u64,
    now: u64,
    executor: E,
) -> Result<(
    agentmage_kernel_contracts::Receipt,
    Option<agentmage_kernel_engine::repository_safety::RepositoryOperationReceipt>,
)> {
    let target = GrantTarget::held_object(&metadata).map_err(|_| err("code.repository.scope"))?;
    let scope = GrantTarget::workspace_scope(
        repository,
        WorkspacePath::new(WorkspaceId::from_raw(TOOL), [".git"])
            .map_err(|_| err("code.repository.scope"))?
            .as_scope(),
    )
    .map_err(|_| err("code.repository.scope"))?;
    let exact = ExactEffect {
        id: prepared.plan().transaction_id.clone(),
        operation: OperationBinding::new(GrantOperation::GitWorktreeCreate),
        target,
        scope,
        plan_sha256: prepared.plan().plan_sha256.clone(),
        request_sha256: prepared.request_sha256().into(),
        tool: RepositoryTool::new(prepared.request_bytes()),
        preview_hash,
        expires_at,
        now,
    };
    let cancellation = CancellationToken::root(
        BoundaryKind::Kernel,
        TaskId::from_raw(&exact.id),
        CorrelationId::from_raw(&exact.id),
    );
    let mut driver =
        RepositoryEffectDriver::new(executor, metadata, prepared, before, cancellation)
            .map_err(|_| err("code.repository.authority"))?;
    let receipt = exact.execute(runtime, &mut driver)?;
    Ok((receipt, driver.take_receipt()))
}

// Both native effects use the same exact, single-use canonical authority path.
struct ExactEffect {
    id: String,
    operation: OperationBinding,
    target: GrantTarget,
    scope: GrantTarget,
    plan_sha256: String,
    request_sha256: String,
    tool: RepositoryTool,
    preview_hash: String,
    expires_at: u64,
    now: u64,
}
impl ExactEffect {
    fn execute(
        self,
        runtime: &mut agentmage_kernel_engine::operational_store::DurableAuthorityRuntime,
        driver: &mut impl agentmage_kernel_engine::authority_transaction::EffectDriver,
    ) -> Result<agentmage_kernel_contracts::Receipt> {
        let Self {
            id,
            operation,
            target,
            scope,
            plan_sha256,
            request_sha256,
            tool,
            preview_hash,
            expires_at,
            now,
        } = self;
        let tool_id = tool.definition.tool_id.clone();
        let request_bytes = tool.bytes.clone();
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("local-control")]),
            tasks: rules([TaskId::from_raw(&id)]),
            actions: rules([ActionId::from_raw(tool_id.as_str())]),
            tools: rules([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".into(),
            }]),
            operations: rules([operation]),
            targets: rules([target.clone()]),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .map_err(|_| err("code.repository.policy"))?;
        let parent = runtime
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw(format!("read-{id}")),
                actor_id: ActorId::from_raw("local-control"),
                session_id: SessionId::from_raw(&id),
                task_id: TaskId::from_raw(&id),
                targets: vec![scope],
                excluded_targets: vec![],
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: now,
                expires_at_epoch_ms: expires_at,
                nonce: GrantNonce::from_raw(format!("read-{id}")),
                maximum_derived_operations: 1,
                preview_sha256: preview_hash.clone(),
                policy_sha256: policy.policy_sha256().into(),
            })
            .map_err(|_| err("code.repository.parent"))?;
        let approval = ApprovalId::from_raw(format!("approval-{id}"));
        let effects = vec![GrantSideEffect {
            operation,
            target_indexes: vec![0],
            details_sha256: plan_sha256.clone(),
        }];
        let grant = runtime
        .derive_operation(
            &parent.grant_id,
            DerivedOperationGrantRequest {
                grant_id: GrantId::from_raw(format!("create-{id}")),
                approval_id: approval.clone(),
                action_id: ActionId::from_raw(tool_id.as_str()),
                action_kind: ActionKind::DeterministicTool,
                operation,
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".into(),
                targets: vec![target.clone()],
                argument_sha256: request_sha256.clone(),
                preimages: vec![],
                expected_side_effects: effects.clone(),
                rollback_description:
                    "Inspect disk and receipts after interruption. No automatic replay or removal."
                        .into(),
                issued_at_epoch_ms: now,
                expires_at_epoch_ms: expires_at,
                nonce: GrantNonce::from_raw(format!("create-{id}")),
                preview_sha256: preview_hash.clone(),
                policy_sha256: policy.policy_sha256().into(),
            },
        )
        .map_err(|_| err("code.repository.grant"))?;
        let definition = tool.definition.clone();
        let mut registry = ToolRegistry::new();
        registry
            .register_shell_tool(Box::new(tool))
            .map_err(|_| err("code.repository.tool"))?;
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(format!("call-{id}")),
            correlation_id: CorrelationId::from_raw(&id),
            action_id: ActionId::from_raw(tool_id.as_str()),
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".into(),
            arguments: ContractPayload {
                schema: definition.input_schema,
                media_type: "application/json".into(),
                bytes: request_bytes,
                sha256: request_sha256.clone(),
            },
        };
        let context = PolicyEvaluationContext {
            actor_id: ActorId::from_raw("local-control"),
            session_id: SessionId::from_raw(&id),
            task_id: TaskId::from_raw(&id),
            action_id: ActionId::from_raw(tool_id.as_str()),
            action_kind: ActionKind::DeterministicTool,
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".into(),
            targets: vec![target],
            argument_sha256: request_sha256.clone(),
            preimages: vec![],
            expected_side_effects: effects,
            preview_sha256: preview_hash,
            now_epoch_ms: now,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        let request = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw(&id),
            OperationAttemptId::from_raw(format!("attempt-{id}")),
            approval,
            grant.grant_id,
            call,
            context,
            now,
            now.to_string(),
        )
        .map_err(|_| err("code.repository.request"))?;
        runtime
            .execute_effect(&registry, &policy, request, driver)
            .map_err(|_| err("code.repository.uncertain"))
    }
}

#[cfg(test)]
#[path = "coding_repository_tests.rs"]
mod tests;
