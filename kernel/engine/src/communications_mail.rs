//! Capability-negotiated standard mail and loopback-only Proton Bridge contracts.
use std::collections::BTreeSet;

/// Exact admitted mail protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MailProtocol {
    /// Internet Message Access Protocol.
    Imap,
    /// Simple Mail Transfer Protocol submission.
    Smtp,
    /// JSON Meta Application Protocol.
    Jmap,
    /// Locally authenticated Proton Mail Bridge profile.
    ProtonBridge,
}

/// Operations registered only when negotiated and policy allowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MailOperation {
    /// Read bounded messages.
    Read,
    /// Read folder state.
    FolderState,
    /// Change message flags.
    SetFlags,
    /// Move a message.
    Move,
    /// Copy a message.
    Copy,
    /// Create or update a draft.
    Draft,
    /// Submit a message.
    Submit,
    /// Query JMAP objects.
    JmapQuery,
    /// Mutate JMAP objects.
    JmapSet,
}

/// Server, transport, account, mailbox, and message namespace identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailIdentity {
    /// Stable server identity.
    pub server_id: String,
    /// Exact server host.
    pub host: String,
    /// Exact server port.
    pub port: u16,
    /// Certificate SHA-256.
    pub certificate_sha256: String,
    /// Account identity.
    pub account_id: String,
    /// Mailbox identity.
    pub mailbox_id: String,
    /// Folder identity.
    pub folder_id: String,
    /// IMAP UIDVALIDITY or equivalent namespace revision.
    pub uid_validity: u64,
    /// Opaque credential reference.
    pub credential_reference: String,
    /// Optional expected Bridge process identity.
    pub bridge_process_id: Option<String>,
}

/// Negotiated, rather than advertised-only, server capabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailCapabilities {
    /// Exact protocol.
    pub protocol: MailProtocol,
    /// Negotiated protocol version.
    pub version: String,
    /// Negotiated extension identities.
    pub extensions: BTreeSet<String>,
    /// Operations supported by the negotiation result.
    pub operations: BTreeSet<MailOperation>,
    /// Whether TLS is active before credential release.
    pub tls_active: bool,
}

/// Closed connection policy fixed before authentication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailPolicy {
    /// Exact allowed host.
    pub host: String,
    /// Exact allowed port.
    pub port: u16,
    /// Exact allowed certificate hash.
    pub certificate_sha256: String,
    /// Exact allowed protocol versions.
    pub versions: BTreeSet<String>,
    /// Operations the local policy permits.
    pub operations: BTreeSet<MailOperation>,
    /// Expected Proton Bridge process identity.
    pub bridge_process_id: Option<String>,
}

/// Exact effect frozen by preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailEffect {
    /// Operation.
    pub operation: MailOperation,
    /// Server identity.
    pub server_id: String,
    /// Account identity.
    pub account_id: String,
    /// Mailbox identity.
    pub mailbox_id: String,
    /// Folder identity.
    pub folder_id: String,
    /// Message or submission identity.
    pub object_id: String,
    /// UID namespace revision.
    pub uid_validity: u64,
    /// Exact recipients.
    pub recipients: Vec<String>,
    /// Canonical MIME or JMAP payload digest.
    pub content_sha256: String,
    /// Source revision.
    pub source_revision: u64,
    /// One-use submission key.
    pub idempotency_key: String,
}

/// Visible completion truth for an attempted operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailOutcome {
    /// No operation has run.
    NotRun,
    /// Effect was proved.
    Succeeded,
    /// Non-effect was proved.
    Failed,
    /// Disconnect or server result has not been reconciled.
    Uncertain,
    /// Adapter authority was removed.
    Removed,
}

/// Stable fail-closed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailError {
    /// Identity was malformed.
    InvalidIdentity,
    /// Host, port, certificate, redirect, or proxy did not match.
    TransportMismatch,
    /// TLS or protocol version would downgrade the connection.
    Downgrade,
    /// Proton Bridge escaped loopback or process identity changed.
    BridgeMismatch,
    /// Operation was not negotiated and allowed.
    Unsupported,
    /// Effect differs from the frozen preview.
    ChangedEffect,
    /// Account, mailbox, server, folder, or UID namespace differs.
    WrongAuthority,
    /// Source revision is stale.
    StaleSource,
    /// One-use submission would be repeated.
    DuplicateSubmission,
    /// Result needs reconciliation.
    Uncertain,
    /// Adapter has been removed.
    Removed,
}

