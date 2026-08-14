//! Built-in authority-free knowledge skill pack and interface-neutral read-only workflows.

use sha2::{Digest, Sha256};

use crate::{
    DeclarativeAssetKind, DeclarativeSkillAsset, DeclarativeSkillCompatibility,
    DeclarativeSkillError, DeclarativeSkillFile, DeclarativeSkillManifest, DeclarativeSkillPackage,
    DeclarativeSkillRegistry, DeclarativeSkillScope, DeclarativeSkillTrustState, KnowledgeError,
    KnowledgeRecord, KnowledgeRecordId, KnowledgeTaskView, KnowledgeTaskViewKind, SkillContext,
    build_task_view, compose_skill_context, seal_declarative_skill_manifest, validate_import,
};

const WORKFLOW_CONTEXT_LIMIT: u64 = 128 * 1024;

/// Closed built-in read-only knowledge workflow family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeWorkflow {
    /// Review active tasks, blockers, priorities, and next actions.
    DailySetup,
    /// Produce a source-backed briefing from current selected records.
    DailyBriefing,
    /// Classify one supplied issue without creating or changing a task.
    IssueIntake,
    /// Assemble a source-backed continuation view without exporting or writing it.
    Handoff,
    /// Extract review candidates from an existing meeting record without changing notes.
    MeetingCleanup,
    /// Summarize source-backed repository knowledge without repository mutation.
    RepositoryLearning,
    /// Review canonical plain-workspace health without changing user files.
    PlainWorkspaceSteward,
    /// Review canonical Obsidian-vault health without changing vault files.
    ObsidianVaultSteward,
}

impl KnowledgeWorkflow {
    /// Complete stable workflow inventory.
    pub const ALL: [Self; 8] = [
        Self::DailySetup,
        Self::DailyBriefing,
        Self::IssueIntake,
        Self::Handoff,
        Self::MeetingCleanup,
        Self::RepositoryLearning,
        Self::PlainWorkspaceSteward,
        Self::ObsidianVaultSteward,
    ];
}

/// Closed retrieval path used to assemble already-approved workflow evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeRetrievalMode {
    /// Deterministic lexical and metadata retrieval only.
    Lexical,
    /// Approved opt-in local semantic retrieval whose results remain non-authoritative.
    ApprovedLocalSemantic,
}

/// Exact source evidence supplied to one read-only workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeWorkflowEvidence {
    /// Retrieval path used to select the same canonical evidence identities.
    pub retrieval_mode: KnowledgeRetrievalMode,
    /// Sorted current citation digests.
    pub citation_sha256: Vec<String>,
    /// Digest of the exact current retrieval query and result projection.
    pub retrieval_result_sha256: String,
    /// Whether source evidence is current and non-conflicting.
    pub current_and_nonconflicting: bool,
}

/// Interface-neutral deterministic result for one built-in read-only workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeWorkflowResult {
    /// Selected built-in workflow.
    pub workflow: KnowledgeWorkflow,
    /// Stable source record identities in lexical order.
    pub source_record_ids: Vec<KnowledgeRecordId>,
    /// Digests of complete canonical source records in the same order.
    pub source_record_sha256: Vec<String>,
    /// Current derived task view when the workflow uses tasks.
    pub task_view: Option<KnowledgeTaskView>,
    /// Exact authority-free skill context and influence receipt.
    pub skill_context: SkillContext,
    /// Exact source evidence preserved through the workflow.
    pub evidence: KnowledgeWorkflowEvidence,
    /// Digest of the interface-neutral output contract.
    pub result_sha256: String,
    /// Fixed zero: v0.2 workflows cannot propose or apply user-file writes.
    pub proposed_write_count: u64,
    /// Fixed false: no source file changes.
    pub source_files_changed: bool,
    /// Fixed true: native Chat and a later CLI may render the same kernel contract.
    pub interface_neutral: bool,
}

/// Returns all eight built-in exact declarative skill packages.
pub fn built_in_knowledge_skill_pack() -> Result<Vec<DeclarativeSkillPackage>, DeclarativeSkillError>
{
    KnowledgeWorkflow::ALL
        .into_iter()
        .map(package_for_workflow)
        .collect()
}

/// Returns the exact built-in registry without loading or executing package content.
pub fn built_in_skill_registry() -> Result<DeclarativeSkillRegistry, DeclarativeSkillError> {
    DeclarativeSkillRegistry::build(
        built_in_knowledge_skill_pack()?
            .into_iter()
            .map(|package| package.manifest)
            .collect(),
    )
}

