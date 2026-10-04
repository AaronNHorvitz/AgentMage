//! Producer fixtures of the runtime producer contract (Decision 0135,
//! AMR-06.1). Every committed fixture of the current version is built here
//! from the runtime's own types and functions and compared byte for byte;
//! each decodes exactly and verifies as a client verifies it, and a changed
//! record does not. Version 2 adds the session's recoverability to the run
//! declarations (Decision 0143), version 3 the catalog host's route grant
//! operation on wire 17 (Decision 0144), and version 4 the standalone
//! evidence host's folder operation on wire 18 (Decision 0150); earlier
//! versions stay as they were and read as unavailable.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use agentmage_kernel_contracts::{
    ContextAdmission, ContextItemCandidate, ContextItemKind, ContextPacketId, ContextSensitivity,
    RuntimeRunRequest,
};
use agentmage_kernel_engine::action_history::{
    ActionAuthorization, ActionKind, ActionOutcome, ActionRecordDraft,
};
use agentmage_kernel_engine::context_inspection::inspect_context;
use agentmage_kernel_engine::context_management::{ContextCompositionBudget, compose_context};
use agentmage_kernel_engine::engineering_recipe::parse_recipe_manifest;
use agentmage_kernel_engine::job_control::{
    JobControlAction, JobControlLedger, JobControlRequest, JobOwnerEvent,
};
use agentmage_kernel_engine::runtime_coordinator::{
    seal_runtime_run_request, verify_runtime_run_request,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::coding_action_history::{
    DecidedJobControl, ENDED_RUN_HISTORIES_SCHEMA_VERSION, EndedRunActionHistories,
    RUN_ACTION_HISTORY_OWNER, RUN_ACTION_RETENTION_MS, RunActionChain, RunActionHistory,
    RunActionRecorder, StoredRunChain, job_control_draft, verified_ended_run_histories,
    verify_run_action_history,
};
use crate::coding_recipe::{
    RuntimeRecipeRequest, instantiate_run_recipe, recipe_constraints, recipe_request,
    test_registry, verify_declared_recipe_plan,
};
use crate::coding_recoverability::{
    SessionEffect, assess_recoverability, assess_run_recoverability, verify_run_recoverability,
    verify_session_recoverability,
};
use crate::coding_route::{
    route_development_run, verify_run_route_history, verify_run_route_receipt,
};
use crate::runtime_transport::{
    RUN_DECLARATIONS_SCHEMA_VERSION, RuntimeJobControl, RuntimeJobStatus, RuntimeRunDeclarations,
    decode_exact,
};

const CONTRACT: &str = "agentmage-runtime-producer";
const CONTRACT_VERSION: u16 = 4;
/// The synthetic time every fixture record was made at.
const MADE_AT_EPOCH_MS: u64 = 1_790_000_000_000;
/// The scope a host derives for the client that sent the control request.
const CLIENT_SCOPE: &str = "peer-00112233445566778899aabbccddeeff";

/// One synthetic run as a producer would describe it.
struct Records {
    request: RuntimeRunRequest,
    recipe: RuntimeRecipeRequest,
    declarations: RuntimeRunDeclarations,
    absent: RuntimeRunDeclarations,
    control_request: JobControlRequest,
    job_control: RuntimeJobControl,
    job_status: RuntimeJobStatus,
    ended: EndedRunActionHistories,
}

#[derive(Serialize)]
struct ManifestEntry {
    file: &'static str,
    record: &'static str,
    schema_version: u16,
    sha256: String,
}

#[derive(Serialize)]
struct Manifest {
    contract: &'static str,
    contract_version: u16,
    wire_version: u16,
    records: Vec<ManifestEntry>,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/runtime-producer/v4")
}

/// An earlier contract version's fixtures, kept unchanged beside the current
/// ones.
fn version_root(version: u16) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../fixtures/runtime-producer/v{version}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A synthetic digest named by its role in the fixture.
fn digest(label: &str) -> String {
    sha256_hex(format!("agentmage-runtime-producer-fixture:{label}").as_bytes())
}

fn pretty(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn candidate(id: &str, kind: ContextItemKind, essential: bool, text: &str) -> ContextItemCandidate {
    ContextItemCandidate {
        item_id: id.to_owned(),
        kind,
        sensitivity: ContextSensitivity::Internal,
        admission: ContextAdmission::Eligible,
        authoritative_evidence: false,
        essential,
        source_id: format!("source:{id}"),
        source_revision: "r1".to_owned(),
        content_sha256: sha256_hex(text.as_bytes()),
        bounded_excerpt: text.to_owned(),
        token_count: u32::try_from(text.len()).unwrap(),
    }
}

fn records() -> Records {
    let base = crate::coding_run::tests::fixture_profile_and_request().1;
    let workspace = base.workspace_id.clone();
    // The run is held to the committed repair recipe sample (Decision 0133);
    // its plan digest is bound into the sealed request's constraints.
    let manifest = parse_recipe_manifest(include_bytes!(
        "../fixtures/recipe-sample/repair-in-src.json"
    ))
    .unwrap();
    let recipe = recipe_request(
        manifest,
        &[
            ("attempts".to_owned(), "2".to_owned()),
            ("target".to_owned(), "src/calc.py".to_owned()),
        ],
        &workspace,
    )
    .unwrap();
    let plan = instantiate_run_recipe(&recipe, &workspace, &test_registry(&workspace)).unwrap();
    let mut request = base;
    request.task.constraints.extend(recipe_constraints(&plan));
    let request = seal_runtime_run_request(request).unwrap();
    let run_id = request.run_id.as_str();

    let recoverability = assess_run_recoverability(
        request.session_id.as_str(),
        request.task.task_id.as_str(),
        run_id,
        &[SessionEffect::Command {
            operation_id: "operation-validation-0001".to_owned(),
        }],
        &|_| None,
    )
    .unwrap();
    // Decision 0143: the session's declaration covers an earlier run's
    // creation as well as this run's command, and names no run.
    let session_recoverability = assess_recoverability(
        request.session_id.as_str(),
        request.task.task_id.as_str(),
        &[
            SessionEffect::Create {
                operation_id: "operation-create-0001".to_owned(),
                path: vec!["src".to_owned(), "helpers.py".to_owned()],
            },
            SessionEffect::Command {
                operation_id: "operation-validation-0001".to_owned(),
            },
        ],
        &|_| None,
    )
    .unwrap();
    let packet = compose_context(
        ContextPacketId::from_raw("packet-producer-fixture"),
        &ContextCompositionBudget {
            max_bytes: 1 << 20,
            max_tokens: 200,
            max_items: 16,
            token_counter_id: "counter-producer-fixture".to_owned(),
        },
        vec![
            candidate(
                "system",
                ContextItemKind::Instruction,
                true,
                "instruction text",
            ),
            candidate(
                "request",
                ContextItemKind::NewestRequest,
                true,
                "request text",
            ),
            candidate(
                "source-a",
                ContextItemKind::Supporting,
                false,
                "supporting source",
            ),
            {
                let mut denied = candidate("memory-a", ContextItemKind::Memory, false, "memory");
                denied.admission = ContextAdmission::Denied;
                denied
            },
        ],
    )
    .unwrap();
    let inspection = inspect_context(&packet).unwrap();

    let retain_until = MADE_AT_EPOCH_MS + RUN_ACTION_RETENTION_MS;
    let mut effects = RunActionRecorder::new();
    effects.record(Some(ActionRecordDraft {
        action_kind: ActionKind::FileWrite,
        action_id: "operation-write-0001".to_owned(),
        authorization: ActionAuthorization::Grant {
            grant_id: "grant-write-0001".to_owned(),
            grant_sha256: digest("grant"),
        },
        effect_sha256: digest("effect-write"),
        outcome: ActionOutcome::Succeeded,
        reason_code: "coding.approved.succeeded".to_owned(),
        evidence_sha256s: {
            let mut evidence = vec![digest("decision"), digest("preview"), digest("authority")];
            evidence.sort();
            evidence
        },
        recorded_at_epoch_ms: MADE_AT_EPOCH_MS,
        retain_until_epoch_ms: retain_until,
    }));
    effects.record(Some(ActionRecordDraft {
        action_kind: ActionKind::FileWrite,
        action_id: "operation-write-0002".to_owned(),
        authorization: ActionAuthorization::Unauthorized {},
        effect_sha256: digest("effect-refused"),
        outcome: ActionOutcome::Denied,
        reason_code: "recipe.out-of-scope".to_owned(),
        evidence_sha256s: {
            let mut evidence = vec![digest("refused-preview"), digest("policy-decision")];
            evidence.sort();
            evidence
        },
        recorded_at_epoch_ms: MADE_AT_EPOCH_MS + 1,
        retain_until_epoch_ms: retain_until + 1,
    }));
    let effect_history = effects.declare().unwrap();

    // The job ran, a client asked to cancel it, and the owner stopped.
    let mut ledger = JobControlLedger::create(run_id, RUN_ACTION_HISTORY_OWNER).unwrap();
    ledger.observe_owner(JobOwnerEvent::Started).unwrap();
    let control_request = JobControlRequest {
        schema_version: 1,
        job_id: run_id.to_owned(),
        request_id: "cancel-producer-fixture-0001".to_owned(),
        action: JobControlAction::Cancel,
        observed_revision: ledger.observation().revision,
    };
    let decision = ledger.control(CLIENT_SCOPE, &control_request).unwrap();
    let status = |ledger: &JobControlLedger| RuntimeJobStatus {
        schema_version: 1,
        run_id: request.run_id.clone(),
        request_sha256: request.request_sha256.clone(),
        job: ledger.observation(),
    };
    let job_control = RuntimeJobControl {
        decision,
        status: status(&ledger),
    };
    let mut controls = RunActionRecorder::new();
    controls.record(job_control_draft(&DecidedJobControl {
        client_scope: CLIENT_SCOPE,
        request: &control_request,
        decision,
        ledger_head_sha256: &ledger.head().head_sha256,
        decided_at_epoch_ms: MADE_AT_EPOCH_MS + 2,
    }));
    let job_control_history = controls.declare().unwrap();
    ledger
        .observe_owner(JobOwnerEvent::CancellationObserved)
        .unwrap();
    let job_status = status(&ledger);

    let route = route_development_run(&request, "producer-fixture", MADE_AT_EPOCH_MS).unwrap();
    let route_history = route.history.clone().unwrap();
    let declarations = RuntimeRunDeclarations {
        schema_version: RUN_DECLARATIONS_SCHEMA_VERSION,
        run_id: request.run_id.clone(),
        request_sha256: request.request_sha256.clone(),
        recoverability: Some(recoverability),
        session_recoverability: Some(session_recoverability),
        context_inspections: Some(vec![inspection]),
        effect_history: Some(effect_history.clone()),
        job_control_history: Some(job_control_history.clone()),
        route_receipt: Some(route.receipt),
        route_history: Some(route_history.clone()),
        recipe_plan: Some(plan),
    };
    let absent = RuntimeRunDeclarations {
        recoverability: None,
        session_recoverability: None,
        context_inspections: None,
        effect_history: None,
        job_control_history: None,
        route_receipt: None,
        route_history: None,
        recipe_plan: None,
        ..declarations.clone()
    };
    let stored = |history: RunActionHistory| StoredRunChain {
        records: history.records,
        head: history.head,
        complete: true,
        closed: true,
    };
    let ended = EndedRunActionHistories {
        schema_version: ENDED_RUN_HISTORIES_SCHEMA_VERSION,
        run_id: run_id.to_owned(),
        effects: Some(stored(effect_history)),
        job_control: Some(stored(job_control_history)),
        routes: Some(stored(route_history)),
    };
    Records {
        request,
        recipe,
        declarations,
        absent,
        control_request,
        job_control,
        job_status,
        ended,
    }
}

/// Every record file with its record name and schema version.
fn record_files(records: &Records) -> Vec<(&'static str, &'static str, u16, Vec<u8>)> {
    vec![
        (
            "run-request.json",
            "run_request",
            records.request.schema_version,
            pretty(&records.request),
        ),
        (
            "run-recipe.json",
            "run_recipe",
            records.recipe.manifest.schema_version,
            pretty(&records.recipe),
        ),
        (
            "run-declarations.json",
            "run_declarations",
            records.declarations.schema_version,
            pretty(&records.declarations),
        ),
        (
            "run-declarations-absent.json",
            "run_declarations",
            records.absent.schema_version,
            pretty(&records.absent),
        ),
        (
            "job-control-request.json",
            "job_control_request",
            records.control_request.schema_version,
            pretty(&records.control_request),
        ),
        (
            "job-control.json",
            "job_control",
            records.job_control.status.schema_version,
            pretty(&records.job_control),
        ),
        (
            "job-status.json",
            "job_status",
            records.job_status.schema_version,
            pretty(&records.job_status),
        ),
        (
            "ended-run-histories.json",
            "ended_run_histories",
            records.ended.schema_version,
            pretty(&records.ended),
        ),
    ]
}

/// Every fixture file, the manifest of their digests included.
fn fixture_files(records: &Records) -> BTreeMap<String, Vec<u8>> {
    let files = record_files(records);
    let manifest = Manifest {
        contract: CONTRACT,
        contract_version: CONTRACT_VERSION,
        wire_version: crate::runtime_ipc::RUNTIME_IPC_WIRE_VERSION,
        records: files
            .iter()
            .map(|(file, record, schema_version, bytes)| ManifestEntry {
                file,
                record,
                schema_version: *schema_version,
                sha256: sha256_hex(bytes),
            })
            .collect(),
    };
    let mut all = files
        .into_iter()
        .map(|(file, _, _, bytes)| (file.to_owned(), bytes))
        .collect::<BTreeMap<_, _>>();
    all.insert("manifest.json".to_owned(), pretty(&manifest));
    all
}

fn committed() -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(fixture_root()).unwrap() {
        let path = entry.unwrap().path();
        assert!(std::fs::symlink_metadata(&path).unwrap().is_file());
        files.insert(
            path.file_name().unwrap().to_str().unwrap().to_owned(),
            std::fs::read(&path).unwrap(),
        );
    }
    files
}

fn exact<T: DeserializeOwned + Serialize>(files: &BTreeMap<String, Vec<u8>>, name: &str) -> T {
    decode_exact(&files[name]).unwrap_or_else(|| panic!("{name} decodes exactly"))
}

/// Whether one fixture's bytes decode exactly as its record.
fn decodes(name: &str, bytes: &[u8]) -> bool {
    match name {
        "run-request.json" => decode_exact::<RuntimeRunRequest>(bytes).is_some(),
        "run-recipe.json" => decode_exact::<RuntimeRecipeRequest>(bytes).is_some(),
        "run-declarations.json" | "run-declarations-absent.json" => {
            decode_exact::<RuntimeRunDeclarations>(bytes).is_some()
        }
        "job-control-request.json" => decode_exact::<JobControlRequest>(bytes).is_some(),
        "job-control.json" => decode_exact::<RuntimeJobControl>(bytes).is_some(),
        "job-status.json" => decode_exact::<RuntimeJobStatus>(bytes).is_some(),
        "ended-run-histories.json" => decode_exact::<EndedRunActionHistories>(bytes).is_some(),
        _ => panic!("unknown fixture {name}"),
    }
}

/// Every JSON pointer of an object inside `value`, the root included.
fn object_pointers(value: &serde_json::Value, pointer: &str, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(members) => {
            found.push(pointer.to_owned());
            for (name, member) in members {
                object_pointers(member, &format!("{pointer}/{name}"), found);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                object_pointers(item, &format!("{pointer}/{index}"), found);
            }
        }
        _ => {}
    }
}

