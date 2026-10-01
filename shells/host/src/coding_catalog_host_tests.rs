// The catalog service over a real encrypted operational store in a private
// temporary directory; no process, transport, network or model.
use std::cell::Cell;
use std::rc::Rc;

use agentmage_kernel_engine::action_history::{
    ActionAuthorization, ActionKind, ActionOutcome, ActionRecordDraft,
};
use agentmage_kernel_engine::run_action_history_store::RunActionChainName;

use super::*;
use crate::coding_action_history::RUN_ACTION_HISTORY_OWNER;
use crate::coding_doc_packs::DocPackRefusal;
use crate::runtime_start_tests::JobLedgerStore;

/// The fixture store, opened afresh for each operation, with a settable
/// clock; it counts how often the service opened it.
struct FixtureStore {
    store: Rc<JobLedgerStore>,
    now_epoch_ms: Option<u64>,
    opened: Rc<Cell<usize>>,
    refuse_open: bool,
    identities: u64,
}

impl CatalogStore for FixtureStore {
    fn open(&mut self) -> Result<CatalogStoreHandles, RuntimeTransportError> {
        if self.refuse_open {
            return Err(RuntimeTransportError::RuntimeFailed);
        }
        let runtime = self
            .store
            .try_runtime()
            .ok_or(RuntimeTransportError::RuntimeFailed)?;
        self.opened.set(self.opened.get() + 1);
        Ok(CatalogStoreHandles::new(
            runtime.doc_pack_catalog(),
            runtime.run_action_histories(),
            runtime.owner_states(),
            Box::new(runtime),
        ))
    }

    fn now_epoch_ms(&self) -> Option<u64> {
        self.now_epoch_ms
    }

    fn new_memory_id(&mut self) -> Option<String> {
        self.identities += 1;
        Some(format!("memory-{:016x}", self.identities))
    }
}

fn service(
    tag: &str,
) -> (
    CodingCatalogService<FixtureStore>,
    Rc<JobLedgerStore>,
    Rc<Cell<usize>>,
) {
    let store = Rc::new(JobLedgerStore::new(tag));
    let opened = Rc::new(Cell::new(0));
    (
        CodingCatalogService::new(FixtureStore {
            store: Rc::clone(&store),
            // 2026-01-02
            now_epoch_ms: Some(1_767_312_000_000),
            opened: Rc::clone(&opened),
            refuse_open: false,
            identities: 0,
        }),
        store,
        opened,
    )
}

