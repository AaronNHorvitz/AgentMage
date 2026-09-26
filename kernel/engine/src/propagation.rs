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
    // A live descendant must keep its ancestry reachable even if the caller
    // drops an intermediate token. Downward edges remain weak, so this cannot
    // form a reference cycle or keep a finished descendant alive.
    _parent: Option<Arc<Mutex<CancellationNode>>>,
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
                _parent: None,
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
            _parent: Some(Arc::clone(&self.node)),
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

/// Content-free failure while observing an existing cancellation owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancellationObservationError {
    /// The existing control owner could not be read safely.
    Unavailable,
    /// Schema, identity shape or declared origin was invalid.
    InvalidSignal,
    /// The signal belongs to another task or correlation scope.
    ScopeMismatch,
}

/// Passive cancellation observation for the existing bounded effect poll loop.
///
/// No launch callback, wake handle, authority or execution loop is exposed.
/// Implementations must return promptly and must not acquire the canonical store.
pub trait EffectCancellationObservation: Sync {
    /// Observes a signal; an error requests fail-closed termination, not continuation.
    fn observe_effect_cancellation(
        &self,
    ) -> Result<Option<CancellationSignal>, CancellationObservationError>;
}

impl EffectCancellationObservation for CancellationToken {
    fn observe_effect_cancellation(
        &self,
    ) -> Result<Option<CancellationSignal>, CancellationObservationError> {
        // The passive effect path must not wait behind another observer or
        // recover a poisoned owner as an uncancelled token. Legacy token APIs
        // keep their existing propagation semantics.
        self.node
            .try_lock()
            .map(|node| node.signal.clone())
            .map_err(|_| CancellationObservationError::Unavailable)
    }
}

impl<T: EffectCancellationObservation + ?Sized> EffectCancellationObservation for &T {
    fn observe_effect_cancellation(
        &self,
    ) -> Result<Option<CancellationSignal>, CancellationObservationError> {
        (**self).observe_effect_cancellation()
    }
}

/// Passive view checked against the consumed permit on EVERY platform poll.
///
/// The caller supplies identities from the consumed permit after exact authority
/// matching. This view neither consumes authority nor creates a cancellation owner.
/// It cannot turn observer failure into user cancellation or a continuation.
pub struct ScopedEffectCancellation<'attempt> {
    observation: &'attempt dyn EffectCancellationObservation,
    task_id: &'attempt TaskId,
    correlation_id: &'attempt CorrelationId,
}

impl<'attempt> ScopedEffectCancellation<'attempt> {
    /// Creates an inert scope check; no observer is called by construction.
    #[must_use]
    pub const fn new(
        observation: &'attempt dyn EffectCancellationObservation,
        task_id: &'attempt TaskId,
        correlation_id: &'attempt CorrelationId,
    ) -> Self {
        Self {
            observation,
            task_id,
            correlation_id,
        }
    }
}

impl EffectCancellationObservation for ScopedEffectCancellation<'_> {
    fn observe_effect_cancellation(
        &self,
    ) -> Result<Option<CancellationSignal>, CancellationObservationError> {
        observe_effect_cancellation(self.observation, self.task_id, self.correlation_id)
    }
}

/// A borrowed, task-bound view of the existing live runtime control owner.
///
/// The first observed cancellation or error remains sticky for this attempt.
/// The source signal and its origin are preserved; no Tool-origin signal is minted.
pub struct BorrowedEffectCancellation<'source> {
    source: &'source dyn agentmage_kernel_contracts::ModelCancellationProbe,
    task_id: TaskId,
    correlation_id: CorrelationId,
    observed: Mutex<Option<Result<CancellationSignal, CancellationObservationError>>>,
}

impl<'source> BorrowedEffectCancellation<'source> {
    /// Creates an inert view. Construction does not poll, spawn or grant an effect.
    #[must_use]
    pub fn new(
        source: &'source dyn agentmage_kernel_contracts::ModelCancellationProbe,
        task_id: TaskId,
        correlation_id: CorrelationId,
    ) -> Self {
        Self {
            source,
            task_id,
            correlation_id,
            observed: Mutex::new(None),
        }
    }
}

