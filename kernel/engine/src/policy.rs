//! Deterministic deny-first evaluation of exact capability grants.

use std::{collections::BTreeSet, fmt::Write};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, CapabilityGrant, GrantClass, GrantId, GrantOperation,
    GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, OperationBinding, SessionId, TaskId,
    ToolId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::grants::GrantIssuer;

/// Operations explicitly prohibited by the strict-local read-only policy profile.
pub const STRICT_LOCAL_DENIED_OPERATIONS: [GrantOperation; 21] = [
    GrantOperation::WorkspaceWrite,
    GrantOperation::WorkspaceDelete,
    GrantOperation::CommandExecute,
    GrantOperation::NetworkAccess,
    GrantOperation::GitClone,
    GrantOperation::GitFetch,
    GrantOperation::GitWorktreeCreate,
    GrantOperation::GitWorktreeRemove,
    GrantOperation::GitBranchFastForward,
    GrantOperation::GitCommit,
    GrantOperation::GitPush,
    GrantOperation::Publish,
    GrantOperation::Send,
    GrantOperation::Upload,
    GrantOperation::Deploy,
    GrantOperation::DatabaseWrite,
    GrantOperation::CredentialAccess,
    GrantOperation::DatabaseRead,
    GrantOperation::ModelInference,
    GrantOperation::DraftCreate,
    GrantOperation::Administration,
];

/// Exact tool identity and contract version admitted by policy.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ToolPolicyBinding {
    /// Registered tool identity.
    pub tool_id: ToolId,
    /// Exact registered tool-contract version.
    pub tool_version: String,
}

/// Explicit exact allow and deny values for one policy scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScopeRules<T>
where
    T: Ord,
{
    /// Exact values admitted when they are not also denied.
    pub allowed: BTreeSet<T>,
    /// Exact values denied with precedence over `allowed`.
    pub denied: BTreeSet<T>,
}

impl<T> ScopeRules<T>
where
    T: Ord,
{
    /// Creates an empty deny-by-default scope.
    #[must_use]
    pub const fn deny_all() -> Self {
        Self {
            allowed: BTreeSet::new(),
            denied: BTreeSet::new(),
        }
    }

    fn allows(&self, value: &T) -> bool {
        !self.denied.contains(value) && self.allowed.contains(value)
    }
}

/// Versioned deterministic rules whose canonical bytes define policy identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PolicyDocument {
    /// Policy document schema version.
    pub schema_version: u16,
    /// Monotonically increasing policy revision.
    pub revision: u32,
    /// Actor allow/deny rules.
    pub actors: ScopeRules<ActorId>,
    /// Task allow/deny rules.
    pub tasks: ScopeRules<TaskId>,
    /// Action allow/deny rules.
    pub actions: ScopeRules<ActionId>,
    /// Tool identity/version allow/deny rules.
    pub tools: ScopeRules<ToolPolicyBinding>,
    /// Operation-class allow/deny rules.
    pub operations: ScopeRules<OperationBinding>,
    /// Workspace-target allow/deny rules.
    pub targets: ScopeRules<GrantTarget>,
    /// Argument digests denied even when an exact grant names them.
    pub denied_argument_sha256s: BTreeSet<String>,
    /// Preimage digests denied even when an exact grant names them.
    pub denied_preimage_sha256s: BTreeSet<String>,
    /// Exact network destinations admitted for network operations.
    pub network_scopes: ScopeRules<String>,
    /// Exact credential references admitted for credential operations.
    pub credential_scopes: ScopeRules<String>,
    /// Exact publication destinations admitted for publication operations.
    pub publication_scopes: ScopeRules<String>,
}

/// Exact identity and workspace scope admitted by the strict-local read-only profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrictLocalReadOnlyScope {
    /// Monotonically increasing policy revision.
    pub revision: u32,
    /// Exact local actors admitted by this profile instance.
    pub actors: BTreeSet<ActorId>,
    /// Exact tasks admitted by this profile instance.
    pub tasks: BTreeSet<TaskId>,
    /// Exact actions admitted by this profile instance.
    pub actions: BTreeSet<ActionId>,
    /// Exact tool identities and versions admitted by this profile instance.
    pub tools: BTreeSet<ToolPolicyBinding>,
    /// Exact workspace targets admitted by this profile instance.
    pub targets: BTreeSet<GrantTarget>,
}

