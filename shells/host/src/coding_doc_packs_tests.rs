// Real encrypted operational stores in private temporary directories and
// hand-written synthetic packs; no process, transport, network or model.
use std::collections::BTreeMap;

use agentmage_capability_knowledge::{
    DocPackFile, DocPackImportIntent, DocPackMediaType, seal_doc_pack_manifest,
};

use super::*;
use crate::runtime_start_tests::JobLedgerStore;

const LICENSE: &str = "LicenseRef-sample";
const V1: DocPackVersion = DocPackVersion {
    major: 1,
    minor: 0,
    patch: 0,
};
const V2: DocPackVersion = DocPackVersion {
    major: 1,
    minor: 1,
    patch: 0,
};

fn source(
    pack_id: &str,
    version: DocPackVersion,
    retrieved_on: &str,
    files: &[(&str, String)],
) -> DocPackSource {
    let files = files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), text.clone()))
        .collect::<BTreeMap<_, _>>();
    let manifest = seal_doc_pack_manifest(DocPackManifest {
        schema_version: 1,
        pack_id: pack_id.to_owned(),
        version,
        title: "Build tool guide".to_owned(),
        publisher: "Synthetic sample".to_owned(),
        license: LICENSE.to_owned(),
        retrieved_on: retrieved_on.to_owned(),
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

fn guide(version: DocPackVersion, retrieved_on: &str, paragraph: &str) -> DocPackSource {
    source(
        "build-tool-guide",
        version,
        retrieved_on,
        &[
            ("guide/cache.md", format!("# Build cache\n\n{paragraph}\n")),
            ("notes.txt", "Offline notes about the cache.\n".to_owned()),
        ],
    )
}

/// An owner over one real store, as the catalog host opens it.
struct Harness {
    store: JobLedgerStore,
    runtime: Option<agentmage_kernel_engine::operational_store::DurableAuthorityRuntime>,
    owner: DocPackOwner,
}

impl Harness {
    fn new(tag: &str) -> Self {
        let store = JobLedgerStore::new(tag);
        let runtime = store.try_runtime();
        assert!(runtime.is_some());
        Self {
            store,
            runtime,
            owner: DocPackOwner::default(),
        }
    }

    fn answer(&mut self, request: DocPackRequest, today: &str) -> DocPackAnswer {
        let catalog = self.runtime.as_ref().unwrap().doc_pack_catalog();
        self.owner
            .answer(request, &mut || Ok(catalog.clone()), Some(today))
    }

    fn import(
        &mut self,
        source: &DocPackSource,
        refresh: Option<DocPackVersion>,
        today: &str,
    ) -> Vec<DocPackAnswer> {
        doc_pack_import_requests(source, &[LICENSE.to_owned()], refresh)
            .into_iter()
            .map(|request| self.answer(request, today))
            .collect()
    }

    fn revision(&self) -> u64 {
        self.runtime
            .as_ref()
            .unwrap()
            .doc_pack_catalog()
            .load()
            .unwrap()
            .revision
    }

    /// Closes the store and opens it again, as the next catalog operation
    /// does.
    fn reopen(&mut self) {
        self.runtime = None;
        self.runtime = self.store.try_runtime();
        assert!(self.runtime.is_some());
    }
}

fn refused(answer: &DocPackAnswer) -> Option<DocPackRefusal> {
    match answer {
        DocPackAnswer::Refused { refusal } => Some(*refusal),
        _ => None,
    }
}

#[test]
fn an_import_arrives_in_chunks_and_commits_once_every_file_matches() {
    // Decision 0130: a staged manifest, each file's text in chunks cut at
    // character boundaries, and one commit that the store keeps.
    let mut harness = Harness::new("doc-pack-import");
    let long = "\u{e9}t\u{e9} ".repeat(300_000);
    let pack = source(
        "build-tool-guide",
        V1,
        "2026-01-01",
        &[
            (
                "guide/cache.md",
                "# Build cache\n\nThe cache keeps compiled units.\n".to_owned(),
            ),
            ("guide/long.txt", long.clone()),
        ],
    );
    let requests = doc_pack_import_requests(&pack, &[LICENSE.to_owned()], None);
    // A begin, one chunk for the short file, two for the long one, a commit.
    assert_eq!(requests.len(), 5);
    let mut rebuilt = BTreeMap::<String, String>::new();
    for request in &requests[1..requests.len() - 1] {
        let DocPackRequest::ImportChunk { path, offset, text } = request else {
            panic!("chunks between the begin and the commit");
        };
        let file = rebuilt.entry(path.clone()).or_default();
        assert_eq!(*offset, file.len() as u64);
        assert!(!text.is_empty() && text.len() <= MAX_DOC_PACK_CHUNK_BYTES);
        file.push_str(text);
    }
    assert_eq!(rebuilt, pack.files);
    let answers = harness.import(&pack, None, "2026-01-02");
    assert_eq!(
        answers[0],
        DocPackAnswer::ImportStaged {
            pack_id: "build-tool-guide".to_owned(),
            version: V1,
            manifest_sha256: pack.manifest.manifest_sha256.clone(),
        }
    );
    assert_eq!(
        answers[3],
        DocPackAnswer::ChunkAccepted {
            path: "guide/long.txt".to_owned(),
            received_bytes: long.len() as u64,
        }
    );
    let DocPackAnswer::Imported { receipt, retention } = &answers[4] else {
        panic!("imported: {:?}", answers[4]);
    };
    assert!(retention.is_empty());
    assert!(verify_doc_pack_receipt(receipt, &pack.manifest));
    assert_eq!((receipt.file_count, receipt.superseded), (2, None));
    // The receipt is the knowledge component's, byte for byte.
    let mut direct = DocPackCatalog::new();
    let expected = direct
        .import(
            &policy(BTreeSet::from([LICENSE.to_owned()])),
            &pack.manifest,
            &pack
                .files
                .iter()
                .map(|(path, text)| (path.clone(), text.as_bytes().to_vec()))
                .collect(),
            DocPackImportIntent::New,
            "2026-01-02",
        )
        .unwrap();
    assert_eq!(*receipt, receipt_view(&expected));
    assert_eq!(harness.revision(), 1);
    // The kept catalog is restored from the store by the next operation.
    harness.reopen();
    let DocPackAnswer::Listed { packs, retention } =
        harness.answer(DocPackRequest::List {}, "2026-01-03")
    else {
        panic!("listed");
    };
    assert!(retention.is_empty());
    assert_eq!(packs.len(), 1);
    assert_eq!(
        (packs[0].current, packs[0].versions[0].fresh),
        (Some(V1), true)
    );
    // Reading changes nothing in the store.
    assert_eq!(harness.revision(), 1);
}

#[test]
fn a_staged_import_takes_only_chunks_that_continue_a_listed_file() {
    let mut harness = Harness::new("doc-pack-staging");
    let pack = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let begin = |licenses: Vec<String>, manifest: DocPackManifest| DocPackRequest::ImportBegin {
        manifest,
        allowed_licenses: licenses,
        refresh: None,
    };
    let chunk = |path: &str, offset: u64, text: &str| DocPackRequest::ImportChunk {
        path: path.to_owned(),
        offset,
        text: text.to_owned(),
    };
    let today = "2026-01-02";
    // Nothing is staged yet.
    for request in [chunk("notes.txt", 0, "x"), DocPackRequest::ImportCommit {}] {
        assert_eq!(
            refused(&harness.answer(request, today)),
            Some(DocPackRefusal::NotStaged)
        );
    }
    // The licenses and the seal are checked when the import is staged.
    let mut unsealed = pack.manifest.clone();
    unsealed.title = "Another title".to_owned();
    for (request, refusal) in [
        (
            begin(vec!["MIT".to_owned()], pack.manifest.clone()),
            DocPackRefusal::LicenseNotAllowed,
        ),
        (
            begin(Vec::new(), pack.manifest.clone()),
            DocPackRefusal::PolicyInvalid,
        ),
        (
            begin(
                vec![LICENSE.to_owned(), LICENSE.to_owned()],
                pack.manifest.clone(),
            ),
            DocPackRefusal::PolicyInvalid,
        ),
        (
            begin(
                (0..=MAX_DOC_PACK_ALLOWED_LICENSES)
                    .map(|index| format!("License-{index}"))
                    .collect(),
                pack.manifest.clone(),
            ),
            DocPackRefusal::PolicyInvalid,
        ),
        (
            begin(vec![LICENSE.to_owned()], unsealed),
            DocPackRefusal::ManifestInvalid,
        ),
    ] {
        assert_eq!(refused(&harness.answer(request, today)), Some(refusal));
        assert_eq!(
            refused(&harness.answer(DocPackRequest::ImportCommit {}, today)),
            Some(DocPackRefusal::NotStaged)
        );
    }
    // Each malformed chunk is refused and discards the staged import.
    let notes = "Offline notes about the cache.\n";
    for bad in [
        chunk("missing.txt", 0, "x"),
        chunk("notes.txt", 1, "ffline"),
        chunk("notes.txt", 0, ""),
        chunk("notes.txt", 0, &format!("{notes}more")),
        chunk("notes.txt", 0, &"x".repeat(MAX_DOC_PACK_CHUNK_BYTES + 1)),
    ] {
        assert!(matches!(
            harness.answer(
                begin(vec![LICENSE.to_owned()], pack.manifest.clone()),
                today
            ),
            DocPackAnswer::ImportStaged { .. }
        ));
        assert_eq!(
            refused(&harness.answer(bad, today)),
            Some(DocPackRefusal::ChunkInvalid)
        );
        assert_eq!(
            refused(&harness.answer(chunk("notes.txt", 0, notes), today)),
            Some(DocPackRefusal::NotStaged)
        );
    }
    // A refused begin discards an import staged before it (Decision 0132).
    assert!(matches!(
        harness.answer(
            begin(vec![LICENSE.to_owned()], pack.manifest.clone()),
            today
        ),
        DocPackAnswer::ImportStaged { .. }
    ));
    assert!(matches!(
        harness.answer(chunk("notes.txt", 0, "Offline"), today),
        DocPackAnswer::ChunkAccepted { .. }
    ));
    assert_eq!(
        refused(&harness.answer(begin(vec!["MIT".to_owned()], pack.manifest.clone()), today)),
        Some(DocPackRefusal::LicenseNotAllowed)
    );
    assert_eq!(
        refused(&harness.answer(DocPackRequest::ImportCommit {}, today)),
        Some(DocPackRefusal::NotStaged)
    );
    // The host bounds a chunk itself, even inside a file long enough to take
    // it (Decision 0132).
    let large = source(
        "large-guide",
        V1,
        "2026-01-01",
        &[("large.txt", "x".repeat(MAX_DOC_PACK_CHUNK_BYTES + 1))],
    );
    assert!(matches!(
        harness.answer(
            begin(vec![LICENSE.to_owned()], large.manifest.clone()),
            today
        ),
        DocPackAnswer::ImportStaged { .. }
    ));
    assert_eq!(
        refused(&harness.answer(
            chunk("large.txt", 0, &"x".repeat(MAX_DOC_PACK_CHUNK_BYTES + 1)),
            today
        )),
        Some(DocPackRefusal::ChunkInvalid)
    );
    assert_eq!(
        refused(&harness.answer(chunk("large.txt", 0, "x"), today)),
        Some(DocPackRefusal::NotStaged)
    );
    // A commit before every file arrived is refused by the import itself,
    // and nothing is stored.
    harness.answer(
        begin(vec![LICENSE.to_owned()], pack.manifest.clone()),
        today,
    );
    harness.answer(chunk("notes.txt", 0, notes), today);
    assert_eq!(
        refused(&harness.answer(DocPackRequest::ImportCommit {}, today)),
        Some(DocPackRefusal::ContentMismatch)
    );
    assert_eq!(harness.revision(), 0);
    // A new begin restarts the import from the first byte.
    harness.answer(
        begin(vec![LICENSE.to_owned()], pack.manifest.clone()),
        today,
    );
    harness.answer(chunk("notes.txt", 0, "Offline"), today);
    let answers = harness.import(&pack, None, today);
    assert!(matches!(
        answers.last(),
        Some(DocPackAnswer::Imported { .. })
    ));
    // The same version again is refused, and the store keeps one import.
    let again = harness.import(&pack, None, today);
    assert_eq!(
        refused(again.last().unwrap()),
        Some(DocPackRefusal::AlreadyImported)
    );
    assert_eq!(harness.revision(), 1);
}

#[test]
fn a_refresh_supersedes_and_retention_deletes_thirty_days_later() {
    let mut harness = Harness::new("doc-pack-retention");
    let first = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let second = guide(
        V2,
        "2026-01-08",
        "The cache keeps compiled units and results.",
    );
    harness.import(&first, None, "2026-01-01");
    // A refresh must name the current version.
    for refresh in [None, Some(V2)] {
        assert_eq!(
            refused(
                harness
                    .import(&second, refresh, "2026-01-10")
                    .last()
                    .unwrap()
            ),
            Some(DocPackRefusal::IntentMismatch)
        );
    }
    let answers = harness.import(&second, Some(V1), "2026-01-10");
    let Some(DocPackAnswer::Imported { receipt, .. }) = answers.last() else {
        panic!("refreshed: {answers:?}");
    };
    assert_eq!(receipt.superseded, Some(V1));
    assert_eq!(harness.revision(), 2);
    let inspect = || DocPackRequest::Inspect {
        pack_id: "build-tool-guide".to_owned(),
    };
    let DocPackAnswer::Inspected { status, retention } = harness.answer(inspect(), "2026-02-08")
    else {
        panic!("inspected");
    };
    assert!(retention.is_empty());
    assert_eq!(status.current, Some(V2));
    assert_eq!(
        (
            status.versions[0].current,
            status.versions[0].superseded_on.as_deref(),
            status.versions[0].delete_on.as_deref()
        ),
        (false, Some("2026-01-10"), Some("2026-02-09"))
    );
    // Thirty days after supersession, any operation applies retention first
    // and the store keeps the deletion.
    let DocPackAnswer::Inspected { status, retention } = harness.answer(inspect(), "2026-02-09")
    else {
        panic!("inspected");
    };
    assert_eq!(
        retention
            .iter()
            .map(|deletion| (deletion.version, deletion.reason))
            .collect::<Vec<_>>(),
        [(V1, DocPackDeletionReasonView::Retention)]
    );
    assert_eq!(status.versions.len(), 1);
    assert_eq!(harness.revision(), 3);
    harness.reopen();
    let stored = harness
        .runtime
        .as_ref()
        .unwrap()
        .doc_pack_catalog()
        .load()
        .unwrap();
    assert_eq!(stored.contents.deletions.len(), 1);
    assert_eq!(
        stored.contents.last_changed_on.as_deref(),
        Some("2026-02-09")
    );
    // A day earlier than the last change is refused and changes nothing.
    assert_eq!(
        refused(&harness.answer(DocPackRequest::List {}, "2026-02-08")),
        Some(DocPackRefusal::InvalidDate)
    );
    assert_eq!(harness.revision(), 3);
}

#[test]
fn deletion_and_search_go_through_the_stored_catalog() {
    let mut harness = Harness::new("doc-pack-delete-search");
    let first = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let second = guide(
        V2,
        "2026-01-08",
        "The cache keeps compiled units and results.",
    );
    harness.import(&first, None, "2026-01-01");
    harness.import(&second, Some(V1), "2026-01-10");
    let search = |terms: &[&str], pack_id: Option<&str>, include_history| DocPackRequest::Search {
        terms: terms.iter().map(|term| (*term).to_owned()).collect(),
        pack_id: pack_id.map(str::to_owned),
        include_history,
    };
    let DocPackAnswer::Found { hits, omitted, .. } =
        harness.answer(search(&["units"], None, false), "2026-01-11")
    else {
        panic!("found");
    };
    assert_eq!(omitted, 0);
    assert_eq!(hits.len(), 1);
    assert_eq!(
        (
            hits[0].version,
            hits[0].historical,
            hits[0].path.as_str(),
            hits[0].start_line
        ),
        (V2, false, "guide/cache.md", 3)
    );
    let DocPackAnswer::Found { hits, .. } = harness.answer(
        search(&["UNITS"], Some("build-tool-guide"), true),
        "2026-01-11",
    ) else {
        panic!("found");
    };
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().any(|hit| hit.version == V1 && hit.historical));
    for (request, refusal) in [
        (search(&[], None, false), DocPackRefusal::QueryInvalid),
        (
            search(&["two words"], None, false),
            DocPackRefusal::QueryInvalid,
        ),
        (
            search(&["x"; MAX_SEARCH_TERMS + 1], None, false),
            DocPackRefusal::QueryInvalid,
        ),
        (
            search(&["cache"], Some("other-pack"), false),
            DocPackRefusal::NotFound,
        ),
        (
            DocPackRequest::Delete {
                pack_id: "build-tool-guide".to_owned(),
                version: Some(DocPackVersion {
                    major: 9,
                    minor: 0,
                    patch: 0,
                }),
            },
            DocPackRefusal::NotFound,
        ),
        (
            DocPackRequest::Inspect {
                pack_id: "other-pack".to_owned(),
            },
            DocPackRefusal::NotFound,
        ),
    ] {
        assert_eq!(
            refused(&harness.answer(request, "2026-01-11")),
            Some(refusal)
        );
    }
    assert_eq!(harness.revision(), 2);
    // Deleting one version, then the pack.
    let DocPackAnswer::Deleted { deletions, .. } = harness.answer(
        DocPackRequest::Delete {
            pack_id: "build-tool-guide".to_owned(),
            version: Some(V1),
        },
        "2026-01-12",
    ) else {
        panic!("deleted");
    };
    assert_eq!(
        deletions
            .iter()
            .map(|deletion| (
                deletion.version,
                deletion.reason,
                deletion.deleted_on.as_str()
            ))
            .collect::<Vec<_>>(),
        [(V1, DocPackDeletionReasonView::Person, "2026-01-12")]
    );
    let DocPackAnswer::Deleted { deletions, .. } = harness.answer(
        DocPackRequest::Delete {
            pack_id: "build-tool-guide".to_owned(),
            version: None,
        },
        "2026-01-12",
    ) else {
        panic!("deleted");
    };
    assert_eq!(deletions.len(), 1);
    assert_eq!(harness.revision(), 4);
    let DocPackAnswer::Listed { packs, .. } = harness.answer(DocPackRequest::List {}, "2026-01-12")
    else {
        panic!("listed");
    };
    assert!(packs.is_empty());
    let DocPackAnswer::Found { hits, .. } =
        harness.answer(search(&["cache"], None, true), "2026-01-12")
    else {
        panic!("found");
    };
    assert!(hits.is_empty());
}

#[test]
fn an_import_leaves_room_for_every_later_deletion() {
    // Decision 0132 (review F2 of 935cfdd8): deletion records and kept
    // versions together stay within the store's deletion bound, so retention,
    // deletion and every read keep working when the history is full; only an
    // import is refused.
    assert_eq!(
        DocPackOwner::default().history_limit,
        MAX_DOC_PACK_DELETIONS
    );
    let mut harness = Harness::new("doc-pack-history-limit");
    harness.owner.history_limit = 3;
    let first = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let second = guide(V2, "2026-01-02", "The cache keeps compiled results.");
    let other = source(
        "linker-notes",
        V1,
        "2026-01-01",
        &[("notes.txt", "Linker notes.\n".to_owned())],
    );
    let third = source(
        "test-notes",
        V1,
        "2026-01-01",
        &[("notes.txt", "Test notes.\n".to_owned())],
    );
    let imported = |answers: Vec<DocPackAnswer>| {
        matches!(answers.last(), Some(DocPackAnswer::Imported { .. }))
    };
    assert!(imported(harness.import(&first, None, "2026-01-02")));
    assert!(imported(harness.import(&second, Some(V1), "2026-01-02")));
    assert!(imported(harness.import(&other, None, "2026-01-02")));
    assert_eq!(harness.revision(), 3);
    // Three kept versions fill the history: the next import is refused and
    // nothing changes.
    assert_eq!(
        refused(harness.import(&third, None, "2026-01-03").last().unwrap()),
        Some(DocPackRefusal::StoreLimit)
    );
    assert_eq!(harness.revision(), 3);
    // Retention still turns the superseded version into its record, and the
    // read that applied it answers.
    let DocPackAnswer::Listed { packs, retention } =
        harness.answer(DocPackRequest::List {}, "2026-02-01")
    else {
        panic!("listed");
    };
    assert_eq!((packs.len(), retention.len()), (2, 1));
    assert_eq!(harness.revision(), 4);
    assert_eq!(
        refused(harness.import(&third, None, "2026-02-01").last().unwrap()),
        Some(DocPackRefusal::StoreLimit)
    );
    // A person's deletion still commits, and reads still answer.
    assert!(matches!(
        harness.answer(
            DocPackRequest::Delete {
                pack_id: "linker-notes".to_owned(),
                version: None,
            },
            "2026-02-01"
        ),
        DocPackAnswer::Deleted { .. }
    ));
    assert_eq!(harness.revision(), 5);
    assert!(matches!(
        harness.answer(
            DocPackRequest::Inspect {
                pack_id: "build-tool-guide".to_owned(),
            },
            "2026-02-02"
        ),
        DocPackAnswer::Inspected { .. }
    ));
    // Below the bound an import is admitted again.
    harness.owner.history_limit = 4;
    assert!(imported(harness.import(&third, None, "2026-02-02")));
    assert_eq!(harness.revision(), 6);
}

#[test]
fn operations_need_a_clock_and_an_open_store_and_refusals_change_nothing() {
    let mut owner = DocPackOwner::default();
    let pack = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let mut unavailable = || Err(DocPackRefusal::StoreUnavailable);
    for request in [
        DocPackRequest::List {},
        DocPackRequest::Inspect {
            pack_id: "build-tool-guide".to_owned(),
        },
        DocPackRequest::Delete {
            pack_id: "build-tool-guide".to_owned(),
            version: None,
        },
        DocPackRequest::Search {
            terms: vec!["cache".to_owned()],
            pack_id: None,
            include_history: false,
        },
    ] {
        assert_eq!(
            refused(&owner.answer(request.clone(), &mut unavailable, None)),
            Some(DocPackRefusal::ClockUnavailable)
        );
        assert_eq!(
            refused(&owner.answer(request, &mut unavailable, Some("2026-01-02"))),
            Some(DocPackRefusal::StoreUnavailable)
        );
    }
    // Staging needs neither; the commit needs both.
    for request in doc_pack_import_requests(&pack, &[LICENSE.to_owned()], None) {
        let commit = request == DocPackRequest::ImportCommit {};
        let answer = owner.answer(request, &mut unavailable, None);
        assert_eq!(
            refused(&answer),
            commit.then_some(DocPackRefusal::ClockUnavailable)
        );
    }
}

#[test]
fn a_stored_version_must_restore_as_the_catalog_kept_it() {
    // The store keeps manifests opaque; the owner refuses any stored version
    // whose manifest does not parse, verify and name its row.
    use agentmage_kernel_engine::doc_pack_store::{
        DocPackCatalogContents, DocPackVersionKey, StoredDocPackFile, StoredDocPackVersion,
    };
    let pack = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let stored_version =
        |pack_id: &str, manifest_json: Vec<u8>, superseded_on: Option<&str>| StoredDocPackVersion {
            key: DocPackVersionKey {
                pack_id: pack_id.to_owned(),
                major: 1,
                minor: 0,
                patch: 0,
            },
            manifest_sha256: pack.manifest.manifest_sha256.clone(),
            manifest_json,
            superseded_on: superseded_on.map(str::to_owned),
            files: pack
                .files
                .iter()
                .map(|(path, text)| StoredDocPackFile {
                    path: path.clone(),
                    sha256: sha256_hex(text.as_bytes()),
                    content: text.as_bytes().to_vec(),
                })
                .collect(),
        };
    let manifest_json = serde_json::to_vec(&pack.manifest).unwrap();
    let mut unsealed = pack.manifest.clone();
    unsealed.title = "Changed".to_owned();
    for (version, last_changed_on, refusal) in [
        (
            stored_version("other-pack", manifest_json.clone(), None),
            "2026-01-02",
            DocPackRefusal::StoreIntegrity,
        ),
        (
            stored_version("build-tool-guide", b"{}".to_vec(), None),
            "2026-01-02",
            DocPackRefusal::StoreIntegrity,
        ),
        (
            stored_version(
                "build-tool-guide",
                serde_json::to_vec(&unsealed).unwrap(),
                None,
            ),
            "2026-01-02",
            DocPackRefusal::StoreIntegrity,
        ),
        // A state the catalog's own operations cannot produce: content
        // obtained after the catalog's last change.
        (
            stored_version("build-tool-guide", manifest_json.clone(), None),
            "2025-12-31",
            DocPackRefusal::StateInvalid,
        ),
    ] {
        let store = JobLedgerStore::new("doc-pack-restore");
        let runtime = store.try_runtime().unwrap();
        let catalog = runtime.doc_pack_catalog();
        catalog
            .commit(
                0,
                &DocPackCatalogContents {
                    versions: vec![version],
                    deletions: Vec::new(),
                    last_changed_on: Some(last_changed_on.to_owned()),
                },
            )
            .unwrap();
        let mut owner = DocPackOwner::default();
        assert_eq!(
            refused(&owner.answer(
                DocPackRequest::List {},
                &mut || Ok(catalog.clone()),
                Some("2026-01-03")
            )),
            Some(refusal)
        );
        assert_eq!(catalog.load().unwrap().revision, 1);
    }
}

#[test]
fn requests_and_answers_are_parsed_closed() {
    let pack = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let requests = vec![
        DocPackRequest::ImportBegin {
            manifest: pack.manifest.clone(),
            allowed_licenses: vec![LICENSE.to_owned()],
            refresh: Some(V1),
        },
        DocPackRequest::ImportChunk {
            path: "notes.txt".to_owned(),
            offset: 0,
            text: "Offline".to_owned(),
        },
        DocPackRequest::ImportCommit {},
        DocPackRequest::List {},
        DocPackRequest::Inspect {
            pack_id: "build-tool-guide".to_owned(),
        },
        DocPackRequest::Delete {
            pack_id: "build-tool-guide".to_owned(),
            version: None,
        },
        DocPackRequest::Search {
            terms: vec!["cache".to_owned()],
            pack_id: None,
            include_history: true,
        },
    ];
    for request in requests {
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(
            serde_json::from_value::<DocPackRequest>(value.clone()).unwrap(),
            request
        );
        let mut extra = value.clone();
        extra["extra"] = serde_json::Value::Bool(true);
        assert!(
            serde_json::from_value::<DocPackRequest>(extra).is_err(),
            "{value}"
        );
        for member in value.as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(member);
            assert!(
                serde_json::from_value::<DocPackRequest>(missing).is_err(),
                "{value} without {member}"
            );
        }
    }
    let deletion = DocPackDeletionView {
        pack_id: "build-tool-guide".to_owned(),
        version: V1,
        manifest_sha256: "a".repeat(64),
        deleted_on: "2026-01-02".to_owned(),
        reason: DocPackDeletionReasonView::Retention,
    };
    let answers = vec![
        DocPackAnswer::Refused {
            refusal: DocPackRefusal::NotFound,
        },
        DocPackAnswer::Deleted {
            deletions: vec![deletion.clone()],
            retention: vec![deletion],
        },
        DocPackAnswer::Found {
            hits: Vec::new(),
            omitted: 0,
            retention: Vec::new(),
        },
    ];
    for answer in answers {
        let value = serde_json::to_value(&answer).unwrap();
        assert_eq!(
            serde_json::from_value::<DocPackAnswer>(value.clone()).unwrap(),
            answer
        );
        let mut extra = value.clone();
        extra["extra"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<DocPackAnswer>(extra).is_err());
    }
    // Nested records are closed too.
    let mut nested = serde_json::to_value(DocPackAnswer::Deleted {
        deletions: Vec::new(),
        retention: vec![DocPackDeletionView {
            pack_id: "p".to_owned(),
            version: V1,
            manifest_sha256: "a".repeat(64),
            deleted_on: "2026-01-02".to_owned(),
            reason: DocPackDeletionReasonView::Person,
        }],
    })
    .unwrap();
    nested["retention"][0]["extra"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<DocPackAnswer>(nested).is_err());
    assert_eq!(
        serde_json::to_value(DocPackRefusal::StoreIntegrity).unwrap(),
        serde_json::json!("store-integrity")
    );
}