impl EffectCancellationObservation for BorrowedEffectCancellation<'_> {
    fn observe_effect_cancellation(
        &self,
    ) -> Result<Option<CancellationSignal>, CancellationObservationError> {
        // This is an attempt-local observation cache, not the source's mutex or
        // the authority store. Never hold it while calling the source observer.
        if let Some(observed) = self
            .observed
            .try_lock()
            .map_err(|_| CancellationObservationError::Unavailable)?
            .as_ref()
            .cloned()
        {
            return observed.map(Some);
        }
        let candidate = self
            .source
            .observe()
            .map_err(|_| CancellationObservationError::Unavailable)
            .and_then(|signal| {
                if let Some(signal) = signal.as_ref() {
                    validate_effect_cancellation(signal, &self.task_id, &self.correlation_id)?;
                }
                Ok(signal)
            });
        let mut observed = self
            .observed
            .try_lock()
            .map_err(|_| CancellationObservationError::Unavailable)?;
        // Another observer may have installed the first terminal observation while
        // this observer read the source. Never overwrite it with a later outcome.
        if observed.is_none() {
            match candidate {
                Ok(Some(signal)) => *observed = Some(Ok(signal)),
                Err(error) => *observed = Some(Err(error)),
                Ok(None) => {}
            }
        }
        observed
            .as_ref()
            .cloned()
            .map_or(Ok(None), |value| value.map(Some))
    }
}

/// Binds passive observation to the exact consumed effect scope.
pub fn observe_effect_cancellation(
    observation: &dyn EffectCancellationObservation,
    task_id: &TaskId,
    correlation_id: &CorrelationId,
) -> Result<Option<CancellationSignal>, CancellationObservationError> {
    let signal = observation.observe_effect_cancellation()?;
    if let Some(signal) = signal.as_ref() {
        validate_effect_cancellation(signal, task_id, correlation_id)?;
    }
    Ok(signal)
}

