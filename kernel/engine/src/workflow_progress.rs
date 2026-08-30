//! Deterministic workflow-state fingerprints and repeated no-progress detection.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use sha2::{Digest, Sha256};

const SHA256_HEX_LENGTH: usize = 64;
const MAX_ORDERED_REFERENCES: usize = 1_024;

/// Content identities for every state dimension that can establish workflow progress.
///
/// Each value is the lowercase SHA-256 of the canonical object named by the field. Empty
/// collections are explicit state; optional or unknown objects must therefore have their own
/// canonical sentinel representation and digest rather than being omitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowStateComponents {
    /// Canonical plan identity.
    pub plan_sha256: String,
    /// Canonical active-step identity.
    pub step_sha256: String,
    /// Ordered canonical observation identities.
    pub observation_sha256: Vec<String>,
    /// Canonical proposal identity, including an explicit no-proposal state.
    pub proposal_sha256: String,
    /// Canonical selected-tool identity, including version and an explicit no-tool state.
    pub tool_sha256: String,
    /// Canonical effective-policy identity.
    pub policy_sha256: String,
    /// Ordered canonical receipt identities.
    pub receipt_sha256: Vec<String>,
    /// Ordered canonical artifact identities.
    pub artifact_sha256: Vec<String>,
    /// Canonical verifier-state identity.
    pub verifier_state_sha256: String,
}

/// Immutable digest over the complete progress-relevant workflow state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkflowStateFingerprint {
    sha256: String,
}

impl WorkflowStateFingerprint {
    /// Validates complete component identities and derives one domain-separated digest.
    pub fn new(components: &WorkflowStateComponents) -> Result<Self, WorkflowProgressError> {
        validate_components(components)?;
        let mut digest = Sha256::new();
        digest.update(b"agentmage.workflow-state-fingerprint.v1\0");
        hash_field(&mut digest, b"plan", &components.plan_sha256);
        hash_field(&mut digest, b"step", &components.step_sha256);
        hash_sequence(&mut digest, b"observations", &components.observation_sha256);
        hash_field(&mut digest, b"proposal", &components.proposal_sha256);
        hash_field(&mut digest, b"tool", &components.tool_sha256);
        hash_field(&mut digest, b"policy", &components.policy_sha256);
        hash_sequence(&mut digest, b"receipts", &components.receipt_sha256);
        hash_sequence(&mut digest, b"artifacts", &components.artifact_sha256);
        hash_field(
            &mut digest,
            b"verifier-state",
            &components.verifier_state_sha256,
        );
        Ok(Self {
            sha256: finish_digest(digest),
        })
    }

    /// Returns the lowercase content-derived SHA-256 identity.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Immutable repeated-state policy with a content-derived identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatedStatePolicy {
    policy_id: String,
    policy_sha256: String,
    max_repeats_per_fingerprint: u64,
}

impl RepeatedStatePolicy {
    /// Creates an exact policy. At least one repeat must be allowed so callers can distinguish a
    /// repeated observation from the terminal no-progress decision.
    pub fn new(
        policy_id: String,
        max_repeats_per_fingerprint: u64,
    ) -> Result<Self, WorkflowProgressError> {
        if !valid_identifier(&policy_id) || max_repeats_per_fingerprint == 0 {
            return Err(WorkflowProgressError::InvalidPolicy);
        }
        let mut policy = Self {
            policy_id,
            policy_sha256: String::new(),
            max_repeats_per_fingerprint,
        };
        let mut digest = Sha256::new();
        digest.update(b"agentmage.repeated-state-policy.v1\0");
        hash_field(&mut digest, b"policy-id", &policy.policy_id);
        digest.update(policy.max_repeats_per_fingerprint.to_be_bytes());
        policy.policy_sha256 = finish_digest(digest);
        Ok(policy)
    }

    /// Returns the stable descriptive policy identity.
    #[must_use]
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    /// Returns the content-derived policy identity.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Returns the exact repeat count that produces a terminal stop.
    #[must_use]
    pub const fn max_repeats_per_fingerprint(&self) -> u64 {
        self.max_repeats_per_fingerprint
    }
}

/// One deterministic progress decision. It contains no grant, effect permit, or executable data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkflowProgressDecision {
    /// A state not previously observed in this detector.
    Advanced {
        /// Complete state fingerprint.
        fingerprint_sha256: String,
        /// Number of distinct states observed after this decision.
        distinct_states: u64,
    },
    /// A previously observed state below the terminal repeat limit.
    Repeated {
        /// Repeated complete state fingerprint.
        fingerprint_sha256: String,
        /// Number of repeats after the original observation.
        repeat_count: u64,
        /// Repeats remaining before terminal stop.
        repeats_remaining: u64,
    },
    /// The exact fingerprint reached the declared repeat limit.
    StopRepeatedState {
        /// Repeated complete state fingerprint.
        fingerprint_sha256: String,
        /// Number of repeats after the original observation.
        repeat_count: u64,
    },
}