/// Runs one deterministic read-only workflow over current canonical records.
pub fn run_read_only_knowledge_workflow(
    workflow: KnowledgeWorkflow,
    records: &[KnowledgeRecord],
    evidence: KnowledgeWorkflowEvidence,
) -> Result<KnowledgeWorkflowResult, KnowledgeError> {
    validate_workflow_evidence(&evidence)?;
    let report = validate_import(records)?;
    let mut ordered = records.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    let source_record_ids = report.record_ids;
    let source_record_sha256 = ordered
        .iter()
        .map(|record| {
            serde_json::to_vec(record)
                .map(|bytes| sha256(&bytes))
                .map_err(|_| KnowledgeError::InvalidMetadata)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let task_view = workflow_task_view(workflow)
        .map(|kind| build_task_view(records, kind))
        .transpose()?;
    let packages = built_in_knowledge_skill_pack().map_err(map_skill_error)?;
    let package = packages
        .into_iter()
        .find(|package| package.manifest.skill_id == skill_id(workflow))
        .ok_or(KnowledgeError::InvalidIdentity)?;
    let registry =
        DeclarativeSkillRegistry::build(vec![package.manifest.clone()]).map_err(map_skill_error)?;
    let skill_context = compose_skill_context(&registry, &[package], WORKFLOW_CONTEXT_LIMIT)
        .map_err(map_skill_error)?;
    let result_sha256 = workflow_result_digest(
        workflow,
        &source_record_ids,
        &source_record_sha256,
        task_view.as_ref(),
        &skill_context,
        &evidence,
    )?;
    Ok(KnowledgeWorkflowResult {
        workflow,
        source_record_ids,
        source_record_sha256,
        task_view,
        skill_context,
        evidence,
        result_sha256,
        proposed_write_count: 0,
        source_files_changed: false,
        interface_neutral: true,
    })
}

fn package_for_workflow(
    workflow: KnowledgeWorkflow,
) -> Result<DeclarativeSkillPackage, DeclarativeSkillError> {
    let id = skill_id(workflow);
    let workflow_id = workflow_wire(workflow);
    let text = workflow_prompt(workflow).as_bytes().to_vec();
    let path = format!("prompts/{workflow_id}.prompt.md");
    let manifest = seal_declarative_skill_manifest(DeclarativeSkillManifest {
        schema_version: 1,
        skill_id: id.to_owned(),
        source_id: "agentmage-built-in-knowledge-skills".to_owned(),
        source_sha256: sha256(b"agentmage-built-in-knowledge-skills-v1"),
        signer_or_provenance: "AgentMage source repository".to_owned(),
        license: "Apache-2.0".to_owned(),
        version: "0.2.0".to_owned(),
        compatibility: DeclarativeSkillCompatibility {
            minimum_knowledge_schema: 1,
            maximum_knowledge_schema: 1,
            minimum_agentmage_version: "0.2.0".to_owned(),
        },
        purpose: workflow_purpose(workflow).to_owned(),
        precedence: 100,
        files: vec![DeclarativeSkillFile {
            path: path.clone(),
            kind: DeclarativeAssetKind::Prompt,
            semantic_key: workflow_id.to_owned(),
            bytes: text.len() as u64,
            content_sha256: sha256(&text),
        }],
        requested_scope: DeclarativeSkillScope {
            max_context_bytes: WORKFLOW_CONTEXT_LIMIT,
            knowledge_record_kinds: workflow_record_kinds(workflow)
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            workflow_ids: vec![workflow_id.to_owned()],
        },
        trust_state: DeclarativeSkillTrustState::Admitted,
        package_sha256: String::new(),
        manifest_sha256: String::new(),
    })?;
    Ok(DeclarativeSkillPackage {
        manifest,
        assets: vec![DeclarativeSkillAsset { path, bytes: text }],
    })
}

fn workflow_task_view(workflow: KnowledgeWorkflow) -> Option<KnowledgeTaskViewKind> {
    match workflow {
        KnowledgeWorkflow::DailySetup => Some(KnowledgeTaskViewKind::Active),
        KnowledgeWorkflow::DailyBriefing => Some(KnowledgeTaskViewKind::NextActions),
        KnowledgeWorkflow::IssueIntake => Some(KnowledgeTaskViewKind::Active),
        KnowledgeWorkflow::Handoff => Some(KnowledgeTaskViewKind::Active),
        KnowledgeWorkflow::MeetingCleanup => Some(KnowledgeTaskViewKind::NextActions),
        KnowledgeWorkflow::RepositoryLearning
        | KnowledgeWorkflow::PlainWorkspaceSteward
        | KnowledgeWorkflow::ObsidianVaultSteward => None,
    }
}

fn validate_workflow_evidence(evidence: &KnowledgeWorkflowEvidence) -> Result<(), KnowledgeError> {
    if !evidence.current_and_nonconflicting
        || !valid_sha256(&evidence.retrieval_result_sha256)
        || evidence.citation_sha256.len() > 1_000
        || evidence.citation_sha256.is_empty()
        || evidence
            .citation_sha256
            .iter()
            .any(|value| !valid_sha256(value))
        || evidence
            .citation_sha256
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(KnowledgeError::InvalidEvidence);
    }
    Ok(())
}

fn workflow_result_digest(
    workflow: KnowledgeWorkflow,
    record_ids: &[KnowledgeRecordId],
    record_sha256: &[String],
    task_view: Option<&KnowledgeTaskView>,
    context: &SkillContext,
    evidence: &KnowledgeWorkflowEvidence,
) -> Result<String, KnowledgeError> {
    let task_view_sha256 = task_view.map(|view| view.view_sha256.as_str());
    serde_json::to_vec(&(
        "agentmage-read-only-knowledge-workflow-v1",
        workflow_wire(workflow),
        record_ids
            .iter()
            .map(KnowledgeRecordId::as_str)
            .collect::<Vec<_>>(),
        record_sha256,
        task_view_sha256,
        &context.receipt.context_sha256,
        retrieval_mode_wire(evidence.retrieval_mode),
        &evidence.citation_sha256,
        &evidence.retrieval_result_sha256,
        0_u64,
        false,
        true,
    ))
    .map(|bytes| sha256(&bytes))
    .map_err(|_| KnowledgeError::InvalidMetadata)
}

fn skill_id(workflow: KnowledgeWorkflow) -> &'static str {
    match workflow {
        KnowledgeWorkflow::DailySetup => "skill-daily-setup",
        KnowledgeWorkflow::DailyBriefing => "skill-daily-briefing",
        KnowledgeWorkflow::IssueIntake => "skill-issue-intake",
        KnowledgeWorkflow::Handoff => "skill-handoff",
        KnowledgeWorkflow::MeetingCleanup => "skill-meeting-cleanup",
        KnowledgeWorkflow::RepositoryLearning => "skill-repository-learning",
        KnowledgeWorkflow::PlainWorkspaceSteward => "skill-plain-workspace-steward",
        KnowledgeWorkflow::ObsidianVaultSteward => "skill-obsidian-vault-steward",
    }
}

