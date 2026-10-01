// Real encrypted operational stores in private temporary directories and a
// hand-written synthetic documentation pack; no process, transport, network or
// model.
use std::collections::BTreeMap;

use agentmage_capability_knowledge::{
    DocPackFile, DocPackManifest, DocPackMediaType, seal_doc_pack_manifest,
};
use agentmage_kernel_engine::operational_store::DurableAuthorityRuntime;

use super::*;
use crate::coding_doc_packs::{
    DocPackAnswer, DocPackOwner, DocPackSource, doc_pack_import_requests,
};
use crate::runtime_start_tests::JobLedgerStore;

const LICENSE: &str = "LicenseRef-sample";
const V1: DocPackVersion = DocPackVersion {
    major: 1,
    minor: 0,
    patch: 0,
};
// 2026-01-02T01:02:03Z
const NOW: u64 = 1_767_312_000_000 + 3_723_000;
const CACHE: &str = "# Build cache\n\nThe cache lives in build/cache.\n";
const NOTES: &str = "Offline notes about the cache.\n";
const SOURCE: &str = "doc-pack:build-tool-guide:1.0.0";

fn guide() -> DocPackSource {
    let files = [("guide/cache.md", CACHE), ("notes.txt", NOTES)]
        .into_iter()
        .map(|(path, text)| (path.to_owned(), text.to_owned()))
        .collect::<BTreeMap<_, _>>();
    let manifest = seal_doc_pack_manifest(DocPackManifest {
        schema_version: 1,
        pack_id: "build-tool-guide".to_owned(),
        version: V1,
        title: "Build tool guide".to_owned(),
        publisher: "Synthetic sample".to_owned(),
        license: LICENSE.to_owned(),
        retrieved_on: "2026-01-01".to_owned(),
        fresh_for_days: 30,
        files: files
            .iter()
            .map(|(path, text)| DocPackFile {
                path: path.clone(),
                media_type: if path.ends_with(".md") {
                    DocPackMediaType::Markdown
                } else {
                    DocPackMediaType::PlainText
                },
                byte_len: text.len() as u64,
                sha256: sha256_hex(text.as_bytes()),
            })
            .collect(),
        total_bytes: files.values().map(|text| text.len() as u64).sum(),
        manifest_sha256: String::new(),
    })
    .unwrap();
    DocPackSource { manifest, files }
}

fn cite(path: &str) -> MemoryCitation {
    MemoryCitation {
        pack_id: "build-tool-guide".to_owned(),
        version: V1,
        path: path.to_owned(),
    }
}

fn remember(content: &str, workspace: &str, path: &str) -> MemoryRequest {
    MemoryRequest::Remember {
        content: content.to_owned(),
        memory_type: MemoryTypeView::Semantic,
        workspace_id: workspace.to_owned(),
        citation: cite(path),
    }
}

/// The memory owner over one real store holding the sample pack, as the
/// catalog host opens it; it counts how often the store was opened.
struct Harness {
    store: JobLedgerStore,
    runtime: Option<DurableAuthorityRuntime>,
    opened: usize,
    identities: u64,
}

impl Harness {
    fn new(tag: &str) -> Self {
        let store = JobLedgerStore::new(tag);
        let runtime = store.try_runtime();
        let catalog = runtime.as_ref().unwrap().doc_pack_catalog();
        let mut owner = DocPackOwner::default();
        let answers = doc_pack_import_requests(&guide(), &[LICENSE.to_owned()], None)
            .into_iter()
            .map(|request| owner.answer(request, &mut || Ok(catalog.clone()), Some("2026-01-02")))
            .collect::<Vec<_>>();
        assert!(matches!(
            answers.last(),
            Some(DocPackAnswer::Imported { .. })
        ));
        Self {
            store,
            runtime,
            opened: 0,
            identities: 0,
        }
    }

    fn answer(&mut self, request: &MemoryRequest) -> MemoryAnswer {
        self.identities += 1;
        let runtime = self.runtime.as_ref().unwrap();
        let opened = &mut self.opened;
        answer_memory(
            request,
            &mut || {
                *opened += 1;
                Ok(MemoryStores {
                    states: runtime.owner_states(),
                    doc_packs: runtime.doc_pack_catalog(),
                })
            },
            Some(NOW),
            Some(format!("memory-{:016x}", self.identities)),
        )
    }

