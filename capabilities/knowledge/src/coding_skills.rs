//! Bounded declarative coding skills with no direct tool or mutation authority.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    DeclarativeAssetKind, DeclarativeSkillAsset, DeclarativeSkillCompatibility,
    DeclarativeSkillError, DeclarativeSkillFile, DeclarativeSkillManifest, DeclarativeSkillPackage,
    DeclarativeSkillScope, DeclarativeSkillTrustState, SkillAuthorityCeiling,
    seal_declarative_skill_manifest,
};

const CODING_CONTEXT_LIMIT: u64 = 128 * 1024;
const MAX_DECLARED_PATHS: u16 = 256;
const REQUIRED_VALIDATION: [&str; 4] = [
    "completion-evidence",
    "no-unrelated-change",
    "scope-check",
    "source-citations",
];
const SUPPORTED_LANGUAGES: [&str; 8] = [
    "go",
    "javascript",
    "mixed",
    "python",
    "shell",
    "sql",
    "typescript",
    "unsupported-lexical-only",
];
const ADVISORY_TOOLS: [&str; 5] = [
    "exact-text-search",
    "read-only-git-history",
    "repository-map",
    "structured-change-preview",
    "trusted-validation-results",
];
const PROHIBITED_OPERATIONS: [&str; 13] = [
    "arbitrary-shell",
    "automatic-commit",
    "automatic-dependency-upgrade",
    "automatic-deploy",
    "automatic-merge",
    "automatic-migration",
    "automatic-pr-publication",
    "automatic-push",
    "automatic-refactor",
    "automatic-release",
    "frontier-handoff",
    "network-publication",
    "unattended-write",
];

/// Closed promoted coding-skill inventory for the v0.4 candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingSkill {
    /// Describe repository structure, entry points, dependencies, and evidence coverage.
    RepositoryCartographer,
    /// Trace one feature across interfaces, services, storage, background work, and tests.
    FeatureTrace,
    /// Identify bounded consequences, validation needs, risk, and rollback for one change.
    ChangeImpact,
    /// Reproduce and diagnose a failure from explicit evidence and competing hypotheses.
    Debugging,
    /// Propose and assess tests and trusted validation evidence.
    TestAndVerification,
    /// Produce source-grounded repository documentation proposals.
    RepositoryDocumentation,
    /// Build a minimal deterministic reproduction without changing the source repository.
    BugReproduction,
    /// Inspect local Git history and change intent without changing repository state.
    GitHistoryAnalysis,
    /// Review an exact bounded change packet and identify evidence-backed findings.
    BoundedReview,
}

impl CodingSkill {
    /// Complete stable promoted coding-skill inventory.
    pub const ALL: [Self; 9] = [
        Self::RepositoryCartographer,
        Self::FeatureTrace,
        Self::ChangeImpact,
        Self::Debugging,
        Self::TestAndVerification,
        Self::RepositoryDocumentation,
        Self::BugReproduction,
        Self::GitHistoryAnalysis,
        Self::BoundedReview,
    ];
}

/// Closed admission result for one exact coding-skill definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingSkillAdmission {
    /// The definition satisfies the bounded declarative contract.
    Admitted,
    /// The definition remains visible but cannot be loaded or promoted.
    Disabled,
}

/// Stable reason an exact coding-skill definition is disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingSkillFinding {
    /// Identity or purpose is absent, malformed, or does not match the selected skill.
    InvalidIdentity,
    /// Completion language is vague rather than tied to reviewable evidence.
    VagueCompletion,
    /// A declared language is outside the closed support and lexical-fallback set.
    UnsupportedLanguage,
    /// A declared advisory tool is outside the closed non-authoritative set.
    UnsupportedTool,
    /// One or more mandatory validation classes are absent.
    MissingValidation,
    /// Declared path scope is absent or exceeds the fixed bound.
    OverbroadScope,
    /// The definition requests authority or omits a mandatory denied operation.
    HiddenAuthority,
}

