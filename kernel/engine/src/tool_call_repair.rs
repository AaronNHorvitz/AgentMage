//! Pure, bounded preparation of malformed tool calls before any effect attempt.

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::workflow_budget::{
    WorkflowBudgetError, WorkflowBudgetEvent, WorkflowBudgetLedger, WorkflowBudgetPolicy,
};

/// Maximum ordered fragments admitted for one proposed tool call.
pub const MAX_TOOL_CALL_FRAGMENTS: usize = 1_024;
/// Maximum assembled bytes admitted for one proposed tool call.
pub const MAX_TOOL_CALL_BYTES: usize = 1_048_576;

/// One byte-exact fragment with its zero-based position in a complete model stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedToolCallFragment {
    /// Required zero-based position.
    pub ordinal: u32,
    /// Exact bytes received at this position.
    pub bytes: Vec<u8>,
}

/// Closed, ordered normalization rules that cannot invent argument values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeterministicNormalizationRule {
    /// Concatenate a complete contiguous sequence without changing fragment bytes.
    ReassembleOrderedFragments,
    /// Remove one UTF-8 byte-order mark at the beginning of the assembled stream.
    StripLeadingUtf8Bom,
    /// Remove only JSON whitespace outside the first and last payload bytes.
    TrimOuterJsonWhitespace,
}

/// Byte-exact normalization result required before schema validation or model repair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedToolCall {
    bytes: Vec<u8>,
    sha256: String,
    applied_rules: Vec<DeterministicNormalizationRule>,
}

impl NormalizedToolCall {
    /// Returns the normalized bytes that must be validated against the exact tool schema.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the digest binding validation and any later repair to these bytes.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Returns the exact ordered rule trace.
    #[must_use]
    pub fn applied_rules(&self) -> &[DeterministicNormalizationRule] {
        &self.applied_rules
    }
}

/// Immutable policy for zero or one targeted repair by one exact model profile and tool schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRepairPolicy {
    model_profile_sha256: String,
    tool_schema_sha256: String,
    repair_enabled: bool,
}

impl ModelRepairPolicy {
    /// Creates a policy bound to exact content digests.
    pub fn new(
        model_profile_sha256: String,
        tool_schema_sha256: String,
        repair_enabled: bool,
    ) -> Result<Self, ToolCallRepairError> {
        if !valid_sha256(&model_profile_sha256) || !valid_sha256(&tool_schema_sha256) {
            return Err(ToolCallRepairError::InvalidPolicyBinding);
        }
        Ok(Self {
            model_profile_sha256,
            tool_schema_sha256,
            repair_enabled,
        })
    }

    /// Returns the only model profile permitted to perform the repair.
    #[must_use]
    pub fn model_profile_sha256(&self) -> &str {
        &self.model_profile_sha256
    }

    /// Returns the exact schema against which the repair must later validate.
    #[must_use]
    pub fn tool_schema_sha256(&self) -> &str {
        &self.tool_schema_sha256
    }

    /// Returns whether this policy permits its one possible repair.
    #[must_use]
    pub const fn repair_enabled(&self) -> bool {
        self.repair_enabled
    }

    /// Returns the structural repair limit: zero when disabled and one when enabled.
    #[must_use]
    pub const fn maximum_repairs(&self) -> u8 {
        if self.repair_enabled { 1 } else { 0 }
    }
}

/// Non-executing admission for one targeted model repair.
///
/// This record carries no grant, approval, attempt, executor, or tool-effect capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetedModelRepairAdmission {
    model_profile_sha256: String,
    tool_schema_sha256: String,
    normalized_call_sha256: String,
    repair_ordinal: u8,
}

impl TargetedModelRepairAdmission {
    /// Returns the exact admitted model-profile digest.
    #[must_use]
    pub fn model_profile_sha256(&self) -> &str {
        &self.model_profile_sha256
    }

    /// Returns the exact admitted tool-schema digest.
    #[must_use]
    pub fn tool_schema_sha256(&self) -> &str {
        &self.tool_schema_sha256
    }

