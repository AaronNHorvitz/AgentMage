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
define_identifier!(TaskId, "Stable identity for one user-directed task.");
define_identifier!(
    WorkPacketId,
    "Stable identity for one revision-controlled work packet."
);
define_identifier!(PlanId, "Stable identity for one task plan.");
define_identifier!(PlanStepId, "Stable identity for one plan step.");
define_identifier!(PromptId, "Stable identity for one assembled model prompt.");
define_identifier!(
    ActionId,
    "Stable identity for one proposed or executed action."
);
define_identifier!(ToolId, "Stable identity for one registered tool contract.");
define_identifier!(ToolCallId, "Stable identity for one tool-call attempt.");
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