#[test]
fn only_a_receipt_for_the_sent_manifest_that_recomputes_is_kept() {
    let pack = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    let mut catalog = DocPackCatalog::new();
    let receipt = receipt_view(
        &catalog
            .import(
                &policy(BTreeSet::from([LICENSE.to_owned()])),
                &pack.manifest,
                &pack
                    .files
                    .iter()
                    .map(|(path, text)| (path.clone(), text.as_bytes().to_vec()))
                    .collect(),
                DocPackImportIntent::New,
                "2026-01-02",
            )
            .unwrap(),
    );
    assert!(verify_doc_pack_receipt(&receipt, &pack.manifest));
    let resealed = |mut value: DocPackImportReceiptView| {
        value.receipt_sha256 = doc_pack_receipt_sha256(&value).unwrap();
        value
    };
    let other = guide(V2, "2026-01-01", "Another paragraph.");
    assert!(!verify_doc_pack_receipt(&receipt, &other.manifest));
    let mut tampered = receipt.clone();
    tampered.fragment_count += 1;
    for changed in [
        tampered,
        resealed(DocPackImportReceiptView {
            network_used: true,
            ..receipt.clone()
        }),
        resealed(DocPackImportReceiptView {
            file_count: 3,
            ..receipt.clone()
        }),
        resealed(DocPackImportReceiptView {
            total_bytes: 1,
            ..receipt.clone()
        }),
        resealed(DocPackImportReceiptView {
            license: "MIT".to_owned(),
            ..receipt.clone()
        }),
        resealed(DocPackImportReceiptView {
            version: V2,
            ..receipt.clone()
        }),
        resealed(DocPackImportReceiptView {
            pack_id: "other-pack".to_owned(),
            ..receipt.clone()
        }),
        resealed(DocPackImportReceiptView {
            manifest_sha256: "b".repeat(64),
            ..receipt.clone()
        }),
    ] {
        assert!(
            !verify_doc_pack_receipt(&changed, &pack.manifest),
            "{changed:?}"
        );
    }
}

