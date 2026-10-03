//! Service startup failure checks using in-memory requests and non-model factories.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use agentmage_kernel_contracts::{
    ModelCancellationProbe, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEvent,
    RuntimeRunId, RuntimeRunRequest,
};
use agentmage_kernel_engine::{
    runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState},
    runtime_coordinator::seal_runtime_run_request,
    runtime_event::RuntimeEventSubscription,
    runtime_loop::RuntimeCoordinatorStep,
};

use agentmage_kernel_engine::job_control::{
    JobControlAction, JobControlDecision, JobControlRefusal, JobControlRequest, JobPhase,
};

use crate::{
    coding_client::{CodingClientError, CodingCoordinatorPort, LiveCodingCoordinatorPort},
    coding_live_runtime::LiveCodingRuntimeService,
    native_chat_runtime::{NativeChatRuntimeFactory, NativeChatRuntimeService},
    runtime_transport::{
        RuntimeClientScope, RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort,
    },
};

#[derive(Default)]
struct Observed {
    preparations: AtomicUsize,
    compositions: AtomicUsize,
    subscriptions: AtomicUsize,
    advances: AtomicUsize,
    drops: AtomicUsize,
}

#[derive(Clone, Copy)]
enum ServiceKind {
    Native,
    Live,
}

impl ServiceKind {
    const fn missing_start(self) -> RuntimeTransportError {
        match self {
            Self::Native => RuntimeTransportError::RunUnavailable,
            Self::Live => RuntimeTransportError::RequestDenied,
        }
    }
}

struct RefusingFactory {
    template: RuntimeRunRequest,
    observed: Arc<Observed>,
    refuse_composition: bool,
}

impl NativeChatRuntimeFactory for RefusingFactory {
    type Coordinator = RefusingCoordinator;

    fn prepare_runtime_request(
        &mut self,
        _input: &RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        let serial = self.observed.preparations.fetch_add(1, Ordering::SeqCst);
        let mut request = self.template.clone();
        request.run_id = RuntimeRunId::from_raw(format!("startup-failure-{serial}"));
        Ok(seal_runtime_run_request(request).expect("synthetic request seals"))
    }

    fn compose_runtime(
        &mut self,
        _request: &RuntimeRunRequest,
    ) -> Result<Self::Coordinator, RuntimeTransportError> {
        self.observed.compositions.fetch_add(1, Ordering::SeqCst);
        if self.refuse_composition {
            return Err(RuntimeTransportError::RuntimeFailed);
        }
        Ok(RefusingCoordinator(Arc::clone(&self.observed)))
    }
}

struct RefusingCoordinator(Arc<Observed>);

impl Drop for RefusingCoordinator {
    fn drop(&mut self) {
        self.0.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl CodingCoordinatorPort for RefusingCoordinator {
    fn advance(
        &mut self,
        _response: Option<&RuntimeApprovalResponse>,
        _cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        self.0.advances.fetch_add(1, Ordering::SeqCst);
        Err(CodingClientError::Runtime)
    }

    fn runtime_events(&self) -> &[RuntimeEvent] {
        &[]
    }

    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
        &[]
    }
}

impl crate::coding_live_runtime::LiveRunDeclarationPort for RefusingCoordinator {
    fn run_recoverability(
        &self,
        _request: &RuntimeRunRequest,
    ) -> Option<crate::coding_recoverability::RecoverabilityReport> {
        panic!("failed startup cannot declare recoverability")
    }

    fn session_recoverability(
        &self,
        _request: &RuntimeRunRequest,
        _now_epoch_ms: u64,
    ) -> Option<crate::coding_recoverability::RecoverabilityReport> {
        panic!("failed startup cannot declare its session's recoverability")
    }

    fn run_context_inspections(
        &self,
    ) -> Option<Vec<agentmage_kernel_engine::context_inspection::ContextInspection>> {
        panic!("failed startup cannot show context views")
    }

    fn run_action_history(
        &self,
        _request: &RuntimeRunRequest,
    ) -> Option<crate::coding_action_history::RunActionHistory> {
        panic!("failed startup cannot declare an action history")
    }
}

impl LiveCodingCoordinatorPort for RefusingCoordinator {
    fn subscribe_live_events(
        &self,
        _capacity: usize,
    ) -> Result<RuntimeEventSubscription, CodingClientError> {
        self.0.subscriptions.fetch_add(1, Ordering::SeqCst);
        Err(CodingClientError::EventStream)
    }

