//! Candidate-neutral, disabled-by-default model gateway tuple admission.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalEndpointClass, CanonicalModelEndpointProfile,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::engineering_records::{ValidateCanonicalRecord, canonical_record_sha256};

const MAX_ID_BYTES: usize = 256;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable refusal from exact gateway candidate admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayCandidateError {
    /// One identity, version, digest, or reference is malformed or absent.
    InvalidIdentity,
    /// Nested endpoint, operator, codec, credential, or qualification identities disagree.
    IdentityMismatch,
    /// The operator is not independently trusted by the current registry.
    UnknownOperator,
    /// The endpoint record attempted activation or fallback.
    ActivationForbidden,
    /// Canonical serialization failed.
    SerializationFailed,
}

/// Exact model artifact/profile identity, independent of its familiar display name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayModelIdentity {
    /// Exact model-profile identity.
    pub model_profile_id: String,
    /// Immutable manifest identity.
    pub manifest_id: String,
    /// Digest of the exact manifest.
    pub manifest_sha256: String,
    /// Exact model artifact/revision identity understood by the codec.
    pub model_revision_id: String,
    /// Digest of the exact model artifact.
    pub artifact_sha256: String,
}

/// Exact runtime-adapter identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayRuntimeAdapterIdentity {
    /// Adapter identity.
    pub adapter_id: String,
    /// Version of the adapter contract.
    pub contract_version: u16,
    /// Exact runtime build identity.
    pub runtime_build_id: String,
    /// Digest of the runtime build.
    pub runtime_sha256: String,
}

/// Exact external protocol codec identity, separate from model-family encoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayProtocolCodecIdentity {
    /// Codec identity.
    pub protocol_codec_id: String,
    /// Codec version.
    pub protocol_codec_version: String,
    /// Digest of the codec implementation and schema tuple.
    pub protocol_codec_sha256: String,
}

/// Exact operator identity independently trusted by local policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayOperatorIdentity {
    /// Operator identity.
    pub operator_id: String,
    /// Version of the operator registry entry.
    pub registry_version: u32,
    /// Digest of the registry entry and reviewed operator policy.
    pub operator_sha256: String,
}

/// Brokered credential reference identity; it never contains credential bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayCredentialReferenceIdentity {
    /// Non-secret broker reference.
    pub credential_reference: String,
    /// Broker namespace/version identity.
    pub broker_version: String,
    /// Digest of reference metadata, never secret material.
    pub reference_sha256: String,
}

/// Exact qualification identity for one whole candidate tuple.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayQualificationIdentity {
    /// Stable qualification identity.
    pub qualification_id: String,
    /// Qualification protocol version.
    pub qualification_version: u32,
    /// Digest of current exact-tuple evidence.
    pub qualification_sha256: String,
}

/// Exact route identity before any selection or fallback decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayRouteIdentity {
    /// Stable candidate route identity.
    pub route_id: String,
    /// Route contract version.
    pub route_version: u32,
    /// Digest of deterministic route policy.
    pub route_policy_sha256: String,
}

/// Complete independently identified candidate tuple; it carries no activation authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DisabledGatewayCandidate {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable whole-tuple identity.
    pub candidate_id: String,
    /// Exact model identity.
    pub model: GatewayModelIdentity,
    /// Exact runtime adapter identity.
    pub runtime_adapter: GatewayRuntimeAdapterIdentity,
    /// Exact protocol codec identity.
    pub protocol_codec: GatewayProtocolCodecIdentity,
    /// Exact endpoint profile identity.
    pub endpoint_profile_id: String,
    /// Digest of the canonical disabled endpoint record.
    pub endpoint_profile_sha256: String,
    /// Endpoint deployment class.
    pub endpoint_class: CanonicalEndpointClass,
    /// Exact candidate route identity.
    pub route: GatewayRouteIdentity,
    /// Independently trusted operator identity.
    pub operator: GatewayOperatorIdentity,
    /// Brokered credential identity, required only for remote classes.
    pub credential: Option<GatewayCredentialReferenceIdentity>,
    /// Exact current tuple qualification.
    pub qualification: GatewayQualificationIdentity,
    /// Always false at candidate admission.
    pub enabled: bool,
    /// Always false; fallback requires a later explicit route decision.
    pub automatic_fallback: bool,
    /// Digest of this whole tuple with this field zeroed.
    pub candidate_sha256: String,
}

