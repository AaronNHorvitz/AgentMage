//! Bounded model-response decoding, repair, replay detection, and advisory fallback.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ClosedModelProposal, ExactModelProfile, ModelFamilyCodec, ModelProfileId, ModelRunRequest,
};
use sha2::{Digest, Sha256};

/// Model repair candidates any profile may add to one bounded response ladder.
pub const MAX_MODEL_REPAIRS: u8 = 1;

const MAX_DECODE_ATTEMPTS: usize = 1 + MAX_MODEL_REPAIRS as usize;
const MAX_ADVISORY_BYTES: usize = 16 * 1024;
const MAX_FENCE_LANGUAGE_BYTES: usize = 32;

/// Inert response disposition after bounded family-codec validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelResponseDisposition {
    /// One complete closed proposal passed exact identity and digest checks.
    Proposed {
        /// Validated inert proposal.
        proposal: Box<ClosedModelProposal>,
        /// One-based candidate that produced it; `attempts - 1` model repairs ran.
        attempts: u8,
        /// Exact bounded-repair ladder step that admitted the candidate.
        admitted_by: RepairLadderStep,
    },
    /// Bounded plain text remains display-only advisory content.
    AdvisoryText {
        /// Exact advisory bytes as received, before normalization.
        bytes: Vec<u8>,
        /// Lowercase SHA-256 digest of the bytes.
        sha256: String,
        /// Number of examined candidates; `attempts - 1` model repairs ran.
        attempts: u8,
    },
    /// No response was admitted as a proposal or advisory.
    Rejected {
        /// Stable content-free reason code.
        reason_code: &'static str,
        /// Number of examined candidates; `attempts - 1` model repairs ran.
        attempts: u8,
    },
}

/// Exact bounded-repair ladder step that admitted one closed proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairLadderStep {
    /// The first candidate validated as received.
    Exact,
    /// Allowlisted deterministic normalization of the first candidate validated.
    Normalized,
    /// The single model repair candidate validated as received.
    ModelRepair,
    /// Deterministic normalization of the model repair candidate validated.
    NormalizedModelRepair,
}

/// Profile-bound permission for the single model repair step of the ladder.
///
/// The permission never widens the ladder: it can only allow the one repair
/// candidate that `MAX_MODEL_REPAIRS` already bounds, and it is refused when it
/// belongs to another profile or when any effect attempt already ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRepairPermission {
    /// Exact profile this permission was issued for.
    pub profile_id: ModelProfileId,
    /// Whether policy admits one targeted model repair for that profile.
    pub permitted: bool,
    /// Whether any effect attempt already ran for this response.
    pub effect_attempted: bool,
}

impl ModelRepairPermission {
    /// Permits the single model repair for one exact profile before any effect.
    #[must_use]
    pub fn permit(profile_id: ModelProfileId) -> Self {
        Self {
            profile_id,
            permitted: true,
            effect_attempted: false,
        }
    }

    /// Denies model repair for one exact profile.
    #[must_use]
    pub fn deny(profile_id: ModelProfileId) -> Self {
        Self {
            profile_id,
            permitted: false,
            effect_attempted: false,
        }
    }

    /// Records a prior effect attempt, which denies any further model repair.
    #[must_use]
    pub fn after_effect_attempt(mut self) -> Self {
        self.effect_attempted = true;
        self
    }
}

