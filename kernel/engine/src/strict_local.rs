//! Deterministic strict-local policy with no network execution authority.

use agentmage_kernel_contracts::{
    LocalEndpointIdentity, LocalTransport, NetworkComponent, NetworkDestinationClass,
    NetworkObservation, StorageFilesystemClass, StrictLocalStorageObservation,
};
use sha2::{Digest, Sha256};

/// Maximum content-free network attempts retained by one in-memory ledger.
pub const MAX_NETWORK_ATTEMPT_RECORDS: usize = 4096;

/// Exact reason a strict-local attempt was admitted or denied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrictLocalDecisionReason {
    /// Exact authenticated local inference endpoint and client matched.
    ExactAuthenticatedInferenceEndpoint,
    /// Component has no local inference client authority.
    ClientNotAuthorized,
    /// Destination is not the exact declared local class.
    DestinationNotLocal,
    /// Transport differs from the declared endpoint.
    TransportMismatch,
    /// Local peer authentication was absent.
    PeerNotAuthenticated,
    /// Endpoint identity was absent or changed.
    EndpointIdentityMismatch,
}

/// Strict-local policy result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrictLocalNetworkDecision {
    /// One exact guarded local inference attempt is admitted.
    Allow {
        /// Exact successful reason.
        reason: StrictLocalDecisionReason,
    },
    /// Attempt remains blocked without contacting its destination.
    Block {
        /// First exact denial reason.
        reason: StrictLocalDecisionReason,
    },
}

/// One exact normal-operation endpoint policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrictLocalNetworkPolicy {
    endpoint: LocalEndpointIdentity,
}

impl StrictLocalNetworkPolicy {
    /// Creates a strict-local policy for one already admitted endpoint identity.
    #[must_use]
    pub const fn new(endpoint: LocalEndpointIdentity) -> Self {
        Self { endpoint }
    }

    /// Returns the sole exact endpoint admitted by this policy.
    #[must_use]
    pub const fn endpoint(&self) -> &LocalEndpointIdentity {
        &self.endpoint
    }

    /// Evaluates one content-free observation without opening or contacting anything.
    #[must_use]
    pub fn evaluate(&self, observation: &NetworkObservation) -> StrictLocalNetworkDecision {
        let reason = if observation.component != self.endpoint.client() {
            Some(StrictLocalDecisionReason::ClientNotAuthorized)
        } else if !destination_matches(self.endpoint.transport(), observation.destination) {
            Some(StrictLocalDecisionReason::DestinationNotLocal)
        } else if observation.transport != Some(self.endpoint.transport()) {
            Some(StrictLocalDecisionReason::TransportMismatch)
        } else if !observation.peer_authenticated {
            Some(StrictLocalDecisionReason::PeerNotAuthenticated)
        } else if observation.endpoint_sha256.as_ref() != Some(self.endpoint.identity_sha256()) {
            Some(StrictLocalDecisionReason::EndpointIdentityMismatch)
        } else {
            None
        };
        match reason {
            Some(reason) => StrictLocalNetworkDecision::Block { reason },
            None => StrictLocalNetworkDecision::Allow {
                reason: StrictLocalDecisionReason::ExactAuthenticatedInferenceEndpoint,
            },
        }
    }
}

const fn destination_matches(
    transport: LocalTransport,
    destination: NetworkDestinationClass,
) -> bool {
    matches!(
        (transport, destination),
        (
            LocalTransport::AuthenticatedUnixSocket,
            NetworkDestinationClass::AuthenticatedLocalSocket
        ) | (
            LocalTransport::GuardedLoopbackTcp,
            NetworkDestinationClass::Loopback
        )
    )
}

/// One bounded content-free attempt record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkAttemptRecord {
    sequence: u64,
    observed_at_millis: u64,
    executable_sha256: [u8; 32],
    component: NetworkComponent,
    destination: NetworkDestinationClass,
    transport: Option<LocalTransport>,
    endpoint_sha256: Option<[u8; 32]>,
    attempted_bytes: u64,
    decision: StrictLocalNetworkDecision,
}

impl NetworkAttemptRecord {
    /// Returns the append sequence.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns the caller-supplied monotonic observation time.
    #[must_use]
    pub const fn observed_at_millis(&self) -> u64 {
        self.observed_at_millis
    }

    /// Returns the attributed executable content digest.
    #[must_use]
    pub const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }

    /// Returns the attributed component.
    #[must_use]
    pub const fn component(&self) -> NetworkComponent {
        self.component
    }

    /// Returns only the destination class.
    #[must_use]
    pub const fn destination(&self) -> NetworkDestinationClass {
        self.destination
    }

    /// Returns the local transport class, if any.
    #[must_use]
    pub const fn transport(&self) -> Option<LocalTransport> {
        self.transport
    }

    /// Returns the content-free observed endpoint digest.
    #[must_use]
    pub const fn endpoint_sha256(&self) -> Option<&[u8; 32]> {
        self.endpoint_sha256.as_ref()
    }

    /// Returns the attempted byte count.
    #[must_use]
    pub const fn attempted_bytes(&self) -> u64 {
        self.attempted_bytes
    }

    /// Returns the policy-computed decision.
    #[must_use]
    pub const fn decision(&self) -> StrictLocalNetworkDecision {
        self.decision
    }
}

/// Attempt-ledger append failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAttemptLedgerError {
    /// The platform did not establish an executable identity.
    InvalidExecutableIdentity,
    /// The closed retention bound has been reached.
    CapacityExceeded,
    /// Observation time moved backward.
    NonMonotonicTime,
    /// Sequence allocation overflowed.
    SequenceOverflow,
}

/// Bounded append-only in-memory network-attempt ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkAttemptLedger {
    records: Vec<NetworkAttemptRecord>,
    next_sequence: u64,
}

