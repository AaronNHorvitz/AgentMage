//! Evidence-backed service catalog identities and inert extension candidates.
#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkState {
    Resolved,
    Inferred,
    Conflicting,
    Stale,
    Missing,
    ProviderMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionLevel {
    L0Manifest,
    L1Fixture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntity {
    pub provider_sha256: String,
    pub tenant_sha256: String,
    pub kind: String,
    pub namespace: String,
    pub name: String,
    pub uid_sha256: String,
    pub owner_sha256: String,
    pub system_sha256: String,
    pub domain_sha256: String,
    pub location_sha256: String,
    pub lifecycle_sha256: String,
    pub permission_sha256: String,
    pub descriptor_untrusted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderLink {
    pub entity_uid_sha256: String,
    pub relationship: String,
    pub catalog_target_sha256: String,
    pub provider_identity_sha256: String,
    pub provider_evidence_sha256: Option<String>,
    pub state: LinkState,
    pub grants_authority: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogExtension {
    pub provider: String,
    pub level: ExtensionLevel,
    pub manifest_sha256: String,
    pub owner_mapping_sha256: String,
    pub namespaced_extensions_sha256: String,
    pub support_matrix_sha256: String,
    pub read_registered: bool,
    pub write_registered: bool,
    pub promoted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogError {
    InvalidEntity,
    InvalidLink,
    InvalidExtension,
    AuthorityEscalation,
}

pub fn validate_entity(entity: &CatalogEntity) -> Result<(), CatalogError> {
    if [
        &entity.provider_sha256,
        &entity.tenant_sha256,
        &entity.uid_sha256,
        &entity.owner_sha256,
        &entity.system_sha256,
        &entity.domain_sha256,
        &entity.location_sha256,
        &entity.lifecycle_sha256,
        &entity.permission_sha256,
    ]
    .into_iter()
    .any(|field| !valid_sha(field))
        || [&entity.kind, &entity.namespace, &entity.name]
            .into_iter()
            .any(|field| !valid_id(field))
        || !entity.descriptor_untrusted
    {
        return Err(CatalogError::InvalidEntity);
    }
    Ok(())
}

pub fn validate_link(link: &ProviderLink) -> Result<(), CatalogError> {
    if !valid_sha(&link.entity_uid_sha256)
        || !valid_id(&link.relationship)
        || !valid_sha(&link.catalog_target_sha256)
        || !valid_sha(&link.provider_identity_sha256)
        || link
            .provider_evidence_sha256
            .as_ref()
            .is_some_and(|field| !valid_sha(field))
        || link.grants_authority
    {
        return Err(CatalogError::InvalidLink);
    }
    if link.state == LinkState::Resolved && link.provider_evidence_sha256.is_none() {
        return Err(CatalogError::InvalidLink);
    }
    Ok(())
}

pub fn validate_extension(extension: &CatalogExtension) -> Result<(), CatalogError> {
    if !matches!(extension.provider.as_str(), "port" | "cortex" | "compass")
        || [
            &extension.manifest_sha256,
            &extension.owner_mapping_sha256,
            &extension.namespaced_extensions_sha256,
            &extension.support_matrix_sha256,
        ]
        .into_iter()
        .any(|field| !valid_sha(field))
        || extension.read_registered
        || extension.write_registered
        || extension.promoted
    {
        return Err(CatalogError::InvalidExtension);
    }
    Ok(())
}

pub fn authority_from_catalog(_: &CatalogEntity) -> Result<(), CatalogError> {
    Err(CatalogError::AuthorityEscalation)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entity() -> CatalogEntity {
        let h = "a".repeat(64);
        CatalogEntity {
            provider_sha256: h.clone(),
            tenant_sha256: h.clone(),
            kind: "component".into(),
            namespace: "default".into(),
            name: "api".into(),
            uid_sha256: h.clone(),
            owner_sha256: h.clone(),
            system_sha256: h.clone(),
            domain_sha256: h.clone(),
            location_sha256: h.clone(),
            lifecycle_sha256: h.clone(),
            permission_sha256: h,
            descriptor_untrusted: true,
        }
    }
    #[test]
    fn exact_entity_is_valid() {
        assert_eq!(validate_entity(&entity()), Ok(()));
    }
    #[test]
    fn resolved_links_require_provider_evidence() {
        let h = "b".repeat(64);
        let link = ProviderLink {
            entity_uid_sha256: h.clone(),
            relationship: "repository".into(),
            catalog_target_sha256: h.clone(),
            provider_identity_sha256: h,
            provider_evidence_sha256: None,
            state: LinkState::Resolved,
            grants_authority: false,
        };
        assert_eq!(validate_link(&link), Err(CatalogError::InvalidLink));
    }
    #[test]
    fn extensions_remain_inert() {
        let h = "c".repeat(64);
        let mut extension = CatalogExtension {
            provider: "port".into(),
            level: ExtensionLevel::L1Fixture,
            manifest_sha256: h.clone(),
            owner_mapping_sha256: h.clone(),
            namespaced_extensions_sha256: h.clone(),
            support_matrix_sha256: h,
            read_registered: false,
            write_registered: false,
            promoted: false,
        };
        assert_eq!(validate_extension(&extension), Ok(()));
        extension.write_registered = true;
        assert_eq!(
            validate_extension(&extension),
            Err(CatalogError::InvalidExtension)
        );
    }
    #[test]
    fn descriptors_never_grant_authority() {
        assert_eq!(
            authority_from_catalog(&entity()),
            Err(CatalogError::AuthorityEscalation)
        );
    }
}