fn workflow_wire(workflow: KnowledgeWorkflow) -> &'static str {
    match workflow {
        KnowledgeWorkflow::DailySetup => "daily-setup",
        KnowledgeWorkflow::DailyBriefing => "daily-briefing",
        KnowledgeWorkflow::IssueIntake => "issue-intake",
        KnowledgeWorkflow::Handoff => "handoff",
        KnowledgeWorkflow::MeetingCleanup => "meeting-cleanup",
        KnowledgeWorkflow::RepositoryLearning => "repository-learning",
        KnowledgeWorkflow::PlainWorkspaceSteward => "plain-workspace-steward",
        KnowledgeWorkflow::ObsidianVaultSteward => "obsidian-vault-steward",
    }
}

fn retrieval_mode_wire(mode: KnowledgeRetrievalMode) -> &'static str {
    match mode {
        KnowledgeRetrievalMode::Lexical => "lexical",
        KnowledgeRetrievalMode::ApprovedLocalSemantic => "approved_local_semantic",
    }
}

fn workflow_purpose(workflow: KnowledgeWorkflow) -> &'static str {
    match workflow {
        KnowledgeWorkflow::DailySetup => {
            "Review current tasks, priorities, blockers, and next actions"
        }
        KnowledgeWorkflow::DailyBriefing => "Brief the user from current source-backed knowledge",
        KnowledgeWorkflow::IssueIntake => {
            "Classify a supplied issue without creating or changing records"
        }
        KnowledgeWorkflow::Handoff => "Assemble a local source-backed continuation view",
        KnowledgeWorkflow::MeetingCleanup => {
            "Identify decisions and follow-ups in existing meeting evidence"
        }
        KnowledgeWorkflow::RepositoryLearning => {
            "Summarize current source-backed repository knowledge"
        }
        KnowledgeWorkflow::PlainWorkspaceSteward => {
            "Review plain-folder knowledge health without writes"
        }
        KnowledgeWorkflow::ObsidianVaultSteward => {
            "Review Obsidian knowledge health without vault writes"
        }
    }
}