#[test]
fn the_committed_fixtures_are_the_records_the_runtime_builds() {
    // Every committed file is reproduced byte for byte from the runtime's
    // own types, and nothing else is in the fixture directory.
    let built = fixture_files(&records());
    let committed = committed();
    assert_eq!(
        committed.keys().collect::<Vec<_>>(),
        built.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in &built {
        assert!(
            committed[name] == *bytes,
            "{name} differs from what the runtime builds; regenerate the fixtures"
        );
    }
    let manifest: serde_json::Value = serde_json::from_slice(&committed["manifest.json"]).unwrap();
    assert_eq!(manifest["contract"], CONTRACT);
    assert_eq!(manifest["contract_version"], CONTRACT_VERSION);
    assert_eq!(manifest["wire_version"], 18);
}

/// Every file of an earlier version, by name.
fn version_files(version: u16) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(version_root(version)).unwrap() {
        let path = entry.unwrap().path();
        files.insert(
            path.file_name().unwrap().to_str().unwrap().to_owned(),
            std::fs::read(&path).unwrap(),
        );
    }
    files
}

#[test]
fn the_third_contract_version_stays_as_it_was_on_its_older_wire() {
    // Decision 0150: version 4 changes only the wire, so version 3's records
    // still decode and verify, but its manifest names wire 17, which this
    // transport refuses, and every byte its manifest names is unchanged.
    let files = version_files(3);
    let manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest["contract_version"], 3);
    assert_eq!(manifest["wire_version"], 17);
    assert_ne!(
        manifest["wire_version"],
        crate::runtime_ipc::RUNTIME_IPC_WIRE_VERSION
    );
    let listed = manifest["records"].as_array().unwrap();
    assert_eq!(listed.len() + 1, files.len());
    let built = fixture_files(&records());
    for entry in listed {
        let name = entry["file"].as_str().unwrap();
        assert_eq!(entry["sha256"], sha256_hex(&files[name]), "{name}");
        assert_eq!(files[name], built[name], "{name}");
    }
}