/// Data-only coding-skill definition evaluated before package construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CodingSkillDefinition {
    /// Closed skill identity.
    pub skill: CodingSkill,
    /// Stable lowercase workflow identity.
    pub workflow_id: String,
    /// Bounded user-visible purpose.
    pub purpose: String,
    /// Closed exact or lexical-only language coverage.
    pub supported_languages: Vec<String>,
    /// Non-authoritative tools whose existing evidence the skill may consume.
    pub advisory_tools: Vec<String>,
    /// Mandatory validation evidence classes.
    pub required_validation: Vec<String>,
    /// Evidence-specific completion conditions.
    pub completion_criteria: Vec<String>,
    /// Maximum number of repository paths one invocation may inspect or discuss.
    pub maximum_paths: u16,
    /// Fixed denied authority ceiling.
    pub authority: SkillAuthorityCeiling,
    /// Complete closed operations that this definition cannot perform automatically.
    pub prohibited_operations: Vec<String>,
}

/// Content-free assessment of one exact coding-skill definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CodingSkillAssessment {
    /// Assessed skill identity.
    pub skill: CodingSkill,
    /// Admission outcome.
    pub admission: CodingSkillAdmission,
    /// Stable sorted findings; empty only for an admitted definition.
    pub findings: Vec<CodingSkillFinding>,
    /// Digest of the complete assessed definition.
    pub definition_sha256: String,
    /// Fixed false marker: assessment grants no capability.
    pub authority_granted: bool,
}

/// Returns the complete canonical v0.4 coding-skill definition set.
#[must_use]
pub fn built_in_coding_skill_definitions() -> Vec<CodingSkillDefinition> {
    CodingSkill::ALL
        .into_iter()
        .map(canonical_definition)
        .collect()
}

/// Evaluates a definition without loading content or granting authority.
#[must_use]
pub fn assess_coding_skill_definition(definition: &CodingSkillDefinition) -> CodingSkillAssessment {
    let mut findings = Vec::new();
    if definition.workflow_id != workflow_id(definition.skill) || !valid_text(&definition.purpose) {
        findings.push(CodingSkillFinding::InvalidIdentity);
    }
    if definition.completion_criteria.len() < 2
        || definition.completion_criteria.len() > 8
        || definition
            .completion_criteria
            .iter()
            .any(|criterion| !criterion.starts_with("Evidence: ") || !valid_text(criterion))
    {
        findings.push(CodingSkillFinding::VagueCompletion);
    }
    if !closed_sorted_subset(&definition.supported_languages, &SUPPORTED_LANGUAGES)
        || !definition
            .supported_languages
            .iter()
            .any(|language| language == "unsupported-lexical-only")
    {
        findings.push(CodingSkillFinding::UnsupportedLanguage);
    }
    if !closed_sorted_subset(&definition.advisory_tools, &ADVISORY_TOOLS) {
        findings.push(CodingSkillFinding::UnsupportedTool);
    }
    if definition.required_validation != REQUIRED_VALIDATION.map(str::to_owned).to_vec() {
        findings.push(CodingSkillFinding::MissingValidation);
    }
    if definition.maximum_paths == 0 || definition.maximum_paths > MAX_DECLARED_PATHS {
        findings.push(CodingSkillFinding::OverbroadScope);
    }
    if definition.authority != SkillAuthorityCeiling::denied()
        || definition.prohibited_operations != PROHIBITED_OPERATIONS.map(str::to_owned).to_vec()
    {
        findings.push(CodingSkillFinding::HiddenAuthority);
    }
    findings.sort();
    findings.dedup();
    let definition_sha256 = serde_json::to_vec(definition)
        .map(|bytes| sha256(&bytes))
        .unwrap_or_else(|_| sha256(b"invalid-coding-skill-definition"));
    CodingSkillAssessment {
        skill: definition.skill,
        admission: if findings.is_empty() {
            CodingSkillAdmission::Admitted
        } else {
            CodingSkillAdmission::Disabled
        },
        findings,
        definition_sha256,
        authority_granted: false,
    }
}

/// Builds the complete admitted, hash-bound, data-only coding skill pack.
pub fn built_in_coding_skill_pack() -> Result<Vec<DeclarativeSkillPackage>, DeclarativeSkillError> {
    built_in_coding_skill_definitions()
        .into_iter()
        .map(package_for_definition)
        .collect()
}

