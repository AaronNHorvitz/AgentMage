//! Cancellation trees and lossless typed failure propagation.

use std::sync::{Arc, Mutex, MutexGuard, Weak};

use agentmage_kernel_contracts::{
    BoundaryFailure, BoundaryKind, BoundaryOutcomeKind, CONTRACT_SCHEMA_VERSION,
    CancellationSignal, CorrelationId, ErrorCategory, TaskId,
};

const MAX_BOUNDARY_ROUTE: usize = 5;

/// Typed cancellation-tree construction or signaling failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancellationError {
    /// Parent and child boundary kinds do not form a declared downward edge.
    InvalidBoundaryEdge,
    /// Signal schema version is unsupported.
    UnsupportedSignalVersion,
    /// Signal task or correlation identity does not match the token tree.
    ContextMismatch,
    /// A fresh signal claims an origin other than the token being cancelled.
    OriginMismatch,
}

/// Typed failure-envelope construction or propagation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropagationError {
    /// Failure or nested cancellation schema version is unsupported.
    UnsupportedVersion,
    /// Failure route does not begin exactly at its declared origin.
    InvalidOriginRoute,
    /// Failure outcome, error category, and cancellation fields disagree.
    OutcomeMismatch,
    /// Cancellation context differs from the failure context.
    ContextMismatch,
    /// Current and next boundaries do not form a declared upward edge.
    InvalidBoundaryEdge,
    /// A boundary is repeated or the maximum route length would be exceeded.
    InvalidRouteExtension,
}

#[derive(Debug)]
struct CancellationNode {
    boundary: BoundaryKind,
    task_id: TaskId,
    correlation_id: CorrelationId,
    signal: Option<CancellationSignal>,
    children: Vec<Weak<Mutex<CancellationNode>>>,
}

/// Cloneable cancellation token whose signal propagates only to descendants.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    node: Arc<Mutex<CancellationNode>>,
}

impl CancellationToken {
    /// Creates one uncancelled root token for an exact task and correlation identity.
    #[must_use]
    pub fn root(boundary: BoundaryKind, task_id: TaskId, correlation_id: CorrelationId) -> Self {
        Self {
            node: Arc::new(Mutex::new(CancellationNode {
                boundary,
                task_id,
                correlation_id,
                signal: None,
                children: Vec::new(),
            })),
        }
    }

    /// Derives one child token along a declared downward boundary edge.
    pub fn derive_child(&self, boundary: BoundaryKind) -> Result<Self, CancellationError> {
        let mut parent = lock_node(&self.node);
        if !is_downward_edge(parent.boundary, boundary) {
            return Err(CancellationError::InvalidBoundaryEdge);
        }
        let child = Arc::new(Mutex::new(CancellationNode {
            boundary,
            task_id: parent.task_id.clone(),
            correlation_id: parent.correlation_id.clone(),
            signal: parent.signal.clone(),
            children: Vec::new(),
        }));
        parent
            .children
            .retain(|existing| existing.strong_count() > 0);
        parent.children.push(Arc::downgrade(&child));
        Ok(Self { node: child })
    }

    /// Cancels this token and every uncancelled current or later descendant.
    ///
    /// Repeated cancellation is idempotent: the first signal remains authoritative.
    pub fn cancel(
        &self,
        signal: CancellationSignal,
    ) -> Result<CancellationSignal, CancellationError> {
        {
            let node = lock_node(&self.node);
            validate_signal(&signal, &node)?;
            if let Some(existing) = &node.signal {
                return Ok(existing.clone());
            }
            if signal.requested_by != node.boundary {
                return Err(CancellationError::OriginMismatch);
            }
        }
        Ok(propagate_cancellation(&self.node, &signal))
    }

    /// Returns the first cancellation signal observed by this token.
    #[must_use]
    pub fn signal(&self) -> Option<CancellationSignal> {
        lock_node(&self.node).signal.clone()
    }

