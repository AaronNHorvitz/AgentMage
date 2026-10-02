// Session declarations over stored run effect records: pure tests over
// replayed chains, and persisted recorders over a real encrypted operational
// store in a private temporary directory; no process, transport, model or
// file effect.
use std::collections::BTreeMap;

use agentmage_capability_repository_map::{StructuredArtifactClass, StructuredLanguage};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, OperationOutcome, RuntimeArtifactId, RuntimeRunId,
};
use sha2::{Digest, Sha256};

use super::*;
use crate::coding_history::seal_change_record;
use crate::coding_recoverability::{
    Recoverability, RunEffectRecorder, verify_run_recoverability, verify_session_recoverability,
};

const CALC: &[&str] = &["src", "calc.py"];
const NOTES: &[&str] = &["docs", "notes.md"];

fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn path(components: &[&str]) -> Vec<String> {
    components
        .iter()
        .map(|component| (*component).to_owned())
        .collect()
}

fn request() -> RuntimeRunRequest {
    crate::runtime_read_tests::completed_native_read_fixture().0
}

/// One sealed change record of `run_id` and its exact payload.
fn change(
    request: &RuntimeRunRequest,
    run_id: &str,
    operation: &str,
    target: &[&str],
    before: &str,
    after: &str,
) -> (RetainedCodingChange, Vec<u8>) {
    let record = seal_change_record(CodingChangeRecord {
        schema_version: 1,
        record_id: format!("change-record:{operation}"),
        session_id: request.session_id.as_str().to_owned(),
        task_id: request.task.task_id.as_str().to_owned(),
        producer_run_id: run_id.to_owned(),
        operation_id: operation.to_owned(),
        path: path(target),
        language: StructuredLanguage::Python,
        artifact_class: StructuredArtifactClass::Code,
        preimage: before.to_owned(),
        preimage_sha256: digest(before),
        postimage_sha256: digest(after),
        generated: false,
        receipt_id: format!("receipt-{operation}"),
        receipt_sha256: digest(operation),
        created_at_epoch_ms: 1,
        record_sha256: "0".repeat(64),
    })
    .unwrap();
    let bytes = serde_json::to_vec(&record).unwrap();
    let reference = RuntimeArtifactRef {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: RuntimeArtifactId::from_raw(format!("artifact-{operation}")),
        manifest_sha256: digest(&format!("manifest-{operation}")),
        payload_sha256: Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        byte_size: bytes.len() as u64,
        media_type: CODING_CHANGE_RECORD_MEDIA_TYPE.to_owned(),
    };
    (RetainedCodingChange { reference, record }, bytes)
}

fn executed(
    operation: &str,
    kind: ExecutedEffectKind,
    outcome: OperationOutcome,
) -> ExecutedEffect {
    ExecutedEffect {
        operation_id: operation.to_owned(),
        kind,
        outcome: ExecutedEffectOutcome::Completed {
            outcome,
            changed: outcome == OperationOutcome::Succeeded,
        },
    }
}

/// The canonical artifact store of these tests: exact payloads by artifact
/// and the policy each was published under.
#[derive(Default)]
struct Payloads(BTreeMap<String, (String, Vec<u8>)>);

impl Payloads {
    fn keep(&mut self, reference: &RuntimeArtifactRef, policy: &str, bytes: Vec<u8>) {
        self.0.insert(
            reference.artifact_id.as_str().to_owned(),
            (policy.to_owned(), bytes),
        );
    }

    fn read(&self, reference: &RuntimeArtifactRef, policy: &str) -> Option<Vec<u8>> {
        self.0
            .get(reference.artifact_id.as_str())
            .filter(|(kept, _)| kept == policy)
            .map(|(_, bytes)| bytes.clone())
    }
}

fn publication(change: &RetainedCodingChange) -> RunEffectEntry {
    RunEffectEntry::Publication {
        reference: change.reference.clone(),
        policy_sha256: digest("policy"),
    }
}

fn stored(
    request: &RuntimeRunRequest,
    run_id: &str,
    position: u64,
    entries: Vec<RunEffectEntry>,
    closed: bool,
) -> StoredRunEffects {
    StoredRunEffects {
        session_id: request.session_id.as_str().to_owned(),
        task_id: request.task.task_id.as_str().to_owned(),
        run_id: run_id.to_owned(),
        session_position: position,
        entry_count: entries.len() as u64,
        entries,
        head_sha256: "0".repeat(64),
        complete: true,
        closed,
    }
}