fn package_for_definition(
    definition: CodingSkillDefinition,
) -> Result<DeclarativeSkillPackage, DeclarativeSkillError> {
    let assessment = assess_coding_skill_definition(&definition);
    if assessment.admission != CodingSkillAdmission::Admitted {
        return Err(DeclarativeSkillError::TrustDenied);
    }
    let prompt = render_prompt(&definition).into_bytes();
    let path = format!("prompts/{}.prompt.md", definition.workflow_id);
    let manifest = seal_declarative_skill_manifest(DeclarativeSkillManifest {
        schema_version: 1,
        skill_id: format!("skill-{}", definition.workflow_id),
        source_id: "agentmage-built-in-coding-skills".to_owned(),
        source_sha256: sha256(b"agentmage-built-in-coding-skills-v1"),
        signer_or_provenance: "AgentMage source repository".to_owned(),
        license: "Apache-2.0".to_owned(),
        version: "0.4.0".to_owned(),
        compatibility: DeclarativeSkillCompatibility {
            minimum_knowledge_schema: 1,
            maximum_knowledge_schema: 1,
            minimum_agentmage_version: "0.4.0".to_owned(),
        },
        purpose: definition.purpose,
        precedence: 100,
        files: vec![DeclarativeSkillFile {
            path: path.clone(),
            kind: DeclarativeAssetKind::Prompt,
            semantic_key: definition.workflow_id.clone(),
            bytes: prompt.len() as u64,
            content_sha256: sha256(&prompt),
        }],
        requested_scope: DeclarativeSkillScope {
            max_context_bytes: CODING_CONTEXT_LIMIT,
            knowledge_record_kinds: vec!["document".to_owned(), "project".to_owned()],
            workflow_ids: vec![definition.workflow_id],
        },
        trust_state: DeclarativeSkillTrustState::Admitted,
        package_sha256: String::new(),
        manifest_sha256: String::new(),
    })?;
    Ok(DeclarativeSkillPackage {
        manifest,
        assets: vec![DeclarativeSkillAsset {
            path,
            bytes: prompt,
        }],
    })
}

fn canonical_definition(skill: CodingSkill) -> CodingSkillDefinition {
    let languages = SUPPORTED_LANGUAGES.map(str::to_owned).to_vec();
    let tools = tools_for(skill)
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    let completion_criteria = completion_for(skill)
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    CodingSkillDefinition {
        skill,
        workflow_id: workflow_id(skill).to_owned(),
        purpose: purpose(skill).to_owned(),
        supported_languages: languages,
        advisory_tools: tools,
        required_validation: REQUIRED_VALIDATION.map(str::to_owned).to_vec(),
        completion_criteria,
        maximum_paths: path_limit(skill),
        authority: SkillAuthorityCeiling::denied(),
        prohibited_operations: PROHIBITED_OPERATIONS.map(str::to_owned).to_vec(),
    }
}

fn workflow_id(skill: CodingSkill) -> &'static str {
    match skill {
        CodingSkill::RepositoryCartographer => "repository-cartographer",
        CodingSkill::FeatureTrace => "feature-trace",
        CodingSkill::ChangeImpact => "change-impact",
        CodingSkill::Debugging => "debugging",
        CodingSkill::TestAndVerification => "test-and-verification",
        CodingSkill::RepositoryDocumentation => "repository-documentation",
        CodingSkill::BugReproduction => "bug-reproduction",
        CodingSkill::GitHistoryAnalysis => "git-history-analysis",
        CodingSkill::BoundedReview => "bounded-review",
    }
}

fn purpose(skill: CodingSkill) -> &'static str {
    match skill {
        CodingSkill::RepositoryCartographer => {
            "Map repository structure, entry points, dependencies, data flow, permissions, tests, and coverage gaps from exact sources"
        }
        CodingSkill::FeatureTrace => {
            "Trace one bounded feature through interfaces, services, storage, background work, responses, and tests"
        }
        CodingSkill::ChangeImpact => {
            "Identify affected symbols, callers, data, permissions, tests, documentation, deployment concerns, risk, and rollback"
        }
        CodingSkill::Debugging => {
            "Diagnose one reproducible failure through explicit hypotheses, checks, root-cause evidence, and regression coverage"
        }
        CodingSkill::TestAndVerification => {
            "Propose repository-style tests and assess trusted validation receipts against explicit expected outcomes"
        }
        CodingSkill::RepositoryDocumentation => {
            "Draft source-grounded repository documentation with citations, limitations, and verification instructions"
        }
        CodingSkill::BugReproduction => {
            "Define a minimal deterministic reproduction and preserve the original repository while gathering evidence"
        }
        CodingSkill::GitHistoryAnalysis => {
            "Explain local change intent from bounded read-only Git history and exact object evidence"
        }
        CodingSkill::BoundedReview => {
            "Review an exact change packet for correctness, security, compatibility, tests, rollback, and unrelated changes"
        }
    }
}