/// Connection attempt inputs observed before credential release.
pub struct ConnectionAttempt<'a> {
    /// Server-selected host after resolution.
    pub resolved_host: &'a str,
    /// Server-selected port.
    pub resolved_port: u16,
    /// Observed certificate hash.
    pub certificate_sha256: &'a str,
    /// Negotiated protocol version.
    pub version: &'a str,
    /// Whether an unapproved redirect occurred.
    pub redirected: bool,
    /// Whether an unapproved proxy is active.
    pub proxied: bool,
    /// Observed Bridge process identity.
    pub bridge_process_id: Option<&'a str>,
}

/// Exact server/account controller.
pub struct MailController {
    identity: MailIdentity,
    capabilities: MailCapabilities,
    allowed_operations: BTreeSet<MailOperation>,
    consumed: BTreeSet<String>,
    credential_released: bool,
    outcome: MailOutcome,
    removed: bool,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "::1" | "localhost")
}

impl MailController {
    /// Validates transport and negotiation before releasing a credential reference.
    pub fn connect(
        identity: MailIdentity,
        capabilities: MailCapabilities,
        policy: &MailPolicy,
        attempt: &ConnectionAttempt<'_>,
    ) -> Result<Self, MailError> {
        if !valid_id(&identity.server_id)
            || !valid_id(&identity.account_id)
            || !valid_id(&identity.mailbox_id)
            || !valid_id(&identity.folder_id)
            || !valid_id(&identity.credential_reference)
            || !digest(&identity.certificate_sha256)
        {
            return Err(MailError::InvalidIdentity);
        }
        if attempt.redirected
            || attempt.proxied
            || identity.host != policy.host
            || identity.port != policy.port
            || attempt.resolved_host != policy.host
            || attempt.resolved_port != policy.port
            || identity.certificate_sha256 != policy.certificate_sha256
            || attempt.certificate_sha256 != policy.certificate_sha256
        {
            return Err(MailError::TransportMismatch);
        }
        if !capabilities.tls_active
            || capabilities.version != attempt.version
            || !policy.versions.contains(attempt.version)
        {
            return Err(MailError::Downgrade);
        }
        if capabilities.protocol == MailProtocol::ProtonBridge
            && (!loopback(attempt.resolved_host)
                || identity.bridge_process_id.as_deref() != policy.bridge_process_id.as_deref()
                || attempt.bridge_process_id != policy.bridge_process_id.as_deref())
        {
            return Err(MailError::BridgeMismatch);
        }
        let allowed_operations = capabilities
            .operations
            .intersection(&policy.operations)
            .copied()
            .collect();
        Ok(Self {
            identity,
            capabilities,
            allowed_operations,
            consumed: BTreeSet::new(),
            credential_released: true,
            outcome: MailOutcome::NotRun,
            removed: false,
        })
    }

    /// Admits one exact negotiated operation and refuses replay.
    pub fn admit(
        &mut self,
        effect: &MailEffect,
        preview: &MailEffect,
        current_revision: u64,
    ) -> Result<(), MailError> {
        if self.removed {
            return Err(MailError::Removed);
        }
        if effect.server_id != self.identity.server_id
            || effect.account_id != self.identity.account_id
            || effect.mailbox_id != self.identity.mailbox_id
            || effect.folder_id != self.identity.folder_id
            || effect.uid_validity != self.identity.uid_validity
        {
            return Err(MailError::WrongAuthority);
        }
        if !self.allowed_operations.contains(&effect.operation) {
            return Err(MailError::Unsupported);
        }
        if effect != preview
            || !valid_id(&effect.object_id)
            || !digest(&effect.content_sha256)
            || (effect.operation == MailOperation::Submit && effect.recipients.is_empty())
        {
            return Err(MailError::ChangedEffect);
        }
        if effect.source_revision != current_revision {
            return Err(MailError::StaleSource);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(MailError::DuplicateSubmission);
        }
        self.outcome = MailOutcome::Succeeded;
        Ok(())
    }

    /// Records a disconnect or ambiguous server response without claiming completion.
    pub fn mark_uncertain(&mut self) -> Result<(), MailError> {
        if self.removed {
            return Err(MailError::Removed);
        }
        self.outcome = MailOutcome::Uncertain;
        Err(MailError::Uncertain)
    }

