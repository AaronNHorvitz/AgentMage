//! Hash-bound executive-assistant skill data with a permanent zero-authority ceiling.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    DeclarativeAssetKind, DeclarativeSkillAsset, DeclarativeSkillCompatibility,
    DeclarativeSkillError, DeclarativeSkillFile, DeclarativeSkillManifest, DeclarativeSkillPackage,
    DeclarativeSkillScope, DeclarativeSkillTrustState, seal_declarative_skill_manifest,
};

const EXECUTIVE_CONTEXT_LIMIT: u64 = 128 * 1024;

/// Closed built-in executive-assistant skill inventory for the v0.6 candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveSkill {
    /// Start-of-cycle, closeout, and recurring briefing projections.
    CycleBriefing,
    /// Commitment, decision, waiting, approval, deadline, and reminder views.
    Trackers,
    /// Deterministic evidence-backed priority explanations.
    PriorityAssistant,
    /// Meeting, decision, person, organization, and briefing-pack views.
    EvidenceBriefs,
    /// Local correspondence drafting and deterministic response review.
    CorrespondenceReview,
    /// Triage over user-provided local message exports.
    LocalMessageTriage,
    /// Portfolio, workload, preparation, changes-since, and forgotten-item reviews.
    PortfolioReview,
    /// Retrieval, retention, indexing, export, and source-influence audit views.
    PrivacyAudit,
}

impl ExecutiveSkill {
    /// Complete stable executive-assistant skill inventory.
    pub const ALL: [Self; 8] = [
        Self::CycleBriefing,
        Self::Trackers,
        Self::PriorityAssistant,
        Self::EvidenceBriefs,
        Self::CorrespondenceReview,
        Self::LocalMessageTriage,
        Self::PortfolioReview,
        Self::PrivacyAudit,
    ];
}

/// Builds the complete admitted, hash-bound, data-only executive-assistant skill pack.
pub fn built_in_executive_skill_pack() -> Result<Vec<DeclarativeSkillPackage>, DeclarativeSkillError>
{
    ExecutiveSkill::ALL
        .into_iter()
        .map(package_for_skill)
        .collect()
}

fn package_for_skill(
    skill: ExecutiveSkill,
) -> Result<DeclarativeSkillPackage, DeclarativeSkillError> {
    let workflow_id = workflow_id(skill);
    let prompt = prompt(skill).as_bytes().to_vec();
    let template = template(skill).as_bytes().to_vec();
    let prompt_path = format!("prompts/{workflow_id}.prompt.md");
    let template_path = format!("templates/{workflow_id}.template.md");
    let files = vec![
        DeclarativeSkillFile {
            path: prompt_path.clone(),
            kind: DeclarativeAssetKind::Prompt,
            semantic_key: format!("{workflow_id}-prompt"),
            bytes: prompt.len() as u64,
            content_sha256: sha256(&prompt),
        },
        DeclarativeSkillFile {
            path: template_path.clone(),
            kind: DeclarativeAssetKind::Template,
            semantic_key: format!("{workflow_id}-template"),
            bytes: template.len() as u64,
            content_sha256: sha256(&template),
        },
    ];
    let manifest = seal_declarative_skill_manifest(DeclarativeSkillManifest {
        schema_version: 1,
        skill_id: format!("skill-{workflow_id}"),
        source_id: "agentmage-built-in-executive-skills".to_owned(),
        source_sha256: sha256(b"agentmage-built-in-executive-skills-v1"),
        signer_or_provenance: "AgentMage source repository".to_owned(),
        license: "Apache-2.0".to_owned(),
        version: "0.6.0".to_owned(),
        compatibility: DeclarativeSkillCompatibility {
            minimum_knowledge_schema: 1,
            maximum_knowledge_schema: 1,
            minimum_agentmage_version: "0.6.0".to_owned(),
        },
        purpose: purpose(skill).to_owned(),
        precedence: 100,
        files,
        requested_scope: DeclarativeSkillScope {
            max_context_bytes: EXECUTIVE_CONTEXT_LIMIT,
            knowledge_record_kinds: vec![
                "decision".to_owned(),
                "document".to_owned(),
                "meeting".to_owned(),
                "message".to_owned(),
                "organization".to_owned(),
                "person".to_owned(),
                "project".to_owned(),
                "task".to_owned(),
            ],
            workflow_ids: vec![workflow_id.to_owned()],
        },
        trust_state: DeclarativeSkillTrustState::Admitted,
        package_sha256: String::new(),
        manifest_sha256: String::new(),
    })?;
    Ok(DeclarativeSkillPackage {
        manifest,
        assets: vec![
            DeclarativeSkillAsset {
                path: prompt_path,
                bytes: prompt,
            },
            DeclarativeSkillAsset {
                path: template_path,
                bytes: template,
            },
        ],
    })
}

fn workflow_id(skill: ExecutiveSkill) -> &'static str {
    match skill {
        ExecutiveSkill::CycleBriefing => "executive-cycle-briefing",
        ExecutiveSkill::Trackers => "executive-trackers",
        ExecutiveSkill::PriorityAssistant => "executive-priority-assistant",
        ExecutiveSkill::EvidenceBriefs => "executive-evidence-briefs",
        ExecutiveSkill::CorrespondenceReview => "executive-correspondence-review",
        ExecutiveSkill::LocalMessageTriage => "executive-local-message-triage",
        ExecutiveSkill::PortfolioReview => "executive-portfolio-review",
        ExecutiveSkill::PrivacyAudit => "executive-privacy-audit",
    }
}

