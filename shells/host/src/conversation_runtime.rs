//! Shared shell routing into the canonical kernel conversation library.

use agentmage_kernel_contracts::{ConversationId, ConversationRecord, ConversationTurnId};
use agentmage_kernel_engine::{
    context_management::ResumeObservation,
    conversation_library::{
        ConversationBranchPreview, ConversationHistory, ConversationLibraryError,
        ConversationMutationReceipt, ConversationQuery, ConversationResumeReview,
        ConversationSearchHit,
    },
    operational_store::OperationalStore,
};

use crate::headless::{ClientCommand, ConversationClientCommand, ThinClientError};

const MAX_RESULTS: u32 = 1_000;

/// Additional kernel-owned evidence required by state-sensitive conversation commands.
#[derive(Clone, Debug)]
pub enum ConversationCommandContext {
    /// No state-sensitive evidence is required.
    None,
    /// Current environment evidence for an in-place resume review.
    Resume(ResumeObservation),
    /// Current environment evidence and the complete proposed child record for a branch.
    Branch {
        /// Exact current environment observation.
        observation: ResumeObservation,
        /// Complete proposed child conversation; the kernel verifies its parent and turn.
        proposed_conversation: Box<ConversationRecord>,
    },
}

/// Closed result family returned to every first-party shell.
#[derive(Clone, Debug)]
pub enum ConversationCommandResult {
    /// Bounded content-minimized search or list results.
    Search(Vec<ConversationSearchHit>),
    /// Complete verified read-only history for show or open.
    History(Box<ConversationHistory>),
    /// No-write resume review against the latest recorded checkpoint.
    ResumeReview(Box<ConversationResumeReview>),
    /// Kernel-created child branch receipt.
    Branch(ConversationMutationReceipt),
}

/// Stable shell-to-kernel conversation routing failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationRuntimeError {
    /// The client command or supplied command context is malformed or mismatched.
    InvalidRequest,
    /// The canonical kernel conversation library rejected the operation.
    Kernel(ConversationLibraryError),
}

impl From<ConversationLibraryError> for ConversationRuntimeError {
    fn from(value: ConversationLibraryError) -> Self {
        Self::Kernel(value)
    }
}

impl From<ThinClientError> for ConversationRuntimeError {
    fn from(_: ThinClientError) -> Self {
        Self::InvalidRequest
    }
}

/// Narrow kernel surface used by the shared shell router.
pub trait ConversationKernel {
    /// Executes one bounded canonical query.
    fn search(
        &self,
        query: &ConversationQuery,
    ) -> Result<Vec<ConversationSearchHit>, ConversationLibraryError>;

    /// Loads one complete verified read-only history.
    fn history(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationHistory, ConversationLibraryError>;

    /// Reviews the latest resumable checkpoint without changing state.
    fn review_latest_resume(
        &self,
        conversation_id: &ConversationId,
        observation: &ResumeObservation,
    ) -> Result<ConversationResumeReview, ConversationLibraryError>;

    /// Constructs an exact no-write child-branch preview.
    fn preview_branch(
        &self,
        conversation_id: &ConversationId,
        turn_id: &ConversationTurnId,
        proposed_conversation: ConversationRecord,
        observation: &ResumeObservation,
    ) -> Result<ConversationBranchPreview, ConversationLibraryError>;

    /// Applies only the exact still-current kernel branch preview.
    fn apply_branch(
        &mut self,
        preview: &ConversationBranchPreview,
        observation: &ResumeObservation,
    ) -> Result<ConversationMutationReceipt, ConversationLibraryError>;
}

impl ConversationKernel for OperationalStore {
    fn search(
        &self,
        query: &ConversationQuery,
    ) -> Result<Vec<ConversationSearchHit>, ConversationLibraryError> {
        self.search_conversations(query)
    }

