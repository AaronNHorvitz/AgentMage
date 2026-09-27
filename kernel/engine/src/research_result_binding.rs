//! Pure full-response binding for the existing native authority/artifact owners.
//! Consistency is not native admission, producer attestation or qualification.
//! This is NOT a grant, native attestation or another execution interface.
//! The native producer must prove confinement/exit/cleanup; the canonical consumer
//! must match the exact actual terminal transaction, receipt and ToolCompleted hash.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::research_fetch::PublicGetWorkerPacket;
use crate::research_response::PublicGetResponse;

const MAX_BINDING_BYTES: usize = 4096;
const PRODUCER: &str = "agentmage.public-get.native-result.v1";

/// Claimed identities only; their syntax cannot establish native admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicGetNativeIdentity {
    /// Exact executable bytes, independently pinned by native admission.
    pub worker_sha256: String,
    /// Complete launcher/runtime/resolver manifest, not just the worker file.
    pub manifest_sha256: String,
    /// Exact admitted namespace/syscall/resource profile and its version.
    pub confinement_sha256: String,
}

/// Trusted parent observations, not values supplied by a model or a fetched page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicGetParentInterval {
    /// Before native preparation/launch, inside the original authorized interval.
    pub started_epoch_ms: u64,
    /// After successful exit and verified owned cleanup. Cleanup may finish later
    /// than the request deadline; the native owner separately proves timely exit.
    pub cleanup_verified_epoch_ms: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBinding {
    schema_version: u16,
    producer: String,
    request_sha256: String,
    reservation_sha256: String,
    native: PublicGetNativeIdentity,
    parent: PublicGetParentInterval,
    frame_sha256: String,
    frame_bytes: u64,
    body_sha256: String,
}

/// Content-free refusal; never format rejected JSON, addresses, queries or bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicGetResultBindingError {
    /// Malformed, noncanonical or unsupported producer material.
    Material,
    /// Exact request/reservation/native/frame identity does not match.
    Binding,
    /// Parent and worker observations cannot establish the original interval.
    Time,
    /// Complete response framing/content/bounds are invalid.
    Response,
}

/// Consistent full response and exact redacted native result material.
/// Construction and re-decoding establish ONLY consistency, not producer trust.
/// No serde/clone conversion or effect/artifact publication method is supplied.
pub struct PublicGetResultBinding {
    material: Vec<u8>,
    wire: WireBinding,
    response: PublicGetResponse,
}

impl fmt::Debug for PublicGetResultBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublicGetResultBinding")
            .field("request_sha256", &self.wire.request_sha256)
            .field("frame_sha256", &self.wire.frame_sha256)
            .finish_non_exhaustive()
    }
}

impl PublicGetResultBinding {
    /// Called only with a complete native success by the trusted driver. Public
    /// for shared pure fixtures; it cannot attest that those observations happened.
    pub fn seal(
        packet: &PublicGetWorkerPacket,
        reservation_sha256: &str,
        native: PublicGetNativeIdentity,
        parent: PublicGetParentInterval,
        frame: &[u8],
    ) -> Result<Self, PublicGetResultBindingError> {
        if !valid_digest(reservation_sha256)
            || !valid_digest(&native.worker_sha256)
            || !valid_digest(&native.manifest_sha256)
            || !valid_digest(&native.confinement_sha256)
        {
            return Err(PublicGetResultBindingError::Binding);
        }
        if parent.started_epoch_ms < packet.prepared_at_epoch_ms()
            || parent.started_epoch_ms >= packet.deadline_epoch_ms()
            || parent.cleanup_verified_epoch_ms < parent.started_epoch_ms
        {
            return Err(PublicGetResultBindingError::Time);
        }
        let response = PublicGetResponse::decode(packet, frame, parent.cleanup_verified_epoch_ms)
            .map_err(|_| PublicGetResultBindingError::Response)?;
        if response.observation().started_epoch_ms < parent.started_epoch_ms {
            return Err(PublicGetResultBindingError::Time);
        }
        let wire = WireBinding {
            schema_version: 1,
            producer: PRODUCER.to_owned(),
            request_sha256: packet.sha256().to_owned(),
            reservation_sha256: reservation_sha256.to_owned(),
            native,
            parent,
            frame_sha256: digest(frame),
            frame_bytes: frame.len() as u64,
            body_sha256: response.observation().body_sha256.clone(),
        };
        // Like the existing local worker packet/response schema, this is schema1,
        // not a frozen public VersionedContract (currently schema2).
        let material =
            serde_json::to_vec(&wire).map_err(|_| PublicGetResultBindingError::Material)?;
        if material.len() > MAX_BINDING_BYTES {
            return Err(PublicGetResultBindingError::Material);
        }
        Ok(Self {
            material,
            wire,
            response,
        })
    }

