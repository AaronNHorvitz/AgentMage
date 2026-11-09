// SPDX-License-Identifier: Apache-2.0
//! One strict edit generation through the original controlled endpoint phase.
//!
//! Trusted composition, with either standalone phase execution or dispatch by
//! the existing coordinator. The caller owns prior source/disclosure consent.

use agentmage_capability_repository_map::coding_proposal::{
    CODING_PROPOSAL_INSTRUCTION, CodingProposalContext,
};
use agentmage_kernel_contracts::{
    EndpointModelFinish, EndpointModelRequest, EndpointModelResult, ModelOperationControl,
    SchemaReference,
};
use agentmage_kernel_engine::runtime_loop::{
    RuntimeClock, RuntimeEndpointPreparationInput, RuntimeEndpointTurnPort, RuntimePortFailure,
    RuntimePreparedEndpointTurn,
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
    coding_endpoint::{CodingEndpointConnection, control_failure, transport_failure},
    coding_endpoint_prepared_startup::{
        CodingEndpointEditProposalError, CodingEndpointPreparedStartup,
    },
};

const MAX_EDIT_ANSWER_BYTES: usize = 64 * 1024;

/// Fixed trusted source selection for strict edits inside one live coordinator.
/// Each turn re-reads current bytes from the original owner under prior consent;
/// no path or executor is selected by a model or HTTP argument. Native effects
/// still require their separate exact approvals. No test/retry scheduler or
/// model-certified completion is supplied by this port.
pub struct CodingEndpointStrictEdits<'config, C> {
    previews: crate::coding_endpoint_edit_previews::CodingEndpointEditPreviews<C>,
    connection: CodingEndpointConnection<'config>,
    paths: Vec<String>,
    source_budget: usize,
    include_usage: bool,
}

impl<'config, C: RuntimeClock + Clone> CodingEndpointStrictEdits<'config, C> {
    /// Inert construction. Consent must cover these exact sources on every
    /// advance. Transport binding and the source allocation are checked here;
    /// no source is read and no model request is dispatched during admission.
    pub fn new(
        previews: crate::coding_endpoint_edit_previews::CodingEndpointEditPreviews<C>,
        connection: CodingEndpointConnection<'config>,
        paths: Vec<String>,
        source_budget: usize,
        include_usage: bool,
    ) -> Result<Self, RuntimePortFailure> {
        let request = previews.startup().request();
        if paths.is_empty()
            || !(2048..=32768).contains(&source_budget)
            || request.model_selection
                != connection
                    .transport()
                    .model_selection(&request.model_selection.model)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        previews
            .startup()
            .check_sources(
                &paths
                    .iter()
                    .map(|path| {
                        agentmage_capability_repository_map::coding_proposal::CodingSource {
                            path: path.clone(),
                            content: String::new(),
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(Self {
            previews,
            connection,
            paths,
            source_budget,
            include_usage,
        })
    }
}

impl<C: RuntimeClock + Clone> RuntimeEndpointTurnPort for CodingEndpointStrictEdits<'_, C> {
    fn model_binding(&self) -> agentmage_kernel_contracts::RuntimeModelBinding<'_> {
        let selection = &self.previews.startup().request().model_selection;
        agentmage_kernel_contracts::RuntimeModelBinding::SelectedEndpoint {
            endpoint: &selection.endpoint,
            model: &selection.model,
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
        // Reject substitution before reading or disclosing a single source byte.
        if input.request != self.previews.startup().request()
            || input.turn == 0
            || input.turn > input.request.limits.max_turns
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let context = self
            .previews
            .read_context(&self.paths, self.source_budget, control)?;
        let connection = match &self.connection {
            CodingEndpointConnection::Borrowed {
                transport,
                approval,
            } => CodingEndpointConnection::Borrowed {
                transport,
                approval,
            },
            CodingEndpointConnection::Shared {
                transport,
                approval,
            } => CodingEndpointConnection::Shared {
                transport: transport.clone(),
                approval: approval.clone(),
            },
        };
        self.previews
            .startup()
            .prepare_edit_generation(connection, context, self.include_usage)
            .and_then(|generation| {
                generation.into_coordinator_edit_turn(
                    input,
                    control,
                    crate::coding_verifier::coding_completion_schema(),
                    &mut self.previews,
                )
            })
            .map_err(|error| match error {
                CodingEndpointGenerationError::Preparation(error) => error,
                _ => RuntimePortFailure::Invalid,
            })
    }
}

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
    /// For coordinator-owned execution, consume this request with
    /// `into_coordinator_turn` from the original turn port. For standalone
    /// `execute`, the caller MUST reserve and account cumulatively: inspect
    /// `binding()` before dispatch and retain `usage()` on refusal. Preparation
    /// alone neither admits a coordinator nor renews a budget.
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

// Only adapts the transport's closed error vocabulary. The coordinator frames
// and executes the original request itself, with its own control and ledger.
struct CoordinatorDispatch<'config>(Dispatch<'config>);
impl RuntimeEndpointModelPort for CoordinatorDispatch<'_> {
    type Error = RuntimePortFailure;

    fn request(&self) -> &EndpointModelRequest {
        self.0.request()
    }

    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        self.0.dispatch(control, text).map_err(transport_failure)
    }
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

