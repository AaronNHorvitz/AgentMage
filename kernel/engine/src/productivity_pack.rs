//! Fail-closed optional productivity-pack contracts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// The only support states exposed by a productivity pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductivitySupportState {
    /// Exact provider tuple has current conformance evidence.
    Supported,
    /// Exact limitations are visible and the remaining operations are current.
    Degraded,
    /// Provider tuple is known but cannot be registered.
    Unsupported,
    /// Installed pack is intentionally inactive.
    Disabled,
    /// Provider authority was revoked and registration is empty.
    Revoked,
    /// Pack and all runtime-owned residue were removed.
    Removed,
}

/// Closed first-GA productivity-pack families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductivityPackKind {
    /// Mail, chat, calendar, task, and document operations.
    Communications,
    /// Read-only institution data and local-only financial drafts.
    Finance,
    /// Strictly read-only cloud observations.
    CloudObserver,
}

/// Closed operation classes; money movement and cloud mutation are unrepresentable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductivityOperation {
    /// Provider read.
    Read,
    /// Local-only draft.
    Draft,
    /// Exact confirmed communication write.
    CommunicationWrite,
    /// Read-only financial observation.
    FinancialRead,
    /// Read-only cloud observation.
    CloudRead,
}

/// Exact lifecycle states independent of provider support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductivityLifecycleState {
    /// No pack is installed.
    Absent,
    /// Bytes are installed but registration is empty.
    Installed,
    /// Pack may expose only manifest-declared local discovery.
    Enabled,
    /// One exact provider account is authenticated.
    Authenticated,
    /// One bounded synchronization is active.
    Synchronizing,
    /// No new operation can begin.
    Suspended,
    /// Provider grant has been revoked.
    Revoked,
    /// Pack remains installed with empty registration.
    Disabled,
    /// Pack-owned state is removed.
    Removed,
    /// Optional-pack authority is empty and the local core remains available.
    StrictLocal,
}

/// User-visible lifecycle actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductivityLifecycleAction {
    /// Install verified bytes without enabling them.
    Install,
    /// Enable local manifest discovery.
    Enable,
    /// Authenticate one exact account.
    Authenticate,
    /// Start bounded synchronization.
    Synchronize,
    /// Stop new work without deleting state.
    Suspend,
    /// Revoke provider authority.
    Revoke,
    /// Disable all registration.
    Disable,
    /// Remove the pack and its owned residue.
    Remove,
    /// Re-establish the zero-authority strict-local profile.
    RestoreStrictLocal,
}

/// Complete runtime-owned inventory. Credential values are intentionally absent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductivityRuntimeInventory {
    /// Pack-owned component identities.
    pub components: Vec<String>,
    /// Supervised process identities.
    pub processes: Vec<String>,
    /// Exact socket identities.
    pub sockets: Vec<String>,
    /// Secret-store references, never values.
    pub credential_references: Vec<String>,
    /// Disposable cache identities.
    pub caches: Vec<String>,
    /// Provider cursor identities.
    pub cursors: Vec<String>,
    /// Schedule identities.
    pub schedules: Vec<String>,
    /// Webhook identities.
    pub webhooks: Vec<String>,
    /// Disposable index identities.
    pub indexes: Vec<String>,
    /// Retained-data class identities.
    pub retained_data: Vec<String>,
    /// Registered tool identities.
    pub tools: Vec<String>,
}

impl ProductivityRuntimeInventory {
    fn is_empty(&self) -> bool {
        self.components.is_empty()
            && self.processes.is_empty()
            && self.sockets.is_empty()
            && self.credential_references.is_empty()
            && self.caches.is_empty()
            && self.cursors.is_empty()
            && self.schedules.is_empty()
            && self.webhooks.is_empty()
            && self.indexes.is_empty()
            && self.retained_data.is_empty()
            && self.tools.is_empty()
    }
}