    /// Recomputes original producer material from separately retained full bytes.
    /// The caller MUST additionally check the actual canonical native result hash,
    /// consumed grant/receipt/event/owner/lifecycle and producer admission. This
    /// method intentionally cannot turn caller-supplied hashes into retrieval proof.
    pub fn decode_consistent(
        packet: &PublicGetWorkerPacket,
        reservation_sha256: &str,
        material: &[u8],
        frame: &[u8],
    ) -> Result<Self, PublicGetResultBindingError> {
        if material.is_empty() || material.len() > MAX_BINDING_BYTES {
            return Err(PublicGetResultBindingError::Material);
        }
        let wire: WireBinding =
            serde_json::from_slice(material).map_err(|_| PublicGetResultBindingError::Material)?;
        if wire.schema_version != 1 || wire.producer != PRODUCER {
            return Err(PublicGetResultBindingError::Material);
        }
        if wire.request_sha256 != packet.sha256() || wire.reservation_sha256 != reservation_sha256 {
            return Err(PublicGetResultBindingError::Binding);
        }
        let rebuilt = Self::seal(packet, reservation_sha256, wire.native, wire.parent, frame)?;
        // The producer always emits one exact compact struct serialization. This also
        // checks reported frame/body digests and sizes, without another hash grammar.
        if rebuilt.material != material {
            return Err(PublicGetResultBindingError::Binding);
        }
        Ok(rebuilt)
    }

    /// Bytes to digest into the existing authority EffectResult, not ToolResult.
    #[must_use]
    pub fn redacted_material(&self) -> &[u8] {
        &self.material
    }

    /// Complete immutable frame/body for the existing canonical artifact owner.
    #[must_use]
    pub const fn response(&self) -> &PublicGetResponse {
        &self.response
    }

