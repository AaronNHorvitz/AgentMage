use std::{collections::BTreeSet, env, fmt::Write as _, fs, path::PathBuf};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, CapabilityGrant, DataSensitivity, GrantId, GrantNonce,
    GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, SessionId, TaskId,
    ToolId, WorkspaceId,
};
use agentmage_kernel_engine::{
    grants::{
        DerivedOperationGrantRequest, GrantConsumeError, GrantIssuer, SessionReadGrantRequest,
    },
    policy::{
        PolicyDenialScope, PolicyEngine, PolicyEvaluationContext, StrictLocalReadOnlyScope,
        ToolPolicyBinding,
    },
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const EMIT_ENV: &str = "AGENTMAGE_EMIT_ADVERSARIAL_GRANT_CORPUS";
const MARKER: &str = "AGENTMAGE_ADVERSARIAL_GRANT_CORPUS=";
const CORPUS_PATH: &str = "fixtures/grants/adversarial/v1/corpus.json";
const CASES_PER_CLASS: usize = 40;
const BASE_SEED: u64 = 0x5a17_c0de_0000_0001;
const SEED_STEP: u64 = 0x9e37_79b9_7f4a_7c15;
const MUTATION_CLASSES: [&str; 14] = [
    "actor",
    "session",
    "task",
    "action",
    "tool",
    "path",
    "argument",
    "preimage",
    "side_effect",
    "expiry",
    "nonce",
    "use_count",
    "parent",
    "preview_digest",
];

#[derive(Serialize)]
struct AdversarialGrantCorpus {
    schema_version: u16,
    corpus_id: &'static str,
    base_profile: &'static str,
    seed_algorithm: &'static str,
    base_seed: u64,
    cases_per_class: usize,
    case_count: usize,
    mutation_classes: &'static [&'static str],
    cases: Vec<AdversarialGrantCase>,
}

#[derive(Serialize)]
struct AdversarialGrantCase {
    schema_version: u16,
    case_id: String,
    seed: u64,
    mutation_class: &'static str,
    mutation_variant: &'static str,
    evaluation_path: &'static str,
    expected_denial_scope: &'static str,
    expected_code: &'static str,
    resulting_status: &'static str,
    resulting_revision: u32,
    resulting_use_count: u32,
    admitted_attempts: u32,
}

struct Fixture {
    issuer: GrantIssuer,
    policy: PolicyEngine,
    grant: CapabilityGrant,
    context: PolicyEvaluationContext,
}

fn hex_sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn seeded_digest(class: &str, seed: u64) -> String {
    hex_sha256(format!("agentmage:{class}:{seed:016x}").as_bytes())
}

fn seeded_identifier(prefix: &str, seed: u64) -> String {
    format!("{prefix}-{seed:016x}")
}

