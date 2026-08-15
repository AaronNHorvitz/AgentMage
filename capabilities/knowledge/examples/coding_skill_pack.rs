use agentmage_capability_knowledge::{
    assess_coding_skill_definition, built_in_coding_skill_definitions, built_in_coding_skill_pack,
};
use serde_json::json;

fn main() {
    let definitions = built_in_coding_skill_definitions();
    let assessments = definitions
        .iter()
        .map(assess_coding_skill_definition)
        .collect::<Vec<_>>();
    let manifests = built_in_coding_skill_pack()
        .expect("the built-in coding skill pack must remain valid")
        .into_iter()
        .map(|package| package.manifest)
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "record_type": "agentmage-coding-skill-pack",
            "version": "0.4.0",
            "definitions": definitions,
            "assessments": assessments,
            "manifests": manifests,
            "product_registration": false,
            "authority_enabled": false,
            "network_access": false,
            "automatic_publication": false
        }))
        .expect("coding skill pack must serialize")
    );
}