/// An earlier released run that wrote `calc.py` v0 -> v1 and created a file,
/// then the declaring run that wrote it v1 -> v2, edited the notes and ran a
/// command; every record is kept in `payloads`.
fn two_runs(request: &RuntimeRunRequest, payloads: &mut Payloads) -> Vec<StoredRunEffects> {
    let declaring = request.run_id.as_str();
    let (first, first_bytes) = change(request, "run-earlier", "w1", CALC, "v0\n", "v1\n");
    let (second, second_bytes) = change(request, declaring, "w2", CALC, "v1\n", "v2\n");
    let (notes, notes_bytes) = change(request, declaring, "w3", NOTES, "n0\n", "n1\n");
    for (retained, bytes) in [
        (&first, first_bytes),
        (&second, second_bytes),
        (&notes, notes_bytes),
    ] {
        payloads.keep(&retained.reference, &digest("policy"), bytes);
    }
    vec![
        stored(
            request,
            "run-earlier",
            1,
            vec![
                execution_entry(&executed(
                    "w1",
                    ExecutedEffectKind::Write,
                    OperationOutcome::Succeeded,
                )),
                publication(&first),
                execution_entry(&executed(
                    "c1",
                    ExecutedEffectKind::Create {
                        path: path(&["src", "new.py"]),
                    },
                    OperationOutcome::Succeeded,
                )),
                execution_entry(&executed(
                    "r1",
                    ExecutedEffectKind::NoEffect,
                    OperationOutcome::Succeeded,
                )),
            ],
            true,
        ),
        stored(
            request,
            declaring,
            2,
            vec![
                execution_entry(&executed(
                    "w2",
                    ExecutedEffectKind::Write,
                    OperationOutcome::Succeeded,
                )),
                publication(&second),
                execution_entry(&executed(
                    "w3",
                    ExecutedEffectKind::Write,
                    OperationOutcome::Succeeded,
                )),
                publication(&notes),
                execution_entry(&ExecutedEffect {
                    operation_id: "t1".to_owned(),
                    kind: ExecutedEffectKind::Command,
                    outcome: ExecutedEffectOutcome::Unknown,
                }),
            ],
            false,
        ),
    ]
}

fn classes(report: &RecoverabilityReport) -> Vec<(&str, Recoverability, &str)> {
    report
        .assessments
        .iter()
        .map(|assessment| {
            (
                assessment.operation_id.as_str(),
                assessment.recoverability,
                assessment.reason_code.as_str(),
            )
        })
        .collect()
}

fn declare(
    runs: &[StoredRunEffects],
    request: &RuntimeRunRequest,
    payloads: &Payloads,
    current: &BTreeMap<Vec<String>, String>,
) -> Result<RecoverabilityReport, RecoverabilityError> {
    declare_session_recoverability(
        runs,
        request,
        &|run, reference, policy| {
            assert_eq!(run.session_id, request.session_id.as_str());
            payloads.read(reference, policy)
        },
        &|target| current.get(target).cloned(),
    )
}

