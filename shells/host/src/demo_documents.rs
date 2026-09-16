//! Read-only, fd-anchored document snapshots using the canonical knowledge capability.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

use agentmage_capability_knowledge::{
    KnowledgeAnswerDraft, KnowledgeContextQuery, KnowledgeEvidenceState, KnowledgeFileType,
    KnowledgeRetrievalResult, KnowledgeSourceAuthority, KnowledgeSourceDocument,
    KnowledgeSourceFragment, KnowledgeSourceFragmentKind, KnowledgeSynthesisEnvelope,
    ObsidianSourceRange, prepare_knowledge_synthesis, render_knowledge_answer, retrieve_knowledge,
};
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath, WorkspaceScopePath};
use rustix::fd::OwnedFd;
use rustix::fs::{Dir, FileType, Mode, OFlags, ResolveFlags, fstat, open, openat2};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_FILE_BYTES: usize = 128 * 1024;
const MAX_TOTAL_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 200;
const MAX_DEPTH: usize = 8;
const CONTEXT_BYTES: usize = 12_000;
const DATE: &str = "2026-09-15";

/// A process-local immutable document snapshot and the most recent synthesis boundary.
#[derive(Default)]
pub struct DemoDocuments {
    documents: Vec<KnowledgeSourceDocument>,
    snapshot_sha256: String,
    envelope: Option<KnowledgeSynthesisEnvelope>,
    citation_ids: BTreeMap<String, String>,
    omitted_count: u64,
}

impl DemoDocuments {
    /// Executes one bounded JSON request, returning a content-free error on failure.
    pub fn execute(&mut self, request: &Value) -> Value {
        let result = match request.get("op").and_then(Value::as_str) {
            Some("ingest") => request
                .get("folder")
                .and_then(Value::as_str)
                .ok_or("folder must be an absolute path")
                .and_then(|folder| self.ingest(folder)),
            Some("query") => request
                .get("question")
                .and_then(Value::as_str)
                .ok_or("question must be text")
                .and_then(|question| self.query(question)),
            Some("validate") => self.validate(request),
            _ => Err("unknown operation"),
        };
        result.unwrap_or_else(|error| json!({"ok":false,"error":error}))
    }

