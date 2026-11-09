// SPDX-License-Identifier: Apache-2.0
//! Trusted, instruction-narrowed composition of original prepared native tools.

use std::sync::{Arc, Mutex};

use agentmage_capability_repository_map::coding_proposal::CodingSource;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    instruction_provenance::{InstructionEvidenceLedger, effective_guidance},
    runtime_coordinator::{seal_runtime_endpoint_run_request, verify_runtime_endpoint_run_request},
    runtime_loop::{
        RuntimeClock, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope,
        RuntimeToolBoundary, RuntimeToolExecution, RuntimeVerifierPort, runtime_tool_references,
    },
    tooling::ToolRegistry,
};
use agentmage_platform_linux::{
    LinuxCodingEndpointCommand, LinuxCodingEndpointCommandBoundary, LinuxCodingEndpointEdit,
    LinuxCodingEndpointEditBoundary, LinuxCodingRepository, linux_coding_endpoint_command_registry,
    linux_coding_endpoint_edit_registry,
};

use crate::{
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_owner::{CodingEndpointAdmission, CodingEndpointRuns},
    coding_endpoint_startup::CodingEndpointStartupError,
    coding_guidance::{CodingGuidancePolicy, compile_coding_read_guidance},
};

type Error = CodingEndpointStartupError;

/// Trusted selection of one original prepared effect, never a model-selected executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingEndpointPreparedKind {
    /// Complete prepared diff, applied through the original atomic writer.
    Edit,
    /// Original prepared command or validation, with unchanged native admission.
    Command,
}
impl CodingEndpointPreparedKind {
    fn registry(self) -> Result<ToolRegistry, Error> {
        match self {
            Self::Edit => linux_coding_endpoint_edit_registry(),
            Self::Command => linux_coding_endpoint_command_registry(),
        }
        .map_err(|_| Error::Binding)
    }
}

/// Inert preparation of a single-effect controlled request. Binding the live
/// creation owner is still required. A path, marker or receipt cannot supply it.
/// This factory does not build a second policy: original native owners retain
/// their own kernel policies and alone issue and consume their exact grants.
pub struct CodingEndpointPreparedStartup {
    request: RuntimeEndpointRunRequest,
    registry: ToolRegistry,
    guidance: CodingGuidancePolicy,
    kind: CodingEndpointPreparedKind,
}