/// Versioned provider and operation manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductivityPackManifest {
    /// Exact supported schema version; currently one.
    pub schema_version: u32,
    /// Stable pack identity.
    pub pack_id: String,
    /// Pack family.
    pub kind: ProductivityPackKind,
    /// Exact provider family and version identity.
    pub provider_id: String,
    /// Exact account reference.
    pub account_id: String,
    /// Closed provider object classes.
    pub objects: Vec<String>,
    /// Closed registered operation classes.
    pub operations: Vec<ProductivityOperation>,
    /// Declared event modes.
    pub events: Vec<String>,
    /// Least-privilege scopes.
    pub scopes: Vec<String>,
    /// Data classification ceiling.
    pub classification: String,
    /// Network boundary identity.
    pub network: String,
    /// Retention policy identity.
    pub retention: String,
    /// Recovery policy identity.
    pub recovery: String,
    /// Removal policy identity.
    pub removal: String,
    /// Current support state.
    pub support: ProductivitySupportState,
    /// Current lifecycle state.
    pub lifecycle: ProductivityLifecycleState,
    /// Exact current conformance-evidence digest.
    pub evidence_sha256: String,
    /// Complete pack-owned inventory.
    pub inventory: ProductivityRuntimeInventory,
}

/// One explicitly allowlisted cross-pack disclosure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductivityDataFlowEdge {
    /// Source pack.
    pub source: ProductivityPackKind,
    /// Destination pack.
    pub destination: ProductivityPackKind,
    /// Exact data classification.
    pub classification: String,
    /// Exact bounded purpose.
    pub purpose: String,
}

/// Stable refusal classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductivityPackError {
    /// Unknown schema or malformed/contradictory manifest.
    InvalidManifest,
    /// State does not permit registration.
    Inactive,
    /// Lifecycle edge is not allowed.
    InvalidTransition,
    /// Cross-pack disclosure is undeclared.
    UndeclaredDataFlow,
}

/// Validate the complete closed manifest without creating authority.
pub fn validate_productivity_manifest(
    value: &ProductivityPackManifest,
) -> Result<(), ProductivityPackError> {
    if value.schema_version != 1
        || !valid_id(&value.pack_id)
        || !valid_id(&value.provider_id)
        || !valid_id(&value.account_id)
        || !valid_id(&value.classification)
        || !valid_id(&value.network)
        || !valid_id(&value.retention)
        || !valid_id(&value.recovery)
        || !valid_id(&value.removal)
        || !valid_sha256(&value.evidence_sha256)
        || value.objects.is_empty()
        || value.operations.is_empty()
        || !unique(&value.objects)
        || !unique(&value.events)
        || !unique(&value.scopes)
        || !unique(&value.operations)
    {
        return Err(ProductivityPackError::InvalidManifest);
    }
    let permitted = match value.kind {
        ProductivityPackKind::Communications => value.operations.iter().all(|operation| {
            matches!(
                operation,
                ProductivityOperation::Read
                    | ProductivityOperation::Draft
                    | ProductivityOperation::CommunicationWrite
            )
        }),
        ProductivityPackKind::Finance => value.operations.iter().all(|operation| {
            matches!(
                operation,
                ProductivityOperation::Read
                    | ProductivityOperation::Draft
                    | ProductivityOperation::FinancialRead
            )
        }),
        ProductivityPackKind::CloudObserver => value.operations.iter().all(|operation| {
            matches!(
                operation,
                ProductivityOperation::Read | ProductivityOperation::CloudRead
            )
        }),
    };
    if !permitted {
        return Err(ProductivityPackError::InvalidManifest);
    }
    if matches!(
        value.support,
        ProductivitySupportState::Unsupported
            | ProductivitySupportState::Disabled
            | ProductivitySupportState::Revoked
            | ProductivitySupportState::Removed
    ) && !value.inventory.is_empty()
    {
        return Err(ProductivityPackError::InvalidManifest);
    }
    Ok(())
}

