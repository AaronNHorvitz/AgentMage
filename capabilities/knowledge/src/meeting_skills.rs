//! Hash-bound meeting and continuity skill data with a permanent zero-authority ceiling.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    DeclarativeAssetKind, DeclarativeSkillAsset, DeclarativeSkillCompatibility,
    DeclarativeSkillError, DeclarativeSkillFile, DeclarativeSkillManifest, DeclarativeSkillPackage,
    DeclarativeSkillScope, DeclarativeSkillTrustState, seal_declarative_skill_manifest,
};

const MEETING_CONTEXT_LIMIT: u64 = 128 * 1024;

/// Closed built-in meeting-record skill inventory for the v0.6 candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingSkill {
    /// Local agenda and meeting-request preparation.
    AgendaAndRequest,
    /// Invitation-response and attendance evidence ledger.
    AttendeeLedger,
    /// Verbatim-preserving local note and transcript cleanup.
    TranscriptCleanup,
    /// Evidence-state-aware local minutes drafting.
    MinutesDraft,
    /// Deterministic closeout and unknown-field review.
    MeetingCloseout,
    /// Source-linked recurring continuity and local follow-up drafting.
    RecurringContinuity,
}

impl MeetingSkill {
    /// Complete stable meeting-record skill inventory.
    pub const ALL: [Self; 6] = [
        Self::AgendaAndRequest,
        Self::AttendeeLedger,
        Self::TranscriptCleanup,
        Self::MinutesDraft,
        Self::MeetingCloseout,
        Self::RecurringContinuity,
    ];
}

/// Builds the complete admitted, hash-bound, data-only meeting skill pack.
pub fn built_in_meeting_skill_pack() -> Result<Vec<DeclarativeSkillPackage>, DeclarativeSkillError>
{
    MeetingSkill::ALL
        .into_iter()
        .map(package_for_skill)
        .collect()
}

fn package_for_skill(
    skill: MeetingSkill,
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
        source_id: "agentmage-built-in-meeting-skills".to_owned(),
        source_sha256: sha256(b"agentmage-built-in-meeting-skills-v1"),
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
            max_context_bytes: MEETING_CONTEXT_LIMIT,
            knowledge_record_kinds: vec![
                "decision".to_owned(),
                "document".to_owned(),
                "meeting".to_owned(),
                "message".to_owned(),
                "person".to_owned(),
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

fn workflow_id(skill: MeetingSkill) -> &'static str {
    match skill {
        MeetingSkill::AgendaAndRequest => "meeting-agenda-and-request",
        MeetingSkill::AttendeeLedger => "meeting-attendee-ledger",
        MeetingSkill::TranscriptCleanup => "meeting-transcript-cleanup",
        MeetingSkill::MinutesDraft => "meeting-minutes-draft",
        MeetingSkill::MeetingCloseout => "meeting-closeout",
        MeetingSkill::RecurringContinuity => "meeting-recurring-continuity",
    }
}

fn purpose(skill: MeetingSkill) -> &'static str {
    match skill {
        MeetingSkill::AgendaAndRequest => {
            "Draft a local agenda or meeting request with purpose, participants, topics, decision needs, preparation, and expected outputs"
        }
        MeetingSkill::AttendeeLedger => {
            "Show invitation response and attendance as independent source-backed states without inferring either state from silence or the other state"
        }
        MeetingSkill::TranscriptCleanup => {
            "Clean user-provided local notes or transcripts while retaining exact verbatim text, timestamps, attribution confidence, and unclear-language markers"
        }
        MeetingSkill::MinutesDraft => {
            "Draft local minutes that distinguish confirmed and proposed decisions, actions, owners, dates, questions, risks, and next-meeting details"
        }
        MeetingSkill::MeetingCloseout => {
            "Review a sealed meeting record for confirmed decisions, proposals, actions, unknown owners, unknown dates, and open continuity items"
        }
        MeetingSkill::RecurringContinuity => {
            "Carry source-linked open items through a recurring series and draft a local follow-up while preserving immutable history"
        }
    }
}