/// Closed construction or observation failure that never changes detector state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowProgressError {
    /// A policy identity or limit is invalid.
    InvalidPolicy,
    /// A component digest, sequence length, or required field is invalid.
    InvalidState,
    /// A different immutable repeated-state policy was supplied.
    PolicyMismatch,
    /// An occurrence or distinct-state counter overflowed.
    CounterOverflow,
}

/// Policy-bound history that stops any recurring fingerprint, including non-adjacent cycles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatedStateDetector {
    policy_id: String,
    policy_sha256: String,
    occurrences: BTreeMap<String, u64>,
    stopped: Option<WorkflowProgressDecision>,
}

impl RepeatedStateDetector {
    /// Creates an empty detector permanently bound to one repeated-state policy.
    #[must_use]
    pub fn new(policy: &RepeatedStatePolicy) -> Self {
        Self {
            policy_id: policy.policy_id.clone(),
            policy_sha256: policy.policy_sha256.clone(),
            occurrences: BTreeMap::new(),
            stopped: None,
        }
    }

    /// Observes one complete fingerprint and returns a deterministic progress decision.
    ///
    /// Once stopped, the detector returns the same stop decision without changing counters.
    pub fn observe(
        &mut self,
        policy: &RepeatedStatePolicy,
        fingerprint: &WorkflowStateFingerprint,
    ) -> Result<WorkflowProgressDecision, WorkflowProgressError> {
        if self.policy_id != policy.policy_id || self.policy_sha256 != policy.policy_sha256 {
            return Err(WorkflowProgressError::PolicyMismatch);
        }
        if let Some(stopped) = &self.stopped {
            return Ok(stopped.clone());
        }

        let Some(previous_occurrences) = self.occurrences.get(fingerprint.sha256()).copied() else {
            let distinct_states = u64::try_from(self.occurrences.len())
                .ok()
                .and_then(|count| count.checked_add(1))
                .ok_or(WorkflowProgressError::CounterOverflow)?;
            self.occurrences.insert(fingerprint.sha256.clone(), 1);
            return Ok(WorkflowProgressDecision::Advanced {
                fingerprint_sha256: fingerprint.sha256.clone(),
                distinct_states,
            });
        };

        let occurrences = previous_occurrences
            .checked_add(1)
            .ok_or(WorkflowProgressError::CounterOverflow)?;
        let repeat_count = occurrences - 1;
        let decision = if repeat_count >= policy.max_repeats_per_fingerprint {
            WorkflowProgressDecision::StopRepeatedState {
                fingerprint_sha256: fingerprint.sha256.clone(),
                repeat_count,
            }
        } else {
            WorkflowProgressDecision::Repeated {
                fingerprint_sha256: fingerprint.sha256.clone(),
                repeat_count,
                repeats_remaining: policy.max_repeats_per_fingerprint - repeat_count,
            }
        };
        self.occurrences
            .insert(fingerprint.sha256.clone(), occurrences);
        if matches!(decision, WorkflowProgressDecision::StopRepeatedState { .. }) {
            self.stopped = Some(decision.clone());
        }
        Ok(decision)
    }

    /// Returns whether repeated-state termination is already sticky.
    #[must_use]
    pub const fn is_stopped(&self) -> bool {
        self.stopped.is_some()
    }

    /// Returns the admitted occurrence count for an exact fingerprint.
    #[must_use]
    pub fn occurrences(&self, fingerprint: &WorkflowStateFingerprint) -> u64 {
        self.occurrences
            .get(fingerprint.sha256())
            .copied()
            .unwrap_or(0)
    }
}