#[test]
fn a_session_is_declared_from_every_run_oldest_first_and_names_no_run() {
    let request = request();
    let mut payloads = Payloads::default();
    let runs = two_runs(&request, &mut payloads);
    let current = BTreeMap::from([(path(CALC), digest("v2\n")), (path(NOTES), digest("n1\n"))]);
    let report = declare(&runs, &request, &payloads, &current).unwrap();
    assert_eq!(report.run_id, None);
    assert_eq!(
        classes(&report),
        [
            (
                "w1",
                Recoverability::Recoverable,
                "inverse-write-restores-preimage"
            ),
            (
                "c1",
                Recoverability::NotRecoverable,
                "create-has-no-admitted-inverse"
            ),
            (
                "w2",
                Recoverability::Recoverable,
                "inverse-write-restores-preimage"
            ),
            (
                "w3",
                Recoverability::Recoverable,
                "inverse-write-restores-preimage"
            ),
            (
                "t1",
                Recoverability::ExternalOrUncertain,
                "effect-outcome-uncertain"
            ),
        ]
    );
    // The writes of both runs to one path chain newest first across runs.
    assert_eq!(report.revert_order, ["w3", "w2", "w1"]);
    assert!(report.requires_reconciliation && !report.fully_recoverable);
    verify_session_recoverability(
        &report,
        request.session_id.as_str(),
        request.task.task_id.as_str(),
    )
    .unwrap();
    assert!(
        verify_run_recoverability(
            &report,
            request.session_id.as_str(),
            request.task.task_id.as_str(),
            request.run_id.as_str(),
        )
        .is_err()
    );
    // A person's later edit of the file blocks both runs' writes to it.
    let edited = BTreeMap::from([
        (path(CALC), digest("edited\n")),
        (path(NOTES), digest("n1\n")),
    ]);
    let report = declare(&runs, &request, &payloads, &edited).unwrap();
    assert_eq!(
        classes(&report)
            .into_iter()
            .filter(|(_, class, _)| *class == Recoverability::Conflict)
            .map(|(operation, _, reason)| (operation, reason))
            .collect::<Vec<_>>(),
        [
            ("w1", "newer-conflict-blocks-older"),
            ("w2", "current-bytes-differ")
        ]
    );
    // A session of only the declaring run is declared as well.
    let alone = vec![stored(
        &request,
        request.run_id.as_str(),
        1,
        Vec::new(),
        false,
    )];
    let report = declare(&alone, &request, &payloads, &current).unwrap();
    assert!(report.assessments.is_empty() && report.fully_recoverable);
}

#[test]
fn a_session_with_any_unreleased_incomplete_or_foreign_run_is_never_declared() {
    let request = request();
    let mut payloads = Payloads::default();
    let runs = two_runs(&request, &mut payloads);
    let current = BTreeMap::from([(path(CALC), digest("v2\n"))]);
    assert!(declare(&runs, &request, &payloads, &current).is_ok());
    type Mutation = fn(&mut Vec<StoredRunEffects>);
    let cases: [(&str, Mutation); 10] = [
        ("no run", |runs| runs.clear()),
        ("declaring run missing", |runs| {
            runs.pop();
        }),
        ("a later run after the declaring one", |runs| {
            let mut later = runs[1].clone();
            later.run_id = "run-later".to_owned();
            later.session_position = 3;
            later.entries.clear();
            runs[1].closed = true;
            runs.push(later);
        }),
        ("declaring run released", |runs| runs[1].closed = true),
        ("earlier run never released", |runs| runs[0].closed = false),
        ("earlier run incomplete", |runs| runs[0].complete = false),
        ("declaring run incomplete", |runs| runs[1].complete = false),
        ("another session", |runs| {
            runs[0].session_id = "session-other".to_owned();
        }),
        ("another task", |runs| {
            runs[1].task_id = "task-other".to_owned();
        }),
        ("a position gap", |runs| runs[1].session_position = 3),
    ];
    for (name, mutate) in cases {
        let mut mutated = runs.clone();
        mutate(&mut mutated);
        assert_eq!(
            declare(&mutated, &request, &payloads, &current).err(),
            Some(RecoverabilityError::Invalid),
            "{name}"
        );
    }
}