#[test]
fn the_second_contract_version_stays_as_it_was_on_its_older_wire() {
    // Decision 0144: version 3 changes only the wire, so version 2's records
    // still decode and verify, but its manifest names wire 16, which this
    // transport refuses, and every byte its manifest names is unchanged.
    let files = version_files(2);
    let manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest["contract_version"], 2);
    assert_eq!(manifest["wire_version"], 16);
    assert_ne!(
        manifest["wire_version"],
        crate::runtime_ipc::RUNTIME_IPC_WIRE_VERSION
    );
    let listed = manifest["records"].as_array().unwrap();
    assert_eq!(listed.len() + 1, files.len());
    let built = fixture_files(&records());
    for entry in listed {
        let name = entry["file"].as_str().unwrap();
        assert_eq!(entry["sha256"], sha256_hex(&files[name]), "{name}");
        assert_eq!(files[name], built[name], "{name}");
    }
}

#[test]
fn the_first_contract_version_stays_as_it_was_and_reads_as_unavailable() {
    // Decision 0143: a change of a record raises the contract's version
    // beside the first one, which keeps every byte its manifest names. Its
    // run declarations are not this version's and are dropped whole.
    let files = version_files(1);
    let manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest["contract_version"], 1);
    assert_eq!(manifest["wire_version"], 15);
    let listed = manifest["records"].as_array().unwrap();
    assert_eq!(listed.len() + 1, files.len());
    for entry in listed {
        let name = entry["file"].as_str().unwrap();
        assert_eq!(entry["sha256"], sha256_hex(&files[name]), "{name}");
    }
    for name in ["run-declarations.json", "run-declarations-absent.json"] {
        assert!(!decodes(name, &files[name]), "{name}");
        // Read leniently, it is still the older schema.
        let older: RuntimeRunDeclarations = serde_json::from_slice(&files[name]).unwrap();
        assert_eq!(older.schema_version, 4);
        let request: RuntimeRunRequest = exact(&files, "run-request.json");
        assert_eq!(
            crate::cli_runtime::verified_run_declarations(older, &request, None),
            None
        );
    }
    // The records the version did not change still decode exactly.
    for name in [
        "run-request.json",
        "run-recipe.json",
        "job-control-request.json",
        "job-control.json",
        "job-status.json",
        "ended-run-histories.json",
    ] {
        assert!(decodes(name, &files[name]), "{name}");
    }
}