    fn stored(&self) -> agentmage_kernel_engine::owner_state_store::StoredOwnerState {
        self.runtime
            .as_ref()
            .unwrap()
            .owner_states()
            .load(OwnerStateName::MemoryCatalog)
            .unwrap()
    }

    /// Closes the store and opens it again, as the next catalog operation
    /// does.
    fn reopen(&mut self) {
        self.runtime = None;
        self.runtime = self.store.try_runtime();
        assert!(self.runtime.is_some());
    }

    fn remembered(&mut self, request: &MemoryRequest) -> MemoryItemView {
        match self.answer(request) {
            MemoryAnswer::Remembered { item, receipt } => {
                assert!(memory_answer_acknowledges(
                    request,
                    &MemoryAnswer::Remembered {
                        item: item.clone(),
                        receipt
                    }
                ));
                item
            }
            other => panic!("remembered: {other:?}"),
        }
    }

    fn listed(&mut self, workspace_id: Option<&str>) -> Vec<MemoryItemView> {
        match self.answer(&MemoryRequest::List {
            workspace_id: workspace_id.map(str::to_owned),
        }) {
            MemoryAnswer::Listed { items, .. } => items,
            other => panic!("listed: {other:?}"),
        }
    }
}

fn refused(answer: &MemoryAnswer) -> Option<MemoryRefusal> {
    match answer {
        MemoryAnswer::Refused { refusal } => Some(*refusal),
        _ => None,
    }
}

#[test]
fn a_remembered_item_cites_a_kept_file_and_survives_reopening() {
    // Decision 0131: the person's invocation approves one eligible candidate
    // citing a kept pack file; the catalog is committed under the revision
    // read and decodes again after the store reopens.
    let mut harness = Harness::new("memory-remember");
    let request = remember(
        "The cache lives in build/cache.",
        "workspace-a",
        "guide/cache.md",
    );
    let answer = harness.answer(&request);
    let MemoryAnswer::Remembered { item, receipt } = &answer else {
        panic!("remembered: {answer:?}");
    };
    assert!(memory_answer_acknowledges(&request, &answer));
    assert_eq!(item.memory_id, "memory-0000000000000001");
    assert_eq!(item.memory_type, Some(MemoryTypeView::Semantic));
    assert_eq!(item.status, MemoryStatusView::Approved);
    assert_eq!(item.workspace_id, "workspace-a");
    assert_eq!(
        item.content.as_deref(),
        Some("The cache lives in build/cache.")
    );
    assert_eq!(
        item.sources,
        vec![MemorySourceView {
            source_id: SOURCE.to_owned(),
            object_id: "path:guide:cache.md".to_owned(),
            content_sha256: sha256_hex(CACHE.as_bytes()),
        }]
    );
    assert_eq!(
        (item.created_at.as_str(), item.decided_at.as_str()),
        ("2026-01-02T01:02:03Z", "2026-01-02T01:02:03Z")
    );
    assert_eq!(receipt.catalog_revision, 1);
    assert_eq!(
        Some(&receipt.decision_sha256),
        memory_decision_sha256(&request).as_ref()
    );
    assert_eq!(harness.opened, 1);
    let stored = harness.stored();
    assert_eq!(stored.revision, 1);
    let catalog = decode_memory_catalog_state(&stored.state).unwrap();
    assert_eq!(catalog.inspect().catalog_sha256, receipt.catalog_sha256);
    let kept = catalog.items();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].evidence[0].kind, EvidenceKind::Document);
    assert_eq!(kept[0].sensitivity, DataSensitivity::Durable);
    harness.reopen();
    let items = harness.listed(None);
    assert_eq!(items, vec![item.clone()]);
    // A second item commits the next revision.
    let notes = remember("Notes are offline.", "workspace-a", "notes.txt");
    let second = harness.remembered(&notes);
    assert_eq!(second.sources[0].object_id, "path:notes.txt");
    assert_eq!(harness.stored().revision, 2);
    assert_eq!(harness.listed(Some("workspace-a")).len(), 2);
    assert!(harness.listed(Some("workspace-b")).is_empty());
}

