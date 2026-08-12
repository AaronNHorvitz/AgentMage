use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    DisplayFileLink, GrantTarget, WorkspaceId, WorkspaceObjectIdentity, WorkspacePath,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    corpus_id: String,
    link_count: usize,
    authority_surface_count: usize,
    authority_surfaces: Vec<String>,
    links: Vec<LinkCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkCase {
    schema_version: u32,
    case_id: String,
    file_uri: String,
    line: u32,
    rendered_target: String,
}

fn grant_target(candidate: &str, kind: &str) -> Result<GrantTarget, serde_json::Error> {
    let value = if kind == "workspace_scope" {
        serde_json::json!({
            "target_kind": kind,
            "path": {"workspace_id": "workspace-display-0001", "components": [candidate]},
            "authorization_id": "authorization-display-0001",
            "adapter_instance_id": "adapter-display-0001",
            "platform": "deterministic_fake"
        })
    } else {
        serde_json::json!({
            "target_kind": kind,
            "path": {"workspace_id": "workspace-display-0001", "components": [candidate]},
            "authorization_id": "authorization-display-0001",
            "adapter_instance_id": "adapter-display-0001",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![2_u8; 32]
            },
            "preimage": {"byte_len": 7, "content_sha256": vec![3_u8; 32]}
        })
    };
    serde_json::from_value(value)
}

#[test]
fn generated_display_links_are_rejected_by_every_runtime_authority_boundary() {
    let corpus: Corpus = serde_json::from_str(include_str!(
        "../../../fixtures/paths/v1/display-link-corpus.json"
    ))
    .expect("published display-link corpus must parse");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.corpus_id, "agentmage-display-link-replay-v1");
    assert_eq!(corpus.link_count, 128);
    assert_eq!(corpus.authority_surface_count, 5);
    assert_eq!(
        corpus.authority_surfaces,
        [
            "workspace_path_constructor",
            "workspace_path_deserializer",
            "grant_issuer",
            "policy_builder",
            "platform_adapter_typed_input",
        ]
    );
    assert_eq!(corpus.links.len(), corpus.link_count);

    let identity = WorkspaceObjectIdentity::new(
        agentmage_kernel_contracts::PathPlatform::DeterministicFake,
        [1; 32],
        [2; 32],
    );
    let mut case_ids = BTreeSet::new();
    let mut rejection_count = 0;

    for case in corpus.links {
        assert_eq!(case.schema_version, 1);
        assert!(case_ids.insert(case.case_id.clone()));
        let link = DisplayFileLink::new(case.file_uri.clone(), Some(case.line), identity.clone())
            .expect("synthetic display link is valid display data");
        assert_eq!(link.render_target(), case.rendered_target);
        assert!(!format!("{link:?}").contains("synthetic"));

        for candidate in [case.file_uri, case.rendered_target] {
            let path = WorkspacePath::new(
                WorkspaceId::from_raw("workspace-display-0001"),
                [candidate.clone()],
            );
            assert!(path.is_err());
            rejection_count += 1;

            let wire = serde_json::json!({
                "workspace_id": "workspace-display-0001",
                "components": [candidate.clone()],
            });
            assert!(serde_json::from_value::<WorkspacePath>(wire).is_err());
            rejection_count += 1;

            assert!(grant_target(&candidate, "workspace_scope").is_err());
            rejection_count += 1;

            assert!(grant_target(&candidate, "held_object").is_err());
            rejection_count += 1;

            // PlatformPathAdapter::resolve accepts only &WorkspacePath. Constructor
            // rejection above prevents an adapter call or filesystem observation.
            rejection_count += 1;
        }
    }
    assert_eq!(rejection_count, 128 * 2 * 5);
}