impl NetworkAttemptLedger {
    /// Creates an empty content-free ledger.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            records: Vec::new(),
            next_sequence: 0,
        }
    }

    /// Evaluates and appends one attempt without storing a raw destination or payload.
    pub fn evaluate_and_record(
        &mut self,
        policy: &StrictLocalNetworkPolicy,
        observation: &NetworkObservation,
        executable_sha256: [u8; 32],
        observed_at_millis: u64,
    ) -> Result<&NetworkAttemptRecord, NetworkAttemptLedgerError> {
        if executable_sha256 == [0; 32] {
            return Err(NetworkAttemptLedgerError::InvalidExecutableIdentity);
        }
        if self.records.len() >= MAX_NETWORK_ATTEMPT_RECORDS {
            return Err(NetworkAttemptLedgerError::CapacityExceeded);
        }
        if self
            .records
            .last()
            .is_some_and(|record| observed_at_millis < record.observed_at_millis)
        {
            return Err(NetworkAttemptLedgerError::NonMonotonicTime);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(NetworkAttemptLedgerError::SequenceOverflow)?;
        self.records.push(NetworkAttemptRecord {
            sequence,
            observed_at_millis,
            executable_sha256,
            component: observation.component,
            destination: observation.destination,
            transport: observation.transport,
            endpoint_sha256: observation.endpoint_sha256,
            attempted_bytes: observation.attempted_bytes,
            decision: policy.evaluate(observation),
        });
        self.records
            .last()
            .ok_or(NetworkAttemptLedgerError::SequenceOverflow)
    }

    /// Returns records in exact append order.
    #[must_use]
    pub fn records(&self) -> &[NetworkAttemptRecord] {
        &self.records
    }

    /// Returns a deterministic content identity without exposing payloads or addresses.
    #[must_use]
    pub fn content_sha256(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"agentmage.network-attempt-ledger.v1\0");
        digest.update((self.records.len() as u64).to_be_bytes());
        for record in &self.records {
            digest.update(record.sequence.to_be_bytes());
            digest.update(record.observed_at_millis.to_be_bytes());
            digest.update(record.executable_sha256);
            digest.update([network_component_tag(record.component)]);
            digest.update([network_destination_tag(record.destination)]);
            digest.update([local_transport_tag(record.transport)]);
            match record.endpoint_sha256 {
                Some(identity) => {
                    digest.update([1]);
                    digest.update(identity);
                }
                None => digest.update([0]),
            }
            digest.update(record.attempted_bytes.to_be_bytes());
            let (decision, reason) = strict_local_decision_tags(record.decision);
            digest.update([decision, reason]);
        }
        digest.finalize().into()
    }
}

impl Default for NetworkAttemptLedger {
    fn default() -> Self {
        Self::new()
    }
}

const fn network_component_tag(component: NetworkComponent) -> u8 {
    match component {
        NetworkComponent::VisualStudioCodeExtension => 1,
        NetworkComponent::NativeBridge => 2,
        NetworkComponent::Kernel => 3,
        NetworkComponent::KernelNativeInferenceAdapter => 4,
        NetworkComponent::KernelDockerInferenceAdapter => 5,
        NetworkComponent::ToolWorker => 6,
        NetworkComponent::Converter => 7,
        NetworkComponent::Indexer => 8,
        NetworkComponent::ModelInstaller => 9,
        NetworkComponent::Undeclared => 10,
    }
}

const fn network_destination_tag(destination: NetworkDestinationClass) -> u8 {
    match destination {
        NetworkDestinationClass::LocalSocket => 1,
        NetworkDestinationClass::AuthenticatedLocalSocket => 2,
        NetworkDestinationClass::Loopback => 3,
        NetworkDestinationClass::Unspecified => 4,
        NetworkDestinationClass::LinkLocal => 5,
        NetworkDestinationClass::PrivateLan => 6,
        NetworkDestinationClass::Multicast => 7,
        NetworkDestinationClass::ContainerNetwork => 8,
        NetworkDestinationClass::Proxy => 9,
        NetworkDestinationClass::Dns => 10,
        NetworkDestinationClass::External => 11,
        NetworkDestinationClass::Unknown => 12,
    }
}

const fn local_transport_tag(transport: Option<LocalTransport>) -> u8 {
    match transport {
        None => 0,
        Some(LocalTransport::AuthenticatedUnixSocket) => 1,
        Some(LocalTransport::GuardedLoopbackTcp) => 2,
    }
}

const fn strict_local_decision_tags(decision: StrictLocalNetworkDecision) -> (u8, u8) {
    match decision {
        StrictLocalNetworkDecision::Allow { reason } => (1, decision_reason_tag(reason)),
        StrictLocalNetworkDecision::Block { reason } => (2, decision_reason_tag(reason)),
    }
}

const fn decision_reason_tag(reason: StrictLocalDecisionReason) -> u8 {
    match reason {
        StrictLocalDecisionReason::ExactAuthenticatedInferenceEndpoint => 1,
        StrictLocalDecisionReason::ClientNotAuthorized => 2,
        StrictLocalDecisionReason::DestinationNotLocal => 3,
        StrictLocalDecisionReason::TransportMismatch => 4,
        StrictLocalDecisionReason::PeerNotAuthenticated => 5,
        StrictLocalDecisionReason::EndpointIdentityMismatch => 6,
    }
}

/// Terminal disposition of one separate model-acquisition attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionExitDisposition {
    /// The exact staged artifact was verified and activated atomically.
    Completed,
    /// Acquisition was cancelled and staging was removed.
    Cancelled,
    /// Candidate bytes failed verification and were quarantined.
    Corrupt,
}

/// Content-free staged-artifact state observed after acquisition exits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagedArtifactState {
    /// A verified artifact was activated.
    ActivatedVerified,
    /// No staged artifact remains.
    Removed,
    /// Rejected bytes remain only in verified quarantine.
    Quarantined,
    /// Staging remains incomplete or cannot be classified.
    Unresolved,
}

/// Fresh content-free platform observation used to prove post-acquisition offline state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflinePreflightObservation {
    /// Monotonic observation time.
    pub observed_at_millis: u64,
    /// Remaining acquisition-process count.
    pub acquisition_process_count: u32,
    /// Remaining acquisition-socket count.
    pub acquisition_socket_count: u32,
    /// Remaining externally scoped network-rule count.
    pub external_network_rule_count: u32,
    /// Bytes observed leaving the local boundary after acquisition exit.
    pub observed_outbound_bytes: u64,
    /// DNS attempts observed after acquisition exit.
    pub observed_dns_attempts: u64,
    /// Content-free staged-artifact disposition.
    pub staged_artifact_state: StagedArtifactState,
    /// Exact reconciled session-boundary report identity.
    pub session_boundary_sha256: [u8; 32],
    /// Exact active offline firewall-policy identity.
    pub firewall_policy_sha256: [u8; 32],
}

