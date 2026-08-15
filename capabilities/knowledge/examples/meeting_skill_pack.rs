use agentmage_capability_knowledge::{
    MeetingSkill, SkillAuthorityCeiling, built_in_meeting_skill_pack,
};
use serde_json::json;

fn main() {
    let packages =
        built_in_meeting_skill_pack().expect("the built-in meeting skill pack must remain valid");
    let manifests = packages
        .iter()
        .map(|package| package.manifest.clone())
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "record_type": "agentmage-meeting-skill-pack",
            "version": "0.6.0",
            "skills": MeetingSkill::ALL,
            "manifests": manifests,
            "prompt_count": packages.len(),
            "template_count": packages.len(),
            "authority": SkillAuthorityCeiling::denied(),
            "product_registration": false,
            "inbox_access": false,
            "network_access": false,
            "invite_access": false,
            "assignment_access": false,
            "notification_access": false,
            "send_access": false,
            "schedule_access": false,
            "calendar_mutation": false,
            "source_mutation": false
        }))
        .expect("meeting skill pack must serialize")
    );
}