#[test]
fn dates_versions_identities_and_terms_parse_exactly() {
    for (epoch_ms, date) in [
        (0, "1970-01-01"),
        (86_399_999, "1970-01-01"),
        (951_782_400_000, "2000-02-29"),
        (951_868_800_000, "2000-03-01"),
        (1_709_164_800_000, "2024-02-29"),
        (1_798_675_200_000, "2026-12-31"),
    ] {
        assert_eq!(
            iso_date_of_epoch_ms(epoch_ms).as_deref(),
            Some(date),
            "{epoch_ms}"
        );
    }
    assert_eq!(iso_date_of_epoch_ms(u64::MAX), None);
    assert_eq!(parse_doc_pack_version("1.0.0"), Some(V1));
    assert_eq!(
        parse_doc_pack_version("10.20.4294967295"),
        Some(DocPackVersion {
            major: 10,
            minor: 20,
            patch: u32::MAX
        })
    );
    for invalid in [
        "",
        "1",
        "1.0",
        "1.0.0.0",
        "01.0.0",
        "1.00.0",
        "a.b.c",
        "1.0.-1",
        "+1.0.0",
        "1.0.0 ",
        "4294967296.0.0",
    ] {
        assert_eq!(parse_doc_pack_version(invalid), None, "{invalid}");
    }
    assert!(plain_doc_pack_id("build-tool-guide") && plain_doc_pack_id("a1.b"));
    for invalid in ["", "Build", "1pack", "pack_id", "pack/id", &"a".repeat(65)] {
        assert!(!plain_doc_pack_id(invalid), "{invalid}");
    }
    assert!(doc_pack_license_id("CC-BY-4.0") && doc_pack_license_id("GPL-2.0+"));
    for invalid in ["", "MIT License", "MIT;rm", &"a".repeat(65)] {
        assert!(!doc_pack_license_id(invalid), "{invalid}");
    }
    assert_eq!(
        doc_pack_search_terms("  build   cache "),
        Some(vec!["build".to_owned(), "cache".to_owned()])
    );
    assert_eq!(doc_pack_search_terms("   "), None);
    assert_eq!(
        doc_pack_search_terms(&"x ".repeat(MAX_SEARCH_TERMS + 1)),
        None
    );
    assert_eq!(
        doc_pack_search_terms(&"x".repeat(MAX_SEARCH_TERM_BYTES + 1)),
        None
    );
}