    fn ingest(&mut self, folder: &str) -> Result<Value, &'static str> {
        // Clear the old dataset before every attempted replacement, including failed admission.
        self.documents.clear();
        self.envelope = None;
        self.citation_ids.clear();
        self.snapshot_sha256.clear();
        self.omitted_count = 0;
        let root_fd = open_root(folder)?;
        let mut inventory = Inventory::default();
        inventory.walk(&root_fd, "", 0)?;
        inventory
            .files
            .sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        inventory.documents.sort_by(|a, b| a.path.cmp(&b.path));
        let identities = inventory
            .documents
            .iter()
            .map(|doc| (relative_path(&doc.path), &doc.content_sha256))
            .collect::<Vec<_>>();
        self.snapshot_sha256 =
            digest(&serde_json::to_vec(&identities).map_err(|_| "snapshot serialization failed")?);
        self.documents = inventory.documents;
        Ok(
            json!({"ok":true,"files":inventory.files,"accepted_count":self.documents.len(),
            "snapshot_sha256":self.snapshot_sha256,"total_bytes":inventory.total_bytes,
            "source_files_changed":false,"inventory_complete":true}),
        )
    }

    fn query(&mut self, question: &str) -> Result<Value, &'static str> {
        self.envelope = None;
        self.citation_ids.clear();
        if question.trim().is_empty()
            || question.len() > 4096
            || question.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err("question must contain 1–4096 bytes of ordinary text");
        }
        if self.documents.is_empty() {
            return Err("no accepted documents; select a supported folder first");
        }
        if question
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word.len() > 128)
        {
            return Err("a retrieval term exceeds 128 bytes; shorten the question");
        }
        let mut terms = query_terms(question);
        if terms.len() > 64 {
            return Err("question exceeds 64 distinct retrieval terms; shorten the question");
        }
        let summary_requested = terms.iter().any(|term| {
            matches!(
                term.as_str(),
                "summarize" | "summarise" | "summary" | "overview"
            )
        });
        if summary_requested {
            // One literal character present in each fragment guarantees all admissible fragments
            // enter the same canonical capability path, retaining its source and secret policies.
            terms = self
                .documents
                .iter()
                .flat_map(|document| &document.fragments)
                .filter_map(|fragment| {
                    fragment
                        .text
                        .chars()
                        .find(|c| !c.is_whitespace())
                        .map(|c| c.to_string())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
        let mut union = BTreeMap::new();
        let mut omitted = 0_u64;
        for term in terms {
            let query = KnowledgeContextQuery {
                terms: vec![term],
                phrases: vec![],
                roots: vec![workspace_root()],
                date_from: None,
                date_to: None,
                as_of_date: DATE.into(),
                file_types: BTreeSet::from([KnowledgeFileType::Markdown, KnowledgeFileType::Text]),
                authorities: BTreeSet::from([KnowledgeSourceAuthority::DirectEvidence]),
                include_historical: false,
                max_results: 1000,
                max_context_bytes: MAX_TOTAL_BYTES as u64,
            };
            let found = retrieve_knowledge(&query, &self.documents)
                .map_err(|_| "canonical retrieval rejected the query or source snapshot")?;
            omitted = omitted.saturating_add(found.omitted_candidate_count);
            for hit in found.hits {
                union.entry(hit.citation_sha256.clone()).or_insert(hit);
            }
        }
        let mut hits = union.into_values().collect::<Vec<_>>();
        hits.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then(a.source_range.cmp(&b.source_range))
        });
        let mut context = String::new();
        let mut citations = vec![];
        let mut kept = vec![];
        for hit in hits {
            let id = format!("S{}", citations.len() + 1);
            let path = relative_path(&hit.path);
            let section = format!(
                "[{id}] {path}:{}-{}\n{}\n\n",
                hit.source_range.start_line, hit.source_range.end_line, hit.text
            );
            if context.len().saturating_add(section.len()) > CONTEXT_BYTES {
                omitted += 1;
                continue;
            }
            context.push_str(&section);
            self.citation_ids
                .insert(id.clone(), hit.citation_sha256.clone());
            citations.push(json!({"id":id,"path":path,"start_line":hit.source_range.start_line,
                "end_line":hit.source_range.end_line,"text":hit.text,"content_sha256":hit.content_sha256,
                "start_column":hit.source_range.start_column,"end_column":hit.source_range.end_column,
                "citation_sha256":hit.citation_sha256}));
            kept.push(hit);
        }
        let evidence_state = if kept.is_empty() {
            KnowledgeEvidenceState::UnknownBlocked
        } else {
            KnowledgeEvidenceState::Observed
        };
        let entries = kept
            .iter()
            .map(
                |hit| agentmage_capability_knowledge::KnowledgeContextEntry {
                    text: hit.text.clone(),
                    citation_sha256: hit.citation_sha256.clone(),
                    path: hit.path.clone(),
                    source_range: hit.source_range,
                },
            )
            .collect();
        self.envelope = Some(prepare_knowledge_synthesis(&KnowledgeRetrievalResult {
            evidence_state,
            hits: kept,
            context: entries,
            conflicting_fact_keys: vec![],
            denied_source_count: 0,
            omitted_candidate_count: omitted,
            semantic_components_used: false,
        }));
        self.omitted_count = omitted;
        Ok(
            json!({"ok":true,"citations":citations,"context":context,"evidence_state":
            if omitted > 0 { "incomplete" } else if evidence_state == KnowledgeEvidenceState::UnknownBlocked { "unknown_blocked" } else { "observed" },
            "omitted_count":omitted,"snapshot_sha256":self.snapshot_sha256,"retrieval":if summary_requested { "canonical_bounded_document_summary" } else { "canonical_disjunctive_lexical" },
            "context_complete":omitted==0}),
        )
    }

    fn validate(&self, request: &Value) -> Result<Value, &'static str> {
        let envelope = self
            .envelope
            .as_ref()
            .ok_or("query is required before answer validation")?;
        let answer = request
            .get("answer")
            .and_then(Value::as_str)
            .ok_or("answer must be text")?;
        if answer.len() > 32_768 {
            return Err("answer exceeds demo limit");
        }
        let ids = request
            .get("citation_ids")
            .and_then(Value::as_array)
            .ok_or("citation_ids must be an array")?;
        let hashes = ids
            .iter()
            .map(|id| {
                id.as_str()
                    .and_then(|id| self.citation_ids.get(id))
                    .cloned()
                    .ok_or("answer refers to a citation outside the retrieved snapshot")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let blocked = envelope.evidence_state == KnowledgeEvidenceState::UnknownBlocked;
        let draft = KnowledgeAnswerDraft {
            evidence_state: envelope.evidence_state,
            text: if blocked {
                String::new()
            } else {
                answer.replace(['\n', '\r', '\t'], " ")
            },
            citation_sha256: hashes,
        };
        let rendered = render_knowledge_answer(envelope, &draft)
            .map_err(|_| "answer failed canonical evidence validation")?;
        Ok(
            json!({"ok":true,"answer":if blocked { rendered.text } else { answer.to_owned() },
            "citation_sha256":rendered.citation_sha256,"validation":"citation_membership_and_evidence_state_only",
            "evidence_state":if self.omitted_count > 0 { "incomplete" } else if blocked { "unknown_blocked" } else { "observed" },
            "context_complete":self.omitted_count == 0,"omitted_count":self.omitted_count,"snapshot_sha256":self.snapshot_sha256}),
        )
    }
}