fn tools_for(skill: CodingSkill) -> &'static [&'static str] {
    match skill {
        CodingSkill::RepositoryCartographer
        | CodingSkill::FeatureTrace
        | CodingSkill::ChangeImpact
        | CodingSkill::RepositoryDocumentation => &["exact-text-search", "repository-map"],
        CodingSkill::Debugging | CodingSkill::BugReproduction => &[
            "exact-text-search",
            "repository-map",
            "trusted-validation-results",
        ],
        CodingSkill::TestAndVerification => &[
            "repository-map",
            "structured-change-preview",
            "trusted-validation-results",
        ],
        CodingSkill::GitHistoryAnalysis => &[
            "exact-text-search",
            "read-only-git-history",
            "repository-map",
        ],
        CodingSkill::BoundedReview => &[
            "repository-map",
            "structured-change-preview",
            "trusted-validation-results",
        ],
    }
}

fn completion_for(skill: CodingSkill) -> &'static [&'static str] {
    match skill {
        CodingSkill::RepositoryCartographer => &[
            "Evidence: every structural claim cites an exact repository-map range or is marked unknown",
            "Evidence: entry points, dependencies, tests, permissions, and coverage limitations are present",
        ],
        CodingSkill::FeatureTrace => &[
            "Evidence: every trace hop cites an exact source range and direction",
            "Evidence: missing hops and unsupported relationships remain visibly unknown",
        ],
        CodingSkill::ChangeImpact => &[
            "Evidence: affected paths, symbols, data, permissions, tests, documentation, risk, and rollback are enumerated",
            "Evidence: impact claims distinguish direct evidence from bounded inference",
        ],
        CodingSkill::Debugging => &[
            "Evidence: the failure is reproduced or explicitly marked not reproduced",
            "Evidence: competing hypotheses, checks, root cause, and regression coverage are recorded",
        ],
        CodingSkill::TestAndVerification => &[
            "Evidence: proposed tests cover stated boundaries, failures, permissions, data changes, and rollback",
            "Evidence: every validation claim is bound to a trusted current receipt",
        ],
        CodingSkill::RepositoryDocumentation => &[
            "Evidence: every behavior statement cites current repository sources",
            "Evidence: commands, limitations, and verification steps are explicit and reviewable",
        ],
        CodingSkill::BugReproduction => &[
            "Evidence: the minimal reproduction records exact inputs, environment, steps, and observed result",
            "Evidence: the original repository remains unchanged and cleanup is deterministic",
        ],
        CodingSkill::GitHistoryAnalysis => &[
            "Evidence: every historical claim names exact local object identities",
            "Evidence: missing or shallow history is reported without fetching or invention",
        ],
        CodingSkill::BoundedReview => &[
            "Evidence: findings cite exact changed ranges, severity, consequence, and verification",
            "Evidence: unrelated changes, rollback, tests, and unresolved uncertainty are explicitly assessed",
        ],
    }
}

const fn path_limit(skill: CodingSkill) -> u16 {
    match skill {
        CodingSkill::RepositoryCartographer => 256,
        CodingSkill::FeatureTrace | CodingSkill::ChangeImpact => 128,
        CodingSkill::Debugging
        | CodingSkill::TestAndVerification
        | CodingSkill::RepositoryDocumentation
        | CodingSkill::BoundedReview => 64,
        CodingSkill::BugReproduction | CodingSkill::GitHistoryAnalysis => 32,
    }
}

fn render_prompt(definition: &CodingSkillDefinition) -> String {
    format!(
        "Purpose: {}.\nUse only current cited evidence from these advisory sources: {}.\nLanguage coverage: {}.\nValidation evidence required: {}.\nMaximum paths: {}.\nCompletion:\n- {}\nProhibited automatic operations: {}.\nThis data-only skill cannot execute tools, grant authority, change files, commit, publish, or access a network.\n",
        definition.purpose,
        definition.advisory_tools.join(", "),
        definition.supported_languages.join(", "),
        definition.required_validation.join(", "),
        definition.maximum_paths,
        definition.completion_criteria.join("\n- "),
        definition.prohibited_operations.join(", "),
    )
}

fn closed_sorted_subset(values: &[String], allowed: &[&str]) -> bool {
    !values.is_empty()
        && values.windows(2).all(|pair| pair[0] < pair[1])
        && values
            .iter()
            .all(|value| allowed.binary_search(&value.as_str()).is_ok())
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 16 * 1024 && !value.contains('\0')
}