fn target(path: &[&str]) -> GrantTarget {
    GrantTarget {
        workspace_id: WorkspaceId::from_raw("workspace-0001"),
        path_components: path.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn fixture() -> Fixture {
    let actor_id = ActorId::from_raw("actor-local-0001");
    let session_id = SessionId::from_raw("session-0001");
    let task_id = TaskId::from_raw("task-0001");
    let action_id = ActionId::from_raw("action-0001");
    let tool_id = ToolId::from_raw("fixture.read");
    let operation_target = target(&["src", "fixture.txt"]);
    let policy = PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision: 1,
        actors: BTreeSet::from([actor_id.clone()]),
        tasks: BTreeSet::from([task_id.clone()]),
        actions: BTreeSet::from([action_id.clone()]),
        tools: BTreeSet::from([ToolPolicyBinding {
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".to_owned(),
        }]),
        targets: BTreeSet::from([operation_target.clone()]),
    })
    .expect("strict fixture policy must build");
    let mut issuer = GrantIssuer::new();
    let parent = issuer
        .issue_session_read(SessionReadGrantRequest {
            grant_id: GrantId::from_raw("grant-parent-0001"),
            actor_id: actor_id.clone(),
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            targets: vec![target(&[])],
            excluded_targets: vec![target(&["private"])],
            sensitivity: DataSensitivity::Ephemeral,
            issued_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 60_000,
            nonce: GrantNonce::from_raw("nonce-parent-0001"),
            maximum_derived_operations: 1,
            preview_sha256: "1".repeat(64),
            policy_sha256: policy.policy_sha256().to_owned(),
        })
        .expect("session parent must issue");
    let grant = issuer
        .derive_operation(
            &parent.grant_id,
            DerivedOperationGrantRequest {
                grant_id: GrantId::from_raw("grant-operation-0001"),
                action_id: action_id.clone(),
                action_kind: ActionKind::DeterministicTool,
                operation: GrantOperation::WorkspaceRead,
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
                targets: vec![operation_target.clone()],
                argument_sha256: "2".repeat(64),
                preimages: vec![GrantPreimage {
                    target_index: 0,
                    content_sha256: "3".repeat(64),
                    observed_revision: Some("fixture-v1".to_owned()),
                }],
                expected_side_effects: vec![GrantSideEffect {
                    operation: GrantOperation::WorkspaceRead,
                    target_indexes: vec![0],
                    details_sha256: "4".repeat(64),
                }],
                rollback_description: "No state change is permitted".to_owned(),
                issued_at_epoch_ms: 2_000,
                expires_at_epoch_ms: 30_000,
                nonce: GrantNonce::from_raw("nonce-operation-0001"),
                preview_sha256: "5".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("operation grant must derive");
    let context = PolicyEvaluationContext {
        actor_id,
        session_id,
        task_id,
        action_id,
        action_kind: ActionKind::DeterministicTool,
        tool_id,
        tool_version: "1.0.0".to_owned(),
        targets: grant.targets.clone(),
        argument_sha256: grant.argument_sha256.clone(),
        preimages: grant.preimages.clone(),
        expected_side_effects: grant.expected_side_effects.clone(),
        preview_sha256: grant.preview_sha256.clone(),
        now_epoch_ms: 3_000,
        network_scope: None,
        credential_scope: None,
        publication_scope: None,
    };
    Fixture {
        issuer,
        policy,
        grant,
        context,
    }
}

fn scope_name(scope: PolicyDenialScope) -> &'static str {
    match scope {
        PolicyDenialScope::Grant => "grant",
        PolicyDenialScope::Actor => "actor",
        PolicyDenialScope::Session => "session",
        PolicyDenialScope::Task => "task",
        PolicyDenialScope::Action => "action",
        PolicyDenialScope::Tool => "tool",
        PolicyDenialScope::Operation => "operation",
        PolicyDenialScope::Path => "path",
        PolicyDenialScope::Argument => "argument",
        PolicyDenialScope::Preimage => "preimage",
        PolicyDenialScope::SideEffect => "side_effect",
        PolicyDenialScope::Preview => "preview",
        PolicyDenialScope::Network => "network",
        PolicyDenialScope::Credential => "credential",
        PolicyDenialScope::Publication => "publication",
    }
}

fn status_name(status: GrantStatus) -> &'static str {
    match status {
        GrantStatus::Issued => "issued",
        GrantStatus::Consumed => "consumed",
        GrantStatus::Revoked => "revoked",
        GrantStatus::Expired => "expired",
        GrantStatus::Invalidated => "invalidated",
        GrantStatus::Uncertain => "uncertain",
    }
}

