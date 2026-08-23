//! Distinct identities used by interface-independent contracts.

macro_rules! define_identifier {
    ($name:ident, $documentation:literal) => {
        #[doc = $documentation]
        #[derive(
            Clone,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a candidate identity from its wire value.
            ///
            /// Strict syntax and size validation belongs to the versioned parser. Keeping
            /// construction separate from parsing lets tests build malformed candidates
            /// without weakening the parser boundary.
            #[must_use]
            pub fn from_raw(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the candidate wire value.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the identity and returns its wire value.
            #[must_use]
            pub fn into_string(self) -> String {
                self.0
            }
        }
    };
}

define_identifier!(
    SessionId,
    "Stable identity for one local AgentMage session."
);
define_identifier!(
    ConversationId,
    "Stable identity for one persisted local conversation."
);
define_identifier!(
    ConversationTurnId,
    "Stable identity for one immutable persisted conversation turn."
);
define_identifier!(
    ConversationCompactionId,
    "Stable identity for one checked append-only conversation compaction."
);
define_identifier!(TaskId, "Stable identity for one user-directed task.");
define_identifier!(
    ArtifactUploadId,
    "Stable identity for one ordered artifact-upload attempt."
);
define_identifier!(
    EndpointProfileId,
    "Stable identity for one exact model endpoint profile."
);
define_identifier!(
    RouteDecisionId,
    "Stable identity for one deterministic model-route decision."
);
define_identifier!(
    CapabilityId,
    "Stable identity for one versioned executable engineering capability."
);
define_identifier!(
    CampaignId,
    "Stable identity for one persistent multi-agent campaign."
);
define_identifier!(
    AgentLeaseId,
    "Stable identity for one bounded multi-agent task lease."
);
define_identifier!(ReviewId, "Stable identity for one independent review.");
define_identifier!(
    IntegrationId,
    "Stable identity for one serialized campaign integration attempt."
);
define_identifier!(
    RuntimeRunId,
    "Stable identity for one bounded reusable-runtime run."
);
define_identifier!(
    RuntimeTurnId,
    "Stable identity for one ordered turn within a runtime run."
);
define_identifier!(
    RuntimeOperationId,
    "Stable identity for one proposed or executing runtime operation."
);
define_identifier!(
    RuntimeEventId,
    "Stable identity for one immutable runtime event."
);
define_identifier!(
    RuntimeArtifactId,
    "Stable identity for one content-addressed runtime artifact."
);
define_identifier!(
    WorkPacketId,
    "Stable identity for one revision-controlled work packet."
);
define_identifier!(PlanId, "Stable identity for one task plan.");
define_identifier!(PlanStepId, "Stable identity for one plan step.");
define_identifier!(PromptId, "Stable identity for one assembled model prompt.");
define_identifier!(
    ProposalId,
    "Stable identity for one model proposal candidate."
);
define_identifier!(ModelRunId, "Stable identity for one bounded model run.");
define_identifier!(
    ModelProfileId,
    "Stable identity for one exact model profile tuple."
);
define_identifier!(
    ModelManifestId,
    "Stable identity for one exact model manifest."
);
define_identifier!(
    ModelAdapterId,
    "Stable identity for one model runtime adapter contract."
);
define_identifier!(
    ModelCodecId,
    "Stable identity for one closed model-family codec."
);
define_identifier!(
    ModelMessageId,
    "Stable identity for one bounded model message."
);
define_identifier!(
    ModelStreamId,
    "Stable identity for one bounded model response stream."
);
define_identifier!(
    ContextPacketId,
    "Stable identity for one bounded context packet."
);
define_identifier!(
    ContextSummaryId,
    "Stable identity for one checked context summary."
);
define_identifier!(
    SessionCheckpointId,
    "Stable identity for one safe-boundary session checkpoint."
);
define_identifier!(
    RepositorySnapshotId,
    "Stable identity for one repository snapshot."
);
define_identifier!(
    ToolCatalogId,
    "Stable identity for one frozen tool catalog."
);
define_identifier!(
    PolicyId,
    "Stable identity for one deterministic policy revision."
);
define_identifier!(
    VerifierId,
    "Stable identity for one deterministic verifier."
);
define_identifier!(
    VerifierRecordId,
    "Stable identity for one verifier result record."
);
define_identifier!(
    PostconditionId,
    "Stable identity for one typed postcondition."
);
define_identifier!(
    ActionId,
    "Stable identity for one proposed or executed action."
);
define_identifier!(ToolId, "Stable identity for one registered tool contract.");
define_identifier!(ToolCallId, "Stable identity for one tool-call attempt.");
define_identifier!(ActorId, "Stable pseudonymous identity for one local actor.");
define_identifier!(GrantId, "Stable identity for one capability grant.");
define_identifier!(GrantNonce, "Single-grant nonce used to prevent replay.");
define_identifier!(
    ApprovalId,
    "Stable identity for one explicit approval decision."
);
define_identifier!(
    AuthorityTransactionId,
    "Stable identity for one kernel-owned authority transaction."
);
define_identifier!(
    OperationAttemptId,
    "Stable identity for one non-replayable operation attempt."
);
define_identifier!(WorkspaceId, "Stable identity for one approved workspace.");
define_identifier!(
    WorkspaceAuthorizationId,
    "Stable identity for one user-approved workspace authorization event."
);
define_identifier!(
    AdapterInstanceId,
    "Stable identity for one selected platform-adapter instance."
);
define_identifier!(EvidenceId, "Stable identity for one evidence record.");
define_identifier!(ReceiptId, "Stable identity for one operation receipt.");
define_identifier!(ErrorId, "Stable identity for one typed contract error.");
define_identifier!(
    CancellationId,
    "Stable identity for one cancellation request and propagation tree."
);
define_identifier!(
    CorrelationId,
    "Stable identity joining one request and its derived records."
);
define_identifier!(
    SchemaId,
    "Stable identity for one versioned payload schema."
);

#[cfg(test)]
mod tests {
    use super::{ActionId, TaskId};

    #[test]
    fn identities_preserve_exact_candidate_values() {
        let task = TaskId::from_raw("task-0001");
        let action = ActionId::from_raw("action-0001");
        assert_eq!(task.as_str(), "task-0001");
        assert_eq!(action.into_string(), "action-0001");
    }
}