#[test]
fn each_change_record_must_read_back_as_that_runs_own_verified_record() {
    let request = request();
    let mut payloads = Payloads::default();
    let runs = two_runs(&request, &mut payloads);
    let current = BTreeMap::from([(path(CALC), digest("v2\n"))]);
    let RunEffectEntry::Publication { reference, .. } = runs[0].entries[1].clone() else {
        unreachable!("the second entry is a publication")
    };
    // A record that cannot be read, or only under another policy revision.
    let mut missing = Payloads::default();
    for run in &runs {
        for entry in &run.entries {
            if let RunEffectEntry::Publication {
                reference: kept, ..
            } = entry
                && kept != &reference
            {
                missing.0.insert(
                    kept.artifact_id.as_str().to_owned(),
                    payloads.0[kept.artifact_id.as_str()].clone(),
                );
            }
        }
    }
    assert_eq!(
        declare(&runs, &request, &missing, &current).err(),
        Some(RecoverabilityError::Invalid)
    );
    let mut other_policy = runs.clone();
    other_policy[0].entries[1] = RunEffectEntry::Publication {
        reference: reference.clone(),
        policy_sha256: digest("another policy"),
    };
    assert_eq!(
        declare(&other_policy, &request, &payloads, &current).err(),
        Some(RecoverabilityError::Invalid)
    );
    // A record whose bytes changed, one another run published, and one
    // recorded under another media type are refused.
    let mut tampered = Payloads(payloads.0.clone());
    let entry = tampered.0.get_mut(reference.artifact_id.as_str()).unwrap();
    entry.1 = String::from_utf8(entry.1.clone())
        .unwrap()
        .replace("v0\\n", "v9\\n")
        .into_bytes();
    assert_eq!(
        declare(&runs, &request, &tampered, &current).err(),
        Some(RecoverabilityError::ForeignOrInvalid)
    );
    let mut swapped = runs.clone();
    swapped[0].entries.swap(0, 2);
    let RunEffectEntry::Publication {
        reference: declaring_record,
        ..
    } = runs[1].entries[1].clone()
    else {
        unreachable!("the second entry is a publication")
    };
    swapped[0].entries[1] = RunEffectEntry::Publication {
        reference: declaring_record,
        policy_sha256: digest("policy"),
    };
    assert_eq!(
        declare(&swapped, &request, &payloads, &current).err(),
        Some(RecoverabilityError::ForeignOrInvalid)
    );
    let mut other_media = runs.clone();
    other_media[0].entries[1] = RunEffectEntry::Publication {
        reference: RuntimeArtifactRef {
            media_type: "text/plain".to_owned(),
            ..reference.clone()
        },
        policy_sha256: digest("policy"),
    };
    assert_eq!(
        declare(&other_media, &request, &payloads, &current).err(),
        Some(RecoverabilityError::ForeignOrInvalid)
    );
    // A record over the read bound, or a session over its total, is not read.
    for byte_size in [
        MAX_CHANGE_RECORD_READ_BYTES + 1,
        MAX_SESSION_CHANGE_RECORD_BYTES,
        u64::MAX,
    ] {
        let mut oversized = runs.clone();
        oversized[0].entries[1] = RunEffectEntry::Publication {
            reference: RuntimeArtifactRef {
                byte_size,
                ..reference.clone()
            },
            policy_sha256: digest("policy"),
        };
        assert_eq!(
            declare(&oversized, &request, &payloads, &current).err(),
            Some(RecoverabilityError::Invalid),
            "{byte_size}"
        );
    }
    // The same operation in two runs is refused.
    let mut repeated = runs.clone();
    repeated[1].entries.push(runs[0].entries[2].clone());
    assert_eq!(
        declare(&repeated, &request, &payloads, &current).err(),
        Some(RecoverabilityError::Duplicate)
    );
    // A run's changed write without its record is uncertain, and a record
    // without its write refuses the session, as for a run.
    let mut unrecorded = runs.clone();
    unrecorded[0].entries.remove(1);
    let report = declare(&unrecorded, &request, &payloads, &current).unwrap();
    assert_eq!(
        classes(&report)[0],
        (
            "w1",
            Recoverability::ExternalOrUncertain,
            "effect-outcome-uncertain"
        )
    );
    let mut unexecuted = runs;
    unexecuted[0].entries.remove(0);
    assert_eq!(
        declare(&unexecuted, &request, &payloads, &current).err(),
        Some(RecoverabilityError::ForeignOrInvalid)
    );
}

