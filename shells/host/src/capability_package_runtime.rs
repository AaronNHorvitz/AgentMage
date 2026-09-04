//! Deterministic capability-package hooks, catalog, recovery, and inventory projections.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::capability_package::{CapabilityCompatibility, CapabilityLifecycleAction};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageHookPhase {
    BeforeInstall,
    AfterInstall,
    BeforeEnable,
    AfterEnable,
    BeforeDisable,
    AfterDisable,
    BeforeUpdate,
    AfterUpdate,
    BeforeRemove,
    AfterRemove,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageHook {
    pub hook_id: String,
    pub phase: PackageHookPhase,
    pub order: u16,
    pub timeout_milliseconds: u64,
    pub cancellable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageHookOutcome {
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageHookObservation {
    pub hook_id: String,
    pub outcome: PackageHookOutcome,
    pub duration_milliseconds: u64,
    pub receipt_sha256: String,
    pub action_transaction_changed: bool,
    pub receipt_suppressed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageHookRun {
    pub ordered_hook_ids: Vec<String>,
    pub observations: Vec<PackageHookObservation>,
    pub safe_mode_required: bool,
    pub failure_isolated: bool,
    pub transaction_unchanged: bool,
    pub all_receipts_present: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogCapabilityKind {
    Tool,
    Skill,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCatalogCandidate {
    pub package_id: String,
    pub manifest_sha256: String,
    pub capability_id: String,
    pub kind: CatalogCapabilityKind,
    pub verified: bool,
    pub enabled: bool,
    pub removal_pending: bool,
    pub activation_reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCatalogEntry {
    pub capability_id: String,
    pub kind: CatalogCapabilityKind,
    pub package_id: String,
    pub manifest_sha256: String,
    pub active: bool,
    pub activation_reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCompatibilityReport {
    pub package_id: String,
    pub compatible: bool,
    pub mismatched_components: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageSafetyPolicy {
    pub public_auto_discovery: bool,
    pub automatic_download: bool,
    pub automatic_enable: bool,
    pub unsigned_execution: bool,
    pub broader_than_task_authority: bool,
    pub alternate_endpoint_bypass: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageLifecycleRecovery {
    pub package_id: String,
    pub action: CapabilityLifecycleAction,
    pub prior_manifest_sha256: Option<String>,
    pub target_manifest_sha256: Option<String>,
    pub committed_manifest_sha256: Option<String>,
    pub interrupted: bool,
    pub hook_failed: bool,
    pub cleanup_complete: bool,
    pub kernel_state_unchanged: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageInventorySnapshot {
    pub processes: Vec<String>,
    pub files: Vec<String>,
    pub tools: Vec<String>,
    pub network_domains: Vec<String>,
    pub storage_namespaces: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageInventoryDelta {
    pub processes: Vec<String>,
    pub files: Vec<String>,
    pub tools: Vec<String>,
    pub network_domains: Vec<String>,
    pub storage_namespaces: Vec<String>,
    pub declared_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageRuntimeError {
    InvalidInput,
    HookBoundary,
    CatalogConflict,
    PolicyDenied,
    RecoveryDenied,
    InventoryMismatch,
}
impl PackageRuntimeError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "package-runtime.input.invalid",
            Self::HookBoundary => "package-runtime.hook.boundary",
            Self::CatalogConflict => "package-runtime.catalog.conflict",
            Self::PolicyDenied => "package-runtime.policy.denied",
            Self::RecoveryDenied => "package-runtime.recovery.denied",
            Self::InventoryMismatch => "package-runtime.inventory.mismatch",
        }
    }
}
impl std::fmt::Display for PackageRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for PackageRuntimeError {}

fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn sorted_unique(values: &[String]) -> bool {
    values.windows(2).all(|p| p[0] < p[1]) && values.iter().all(|v| id(v))
}

pub fn resolve_package_hooks(
    hooks: &[PackageHook],
    observations: &[PackageHookObservation],
) -> Result<PackageHookRun, PackageRuntimeError> {
    if hooks.len() > 64
        || hooks.windows(2).any(|p| {
            (p[0].phase, p[0].order, p[0].hook_id.as_str())
                >= (p[1].phase, p[1].order, p[1].hook_id.as_str())
        })
        || hooks.iter().any(|h| {
            !id(&h.hook_id) || h.timeout_milliseconds == 0 || h.timeout_milliseconds > 60_000
        })
        || observations.len() != hooks.len()
    {
        return Err(PackageRuntimeError::InvalidInput);
    }
    let map: BTreeMap<_, _> = observations
        .iter()
        .map(|o| (o.hook_id.as_str(), o))
        .collect();
    if map.len() != observations.len() {
        return Err(PackageRuntimeError::InvalidInput);
    }
    let mut safe = false;
    for hook in hooks {
        let observation = map
            .get(hook.hook_id.as_str())
            .ok_or(PackageRuntimeError::InvalidInput)?;
        if !sha(&observation.receipt_sha256)
            || observation.duration_milliseconds > hook.timeout_milliseconds
                && observation.outcome != PackageHookOutcome::TimedOut
            || observation.outcome == PackageHookOutcome::Cancelled && !hook.cancellable
            || observation.action_transaction_changed
            || observation.receipt_suppressed
        {
            return Err(PackageRuntimeError::HookBoundary);
        }
        safe |= observation.outcome != PackageHookOutcome::Completed;
    }
    Ok(PackageHookRun {
        ordered_hook_ids: hooks.iter().map(|h| h.hook_id.clone()).collect(),
        observations: hooks
            .iter()
            .map(|h| (*map[h.hook_id.as_str()]).clone())
            .collect(),
        safe_mode_required: safe,
        failure_isolated: true,
        transaction_unchanged: true,
        all_receipts_present: true,
    })
}

pub fn build_package_catalog(
    mut candidates: Vec<PackageCatalogCandidate>,
    safe_mode: bool,
) -> Result<Vec<PackageCatalogEntry>, PackageRuntimeError> {
    candidates.sort_by(|a, b| {
        (a.capability_id.as_str(), a.kind, a.package_id.as_str()).cmp(&(
            b.capability_id.as_str(),
            b.kind,
            b.package_id.as_str(),
        ))
    });
    if candidates.iter().any(|c| {
        !id(&c.package_id)
            || !id(&c.capability_id)
            || !sha(&c.manifest_sha256)
            || c.activation_reason.trim().is_empty()
            || c.activation_reason.len() > 256
    }) || candidates
        .windows(2)
        .any(|p| p[0].capability_id == p[1].capability_id && p[0].kind == p[1].kind)
    {
        return Err(PackageRuntimeError::CatalogConflict);
    }
    Ok(candidates
        .into_iter()
        .map(|c| PackageCatalogEntry {
            capability_id: c.capability_id,
            kind: c.kind,
            package_id: c.package_id,
            manifest_sha256: c.manifest_sha256,
            active: !safe_mode && c.verified && c.enabled && !c.removal_pending,
            activation_reason: c.activation_reason,
        })
        .collect())
}

pub fn assess_package_compatibility(
    package_id: &str,
    declared: &CapabilityCompatibility,
    current: &CapabilityCompatibility,
) -> Result<PackageCompatibilityReport, PackageRuntimeError> {
    if !id(package_id) {
        return Err(PackageRuntimeError::InvalidInput);
    }
    let pairs = [
        ("kernel", &declared.kernel_version, &current.kernel_version),
        (
            "tool_protocol",
            &declared.tool_protocol_version,
            &current.tool_protocol_version,
        ),
        (
            "configuration",
            &declared.configuration_version,
            &current.configuration_version,
        ),
        ("memory", &declared.memory_version, &current.memory_version),
        (
            "storage",
            &declared.storage_version,
            &current.storage_version,
        ),
        ("shell", &declared.shell_version, &current.shell_version),
        ("policy", &declared.policy_version, &current.policy_version),
    ];
    let mismatched_components = pairs
        .into_iter()
        .filter(|(_, a, b)| a != b)
        .map(|(name, _, _)| name.to_owned())
        .collect::<Vec<_>>();
    Ok(PackageCompatibilityReport {
        package_id: package_id.to_owned(),
        compatible: mismatched_components.is_empty(),
        mismatched_components,
    })
}

pub fn validate_package_safety_policy(
    policy: &PackageSafetyPolicy,
) -> Result<(), PackageRuntimeError> {
    if policy.public_auto_discovery
        || policy.automatic_download
        || policy.automatic_enable
        || policy.unsigned_execution
        || policy.broader_than_task_authority
        || policy.alternate_endpoint_bypass
    {
        return Err(PackageRuntimeError::PolicyDenied);
    }
    Ok(())
}

pub fn recover_package_lifecycle(
    input: &PackageLifecycleRecovery,
) -> Result<Option<String>, PackageRuntimeError> {
    if !id(&input.package_id)
        || input
            .prior_manifest_sha256
            .as_deref()
            .is_some_and(|v| !sha(v))
        || input
            .target_manifest_sha256
            .as_deref()
            .is_some_and(|v| !sha(v))
        || input
            .committed_manifest_sha256
            .as_deref()
            .is_some_and(|v| !sha(v))
        || !input.kernel_state_unchanged
        || input.interrupted && !input.cleanup_complete
    {
        return Err(PackageRuntimeError::RecoveryDenied);
    }
    let expected = if input.interrupted || input.hook_failed {
        input.prior_manifest_sha256.clone()
    } else {
        input.target_manifest_sha256.clone()
    };
    if input.committed_manifest_sha256 != expected {
        return Err(PackageRuntimeError::RecoveryDenied);
    }
    Ok(expected)
}

fn difference(after: &[String], before: &[String]) -> Vec<String> {
    after
        .iter()
        .filter(|v| before.binary_search(v).is_err())
        .cloned()
        .collect()
}
pub fn verify_inventory_delta(
    before: &PackageInventorySnapshot,
    after: &PackageInventorySnapshot,
    declared: &PackageInventorySnapshot,
) -> Result<PackageInventoryDelta, PackageRuntimeError> {
    for list in [
        &before.processes,
        &before.files,
        &before.tools,
        &before.network_domains,
        &before.storage_namespaces,
        &after.processes,
        &after.files,
        &after.tools,
        &after.network_domains,
        &after.storage_namespaces,
        &declared.processes,
        &declared.files,
        &declared.tools,
        &declared.network_domains,
        &declared.storage_namespaces,
    ] {
        if !sorted_unique(list) {
            return Err(PackageRuntimeError::InvalidInput);
        }
    }
    let delta = PackageInventoryDelta {
        processes: difference(&after.processes, &before.processes),
        files: difference(&after.files, &before.files),
        tools: difference(&after.tools, &before.tools),
        network_domains: difference(&after.network_domains, &before.network_domains),
        storage_namespaces: difference(&after.storage_namespaces, &before.storage_namespaces),
        declared_only: false,
    };
    if delta.processes != declared.processes
        || delta.files != declared.files
        || delta.tools != declared.tools
        || delta.network_domains != declared.network_domains
        || delta.storage_namespaces != declared.storage_namespaces
    {
        return Err(PackageRuntimeError::InventoryMismatch);
    }
    Ok(PackageInventoryDelta {
        declared_only: true,
        ..delta
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn compat() -> CapabilityCompatibility {
        CapabilityCompatibility {
            kernel_version: "1.0.0".into(),
            tool_protocol_version: "1.0.0".into(),
            configuration_version: "1.0.0".into(),
            memory_version: "1.0.0".into(),
            storage_version: "1.0.0".into(),
            shell_version: "1.0.0".into(),
            policy_version: "1.0.0".into(),
        }
    }
    #[test]
    fn hooks_order_and_isolate_failure() {
        let hooks = vec![PackageHook {
            hook_id: "hook-1".into(),
            phase: PackageHookPhase::BeforeEnable,
            order: 1,
            timeout_milliseconds: 10,
            cancellable: true,
        }];
        let obs = vec![PackageHookObservation {
            hook_id: "hook-1".into(),
            outcome: PackageHookOutcome::Failed,
            duration_milliseconds: 1,
            receipt_sha256: A.into(),
            action_transaction_changed: false,
            receipt_suppressed: false,
        }];
        let run = resolve_package_hooks(&hooks, &obs).unwrap();
        assert!(run.safe_mode_required && run.failure_isolated && run.all_receipts_present);
    }
    #[test]
    fn hook_mutation_or_suppression_fails() {
        let hooks = vec![PackageHook {
            hook_id: "hook-1".into(),
            phase: PackageHookPhase::BeforeEnable,
            order: 1,
            timeout_milliseconds: 10,
            cancellable: true,
        }];
        for (changed, suppressed) in [(true, false), (false, true)] {
            let obs = vec![PackageHookObservation {
                hook_id: "hook-1".into(),
                outcome: PackageHookOutcome::Failed,
                duration_milliseconds: 1,
                receipt_sha256: A.into(),
                action_transaction_changed: changed,
                receipt_suppressed: suppressed,
            }];
            assert!(resolve_package_hooks(&hooks, &obs).is_err());
        }
    }
    #[test]
    fn catalog_explains_active_provider_and_safe_mode_disables() {
        let candidate = PackageCatalogCandidate {
            package_id: "package-1".into(),
            manifest_sha256: A.into(),
            capability_id: "tool-1".into(),
            kind: CatalogCapabilityKind::Tool,
            verified: true,
            enabled: true,
            removal_pending: false,
            activation_reason: "approved exact package".into(),
        };
        assert!(build_package_catalog(vec![candidate.clone()], false).unwrap()[0].active);
        assert!(!build_package_catalog(vec![candidate], true).unwrap()[0].active);
    }
    #[test]
    fn all_seven_compatibility_versions_are_compared() {
        assert!(
            assess_package_compatibility("package-1", &compat(), &compat())
                .unwrap()
                .compatible
        );
        let mut current = compat();
        current.policy_version = "2.0.0".into();
        assert_eq!(
            assess_package_compatibility("package-1", &compat(), &current)
                .unwrap()
                .mismatched_components,
            vec!["policy"]
        );
    }
    #[test]
    fn prohibited_automation_is_rejected() {
        let mut policy = PackageSafetyPolicy {
            public_auto_discovery: false,
            automatic_download: false,
            automatic_enable: false,
            unsigned_execution: false,
            broader_than_task_authority: false,
            alternate_endpoint_bypass: false,
        };
        assert_eq!(validate_package_safety_policy(&policy), Ok(()));
        policy.automatic_enable = true;
        assert_eq!(
            validate_package_safety_policy(&policy),
            Err(PackageRuntimeError::PolicyDenied)
        );
    }
    #[test]
    fn interrupted_lifecycle_restores_prior_exact_state() {
        let input = PackageLifecycleRecovery {
            package_id: "package-1".into(),
            action: CapabilityLifecycleAction::Update,
            prior_manifest_sha256: Some(A.into()),
            target_manifest_sha256: Some("b".repeat(64)),
            committed_manifest_sha256: Some(A.into()),
            interrupted: true,
            hook_failed: false,
            cleanup_complete: true,
            kernel_state_unchanged: true,
        };
        assert_eq!(recover_package_lifecycle(&input).unwrap(), Some(A.into()));
    }
    #[test]
    fn incomplete_cleanup_or_kernel_change_fails() {
        let mut input = PackageLifecycleRecovery {
            package_id: "package-1".into(),
            action: CapabilityLifecycleAction::Enable,
            prior_manifest_sha256: Some(A.into()),
            target_manifest_sha256: Some(A.into()),
            committed_manifest_sha256: Some(A.into()),
            interrupted: true,
            hook_failed: false,
            cleanup_complete: false,
            kernel_state_unchanged: true,
        };
        assert!(recover_package_lifecycle(&input).is_err());
        input.cleanup_complete = true;
        input.kernel_state_unchanged = false;
        assert!(recover_package_lifecycle(&input).is_err());
    }
    #[test]
    fn inventory_delta_must_equal_declaration() {
        let before = PackageInventorySnapshot::default();
        let after = PackageInventorySnapshot {
            tools: vec!["tool-1".into()],
            ..Default::default()
        };
        assert!(
            verify_inventory_delta(&before, &after, &after)
                .unwrap()
                .declared_only
        );
        let declared = PackageInventorySnapshot {
            tools: vec!["tool-2".into()],
            ..Default::default()
        };
        assert_eq!(
            verify_inventory_delta(&before, &after, &declared),
            Err(PackageRuntimeError::InventoryMismatch)
        );
    }
}
