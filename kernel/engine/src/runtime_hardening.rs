//! Request-bound runtime resource ceilings and content-free accounting.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::{
    BudgetResource, RuntimeResourceUsage, RuntimeRunRequest, RuntimeSessionMode,
};

/// Maximum event envelopes retained by one client queue.
pub const MAX_RUNTIME_CLIENT_QUEUE_EVENTS: u32 = 4_096;
/// Maximum canonical bytes represented by one client queue.
pub const MAX_RUNTIME_CLIENT_QUEUE_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum canonical bytes admitted for one runtime event envelope.
pub const MAX_RUNTIME_EVENT_ENVELOPE_BYTES: u64 = 64 * 1024;
/// Maximum artifacts produced by one runtime invocation.
pub const MAX_RUNTIME_ARTIFACT_COUNT: u32 = 1_024;
/// Maximum aggregate artifact payload bytes produced by one runtime invocation.
pub const MAX_RUNTIME_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum bytes returned by one artifact preview page.
pub const MAX_RUNTIME_ARTIFACT_PAGE_BYTES: u32 = 4 * 1024;

const _: () = {
    assert!(MAX_RUNTIME_CLIENT_QUEUE_EVENTS > 0);
    assert!(MAX_RUNTIME_CLIENT_QUEUE_BYTES > MAX_RUNTIME_EVENT_ENVELOPE_BYTES);
    assert!(MAX_RUNTIME_ARTIFACT_COUNT > 0);
    assert!(MAX_RUNTIME_ARTIFACT_BYTES > MAX_RUNTIME_ARTIFACT_PAGE_BYTES as u64);
};

/// Stable fail-closed reason for a runtime hardening decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeHardeningError {
    /// A controlled-write request omitted one required resource budget.
    MissingBudget(BudgetResource),
    /// A work-packet budget is narrower than the exact runtime request.
    BudgetBinding(BudgetResource),
    /// A derived queue, artifact, or failure ceiling is invalid.
    InvalidLimits,
    /// An attempted cumulative resource use exceeds its declared ceiling.
    ResourceExhausted(BudgetResource),
    /// An attempted event envelope exceeds its fixed byte ceiling.
    EventEnvelopeExhausted,
    /// An attempted artifact count or aggregate byte total exceeds its ceiling.
    ArtifactExhausted,
    /// A retry was attempted even though runtime retries are disabled.
    RetryDenied,
    /// A resource counter overflowed before comparison.
    CounterOverflow,
}

impl RuntimeHardeningError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingBudget(_) => "runtime.hardening.budget_missing",
            Self::BudgetBinding(_) => "runtime.hardening.budget_binding",
            Self::InvalidLimits => "runtime.hardening.limits_invalid",
            Self::ResourceExhausted(_) => "runtime.budget.exhausted",
            Self::EventEnvelopeExhausted => "runtime.event.envelope_exhausted",
            Self::ArtifactExhausted => "runtime.artifact.budget_exhausted",
            Self::RetryDenied => "runtime.retry.denied",
            Self::CounterOverflow => "runtime.hardening.counter_overflow",
        }
    }
}

/// Complete non-authoritative resource envelope derived from one sealed request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeHardeningLimits {
    /// Maximum canonical events admitted across the complete run.
    pub run_events: u32,
    /// Maximum event envelopes admitted to one client subscription.
    pub client_queue_events: u32,
    /// Maximum canonical event bytes represented by one client subscription.
    pub client_queue_bytes: u64,
    /// Maximum bytes in one canonical event envelope.
    pub event_envelope_bytes: u64,
    /// Maximum artifacts produced by the run.
    pub artifact_count: u32,
    /// Maximum aggregate artifact payload bytes produced by the run.
    pub artifact_bytes: u64,
    /// Maximum bytes returned by one artifact preview page.
    pub artifact_page_bytes: u32,
    /// Runtime-internal retry attempts; fixed at zero for the current runtime.
    pub retry_attempts: u32,
    /// Maximum policy or user denials observed before terminal closure.
    pub denials: u32,
    /// Maximum malformed model proposals observed before terminal closure.
    pub parser_failures: u32,
}