fn validate_effect_cancellation(
    signal: &CancellationSignal,
    task_id: &TaskId,
    correlation_id: &CorrelationId,
) -> Result<(), CancellationObservationError> {
    let identifier = |value: &str| {
        !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
    };
    if signal.schema_version != CONTRACT_SCHEMA_VERSION
        || !identifier(signal.cancellation_id.as_str())
        || !identifier(signal.task_id.as_str())
        || !identifier(signal.correlation_id.as_str())
        || !matches!(
            signal.requested_by,
            BoundaryKind::Shell
                | BoundaryKind::Kernel
                | BoundaryKind::PlatformAdapter
                | BoundaryKind::Tool
        )
    {
        return Err(CancellationObservationError::InvalidSignal);
    }
    if signal.task_id != *task_id || signal.correlation_id != *correlation_id {
        return Err(CancellationObservationError::ScopeMismatch);
    }
    Ok(())
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
    use std::sync::{Arc, Barrier, Mutex};

    use super::{
        BorrowedEffectCancellation, CancellationError, CancellationObservationError,
        CancellationToken, EffectCancellationObservation, PropagationError,
        ScopedEffectCancellation, observe_effect_cancellation, propagate_failure, validate_origin,
    };
    use agentmage_kernel_contracts::{
        BoundaryFailure, BoundaryKind, BoundaryOutcomeKind, CONTRACT_SCHEMA_VERSION,
        CancellationId, CancellationReason, CancellationSignal, ContractError, CorrelationId,
        ErrorCategory, ErrorId, ModelCancellationProbe, ModelRuntimeFailure, RetryDisposition,
        TaskId, from_json, to_canonical_json,
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

    struct MutableProbe(Mutex<Result<Option<CancellationSignal>, ModelRuntimeFailure>>);
    impl ModelCancellationProbe for MutableProbe {
        fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
            self.0.lock().unwrap().clone()
        }
    }
    fn effect_signal() -> CancellationSignal {
        CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-native-effect"),
            correlation_id: CorrelationId::from_raw("correlation-native-effect"),
            task_id: TaskId::from_raw("task-native-effect"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        }
    }
    fn view(source: &MutableProbe) -> BorrowedEffectCancellation<'_> {
        let signal = effect_signal();
        BorrowedEffectCancellation::new(source, signal.task_id, signal.correlation_id)
    }

    #[test]
    fn borrowed_effect_observation_retains_exact_shell_signal_after_source_reset() {
        let source = MutableProbe(Mutex::new(Ok(None)));
        let observation = view(&source);
        assert_eq!(observation.observe_effect_cancellation(), Ok(None));
        *source.0.lock().unwrap() = Ok(Some(effect_signal()));
        assert_eq!(
            observation.observe_effect_cancellation(),
            Ok(Some(effect_signal()))
        );
        *source.0.lock().unwrap() = Ok(None);
        assert_eq!(
            observation.observe_effect_cancellation(),
            Ok(Some(effect_signal()))
        );
    }

    #[test]
    fn borrowed_effect_observation_rejects_foreign_malformed_and_model_origin_signals() {
        for case in 0..6 {
            let source = MutableProbe(Mutex::new(Ok(None)));
            let observation = view(&source);
            let mut signal = effect_signal();
            let expected = match case {
                0 => {
                    signal.task_id = TaskId::from_raw("task-other");
                    CancellationObservationError::ScopeMismatch
                }
                1 => {
                    signal.correlation_id = CorrelationId::from_raw("correlation-other");
                    CancellationObservationError::ScopeMismatch
                }
                2 => {
                    signal.schema_version += 1;
                    CancellationObservationError::InvalidSignal
                }
                3 => {
                    signal.cancellation_id = CancellationId::from_raw("");
                    CancellationObservationError::InvalidSignal
                }
                4 => {
                    signal.cancellation_id = CancellationId::from_raw("private\nvalue");
                    CancellationObservationError::InvalidSignal
                }
                _ => {
                    signal.requested_by = BoundaryKind::Model;
                    CancellationObservationError::InvalidSignal
                }
            };
            *source.0.lock().unwrap() = Ok(Some(signal));
            assert_eq!(observation.observe_effect_cancellation(), Err(expected));
            *source.0.lock().unwrap() = Ok(Some(effect_signal()));
            assert_eq!(observation.observe_effect_cancellation(), Err(expected));
        }
    }

    #[test]
    fn borrowed_effect_observation_error_is_redacted_sticky_and_not_cancellation() {
        let source = MutableProbe(Mutex::new(Err(ModelRuntimeFailure {
            code: "private-source-detail".to_owned(),
            retryable_after_correction: true,
            dependency_recovery_required: false,
            contract_error: None,
        })));
        let observation = view(&source);
        assert_eq!(
            observation.observe_effect_cancellation(),
            Err(CancellationObservationError::Unavailable)
        );
        *source.0.lock().unwrap() = Ok(None);
        assert_eq!(
            observation.observe_effect_cancellation(),
            Err(CancellationObservationError::Unavailable)
        );
    }

    #[test]
    fn existing_token_observation_preserves_descendant_tree_and_consumed_scope() {
        let signal = effect_signal();
        let root = CancellationToken::root(
            BoundaryKind::Shell,
            signal.task_id.clone(),
            signal.correlation_id.clone(),
        );
        let tool = root
            .derive_child(BoundaryKind::Kernel)
            .unwrap()
            .derive_child(BoundaryKind::Tool)
            .unwrap();
        assert_eq!(
            observe_effect_cancellation(&tool, &signal.task_id, &signal.correlation_id),
            Ok(None)
        );
        root.cancel(signal.clone()).unwrap();
        assert_eq!(
            observe_effect_cancellation(&tool, &signal.task_id, &signal.correlation_id),
            Ok(Some(signal.clone()))
        );
        assert_eq!(
            observe_effect_cancellation(
                &tool,
                &TaskId::from_raw("other-task"),
                &signal.correlation_id
            ),
            Err(CancellationObservationError::ScopeMismatch)
        );
    }

    #[test]
    fn live_descendant_retains_ancestry_without_retaining_finished_branches() {
        let signal = effect_signal();
        let root = CancellationToken::root(
            BoundaryKind::Shell,
            signal.task_id.clone(),
            signal.correlation_id.clone(),
        );
        let root_node = Arc::downgrade(&root.node);
        let kernel = root.derive_child(BoundaryKind::Kernel).unwrap();
        let kernel_node = Arc::downgrade(&kernel.node);
        let platform = kernel.derive_child(BoundaryKind::PlatformAdapter).unwrap();
        let platform_node = Arc::downgrade(&platform.node);
        let tool = platform.derive_child(BoundaryKind::Tool).unwrap();
        let tool_node = Arc::downgrade(&tool.node);
        let finished = kernel.derive_child(BoundaryKind::Model).unwrap();
        let finished_node = Arc::downgrade(&finished.node);
        drop(finished);
        assert!(finished_node.upgrade().is_none());
        drop(platform);
        drop(kernel);
        assert!(kernel_node.upgrade().is_some());
        assert!(platform_node.upgrade().is_some());
        root.cancel(signal.clone()).unwrap();
        assert_eq!(tool.observe_effect_cancellation(), Ok(Some(signal)));
        drop(root);
        assert!(root_node.upgrade().is_some());
        drop(tool);
        for node in [root_node, kernel_node, platform_node, tool_node] {
            assert!(node.upgrade().is_none());
        }
    }

    #[test]
    fn consumed_scope_rechecks_signals_first_seen_during_effect() {
        // A source view can be internally valid yet bound to another task. The
        // launch-time None must not suppress exact checks on subsequent polls.
        let source = MutableProbe(Mutex::new(Ok(None)));
        let observation = view(&source);
        let signal = effect_signal();
        let consumed_task = TaskId::from_raw("task-consumed-permit");
        let scoped =
            ScopedEffectCancellation::new(&observation, &consumed_task, &signal.correlation_id);
        assert_eq!(scoped.observe_effect_cancellation(), Ok(None));
        *source.0.lock().unwrap() = Ok(Some(signal.clone()));
        assert_eq!(
            scoped.observe_effect_cancellation(),
            Err(CancellationObservationError::ScopeMismatch)
        );
        *source.0.lock().unwrap() = Ok(None);
        assert_eq!(
            scoped.observe_effect_cancellation(),
            Err(CancellationObservationError::ScopeMismatch)
        );
    }

    #[test]
    fn consumed_scope_borrow_preserves_identity_and_original_origin() {
        let source = MutableProbe(Mutex::new(Ok(None)));
        let observation = view(&source);
        let signal = effect_signal();
        let borrowed = &observation;
        let scoped =
            ScopedEffectCancellation::new(&borrowed, &signal.task_id, &signal.correlation_id);
        assert_eq!(scoped.observe_effect_cancellation(), Ok(None));
        *source.0.lock().unwrap() = Ok(Some(signal.clone()));
        assert_eq!(scoped.observe_effect_cancellation(), Ok(Some(signal)));
    }

    #[test]
    fn token_effect_observation_refuses_busy_or_poisoned_owner_without_recovery() {
        let signal = effect_signal();
        let token =
            CancellationToken::root(BoundaryKind::Shell, signal.task_id, signal.correlation_id);
        {
            let _held = token.node.lock().unwrap();
            assert_eq!(
                token.observe_effect_cancellation(),
                Err(CancellationObservationError::Unavailable)
            );
        }
        assert_eq!(token.observe_effect_cancellation(), Ok(None));
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _held = token.node.lock().unwrap();
            panic!("synthetic token poison");
        }));
        assert!(panic.is_err());
        assert_eq!(
            token.observe_effect_cancellation(),
            Err(CancellationObservationError::Unavailable)
        );
        assert!(token.node.is_poisoned());
    }

    #[test]
    fn borrowed_observation_refuses_busy_or_poisoned_cache_before_polling_source() {
        struct NeverCalled;
        impl ModelCancellationProbe for NeverCalled {
            fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
                panic!("a busy or poisoned observation cache must not poll its source");
            }
        }
        let signal = effect_signal();
        let source = NeverCalled;
        let observation =
            BorrowedEffectCancellation::new(&source, signal.task_id, signal.correlation_id);
        {
            let _held = observation.observed.lock().unwrap();
            assert_eq!(
                observation.observe_effect_cancellation(),
                Err(CancellationObservationError::Unavailable)
            );
        }
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _held = observation.observed.lock().unwrap();
            panic!("synthetic observation cache poison");
        }));
        assert!(panic.is_err());
        assert_eq!(
            observation.observe_effect_cancellation(),
            Err(CancellationObservationError::Unavailable)
        );
        assert!(observation.observed.is_poisoned());
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