/// Decodes one candidate, then at most one permitted model repair candidate.
///
/// Each examined candidate is validated exactly as received and, only when
/// deterministic normalization changed it, once more in normalized form. That
/// deterministic step consumes no repair budget, so a wrapper-only defect never
/// costs the single profile-bound model repair. Nothing here executes, grants,
/// reconciles an effect, or claims completion.
#[must_use]
pub fn decode_with_bounded_repair<C: ModelFamilyCodec>(
    codec: &C,
    profile: &ExactModelProfile,
    request: &ModelRunRequest,
    candidates: &[Vec<u8>],
    repair: &ModelRepairPermission,
) -> ModelResponseDisposition {
    if candidates.is_empty() {
        return ModelResponseDisposition::Rejected {
            reason_code: "model.response.missing",
            attempts: 0,
        };
    }
    let denial = model_repair_denial(repair, profile);
    let examined = match denial {
        Some(_) => 1,
        None => MAX_DECODE_ATTEMPTS,
    };
    let mut seen = BTreeSet::new();
    let mut last = None;
    for (index, candidate) in candidates.iter().take(examined).enumerate() {
        let attempts = u8::try_from(index + 1).expect("bounded attempt count");
        let normalized = normalize_candidate(candidate);
        // Replay detection compares normalized bytes so that a wrapper-only
        // rewrite cannot present the same proposal as new progress.
        if !seen.insert(sha256(&normalized)) {
            return ModelResponseDisposition::Rejected {
                reason_code: "model.response.replayed",
                attempts,
            };
        }
        let repaired = index > 0;
        if let Ok(proposal) = codec.decode_proposal(profile, request, candidate) {
            return ModelResponseDisposition::Proposed {
                proposal: Box::new(proposal),
                attempts,
                admitted_by: if repaired {
                    RepairLadderStep::ModelRepair
                } else {
                    RepairLadderStep::Exact
                },
            };
        }
        if normalized != *candidate
            && let Ok(proposal) = codec.decode_proposal(profile, request, &normalized)
        {
            return ModelResponseDisposition::Proposed {
                proposal: Box::new(proposal),
                attempts,
                admitted_by: if repaired {
                    RepairLadderStep::NormalizedModelRepair
                } else {
                    RepairLadderStep::Normalized
                },
            };
        }
        last = Some((candidate, normalized, attempts));
    }
    let Some((candidate, normalized, attempts)) = last else {
        return ModelResponseDisposition::Rejected {
            reason_code: "model.response.missing",
            attempts: 0,
        };
    };
    // Advisory classification reads normalized bytes so that a fenced structured
    // failure cannot be downgraded to display-only prose.
    if plain_text_advisory(&normalized) {
        ModelResponseDisposition::AdvisoryText {
            bytes: candidate.clone(),
            sha256: sha256(candidate),
            attempts,
        }
    } else {
        ModelResponseDisposition::Rejected {
            reason_code: rejection_reason(denial, candidates.len()),
            attempts,
        }
    }
}

/// Applies the allowlisted deterministic normalization pass to one candidate.
///
/// The pass only removes an outer wrapper: a leading byte-order mark,
/// surrounding whitespace, and exactly one complete fenced code block with an
/// optional short language tag. It never inserts, substitutes, reorders, or
/// invents bytes, and it returns non-UTF-8 candidates unchanged. It is applied
/// exactly once, so nested wrappers stay unrepaired rather than unwrapped by an
/// unbounded loop.
#[must_use]
pub fn normalize_candidate(candidate: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(candidate) else {
        return candidate.to_vec();
    };
    let trimmed = text.strip_prefix('\u{feff}').unwrap_or(text).trim();
    let unwrapped = unwrap_single_code_fence(trimmed).unwrap_or(trimmed);
    unwrapped.trim().as_bytes().to_vec()
}

fn model_repair_denial(
    repair: &ModelRepairPermission,
    profile: &ExactModelProfile,
) -> Option<&'static str> {
    if repair.profile_id != profile.profile_id {
        Some("model.response.repair-profile-mismatch")
    } else if repair.effect_attempted {
        Some("model.response.repair-after-effect")
    } else if repair.permitted {
        None
    } else {
        Some("model.response.repair-not-permitted")
    }
}

