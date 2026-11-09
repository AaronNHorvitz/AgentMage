// SPDX-License-Identifier: Apache-2.0
//! Strict responses and exact approvals over ONE original live native owner.

use std::sync::{Arc, Mutex};

use agentmage_capability_repository_map::coding_proposal::{
    CodingProposalContext, CodingSource, prepare_coding_proposal,
};
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::runtime_loop::{
    RuntimeClock, RuntimeEndpointPreparationInput, RuntimeEndpointToolPreparationInput,
    RuntimeEndpointToolProposal, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope,
    RuntimeToolBoundary, RuntimeToolExecution, runtime_tool_references,
};
use agentmage_platform_linux::{
    LinuxCodingEndpointCommandBoundary, LinuxCodingEndpointEditBoundary, LinuxCodingRepository,
    LinuxCodingTestFormat, linux_coding_endpoint_coding_registry,
    linux_coding_endpoint_command_registry, linux_coding_endpoint_edit_registry,
};
use sha2::{Digest, Sha256};

use crate::{
    coding_endpoint::control_failure,
    coding_endpoint_prepared_startup::CodingEndpointPreparedStartup,
};

enum Owner<C> {
    Ready(Box<LinuxCodingRepository>),
    Pending(Box<LinuxCodingEndpointEditBoundary<C>>),
    Command(Box<LinuxCodingEndpointCommandBoundary<C>>),
}

/// Fixed trusted validation definition. No model or HTTP field can select a
/// command here. Native manifest checks and each exact approval remain required.
pub struct CodingEndpointValidation {
    /// Exact user-selected offline command, including identity and bounds.
    pub command: agentmage_kernel_engine::command_runner::CommandSpec,
    /// Test category checked by the original native validation owner.
    pub kind: agentmage_kernel_engine::validation_template::ValidationKind,
    /// Minimum observed test count; zero is refused.
    pub minimum_test_count: u32,
    /// Explicit closed output parser, never inferred from process prose.
    pub output_format: LinuxCodingTestFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Next {
    Edit,
    Test,
    Completion,
}

pub(super) enum PreparedNext<'a> {
    Edit,
    Test,
    Completion(CompletionReady<'a>),
}

type CompletionCheck<'a> =
    Box<dyn FnOnce(&dyn ModelOperationControl) -> Result<(), RuntimePortFailure> + 'a>;

// Issued only by the original checked owner. Single use, process-local and
// bound to one sealed request/turn. This is sampled readiness, never acceptance.
pub(super) struct CompletionReady<'a> {
    request: Box<RuntimeEndpointRunRequest>,
    turn: u32,
    check: CompletionCheck<'a>,
}

impl<'a> CompletionReady<'a> {
    pub(super) fn into_check(
        self,
        input: &RuntimeEndpointPreparationInput<'_>,
    ) -> Result<CompletionCheck<'a>, RuntimePortFailure> {
        if self.request.as_ref() != input.request || self.turn != input.turn {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(self.check)
    }

    #[cfg(test)]
    pub(super) fn synthetic_for_framing(
        request: &RuntimeEndpointRunRequest,
        turn: u32,
        check: impl FnOnce(&dyn ModelOperationControl) -> Result<(), RuntimePortFailure> + 'a,
    ) -> Self {
        // Grammar/socket fixtures only: deliberately skips native readiness.
        // No production constructor, original validation or success is implied.
        Self {
            request: Box::new(request.clone()),
            turn,
            check: Box::new(check),
        }
    }
}
struct State<C> {
    owner: Option<Owner<C>>,
    control: Arc<dyn ModelOperationControl + Send + Sync>,
    clock: C,
    validation: Option<CodingEndpointValidation>,
    next: Next,
    feedback: Option<serde_json::Value>,
    prior_receipt: Option<String>,
}

/// Trusted turn-side reference to the same original repository as the tool
/// boundary. No cloning, serialized restoration or path adoption is available.
/// Construction and each prepared response require prior source-read consent.
pub struct CodingEndpointEditPreviews<C> {
    startup: Arc<CodingEndpointPreparedStartup>,
    state: Arc<Mutex<State<C>>>,
}

/// Tool-side reference. Original native boundaries alone grant and execute.
/// Pending and uncertain effects stay held. An eligible failed test returns its
/// owner only after the original failure classifier consumes its selection.
pub struct CodingEndpointEditBoundary<C> {
    state: Arc<Mutex<State<C>>>,
}

