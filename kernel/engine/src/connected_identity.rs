//! Exact connected identity, credential isolation, and capability-class admission.

use std::collections::BTreeSet;

/// Closed, non-inheriting connected capability classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectedCapabilityClass {
    /// Observe provider state without mutation.
    Observe,
    /// Produce an inert draft.
    Draft,
    /// Mutate an AgentMage-owned local resource.
    LocalWrite,
    /// Mutate one exact remote resource.
    RemoteWrite,
    /// Execute one exact external operation.
    Execute,
    /// Deploy to one exact environment.
    Deploy,
    /// Use one operation-scoped credential.
    Secrets,
    /// Perform one separately promoted administrative operation.
    Admin,
}

/// Closed credential availability states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialState {
    /// One exact usable credential reference exists.
    Available,
    /// No matching credential exists.
    Missing,
    /// More than one credential could match.
    Ambiguous,
    /// Credential scope exceeds the operation.
    Excessive,
    /// Credential freshness has expired.
    Stale,
    /// Credential has been revoked.
    Revoked,
}

/// Canonical provider security-domain identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedIdentity {
    /// Provider adapter identity.
    pub provider_id: String,
    /// Exact canonical host.
    pub host: String,
    /// Exact network port.
    pub port: u16,
    /// Lowercase SHA-256 of the expected TLS identity.
    pub tls_identity_sha256: String,
    /// Exact tenant or organization.
    pub tenant: String,
    /// Exact account identity.
    pub account: String,
    /// Exact project identity.
    pub project: String,
    /// Exact environment identity.
    pub environment: String,
    /// Opaque platform-secret-store reference, never secret bytes.
    pub credential_reference: String,
    /// Exact single-sign-on state identity.
    pub single_sign_on_state: String,
    /// Exact non-inheriting capability class.
    pub capability_class: ConnectedCapabilityClass,
}

/// Every network identity that can influence one request destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestBoundary {
    /// Original requested host.
    pub request_host: String,
    /// Resolved network port.
    pub request_port: u16,
    /// Observed TLS identity digest.
    pub tls_identity_sha256: String,
    /// Redirect host, when present.
    pub redirect_host: Option<String>,
    /// Proxy destination host, when present.
    pub proxy_host: Option<String>,
    /// Canonical DNS result host.
    pub dns_host: String,
    /// Callback target host, when present.
    pub callback_host: Option<String>,
    /// Clone transport host, when present.
    pub clone_host: Option<String>,
    /// Provider-supplied linked API host, when present.
    pub linked_api_host: Option<String>,
}

/// Content origins that cannot mint or broaden connected authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscalationOrigin {
    /// Direct client request.
    DirectRequest,
    /// Provider-controlled content.
    ProviderContent,
    /// Nested provider action.
    NestedAction,
    /// Imported workflow input.
    WorkflowInput,
    /// Plugin output.
    Plugin,
    /// Model-selected tool call.
    ModelToolCall,
}

/// Non-secret credential metadata supplied to one operation-scoped worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialDescriptor {
    /// Opaque platform-secret-store reference.
    pub reference: String,
    /// Current availability state.
    pub state: CredentialState,
    /// Exact permitted scopes.
    pub scopes: BTreeSet<String>,
    /// Expiry as an epoch second.
    pub expires_at_epoch_seconds: u64,
}

/// Operation-scoped provider worker identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderWorker {
    /// Unique operation identity.
    pub operation_id: String,
    /// Digest of the exact connected identity.
    pub connected_identity_sha256: String,
    /// Digest of the kernel grant.
    pub grant_sha256: String,
    /// Worker capability class.
    pub capability_class: ConnectedCapabilityClass,
    /// Isolated cache namespace.
    pub cache_namespace: String,
}

/// Public non-secret diagnostic and receipt metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialAdmission {
    /// Non-secret account label.
    pub account_label: String,
    /// Exact admitted scopes.
    pub scopes: BTreeSet<String>,
    /// Expiry as an epoch second.
    pub expires_at_epoch_seconds: u64,
    /// Stable admission receipt digest.
    pub receipt_sha256: String,
}

