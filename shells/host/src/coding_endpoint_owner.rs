// SPDX-License-Identifier: Apache-2.0
//! Exclusive in-process ownership of ephemeral configured-endpoint coding runs.
//!
//! Keep one owner in the hosting service. Requests identify retained instances;
//! they cannot reconstruct their context, clock, budgets or authority. This is
//! synchronous scheduling, not durable recovery or an HTTP authentication layer.

use std::collections::BTreeMap;

use agentmage_capability_repository_map::coding_proposal::CodingSource;
use agentmage_kernel_contracts::{
    CorrelationId, ModelCancellationProbe, RuntimeApprovalResponse, RuntimeEndpointRunRequest,
    RuntimeEvent, RuntimeRunId,
};
use agentmage_kernel_engine::{
    runtime_coordinator::verify_runtime_endpoint_run_request,
    runtime_loop::{
        ReusableRuntimeCoordinator, RuntimeClock, RuntimeEndpointCoordinatorStep,
        RuntimeEndpointDriver, RuntimeLoopError, RuntimeNoNativeContext, RuntimePortFailure,
        RuntimeToolBoundary, RuntimeVerifierPort,
    },
    tooling::ToolRegistry,
};

use crate::{
    coding_endpoint::{CodingEndpointConnection, ConfiguredCodingEndpoint},
    coding_endpoint_context::CodingEndpointContext,
};

const MAX_HELD_RUNS: usize = 4;
const MAX_ADMITTED_IDENTITIES: usize = 256;

type Coordinator<'config, T, V, C> = ReusableRuntimeCoordinator<
    RuntimeEndpointDriver<ConfiguredCodingEndpoint<'config, CodingEndpointContext>>,
    RuntimeNoNativeContext,
    T,
    V,
    C,
>;

/// Trusted composition inputs. Prior source-read/disclosure consent and actual
/// scoped tool/grant ownership remain the caller's responsibility. None of
/// these ports or source snapshots may be selected by untrusted wire arguments.
pub struct CodingEndpointAdmission<'config, 'source, T, V, C> {
    /// Original sealed request. No cursor or native profile is accepted.
    pub request: RuntimeEndpointRunRequest,
    /// Original configuration, borrowed or retained independently of settings.
    /// Destination approval is rechecked at every dispatch.
    pub connection: CodingEndpointConnection<'config>,
    /// Exact model-visible catalog, handed to the original coordinator.
    pub registry: ToolRegistry,
    /// Already consented and freshly captured source snapshots.
    pub sources: &'source [CodingSource],
    /// Existing deterministic excerpt allocation, in bytes.
    pub source_budget: usize,
    /// Ask the configured endpoint for usage observations.
    pub include_usage: bool,
    /// Original permission and tool boundary, never reconstructed from metadata.
    pub tool_boundary: T,
    /// Original independent deterministic completion checks.
    pub verifier: V,
    /// Clock retained across every advance and permission wait.
    pub clock: C,
}

/// Content-free admission/scheduling failures. A refused lookup never advances
/// a run. A stateful coordinator error latches its entry closed before returning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingEndpointOwnerError {
    /// Invalid sealed request, context or catalog.
    Invalid,
    /// This run identity was already admitted, including released/faulted runs.
    AlreadyAdmitted,
    /// Bounded active slots or lifetime identity retention is full.
    Capacity,
    /// No run with this identity and exact sealed request is held.
    NotHeld,
    /// Release was requested while the coordinator can still advance.
    StillRunning,
    /// A prior coordinator error or unwind prevents further dispatch.
    Faulted,
    /// Original context construction failed without admission or dispatch.
    Context(RuntimePortFailure),
    /// Original coordinator error; retained events remain inspectable.
    Coordinator(RuntimeLoopError),
}

enum Held<'config, T: RuntimeToolBoundary, V: RuntimeVerifierPort, C: RuntimeClock> {
    Retained {
        coordinator: Box<Coordinator<'config, T, V, C>>,
        faulted: bool,
    },
    Released,
}

struct Entry<'config, T: RuntimeToolBoundary, V: RuntimeVerifierPort, C: RuntimeClock> {
    request_sha256: String,
    held: Held<'config, T, V, C>,
}

/// A non-cloneable owner of at most four complete coordinators. All operations
/// borrow this same owner; no method extracts a live coordinator or replaces
/// its ports. Released identities remain spent for this owner's lifetime.
/// After 256 admissions, create a new service with fresh run identities rather
/// than evicting tombstones. A new owner does not recover any prior run.
pub struct CodingEndpointRuns<
    'config,
    T: RuntimeToolBoundary,
    V: RuntimeVerifierPort,
    C: RuntimeClock,
> {
    entries: BTreeMap<RuntimeRunId, Entry<'config, T, V, C>>,
}