/// Truthful result when identity admission succeeds but no route is activated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DisabledGatewayAdmission {
    /// Exact verified disabled candidate.
    pub candidate: DisabledGatewayCandidate,
    /// Stable visible blocked reason.
    pub reason_code: &'static str,
    /// No route can be selected at this boundary.
    pub selected_route_id: Option<String>,
    /// A candidate cannot enable fallback.
    pub fallback_enabled: bool,
    /// A returned model proposal establishes no authority.
    pub proposal_has_authority: bool,
}

/// Complete caller-supplied candidate identities checked against one endpoint record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayCandidateInput {
    /// Stable whole-tuple identity.
    pub candidate_id: String,
    /// Exact model identity.
    pub model: GatewayModelIdentity,
    /// Exact runtime adapter identity.
    pub runtime_adapter: GatewayRuntimeAdapterIdentity,
    /// Exact protocol codec identity.
    pub protocol_codec: GatewayProtocolCodecIdentity,
    /// Exact candidate route identity.
    pub route: GatewayRouteIdentity,
    /// Independently trusted operator identity.
    pub operator: GatewayOperatorIdentity,
    /// Brokered credential reference identity, when required.
    pub credential: Option<GatewayCredentialReferenceIdentity>,
    /// Exact qualification identity.
    pub qualification: GatewayQualificationIdentity,
}

/// Seals one complete exact tuple against a canonical disabled endpoint record.
pub fn admit_disabled_gateway_candidate(
    endpoint: &CanonicalModelEndpointProfile,
    input: GatewayCandidateInput,
    trusted_operator_sha256: &str,
) -> Result<DisabledGatewayAdmission, GatewayCandidateError> {
    endpoint
        .validate_canonical()
        .map_err(|_| GatewayCandidateError::InvalidIdentity)?;
    if endpoint.enabled || endpoint.automatic_fallback {
        return Err(GatewayCandidateError::ActivationForbidden);
    }
    validate_model(&input.model)?;
    validate_runtime(&input.runtime_adapter)?;
    validate_codec(&input.protocol_codec)?;
    validate_route(&input.route)?;
    validate_operator(&input.operator)?;
    validate_qualification(&input.qualification)?;
    if !valid_id(&input.candidate_id)
        || !valid_sha256(trusted_operator_sha256)
        || input.operator.operator_sha256 != trusted_operator_sha256
    {
        return Err(GatewayCandidateError::UnknownOperator);
    }
    if endpoint.operator_id != input.operator.operator_id
        || endpoint.protocol_codec_id != input.protocol_codec.protocol_codec_id
        || endpoint.qualification_sha256.as_deref()
            != Some(input.qualification.qualification_sha256.as_str())
    {
        return Err(GatewayCandidateError::IdentityMismatch);
    }
    validate_credential_binding(endpoint, input.credential.as_ref())?;
    let mut candidate = DisabledGatewayCandidate {
        schema_version: CONTRACT_SCHEMA_VERSION,
        candidate_id: input.candidate_id,
        model: input.model,
        runtime_adapter: input.runtime_adapter,
        protocol_codec: input.protocol_codec,
        endpoint_profile_id: endpoint.endpoint_profile_id.clone(),
        endpoint_profile_sha256: canonical_record_sha256(endpoint)
            .map_err(|_| GatewayCandidateError::SerializationFailed)?,
        endpoint_class: endpoint.profile_class,
        route: input.route,
        operator: input.operator,
        credential: input.credential,
        qualification: input.qualification,
        enabled: false,
        automatic_fallback: false,
        candidate_sha256: ZERO_SHA256.to_owned(),
    };
    candidate.candidate_sha256 = candidate_digest(&candidate)?;
    Ok(DisabledGatewayAdmission {
        candidate,
        reason_code: "model-gateway.candidate.identity-verified-activation-required",
        selected_route_id: None,
        fallback_enabled: false,
        proposal_has_authority: false,
    })
}