    /// Claim to match against the trusted native producer and actual effect receipt.
    #[must_use]
    pub const fn native_identity(&self) -> &PublicGetNativeIdentity {
        &self.wire.native
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_budget::{
        ResearchDepth, ResearchLimits, ResearchNetworkMode, ResearchScope,
    };
    use crate::research_fetch::{PreparedPublicGet, PublicGetDraft, PublicGetTarget};
    use crate::research_response::{PublicGetHop, PublicGetObservation, PublicSourceMedia};
    use std::collections::BTreeSet;

    fn packet() -> PreparedPublicGet {
        let scope = ResearchScope::new(
            "task-0001".into(),
            ResearchDepth::Quick,
            ResearchNetworkMode::Ask,
            ResearchLimits::ceiling(ResearchDepth::Quick),
            BTreeSet::from(["docs.example.com".into()]),
            &["synthetic public query".into()],
        )
        .unwrap();
        PreparedPublicGet::prepare(
            &scope,
            PublicGetDraft {
                schema_version: 1,
                operation_id: "call-0001".into(),
                target: PublicGetTarget {
                    domain: "docs.example.com".into(),
                    path: "/api".into(),
                    query: vec![("q".into(), "synthetic public query".into())],
                },
                maximum_response_bytes: 1024,
                redirect_limit: 0,
                timeout_ms: 1000,
            },
            1000,
            3000,
        )
        .unwrap()
    }

    fn result(packet: &PublicGetWorkerPacket) -> PublicGetResultBinding {
        let body = b"Synthetic public source. Inert content is not authority.";
        let response = PublicGetResponse::encode(
            packet,
            PublicGetObservation {
                schema_version: 1,
                request_sha256: packet.sha256().into(),
                operation_id: packet.request().operation_id.clone(),
                started_epoch_ms: 3200,
                completed_epoch_ms: 3300,
                hops: vec![PublicGetHop {
                    target: packet.request().target.clone(),
                    status: 200,
                    header_bytes: 80,
                    body_bytes: body.len() as u64,
                }],
                media: PublicSourceMedia::Text,
                body_sha256: digest(body),
            },
            body,
            3300,
        )
        .unwrap();
        PublicGetResultBinding::seal(
            packet,
            &"1".repeat(64),
            PublicGetNativeIdentity {
                worker_sha256: "2".repeat(64),
                manifest_sha256: "3".repeat(64),
                confinement_sha256: "4".repeat(64),
            },
            PublicGetParentInterval {
                started_epoch_ms: 3100,
                cleanup_verified_epoch_ms: 3400,
            },
            response.frame(),
        )
        .unwrap()
    }

    #[test]
    fn exact_round_trip_preserves_full_bytes_without_making_attestation_claims() {
        let prepared = packet();
        let packet = prepared.packet();
        let original = result(packet);
        let checked = PublicGetResultBinding::decode_consistent(
            packet,
            &"1".repeat(64),
            original.redacted_material(),
            original.response().frame(),
        )
        .unwrap();
        assert_eq!(checked.redacted_material(), original.redacted_material());
        assert_eq!(checked.response().body(), original.response().body());
        let material = std::str::from_utf8(checked.redacted_material()).unwrap();
        assert!(!material.contains("docs.example.com"));
        assert!(!material.contains("synthetic public query"));
        assert!(!material.contains("Inert content"));
        assert!(!format!("{checked:?}").contains("Synthetic"));
    }

    #[test]
    fn altered_material_reservation_frame_and_schema_are_refused() {
        let prepared = packet();
        let packet = prepared.packet();
        let original = result(packet);
        for field in 0..7 {
            // Keep the exact producer field ordering, so a mutation cannot pass
            // merely because a map serializer introduced a different order.
            let mut wire: WireBinding =
                serde_json::from_slice(original.redacted_material()).unwrap();
            match field {
                0 => wire.schema_version = 2,
                1 => wire.producer = "another.producer".into(),
                2 => wire.request_sha256 = "5".repeat(64),
                3 => wire.reservation_sha256 = "5".repeat(64),
                4 => wire.frame_sha256 = "5".repeat(64),
                5 => wire.frame_bytes += 1,
                6 => wire.body_sha256 = "5".repeat(64),
                _ => unreachable!(),
            }
            let material = serde_json::to_vec(&wire).unwrap();
            assert_ne!(material, original.redacted_material());
            assert!(
                PublicGetResultBinding::decode_consistent(
                    packet,
                    &"1".repeat(64),
                    &material,
                    original.response().frame()
                )
                .is_err(),
                "{field}"
            );
        }
        let mut changed = original.response().frame().to_vec();
        *changed.last_mut().unwrap() ^= 1;
        assert!(
            PublicGetResultBinding::decode_consistent(
                packet,
                &"1".repeat(64),
                original.redacted_material(),
                &changed
            )
            .is_err()
        );
        assert!(
            PublicGetResultBinding::decode_consistent(
                packet,
                &"6".repeat(64),
                original.redacted_material(),
                original.response().frame()
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_noncanonical_duplicate_unknown_and_oversized_material_is_rejected() {
        let prepared = packet();
        let packet = prepared.packet();
        let original = result(packet);
        let text = std::str::from_utf8(original.redacted_material()).unwrap();
        for material in [
            Vec::new(),
            vec![b' '; MAX_BINDING_BYTES + 1],
            vec![0xff],
            format!(" {text}").into_bytes(),
            text.replacen("{", "{\"extra\":true,", 1).into_bytes(),
            text.replacen("{", "{\"schema_version\":1,", 1).into_bytes(),
            text.replace("\"schema_version\":1", "\"schema_version\":1.0")
                .into_bytes(),
            text.replace("\"schema_version\":1", "\"schema_version\":true")
                .into_bytes(),
            text.replacen("\"native\":{", "\"native\":{\"extra\":true,", 1)
                .into_bytes(),
        ] {
            assert_ne!(material, original.redacted_material());
            assert!(
                PublicGetResultBinding::decode_consistent(
                    packet,
                    &"1".repeat(64),
                    &material,
                    original.response().frame()
                )
                .is_err()
            );
        }
        for frame in [
            &original.response().frame()[..3],
            &original.response().frame()[..original.response().frame().len() - 1],
        ] {
            assert!(
                PublicGetResultBinding::decode_consistent(
                    packet,
                    &"1".repeat(64),
                    original.redacted_material(),
                    frame
                )
                .is_err()
            );
        }
    }

    #[test]
    fn every_claimed_identity_requires_a_complete_nonzero_lowercase_digest() {
        let prepared = packet();
        let packet = prepared.packet();
        let original = result(packet);
        for invalid in [
            "".into(),
            "0".repeat(64),
            "A".repeat(64),
            "x".repeat(64),
            "1".repeat(63),
            "1".repeat(65),
        ] {
            for field in 0..4 {
                let mut native = original.native_identity().clone();
                let mut reservation = "1".repeat(64);
                match field {
                    0 => reservation = invalid.clone(),
                    1 => native.worker_sha256 = invalid.clone(),
                    2 => native.manifest_sha256 = invalid.clone(),
                    3 => native.confinement_sha256 = invalid.clone(),
                    _ => unreachable!(),
                }
                let error = PublicGetResultBinding::seal(
                    packet,
                    &reservation,
                    native,
                    original.wire.parent,
                    original.response().frame(),
                )
                .unwrap_err();
                assert_eq!(error, PublicGetResultBindingError::Binding);
            }
        }
    }

    #[test]
    fn consistent_forgery_still_requires_canonical_producer_attestation() {
        use crate::authority_transaction::EffectResult;
        use agentmage_kernel_contracts::{OperationOutcome, StateChange};
        let prepared = packet();
        let packet = prepared.packet();
        let original = result(packet);
        let mut altered = original.native_identity().clone();
        altered.worker_sha256 = "5".repeat(64);
        let forged = PublicGetResultBinding::seal(
            packet,
            &"1".repeat(64),
            altered,
            original.wire.parent,
            original.response().frame(),
        )
        .unwrap();
        // Both are syntactically consistent. Only an independently matched native
        // transaction/receipt may choose a producer; this pure type cannot do so.
        assert!(
            PublicGetResultBinding::decode_consistent(
                packet,
                &"1".repeat(64),
                forged.redacted_material(),
                forged.response().frame()
            )
            .is_ok()
        );
        let effect = |binding: &PublicGetResultBinding| {
            EffectResult::from_redacted_material(
                OperationOutcome::Succeeded,
                binding.redacted_material(),
                StateChange::NotChanged,
            )
        };
        assert_ne!(
            effect(&original).result_sha256(),
            effect(&forged).result_sha256()
        );
    }

    #[test]
    fn original_parent_window_and_worker_start_cannot_be_rewritten() {
        let prepared = packet();
        let packet = prepared.packet();
        let original = result(packet);
        for parent in [
            PublicGetParentInterval {
                started_epoch_ms: 2999,
                cleanup_verified_epoch_ms: 3400,
            },
            PublicGetParentInterval {
                started_epoch_ms: 3201,
                cleanup_verified_epoch_ms: 3400,
            },
            PublicGetParentInterval {
                started_epoch_ms: 4000,
                cleanup_verified_epoch_ms: 4100,
            },
            PublicGetParentInterval {
                started_epoch_ms: 3100,
                cleanup_verified_epoch_ms: 3099,
            },
            PublicGetParentInterval {
                started_epoch_ms: 3100,
                cleanup_verified_epoch_ms: 3299,
            },
        ] {
            assert!(
                PublicGetResultBinding::seal(
                    packet,
                    &"1".repeat(64),
                    original.native_identity().clone(),
                    parent,
                    original.response().frame()
                )
                .is_err()
            );
        }
        // Historical decoding preserves the ORIGINAL window. This observation is
        // after a proven timely native exit, not permission for a delayed request.
        let later_cleanup = PublicGetParentInterval {
            started_epoch_ms: 3100,
            cleanup_verified_epoch_ms: 4100,
        };
        assert!(
            PublicGetResultBinding::seal(
                packet,
                &"1".repeat(64),
                original.native_identity().clone(),
                later_cleanup,
                original.response().frame()
            )
            .is_ok()
        );
    }
}