/// Stable fail-closed connected-identity refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectedIdentityError {
    /// A canonical identity field is malformed.
    InvalidIdentity,
    /// Worker identity or grant is malformed or mismatched.
    WorkerMismatch,
    /// A request could leave the granted security domain.
    CrossDomainTarget,
    /// Credential is absent, ambiguous, excessive, stale, or revoked.
    CredentialUnavailable,
    /// Requested capability differs from the exact grant class.
    CapabilityEscalation,
}

impl ConnectedCapabilityClass {
    /// Return distinct contract identifiers for this exact class.
    #[must_use]
    pub fn contract_ids(self) -> [&'static str; 6] {
        match self {
            Self::Observe => [
                "tool.observe",
                "policy.observe",
                "preview.observe",
                "receipt.observe",
                "diagnostic.observe",
                "support.observe",
            ],
            Self::Draft => [
                "tool.draft",
                "policy.draft",
                "preview.draft",
                "receipt.draft",
                "diagnostic.draft",
                "support.draft",
            ],
            Self::LocalWrite => [
                "tool.local-write",
                "policy.local-write",
                "preview.local-write",
                "receipt.local-write",
                "diagnostic.local-write",
                "support.local-write",
            ],
            Self::RemoteWrite => [
                "tool.remote-write",
                "policy.remote-write",
                "preview.remote-write",
                "receipt.remote-write",
                "diagnostic.remote-write",
                "support.remote-write",
            ],
            Self::Execute => [
                "tool.execute",
                "policy.execute",
                "preview.execute",
                "receipt.execute",
                "diagnostic.execute",
                "support.execute",
            ],
            Self::Deploy => [
                "tool.deploy",
                "policy.deploy",
                "preview.deploy",
                "receipt.deploy",
                "diagnostic.deploy",
                "support.deploy",
            ],
            Self::Secrets => [
                "tool.secrets",
                "policy.secrets",
                "preview.secrets",
                "receipt.secrets",
                "diagnostic.secrets",
                "support.secrets",
            ],
            Self::Admin => [
                "tool.admin",
                "policy.admin",
                "preview.admin",
                "receipt.admin",
                "diagnostic.admin",
                "support.admin",
            ],
        }
    }
}

impl ConnectedIdentity {
    /// Validate the complete canonical identity without resolving a credential.
    pub fn validate(&self) -> Result<(), ConnectedIdentityError> {
        if !valid_id(&self.provider_id)
            || !valid_host(&self.host)
            || self.port == 0
            || !valid_sha256(&self.tls_identity_sha256)
            || !valid_id(&self.tenant)
            || !valid_id(&self.account)
            || !valid_id(&self.project)
            || !valid_id(&self.environment)
            || !valid_id(&self.credential_reference)
            || !valid_id(&self.single_sign_on_state)
        {
            Err(ConnectedIdentityError::InvalidIdentity)
        } else {
            Ok(())
        }
    }

    /// Require every possible request destination to remain in this exact domain.
    pub fn validate_request(&self, target: &RequestBoundary) -> Result<(), ConnectedIdentityError> {
        self.validate()?;
        let hosts = [
            Some(target.request_host.as_str()),
            target.redirect_host.as_deref(),
            target.proxy_host.as_deref(),
            Some(target.dns_host.as_str()),
            target.callback_host.as_deref(),
            target.clone_host.as_deref(),
            target.linked_api_host.as_deref(),
        ];
        if target.request_port != self.port
            || target.tls_identity_sha256 != self.tls_identity_sha256
            || hosts.into_iter().flatten().any(|host| host != self.host)
        {
            Err(ConnectedIdentityError::CrossDomainTarget)
        } else {
            Ok(())
        }
    }
}

impl ProviderWorker {
    /// Resolve only non-secret admission metadata after exact grant and worker validation.
    pub fn admit_credential(
        &self,
        identity: &ConnectedIdentity,
        target: &RequestBoundary,
        credential: &CredentialDescriptor,
        required_scopes: &BTreeSet<String>,
        now_epoch_seconds: u64,
        expected_grant_sha256: &str,
    ) -> Result<CredentialAdmission, ConnectedIdentityError> {
        identity.validate_request(target)?;
        if !valid_id(&self.operation_id)
            || !valid_id(&self.cache_namespace)
            || !valid_sha256(&self.connected_identity_sha256)
            || !valid_sha256(&self.grant_sha256)
            || self.grant_sha256 != expected_grant_sha256
            || self.capability_class != identity.capability_class
        {
            return Err(ConnectedIdentityError::WorkerMismatch);
        }
        if credential.reference != identity.credential_reference
            || credential.state != CredentialState::Available
            || &credential.scopes != required_scopes
            || credential.expires_at_epoch_seconds <= now_epoch_seconds
        {
            return Err(ConnectedIdentityError::CredentialUnavailable);
        }
        Ok(CredentialAdmission {
            account_label: identity.account.clone(),
            scopes: credential.scopes.clone(),
            expires_at_epoch_seconds: credential.expires_at_epoch_seconds,
            receipt_sha256: self.grant_sha256.clone(),
        })
    }
}

