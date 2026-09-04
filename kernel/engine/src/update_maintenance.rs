//! Explicit offline update staging and fail-closed supply-chain maintenance.

use std::cmp::Ordering;
use std::collections::BTreeSet;

/// Exact signed update package presented for offline staging.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePackage {
    /// Stable product identity.
    pub product_id: String,
    /// Exact installed semantic version.
    pub from_version: String,
    /// Exact proposed semantic version.
    pub to_version: String,
    /// Digest of the immutable package bytes.
    pub package_sha256: String,
    /// Digest of the signed manifest.
    pub manifest_sha256: String,
    /// Pinned signer identity.
    pub signer_key_id: String,
    /// Detached signature digest retained with the preview.
    pub signature_sha256: String,
    /// Exact human-readable change preview digest.
    pub change_preview_sha256: String,
    /// Exact compatibility result digest.
    pub compatibility_sha256: String,
    /// Exact last-known-good rollback point.
    pub rollback_point_sha256: String,
    /// Signature was verified against the pinned signer outside this policy object.
    pub signature_verified: bool,
    /// Package and manifest digests were independently recomputed.
    pub integrity_verified: bool,
    /// Compatibility checks passed for the exact installed state.
    pub compatibility_passed: bool,
    /// An automatic remote update check was attempted.
    pub automatic_remote_check: bool,
}

/// Immutable authorization required after staging and exact revalidation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateActivationGrant {
    /// Digest of the exact preview the user approved.
    pub approved_preview_sha256: String,
    /// Digest of the exact package approved for activation.
    pub approved_package_sha256: String,
    /// Digest of the exact staged tree.
    pub staged_tree_sha256: String,
    /// User explicitly approved activation after seeing the preview.
    pub explicitly_approved: bool,
}

/// Update lifecycle refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// Identity, version, or digest shape is invalid.
    InvalidPackage,
    /// Signature, integrity, or compatibility evidence is absent.
    VerificationFailed,
    /// The package is a downgrade or does not advance the installed version.
    Downgrade,
    /// Any automatic remote check is prohibited.
    RemoteCheckProhibited,
    /// Activation differs from the staged and approved preview.
    StaleApproval,
}

/// Closed activation decision; actual filesystem mutation remains outside this module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedUpdate {
    /// Exact package digest.
    pub package_sha256: String,
    /// Exact approved preview digest.
    pub preview_sha256: String,
    /// Exact staged tree digest.
    pub staged_tree_sha256: String,
    /// Exact rollback point retained before activation.
    pub rollback_point_sha256: String,
}

/// Validate an offline package before any staging side effect.
pub fn validate_update_package(value: &UpdatePackage) -> Result<(), UpdateError> {
    if value.automatic_remote_check {
        return Err(UpdateError::RemoteCheckProhibited);
    }
    if !valid_id(&value.product_id)
        || !valid_id(&value.signer_key_id)
        || parse_version(&value.from_version).is_none()
        || parse_version(&value.to_version).is_none()
        || ![
            &value.package_sha256,
            &value.manifest_sha256,
            &value.signature_sha256,
            &value.change_preview_sha256,
            &value.compatibility_sha256,
            &value.rollback_point_sha256,
        ]
        .into_iter()
        .all(|digest| valid_sha256(digest))
    {
        return Err(UpdateError::InvalidPackage);
    }
    if compare_versions(&value.to_version, &value.from_version) != Some(Ordering::Greater) {
        return Err(UpdateError::Downgrade);
    }
    if !value.signature_verified || !value.integrity_verified || !value.compatibility_passed {
        return Err(UpdateError::VerificationFailed);
    }
    Ok(())
}

/// Authorize exact staged activation without performing it.
pub fn authorize_staged_activation(
    package: &UpdatePackage,
    grant: &UpdateActivationGrant,
) -> Result<StagedUpdate, UpdateError> {
    validate_update_package(package)?;
    if !grant.explicitly_approved
        || grant.approved_preview_sha256 != package.change_preview_sha256
        || grant.approved_package_sha256 != package.package_sha256
        || !valid_sha256(&grant.staged_tree_sha256)
    {
        return Err(UpdateError::StaleApproval);
    }
    Ok(StagedUpdate {
        package_sha256: package.package_sha256.clone(),
        preview_sha256: package.change_preview_sha256.clone(),
        staged_tree_sha256: grant.staged_tree_sha256.clone(),
        rollback_point_sha256: package.rollback_point_sha256.clone(),
    })
}

/// Complete local supply-chain maintenance packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyMaintenancePacket {
    /// Dependency provenance digest.
    pub provenance_sha256: String,
    /// CycloneDX SBOM digest.
    pub sbom_sha256: String,
    /// Dependency hash-manifest digest.
    pub dependency_hashes_sha256: String,
    /// Removal-plan digest covering every component identity.
    pub removal_plan_sha256: String,
    /// Exact component count shared by provenance and SBOM.
    pub component_count: usize,
    /// Components with an explicit non-empty license disposition.
    pub licensed_component_count: usize,
    /// Components whose integrity/content digest is present.
    pub hashed_component_count: usize,
    /// Component identities covered by a deterministic removal disposition.
    pub removal_component_count: usize,
    /// Current vulnerability-review result digest, if a current feed was available.
    pub vulnerability_review_sha256: Option<String>,
    /// Number of suppressed vulnerability results.
    pub vulnerability_suppression_count: usize,
}