impl RuntimeHardeningLimits {
    /// Derives exact hardening ceilings and verifies controlled-write budget coverage.
    pub fn from_request(request: &RuntimeRunRequest) -> Result<Self, RuntimeHardeningError> {
        let budgets = request
            .work_packet
            .budgets
            .iter()
            .map(|budget| (budget.resource, budget.limit))
            .collect::<BTreeMap<_, _>>();
        if request.mode == RuntimeSessionMode::ControlledWrite {
            validate_controlled_write_budgets(request, &budgets)?;
        }

        let client_queue_events = request
            .limits
            .max_events
            .min(MAX_RUNTIME_CLIENT_QUEUE_EVENTS);
        let client_queue_bytes = u64::from(client_queue_events)
            .checked_mul(MAX_RUNTIME_EVENT_ENVELOPE_BYTES)
            .ok_or(RuntimeHardeningError::InvalidLimits)?
            .min(MAX_RUNTIME_CLIENT_QUEUE_BYTES);
        let artifact_count = request
            .limits
            .max_tool_calls
            .checked_add(request.limits.max_turns)
            .and_then(|value| value.checked_add(1))
            .ok_or(RuntimeHardeningError::InvalidLimits)?
            .clamp(1, MAX_RUNTIME_ARTIFACT_COUNT);
        let artifact_bytes = budgets
            .get(&BudgetResource::DiskBytes)
            .copied()
            .unwrap_or(request.limits.max_output_bytes)
            .min(MAX_RUNTIME_ARTIFACT_BYTES);
        if client_queue_events == 0
            || client_queue_bytes == 0
            || artifact_bytes == 0
            || request.limits.max_output_bytes == 0
        {
            return Err(RuntimeHardeningError::InvalidLimits);
        }
        Ok(Self {
            run_events: request.limits.max_events,
            client_queue_events,
            client_queue_bytes,
            event_envelope_bytes: MAX_RUNTIME_EVENT_ENVELOPE_BYTES,
            artifact_count,
            artifact_bytes,
            artifact_page_bytes: MAX_RUNTIME_ARTIFACT_PAGE_BYTES,
            retry_attempts: 0,
            denials: 1,
            parser_failures: 1,
        })
    }
}

/// Content-free cumulative and peak resource observations for one run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeResourceSnapshot {
    /// Cumulative admitted values keyed by closed work-packet resource.
    pub cumulative: BTreeMap<BudgetResource, u64>,
    /// Peak memory observed from a trusted model or process boundary.
    pub peak_memory_bytes: u64,
    /// Canonical event envelopes admitted by the coordinator.
    pub event_count: u32,
    /// Aggregate canonical event-envelope bytes admitted by the coordinator.
    pub event_bytes: u64,
    /// Immutable artifacts admitted by the coordinator.
    pub artifact_count: u32,
    /// Aggregate immutable artifact payload bytes admitted by the coordinator.
    pub artifact_bytes: u64,
    /// Policy or user denials observed by the coordinator.
    pub denial_count: u32,
    /// Malformed model proposals observed by the coordinator.
    pub parser_failure_count: u32,
    /// Runtime-internal retries attempted by the coordinator.
    pub retry_count: u32,
}

/// Stateful resource ledger that cannot broaden its sealed request budgets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceLedger {
    mode: RuntimeSessionMode,
    budgets: BTreeMap<BudgetResource, u64>,
    limits: RuntimeHardeningLimits,
    snapshot: RuntimeResourceSnapshot,
}

impl RuntimeResourceLedger {
    /// Creates an empty ledger from one already framed runtime request.
    pub fn new(request: &RuntimeRunRequest) -> Result<Self, RuntimeHardeningError> {
        let limits = RuntimeHardeningLimits::from_request(request)?;
        Ok(Self {
            mode: request.mode,
            budgets: request
                .work_packet
                .budgets
                .iter()
                .map(|budget| (budget.resource, budget.limit))
                .collect(),
            limits,
            snapshot: RuntimeResourceSnapshot::default(),
        })
    }

