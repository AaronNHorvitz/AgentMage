//! Bounded model-response decoding, repair, replay detection, and advisory fallback.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ClosedModelProposal, ExactModelProfile, ModelFamilyCodec, ModelRunRequest,
};
use sha2::{Digest, Sha256};

const MAX_DECODE_ATTEMPTS: usize = 2;
const MAX_ADVISORY_BYTES: usize = 16 * 1024;

/// Inert response disposition after bounded family-codec validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelResponseDisposition {
    /// One complete closed proposal passed exact identity and digest checks.
    Proposed {
        /// Validated inert proposal.
        proposal: Box<ClosedModelProposal>,
        /// One-based attempt that produced it.
        attempts: u8,
    },
    /// Bounded plain text remains display-only advisory content.
    AdvisoryText {
        /// Exact advisory bytes.
        bytes: Vec<u8>,
        /// Lowercase SHA-256 digest of the bytes.
        sha256: String,
        /// Number of bounded attempts consumed.
        attempts: u8,
    },
    /// No response was admitted as a proposal or advisory.
    Rejected {
        /// Stable content-free reason code.
        reason_code: &'static str,
        /// Number of bounded attempts consumed.
        attempts: u8,
    },
}

/// Decodes at most two candidates without executing, granting, or claiming completion.
#[must_use]
pub fn decode_with_bounded_repair<C: ModelFamilyCodec>(
    codec: &C,
    profile: &ExactModelProfile,
    request: &ModelRunRequest,
    candidates: &[Vec<u8>],
) -> ModelResponseDisposition {
    if candidates.is_empty() {
        return ModelResponseDisposition::Rejected {
            reason_code: "model.response.missing",
            attempts: 0,
        };
    }
    let mut seen = BTreeSet::new();
    let mut last = None;
    for (index, candidate) in candidates.iter().take(MAX_DECODE_ATTEMPTS).enumerate() {
        let attempts = u8::try_from(index + 1).expect("bounded attempt count");
        let candidate_sha256 = sha256(candidate);
        if !seen.insert(candidate_sha256.clone()) {
            return ModelResponseDisposition::Rejected {
                reason_code: "model.response.replayed",
                attempts,
            };
        }
        if let Ok(proposal) = codec.decode_proposal(profile, request, candidate) {
            return ModelResponseDisposition::Proposed {
                proposal: Box::new(proposal),
                attempts,
            };
        }
        last = Some((candidate, candidate_sha256, attempts));
    }
    let Some((candidate, candidate_sha256, attempts)) = last else {
        return ModelResponseDisposition::Rejected {
            reason_code: "model.response.missing",
            attempts: 0,
        };
    };
    if plain_text_advisory(candidate) {
        ModelResponseDisposition::AdvisoryText {
            bytes: candidate.clone(),
            sha256: candidate_sha256,
            attempts,
        }
    } else {
        ModelResponseDisposition::Rejected {
            reason_code: if candidates.len() > MAX_DECODE_ATTEMPTS {
                "model.response.repair-exhausted"
            } else {
                "model.response.invalid"
            },
            attempts,
        }
    }
}

fn plain_text_advisory(candidate: &[u8]) -> bool {
    if candidate.is_empty() || candidate.len() > MAX_ADVISORY_BYTES {
        return false;
    }
    let Ok(text) = std::str::from_utf8(candidate) else {
        return false;
    };
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('{')
        && !trimmed.starts_with('[')
        && text
            .chars()
            .all(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ClosedModelProposal, EncodedModelContext, ExactModelProfile, FamilyCodecIdentity,
        ModelContextPacket, ModelFamilyCodec, ModelRunRequest, ModelRuntimeFailure,
    };

    use super::{ModelResponseDisposition, decode_with_bounded_repair};

    #[derive(Clone)]
    struct FixtureCodec {
        identity: FamilyCodecIdentity,
        accepted: Vec<u8>,
        proposal: ClosedModelProposal,
    }

    impl ModelFamilyCodec for FixtureCodec {
        fn identity(&self) -> &FamilyCodecIdentity {
            &self.identity
        }

        fn encode_context(
            &self,
            _profile: &ExactModelProfile,
            _packet: &ModelContextPacket,
        ) -> Result<EncodedModelContext, ModelRuntimeFailure> {
            unreachable!("response fixture does not encode")
        }

        fn decode_proposal(
            &self,
            _profile: &ExactModelProfile,
            _request: &ModelRunRequest,
            response: &[u8],
        ) -> Result<ClosedModelProposal, ModelRuntimeFailure> {
            if response == self.accepted {
                Ok(self.proposal.clone())
            } else {
                Err(ModelRuntimeFailure {
                    code: "fixture.invalid".to_owned(),
                    retryable_after_correction: true,
                    dependency_recovery_required: false,
                    contract_error: None,
                })
            }
        }
    }

    fn fixture() -> (FixtureCodec, ExactModelProfile, ModelRunRequest) {
        use crate::model_codec::tests_support;
        let profile = tests_support::profile("response");
        let request = tests_support::request(&profile);
        let proposal = tests_support::proposal(&profile);
        (
            FixtureCodec {
                identity: profile.codec.clone(),
                accepted: b"valid-proposal".to_vec(),
                proposal,
            },
            profile,
            request,
        )
    }

    #[test]
    fn corrected_second_candidate_is_admitted_and_third_is_never_examined() {
        let (codec, profile, request) = fixture();
        let result = decode_with_bounded_repair(
            &codec,
            &profile,
            &request,
            &[
                b"{malformed".to_vec(),
                b"valid-proposal".to_vec(),
                b"ignored".to_vec(),
            ],
        );
        assert!(matches!(
            result,
            ModelResponseDisposition::Proposed { attempts: 2, .. }
        ));
    }

    #[test]
    fn plain_text_is_bounded_display_only_and_structured_failures_never_downgrade() {
        let (codec, profile, request) = fixture();
        let advisory = decode_with_bounded_repair(
            &codec,
            &profile,
            &request,
            &[b"I need more evidence before continuing.".to_vec()],
        );
        assert!(matches!(
            advisory,
            ModelResponseDisposition::AdvisoryText { attempts: 1, .. }
        ));
        for invalid in [
            b"{\"grant\":true}".to_vec(),
            vec![0xff],
            vec![b'x'; 16 * 1024 + 1],
            b"\0control".to_vec(),
        ] {
            assert!(matches!(
                decode_with_bounded_repair(&codec, &profile, &request, &[invalid]),
                ModelResponseDisposition::Rejected { .. }
            ));
        }
    }

    #[test]
    fn replay_missing_and_exhausted_repairs_have_distinct_closed_results() {
        let (codec, profile, request) = fixture();
        assert_eq!(
            decode_with_bounded_repair(&codec, &profile, &request, &[]),
            ModelResponseDisposition::Rejected {
                reason_code: "model.response.missing",
                attempts: 0,
            }
        );
        assert_eq!(
            decode_with_bounded_repair(
                &codec,
                &profile,
                &request,
                &[b"{bad".to_vec(), b"{bad".to_vec()],
            ),
            ModelResponseDisposition::Rejected {
                reason_code: "model.response.replayed",
                attempts: 2,
            }
        );
        assert_eq!(
            decode_with_bounded_repair(
                &codec,
                &profile,
                &request,
                &[
                    b"{one".to_vec(),
                    b"[two".to_vec(),
                    b"valid-proposal".to_vec()
                ],
            ),
            ModelResponseDisposition::Rejected {
                reason_code: "model.response.repair-exhausted",
                attempts: 2,
            }
        );
    }
}