/// Supply maintenance refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyMaintenanceError {
    /// Inventory, license, hash, removal, or digest closure is incomplete.
    IncompleteInventory,
    /// No current vulnerability review is bound.
    VulnerabilityReviewUnavailable,
    /// Suppressed findings make the packet incomplete.
    SuppressedFinding,
}

/// Validate complete local structure and require a separate current vulnerability review.
pub fn validate_supply_maintenance(
    value: &SupplyMaintenancePacket,
) -> Result<(), SupplyMaintenanceError> {
    if value.component_count == 0
        || value.licensed_component_count != value.component_count
        || value.hashed_component_count != value.component_count
        || value.removal_component_count != value.component_count
        || ![
            &value.provenance_sha256,
            &value.sbom_sha256,
            &value.dependency_hashes_sha256,
            &value.removal_plan_sha256,
        ]
        .into_iter()
        .all(|digest| valid_sha256(digest))
    {
        return Err(SupplyMaintenanceError::IncompleteInventory);
    }
    if value.vulnerability_suppression_count != 0 {
        return Err(SupplyMaintenanceError::SuppressedFinding);
    }
    if !value
        .vulnerability_review_sha256
        .as_deref()
        .is_some_and(valid_sha256)
    {
        return Err(SupplyMaintenanceError::VulnerabilityReviewUnavailable);
    }
    Ok(())
}

fn parse_version(value: &str) -> Option<Vec<u64>> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    parts
        .into_iter()
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    Some(parse_version(left)?.cmp(&parse_version(right)?))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().collect::<BTreeSet<_>>().len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(first: char) -> String {
        format!("{first}{}", "a".repeat(63))
    }

    fn package() -> UpdatePackage {
        UpdatePackage {
            product_id: "agentmage-desktop".to_owned(),
            from_version: "0.1.0".to_owned(),
            to_version: "0.1.1".to_owned(),
            package_sha256: digest('1'),
            manifest_sha256: digest('2'),
            signer_key_id: "release-key-01".to_owned(),
            signature_sha256: digest('3'),
            change_preview_sha256: digest('4'),
            compatibility_sha256: digest('5'),
            rollback_point_sha256: digest('6'),
            signature_verified: true,
            integrity_verified: true,
            compatibility_passed: true,
            automatic_remote_check: false,
        }
    }

    #[test]
    fn exact_offline_package_stages_after_fresh_approval() {
        let value = package();
        let grant = UpdateActivationGrant {
            approved_preview_sha256: value.change_preview_sha256.clone(),
            approved_package_sha256: value.package_sha256.clone(),
            staged_tree_sha256: digest('7'),
            explicitly_approved: true,
        };
        let staged = authorize_staged_activation(&value, &grant).expect("staged");
        assert_eq!(staged.rollback_point_sha256, value.rollback_point_sha256);
    }

    #[test]
    fn downgrade_tamper_incompatibility_and_remote_check_fail_closed() {
        let mut value = package();
        value.to_version = "0.0.9".to_owned();
        assert_eq!(validate_update_package(&value), Err(UpdateError::Downgrade));
        value = package();
        value.signature_verified = false;
        assert_eq!(
            validate_update_package(&value),
            Err(UpdateError::VerificationFailed)
        );
        value = package();
        value.compatibility_passed = false;
        assert_eq!(
            validate_update_package(&value),
            Err(UpdateError::VerificationFailed)
        );
        value = package();
        value.automatic_remote_check = true;
        assert_eq!(
            validate_update_package(&value),
            Err(UpdateError::RemoteCheckProhibited)
        );
    }

    #[test]
    fn stale_or_missing_activation_approval_cannot_mutate() {
        let value = package();
        let grant = UpdateActivationGrant {
            approved_preview_sha256: digest('8'),
            approved_package_sha256: value.package_sha256.clone(),
            staged_tree_sha256: digest('7'),
            explicitly_approved: true,
        };
        assert_eq!(
            authorize_staged_activation(&value, &grant),
            Err(UpdateError::StaleApproval)
        );
    }

    #[test]
    fn maintenance_requires_complete_closure_and_current_unsuppressed_review() {
        let mut packet = SupplyMaintenancePacket {
            provenance_sha256: digest('1'),
            sbom_sha256: digest('2'),
            dependency_hashes_sha256: digest('3'),
            removal_plan_sha256: digest('4'),
            component_count: 498,
            licensed_component_count: 498,
            hashed_component_count: 498,
            removal_component_count: 498,
            vulnerability_review_sha256: Some(digest('5')),
            vulnerability_suppression_count: 0,
        };
        assert_eq!(validate_supply_maintenance(&packet), Ok(()));
        packet.vulnerability_review_sha256 = None;
        assert_eq!(
            validate_supply_maintenance(&packet),
            Err(SupplyMaintenanceError::VulnerabilityReviewUnavailable)
        );
        packet.vulnerability_review_sha256 = Some(digest('5'));
        packet.vulnerability_suppression_count = 1;
        assert_eq!(
            validate_supply_maintenance(&packet),
            Err(SupplyMaintenanceError::SuppressedFinding)
        );
    }
}