    /// Restores exact content-free usage without resetting any cumulative ceiling.
    pub fn restore(
        request: &RuntimeRunRequest,
        usage: &RuntimeResourceUsage,
    ) -> Result<Self, RuntimeHardeningError> {
        let mut ledger = Self::new(request)?;
        for (resource, amount) in [
            (BudgetResource::PlanSteps, usage.plan_steps),
            (BudgetResource::ModelCalls, usage.model_calls),
            (BudgetResource::ToolCalls, usage.tool_calls),
            (BudgetResource::InputBytes, usage.input_bytes),
            (BudgetResource::OutputBytes, usage.output_bytes),
            (BudgetResource::ElapsedMilliseconds, usage.elapsed_ms),
            (BudgetResource::DiskBytes, usage.disk_bytes),
            (BudgetResource::ProcessCount, usage.process_count),
        ] {
            ledger.consume(resource, amount)?;
        }
        ledger.observe_memory_peak(usage.peak_memory_bytes)?;
        if usage.event_count > ledger.limits.run_events
            || usage.event_bytes
                > u64::from(usage.event_count)
                    .checked_mul(ledger.limits.event_envelope_bytes)
                    .ok_or(RuntimeHardeningError::CounterOverflow)?
            || usage.artifact_count > ledger.limits.artifact_count
            || usage.artifact_bytes > ledger.limits.artifact_bytes
            || usage.artifact_bytes > usage.disk_bytes
            || usage.denial_count > ledger.limits.denials
            || usage.parser_failure_count > ledger.limits.parser_failures
            || usage.retry_count != 0
        {
            return Err(RuntimeHardeningError::InvalidLimits);
        }
        ledger.snapshot.event_count = usage.event_count;
        ledger.snapshot.event_bytes = usage.event_bytes;
        ledger.snapshot.artifact_count = usage.artifact_count;
        ledger.snapshot.artifact_bytes = usage.artifact_bytes;
        ledger.snapshot.denial_count = usage.denial_count;
        ledger.snapshot.parser_failure_count = usage.parser_failure_count;
        ledger.snapshot.retry_count = usage.retry_count;
        Ok(ledger)
    }

    /// Returns the exact derived hardening limits.
    #[must_use]
    pub const fn limits(&self) -> &RuntimeHardeningLimits {
        &self.limits
    }

    /// Returns current content-free usage without changing authority or state.
    #[must_use]
    pub const fn snapshot(&self) -> &RuntimeResourceSnapshot {
        &self.snapshot
    }

    /// Projects current usage into the interface-neutral durable continuation contract.
    #[must_use]
    pub fn durable_usage(&self) -> RuntimeResourceUsage {
        RuntimeResourceUsage {
            plan_steps: self.usage(BudgetResource::PlanSteps),
            model_calls: self.usage(BudgetResource::ModelCalls),
            tool_calls: self.usage(BudgetResource::ToolCalls),
            input_bytes: self.usage(BudgetResource::InputBytes),
            output_bytes: self.usage(BudgetResource::OutputBytes),
            elapsed_ms: self.usage(BudgetResource::ElapsedMilliseconds),
            peak_memory_bytes: self.snapshot.peak_memory_bytes,
            disk_bytes: self.usage(BudgetResource::DiskBytes),
            process_count: self.usage(BudgetResource::ProcessCount),
            event_count: self.snapshot.event_count,
            event_bytes: self.snapshot.event_bytes,
            artifact_count: self.snapshot.artifact_count,
            artifact_bytes: self.snapshot.artifact_bytes,
            denial_count: self.snapshot.denial_count,
            parser_failure_count: self.snapshot.parser_failure_count,
            retry_count: self.snapshot.retry_count,
        }
    }