fn atomic_case(
    mutation_class: &'static str,
    mutation_variant: &'static str,
    seed: u64,
    expected_scope: PolicyDenialScope,
    expected_status: GrantStatus,
    mutate: impl FnOnce(&mut PolicyEvaluationContext),
) -> AdversarialGrantCase {
    let mut fixture = fixture();
    mutate(&mut fixture.context);
    let error = fixture
        .issuer
        .consume_for_execution(&fixture.grant.grant_id, &fixture.policy, &fixture.context)
        .expect_err("adversarial case must not consume authority");
    assert_eq!(error, GrantConsumeError::PolicyDenied(expected_scope));
    let retained = fixture
        .issuer
        .current(&fixture.grant.grant_id)
        .expect("grant must remain retained");
    assert_eq!(retained.status, expected_status);
    assert_eq!(retained.revision, 2);
    assert_eq!(retained.use_count, 0);
    let resulting_revision = retained.revision;
    let resulting_use_count = retained.use_count;
    let mut replay = fixture.context.clone();
    replay.actor_id = fixture.grant.actor_id.clone();
    replay.session_id = fixture.grant.session_id.clone();
    replay.task_id = fixture.grant.task_id.clone();
    replay.action_id = fixture.grant.action_id.clone().expect("action");
    replay.action_kind = fixture.grant.action_kind.expect("action kind");
    replay.tool_id = fixture.grant.tool_id.clone().expect("tool");
    replay.tool_version = fixture.grant.tool_version.clone().expect("tool version");
    replay.targets = fixture.grant.targets.clone();
    replay.argument_sha256 = fixture.grant.argument_sha256.clone();
    replay.preimages = fixture.grant.preimages.clone();
    replay.expected_side_effects = fixture.grant.expected_side_effects.clone();
    replay.preview_sha256 = fixture.grant.preview_sha256.clone();
    replay.now_epoch_ms = 3_000;
    assert_eq!(
        fixture
            .issuer
            .consume_for_execution(&fixture.grant.grant_id, &fixture.policy, &replay),
        Err(GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant))
    );
    AdversarialGrantCase {
        schema_version: 1,
        case_id: format!("grant-{mutation_class}-{seed:016x}"),
        seed,
        mutation_class,
        mutation_variant,
        evaluation_path: "atomic_consumption",
        expected_denial_scope: scope_name(expected_scope),
        expected_code: expected_scope.code(),
        resulting_status: status_name(expected_status),
        resulting_revision,
        resulting_use_count,
        admitted_attempts: 0,
    }
}

fn candidate_case(
    mutation_class: &'static str,
    mutation_variant: &'static str,
    seed: u64,
    mutate: impl FnOnce(&mut CapabilityGrant),
) -> AdversarialGrantCase {
    let fixture = fixture();
    let original = fixture
        .issuer
        .current(&fixture.grant.grant_id)
        .expect("issued grant")
        .clone();
    let mut candidate = fixture.grant.clone();
    mutate(&mut candidate);
    let decision = fixture
        .policy
        .evaluate(&fixture.issuer, &candidate, &fixture.context);
    assert!(!decision.allowed);
    assert_eq!(decision.denial_scope, Some(PolicyDenialScope::Grant));
    assert_eq!(decision.code, PolicyDenialScope::Grant.code());
    assert_eq!(
        fixture.issuer.current(&fixture.grant.grant_id),
        Some(&original)
    );
    AdversarialGrantCase {
        schema_version: 1,
        case_id: format!("grant-{mutation_class}-{seed:016x}"),
        seed,
        mutation_class,
        mutation_variant,
        evaluation_path: "forged_candidate_evaluation",
        expected_denial_scope: "grant",
        expected_code: PolicyDenialScope::Grant.code(),
        resulting_status: "issued",
        resulting_revision: original.revision,
        resulting_use_count: original.use_count,
        admitted_attempts: 0,
    }
}