/// Stable post-acquisition offline-proof refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfflineProofError {
    /// Acquisition did not follow the one-way lifecycle.
    InvalidLifecycle,
    /// The preflight was not strictly newer than acquisition exit.
    StaleObservation,
    /// An acquisition process, socket, or external network rule remained active.
    AcquisitionAuthorityActive,
    /// Staged artifact state did not match the recorded exit disposition.
    ArtifactStateMismatch,
    /// Session or firewall identity was not established.
    InvalidBoundaryIdentity,
    /// Outbound bytes or DNS activity were observed after acquisition exit.
    OutboundActivityObserved,
    /// Ledger range, order, time, identity, or arithmetic was invalid.
    LedgerInvalid,
    /// This workflow already issued its sole proof.
    AlreadyProven,
}

impl std::fmt::Display for OfflineProofError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLifecycle => "offline_proof.invalid_lifecycle",
            Self::StaleObservation => "offline_proof.stale_observation",
            Self::AcquisitionAuthorityActive => "offline_proof.acquisition_authority_active",
            Self::ArtifactStateMismatch => "offline_proof.artifact_state_mismatch",
            Self::InvalidBoundaryIdentity => "offline_proof.invalid_boundary_identity",
            Self::OutboundActivityObserved => "offline_proof.outbound_activity_observed",
            Self::LedgerInvalid => "offline_proof.ledger_invalid",
            Self::AlreadyProven => "offline_proof.already_proven",
        })
    }
}

impl std::error::Error for OfflineProofError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OfflineProofState {
    Acquiring,
    Exited {
        disposition: AcquisitionExitDisposition,
        exited_at_millis: u64,
    },
    Proven,
}

/// Kernel-owned one-way workflow for a fresh post-acquisition offline proof.
#[derive(Debug, PartialEq, Eq)]
pub struct OfflineProofWorkflow {
    acquisition_epoch: u64,
    started_at_millis: u64,
    ledger_start_sequence: u64,
    state: OfflineProofState,
}

impl OfflineProofWorkflow {
    /// Starts one uniquely owned acquisition epoch without granting authority.
    ///
    /// The acquisition orchestrator must prevent duplicate construction for the
    /// same epoch. This type is intentionally not cloneable.
    #[must_use]
    pub const fn new(
        acquisition_epoch: u64,
        started_at_millis: u64,
        ledger_start_sequence: u64,
    ) -> Self {
        Self {
            acquisition_epoch,
            started_at_millis,
            ledger_start_sequence,
            state: OfflineProofState::Acquiring,
        }
    }

    /// Records the terminal acquisition disposition and exact exit time once.
    pub fn record_exit(
        &mut self,
        disposition: AcquisitionExitDisposition,
        exited_at_millis: u64,
    ) -> Result<(), OfflineProofError> {
        if self.state != OfflineProofState::Acquiring || exited_at_millis < self.started_at_millis {
            return Err(OfflineProofError::InvalidLifecycle);
        }
        self.state = OfflineProofState::Exited {
            disposition,
            exited_at_millis,
        };
        Ok(())
    }

    /// Issues the sole proof only from fresh, closed, internally consistent facts.
    pub fn prove(
        &mut self,
        observation: &OfflinePreflightObservation,
        ledger: &NetworkAttemptLedger,
    ) -> Result<OfflineProofReceipt, OfflineProofError> {
        let (disposition, exited_at_millis) = match self.state {
            OfflineProofState::Acquiring => return Err(OfflineProofError::InvalidLifecycle),
            OfflineProofState::Exited {
                disposition,
                exited_at_millis,
            } => (disposition, exited_at_millis),
            OfflineProofState::Proven => return Err(OfflineProofError::AlreadyProven),
        };
        if observation.observed_at_millis <= exited_at_millis {
            return Err(OfflineProofError::StaleObservation);
        }
        if observation.acquisition_process_count != 0
            || observation.acquisition_socket_count != 0
            || observation.external_network_rule_count != 0
        {
            return Err(OfflineProofError::AcquisitionAuthorityActive);
        }
        if !artifact_state_matches(disposition, observation.staged_artifact_state) {
            return Err(OfflineProofError::ArtifactStateMismatch);
        }
        if observation.session_boundary_sha256 == [0; 32]
            || observation.firewall_policy_sha256 == [0; 32]
        {
            return Err(OfflineProofError::InvalidBoundaryIdentity);
        }
        if observation.observed_outbound_bytes != 0 || observation.observed_dns_attempts != 0 {
            return Err(OfflineProofError::OutboundActivityObserved);
        }
        let start = usize::try_from(self.ledger_start_sequence)
            .ok()
            .filter(|start| *start <= ledger.records.len())
            .ok_or(OfflineProofError::LedgerInvalid)?;
        let mut blocked_attempts = 0_u64;
        let mut blocked_egress_attempts = 0_u64;
        let mut allowed_local_attempts = 0_u64;
        for (offset, record) in ledger.records[start..].iter().enumerate() {
            let expected_sequence = self
                .ledger_start_sequence
                .checked_add(u64::try_from(offset).map_err(|_| OfflineProofError::LedgerInvalid)?)
                .ok_or(OfflineProofError::LedgerInvalid)?;
            if record.sequence != expected_sequence
                || record.observed_at_millis < self.started_at_millis
                || record.observed_at_millis > observation.observed_at_millis
                || record.executable_sha256 == [0; 32]
            {
                return Err(OfflineProofError::LedgerInvalid);
            }
            match record.decision {
                StrictLocalNetworkDecision::Allow { .. } => {
                    allowed_local_attempts = allowed_local_attempts
                        .checked_add(1)
                        .ok_or(OfflineProofError::LedgerInvalid)?;
                }
                StrictLocalNetworkDecision::Block { .. } => {
                    blocked_attempts = blocked_attempts
                        .checked_add(1)
                        .ok_or(OfflineProofError::LedgerInvalid)?;
                    if !matches!(
                        record.destination,
                        NetworkDestinationClass::LocalSocket
                            | NetworkDestinationClass::AuthenticatedLocalSocket
                            | NetworkDestinationClass::Loopback
                    ) {
                        blocked_egress_attempts = blocked_egress_attempts
                            .checked_add(1)
                            .ok_or(OfflineProofError::LedgerInvalid)?;
                    }
                }
            }
        }
        let ledger_record_count = u64::try_from(ledger.records.len() - start)
            .map_err(|_| OfflineProofError::LedgerInvalid)?;
        let ledger_sha256 = ledger.content_sha256();
        let proof_sha256 = offline_proof_sha256(
            self.acquisition_epoch,
            self.started_at_millis,
            disposition,
            exited_at_millis,
            observation,
            self.ledger_start_sequence,
            ledger_record_count,
            blocked_attempts,
            blocked_egress_attempts,
            allowed_local_attempts,
            ledger_sha256,
        );
        self.state = OfflineProofState::Proven;
        Ok(OfflineProofReceipt {
            acquisition_epoch: self.acquisition_epoch,
            started_at_millis: self.started_at_millis,
            disposition,
            exited_at_millis,
            observed_at_millis: observation.observed_at_millis,
            ledger_start_sequence: self.ledger_start_sequence,
            ledger_record_count,
            blocked_attempts,
            blocked_egress_attempts,
            allowed_local_attempts,
            session_boundary_sha256: observation.session_boundary_sha256,
            firewall_policy_sha256: observation.firewall_policy_sha256,
            ledger_sha256,
            proof_sha256,
        })
    }
}

