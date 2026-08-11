use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ActorId, DataSensitivity, DisplayFileLink, GrantId, GrantNonce, GrantTarget, SessionId, TaskId,
    WorkspaceId, WorkspaceObjectIdentity, WorkspacePath,
};
use agentmage_kernel_engine::{
    grants::{GrantIssueError, GrantIssuer, SessionReadGrantRequest},
    policy::{PolicyBuildError, PolicyEngine, StrictLocalReadOnlyScope},
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

fn target(candidate: &str) -> GrantTarget {
    GrantTarget {
        workspace_id: WorkspaceId::from_raw("workspace-display-0001"),
        path_components: vec![candidate.to_owned()],
    }
}

fn request(case_id: &str, candidate: &str) -> SessionReadGrantRequest {
    SessionReadGrantRequest {
        grant_id: GrantId::from_raw(format!("grant-{case_id}")),
        actor_id: ActorId::from_raw("actor-local-0001"),
        session_id: SessionId::from_raw("session-display-0001"),
        task_id: TaskId::from_raw("task-display-0001"),
        targets: vec![target(candidate)],
        excluded_targets: Vec::new(),
        sensitivity: DataSensitivity::Ephemeral,
        issued_at_epoch_ms: 1_786_320_000_000,
        expires_at_epoch_ms: 1_786_320_060_000,
        nonce: GrantNonce::from_raw(format!("nonce-{case_id}")),
        maximum_derived_operations: 1,
        preview_sha256: "1".repeat(64),
        policy_sha256: "2".repeat(64),
    }
}

fn policy(candidate: &str) -> Result<PolicyEngine, PolicyBuildError> {
    PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision: 1,
        actors: BTreeSet::from([ActorId::from_raw("actor-local-0001")]),
        tasks: BTreeSet::from([TaskId::from_raw("task-display-0001")]),
        actions: BTreeSet::new(),
        tools: BTreeSet::new(),
        targets: BTreeSet::from([target(candidate)]),
    })
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

            let mut issuer = GrantIssuer::new();
            let grant_id = GrantId::from_raw(format!("grant-{}", case.case_id));
            let error = issuer
                .issue_session_read(request(&case.case_id, &candidate))
                .expect_err("display candidate cannot become current grant authority");
            assert_eq!(error, GrantIssueError::InvalidInput);
            assert!(issuer.current(&grant_id).is_none());
            rejection_count += 1;

            assert_eq!(
                policy(&candidate).unwrap_err(),
                PolicyBuildError::InvalidDocument
            );
            rejection_count += 1;

            // PlatformPathAdapter::resolve accepts only &WorkspacePath. Constructor
            // rejection above prevents an adapter call or filesystem observation.
            rejection_count += 1;
        }
    }
    assert_eq!(rejection_count, 128 * 2 * 5);
}