fn case_for(mutation_class: &'static str, seed: u64) -> AdversarialGrantCase {
    match mutation_class {
        "actor" => atomic_case(
            mutation_class,
            "context_actor_identity",
            seed,
            PolicyDenialScope::Actor,
            GrantStatus::Invalidated,
            |context| {
                context.actor_id = ActorId::from_raw(seeded_identifier("actor-mut", seed));
            },
        ),
        "session" => atomic_case(
            mutation_class,
            "context_session_identity",
            seed,
            PolicyDenialScope::Session,
            GrantStatus::Invalidated,
            |context| {
                context.session_id = SessionId::from_raw(seeded_identifier("session-mut", seed));
            },
        ),
        "task" => atomic_case(
            mutation_class,
            "context_task_identity",
            seed,
            PolicyDenialScope::Task,
            GrantStatus::Invalidated,
            |context| {
                context.task_id = TaskId::from_raw(seeded_identifier("task-mut", seed));
            },
        ),
        "action" => atomic_case(
            mutation_class,
            "context_action_identity",
            seed,
            PolicyDenialScope::Action,
            GrantStatus::Invalidated,
            |context| {
                context.action_id = ActionId::from_raw(seeded_identifier("action-mut", seed));
            },
        ),
        "tool" => atomic_case(
            mutation_class,
            "context_tool_version",
            seed,
            PolicyDenialScope::Tool,
            GrantStatus::Invalidated,
            |context| {
                context.tool_version = format!("9.0.{}", seed % 1_000);
            },
        ),
        "path" => atomic_case(
            mutation_class,
            "context_target_component",
            seed,
            PolicyDenialScope::Path,
            GrantStatus::Invalidated,
            |context| {
                context.targets[0]
                    .path_components
                    .push(seeded_identifier("path-mut", seed));
            },
        ),
        "argument" => atomic_case(
            mutation_class,
            "context_argument_digest",
            seed,
            PolicyDenialScope::Argument,
            GrantStatus::Invalidated,
            |context| {
                context.argument_sha256 = seeded_digest("argument", seed);
            },
        ),
        "preimage" => atomic_case(
            mutation_class,
            "context_preimage_digest",
            seed,
            PolicyDenialScope::Preimage,
            GrantStatus::Invalidated,
            |context| {
                context.preimages[0].content_sha256 = seeded_digest("preimage", seed);
            },
        ),
        "side_effect" => atomic_case(
            mutation_class,
            "context_effect_digest",
            seed,
            PolicyDenialScope::SideEffect,
            GrantStatus::Invalidated,
            |context| {
                context.expected_side_effects[0].details_sha256 =
                    seeded_digest("side-effect", seed);
            },
        ),
        "expiry" => atomic_case(
            mutation_class,
            "clock_at_or_after_expiry",
            seed,
            PolicyDenialScope::Grant,
            GrantStatus::Expired,
            |context| {
                context.now_epoch_ms = 30_000 + seed % 1_000;
            },
        ),
        "nonce" => candidate_case(mutation_class, "candidate_nonce", seed, |candidate| {
            candidate.nonce = GrantNonce::from_raw(seeded_identifier("nonce-mut", seed));
        }),
        "use_count" => candidate_case(mutation_class, "candidate_use_count", seed, |candidate| {
            candidate.use_count = 1 + u32::try_from(seed % 8).expect("bounded seed");
        }),
        "parent" => candidate_case(
            mutation_class,
            if seed.is_multiple_of(2) {
                "candidate_parent_identity"
            } else {
                "candidate_parent_revision_digest"
            },
            seed,
            |candidate| {
                if seed.is_multiple_of(2) {
                    candidate.parent_grant_id = Some(GrantId::from_raw(seeded_identifier(
                        "grant-parent-mut",
                        seed,
                    )));
                } else {
                    candidate.parent_grant_sha256 = Some(seeded_digest("parent", seed));
                }
            },
        ),
        "preview_digest" => atomic_case(
            mutation_class,
            "context_preview_digest",
            seed,
            PolicyDenialScope::Preview,
            GrantStatus::Invalidated,
            |context| {
                context.preview_sha256 = seeded_digest("preview", seed);
            },
        ),
        _ => panic!("unknown mutation class"),
    }
}

fn generated_corpus() -> AdversarialGrantCorpus {
    let mut cases = Vec::with_capacity(MUTATION_CLASSES.len() * CASES_PER_CLASS);
    for (class_index, mutation_class) in MUTATION_CLASSES.iter().enumerate() {
        for case_index in 0..CASES_PER_CLASS {
            let ordinal = class_index * CASES_PER_CLASS + case_index;
            let seed = BASE_SEED.wrapping_add(
                u64::try_from(ordinal)
                    .expect("case ordinal must fit")
                    .wrapping_mul(SEED_STEP),
            );
            cases.push(case_for(mutation_class, seed));
        }
    }
    AdversarialGrantCorpus {
        schema_version: 1,
        corpus_id: "agentmage-adversarial-grants-v1",
        base_profile: "strict-local-read-only-exact-operation",
        seed_algorithm: "base_plus_wrapping_ordinal_times_step_v1",
        base_seed: BASE_SEED,
        cases_per_class: CASES_PER_CLASS,
        case_count: cases.len(),
        mutation_classes: &MUTATION_CLASSES,
        cases,
    }
}

#[test]
fn adversarial_grant_corpus_denies_every_seeded_case() {
    let corpus = generated_corpus();
    assert_eq!(corpus.case_count, 560);
    let bytes = serde_json::to_vec(&corpus).expect("corpus must serialize");
    if env::var(EMIT_ENV).as_deref() == Ok("1") {
        println!(
            "{MARKER}{}",
            String::from_utf8(bytes).expect("corpus JSON must be UTF-8")
        );
        return;
    }
    let retained = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(CORPUS_PATH),
    )
    .expect("retained adversarial grant corpus must exist");
    assert_eq!(retained, bytes);
}