    fn history(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationHistory, ConversationLibraryError> {
        self.conversation_history(conversation_id)
    }

    fn review_latest_resume(
        &self,
        conversation_id: &ConversationId,
        observation: &ResumeObservation,
    ) -> Result<ConversationResumeReview, ConversationLibraryError> {
        self.review_latest_conversation_resume(conversation_id, observation)
    }

    fn preview_branch(
        &self,
        conversation_id: &ConversationId,
        turn_id: &ConversationTurnId,
        proposed_conversation: ConversationRecord,
        observation: &ResumeObservation,
    ) -> Result<ConversationBranchPreview, ConversationLibraryError> {
        self.preview_conversation_branch(
            conversation_id,
            turn_id,
            proposed_conversation,
            observation,
        )
    }

    fn apply_branch(
        &mut self,
        preview: &ConversationBranchPreview,
        observation: &ResumeObservation,
    ) -> Result<ConversationMutationReceipt, ConversationLibraryError> {
        self.apply_conversation_branch(preview, observation)
    }
}

/// Routes a verified command from any first-party shell through the kernel conversation API.
pub fn execute_conversation_command<K: ConversationKernel>(
    kernel: &mut K,
    command: &ConversationClientCommand,
    context: ConversationCommandContext,
) -> Result<ConversationCommandResult, ConversationRuntimeError> {
    ClientCommand::Conversations {
        action: command.clone(),
    }
    .verify()?;
    match (command, context) {
        (ConversationClientCommand::List { from, to }, ConversationCommandContext::None) => {
            let query = ConversationQuery {
                from_local_date: from.clone(),
                to_local_date: to.clone(),
                limit: MAX_RESULTS,
                ..ConversationQuery::default()
            };
            Ok(ConversationCommandResult::Search(kernel.search(&query)?))
        }
        (ConversationClientCommand::Search { query }, ConversationCommandContext::None) => {
            let query = ConversationQuery {
                text: Some(query.clone()),
                limit: MAX_RESULTS,
                ..ConversationQuery::default()
            };
            Ok(ConversationCommandResult::Search(kernel.search(&query)?))
        }
        (
            ConversationClientCommand::Show { conversation_id }
            | ConversationClientCommand::Open { conversation_id },
            ConversationCommandContext::None,
        ) => Ok(ConversationCommandResult::History(Box::new(
            kernel.history(&ConversationId::from_raw(conversation_id.clone()))?,
        ))),
        (
            ConversationClientCommand::Resume { conversation_id },
            ConversationCommandContext::Resume(observation),
        ) => Ok(ConversationCommandResult::ResumeReview(Box::new(
            kernel.review_latest_resume(
                &ConversationId::from_raw(conversation_id.clone()),
                &observation,
            )?,
        ))),
        (
            ConversationClientCommand::Branch {
                conversation_id,
                turn_id,
            },
            ConversationCommandContext::Branch {
                observation,
                proposed_conversation,
            },
        ) => {
            let preview = kernel.preview_branch(
                &ConversationId::from_raw(conversation_id.clone()),
                &ConversationTurnId::from_raw(turn_id.clone()),
                *proposed_conversation,
                &observation,
            )?;
            Ok(ConversationCommandResult::Branch(
                kernel.apply_branch(&preview, &observation)?,
            ))
        }
        _ => Err(ConversationRuntimeError::InvalidRequest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct QueryKernel;

    impl ConversationKernel for QueryKernel {
        fn search(
            &self,
            _query: &ConversationQuery,
        ) -> Result<Vec<ConversationSearchHit>, ConversationLibraryError> {
            Ok(Vec::new())
        }

        fn history(
            &self,
            _conversation_id: &ConversationId,
        ) -> Result<ConversationHistory, ConversationLibraryError> {
            Err(ConversationLibraryError::NotFound)
        }

        fn review_latest_resume(
            &self,
            _conversation_id: &ConversationId,
            _observation: &ResumeObservation,
        ) -> Result<ConversationResumeReview, ConversationLibraryError> {
            Err(ConversationLibraryError::NotFound)
        }

        fn preview_branch(
            &self,
            _conversation_id: &ConversationId,
            _turn_id: &ConversationTurnId,
            _proposed_conversation: ConversationRecord,
            _observation: &ResumeObservation,
        ) -> Result<ConversationBranchPreview, ConversationLibraryError> {
            Err(ConversationLibraryError::NotFound)
        }

        fn apply_branch(
            &mut self,
            _preview: &ConversationBranchPreview,
            _observation: &ResumeObservation,
        ) -> Result<ConversationMutationReceipt, ConversationLibraryError> {
            Err(ConversationLibraryError::NotFound)
        }
    }

    #[test]
    fn list_and_search_have_no_non_kernel_context_or_authority() {
        let mut kernel = QueryKernel;
        for command in [
            ConversationClientCommand::List {
                from: Some("2026-01-01".to_owned()),
                to: Some("2026-12-31".to_owned()),
            },
            ConversationClientCommand::Search {
                query: "exact text".to_owned(),
            },
        ] {
            let result = execute_conversation_command(
                &mut kernel,
                &command,
                ConversationCommandContext::None,
            )
            .expect("query routes through kernel");
            assert!(matches!(result, ConversationCommandResult::Search(items) if items.is_empty()));
        }
    }

    #[test]
    fn state_sensitive_commands_fail_without_exact_kernel_context() {
        let mut kernel = QueryKernel;
        for command in [
            ConversationClientCommand::Resume {
                conversation_id: "conversation-01".to_owned(),
            },
            ConversationClientCommand::Branch {
                conversation_id: "conversation-01".to_owned(),
                turn_id: "turn-01".to_owned(),
            },
        ] {
            assert_eq!(
                execute_conversation_command(
                    &mut kernel,
                    &command,
                    ConversationCommandContext::None,
                )
                .expect_err("context is mandatory"),
                ConversationRuntimeError::InvalidRequest,
            );
        }
    }
}