fn purpose(skill: ExecutiveSkill) -> &'static str {
    match skill {
        ExecutiveSkill::CycleBriefing => {
            "Project approved local schedule, priority, deadline, waiting, preparation, completion, and handoff records into bounded cycle views"
        }
        ExecutiveSkill::Trackers => {
            "Project exact local commitments, decisions, waiting items, approvals, deadlines, and reminders without notifying or changing another system"
        }
        ExecutiveSkill::PriorityAssistant => {
            "Explain deterministic priority ranking from urgency, importance, dependencies, schedule, effort, preference, consequence, and evidence state"
        }
        ExecutiveSkill::EvidenceBriefs => {
            "Draft meeting, decision, person, organization, and briefing-pack views while preserving every evidence state and exact source"
        }
        ExecutiveSkill::CorrespondenceReview => {
            "Draft locally and check questions, commitments, dates, attachments, claims, privacy, and names without selecting recipients or sending"
        }
        ExecutiveSkill::LocalMessageTriage => {
            "Classify user-provided local message exports without connecting to, reading, changing, or sending through an inbox"
        }
        ExecutiveSkill::PortfolioReview => {
            "Review portfolio status, workload conflicts, preparation, changes, stale records, and forgotten items from canonical local history"
        }
        ExecutiveSkill::PrivacyAudit => {
            "Show exact source influence and enforce ordinary, private, confidential, and highly restricted retrieval, index, retention, and export rules"
        }
    }
}

fn prompt(skill: ExecutiveSkill) -> String {
    format!(
        "Purpose: {}.\nUse only already-approved local canonical records and exact source identities.\nPreserve confirmed, inferred, historical, disputed, and unknown states exactly.\nShow assumptions, limitations, conflicts, missing fields, method identity, and source influence.\nTreat every output as a local proposal the user may edit or ignore.\nDo not infer sensitive traits, assign work, notify, send, schedule, select recipients, change a calendar, access an inbox, mutate a source, create authority, or use a network.\n",
        purpose(skill),
    )
}

fn template(skill: ExecutiveSkill) -> String {
    match skill {
        ExecutiveSkill::CycleBriefing => "# Cycle Briefing\n\n## Schedule\n\n## Priorities\n\n## Deadlines and Waiting\n\n## Preparation\n\n## Limitations and Sources\n",
        ExecutiveSkill::Trackers => "# Tracker\n\n| Item | State | Evidence state | Person | Date | Sources |\n|---|---|---|---|---|---|\n",
        ExecutiveSkill::PriorityAssistant => "# Priority Explanation\n\n## Method\n\n## Ranked Items\n\n## Components\n\n## Assumptions, Conflicts, and Sources\n",
        ExecutiveSkill::EvidenceBriefs => "# Evidence Brief\n\n## Purpose or Question\n\n## Confirmed\n\n## Inferred or Historical\n\n## Disputed or Unknown\n\n## Options, Risks, and Dissent\n\n## Sources\n",
        ExecutiveSkill::CorrespondenceReview => "# Correspondence Draft\n\n**Recipients:** [user-reviewed]\n\n**Subject:** [bounded subject]\n\n[local draft]\n\n## Review Findings\n\n## Claims and Sources\n",
        ExecutiveSkill::LocalMessageTriage => "# Local Message Export Triage\n\n| Message | Classification | Reason | Source |\n|---|---|---|---|\n",
        ExecutiveSkill::PortfolioReview => "# Portfolio Review\n\n## Active Projects\n\n## Conflicts and Risks\n\n## Changes Since\n\n## Forgotten Items\n\n## History and Sources\n",
        ExecutiveSkill::PrivacyAudit => "# Executive Audit\n\n| Output | Record | Privacy class | Evidence state | Method | Sources | Limitations |\n|---|---|---|---|---|---|---|\n",
    }.to_owned()
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
    use crate::{DeclarativeSkillRegistry, SkillAuthorityCeiling, compose_skill_context};

    use super::{EXECUTIVE_CONTEXT_LIMIT, ExecutiveSkill, built_in_executive_skill_pack};

    #[test]
    fn executive_pack_is_complete_hash_bound_removable_and_authority_free() {
        let packages = built_in_executive_skill_pack().expect("executive skill pack");
        assert_eq!(packages.len(), ExecutiveSkill::ALL.len());
        let registry = DeclarativeSkillRegistry::build(
            packages
                .iter()
                .map(|package| package.manifest.clone())
                .collect(),
        )
        .expect("registry");
        let context =
            compose_skill_context(&registry, &packages, EXECUTIVE_CONTEXT_LIMIT).expect("context");
        assert_eq!(context.receipt.authority, SkillAuthorityCeiling::denied());
        assert!(!context.trusted_instruction_channel);
        assert!(!context.executed);
        for package in &packages {
            assert_eq!(package.manifest.version, "0.6.0");
            assert_eq!(package.manifest.files.len(), 2);
            assert_eq!(package.assets.len(), 2);
            assert!(package.assets.iter().all(|asset| {
                let text = String::from_utf8_lossy(&asset.bytes);
                text.contains("Source") || text.contains("approved local canonical records")
            }));
        }
        let first = registry.manifests()[0].skill_id.clone();
        let reduced = registry.without(&first).expect("removable registry entry");
        assert_eq!(reduced.manifests().len() + 1, registry.manifests().len());
    }

    #[test]
    fn every_prompt_preserves_truth_and_prohibits_external_actions() {
        for package in built_in_executive_skill_pack().expect("executive skill pack") {
            let prompt = String::from_utf8(package.assets[0].bytes.clone()).expect("prompt text");
            for phrase in [
                "confirmed, inferred, historical, disputed, and unknown",
                "Do not infer sensitive traits",
                "notify",
                "send",
                "schedule",
                "access an inbox",
                "mutate a source",
                "use a network",
            ] {
                assert!(prompt.contains(phrase));
            }
        }
    }
}