impl<C: RuntimeClock + Clone> CodingEndpointEditPreviews<C> {
    /// Retain the original created repository, narrowed startup, continuous
    /// control and shared epoch clock. The caller must give the coordinator
    /// that SAME clock owner; Clone must share time, never reset a deadline.
    /// Refusal drops process-held ownership; it never reconstructs a repository.
    pub fn new(
        startup: CodingEndpointPreparedStartup,
        repository: LinuxCodingRepository,
        control: Arc<dyn ModelOperationControl + Send + Sync>,
        clock: C,
    ) -> Result<(Self, CodingEndpointEditBoundary<C>), RuntimePortFailure> {
        Self::new_inner(startup, repository, control, clock, None)
    }

    /// One original owner alternates strict edits and this frozen validation.
    /// Must use the explicit combined startup; commands with read exclusions
    /// refuse there. Prior inspection consent must also cover bounded complete
    /// worktree reads before and after summary generation, including hidden files,
    /// configuration and tests. Those bytes are not added to model context.
    /// No command, grant or model request occurs during construction.
    pub fn new_coding(
        startup: CodingEndpointPreparedStartup,
        repository: LinuxCodingRepository,
        control: Arc<dyn ModelOperationControl + Send + Sync>,
        clock: C,
        validation: CodingEndpointValidation,
    ) -> Result<(Self, CodingEndpointEditBoundary<C>), RuntimePortFailure> {
        if validation.minimum_test_count == 0 || !validation.kind.is_test() {
            return Err(RuntimePortFailure::Invalid);
        }
        Self::new_inner(startup, repository, control, clock, Some(validation))
    }

    fn new_inner(
        startup: CodingEndpointPreparedStartup,
        repository: LinuxCodingRepository,
        control: Arc<dyn ModelOperationControl + Send + Sync>,
        clock: C,
        validation: Option<CodingEndpointValidation>,
    ) -> Result<(Self, CodingEndpointEditBoundary<C>), RuntimePortFailure> {
        let registry = if validation.is_some() {
            linux_coding_endpoint_coding_registry()
        } else {
            linux_coding_endpoint_edit_registry()
        }
        .map_err(|_| RuntimePortFailure::Invalid)?;
        if repository.created_workspace_id() != Some(&startup.request().workspace_id)
            || startup.request().visible_tools
                != runtime_tool_references(&registry).map_err(|_| RuntimePortFailure::Invalid)?
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let state = Arc::new(Mutex::new(State {
            owner: Some(Owner::Ready(Box::new(repository))),
            control,
            clock,
            validation,
            next: Next::Edit,
            feedback: None,
            prior_receipt: None,
        }));
        Ok((
            Self {
                startup: Arc::new(startup),
                state: state.clone(),
            },
            CodingEndpointEditBoundary { state },
        ))
    }

    /// Original frozen startup for preparing disclosure-approved strict turns.
    #[must_use]
    pub fn startup(&self) -> &CodingEndpointPreparedStartup {
        &self.startup
    }

