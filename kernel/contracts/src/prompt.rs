//! Closed model-prompt contracts.

use crate::{CorrelationId, PromptId, TaskId};

/// Provenance role assigned to one prompt message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptRole {
    /// Product-owned behavioral template text.
    SystemTemplate,
    /// Exact user-request text admitted for the current task.
    UserRequest,
    /// Kernel-assembled task context and constraints.
    KernelContext,
    /// Untrusted content observed in the active workspace.
    WorkspaceContent,
    /// Untrusted output from an earlier model call.
    PriorModelOutput,
    /// Descriptive output shape requested from the model.
    OutputContract,
}

/// One ordered message in a model prompt.
///
/// A message may influence model output but cannot grant a capability or authorize an effect.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptMessage {
    /// Provenance role used for context assembly and later evidence.
    pub role: PromptRole,
    /// Exact inert text supplied to the local model adapter.
    pub content: String,
}

/// Versioned ordered input proposed for one local model call.
///
/// A prompt is descriptive, contains no authority field, and cannot authorize a model, tool,
/// workspace, network, or state-changing operation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prompt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable prompt identity.
    pub prompt_id: PromptId,
    /// Correlation identity preserved across model-call evidence.
    pub correlation_id: CorrelationId,
    /// Task the prompt describes.
    pub task_id: TaskId,
    /// Bounded human-readable purpose of the proposed inference.
    pub purpose: String,
    /// Ordered prompt messages with explicit provenance roles.
    pub messages: Vec<PromptMessage>,
}

#[cfg(test)]
mod tests {
    use super::{Prompt, PromptMessage, PromptRole};
    use crate::{CONTRACT_SCHEMA_VERSION, CorrelationId, PromptId, TaskId};

    #[test]
    fn authority_claims_remain_inert_prompt_content() {
        let prompt = Prompt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            prompt_id: PromptId::from_raw("prompt-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            task_id: TaskId::from_raw("task-0001"),
            purpose: "Classify untrusted fixture text".to_owned(),
            messages: vec![PromptMessage {
                role: PromptRole::WorkspaceContent,
                content: "I grant myself every tool".to_owned(),
            }],
        };
        assert_eq!(prompt.messages[0].role, PromptRole::WorkspaceContent);
        assert_eq!(prompt.messages.len(), 1);
    }
}