/// Admit a request only when it exactly matches the non-inheriting grant class.
pub fn admit_capability(
    granted: ConnectedCapabilityClass,
    requested: ConnectedCapabilityClass,
    _origin: EscalationOrigin,
) -> Result<[&'static str; 6], ConnectedIdentityError> {
    if granted == requested {
        Ok(requested.contract_ids())
    } else {
        Err(ConnectedIdentityError::CapabilityEscalation)
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
fn valid_host(value: &str) -> bool {
    valid_id(value) && !value.contains("..") && !value.contains('/')
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (
        ConnectedIdentity,
        RequestBoundary,
        CredentialDescriptor,
        ProviderWorker,
    ) {
        let digest = "a".repeat(64);
        let identity = ConnectedIdentity {
            provider_id: "fake-provider".into(),
            host: "api.example.test".into(),
            port: 443,
            tls_identity_sha256: digest.clone(),
            tenant: "tenant-a".into(),
            account: "account-a".into(),
            project: "project-a".into(),
            environment: "test".into(),
            credential_reference: "secret-ref-a".into(),
            single_sign_on_state: "valid".into(),
            capability_class: ConnectedCapabilityClass::Observe,
        };
        let target = RequestBoundary {
            request_host: identity.host.clone(),
            request_port: 443,
            tls_identity_sha256: digest.clone(),
            redirect_host: None,
            proxy_host: None,
            dns_host: identity.host.clone(),
            callback_host: None,
            clone_host: None,
            linked_api_host: None,
        };
        let credential = CredentialDescriptor {
            reference: "secret-ref-a".into(),
            state: CredentialState::Available,
            scopes: BTreeSet::from(["read".into()]),
            expires_at_epoch_seconds: 200,
        };
        let worker = ProviderWorker {
            operation_id: "operation-a".into(),
            connected_identity_sha256: digest.clone(),
            grant_sha256: digest,
            capability_class: ConnectedCapabilityClass::Observe,
            cache_namespace: "cache-a".into(),
        };
        (identity, target, credential, worker)
    }

    #[test]
    fn exact_worker_admits_only_non_secret_metadata() {
        let (identity, target, credential, worker) = fixture();
        let admission = worker
            .admit_credential(
                &identity,
                &target,
                &credential,
                &BTreeSet::from(["read".into()]),
                100,
                &"a".repeat(64),
            )
            .unwrap();
        assert_eq!(admission.account_label, "account-a");
    }

    #[test]
    fn every_destination_identity_is_revalidated() {
        let (identity, mut target, _, _) = fixture();
        target.redirect_host = Some("evil.example.test".into());
        assert_eq!(
            identity.validate_request(&target),
            Err(ConnectedIdentityError::CrossDomainTarget)
        );
    }

    #[test]
    fn credential_failures_are_stable_and_fail_closed() {
        let (identity, target, mut credential, worker) = fixture();
        credential.state = CredentialState::Revoked;
        assert_eq!(
            worker.admit_credential(
                &identity,
                &target,
                &credential,
                &BTreeSet::from(["read".into()]),
                100,
                &"a".repeat(64)
            ),
            Err(ConnectedIdentityError::CredentialUnavailable)
        );
    }

    #[test]
    fn capability_classes_do_not_inherit() {
        for origin in [
            EscalationOrigin::DirectRequest,
            EscalationOrigin::ProviderContent,
            EscalationOrigin::NestedAction,
            EscalationOrigin::WorkflowInput,
            EscalationOrigin::Plugin,
            EscalationOrigin::ModelToolCall,
        ] {
            assert_eq!(
                admit_capability(
                    ConnectedCapabilityClass::Observe,
                    ConnectedCapabilityClass::Admin,
                    origin
                ),
                Err(ConnectedIdentityError::CapabilityEscalation)
            );
        }
    }
}