impl<T: RuntimeToolBoundary, V: RuntimeVerifierPort, C: RuntimeClock> Default
    for CodingEndpointRuns<'_, T, V, C>
{
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<'config, T: RuntimeToolBoundary, V: RuntimeVerifierPort, C: RuntimeClock>
    CodingEndpointRuns<'config, T, V, C>
{
    /// Admits once without executing a model or tool. Failed construction does
    /// not consume an identity, because no run has started and no effect occurred.
    pub fn admit(
        &mut self,
        input: CodingEndpointAdmission<'config, '_, T, V, C>,
    ) -> Result<(), CodingEndpointOwnerError> {
        verify_runtime_endpoint_run_request(&input.request)
            .map_err(|_| CodingEndpointOwnerError::Invalid)?;
        if self.entries.contains_key(&input.request.run_id) {
            return Err(CodingEndpointOwnerError::AlreadyAdmitted);
        }
        if self.entries.len() >= MAX_ADMITTED_IDENTITIES
            || self
                .entries
                .values()
                .filter(|entry| !matches!(entry.held, Held::Released))
                .count()
                >= MAX_HELD_RUNS
        {
            return Err(CodingEndpointOwnerError::Capacity);
        }
        let port = ConfiguredCodingEndpoint::for_coding_run_with_connection(
            input.connection,
            &input.request,
            &input.registry,
            input.sources,
            input.source_budget,
            input.include_usage,
        )
        .map_err(CodingEndpointOwnerError::Context)?;
        let id = input.request.run_id.clone();
        let request_sha256 = input.request.request_sha256.clone();
        let coordinator = ReusableRuntimeCoordinator::new_endpoint(
            input.request,
            port,
            input.registry,
            input.tool_boundary,
            input.verifier,
            input.clock,
        )
        .map_err(CodingEndpointOwnerError::Coordinator)?;
        self.entries.insert(
            id,
            Entry {
                request_sha256,
                held: Held::Retained {
                    coordinator: Box::new(coordinator),
                    faulted: false,
                },
            },
        );
        Ok(())
    }

    /// Advances only the retained instance. The coordinator owns exact approval
    /// validation, cumulative budgets, cancellation and idempotent terminal reads.
    /// Rejected approval input preserves the original challenge. On a stateful
    /// boundary error retain events, but never try again; the original kernel
    /// reconciliation state decides this, never an error-code guess. Mark the
    /// entry faulted before entering dependencies so a caught unwind cannot
    /// make partially advanced state reusable.
    pub fn advance(
        &mut self,
        request: &RuntimeEndpointRunRequest,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeEndpointCoordinatorStep, CodingEndpointOwnerError> {
        let entry = self.entry(request)?;
        let Held::Retained {
            coordinator,
            faulted,
        } = &mut entry.held
        else {
            return Err(CodingEndpointOwnerError::NotHeld);
        };
        if *faulted {
            return Err(CodingEndpointOwnerError::Faulted);
        }
        *faulted = true;
        let result = coordinator.run_endpoint_until_boundary(response, cancellation);
        *faulted = result.is_err() && coordinator.requires_reconciliation();
        result.map_err(CodingEndpointOwnerError::Coordinator)
    }

    /// Borrows the retained coordinator's original cancellation correlation,
    /// including before its first event. No caller recreates a derived id.
    pub fn correlation_id(
        &mut self,
        request: &RuntimeEndpointRunRequest,
    ) -> Result<&CorrelationId, CodingEndpointOwnerError> {
        match &self.entry(request)?.held {
            Held::Retained { coordinator, .. } => Ok(coordinator.correlation_id()),
            Held::Released => Err(CodingEndpointOwnerError::NotHeld),
        }
    }

    /// Borrows complete retained canonical events, including a faulted run's
    /// prefix. A prefix without a terminal never claims completion or recovery.
    pub fn events(
        &mut self,
        request: &RuntimeEndpointRunRequest,
    ) -> Result<&[RuntimeEvent], CodingEndpointOwnerError> {
        match &self.entry(request)?.held {
            Held::Retained { coordinator, .. } => Ok(coordinator.events()),
            Held::Released => Err(CodingEndpointOwnerError::NotHeld),
        }
    }

    /// Drops private context and ports only after terminal publication or a
    /// latched fault. Keeps the identity/digest so release cannot renew budgets.
    /// Export required events before release; this owner makes no storage claim.
    pub fn release(
        &mut self,
        request: &RuntimeEndpointRunRequest,
    ) -> Result<(), CodingEndpointOwnerError> {
        let entry = self.entry(request)?;
        match &entry.held {
            Held::Retained {
                coordinator,
                faulted: false,
            } if coordinator.endpoint_outcome().is_none() => {
                return Err(CodingEndpointOwnerError::StillRunning);
            }
            Held::Released => return Err(CodingEndpointOwnerError::NotHeld),
            _ => {}
        }
        entry.held = Held::Released;
        Ok(())
    }

    fn entry(
        &mut self,
        request: &RuntimeEndpointRunRequest,
    ) -> Result<&mut Entry<'config, T, V, C>, CodingEndpointOwnerError> {
        verify_runtime_endpoint_run_request(request)
            .map_err(|_| CodingEndpointOwnerError::Invalid)?;
        self.entries
            .get_mut(&request.run_id)
            .filter(|entry| entry.request_sha256 == request.request_sha256)
            .ok_or(CodingEndpointOwnerError::NotHeld)
    }
}
