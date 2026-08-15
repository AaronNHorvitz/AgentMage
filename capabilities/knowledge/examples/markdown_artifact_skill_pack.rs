use agentmage_capability_knowledge::{
    MarkdownArtifactSkill, SkillAuthorityCeiling, built_in_markdown_artifact_skill_pack,
};
use serde_json::json;

fn main() {
    let packages = built_in_markdown_artifact_skill_pack()
        .expect("the built-in Markdown artifact skill pack must remain valid");
    let manifests = packages
        .iter()
        .map(|package| package.manifest.clone())
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "record_type": "agentmage-markdown-artifact-skill-pack",
            "version": "0.6.0",
            "skills": MarkdownArtifactSkill::ALL,
            "manifests": manifests,
            "prompt_count": packages.len(),
            "template_count": packages.len(),
            "authority": SkillAuthorityCeiling::denied(),
            "product_registration": false,
            "filesystem_access": false,
            "network_access": false,
            "execution_access": false,
            "renderer_access": false,
            "source_mutation": false,
            "citation_invention": false,
            "acronym_expansion": false,
            "remote_asset_fetch": false
        }))
        .expect("Markdown artifact skill pack must serialize")
    );
}