impl CodingEndpointPreparedStartup {
    /// Compile the original trusted narrowing before binding any native tool.
    /// The caller supplies current instruction provenance under prior inspection
    /// consent. No file is opened, source read, grant issued or model dispatched.
    pub fn prepare(
        mut request: RuntimeEndpointRunRequest,
        kind: CodingEndpointPreparedKind,
        ledger: &InstructionEvidenceLedger,
    ) -> Result<Self, Error> {
        verify_runtime_endpoint_run_request(&request).map_err(|_| Error::Binding)?;
        let mut registry = kind.registry()?;
        if request.mode != RuntimeSessionMode::ControlledWrite
            || request.event_cursor.is_some()
            || request.visible_tools
                != runtime_tool_references(&registry).map_err(|_| Error::Binding)?
        {
            return Err(Error::Binding);
        }
        if ledger.workspace_manifest_sha256 != request.workspace_snapshot_sha256 {
            return Err(Error::Instructions);
        }
        let effective = effective_guidance(ledger).map_err(|_| Error::Instructions)?;
        // No independently bound completion-validation registry exists here.
        // RequireVerification therefore refuses even for a prepared test command;
        // a command result or model assertion cannot satisfy that requirement.
        let guidance = compile_coding_read_guidance(
            &request.workspace_id,
            ledger,
            &effective,
            &registry,
            &request.limits,
        )
        .map_err(Error::Guidance)?;
        guidance.restrict_registry(&mut registry);
        if registry.list_tools().is_empty() || guidance.limits().max_tool_calls == 0 {
            return Err(Error::NoTools);
        }
        let excluded = |path: &str| -> Result<bool, Error> {
            let path = WorkspacePath::new(request.workspace_id.clone(), path.split('/'))
                .map_err(|_| Error::Binding)?;
            Ok(guidance
                .excluded_scopes()
                .iter()
                .any(|s| s.contains_path(&path)))
        };
        // Validate every original path, including ones that would be excluded.
        for path in request
            .work_packet
            .mutable_files
            .iter()
            .chain(&request.work_packet.protected_files)
        {
            excluded(path)?;
        }
        if kind == CodingEndpointPreparedKind::Command && !guidance.excluded_scopes().is_empty() {
            // The original offline command can read its whole held worktree.
            // It cannot honor per-directory read exclusions: refuse admission.
            return Err(Error::Source);
        }
        request.work_packet.mutable_files = request
            .work_packet
            .mutable_files
            .iter()
            .filter_map(|p| match excluded(p) {
                Ok(false) => Some(Ok(p.clone())),
                Ok(true) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<_, _>>()?;
        if kind == CodingEndpointPreparedKind::Edit && request.work_packet.mutable_files.is_empty()
        {
            return Err(Error::NoTools);
        }
        request.limits = guidance.limits().clone();
        request.task.constraints.push(format!(
            "Trusted repository narrowing policy SHA-256: {}",
            guidance.policy_sha256()
        ));
        request = seal_runtime_endpoint_run_request(request).map_err(|_| Error::Binding)?;
        Ok(Self {
            request,
            registry,
            guidance,
            kind,
        })
    }

    /// Final sealed request, including exclusions and reduced run ceilings.
    #[must_use]
    pub const fn request(&self) -> &RuntimeEndpointRunRequest {
        &self.request
    }

    /// Original verified narrowing policy; this metadata is never effect authority.
    #[must_use]
    pub const fn guidance(&self) -> &CodingGuidancePolicy {
        &self.guidance
    }

    /// Reject malformed or excluded source snapshots before context composition.
    /// The caller still owns prior read/disclosure consent and snapshot freshness.
    pub fn check_sources(&self, sources: &[CodingSource]) -> Result<(), Error> {
        for source in sources {
            let path =
                WorkspacePath::new(self.request.workspace_id.clone(), source.path.split('/'))
                    .map_err(|_| Error::Source)?;
            if self
                .guidance
                .excluded_scopes()
                .iter()
                .any(|s| s.contains_path(&path))
            {
                return Err(Error::Source);
            }
        }
        Ok(())
    }

    /// Consume the original live creation owner and its exact prepared call.
    /// Original adapters verify workspace/creation and the complete preview.
    /// Edits check mutable/protected paths; commands check resource ceilings
    /// against this final request. Refusal returns the same repository intact.
    /// No effect occurs.
    #[allow(clippy::result_large_err)]
    pub fn bind(
        self,
        repository: LinuxCodingRepository,
        turn_id: RuntimeTurnId,
        operation_id: RuntimeOperationId,
        call: ToolCall,
    ) -> Result<CodingEndpointPrepared, (LinuxCodingRepository, Error)> {
        let tool = match self.kind {
            CodingEndpointPreparedKind::Edit => repository
                .into_endpoint_edit(self.request.clone(), turn_id, operation_id, call)
                .map(NativePrepared::Edit),
            CodingEndpointPreparedKind::Command => repository
                .into_endpoint_command(self.request.clone(), turn_id, operation_id, call)
                .map(NativePrepared::Command),
        }
        .map_err(|(owner, _)| (owner, Error::Native))?;
        Ok(CodingEndpointPrepared {
            startup: self,
            tool,
        })
    }
}

enum NativePrepared {
    Edit(LinuxCodingEndpointEdit),
    Command(LinuxCodingEndpointCommand),
}

/// Original native owner bound to its final instruction-narrowed endpoint run.
/// Neither receipts nor serialized API arguments can construct this object.
pub struct CodingEndpointPrepared {
    startup: CodingEndpointPreparedStartup,
    tool: NativePrepared,
}

/// Trusted ports and already consented snapshots for the retained coordinator.
/// Nothing here may be reconstructed from a model proposal or an HTTP body.
pub struct CodingEndpointPreparedInput<'config, 'source, V, C> {
    /// Original credential-owning endpoint connection and destination approval.
    pub connection: CodingEndpointConnection<'config>,
    /// Original continuous control, never renewed on an approval or tool turn.
    pub control: Arc<dyn ModelOperationControl + Send + Sync>,
    /// One epoch clock, shared by coordinator and native boundary.
    pub clock: C,
    /// Independent acceptance checks chosen by the trusted host, never the model.
    pub verifier: V,
    /// Current source snapshots under prior separate read and disclosure consent.
    pub sources: &'source [CodingSource],
    /// Deterministic excerpt byte budget, never a model-token estimate.
    pub source_budget: usize,
    /// Request actual endpoint usage observations, without fabricating missing usage.
    pub include_usage: bool,
}

/// Two private references to one original epoch clock. Poisoning fails closed;
/// a tool decision cannot replace the clock or start a fresh running-time budget.
pub struct CodingEndpointPreparedClock<C>(Arc<Mutex<C>>);
impl<C: RuntimeClock> RuntimeClock for CodingEndpointPreparedClock<C> {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        self.0
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?
            .now_epoch_ms()
    }
}

impl CodingEndpointPrepared {
    /// Exact final request. No native model profile or execution grant is inferred.
    #[must_use]
    pub const fn request(&self) -> &RuntimeEndpointRunRequest {
        self.startup.request()
    }

    /// Original protected challenge for inspection; its metadata alone cannot act.
    #[must_use]
    pub fn challenge(&self) -> &RuntimeApprovalChallenge {
        match &self.tool {
            NativePrepared::Edit(t) => t.challenge(),
            NativePrepared::Command(t) => t.challenge(),
        }
    }

    /// Abandon the prepared effect and return the same live repository owner.
    /// No pending preview or issued grant is restored and no effect is replayed.
    #[must_use]
    pub fn into_repository(self) -> LinuxCodingRepository {
        match self.tool {
            NativePrepared::Edit(t) => t.into_repository(),
            NativePrepared::Command(t) => t.into_repository(),
        }
    }

