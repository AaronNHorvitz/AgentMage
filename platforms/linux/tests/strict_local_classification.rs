use std::collections::BTreeSet;
use std::fs;
use std::net::IpAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_kernel_contracts::{
    LocalEndpointIdentity, LocalTransport, NetworkComponent, NetworkDestinationClass,
    NetworkObservation, StorageFilesystemClass, classify_ip_destination,
};
use agentmage_kernel_engine::strict_local::{StrictLocalNetworkDecision, StrictLocalNetworkPolicy};
use agentmage_platform_linux::{
    LinuxStrictLocalRootErrorKind, LinuxStrictLocalRootInspector, classify_linux_filesystem_magic,
};
use serde_json::Value;

const BOUNDARY_FIXTURES: &str =
    include_str!("../../../fixtures/strict-local-boundary/v1/classification-fixtures.json");
const STORAGE_FIXTURES: &str =
    include_str!("../../../fixtures/strict-local-storage/v1/detection-fixtures.json");
static TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryFixtures {
    schema_version: u32,
    storage_fixture: String,
    ip_cases: Vec<IpCase>,
    policy_cases: Vec<PolicyCase>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct IpCase {
    id: String,
    address: String,
    expected: NetworkDestinationClass,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyCase {
    id: String,
    topology: String,
    component: NetworkComponent,
    destination: NetworkDestinationClass,
    transport: Option<LocalTransport>,
    endpoint: String,
    peer_authenticated: bool,
    expected: String,
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("agentmage-s010-ut01-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("fixture root creates");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("fixture root is private");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("fixture root removes");
    }
}

fn endpoint(topology: &str) -> LocalEndpointIdentity {
    match topology {
        "native" => LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [7; 32],
        ),
        "docker" => LocalEndpointIdentity::new(
            NetworkComponent::KernelDockerInferenceAdapter,
            LocalTransport::GuardedLoopbackTcp,
            [8; 32],
        ),
        _ => panic!("unknown fixture topology"),
    }
    .expect("fixture endpoint is valid")
}

fn endpoint_observation(kind: &str, topology: &str) -> Option<[u8; 32]> {
    match kind {
        "match" => Some(if topology == "native" {
            [7; 32]
        } else {
            [8; 32]
        }),
        "mismatch" => Some([9; 32]),
        "none" => None,
        _ => panic!("unknown endpoint fixture"),
    }
}

fn expected_filesystem(label: &str) -> StorageFilesystemClass {
    match label {
        "local" => StorageFilesystemClass::Local,
        "remote" => StorageFilesystemClass::Remote,
        "fuse" => StorageFilesystemClass::Fuse,
        "unknown" => StorageFilesystemClass::Unknown,
        _ => panic!("unknown filesystem fixture"),
    }
}

#[test]
fn s_010_ut01_classifies_network_storage_and_sync_fixtures() {
    let fixtures: BoundaryFixtures =
        serde_json::from_str(BOUNDARY_FIXTURES).expect("boundary fixtures parse");
    assert_eq!(fixtures.schema_version, 1);
    assert_eq!(
        fixtures.storage_fixture,
        "fixtures/strict-local-storage/v1/detection-fixtures.json"
    );
    assert_eq!(fixtures.ip_cases.len(), 13);
    assert_eq!(fixtures.policy_cases.len(), 15);

    let mut identifiers = BTreeSet::new();
    for case in fixtures.ip_cases {
        assert!(identifiers.insert(case.id));
        let address: IpAddr = case.address.parse().expect("fixture address parses");
        assert_eq!(classify_ip_destination(address), case.expected);
    }

    let mut allowed = 0_usize;
    let mut blocked = 0_usize;
    for case in fixtures.policy_cases {
        assert!(identifiers.insert(case.id));
        let policy = StrictLocalNetworkPolicy::new(endpoint(&case.topology));
        let decision = policy.evaluate(&NetworkObservation {
            component: case.component,
            destination: case.destination,
            transport: case.transport,
            endpoint_sha256: endpoint_observation(&case.endpoint, &case.topology),
            peer_authenticated: case.peer_authenticated,
            attempted_bytes: 1,
        });
        match case.expected.as_str() {
            "allow" => {
                assert!(matches!(decision, StrictLocalNetworkDecision::Allow { .. }));
                allowed += 1;
            }
            "block" => {
                assert!(matches!(decision, StrictLocalNetworkDecision::Block { .. }));
                blocked += 1;
            }
            _ => panic!("unknown policy expectation"),
        }
    }
    assert_eq!((allowed, blocked), (2, 13));

    let storage: Value = serde_json::from_str(STORAGE_FIXTURES).expect("storage fixtures parse");
    let magic_cases = storage["filesystem_magic"]
        .as_array()
        .expect("filesystem fixtures");
    assert_eq!(magic_cases.len(), 22);
    for case in magic_cases {
        let encoded = case["magic"].as_str().expect("filesystem magic");
        let magic =
            u64::from_str_radix(encoded.strip_prefix("0x").expect("hexadecimal fixture"), 16)
                .expect("valid filesystem magic");
        assert_eq!(
            classify_linux_filesystem_magic(magic),
            expected_filesystem(case["expected"].as_str().expect("filesystem expectation"))
        );
    }

    for family in ["provider_components", "root_sentinels"] {
        for case in storage[family]
            .as_array()
            .expect("synchronization fixtures")
        {
            let root = TestDirectory::new();
            let candidate = if family == "provider_components" {
                let candidate = root
                    .0
                    .join(case["component"].as_str().expect("provider component"));
                fs::create_dir(&candidate).expect("provider fixture creates");
                fs::set_permissions(&candidate, fs::Permissions::from_mode(0o700))
                    .expect("provider fixture is private");
                candidate
            } else {
                fs::create_dir(root.0.join(case["name"].as_str().expect("sentinel name")))
                    .expect("sentinel fixture creates");
                root.0.clone()
            };
            let held = LinuxStrictLocalRootInspector::inspect(&candidate)
                .expect("synchronization fixture inspects");
            assert_eq!(
                held.revalidate()
                    .expect_err("synchronization fixture rejects")
                    .kind(),
                LinuxStrictLocalRootErrorKind::CloudSynchronized
            );
        }
    }
}