fn prompt(skill: MeetingSkill) -> String {
    format!(
        "Purpose: {}.\nUse only already-approved local meeting records and exact source identities.\nPreserve verbatim text, timestamps, attribution confidence, unclear language, and confirmed, inferred, historical, disputed, and unknown states exactly.\nKeep invitation response separate from attendance; never infer either from silence or the other state.\nMark every missing owner or date unknown and keep confirmed and proposed decisions distinct.\nTreat every agenda, request, cleanup, minutes record, closeout, continuity record, and follow-up as a local proposal the user may edit or ignore.\nDo not invite, assign, notify, send, schedule, select recipients, access an inbox, change a calendar, mutate a source, create authority, or use a network.\n",
        purpose(skill),
    )
}

fn template(skill: MeetingSkill) -> String {
    match skill {
        MeetingSkill::AgendaAndRequest => "# Meeting Draft\n\n## Purpose\n\n## Participants and Observed States\n\n## Topics\n\n## Decision Needs\n\n## Preparation\n\n## Expected Outputs\n\n## Sources\n",
        MeetingSkill::AttendeeLedger => "# Attendee Ledger\n\n| Participant | Invitation state | Invitation evidence | Attendance state | Attendance evidence | Sources |\n|---|---|---|---|---|---|\n",
        MeetingSkill::TranscriptCleanup => "# Source-Preserving Cleanup\n\n## Verbatim Segments\n\n## Cleanup Proposals\n\n## Timestamps and Attribution\n\n## Unclear Language\n\n## Sources\n",
        MeetingSkill::MinutesDraft => "# Meeting Minutes Draft\n\n## Confirmed Decisions\n\n## Proposed Decisions\n\n## Actions, Owners, and Dates\n\n## Questions and Risks\n\n## Next Meeting\n\n## Unknowns and Sources\n",
        MeetingSkill::MeetingCloseout => "# Meeting Closeout\n\n## Decisions and Proposals\n\n## Actions\n\n## Unknown Owners and Dates\n\n## Open Continuity\n\n## Limitations\n",
        MeetingSkill::RecurringContinuity => "# Recurring Continuity\n\n| Item | State | Owner state | Date state | History | Sources |\n|---|---|---|---|---|---|\n\n## Local Follow-Up Draft\n",
    }
    .to_owned()
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

    use super::{MEETING_CONTEXT_LIMIT, MeetingSkill, built_in_meeting_skill_pack};

    #[test]
    fn meeting_pack_is_complete_hash_bound_removable_and_authority_free() {
        let packages = built_in_meeting_skill_pack().expect("meeting skill pack");
        assert_eq!(packages.len(), MeetingSkill::ALL.len());
        let registry = DeclarativeSkillRegistry::build(
            packages
                .iter()
                .map(|package| package.manifest.clone())
                .collect(),
        )
        .expect("registry");
        let context =
            compose_skill_context(&registry, &packages, MEETING_CONTEXT_LIMIT).expect("context");
        assert_eq!(context.receipt.authority, SkillAuthorityCeiling::denied());
        assert!(!context.trusted_instruction_channel);
        assert!(!context.executed);
        let first = registry.manifests()[0].skill_id.clone();
        assert_eq!(
            registry
                .without(&first)
                .expect("removable")
                .manifests()
                .len()
                + 1,
            registry.manifests().len()
        );
    }

    #[test]
    fn every_prompt_preserves_meeting_truth_and_prohibits_effects() {
        for package in built_in_meeting_skill_pack().expect("meeting skill pack") {
            let prompt = String::from_utf8(package.assets[0].bytes.clone()).expect("prompt");
            for phrase in [
                "Preserve verbatim text",
                "confirmed, inferred, historical, disputed, and unknown",
                "Keep invitation response separate from attendance",
                "Mark every missing owner or date unknown",
                "Do not invite, assign, notify, send, schedule",
                "access an inbox",
                "mutate a source",
                "use a network",
            ] {
                assert!(prompt.contains(phrase), "missing {phrase}");
            }
        }
    }
}
