//! Deterministic measured local routing without model confidence or authority transfer.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_PROFILES: usize = 64;
const MIN_ROUTING_TRIALS: u32 = 30;
const MIN_ROUTING_IMPROVEMENT_BPS: i32 = 250;
const MIN_VERIFICATION_IMPROVEMENT_BPS: i32 = 150;

/// Independent role for which one exact profile may carry measured evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingRole {
    /// Conversational response and explanation.
    Dialogue,
    /// Selection of one typed tool candidate.
    ToolSelection,
    /// Bounded synthesis of already-grounded evidence.
    Summarization,
    /// Interpretation of a deterministic repository map.
    RepositoryMap,
    /// Retrieval embedding generation.
    Embedding,
    /// Retrieval reranking.
    Reranking,
    /// Structured patch proposal generation.
    PatchGeneration,
    /// Citation-to-source verification.
    CitationVerification,
    /// Bounded plan proposal.
    Planning,
    /// Repository-aware coding proposal.
    Coding,
    /// Grounded retrieval synthesis.
    Retrieval,
    /// Bounded document interpretation.
    Document,
}

/// Deterministically assigned task class; no model or learned classifier supplies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingTaskClass {
    /// Dialogue task.
    Dialogue,
    /// Tool-selection task.
    ToolSelection,
    /// Summarization task.
    Summarization,
    /// Repository-map task.
    RepositoryMap,
    /// Embedding task.
    Embedding,
    /// Reranking task.
    Reranking,
    /// Patch-generation task.
    PatchGeneration,
    /// Citation-verification task.
    CitationVerification,
    /// Planning task.
    Planning,
    /// Coding task.
    Coding,
    /// Retrieval task.
    Retrieval,
    /// Document task.
    Document,
}

impl RoutingTaskClass {
    const fn role(self) -> RoutingRole {
        match self {
            Self::Dialogue => RoutingRole::Dialogue,
            Self::ToolSelection => RoutingRole::ToolSelection,
            Self::Summarization => RoutingRole::Summarization,
            Self::RepositoryMap => RoutingRole::RepositoryMap,
            Self::Embedding => RoutingRole::Embedding,
            Self::Reranking => RoutingRole::Reranking,
            Self::PatchGeneration => RoutingRole::PatchGeneration,
            Self::CitationVerification => RoutingRole::CitationVerification,
            Self::Planning => RoutingRole::Planning,
            Self::Coding => RoutingRole::Coding,
            Self::Retrieval => RoutingRole::Retrieval,
            Self::Document => RoutingRole::Document,
        }
    }
}

/// Separately computed action risk that never derives from model confidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingActionRisk {
    /// No consequential effect is proposed.
    Low,
    /// Reversible local consequence requires ordinary review.
    Moderate,
    /// Material consequence requires enhanced review.
    High,
    /// Irreversible or externally consequential work requires independent verification.
    Critical,
}

/// Visible user-selected inference budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingBudget {
    /// Lowest bounded latency and context profile.
    Fast,
    /// Default balanced profile.
    Standard,
    /// Larger local context and review profile.
    Deep,
    /// Deep profile with an eligible measured second-model verifier when useful.
    Verify,
}

/// Review boundary carried by every routing decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingReviewBoundary {
    /// Ordinary deterministic verification remains required.
    Standard,
    /// Enhanced deterministic and human review remains required.
    Enhanced,
    /// Independent verification is required; a measured second model may be advisory.
    Independent,
}

/// Exact model, context, tool, and review ceilings for one visible budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RoutingBudgetBoundary {
    /// Visible budget name.
    pub budget: RoutingBudget,
    /// Maximum admitted context tokens.
    pub max_context_tokens: u32,
    /// Maximum typed tool proposals.
    pub max_tool_proposals: u16,
    /// Required review boundary.
    pub review: RoutingReviewBoundary,
    /// Models remain local-only for every budget.
    pub local_only: bool,
}