fn rejection_reason(denial: Option<&'static str>, candidate_count: usize) -> &'static str {
    match denial {
        Some(reason_code) if candidate_count > 1 => reason_code,
        _ if candidate_count > 1 => "model.response.repair-exhausted",
        _ => "model.response.invalid",
    }
}

fn unwrap_single_code_fence(text: &str) -> Option<&str> {
    let inner = text.strip_prefix("```")?.strip_suffix("```")?;
    let (language, body) = inner.split_once('\n')?;
    let language = language.trim();
    if language.len() > MAX_FENCE_LANGUAGE_BYTES || body.contains("```") {
        return None;
    }
    if !language.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    Some(body)
}

pub(crate) fn plain_text_advisory(candidate: &[u8]) -> bool {
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
        ModelContextPacket, ModelFamilyCodec, ModelProfileId, ModelRunRequest, ModelRuntimeFailure,
    };

    use super::{
        ModelRepairPermission, ModelResponseDisposition, RepairLadderStep,
        decode_with_bounded_repair, normalize_candidate,
    };

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

    fn permit(profile: &ExactModelProfile) -> ModelRepairPermission {
        ModelRepairPermission::permit(profile.profile_id.clone())
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
            &permit(&profile),
        );
        assert!(matches!(
            result,
            ModelResponseDisposition::Proposed {
                attempts: 2,
                admitted_by: RepairLadderStep::ModelRepair,
                ..
            }
        ));
    }

    #[test]
    fn deterministic_normalization_admits_a_wrapper_defect_without_model_repair() {
        let (codec, profile, request) = fixture();
        let wrapped = "\u{feff}  ```json\nvalid-proposal\n```  ";
        let result = decode_with_bounded_repair(
            &codec,
            &profile,
            &request,
            &[wrapped.as_bytes().to_vec(), b"never-examined".to_vec()],
            &permit(&profile),
        );
        assert!(matches!(
            result,
            ModelResponseDisposition::Proposed {
                attempts: 1,
                admitted_by: RepairLadderStep::Normalized,
                ..
            }
        ));
        assert_eq!(
            normalize_candidate("```\nvalid-proposal\n```".as_bytes()),
            b"valid-proposal".to_vec()
        );
        for unchanged in [
            b"``` not a complete fence".to_vec(),
            "```json\n```\nvalid\n```".as_bytes().to_vec(),
            vec![0xff, 0xfe],
        ] {
            assert_eq!(normalize_candidate(&unchanged), unchanged);
        }
    }

    #[test]
    fn model_repair_is_denied_without_a_matching_profile_and_after_any_effect() {
        let (codec, profile, request) = fixture();
        let candidates = [b"{malformed".to_vec(), b"valid-proposal".to_vec()];
        let denied = |permission: &ModelRepairPermission, reason_code: &'static str| {
            assert_eq!(
                decode_with_bounded_repair(&codec, &profile, &request, &candidates, permission),
                ModelResponseDisposition::Rejected {
                    reason_code,
                    attempts: 1,
                }
            );
        };
        denied(
            &ModelRepairPermission::deny(profile.profile_id.clone()),
            "model.response.repair-not-permitted",
        );
        denied(
            &permit(&profile).after_effect_attempt(),
            "model.response.repair-after-effect",
        );
        denied(
            &ModelRepairPermission::permit(ModelProfileId::from_raw("other")),
            "model.response.repair-profile-mismatch",
        );
    }

    #[test]
    fn plain_text_is_bounded_display_only_and_structured_failures_never_downgrade() {
        let (codec, profile, request) = fixture();
        let permission = permit(&profile);
        let advisory = decode_with_bounded_repair(
            &codec,
            &profile,
            &request,
            &[b"I need more evidence before continuing.".to_vec()],
            &permission,
        );
        assert!(matches!(
            advisory,
            ModelResponseDisposition::AdvisoryText { attempts: 1, .. }
        ));
        for invalid in [
            b"{\"grant\":true}".to_vec(),
            "```json\n{\"grant\":true}\n```".as_bytes().to_vec(),
            vec![0xff],
            vec![b'x'; 16 * 1024 + 1],
            b"\0control".to_vec(),
        ] {
            assert!(matches!(
                decode_with_bounded_repair(&codec, &profile, &request, &[invalid], &permission),
                ModelResponseDisposition::Rejected { .. }
            ));
        }
    }

    #[test]
    fn replay_missing_and_exhausted_repairs_have_distinct_closed_results() {
        let (codec, profile, request) = fixture();
        assert_eq!(
            decode_with_bounded_repair(&codec, &profile, &request, &[], &permit(&profile)),
            ModelResponseDisposition::Rejected {
                reason_code: "model.response.missing",
                attempts: 0,
            }
        );
        for replay in [
            vec![b"{bad".to_vec(), b"{bad".to_vec()],
            vec![b"{bad".to_vec(), "```\n{bad\n```".as_bytes().to_vec()],
        ] {
            assert_eq!(
                decode_with_bounded_repair(&codec, &profile, &request, &replay, &permit(&profile)),
                ModelResponseDisposition::Rejected {
                    reason_code: "model.response.replayed",
                    attempts: 2,
                }
            );
        }
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
                &permit(&profile),
            ),
            ModelResponseDisposition::Rejected {
                reason_code: "model.response.repair-exhausted",
                attempts: 2,
            }
        );
    }
}