/// Verifies the immutable whole-tuple identity and disabled baseline.
pub fn verify_disabled_gateway_candidate(
    candidate: &DisabledGatewayCandidate,
    endpoint: &CanonicalModelEndpointProfile,
    trusted_operator_sha256: &str,
) -> Result<(), GatewayCandidateError> {
    endpoint
        .validate_canonical()
        .map_err(|_| GatewayCandidateError::InvalidIdentity)?;
    validate_model(&candidate.model)?;
    validate_runtime(&candidate.runtime_adapter)?;
    validate_codec(&candidate.protocol_codec)?;
    validate_route(&candidate.route)?;
    validate_operator(&candidate.operator)?;
    validate_qualification(&candidate.qualification)?;
    if candidate.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(&candidate.candidate_id)
        || !valid_id(&candidate.endpoint_profile_id)
        || !valid_sha256(&candidate.endpoint_profile_sha256)
        || candidate.endpoint_profile_id != endpoint.endpoint_profile_id
        || candidate.endpoint_profile_sha256
            != canonical_record_sha256(endpoint)
                .map_err(|_| GatewayCandidateError::SerializationFailed)?
        || candidate.endpoint_class != endpoint.profile_class
        || candidate.operator.operator_id != endpoint.operator_id
        || candidate.operator.operator_sha256 != trusted_operator_sha256
        || candidate.protocol_codec.protocol_codec_id != endpoint.protocol_codec_id
        || endpoint.qualification_sha256.as_deref()
            != Some(candidate.qualification.qualification_sha256.as_str())
        || candidate.enabled
        || candidate.automatic_fallback
        || !valid_sha256(&candidate.candidate_sha256)
        || candidate_digest(candidate)? != candidate.candidate_sha256
    {
        return Err(GatewayCandidateError::InvalidIdentity);
    }
    validate_credential_binding(endpoint, candidate.credential.as_ref())?;
    Ok(())
}

fn validate_credential_binding(
    endpoint: &CanonicalModelEndpointProfile,
    credential: Option<&GatewayCredentialReferenceIdentity>,
) -> Result<(), GatewayCandidateError> {
    match endpoint.profile_class {
        CanonicalEndpointClass::StrictLocal | CanonicalEndpointClass::LocalNetworkPrivate => {
            if endpoint.credential_reference.is_some() || credential.is_some() {
                return Err(GatewayCandidateError::IdentityMismatch);
            }
        }
        CanonicalEndpointClass::RemotePrivate | CanonicalEndpointClass::RemoteManaged => {
            let credential = credential.ok_or(GatewayCandidateError::IdentityMismatch)?;
            validate_credential(credential)?;
            if endpoint.credential_reference.as_deref()
                != Some(credential.credential_reference.as_str())
            {
                return Err(GatewayCandidateError::IdentityMismatch);
            }
        }
    }
    Ok(())
}

fn validate_model(value: &GatewayModelIdentity) -> Result<(), GatewayCandidateError> {
    valid_fields(
        &[
            &value.model_profile_id,
            &value.manifest_id,
            &value.model_revision_id,
        ],
        &[&value.manifest_sha256, &value.artifact_sha256],
    )
}

fn validate_runtime(value: &GatewayRuntimeAdapterIdentity) -> Result<(), GatewayCandidateError> {
    if value.contract_version == 0 {
        return Err(GatewayCandidateError::InvalidIdentity);
    }
    valid_fields(
        &[&value.adapter_id, &value.runtime_build_id],
        &[&value.runtime_sha256],
    )
}

fn validate_codec(value: &GatewayProtocolCodecIdentity) -> Result<(), GatewayCandidateError> {
    valid_fields(
        &[&value.protocol_codec_id, &value.protocol_codec_version],
        &[&value.protocol_codec_sha256],
    )
}

fn validate_route(value: &GatewayRouteIdentity) -> Result<(), GatewayCandidateError> {
    if value.route_version == 0 {
        return Err(GatewayCandidateError::InvalidIdentity);
    }
    valid_fields(&[&value.route_id], &[&value.route_policy_sha256])
}

fn validate_operator(value: &GatewayOperatorIdentity) -> Result<(), GatewayCandidateError> {
    if value.registry_version == 0 {
        return Err(GatewayCandidateError::InvalidIdentity);
    }
    valid_fields(&[&value.operator_id], &[&value.operator_sha256])
}

fn validate_credential(
    value: &GatewayCredentialReferenceIdentity,
) -> Result<(), GatewayCandidateError> {
    if value.credential_reference.contains("secret") || value.credential_reference.contains('@') {
        return Err(GatewayCandidateError::InvalidIdentity);
    }
    valid_fields(
        &[&value.credential_reference, &value.broker_version],
        &[&value.reference_sha256],
    )
}