fn workflow_prompt(workflow: KnowledgeWorkflow) -> &'static str {
    match workflow {
        KnowledgeWorkflow::DailySetup => {
            "Use only cited current task records. List priorities, blockers, and next actions. Do not change records."
        }
        KnowledgeWorkflow::DailyBriefing => {
            "Summarize only current cited records. Preserve uncertainty and conflicts. Do not change records."
        }
        KnowledgeWorkflow::IssueIntake => {
            "Classify the supplied issue and identify missing evidence. Do not create, update, or close a task."
        }
        KnowledgeWorkflow::Handoff => {
            "Assemble a source-backed local handoff view with open work and constraints. Do not export or write."
        }
        KnowledgeWorkflow::MeetingCleanup => {
            "Identify candidate decisions, commitments, and follow-ups from cited meeting evidence. Do not edit notes."
        }
        KnowledgeWorkflow::RepositoryLearning => {
            "Summarize cited repository knowledge and unresolved questions. Do not execute or modify repository content."
        }
        KnowledgeWorkflow::PlainWorkspaceSteward => {
            "Report canonical plain-folder coverage, stale records, and link issues. Do not change user files."
        }
        KnowledgeWorkflow::ObsidianVaultSteward => {
            "Report canonical vault coverage, stale records, and link issues. Do not change Obsidian files."
        }
    }
}

fn workflow_record_kinds(workflow: KnowledgeWorkflow) -> &'static [&'static str] {
    match workflow {
        KnowledgeWorkflow::DailySetup => &["project", "task"],
        KnowledgeWorkflow::DailyBriefing => &["deadline", "meeting", "project", "task"],
        KnowledgeWorkflow::IssueIntake => &["project", "task"],
        KnowledgeWorkflow::Handoff => &["handoff", "project", "task"],
        KnowledgeWorkflow::MeetingCleanup => &["decision", "meeting", "task"],
        KnowledgeWorkflow::RepositoryLearning => &["document", "project", "question"],
        KnowledgeWorkflow::PlainWorkspaceSteward | KnowledgeWorkflow::ObsidianVaultSteward => {
            &["document", "project", "task"]
        }
    }
}

