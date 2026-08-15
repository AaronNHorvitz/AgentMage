use agentmage_capability_knowledge::{
    ExecutiveSkill, SkillAuthorityCeiling, built_in_executive_skill_pack,
};
use serde_json::json;

fn main() {
    let packages = built_in_executive_skill_pack()
        .expect("the built-in executive skill pack must remain valid");
    let manifests = packages
        .iter()
        .map(|package| package.manifest.clone())
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "record_type": "agentmage-executive-skill-pack",
            "version": "0.6.0",
            "skills": ExecutiveSkill::ALL,
            "manifests": manifests,
            "prompt_count": packages.len(),
            "template_count": packages.len(),
            "authority": SkillAuthorityCeiling::denied(),
            "product_registration": false,
            "inbox_access": false,
            "network_access": false,
            "notification_access": false,
            "send_access": false,
            "schedule_access": false,
            "source_mutation": false
        }))
        .expect("executive skill pack must serialize")
    );
}