/// Discover only exact operations from an active, supported manifest.
pub fn discover_productivity_operations(
    value: &ProductivityPackManifest,
) -> Result<Vec<ProductivityOperation>, ProductivityPackError> {
    validate_productivity_manifest(value)?;
    if !matches!(
        value.support,
        ProductivitySupportState::Supported | ProductivitySupportState::Degraded
    ) || !matches!(
        value.lifecycle,
        ProductivityLifecycleState::Enabled
            | ProductivityLifecycleState::Authenticated
            | ProductivityLifecycleState::Synchronizing
    ) {
        return Err(ProductivityPackError::Inactive);
    }
    Ok(value.operations.clone())
}

/// Apply one closed lifecycle action without fallback edges.
pub fn transition_productivity_lifecycle(
    current: ProductivityLifecycleState,
    action: ProductivityLifecycleAction,
) -> Result<ProductivityLifecycleState, ProductivityPackError> {
    use ProductivityLifecycleAction as A;
    use ProductivityLifecycleState as S;
    match (current, action) {
        (S::Absent | S::Removed | S::StrictLocal, A::Install) => Ok(S::Installed),
        (S::Installed | S::Disabled, A::Enable) => Ok(S::Enabled),
        (S::Enabled, A::Authenticate) => Ok(S::Authenticated),
        (S::Authenticated, A::Synchronize) => Ok(S::Synchronizing),
        (S::Enabled | S::Authenticated | S::Synchronizing, A::Suspend) => Ok(S::Suspended),
        (S::Authenticated | S::Synchronizing | S::Suspended, A::Revoke) => Ok(S::Revoked),
        (
            S::Installed
            | S::Enabled
            | S::Authenticated
            | S::Synchronizing
            | S::Suspended
            | S::Revoked,
            A::Disable,
        ) => Ok(S::Disabled),
        (
            S::Installed
            | S::Enabled
            | S::Authenticated
            | S::Synchronizing
            | S::Suspended
            | S::Revoked
            | S::Disabled,
            A::Remove,
        ) => Ok(S::Removed),
        (S::Absent | S::Removed | S::Disabled | S::Revoked, A::RestoreStrictLocal) => {
            Ok(S::StrictLocal)
        }
        _ => Err(ProductivityPackError::InvalidTransition),
    }
}

