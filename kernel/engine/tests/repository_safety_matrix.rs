use agentmage_kernel_engine::repository_safety::{
    GitRemoteIdentity, RepositoryOwnedDelta, RepositoryPreservationManifest, RepositorySafetyError,
    plan_branch_fast_forward, plan_fetch, reconcile_preservation,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const CORPUS: &str =
    include_str!("../../../docs/verification/sprint-42-repository-mutation-corpus.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u16,
    record_type: String,
    description: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    kind: String,
    value: String,
    approved_host: Option<String>,
    expected: String,
}

fn hash(label: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(label.as_ref());
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}

fn object(value: u8) -> String {
    format!("{value:040x}")
}

fn manifest() -> RepositoryPreservationManifest {
    RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
        schema_version: 0,
        repository_sha256: hash("repository"),
        common_directory_sha256: hash("common"),
        object_format: "sha1".to_owned(),
        safe_ownership: true,
        head_object: Some(object(1)),
        current_branch: Some("refs/heads/main".to_owned()),
        upstream: None,
        detached: false,
        index_sha256: hash("index"),
        path_dispositions_sha256: hash("paths"),
        untracked_count: 0,
        ignored_count: 0,
        local_branches_sha256: hash("local"),
        remote_tracking_refs_sha256: hash("remote"),
        tags_sha256: hash("tags"),
        notes_sha256: hash("notes"),
        stash_sha256: hash("stash"),
        replacement_refs_sha256: hash("replacement"),
        reflogs_sha256: hash("reflogs"),
        agentmage_refs_sha256: hash("agentmage"),
        worktrees_sha256: hash("worktrees"),
        submodules_sha256: hash("submodules"),
        lfs_sha256: hash("lfs"),
        object_database_sha256: hash("objects"),
        configuration_sha256: hash("configuration"),
        hooks_sha256: hash("hooks"),
        content_drivers_sha256: hash("drivers"),
        remotes_sha256: hash("remotes"),
        operations_sha256: hash("operations"),
        shallow: false,
        partial: false,
        hazardous_configuration: false,
        manifest_sha256: String::new(),
    })
    .expect("baseline manifest")
}

#[test]
fn fixed_repository_mutation_corpus_fails_closed() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("corpus parses");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.record_type, "sprint_42_repository_mutation_corpus");
    assert!(!corpus.description.is_empty());
    assert_eq!(corpus.cases.len(), 44);
    let mut ids = std::collections::BTreeSet::new();
    let remote = GitRemoteIdentity::parse("https://github.com/team/repository.git", "github.com")
        .expect("baseline remote");

    for case in corpus.cases {
        assert!(ids.insert(case.id.clone()), "duplicate {}", case.id);
        let allowed = match case.kind.as_str() {
            "remote_url" => GitRemoteIdentity::parse(
                &case.value,
                case.approved_host.as_deref().expect("remote host"),
            )
            .is_ok(),
            "source_ref" => plan_fetch(
                "transaction-corpus",
                remote.clone(),
                &case.value,
                &hash("repository-path"),
                &manifest(),
            )
            .is_ok(),
            "prohibited_argument" => {
                let mut plan = plan_branch_fast_forward(
                    "transaction-corpus",
                    "refs/heads/agentmage/tasks/corpus",
                    &object(1),
                    &object(2),
                    true,
                    &hash("repository-path"),
                    &manifest(),
                )
                .expect("baseline plan");
                plan.invocations[0].arguments.push(case.value.clone());
                plan.verify().is_ok()
            }
            other => panic!("unknown corpus kind: {other}"),
        };
        assert_eq!(allowed, case.expected == "allow", "{}", case.id);
    }
}

#[test]
fn ten_thousand_protected_manifest_mutations_have_zero_acceptance() {
    let before = manifest();
    for sequence in 0_u32..10_000 {
        let mut after = before.clone();
        let changed = hash(sequence.to_be_bytes());
        match sequence % 20 {
            0 => after.index_sha256 = changed,
            1 => after.path_dispositions_sha256 = changed,
            2 => after.local_branches_sha256 = changed,
            3 => after.remote_tracking_refs_sha256 = changed,
            4 => after.tags_sha256 = changed,
            5 => after.notes_sha256 = changed,
            6 => after.stash_sha256 = changed,
            7 => after.replacement_refs_sha256 = changed,
            8 => after.reflogs_sha256 = changed,
            9 => after.worktrees_sha256 = changed,
            10 => after.submodules_sha256 = changed,
            11 => after.lfs_sha256 = changed,
            12 => after.configuration_sha256 = changed,
            13 => after.hooks_sha256 = changed,
            14 => after.content_drivers_sha256 = changed,
            15 => after.remotes_sha256 = changed,
            16 => after.operations_sha256 = changed,
            17 => after.untracked_count = sequence + 1,
            18 => after.ignored_count = sequence + 1,
            _ => after.shallow = true,
        }
        let after = RepositoryPreservationManifest::seal(after).expect("mutation seals");
        assert_eq!(
            reconcile_preservation(&before, &after, RepositoryOwnedDelta::Fetch),
            Err(RepositorySafetyError::PreservationMismatch),
            "protected mutation {sequence} must fail"
        );
    }
}