impl RoutingBudgetBoundary {
    /// Returns the fixed boundary for one visible budget.
    #[must_use]
    pub const fn for_budget(budget: RoutingBudget) -> Self {
        match budget {
            RoutingBudget::Fast => Self {
                budget,
                max_context_tokens: 8_192,
                max_tool_proposals: 2,
                review: RoutingReviewBoundary::Standard,
                local_only: true,
            },
            RoutingBudget::Standard => Self {
                budget,
                max_context_tokens: 16_384,
                max_tool_proposals: 4,
                review: RoutingReviewBoundary::Standard,
                local_only: true,
            },
            RoutingBudget::Deep => Self {
                budget,
                max_context_tokens: 32_768,
                max_tool_proposals: 8,
                review: RoutingReviewBoundary::Enhanced,
                local_only: true,
            },
            RoutingBudget::Verify => Self {
                budget,
                max_context_tokens: 32_768,
                max_tool_proposals: 8,
                review: RoutingReviewBoundary::Independent,
                local_only: true,
            },
        }
    }
}

/// Platform carrying independently admitted evidence for one exact profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingPlatform {
    /// Fedora Linux x86-64.
    FedoraX86_64,
    /// Ubuntu Linux x86-64.
    UbuntuX86_64,
    /// Windows 11 x86-64.
    Windows11X86_64,
    /// Apple Silicon macOS retained post-GA.
    MacosArm64,
}

/// Origin-policy disposition computed before routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingOriginDisposition {
    /// Publisher, lineage, and origin satisfy the current policy.
    Allowed,
    /// Chinese origin is prohibited by current policy.
    ProhibitedChinese,
    /// Chinese-derived lineage is prohibited by current policy.
    ProhibitedChineseDerived,
    /// Origin or lineage is unresolved.
    Unresolved,
}

/// Exact lifecycle disposition admitted for measured routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingProfileState {
    /// Every required gate passed for the exact declared boundary.
    Approved,
    /// A limited profile is admitted only within its recorded evidence.
    Degraded,
    /// Candidate is not eligible for product routing.
    Candidate,
    /// Profile is quarantined.
    Quarantined,
    /// Profile failed a mandatory gate.
    Rejected,
    /// Profile is retained but unavailable for new work.
    Retired,
}

/// Exact measured evidence for one role and immutable profile tuple.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleBenchmarkEvidence {
    /// Exact benchmark-corpus digest.
    pub corpus_sha256: String,
    /// Exact deterministic-grader digest.
    pub grader_sha256: String,
    /// Exact benchmark implementation digest.
    pub benchmark_sha256: String,
    /// Repeated-trial count.
    pub trial_count: u32,
    /// Declared-role successes.
    pub quality_pass_count: u32,
    /// Best published manual-selection baseline successes over the same trials.
    pub baseline_pass_count: u32,
    /// Grounding successes.
    pub grounding_pass_count: u32,
    /// Reliability successes.
    pub reliability_pass_count: u32,
    /// Observed improvement over the manual baseline in basis points.
    pub improvement_bps: i32,
    /// Measured improvement when used as a second verifier, in basis points.
    pub verification_improvement_bps: i32,
    /// Repeated-trial evidence met the declared statistical rule.
    pub statistically_supported: bool,
    /// Evidence has not expired or drifted.
    pub current: bool,
    /// P95 local latency for this exact tuple.
    pub latency_p95_ms: u64,
    /// Peak resident and accelerator memory for this exact tuple.
    pub peak_memory_bytes: u64,
    /// Measured energy when available; absence remains explicit.
    pub energy_millijoules: Option<u64>,
}

impl RoleBenchmarkEvidence {
    fn valid(&self) -> bool {
        self.trial_count >= MIN_ROUTING_TRIALS
            && self.quality_pass_count <= self.trial_count
            && self.baseline_pass_count <= self.trial_count
            && self.grounding_pass_count <= self.trial_count
            && self.reliability_pass_count <= self.trial_count
            && self.improvement_bps == improvement_bps(self)
            && self.improvement_bps >= MIN_ROUTING_IMPROVEMENT_BPS
            && self.verification_improvement_bps >= 0
            && self.statistically_supported
            && self.current
            && self.latency_p95_ms > 0
            && self.peak_memory_bytes > 0
            && valid_sha256(&self.corpus_sha256)
            && valid_sha256(&self.grader_sha256)
            && valid_sha256(&self.benchmark_sha256)
    }
}