fn sha256(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeclarativeSkillRegistry, compose_skill_context};

    #[test]
    fn canonical_coding_skill_pack_is_complete_hash_bound_and_authority_free() {
        let definitions = built_in_coding_skill_definitions();
        let packages = built_in_coding_skill_pack().expect("canonical coding pack");
        assert_eq!(definitions.len(), CodingSkill::ALL.len());
        assert_eq!(packages.len(), CodingSkill::ALL.len());
        for (definition, package) in definitions.iter().zip(&packages) {
            let assessment = assess_coding_skill_definition(definition);
            assert_eq!(assessment.admission, CodingSkillAdmission::Admitted);
            assert!(assessment.findings.is_empty());
            assert!(!assessment.authority_granted);
            assert_eq!(package.manifest.version, "0.4.0");
            assert_eq!(
                package.manifest.trust_state,
                DeclarativeSkillTrustState::Admitted
            );
            assert_eq!(package.manifest.files.len(), 1);
            assert_eq!(package.assets.len(), 1);
            let registry = DeclarativeSkillRegistry::build(vec![package.manifest.clone()])
                .expect("exact registry");
            let context = compose_skill_context(
                &registry,
                std::slice::from_ref(package),
                CODING_CONTEXT_LIMIT,
            )
            .expect("data-only context");
            assert_eq!(context.receipt.authority, SkillAuthorityCeiling::denied());
            assert!(!context.trusted_instruction_channel);
            assert!(!context.executed);
        }
    }

    #[test]
    fn vague_completion_hidden_authority_and_overbroad_scope_are_disabled() {
        let mut definition = canonical_definition(CodingSkill::Debugging);
        definition.completion_criteria = vec!["Done".to_owned()];
        definition.maximum_paths = MAX_DECLARED_PATHS + 1;
        definition.authority.writes = true;
        definition.prohibited_operations.pop();
        let assessment = assess_coding_skill_definition(&definition);
        assert_eq!(assessment.admission, CodingSkillAdmission::Disabled);
        assert_eq!(
            assessment.findings,
            vec![
                CodingSkillFinding::VagueCompletion,
                CodingSkillFinding::OverbroadScope,
                CodingSkillFinding::HiddenAuthority,
            ]
        );
        assert!(!assessment.authority_granted);
    }

    #[test]
    fn unsupported_tools_languages_and_missing_validation_are_disabled() {
        let mut definition = canonical_definition(CodingSkill::FeatureTrace);
        definition
            .supported_languages
            .push("unsupported-language-with-parser-claim".to_owned());
        definition.advisory_tools.push("ambient-shell".to_owned());
        definition.required_validation.pop();
        let assessment = assess_coding_skill_definition(&definition);
        assert_eq!(assessment.admission, CodingSkillAdmission::Disabled);
        assert_eq!(
            assessment.findings,
            vec![
                CodingSkillFinding::UnsupportedLanguage,
                CodingSkillFinding::UnsupportedTool,
                CodingSkillFinding::MissingValidation,
            ]
        );
    }

    #[test]
    fn skill_identity_and_sorted_closures_fail_closed() {
        let mut definition = canonical_definition(CodingSkill::BoundedReview);
        definition.workflow_id = "debugging".to_owned();
        definition.supported_languages.reverse();
        definition.advisory_tools.reverse();
        let assessment = assess_coding_skill_definition(&definition);
        assert_eq!(assessment.admission, CodingSkillAdmission::Disabled);
        assert_eq!(
            assessment.findings,
            vec![
                CodingSkillFinding::InvalidIdentity,
                CodingSkillFinding::UnsupportedLanguage,
                CodingSkillFinding::UnsupportedTool,
            ]
        );
    }

    #[test]
    fn every_prompt_names_completion_validation_limits_and_exclusions() {
        for definition in built_in_coding_skill_definitions() {
            let prompt = render_prompt(&definition);
            assert!(prompt.contains("Validation evidence required:"));
            assert!(prompt.contains("Maximum paths:"));
            assert!(prompt.contains("Completion:"));
            assert!(prompt.contains("Prohibited automatic operations:"));
            assert!(prompt.contains("cannot execute tools"));
            for operation in PROHIBITED_OPERATIONS {
                assert!(prompt.contains(operation));
            }
        }
    }
}
