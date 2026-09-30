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

    fn run_context_inspections(
        &self,
    ) -> Option<Vec<agentmage_kernel_engine::context_inspection::ContextInspection>> {
        panic!("failed startup cannot show context views")
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

    fn run_context_inspections(
        &self,
    ) -> Option<Vec<agentmage_kernel_engine::context_inspection::ContextInspection>> {
        Some(Vec::new())
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
    };
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
            .expect("the store opens")
            .job_ledgers()
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
    // each client's request under the scope it derived, refuses suspension,
    // resumption and direct cancellation, and keeps every decision durable.
    let (request, events, outcome, observed_result) =
        crate::runtime_read_tests::completed_native_read_fixture();
    assert!(observed_result && events.len() > 1);
    let store = JobLedgerStore::new("owner");
    let ledgers = store.ledgers();
    let (mut service, open_gate, input) =
        gated_live_service(&request, events, outcome.clone(), ledgers.clone());
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
    // suspension or resumption until the host can stop at a safe boundary.
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
    service.release(run, sha).unwrap();
    assert_eq!(
        service.job_status(run, sha),
        Err(RuntimeTransportError::RunUnavailable)
    );
    drop(service);
    // The durable record replays after the store is reopened.
    drop(ledgers);
    let reopened = store.ledgers().observation(run.as_str()).unwrap();
    assert_eq!(reopened, terminal.status.job);
}