#[test]
fn the_catalog_host_refuses_every_run_operation() {
    let (mut catalog, _store, opened) = service("catalog-runs");
    let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
    let run = &request.run_id;
    let sha = request.request_sha256.as_str();
    assert_eq!(
        catalog
            .prepare(RuntimePrepareInput {
                resume: false,
                record_session: false,
                slow_subscriber_probe: false,
                preauthorization: None,
                engineering_session_id: None,
                profile_id: request.model_profile.profile_id.as_str().to_owned(),
                expected_entry_sha256: "a".repeat(64),
                workspace_id: request.workspace_id.as_str().to_owned(),
                workspace_root: "/tmp/agentmage-catalog-fixture".to_owned(),
                prompt: request.task.objective.clone(),
                recipe: None,
            })
            .err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        catalog.start(request.clone()).err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        catalog.advance(run, sha, None, None).err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        catalog
            .cancel(
                run,
                sha,
                agentmage_kernel_contracts::CancellationId::from_raw("cancel"),
                None
            )
            .err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        catalog.run_declarations(run, sha).err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    assert_eq!(
        catalog.job_status(run, sha).err(),
        Some(RuntimeTransportError::JobControlUnavailable)
    );
    assert_eq!(
        catalog.release(run, sha),
        Err(RuntimeTransportError::RequestDenied)
    );
    // None of them opened the store.
    assert_eq!(opened.get(), 0);
}

#[test]
fn each_documentation_pack_operation_opens_the_store_and_closes_it_again() {
    let (mut catalog, store, opened) = service("catalog-doc-packs");
    let listed = catalog.doc_pack(DocPackRequest::List {}).unwrap();
    assert_eq!(
        listed,
        DocPackAnswer::Listed {
            packs: Vec::new(),
            retention: Vec::new()
        }
    );
    assert_eq!(opened.get(), 1);
    // The store admits one connection; it opens here, so the service holds
    // none between operations.
    drop(store.try_runtime().expect("the service closed the store"));
    // A staged import needs no store until it commits.
    let staged = catalog
        .doc_pack(DocPackRequest::ImportChunk {
            path: "notes.txt".to_owned(),
            offset: 0,
            text: "x".to_owned(),
        })
        .unwrap();
    assert_eq!(
        staged,
        DocPackAnswer::Refused {
            refusal: DocPackRefusal::NotStaged
        }
    );
    assert_eq!(opened.get(), 1);
    drop(store.try_runtime().expect("still closed"));
}

#[test]
fn documentation_packs_need_the_host_clock_and_store() {
    let (mut catalog, _store, opened) = service("catalog-unavailable");
    catalog.store.now_epoch_ms = None;
    assert_eq!(
        catalog.doc_pack(DocPackRequest::List {}).unwrap(),
        DocPackAnswer::Refused {
            refusal: DocPackRefusal::ClockUnavailable
        }
    );
    // A clock past the last representable date has no day either.
    catalog.store.now_epoch_ms = Some(u64::MAX);
    assert_eq!(
        catalog.doc_pack(DocPackRequest::List {}).unwrap(),
        DocPackAnswer::Refused {
            refusal: DocPackRefusal::ClockUnavailable
        }
    );
    catalog.store.now_epoch_ms = Some(1_767_312_000_000);
    catalog.store.refuse_open = true;
    assert_eq!(
        catalog.doc_pack(DocPackRequest::List {}).unwrap(),
        DocPackAnswer::Refused {
            refusal: DocPackRefusal::StoreUnavailable
        }
    );
    assert_eq!(opened.get(), 0);
}

#[test]
fn an_ended_runs_stored_chains_are_read_with_retention_applied() {
    // Decisions 0129 and 0130: the catalog host reads an ended run's chains,
    // applying retention to a closed chain first; a chain it does not hold is
    // absent, and an open chain is shown as open.
    let (mut catalog, store, opened) = service("catalog-ended-run");
    {
        let runtime = store.try_runtime().unwrap();
        let histories = runtime.run_action_histories();
        let draft = |action_kind: ActionKind, at: u64| ActionRecordDraft {
            action_kind,
            action_id: format!("action-{at}"),
            authorization: ActionAuthorization::PersonDecision {
                decision_sha256: "7".repeat(64),
            },
            effect_sha256: "8".repeat(64),
            outcome: ActionOutcome::Denied,
            reason_code: "job.control.cancel.stale-revision".to_owned(),
            evidence_sha256s: vec!["9".repeat(64)],
            recorded_at_epoch_ms: at,
            retain_until_epoch_ms: at + 1_000,
        };
        for (chain, kind, close) in [
            (RunActionChainName::JobControl, ActionKind::JobControl, true),
            (RunActionChainName::Effects, ActionKind::CommandRun, false),
        ] {
            histories
                .create("run-ended", chain, RUN_ACTION_HISTORY_OWNER)
                .unwrap();
            histories
                .append(
                    "run-ended",
                    chain,
                    RUN_ACTION_HISTORY_OWNER,
                    &draft(kind, 10),
                )
                .unwrap();
            if close {
                histories
                    .close("run-ended", chain, RUN_ACTION_HISTORY_OWNER)
                    .unwrap();
            }
        }
    }
    let ended = catalog
        .ended_run_action_histories(&RuntimeRunId::from_raw("run-ended"))
        .unwrap();
    assert_eq!(opened.get(), 1);
    assert_eq!(ended.run_id, "run-ended");
    assert_eq!(ended.routes, None);
    // The closed chain's only entry passed its retention: only its place
    // remains. The open chain keeps its entry.
    let job_control = ended.job_control.clone().unwrap();
    assert!(job_control.closed && job_control.complete);
    assert!(matches!(
        job_control.records.as_slice(),
        [agentmage_kernel_engine::action_history::ActionHistoryRecord::Expired { .. }]
    ));
    let effects = ended.effects.clone().unwrap();
    assert!(!effects.closed);
    assert!(matches!(
        effects.records.as_slice(),
        [agentmage_kernel_engine::action_history::ActionHistoryRecord::Kept(_)]
    ));
    assert_eq!(
        crate::coding_action_history::verified_ended_run_histories(ended.clone(), "run-ended"),
        Some(ended)
    );
    drop(store.try_runtime().expect("the service closed the store"));
    // A run with nothing stored has no chain; a malformed run identity is
    // refused; no clock or no store fails.
    let nothing = catalog
        .ended_run_action_histories(&RuntimeRunId::from_raw("run-unknown"))
        .unwrap();
    assert_eq!(
        (nothing.effects, nothing.job_control, nothing.routes),
        (None, None, None)
    );
    assert_eq!(
        catalog
            .ended_run_action_histories(&RuntimeRunId::from_raw("r".repeat(129)))
            .err(),
        Some(RuntimeTransportError::RequestDenied)
    );
    catalog.store.now_epoch_ms = None;
    assert_eq!(
        catalog
            .ended_run_action_histories(&RuntimeRunId::from_raw("run-ended"))
            .err(),
        Some(RuntimeTransportError::RuntimeFailed)
    );
    catalog.store.now_epoch_ms = Some(1_767_312_000_000);
    catalog.store.refuse_open = true;
    assert_eq!(
        catalog
            .ended_run_action_histories(&RuntimeRunId::from_raw("run-ended"))
            .err(),
        Some(RuntimeTransportError::RuntimeFailed)
    );
}

#[test]
fn each_memory_operation_opens_the_store_once_and_closes_it_again() {
    // Decision 0131: the catalog host answers memory over its own store,
    // opened for the operation and closed afterwards; only a remembered item
    // draws an identity.
    use crate::coding_memory::{MemoryAnswer, MemoryRefusal, MemoryRequest};
    let (mut catalog, store, opened) = service("catalog-memory");
    assert_eq!(
        catalog
            .memory(MemoryRequest::List { workspace_id: None })
            .unwrap(),
        MemoryAnswer::Listed {
            items: Vec::new(),
            catalog_revision: 0
        }
    );
    assert_eq!(opened.get(), 1);
    drop(store.try_runtime().expect("the service closed the store"));
    assert_eq!(
        catalog
            .memory(MemoryRequest::Revoke {
                memory_id: "memory-0123456789abcdef".to_owned()
            })
            .unwrap(),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::NotFound
        }
    );
    assert_eq!((opened.get(), catalog.store.identities), (2, 0));
    // A citation of a pack the catalog does not keep draws an identity and
    // stores nothing.
    let remember = MemoryRequest::Remember {
        content: "The cache lives in build/cache.".to_owned(),
        memory_type: crate::coding_memory::MemoryTypeView::Semantic,
        workspace_id: "workspace-a".to_owned(),
        citation: crate::coding_memory::MemoryCitation {
            pack_id: "build-tool-guide".to_owned(),
            version: agentmage_capability_knowledge::DocPackVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            path: "guide/cache.md".to_owned(),
        },
    };
    assert_eq!(
        catalog.memory(remember.clone()).unwrap(),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::CitationNotFound
        }
    );
    assert_eq!((opened.get(), catalog.store.identities), (3, 1));
    drop(store.try_runtime().expect("still closed"));
    // No clock refuses before the store opens; a store that does not open is
    // unavailable.
    catalog.store.now_epoch_ms = None;
    assert_eq!(
        catalog.memory(remember.clone()).unwrap(),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::ClockUnavailable
        }
    );
    catalog.store.now_epoch_ms = Some(1_767_312_000_000);
    catalog.store.refuse_open = true;
    assert_eq!(
        catalog.memory(remember).unwrap(),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::StoreUnavailable
        }
    );
    assert_eq!(opened.get(), 3);
}