#[test]
fn each_execution_maps_to_its_stored_entry_and_back() {
    let effects = [
        executed("w", ExecutedEffectKind::Write, OperationOutcome::Succeeded),
        executed(
            "c",
            ExecutedEffectKind::Create { path: path(CALC) },
            OperationOutcome::Failed,
        ),
        executed("t", ExecutedEffectKind::Command, OperationOutcome::Denied),
        executed(
            "r",
            ExecutedEffectKind::NoEffect,
            OperationOutcome::Uncertain,
        ),
        ExecutedEffect {
            operation_id: "u".to_owned(),
            kind: ExecutedEffectKind::Write,
            outcome: ExecutedEffectOutcome::Unknown,
        },
    ];
    for effect in effects {
        let entry = execution_entry(&effect);
        assert_eq!(executed_effect(&entry), Some(effect));
    }
    assert_eq!(
        execution_entry(&ExecutedEffect {
            operation_id: "u".to_owned(),
            kind: ExecutedEffectKind::Command,
            outcome: ExecutedEffectOutcome::Unknown,
        }),
        RunEffectEntry::Execution {
            operation_id: "u".to_owned(),
            kind: RunEffectKind::Command,
            path: None,
            outcome: None,
            changed: false,
        }
    );
    // An entry outside the closed shape maps to nothing.
    for entry in [
        RunEffectEntry::Execution {
            operation_id: "x".to_owned(),
            kind: RunEffectKind::Create,
            path: None,
            outcome: Some(OperationOutcome::Succeeded),
            changed: true,
        },
        RunEffectEntry::Execution {
            operation_id: "x".to_owned(),
            kind: RunEffectKind::Write,
            path: Some(path(CALC)),
            outcome: Some(OperationOutcome::Succeeded),
            changed: true,
        },
        RunEffectEntry::Execution {
            operation_id: "x".to_owned(),
            kind: RunEffectKind::Write,
            path: None,
            outcome: None,
            changed: true,
        },
    ] {
        assert_eq!(executed_effect(&entry), None);
    }
}

mod persisted {
    use super::*;
    use crate::runtime_start_tests::JobLedgerStore;

    fn run_request(base: &RuntimeRunRequest, run_id: &str) -> RuntimeRunRequest {
        RuntimeRunRequest {
            run_id: RuntimeRunId::from_raw(run_id),
            ..base.clone()
        }
    }

    #[test]
    fn a_persisted_recorder_stores_each_kept_record_and_the_session_declares_it_later() {
        let store = JobLedgerStore::new("session-effects-persisted");
        let request = request();
        let earlier = run_request(&request, "run-earlier");
        let runtime = store.try_runtime().unwrap();
        let records = runtime.run_effect_records();

        // The earlier run: a write and its record, then a create.
        let (first, first_bytes) = change(&request, "run-earlier", "w1", CALC, "v0\n", "v1\n");
        let mut recorder = RunEffectRecorder::persisted(
            PersistedRunEffects::begin(records.clone(), &earlier, StoredChainStart::New).unwrap(),
        );
        recorder.record_execution(executed(
            "w1",
            ExecutedEffectKind::Write,
            OperationOutcome::Succeeded,
        ));
        recorder.record_publication(&first.reference, &digest("policy"), &first_bytes);
        recorder.record_execution(executed(
            "c1",
            ExecutedEffectKind::Create {
                path: path(&["src", "new.py"]),
            },
            OperationOutcome::Succeeded,
        ));
        assert!(recorder.stored_chain().is_some());
        let stored = records.run("run-earlier").unwrap();
        assert_eq!(
            stored.entries,
            [
                execution_entry(&executed(
                    "w1",
                    ExecutedEffectKind::Write,
                    OperationOutcome::Succeeded
                )),
                publication(&first),
                execution_entry(&executed(
                    "c1",
                    ExecutedEffectKind::Create {
                        path: path(&["src", "new.py"]),
                    },
                    OperationOutcome::Succeeded
                )),
            ]
        );
        assert!(stored.complete && !stored.closed);
        drop(recorder);
        records
            .close("run-earlier", RUN_EFFECT_RECORD_OWNER)
            .unwrap();
        drop((records, runtime));

        // A later host process: the declaring run of the same session.
        let runtime = store.try_runtime().unwrap();
        let records = runtime.run_effect_records();
        let (second, second_bytes) = change(
            &request,
            request.run_id.as_str(),
            "w2",
            CALC,
            "v1\n",
            "v2\n",
        );
        let mut recorder = RunEffectRecorder::persisted(
            PersistedRunEffects::begin(records.clone(), &request, StoredChainStart::New).unwrap(),
        );
        recorder.record_execution(executed(
            "w2",
            ExecutedEffectKind::Write,
            OperationOutcome::Succeeded,
        ));
        recorder.record_publication(&second.reference, &digest("policy"), &second_bytes);
        let chain = recorder.stored_chain().unwrap();
        let runs = chain.session(request.session_id.as_str()).unwrap();
        assert_eq!(
            runs.iter()
                .map(|run| (run.run_id.as_str(), run.session_position, run.closed))
                .collect::<Vec<_>>(),
            [
                ("run-earlier", 1, true),
                (request.run_id.as_str(), 2, false)
            ]
        );
        let mut payloads = Payloads::default();
        payloads.keep(&first.reference, &digest("policy"), first_bytes);
        payloads.keep(&second.reference, &digest("policy"), second_bytes);
        let current = BTreeMap::from([(path(CALC), digest("v2\n"))]);
        let report = declare(&runs, &request, &payloads, &current).unwrap();
        assert_eq!(report.revert_order, ["w2", "w1"]);
        assert_eq!(
            classes(&report)[1],
            (
                "c1",
                Recoverability::NotRecoverable,
                "create-has-no-admitted-inverse"
            )
        );
    }