    /// Reconciles additional durable journal and continuation-artifact records after restart.
    pub fn reconcile_durable_projection(
        &mut self,
        event_count: u32,
        event_bytes: u64,
        artifact_count: u32,
        artifact_bytes: u64,
    ) -> Result<(), RuntimeHardeningError> {
        if event_count < self.snapshot.event_count
            || event_bytes < self.snapshot.event_bytes
            || artifact_count < self.snapshot.artifact_count
            || artifact_bytes < self.snapshot.artifact_bytes
            || event_count > self.limits.run_events
            || event_bytes
                > u64::from(event_count)
                    .checked_mul(self.limits.event_envelope_bytes)
                    .ok_or(RuntimeHardeningError::CounterOverflow)?
            || artifact_count > self.limits.artifact_count
            || artifact_bytes > self.limits.artifact_bytes
        {
            return Err(RuntimeHardeningError::InvalidLimits);
        }
        let prior_disk = self.usage(BudgetResource::DiskBytes);
        let additional_artifact_bytes = artifact_bytes.saturating_sub(self.snapshot.artifact_bytes);
        let reconciled_disk = prior_disk
            .checked_add(additional_artifact_bytes)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        let Some(disk_limit) = self.budgets.get(&BudgetResource::DiskBytes).copied() else {
            if self.mode == RuntimeSessionMode::ControlledWrite {
                return Err(RuntimeHardeningError::MissingBudget(
                    BudgetResource::DiskBytes,
                ));
            }
            self.snapshot.event_count = event_count;
            self.snapshot.event_bytes = event_bytes;
            self.snapshot.artifact_count = artifact_count;
            self.snapshot.artifact_bytes = artifact_bytes;
            return Ok(());
        };
        if reconciled_disk > disk_limit {
            return Err(RuntimeHardeningError::ResourceExhausted(
                BudgetResource::DiskBytes,
            ));
        }
        self.snapshot
            .cumulative
            .insert(BudgetResource::DiskBytes, reconciled_disk);
        self.snapshot.event_count = event_count;
        self.snapshot.event_bytes = event_bytes;
        self.snapshot.artifact_count = artifact_count;
        self.snapshot.artifact_bytes = artifact_bytes;
        Ok(())
    }

    /// Verifies that one subscriber queue fits both count and worst-case byte ceilings.
    pub fn validate_subscription_capacity(
        &self,
        capacity: usize,
    ) -> Result<(), RuntimeHardeningError> {
        let capacity = u64::try_from(capacity).map_err(|_| RuntimeHardeningError::InvalidLimits)?;
        let represented_bytes = capacity
            .checked_mul(self.limits.event_envelope_bytes)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        if capacity == 0
            || capacity > u64::from(self.limits.client_queue_events)
            || represented_bytes > self.limits.client_queue_bytes
        {
            return Err(RuntimeHardeningError::InvalidLimits);
        }
        Ok(())
    }

    /// Admits cumulative use when declared capacity remains.
    pub fn consume(
        &mut self,
        resource: BudgetResource,
        amount: u64,
    ) -> Result<(), RuntimeHardeningError> {
        let Some(limit) = self.budgets.get(&resource).copied() else {
            return if self.mode == RuntimeSessionMode::ControlledWrite {
                Err(RuntimeHardeningError::MissingBudget(resource))
            } else {
                Ok(())
            };
        };
        let used = self
            .snapshot
            .cumulative
            .get(&resource)
            .copied()
            .unwrap_or(0);
        let attempted = used
            .checked_add(amount)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        if attempted > limit {
            return Err(RuntimeHardeningError::ResourceExhausted(resource));
        }
        self.snapshot.cumulative.insert(resource, attempted);
        Ok(())
    }

    /// Admits a peak-memory observation without treating repeated observations as cumulative.
    pub fn observe_memory_peak(&mut self, bytes: u64) -> Result<(), RuntimeHardeningError> {
        let Some(limit) = self.budgets.get(&BudgetResource::MemoryBytes).copied() else {
            return if self.mode == RuntimeSessionMode::ControlledWrite {
                Err(RuntimeHardeningError::MissingBudget(
                    BudgetResource::MemoryBytes,
                ))
            } else {
                Ok(())
            };
        };
        if bytes > limit {
            return Err(RuntimeHardeningError::ResourceExhausted(
                BudgetResource::MemoryBytes,
            ));
        }
        self.snapshot.peak_memory_bytes = self.snapshot.peak_memory_bytes.max(bytes);
        self.snapshot
            .cumulative
            .insert(BudgetResource::MemoryBytes, self.snapshot.peak_memory_bytes);
        Ok(())
    }

    /// Accounts one canonical event envelope before publication or persistence.
    pub fn admit_event(&mut self, canonical_bytes: u64) -> Result<(), RuntimeHardeningError> {
        if canonical_bytes == 0 || canonical_bytes > self.limits.event_envelope_bytes {
            return Err(RuntimeHardeningError::EventEnvelopeExhausted);
        }
        let event_count = self
            .snapshot
            .event_count
            .checked_add(1)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        let event_bytes = self
            .snapshot
            .event_bytes
            .checked_add(canonical_bytes)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        if event_count > self.limits.run_events {
            return Err(RuntimeHardeningError::EventEnvelopeExhausted);
        }
        self.snapshot.event_count = event_count;
        self.snapshot.event_bytes = event_bytes;
        Ok(())
    }