/// Content-free, hash-bound proof that acquisition authority was removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflineProofReceipt {
    acquisition_epoch: u64,
    started_at_millis: u64,
    disposition: AcquisitionExitDisposition,
    exited_at_millis: u64,
    observed_at_millis: u64,
    ledger_start_sequence: u64,
    ledger_record_count: u64,
    blocked_attempts: u64,
    blocked_egress_attempts: u64,
    allowed_local_attempts: u64,
    session_boundary_sha256: [u8; 32],
    firewall_policy_sha256: [u8; 32],
    ledger_sha256: [u8; 32],
    proof_sha256: [u8; 32],
}

impl OfflineProofReceipt {
    /// Returns the acquisition epoch.
    #[must_use]
    pub const fn acquisition_epoch(&self) -> u64 {
        self.acquisition_epoch
    }

    /// Returns the exact acquisition start time.
    #[must_use]
    pub const fn started_at_millis(&self) -> u64 {
        self.started_at_millis
    }

    /// Returns the terminal acquisition disposition.
    #[must_use]
    pub const fn disposition(&self) -> AcquisitionExitDisposition {
        self.disposition
    }

    /// Returns the exact acquisition exit time.
    #[must_use]
    pub const fn exited_at_millis(&self) -> u64 {
        self.exited_at_millis
    }

    /// Returns the newer offline-preflight observation time.
    #[must_use]
    pub const fn observed_at_millis(&self) -> u64 {
        self.observed_at_millis
    }

    /// Returns the first ledger sequence covered by this acquisition epoch.
    #[must_use]
    pub const fn ledger_start_sequence(&self) -> u64 {
        self.ledger_start_sequence
    }

    /// Returns the covered ledger record count.
    #[must_use]
    pub const fn ledger_record_count(&self) -> u64 {
        self.ledger_record_count
    }

    /// Returns the number of blocked attempts in the covered ledger range.
    #[must_use]
    pub const fn blocked_attempts(&self) -> u64 {
        self.blocked_attempts
    }

    /// Returns blocked attempts directed outside local destination classes.
    #[must_use]
    pub const fn blocked_egress_attempts(&self) -> u64 {
        self.blocked_egress_attempts
    }

    /// Returns exact guarded local attempts admitted by kernel policy.
    #[must_use]
    pub const fn allowed_local_attempts(&self) -> u64 {
        self.allowed_local_attempts
    }

    /// Returns the exact reconciled session-boundary identity.
    #[must_use]
    pub const fn session_boundary_sha256(&self) -> &[u8; 32] {
        &self.session_boundary_sha256
    }

    /// Returns the exact active firewall-policy identity.
    #[must_use]
    pub const fn firewall_policy_sha256(&self) -> &[u8; 32] {
        &self.firewall_policy_sha256
    }

    /// Returns the deterministic content-free ledger identity.
    #[must_use]
    pub const fn ledger_sha256(&self) -> &[u8; 32] {
        &self.ledger_sha256
    }

    /// Returns this complete proof identity.
    #[must_use]
    pub const fn proof_sha256(&self) -> &[u8; 32] {
        &self.proof_sha256
    }
}

const fn artifact_state_matches(
    disposition: AcquisitionExitDisposition,
    state: StagedArtifactState,
) -> bool {
    matches!(
        (disposition, state),
        (
            AcquisitionExitDisposition::Completed,
            StagedArtifactState::ActivatedVerified
        ) | (
            AcquisitionExitDisposition::Cancelled,
            StagedArtifactState::Removed
        ) | (
            AcquisitionExitDisposition::Corrupt,
            StagedArtifactState::Quarantined
        )
    )
}

#[allow(clippy::too_many_arguments)]
fn offline_proof_sha256(
    acquisition_epoch: u64,
    started_at_millis: u64,
    disposition: AcquisitionExitDisposition,
    exited_at_millis: u64,
    observation: &OfflinePreflightObservation,
    ledger_start_sequence: u64,
    ledger_record_count: u64,
    blocked_attempts: u64,
    blocked_egress_attempts: u64,
    allowed_local_attempts: u64,
    ledger_sha256: [u8; 32],
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.offline-proof.v1\0");
    digest.update(acquisition_epoch.to_be_bytes());
    digest.update(started_at_millis.to_be_bytes());
    digest.update([acquisition_disposition_tag(disposition)]);
    digest.update(exited_at_millis.to_be_bytes());
    digest.update(observation.observed_at_millis.to_be_bytes());
    digest.update(ledger_start_sequence.to_be_bytes());
    digest.update(ledger_record_count.to_be_bytes());
    digest.update(blocked_attempts.to_be_bytes());
    digest.update(blocked_egress_attempts.to_be_bytes());
    digest.update(allowed_local_attempts.to_be_bytes());
    digest.update(observation.session_boundary_sha256);
    digest.update(observation.firewall_policy_sha256);
    digest.update(ledger_sha256);
    digest.finalize().into()
}

