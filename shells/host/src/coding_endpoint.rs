// SPDX-License-Identifier: Apache-2.0
//! Approved endpoint turns in the reusable coordinator, without another agent loop.
//!
//! The caller owns the original disclosure consent and trusted context source.
//! This adapter never reads files, chooses tools, grants permissions or restores
//! run ownership. The coordinator alone admits turns, tools and completion.

use std::sync::Arc;

use agentmage_kernel_contracts::{
    EndpointModelRequest, EndpointModelResult, EndpointModelSelection, ModelOperationControl,
    ModelOperationStop, RuntimeModelBinding, SchemaReference,
};
use agentmage_kernel_engine::runtime_loop::{
    RuntimeEndpointPreparationInput, RuntimeEndpointTurnPort, RuntimePortFailure,
    RuntimePreparedEndpointTurn,
    endpoint::{
        RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
        RuntimeEndpointModelPort,
    },
};
use agentmage_platform_linux_inference::{
    local_chat::{ChatMessage, ChatRole},
    local_chat_transport::{
        ChatCancellation, LocalChatApproval, LocalChatTransport, PreparedEndpointChat,
        TransportError,
    },
};

/// Trusted context preparation for the current coordinator-owned observations.
/// This is an in-process port, never an authority-bearing wire descriptor.
pub trait CodingEndpointContextPort {
    /// Builds the original ordered messages once for this turn. Source reads and
    /// disclosure require the caller's existing consent; untrusted source/model/
    /// tool text must stay identified as such. Honor the borrowed run control.
    /// Context may not infer new grants or completed effects from model prose.
    /// The source owns private text and its erasure; the adapter borrows it only
    /// while freezing the transport body, which the transport erases on drop.
    fn build_messages(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<Vec<ChatMessage<'_>>, RuntimePortFailure>;
}

/// Original transport and destination approval, either borrowed or retained.
/// Shared ownership keeps exactly these immutable instances alive; it grants
/// no disclosure, endpoint selection, tool or resume authority.
pub enum CodingEndpointConnection<'config> {
    /// Configuration held by the caller for the run's entire lifetime.
    Borrowed {
        /// Original credential-owning transport.
        transport: &'config LocalChatTransport,
        /// Original exact destination consent.
        approval: &'config LocalChatApproval,
    },
    /// Configuration retained independently of later settings replacements.
    Shared {
        /// Original credential-owning transport; credentials are never copied.
        transport: Arc<LocalChatTransport>,
        /// Original exact destination consent, rechecked each preparation.
        approval: Arc<LocalChatApproval>,
    },
}

impl CodingEndpointConnection<'_> {
    pub(super) fn transport(&self) -> &LocalChatTransport {
        match self {
            Self::Borrowed { transport, .. } => transport,
            Self::Shared { transport, .. } => transport,
        }
    }
}

/// Credential-owning connection plus the original destination approval.
/// Construction is inert. Keep this inside the run's exclusive live owner; its
/// metadata cannot restore a run, renew budgets or admit a native model.
pub struct ConfiguredCodingEndpoint<'config, X> {
    connection: CodingEndpointConnection<'config>,
    selection: EndpointModelSelection,
    max_output_tokens: u32,
    include_usage: bool,
    completion_schema: SchemaReference,
    context: X,
}

impl<'config, X: CodingEndpointContextPort> ConfiguredCodingEndpoint<'config, X> {
    /// Binds the actual configured transport, the chosen model and trusted
    /// context owner. Destination approval and request limits are checked again
    /// for each frozen request. No probe, socket, file read or dispatch occurs.
    pub fn new(
        transport: &'config LocalChatTransport,
        approval: &'config LocalChatApproval,
        model: &str,
        max_output_tokens: u32,
        include_usage: bool,
        completion_schema: SchemaReference,
        context: X,
    ) -> Result<Self, RuntimePortFailure> {
        Self::with_connection(
            CodingEndpointConnection::Borrowed {
                transport,
                approval,
            },
            model,
            max_output_tokens,
            include_usage,
            completion_schema,
            context,
        )
    }

