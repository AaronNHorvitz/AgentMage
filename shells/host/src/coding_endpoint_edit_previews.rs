// SPDX-License-Identifier: Apache-2.0
//! Strict responses and exact approvals over ONE original live native owner.

use std::sync::{Arc, Mutex};

use agentmage_capability_repository_map::coding_proposal::CodingProposalContext;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::runtime_loop::{
    RuntimeClock, RuntimeEndpointToolPreparationInput, RuntimeEndpointToolProposal,
    RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope, RuntimeToolBoundary,
    RuntimeToolExecution, runtime_tool_references,
};
use agentmage_platform_linux::{
    LinuxCodingEndpointEditBoundary, LinuxCodingRepository, linux_coding_endpoint_edit_registry,
};
use sha2::{Digest, Sha256};

use crate::{
    coding_endpoint::control_failure,
    coding_endpoint_prepared_startup::CodingEndpointPreparedStartup,
};

enum Owner<C> {
    Ready(Box<LinuxCodingRepository>),
    Pending(Box<LinuxCodingEndpointEditBoundary<C>>),
}
struct State<C> {
    owner: Option<Owner<C>>,
    control: Arc<dyn ModelOperationControl + Send + Sync>,
    clock: C,
}

/// Trusted turn-side reference to the same original repository as the tool
/// boundary. No cloning, serialized restoration or path adoption is available.
/// Construction and each prepared response require prior source-read consent.
pub struct CodingEndpointEditPreviews<C> {
    startup: Arc<CodingEndpointPreparedStartup>,
    state: Arc<Mutex<State<C>>>,
}

/// Tool-side reference. Only the existing original native boundary can grant
/// or execute an edit. A pending/failed/uncertain effect cannot be replaced.
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
        let registry =
            linux_coding_endpoint_edit_registry().map_err(|_| RuntimePortFailure::Invalid)?;
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
                let bytes =
                    serde_json::to_vec(&arguments).map_err(|_| RuntimePortFailure::Invalid)?;
                let call = ToolCall {
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
                };
                // Retire before handoff: unwind cannot restore this effect. Ordinary
                // refusal returns the same original repository, without a grant.
                let Some(Owner::Ready(repository)) = state.owner.take() else {
                    unreachable!()
                };
                match (*repository).into_endpoint_edit(
                    input.request.clone(),
                    input.turn_id.clone(),
                    input.operation_id.clone(),
                    call,
                ) {
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
            None => return Err(RuntimePortFailure::Invalid),
        }
        .map_err(|_| RuntimePortFailure::Unavailable)
    }
}

impl<C: RuntimeClock> RuntimeToolBoundary for CodingEndpointEditBoundary<C> {
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
        let Some(Owner::Pending(boundary)) = state.owner.as_mut() else {
            return Err(RuntimePortFailure::Invalid);
        };
        boundary.evaluate_scoped(s, op, d, c, now)
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
        let Some(Owner::Pending(boundary)) = state.owner.as_mut() else {
            return Err(RuntimePortFailure::Invalid);
        };
        boundary.resolve_scoped(s, ch, r, d, c, now)
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
        let Some(Owner::Pending(boundary)) = state.owner.as_mut() else {
            return Err(RuntimePortFailure::Invalid);
        };
        let result = boundary.execute_scoped(s, e, d, c, p)?;
        // Only an observed successful effect permits another preview in this
        // SAME coordinator. Failed/uncertain/spent boundaries remain held.
        if result.result.outcome == OperationOutcome::Succeeded {
            let Some(Owner::Pending(boundary)) = state.owner.take() else {
                unreachable!()
            };
            state.owner = Some(Owner::Ready(Box::new((*boundary).into_repository())));
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
