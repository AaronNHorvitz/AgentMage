use agentmage_capability_knowledge::{
    DocumentControlSkill, SkillAuthorityCeiling, built_in_document_control_skill_pack,
};
use serde_json::json;

fn main() {
    let packages = built_in_document_control_skill_pack()
        .expect("the built-in document-control skill pack must remain valid");
    let manifests = packages
        .iter()
        .map(|package| package.manifest.clone())
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "record_type": "agentmage-document-control-skill-pack",
            "version": "0.6.0",
            "skills": DocumentControlSkill::ALL,
            "manifests": manifests,
            "prompt_count": packages.len(),
            "template_count": packages.len(),
            "authority": SkillAuthorityCeiling::denied(),
            "product_registration": false,
            "inbox_access": false,
            "network_access": false,
            "recipient_selection": false,
            "send_access": false,
            "schedule_access": false,
            "notification_access": false,
            "filesystem_mutation": false,
            "records_disposition": false,
            "final_language_mutation": false
        }))
        .expect("document-control skill pack must serialize")
    );
}