#[derive(Default)]
struct Inventory {
    documents: Vec<KnowledgeSourceDocument>,
    files: Vec<Value>,
    entries: usize,
    total_bytes: usize,
}

impl Inventory {
    fn report(&mut self, path: &str, status: &str, reason: &str) {
        self.files
            .push(json!({"path":path,"status":status,"reason":reason}));
    }

    fn walk(&mut self, root: &OwnedFd, prefix: &str, depth: usize) -> Result<(), &'static str> {
        let fd = beneath(
            root,
            if prefix.is_empty() { "." } else { prefix },
            OFlags::RDONLY | OFlags::DIRECTORY,
        )
        .map_err(|_| "folder changed or cannot be securely inventoried; no dataset admitted")?;
        let mut directory =
            Dir::new(fd).map_err(|_| "directory inventory failed; no dataset admitted")?;
        let directory_before = fstat(
            directory
                .fd()
                .map_err(|_| "directory metadata unavailable")?,
        )
        .map_err(|_| "directory metadata unavailable")?;
        while let Some(entry) = directory.read() {
            let entry = entry.map_err(|_| "directory inventory failed; no dataset admitted")?;
            let name_bytes = entry.file_name().to_bytes();
            if name_bytes == b"." || name_bytes == b".." {
                continue;
            }
            self.entries += 1;
            if self.entries > MAX_ENTRIES {
                return Err(
                    "folder exceeds 200 inventory entries; no dataset admitted; select a smaller folder",
                );
            }
            let Ok(name) = entry.file_name().to_str() else {
                self.report("[non-UTF8 filename]", "rejected", "filename is not UTF-8");
                continue;
            };
            let path = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            if name.starts_with('.') {
                self.report(&path, "skipped", "hidden files and folders are excluded");
                continue;
            }
            if entry.file_type() == FileType::Symlink {
                self.report(
                    &path,
                    "rejected",
                    "symbolic links are excluded to protect folder boundaries",
                );
                continue;
            }
            let fd = match beneath(root, &path, OFlags::RDONLY | OFlags::NONBLOCK) {
                Ok(fd) => fd,
                Err(_) => {
                    self.report(
                        &path,
                        "rejected",
                        "secure read failed; symlink, changed entry, or permissions",
                    );
                    continue;
                }
            };
            let before = fstat(&fd).map_err(|_| "source metadata read failed")?;
            if FileType::from_raw_mode(before.st_mode) == FileType::Directory {
                if depth >= MAX_DEPTH {
                    return Err("folder exceeds maximum nesting depth 8; no dataset admitted");
                }
                self.walk(root, &path, depth + 1)?;
                continue;
            }
            if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile {
                self.report(&path, "rejected", "only regular files are supported");
                continue;
            }
            let extension = Path::new(name)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "txt" | "md" | "markdown") {
                self.report(
                    &path,
                    "skipped",
                    "unsupported format; supported: .txt, .md, .markdown",
                );
                continue;
            }
            if before.st_size < 0 || before.st_size as u64 > MAX_FILE_BYTES as u64 {
                self.report(&path, "rejected", "file exceeds 128 KiB");
                continue;
            }
            let file = File::from(fd);
            let mut bytes = Vec::new();
            (&file)
                .take((MAX_FILE_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| "source read failed; no dataset admitted")?;
            let after = fstat(&file).map_err(|_| "source metadata read failed")?;
            if bytes.len() > MAX_FILE_BYTES
                || before.st_size != after.st_size
                || before.st_mtime != after.st_mtime
                || before.st_mtime_nsec != after.st_mtime_nsec
                || before.st_ctime != after.st_ctime
                || before.st_ctime_nsec != after.st_ctime_nsec
            {
                self.report(&path, "rejected", "source changed during snapshot capture");
                continue;
            }
            if self.total_bytes.saturating_add(bytes.len()) > MAX_TOTAL_BYTES {
                return Err(
                    "folder exceeds 1 MiB admitted text; no dataset admitted; select a smaller folder",
                );
            }
            let Ok(text) = std::str::from_utf8(&bytes) else {
                self.report(&path, "rejected", "document is not valid UTF-8 text");
                continue;
            };
            if text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            {
                self.report(
                    &path,
                    "rejected",
                    "document contains binary/control characters",
                );
                continue;
            }
            if text.trim().is_empty() {
                self.report(&path, "skipped", "empty document");
                continue;
            }
            if text.lines().any(|line| line.len() > 4096) {
                self.report(
                    &path,
                    "rejected",
                    "a line exceeds 4096 bytes; split long lines for exact source citations",
                );
                continue;
            }
            let workspace_path =
                match WorkspacePath::new(WorkspaceId::from_raw("local-demo"), path.split('/')) {
                    Ok(path) => path,
                    Err(_) => {
                        self.report(
                            &path,
                            "rejected",
                            "filename does not satisfy canonical workspace path rules",
                        );
                        continue;
                    }
                };
            self.total_bytes += bytes.len();
            let hash = digest(&bytes);
            self.documents.push(KnowledgeSourceDocument {
                root: workspace_root(),
                path: workspace_path,
                file_type: if extension == "txt" {
                    KnowledgeFileType::Text
                } else {
                    KnowledgeFileType::Markdown
                },
                authority: KnowledgeSourceAuthority::DirectEvidence,
                content_sha256: hash.clone(),
                current_content_sha256: hash,
                verified_on: DATE.into(),
                source_date: DATE.into(),
                note_kind: None,
                historical: false,
                denied: false,
                fragments: fragments(text),
            });
            self.report(
                &path,
                "accepted",
                "UTF-8 text captured read-only; citations refer to this immutable snapshot",
            );
        }
        let directory_after = fstat(
            directory
                .fd()
                .map_err(|_| "directory metadata unavailable")?,
        )
        .map_err(|_| "directory metadata unavailable")?;
        if directory_before.st_mtime != directory_after.st_mtime
            || directory_before.st_mtime_nsec != directory_after.st_mtime_nsec
            || directory_before.st_ctime != directory_after.st_ctime
            || directory_before.st_ctime_nsec != directory_after.st_ctime_nsec
        {
            return Err(
                "folder entries changed during inventory; no dataset admitted; retry when changes stop",
            );
        }
        Ok(())
    }
}