    /// Returns the normalized malformed-call digest supplied to repair.
    #[must_use]
    pub fn normalized_call_sha256(&self) -> &str {
        &self.normalized_call_sha256
    }

    /// Returns the only possible repair ordinal.
    #[must_use]
    pub const fn repair_ordinal(&self) -> u8 {
        self.repair_ordinal
    }
}

/// Closed failure reasons for normalization and repair admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCallRepairError {
    /// No fragment was supplied, so completeness cannot be established.
    EmptyFragments,
    /// The bounded fragment count was exceeded.
    TooManyFragments,
    /// Fragment positions were duplicated, skipped, or supplied out of order.
    NonContiguousFragments,
    /// Checked byte accounting exceeded the call limit.
    PayloadTooLarge,
    /// Deterministic normalization left no payload to validate.
    EmptyPayload,
    /// Policy profile or schema identity was not a canonical SHA-256 digest.
    InvalidPolicyBinding,
    /// Policy does not permit model repair.
    RepairDisabled,
    /// Exact schema validation has not rejected the normalized payload.
    ExactSchemaNotRejected,
    /// The selected model profile differs from the immutable policy binding.
    ModelProfileMismatch,
    /// The selected tool schema differs from the immutable policy binding.
    ToolSchemaMismatch,
    /// The one permitted repair was already used.
    RepairAlreadyUsed,
    /// An effect attempt already exists, so malformed-call repair cannot be admitted.
    EffectAttemptAlreadyExists,
}

/// Closed failure from a repair operation that must also consume its independent workflow budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetedToolCallRepairError {
    /// The underlying deterministic normalization or model-repair policy denied the operation.
    Repair(ToolCallRepairError),
    /// The exact independent parser- or model-repair budget denied the operation.
    Budget(WorkflowBudgetError),
}

/// Reassembles and normalizes a complete ordered call using only the closed lossless rules.
pub fn normalize_tool_call(
    fragments: &[OrderedToolCallFragment],
) -> Result<NormalizedToolCall, ToolCallRepairError> {
    if fragments.is_empty() {
        return Err(ToolCallRepairError::EmptyFragments);
    }
    if fragments.len() > MAX_TOOL_CALL_FRAGMENTS {
        return Err(ToolCallRepairError::TooManyFragments);
    }

    let mut assembled = Vec::new();
    for (expected, fragment) in fragments.iter().enumerate() {
        if usize::try_from(fragment.ordinal).ok() != Some(expected) {
            return Err(ToolCallRepairError::NonContiguousFragments);
        }
        let length = assembled
            .len()
            .checked_add(fragment.bytes.len())
            .ok_or(ToolCallRepairError::PayloadTooLarge)?;
        if length > MAX_TOOL_CALL_BYTES {
            return Err(ToolCallRepairError::PayloadTooLarge);
        }
        assembled.extend_from_slice(&fragment.bytes);
    }

    let mut rules = vec![DeterministicNormalizationRule::ReassembleOrderedFragments];
    if assembled.starts_with(&[0xef, 0xbb, 0xbf]) {
        assembled.drain(..3);
        rules.push(DeterministicNormalizationRule::StripLeadingUtf8Bom);
    }

    let start = assembled
        .iter()
        .position(|byte| !is_json_whitespace(*byte))
        .ok_or(ToolCallRepairError::EmptyPayload)?;
    let end = assembled
        .iter()
        .rposition(|byte| !is_json_whitespace(*byte))
        .ok_or(ToolCallRepairError::EmptyPayload)?
        + 1;
    if start != 0 || end != assembled.len() {
        assembled = assembled[start..end].to_vec();
        rules.push(DeterministicNormalizationRule::TrimOuterJsonWhitespace);
    }

    Ok(NormalizedToolCall {
        sha256: sha256(&assembled),
        bytes: assembled,
        applied_rules: rules,
    })
}