fn map_skill_error(error: DeclarativeSkillError) -> KnowledgeError {
    match error {
        DeclarativeSkillError::UnsupportedVersion => KnowledgeError::UnsupportedSchema,
        DeclarativeSkillError::HashMismatch => KnowledgeError::ContentDrift,
        DeclarativeSkillError::DuplicateIdentity => KnowledgeError::DuplicateRecord,
        DeclarativeSkillError::InvalidManifest
        | DeclarativeSkillError::TrustDenied
        | DeclarativeSkillError::ProhibitedContent
        | DeclarativeSkillError::ResourceLimit => KnowledgeError::InvalidMetadata,
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    use agentmage_kernel_contracts::DataSensitivity;

    use super::*;
    use crate::{
        KnowledgeField, KnowledgePrivacy, KnowledgeRecordKind, KnowledgeRetention,
        KnowledgeRetentionKind, SkillAuthorityCeiling,
    };

    fn task_record() -> KnowledgeRecord {
        KnowledgeRecord {
            schema_version: crate::KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse("knowledge-task-workflow").expect("identity"),
            kind: KnowledgeRecordKind::Task,
            title: "Complete bounded workflow".to_owned(),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            updated_at: "2026-08-14T00:00:00Z".to_owned(),
            last_verified_at: None,
            fields: vec![
                KnowledgeField {
                    name: "next_action".to_owned(),
                    value: "Run source-backed checks".to_owned(),
                },
                KnowledgeField {
                    name: "owner".to_owned(),
                    value: "Aaron".to_owned(),
                },
                KnowledgeField {
                    name: "priority".to_owned(),
                    value: "high".to_owned(),
                },
                KnowledgeField {
                    name: "project".to_owned(),
                    value: "AgentMage".to_owned(),
                },
                KnowledgeField {
                    name: "status".to_owned(),
                    value: "open".to_owned(),
                },
            ],
            links: Vec::new(),
            tags: vec!["workflow".to_owned()],
            evidence: Vec::new(),
        }
    }

    fn evidence(mode: KnowledgeRetrievalMode) -> KnowledgeWorkflowEvidence {
        KnowledgeWorkflowEvidence {
            retrieval_mode: mode,
            citation_sha256: vec!["a".repeat(64), "b".repeat(64)],
            retrieval_result_sha256: "c".repeat(64),
            current_and_nonconflicting: true,
        }
    }

    #[test]
    fn built_in_pack_is_exact_complete_hash_bound_and_removable() {
        let packages = built_in_knowledge_skill_pack().expect("built-in pack");
        assert_eq!(packages.len(), KnowledgeWorkflow::ALL.len());
        let identities = packages
            .iter()
            .map(|package| package.manifest.skill_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(identities.len(), 8);
        assert!(packages.iter().all(|package| {
            package.manifest.trust_state == DeclarativeSkillTrustState::Admitted
                && package.manifest.version == "0.2.0"
                && package.manifest.files.len() == 1
                && package.assets.len() == 1
        }));
        let registry = built_in_skill_registry().expect("registry");
        let first = packages[0].manifest.skill_id.clone();
        assert_eq!(
            registry.without(&first).expect("removal").manifests().len(),
            7
        );
    }

    #[test]
    fn every_promoted_workflow_is_read_only_source_backed_and_interface_neutral() {
        let records = vec![task_record()];
        let before = serde_json::to_vec(&records).expect("source snapshot");
        for workflow in KnowledgeWorkflow::ALL {
            let result = run_read_only_knowledge_workflow(
                workflow,
                &records,
                evidence(KnowledgeRetrievalMode::Lexical),
            )
            .expect("workflow runs");
            assert_eq!(result.workflow, workflow);
            assert_eq!(result.source_record_ids, vec![records[0].record_id.clone()]);
            assert_eq!(result.proposed_write_count, 0);
            assert!(!result.source_files_changed);
            assert!(result.interface_neutral);
            assert_eq!(
                result.skill_context.receipt.authority,
                SkillAuthorityCeiling::denied()
            );
            assert!(!result.skill_context.trusted_instruction_channel);
            assert!(!result.skill_context.executed);
            assert_eq!(
                serde_json::to_vec(&records).expect("source unchanged"),
                before
            );
        }
    }

    #[test]
    fn lexical_and_approved_semantic_paths_preserve_source_and_task_evidence_parity() {
        let records = vec![task_record()];
        for workflow in KnowledgeWorkflow::ALL {
            let lexical = run_read_only_knowledge_workflow(
                workflow,
                &records,
                evidence(KnowledgeRetrievalMode::Lexical),
            )
            .expect("lexical workflow");
            let semantic = run_read_only_knowledge_workflow(
                workflow,
                &records,
                evidence(KnowledgeRetrievalMode::ApprovedLocalSemantic),
            )
            .expect("semantic workflow");
            assert_eq!(lexical.source_record_ids, semantic.source_record_ids);
            assert_eq!(lexical.source_record_sha256, semantic.source_record_sha256);
            assert_eq!(lexical.task_view, semantic.task_view);
            assert_eq!(
                lexical.evidence.citation_sha256,
                semantic.evidence.citation_sha256
            );
            assert_eq!(
                lexical.skill_context.receipt.context_sha256,
                semantic.skill_context.receipt.context_sha256
            );
        }
    }

    #[test]
    fn plain_and_obsidian_stewards_share_identical_canonical_record_contracts() {
        let records = vec![task_record()];
        let plain = run_read_only_knowledge_workflow(
            KnowledgeWorkflow::PlainWorkspaceSteward,
            &records,
            evidence(KnowledgeRetrievalMode::Lexical),
        )
        .expect("plain workflow");
        let obsidian = run_read_only_knowledge_workflow(
            KnowledgeWorkflow::ObsidianVaultSteward,
            &records,
            evidence(KnowledgeRetrievalMode::Lexical),
        )
        .expect("Obsidian workflow");
        assert_eq!(plain.source_record_ids, obsidian.source_record_ids);
        assert_eq!(plain.source_record_sha256, obsidian.source_record_sha256);
        assert_eq!(plain.task_view, obsidian.task_view);
        assert_eq!(plain.proposed_write_count, obsidian.proposed_write_count);
    }

    #[test]
    fn stale_conflicting_unsorted_or_missing_evidence_blocks_every_workflow() {
        for workflow in KnowledgeWorkflow::ALL {
            let mut stale = evidence(KnowledgeRetrievalMode::Lexical);
            stale.current_and_nonconflicting = false;
            assert!(run_read_only_knowledge_workflow(workflow, &[task_record()], stale).is_err());
            let mut unsorted = evidence(KnowledgeRetrievalMode::Lexical);
            unsorted.citation_sha256.reverse();
            assert!(
                run_read_only_knowledge_workflow(workflow, &[task_record()], unsorted).is_err()
            );
            let mut missing = evidence(KnowledgeRetrievalMode::Lexical);
            missing.citation_sha256.clear();
            assert!(run_read_only_knowledge_workflow(workflow, &[task_record()], missing).is_err());
        }
    }
}