#[test]
fn each_fixture_decodes_exactly_and_verifies_as_a_client_verifies_it() {
    let built = records();
    let files = committed();
    let request: RuntimeRunRequest = exact(&files, "run-request.json");
    assert_eq!(request, built.request);
    assert!(verify_runtime_run_request(&request).is_ok());
    let recipe: RuntimeRecipeRequest = exact(&files, "run-recipe.json");
    assert_eq!(recipe, built.recipe);

    // Every part of the declarations is kept by the client's verification,
    // and each part verifies on its own with the function the client uses.
    let declarations: RuntimeRunDeclarations = exact(&files, "run-declarations.json");
    assert_eq!(declarations, built.declarations);
    assert_eq!(
        crate::cli_runtime::verified_run_declarations(
            declarations.clone(),
            &request,
            Some(&recipe)
        ),
        Some(declarations.clone())
    );
    assert!(
        verify_run_recoverability(
            declarations.recoverability.as_ref().unwrap(),
            request.session_id.as_str(),
            request.task.task_id.as_str(),
            request.run_id.as_str(),
        )
        .is_ok()
    );
    assert!(
        verify_session_recoverability(
            declarations.session_recoverability.as_ref().unwrap(),
            request.session_id.as_str(),
            request.task.task_id.as_str(),
        )
        .is_ok()
    );
    assert_eq!(declarations.context_inspections.as_ref().unwrap().len(), 1);
    for (history, chain) in [
        (&declarations.effect_history, RunActionChain::Effects),
        (
            &declarations.job_control_history,
            RunActionChain::JobControl,
        ),
        (&declarations.route_history, RunActionChain::Routes),
    ] {
        assert!(verify_run_action_history(history.as_ref().unwrap(), chain).is_ok());
    }
    let receipt = declarations.route_receipt.as_ref().unwrap();
    assert!(verify_run_route_receipt(receipt, &request).is_ok());
    assert!(
        verify_run_route_history(
            declarations.route_history.as_ref().unwrap(),
            &request,
            Some(receipt)
        )
        .is_ok()
    );
    assert!(verify_declared_recipe_plan(
        declarations.recipe_plan.as_ref().unwrap(),
        &request,
        Some(&recipe)
    ));

    // Absent parts stay absent: the host could not declare them completely,
    // and the client shows each as unavailable rather than partial.
    let absent: RuntimeRunDeclarations = exact(&files, "run-declarations-absent.json");
    assert_eq!(absent, built.absent);
    assert_eq!(
        crate::cli_runtime::verified_run_declarations(absent.clone(), &request, Some(&recipe)),
        Some(absent)
    );

    let control_request: JobControlRequest = exact(&files, "job-control-request.json");
    assert_eq!(control_request, built.control_request);
    let control: RuntimeJobControl = exact(&files, "job-control.json");
    assert_eq!(control, built.job_control);
    assert!(control.answers(&request, &control_request));
    let status: RuntimeJobStatus = exact(&files, "job-status.json");
    assert_eq!(status, built.job_status);
    assert!(status.describes(&request));
    assert!(status.job.revision > control.status.job.revision);

    let ended: EndedRunActionHistories = exact(&files, "ended-run-histories.json");
    assert_eq!(ended, built.ended);
    assert_eq!(
        verified_ended_run_histories(ended.clone(), request.run_id.as_str()),
        Some(ended)
    );
}

