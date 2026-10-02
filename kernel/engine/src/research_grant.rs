//! The engine's issuance of one exact public GET grant for an admitted research
//! run (Decision 0139, AMR-02.2).
//!
//! The existing grant issuer derives the grant from the session parent that
//! records the person's confirmation of the exact plan. Under a task-authorized
//! plan that confirmation covers each request inside the plan; under an ask plan
//! the person must also approve the exact request. Before anything is committed,
//! the budget owner checks the request without spending, and the governing
//! policy evaluates the exact grant and its destination. Issuance spends no
//! budget and sends nothing. Reservation, grant consumption and dual-proof
//! dispatch stay at the effect's start.

use std::fmt;

use agentmage_kernel_contracts::{
    ActionKind, ApprovalId, CapabilityGrant, GrantId, GrantNonce, GrantOperation, GrantPreimage,
    GrantSideEffect, GrantTarget, OperationBinding, RuntimeEvent, ToolCall,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{DurableAuthorityError, DurableAuthorityRuntime};
use crate::grants::{DerivedOperationGrantRequest, GrantIssueError};
use crate::policy::{PolicyDenialScope, PolicyEngine, PolicyEvaluationContext};
use crate::research_budget::ResearchNetworkMode;
use crate::research_fetch::{PreparedPublicGet, PublicGetDraft};
use crate::research_journal::{
    CheckedResearchRequest, ResearchBudgetContext, ResearchJournalError,
};
use crate::runtime_artifact::RuntimeArtifactPayloadStore;
use crate::runtime_journal::RuntimeJournalError;
use crate::tooling::ToolRegistry;

const DECISION_DOMAIN: &str = "agentmage.research.public-get-grant";
const ROLLBACK_DESCRIPTION: &str = "A disclosed public GET cannot be undone";

/// The confirmation that authorizes one exact request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicGetAuthorization {
    /// The plan is task-authorized: the parent's confirmation of the exact
    /// plan covers each request inside it.
    TaskPlan,
    /// The plan asks: the person approved this exact request.
    ExactApproval {
        /// Digest of the request preview the person approved.
        approved_preview_sha256: String,
    },
}

/// Trusted inputs and kernel-selected identities for one issuance. The
/// request itself comes only from the call's exact argument bytes.
pub struct PublicGetGrantRequest<'a> {
    /// The admitted run's budget context.
    pub context: &'a ResearchBudgetContext,
    /// The exact model-proposed call.
    pub call: &'a ToolCall,
    /// The session parent that records the person's confirmation of the plan.
    pub parent_grant_id: &'a GrantId,
    /// The confirmation that authorizes this request.
    pub authorization: PublicGetAuthorization,
    /// The approval identity the grant carries.
    pub approval_id: ApprovalId,
    /// New single-use grant identity.
    pub grant_id: GrantId,
    /// Unique anti-replay nonce.
    pub nonce: GrantNonce,
    /// The one exact target, inside the parent's scope.
    pub target: GrantTarget,
    /// Kernel-clock instant at which the request's packet was prepared; for
    /// an ask plan, when it was presented for approval.
    pub prepared_at_epoch_ms: u64,
    /// Kernel-clock issuance instant.
    pub now_epoch_ms: u64,
}

/// Closed, content-free refusal. Nothing is committed and nothing is spent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicGetGrantError {
    /// The call is not one exact public GET under its registered tool.
    Call,
    /// The plan's network mode does not admit this kind of confirmation.
    Mode,
    /// The parent is not this task's confirmation of this exact plan.
    Parent,
    /// The person's approval does not name this exact request.
    Approval,
    /// The governing policy refuses the exact grant or its destination.
    Policy(PolicyDenialScope),
    /// The budget owner would not reserve this request now.
    Research(ResearchJournalError),
    /// The grant issuer refused the derivation.
    Grant(GrantIssueError),
    /// The durable owner failed or refused to commit.
    Authority(DurableAuthorityError),
}

/// One issued grant and the packet it names. It is not a reservation, a
/// consumed permit or a dispatch proof.
pub struct IssuedPublicGetGrant {
    grant: CapabilityGrant,
    authority_sha256: String,
    prepared: PreparedPublicGet,
    network: ResearchNetworkMode,
    decision_sha256: String,
}

impl fmt::Debug for IssuedPublicGetGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IssuedPublicGetGrant")
            .field("grant_id", &self.grant.grant_id)
            .field("decision_sha256", &self.decision_sha256)
            .finish_non_exhaustive()
    }
}

impl IssuedPublicGetGrant {
    /// The issued single-use network grant.
    #[must_use]
    pub const fn grant(&self) -> &CapabilityGrant {
        &self.grant
    }

    /// Digest of the issued grant revision, which the effect's start names.
    #[must_use]
    pub fn authority_sha256(&self) -> &str {
        &self.authority_sha256
    }

    /// The exact packet the grant's effect details name.
    #[must_use]
    pub const fn prepared(&self) -> &PreparedPublicGet {
        &self.prepared
    }

    /// The plan's network mode at issuance.
    #[must_use]
    pub const fn network(&self) -> ResearchNetworkMode {
        self.network
    }

