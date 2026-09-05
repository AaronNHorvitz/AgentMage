//! Provider-native AWS, Azure, and Google Cloud observation profiles.
#![allow(missing_docs)]

use std::collections::BTreeSet;

use crate::cloud_observer::{
    CloudObserverError, CloudObserverScope, CloudQueryBudget, CloudReadKind, CloudResultState,
    validate_budget, validate_scope,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCloudProvider {
    Aws,
    Azure,
    GoogleCloud,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderHierarchy {
    pub provider: NativeCloudProvider,
    pub partition_or_cloud: String,
    pub organization_or_tenant: String,
    pub folder_or_management_group: Option<String>,
    pub account_subscription_or_project: String,
    pub resource_group: Option<String>,
    pub region: String,
    pub zone: Option<String>,
    pub service_or_provider: String,
    pub native_resource_id: String,
    pub principal_id: String,
    pub token_audience: String,
    pub endpoint: String,
    pub quota_project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderObserverProfile {
    pub profile_id: String,
    pub scope: CloudObserverScope,
    pub hierarchy: ProviderHierarchy,
    pub api_version: String,
    pub external_id: Option<String>,
    pub native_fields: BTreeSet<String>,
    pub secret_value_count: u32,
    pub effect_capability_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderReadRequest {
    pub provider: NativeCloudProvider,
    pub account_subscription_or_project: String,
    pub region: String,
    pub service_or_provider: String,
    pub native_resource_id: String,
    pub endpoint: String,
    pub token_audience: String,
    pub api_version: String,
    pub quota_project: Option<String>,
    pub read_kind: CloudReadKind,
    pub budget: CloudQueryBudget,
    pub requested_fields: BTreeSet<String>,
    pub data_plane: bool,
    pub effect: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderRecoveryReceipt {
    pub provider: NativeCloudProvider,
    pub state: CloudResultState,
    pub limitation_codes: BTreeSet<String>,
    pub retry_count: u32,
    pub residual_credential_count: u32,
    pub residual_session_count: u32,
    pub residual_cursor_count: u32,
    pub residual_worker_count: u32,
    pub residual_socket_count: u32,
    pub residual_cloud_authority_count: u32,
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048
}

fn native_prefix(provider: NativeCloudProvider, id: &str, account: &str) -> bool {
    match provider {
        NativeCloudProvider::Aws => id.starts_with("arn:") && id.contains(&format!(":{account}:")),
        NativeCloudProvider::Azure => id.starts_with(&format!("/subscriptions/{account}/")),
        NativeCloudProvider::GoogleCloud => id.starts_with("//") && id.contains(account),
    }
}

pub fn validate_provider_profile(
    profile: &ProviderObserverProfile,
) -> Result<(), CloudObserverError> {
    validate_scope(&profile.scope)?;
    let hierarchy = &profile.hierarchy;
    if !bounded(&profile.profile_id)
        || !bounded(&hierarchy.partition_or_cloud)
        || !bounded(&hierarchy.organization_or_tenant)
        || !bounded(&hierarchy.account_subscription_or_project)
        || !bounded(&hierarchy.region)
        || !bounded(&hierarchy.service_or_provider)
        || !bounded(&hierarchy.native_resource_id)
        || !bounded(&hierarchy.principal_id)
        || !bounded(&hierarchy.token_audience)
        || !bounded(&hierarchy.endpoint)
        || !bounded(&profile.api_version)
        || profile.native_fields.is_empty()
        || profile.secret_value_count != 0
        || profile.effect_capability_count != 0
        || profile.scope.account_id != hierarchy.account_subscription_or_project
        || !profile.scope.regions.contains(&hierarchy.region)
        || !profile
            .scope
            .services
            .contains(&hierarchy.service_or_provider)
        || !native_prefix(
            hierarchy.provider,
            &hierarchy.native_resource_id,
            &hierarchy.account_subscription_or_project,
        )
    {
        return Err(CloudObserverError::OutOfScope);
    }
    let exact = match hierarchy.provider {
        NativeCloudProvider::Aws => {
            profile.scope.provider_id == "aws"
                && profile.scope.tenant_id.is_none()
                && profile.scope.subscription_or_project_id == profile.scope.account_id
                && hierarchy.partition_or_cloud.starts_with("aws")
                && hierarchy.endpoint.ends_with("amazonaws.com")
                && hierarchy.quota_project.is_none()
        }
        NativeCloudProvider::Azure => {
            profile.scope.provider_id == "azure"
                && profile.scope.tenant_id.as_deref()
                    == Some(hierarchy.organization_or_tenant.as_str())
                && profile.scope.subscription_or_project_id == profile.scope.account_id
                && hierarchy.resource_group.is_some()
                && hierarchy.endpoint.ends_with("azure.com")
                && hierarchy.quota_project.is_none()
        }
        NativeCloudProvider::GoogleCloud => {
            profile.scope.provider_id == "gcp"
                && profile.scope.tenant_id.is_none()
                && profile.scope.subscription_or_project_id == profile.scope.account_id
                && hierarchy.endpoint.ends_with("googleapis.com")
                && hierarchy.quota_project.as_deref() == Some(profile.scope.account_id.as_str())
        }
    };
    if !exact {
        return Err(CloudObserverError::OutOfScope);
    }
    Ok(())
}

pub fn admit_provider_read(
    request: &ProviderReadRequest,
    profile: &ProviderObserverProfile,
) -> Result<(), CloudObserverError> {
    validate_provider_profile(profile)?;
    validate_budget(&request.budget)?;
    let hierarchy = &profile.hierarchy;
    if request.provider != hierarchy.provider
        || request.account_subscription_or_project != hierarchy.account_subscription_or_project
        || request.region != hierarchy.region
        || request.service_or_provider != hierarchy.service_or_provider
        || request.native_resource_id != hierarchy.native_resource_id
        || request.endpoint != hierarchy.endpoint
        || request.token_audience != hierarchy.token_audience
        || request.api_version != profile.api_version
        || request.quota_project != hierarchy.quota_project
        || !profile.scope.allowed_reads.contains(&request.read_kind)
        || !request
            .requested_fields
            .iter()
            .all(|field| profile.native_fields.contains(field))
        || request.data_plane
        || request.effect
    {
        return Err(CloudObserverError::OutOfScope);
    }
    Ok(())
}

pub fn reject_provider_operation(_operation: &str) -> Result<(), CloudObserverError> {
    Err(CloudObserverError::UnsupportedEffect)
}

pub fn validate_provider_recovery(
    receipt: &ProviderRecoveryReceipt,
) -> Result<(), CloudObserverError> {
    let limited = receipt.state != CloudResultState::Complete;
    if (limited && receipt.limitation_codes.is_empty())
        || receipt.retry_count > 3
        || [
            receipt.residual_credential_count,
            receipt.residual_session_count,
            receipt.residual_cursor_count,
            receipt.residual_worker_count,
            receipt.residual_socket_count,
            receipt.residual_cloud_authority_count,
        ]
        .into_iter()
        .any(|count| count != 0)
    {
        return Err(CloudObserverError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(provider: NativeCloudProvider) -> ProviderObserverProfile {
        let (
            provider_id,
            partition,
            organization,
            tenant,
            account,
            service,
            resource,
            endpoint,
            quota,
            group,
        ) = match provider {
            NativeCloudProvider::Aws => (
                "aws",
                "aws",
                "o-1",
                None,
                "111122223333",
                "ec2",
                "arn:aws:ec2:us-east-1:111122223333:instance/i-1",
                "ec2.amazonaws.com",
                None,
                None,
            ),
            NativeCloudProvider::Azure => (
                "azure",
                "public",
                "tenant-1",
                Some("tenant-1"),
                "sub-1",
                "Microsoft.Compute",
                "/subscriptions/sub-1/resourceGroups/rg-1/providers/Microsoft.Compute/virtualMachines/vm-1",
                "management.azure.com",
                None,
                Some("rg-1"),
            ),
            NativeCloudProvider::GoogleCloud => (
                "gcp",
                "googleapis",
                "organizations/1",
                None,
                "project-1",
                "compute.googleapis.com",
                "//compute.googleapis.com/projects/project-1/zones/us-central1-a/instances/vm-1",
                "cloudasset.googleapis.com",
                Some("project-1"),
                None,
            ),
        };
        ProviderObserverProfile {
            profile_id: format!("{provider_id}-profile"),
            scope: CloudObserverScope {
                provider_id: provider_id.into(),
                organization_id: organization.into(),
                tenant_id: tenant.map(str::to_string),
                account_id: account.into(),
                subscription_or_project_id: account.into(),
                regions: BTreeSet::from(["us-east-1".into()]),
                services: BTreeSet::from([service.into()]),
                resource_prefixes: BTreeSet::from([resource.into()]),
                allowed_reads: BTreeSet::from([CloudReadKind::Inventory]),
                credential_reference: "credential-ref".into(),
                support_profile_id: format!("{provider_id}-support"),
            },
            hierarchy: ProviderHierarchy {
                provider,
                partition_or_cloud: partition.into(),
                organization_or_tenant: organization.into(),
                folder_or_management_group: None,
                account_subscription_or_project: account.into(),
                resource_group: group.map(str::to_string),
                region: "us-east-1".into(),
                zone: None,
                service_or_provider: service.into(),
                native_resource_id: resource.into(),
                principal_id: "principal-1".into(),
                token_audience: endpoint.into(),
                endpoint: endpoint.into(),
                quota_project: quota.map(str::to_string),
            },
            api_version: "v1".into(),
            external_id: None,
            native_fields: BTreeSet::from(["id".into()]),
            secret_value_count: 0,
            effect_capability_count: 0,
        }
    }

    #[test]
    fn all_native_profiles_are_exact() {
        for provider in [
            NativeCloudProvider::Aws,
            NativeCloudProvider::Azure,
            NativeCloudProvider::GoogleCloud,
        ] {
            assert_eq!(validate_provider_profile(&profile(provider)), Ok(()));
        }
    }

    #[test]
    fn cross_provider_identity_is_rejected() {
        let mut value = profile(NativeCloudProvider::Aws);
        value.scope.provider_id = "azure".into();
        assert_eq!(
            validate_provider_profile(&value),
            Err(CloudObserverError::OutOfScope)
        );
    }

    #[test]
    fn data_plane_and_effects_are_absent() {
        assert_eq!(
            reject_provider_operation("deploy"),
            Err(CloudObserverError::UnsupportedEffect)
        );
        let value = profile(NativeCloudProvider::Aws);
        let mut request = ProviderReadRequest {
            provider: NativeCloudProvider::Aws,
            account_subscription_or_project: value
                .hierarchy
                .account_subscription_or_project
                .clone(),
            region: value.hierarchy.region.clone(),
            service_or_provider: value.hierarchy.service_or_provider.clone(),
            native_resource_id: value.hierarchy.native_resource_id.clone(),
            endpoint: value.hierarchy.endpoint.clone(),
            token_audience: value.hierarchy.token_audience.clone(),
            api_version: value.api_version.clone(),
            quota_project: None,
            read_kind: CloudReadKind::Inventory,
            budget: CloudQueryBudget {
                query_id: "q".into(),
                time_start: "a".into(),
                time_end: "b".into(),
                fields: BTreeSet::from(["id".into()]),
                max_rows: 1,
                max_bytes: 1,
                max_requests: 1,
                rate_per_minute: 1,
                cancellation_millis: 1,
                cache_max_age_seconds: 0,
            },
            requested_fields: BTreeSet::from(["id".into()]),
            data_plane: false,
            effect: false,
        };
        assert_eq!(admit_provider_read(&request, &value), Ok(()));
        request.effect = true;
        assert_eq!(
            admit_provider_read(&request, &value),
            Err(CloudObserverError::OutOfScope)
        );
    }

    #[test]
    fn recovery_is_bounded_and_authority_free() {
        let receipt = ProviderRecoveryReceipt {
            provider: NativeCloudProvider::GoogleCloud,
            state: CloudResultState::Removed,
            limitation_codes: BTreeSet::from(["removed".into()]),
            retry_count: 0,
            residual_credential_count: 0,
            residual_session_count: 0,
            residual_cursor_count: 0,
            residual_worker_count: 0,
            residual_socket_count: 0,
            residual_cloud_authority_count: 0,
        };
        assert_eq!(validate_provider_recovery(&receipt), Ok(()));
    }
}
