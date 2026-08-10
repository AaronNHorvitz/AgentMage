//! Cancellation and typed cross-boundary failure contracts.

use crate::{CancellationId, ContractError, CorrelationId, TaskId};

/// Logical component boundary participating in propagation.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryKind {
    /// Visual Studio Code or another approved interaction shell.
    Shell,
    /// Security-authoritative interface-independent kernel.
    Kernel,
    /// Platform adapter that owns operating-system enforcement.
    PlatformAdapter,
    /// Sandboxed deterministic tool worker.
    Tool,
    /// Local model adapter or service.
    Model,
}

/// Stable reason a cancellation tree was stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
    /// The user requested cancellation.
    UserRequested,
    /// A parent task or operation was cancelled.
    ParentCancelled,
    /// A declared deadline was reached.
    DeadlineReached,
    /// A declared resource budget was exhausted.
    BudgetExhausted,
    /// Current policy denied continued work.
    PolicyDenied,
    /// Product shutdown requires bounded termination.
    Shutdown,
    /// A dependency failure prevents safe continuation.
    DependencyFailure,
}

/// Versioned cancellation request propagated to descendant boundaries.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancellationSignal {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable cancellation-tree identity.
    pub cancellation_id: CancellationId,
    /// Correlation identity of the affected request.
    pub correlation_id: CorrelationId,
    /// Task whose descendants must stop.
    pub task_id: TaskId,
    /// Stable cancellation reason.
    pub reason: CancellationReason,
    /// Boundary that originated the request.
    pub requested_by: BoundaryKind,
}

/// Non-success outcome propagated through a boundary route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryOutcomeKind {
    /// Current policy denied the operation.
    Denied,
    /// Cancellation terminated the operation.
    Cancelled,
    /// The operation exceeded its deadline.
    TimedOut,
    /// The operation failed with a typed error.
    Failed,
    /// Whether an effect occurred cannot be established safely.
    Uncertain,
}

/// Versioned typed failure preserving exact context across boundaries.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFailure {
    /// Contract schema version.
    pub schema_version: u16,
    /// Correlation identity preserved end to end.
    pub correlation_id: CorrelationId,
    /// Task identity preserved end to end.
    pub task_id: TaskId,
    /// Boundary where the failure originated.
    pub origin: BoundaryKind,
    /// Ordered route beginning at `origin` and ending at the current observer.
    pub route: Vec<BoundaryKind>,
    /// Exact non-success outcome.
    pub outcome: BoundaryOutcomeKind,
    /// Original typed error preserved without replacement.
    pub error: ContractError,
    /// Exact cancellation signal for a cancelled outcome only.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub cancellation: Option<CancellationSignal>,
}

#[cfg(test)]
mod tests {
    use super::{BoundaryKind, CancellationReason, CancellationSignal};
    use crate::{CONTRACT_SCHEMA_VERSION, CancellationId, CorrelationId, TaskId};

    #[test]
    fn cancellation_signal_binds_task_correlation_origin_and_reason() {
        let signal = CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            task_id: TaskId::from_raw("task-0001"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        };
        assert_eq!(signal.requested_by, BoundaryKind::Shell);
        assert_eq!(signal.reason, CancellationReason::UserRequested);
    }
}
