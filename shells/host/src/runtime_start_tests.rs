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

use crate::{
    coding_client::{CodingClientError, CodingCoordinatorPort, LiveCodingCoordinatorPort},
    coding_live_runtime::LiveCodingRuntimeService,
    native_chat_runtime::{NativeChatRuntimeFactory, NativeChatRuntimeService},
    runtime_transport::{RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort},
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