fn validate_qualification(
    value: &GatewayQualificationIdentity,
) -> Result<(), GatewayCandidateError> {
    if value.qualification_version == 0 {
        return Err(GatewayCandidateError::InvalidIdentity);
    }
    valid_fields(&[&value.qualification_id], &[&value.qualification_sha256])
}

fn valid_fields(ids: &[&str], hashes: &[&str]) -> Result<(), GatewayCandidateError> {
    if ids.iter().all(|value| valid_id(value)) && hashes.iter().all(|value| valid_sha256(value)) {
        Ok(())
    } else {
        Err(GatewayCandidateError::InvalidIdentity)
    }
}

fn candidate_digest(value: &DisabledGatewayCandidate) -> Result<String, GatewayCandidateError> {
    let mut candidate = value.clone();
    candidate.candidate_sha256 = ZERO_SHA256.to_owned();
    let bytes =
        serde_json::to_vec(&candidate).map_err(|_| GatewayCandidateError::SerializationFailed)?;
    Ok(hex(&Sha256::digest(bytes)))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{
        CanonicalEndpointClass, CanonicalModelEndpointProfile, CanonicalTlsPolicy,
    };

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }
    fn model() -> GatewayModelIdentity {
        GatewayModelIdentity {
            model_profile_id: "model-profile-1".to_owned(),
            manifest_id: "manifest-1".to_owned(),
            manifest_sha256: hash('1'),
            model_revision_id: "model-revision-1".to_owned(),
            artifact_sha256: hash('2'),
        }
    }
    fn runtime() -> GatewayRuntimeAdapterIdentity {
        GatewayRuntimeAdapterIdentity {
            adapter_id: "adapter-1".to_owned(),
            contract_version: 1,
            runtime_build_id: "runtime-1".to_owned(),
            runtime_sha256: hash('3'),
        }
    }
    fn codec() -> GatewayProtocolCodecIdentity {
        GatewayProtocolCodecIdentity {
            protocol_codec_id: "codec-1".to_owned(),
            protocol_codec_version: "1".to_owned(),
            protocol_codec_sha256: hash('4'),
        }
    }
    fn route() -> GatewayRouteIdentity {
        GatewayRouteIdentity {
            route_id: "route-1".to_owned(),
            route_version: 1,
            route_policy_sha256: hash('5'),
        }
    }
    fn operator() -> GatewayOperatorIdentity {
        GatewayOperatorIdentity {
            operator_id: "operator-1".to_owned(),
            registry_version: 1,
            operator_sha256: hash('6'),
        }
    }
    fn qualification() -> GatewayQualificationIdentity {
        GatewayQualificationIdentity {
            qualification_id: "qualification-1".to_owned(),
            qualification_version: 1,
            qualification_sha256: hash('7'),
        }
    }
    fn endpoint(class: CanonicalEndpointClass) -> CanonicalModelEndpointProfile {
        let remote = matches!(
            class,
            CanonicalEndpointClass::RemotePrivate | CanonicalEndpointClass::RemoteManaged
        );
        CanonicalModelEndpointProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            endpoint_profile_id: "endpoint-1".to_owned(),
            profile_class: class,
            operator_id: "operator-1".to_owned(),
            endpoint_reference: if remote {
                "https://endpoint.example"
            } else {
                "local://runtime"
            }
            .to_owned(),
            protocol_codec_id: "codec-1".to_owned(),
            tls_policy: if remote {
                CanonicalTlsPolicy::VerifiedTls
            } else {
                CanonicalTlsPolicy::NotApplicableLocal
            },
            host_policy_sha256: hash('8'),
            credential_reference: remote.then(|| "credential://broker/reference-1".to_owned()),
            region: remote.then(|| "us-central".to_owned()),
            retention_policy: "retention-none".to_owned(),
            logging_policy: "logging-none".to_owned(),
            training_use_policy: "training-prohibited".to_owned(),
            quota_policy_sha256: hash('9'),
            cost_policy_sha256: hash('a'),
            qualification_sha256: Some(hash('7')),
            enabled: false,
            automatic_fallback: false,
        }
    }
    fn credential() -> GatewayCredentialReferenceIdentity {
        GatewayCredentialReferenceIdentity {
            credential_reference: "credential://broker/reference-1".to_owned(),
            broker_version: "1".to_owned(),
            reference_sha256: hash('b'),
        }
    }

    fn input(credential: Option<GatewayCredentialReferenceIdentity>) -> GatewayCandidateInput {
        GatewayCandidateInput {
            candidate_id: "candidate-1".to_owned(),
            model: model(),
            runtime_adapter: runtime(),
            protocol_codec: codec(),
            route: route(),
            operator: operator(),
            credential,
            qualification: qualification(),
        }
    }

    fn admit(
        class: CanonicalEndpointClass,
    ) -> Result<DisabledGatewayAdmission, GatewayCandidateError> {
        let credential = matches!(
            class,
            CanonicalEndpointClass::RemotePrivate | CanonicalEndpointClass::RemoteManaged
        )
        .then(credential);
        admit_disabled_gateway_candidate(&endpoint(class), input(credential), &hash('6'))
    }

    #[test]
    fn story_13_5_all_endpoint_classes_remain_disabled_without_fallback() {
        for class in [
            CanonicalEndpointClass::StrictLocal,
            CanonicalEndpointClass::LocalNetworkPrivate,
            CanonicalEndpointClass::RemotePrivate,
            CanonicalEndpointClass::RemoteManaged,
        ] {
            let admission = admit(class).expect("complete disabled tuple");
            assert_eq!(admission.candidate.endpoint_class, class);
            assert!(!admission.candidate.enabled);
            assert!(!admission.fallback_enabled);
            assert_eq!(admission.selected_route_id, None);
            assert!(!admission.proposal_has_authority);
            verify_disabled_gateway_candidate(&admission.candidate, &endpoint(class), &hash('6'))
                .expect("verified tuple");
        }
    }

    #[test]
    fn story_13_5_unknown_operator_and_partial_remote_tuple_fail_before_route() {
        assert_eq!(
            admit_disabled_gateway_candidate(
                &endpoint(CanonicalEndpointClass::StrictLocal),
                input(None),
                &hash('0')
            ),
            Err(GatewayCandidateError::UnknownOperator)
        );
        assert_eq!(
            admit_disabled_gateway_candidate(
                &endpoint(CanonicalEndpointClass::RemotePrivate),
                input(None),
                &hash('6')
            ),
            Err(GatewayCandidateError::IdentityMismatch)
        );
    }

    #[test]
    fn story_13_5_cross_class_credential_and_nested_identity_substitution_fail() {
        assert_eq!(
            admit_disabled_gateway_candidate(
                &endpoint(CanonicalEndpointClass::StrictLocal),
                input(Some(credential())),
                &hash('6')
            ),
            Err(GatewayCandidateError::IdentityMismatch)
        );
        let mut changed = codec();
        changed.protocol_codec_id = "codec-substitute".to_owned();
        let mut changed_input = input(None);
        changed_input.protocol_codec = changed;
        assert_eq!(
            admit_disabled_gateway_candidate(
                &endpoint(CanonicalEndpointClass::StrictLocal),
                changed_input,
                &hash('6')
            ),
            Err(GatewayCandidateError::IdentityMismatch)
        );
    }

    #[test]
    fn story_13_5_every_independent_identity_changes_whole_tuple_digest() {
        let original = admit(CanonicalEndpointClass::StrictLocal)
            .expect("original")
            .candidate;
        let mut changed = original.clone();
        changed.model.artifact_sha256 = hash('c');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(original.candidate_sha256, changed.candidate_sha256);
        let mut changed = original.clone();
        changed.runtime_adapter.runtime_sha256 = hash('d');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(original.candidate_sha256, changed.candidate_sha256);
        let mut changed = original.clone();
        changed.protocol_codec.protocol_codec_sha256 = hash('e');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(original.candidate_sha256, changed.candidate_sha256);
        let mut changed = original.clone();
        changed.endpoint_profile_sha256 = hash('f');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(original.candidate_sha256, changed.candidate_sha256);
        let mut changed = original.clone();
        changed.route.route_policy_sha256 = hash('1');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(original.candidate_sha256, changed.candidate_sha256);
        let mut changed = original.clone();
        changed.operator.operator_sha256 = hash('2');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(original.candidate_sha256, changed.candidate_sha256);
        let original_sha256 = original.candidate_sha256.clone();
        let mut changed = original;
        changed.qualification.qualification_sha256 = hash('3');
        changed.candidate_sha256 = ZERO_SHA256.to_owned();
        changed.candidate_sha256 = candidate_digest(&changed).expect("changed digest");
        assert_ne!(changed.candidate_sha256, original_sha256);
    }
}