    fn read_artifact_page(
        &mut self,
        _reference: &RuntimeArtifactRef,
        _offset: u64,
        _maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, CodingClientError> {
        panic!("failed startup cannot read artifacts")
    }

    fn release_artifact(
        &mut self,
        _reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, CodingClientError> {
        panic!("failed startup cannot release artifacts")
    }
}

fn fixture(
    kind: ServiceKind,
    refuse_composition: bool,
) -> (
    Box<dyn RuntimeTransportPort>,
    RuntimePrepareInput,
    Arc<Observed>,
) {
    // This helper runs an in-memory read boundary and fake model, not native effects.
    let (request, _, _, observed_result) =
        crate::runtime_read_tests::completed_native_read_fixture();
    assert!(observed_result);
    let input = RuntimePrepareInput {
        resume: false,
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: None,
        profile_id: request.model_profile.profile_id.as_str().to_owned(),
        expected_entry_sha256: "a".repeat(64),
        workspace_id: request.workspace_id.as_str().to_owned(),
        workspace_root: "/tmp/agentmage-startup-fixture".to_owned(),
        prompt: request.task.objective.clone(),
        recipe: None,
    };
    let observed = Arc::new(Observed::default());
    let factory = RefusingFactory {
        template: request,
        observed: Arc::clone(&observed),
        refuse_composition,
    };
    let service: Box<dyn RuntimeTransportPort> = match kind {
        ServiceKind::Native => Box::new(NativeChatRuntimeService::new(factory)),
        ServiceKind::Live => Box::new(LiveCodingRuntimeService::new(factory)),
    };
    (service, input, observed)
}

fn composition_failure_is_consumed(kind: ServiceKind) {
    let (mut service, input, observed) = fixture(kind, true);
    let request = service.prepare(input).unwrap();
    assert_eq!(
        service.start(request.clone()),
        Err(RuntimeTransportError::RuntimeFailed)
    );
    let duplicate = service.start(request.clone());
    assert_eq!(observed.compositions.load(Ordering::SeqCst), 1);
    assert_eq!(duplicate, Err(kind.missing_start()));
    assert_eq!(
        service.release(&request.run_id, &request.request_sha256),
        Err(RuntimeTransportError::RunUnavailable)
    );
    assert_eq!(observed.advances.load(Ordering::SeqCst), 0);
    assert_eq!(observed.subscriptions.load(Ordering::SeqCst), 0);
}

#[test]
fn native_failed_composition_cannot_reenter_factory() {
    composition_failure_is_consumed(ServiceKind::Native);
}

#[test]
fn live_failed_composition_cannot_reenter_factory() {
    composition_failure_is_consumed(ServiceKind::Live);
}

fn refused_requests_preserve_ticket(kind: ServiceKind) {
    let (mut service, input, observed) = fixture(kind, true);
    let request = service.prepare(input).unwrap();
    let mut malformed = request.clone();
    malformed.task.objective.push_str(" changed");
    assert_eq!(
        service.start(malformed.clone()),
        Err(RuntimeTransportError::RequestDenied)
    );
    let mismatched = seal_runtime_run_request(malformed).unwrap();
    assert_eq!(
        service.start(mismatched),
        Err(RuntimeTransportError::RequestDenied)
    );
    let mut foreign = request.clone();
    foreign.run_id = RuntimeRunId::from_raw("startup-unprepared");
    assert_eq!(
        service.start(seal_runtime_run_request(foreign).unwrap()),
        Err(kind.missing_start())
    );
    assert_eq!(
        service.release(&request.run_id, &"f".repeat(64)),
        Err(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(observed.compositions.load(Ordering::SeqCst), 0);
    assert_eq!(
        service.start(request),
        Err(RuntimeTransportError::RuntimeFailed)
    );
    assert_eq!(observed.compositions.load(Ordering::SeqCst), 1);
}

#[test]
fn native_invalid_or_substituted_start_preserves_exact_preparation() {
    refused_requests_preserve_ticket(ServiceKind::Native);
}

#[test]
fn live_invalid_or_substituted_start_preserves_exact_preparation() {
    refused_requests_preserve_ticket(ServiceKind::Live);
}

fn failed_starts_leave_capacity(kind: ServiceKind) {
    let (mut service, input, observed) = fixture(kind, true);
    for _ in 0..8 {
        let request = service.prepare(input.clone()).unwrap();
        assert_eq!(
            service.start(request),
            Err(RuntimeTransportError::RuntimeFailed)
        );
    }
    assert_eq!(observed.preparations.load(Ordering::SeqCst), 8);
    assert_eq!(observed.compositions.load(Ordering::SeqCst), 8);
}

#[test]
fn native_failed_starts_do_not_retain_prepared_slots() {
    failed_starts_leave_capacity(ServiceKind::Native);
}

#[test]
fn live_failed_starts_do_not_retain_prepared_slots() {
    failed_starts_leave_capacity(ServiceKind::Live);
}

#[test]
fn live_subscription_failure_consumes_ticket_and_drops_owned_coordinator() {
    for probe in [false, true] {
        let (mut service, mut input, observed) = fixture(ServiceKind::Live, false);
        input.slow_subscriber_probe = probe;
        let request = service.prepare(input).unwrap();
        assert_eq!(
            service.start(request.clone()),
            Err(RuntimeTransportError::RuntimeEvidenceDenied)
        );
        assert_eq!(observed.drops.load(Ordering::SeqCst), 1);
        let duplicate = service.start(request);
        assert_eq!(observed.compositions.load(Ordering::SeqCst), 1);
        assert_eq!(observed.subscriptions.load(Ordering::SeqCst), 1);
        assert_eq!(observed.advances.load(Ordering::SeqCst), 0);
        assert_eq!(observed.drops.load(Ordering::SeqCst), 1);
        assert_eq!(duplicate, Err(RuntimeTransportError::RequestDenied));
    }
}

#[test]
fn native_initial_advance_failure_keeps_ticket_consumed_and_drops_coordinator() {
    let (mut service, input, observed) = fixture(ServiceKind::Native, false);
    let request = service.prepare(input).unwrap();
    assert_eq!(
        service.start(request.clone()),
        Err(RuntimeTransportError::RuntimeFailed)
    );
    assert_eq!(observed.drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        service.start(request),
        Err(RuntimeTransportError::RunUnavailable)
    );
    assert_eq!(observed.compositions.load(Ordering::SeqCst), 1);
    assert_eq!(observed.advances.load(Ordering::SeqCst), 1);
    assert_eq!(observed.drops.load(Ordering::SeqCst), 1);
}

#[test]
fn explicit_release_still_discards_unstarted_ticket_without_composition() {
    for kind in [ServiceKind::Native, ServiceKind::Live] {
        let (mut service, input, observed) = fixture(kind, true);
        let request = service.prepare(input).unwrap();
        service
            .release(&request.run_id, &request.request_sha256)
            .unwrap();
        assert_eq!(service.start(request), Err(kind.missing_start()));
        assert_eq!(observed.compositions.load(Ordering::SeqCst), 0);
    }
}

#[cfg(feature = "interactive-cli")]
#[test]
fn cli_start_failure_is_not_replaced_by_failed_best_effort_release() {
    use crate::{
        cli_runtime::{
            InteractiveCliRuntimeError, NeverCancelInteractiveCli, drive_interactive_cli_runtime,
        },
        coding_client::{CodingEventSink, DenyHeadlessApproval},
    };
    struct NoPresentation;
    impl CodingEventSink for NoPresentation {
        fn present(&mut self, _event: &RuntimeEvent) -> Result<(), CodingClientError> {
            panic!("failed composition cannot present completion")
        }
    }
    for kind in [ServiceKind::Native, ServiceKind::Live] {
        let (mut service, input, observed) = fixture(kind, true);
        assert!(matches!(
            drive_interactive_cli_runtime(
                service.as_mut(),
                input,
                &mut DenyHeadlessApproval,
                &mut NoPresentation,
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Runtime)
        ));
        assert_eq!(observed.compositions.load(Ordering::SeqCst), 1);
    }
}

/// Publishes the first event of a completed read fixture, waits for the test
/// to open its gate, then publishes the rest and completes.
struct GatedCoordinator {
    publisher: agentmage_kernel_engine::runtime_event::RuntimeEventPublisher,
    events: Vec<RuntimeEvent>,
    outcome: Option<agentmage_kernel_contracts::RuntimeOutcome>,
    gate: std::sync::mpsc::Receiver<()>,
    declarations: Arc<AtomicUsize>,
    report: crate::coding_recoverability::RecoverabilityReport,
}

impl CodingCoordinatorPort for GatedCoordinator {
    fn advance(
        &mut self,
        _response: Option<&RuntimeApprovalResponse>,
        _cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        let mut events = std::mem::take(&mut self.events).into_iter();
        let first = events.next().ok_or(CodingClientError::Runtime)?;
        self.publisher
            .publish(first)
            .map_err(|_| CodingClientError::EventStream)?;
        self.gate.recv().map_err(|_| CodingClientError::Runtime)?;
        for event in events {
            self.publisher
                .publish(event)
                .map_err(|_| CodingClientError::EventStream)?;
        }
        let outcome = self.outcome.take().ok_or(CodingClientError::Runtime)?;
        Ok(RuntimeCoordinatorStep::Complete { outcome })
    }

    fn runtime_events(&self) -> &[RuntimeEvent] {
        &[]
    }

    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
        &[]
    }
}

impl crate::coding_live_runtime::LiveRunDeclarationPort for GatedCoordinator {
    fn run_recoverability(
        &self,
        _request: &RuntimeRunRequest,
    ) -> Option<crate::coding_recoverability::RecoverabilityReport> {
        self.declarations.fetch_add(1, Ordering::SeqCst);
        Some(self.report.clone())
    }

    fn session_recoverability(
        &self,
        _request: &RuntimeRunRequest,
        _now_epoch_ms: u64,
    ) -> Option<crate::coding_recoverability::RecoverabilityReport> {
        None
    }

    fn run_context_inspections(
        &self,
    ) -> Option<Vec<agentmage_kernel_engine::context_inspection::ContextInspection>> {
        Some(Vec::new())
    }

    fn run_action_history(
        &self,
        _request: &RuntimeRunRequest,
    ) -> Option<crate::coding_action_history::RunActionHistory> {
        crate::coding_action_history::RunActionRecorder::new().declare()
    }
}

impl LiveCodingCoordinatorPort for GatedCoordinator {
    fn subscribe_live_events(
        &self,
        capacity: usize,
    ) -> Result<RuntimeEventSubscription, CodingClientError> {
        self.publisher
            .subscribe(capacity)
            .map_err(|_| CodingClientError::EventStream)
    }

    fn read_artifact_page(
        &mut self,
        _reference: &RuntimeArtifactRef,
        _offset: u64,
        _maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, CodingClientError> {
        Err(CodingClientError::Runtime)
    }

    fn release_artifact(
        &mut self,
        _reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, CodingClientError> {
        Err(CodingClientError::Runtime)
    }
}

struct GatedFactory {
    request: RuntimeRunRequest,
    events: Vec<RuntimeEvent>,
    outcome: agentmage_kernel_contracts::RuntimeOutcome,
    gate: Option<std::sync::mpsc::Receiver<()>>,
    declarations: Arc<AtomicUsize>,
    report: crate::coding_recoverability::RecoverabilityReport,
    job_ledgers: Option<agentmage_kernel_engine::job_ledger_store::DurableJobLedgers>,
    route: Option<crate::coding_route::RunRouteDeclaration>,
    run_histories:
        Option<agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories>,
    recipe_plan: Option<agentmage_kernel_engine::engineering_recipe::RecipePlan>,
    run_effects: Option<crate::coding_session_recoverability::PersistedRunEffects>,
}

impl NativeChatRuntimeFactory for GatedFactory {
    type Coordinator = GatedCoordinator;

    fn prepare_runtime_request(
        &mut self,
        _input: &RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        Ok(self.request.clone())
    }

    fn compose_runtime(
        &mut self,
        _request: &RuntimeRunRequest,
    ) -> Result<Self::Coordinator, RuntimeTransportError> {
        Ok(GatedCoordinator {
            publisher: agentmage_kernel_engine::runtime_event::RuntimeEventPublisher::new(),
            events: self.events.clone(),
            outcome: Some(self.outcome.clone()),
            gate: self
                .gate
                .take()
                .ok_or(RuntimeTransportError::RuntimeFailed)?,
            declarations: Arc::clone(&self.declarations),
            report: self.report.clone(),
        })
    }

    fn take_job_ledgers(
        &mut self,
        _run_id: &RuntimeRunId,
    ) -> Option<agentmage_kernel_engine::job_ledger_store::DurableJobLedgers> {
        self.job_ledgers.take()
    }

    fn take_route_declaration(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Option<crate::coding_route::RunRouteDeclaration> {
        self.route.take().filter(|_| run_id == &self.request.run_id)
    }

    fn take_run_action_histories(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Option<agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories> {
        self.run_histories
            .clone()
            .filter(|_| run_id == &self.request.run_id)
    }

    fn take_recipe_plan(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Option<agentmage_kernel_engine::engineering_recipe::RecipePlan> {
        self.recipe_plan
            .take()
            .filter(|_| run_id == &self.request.run_id)
    }

    fn take_run_effect_records(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Option<crate::coding_session_recoverability::PersistedRunEffects> {
        self.run_effects
            .take()
            .filter(|_| run_id == &self.request.run_id)
    }
}

fn last_cursor(
    step: &crate::runtime_transport::RuntimeTransportStep,
) -> agentmage_kernel_contracts::RuntimeEventCursor {
    let event = step.events.last().expect("a step with events");
    agentmage_kernel_contracts::RuntimeEventCursor {
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        sequence: event.sequence,
        event_sha256: event.event_sha256.clone(),
    }
}

#[test]
fn the_live_service_declares_only_a_held_ended_run_that_is_not_busy() {
    // Review V3 of 8fbd2bc6: the host service refuses declarations for an
    // unknown run, a foreign request digest, and a run that can still advance
    // or whose worker is busy, and returns the coordinator's own afterwards.
    let (request, events, outcome, observed_result) =
        crate::runtime_read_tests::completed_native_read_fixture();
    assert!(observed_result && events.len() > 1);
    let report = crate::coding_recoverability::assess_run_recoverability(
        request.session_id.as_str(),
        request.task.task_id.as_str(),
        request.run_id.as_str(),
        &[],
        &|_| None,
    )
    .unwrap();
    let route =
        crate::coding_route::route_development_run(&request, "contract-test", 5_000).unwrap();
    let (recipe, plan) = crate::coding_recipe::test_recipe(&request.workspace_id, "src", 1);
    let (open_gate, gate) = std::sync::mpsc::channel();
    let declarations = Arc::new(AtomicUsize::new(0));
    let mut service = LiveCodingRuntimeService::new(GatedFactory {
        request: request.clone(),
        events,
        outcome,
        gate: Some(gate),
        declarations: Arc::clone(&declarations),
        report: report.clone(),
        job_ledgers: None,
        route: Some(route.clone()),
        run_histories: None,
        recipe_plan: Some(plan.clone()),
        run_effects: None,
    });
    let input = RuntimePrepareInput {
        resume: false,
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: None,
        profile_id: request.model_profile.profile_id.as_str().to_owned(),
        expected_entry_sha256: "a".repeat(64),
        workspace_id: request.workspace_id.as_str().to_owned(),
        workspace_root: "/tmp/agentmage-declaration-fixture".to_owned(),
        prompt: request.task.objective.clone(),
        recipe: None,
    };
    // Decision 0133: a factory whose request binds no plan cannot serve a
    // run sent with a recipe.
    assert_eq!(
        service.prepare(RuntimePrepareInput {
            recipe: Some(recipe),
            ..input.clone()
        }),
        Err(RuntimeTransportError::RequestDenied)
    );
    let prepared = service.prepare(input).unwrap();
    assert_eq!(prepared, request);
    // Prepared but not started: nothing to declare.
    assert_eq!(
        service.run_declarations(&request.run_id, &request.request_sha256),
        Err(RuntimeTransportError::RunUnavailable)
    );
    let mut step = service.start(request.clone()).unwrap();
    assert!(step.outcome.is_none() && !step.events.is_empty());
    assert_eq!(
        service.run_declarations(
            &RuntimeRunId::from_raw("declaration-unknown-run"),
            &request.request_sha256
        ),
        Err(RuntimeTransportError::RunUnavailable)
    );
    assert_eq!(
        service.run_declarations(&request.run_id, &"f".repeat(64)),
        Err(RuntimeTransportError::RequestDenied)
    );
    // The worker is busy inside the run and the run has no outcome.
    assert_eq!(
        service.run_declarations(&request.run_id, &request.request_sha256),
        Err(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(declarations.load(Ordering::SeqCst), 0);
    open_gate.send(()).unwrap();
    for _ in 0..400 {
        if step.outcome.is_some() {
            break;
        }
        let cursor = last_cursor(&step);
        let next = service
            .advance(
                &request.run_id,
                &request.request_sha256,
                Some(&cursor),
                None,
            )
            .unwrap();
        if !next.events.is_empty() || next.outcome.is_some() {
            let mut events = step.events.clone();
            events.extend(next.events.iter().cloned());
            step = crate::runtime_transport::RuntimeTransportStep { events, ..next };
        }
    }
    assert!(step.outcome.is_some());
    let declared = service
        .run_declarations(&request.run_id, &request.request_sha256)
        .unwrap();
    assert_eq!(declarations.load(Ordering::SeqCst), 1);
    assert_eq!(declared.run_id, request.run_id);
    assert_eq!(declared.request_sha256, request.request_sha256);
    assert_eq!(declared.recoverability, Some(report));
    assert_eq!(declared.context_inspections, Some(Vec::new()));
    // Decision 0127: the tool boundary's history is declared; this run has no
    // job, so it declares no job control history.
    assert_eq!(declared.schema_version, 5);
    // Decision 0143: this fixture's owner declares no session.
    assert_eq!(declared.session_recoverability, None);
    assert_eq!(
        declared.effect_history,
        crate::coding_action_history::RunActionRecorder::new().declare()
    );
    assert_eq!(declared.job_control_history, None);
    // Decision 0128: the route the factory chose for this composition is
    // declared by the service with its history.
    assert_eq!(declared.route_receipt, Some(route.receipt));
    assert_eq!(declared.route_history, route.history);
    // Decision 0133: the plan the factory held the run to is declared with it.
    assert_eq!(declared.recipe_plan, Some(plan));
    service
        .release(&request.run_id, &request.request_sha256)
        .unwrap();
    // A released run can no longer be declared.
    assert_eq!(
        service.run_declarations(&request.run_id, &request.request_sha256),
        Err(RuntimeTransportError::RunUnavailable)
    );
    assert_eq!(declarations.load(Ordering::SeqCst), 1);
}

pub(crate) use job_store::JobLedgerStore;

// This whole file is test-only; the explicit test module marks the store
// directory setup as test code for the effect boundary scan.
#[cfg(test)]
mod job_store {
    use std::path::PathBuf;

    /// A real encrypted operational store in a private temporary directory.
    pub(crate) struct JobLedgerStore {
        directory: PathBuf,
        path: PathBuf,
    }

    struct JobStoreKey;

    impl agentmage_kernel_engine::operational_store::OperationalStoreKeyProvider for JobStoreKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, agentmage_kernel_engine::operational_store::OperationalStoreKeyError>
        {
            Ok(operation(&[23; 32]))
        }
    }

    impl JobLedgerStore {
        pub(crate) fn new(tag: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("agentmage-live-job-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir(&directory).unwrap();
            let path = directory.join("authority.db");
            Self { directory, path }
        }

        pub(crate) fn ledgers(
            &self,
        ) -> agentmage_kernel_engine::job_ledger_store::DurableJobLedgers {
            self.try_ledgers().expect("the store opens")
        }

        /// The job ledgers and run action histories of one open store.
        pub(crate) fn handles(
            &self,
        ) -> (
            agentmage_kernel_engine::job_ledger_store::DurableJobLedgers,
            agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories,
        ) {
            self.try_handles().expect("the store opens")
        }

        /// Opens the store, which fails while another connection holds it.
        pub(crate) fn try_ledgers(
            &self,
        ) -> Option<agentmage_kernel_engine::job_ledger_store::DurableJobLedgers> {
            self.try_handles().map(|(ledgers, _)| ledgers)
        }

        /// Both handles of one open store, or `None` while another
        /// connection holds it.
        pub(crate) fn try_handles(
            &self,
        ) -> Option<(
            agentmage_kernel_engine::job_ledger_store::DurableJobLedgers,
            agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories,
        )> {
            self.try_runtime()
                .map(|runtime| (runtime.job_ledgers(), runtime.run_action_histories()))
        }

        /// The store's runtime, or `None` while another connection holds it.
        pub(crate) fn try_runtime(
            &self,
        ) -> Option<agentmage_kernel_engine::operational_store::DurableAuthorityRuntime> {
            agentmage_kernel_engine::operational_store::DurableAuthorityRuntime::open(
                &self.path,
                &agentmage_kernel_contracts::StrictLocalStorageObservation {
                    filesystem: agentmage_kernel_contracts::StorageFilesystemClass::Local,
                    synchronization_marker: None,
                    root_identity_sha256: [7; 32],
                    symlink_free: true,
                },
                &mut JobStoreKey,
                1,
            )
            .ok()
        }
    }

    impl Drop for JobLedgerStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
}

fn gated_live_service(
    request: &RuntimeRunRequest,
    events: Vec<RuntimeEvent>,
    outcome: agentmage_kernel_contracts::RuntimeOutcome,
    ledgers: agentmage_kernel_engine::job_ledger_store::DurableJobLedgers,
    run_histories: Option<
        agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories,
    >,
    run_effects: Option<crate::coding_session_recoverability::PersistedRunEffects>,
) -> (
    LiveCodingRuntimeService<GatedFactory>,
    std::sync::mpsc::Sender<()>,
    RuntimePrepareInput,
) {
    let report = crate::coding_recoverability::assess_run_recoverability(
        request.session_id.as_str(),
        request.task.task_id.as_str(),
        request.run_id.as_str(),
        &[],
        &|_| None,
    )
    .unwrap();
    let (open_gate, gate) = std::sync::mpsc::channel();
    let service = LiveCodingRuntimeService::new(GatedFactory {
        request: request.clone(),
        events,
        outcome,
        gate: Some(gate),
        declarations: Arc::new(AtomicUsize::new(0)),
        report,
        job_ledgers: Some(ledgers),
        route: None,
        run_histories,
        recipe_plan: None,
        run_effects,
    });
    let input = RuntimePrepareInput {
        resume: request.event_cursor.is_some(),
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: None,
        profile_id: request.model_profile.profile_id.as_str().to_owned(),
        expected_entry_sha256: "a".repeat(64),
        workspace_id: request.workspace_id.as_str().to_owned(),
        workspace_root: "/tmp/agentmage-job-fixture".to_owned(),
        prompt: request.task.objective.clone(),
        recipe: None,
    };
    (service, open_gate, input)
}

fn cancel_request(id: &str, observed_revision: u64, run_id: &RuntimeRunId) -> JobControlRequest {
    JobControlRequest {
        schema_version: 1,
        job_id: run_id.as_str().to_owned(),
        request_id: id.to_owned(),
        action: JobControlAction::Cancel,
        observed_revision,
    }
}

#[test]
fn the_live_service_owns_each_run_job_and_decides_cancellation_through_the_ledger() {
    // Decision 0120: the host records the job's start and outcome, decides
    // each client's request under the scope it derived, refuses direct
    // cancellation, and keeps every decision durable. This run commits no
    // checkpoints, so suspension and resumption are refused before the
    // ledger (Decision 0122).
    let (request, events, outcome, observed_result) =
        crate::runtime_read_tests::completed_native_read_fixture();
    assert!(observed_result && events.len() > 1);
    let store = JobLedgerStore::new("owner");
    let runtime = store.try_runtime().unwrap();
    let (ledgers, histories, effects) = (
        runtime.job_ledgers(),
        runtime.run_action_histories(),
        runtime.run_effect_records(),
    );
    drop(runtime);
    // Review F3 of 6c0f51fe: the factory hands over the run's effect record.
    let chain = crate::coding_session_recoverability::PersistedRunEffects::begin(
        effects.clone(),
        &request,
        crate::coding_action_history::StoredChainStart::New,
    )
    .unwrap();
    let (mut service, open_gate, input) = gated_live_service(
        &request,
        events,
        outcome.clone(),
        ledgers.clone(),
        Some(histories.clone()),
        Some(chain),
    );
    service.prepare(input).unwrap();
    // A prepared run has no job yet.
    assert_eq!(
        service.job_status(&request.run_id, &request.request_sha256),
        Err(RuntimeTransportError::RunUnavailable)
    );
    let mut step = service.start(request.clone()).unwrap();
    assert!(step.outcome.is_none());
    let running = service
        .job_status(&request.run_id, &request.request_sha256)
        .unwrap();
    assert!(running.describes(&request));
    assert_eq!(
        (running.job.phase, running.job.revision),
        (JobPhase::Running, 1)
    );
    assert_eq!(
        service.job_status(&request.run_id, &"f".repeat(64)),
        Err(RuntimeTransportError::RequestDenied)
    );

    let client = RuntimeClientScope::derived("peer-client-a".to_owned());
    let other = RuntimeClientScope::derived("peer-client-b".to_owned());
    let run = &request.run_id;
    let sha = request.request_sha256.as_str();
    // No authenticated client in process, no direct cancellation, and no
    // suspension or resumption of a run that commits no checkpoints.
    assert_eq!(
        service.control_job(run, sha, &cancel_request("c0", 1, run)),
        Err(RuntimeTransportError::JobControlUnavailable)
    );
    assert_eq!(
        service.cancel(
            run,
            sha,
            agentmage_kernel_contracts::CancellationId::from_raw("direct-cancel"),
            None
        ),
        Err(RuntimeTransportError::RequestDenied)
    );
    for action in [JobControlAction::Suspend, JobControlAction::Resume] {
        let mut suspend = cancel_request("s0", 1, run);
        suspend.action = action;
        assert_eq!(
            service.control_job_for_client(&client, run, sha, &suspend),
            Err(RuntimeTransportError::RequestDenied)
        );
    }
    assert_eq!(service.job_status(run, sha).unwrap(), running);

    // A stale request is refused and recorded; the current one applies once.
    let stale = service
        .control_job_for_client(&client, run, sha, &cancel_request("c1", 0, run))
        .unwrap();
    assert!(stale.answers(&request, &cancel_request("c1", 0, run)));
    assert_eq!(
        stale.decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::StaleRevision,
            revision: 1,
            phase: JobPhase::Running,
        }
    );
    let applied = service
        .control_job_for_client(&client, run, sha, &cancel_request("c2", 1, run))
        .unwrap();
    assert_eq!(
        applied.decision,
        JobControlDecision::Applied {
            revision: 2,
            phase: JobPhase::Cancelling,
        }
    );
    assert!(applied.status.job.cancellation_requested);
    // A retry from the same client gets its original decision; the same
    // identity from another client is that client's own request.
    assert_eq!(
        service
            .control_job_for_client(&client, run, sha, &cancel_request("c2", 1, run))
            .unwrap()
            .decision,
        applied.decision
    );
    assert!(matches!(
        service
            .control_job_for_client(&other, run, sha, &cancel_request("c2", 1, run))
            .unwrap()
            .decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::StaleRevision,
            ..
        }
    ));
    assert!(matches!(
        service
            .control_job_for_client(&other, run, sha, &cancel_request("c3", 2, run))
            .unwrap()
            .decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::AlreadyInEffect,
            ..
        }
    ));
    assert_eq!(
        service
            .control_job_for_client(&client, run, sha, &cancel_request("c2", 2, run))
            .err(),
        Some(RuntimeTransportError::RequestDenied)
    );

    // The work finishes before it reaches a safe boundary: the finished state
    // stands and the accepted request stays in the ledger.
    open_gate.send(()).unwrap();
    for _ in 0..400 {
        if step.outcome.is_some() {
            break;
        }
        let cursor = last_cursor(&step);
        let next = service.advance(run, sha, Some(&cursor), None).unwrap();
        if !next.events.is_empty() || next.outcome.is_some() {
            let mut events = step.events.clone();
            events.extend(next.events.iter().cloned());
            step = crate::runtime_transport::RuntimeTransportStep { events, ..next };
        }
    }
    assert_eq!(step.outcome, Some(outcome));
    let ended = service.job_status(run, sha).unwrap();
    assert_eq!(
        (ended.job.phase, ended.job.revision),
        (JobPhase::Completed, 3)
    );
    assert!(ended.job.cancellation_requested);
    let terminal = service
        .control_job_for_client(&client, run, sha, &cancel_request("c4", 3, run))
        .unwrap();
    assert!(matches!(
        terminal.decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::Terminal,
            revision: 3,
            ..
        }
    ));
    // The refusal is recorded, so the head moves while the revision stays.
    assert_eq!(terminal.status.job.revision, 3);
    assert_ne!(terminal.status.job.head_sha256, ended.job.head_sha256);
    // Decision 0127: every request the ledger decided has one entry, in
    // order, under the scope that sent it; the retry and the conflicting
    // request add nothing, and the last entry names the ledger's last head.
    let declared = service.run_declarations(run, sha).unwrap();
    let history = declared.job_control_history.expect("job control history");
    let replayed = crate::coding_action_history::verify_run_action_history(
        &history,
        crate::coding_action_history::RunActionChain::JobControl,
    )
    .unwrap();
    let kept = replayed
        .records()
        .iter()
        .map(|record| match record {
            agentmage_kernel_engine::action_history::ActionHistoryRecord::Kept(entry) => (
                entry.action_id.clone(),
                entry.outcome,
                entry.reason_code.clone(),
                entry.evidence_sha256s.clone(),
            ),
            agentmage_kernel_engine::action_history::ActionHistoryRecord::Expired { .. } => {
                panic!("nothing expires within a run")
            }
        })
        .collect::<Vec<_>>();
    use agentmage_kernel_engine::action_history::ActionOutcome;
    assert_eq!(
        kept.iter()
            .map(|(id, outcome, reason, _)| (id.as_str(), *outcome, reason.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                "c1",
                ActionOutcome::Denied,
                "job.control.cancel.stale-revision"
            ),
            ("c2", ActionOutcome::Succeeded, "job.control.cancel.applied"),
            (
                "c2",
                ActionOutcome::Denied,
                "job.control.cancel.stale-revision"
            ),
            (
                "c3",
                ActionOutcome::Denied,
                "job.control.cancel.already-in-effect"
            ),
            ("c4", ActionOutcome::Denied, "job.control.cancel.terminal"),
        ]
    );
    assert_eq!(kept[1].3, vec![applied.status.job.head_sha256.clone()]);
    assert_eq!(kept[4].3, vec![terminal.status.job.head_sha256.clone()]);
    // Decision 0129: each entry was also stored, and the chain stays open
    // until the run is released.
    use agentmage_kernel_engine::run_action_history_store::RunActionChainName;
    let stored = histories
        .history(run.as_str(), RunActionChainName::JobControl)
        .unwrap();
    assert_eq!(stored.records, history.records);
    assert_eq!(stored.head, history.head);
    assert!(stored.complete && !stored.closed);
    let effect_record = effects.run(run.as_str()).unwrap();
    assert!(effect_record.complete && !effect_record.closed);
    // The catalog host, not the development host, reads ended runs
    // (Decision 0130) and keeps route grants (Decision 0144).
    assert_eq!(
        service.ended_run_action_histories(run),
        Err(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        service.route_grant(crate::coding_route_grants::RouteGrantRequest::List {
            workspace_id: None
        }),
        Err(RuntimeTransportError::RequestDenied)
    );
    service.release(run, sha).unwrap();
    assert_eq!(
        service.job_status(run, sha),
        Err(RuntimeTransportError::RunUnavailable)
    );
    assert_eq!(
        service.ended_run_action_histories(run),
        Err(RuntimeTransportError::RequestDenied)
    );
    // Released: the chains are closed, the effect record with them, and the
    // ended run's stored histories are read back from the store as the
    // catalog host reads them.
    let effect_record = effects.run(run.as_str()).unwrap();
    assert!(effect_record.complete && effect_record.closed);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap())
        .unwrap();
    let ended = crate::coding_action_history::read_ended_run_action_histories(
        &histories,
        run.as_str(),
        now,
    )
    .unwrap();
    let job_control = ended.job_control.clone().expect("stored job control chain");
    assert!(job_control.closed && job_control.complete);
    assert_eq!(job_control.records, history.records);
    assert_eq!((ended.effects.clone(), ended.routes.clone()), (None, None));
    assert_eq!(
        crate::coding_action_history::verified_ended_run_histories(ended.clone(), run.as_str()),
        Some(ended)
    );
    drop(service);
    // The durable record replays after the store is reopened.
    drop((ledgers, histories, effects));
    let (ledgers, histories) = store.handles();
    let reopened = ledgers.observation(run.as_str()).unwrap();
    assert_eq!(reopened, terminal.status.job);
    let stored = histories
        .history(run.as_str(), RunActionChainName::JobControl)
        .unwrap();
    assert!(stored.closed);
    assert_eq!(stored.records, history.records);
}

#[test]
fn the_live_service_closes_the_effect_record_of_a_held_ended_run_and_of_a_new_run_it_cannot_start()
{
    use crate::coding_action_history::StoredChainStart;
    use crate::coding_session_recoverability::PersistedRunEffects;
    use agentmage_kernel_engine::run_action_history_store::RunActionChainName;
    let (request, events, outcome, _) = crate::runtime_read_tests::completed_native_read_fixture();
    let run = &request.run_id;
    let sha = request.request_sha256.as_str();

    // Review F3 of 6c0f51fe: an ended run the service still holds when it is
    // dropped has its effect record closed.
    let store = JobLedgerStore::new("held-ended-effects");
    let runtime = store.try_runtime().unwrap();
    let (ledgers, histories, effects) = (
        runtime.job_ledgers(),
        runtime.run_action_histories(),
        runtime.run_effect_records(),
    );
    drop(runtime);
    let chain =
        PersistedRunEffects::begin(effects.clone(), &request, StoredChainStart::New).unwrap();
    let (mut service, open_gate, input) = gated_live_service(
        &request,
        events.clone(),
        outcome.clone(),
        ledgers,
        Some(histories),
        Some(chain),
    );
    service.prepare(input).unwrap();
    let mut step = service.start(request.clone()).unwrap();
    open_gate.send(()).unwrap();
    for _ in 0..400 {
        if step.outcome.is_some() {
            break;
        }
        let cursor = last_cursor(&step);
        let next = service.advance(run, sha, Some(&cursor), None).unwrap();
        if !next.events.is_empty() || next.outcome.is_some() {
            let mut events = step.events.clone();
            events.extend(next.events.iter().cloned());
            step = crate::runtime_transport::RuntimeTransportStep { events, ..next };
        }
    }
    assert_eq!(step.outcome, Some(outcome.clone()));
    assert!(!effects.run(run.as_str()).unwrap().closed);
    drop(service);
    let stored = effects.run(run.as_str()).unwrap();
    assert!(stored.complete && stored.closed);
    drop((effects, store));

    // Review F5 of 6c0f51fe: a new run whose job cannot begin ran nothing,
    // so its effect record and job control chain close empty.
    let store = JobLedgerStore::new("unstarted-effects");
    let runtime = store.try_runtime().unwrap();
    let (ledgers, histories, effects) = (
        runtime.job_ledgers(),
        runtime.run_action_histories(),
        runtime.run_effect_records(),
    );
    drop(runtime);
    ledgers
        .create(
            run.as_str(),
            crate::coding_live_runtime::CODING_HOST_JOB_OWNER,
        )
        .unwrap();
    let chain =
        PersistedRunEffects::begin(effects.clone(), &request, StoredChainStart::New).unwrap();
    let (mut service, _open_gate, input) = gated_live_service(
        &request,
        events,
        outcome,
        ledgers,
        Some(histories.clone()),
        Some(chain),
    );
    service.prepare(input).unwrap();
    assert!(service.start(request.clone()).is_err());
    let stored = effects.run(run.as_str()).unwrap();
    assert!(stored.complete && stored.closed && stored.entries.is_empty());
    let job_control = histories
        .history(run.as_str(), RunActionChainName::JobControl)
        .unwrap();
    assert!(job_control.closed && job_control.records.is_empty());
    drop(service);
}

/// The completed native read fixture as a controlled-write run that committed
/// one checkpoint after its first turn (Decision 0122). Every event is sealed
/// again in its new position, so each stream verifies.
pub(crate) struct SuspendedRunFixture {
    /// The run's request.
    pub(crate) request: RuntimeRunRequest,
    /// Events through the checkpoint's commit event.
    pub(crate) stopped: Vec<RuntimeEvent>,
    /// Where the run stops for a suspension.
    pub(crate) point: agentmage_kernel_engine::runtime_loop::RuntimeSuspensionPoint,
    /// The fixture's remaining events before its terminal event.
    tail: Vec<RuntimeEvent>,
    /// The fixture's outcome, sealed again for each ending.
    outcome: agentmage_kernel_contracts::RuntimeOutcome,
}

impl SuspendedRunFixture {
    pub(crate) fn new() -> Self {
        use agentmage_kernel_contracts::{
            BudgetLimit, BudgetResource, RuntimeEventId, RuntimeEventKind, RuntimeSessionMode,
            SessionCheckpointId,
        };
        use agentmage_kernel_engine::runtime_event::RuntimeEventSequence;
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let mut request = request;
        request.mode = RuntimeSessionMode::ControlledWrite;
        // A controlled write's work packet covers each run ceiling.
        let limits = request.limits.clone();
        let mut budgets = request
            .work_packet
            .budgets
            .iter()
            .map(|budget| (budget.resource, budget.limit))
            .collect::<std::collections::BTreeMap<_, _>>();
        for (resource, minimum) in [
            (BudgetResource::PlanSteps, u64::from(limits.max_turns)),
            (
                BudgetResource::ToolCallDepth,
                u64::from(limits.max_tool_call_depth),
            ),
            (
                BudgetResource::ModelCalls,
                u64::from(limits.max_model_calls),
            ),
            (BudgetResource::ToolCalls, u64::from(limits.max_tool_calls)),
            (
                BudgetResource::InputBytes,
                request.context_budget.max_input_bytes * u64::from(limits.max_context_refreshes),
            ),
            (BudgetResource::OutputBytes, limits.max_output_bytes),
            (BudgetResource::ElapsedMilliseconds, limits.max_elapsed_ms),
            (BudgetResource::MemoryBytes, 1),
            (BudgetResource::DiskBytes, limits.max_output_bytes),
            (BudgetResource::ProcessCount, 1),
        ] {
            let limit = budgets.entry(resource).or_insert(1);
            *limit = (*limit).max(minimum).max(1);
        }
        request.work_packet.budgets = budgets
            .into_iter()
            .map(|(resource, limit)| BudgetLimit { resource, limit })
            .collect();
        let request = seal_runtime_run_request(request).unwrap();
        let mut events = events;
        let terminal = events.pop().unwrap();
        assert!(matches!(
            terminal.kind,
            RuntimeEventKind::RunTerminal { .. }
        ));
        if let RuntimeEventKind::RunStarted { request_sha256 } = &mut events[0].kind {
            request_sha256.clone_from(&request.request_sha256);
        }
        // The checkpoint follows the first completed turn.
        let split = events
            .iter()
            .position(|event| matches!(event.kind, RuntimeEventKind::TurnCompleted { .. }))
            .unwrap();
        assert!(split + 1 < events.len(), "a turn follows the checkpoint");
        let tail = events.split_off(split + 1);
        let mut stopped = rechain(events, None);
        let last = stopped.last().unwrap().clone();
        let checkpoint_id = SessionCheckpointId::from_raw("checkpoint-suspension-fixture");
        let checkpoint_sha256 = "5".repeat(64);
        let mut checkpoint = template_event(
            &last,
            RuntimeEventKind::CheckpointCommitted {
                checkpoint_id: checkpoint_id.clone(),
                checkpoint_sha256: checkpoint_sha256.clone(),
            },
        );
        checkpoint.event_id = RuntimeEventId::from_raw("event-checkpoint-suspension-fixture");
        stopped.extend(rechain(vec![checkpoint], Some(&last)));
        let mut sequence = RuntimeEventSequence::new();
        for event in &stopped {
            sequence.push(event).unwrap();
        }
        let committed = stopped.last().unwrap();
        let point = agentmage_kernel_engine::runtime_loop::RuntimeSuspensionPoint {
            checkpoint_id,
            checkpoint_sha256,
            event_cursor: agentmage_kernel_contracts::RuntimeEventCursor {
                run_id: committed.run_id.clone(),
                event_id: committed.event_id.clone(),
                sequence: committed.sequence,
                event_sha256: committed.event_sha256.clone(),
            },
        };
        Self {
            request,
            stopped,
            point,
            tail,
            outcome,
        }
    }

    /// The request that continues the run from the checkpoint.
    pub(crate) fn resumed(&self) -> RuntimeRunRequest {
        crate::runtime_transport::resumed_run_request(&self.request, &self.point.event_cursor)
            .unwrap()
    }

    /// The rest of the run after the checkpoint under `request`, and its
    /// outcome: the fixture's own ending, or a cancellation observed at once.
    pub(crate) fn ending(
        &self,
        request: &RuntimeRunRequest,
        cancellation: Option<&agentmage_kernel_contracts::CancellationId>,
    ) -> (
        Vec<RuntimeEvent>,
        agentmage_kernel_contracts::RuntimeOutcome,
    ) {
        use agentmage_kernel_contracts::{AgentStateKind, RuntimeEventKind};
        let checkpoint = self.stopped.last().unwrap();
        let (mut events, state) = match cancellation {
            Some(cancellation_id) => {
                let requested = template_event(
                    checkpoint,
                    RuntimeEventKind::CancellationRequested {
                        cancellation_id: cancellation_id.clone(),
                    },
                );
                let mut observed = template_event(
                    checkpoint,
                    RuntimeEventKind::CancellationObserved {
                        cancellation_id: cancellation_id.clone(),
                    },
                );
                observed.event_id = agentmage_kernel_contracts::RuntimeEventId::from_raw(
                    "event-cancellation-observed-fixture",
                );
                let mut requested = requested;
                requested.event_id = agentmage_kernel_contracts::RuntimeEventId::from_raw(
                    "event-cancellation-requested-fixture",
                );
                (
                    rechain(vec![requested, observed], Some(checkpoint)),
                    AgentStateKind::Cancelled,
                )
            }
            None => (
                rechain(self.tail.clone(), Some(checkpoint)),
                self.outcome.state,
            ),
        };
        let prior = events.last().unwrap().clone();
        let mut outcome = self.outcome.clone();
        outcome.request_sha256.clone_from(&request.request_sha256);
        outcome.state = state;
        outcome.prior_event_id = prior.event_id.clone();
        outcome.prior_event_sha256.clone_from(&prior.event_sha256);
        if cancellation.is_some() {
            outcome.output = None;
            outcome.answer_evidence = None;
            outcome.unresolved_codes = vec!["runtime.cancelled".to_owned()];
        }
        let outcome =
            agentmage_kernel_engine::runtime_coordinator::seal_runtime_outcome(outcome, request)
                .unwrap();
        let mut terminal = template_event(
            &prior,
            RuntimeEventKind::RunTerminal {
                state,
                outcome_sha256: outcome.outcome_sha256.clone(),
            },
        );
        terminal.event_id =
            agentmage_kernel_contracts::RuntimeEventId::from_raw("event-terminal-fixture");
        events.extend(rechain(vec![terminal], Some(&prior)));
        (events, outcome)
    }
}

/// An event after `prior` outside any turn, with this kind.
fn template_event(
    prior: &RuntimeEvent,
    kind: agentmage_kernel_contracts::RuntimeEventKind,
) -> RuntimeEvent {
    let mut event = prior.clone();
    event.turn_id = None;
    event.operation_id = None;
    event.payload_reference = None;
    event.causation_event_id = Some(prior.event_id.clone());
    event.persistence = agentmage_kernel_engine::runtime_event::runtime_event_persistence(&kind);
    event.kind = kind;
    event
}

/// Seals `events` again in order after `prior`, or from the stream's start.
fn rechain(events: Vec<RuntimeEvent>, prior: Option<&RuntimeEvent>) -> Vec<RuntimeEvent> {
    let mut previous = prior.map_or_else(
        || ("0".repeat(64), 0),
        |event| (event.event_sha256.clone(), event.sequence + 1),
    );
    events
        .into_iter()
        .map(|mut event| {
            event.sequence = previous.1;
            event.previous_event_sha256 = previous.0.clone();
            let event = agentmage_kernel_engine::runtime_event::seal_runtime_event(event).unwrap();
            previous = (event.event_sha256.clone(), event.sequence + 1);
            event
        })
        .collect()
}

/// Stands in for a durable coordinator over the suspended-run fixture. A fresh
/// composition publishes the run's first event, waits for the test's gate,
/// publishes the rest through the checkpoint and then stops for a claimed
/// suspension, ends at once for an observed cancellation, or runs to the
/// fixture's ending. A composition resumed from the checkpoint holds the
/// stopped events as its history and continues from there.
struct CheckpointingCoordinator {
    publisher: agentmage_kernel_engine::runtime_event::RuntimeEventPublisher,
    request: RuntimeRunRequest,
    fixture: Arc<SuspendedRunFixture>,
    history: Vec<RuntimeEvent>,
    gate: Option<std::sync::mpsc::Receiver<()>>,
    /// Holds the worker after it committed to stopping, until the test lets
    /// it return.
    after_claim: Option<std::sync::mpsc::Receiver<()>>,
    /// Stops without asking the suspension probe.
    suspend_unasked: bool,
    consulted: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
    advanced: bool,
}

impl Drop for CheckpointingCoordinator {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

impl CheckpointingCoordinator {
    fn end(
        &mut self,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        let signal = cancellation
            .map(ModelCancellationProbe::observe)
            .transpose()
            .map_err(|_| CodingClientError::Runtime)?
            .flatten();
        let (events, outcome) = self.fixture.ending(
            &self.request,
            signal.as_ref().map(|signal| &signal.cancellation_id),
        );
        for event in events {
            self.publisher
                .publish(event)
                .map_err(|_| CodingClientError::EventStream)?;
        }
        Ok(RuntimeCoordinatorStep::Complete { outcome })
    }
}

impl CodingCoordinatorPort for CheckpointingCoordinator {
    fn advance(
        &mut self,
        _response: Option<&RuntimeApprovalResponse>,
        _cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
        Err(CodingClientError::Runtime)
    }

    fn runtime_events(&self) -> &[RuntimeEvent] {
        &self.history
    }

    fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
        &[]
    }
}

impl crate::coding_live_runtime::LiveRunDeclarationPort for CheckpointingCoordinator {
    fn run_recoverability(
        &self,
        _request: &RuntimeRunRequest,
    ) -> Option<crate::coding_recoverability::RecoverabilityReport> {
        None
    }

    fn session_recoverability(
        &self,
        _request: &RuntimeRunRequest,
        _now_epoch_ms: u64,
    ) -> Option<crate::coding_recoverability::RecoverabilityReport> {
        None
    }

    fn run_context_inspections(
        &self,
    ) -> Option<Vec<agentmage_kernel_engine::context_inspection::ContextInspection>> {
        None
    }

    fn run_action_history(
        &self,
        _request: &RuntimeRunRequest,
    ) -> Option<crate::coding_action_history::RunActionHistory> {
        None
    }
}

impl LiveCodingCoordinatorPort for CheckpointingCoordinator {
    fn subscribe_live_events(
        &self,
        capacity: usize,
    ) -> Result<RuntimeEventSubscription, CodingClientError> {
        self.publisher
            .subscribe(capacity)
            .map_err(|_| CodingClientError::EventStream)
    }

    fn read_artifact_page(
        &mut self,
        _reference: &RuntimeArtifactRef,
        _offset: u64,
        _maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, CodingClientError> {
        Err(CodingClientError::Runtime)
    }

    fn release_artifact(
        &mut self,
        _reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, CodingClientError> {
        Err(CodingClientError::Runtime)
    }

    fn advance_or_suspend(
        &mut self,
        _response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
        suspension: &dyn agentmage_kernel_engine::runtime_loop::RuntimeSuspensionProbe,
    ) -> Result<agentmage_kernel_engine::runtime_loop::RuntimeSuspendableStep, CodingClientError>
    {
        use agentmage_kernel_engine::runtime_loop::RuntimeSuspendableStep;
        if std::mem::replace(&mut self.advanced, true) {
            return Err(CodingClientError::Runtime);
        }
        if self.request.event_cursor.is_none() {
            let mut stopped = self.fixture.stopped.clone().into_iter();
            self.publisher
                .publish(stopped.next().ok_or(CodingClientError::Runtime)?)
                .map_err(|_| CodingClientError::EventStream)?;
            self.gate
                .take()
                .ok_or(CodingClientError::Runtime)?
                .recv()
                .map_err(|_| CodingClientError::Runtime)?;
            for event in stopped {
                self.publisher
                    .publish(event)
                    .map_err(|_| CodingClientError::EventStream)?;
            }
            if self.suspend_unasked {
                return Ok(RuntimeSuspendableStep::Suspended {
                    point: self.fixture.point.clone(),
                });
            }
            // As the engine does: a cancellation at the boundary wins.
            let cancelled = cancellation
                .map(ModelCancellationProbe::observe)
                .transpose()
                .map_err(|_| CodingClientError::Runtime)?
                .flatten()
                .is_some();
            if !cancelled {
                let claimed = suspension.suspend_at_safe_boundary();
                // Counted after the answer: a test that sees the count knows
                // the worker has already committed to it.
                self.consulted.fetch_add(1, Ordering::SeqCst);
                if claimed {
                    if let Some(release) = self.after_claim.take() {
                        release.recv().map_err(|_| CodingClientError::Runtime)?;
                    }
                    return Ok(RuntimeSuspendableStep::Suspended {
                        point: self.fixture.point.clone(),
                    });
                }
            }
        }
        self.end(cancellation).map(RuntimeSuspendableStep::Boundary)
    }

    fn suspendable(&self) -> bool {
        true
    }
}

/// Composes the fixture's run and its continuation from the checkpoint,
/// opening a fresh store handle for each composition as the development host
/// does, and handing over that store's job ledgers and run action histories.
/// Preparing a continuation first opens the store, which proves the service
/// closed every handle of its own, the job control recorder's included.
struct CheckpointingFactory {
    fixture: Arc<SuspendedRunFixture>,
    store: Arc<JobLedgerStore>,
    /// Prepares the continuation from the checkpoint, as a host started
    /// again after the one that held the run ended.
    restarted: bool,
    gate: Option<std::sync::mpsc::Receiver<()>>,
    after_claim: Option<std::sync::mpsc::Receiver<()>>,
    suspend_unasked: bool,
    /// Restores a history other than the presented one.
    tamper_history: bool,
    /// Prepares a request bound to another cursor.
    foreign_resumed: bool,
    composed: Vec<RuntimeRunRequest>,
    resumptions: Vec<(
        RuntimeRunRequest,
        agentmage_kernel_contracts::RuntimeEventCursor,
    )>,
    consulted: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
    /// How many compositions opened the store.
    opened: usize,
    ledgers: Option<agentmage_kernel_engine::job_ledger_store::DurableJobLedgers>,
    run_histories:
        Option<agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories>,
    /// Begins each composition's effect record; the first composition's
    /// store writes are refused while the flag is set, and it records one
    /// entry (review F1 of `6c0f51fe`).
    refusing_effects: Option<Arc<std::sync::atomic::AtomicBool>>,
    effect_chain: Option<crate::coding_session_recoverability::PersistedRunEffects>,
}

impl CheckpointingFactory {
    fn new(
        fixture: &Arc<SuspendedRunFixture>,
        store: &Arc<JobLedgerStore>,
        gate: Option<std::sync::mpsc::Receiver<()>>,
        consulted: &Arc<AtomicUsize>,
        dropped: &Arc<AtomicUsize>,
    ) -> Self {
        Self {
            fixture: Arc::clone(fixture),
            store: Arc::clone(store),
            restarted: false,
            gate,
            after_claim: None,
            suspend_unasked: false,
            tamper_history: false,
            foreign_resumed: false,
            composed: Vec::new(),
            resumptions: Vec::new(),
            consulted: Arc::clone(consulted),
            dropped: Arc::clone(dropped),
            opened: 0,
            ledgers: None,
            run_histories: None,
            refusing_effects: None,
            effect_chain: None,
        }
    }
}

impl NativeChatRuntimeFactory for CheckpointingFactory {
    type Coordinator = CheckpointingCoordinator;

    fn prepare_runtime_request(
        &mut self,
        _input: &RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        Ok(if self.restarted {
            self.fixture.resumed()
        } else {
            self.fixture.request.clone()
        })
    }

    fn compose_runtime(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Self::Coordinator, RuntimeTransportError> {
        let resumed = request.event_cursor.is_some();
        if resumed && *request != self.fixture.resumed() && !self.foreign_resumed
            || !resumed && *request != self.fixture.request
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.composed.push(request.clone());
        let runtime = self
            .store
            .try_runtime()
            .ok_or(RuntimeTransportError::RuntimeFailed)?;
        self.opened += 1;
        self.ledgers = Some(runtime.job_ledgers());
        self.run_histories = Some(runtime.run_action_histories());
        if let Some(refuse) = &self.refusing_effects {
            use crate::coding_action_history::StoredChainStart;
            let start = match (resumed, self.restarted) {
                (false, _) => StoredChainStart::New,
                (true, true) => StoredChainStart::AfterRestart,
                (true, false) => StoredChainStart::ContinueInHost,
            };
            let chain = crate::coding_session_recoverability::PersistedRunEffects::begin(
                runtime.run_effect_records(),
                request,
                start,
            )
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
            self.effect_chain = Some(if resumed {
                chain
            } else {
                let chain = chain.refusing_writes(refuse);
                chain.append(&crate::coding_session_recoverability::execution_entry(
                    &crate::coding_recoverability::ExecutedEffect {
                        operation_id: "refused-effect".to_owned(),
                        kind: crate::coding_recoverability::ExecutedEffectKind::Command,
                        outcome: crate::coding_recoverability::ExecutedEffectOutcome::Unknown,
                    },
                ));
                chain
            });
        }
        // As the engine does when it restores a checkpoint, the history is
        // published before any subscriber attaches.
        let publisher = agentmage_kernel_engine::runtime_event::RuntimeEventPublisher::new();
        if resumed {
            let shown = self.fixture.stopped.len() - usize::from(self.tamper_history);
            for event in &self.fixture.stopped[..shown] {
                publisher
                    .publish(event.clone())
                    .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
            }
        }
        Ok(CheckpointingCoordinator {
            publisher,
            request: request.clone(),
            fixture: Arc::clone(&self.fixture),
            history: if resumed {
                let mut history = self.fixture.stopped.clone();
                if self.tamper_history {
                    history.pop();
                }
                history
            } else {
                Vec::new()
            },
            gate: if resumed { None } else { self.gate.take() },
            after_claim: self.after_claim.take(),
            suspend_unasked: self.suspend_unasked,
            consulted: Arc::clone(&self.consulted),
            dropped: Arc::clone(&self.dropped),
            advanced: false,
        })
    }

    fn take_job_ledgers(
        &mut self,
        _run_id: &RuntimeRunId,
    ) -> Option<agentmage_kernel_engine::job_ledger_store::DurableJobLedgers> {
        self.ledgers.take()
    }

    fn take_run_action_histories(
        &mut self,
        _run_id: &RuntimeRunId,
    ) -> Option<agentmage_kernel_engine::run_action_history_store::DurableRunActionHistories> {
        self.run_histories.take()
    }

    fn take_run_effect_records(
        &mut self,
        _run_id: &RuntimeRunId,
    ) -> Option<crate::coding_session_recoverability::PersistedRunEffects> {
        self.effect_chain.take()
    }

    fn prepare_in_host_resume(
        &mut self,
        request: &RuntimeRunRequest,
        cursor: &agentmage_kernel_contracts::RuntimeEventCursor,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        // The store admits one connection: this open fails while the service
        // still holds the suspended run's handle.
        drop(
            self.store
                .try_ledgers()
                .ok_or(RuntimeTransportError::RuntimeFailed)?,
        );
        self.resumptions.push((request.clone(), cursor.clone()));
        if self.foreign_resumed {
            let mut other = cursor.clone();
            other.sequence -= 1;
            return crate::runtime_transport::resumed_run_request(request, &other);
        }
        Ok(self.fixture.resumed())
    }
}

/// The preparation of the fixture's run, or of its continuation after a
/// restart when `request` was resumed from an event cursor.
fn suspension_prepare_input(request: &RuntimeRunRequest) -> RuntimePrepareInput {
    RuntimePrepareInput {
        resume: request.event_cursor.is_some(),
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: None,
        profile_id: request.model_profile.profile_id.as_str().to_owned(),
        expected_entry_sha256: "a".repeat(64),
        workspace_id: request.workspace_id.as_str().to_owned(),
        workspace_root: "/tmp/agentmage-suspension-fixture".to_owned(),
        prompt: request.task.objective.clone(),
        recipe: None,
    }
}

struct SuspensionHarness {
    service: LiveCodingRuntimeService<CheckpointingFactory>,
    fixture: Arc<SuspendedRunFixture>,
    store: Arc<JobLedgerStore>,
    open_gate: std::sync::mpsc::Sender<()>,
    consulted: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
    client: RuntimeClientScope,
    step: crate::runtime_transport::RuntimeTransportStep,
}

impl SuspensionHarness {
    /// Starts the fixture's run; its job is running at revision 1.
    fn start(tag: &str) -> Self {
        Self::start_with(tag, |_| {})
    }

    fn start_with(tag: &str, configure: impl FnOnce(&mut CheckpointingFactory)) -> Self {
        let fixture = Arc::new(SuspendedRunFixture::new());
        let store = Arc::new(JobLedgerStore::new(tag));
        let (open_gate, gate) = std::sync::mpsc::channel();
        let consulted = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut factory =
            CheckpointingFactory::new(&fixture, &store, Some(gate), &consulted, &dropped);
        configure(&mut factory);
        let mut service = LiveCodingRuntimeService::new(factory);
        let request = &fixture.request;
        service.prepare(suspension_prepare_input(request)).unwrap();
        let step = service.start(request.clone()).unwrap();
        assert!(step.outcome.is_none() && step.suspended.is_none());
        let status = service
            .job_status(&request.run_id, &request.request_sha256)
            .unwrap();
        assert_eq!(
            (status.job.phase, status.job.revision),
            (JobPhase::Running, 1)
        );
        Self {
            service,
            fixture,
            store,
            open_gate,
            consulted,
            dropped,
            client: RuntimeClientScope::derived("peer-suspension-client".to_owned()),
            step,
        }
    }

    fn control(
        &mut self,
        request: &RuntimeRunRequest,
        id: &str,
        action: JobControlAction,
        observed_revision: u64,
    ) -> Result<crate::runtime_transport::RuntimeJobControl, RuntimeTransportError> {
        let mut control = cancel_request(id, observed_revision, &request.run_id);
        control.action = action;
        self.service.control_job_for_client(
            &self.client,
            &request.run_id,
            &request.request_sha256,
            &control,
        )
    }

    /// Advances under `request` until the run is suspended or has its outcome,
    /// keeping every presented event.
    fn advance_until_stopped(&mut self, request: &RuntimeRunRequest) {
        for _ in 0..400 {
            if self.step.outcome.is_some() || self.step.suspended.is_some() {
                return;
            }
            let cursor = last_cursor(&self.step);
            let next = self
                .service
                .advance(
                    &request.run_id,
                    &request.request_sha256,
                    Some(&cursor),
                    None,
                )
                .unwrap();
            let mut events = self.step.events.clone();
            events.extend(next.events.iter().cloned());
            self.step = crate::runtime_transport::RuntimeTransportStep { events, ..next };
        }
        panic!("the run neither stopped nor ended");
    }

    fn factory(&self) -> &CheckpointingFactory {
        self.service.factory_for_tests()
    }
}

#[test]
fn an_applied_suspension_stops_at_the_checkpoint_and_a_resumption_continues_it_in_host() {
    // Decision 0122: the run stops at its committed boundary, the owner's
    // observation is recorded, the composition is released, and a resumption
    // continues the same run from the checkpoint under the cursor-bound
    // request, with the store closed while the continuation is prepared.
    let mut harness = SuspensionHarness::start("suspend-resume");
    let request = harness.fixture.request.clone();
    let resumed = harness.fixture.resumed();
    let (run, sha) = (&request.run_id, request.request_sha256.as_str());
    let suspend = harness
        .control(&request, "s1", JobControlAction::Suspend, 1)
        .unwrap();
    assert_eq!(
        suspend.decision,
        JobControlDecision::Applied {
            revision: 2,
            phase: JobPhase::Suspending,
        }
    );
    assert!(suspend.status.describes(&request));
    harness.open_gate.send(()).unwrap();
    harness.advance_until_stopped(&request);
    assert!(harness.step.outcome.is_none());
    assert_eq!(harness.step.suspended, Some(harness.fixture.point.clone()));
    assert_eq!(harness.step.events, harness.fixture.stopped);
    assert_eq!(harness.consulted.load(Ordering::SeqCst), 1);
    // The owner observed the suspension and released the composition.
    let suspended = harness.service.job_status(run, sha).unwrap();
    assert_eq!(
        (suspended.job.phase, suspended.job.revision),
        (JobPhase::Suspended, 3)
    );
    assert_eq!(harness.dropped.load(Ordering::SeqCst), 1);
    // Nothing runs, ends or is released while the run is suspended.
    let cursor = last_cursor(&harness.step);
    let idle = harness
        .service
        .advance(run, sha, Some(&cursor), None)
        .unwrap();
    assert!(idle.events.is_empty() && idle.outcome.is_none());
    assert_eq!(idle.suspended, Some(harness.fixture.point.clone()));
    assert_eq!(
        harness.service.release(run, sha),
        Err(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        harness.service.run_declarations(run, sha),
        Err(RuntimeTransportError::RequestDenied)
    );
    // A repeated suspension is already in effect and changes nothing.
    let again = harness
        .control(&request, "s2", JobControlAction::Suspend, 3)
        .unwrap();
    assert!(matches!(
        again.decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::AlreadyInEffect,
            ..
        }
    ));
    assert!(harness.factory().resumptions.is_empty());

    // The resumption queues the job; the host continues the run from the
    // checkpoint under the cursor-bound request and starts the job again.
    let resume = harness
        .control(&request, "r1", JobControlAction::Resume, 3)
        .unwrap();
    assert_eq!(
        resume.decision,
        JobControlDecision::Applied {
            revision: 4,
            phase: JobPhase::Queued,
        }
    );
    assert!(resume.status.describes(&resumed));
    assert!(!resume.status.describes(&request));
    // The job was started again (revision 5); the fixture's ending may
    // already have completed it (revision 6).
    assert!(matches!(
        (resume.status.job.phase, resume.status.job.revision),
        (JobPhase::Running, 5) | (JobPhase::Completed, 6)
    ));
    assert_eq!(
        harness.factory().resumptions,
        [(request.clone(), harness.fixture.point.event_cursor.clone())]
    );
    assert_eq!(
        harness.factory().composed,
        [request.clone(), resumed.clone()]
    );
    // Each composition opened the store afresh (Decision 0129).
    assert_eq!(harness.factory().opened, 2);
    // The run is now held under the resumed request only.
    assert_eq!(
        harness.service.advance(run, sha, Some(&cursor), None).err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    harness.step.suspended = None;
    harness.advance_until_stopped(&resumed);
    let (ending, outcome) = harness.fixture.ending(&resumed, None);
    assert_eq!(harness.step.outcome, Some(outcome));
    assert!(harness.step.suspended.is_none());
    assert_eq!(
        &harness.step.events[harness.fixture.stopped.len()..],
        ending.as_slice()
    );
    let ended = harness
        .service
        .job_status(run, &resumed.request_sha256)
        .unwrap();
    assert_eq!(
        (ended.job.phase, ended.job.revision),
        (JobPhase::Completed, 6)
    );
    // A resumed run declares nothing its composition kept (Decisions 0117 and
    // 0127); the service decided every control request, so its job control
    // history continues across the suspension.
    let declared = harness
        .service
        .run_declarations(run, &resumed.request_sha256)
        .unwrap();
    assert!(declared.recoverability.is_none() && declared.context_inspections.is_none());
    assert!(declared.effect_history.is_none());
    let history = declared.job_control_history.expect("job control history");
    crate::coding_action_history::verify_run_action_history(
        &history,
        crate::coding_action_history::RunActionChain::JobControl,
    )
    .unwrap();
    let reasons = history
        .records
        .iter()
        .map(|record| match record {
            agentmage_kernel_engine::action_history::ActionHistoryRecord::Kept(entry) => {
                (entry.action_id.as_str(), entry.reason_code.as_str())
            }
            agentmage_kernel_engine::action_history::ActionHistoryRecord::Expired { .. } => {
                panic!("nothing expires within a run")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        [
            ("s1", "job.control.suspend.applied"),
            ("s2", "job.control.suspend.already-in-effect"),
            ("r1", "job.control.resume.applied"),
        ]
    );
    // A request decided after the outcome is stored through the
    // continuation's own handle (review F2 of `bc1e2d39`).
    let terminal = harness
        .control(&resumed, "c9", JobControlAction::Cancel, 6)
        .unwrap();
    assert!(matches!(
        terminal.decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::Terminal,
            ..
        }
    ));
    harness
        .service
        .release(run, &resumed.request_sha256)
        .unwrap();
    assert_eq!(harness.dropped.load(Ordering::SeqCst), 2);
    let store = Arc::clone(&harness.store);
    drop(harness);
    let (ledgers, histories) = store.handles();
    let reopened = ledgers.observation(run.as_str()).unwrap();
    assert_eq!(reopened, terminal.status.job);
    assert_eq!(reopened.revision, ended.job.revision);
    // Decision 0129: the job control chain stayed open and complete across
    // the suspension and the continuation in this host, holds every decided
    // request, and closed when the ended run was released.
    let stored = histories
        .history(
            run.as_str(),
            agentmage_kernel_engine::run_action_history_store::RunActionChainName::JobControl,
        )
        .unwrap();
    assert!(stored.complete && stored.closed);
    assert_eq!(stored.records.len(), 4);
    assert_eq!(stored.records[..3], history.records[..]);
}

#[test]
fn a_miss_the_store_could_not_mark_carries_across_an_in_host_continuation() {
    // Review F1 of 6c0f51fe (Decision 0145): the run's first composition
    // misses an entry and the store refuses its mark. The continuation in
    // this host reopens the store with a new handle, which continues from
    // what the host knew and stores the mark, so the released chain closes
    // as incomplete.
    let refuse = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut harness = SuspensionHarness::start_with("suspend-effect-miss", |factory| {
        factory.refusing_effects = Some(Arc::clone(&refuse));
    });
    let request = harness.fixture.request.clone();
    let resumed = harness.fixture.resumed();
    harness
        .control(&request, "s1", JobControlAction::Suspend, 1)
        .unwrap();
    harness.open_gate.send(()).unwrap();
    harness.advance_until_stopped(&request);
    assert!(harness.step.suspended.is_some());
    harness
        .control(&request, "r1", JobControlAction::Resume, 3)
        .unwrap();
    harness.step.suspended = None;
    harness.advance_until_stopped(&resumed);
    assert!(harness.step.outcome.is_some());
    harness
        .service
        .release(&request.run_id, &resumed.request_sha256)
        .unwrap();
    let store = Arc::clone(&harness.store);
    drop(harness);
    let stored = store
        .try_runtime()
        .unwrap()
        .run_effect_records()
        .run(request.run_id.as_str())
        .unwrap();
    assert!(!stored.complete && stored.closed);
    assert!(stored.entries.is_empty());
}

#[test]
fn a_suspended_run_whose_host_ends_keeps_its_stored_chain_open() {
    // Review F2 of `bc1e2d39` (Decision 0129): only an ended run's chains
    // close when its session is dropped. A run suspended when its host ends
    // keeps its job control chain open and complete.
    let mut harness = SuspensionHarness::start("suspend-host-ends");
    let request = harness.fixture.request.clone();
    harness
        .control(&request, "s1", JobControlAction::Suspend, 1)
        .unwrap();
    harness.open_gate.send(()).unwrap();
    harness.advance_until_stopped(&request);
    assert!(harness.step.suspended.is_some());
    let store = Arc::clone(&harness.store);
    drop(harness);
    let (ledgers, histories) = store.handles();
    assert_eq!(
        ledgers.observation(request.run_id.as_str()).unwrap().phase,
        JobPhase::Suspended
    );
    let stored = histories
        .history(
            request.run_id.as_str(),
            agentmage_kernel_engine::run_action_history_store::RunActionChainName::JobControl,
        )
        .unwrap();
    assert!(stored.complete && !stored.closed);
    assert_eq!(stored.records.len(), 1);
}

#[test]
fn a_run_resumed_after_a_restart_marks_its_stored_job_control_chain_incomplete() {
    // Review F2 of `bc1e2d39` (Decision 0129): the host that ended may have
    // missed an entry, so the same run started again from an event cursor
    // attaches to its open job control chain marked incomplete, and declares
    // no job control history.
    use agentmage_kernel_engine::run_action_history_store::RunActionChainName;
    let mut harness = SuspensionHarness::start("restart-chain");
    let request = harness.fixture.request.clone();
    let resumed = harness.fixture.resumed();
    let run = &request.run_id;
    // One decided request is stored before the host ends with the run
    // still running.
    let stale = harness
        .control(&request, "c0", JobControlAction::Cancel, 0)
        .unwrap();
    assert!(matches!(
        stale.decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::StaleRevision,
            ..
        }
    ));
    let (fixture, store) = (Arc::clone(&harness.fixture), Arc::clone(&harness.store));
    let (consulted, dropped) = (Arc::clone(&harness.consulted), Arc::clone(&harness.dropped));
    drop(harness);
    {
        let (ledgers, histories) = store.handles();
        assert_eq!(
            ledgers.observation(run.as_str()).unwrap().phase,
            JobPhase::Running
        );
        let stored = histories
            .history(run.as_str(), RunActionChainName::JobControl)
            .unwrap();
        assert!(stored.complete && !stored.closed);
        assert_eq!(stored.records.len(), 1);
    }

    // A new host continues the running job from the checkpoint.
    let mut factory = CheckpointingFactory::new(&fixture, &store, None, &consulted, &dropped);
    factory.restarted = true;
    let mut service = LiveCodingRuntimeService::new(factory);
    service.prepare(suspension_prepare_input(&resumed)).unwrap();
    let mut step = service.start(resumed.clone()).unwrap();
    for _ in 0..400 {
        if step.outcome.is_some() {
            break;
        }
        let cursor = last_cursor(&step);
        let next = service
            .advance(run, &resumed.request_sha256, Some(&cursor), None)
            .unwrap();
        let mut events = step.events.clone();
        events.extend(next.events.iter().cloned());
        step = crate::runtime_transport::RuntimeTransportStep { events, ..next };
    }
    assert_eq!(
        step.outcome.as_ref().map(|outcome| outcome.state),
        Some(agentmage_kernel_contracts::AgentStateKind::Success)
    );
    let declared = service
        .run_declarations(run, &resumed.request_sha256)
        .unwrap();
    assert_eq!(declared.job_control_history, None);
    service.release(run, &resumed.request_sha256).unwrap();
    drop(service);
    let (_, histories) = store.handles();
    let stored = histories
        .history(run.as_str(), RunActionChainName::JobControl)
        .unwrap();
    assert!(!stored.complete && stored.closed);
    assert_eq!(stored.records.len(), 1);
}

#[test]
fn a_cancellation_of_a_suspended_run_ends_it_from_its_checkpoint() {
    // Decision 0122: a stop the worker already committed to is recorded
    // before any later request is decided, and the cancellation of a
    // suspended job ends the run at once with its own cancelled outcome.
    let (release, after_claim) = std::sync::mpsc::channel();
    let mut harness = SuspensionHarness::start_with("suspend-cancel", |factory| {
        factory.after_claim = Some(after_claim);
    });
    let request = harness.fixture.request.clone();
    let resumed = harness.fixture.resumed();
    harness
        .control(&request, "s1", JobControlAction::Suspend, 1)
        .unwrap();
    harness.open_gate.send(()).unwrap();
    // Wait until the worker has committed to stopping at the boundary, without
    // asking the service about it. The worker then holds its stop back until
    // shortly after the next request reaches the service.
    for _ in 0..400 {
        if harness.consulted.load(Ordering::SeqCst) == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(harness.consulted.load(Ordering::SeqCst), 1);
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        release.send(()).unwrap();
    });
    // A cancellation built on the suspending revision arrives after the stop:
    // the suspension is recorded first, so it is stale.
    let stale = harness
        .control(&request, "c1", JobControlAction::Cancel, 2)
        .unwrap();
    assert_eq!(
        stale.decision,
        JobControlDecision::Refused {
            refusal: JobControlRefusal::StaleRevision,
            revision: 3,
            phase: JobPhase::Suspended,
        }
    );
    assert!(stale.status.describes(&request));
    releaser.join().unwrap();
    let cancelled = harness
        .control(&request, "c2", JobControlAction::Cancel, 3)
        .unwrap();
    assert_eq!(
        cancelled.decision,
        JobControlDecision::Applied {
            revision: 4,
            phase: JobPhase::Cancelled,
        }
    );
    assert!(cancelled.status.describes(&resumed));
    assert_eq!(cancelled.status.job.phase, JobPhase::Cancelled);
    harness.step.suspended = None;
    harness.step.events = harness.fixture.stopped.clone();
    harness.advance_until_stopped(&resumed);
    let outcome = harness.step.outcome.clone().unwrap();
    assert_eq!(
        outcome.state,
        agentmage_kernel_contracts::AgentStateKind::Cancelled
    );
    // The ledger's decision stands; the run's own outcome is not recorded again.
    let ended = harness
        .service
        .job_status(&request.run_id, &resumed.request_sha256)
        .unwrap();
    assert_eq!(
        (ended.job.phase, ended.job.revision),
        (JobPhase::Cancelled, 4)
    );
    harness
        .service
        .release(&request.run_id, &resumed.request_sha256)
        .unwrap();
}

#[test]
fn a_withdrawn_or_cancelled_suspension_never_stops_the_run() {
    // Decision 0122: a resumption before the boundary withdraws the pending
    // suspension, and a cancellation supersedes it; the worker then passes
    // the boundary.
    for (tag, action, phase, state, revision) in [
        (
            "suspend-withdraw",
            JobControlAction::Resume,
            JobPhase::Running,
            agentmage_kernel_contracts::AgentStateKind::Success,
            (JobPhase::Completed, 4),
        ),
        (
            "suspend-then-cancel",
            JobControlAction::Cancel,
            JobPhase::Cancelling,
            agentmage_kernel_contracts::AgentStateKind::Cancelled,
            (JobPhase::Cancelled, 4),
        ),
    ] {
        let mut harness = SuspensionHarness::start(tag);
        let request = harness.fixture.request.clone();
        harness
            .control(&request, "s1", JobControlAction::Suspend, 1)
            .unwrap();
        let answer = harness.control(&request, "x1", action, 2).unwrap();
        assert_eq!(
            answer.decision,
            JobControlDecision::Applied { revision: 3, phase }
        );
        harness.open_gate.send(()).unwrap();
        harness.advance_until_stopped(&request);
        assert!(harness.step.suspended.is_none());
        assert_eq!(harness.step.outcome.as_ref().unwrap().state, state);
        // The worker consults the suspension only when no cancellation won.
        assert_eq!(
            harness.consulted.load(Ordering::SeqCst),
            usize::from(action == JobControlAction::Resume)
        );
        let ended = harness
            .service
            .job_status(&request.run_id, &request.request_sha256)
            .unwrap();
        assert_eq!((ended.job.phase, ended.job.revision), revision);
        assert!(harness.factory().resumptions.is_empty());
        harness
            .service
            .release(&request.run_id, &request.request_sha256)
            .unwrap();
    }
}

#[test]
fn a_stop_without_a_request_or_a_continuation_that_differs_is_refused() {
    // Decision 0122: the host accepts a stop only for the suspension its
    // worker claimed, and continues a run only under the request it derived
    // and with exactly the history it presented.
    let mut harness = SuspensionHarness::start_with("suspend-unasked", |factory| {
        factory.suspend_unasked = true;
    });
    let request = harness.fixture.request.clone();
    harness.open_gate.send(()).unwrap();
    let cursor = last_cursor(&harness.step);
    let mut refused = None;
    for _ in 0..40 {
        match harness.service.advance(
            &request.run_id,
            &request.request_sha256,
            Some(&cursor),
            None,
        ) {
            Ok(step) => assert!(step.suspended.is_none()),
            Err(error) => {
                refused = Some(error);
                break;
            }
        }
    }
    assert_eq!(refused, Some(RuntimeTransportError::RuntimeEvidenceDenied));
    let status = harness
        .service
        .job_status(&request.run_id, &request.request_sha256)
        .unwrap();
    assert_eq!(status.job.phase, JobPhase::Running);

    for (tag, tamper_history, foreign_resumed) in [
        ("resume-tampered-history", true, false),
        ("resume-foreign-request", false, true),
    ] {
        let mut harness = SuspensionHarness::start_with(tag, |factory| {
            factory.tamper_history = tamper_history;
            factory.foreign_resumed = foreign_resumed;
        });
        let request = harness.fixture.request.clone();
        harness
            .control(&request, "s1", JobControlAction::Suspend, 1)
            .unwrap();
        harness.open_gate.send(()).unwrap();
        harness.advance_until_stopped(&request);
        assert!(harness.step.suspended.is_some());
        let refused = harness.control(&request, "r1", JobControlAction::Resume, 3);
        assert_eq!(
            refused.err(),
            Some(if tamper_history {
                RuntimeTransportError::RuntimeEvidenceDenied
            } else {
                RuntimeTransportError::RequestDenied
            })
        );
        // The run is no longer held, and the job keeps the phase the ledger
        // decided, so a run resumed after a restart can start it again.
        assert_eq!(
            harness
                .service
                .job_status(&request.run_id, &request.request_sha256)
                .err(),
            Some(RuntimeTransportError::RunUnavailable)
        );
        let store = Arc::clone(&harness.store);
        drop(harness);
        let job = store
            .ledgers()
            .observation(request.run_id.as_str())
            .unwrap();
        assert_eq!((job.phase, job.revision), (JobPhase::Queued, 4));
    }
}

#[test]
fn a_persisted_recorder_keeps_each_entry_in_memory_and_in_the_store() {
    // Decision 0129: a persisted recorder keeps each entry in memory, then in
    // its run's stored chain; an entry either could not keep leaves both
    // incomplete; a chain joins as new, continued in this host or resumed
    // after a restart, which marks it incomplete.
    use crate::coding_action_history::{
        PersistedRunChain, RunActionChain, RunActionRecorder, StoredChainStart,
    };
    use agentmage_kernel_engine::action_history::{
        ActionAuthorization, ActionKind, ActionOutcome, ActionRecordDraft,
    };
    use agentmage_kernel_engine::run_action_history_store::{
        RunActionChainName, RunActionHistoryStoreError,
    };
    let draft = |id: &str, at: u64| ActionRecordDraft {
        action_kind: ActionKind::CommandRun,
        action_id: id.to_owned(),
        authorization: ActionAuthorization::Grant {
            grant_id: format!("grant-{id}"),
            grant_sha256: "7".repeat(64),
        },
        effect_sha256: "8".repeat(64),
        outcome: ActionOutcome::Succeeded,
        reason_code: "coding.approved.succeeded".to_owned(),
        evidence_sha256s: vec!["9".repeat(64)],
        recorded_at_epoch_ms: at,
        retain_until_epoch_ms: at + 1_000,
    };
    let store = JobLedgerStore::new("persisted-recorder");
    let (_, histories) = store.handles();
    let begin = |start| {
        PersistedRunChain::begin(
            histories.clone(),
            "run-persisted",
            RunActionChain::Effects,
            start,
        )
    };
    // A run continued in this host never attaches to a missing chain.
    assert_eq!(
        begin(StoredChainStart::ContinueInHost).err(),
        Some(RunActionHistoryStoreError::NotFound)
    );
    // A run resumed after a restart whose chain the ended host never stored
    // (review N4 of `bc1e2d39`) begins it marked incomplete, whatever the
    // chain's owner stores first.
    for (run, start) in [
        ("run-before-22-effects", StoredChainStart::AfterRestart),
        (
            "run-before-22-routes",
            StoredChainStart::AfterRestartStoredFirst,
        ),
    ] {
        let mut resumed = RunActionRecorder::persisted(
            PersistedRunChain::begin(histories.clone(), run, RunActionChain::Effects, start)
                .unwrap(),
        );
        resumed.record(Some(draft("command-after-restart", 10)));
        let stored = histories.history(run, RunActionChainName::Effects).unwrap();
        assert!(!stored.complete && !stored.closed);
        assert_eq!(stored.head.count, 1);
    }
    let mut recorder = RunActionRecorder::persisted(begin(StoredChainStart::New).unwrap());
    assert!(recorder.is_persisted());
    assert_eq!(
        begin(StoredChainStart::New).err(),
        Some(RunActionHistoryStoreError::Exists)
    );
    recorder.record(Some(draft("command-1", 10)));
    recorder.record(Some(draft("command-2", 20)));
    let declared = recorder.declare().expect("complete in memory");
    let stored = histories
        .history("run-persisted", RunActionChainName::Effects)
        .unwrap();
    assert_eq!(stored.records, declared.records);
    assert_eq!(stored.head, declared.head);
    assert!(stored.complete);
    // Continuing in this host keeps the chain complete and appends after it.
    let mut continued =
        RunActionRecorder::persisted(begin(StoredChainStart::ContinueInHost).unwrap());
    continued.record(Some(draft("command-3", 30)));
    let stored = histories
        .history("run-persisted", RunActionChainName::Effects)
        .unwrap();
    assert_eq!(stored.head.count, 3);
    assert!(stored.complete);
    // An entry neither could keep (time backwards) leaves both incomplete.
    recorder.record(Some(draft("command-4", 5)));
    assert_eq!(recorder.declare(), None);
    assert!(
        !histories
            .history("run-persisted", RunActionChainName::Effects)
            .unwrap()
            .complete
    );
    // A store refusal of an entry memory kept (the chain was closed) leaves
    // both incomplete too.
    let mut refused = RunActionRecorder::persisted(
        PersistedRunChain::begin(
            histories.clone(),
            "run-refused",
            RunActionChain::Effects,
            StoredChainStart::New,
        )
        .unwrap(),
    );
    refused.record(Some(draft("command-1", 10)));
    histories
        .close(
            "run-refused",
            RunActionChainName::Effects,
            crate::coding_action_history::RUN_ACTION_HISTORY_OWNER,
        )
        .unwrap();
    refused.record(Some(draft("command-2", 20)));
    assert_eq!(refused.declare(), None);
    // An owner that knows it missed an entry marks the stored chain too.
    let mut marked = RunActionRecorder::persisted(
        PersistedRunChain::begin(
            histories.clone(),
            "run-marked",
            RunActionChain::Routes,
            StoredChainStart::New,
        )
        .unwrap(),
    );
    marked.mark_incomplete();
    assert_eq!(marked.declare(), None);
    assert!(
        !histories
            .history("run-marked", RunActionChainName::Routes)
            .unwrap()
            .complete
    );
    // A detached recorder keeps its memory history and stops appending.
    let mut detached = RunActionRecorder::persisted(
        PersistedRunChain::begin(
            histories.clone(),
            "run-detached",
            RunActionChain::Routes,
            StoredChainStart::New,
        )
        .unwrap(),
    );
    detached.detach_store();
    assert!(!detached.is_persisted());
    detached.record(Some(draft("route-1", 10)));
    assert_eq!(detached.declare().unwrap().records.len(), 1);
    assert_eq!(
        histories
            .history("run-detached", RunActionChainName::Routes)
            .unwrap()
            .head
            .count,
        0
    );
    // Resuming after a restart marks the chain incomplete for good; a chain
    // whose owner stores each entry first stays complete.
    for (chain, start, complete) in [
        (
            RunActionChain::JobControl,
            StoredChainStart::AfterRestart,
            false,
        ),
        (
            RunActionChain::Routes,
            StoredChainStart::AfterRestartStoredFirst,
            true,
        ),
    ] {
        let created = PersistedRunChain::begin(
            histories.clone(),
            "run-restarted",
            chain,
            StoredChainStart::New,
        )
        .unwrap();
        drop(created);
        PersistedRunChain::begin(histories.clone(), "run-restarted", chain, start).unwrap();
        assert_eq!(
            histories
                .history("run-restarted", chain.stored())
                .unwrap()
                .complete,
            complete
        );
    }
    // The host keeps every chain under the identity of its job ledgers.
    assert_eq!(
        crate::coding_action_history::RUN_ACTION_HISTORY_OWNER,
        crate::coding_live_runtime::CODING_HOST_JOB_OWNER
    );
}