/// Exact attributable profile and routing evidence; no family-wide result is accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoutableProfile {
    /// Exact profile identity.
    pub profile_id: String,
    /// First-party publisher identity.
    pub publisher_id: String,
    /// Exact lineage digest.
    pub lineage_sha256: String,
    /// Exact reviewed license identity.
    pub license_id: String,
    /// Exact license text digest.
    pub license_sha256: String,
    /// Origin-policy result.
    pub origin: RoutingOriginDisposition,
    /// Exact model artifact digest.
    pub artifact_sha256: String,
    /// Exact tokenizer digest.
    pub tokenizer_sha256: String,
    /// Exact template digest.
    pub template_sha256: String,
    /// Exact family-codec digest.
    pub codec_sha256: String,
    /// Exact local runtime digest.
    pub runtime_sha256: String,
    /// Exact context-profile digest.
    pub context_sha256: String,
    /// Exact decoding-profile digest.
    pub decoding_sha256: String,
    /// Exact resource-envelope digest.
    pub resource_sha256: String,
    /// Exact platform evidence digest.
    pub platform_evidence_sha256: String,
    /// Exact policy under which this profile and role evidence were admitted.
    pub policy_sha256: String,
    /// Exact benchmark generation that produced the role evidence.
    pub benchmark_generation_sha256: String,
    /// Current lifecycle state.
    pub state: RoutingProfileState,
    /// Profile is currently enabled for only its measured roles.
    pub enabled: bool,
    /// Profile cannot contact or route to a remote service.
    pub local_only: bool,
    /// Invisible fallback remains prohibited.
    pub automatic_fallback: bool,
    /// Independently admitted platforms.
    pub platforms: BTreeSet<RoutingPlatform>,
    /// Maximum context admitted for this exact profile.
    pub max_context_tokens: u32,
    /// Maximum resident plus accelerator memory admitted for this exact profile.
    pub required_memory_bytes: u64,
    /// Independently measured role evidence.
    pub roles: BTreeMap<RoutingRole, RoleBenchmarkEvidence>,
}

impl RoutableProfile {
    fn exact_identity_valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && valid_identifier(&self.publisher_id)
            && valid_identifier(&self.license_id)
            && [
                &self.lineage_sha256,
                &self.license_sha256,
                &self.artifact_sha256,
                &self.tokenizer_sha256,
                &self.template_sha256,
                &self.codec_sha256,
                &self.runtime_sha256,
                &self.context_sha256,
                &self.decoding_sha256,
                &self.resource_sha256,
                &self.platform_evidence_sha256,
                &self.policy_sha256,
                &self.benchmark_generation_sha256,
            ]
            .into_iter()
            .all(|value| valid_sha256(value))
            && self.origin == RoutingOriginDisposition::Allowed
            && self.state == RoutingProfileState::Approved
            && self.enabled
            && self.local_only
            && !self.automatic_fallback
            && !self.platforms.is_empty()
            && self.max_context_tokens > 0
            && self.required_memory_bytes > 0
    }
}

/// Resource state observed independently from the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RoutingResourceState {
    /// Available resident plus accelerator memory.
    pub available_memory_bytes: u64,
    /// Maximum context currently available.
    pub available_context_tokens: u32,
    /// Runtime and resource observation is current.
    pub current: bool,
}

/// Exact deterministic routing request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MeasuredRoutingRequest {
    /// Stable task identity.
    pub task_id: String,
    /// Deterministically assigned task class.
    pub task_class: RoutingTaskClass,
    /// Separately computed action risk.
    pub action_risk: RoutingActionRisk,
    /// Visible user budget.
    pub budget: RoutingBudget,
    /// Exact execution platform.
    pub platform: RoutingPlatform,
    /// Required bounded context.
    pub required_context_tokens: u32,
    /// Required typed tool-proposal allowance.
    pub required_tool_proposals: u16,
    /// Independently observed resource fit.
    pub resources: RoutingResourceState,
    /// Optional exact user-selected profile.
    pub manual_profile_id: Option<String>,
    /// Exact current policy digest.
    pub policy_sha256: String,
    /// Exact current benchmark-generation digest.
    pub benchmark_generation_sha256: String,
}