#[test]
fn revoking_a_source_reaches_only_the_named_workspace() {
    let mut harness = Harness::new("memory-revoke-source");
    let a_cache = harness.remembered(&remember("Cache in A.", "workspace-a", "guide/cache.md"));
    let a_notes = harness.remembered(&remember("Notes in A.", "workspace-a", "notes.txt"));
    let b_cache = harness.remembered(&remember("Cache in B.", "workspace-b", "guide/cache.md"));
    // One object of the source, in one workspace.
    let object = MemoryRequest::RevokeSource {
        workspace_id: "workspace-a".to_owned(),
        source_id: SOURCE.to_owned(),
        object_id: Some("path:notes.txt".to_owned()),
    };
    let answer = harness.answer(&object);
    assert!(memory_answer_acknowledges(&object, &answer));
    let MemoryAnswer::Revoked { receipt } = answer else {
        panic!("revoked");
    };
    assert_eq!(receipt.memory_ids, vec![a_notes.memory_id.clone()]);
    assert_eq!(receipt.catalog_revision, 4);
    // The whole source in the same workspace reaches what is left there.
    let source = MemoryRequest::RevokeSource {
        workspace_id: "workspace-a".to_owned(),
        source_id: SOURCE.to_owned(),
        object_id: None,
    };
    let MemoryAnswer::Revoked { receipt } = harness.answer(&source) else {
        panic!("revoked");
    };
    assert_eq!(receipt.memory_ids, vec![a_cache.memory_id.clone()]);
    // Nothing is left to revoke there; another workspace with no item is
    // not found either. Neither changes the stored catalog.
    let revision = harness.stored().revision;
    assert_eq!(
        refused(&harness.answer(&source)),
        Some(MemoryRefusal::NotFound)
    );
    let elsewhere = MemoryRequest::RevokeSource {
        workspace_id: "workspace-c".to_owned(),
        source_id: SOURCE.to_owned(),
        object_id: None,
    };
    assert_eq!(
        refused(&harness.answer(&elsewhere)),
        Some(MemoryRefusal::NotFound)
    );
    assert_eq!(harness.stored().revision, revision);
    // The other workspace's item citing the same file is untouched; the
    // revoked items keep their text for inspection.
    let statuses = harness
        .listed(None)
        .into_iter()
        .map(|item| (item.memory_id, item.status, item.content.is_some()))
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        vec![
            (a_cache.memory_id.clone(), MemoryStatusView::Revoked, true),
            (a_notes.memory_id.clone(), MemoryStatusView::Revoked, true),
            (b_cache.memory_id.clone(), MemoryStatusView::Approved, true),
        ]
    );
    // One item by identity, once.
    let revoke = MemoryRequest::Revoke {
        memory_id: b_cache.memory_id.clone(),
    };
    let answer = harness.answer(&revoke);
    assert!(memory_answer_acknowledges(&revoke, &answer));
    assert_eq!(
        refused(&harness.answer(&revoke)),
        Some(MemoryRefusal::InvalidTransition)
    );
    // Deletion keeps a tombstone without the text, once.
    let delete = MemoryRequest::Delete {
        memory_id: a_cache.memory_id.clone(),
    };
    let answer = harness.answer(&delete);
    assert!(memory_answer_acknowledges(&delete, &answer));
    assert_eq!(
        refused(&harness.answer(&delete)),
        Some(MemoryRefusal::InvalidTransition)
    );
    let unknown = MemoryRequest::Delete {
        memory_id: "memory-ffffffffffffffff".to_owned(),
    };
    assert_eq!(
        refused(&harness.answer(&unknown)),
        Some(MemoryRefusal::NotFound)
    );
    harness.reopen();
    let deleted = harness
        .listed(Some("workspace-a"))
        .into_iter()
        .find(|item| item.memory_id == a_cache.memory_id)
        .unwrap();
    assert_eq!(
        (deleted.status, deleted.content),
        (MemoryStatusView::Deleted, None)
    );
}