    /// Retains the original approved connection without borrowing service
    /// settings. The ordinary preparation path still checks exact approval on
    /// every turn, and a prepared request retains its original transport.
    pub fn with_connection(
        connection: CodingEndpointConnection<'config>,
        model: &str,
        max_output_tokens: u32,
        include_usage: bool,
        completion_schema: SchemaReference,
        context: X,
    ) -> Result<Self, RuntimePortFailure> {
        if !(1..=32_768).contains(&max_output_tokens)
            || model.is_empty()
            || model.len() > 256
            || model.trim() != model
            || model.chars().any(char::is_control)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(Self {
            selection: connection.transport().model_selection(model),
            connection,
            max_output_tokens,
            include_usage,
            completion_schema,
            context,
        })
    }
}

// The sole prepared object borrows external configuration or retains its exact
// shared transport. Encoded bytes outlive the temporary context borrow.
// Neither dispatch nor a callback can rebuild context or resolve new credentials.
struct ConfiguredDispatch<'config> {
    prepared: PreparedEndpointChat<'config>,
}

impl RuntimeEndpointModelPort for ConfiguredDispatch<'_> {
    type Error = RuntimePortFailure;

    fn request(&self) -> &EndpointModelRequest {
        self.prepared.binding()
    }

    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        // The transport's cancellation is additional, never a replacement for
        // the original coordinator control passed through every socket wait.
        self.prepared
            .execute_recorded_controlled(&ChatCancellation::default(), control, text)
            .map_err(transport_failure)
    }
}

impl<X: CodingEndpointContextPort> RuntimeEndpointTurnPort for ConfiguredCodingEndpoint<'_, X> {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        RuntimeModelBinding::SelectedEndpoint {
            endpoint: &self.selection.endpoint,
            model: &self.selection.model,
        }
    }

    fn prepare<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        control.remaining_ms().map_err(control_failure)?;
        if input.request.model_selection != self.selection {
            return Err(RuntimePortFailure::Invalid);
        }
        let original = self.context.build_messages(input, control)?;
        control.remaining_ms().map_err(control_failure)?;
        let messages: Vec<_> = original
            .iter()
            .map(|message| RuntimeEndpointMessage {
                role: match message.role {
                    ChatRole::System => RuntimeEndpointMessageRole::System,
                    ChatRole::User => RuntimeEndpointMessageRole::User,
                    ChatRole::Assistant => RuntimeEndpointMessageRole::Assistant,
                },
                content: message.content,
            })
            .collect();
        let context = RuntimeEndpointContextPacket::from_messages(&messages)?;
        let prepared = match &self.connection {
            CodingEndpointConnection::Borrowed {
                transport,
                approval,
            } => transport.prepare_chat(
                approval,
                &self.selection.model,
                &original,
                self.max_output_tokens,
                self.include_usage,
            ),
            CodingEndpointConnection::Shared {
                transport,
                approval,
            } => Arc::clone(transport).prepare_chat_shared(
                approval,
                &self.selection.model,
                &original,
                self.max_output_tokens,
                self.include_usage,
            ),
        }
        .map_err(transport_failure)?;
        control.remaining_ms().map_err(control_failure)?;
        Ok(RuntimePreparedEndpointTurn::new(
            context,
            ConfiguredDispatch { prepared },
            self.completion_schema.clone(),
        ))
    }
}

pub(super) fn control_failure(stop: ModelOperationStop) -> RuntimePortFailure {
    match stop {
        ModelOperationStop::Cancelled => RuntimePortFailure::Cancelled,
        ModelOperationStop::TimedOut => RuntimePortFailure::TimedOut,
        ModelOperationStop::Unavailable => RuntimePortFailure::Unavailable,
        ModelOperationStop::Invalid => RuntimePortFailure::Invalid,
    }
}

pub(super) fn transport_failure(error: TransportError) -> RuntimePortFailure {
    match error {
        TransportError::OperationStopped(stop) => control_failure(stop),
        TransportError::Cancelled => RuntimePortFailure::Cancelled,
        TransportError::Timeout => RuntimePortFailure::TimedOut,
        TransportError::Connection | TransportError::Authentication | TransportError::Status => {
            RuntimePortFailure::Unavailable
        }
        TransportError::Limit => RuntimePortFailure::ResourceExhausted,
        _ => RuntimePortFailure::Invalid,
    }
}