fn validate_components(components: &WorkflowStateComponents) -> Result<(), WorkflowProgressError> {
    let scalars = [
        &components.plan_sha256,
        &components.step_sha256,
        &components.proposal_sha256,
        &components.tool_sha256,
        &components.policy_sha256,
        &components.verifier_state_sha256,
    ];
    if scalars.into_iter().any(|value| !valid_sha256(value))
        || components.observation_sha256.len() > MAX_ORDERED_REFERENCES
        || components.receipt_sha256.len() > MAX_ORDERED_REFERENCES
        || components.artifact_sha256.len() > MAX_ORDERED_REFERENCES
        || components
            .observation_sha256
            .iter()
            .chain(&components.receipt_sha256)
            .chain(&components.artifact_sha256)
            .any(|value| !valid_sha256(value))
    {
        return Err(WorkflowProgressError::InvalidState);
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hash_field(digest: &mut Sha256, tag: &[u8], value: &str) {
    digest.update(u64::try_from(tag.len()).unwrap_or(u64::MAX).to_be_bytes());
    digest.update(tag);
    digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    digest.update(value.as_bytes());
}

fn hash_sequence(digest: &mut Sha256, tag: &[u8], values: &[String]) {
    digest.update(u64::try_from(tag.len()).unwrap_or(u64::MAX).to_be_bytes());
    digest.update(tag);
    digest.update(
        u64::try_from(values.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for value in values {
        digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
        digest.update(value.as_bytes());
    }
}

fn finish_digest(digest: Sha256) -> String {
    let mut output = String::with_capacity(SHA256_HEX_LENGTH);
    for byte in digest.finalize() {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(SHA256_HEX_LENGTH)
    }

    fn components() -> WorkflowStateComponents {
        WorkflowStateComponents {
            plan_sha256: digest('0'),
            step_sha256: digest('1'),
            observation_sha256: vec![digest('2'), digest('3')],
            proposal_sha256: digest('4'),
            tool_sha256: digest('5'),
            policy_sha256: digest('6'),
            receipt_sha256: vec![digest('7'), digest('8')],
            artifact_sha256: vec![digest('9'), digest('a')],
            verifier_state_sha256: digest('b'),
        }
    }

    fn fingerprint(components: &WorkflowStateComponents) -> WorkflowStateFingerprint {
        WorkflowStateFingerprint::new(components).expect("complete state")
    }

    #[test]
    fn fingerprint_is_deterministic_complete_and_boundary_preserving() {
        let original = components();
        assert_eq!(fingerprint(&original), fingerprint(&original));
        let expected = fingerprint(&original);

        for dimension in 0..9 {
            let mut changed = original.clone();
            match dimension {
                0 => changed.plan_sha256 = digest('c'),
                1 => changed.step_sha256 = digest('c'),
                2 => changed.observation_sha256[0] = digest('c'),
                3 => changed.proposal_sha256 = digest('c'),
                4 => changed.tool_sha256 = digest('c'),
                5 => changed.policy_sha256 = digest('c'),
                6 => changed.receipt_sha256[0] = digest('c'),
                7 => changed.artifact_sha256[0] = digest('c'),
                8 => changed.verifier_state_sha256 = digest('c'),
                _ => unreachable!("closed test dimension"),
            }
            assert_ne!(fingerprint(&changed), expected, "dimension {dimension}");
        }

        let mut reordered = original.clone();
        reordered.observation_sha256.swap(0, 1);
        assert_ne!(fingerprint(&reordered), expected);
        let mut moved_boundary = original.clone();
        moved_boundary.observation_sha256.pop();
        moved_boundary.receipt_sha256.insert(0, digest('3'));
        assert_ne!(fingerprint(&moved_boundary), expected);
    }

    #[test]
    fn incomplete_malformed_or_oversized_state_fails_closed() {
        let mut malformed = components();
        malformed.plan_sha256 = digest('A');
        assert_eq!(
            WorkflowStateFingerprint::new(&malformed),
            Err(WorkflowProgressError::InvalidState)
        );
        let mut oversized = components();
        oversized.observation_sha256 = vec![digest('0'); MAX_ORDERED_REFERENCES + 1];
        assert_eq!(
            WorkflowStateFingerprint::new(&oversized),
            Err(WorkflowProgressError::InvalidState)
        );
    }

    #[test]
    fn non_adjacent_repeated_state_stops_at_the_exact_policy_limit() {
        let policy = RepeatedStatePolicy::new("repeat-policy-1".to_owned(), 2).expect("policy");
        let mut detector = RepeatedStateDetector::new(&policy);
        let state_a = fingerprint(&components());
        let mut alternate = components();
        alternate.step_sha256 = digest('c');
        let state_b = fingerprint(&alternate);

        assert!(matches!(
            detector.observe(&policy, &state_a),
            Ok(WorkflowProgressDecision::Advanced {
                distinct_states: 1,
                ..
            })
        ));
        assert!(matches!(
            detector.observe(&policy, &state_b),
            Ok(WorkflowProgressDecision::Advanced {
                distinct_states: 2,
                ..
            })
        ));
        assert_eq!(
            detector.observe(&policy, &state_a),
            Ok(WorkflowProgressDecision::Repeated {
                fingerprint_sha256: state_a.sha256().to_owned(),
                repeat_count: 1,
                repeats_remaining: 1,
            })
        );
        let stopped = WorkflowProgressDecision::StopRepeatedState {
            fingerprint_sha256: state_a.sha256().to_owned(),
            repeat_count: 2,
        };
        assert_eq!(detector.observe(&policy, &state_a), Ok(stopped.clone()));
        assert!(detector.is_stopped());
        assert_eq!(detector.observe(&policy, &state_b), Ok(stopped));
        assert_eq!(detector.occurrences(&state_a), 3);
        assert_eq!(detector.occurrences(&state_b), 1);
    }

    #[test]
    fn policy_identity_is_immutable_and_substitution_changes_nothing() {
        assert_eq!(
            RepeatedStatePolicy::new("repeat-policy-1".to_owned(), 0),
            Err(WorkflowProgressError::InvalidPolicy)
        );
        let policy = RepeatedStatePolicy::new("repeat-policy-1".to_owned(), 2).expect("policy");
        let changed =
            RepeatedStatePolicy::new("repeat-policy-1".to_owned(), 3).expect("changed policy");
        assert_ne!(policy.policy_sha256(), changed.policy_sha256());
        let mut detector = RepeatedStateDetector::new(&policy);
        let state = fingerprint(&components());
        assert_eq!(
            detector.observe(&changed, &state),
            Err(WorkflowProgressError::PolicyMismatch)
        );
        assert_eq!(detector.occurrences(&state), 0);
        assert!(!detector.is_stopped());
    }
}