#[test]
fn a_member_the_types_do_not_name_is_refused_in_every_fixture() {
    let files = committed();
    let mut positions = 0;
    for (name, bytes) in files.iter().filter(|(name, _)| *name != "manifest.json") {
        assert!(decodes(name, bytes), "{name}");
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let mut pointers = Vec::new();
        object_pointers(&value, "", &mut pointers);
        for pointer in pointers {
            let mut extra = value.clone();
            extra.pointer_mut(&pointer).unwrap()["review"] = serde_json::Value::Bool(true);
            assert!(
                !decodes(name, &serde_json::to_vec(&extra).unwrap()),
                "{name} {pointer}"
            );
            positions += 1;
        }
    }
    assert!(positions > 100, "{positions}");
}

#[test]
fn a_repeated_member_is_refused_in_every_fixture() {
    // Decision 0136 (review F2 of `4bef629b`): a member written twice is
    // refused at every object position, inside a map as well as a struct.
    let files = committed();
    let mut positions = 0;
    for (name, bytes) in files.iter().filter(|(name, _)| *name != "manifest.json") {
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let mut pointers = Vec::new();
        object_pointers(&value, "", &mut pointers);
        for pointer in pointers {
            let members = value
                .pointer(&pointer)
                .and_then(serde_json::Value::as_object);
            if members.is_none_or(serde_json::Map::is_empty) {
                continue;
            }
            let repeated = crate::runtime_transport::with_first_member_repeated(&value, &pointer);
            assert!(!decodes(name, &repeated), "{name} {pointer}");
            positions += 1;
        }
    }
    assert!(positions > 100, "{positions}");
}