/// Admit only an exact member of the closed allowlist.
pub fn admit_productivity_data_flow(
    edge: &ProductivityDataFlowEdge,
    allowlist: &[ProductivityDataFlowEdge],
) -> Result<(), ProductivityPackError> {
    if !valid_id(&edge.classification)
        || !valid_id(&edge.purpose)
        || edge.source == edge.destination
        || !allowlist.contains(edge)
    {
        return Err(ProductivityPackError::UndeclaredDataFlow);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn unique<T: Ord + Clone>(values: &[T]) -> bool {
    values.iter().cloned().collect::<BTreeSet<_>>().len() == values.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(kind: ProductivityPackKind) -> ProductivityPackManifest {
        ProductivityPackManifest {
            schema_version: 1,
            pack_id: "productivity-pack".into(),
            kind,
            provider_id: "provider-v1".into(),
            account_id: "account-reference".into(),
            objects: vec!["record".into()],
            operations: match kind {
                ProductivityPackKind::Communications => vec![
                    ProductivityOperation::Read,
                    ProductivityOperation::CommunicationWrite,
                ],
                ProductivityPackKind::Finance => vec![
                    ProductivityOperation::FinancialRead,
                    ProductivityOperation::Draft,
                ],
                ProductivityPackKind::CloudObserver => vec![ProductivityOperation::CloudRead],
            },
            events: vec!["poll".into()],
            scopes: vec!["least-privilege".into()],
            classification: "private".into(),
            network: "exact-provider".into(),
            retention: "pack-owned".into(),
            recovery: "reconcile-before-retry".into(),
            removal: "remove-all-owned-state".into(),
            support: ProductivitySupportState::Supported,
            lifecycle: ProductivityLifecycleState::Enabled,
            evidence_sha256: "a".repeat(64),
            inventory: ProductivityRuntimeInventory::default(),
        }
    }

    #[test]
    fn each_first_ga_family_exposes_only_its_closed_operations() {
        for kind in [
            ProductivityPackKind::Communications,
            ProductivityPackKind::Finance,
            ProductivityPackKind::CloudObserver,
        ] {
            let value = manifest(kind);
            assert_eq!(
                discover_productivity_operations(&value),
                Ok(value.operations)
            );
        }
    }

    #[test]
    fn money_movement_and_cloud_mutation_have_no_operation_variant() {
        let encoded = serde_json::to_string(&manifest(ProductivityPackKind::Finance)).unwrap();
        assert!(!encoded.contains("money_movement"));
        assert!(!encoded.contains("cloud_mutation"));
        let mut cloud = manifest(ProductivityPackKind::CloudObserver);
        cloud.operations = vec![ProductivityOperation::CommunicationWrite];
        assert_eq!(
            validate_productivity_manifest(&cloud),
            Err(ProductivityPackError::InvalidManifest)
        );
    }

    #[test]
    fn unknown_version_duplicate_and_contradictory_manifest_fail_closed() {
        let mut value = manifest(ProductivityPackKind::Finance);
        value.schema_version = 2;
        assert_eq!(
            validate_productivity_manifest(&value),
            Err(ProductivityPackError::InvalidManifest)
        );
        value.schema_version = 1;
        value.operations.push(ProductivityOperation::FinancialRead);
        assert_eq!(
            validate_productivity_manifest(&value),
            Err(ProductivityPackError::InvalidManifest)
        );
        value.operations.pop();
        value.support = ProductivitySupportState::Disabled;
        value.inventory.tools.push("hidden-tool".into());
        assert_eq!(
            validate_productivity_manifest(&value),
            Err(ProductivityPackError::InvalidManifest)
        );
    }

    #[test]
    fn inactive_support_and_lifecycle_never_register_operations() {
        for support in [
            ProductivitySupportState::Unsupported,
            ProductivitySupportState::Disabled,
            ProductivitySupportState::Revoked,
            ProductivitySupportState::Removed,
        ] {
            let mut value = manifest(ProductivityPackKind::Communications);
            value.support = support;
            assert_eq!(
                discover_productivity_operations(&value),
                Err(ProductivityPackError::Inactive)
            );
        }
        let mut absent = manifest(ProductivityPackKind::Communications);
        absent.lifecycle = ProductivityLifecycleState::Absent;
        assert_eq!(
            discover_productivity_operations(&absent),
            Err(ProductivityPackError::Inactive)
        );
    }

    #[test]
    fn lifecycle_has_only_explicit_edges_and_restores_strict_local() {
        use ProductivityLifecycleAction as A;
        use ProductivityLifecycleState as S;
        let path = [
            (S::Absent, A::Install, S::Installed),
            (S::Installed, A::Enable, S::Enabled),
            (S::Enabled, A::Authenticate, S::Authenticated),
            (S::Authenticated, A::Synchronize, S::Synchronizing),
            (S::Synchronizing, A::Suspend, S::Suspended),
            (S::Suspended, A::Revoke, S::Revoked),
            (S::Revoked, A::Disable, S::Disabled),
            (S::Disabled, A::Remove, S::Removed),
            (S::Removed, A::RestoreStrictLocal, S::StrictLocal),
        ];
        for (current, action, expected) in path {
            assert_eq!(
                transition_productivity_lifecycle(current, action),
                Ok(expected)
            );
        }
        assert_eq!(
            transition_productivity_lifecycle(S::Absent, A::Synchronize),
            Err(ProductivityPackError::InvalidTransition)
        );
    }

    #[test]
    fn only_exact_allowlisted_cross_pack_flow_is_admitted() {
        let edge = ProductivityDataFlowEdge {
            source: ProductivityPackKind::Communications,
            destination: ProductivityPackKind::Finance,
            classification: "private".into(),
            purpose: "user-confirmed-receipt-match".into(),
        };
        assert_eq!(
            admit_productivity_data_flow(&edge, std::slice::from_ref(&edge)),
            Ok(())
        );
        let mut changed = edge.clone();
        changed.purpose = "model-inferred-disclosure".into();
        assert_eq!(
            admit_productivity_data_flow(&changed, &[edge]),
            Err(ProductivityPackError::UndeclaredDataFlow)
        );
    }
}
