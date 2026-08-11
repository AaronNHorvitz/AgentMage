//! Deterministic strict-local policy with no network execution authority.

use agentmage_kernel_contracts::{
    LocalEndpointIdentity, LocalTransport, NetworkComponent, NetworkDestinationClass,
    NetworkObservation, StorageFilesystemClass, StrictLocalStorageObservation,
};

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
        observed_at_millis: u64,
    ) -> Result<&NetworkAttemptRecord, NetworkAttemptLedgerError> {
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
}

impl Default for NetworkAttemptLedger {
    fn default() -> Self {
        Self::new()
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
        MAX_NETWORK_ATTEMPT_RECORDS, NetworkAttemptLedger, NetworkAttemptLedgerError,
        StrictLocalDecisionReason, StrictLocalNetworkDecision, StrictLocalNetworkPolicy,
        StrictLocalStorageDecision, StrictLocalStorageDenial, evaluate_storage,
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
    fn ledger_is_ordered_content_free_bounded_and_computes_its_own_decision() {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        let mut ledger = NetworkAttemptLedger::new();
        let allowed = ledger
            .evaluate_and_record(&policy, &exact_observation(), 10)
            .expect("allowed record");
        assert_eq!(allowed.sequence(), 0);
        assert!(matches!(
            allowed.decision(),
            StrictLocalNetworkDecision::Allow { .. }
        ));
        let blocked_observation = NetworkObservation {
            component: NetworkComponent::ToolWorker,
            destination: NetworkDestinationClass::External,
            transport: None,
            endpoint_sha256: None,
            peer_authenticated: false,
            attempted_bytes: 12,
        };
        let blocked = ledger
            .evaluate_and_record(&policy, &blocked_observation, 10)
            .expect("blocked record");
        assert_eq!(blocked.sequence(), 1);
        assert!(matches!(
            blocked.decision(),
            StrictLocalNetworkDecision::Block { .. }
        ));
        assert_eq!(
            ledger.evaluate_and_record(&policy, &blocked_observation, 9),
            Err(NetworkAttemptLedgerError::NonMonotonicTime)
        );

        for timestamp in 2..MAX_NETWORK_ATTEMPT_RECORDS as u64 {
            ledger
                .evaluate_and_record(&policy, &blocked_observation, 10 + timestamp)
                .expect("bounded record");
        }
        assert_eq!(ledger.records().len(), MAX_NETWORK_ATTEMPT_RECORDS);
        assert_eq!(
            ledger.evaluate_and_record(&policy, &blocked_observation, u64::MAX),
            Err(NetworkAttemptLedgerError::CapacityExceeded)
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