/// Terminal router disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingDisposition {
    /// Measured automatic routing selected one exact local profile.
    Selected,
    /// The user's exact eligible selection was preserved.
    ManualSelected,
    /// No exact eligible selection exists.
    Blocked,
}

/// Visible disposition for one considered exact profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoutingCandidateAudit {
    /// Exact considered profile.
    pub profile_id: String,
    /// Whether the profile is eligible for this exact request.
    pub eligible: bool,
    /// Stable ordered rationale codes.
    pub rationale_codes: Vec<String>,
    /// Exact role benchmark digest when eligible.
    pub role_benchmark_sha256: Option<String>,
}

/// Content-minimized, reproducible measured-routing decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MeasuredRoutingReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: String,
    /// Deterministic task class.
    pub task_class: RoutingTaskClass,
    /// Exact required role.
    pub required_role: RoutingRole,
    /// Separately computed action risk.
    pub action_risk: RoutingActionRisk,
    /// Exact visible budget boundary.
    pub budget_boundary: RoutingBudgetBoundary,
    /// Terminal routing disposition.
    pub disposition: RoutingDisposition,
    /// Selected exact primary profile, when eligible.
    pub selected_profile_id: Option<String>,
    /// Optional exact measured second-model verifier.
    pub verification_profile_id: Option<String>,
    /// Every equally scored eligible profile that disagreed with the deterministic tie break.
    pub disagreement_profile_ids: Vec<String>,
    /// Stable terminal rationale.
    pub result_code: String,
    /// Exact current policy digest.
    pub policy_sha256: String,
    /// Exact current benchmark-generation digest.
    pub benchmark_generation_sha256: String,
    /// Sorted audit result for every considered profile.
    pub candidates: Vec<RoutingCandidateAudit>,
    /// Cloud or frontier transfer is structurally unavailable.
    pub frontier_transfer: bool,
    /// Models supplied no authority-bearing routing input.
    pub model_confidence_used: bool,
    /// Digest of every preceding receipt field.
    pub receipt_sha256: String,
}

/// Computes one deterministic local routing decision over exact typed evidence.
pub fn route_measured_local(
    request: MeasuredRoutingRequest,
    profiles: Vec<RoutableProfile>,
) -> Result<MeasuredRoutingReceipt, &'static str> {
    if !request_valid(&request) || profiles.len() > MAX_PROFILES {
        return Err("model.routing.input-invalid");
    }
    let boundary = RoutingBudgetBoundary::for_budget(request.budget);
    let role = request.task_class.role();
    let mut profiles = profiles;
    profiles.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    if profiles
        .windows(2)
        .any(|pair| pair[0].profile_id == pair[1].profile_id)
    {
        return Err("model.routing.profile-duplicate");
    }

    let mut eligible = Vec::new();
    let mut candidates = Vec::with_capacity(profiles.len());
    for profile in &profiles {
        let (audit, score) = audit_profile(profile, &request, role, boundary);
        if let Some(score) = score {
            eligible.push((profile, score));
        }
        candidates.push(audit);
    }

    let (disposition, selected, disagreements, result_code) =
        if let Some(manual_profile_id) = request.manual_profile_id.as_deref() {
            match eligible
                .iter()
                .find(|(profile, _)| profile.profile_id == manual_profile_id)
            {
                Some((profile, _)) => (
                    RoutingDisposition::ManualSelected,
                    Some(*profile),
                    Vec::new(),
                    "model.routing.manual-selected",
                ),
                None => (
                    RoutingDisposition::Blocked,
                    None,
                    Vec::new(),
                    "model.routing.manual-selection-ineligible",
                ),
            }
        } else if eligible.is_empty() {
            (
                RoutingDisposition::Blocked,
                None,
                Vec::new(),
                "model.routing.no-eligible-profile",
            )
        } else {
            eligible.sort_by(|(left_profile, left_score), (right_profile, right_score)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left_profile.profile_id.cmp(&right_profile.profile_id))
            });
            let best_score = eligible[0].1;
            let selected = eligible[0].0;
            let disagreements = eligible
                .iter()
                .skip(1)
                .take_while(|(_, score)| *score == best_score)
                .map(|(profile, _)| profile.profile_id.clone())
                .collect();
            (
                RoutingDisposition::Selected,
                Some(selected),
                disagreements,
                "model.routing.measured-selected",
            )
        };

    let verifier =
        selected.and_then(|primary| select_verifier(&request, role, boundary, primary, &eligible));
    let mut receipt = MeasuredRoutingReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id: request.task_id,
        task_class: request.task_class,
        required_role: role,
        action_risk: request.action_risk,
        budget_boundary: boundary,
        disposition,
        selected_profile_id: selected.map(|profile| profile.profile_id.clone()),
        verification_profile_id: verifier.map(|profile| profile.profile_id.clone()),
        disagreement_profile_ids: disagreements,
        result_code: result_code.to_owned(),
        policy_sha256: request.policy_sha256,
        benchmark_generation_sha256: request.benchmark_generation_sha256,
        candidates,
        frontier_transfer: false,
        model_confidence_used: false,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_sha256(&receipt);
    Ok(receipt)
}