    /// Returns this token's logical boundary.
    #[must_use]
    pub fn boundary(&self) -> BoundaryKind {
        lock_node(&self.node).boundary
    }

    /// Reports whether this token has observed cancellation.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        lock_node(&self.node).signal.is_some()
    }
}

/// Validates a newly originated failure envelope.
pub fn validate_origin(failure: &BoundaryFailure) -> Result<(), PropagationError> {
    if failure.schema_version != CONTRACT_SCHEMA_VERSION
        || failure.error.schema_version != CONTRACT_SCHEMA_VERSION
        || failure
            .cancellation
            .as_ref()
            .is_some_and(|signal| signal.schema_version != CONTRACT_SCHEMA_VERSION)
    {
        return Err(PropagationError::UnsupportedVersion);
    }
    if failure.route != [failure.origin] {
        return Err(PropagationError::InvalidOriginRoute);
    }
    validate_outcome(failure)
}

/// Appends one declared observer while preserving every original failure field.
pub fn propagate_failure(
    mut failure: BoundaryFailure,
    observer: BoundaryKind,
) -> Result<BoundaryFailure, PropagationError> {
    validate_existing_route(&failure)?;
    let current = *failure
        .route
        .last()
        .ok_or(PropagationError::InvalidOriginRoute)?;
    if !is_upward_edge(current, observer) {
        return Err(PropagationError::InvalidBoundaryEdge);
    }
    if failure.route.len() >= MAX_BOUNDARY_ROUTE || failure.route.contains(&observer) {
        return Err(PropagationError::InvalidRouteExtension);
    }
    failure.route.push(observer);
    Ok(failure)
}

fn validate_existing_route(failure: &BoundaryFailure) -> Result<(), PropagationError> {
    if failure.schema_version != CONTRACT_SCHEMA_VERSION
        || failure.error.schema_version != CONTRACT_SCHEMA_VERSION
    {
        return Err(PropagationError::UnsupportedVersion);
    }
    if failure.route.first() != Some(&failure.origin) || failure.route.is_empty() {
        return Err(PropagationError::InvalidOriginRoute);
    }
    if failure.route.len() > MAX_BOUNDARY_ROUTE {
        return Err(PropagationError::InvalidRouteExtension);
    }
    let unique = failure
        .route
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != failure.route.len()
        || failure
            .route
            .windows(2)
            .any(|edge| !is_upward_edge(edge[0], edge[1]))
    {
        return Err(PropagationError::InvalidRouteExtension);
    }
    validate_outcome(failure)
}

fn validate_outcome(failure: &BoundaryFailure) -> Result<(), PropagationError> {
    let category_matches = match failure.outcome {
        BoundaryOutcomeKind::Denied => failure.error.category == ErrorCategory::Policy,
        BoundaryOutcomeKind::Cancelled => {
            failure.error.category == ErrorCategory::Cancellation && failure.cancellation.is_some()
        }
        BoundaryOutcomeKind::TimedOut => failure.error.category == ErrorCategory::Timeout,
        BoundaryOutcomeKind::Failed => !matches!(
            failure.error.category,
            ErrorCategory::Policy
                | ErrorCategory::Cancellation
                | ErrorCategory::Timeout
                | ErrorCategory::Uncertain
        ),
        BoundaryOutcomeKind::Uncertain => failure.error.category == ErrorCategory::Uncertain,
    };
    if !category_matches
        || (failure.outcome != BoundaryOutcomeKind::Cancelled && failure.cancellation.is_some())
    {
        return Err(PropagationError::OutcomeMismatch);
    }
    if let Some(signal) = &failure.cancellation {
        if signal.schema_version != CONTRACT_SCHEMA_VERSION {
            return Err(PropagationError::UnsupportedVersion);
        }
        if signal.task_id != failure.task_id || signal.correlation_id != failure.correlation_id {
            return Err(PropagationError::ContextMismatch);
        }
    }
    Ok(())
}