    #[test]
    fn a_restart_or_a_missed_record_leaves_the_session_undeclared() {
        let store = JobLedgerStore::new("session-effects-restart");
        let request = request();
        let runtime = store.try_runtime().unwrap();
        let records = runtime.run_effect_records();
        let begin = |start| PersistedRunEffects::begin(records.clone(), &request, start);
        // Nothing to continue or resume in this host yet.
        assert_eq!(
            begin(StoredChainStart::ContinueInHost).err(),
            Some(RunEffectRecordStoreError::NotFound)
        );
        let chain = begin(StoredChainStart::New).unwrap();
        assert_eq!(
            begin(StoredChainStart::New).err(),
            Some(RunEffectRecordStoreError::Exists)
        );
        // Continuing in this host keeps the record complete.
        let continued = begin(StoredChainStart::ContinueInHost).unwrap();
        assert!(continued.kept_every_entry());
        assert!(records.run(request.run_id.as_str()).unwrap().complete);
        // Resuming after a restart marks it incomplete for good, so the
        // session is never declared from it.
        drop((chain, continued));
        let resumed = begin(StoredChainStart::AfterRestart).unwrap();
        let runs = resumed.session(request.session_id.as_str()).unwrap();
        assert!(!runs[0].complete);
        assert_eq!(
            declare(&runs, &request, &Payloads::default(), &BTreeMap::new()).err(),
            Some(RecoverabilityError::Invalid)
        );
        // A run recorded before schema 25 begins here, marked incomplete.
        let older = run_request(&request, "run-before-schema-25");
        PersistedRunEffects::begin(records.clone(), &older, StoredChainStart::AfterRestart)
            .unwrap();
        assert!(!records.run("run-before-schema-25").unwrap().complete);
        // A chain of another session is never attached for this run.
        let foreign = RuntimeRunRequest {
            session_id: agentmage_kernel_contracts::SessionId::from_raw("session-other"),
            ..older
        };
        assert_eq!(
            PersistedRunEffects::begin(records.clone(), &foreign, StoredChainStart::ContinueInHost)
                .err(),
            Some(RunEffectRecordStoreError::InvalidInput)
        );

        // A record the store refuses leaves the run's own declaration intact
        // but withholds the chain from any session declaration.
        let later = run_request(&request, "run-store-refused");
        let mut recorder = RunEffectRecorder::persisted(
            PersistedRunEffects::begin(records.clone(), &later, StoredChainStart::New).unwrap(),
        );
        records
            .close("run-store-refused", RUN_EFFECT_RECORD_OWNER)
            .unwrap();
        recorder.record_execution(executed(
            "t1",
            ExecutedEffectKind::Command,
            OperationOutcome::Failed,
        ));
        assert!(recorder.stored_chain().is_none());
        assert_eq!(
            recorder
                .declare(
                    later.session_id.as_str(),
                    later.task.task_id.as_str(),
                    later.run_id.as_str(),
                    &|_| None
                )
                .unwrap()
                .assessments
                .len(),
            1
        );
        // A record the memory cannot keep marks the stored chain incomplete.
        let full = run_request(&request, "run-memory-full");
        let mut recorder = RunEffectRecorder::persisted(
            PersistedRunEffects::begin(records.clone(), &full, StoredChainStart::New).unwrap(),
        );
        let (unverified, _) = change(&request, "run-memory-full", "w9", CALC, "a\n", "b\n");
        recorder.record_publication(&unverified.reference, &digest("policy"), b"{}");
        assert!(recorder.stored_chain().is_none());
        let stored = records.run("run-memory-full").unwrap();
        assert!(!stored.complete && stored.entries.is_empty());
    }
}