    /// Transfer the exact owner, scoped tool and original clock to the existing
    /// endpoint run registry. Admission performs no model/tool dispatch. Each
    /// later advance needs disclosure consent; effects require original exact
    /// decisions. Failure drops the owner, never reconstructing it from disk.
    /// No durable recovery or executable service activation is established here.
    pub fn admit<'config, V: RuntimeVerifierPort, C: RuntimeClock>(
        self,
        runs: &mut CodingEndpointRuns<
            'config,
            CodingEndpointPreparedBoundary<C>,
            V,
            CodingEndpointPreparedClock<C>,
        >,
        input: CodingEndpointPreparedInput<'config, '_, V, C>,
    ) -> Result<RuntimeEndpointRunRequest, Error> {
        self.startup.check_sources(input.sources)?;
        let shared = Arc::new(Mutex::new(input.clock));
        let clock = CodingEndpointPreparedClock(shared.clone());
        let tool_boundary = match self.tool {
            NativePrepared::Edit(t) => {
                CodingEndpointPreparedBoundary::Edit(t.into_boundary(input.control, clock))
            }
            NativePrepared::Command(t) => {
                CodingEndpointPreparedBoundary::Command(t.into_boundary(input.control, clock))
            }
        };
        let request = self.startup.request;
        runs.admit(CodingEndpointAdmission {
            request: request.clone(),
            connection: input.connection,
            registry: self.startup.registry,
            sources: input.sources,
            source_budget: input.source_budget,
            include_usage: input.include_usage,
            tool_boundary,
            verifier: input.verifier,
            clock: CodingEndpointPreparedClock(shared),
        })
        .map_err(Error::Owner)?;
        Ok(request)
    }
}

/// Original edit OR command boundary, with no substitute policy or executor.
/// Variant selection is fixed by trusted startup, not by a model's tool request.
/// Native-family entry points refuse; scoped methods delegate without alteration.
pub enum CodingEndpointPreparedBoundary<C> {
    /// Original native atomic edit and its privately retained exact grant.
    Edit(LinuxCodingEndpointEditBoundary<CodingEndpointPreparedClock<C>>),
    /// Original native command/test and its privately retained exact grant.
    Command(LinuxCodingEndpointCommandBoundary<CodingEndpointPreparedClock<C>>),
}
impl<C: RuntimeClock> CodingEndpointPreparedBoundary<C> {
    /// Abandon any pending single-effect decision and return the same original
    /// live repository, including successful edit lineage and validation state.
    /// After admission obtain this boundary only through terminal owner release;
    /// old approvals and run budgets cannot be resumed or reconstructed.
    #[must_use]
    pub fn into_repository(self) -> LinuxCodingRepository {
        match self {
            Self::Edit(tool) => tool.into_repository(),
            Self::Command(tool) => tool.into_repository(),
        }
    }
}
impl<C: RuntimeClock> RuntimeToolBoundary for CodingEndpointPreparedBoundary<C> {
    fn evaluate_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        op: &RuntimeOperationId,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        match self {
            Self::Edit(t) => t.evaluate_scoped(s, op, d, c, now),
            Self::Command(t) => t.evaluate_scoped(s, op, d, c, now),
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
        match self {
            Self::Edit(t) => t.resolve_scoped(s, ch, r, d, c, now),
            Self::Command(t) => t.resolve_scoped(s, ch, r, d, c, now),
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
        match self {
            Self::Edit(t) => t.execute_scoped(s, e, d, c, p),
            Self::Command(t) => t.execute_scoped(s, e, d, c, p),
        }
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

#[cfg(test)]
mod clock_tests {
    use super::*;
    struct Clock(u64);
    impl RuntimeClock for Clock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            self.0 += 1;
            Ok(self.0)
        }
    }
    #[test]
    fn prepared_endpoint_startup_clock_is_the_same_owner_across_both_consumers() {
        let original = Arc::new(Mutex::new(Clock(1000)));
        let mut coordinator = CodingEndpointPreparedClock(original.clone());
        let mut tool = CodingEndpointPreparedClock(original.clone());
        assert_eq!(coordinator.now_epoch_ms(), Ok(1001));
        assert_eq!(tool.now_epoch_ms(), Ok(1002));
        assert_eq!(coordinator.now_epoch_ms(), Ok(1003));
        assert_eq!(original.lock().unwrap().0, 1003);
    }
    #[test]
    fn prepared_endpoint_startup_clock_poison_refuses_without_replacing_original_time() {
        let original = Arc::new(Mutex::new(Clock(1000)));
        let owned = original.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = owned.lock().unwrap();
                panic!("synthetic original epoch clock unwind");
            })
            .join()
            .is_err()
        );
        let mut coordinator = CodingEndpointPreparedClock(original.clone());
        let mut tool = CodingEndpointPreparedClock(original);
        assert_eq!(
            coordinator.now_epoch_ms(),
            Err(RuntimePortFailure::Unavailable)
        );
        assert_eq!(tool.now_epoch_ms(), Err(RuntimePortFailure::Unavailable));
    }
}