    /// Capture fresh selected snapshots under PRIOR read and model-disclosure
    /// consent. Exclusions and workspace binding are checked before any read.
    /// Pending/failed/uncertain effects cannot supply a later model context.
    pub fn read_context(
        &self,
        paths: &[String],
        source_budget: usize,
        control: &dyn ModelOperationControl,
    ) -> Result<CodingProposalContext, RuntimePortFailure> {
        control.remaining_ms().map_err(control_failure)?;
        if paths.is_empty() || !(2048..=32768).contains(&source_budget) {
            return Err(RuntimePortFailure::Invalid);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        state.control.remaining_ms().map_err(control_failure)?;
        let Some(Owner::Ready(repository)) = state.owner.as_mut() else {
            return Err(RuntimePortFailure::Invalid);
        };
        if repository.created_workspace_id() != Some(&self.startup.request().workspace_id) {
            return Err(RuntimePortFailure::Invalid);
        }
        // Reuse the narrowed path validator before source selection or reads.
        self.startup
            .check_sources(
                &paths
                    .iter()
                    .map(|path| CodingSource {
                        path: path.clone(),
                        content: String::new(),
                    })
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let sources = repository
            .select_created_sources(paths)
            .map_err(|_| RuntimePortFailure::Unavailable)?
            .read(paths)
            .map_err(|_| RuntimePortFailure::Unavailable)?
            .into_iter()
            .map(|source| CodingSource {
                path: source.path,
                content: source.content,
            })
            .collect::<Vec<_>>();
        control.remaining_ms().map_err(control_failure)?;
        state.control.remaining_ms().map_err(control_failure)?;
        let context = prepare_coding_proposal(
            &self.startup.request().task.objective,
            &sources,
            source_budget,
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        self.startup
            .check_edit_context(&context)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(context)
    }

    #[cfg(test)]
    pub(super) fn synthetic_observation(&self, next: Next, feedback: Option<serde_json::Value>) {
        // Host prompt/scheduling fixtures only. Native admission and original
        // failure-to-owner transfer are tested at their private platform seam.
        let mut state = self.state.lock().unwrap();
        assert!(matches!(state.owner, Some(Owner::Ready(_))));
        state.next = next;
        state.feedback = feedback;
    }

    pub(super) fn next<'a>(
        &self,
        input: &RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<PreparedNext<'a>, RuntimePortFailure>
    where
        C: 'a,
    {
        if input.request != self.startup.request()
            || input.turn == 0
            || input.turn > input.request.limits.max_turns
        {
            return Err(RuntimePortFailure::Invalid);
        }
        control.remaining_ms().map_err(control_failure)?;
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        state.control.remaining_ms().map_err(control_failure)?;
        if !matches!(state.owner, Some(Owner::Ready(_))) {
            return Err(RuntimePortFailure::Invalid);
        }
        if state.next == Next::Completion {
            let Some(Owner::Ready(repository)) = state.owner.as_ref() else {
                return Err(RuntimePortFailure::Invalid);
            };
            repository
                .check_completion_validation()
                .map_err(|_| RuntimePortFailure::Invalid)?;
            control.remaining_ms().map_err(control_failure)?;
            state.control.remaining_ms().map_err(control_failure)?;
        }
        Ok(match state.next {
            Next::Edit => PreparedNext::Edit,
            Next::Test => PreparedNext::Test,
            Next::Completion => PreparedNext::Completion(CompletionReady {
                request: Box::new(input.request.clone()),
                turn: input.turn,
                check: Box::new(self.completion_check()),
            }),
        })
    }

    /// Retain only a reference to the original owner for a second inspection
    /// after model generation. This cannot restore ownership or create a pass.
    pub(super) fn completion_check(
        &self,
    ) -> impl FnOnce(&dyn ModelOperationControl) -> Result<(), RuntimePortFailure> + use<C> {
        let state = self.state.clone();
        move |control| {
            control.remaining_ms().map_err(control_failure)?;
            let state = state.lock().map_err(|_| RuntimePortFailure::Unavailable)?;
            state.control.remaining_ms().map_err(control_failure)?;
            if state.next != Next::Completion {
                return Err(RuntimePortFailure::Invalid);
            }
            let Some(Owner::Ready(repository)) = state.owner.as_ref() else {
                return Err(RuntimePortFailure::Invalid);
            };
            let checked = repository
                .check_completion_validation()
                .map_err(|_| RuntimePortFailure::Invalid);
            control.remaining_ms().map_err(control_failure)?;
            state.control.remaining_ms().map_err(control_failure)?;
            checked
        }
    }

    // This exact native selection was consumed when the failed observation was
    // admitted. Taking it for one fresh prompt never restores a repair selection.
    pub(super) fn take_feedback(&self) -> Result<Option<serde_json::Value>, RuntimePortFailure> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        state.control.remaining_ms().map_err(control_failure)?;
        if state.next != Next::Edit || !matches!(state.owner, Some(Owner::Ready(_))) {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(state.feedback.take())
    }

    pub(super) fn validation_response<'a>(
        &self,
    ) -> Result<
        impl FnOnce(
            RuntimeEndpointToolPreparationInput<'_>,
            &dyn ModelOperationControl,
        ) -> Result<RuntimeEndpointToolProposal, RuntimePortFailure>
        + 'a,
        RuntimePortFailure,
    >
    where
        C: 'a,
    {
        let startup = self.startup.clone();
        let state = self.state.clone();
        Ok(
            move |input: RuntimeEndpointToolPreparationInput<'_>,
                  control: &dyn ModelOperationControl| {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Validate {
                    validate: bool,
                }
                if input.request != startup.request()
                    || !serde_json::from_str::<Validate>(input.text)
                        .map_err(|_| RuntimePortFailure::Invalid)?
                        .validate
                {
                    return Err(RuntimePortFailure::Invalid);
                }
                control.remaining_ms().map_err(control_failure)?;
                let mut state = state.lock().map_err(|_| RuntimePortFailure::Unavailable)?;
                state.control.remaining_ms().map_err(control_failure)?;
                if state.next != Next::Test {
                    return Err(RuntimePortFailure::Invalid);
                }
                let now = state.clock.now_epoch_ms()?;
                let State {
                    owner,
                    validation,
                    prior_receipt,
                    ..
                } = &mut *state;
                let Some(Owner::Ready(repository)) = owner.as_mut() else {
                    return Err(RuntimePortFailure::Invalid);
                };
                let validation = validation.as_ref().ok_or(RuntimePortFailure::Invalid)?;
                let preview = repository
                    .prepare_validation(
                        validation.command.clone(),
                        validation.kind,
                        validation.minimum_test_count,
                        validation.output_format,
                        prior_receipt.as_deref(),
                        now,
                    )
                    .map_err(|_| RuntimePortFailure::Unavailable)?;
                control.remaining_ms().map_err(control_failure)?;
                state.control.remaining_ms().map_err(control_failure)?;
                let registry = linux_coding_endpoint_command_registry()
                    .map_err(|_| RuntimePortFailure::Invalid)?;
                let definition = registry.list_tools()[0];
                let arguments = serde_json::json!({"preview": preview});
                let call = prepared_call(&input, definition, &arguments)?;
                let Some(Owner::Ready(repository)) = state.owner.take() else {
                    unreachable!()
                };
                match (*repository).into_endpoint_coding_command(
                    input.request.clone(),
                    input.turn_id.clone(),
                    input.operation_id.clone(),
                    call,
                ) {
                    Ok(command) => {
                        state.owner = Some(Owner::Command(Box::new(
                            command
                                .into_boundary(state.control.clone(), state.clock.clone())
                                .with_failure_observations(),
                        )));
                        Ok(RuntimeEndpointToolProposal {
                            tool_id: definition.tool_id.clone(),
                            tool_version: definition.tool_version.clone(),
                            arguments,
                        })
                    }
                    Err((repository, _)) => {
                        state.owner = Some(Owner::Ready(Box::new(repository)));
                        Err(RuntimePortFailure::Invalid)
                    }
                }
            },
        )
    }

    pub(super) fn response<'a>(
        &self,
        startup_sha256: &str,
        context: CodingProposalContext,
    ) -> Result<
        impl FnOnce(
            RuntimeEndpointToolPreparationInput<'_>,
            &dyn ModelOperationControl,
        ) -> Result<RuntimeEndpointToolProposal, RuntimePortFailure>
        + 'a,
        RuntimePortFailure,
    >
    where
        C: 'a,
    {
        if startup_sha256 != self.startup.request().request_sha256 {
            return Err(RuntimePortFailure::Invalid);
        }
        self.startup
            .check_edit_context(&context)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let startup = self.startup.clone();
        let state = self.state.clone();
        Ok(
            move |input: RuntimeEndpointToolPreparationInput<'_>,
                  control: &dyn ModelOperationControl| {
                if input.request != startup.request() {
                    return Err(RuntimePortFailure::Invalid);
                }
                control.remaining_ms().map_err(control_failure)?;
                let mut state = state.lock().map_err(|_| RuntimePortFailure::Unavailable)?;
                state.control.remaining_ms().map_err(control_failure)?;
                if state.next != Next::Edit {
                    return Err(RuntimePortFailure::Invalid);
                }
                let now = state.clock.now_epoch_ms()?;
                let Some(Owner::Ready(repository)) = state.owner.as_mut() else {
                    return Err(RuntimePortFailure::Invalid);
                };
                let preview = startup
                    .prepare_edit_proposal(repository, &context, input.text, now)
                    .map_err(|_| RuntimePortFailure::Invalid)?;
                control.remaining_ms().map_err(control_failure)?;
                state.control.remaining_ms().map_err(control_failure)?;
                let registry = linux_coding_endpoint_edit_registry()
                    .map_err(|_| RuntimePortFailure::Invalid)?;
                let definition = registry.list_tools()[0];
                let arguments = serde_json::json!({"preview": preview});
                let call = prepared_call(&input, definition, &arguments)?;
                let combined = state.validation.is_some();
                // Retire before handoff: unwind cannot restore this effect. Ordinary
                // refusal returns the same original repository, without a grant.
                let Some(Owner::Ready(repository)) = state.owner.take() else {
                    unreachable!()
                };
                let prepared = if combined {
                    (*repository).into_endpoint_coding_edit(
                        input.request.clone(),
                        input.turn_id.clone(),
                        input.operation_id.clone(),
                        call,
                    )
                } else {
                    (*repository).into_endpoint_edit(
                        input.request.clone(),
                        input.turn_id.clone(),
                        input.operation_id.clone(),
                        call,
                    )
                };
                match prepared {
                    Ok(edit) => {
                        state.owner = Some(Owner::Pending(Box::new(
                            edit.into_boundary(state.control.clone(), state.clock.clone()),
                        )));
                        Ok(RuntimeEndpointToolProposal {
                            tool_id: definition.tool_id.clone(),
                            tool_version: definition.tool_version.clone(),
                            arguments,
                        })
                    }
                    Err((repository, _)) => {
                        state.owner = Some(Owner::Ready(Box::new(repository)));
                        Err(RuntimePortFailure::Invalid)
                    }
                }
            },
        )
    }
}

impl<C: RuntimeClock> CodingEndpointEditBoundary<C> {
    /// Inert inspection of the original preview and actual last native result.
    pub fn inspect(&self) -> Result<serde_json::Value, RuntimePortFailure> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        match state.owner.as_ref() {
            Some(Owner::Ready(repository)) => repository.inspect_edit(),
            Some(Owner::Pending(boundary)) => boundary.inspect(),
            Some(Owner::Command(boundary)) => boundary.inspect(),
            None => return Err(RuntimePortFailure::Invalid),
        }
        .map_err(|_| RuntimePortFailure::Unavailable)
    }
}