#[test]
fn each_extension_operation_opens_the_store_once_and_closes_it_again() {
    // Decision 0132: the catalog host answers extension requests over its own
    // store, opened for the operation and closed afterwards; only an
    // installation needs the clock.
    use crate::coding_extensions::{
        ExtensionAnswer, ExtensionKeyRole, ExtensionRefusal, ExtensionRequest,
        ExtensionTrustStatement,
    };
    let (mut catalog, store, opened) = service("catalog-extension");
    assert_eq!(
        catalog
            .extension(ExtensionRequest::List { workspace_id: None })
            .unwrap(),
        ExtensionAnswer::Listed {
            scopes: Vec::new(),
            catalog_revision: 0
        }
    );
    assert_eq!(opened.get(), 1);
    drop(store.try_runtime().expect("the service closed the store"));
    let trust = ExtensionRequest::Trust {
        workspace_id: "workspace-a".to_owned(),
        statement: ExtensionTrustStatement {
            role: ExtensionKeyRole::RevocationIssuer,
            key_id: "agentmage-sample-issuer".to_owned(),
            public_key: ed25519_dalek::SigningKey::from_bytes(&[0x1d; 32])
                .verifying_key()
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        },
    };
    // Without a clock a key is still trusted; nothing but an installation
    // reads it.
    catalog.store.now_epoch_ms = None;
    assert!(matches!(
        catalog.extension(trust.clone()).unwrap(),
        ExtensionAnswer::Trusted { .. }
    ));
    assert_eq!(opened.get(), 2);
    drop(store.try_runtime().expect("still closed"));
    catalog.store.now_epoch_ms = Some(1_767_312_000_000);
    catalog.store.refuse_open = true;
    assert_eq!(
        catalog.extension(trust).unwrap(),
        ExtensionAnswer::Refused {
            refusal: ExtensionRefusal::StoreUnavailable
        }
    );
    assert_eq!(opened.get(), 2);
}