fn open_root(folder: &str) -> Result<OwnedFd, &'static str> {
    let path = Path::new(folder);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err("folder must be an absolute path without traversal components");
    }
    let slash = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| "cannot open filesystem root")?;
    let relative = folder.trim_start_matches('/');
    if relative.is_empty() {
        return Err("filesystem root cannot be selected");
    }
    beneath(&slash, relative, OFlags::RDONLY | OFlags::DIRECTORY).map_err(
        |_| "folder unavailable or its path contains a symbolic link; use its real absolute path",
    )
}

fn beneath(root: &OwnedFd, path: &str, flags: OFlags) -> rustix::io::Result<OwnedFd> {
    openat2(
        root,
        path,
        flags | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
}

fn workspace_root() -> WorkspaceScopePath {
    WorkspaceScopePath::new(
        WorkspaceId::from_raw("local-demo"),
        std::iter::empty::<String>(),
    )
    .expect("fixed workspace identity")
}

fn relative_path(path: &WorkspacePath) -> String {
    path.components()
        .iter()
        .map(|part| part.as_str())
        .collect::<Vec<_>>()
        .join("/")
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fragments(text: &str) -> Vec<KnowledgeSourceFragment> {
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let mut end = start + 1;
        let mut size = lines[start].len();
        while end < lines.len() && size + lines[end].len() <= 1500 {
            size += lines[end].len();
            end += 1;
        }
        let mut exact = lines[start..end].concat();
        // Internal newlines are exact; omit the last LF, outside the exclusive end column.
        if exact.ends_with('\n') {
            exact.pop();
        }
        if !exact.trim().is_empty() {
            let final_line = lines[end - 1].trim_end_matches('\n');
            output.push(KnowledgeSourceFragment {
                kind: KnowledgeSourceFragmentKind::Body,
                text: exact,
                source_range: ObsidianSourceRange {
                    start_line: start as u32 + 1,
                    start_column: 1,
                    end_line: end as u32,
                    end_column: final_line.len() as u32 + 1,
                },
                fact_key: None,
            });
        }
        start = end;
    }
    output
}

fn query_terms(question: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "a",
        "an",
        "and",
        "are",
        "as",
        "at",
        "be",
        "by",
        "can",
        "did",
        "do",
        "does",
        "for",
        "from",
        "had",
        "has",
        "have",
        "how",
        "i",
        "in",
        "is",
        "it",
        "me",
        "my",
        "of",
        "on",
        "or",
        "our",
        "please",
        "s",
        "tell",
        "that",
        "the",
        "their",
        "there",
        "these",
        "they",
        "this",
        "to",
        "us",
        "was",
        "we",
        "were",
        "what",
        "when",
        "where",
        "which",
        "who",
        "why",
        "will",
        "with",
        "would",
        "you",
        "your",
        "about",
        "documents",
        "document",
        "selected",
        "according",
    ];
    question
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| term.len() >= 2 && term.len() <= 128 && !STOP.contains(term))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-demo-docs-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn ingest(&self, runtime: &mut DemoDocuments) -> Value {
            runtime.execute(&json!({"op":"ingest","folder":self.0}))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn snapshots_retrieve_exact_sources_and_validate_membership() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.0.join("launch.md"),
            "# Lark project\nLark launches on 12 October 2026.\nOwner: Ada.\n",
        )
        .unwrap();
        std::fs::write(
            fixture.0.join("budget.txt"),
            "Lark budget is 4200 credits.\n",
        )
        .unwrap();
        let mut runtime = DemoDocuments::default();
        assert_eq!(fixture.ingest(&mut runtime)["accepted_count"], 2);
        std::fs::write(fixture.0.join("launch.md"), "Replaced after capture.\n").unwrap();
        let response = runtime.execute(&json!({"op":"query","question":"When does Lark launch?"}));
        assert_eq!(response["evidence_state"], "observed");
        assert!(
            response["context"]
                .as_str()
                .unwrap()
                .contains("12 October 2026")
        );
        assert_eq!(response["citations"][1]["start_line"], 1);
        assert_eq!(
            response["citations"][1]["text"],
            "# Lark project\nLark launches on 12 October 2026.\nOwner: Ada."
        );
        assert_eq!(
            response["citations"][1]["content_sha256"],
            digest(b"# Lark project\nLark launches on 12 October 2026.\nOwner: Ada.\n")
        );
        assert_eq!(
            runtime.execute(
                &json!({"op":"validate","answer":"12 October [S2]","citation_ids":["S2"]})
            )["ok"],
            true
        );
        assert_eq!(
            runtime.execute(&json!({"op":"validate","answer":"Fake [S99]","citation_ids":["S99"]}))
                ["ok"],
            false
        );
        assert_eq!(
            runtime.execute(&json!({"op":"query","question":"Who invented superconductivity?"}))["evidence_state"],
            "unknown_blocked"
        );
        let summary =
            runtime.execute(&json!({"op":"query","question":"Summarize the selected documents."}));
        assert_eq!(summary["retrieval"], "canonical_bounded_document_summary");
        assert_eq!(summary["citations"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn boundary_links_hidden_binary_and_unsupported_are_reported() {
        let fixture = Fixture::new();
        let outside = Fixture::new();
        std::fs::write(outside.0.join("secret.txt"), "Outside material.").unwrap();
        symlink(outside.0.join("secret.txt"), fixture.0.join("escape.txt")).unwrap();
        symlink(&outside.0, fixture.0.join("escape-dir")).unwrap();
        std::fs::write(fixture.0.join(".hidden.txt"), "Excluded.").unwrap();
        std::fs::write(fixture.0.join("binary.txt"), [255, 254]).unwrap();
        std::fs::write(fixture.0.join("report.pdf"), "Unsupported.").unwrap();
        std::fs::write(fixture.0.join("ok.txt"), "Admitted.").unwrap();
        let mut runtime = DemoDocuments::default();
        let response = fixture.ingest(&mut runtime);
        assert_eq!(response["accepted_count"], 1);
        assert_eq!(response["files"].as_array().unwrap().len(), 6);
        let rejected = response["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["status"] == "rejected")
            .count();
        assert_eq!(rejected, 3);
        let traversal =
            runtime.execute(&json!({"op":"ingest","folder":format!("{}/../",fixture.0.display())}));
        assert_eq!(traversal["ok"], false);
        assert!(runtime.documents.is_empty());
    }

    #[test]
    fn inventory_limit_fails_closed_and_context_omission_is_visible() {
        let fixture = Fixture::new();
        let mut runtime = DemoDocuments::default();
        for index in 0..20 {
            std::fs::write(
                fixture.0.join(format!("{index:03}.txt")),
                format!("Lark {}\n", "facts ".repeat(180)),
            )
            .unwrap();
        }
        assert_eq!(fixture.ingest(&mut runtime)["ok"], true);
        let response = runtime.execute(&json!({"op":"query","question":"Lark facts"}));
        assert_eq!(response["evidence_state"], "incomplete");
        assert!(response["omitted_count"].as_u64().unwrap() > 0);
        for index in 20..201 {
            std::fs::write(fixture.0.join(format!("{index:03}.txt")), "Lark.").unwrap();
        }
        assert_eq!(fixture.ingest(&mut runtime)["ok"], false);
        assert!(runtime.documents.is_empty());
    }

    #[test]
    fn file_total_depth_and_selected_root_bounds_are_enforced() {
        let fixture = Fixture::new();
        let mut runtime = DemoDocuments::default();
        std::fs::write(fixture.0.join("large.txt"), vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
        let response = fixture.ingest(&mut runtime);
        assert_eq!(response["accepted_count"], 0);
        assert_eq!(response["files"][0]["status"], "rejected");
        std::fs::remove_file(fixture.0.join("large.txt")).unwrap();
        for index in 0..10 {
            std::fs::write(
                fixture.0.join(format!("{index}.txt")),
                "Lark facts.\n".repeat(10_000),
            )
            .unwrap();
        }
        assert_eq!(fixture.ingest(&mut runtime)["ok"], false);
        assert!(runtime.documents.is_empty());
        let deep = Fixture::new();
        let mut nested = deep.0.clone();
        for _ in 0..9 {
            nested = nested.join("nested");
            std::fs::create_dir(&nested).unwrap();
        }
        assert_eq!(deep.ingest(&mut runtime)["ok"], false);
        let alias = fixture.0.join("selected-alias");
        symlink(&deep.0, &alias).unwrap();
        assert_eq!(
            runtime.execute(&json!({"op":"ingest","folder":alias}))["ok"],
            false
        );
    }

    #[test]
    fn multiline_source_ranges_extract_exact_utf8_bytes() {
        let text = format!(
            "Ada café\r\n\n{}\nLast line.\n",
            "Bounded line.\n".repeat(220)
        );
        let lines = text.split_inclusive('\n').collect::<Vec<_>>();
        for fragment in fragments(&text) {
            let range = fragment.source_range;
            let start = lines[..range.start_line as usize - 1]
                .iter()
                .map(|line| line.len())
                .sum::<usize>()
                + range.start_column as usize
                - 1;
            let end = lines[..range.end_line as usize - 1]
                .iter()
                .map(|line| line.len())
                .sum::<usize>()
                + range.end_column as usize
                - 1;
            assert_eq!(&text[start..end], fragment.text);
        }
    }
}