/// Performs deterministic parser repair only after reserving one independent parser-repair unit.
///
/// Normalization is evaluated first because it is pure. Invalid input consumes no budget; a valid
/// normalization is returned only if the parser-repair and total-work charges both commit.
pub fn normalize_tool_call_with_budget(
    fragments: &[OrderedToolCallFragment],
    budget_policy: &WorkflowBudgetPolicy,
    budget_ledger: &mut WorkflowBudgetLedger,
) -> Result<NormalizedToolCall, BudgetedToolCallRepairError> {
    let normalized = normalize_tool_call(fragments).map_err(BudgetedToolCallRepairError::Repair)?;
    budget_ledger
        .consume(budget_policy, WorkflowBudgetEvent::ParserRepair, 1)
        .map_err(BudgetedToolCallRepairError::Budget)?;
    Ok(normalized)
}

/// Admits the one profile-bound repair only after deterministic normalization and schema failure.
///
/// The caller must prove there has been no effect attempt. A successful result is merely a
/// content-bound repair admission and cannot dispatch a tool or mint authority.
#[allow(clippy::too_many_arguments)]
pub fn admit_targeted_model_repair(
    policy: &ModelRepairPolicy,
    normalized: &NormalizedToolCall,
    selected_model_profile_sha256: &str,
    selected_tool_schema_sha256: &str,
    exact_schema_rejected: bool,
    previous_repair_count: u8,
    effect_attempt_count: u32,
) -> Result<TargetedModelRepairAdmission, ToolCallRepairError> {
    if !policy.repair_enabled {
        return Err(ToolCallRepairError::RepairDisabled);
    }
    if !exact_schema_rejected {
        return Err(ToolCallRepairError::ExactSchemaNotRejected);
    }
    if selected_model_profile_sha256 != policy.model_profile_sha256 {
        return Err(ToolCallRepairError::ModelProfileMismatch);
    }
    if selected_tool_schema_sha256 != policy.tool_schema_sha256 {
        return Err(ToolCallRepairError::ToolSchemaMismatch);
    }
    if previous_repair_count != 0 {
        return Err(ToolCallRepairError::RepairAlreadyUsed);
    }
    if effect_attempt_count != 0 {
        return Err(ToolCallRepairError::EffectAttemptAlreadyExists);
    }

    Ok(TargetedModelRepairAdmission {
        model_profile_sha256: policy.model_profile_sha256.clone(),
        tool_schema_sha256: policy.tool_schema_sha256.clone(),
        normalized_call_sha256: normalized.sha256.clone(),
        repair_ordinal: 1,
    })
}

/// Admits one targeted model repair only after its independent budget charge commits.
///
/// All repair-policy checks run first and are pure. A denied repair consumes no budget, while an
/// exhausted model-repair or total-work budget returns no admission.
#[allow(clippy::too_many_arguments)]
pub fn admit_targeted_model_repair_with_budget(
    policy: &ModelRepairPolicy,
    normalized: &NormalizedToolCall,
    selected_model_profile_sha256: &str,
    selected_tool_schema_sha256: &str,
    exact_schema_rejected: bool,
    previous_repair_count: u8,
    effect_attempt_count: u32,
    budget_policy: &WorkflowBudgetPolicy,
    budget_ledger: &mut WorkflowBudgetLedger,
) -> Result<TargetedModelRepairAdmission, BudgetedToolCallRepairError> {
    let admission = admit_targeted_model_repair(
        policy,
        normalized,
        selected_model_profile_sha256,
        selected_tool_schema_sha256,
        exact_schema_rejected,
        previous_repair_count,
        effect_attempt_count,
    )
    .map_err(BudgetedToolCallRepairError::Repair)?;
    budget_ledger
        .consume(budget_policy, WorkflowBudgetEvent::ModelRepair, 1)
        .map_err(BudgetedToolCallRepairError::Budget)?;
    Ok(admission)
}