fn validate_signal(
    signal: &CancellationSignal,
    node: &CancellationNode,
) -> Result<(), CancellationError> {
    if signal.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(CancellationError::UnsupportedSignalVersion);
    }
    if signal.task_id != node.task_id || signal.correlation_id != node.correlation_id {
        return Err(CancellationError::ContextMismatch);
    }
    Ok(())
}

fn propagate_cancellation(
    node: &Arc<Mutex<CancellationNode>>,
    signal: &CancellationSignal,
) -> CancellationSignal {
    let (installed, children) = {
        let mut node = lock_node(node);
        if let Some(existing) = &node.signal {
            return existing.clone();
        }
        node.signal = Some(signal.clone());
        (
            signal.clone(),
            node.children
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>(),
        )
    };
    for child in children {
        propagate_cancellation(&child, signal);
    }
    installed
}

fn lock_node(node: &Arc<Mutex<CancellationNode>>) -> MutexGuard<'_, CancellationNode> {
    node.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn is_downward_edge(parent: BoundaryKind, child: BoundaryKind) -> bool {
    matches!(
        (parent, child),
        (BoundaryKind::Shell, BoundaryKind::Kernel)
            | (BoundaryKind::Kernel, BoundaryKind::PlatformAdapter)
            | (BoundaryKind::Kernel, BoundaryKind::Tool)
            | (BoundaryKind::Kernel, BoundaryKind::Model)
            | (BoundaryKind::PlatformAdapter, BoundaryKind::Tool)
            | (BoundaryKind::PlatformAdapter, BoundaryKind::Model)
    )
}