const fn acquisition_disposition_tag(disposition: AcquisitionExitDisposition) -> u8 {
    match disposition {
        AcquisitionExitDisposition::Completed => 1,
        AcquisitionExitDisposition::Cancelled => 2,
        AcquisitionExitDisposition::Corrupt => 3,
    }
}

/// Strict-local data-root refusal reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrictLocalStorageDenial {
    /// Root identity digest was not established.
    InvalidRootIdentity,
    /// A symbolic-link component was observed.
    SymbolicLink,
    /// A known synchronization marker was observed.
    CloudSynchronized,
    /// Filesystem is remote.
    RemoteFilesystem,
    /// Filesystem is userspace-backed and conservatively rejected.
    FuseFilesystem,
    /// Filesystem class could not be established.
    UnknownFilesystem,
}

/// Strict-local storage admission result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrictLocalStorageDecision {
    /// Root is eligible for later user selection and persistence policy.
    Eligible,
    /// Root is not eligible in the strict-local profile.
    Reject {
        /// First exact refusal reason.
        reason: StrictLocalStorageDenial,
    },
}

/// Evaluates a content-free platform storage observation.
#[must_use]
pub fn evaluate_storage(observation: &StrictLocalStorageObservation) -> StrictLocalStorageDecision {
    let reason = if observation.root_identity_sha256 == [0; 32] {
        Some(StrictLocalStorageDenial::InvalidRootIdentity)
    } else if !observation.symlink_free {
        Some(StrictLocalStorageDenial::SymbolicLink)
    } else if observation.synchronization_marker.is_some() {
        Some(StrictLocalStorageDenial::CloudSynchronized)
    } else {
        match observation.filesystem {
            StorageFilesystemClass::Local => None,
            StorageFilesystemClass::Remote => Some(StrictLocalStorageDenial::RemoteFilesystem),
            StorageFilesystemClass::Fuse => Some(StrictLocalStorageDenial::FuseFilesystem),
            StorageFilesystemClass::Unknown => Some(StrictLocalStorageDenial::UnknownFilesystem),
        }
    };
    match reason {
        Some(reason) => StrictLocalStorageDecision::Reject { reason },
        None => StrictLocalStorageDecision::Eligible,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CloudSynchronizationMarker, LocalEndpointIdentity, LocalTransport, NetworkComponent,
        NetworkDestinationClass, NetworkObservation, StorageFilesystemClass,
        StrictLocalStorageObservation,
    };

    use super::{
        AcquisitionExitDisposition, MAX_NETWORK_ATTEMPT_RECORDS, NetworkAttemptLedger,
        NetworkAttemptLedgerError, OfflinePreflightObservation, OfflineProofError,
        OfflineProofWorkflow, StagedArtifactState, StrictLocalDecisionReason,
        StrictLocalNetworkDecision, StrictLocalNetworkPolicy, StrictLocalStorageDecision,
        StrictLocalStorageDenial, evaluate_storage,
    };

    fn endpoint() -> LocalEndpointIdentity {
        LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [7; 32],
        )
        .expect("native endpoint")
    }

    fn exact_observation() -> NetworkObservation {
        NetworkObservation {
            component: NetworkComponent::KernelNativeInferenceAdapter,
            destination: NetworkDestinationClass::AuthenticatedLocalSocket,
            transport: Some(LocalTransport::AuthenticatedUnixSocket),
            endpoint_sha256: Some([7; 32]),
            peer_authenticated: true,
            attempted_bytes: 128,
        }
    }

    fn blocked_observation(destination: NetworkDestinationClass) -> NetworkObservation {
        NetworkObservation {
            component: NetworkComponent::ToolWorker,
            destination,
            transport: None,
            endpoint_sha256: None,
            peer_authenticated: false,
            attempted_bytes: 12,
        }
    }

    fn offline_observation(state: StagedArtifactState) -> OfflinePreflightObservation {
        OfflinePreflightObservation {
            observed_at_millis: 201,
            acquisition_process_count: 0,
            acquisition_socket_count: 0,
            external_network_rule_count: 0,
            observed_outbound_bytes: 0,
            observed_dns_attempts: 0,
            staged_artifact_state: state,
            session_boundary_sha256: [21; 32],
            firewall_policy_sha256: [22; 32],
        }
    }

    fn representative_ledger() -> NetworkAttemptLedger {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        let mut ledger = NetworkAttemptLedger::new();
        ledger
            .evaluate_and_record(&policy, &exact_observation(), [31; 32], 110)
            .expect("allowed local attempt");
        ledger
            .evaluate_and_record(
                &policy,
                &blocked_observation(NetworkDestinationClass::External),
                [32; 32],
                120,
            )
            .expect("blocked external attempt");
        ledger
            .evaluate_and_record(
                &policy,
                &blocked_observation(NetworkDestinationClass::LocalSocket),
                [33; 32],
                130,
            )
            .expect("blocked local attempt");
        ledger
    }

    #[test]
    fn only_the_exact_authenticated_inference_observation_is_allowed() {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        assert_eq!(
            policy.evaluate(&exact_observation()),
            StrictLocalNetworkDecision::Allow {
                reason: StrictLocalDecisionReason::ExactAuthenticatedInferenceEndpoint
            }
        );

        let mutations = [
            NetworkObservation {
                component: NetworkComponent::VisualStudioCodeExtension,
                ..exact_observation()
            },
            NetworkObservation {
                destination: NetworkDestinationClass::Loopback,
                ..exact_observation()
            },
            NetworkObservation {
                transport: Some(LocalTransport::GuardedLoopbackTcp),
                ..exact_observation()
            },
            NetworkObservation {
                peer_authenticated: false,
                ..exact_observation()
            },
            NetworkObservation {
                endpoint_sha256: Some([8; 32]),
                ..exact_observation()
            },
        ];
        for mutation in mutations {
            assert!(matches!(
                policy.evaluate(&mutation),
                StrictLocalNetworkDecision::Block { .. }
            ));
        }
    }

    #[test]
    fn every_undeclared_destination_class_is_blocked() {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        for destination in [
            NetworkDestinationClass::LocalSocket,
            NetworkDestinationClass::Loopback,
            NetworkDestinationClass::Unspecified,
            NetworkDestinationClass::LinkLocal,
            NetworkDestinationClass::PrivateLan,
            NetworkDestinationClass::Multicast,
            NetworkDestinationClass::ContainerNetwork,
            NetworkDestinationClass::Proxy,
            NetworkDestinationClass::Dns,
            NetworkDestinationClass::External,
            NetworkDestinationClass::Unknown,
        ] {
            let observation = NetworkObservation {
                destination,
                ..exact_observation()
            };
            assert_eq!(
                policy.evaluate(&observation),
                StrictLocalNetworkDecision::Block {
                    reason: StrictLocalDecisionReason::DestinationNotLocal
                }
            );
        }
    }

    #[test]
    fn every_normal_component_is_denied_except_the_selected_guarded_adapter() {
        let topologies = [
            (
                NetworkComponent::KernelNativeInferenceAdapter,
                LocalTransport::AuthenticatedUnixSocket,
                NetworkDestinationClass::AuthenticatedLocalSocket,
            ),
            (
                NetworkComponent::KernelDockerInferenceAdapter,
                LocalTransport::GuardedLoopbackTcp,
                NetworkDestinationClass::Loopback,
            ),
        ];
        let components = [
            NetworkComponent::VisualStudioCodeExtension,
            NetworkComponent::NativeBridge,
            NetworkComponent::Kernel,
            NetworkComponent::KernelNativeInferenceAdapter,
            NetworkComponent::KernelDockerInferenceAdapter,
            NetworkComponent::ToolWorker,
            NetworkComponent::Converter,
            NetworkComponent::Indexer,
            NetworkComponent::ModelInstaller,
            NetworkComponent::Undeclared,
        ];
        for (selected, transport, destination) in topologies {
            let endpoint =
                LocalEndpointIdentity::new(selected, transport, [9; 32]).expect("guarded endpoint");
            let policy = StrictLocalNetworkPolicy::new(endpoint);
            for component in components {
                let decision = policy.evaluate(&NetworkObservation {
                    component,
                    destination,
                    transport: Some(transport),
                    endpoint_sha256: Some([9; 32]),
                    peer_authenticated: true,
                    attempted_bytes: 1,
                });
                if component == selected {
                    assert_eq!(
                        decision,
                        StrictLocalNetworkDecision::Allow {
                            reason: StrictLocalDecisionReason::ExactAuthenticatedInferenceEndpoint
                        }
                    );
                } else {
                    assert_eq!(
                        decision,
                        StrictLocalNetworkDecision::Block {
                            reason: StrictLocalDecisionReason::ClientNotAuthorized
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn native_and_docker_transports_cannot_substitute_for_each_other() {
        let native = StrictLocalNetworkPolicy::new(
            LocalEndpointIdentity::new(
                NetworkComponent::KernelNativeInferenceAdapter,
                LocalTransport::AuthenticatedUnixSocket,
                [3; 32],
            )
            .expect("native endpoint"),
        );
        let docker_observation = NetworkObservation {
            component: NetworkComponent::KernelDockerInferenceAdapter,
            destination: NetworkDestinationClass::Loopback,
            transport: Some(LocalTransport::GuardedLoopbackTcp),
            endpoint_sha256: Some([3; 32]),
            peer_authenticated: true,
            attempted_bytes: 1,
        };
        assert_eq!(
            native.evaluate(&docker_observation),
            StrictLocalNetworkDecision::Block {
                reason: StrictLocalDecisionReason::ClientNotAuthorized
            }
        );

        let docker = StrictLocalNetworkPolicy::new(
            LocalEndpointIdentity::new(
                NetworkComponent::KernelDockerInferenceAdapter,
                LocalTransport::GuardedLoopbackTcp,
                [3; 32],
            )
            .expect("Docker endpoint"),
        );
        assert_eq!(
            docker.evaluate(&exact_observation()),
            StrictLocalNetworkDecision::Block {
                reason: StrictLocalDecisionReason::ClientNotAuthorized
            }
        );
    }

    #[test]
    fn ledger_is_ordered_content_free_bounded_and_computes_its_own_decision() {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        let mut ledger = NetworkAttemptLedger::new();
        let allowed = ledger
            .evaluate_and_record(&policy, &exact_observation(), [41; 32], 10)
            .expect("allowed record");
        assert_eq!(allowed.sequence(), 0);
        assert_eq!(allowed.executable_sha256(), &[41; 32]);
        assert!(matches!(
            allowed.decision(),
            StrictLocalNetworkDecision::Allow { .. }
        ));
        let blocked_observation = blocked_observation(NetworkDestinationClass::External);
        let blocked = ledger
            .evaluate_and_record(&policy, &blocked_observation, [42; 32], 10)
            .expect("blocked record");
        assert_eq!(blocked.sequence(), 1);
        assert!(matches!(
            blocked.decision(),
            StrictLocalNetworkDecision::Block { .. }
        ));
        assert_eq!(
            ledger.evaluate_and_record(&policy, &blocked_observation, [42; 32], 9),
            Err(NetworkAttemptLedgerError::NonMonotonicTime)
        );

        for timestamp in 2..MAX_NETWORK_ATTEMPT_RECORDS as u64 {
            ledger
                .evaluate_and_record(&policy, &blocked_observation, [42; 32], 10 + timestamp)
                .expect("bounded record");
        }
        assert_eq!(ledger.records().len(), MAX_NETWORK_ATTEMPT_RECORDS);
        assert_eq!(
            ledger.evaluate_and_record(&policy, &blocked_observation, [42; 32], u64::MAX),
            Err(NetworkAttemptLedgerError::CapacityExceeded)
        );
    }

    #[test]
    fn ledger_rejects_missing_executable_identity_and_hashes_every_retained_fact() {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        let mut ledger = NetworkAttemptLedger::new();
        assert_eq!(
            ledger.evaluate_and_record(&policy, &exact_observation(), [0; 32], 10),
            Err(NetworkAttemptLedgerError::InvalidExecutableIdentity)
        );
        ledger
            .evaluate_and_record(&policy, &exact_observation(), [41; 32], 10)
            .expect("baseline record");
        let baseline = ledger.content_sha256();
        assert_eq!(baseline, ledger.clone().content_sha256());

        let mut mutations = Vec::new();
        let mut executable = ledger.clone();
        executable.records[0].executable_sha256 = [42; 32];
        mutations.push(executable);
        let mut destination = ledger.clone();
        destination.records[0].destination = NetworkDestinationClass::External;
        mutations.push(destination);
        let mut time = ledger.clone();
        time.records[0].observed_at_millis = 11;
        mutations.push(time);
        let mut bytes = ledger.clone();
        bytes.records[0].attempted_bytes = 129;
        mutations.push(bytes);
        let mut decision = ledger.clone();
        decision.records[0].decision = StrictLocalNetworkDecision::Block {
            reason: StrictLocalDecisionReason::ClientNotAuthorized,
        };
        mutations.push(decision);
        for mutation in mutations {
            assert_ne!(baseline, mutation.content_sha256());
        }
    }

    #[test]
    fn strict_local_offline_proof_accepts_each_exact_terminal_artifact_state() {
        for (disposition, state) in [
            (
                AcquisitionExitDisposition::Completed,
                StagedArtifactState::ActivatedVerified,
            ),
            (
                AcquisitionExitDisposition::Cancelled,
                StagedArtifactState::Removed,
            ),
            (
                AcquisitionExitDisposition::Corrupt,
                StagedArtifactState::Quarantined,
            ),
        ] {
            let ledger = representative_ledger();
            let mut workflow = OfflineProofWorkflow::new(7, 100, 0);
            workflow
                .record_exit(disposition, 200)
                .expect("terminal acquisition exit");
            let receipt = workflow
                .prove(&offline_observation(state), &ledger)
                .expect("offline proof");
            assert_eq!(receipt.acquisition_epoch(), 7);
            assert_eq!(receipt.started_at_millis(), 100);
            assert_eq!(receipt.disposition(), disposition);
            assert_eq!(receipt.exited_at_millis(), 200);
            assert_eq!(receipt.observed_at_millis(), 201);
            assert_eq!(receipt.ledger_start_sequence(), 0);
            assert_eq!(receipt.ledger_record_count(), 3);
            assert_eq!(receipt.allowed_local_attempts(), 1);
            assert_eq!(receipt.blocked_attempts(), 2);
            assert_eq!(receipt.blocked_egress_attempts(), 1);
            assert_eq!(receipt.session_boundary_sha256(), &[21; 32]);
            assert_eq!(receipt.firewall_policy_sha256(), &[22; 32]);
            assert_eq!(receipt.ledger_sha256(), &ledger.content_sha256());
            assert_ne!(receipt.proof_sha256(), &[0; 32]);
        }
    }

    #[test]
    fn strict_local_offline_proof_enforces_one_way_lifecycle_and_freshness() {
        let ledger = representative_ledger();
        let mut workflow = OfflineProofWorkflow::new(7, 100, 0);
        assert_eq!(
            workflow.prove(
                &offline_observation(StagedArtifactState::ActivatedVerified),
                &ledger,
            ),
            Err(OfflineProofError::InvalidLifecycle)
        );
        assert_eq!(
            workflow.record_exit(AcquisitionExitDisposition::Completed, 99),
            Err(OfflineProofError::InvalidLifecycle)
        );
        workflow
            .record_exit(AcquisitionExitDisposition::Completed, 200)
            .expect("terminal acquisition exit");
        assert_eq!(
            workflow.record_exit(AcquisitionExitDisposition::Cancelled, 200),
            Err(OfflineProofError::InvalidLifecycle)
        );
        for observed_at_millis in [199, 200] {
            let mut observation = offline_observation(StagedArtifactState::ActivatedVerified);
            observation.observed_at_millis = observed_at_millis;
            assert_eq!(
                workflow.prove(&observation, &ledger),
                Err(OfflineProofError::StaleObservation)
            );
        }
        let observation = offline_observation(StagedArtifactState::ActivatedVerified);
        workflow.prove(&observation, &ledger).expect("first proof");
        assert_eq!(
            workflow.prove(&observation, &ledger),
            Err(OfflineProofError::AlreadyProven)
        );
    }

    #[test]
    fn strict_local_offline_proof_rejects_remaining_acquisition_authority() {
        let ledger = representative_ledger();
        for observation in [
            OfflinePreflightObservation {
                acquisition_process_count: 1,
                ..offline_observation(StagedArtifactState::ActivatedVerified)
            },
            OfflinePreflightObservation {
                acquisition_socket_count: 1,
                ..offline_observation(StagedArtifactState::ActivatedVerified)
            },
            OfflinePreflightObservation {
                external_network_rule_count: 1,
                ..offline_observation(StagedArtifactState::ActivatedVerified)
            },
        ] {
            let mut workflow = OfflineProofWorkflow::new(7, 100, 0);
            workflow
                .record_exit(AcquisitionExitDisposition::Completed, 200)
                .expect("terminal acquisition exit");
            assert_eq!(
                workflow.prove(&observation, &ledger),
                Err(OfflineProofError::AcquisitionAuthorityActive)
            );
        }
    }

    #[test]
    fn strict_local_offline_proof_rejects_every_artifact_mismatch() {
        let ledger = representative_ledger();
        for (disposition, expected) in [
            (
                AcquisitionExitDisposition::Completed,
                StagedArtifactState::ActivatedVerified,
            ),
            (
                AcquisitionExitDisposition::Cancelled,
                StagedArtifactState::Removed,
            ),
            (
                AcquisitionExitDisposition::Corrupt,
                StagedArtifactState::Quarantined,
            ),
        ] {
            for actual in [
                StagedArtifactState::ActivatedVerified,
                StagedArtifactState::Removed,
                StagedArtifactState::Quarantined,
                StagedArtifactState::Unresolved,
            ] {
                if actual == expected {
                    continue;
                }
                let mut workflow = OfflineProofWorkflow::new(7, 100, 0);
                workflow
                    .record_exit(disposition, 200)
                    .expect("terminal acquisition exit");
                assert_eq!(
                    workflow.prove(&offline_observation(actual), &ledger),
                    Err(OfflineProofError::ArtifactStateMismatch)
                );
            }
        }
    }

    #[test]
    fn strict_local_offline_proof_rejects_missing_boundaries_and_observed_activity() {
        let ledger = representative_ledger();
        let cases = [
            (
                OfflinePreflightObservation {
                    session_boundary_sha256: [0; 32],
                    ..offline_observation(StagedArtifactState::ActivatedVerified)
                },
                OfflineProofError::InvalidBoundaryIdentity,
            ),
            (
                OfflinePreflightObservation {
                    firewall_policy_sha256: [0; 32],
                    ..offline_observation(StagedArtifactState::ActivatedVerified)
                },
                OfflineProofError::InvalidBoundaryIdentity,
            ),
            (
                OfflinePreflightObservation {
                    observed_outbound_bytes: 1,
                    ..offline_observation(StagedArtifactState::ActivatedVerified)
                },
                OfflineProofError::OutboundActivityObserved,
            ),
            (
                OfflinePreflightObservation {
                    observed_dns_attempts: 1,
                    ..offline_observation(StagedArtifactState::ActivatedVerified)
                },
                OfflineProofError::OutboundActivityObserved,
            ),
        ];
        for (observation, expected) in cases {
            let mut workflow = OfflineProofWorkflow::new(7, 100, 0);
            workflow
                .record_exit(AcquisitionExitDisposition::Completed, 200)
                .expect("terminal acquisition exit");
            assert_eq!(workflow.prove(&observation, &ledger), Err(expected));
        }
    }

    #[test]
    fn strict_local_offline_proof_rejects_invalid_ledger_boundaries() {
        let observation = offline_observation(StagedArtifactState::ActivatedVerified);
        for (ledger, start) in [
            (representative_ledger(), 4),
            (
                {
                    let mut ledger = representative_ledger();
                    ledger.records[0].sequence = 1;
                    ledger
                },
                0,
            ),
            (
                {
                    let mut ledger = representative_ledger();
                    ledger.records[0].observed_at_millis = 99;
                    ledger
                },
                0,
            ),
            (
                {
                    let mut ledger = representative_ledger();
                    ledger.records[2].observed_at_millis = 202;
                    ledger
                },
                0,
            ),
            (
                {
                    let mut ledger = representative_ledger();
                    ledger.records[1].executable_sha256 = [0; 32];
                    ledger
                },
                0,
            ),
        ] {
            let mut workflow = OfflineProofWorkflow::new(7, 100, start);
            workflow
                .record_exit(AcquisitionExitDisposition::Completed, 200)
                .expect("terminal acquisition exit");
            assert_eq!(
                workflow.prove(&observation, &ledger),
                Err(OfflineProofError::LedgerInvalid)
            );
        }
    }

    #[test]
    fn strict_local_offline_proof_identity_is_deterministic_and_fact_bound() {
        let ledger = representative_ledger();
        let prove = |epoch, observation: OfflinePreflightObservation| {
            let mut workflow = OfflineProofWorkflow::new(epoch, 100, 0);
            workflow
                .record_exit(AcquisitionExitDisposition::Completed, 200)
                .expect("terminal acquisition exit");
            *workflow
                .prove(&observation, &ledger)
                .expect("offline proof")
                .proof_sha256()
        };
        let observation = offline_observation(StagedArtifactState::ActivatedVerified);
        let baseline = prove(7, observation.clone());
        assert_eq!(baseline, prove(7, observation.clone()));
        assert_ne!(baseline, prove(8, observation.clone()));
        assert_ne!(
            baseline,
            prove(
                7,
                OfflinePreflightObservation {
                    session_boundary_sha256: [23; 32],
                    ..observation
                },
            )
        );
        assert_eq!(
            OfflineProofError::OutboundActivityObserved.to_string(),
            "offline_proof.outbound_activity_observed"
        );
    }

    #[test]
    fn storage_policy_rejects_every_risky_or_unknown_observation() {
        let local = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [1; 32],
            symlink_free: true,
        };
        assert_eq!(
            evaluate_storage(&local),
            StrictLocalStorageDecision::Eligible
        );
        for (observation, expected) in [
            (
                StrictLocalStorageObservation {
                    root_identity_sha256: [0; 32],
                    ..local.clone()
                },
                StrictLocalStorageDenial::InvalidRootIdentity,
            ),
            (
                StrictLocalStorageObservation {
                    symlink_free: false,
                    ..local.clone()
                },
                StrictLocalStorageDenial::SymbolicLink,
            ),
            (
                StrictLocalStorageObservation {
                    synchronization_marker: Some(CloudSynchronizationMarker::Dropbox),
                    ..local.clone()
                },
                StrictLocalStorageDenial::CloudSynchronized,
            ),
            (
                StrictLocalStorageObservation {
                    filesystem: StorageFilesystemClass::Remote,
                    ..local.clone()
                },
                StrictLocalStorageDenial::RemoteFilesystem,
            ),
            (
                StrictLocalStorageObservation {
                    filesystem: StorageFilesystemClass::Fuse,
                    ..local.clone()
                },
                StrictLocalStorageDenial::FuseFilesystem,
            ),
            (
                StrictLocalStorageObservation {
                    filesystem: StorageFilesystemClass::Unknown,
                    ..local.clone()
                },
                StrictLocalStorageDenial::UnknownFilesystem,
            ),
        ] {
            assert_eq!(
                evaluate_storage(&observation),
                StrictLocalStorageDecision::Reject { reason: expected }
            );
        }
    }
}