/// Current deterministic observations supplied immediately before policy evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyEvaluationContext {
    /// Current actor identity.
    pub actor_id: ActorId,
    /// Current session identity.
    pub session_id: SessionId,
    /// Current task identity.
    pub task_id: TaskId,
    /// Current action identity.
    pub action_id: ActionId,
    /// Current action class.
    pub action_kind: ActionKind,
    /// Current registered tool identity.
    pub tool_id: ToolId,
    /// Current registered tool-contract version.
    pub tool_version: String,
    /// Current exact workspace targets.
    pub targets: Vec<GrantTarget>,
    /// Digest of current canonical arguments.
    pub argument_sha256: String,
    /// Current exact target preimages.
    pub preimages: Vec<GrantPreimage>,
    /// Current exact expected side effects.
    pub expected_side_effects: Vec<GrantSideEffect>,
    /// Digest of the preview currently displayed for the operation.
    pub preview_sha256: String,
    /// Current kernel-clock instant in UTC Unix epoch milliseconds.
    pub now_epoch_ms: u64,
    /// Exact network destination for a network operation.
    pub network_scope: Option<String>,
    /// Exact credential reference for a credential operation.
    pub credential_scope: Option<String>,
    /// Exact publication destination for a publication operation.
    pub publication_scope: Option<String>,
}

/// First failing scope in the fixed deny-precedence order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyDenialScope {
    /// Grant identity, lifecycle, issuer, policy, or schema state is invalid.
    Grant,
    /// Actor identity or actor policy denied the operation.
    Actor,
    /// Session identity differs from the grant.
    Session,
    /// Task identity or task policy denied the operation.
    Task,
    /// Action identity, kind, or action policy denied the operation.
    Action,
    /// Tool identity/version or tool policy denied the operation.
    Tool,
    /// Operation policy denied the closed operation class.
    Operation,
    /// Current targets differ or target policy denied the operation.
    Path,
    /// Current arguments differ or their digest is explicitly denied.
    Argument,
    /// Current preimages differ or a preimage is explicitly denied.
    Preimage,
    /// Current expected effects differ from the confirmed grant.
    SideEffect,
    /// Current preview differs from the user-confirmed preview.
    Preview,
    /// Network scope is missing, unexpected, or denied.
    Network,
    /// Credential scope is missing, unexpected, or denied.
    Credential,
    /// Publication scope is missing, unexpected, or denied.
    Publication,
}

impl PolicyDenialScope {
    /// Returns the stable redacted denial code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Grant => "policy.deny.grant",
            Self::Actor => "policy.deny.actor",
            Self::Session => "policy.deny.session",
            Self::Task => "policy.deny.task",
            Self::Action => "policy.deny.action",
            Self::Tool => "policy.deny.tool",
            Self::Operation => "policy.deny.operation",
            Self::Path => "policy.deny.path",
            Self::Argument => "policy.deny.argument",
            Self::Preimage => "policy.deny.preimage",
            Self::SideEffect => "policy.deny.side_effect",
            Self::Preview => "policy.deny.preview",
            Self::Network => "policy.deny.network",
            Self::Credential => "policy.deny.credential",
            Self::Publication => "policy.deny.publication",
        }
    }
}

/// Redacted deterministic result of one policy evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecision {
    /// Exact grant identity evaluated.
    pub grant_id: GrantId,
    /// Kernel-computed policy digest.
    pub policy_sha256: String,
    /// Whether every exact scope admitted the operation.
    pub allowed: bool,
    /// First fixed-precedence denial scope, absent only for an allow.
    pub denial_scope: Option<PolicyDenialScope>,
    /// Stable result code with no candidate values.
    pub code: &'static str,
}

