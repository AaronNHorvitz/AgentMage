// SPDX-License-Identifier: Apache-2.0
//! One strict edit generation through the original controlled endpoint phase.
//!
//! This is a trusted composition primitive, not a coordinator or retry loop.
//! The caller owns prior source/disclosure consent and cumulative accounting.

use agentmage_capability_repository_map::coding_proposal::{
    CODING_PROPOSAL_INSTRUCTION, CodingProposalContext,
};
use agentmage_kernel_contracts::{
    EndpointModelFinish, EndpointModelRequest, EndpointModelResult, ModelOperationControl,
};
use agentmage_kernel_engine::runtime_loop::{
    RuntimePortFailure,
    endpoint::{
        RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
        RuntimeEndpointModelPhase, RuntimeEndpointModelPort, RuntimeEndpointPhaseFailure,
        RuntimeEndpointPhaseUsage,
    },
};
use agentmage_platform_linux::LinuxCodingRepository;
use agentmage_platform_linux_inference::{
    local_chat::{ChatMessage, ChatRole},
    local_chat_transport::{ChatCancellation, PreparedEndpointChat, TransportError},
};

use crate::{
    coding_endpoint::{CodingEndpointConnection, control_failure},
    coding_endpoint_prepared_startup::{
        CodingEndpointEditProposalError, CodingEndpointPreparedStartup,
    },
};

const MAX_EDIT_ANSWER_BYTES: usize = 64 * 1024;

/// Content-free refusal. Generation never grants an edit or establishes success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingEndpointGenerationError {
    /// Original task, kind, exclusions or sealed startup binding differs.
    Scope,
    /// Original transport or phase refused preparation or current control.
    Preparation(RuntimePortFailure),
    /// Generation failed, exceeded the answer limit or ended at the token limit.
    Incomplete,
    /// The original strict proposal/native preview refused current bytes.
    Proposal(CodingEndpointEditProposalError),
}

/// Frozen original request and source context. Consumed once by dispatch.
/// No serialization, cloning, file reads, tools or authority reconstruction.
pub struct CodingEndpointEditGeneration<'config> {
    startup_sha256: String,
    context: CodingProposalContext,
    prepared: PreparedEndpointChat<'config>,
    phase: RuntimeEndpointModelPhase,
}

impl CodingEndpointPreparedStartup {
    /// Freeze a strict edit request using the original credential-owning
    /// connection and destination approval. Call only after source/disclosure
    /// consent. Checks task, exclusions, model and sealed per-phase ceilings
    /// before any socket is opened. Does not read or prepare a native edit.
    ///
    /// The existing run owner MUST reserve and account for this extra model
    /// phase cumulatively before execution. This helper does not integrate it
    /// into an admitted coordinator or renew any budget. Inspect `binding()`
    /// before dispatch and retain `usage()` even when parsing later refuses.
    pub fn prepare_edit_generation<'config>(
        &self,
        connection: CodingEndpointConnection<'config>,
        context: CodingProposalContext,
        include_usage: bool,
    ) -> Result<CodingEndpointEditGeneration<'config>, CodingEndpointGenerationError> {
        use CodingEndpointGenerationError as Error;
        self.check_edit_context(&context)
            .map_err(|_| Error::Scope)?;
        let messages = [
            ChatMessage {
                role: ChatRole::System,
                content: CODING_PROPOSAL_INSTRUCTION,
            },
            ChatMessage {
                role: ChatRole::User,
                content: context.user_message(),
            },
        ];
        let packet = RuntimeEndpointContextPacket::from_messages(&[
            RuntimeEndpointMessage {
                role: RuntimeEndpointMessageRole::System,
                content: messages[0].content,
            },
            RuntimeEndpointMessage {
                role: RuntimeEndpointMessageRole::User,
                content: messages[1].content,
            },
        ])
        .map_err(Error::Preparation)?;
        let request = self.request();
        let prepared = match connection {
            CodingEndpointConnection::Borrowed {
                transport,
                approval,
            } => transport.prepare_chat(
                approval,
                &request.model_selection.model,
                &messages,
                request.max_output_tokens,
                include_usage,
            ),
            CodingEndpointConnection::Shared {
                transport,
                approval,
            } => transport.prepare_chat_shared(
                &approval,
                &request.model_selection.model,
                &messages,
                request.max_output_tokens,
                include_usage,
            ),
        }
        .and_then(|prepared| {
            prepared.with_response_text_limit(
                request
                    .limits
                    .max_output_bytes
                    .min(MAX_EDIT_ANSWER_BYTES as u64) as usize,
            )
        })
        .map_err(|_| Error::Preparation(RuntimePortFailure::Invalid))?;
        let phase = RuntimeEndpointModelPhase::for_run(request, prepared.binding(), packet)
            .map_err(Error::Preparation)?;
        Ok(CodingEndpointEditGeneration {
            startup_sha256: request.request_sha256.clone(),
            context,
            prepared,
            phase,
        })
    }
}

