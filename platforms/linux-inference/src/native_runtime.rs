//! Exact native llama.cpp package, model-store, IPC, and resource contract.

use std::fmt;

use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

/// Exact runtime package identifier admitted by the Linux reference boundary.
pub const NATIVE_RUNTIME_PACKAGE_ID: &str = "agentmage-llama-cpp-b10333-cpu-linux-x86_64";

/// SHA-256 of the checked native runtime package profile.
pub const NATIVE_RUNTIME_PROFILE_SHA256: [u8; 32] = [
    0x21, 0x34, 0x6c, 0x06, 0xfb, 0x86, 0xb4, 0x18, 0x70, 0x63, 0x26, 0xb1, 0x86, 0x60, 0x9f, 0x8e,
    0x1f, 0x69, 0x0d, 0x6b, 0x57, 0xb5, 0x3e, 0x73, 0xd0, 0x2b, 0x4a, 0xd8, 0xe2, 0x8e, 0x53, 0xea,
];

/// Lowercase hexadecimal identity of the checked native runtime package profile.
pub const NATIVE_RUNTIME_PROFILE_SHA256_HEX: &str =
    "21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea";

/// SHA-256 of the immutable upstream llama.cpp b10333 Linux archive.
pub const NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256: [u8; 32] = [
    0xf1, 0x4e, 0x31, 0x2f, 0xbe, 0xe3, 0x3c, 0xe6, 0x0d, 0x2e, 0xed, 0x70, 0x36, 0xde, 0x5d, 0xeb,
    0xe3, 0x1c, 0x1d, 0x7f, 0x4d, 0x8f, 0x0e, 0x37, 0x92, 0x0e, 0xb0, 0xa2, 0xde, 0x08, 0x54, 0xa5,
];

/// Content-free native runtime contract failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeRuntimeContractError {
    /// The runtime package identifier or digest did not match the checked profile.
    RuntimeIdentity,
    /// The model-store object identity was empty.
    ModelStoreIdentity,
    /// The model-store owner was privileged or did not match the launch user.
    ModelStoreOwner,
    /// The model-store directory was not private mode `0700`.
    ModelStoreMode,
    /// One or more resource settings exceeded the closed envelope.
    ResourceEnvelope,
    /// The local endpoint was not the authenticated native-inference path.
    EndpointIdentity,
}

impl NativeRuntimeContractError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RuntimeIdentity => "native-inference.runtime.identity",
            Self::ModelStoreIdentity => "native-inference.model-store.identity",
            Self::ModelStoreOwner => "native-inference.model-store.owner",
            Self::ModelStoreMode => "native-inference.model-store.mode",
            Self::ResourceEnvelope => "native-inference.resources.invalid",
            Self::EndpointIdentity => "native-inference.endpoint.invalid",
        }
    }
}

impl fmt::Display for NativeRuntimeContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for NativeRuntimeContractError {}

/// Verified identity of the one package admitted by this runtime increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinnedNativeRuntimeIdentity {
    _private: (),
}

impl PinnedNativeRuntimeIdentity {
    /// Verifies the package and source archive identities without fallback.
    pub fn verify(
        package_id: &str,
        profile_sha256: [u8; 32],
        source_archive_sha256: [u8; 32],
    ) -> Result<Self, NativeRuntimeContractError> {
        if package_id != NATIVE_RUNTIME_PACKAGE_ID
            || profile_sha256 != NATIVE_RUNTIME_PROFILE_SHA256
            || source_archive_sha256 != NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256
        {
            return Err(NativeRuntimeContractError::RuntimeIdentity);
        }
        Ok(Self { _private: () })
    }