#[test]
fn every_refusal_changes_nothing() {
    let mut harness = Harness::new("memory-refusals");
    harness.remembered(&remember("Cache.", "workspace-a", "guide/cache.md"));
    let stored = harness.stored();
    // Malformed requests are refused before the store opens.
    for request in [
        remember("Cache.", "Workspace A", "guide/cache.md"),
        remember("   ", "workspace-a", "guide/cache.md"),
        remember("line\nbreak", "workspace-a", "guide/cache.md"),
        remember(
            &"x".repeat(MAX_CONTENT_BYTES + 1),
            "workspace-a",
            "guide/cache.md",
        ),
        MemoryRequest::Remember {
            content: "Cache.".to_owned(),
            memory_type: MemoryTypeView::Semantic,
            workspace_id: "workspace-a".to_owned(),
            citation: MemoryCitation {
                pack_id: "Build".to_owned(),
                version: V1,
                path: "guide/cache.md".to_owned(),
            },
        },
        MemoryRequest::List {
            workspace_id: Some(String::new()),
        },
        MemoryRequest::Revoke {
            memory_id: "Memory-1".to_owned(),
        },
        MemoryRequest::Delete {
            memory_id: "item-1".to_owned(),
        },
        MemoryRequest::RevokeSource {
            workspace_id: "workspace-a".to_owned(),
            source_id: SOURCE.to_owned(),
            object_id: Some("path:Guide".to_owned()),
        },
        MemoryRequest::RevokeSource {
            workspace_id: "workspace-a".to_owned(),
            source_id: "doc pack".to_owned(),
            object_id: None,
        },
    ] {
        let opened = harness.opened;
        assert_eq!(
            refused(&harness.answer(&request)),
            Some(MemoryRefusal::InvalidInput),
            "{request:?}"
        );
        assert_eq!(harness.opened, opened, "{request:?}");
    }
    // Citations the catalog cannot resolve.
    for (path, refusal) in [
        ("guide/missing.md", MemoryRefusal::CitationNotFound),
        ("Guide/Cache.md", MemoryRefusal::CitationNotPortable),
        ("guide/../notes.txt", MemoryRefusal::CitationNotPortable),
        ("guide//cache.md", MemoryRefusal::CitationNotPortable),
    ] {
        assert_eq!(
            refused(&harness.answer(&remember("Cache.", "workspace-a", path))),
            Some(refusal),
            "{path}"
        );
    }
    let mut other_version = remember("Cache.", "workspace-a", "guide/cache.md");
    if let MemoryRequest::Remember { citation, .. } = &mut other_version {
        citation.version.minor = 1;
    }
    assert_eq!(
        refused(&harness.answer(&other_version)),
        Some(MemoryRefusal::CitationNotFound)
    );
    // The policy and the portable rules refuse credentials and machine paths.
    for content in [
        "The deploy password=hunter2 is shared.",
        "Keys are in /Users/example/keys.",
    ] {
        assert_eq!(
            refused(&harness.answer(&remember(content, "workspace-a", "guide/cache.md"))),
            Some(MemoryRefusal::Candidate(MemoryCandidateRefusal::Prohibited)),
            "{content}"
        );
    }
    assert_eq!(harness.stored(), stored);
    // No clock, no identity and no store each refuse.
    let request = remember("Cache.", "workspace-a", "guide/cache.md");
    let runtime = harness.runtime.as_ref().unwrap();
    let stores = || {
        Ok(MemoryStores {
            states: runtime.owner_states(),
            doc_packs: runtime.doc_pack_catalog(),
        })
    };
    assert_eq!(
        answer_memory(
            &request,
            &mut stores.clone(),
            None,
            Some("memory-1".to_owned())
        ),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::ClockUnavailable
        }
    );
    assert_eq!(
        answer_memory(
            &request,
            &mut stores.clone(),
            Some(u64::MAX),
            Some("memory-1".to_owned())
        ),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::ClockUnavailable
        }
    );
    for identity in [None, Some("item-1".to_owned())] {
        assert_eq!(
            answer_memory(&request, &mut stores.clone(), Some(NOW), identity),
            MemoryAnswer::Refused {
                refusal: MemoryRefusal::StoreUnavailable
            }
        );
    }
    assert_eq!(
        answer_memory(
            &request,
            &mut || Err(MemoryRefusal::StoreUnavailable),
            Some(NOW),
            Some("memory-1".to_owned())
        ),
        MemoryAnswer::Refused {
            refusal: MemoryRefusal::StoreUnavailable
        }
    );
    assert_eq!(harness.stored(), stored);
}