    /// Reconciles an uncertain operation to proved effect or proved non-effect.
    pub fn reconcile(&mut self, effect_proved: bool) -> Result<MailOutcome, MailError> {
        if self.outcome != MailOutcome::Uncertain {
            return Err(MailError::Uncertain);
        }
        self.outcome = if effect_proved {
            MailOutcome::Succeeded
        } else {
            MailOutcome::Failed
        };
        Ok(self.outcome)
    }

    /// Immediately removes registrations lost to capability drift.
    pub fn update_capabilities(&mut self, negotiated: BTreeSet<MailOperation>) {
        self.capabilities.operations = negotiated;
        self.allowed_operations = self
            .capabilities
            .operations
            .intersection(&self.allowed_operations)
            .copied()
            .collect();
    }

    /// Closes authority, connection state, schedules, cursors, and credential access.
    pub fn remove(&mut self) {
        self.allowed_operations.clear();
        self.consumed.clear();
        self.identity.credential_reference.clear();
        self.credential_released = false;
        self.removed = true;
        self.outcome = MailOutcome::Removed;
    }

    /// Whether authentication was released only after transport validation.
    pub fn credential_released(&self) -> bool {
        self.credential_released
    }

    /// Current visible result truth.
    pub fn outcome(&self) -> MailOutcome {
        self.outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected(protocol: MailProtocol, host: &str) -> MailController {
        let hash = "a".repeat(64);
        let bridge = (protocol == MailProtocol::ProtonBridge).then(|| "bridge-1".into());
        let identity = MailIdentity {
            server_id: "server".into(),
            host: host.into(),
            port: 993,
            certificate_sha256: hash.clone(),
            account_id: "account".into(),
            mailbox_id: "mailbox".into(),
            folder_id: "inbox".into(),
            uid_validity: 7,
            credential_reference: "credential".into(),
            bridge_process_id: bridge.clone(),
        };
        let operations = BTreeSet::from([MailOperation::Read, MailOperation::Submit]);
        let capabilities = MailCapabilities {
            protocol,
            version: "v1".into(),
            extensions: BTreeSet::from(["UIDPLUS".into()]),
            operations: operations.clone(),
            tls_active: true,
        };
        let policy = MailPolicy {
            host: host.into(),
            port: 993,
            certificate_sha256: hash.clone(),
            versions: BTreeSet::from(["v1".into()]),
            operations,
            bridge_process_id: bridge.clone(),
        };
        MailController::connect(
            identity,
            capabilities,
            &policy,
            &ConnectionAttempt {
                resolved_host: host,
                resolved_port: 993,
                certificate_sha256: &hash,
                version: "v1",
                redirected: false,
                proxied: false,
                bridge_process_id: bridge.as_deref(),
            },
        )
        .unwrap()
    }

    fn effect() -> MailEffect {
        MailEffect {
            operation: MailOperation::Submit,
            server_id: "server".into(),
            account_id: "account".into(),
            mailbox_id: "mailbox".into(),
            folder_id: "inbox".into(),
            object_id: "submission".into(),
            uid_validity: 7,
            recipients: vec!["recipient@example.test".into()],
            content_sha256: "b".repeat(64),
            source_revision: 4,
            idempotency_key: "once".into(),
        }
    }

    #[test]
    fn only_negotiated_operations_and_exact_authority_are_admitted() {
        let mut controller = connected(MailProtocol::Imap, "mail.example.test");
        let preview = effect();
        let mut wrong = effect();
        wrong.account_id = "other".into();
        assert_eq!(
            controller.admit(&wrong, &preview, 4),
            Err(MailError::WrongAuthority)
        );
        assert_eq!(controller.admit(&preview, &preview, 4), Ok(()));
        assert_eq!(
            controller.admit(&preview, &preview, 4),
            Err(MailError::DuplicateSubmission)
        );
    }

    #[test]
    fn uncertainty_requires_reconciliation() {
        let mut controller = connected(MailProtocol::Jmap, "jmap.example.test");
        assert_eq!(controller.mark_uncertain(), Err(MailError::Uncertain));
        assert_eq!(controller.outcome(), MailOutcome::Uncertain);
        assert_eq!(controller.reconcile(false), Ok(MailOutcome::Failed));
    }

    #[test]
    fn bridge_is_loopback_and_removal_closes_authority() {
        let mut controller = connected(MailProtocol::ProtonBridge, "127.0.0.1");
        assert!(controller.credential_released());
        controller.remove();
        assert!(!controller.credential_released());
        assert_eq!(controller.outcome(), MailOutcome::Removed);
        assert_eq!(
            controller.admit(&effect(), &effect(), 4),
            Err(MailError::Removed)
        );
    }
}