    /// Reserves one immutable artifact before bytes cross the artifact boundary.
    pub fn admit_artifact(&mut self, bytes: u64) -> Result<(), RuntimeHardeningError> {
        let artifact_count = self
            .snapshot
            .artifact_count
            .checked_add(1)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        let artifact_bytes = self
            .snapshot
            .artifact_bytes
            .checked_add(bytes)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        if bytes == 0
            || artifact_count > self.limits.artifact_count
            || artifact_bytes > self.limits.artifact_bytes
        {
            return Err(RuntimeHardeningError::ArtifactExhausted);
        }
        self.consume(BudgetResource::DiskBytes, bytes)?;
        self.snapshot.artifact_count = artifact_count;
        self.snapshot.artifact_bytes = artifact_bytes;
        Ok(())
    }

    /// Records one denial; callers close the run at the first admitted denial.
    pub fn record_denial(&mut self) -> Result<(), RuntimeHardeningError> {
        let denial_count = self
            .snapshot
            .denial_count
            .checked_add(1)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        if denial_count > self.limits.denials {
            return Err(RuntimeHardeningError::InvalidLimits);
        }
        self.snapshot.denial_count = denial_count;
        Ok(())
    }

    /// Records one malformed model proposal; callers close the run at the first occurrence.
    pub fn record_parser_failure(&mut self) -> Result<(), RuntimeHardeningError> {
        let parser_failure_count = self
            .snapshot
            .parser_failure_count
            .checked_add(1)
            .ok_or(RuntimeHardeningError::CounterOverflow)?;
        if parser_failure_count > self.limits.parser_failures {
            return Err(RuntimeHardeningError::InvalidLimits);
        }
        self.snapshot.parser_failure_count = parser_failure_count;
        Ok(())
    }

    /// Rejects every runtime-internal retry without changing the zero-retry ledger.
    pub fn record_retry(&mut self) -> Result<(), RuntimeHardeningError> {
        Err(RuntimeHardeningError::RetryDenied)
    }

    fn usage(&self, resource: BudgetResource) -> u64 {
        self.snapshot
            .cumulative
            .get(&resource)
            .copied()
            .unwrap_or(0)
    }
}

fn validate_controlled_write_budgets(
    request: &RuntimeRunRequest,
    budgets: &BTreeMap<BudgetResource, u64>,
) -> Result<(), RuntimeHardeningError> {
    let cumulative_input = request
        .context_budget
        .max_input_bytes
        .checked_mul(u64::from(request.limits.max_context_refreshes))
        .ok_or(RuntimeHardeningError::InvalidLimits)?;
    let required = [
        (
            BudgetResource::PlanSteps,
            u64::from(request.limits.max_turns),
        ),
        (
            BudgetResource::ToolCallDepth,
            u64::from(request.limits.max_tool_call_depth),
        ),
        (
            BudgetResource::ModelCalls,
            u64::from(request.limits.max_model_calls),
        ),
        (
            BudgetResource::ToolCalls,
            u64::from(request.limits.max_tool_calls),
        ),
        (BudgetResource::InputBytes, cumulative_input),
        (BudgetResource::OutputBytes, request.limits.max_output_bytes),
        (
            BudgetResource::ElapsedMilliseconds,
            request.limits.max_elapsed_ms,
        ),
        (BudgetResource::MemoryBytes, 1),
        (BudgetResource::DiskBytes, request.limits.max_output_bytes),
        (
            BudgetResource::ProcessCount,
            u64::from(request.limits.max_tool_calls.min(1)),
        ),
    ];
    for (resource, minimum) in required {
        let Some(limit) = budgets.get(&resource).copied() else {
            return Err(RuntimeHardeningError::MissingBudget(resource));
        };
        if limit < minimum {
            return Err(RuntimeHardeningError::BudgetBinding(resource));
        }
    }
    Ok(())
}