#[test]
fn a_stored_catalog_that_does_not_decode_is_refused() {
    let mut harness = Harness::new("memory-integrity");
    harness
        .runtime
        .as_ref()
        .unwrap()
        .owner_states()
        .commit(OwnerStateName::MemoryCatalog, 0, b"not a catalog")
        .unwrap();
    for request in [
        MemoryRequest::List { workspace_id: None },
        remember("Cache.", "workspace-a", "guide/cache.md"),
    ] {
        assert_eq!(
            refused(&harness.answer(&request)),
            Some(MemoryRefusal::StoreIntegrity)
        );
    }
    assert_eq!(harness.stored().state, b"not a catalog");
    // Store failures map to closed refusals.
    for (error, refusal) in [
        (OwnerStateStoreError::Stale, MemoryRefusal::StoreConflict),
        (
            OwnerStateStoreError::Integrity,
            MemoryRefusal::StoreIntegrity,
        ),
        (
            OwnerStateStoreError::ResourceLimit,
            MemoryRefusal::ResourceLimit,
        ),
        (
            OwnerStateStoreError::Storage,
            MemoryRefusal::StoreUnavailable,
        ),
        (
            OwnerStateStoreError::Unavailable,
            MemoryRefusal::StoreUnavailable,
        ),
        (
            OwnerStateStoreError::InvalidInput,
            MemoryRefusal::StoreUnavailable,
        ),
    ] {
        assert_eq!(MemoryRefusal::of_store(error), refusal);
    }
}

#[test]
fn requests_and_answers_are_parsed_closed() {
    let requests = [
        remember("Cache.", "workspace-a", "guide/cache.md"),
        MemoryRequest::List { workspace_id: None },
        MemoryRequest::Revoke {
            memory_id: "memory-1".to_owned(),
        },
        MemoryRequest::RevokeSource {
            workspace_id: "workspace-a".to_owned(),
            source_id: SOURCE.to_owned(),
            object_id: None,
        },
        MemoryRequest::Delete {
            memory_id: "memory-1".to_owned(),
        },
    ];
    for request in &requests {
        let text = serde_json::to_string(request).unwrap();
        assert_eq!(
            serde_json::from_str::<MemoryRequest>(&text).unwrap(),
            *request
        );
        let extra = text.replacen('{', "{\"x\":1,", 1);
        assert!(
            serde_json::from_str::<MemoryRequest>(&extra).is_err(),
            "{extra}"
        );
    }
    for text in [
        // A required optional member must be present.
        r#"{"operation":"list"}"#,
        r#"{"operation":"revoke-source","workspace_id":"w","source_id":"s"}"#,
        // Unknown operations and types.
        r#"{"operation":"forget","memory_id":"memory-1"}"#,
        r#"{"operation":"remember","content":"c","memory_type":"working","workspace_id":"w","citation":{"pack_id":"p","version":{"major":1,"minor":0,"patch":0},"path":"a.md"}}"#,
        r#"{"operation":"remember","content":"c","memory_type":"semantic","workspace_id":"w","citation":{"pack_id":"p","version":{"major":1,"minor":0,"patch":0},"path":"a.md","x":1}}"#,
    ] {
        assert!(
            serde_json::from_str::<MemoryRequest>(text).is_err(),
            "{text}"
        );
    }
    let refusal = MemoryAnswer::Refused {
        refusal: MemoryRefusal::Candidate(MemoryCandidateRefusal::Prohibited),
    };
    let text = serde_json::to_string(&refusal).unwrap();
    assert_eq!(
        serde_json::from_str::<MemoryAnswer>(&text).unwrap(),
        refusal
    );
    assert!(serde_json::from_str::<MemoryAnswer>(&text.replace("prohibited", "secret")).is_err());
}