    /// Hand the ORIGINAL frozen request to an existing coordinator's turn port.
    /// Call from `RuntimeEndpointTurnPort::prepare` with its original input and
    /// borrowed control. The exact sealed startup must match. This consumes the
    /// unexecuted standalone phase; it does not dispatch or import phase usage.
    /// The coordinator alone frames, reserves and charges this request against
    /// its cumulative model/input/output/time ledger and continuous control.
    ///
    /// The returned text remains an untrusted proposal. This does not interpret
    /// strict edits, prepare a native preview, grant authority or certify a
    /// completion. The trusted caller selects the completion schema and original
    /// independent verifier. General edit/test/retry scheduling remains separate.
    pub fn into_coordinator_turn<'a>(
        self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
        completion_schema: SchemaReference,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, CodingEndpointGenerationError>
    where
        Self: 'a,
    {
        self.check_coordinator_input(&input, control)?;
        Ok(self.coordinator_turn(completion_schema))
    }

    /// Interpret the complete strict answer into the next original native
    /// preview INSIDE the same coordinator. Prior consent must cover re-reading
    /// all context files. Only this process-local composition selects the
    /// interpreter. Separate exact approval still controls the original writer.
    pub fn into_coordinator_edit_turn<'a, C>(
        self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
        completion_schema: SchemaReference,
        previews: &mut crate::coding_endpoint_edit_previews::CodingEndpointEditPreviews<C>,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, CodingEndpointGenerationError>
    where
        Self: 'a,
        C: agentmage_kernel_engine::runtime_loop::RuntimeClock + Clone + 'a,
    {
        self.check_coordinator_input(&input, control)?;
        let prepare = previews
            .response(&self.startup_sha256, self.context)
            .map_err(CodingEndpointGenerationError::Preparation)?;
        Ok(RuntimePreparedEndpointTurn::new(
            self.phase.context().clone(),
            CoordinatorDispatch(Dispatch {
                prepared: self.prepared,
                cancellation: ChatCancellation::default(),
            }),
            completion_schema,
        )
        .with_tool_preparation(prepare))
    }

    fn check_coordinator_input(
        &self,
        input: &RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<(), CodingEndpointGenerationError> {
        use CodingEndpointGenerationError as Error;
        agentmage_kernel_engine::runtime_coordinator::verify_runtime_endpoint_run_request(
            input.request,
        )
        .map_err(|_| Error::Scope)?;
        if self.startup_sha256 != input.request.request_sha256
            || input.turn == 0
            || input.turn > input.request.limits.max_turns
        {
            return Err(Error::Scope);
        }
        control
            .remaining_ms()
            .map_err(control_failure)
            .map_err(Error::Preparation)?;
        Ok(())
    }

    fn coordinator_turn<'a>(
        self,
        completion_schema: SchemaReference,
    ) -> RuntimePreparedEndpointTurn<'a>
    where
        Self: 'a,
    {
        RuntimePreparedEndpointTurn::new(
            self.phase.context().clone(),
            CoordinatorDispatch(Dispatch {
                prepared: self.prepared,
                cancellation: ChatCancellation::default(),
            }),
            completion_schema,
        )
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