fn is_upward_edge(origin: BoundaryKind, observer: BoundaryKind) -> bool {
    is_downward_edge(observer, origin)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use super::{
        CancellationError, CancellationToken, PropagationError, propagate_failure, validate_origin,
    };
    use agentmage_kernel_contracts::{
        BoundaryFailure, BoundaryKind, BoundaryOutcomeKind, CONTRACT_SCHEMA_VERSION,
        CancellationId, CancellationReason, CancellationSignal, ContractError, CorrelationId,
        ErrorCategory, ErrorId, RetryDisposition, TaskId, from_json, to_canonical_json,
    };

    fn task_id() -> TaskId {
        TaskId::from_raw("task-0001")
    }

    fn correlation_id() -> CorrelationId {
        CorrelationId::from_raw("correlation-0001")
    }

    fn signal(reason: CancellationReason) -> CancellationSignal {
        signal_at(reason, "cancel-0001", BoundaryKind::Shell)
    }

    fn signal_at(
        reason: CancellationReason,
        cancellation_id: &str,
        requested_by: BoundaryKind,
    ) -> CancellationSignal {
        CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw(cancellation_id),
            correlation_id: correlation_id(),
            task_id: task_id(),
            reason,
            requested_by,
        }
    }

    fn category(outcome: BoundaryOutcomeKind) -> ErrorCategory {
        match outcome {
            BoundaryOutcomeKind::Denied => ErrorCategory::Policy,
            BoundaryOutcomeKind::Cancelled => ErrorCategory::Cancellation,
            BoundaryOutcomeKind::TimedOut => ErrorCategory::Timeout,
            BoundaryOutcomeKind::Failed => ErrorCategory::Dependency,
            BoundaryOutcomeKind::Uncertain => ErrorCategory::Uncertain,
        }
    }

    fn failure(origin: BoundaryKind, outcome: BoundaryOutcomeKind) -> BoundaryFailure {
        BoundaryFailure {
            schema_version: CONTRACT_SCHEMA_VERSION,
            correlation_id: correlation_id(),
            task_id: task_id(),
            origin,
            route: vec![origin],
            outcome,
            error: ContractError {
                schema_version: CONTRACT_SCHEMA_VERSION,
                error_id: ErrorId::from_raw("error-0001"),
                code: "fixture.failure".to_owned(),
                category: category(outcome),
                message: "Synthetic bounded failure".to_owned(),
                field_path: Vec::new(),
                retry: RetryDisposition::Never,
                caused_by: None,
            },
            cancellation: (outcome == BoundaryOutcomeKind::Cancelled)
                .then(|| signal(CancellationReason::UserRequested)),
        }
    }

    #[test]
    fn parent_cancellation_reaches_every_current_and_late_descendant() {
        let shell = CancellationToken::root(BoundaryKind::Shell, task_id(), correlation_id());
        let kernel = shell
            .derive_child(BoundaryKind::Kernel)
            .expect("shell to kernel");
        let model = kernel
            .derive_child(BoundaryKind::Model)
            .expect("kernel to model");
        let platform = kernel
            .derive_child(BoundaryKind::PlatformAdapter)
            .expect("kernel to platform");
        let tool = platform
            .derive_child(BoundaryKind::Tool)
            .expect("platform to tool");
        let cancellation = signal(CancellationReason::UserRequested);
        assert_eq!(shell.cancel(cancellation.clone()), Ok(cancellation.clone()));
        for token in [&shell, &kernel, &model, &platform, &tool] {
            assert_eq!(token.signal(), Some(cancellation.clone()));
        }
        let late = kernel.derive_child(BoundaryKind::Tool).expect("late child");
        assert_eq!(late.signal(), Some(cancellation));
    }

    #[test]
    fn child_cancellation_is_descendant_only_and_first_signal_wins() {
        let root = CancellationToken::root(BoundaryKind::Kernel, task_id(), correlation_id());
        let tool = root
            .derive_child(BoundaryKind::Tool)
            .expect("kernel to tool");
        let model = root
            .derive_child(BoundaryKind::Model)
            .expect("kernel to model");
        let first = signal_at(
            CancellationReason::DependencyFailure,
            "cancel-dependency",
            BoundaryKind::Tool,
        );
        let second = signal_at(
            CancellationReason::Shutdown,
            "cancel-shutdown",
            BoundaryKind::Tool,
        );
        assert_eq!(tool.cancel(first.clone()), Ok(first.clone()));
        assert_eq!(tool.cancel(second), Ok(first.clone()));
        assert_eq!(tool.signal(), Some(first));
        assert!(!root.is_cancelled());
        assert!(!model.is_cancelled());
    }

    #[test]
    fn invalid_edges_versions_and_context_fail_closed() {
        let root = CancellationToken::root(BoundaryKind::Shell, task_id(), correlation_id());
        assert!(matches!(
            root.derive_child(BoundaryKind::Tool),
            Err(CancellationError::InvalidBoundaryEdge)
        ));
        let mut wrong_version = signal(CancellationReason::UserRequested);
        wrong_version.schema_version += 1;
        assert_eq!(
            root.cancel(wrong_version),
            Err(CancellationError::UnsupportedSignalVersion)
        );
        let mut wrong_task = signal(CancellationReason::UserRequested);
        wrong_task.task_id = TaskId::from_raw("task-elsewhere");
        assert_eq!(
            root.cancel(wrong_task),
            Err(CancellationError::ContextMismatch)
        );
        let mut wrong_origin = signal(CancellationReason::UserRequested);
        wrong_origin.requested_by = BoundaryKind::Kernel;
        assert_eq!(
            root.cancel(wrong_origin),
            Err(CancellationError::OriginMismatch)
        );
        assert!(!root.is_cancelled());
    }

    #[test]
    fn simultaneous_cancellation_returns_the_one_atomically_installed_signal() {
        let token = CancellationToken::root(BoundaryKind::Kernel, task_id(), correlation_id());
        let barrier = Arc::new(Barrier::new(3));
        let callers = [
            signal_at(
                CancellationReason::Shutdown,
                "cancel-shutdown",
                BoundaryKind::Kernel,
            ),
            signal_at(
                CancellationReason::DependencyFailure,
                "cancel-dependency",
                BoundaryKind::Kernel,
            ),
        ]
        .into_iter()
        .map(|candidate| {
            let token = token.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                token.cancel(candidate).expect("valid cancellation")
            })
        })
        .collect::<Vec<_>>();

        barrier.wait();
        let returned = callers
            .into_iter()
            .map(|caller| caller.join().expect("caller must not panic"))
            .collect::<Vec<_>>();
        assert_eq!(returned[0], returned[1]);
        assert_eq!(token.signal(), Some(returned[0].clone()));
    }

    #[test]
    fn every_failure_outcome_preserves_context_through_declared_routes() {
        for outcome in [
            BoundaryOutcomeKind::Denied,
            BoundaryOutcomeKind::Cancelled,
            BoundaryOutcomeKind::TimedOut,
            BoundaryOutcomeKind::Failed,
            BoundaryOutcomeKind::Uncertain,
        ] {
            let originated = failure(BoundaryKind::Tool, outcome);
            validate_origin(&originated).expect("valid origin");
            let platform = propagate_failure(originated.clone(), BoundaryKind::PlatformAdapter)
                .expect("tool to platform");
            let kernel =
                propagate_failure(platform, BoundaryKind::Kernel).expect("platform to kernel");
            let shell = propagate_failure(kernel, BoundaryKind::Shell).expect("kernel to shell");
            assert_eq!(
                shell.route,
                [
                    BoundaryKind::Tool,
                    BoundaryKind::PlatformAdapter,
                    BoundaryKind::Kernel,
                    BoundaryKind::Shell,
                ]
            );
            assert_eq!(shell.error, originated.error);
            assert_eq!(shell.correlation_id, originated.correlation_id);
            assert_eq!(shell.task_id, originated.task_id);
            assert_eq!(shell.cancellation, originated.cancellation);
        }
    }

    #[test]
    fn model_failure_routes_directly_to_kernel_and_shell() {
        let originated = failure(BoundaryKind::Model, BoundaryOutcomeKind::Failed);
        let kernel = propagate_failure(originated, BoundaryKind::Kernel).expect("model to kernel");
        let shell = propagate_failure(kernel, BoundaryKind::Shell).expect("kernel to shell");
        assert_eq!(
            shell.route,
            [
                BoundaryKind::Model,
                BoundaryKind::Kernel,
                BoundaryKind::Shell
            ]
        );
    }

    #[test]
    fn outcome_context_route_and_serialization_mutations_fail_closed() {
        let cancelled = failure(BoundaryKind::Tool, BoundaryOutcomeKind::Cancelled);
        let bytes = to_canonical_json(&cancelled).expect("failure must serialize");
        assert_eq!(
            from_json::<BoundaryFailure>(&bytes).expect("failure must parse"),
            cancelled
        );

        let mut wrong_category = failure(BoundaryKind::Tool, BoundaryOutcomeKind::Denied);
        wrong_category.error.category = ErrorCategory::Internal;
        assert_eq!(
            validate_origin(&wrong_category),
            Err(PropagationError::OutcomeMismatch)
        );
        let mut wrong_context = failure(BoundaryKind::Tool, BoundaryOutcomeKind::Cancelled);
        wrong_context
            .cancellation
            .as_mut()
            .expect("fixture cancellation")
            .task_id = TaskId::from_raw("task-elsewhere");
        assert_eq!(
            validate_origin(&wrong_context),
            Err(PropagationError::ContextMismatch)
        );
        let mut wrong_route = failure(BoundaryKind::Tool, BoundaryOutcomeKind::Failed);
        wrong_route.route = vec![BoundaryKind::Kernel];
        assert_eq!(
            validate_origin(&wrong_route),
            Err(PropagationError::InvalidOriginRoute)
        );
        assert_eq!(
            propagate_failure(
                failure(BoundaryKind::Tool, BoundaryOutcomeKind::Failed),
                BoundaryKind::Shell,
            ),
            Err(PropagationError::InvalidBoundaryEdge)
        );
    }
}