#[test]
fn a_run_request_with_sampling_values_decodes_exactly_and_verifies() {
    // Decision 0136 (review F1 of `4bef629b`): the decoding values are 32-bit
    // and written as their shortest decimal; a request carrying values that
    // are not short binary fractions decodes exactly and verifies.
    let mut request = records().request;
    request.model_profile.decoding.temperature = 0.7;
    request.model_profile.decoding.top_p = 0.95;
    request.model_profile.decoding.repeat_penalty = 1.1;
    let request = seal_runtime_run_request(request).unwrap();
    let bytes = pretty(&request);
    let text = String::from_utf8(bytes.clone()).unwrap();
    for written in [
        "\"temperature\": 0.7,",
        "\"top_p\": 0.95,",
        "\"repeat_penalty\": 1.1,",
    ] {
        assert!(text.contains(written), "{written}");
    }
    let decoded = decode_exact::<RuntimeRunRequest>(&bytes).unwrap();
    assert_eq!(decoded, request);
    assert!(verify_runtime_run_request(&decoded).is_ok());
}

#[test]
fn a_changed_record_is_refused_or_dropped_by_the_client() {
    let built = records();
    let request = &built.request;
    let recipe = Some(&built.recipe);
    // The request digest covers its constraints, the plan binding included.
    let mut changed = request.clone();
    changed.task.constraints.pop();
    assert!(verify_runtime_run_request(&changed).is_err());
    // Declarations for another request are dropped whole.
    let mut foreign = built.declarations.clone();
    foreign.request_sha256 = digest("another-request");
    assert_eq!(
        crate::cli_runtime::verified_run_declarations(foreign, request, recipe),
        None
    );
    let mut older = built.declarations.clone();
    older.schema_version = RUN_DECLARATIONS_SCHEMA_VERSION - 1;
    assert_eq!(
        crate::cli_runtime::verified_run_declarations(older, request, recipe),
        None
    );
    // A part that no longer verifies is dropped alone.
    let mut tampered = built.declarations.clone();
    tampered.effect_history.as_mut().unwrap().head.head_sha256 = digest("another-head");
    tampered.recipe_plan.as_mut().unwrap().max_changed_files += 1;
    tampered.session_recoverability.as_mut().unwrap().run_id =
        Some(request.run_id.as_str().to_owned());
    let kept = crate::cli_runtime::verified_run_declarations(tampered, request, recipe).unwrap();
    assert_eq!(kept.effect_history, None);
    assert_eq!(kept.recipe_plan, None);
    assert_eq!(kept.session_recoverability, None);
    assert_eq!(kept.recoverability, built.declarations.recoverability);
    assert_eq!(
        kept.job_control_history,
        built.declarations.job_control_history
    );
    // A plan is kept only for the recipe that was sent with the run.
    assert_eq!(
        crate::cli_runtime::verified_run_declarations(built.declarations.clone(), request, None)
            .unwrap()
            .recipe_plan,
        None
    );
    // Status and control answers describe only this run.
    let mut other = built.job_status.clone();
    other.request_sha256 = digest("another-request");
    assert!(!other.describes(request));
    let mut other_control = built.control_request.clone();
    other_control.job_id = "another-job".to_owned();
    assert!(!built.job_control.answers(request, &other_control));
    // Stored chains of another run are not read, and a chain that no longer
    // replays is dropped alone.
    assert_eq!(
        verified_ended_run_histories(built.ended.clone(), "another-run"),
        None
    );
    let mut ended = built.ended.clone();
    ended.routes.as_mut().unwrap().head.head_sha256 = digest("another-head");
    let kept = verified_ended_run_histories(ended, request.run_id.as_str()).unwrap();
    assert_eq!(kept.routes, None);
    assert_eq!(kept.effects, built.ended.effects);
}

/// Prints every fixture as one JSON object between marker lines, for
/// `scripts/runtime_producer_fixtures.py --write` after a deliberate change
/// to a record. It writes nothing itself.
#[test]
#[ignore = "run by scripts/runtime_producer_fixtures.py"]
fn print_runtime_producer_fixtures() {
    let files = fixture_files(&records())
        .into_iter()
        .map(|(name, bytes)| (name, String::from_utf8(bytes).unwrap()))
        .collect::<BTreeMap<_, _>>();
    println!("BEGIN-RUNTIME-PRODUCER-FIXTURES");
    println!("{}", serde_json::to_string(&files).unwrap());
    println!("END-RUNTIME-PRODUCER-FIXTURES");
}