const fn is_json_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SCHEMA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn fragment(ordinal: u32, bytes: &[u8]) -> OrderedToolCallFragment {
        OrderedToolCallFragment {
            ordinal,
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn normalization_is_ordered_bounded_and_does_not_invent_values() {
        let normalized = normalize_tool_call(&[
            fragment(0, b"\xef\xbb\xbf \n{\"de"),
            fragment(1, b"bt\":42}\t"),
        ])
        .expect("closed normalization must succeed");
        assert_eq!(normalized.bytes(), br#"{"debt":42}"#);
        assert_eq!(
            normalized.applied_rules(),
            &[
                DeterministicNormalizationRule::ReassembleOrderedFragments,
                DeterministicNormalizationRule::StripLeadingUtf8Bom,
                DeterministicNormalizationRule::TrimOuterJsonWhitespace,
            ],
        );
        assert_eq!(normalized.sha256(), sha256(br#"{"debt":42}"#));

        assert_eq!(
            normalize_tool_call(&[]),
            Err(ToolCallRepairError::EmptyFragments)
        );
        assert_eq!(
            normalize_tool_call(&[fragment(1, b"{}")]),
            Err(ToolCallRepairError::NonContiguousFragments)
        );
        assert_eq!(
            normalize_tool_call(&[fragment(0, b" \n\t")]),
            Err(ToolCallRepairError::EmptyPayload)
        );
        assert_eq!(
            normalize_tool_call(&[fragment(0, &vec![b'x'; MAX_TOOL_CALL_BYTES + 1])]),
            Err(ToolCallRepairError::PayloadTooLarge)
        );
    }

    #[test]
    fn model_repair_is_single_profile_bound_and_never_an_effect_attempt() {
        let normalized = normalize_tool_call(&[fragment(0, br#"{"debt":}"#)])
            .expect("malformed payload still normalizes deterministically");
        let policy = ModelRepairPolicy::new(PROFILE.to_owned(), SCHEMA.to_owned(), true)
            .expect("exact bindings must produce policy");
        assert_eq!(policy.maximum_repairs(), 1);

        let admission =
            admit_targeted_model_repair(&policy, &normalized, PROFILE, SCHEMA, true, 0, 0)
                .expect("first profile-bound repair must be admitted");
        assert_eq!(admission.repair_ordinal(), 1);
        assert_eq!(admission.model_profile_sha256(), PROFILE);
        assert_eq!(admission.tool_schema_sha256(), SCHEMA);
        assert_eq!(admission.normalized_call_sha256(), normalized.sha256());

        for (profile, schema, rejected, repairs, attempts, expected) in [
            (
                PROFILE,
                SCHEMA,
                false,
                0,
                0,
                ToolCallRepairError::ExactSchemaNotRejected,
            ),
            (
                SCHEMA,
                SCHEMA,
                true,
                0,
                0,
                ToolCallRepairError::ModelProfileMismatch,
            ),
            (
                PROFILE,
                PROFILE,
                true,
                0,
                0,
                ToolCallRepairError::ToolSchemaMismatch,
            ),
            (
                PROFILE,
                SCHEMA,
                true,
                1,
                0,
                ToolCallRepairError::RepairAlreadyUsed,
            ),
            (
                PROFILE,
                SCHEMA,
                true,
                0,
                1,
                ToolCallRepairError::EffectAttemptAlreadyExists,
            ),
        ] {
            assert_eq!(
                admit_targeted_model_repair(
                    &policy,
                    &normalized,
                    profile,
                    schema,
                    rejected,
                    repairs,
                    attempts,
                ),
                Err(expected),
            );
        }

        let disabled = ModelRepairPolicy::new(PROFILE.to_owned(), SCHEMA.to_owned(), false)
            .expect("disabled policy is valid");
        assert_eq!(disabled.maximum_repairs(), 0);
        assert_eq!(
            admit_targeted_model_repair(&disabled, &normalized, PROFILE, SCHEMA, true, 0, 0),
            Err(ToolCallRepairError::RepairDisabled),
        );
    }
}
