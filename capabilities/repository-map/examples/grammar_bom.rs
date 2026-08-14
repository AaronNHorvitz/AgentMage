//! Emits the compiled grammar inventory for local evidence generation.

fn main() {
    let descriptors = agentmage_capability_repository_map::supported_grammars();
    let document = serde_json::json!({
        "schema_version": 1,
        "grammar_set_sha256": agentmage_capability_repository_map::grammar_set_sha256(),
        "grammars": descriptors,
    });
    println!(
        "{}",
        serde_json::to_string(&document).expect("compiled grammar inventory is serializable")
    );
}