#[test]
fn only_an_answer_to_the_sent_request_is_acknowledged() {
    let mut harness = Harness::new("memory-acknowledge");
    let request = remember("Cache.", "workspace-a", "guide/cache.md");
    let answer = harness.answer(&request);
    let MemoryAnswer::Remembered { item, receipt } = answer.clone() else {
        panic!("remembered");
    };
    assert!(memory_answer_acknowledges(&request, &answer));
    // Another request's decision, item text, workspace or kind is refused.
    let other = remember("Other.", "workspace-a", "guide/cache.md");
    assert!(!memory_answer_acknowledges(&other, &answer));
    for changed in [
        MemoryItemView {
            workspace_id: "workspace-b".to_owned(),
            ..item.clone()
        },
        MemoryItemView {
            memory_type: Some(MemoryTypeView::Episodic),
            ..item.clone()
        },
        MemoryItemView {
            status: MemoryStatusView::Candidate,
            ..item.clone()
        },
        MemoryItemView {
            sources: Vec::new(),
            ..item.clone()
        },
        MemoryItemView {
            memory_id: "memory-2".to_owned(),
            ..item.clone()
        },
    ] {
        assert!(!memory_answer_acknowledges(
            &request,
            &MemoryAnswer::Remembered {
                item: changed,
                receipt: receipt.clone()
            }
        ));
    }
    let list = MemoryRequest::List {
        workspace_id: Some("workspace-b".to_owned()),
    };
    assert!(!memory_answer_acknowledges(
        &list,
        &MemoryAnswer::Listed {
            items: vec![item.clone()],
            catalog_revision: 1
        }
    ));
    let revoke = MemoryRequest::Revoke {
        memory_id: item.memory_id.clone(),
    };
    let revoked = |memory_ids: Vec<String>, decision_sha256: String| MemoryAnswer::Revoked {
        receipt: MemoryTransitionView {
            memory_ids,
            decision_sha256,
            ..receipt.clone()
        },
    };
    let decision = memory_decision_sha256(&revoke).unwrap();
    assert!(memory_answer_acknowledges(
        &revoke,
        &revoked(vec![item.memory_id.clone()], decision.clone())
    ));
    for answer in [
        revoked(vec!["memory-2".to_owned()], decision.clone()),
        revoked(Vec::new(), decision.clone()),
        revoked(vec![item.memory_id.clone()], "0".repeat(64)),
        MemoryAnswer::Deleted {
            receipt: receipt.clone(),
        },
        answer.clone(),
    ] {
        assert!(!memory_answer_acknowledges(&revoke, &answer));
    }
    let source = MemoryRequest::RevokeSource {
        workspace_id: "workspace-a".to_owned(),
        source_id: SOURCE.to_owned(),
        object_id: None,
    };
    let source_decision = memory_decision_sha256(&source).unwrap();
    assert!(!memory_answer_acknowledges(
        &source,
        &revoked(Vec::new(), source_decision)
    ));
}

#[test]
fn citations_paths_and_times_parse_exactly() {
    assert_eq!(
        parse_memory_citation("build-tool-guide@1.0.0:guide/cache.md"),
        Some(cite("guide/cache.md"))
    );
    for value in [
        "build-tool-guide@1.0:guide/cache.md",
        "build-tool-guide:guide/cache.md",
        "build-tool-guide@1.0.0:",
        "Build@1.0.0:a.md",
        "@1.0.0:a.md",
    ] {
        assert_eq!(parse_memory_citation(value), None, "{value}");
    }
    assert_eq!(
        doc_pack_object_id("guide/cache.md").as_deref(),
        Some("path:guide:cache.md")
    );
    for path in ["Guide/a.md", "a//b", "../a", "./a", "a b", "a/", ""] {
        assert_eq!(doc_pack_object_id(path), None, "{path:?}");
    }
    assert_eq!(doc_pack_object_id(&"a".repeat(252)), None);
    assert_eq!(
        doc_pack_object_id(&"a".repeat(251)).map(|id| id.len()),
        Some(256)
    );
    assert_eq!(doc_pack_source_id("build-tool-guide", V1), SOURCE);
    assert_eq!(
        memory_timestamp(NOW).as_deref(),
        Some("2026-01-02T01:02:03Z")
    );
    assert_eq!(
        memory_timestamp(1_767_312_000_000 - 1).as_deref(),
        Some("2026-01-01T23:59:59Z")
    );
    assert_eq!(memory_timestamp(u64::MAX), None);
    for (value, parsed) in [
        ("semantic", Some(MemoryTypeView::Semantic)),
        ("preference", Some(MemoryTypeView::Preference)),
        ("procedural", Some(MemoryTypeView::Procedural)),
        ("episodic", Some(MemoryTypeView::Episodic)),
        ("working", None),
        ("Semantic", None),
    ] {
        assert_eq!(MemoryTypeView::parse(value), parsed, "{value}");
    }
    assert!(portable_memory_label(&"a".repeat(128)));
    assert!(!portable_memory_label(&"a".repeat(129)));
    assert!(portable_memory_object(&"a".repeat(256)));
    assert!(!portable_memory_object(&"a".repeat(257)));
}