struct Dispatch<'config> {
    prepared: PreparedEndpointChat<'config>,
    cancellation: ChatCancellation,
}
impl RuntimeEndpointModelPort for Dispatch<'_> {
    type Error = TransportError;
    fn request(&self) -> &EndpointModelRequest {
        self.prepared.binding()
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        self.prepared
            .execute_recorded_controlled(&self.cancellation, control, text)
    }
}

impl CodingEndpointEditGeneration<'_> {
    /// Exact frozen request facts for the caller's cumulative preflight.
    #[must_use]
    pub fn binding(&self) -> &EndpointModelRequest {
        self.prepared.binding()
    }

    /// Dispatch once under the original borrowed run control. Stream fragments
    /// are untrusted/partial; only normal EOF permits later proposal checking.
    /// Retain actual phase accounting for failed, incomplete and invalid edits.
    /// No retry, clock renewal, file read, preview, grant or write occurs here.
    pub fn execute(
        self,
        control: &dyn ModelOperationControl,
        mut text: impl FnMut(&str),
    ) -> CodingEndpointGeneratedEdit {
        let Self {
            startup_sha256,
            context,
            prepared,
            mut phase,
        } = self;
        let cancellation = ChatCancellation::default();
        let answer_limit = prepared.binding().limits().response_text_limit_bytes;
        let mut answer = String::new();
        let mut oversized = false;
        let result = phase.execute(
            control,
            Dispatch {
                prepared,
                cancellation: cancellation.clone(),
            },
            |fragment| {
                if fragment.len() > answer_limit.saturating_sub(answer.len()) {
                    oversized = true;
                    cancellation.cancel();
                } else if !oversized {
                    answer.push_str(fragment);
                    text(fragment);
                }
            },
        );
        CodingEndpointGeneratedEdit {
            startup_sha256,
            context,
            answer,
            oversized,
            result,
            usage: phase.usage(),
        }
    }
}

/// Original generation observations, including rejected/partial attempts.
/// Private source/answer buffers are process-held; no durable recovery claim.
pub struct CodingEndpointGeneratedEdit {
    startup_sha256: String,
    context: CodingProposalContext,
    answer: String,
    oversized: bool,
    result: Result<EndpointModelResult, RuntimeEndpointPhaseFailure<TransportError>>,
    usage: RuntimeEndpointPhaseUsage,
}
impl CodingEndpointGeneratedEdit {
    /// Actual completed descriptor or original phase/transport refusal.
    pub const fn result(
        &self,
    ) -> &Result<EndpointModelResult, RuntimeEndpointPhaseFailure<TransportError>> {
        &self.result
    }

    /// Actual phase usage, preserved even when native preparation later refuses.
    /// Absent completed counts remain absent; token counts are provider reports.
    #[must_use]
    pub const fn usage(&self) -> &RuntimeEndpointPhaseUsage {
        &self.usage
    }

    /// After separate consent to re-read every original context file, prepare
    /// the original native complete diff. The same sealed startup is required.
    /// Failure/incomplete output refuses BEFORE any read. Fresh control is the
    /// original run's control, never a replacement timeout; time is caller-owned.
    /// The original exact approval and native writer remain separate.
    pub fn prepare_preview(
        &self,
        startup: &CodingEndpointPreparedStartup,
        repository: &mut LinuxCodingRepository,
        control: &dyn ModelOperationControl,
        now_epoch_ms: u64,
    ) -> Result<serde_json::Value, CodingEndpointGenerationError> {
        use CodingEndpointGenerationError as Error;
        if self.startup_sha256 != startup.request().request_sha256 {
            return Err(Error::Scope);
        }
        if self.oversized
            || !matches!(&self.result, Ok(result) if result.finish == EndpointModelFinish::Stop)
        {
            return Err(Error::Incomplete);
        }
        control
            .remaining_ms()
            .map_err(control_failure)
            .map_err(Error::Preparation)?;
        startup
            .prepare_edit_proposal(repository, &self.context, &self.answer, now_epoch_ms)
            .map_err(Error::Proposal)
    }
}