fn audit_profile(
    profile: &RoutableProfile,
    request: &MeasuredRoutingRequest,
    role: RoutingRole,
    boundary: RoutingBudgetBoundary,
) -> (RoutingCandidateAudit, Option<i32>) {
    let mut codes = Vec::new();
    if !profile.exact_identity_valid() {
        codes.push("profile-boundary-failed".to_owned());
    }
    if !profile.platforms.contains(&request.platform) {
        codes.push("platform-unmeasured".to_owned());
    }
    if profile.policy_sha256 != request.policy_sha256 {
        codes.push("profile-policy-stale".to_owned());
    }
    if profile.benchmark_generation_sha256 != request.benchmark_generation_sha256 {
        codes.push("benchmark-generation-stale".to_owned());
    }
    if !request.resources.current {
        codes.push("resource-observation-stale".to_owned());
    }
    if profile.required_memory_bytes > request.resources.available_memory_bytes {
        codes.push("memory-unavailable".to_owned());
    }
    if request.required_context_tokens > boundary.max_context_tokens
        || request.required_context_tokens > profile.max_context_tokens
        || request.required_context_tokens > request.resources.available_context_tokens
    {
        codes.push("context-unavailable".to_owned());
    }
    if request.required_tool_proposals > boundary.max_tool_proposals {
        codes.push("tool-budget-exceeded".to_owned());
    }
    let evidence = profile.roles.get(&role);
    if evidence.is_none_or(|value| !value.valid()) {
        codes.push("role-evidence-unmeasured-or-stale".to_owned());
    }
    let eligible = codes.is_empty();
    if eligible {
        codes.push("eligible-exact-measured-local-profile".to_owned());
    }
    let role_benchmark_sha256 = evidence.and_then(|value| canonical_sha256(value).ok());
    let score = if eligible {
        evidence.map(|value| value.improvement_bps)
    } else {
        None
    };
    (
        RoutingCandidateAudit {
            profile_id: profile.profile_id.clone(),
            eligible,
            rationale_codes: codes,
            role_benchmark_sha256,
        },
        score,
    )
}

fn select_verifier<'a>(
    request: &MeasuredRoutingRequest,
    role: RoutingRole,
    boundary: RoutingBudgetBoundary,
    primary: &RoutableProfile,
    eligible: &'a [(&'a RoutableProfile, i32)],
) -> Option<&'a RoutableProfile> {
    if boundary.budget != RoutingBudget::Verify
        || boundary.review != RoutingReviewBoundary::Independent
        || request.action_risk < RoutingActionRisk::High
    {
        return None;
    }
    eligible
        .iter()
        .filter(|(profile, _)| profile.profile_id != primary.profile_id)
        .filter_map(|(profile, _)| {
            let evidence = profile.roles.get(&role)?;
            (evidence.verification_improvement_bps >= MIN_VERIFICATION_IMPROVEMENT_BPS)
                .then_some((*profile, evidence.verification_improvement_bps))
        })
        .max_by(|(left_profile, left_score), (right_profile, right_score)| {
            left_score
                .cmp(right_score)
                .then_with(|| right_profile.profile_id.cmp(&left_profile.profile_id))
        })
        .map(|(profile, _)| profile)
}