#[test]
fn answers_render_as_escaped_text_lines_or_json_rows() {
    let item = MemoryItemView {
        memory_id: "memory-1".to_owned(),
        memory_type: Some(MemoryTypeView::Preference),
        workspace_id: "workspace-a".to_owned(),
        status: MemoryStatusView::Approved,
        content: Some("Prefer \u{1b}[31mred".to_owned()),
        sources: vec![MemorySourceView {
            source_id: SOURCE.to_owned(),
            object_id: "path:notes.txt".to_owned(),
            content_sha256: "c".repeat(64),
        }],
        created_at: "2026-01-02T01:02:03Z".to_owned(),
        decided_at: "2026-01-02T01:02:03Z".to_owned(),
    };
    let listed = MemoryAnswer::Listed {
        items: vec![
            item.clone(),
            MemoryItemView {
                memory_id: "memory-2".to_owned(),
                status: MemoryStatusView::Deleted,
                content: None,
                ..item.clone()
            },
        ],
        catalog_revision: 3,
    };
    let text = render_memory_answer(&listed, false);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("memory memory-1 [approved] preference in workspace-a"));
    assert!(text.contains("  | Prefer \\u{1b}[31mred\n"));
    assert!(text.contains("  | (no text kept)\n"));
    assert!(text.contains(&format!("  cites {SOURCE} path:notes.txt (cccccccccccc)\n")));
    assert!(text.ends_with("memory: 2 items; catalog revision 3\n"));
    let rows = render_memory_answer(&listed, true);
    let rows = rows
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["type"], "memory_item");
    assert_eq!(rows[0]["item"]["content"], "Prefer \u{1b}[31mred");
    assert_eq!(rows[2]["type"], "memory_list");
    assert_eq!(rows[2]["items"], 2);
    let receipt = MemoryTransitionView {
        catalog_revision: 4,
        catalog_sha256: "a".repeat(64),
        decision_sha256: "b".repeat(64),
        memory_ids: vec!["memory-1".to_owned(), "memory-2".to_owned()],
    };
    assert_eq!(
        render_memory_answer(
            &MemoryAnswer::Revoked {
                receipt: receipt.clone()
            },
            false
        ),
        "memory revoked: memory-1, memory-2; catalog revision 4; decision bbbbbbbbbbbb\n"
    );
    let deleted = render_memory_answer(&MemoryAnswer::Deleted { receipt }, true);
    assert!(deleted.starts_with("{\"receipt\""));
    assert!(deleted.contains("\"type\":\"memory_deleted\""));
    for (refusal, code, exit) in [
        (
            MemoryRefusal::InvalidInput,
            "memory.invalid-input",
            crate::headless::ClientExitCode::InvalidInput,
        ),
        (
            MemoryRefusal::CitationNotPortable,
            "memory.citation-not-portable",
            crate::headless::ClientExitCode::InvalidInput,
        ),
        (
            MemoryRefusal::Candidate(MemoryCandidateRefusal::Prohibited),
            "memory.candidate.prohibited",
            crate::headless::ClientExitCode::PolicyDenied,
        ),
        (
            MemoryRefusal::CitationNotFound,
            "memory.citation-not-found",
            crate::headless::ClientExitCode::PolicyDenied,
        ),
        (
            MemoryRefusal::NotFound,
            "memory.not-found",
            crate::headless::ClientExitCode::PolicyDenied,
        ),
        (
            MemoryRefusal::InvalidTransition,
            "memory.invalid-transition",
            crate::headless::ClientExitCode::PolicyDenied,
        ),
        (
            MemoryRefusal::ResourceLimit,
            "memory.resource-limit",
            crate::headless::ClientExitCode::ResourceBound,
        ),
        (
            MemoryRefusal::ClockUnavailable,
            "memory.clock-unavailable",
            crate::headless::ClientExitCode::ServiceUnavailable,
        ),
        (
            MemoryRefusal::StoreConflict,
            "memory.store-conflict",
            crate::headless::ClientExitCode::ServiceUnavailable,
        ),
        (
            MemoryRefusal::StoreIntegrity,
            "memory.store-integrity",
            crate::headless::ClientExitCode::ServiceUnavailable,
        ),
    ] {
        assert_eq!(refusal.code(), code);
        assert_eq!(memory_refusal_exit(refusal), exit, "{code}");
        assert_eq!(
            render_memory_refusal(refusal, false),
            format!("memory refused: {code}\n")
        );
        assert_eq!(
            render_memory_refusal(refusal, true),
            format!("{{\"code\":\"{code}\",\"type\":\"memory_refused\"}}\n")
        );
    }
    assert_eq!(
        render_memory_answer(
            &MemoryAnswer::Refused {
                refusal: MemoryRefusal::NotFound
            },
            false
        ),
        ""
    );
}