    /// Returns the exact admitted package identifier.
    #[must_use]
    pub const fn package_id(self) -> &'static str {
        NATIVE_RUNTIME_PACKAGE_ID
    }

    /// Returns the exact checked profile identity.
    #[must_use]
    pub const fn profile_sha256(self) -> &'static [u8; 32] {
        &NATIVE_RUNTIME_PROFILE_SHA256
    }

    /// Returns the exact upstream source-archive identity.
    #[must_use]
    pub const fn source_archive_sha256(self) -> &'static [u8; 32] {
        &NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256
    }
}

/// Identity of a private model directory already opened and held by the kernel.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RestrictedModelStoreIdentity {
    directory_identity_sha256: [u8; 32],
    owner_uid: u32,
}

impl RestrictedModelStoreIdentity {
    /// Verifies a non-root, launch-user-owned, private model-store identity.
    pub fn verify(
        directory_identity_sha256: [u8; 32],
        owner_uid: u32,
        launch_uid: u32,
        directory_mode: u32,
    ) -> Result<Self, NativeRuntimeContractError> {
        if directory_identity_sha256 == [0; 32] {
            return Err(NativeRuntimeContractError::ModelStoreIdentity);
        }
        if owner_uid == 0 || owner_uid != launch_uid {
            return Err(NativeRuntimeContractError::ModelStoreOwner);
        }
        if directory_mode != 0o700 {
            return Err(NativeRuntimeContractError::ModelStoreMode);
        }
        Ok(Self {
            directory_identity_sha256,
            owner_uid,
        })
    }

    /// Returns the content-free directory object identity.
    #[must_use]
    pub const fn directory_identity_sha256(&self) -> &[u8; 32] {
        &self.directory_identity_sha256
    }

    /// Returns the verified standard-user owner.
    #[must_use]
    pub const fn owner_uid(&self) -> u32 {
        self.owner_uid
    }
}

impl fmt::Debug for RestrictedModelStoreIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RestrictedModelStoreIdentity")
            .field("owner_uid", &self.owner_uid)
            .finish_non_exhaustive()
    }
}

/// Exact cgroup, duration, output, and concurrency envelope for one native process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeInferenceResourceEnvelope {
    memory_bytes: u64,
    tasks: u32,
    cpu_percent: u16,
    runtime_seconds: u16,
    output_bytes: u32,
}

impl NativeInferenceResourceEnvelope {
    /// Verifies one no-swap, one-slot resource envelope below every hard ceiling.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        memory_bytes: u64,
        tasks: u32,
        cpu_percent: u16,
        runtime_seconds: u16,
        output_bytes: u32,
        swap_bytes: u64,
        parallel_slots: u8,
    ) -> Result<Self, NativeRuntimeContractError> {
        if !(256 * 1024 * 1024..=64 * 1024 * 1024 * 1024).contains(&memory_bytes)
            || !(1..=64).contains(&tasks)
            || !(1..=3200).contains(&cpu_percent)
            || !(1..=3600).contains(&runtime_seconds)
            || !(1..=16 * 1024 * 1024).contains(&output_bytes)
            || swap_bytes != 0
            || parallel_slots != 1
        {
            return Err(NativeRuntimeContractError::ResourceEnvelope);
        }
        Ok(Self {
            memory_bytes,
            tasks,
            cpu_percent,
            runtime_seconds,
            output_bytes,
        })
    }

    /// Returns the cgroup memory ceiling in bytes.
    #[must_use]
    pub const fn memory_bytes(self) -> u64 {
        self.memory_bytes
    }

    /// Returns the cgroup task ceiling.
    #[must_use]
    pub const fn tasks(self) -> u32 {
        self.tasks
    }

    /// Returns the cgroup CPU percentage ceiling.
    #[must_use]
    pub const fn cpu_percent(self) -> u16 {
        self.cpu_percent
    }

    /// Returns the process lifetime ceiling in seconds.
    #[must_use]
    pub const fn runtime_seconds(self) -> u16 {
        self.runtime_seconds
    }

    /// Returns the retained output ceiling in bytes.
    #[must_use]
    pub const fn output_bytes(self) -> u32 {
        self.output_bytes
    }

    /// Reports the immutable no-swap requirement.
    #[must_use]
    pub const fn swap_bytes(self) -> u64 {
        0
    }

    /// Reports the immutable one-slot requirement.
    #[must_use]
    pub const fn parallel_slots(self) -> u8 {
        1
    }
}