#[test]
fn answers_render_as_escaped_text_lines_or_json_rows() {
    let hit = DocPackHitView {
        pack_id: "build-tool-guide".to_owned(),
        version: V1,
        historical: true,
        path: "guide/cache.md".to_owned(),
        start_line: 3,
        end_line: 4,
        heading: false,
        text: "Plain line.\nHostile \u{1b}[2J \u{202e}reversed\u{2028}line\r".to_owned(),
        citation_sha256: "c".repeat(64),
    };
    let found = DocPackAnswer::Found {
        hits: vec![hit],
        omitted: 2,
        retention: vec![DocPackDeletionView {
            pack_id: "build-tool-guide".to_owned(),
            version: V2,
            manifest_sha256: "a".repeat(64),
            deleted_on: "2026-01-02".to_owned(),
            reason: DocPackDeletionReasonView::Retention,
        }],
    };
    let text = render_doc_pack_answer(&found, false);
    assert_eq!(
        text,
        "doc pack deleted: build-tool-guide 1.1.0 (retention) on 2026-01-02; manifest aaaaaaaaaaaa\n\
         doc pack hit 1: build-tool-guide 1.0.0 (history) guide/cache.md:3-4; citation cccccccccccc\n\
         \x20 | Plain line.\n\
         \x20 | Hostile \\u{1b}[2J \\u{202e}reversed\\u{2028}line\\r\n\
         doc pack search: 1 hits; 2 more not shown\n"
    );
    assert!(!text.contains('\u{1b}') && !text.contains('\u{202e}') && !text.contains('\r'));
    let rows = render_doc_pack_answer(&found, true)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rows.iter()
            .map(|row| row["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["doc_pack_deletion", "doc_pack_hit", "doc_pack_search"]
    );
    assert_eq!(rows[2]["omitted"], 2);
    let empty = DocPackAnswer::Listed {
        packs: Vec::new(),
        retention: Vec::new(),
    };
    assert_eq!(
        render_doc_pack_answer(&empty, false),
        "doc packs: none kept\n"
    );
    // A listing always ends with a summary row, so an empty one is not
    // silent.
    assert_eq!(
        render_doc_pack_answer(&empty, true),
        "{\"packs\":0,\"type\":\"doc_pack_list\"}\n"
    );
    assert_eq!(
        render_doc_pack_refusal(DocPackRefusal::LicenseNotAllowed, false),
        "doc pack refused: doc-pack.license-not-allowed\n"
    );
    assert_eq!(
        render_doc_pack_refusal(DocPackRefusal::LicenseNotAllowed, true),
        "{\"code\":\"doc-pack.license-not-allowed\",\"type\":\"doc_pack_refused\"}\n"
    );
    for (refusal, exit) in [
        (
            DocPackRefusal::ManifestInvalid,
            crate::headless::ClientExitCode::InvalidInput,
        ),
        (
            DocPackRefusal::NotFound,
            crate::headless::ClientExitCode::PolicyDenied,
        ),
        (
            DocPackRefusal::StoreLimit,
            crate::headless::ClientExitCode::ResourceBound,
        ),
        (
            DocPackRefusal::StoreIntegrity,
            crate::headless::ClientExitCode::ServiceUnavailable,
        ),
    ] {
        assert_eq!(doc_pack_refusal_exit(refusal), exit);
    }
}

#[test]
fn a_pack_is_read_only_from_regular_files_inside_its_directory() {
    let directory = fixture::PackDirectory::new("doc-pack-source");
    let pack = guide(V1, "2026-01-01", "The cache keeps compiled units.");
    directory.write(&pack);
    assert_eq!(read_doc_pack_source(directory.path()).unwrap(), pack);
    assert_eq!(
        read_doc_pack_source(std::path::Path::new("relative/pack")),
        Err(DocPackRefusal::ContentMismatch)
    );
    for (name, change, refusal) in fixture::changes() {
        let directory = fixture::PackDirectory::new(name);
        directory.write(&pack);
        change(&directory);
        assert_eq!(
            read_doc_pack_source(directory.path()),
            Err(refusal),
            "{name}"
        );
    }
}

// The pack directory fixture creates, links and replaces files, so it is a
// test module of its own for the effect boundary scan.
#[cfg(test)]
mod fixture {
    use std::path::{Path, PathBuf};

    use super::{DocPackRefusal, DocPackSource};

    pub(super) struct PackDirectory {
        root: PathBuf,
        pack: PathBuf,
    }

    impl PackDirectory {
        pub(super) fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            let pack = root.join("pack");
            std::fs::create_dir(&pack).unwrap();
            Self { root, pack }
        }

        pub(super) fn path(&self) -> &Path {
            &self.pack
        }

        pub(super) fn write(&self, source: &DocPackSource) {
            std::fs::write(
                self.pack.join("manifest.json"),
                serde_json::to_vec_pretty(&source.manifest).unwrap(),
            )
            .unwrap();
            for (path, text) in &source.files {
                let target = self.pack.join(path);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::write(target, text).unwrap();
            }
        }
    }

    impl Drop for PackDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    type Change = fn(&PackDirectory);

    /// Each change of a written pack and the refusal it meets.
    pub(super) fn changes() -> Vec<(&'static str, Change, DocPackRefusal)> {
        vec![
            (
                "doc-pack-source-missing-file",
                |directory| std::fs::remove_file(directory.pack.join("notes.txt")).unwrap(),
                DocPackRefusal::ContentMismatch,
            ),
            (
                "doc-pack-source-longer-file",
                |directory| {
                    std::fs::write(
                        directory.pack.join("notes.txt"),
                        "Offline notes about it all.\n",
                    )
                    .unwrap();
                },
                DocPackRefusal::ContentMismatch,
            ),
            (
                "doc-pack-source-linked-file",
                |directory| {
                    let moved = directory.root.join("notes.txt");
                    std::fs::rename(directory.pack.join("notes.txt"), &moved).unwrap();
                    std::os::unix::fs::symlink(&moved, directory.pack.join("notes.txt")).unwrap();
                },
                DocPackRefusal::ContentMismatch,
            ),
            (
                "doc-pack-source-linked-directory",
                |directory| {
                    let moved = directory.root.join("guide");
                    std::fs::rename(directory.pack.join("guide"), &moved).unwrap();
                    std::os::unix::fs::symlink(&moved, directory.pack.join("guide")).unwrap();
                },
                DocPackRefusal::ContentMismatch,
            ),
            (
                "doc-pack-source-linked-manifest",
                |directory| {
                    let moved = directory.root.join("manifest.json");
                    std::fs::rename(directory.pack.join("manifest.json"), &moved).unwrap();
                    std::os::unix::fs::symlink(&moved, directory.pack.join("manifest.json"))
                        .unwrap();
                },
                DocPackRefusal::ContentMismatch,
            ),
            (
                "doc-pack-source-fifo",
                |directory| {
                    std::fs::remove_file(directory.pack.join("notes.txt")).unwrap();
                    rustix::fs::mknodat(
                        rustix::fs::CWD,
                        directory.pack.join("notes.txt").as_path(),
                        rustix::fs::FileType::Fifo,
                        rustix::fs::Mode::from_raw_mode(0o600),
                        0,
                    )
                    .unwrap();
                },
                DocPackRefusal::ContentMismatch,
            ),
            (
                "doc-pack-source-unsealed-manifest",
                |directory| {
                    let path = directory.pack.join("manifest.json");
                    let text = std::fs::read_to_string(&path)
                        .unwrap()
                        .replace("Build tool guide", "Build tool notes");
                    std::fs::write(path, text).unwrap();
                },
                DocPackRefusal::ManifestInvalid,
            ),
            (
                "doc-pack-source-control-character",
                |directory| {
                    // Same length; a control character the host refuses at
                    // commit is refused when the file is read (Decision 0132).
                    std::fs::write(
                        directory.pack.join("notes.txt"),
                        "Offline notes about the cache\u{1}\n",
                    )
                    .unwrap();
                },
                DocPackRefusal::ContentInvalid,
            ),
            (
                "doc-pack-source-not-text",
                |directory| {
                    // Same length, invalid UTF-8; the manifest digest still
                    // differs, but encoding is checked when the file is read.
                    let path = directory.pack.join("notes.txt");
                    let length = std::fs::metadata(&path).unwrap().len() as usize;
                    std::fs::write(path, vec![0xff; length]).unwrap();
                },
                DocPackRefusal::ContentInvalid,
            ),
        ]
    }
}