fn request_valid(request: &MeasuredRoutingRequest) -> bool {
    valid_identifier(&request.task_id)
        && valid_sha256(&request.policy_sha256)
        && valid_sha256(&request.benchmark_generation_sha256)
        && request.required_context_tokens > 0
        && request.resources.available_memory_bytes > 0
        && request.resources.available_context_tokens > 0
        && request
            .manual_profile_id
            .as_deref()
            .is_none_or(valid_identifier)
}

fn improvement_bps(evidence: &RoleBenchmarkEvidence) -> i32 {
    let difference =
        i64::from(evidence.quality_pass_count) - i64::from(evidence.baseline_pass_count);
    let basis_points = difference.saturating_mul(10_000) / i64::from(evidence.trial_count.max(1));
    i32::try_from(basis_points).unwrap_or(if basis_points.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
}

fn receipt_sha256(receipt: &MeasuredRoutingReceipt) -> String {
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    canonical_sha256(&unsigned).expect("measured routing receipt is serializable")
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, &'static str> {
    let bytes = serde_json::to_vec(value).map_err(|_| "model.routing.serialization-failed")?;
    Ok(lower_hex(&Sha256::digest(bytes)))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lower_hex(bytes: &[u8]) -> String {
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

    fn hash(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn evidence(quality: u32, baseline: u32, verification_bps: i32) -> RoleBenchmarkEvidence {
        RoleBenchmarkEvidence {
            corpus_sha256: hash('a'),
            grader_sha256: hash('b'),
            benchmark_sha256: hash('c'),
            trial_count: 100,
            quality_pass_count: quality,
            baseline_pass_count: baseline,
            grounding_pass_count: quality,
            reliability_pass_count: quality,
            improvement_bps: i32::try_from((i64::from(quality) - i64::from(baseline)) * 100)
                .expect("basis points"),
            verification_improvement_bps: verification_bps,
            statistically_supported: true,
            current: true,
            latency_p95_ms: 1_000,
            peak_memory_bytes: 8 * 1024 * 1024 * 1024,
            energy_millijoules: None,
        }
    }

    fn profile(id: &str, quality: u32, verification_bps: i32) -> RoutableProfile {
        RoutableProfile {
            profile_id: id.to_owned(),
            publisher_id: "publisher-first-party".to_owned(),
            lineage_sha256: hash('1'),
            license_id: "apache-2.0".to_owned(),
            license_sha256: hash('2'),
            origin: RoutingOriginDisposition::Allowed,
            artifact_sha256: hash('3'),
            tokenizer_sha256: hash('4'),
            template_sha256: hash('5'),
            codec_sha256: hash('6'),
            runtime_sha256: hash('7'),
            context_sha256: hash('8'),
            decoding_sha256: hash('9'),
            resource_sha256: hash('a'),
            platform_evidence_sha256: hash('b'),
            policy_sha256: hash('c'),
            benchmark_generation_sha256: hash('d'),
            state: RoutingProfileState::Approved,
            enabled: true,
            local_only: true,
            automatic_fallback: false,
            platforms: BTreeSet::from([RoutingPlatform::FedoraX86_64]),
            max_context_tokens: 32_768,
            required_memory_bytes: 16 * 1024 * 1024 * 1024,
            roles: BTreeMap::from([(RoutingRole::Coding, evidence(quality, 60, verification_bps))]),
        }
    }

    fn request() -> MeasuredRoutingRequest {
        MeasuredRoutingRequest {
            task_id: "task-0001".to_owned(),
            task_class: RoutingTaskClass::Coding,
            action_risk: RoutingActionRisk::Moderate,
            budget: RoutingBudget::Standard,
            platform: RoutingPlatform::FedoraX86_64,
            required_context_tokens: 8_192,
            required_tool_proposals: 2,
            resources: RoutingResourceState {
                available_memory_bytes: 64 * 1024 * 1024 * 1024,
                available_context_tokens: 32_768,
                current: true,
            },
            manual_profile_id: None,
            policy_sha256: hash('c'),
            benchmark_generation_sha256: hash('d'),
        }
    }

    #[test]
    fn every_visible_budget_has_fixed_local_context_tool_and_review_boundaries() {
        let boundaries = [
            RoutingBudgetBoundary::for_budget(RoutingBudget::Fast),
            RoutingBudgetBoundary::for_budget(RoutingBudget::Standard),
            RoutingBudgetBoundary::for_budget(RoutingBudget::Deep),
            RoutingBudgetBoundary::for_budget(RoutingBudget::Verify),
        ];
        assert!(boundaries.iter().all(|boundary| boundary.local_only));
        assert_eq!(boundaries[0].max_context_tokens, 8_192);
        assert_eq!(boundaries[1].max_context_tokens, 16_384);
        assert_eq!(boundaries[2].review, RoutingReviewBoundary::Enhanced);
        assert_eq!(boundaries[3].review, RoutingReviewBoundary::Independent);
    }

    #[test]
    fn measured_router_is_stable_at_threshold_and_exposes_ties() {
        let first = profile("profile-alpha", 65, 0);
        let second = profile("profile-beta", 65, 0);
        let receipt = route_measured_local(request(), vec![second, first]).expect("routing");
        assert_eq!(receipt.disposition, RoutingDisposition::Selected);
        assert_eq!(
            receipt.selected_profile_id.as_deref(),
            Some("profile-alpha")
        );
        assert_eq!(receipt.disagreement_profile_ids, ["profile-beta"]);
        assert!(!receipt.frontier_transfer);
        assert!(!receipt.model_confidence_used);
        assert!(valid_sha256(&receipt.receipt_sha256));
    }

    #[test]
    fn manual_choice_is_preserved_and_never_falls_back() {
        let mut exact = request();
        exact.manual_profile_id = Some("profile-beta".to_owned());
        let receipt = route_measured_local(
            exact.clone(),
            vec![
                profile("profile-alpha", 90, 0),
                profile("profile-beta", 65, 0),
            ],
        )
        .expect("manual routing");
        assert_eq!(receipt.disposition, RoutingDisposition::ManualSelected);
        assert_eq!(receipt.selected_profile_id.as_deref(), Some("profile-beta"));

        exact.manual_profile_id = Some("profile-missing".to_owned());
        let refused = route_measured_local(exact, vec![profile("profile-alpha", 90, 0)])
            .expect("manual refusal");
        assert_eq!(refused.disposition, RoutingDisposition::Blocked);
        assert_eq!(refused.selected_profile_id, None);
        assert_eq!(
            refused.result_code,
            "model.routing.manual-selection-ineligible"
        );
    }

    #[test]
    fn unmeasured_stale_prohibited_nonapproved_and_unfit_profiles_fail_closed() {
        let mut cases = Vec::new();
        let mut unmeasured = profile("profile-unmeasured", 62, 0);
        unmeasured
            .roles
            .get_mut(&RoutingRole::Coding)
            .expect("role")
            .trial_count = 1;
        cases.push(unmeasured);
        let mut stale = profile("profile-stale", 70, 0);
        stale
            .roles
            .get_mut(&RoutingRole::Coding)
            .expect("role")
            .current = false;
        cases.push(stale);
        let mut stale_policy = profile("profile-stale-policy", 70, 0);
        stale_policy.policy_sha256 = hash('e');
        cases.push(stale_policy);
        let mut stale_generation = profile("profile-stale-generation", 70, 0);
        stale_generation.benchmark_generation_sha256 = hash('e');
        cases.push(stale_generation);
        let mut prohibited = profile("profile-prohibited", 70, 0);
        prohibited.origin = RoutingOriginDisposition::ProhibitedChineseDerived;
        cases.push(prohibited);
        let mut candidate = profile("profile-candidate", 70, 0);
        candidate.state = RoutingProfileState::Candidate;
        cases.push(candidate);
        let mut fallback = profile("profile-fallback", 70, 0);
        fallback.automatic_fallback = true;
        cases.push(fallback);
        let mut remote = profile("profile-remote", 70, 0);
        remote.local_only = false;
        cases.push(remote);
        let mut unfit = profile("profile-unfit", 70, 0);
        unfit.required_memory_bytes = u64::MAX;
        cases.push(unfit);

        let receipt = route_measured_local(request(), cases).expect("blocked routing");
        assert_eq!(receipt.disposition, RoutingDisposition::Blocked);
        assert_eq!(receipt.selected_profile_id, None);
        assert!(
            receipt
                .candidates
                .iter()
                .all(|candidate| !candidate.eligible)
        );
    }

    #[test]
    fn stale_resources_context_tools_and_platform_fail_closed() {
        for mutation in ["resources", "context", "tools", "platform"] {
            let mut changed = request();
            match mutation {
                "resources" => changed.resources.current = false,
                "context" => changed.required_context_tokens = 32_768,
                "tools" => changed.required_tool_proposals = 5,
                "platform" => changed.platform = RoutingPlatform::Windows11X86_64,
                _ => unreachable!(),
            }
            let receipt = route_measured_local(changed, vec![profile("profile-alpha", 70, 0)])
                .expect("blocked request");
            assert_eq!(
                receipt.disposition,
                RoutingDisposition::Blocked,
                "{mutation}"
            );
        }
    }

    #[test]
    fn second_model_is_used_only_for_measured_high_risk_verify_work() {
        let primary = profile("profile-primary", 90, 0);
        let verifier = profile("profile-verifier", 70, 200);
        for (risk, budget, expected) in [
            (RoutingActionRisk::Moderate, RoutingBudget::Verify, None),
            (RoutingActionRisk::High, RoutingBudget::Deep, None),
            (
                RoutingActionRisk::High,
                RoutingBudget::Verify,
                Some("profile-verifier"),
            ),
        ] {
            let mut changed = request();
            changed.action_risk = risk;
            changed.budget = budget;
            let receipt = route_measured_local(changed, vec![primary.clone(), verifier.clone()])
                .expect("verified routing");
            assert_eq!(receipt.verification_profile_id.as_deref(), expected);
        }
    }

    #[test]
    fn promotion_threshold_degraded_and_empty_states_are_stable() {
        let below = route_measured_local(request(), vec![profile("profile-below", 62, 0)])
            .expect("below-threshold routing");
        assert_eq!(below.disposition, RoutingDisposition::Blocked);

        let mut threshold = profile("profile-threshold", 63, 0);
        let threshold_evidence = threshold
            .roles
            .get_mut(&RoutingRole::Coding)
            .expect("threshold role");
        threshold_evidence.trial_count = 200;
        threshold_evidence.quality_pass_count = 125;
        threshold_evidence.baseline_pass_count = 120;
        threshold_evidence.grounding_pass_count = 125;
        threshold_evidence.reliability_pass_count = 125;
        threshold_evidence.improvement_bps = MIN_ROUTING_IMPROVEMENT_BPS;
        let above = route_measured_local(request(), vec![threshold]).expect("threshold routing");
        assert_eq!(above.disposition, RoutingDisposition::Selected);
        assert_eq!(
            above.selected_profile_id.as_deref(),
            Some("profile-threshold")
        );

        let mut degraded = profile("profile-degraded", 70, 0);
        degraded.state = RoutingProfileState::Degraded;
        let degraded =
            route_measured_local(request(), vec![degraded]).expect("exact degraded routing");
        assert_eq!(degraded.disposition, RoutingDisposition::Blocked);

        let absent = route_measured_local(request(), Vec::new()).expect("empty routing");
        assert_eq!(absent.disposition, RoutingDisposition::Blocked);
        assert_eq!(absent.result_code, "model.routing.no-eligible-profile");
    }

    #[test]
    fn duplicate_profiles_and_invalid_requests_are_rejected() {
        let duplicate = profile("profile-alpha", 70, 0);
        assert_eq!(
            route_measured_local(request(), vec![duplicate.clone(), duplicate]),
            Err("model.routing.profile-duplicate")
        );
        let mut invalid = request();
        invalid.policy_sha256 = "invalid".to_owned();
        assert_eq!(
            route_measured_local(invalid, Vec::new()),
            Err("model.routing.input-invalid")
        );
    }
}