/// Complete authority-free topology passed to the future native runtime supervisor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeInferenceTopology {
    runtime: PinnedNativeRuntimeIdentity,
    model_store: RestrictedModelStoreIdentity,
    resources: NativeInferenceResourceEnvelope,
    endpoint: LocalEndpointIdentity,
}

impl NativeInferenceTopology {
    /// Binds one pinned runtime, private model store, bounded cgroup, and guarded IPC path.
    pub fn verify(
        runtime: PinnedNativeRuntimeIdentity,
        model_store: RestrictedModelStoreIdentity,
        resources: NativeInferenceResourceEnvelope,
        endpoint: LocalEndpointIdentity,
    ) -> Result<Self, NativeRuntimeContractError> {
        if endpoint.client() != NetworkComponent::KernelNativeInferenceAdapter
            || endpoint.transport() != LocalTransport::AuthenticatedUnixSocket
        {
            return Err(NativeRuntimeContractError::EndpointIdentity);
        }
        Ok(Self {
            runtime,
            model_store,
            resources,
            endpoint,
        })
    }

    /// Returns the verified runtime identity.
    #[must_use]
    pub const fn runtime(&self) -> PinnedNativeRuntimeIdentity {
        self.runtime
    }

    /// Returns the verified private model-store identity.
    #[must_use]
    pub const fn model_store(&self) -> &RestrictedModelStoreIdentity {
        &self.model_store
    }

    /// Returns the verified resource envelope.
    #[must_use]
    pub const fn resources(&self) -> NativeInferenceResourceEnvelope {
        self.resources
    }