/// Policy construction failure that retains no policy values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyBuildError {
    /// The policy document is malformed, unsupported, or contains broad scopes.
    InvalidDocument,
}

/// Immutable deterministic policy engine.
#[derive(Clone, Debug)]
pub struct PolicyEngine {
    document: PolicyDocument,
    policy_sha256: String,
}

impl PolicyEngine {
    /// Builds the immutable strict-local profile with no stateful or external operation path.
    pub fn strict_local_read_only(
        scope: StrictLocalReadOnlyScope,
    ) -> Result<Self, PolicyBuildError> {
        Self::new(PolicyDocument {
            schema_version: 1,
            revision: scope.revision,
            actors: ScopeRules {
                allowed: scope.actors,
                denied: BTreeSet::new(),
            },
            tasks: ScopeRules {
                allowed: scope.tasks,
                denied: BTreeSet::new(),
            },
            actions: ScopeRules {
                allowed: scope.actions,
                denied: BTreeSet::new(),
            },
            tools: ScopeRules {
                allowed: scope.tools,
                denied: BTreeSet::new(),
            },
            operations: ScopeRules {
                allowed: BTreeSet::from([OperationBinding::new(GrantOperation::WorkspaceRead)]),
                denied: STRICT_LOCAL_DENIED_OPERATIONS
                    .into_iter()
                    .map(OperationBinding::new)
                    .collect(),
            },
            targets: ScopeRules {
                allowed: scope.targets,
                denied: BTreeSet::new(),
            },
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
    }

    /// Validates a closed document and computes its canonical policy identity.
    pub fn new(document: PolicyDocument) -> Result<Self, PolicyBuildError> {
        validate_document(&document)?;
        let bytes = serde_json::to_vec(&document).map_err(|_| PolicyBuildError::InvalidDocument)?;
        Ok(Self {
            document,
            policy_sha256: hex_sha256(&bytes),
        })
    }

    /// Returns the kernel-computed digest callers must bind into issued grants.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Evaluates one current issuer-retained operation grant in fixed deny order.
    #[must_use]
    pub fn evaluate(
        &self,
        issuer: &GrantIssuer,
        grant: &CapabilityGrant,
        context: &PolicyEvaluationContext,
    ) -> PolicyDecision {
        let deny = |scope: PolicyDenialScope| PolicyDecision {
            grant_id: grant.grant_id.clone(),
            policy_sha256: self.policy_sha256.clone(),
            allowed: false,
            denial_scope: Some(scope),
            code: scope.code(),
        };
        if grant.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
            || grant.grant_class != GrantClass::Operation
            || grant.status != GrantStatus::Issued
            || grant.use_limit != 1
            || grant.use_count != 0
            || grant.policy_sha256 != self.policy_sha256
            || context.now_epoch_ms < grant.issued_at_epoch_ms
            || context.now_epoch_ms >= grant.expires_at_epoch_ms
            || issuer.current(&grant.grant_id) != Some(grant)
        {
            return deny(PolicyDenialScope::Grant);
        }
        if context.actor_id != grant.actor_id || !self.document.actors.allows(&context.actor_id) {
            return deny(PolicyDenialScope::Actor);
        }
        if context.session_id != grant.session_id {
            return deny(PolicyDenialScope::Session);
        }
        if context.task_id != grant.task_id || !self.document.tasks.allows(&context.task_id) {
            return deny(PolicyDenialScope::Task);
        }
        if grant.action_id.as_ref() != Some(&context.action_id)
            || grant.action_kind != Some(context.action_kind)
            || !self.document.actions.allows(&context.action_id)
        {
            return deny(PolicyDenialScope::Action);
        }
        let tool = ToolPolicyBinding {
            tool_id: context.tool_id.clone(),
            tool_version: context.tool_version.clone(),
        };
        if grant.tool_id.as_ref() != Some(&context.tool_id)
            || grant.tool_version.as_deref() != Some(context.tool_version.as_str())
            || !self.document.tools.allows(&tool)
        {
            return deny(PolicyDenialScope::Tool);
        }
        if !self.document.operations.allows(&grant.operation) {
            return deny(PolicyDenialScope::Operation);
        }
        if context.targets != grant.targets
            || context
                .targets
                .iter()
                .any(|target| !self.document.targets.allows(target))
        {
            return deny(PolicyDenialScope::Path);
        }
        if context.argument_sha256 != grant.argument_sha256
            || self
                .document
                .denied_argument_sha256s
                .contains(&context.argument_sha256)
        {
            return deny(PolicyDenialScope::Argument);
        }
        if context.preimages != grant.preimages
            || context.preimages.iter().any(|preimage| {
                self.document
                    .denied_preimage_sha256s
                    .contains(&preimage.content_sha256)
            })
        {
            return deny(PolicyDenialScope::Preimage);
        }
        if context.expected_side_effects != grant.expected_side_effects {
            return deny(PolicyDenialScope::SideEffect);
        }
        if context.preview_sha256 != grant.preview_sha256 {
            return deny(PolicyDenialScope::Preview);
        }
        if !external_scope_allowed(
            grant.operation.operation() == GrantOperation::NetworkAccess,
            context.network_scope.as_ref(),
            &self.document.network_scopes,
        ) {
            return deny(PolicyDenialScope::Network);
        }
        if !external_scope_allowed(
            grant.operation.operation() == GrantOperation::CredentialAccess,
            context.credential_scope.as_ref(),
            &self.document.credential_scopes,
        ) {
            return deny(PolicyDenialScope::Credential);
        }
        if !external_scope_allowed(
            is_publication_operation(grant.operation.operation()),
            context.publication_scope.as_ref(),
            &self.document.publication_scopes,
        ) {
            return deny(PolicyDenialScope::Publication);
        }
        PolicyDecision {
            grant_id: grant.grant_id.clone(),
            policy_sha256: self.policy_sha256.clone(),
            allowed: true,
            denial_scope: None,
            code: "policy.allow.exact_grant",
        }
    }
}

fn external_scope_allowed(
    required: bool,
    candidate: Option<&String>,
    rules: &ScopeRules<String>,
) -> bool {
    match (required, candidate) {
        (true, Some(value)) => rules.allows(value),
        (false, None) => true,
        _ => false,
    }
}

fn is_publication_operation(operation: GrantOperation) -> bool {
    matches!(
        operation,
        GrantOperation::GitPush
            | GrantOperation::Publish
            | GrantOperation::Send
            | GrantOperation::Upload
            | GrantOperation::Deploy
    )
}

fn validate_document(document: &PolicyDocument) -> Result<(), PolicyBuildError> {
    if document.schema_version != 1 || document.revision == 0 {
        return Err(PolicyBuildError::InvalidDocument);
    }
    for value in document
        .actors
        .allowed
        .iter()
        .chain(&document.actors.denied)
        .map(ActorId::as_str)
        .chain(
            document
                .tasks
                .allowed
                .iter()
                .chain(&document.tasks.denied)
                .map(TaskId::as_str),
        )
        .chain(
            document
                .actions
                .allowed
                .iter()
                .chain(&document.actions.denied)
                .map(ActionId::as_str),
        )
    {
        validate_scope_value(value)?;
    }
    for tool in document.tools.allowed.iter().chain(&document.tools.denied) {
        validate_scope_value(tool.tool_id.as_str())?;
        validate_scope_value(&tool.tool_version)?;
    }
    for target in document
        .targets
        .allowed
        .iter()
        .chain(&document.targets.denied)
    {
        if !target.is_operation_target() {
            return Err(PolicyBuildError::InvalidDocument);
        }
        validate_scope_value(target.workspace_id().as_str())?;
        validate_scope_value(target.authorization_id().as_str())?;
        validate_scope_value(target.adapter_instance_id().as_str())?;
        for component in target.path_components() {
            validate_scope_value(component.as_str())?;
        }
    }
    for digest in document
        .denied_argument_sha256s
        .iter()
        .chain(&document.denied_preimage_sha256s)
    {
        if !valid_digest(digest) {
            return Err(PolicyBuildError::InvalidDocument);
        }
    }
    for value in document
        .network_scopes
        .allowed
        .iter()
        .chain(&document.network_scopes.denied)
        .chain(&document.credential_scopes.allowed)
        .chain(&document.credential_scopes.denied)
        .chain(&document.publication_scopes.allowed)
        .chain(&document.publication_scopes.denied)
    {
        validate_scope_value(value)?;
    }
    Ok(())
}

fn validate_scope_value(value: &str) -> Result<(), PolicyBuildError> {
    if value.is_empty() || value.len() > 512 || value.contains('*') || value.contains('\0') {
        return Err(PolicyBuildError::InvalidDocument);
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn hex_sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        PolicyBuildError, PolicyDenialScope, PolicyDocument, PolicyEngine, PolicyEvaluationContext,
        STRICT_LOCAL_DENIED_OPERATIONS, ScopeRules, StrictLocalReadOnlyScope, ToolPolicyBinding,
    };
    use crate::grants::{DerivedOperationGrantRequest, GrantIssuer, SessionReadGrantRequest};
    use crate::test_target::{preimage, scope, target};
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, ApprovalId, DataSensitivity, GrantId, GrantNonce,
        GrantOperation, GrantSideEffect, OperationBinding, SessionId, TaskId, ToolId,
    };

    fn set<T: Ord>(value: T) -> BTreeSet<T> {
        BTreeSet::from([value])
    }

    fn rules<T: Ord>(value: T) -> ScopeRules<T> {
        ScopeRules {
            allowed: set(value),
            denied: BTreeSet::new(),
        }
    }

    fn document(operation: GrantOperation) -> PolicyDocument {
        PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules(ActorId::from_raw("actor-local-0001")),
            tasks: rules(TaskId::from_raw("task-0001")),
            actions: rules(ActionId::from_raw("action-0001")),
            tools: rules(ToolPolicyBinding {
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
            }),
            operations: rules(OperationBinding::new(operation)),
            targets: rules(target(&["src"])),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        }
    }