    /// Digest of the issuance decision: mode, plan, parent, grant, request,
    /// approval and policy result.
    #[must_use]
    pub fn decision_sha256(&self) -> &str {
        &self.decision_sha256
    }

    /// Releases the grant and the packet for the effect's start.
    #[must_use]
    pub fn into_parts(self) -> (CapabilityGrant, PreparedPublicGet) {
        (self.grant, self.prepared)
    }
}

#[derive(Serialize)]
struct IssuanceDecision<'a> {
    domain: &'static str,
    schema_version: u16,
    network: ResearchNetworkMode,
    plan_sha256: &'a str,
    parent_grant_sha256: &'a str,
    grant_sha256: &'a str,
    request_sha256: &'a str,
    approved_preview_sha256: Option<&'a str>,
    policy_code: &'static str,
}

impl DurableAuthorityRuntime {
    /// Prepares, without spending or recording anything, the exact packet an
    /// ask plan presents for approval. Preparation is not an approval or a
    /// grant; the budget may change before the person answers.
    pub fn prepare_public_get<S: RuntimeArtifactPayloadStore>(
        &mut self,
        payloads: &S,
        registry: &ToolRegistry,
        context: &ResearchBudgetContext,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<PreparedPublicGet, PublicGetGrantError> {
        self.check_public_get(
            payloads,
            registry,
            context,
            call,
            now_epoch_ms,
            now_epoch_ms,
        )
        .map(|checked| checked.prepared)
    }

    /// Issues one exact single-use network grant for an admitted run and
    /// co-publishes its correctness event (Decision 0139).
    ///
    /// The budget owner must admit the request now under the retained plan; the
    /// plan's mode must match the confirmation; the parent must be this
    /// session's and task's confirmation of the exact plan; and the governing
    /// policy must allow the exact grant and the packet's destination. The
    /// grant names the packet as its only effect and expires no later than the
    /// packet or the parent.
    pub fn issue_public_get_grant<S, R, F>(
        &mut self,
        payloads: &S,
        registry: &ToolRegistry,
        policy: &PolicyEngine,
        request: PublicGetGrantRequest<'_>,
        build_event: F,
    ) -> Result<(IssuedPublicGetGrant, R, RuntimeEvent), PublicGetGrantError>
    where
        S: RuntimeArtifactPayloadStore,
        F: FnOnce(&IssuedPublicGetGrant) -> Result<(R, RuntimeEvent), RuntimeJournalError>,
    {
        let PublicGetGrantRequest {
            context,
            call,
            parent_grant_id,
            authorization,
            approval_id,
            grant_id,
            nonce,
            target,
            prepared_at_epoch_ms,
            now_epoch_ms,
        } = request;
        let checked = self.check_public_get(
            payloads,
            registry,
            context,
            call,
            prepared_at_epoch_ms,
            now_epoch_ms,
        )?;
        let packet = checked.prepared.packet();
        let approved_preview_sha256 = match (&authorization, checked.network) {
            (PublicGetAuthorization::TaskPlan, ResearchNetworkMode::TaskAuthorized) => None,
            (
                PublicGetAuthorization::ExactApproval {
                    approved_preview_sha256,
                },
                ResearchNetworkMode::Ask,
            ) => {
                if approved_preview_sha256 != packet.sha256() {
                    return Err(PublicGetGrantError::Approval);
                }
                Some(approved_preview_sha256.as_str())
            }
            _ => return Err(PublicGetGrantError::Mode),
        };
        if policy.policy_sha256() != context.policy_sha256 {
            return Err(PublicGetGrantError::Grant(GrantIssueError::PolicyChanged));
        }
        let parent = self
            .issuer
            .current(parent_grant_id)
            .cloned()
            .ok_or(PublicGetGrantError::Grant(GrantIssueError::ParentNotFound))?;
        let parent_sha256 = self
            .issuer
            .revision_hash(&parent.grant_id, parent.revision)
            .ok_or(PublicGetGrantError::Parent)?
            .to_owned();
        // The parent's preview is what the person confirmed: the exact plan.
        // Derivation below refuses a parent that is not a session read.
        if parent.session_id != context.session_id
            || parent.task_id != context.task_id
            || parent.preview_sha256 != checked.plan_sha256
        {
            return Err(PublicGetGrantError::Parent);
        }
        // One grant per call: an action that already holds a grant, in any
        // state, never receives another.
        if self
            .issuer
            .durable_parts()
            .histories
            .values()
            .flatten()
            .any(|grant| grant.action_id.as_ref() == Some(&call.action_id))
        {
            return Err(PublicGetGrantError::Grant(GrantIssueError::DuplicateGrant));
        }
        let operation = OperationBinding::new(GrantOperation::NetworkAccess);
        let preimages: Vec<GrantPreimage> =
            GrantPreimage::for_target(0, &target).into_iter().collect();
        let mut candidate = self.issuer.clone();
        let grant = candidate
            .derive_operation(
                parent_grant_id,
                DerivedOperationGrantRequest {
                    grant_id,
                    approval_id,
                    action_id: call.action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    operation,
                    tool_id: call.tool_id.clone(),
                    tool_version: call.tool_version.clone(),
                    targets: vec![target],
                    argument_sha256: call.arguments.sha256.clone(),
                    preimages,
                    expected_side_effects: vec![GrantSideEffect {
                        operation,
                        target_indexes: vec![0],
                        details_sha256: packet.sha256().to_owned(),
                    }],
                    rollback_description: ROLLBACK_DESCRIPTION.to_owned(),
                    issued_at_epoch_ms: now_epoch_ms,
                    expires_at_epoch_ms: packet.deadline_epoch_ms().min(parent.expires_at_epoch_ms),
                    nonce,
                    preview_sha256: packet.sha256().to_owned(),
                    policy_sha256: context.policy_sha256.clone(),
                },
            )
            .map_err(PublicGetGrantError::Grant)?;
        // The same evaluation the authority transaction repeats at the start,
        // in the run's session and task, with the packet's own destination.
        let decision = policy.evaluate(
            &candidate,
            &grant,
            &PolicyEvaluationContext {
                actor_id: grant.actor_id.clone(),
                session_id: context.session_id.clone(),
                task_id: context.task_id.clone(),
                action_id: call.action_id.clone(),
                action_kind: ActionKind::DeterministicTool,
                tool_id: call.tool_id.clone(),
                tool_version: call.tool_version.clone(),
                targets: grant.targets.clone(),
                argument_sha256: grant.argument_sha256.clone(),
                preimages: grant.preimages.clone(),
                expected_side_effects: grant.expected_side_effects.clone(),
                preview_sha256: grant.preview_sha256.clone(),
                now_epoch_ms,
                network_scope: Some(format!("https:{}:443", packet.request().target.domain)),
                credential_scope: None,
                publication_scope: None,
            },
        );
        if !decision.allowed {
            return Err(PublicGetGrantError::Policy(
                decision.denial_scope.unwrap_or(PolicyDenialScope::Grant),
            ));
        }
        let authority_sha256 = candidate
            .revision_hash(&grant.grant_id, grant.revision)
            .ok_or(PublicGetGrantError::Authority(
                DurableAuthorityError::Poisoned,
            ))?
            .to_owned();
        let decision_bytes = serde_json::to_vec(&IssuanceDecision {
            domain: DECISION_DOMAIN,
            schema_version: 1,
            network: checked.network,
            plan_sha256: &checked.plan_sha256,
            parent_grant_sha256: &parent_sha256,
            grant_sha256: &authority_sha256,
            request_sha256: packet.sha256(),
            approved_preview_sha256,
            policy_code: decision.code,
        })
        .map_err(|_| PublicGetGrantError::Call)?;
        let issued = IssuedPublicGetGrant {
            grant,
            authority_sha256,
            decision_sha256: sha256_hex(&decision_bytes),
            network: checked.network,
            prepared: checked.prepared,
        };
        let (result, event) = build_event(&issued).map_err(|error| {
            PublicGetGrantError::Authority(DurableAuthorityError::RuntimeJournal(error))
        })?;
        self.commit_candidate_with_runtime_events(candidate, std::slice::from_ref(&event))
            .map_err(PublicGetGrantError::Authority)?;
        Ok((issued, result, event))
    }

    fn check_public_get<S: RuntimeArtifactPayloadStore>(
        &mut self,
        payloads: &S,
        registry: &ToolRegistry,
        context: &ResearchBudgetContext,
        call: &ToolCall,
        prepared_at_epoch_ms: u64,
        now_epoch_ms: u64,
    ) -> Result<CheckedResearchRequest, PublicGetGrantError> {
        self.ensure_usable()
            .map_err(PublicGetGrantError::Authority)?;
        let definition = registry
            .get_tool(&call.tool_id, &call.tool_version)
            .ok_or(PublicGetGrantError::Call)?;
        // The argument bytes are bounded and bound to their digest before they
        // are decoded (Decision 0140, note N2); the call validation below
        // repeats both checks against the prepared packet.
        if call.arguments.bytes.is_empty()
            || call.arguments.bytes.len() > crate::research_effect_binding::MAX_ARGUMENT_BYTES
            || sha256_hex(&call.arguments.bytes) != call.arguments.sha256
        {
            return Err(PublicGetGrantError::Call);
        }
        let request: PublicGetDraft =
            serde_json::from_slice(&call.arguments.bytes).map_err(|_| PublicGetGrantError::Call)?;
        let result = {
            let store = self.lock_store().map_err(PublicGetGrantError::Authority)?;
            crate::research_journal::check_request(
                &store,
                payloads,
                context,
                request,
                prepared_at_epoch_ms,
                now_epoch_ms,
            )
        };
        let checked = result.map_err(|error| {
            if error.poisons_runtime() {
                self.poisoned = true;
            }
            PublicGetGrantError::Research(error)
        })?;
        crate::research_effect_binding::validate_call(definition, call, checked.prepared.packet())
            .map_err(|_| PublicGetGrantError::Call)?;
        Ok(checked)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