    /// Returns the guarded local endpoint identity.
    #[must_use]
    pub const fn endpoint(&self) -> &LocalEndpointIdentity {
        &self.endpoint
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

    use super::{
        NATIVE_RUNTIME_PACKAGE_ID, NATIVE_RUNTIME_PROFILE_SHA256,
        NATIVE_RUNTIME_PROFILE_SHA256_HEX, NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256,
        NativeInferenceResourceEnvelope, NativeInferenceTopology, NativeRuntimeContractError,
        PinnedNativeRuntimeIdentity, RestrictedModelStoreIdentity,
    };

    fn runtime() -> PinnedNativeRuntimeIdentity {
        PinnedNativeRuntimeIdentity::verify(
            NATIVE_RUNTIME_PACKAGE_ID,
            NATIVE_RUNTIME_PROFILE_SHA256,
            NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256,
        )
        .expect("pinned identity")
    }

    fn store() -> RestrictedModelStoreIdentity {
        RestrictedModelStoreIdentity::verify([3; 32], 1000, 1000, 0o700)
            .expect("private standard-user store")
    }

    fn resources() -> NativeInferenceResourceEnvelope {
        NativeInferenceResourceEnvelope::verify(
            16 * 1024 * 1024 * 1024,
            32,
            1600,
            900,
            4 * 1024 * 1024,
            0,
            1,
        )
        .expect("bounded resources")
    }

    #[test]
    fn runtime_identity_has_no_mutable_name_or_fallback() {
        let identity = runtime();
        let profile_hex = NATIVE_RUNTIME_PROFILE_SHA256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(identity.package_id(), NATIVE_RUNTIME_PACKAGE_ID);
        assert_eq!(profile_hex, NATIVE_RUNTIME_PROFILE_SHA256_HEX);
        assert_eq!(identity.profile_sha256(), &NATIVE_RUNTIME_PROFILE_SHA256);
        assert_eq!(
            identity.source_archive_sha256(),
            &NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256
        );
        for (package, profile, archive) in [
            (
                "llama.cpp-latest",
                NATIVE_RUNTIME_PROFILE_SHA256,
                NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256,
            ),
            (
                NATIVE_RUNTIME_PACKAGE_ID,
                [0; 32],
                NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256,
            ),
            (
                NATIVE_RUNTIME_PACKAGE_ID,
                NATIVE_RUNTIME_PROFILE_SHA256,
                [0; 32],
            ),
        ] {
            assert_eq!(
                PinnedNativeRuntimeIdentity::verify(package, profile, archive),
                Err(NativeRuntimeContractError::RuntimeIdentity)
            );
        }
    }

    #[test]
    fn model_store_requires_non_root_exact_owner_and_private_mode() {
        assert_eq!(store().owner_uid(), 1000);
        assert_eq!(store().directory_identity_sha256(), &[3; 32]);
        for (identity, owner, launch, mode, expected) in [
            (
                [0; 32],
                1000,
                1000,
                0o700,
                NativeRuntimeContractError::ModelStoreIdentity,
            ),
            (
                [3; 32],
                0,
                0,
                0o700,
                NativeRuntimeContractError::ModelStoreOwner,
            ),
            (
                [3; 32],
                1000,
                1001,
                0o700,
                NativeRuntimeContractError::ModelStoreOwner,
            ),
            (
                [3; 32],
                1000,
                1000,
                0o750,
                NativeRuntimeContractError::ModelStoreMode,
            ),
        ] {
            assert_eq!(
                RestrictedModelStoreIdentity::verify(identity, owner, launch, mode),
                Err(expected)
            );
        }
    }

    #[test]
    fn resources_require_no_swap_one_slot_and_bounded_values() {
        let accepted = resources();
        assert_eq!(accepted.memory_bytes(), 16 * 1024 * 1024 * 1024);
        assert_eq!(accepted.tasks(), 32);
        assert_eq!(accepted.cpu_percent(), 1600);
        assert_eq!(accepted.runtime_seconds(), 900);
        assert_eq!(accepted.output_bytes(), 4 * 1024 * 1024);
        assert_eq!(accepted.swap_bytes(), 0);
        assert_eq!(accepted.parallel_slots(), 1);
        for values in [
            (1, 32, 1600, 900, 4 * 1024 * 1024, 0, 1),
            (
                16 * 1024 * 1024 * 1024,
                65,
                1600,
                900,
                4 * 1024 * 1024,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                3201,
                900,
                4 * 1024 * 1024,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                3601,
                4 * 1024 * 1024,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                900,
                16 * 1024 * 1024 + 1,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                900,
                4 * 1024 * 1024,
                1,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                900,
                4 * 1024 * 1024,
                0,
                2,
            ),
        ] {
            assert_eq!(
                NativeInferenceResourceEnvelope::verify(
                    values.0, values.1, values.2, values.3, values.4, values.5, values.6,
                ),
                Err(NativeRuntimeContractError::ResourceEnvelope)
            );
        }
    }

    #[test]
    fn topology_accepts_only_guarded_native_kernel_endpoint() {
        let native = LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [8; 32],
        )
        .expect("native endpoint");
        let topology = NativeInferenceTopology::verify(runtime(), store(), resources(), native)
            .expect("native topology");
        assert_eq!(topology.runtime(), runtime());
        assert_eq!(topology.model_store(), &store());
        assert_eq!(topology.resources(), resources());
        assert_eq!(
            topology.endpoint().client(),
            NetworkComponent::KernelNativeInferenceAdapter
        );

        let docker = LocalEndpointIdentity::new(
            NetworkComponent::KernelDockerInferenceAdapter,
            LocalTransport::GuardedLoopbackTcp,
            [8; 32],
        )
        .expect("Docker endpoint");
        assert_eq!(
            NativeInferenceTopology::verify(runtime(), store(), resources(), docker),
            Err(NativeRuntimeContractError::EndpointIdentity)
        );
    }
}