    fn issued_operation(
        engine: &PolicyEngine,
        operation: GrantOperation,
    ) -> (GrantIssuer, agentmage_kernel_contracts::CapabilityGrant) {
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-0001"),
                actor_id: ActorId::from_raw("actor-local-0001"),
                session_id: SessionId::from_raw("session-0001"),
                task_id: TaskId::from_raw("task-0001"),
                targets: vec![scope(&[])],
                excluded_targets: vec![scope(&["private"])],
                sensitivity: DataSensitivity::Ephemeral,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 60_000,
                nonce: GrantNonce::from_raw("nonce-parent-0001"),
                maximum_derived_operations: 1,
                preview_sha256: "1".repeat(64),
                policy_sha256: engine.policy_sha256().to_owned(),
            })
            .expect("parent must issue");
        let child = issuer
            .derive_operation(&parent.grant_id, {
                let target = target(&["src"]);
                DerivedOperationGrantRequest {
                    grant_id: GrantId::from_raw("grant-child-0001"),
                    approval_id: ApprovalId::from_raw("approval-0001"),
                    action_id: ActionId::from_raw("action-0001"),
                    action_kind: ActionKind::DeterministicTool,
                    operation: OperationBinding::new(operation),
                    tool_id: ToolId::from_raw("fixture.read"),
                    tool_version: "1.0.0".to_owned(),
                    targets: vec![target.clone()],
                    argument_sha256: "2".repeat(64),
                    preimages: vec![preimage(0, &target)],
                    expected_side_effects: vec![GrantSideEffect {
                        operation: OperationBinding::new(operation),
                        target_indexes: vec![0],
                        details_sha256: "4".repeat(64),
                    }],
                    rollback_description: "No state change is permitted".to_owned(),
                    issued_at_epoch_ms: 2_000,
                    expires_at_epoch_ms: 30_000,
                    nonce: GrantNonce::from_raw("nonce-child-0001"),
                    preview_sha256: "5".repeat(64),
                    policy_sha256: engine.policy_sha256().to_owned(),
                }
            })
            .expect("child must derive");
        (issuer, child)
    }

    fn context(grant: &agentmage_kernel_contracts::CapabilityGrant) -> PolicyEvaluationContext {
        PolicyEvaluationContext {
            actor_id: grant.actor_id.clone(),
            session_id: grant.session_id.clone(),
            task_id: grant.task_id.clone(),
            action_id: grant.action_id.clone().expect("operation action"),
            action_kind: grant.action_kind.expect("operation action kind"),
            tool_id: grant.tool_id.clone().expect("operation tool"),
            tool_version: grant.tool_version.clone().expect("operation tool version"),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: 3_000,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        }
    }

    fn assert_document_denies(
        document: PolicyDocument,
        operation: GrantOperation,
        expected: PolicyDenialScope,
    ) {
        let engine = PolicyEngine::new(document).expect("policy must build");
        let (issuer, grant) = issued_operation(&engine, operation);
        let decision = engine.evaluate(&issuer, &grant, &context(&grant));
        assert!(!decision.allowed);
        assert_eq!(decision.denial_scope, Some(expected));
    }

    #[test]
    fn exact_issuer_retained_grant_is_allowed_and_every_context_mutation_denies_in_order() {
        let engine =
            PolicyEngine::new(document(GrantOperation::WorkspaceRead)).expect("policy must build");
        let (issuer, grant) = issued_operation(&engine, GrantOperation::WorkspaceRead);
        let exact = context(&grant);
        let decision = engine.evaluate(&issuer, &grant, &exact);
        assert!(decision.allowed);
        assert_eq!(decision.denial_scope, None);

        let mutations: Vec<(PolicyDenialScope, PolicyEvaluationContext)> = vec![
            {
                let mut value = exact.clone();
                value.actor_id = ActorId::from_raw("actor-other");
                (PolicyDenialScope::Actor, value)
            },
            {
                let mut value = exact.clone();
                value.session_id = SessionId::from_raw("session-other");
                (PolicyDenialScope::Session, value)
            },
            {
                let mut value = exact.clone();
                value.task_id = TaskId::from_raw("task-other");
                (PolicyDenialScope::Task, value)
            },
            {
                let mut value = exact.clone();
                value.action_id = ActionId::from_raw("action-other");
                (PolicyDenialScope::Action, value)
            },
            {
                let mut value = exact.clone();
                value.tool_version = "2.0.0".to_owned();
                (PolicyDenialScope::Tool, value)
            },
            {
                let mut value = exact.clone();
                value.targets = vec![target(&["tests"])];
                (PolicyDenialScope::Path, value)
            },
            {
                let mut value = exact.clone();
                value.argument_sha256 = "6".repeat(64);
                (PolicyDenialScope::Argument, value)
            },
            {
                let mut value = exact.clone();
                value.preimages[0].content_sha256 = "7".repeat(64);
                (PolicyDenialScope::Preimage, value)
            },
            {
                let mut value = exact.clone();
                value.expected_side_effects[0].details_sha256 = "8".repeat(64);
                (PolicyDenialScope::SideEffect, value)
            },
            {
                let mut value = exact.clone();
                value.preview_sha256 = "9".repeat(64);
                (PolicyDenialScope::Preview, value)
            },
            {
                let mut value = exact.clone();
                value.network_scope = Some("https:example.invalid:443".to_owned());
                (PolicyDenialScope::Network, value)
            },
            {
                let mut value = exact.clone();
                value.credential_scope = Some("credential:fixture".to_owned());
                (PolicyDenialScope::Credential, value)
            },
            {
                let mut value = exact.clone();
                value.publication_scope = Some("destination:fixture".to_owned());
                (PolicyDenialScope::Publication, value)
            },
        ];
        for (expected, changed) in mutations {
            let decision = engine.evaluate(&issuer, &grant, &changed);
            assert!(!decision.allowed);
            assert_eq!(decision.denial_scope, Some(expected));
            assert_eq!(decision.code, expected.code());
        }

        let mut forged = grant.clone();
        forged.preview_sha256 = "f".repeat(64);
        assert_eq!(
            engine.evaluate(&issuer, &forged, &exact).denial_scope,
            Some(PolicyDenialScope::Grant)
        );
    }

    #[test]
    fn explicit_deny_precedes_allow_and_external_scopes_require_exact_allow() {
        let operation = GrantOperation::WorkspaceRead;
        let mut collisions = Vec::new();

        let mut actor_denied = document(operation);
        actor_denied
            .actors
            .denied
            .insert(ActorId::from_raw("actor-local-0001"));
        collisions.push((actor_denied, PolicyDenialScope::Actor));

        let mut task_denied = document(operation);
        task_denied
            .tasks
            .denied
            .insert(TaskId::from_raw("task-0001"));
        collisions.push((task_denied, PolicyDenialScope::Task));

        let mut action_denied = document(operation);
        action_denied
            .actions
            .denied
            .insert(ActionId::from_raw("action-0001"));
        collisions.push((action_denied, PolicyDenialScope::Action));

        let mut tool_denied = document(operation);
        tool_denied.tools.denied.insert(ToolPolicyBinding {
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
        });
        collisions.push((tool_denied, PolicyDenialScope::Tool));

        let mut operation_denied = document(operation);
        operation_denied
            .operations
            .denied
            .insert(OperationBinding::new(operation));
        collisions.push((operation_denied, PolicyDenialScope::Operation));

        let mut path_denied = document(operation);
        path_denied.targets.denied.insert(target(&["src"]));
        collisions.push((path_denied, PolicyDenialScope::Path));

        let mut argument_denied = document(operation);
        argument_denied
            .denied_argument_sha256s
            .insert("2".repeat(64));
        collisions.push((argument_denied, PolicyDenialScope::Argument));

        let mut preimage_denied = document(operation);
        preimage_denied
            .denied_preimage_sha256s
            .insert(preimage(0, &target(&["src"])).content_sha256);
        collisions.push((preimage_denied, PolicyDenialScope::Preimage));

        for (denied_document, expected) in collisions {
            assert_document_denies(denied_document, operation, expected);
        }

        for (operation, expected_scope, external_value) in [
            (
                GrantOperation::NetworkAccess,
                PolicyDenialScope::Network,
                "https:example.invalid:443",
            ),
            (
                GrantOperation::CredentialAccess,
                PolicyDenialScope::Credential,
                "credential:fixture",
            ),
            (
                GrantOperation::Publish,
                PolicyDenialScope::Publication,
                "destination:fixture",
            ),
        ] {
            let mut document = document(operation);
            match expected_scope {
                PolicyDenialScope::Network => {
                    document.network_scopes = rules(external_value.to_owned());
                }
                PolicyDenialScope::Credential => {
                    document.credential_scopes = rules(external_value.to_owned());
                }
                PolicyDenialScope::Publication => {
                    document.publication_scopes = rules(external_value.to_owned());
                }
                _ => unreachable!(),
            }
            let engine = PolicyEngine::new(document.clone()).expect("policy must build");
            let (issuer, grant) = issued_operation(&engine, operation);
            let missing = context(&grant);
            assert_eq!(
                engine.evaluate(&issuer, &grant, &missing).denial_scope,
                Some(expected_scope)
            );
            let mut exact = missing;
            match expected_scope {
                PolicyDenialScope::Network => {
                    exact.network_scope = Some(external_value.to_owned());
                }
                PolicyDenialScope::Credential => {
                    exact.credential_scope = Some(external_value.to_owned());
                }
                PolicyDenialScope::Publication => {
                    exact.publication_scope = Some(external_value.to_owned());
                }
                _ => unreachable!(),
            }
            assert!(engine.evaluate(&issuer, &grant, &exact).allowed);

            match expected_scope {
                PolicyDenialScope::Network => {
                    document
                        .network_scopes
                        .denied
                        .insert(external_value.to_owned());
                }
                PolicyDenialScope::Credential => {
                    document
                        .credential_scopes
                        .denied
                        .insert(external_value.to_owned());
                }
                PolicyDenialScope::Publication => {
                    document
                        .publication_scopes
                        .denied
                        .insert(external_value.to_owned());
                }
                _ => unreachable!(),
            }
            let denied_engine = PolicyEngine::new(document).expect("policy must build");
            let (denied_issuer, denied_grant) = issued_operation(&denied_engine, operation);
            let mut denied_context = context(&denied_grant);
            match expected_scope {
                PolicyDenialScope::Network => {
                    denied_context.network_scope = Some(external_value.to_owned());
                }
                PolicyDenialScope::Credential => {
                    denied_context.credential_scope = Some(external_value.to_owned());
                }
                PolicyDenialScope::Publication => {
                    denied_context.publication_scope = Some(external_value.to_owned());
                }
                _ => unreachable!(),
            }
            assert_eq!(
                denied_engine
                    .evaluate(&denied_issuer, &denied_grant, &denied_context)
                    .denial_scope,
                Some(expected_scope)
            );
        }
    }

    #[test]
    fn malformed_or_broad_policy_values_are_rejected_and_hashing_is_deterministic() {
        let baseline = document(GrantOperation::WorkspaceRead);
        let first = PolicyEngine::new(baseline.clone()).expect("policy must build");
        let second = PolicyEngine::new(baseline.clone()).expect("policy must build");
        assert_eq!(first.policy_sha256(), second.policy_sha256());

        let mut wildcard = baseline.clone();
        wildcard
            .network_scopes
            .allowed
            .insert("https://*.example.invalid".to_owned());
        assert_eq!(
            PolicyEngine::new(wildcard).expect_err("wildcard must fail"),
            PolicyBuildError::InvalidDocument
        );

        let traversal = serde_json::json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-0001", "components": [".."]},
            "authorization_id": "authorization-0001",
            "adapter_instance_id": "adapter-0001",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![2_u8; 32]
            },
            "preimage": {"byte_len": 7, "content_sha256": vec![3_u8; 32]}
        });
        assert!(
            serde_json::from_value::<agentmage_kernel_contracts::GrantTarget>(traversal).is_err()
        );

        let mut malformed_digest = baseline;
        malformed_digest
            .denied_argument_sha256s
            .insert("A".repeat(64));
        assert_eq!(
            PolicyEngine::new(malformed_digest).expect_err("digest must fail"),
            PolicyBuildError::InvalidDocument
        );
    }

    #[test]
    fn strict_local_profile_allows_only_exact_workspace_read_and_denies_every_other_operation() {
        let engine = PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
            revision: 1,
            actors: set(ActorId::from_raw("actor-local-0001")),
            tasks: set(TaskId::from_raw("task-0001")),
            actions: set(ActionId::from_raw("action-0001")),
            tools: set(ToolPolicyBinding {
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
            }),
            targets: set(target(&["src"])),
        })
        .expect("strict policy must build");
        assert_eq!(
            engine.document.operations.allowed,
            set(OperationBinding::new(GrantOperation::WorkspaceRead))
        );
        assert_eq!(
            engine.document.operations.denied,
            STRICT_LOCAL_DENIED_OPERATIONS
                .into_iter()
                .map(OperationBinding::new)
                .collect::<BTreeSet<_>>()
        );
        assert!(engine.document.network_scopes.allowed.is_empty());
        assert!(engine.document.credential_scopes.allowed.is_empty());
        assert!(engine.document.publication_scopes.allowed.is_empty());

        let (read_issuer, read_grant) = issued_operation(&engine, GrantOperation::WorkspaceRead);
        assert!(
            engine
                .evaluate(&read_issuer, &read_grant, &context(&read_grant))
                .allowed
        );

        for operation in STRICT_LOCAL_DENIED_OPERATIONS {
            let (issuer, grant) = issued_operation(&engine, operation);
            let decision = engine.evaluate(&issuer, &grant, &context(&grant));
            assert!(
                !decision.allowed,
                "operation unexpectedly allowed: {operation:?}"
            );
            assert_eq!(
                decision.denial_scope,
                Some(PolicyDenialScope::Operation),
                "wrong denial scope for: {operation:?}"
            );
        }
    }
}