impl<C: RuntimeClock> RuntimeToolBoundary for CodingEndpointEditBoundary<C> {
    fn permits_failure_observation(
        &mut self,
        scope: RuntimeRunScope<'_>,
        definition: &ToolDefinition,
        call: &ToolCall,
        execution: &RuntimeToolExecution,
    ) -> Result<bool, RuntimePortFailure> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let Some(Owner::Command(boundary)) = state.owner.as_mut() else {
            return Ok(false);
        };
        if !boundary.permits_failure_observation(scope, definition, call, execution)? {
            return Ok(false);
        }
        let Some(Owner::Command(boundary)) = state.owner.take() else {
            unreachable!()
        };
        match (*boundary).into_repair_repository() {
            Ok((repository, feedback)) => {
                state.prior_receipt = feedback["receipt_sha256"].as_str().map(str::to_owned);
                state.feedback = Some(feedback);
                state.owner = Some(Owner::Ready(Box::new(repository)));
                state.next = Next::Edit;
                Ok(true)
            }
            Err(boundary) => {
                state.owner = Some(Owner::Command(Box::new(boundary)));
                Err(RuntimePortFailure::Invalid)
            }
        }
    }

    fn evaluate_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        op: &RuntimeOperationId,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        match state.owner.as_mut() {
            Some(Owner::Pending(boundary)) => boundary.evaluate_scoped(s, op, d, c, now),
            Some(Owner::Command(boundary)) => boundary.evaluate_scoped(s, op, d, c, now),
            _ => Err(RuntimePortFailure::Invalid),
        }
    }
    fn resolve_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        ch: &RuntimeApprovalChallenge,
        r: &RuntimeApprovalResponse,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        match state.owner.as_mut() {
            Some(Owner::Pending(boundary)) => boundary.resolve_scoped(s, ch, r, d, c, now),
            Some(Owner::Command(boundary)) => boundary.resolve_scoped(s, ch, r, d, c, now),
            _ => Err(RuntimePortFailure::Invalid),
        }
    }
    fn execute_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        e: &RuntimePermissionEvaluation,
        d: &ToolDefinition,
        c: &ToolCall,
        p: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let result = match state.owner.as_mut() {
            Some(Owner::Pending(boundary)) => boundary.execute_scoped(s, e, d, c, p)?,
            Some(Owner::Command(boundary)) => boundary.execute_scoped(s, e, d, c, p)?,
            _ => return Err(RuntimePortFailure::Invalid),
        };
        // Failed commands stay held until the original classifier admits their
        // exact observation. No failed/uncertain edit can re-enter preparation.
        if result.result.outcome == OperationOutcome::Succeeded {
            match state.owner.take() {
                Some(Owner::Pending(boundary)) => {
                    state.owner = Some(Owner::Ready(Box::new((*boundary).into_repository())));
                    state.next = if state.validation.is_some() {
                        Next::Test
                    } else {
                        Next::Edit
                    };
                }
                Some(Owner::Command(boundary)) => {
                    state.owner = Some(Owner::Ready(Box::new((*boundary).into_repository())));
                    state.next = Next::Completion;
                }
                _ => unreachable!(),
            }
        }
        Ok(result)
    }
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
}

fn prepared_call(
    input: &RuntimeEndpointToolPreparationInput<'_>,
    definition: &ToolDefinition,
    arguments: &serde_json::Value,
) -> Result<ToolCall, RuntimePortFailure> {
    let bytes = serde_json::to_vec(arguments).map_err(|_| RuntimePortFailure::Invalid)?;
    Ok(ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: input.tool_call_id.clone(),
        correlation_id: input.correlation_id.clone(),
        action_id: input.action_id.clone(),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".into(),
            sha256: Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            bytes,
        },
    })
}
