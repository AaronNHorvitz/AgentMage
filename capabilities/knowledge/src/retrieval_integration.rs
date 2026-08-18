//! Raw-source and disposable-index parity for deterministic knowledge answers.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::WorkspaceScopePath;

use crate::{
    KnowledgeAnswerDraft, KnowledgeContextQuery, KnowledgeEvidenceState, KnowledgeRenderedAnswer,
    KnowledgeRetrievalError, KnowledgeRetrievalResult, KnowledgeSourceAuthority,
    ObsidianAccessReceipt, ObsidianIndexError, ObsidianVaultFreshness, ObsidianVaultIndex,
    ObsidianVaultSnapshot, prepare_knowledge_synthesis, render_knowledge_answer,
    retrieval_documents_from_snapshot, retrieve_knowledge,
};

/// Content-free failure while reconciling raw and rebuilt-index retrieval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsidianRetrievalIntegrationError {
    /// The expected citation set or source/index relationship was invalid.
    InvalidInput,
    /// The disposable index did not represent the supplied canonical snapshot.
    StaleIndex,
    /// The disposable index could not be read or verified.
    Index(ObsidianIndexError),
    /// The deterministic retrieval or rendering contract refused the input.
    Retrieval(KnowledgeRetrievalError),
}

impl From<ObsidianIndexError> for ObsidianRetrievalIntegrationError {
    fn from(value: ObsidianIndexError) -> Self {
        Self::Index(value)
    }
}

impl From<KnowledgeRetrievalError> for ObsidianRetrievalIntegrationError {
    fn from(value: KnowledgeRetrievalError) -> Self {
        Self::Retrieval(value)
    }
}

/// Exact raw-source, rebuilt-index, expected-citation, and answer comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObsidianQuestionParityReport {
    /// Retrieval executed over canonical parsed Markdown.
    pub raw_result: KnowledgeRetrievalResult,
    /// Retrieval executed over independently read derived SQLite rows.
    pub rebuilt_index_result: KnowledgeRetrievalResult,
    /// Deterministic cited answer rendered from canonical parsed Markdown.
    pub raw_answer: KnowledgeRenderedAnswer,
    /// Deterministic cited answer rendered from rebuilt SQLite rows.
    pub rebuilt_index_answer: KnowledgeRenderedAnswer,
    /// Expected citations absent from canonical-source results.
    pub expected_missing_from_raw: Vec<String>,
    /// Expected citations absent from rebuilt-index results.
    pub expected_missing_from_index: Vec<String>,
    /// Canonical-source citations not declared by the labeled fixture.
    pub unexpected_in_raw: Vec<String>,
    /// Rebuilt-index citations not declared by the labeled fixture.
    pub unexpected_in_index: Vec<String>,
    /// Canonical-source citations absent from the rebuilt-index result.
    pub missing_from_index: Vec<String>,
    /// Rebuilt-index citations absent from the canonical-source result.
    pub index_only: Vec<String>,
    /// Content-free receipt for reading the disposable index projection.
    pub index_access_receipt: ObsidianAccessReceipt,
}

impl ObsidianQuestionParityReport {
    /// Reports exact expected-set and raw-versus-index citation parity.
    #[must_use]
    pub fn exact_match(&self) -> bool {
        self.expected_missing_from_raw.is_empty()
            && self.expected_missing_from_index.is_empty()
            && self.unexpected_in_raw.is_empty()
            && self.unexpected_in_index.is_empty()
            && self.missing_from_index.is_empty()
            && self.index_only.is_empty()
            && self.raw_answer.citation_sha256 == self.rebuilt_index_answer.citation_sha256
            && self.raw_answer.evidence_state == self.rebuilt_index_answer.evidence_state
    }
}

/// Answers one labeled question from raw notes and a rebuilt index, preserving every blind spot.
pub fn evaluate_obsidian_question_parity(
    snapshot: &ObsidianVaultSnapshot,
    index: &mut ObsidianVaultIndex,
    root: &WorkspaceScopePath,
    verified_on: &str,
    default_source_date: &str,
    query: &KnowledgeContextQuery,
    expected_citation_sha256: &[String],
) -> Result<ObsidianQuestionParityReport, ObsidianRetrievalIntegrationError> {
    let expected = citation_set(expected_citation_sha256)?;
    if index.freshness(snapshot)? != ObsidianVaultFreshness::Current {
        return Err(ObsidianRetrievalIntegrationError::StaleIndex);
    }
    let raw_documents =
        retrieval_documents_from_snapshot(snapshot, root, verified_on, default_source_date)?;
    let indexed = index.retrieval_documents(root, verified_on, default_source_date)?;

    let mut raw_query = query.clone();
    raw_query.authorities = BTreeSet::from([KnowledgeSourceAuthority::CanonicalMarkdown]);
    let mut index_query = query.clone();
    index_query.authorities = BTreeSet::from([KnowledgeSourceAuthority::DerivedProjection]);
    let raw_result = retrieve_knowledge(&raw_query, &raw_documents)?;
    let rebuilt_index_result = retrieve_knowledge(&index_query, &indexed.documents)?;
    let raw_answer = render_extractive_answer(&raw_result)?;
    let rebuilt_index_answer = render_extractive_answer(&rebuilt_index_result)?;
    let raw = result_citations(&raw_result);
    let rebuilt = result_citations(&rebuilt_index_result);

    Ok(ObsidianQuestionParityReport {
        expected_missing_from_raw: difference(&expected, &raw),
        expected_missing_from_index: difference(&expected, &rebuilt),
        unexpected_in_raw: difference(&raw, &expected),
        unexpected_in_index: difference(&rebuilt, &expected),
        missing_from_index: difference(&raw, &rebuilt),
        index_only: difference(&rebuilt, &raw),
        raw_result,
        rebuilt_index_result,
        raw_answer,
        rebuilt_index_answer,
        index_access_receipt: indexed.receipt,
    })
}

fn render_extractive_answer(
    result: &KnowledgeRetrievalResult,
) -> Result<KnowledgeRenderedAnswer, KnowledgeRetrievalError> {
    let envelope = prepare_knowledge_synthesis(result);
    let (text, citation_sha256) = if result.evidence_state == KnowledgeEvidenceState::UnknownBlocked
    {
        (String::new(), Vec::new())
    } else {
        let entry = result
            .context
            .first()
            .ok_or(KnowledgeRetrievalError::EvidenceDrift)?;
        (entry.text.clone(), vec![entry.citation_sha256.clone()])
    };
    render_knowledge_answer(
        &envelope,
        &KnowledgeAnswerDraft {
            evidence_state: result.evidence_state,
            text,
            citation_sha256,
        },
    )
}

fn citation_set(values: &[String]) -> Result<BTreeSet<String>, ObsidianRetrievalIntegrationError> {
    let set = values.iter().cloned().collect::<BTreeSet<_>>();
    if set.len() != values.len()
        || set.iter().any(|value| {
            value.len() != 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        })
    {
        return Err(ObsidianRetrievalIntegrationError::InvalidInput);
    }
    Ok(set)
}

fn result_citations(result: &KnowledgeRetrievalResult) -> BTreeSet<String> {
    result
        .hits
        .iter()
        .map(|hit| hit.citation_sha256.clone())
        .collect()
}

fn difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        StorageFilesystemClass, StrictLocalStorageObservation, WorkspaceId, WorkspacePath,
        WorkspaceScopePath,
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        KnowledgeFileType, ObsidianEntryKind, ObsidianNoteInput, ObsidianVaultSelection,
        retrieval_documents_from_snapshot,
    };

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-retrieval-integration")
    }

    fn root() -> WorkspaceScopePath {
        WorkspaceScopePath::new(workspace(), ["Vault"]).expect("root")
    }

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(workspace(), ["Vault", name]).expect("path")
    }

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn input(name: &str, content: &str) -> ObsidianNoteInput {
        ObsidianNoteInput {
            path: path(name),
            entry_kind: ObsidianEntryKind::RegularFile,
            hidden: false,
            cloud_synchronized: false,
            content_sha256: sha256(content.as_bytes()),
            content: content.as_bytes().to_vec(),
        }
    }

    fn snapshot(extra: Option<(&str, &str)>) -> ObsidianVaultSnapshot {
        let selection = ObsidianVaultSelection::admit(
            root(),
            StrictLocalStorageObservation {
                filesystem: StorageFilesystemClass::Local,
                synchronization_marker: None,
                root_identity_sha256: [7; 32],
                symlink_free: true,
            },
            Vec::new(),
        )
        .expect("selection");
        let mut inputs = vec![
            input(
                "handoff.md",
                "---\ntype: handoff\nstatus: current\n---\n# Release owner Alice\n",
            ),
            input("roadmap.md", "# Roadmap\n- [ ] Next gate diagnostics\n"),
            input("evidence.md", "# Evidence\nverified: 2026-08-10\n"),
        ];
        if let Some((name, content)) = extra {
            inputs.push(input(name, content));
        }
        ObsidianVaultSnapshot::from_snapshots(&selection, inputs).expect("snapshot")
    }

    fn query(phrase: &str) -> KnowledgeContextQuery {
        KnowledgeContextQuery {
            terms: Vec::new(),
            phrases: vec![phrase.to_owned()],
            roots: vec![root()],
            date_from: None,
            date_to: None,
            as_of_date: "2026-08-18".to_owned(),
            file_types: BTreeSet::from([KnowledgeFileType::Markdown]),
            authorities: BTreeSet::from([KnowledgeSourceAuthority::CanonicalMarkdown]),
            include_historical: false,
            max_results: 8,
            max_context_bytes: 4096,
        }
    }

    fn expected(snapshot: &ObsidianVaultSnapshot, query: &KnowledgeContextQuery) -> Vec<String> {
        let documents =
            retrieval_documents_from_snapshot(snapshot, &root(), "2026-08-18", "2026-08-10")
                .expect("raw projection");
        retrieve_knowledge(query, &documents)
            .expect("raw retrieval")
            .hits
            .into_iter()
            .map(|hit| hit.citation_sha256)
            .collect()
    }

    #[test]
    fn labeled_raw_and_rebuilt_index_answers_have_exact_citation_parity() {
        let snapshot = snapshot(None);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&snapshot).expect("rebuild");
        for phrase in ["release owner", "next gate", "verified"] {
            let query = query(phrase);
            let expected = expected(&snapshot, &query);
            let report = evaluate_obsidian_question_parity(
                &snapshot,
                &mut index,
                &root(),
                "2026-08-18",
                "2026-08-10",
                &query,
                &expected,
            )
            .expect("parity");
            assert!(report.exact_match());
            assert_eq!(report.raw_answer.text, report.rebuilt_index_answer.text);
            assert!(!report.index_access_receipt.source_files_mutated);
            assert!(!report.index_access_receipt.external_process_started);
            assert!(!report.index_access_receipt.network_accessed);
        }
    }

    #[test]
    fn absent_expected_evidence_is_reported_as_a_blind_spot() {
        let snapshot = snapshot(None);
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&snapshot).expect("rebuild");
        let report = evaluate_obsidian_question_parity(
            &snapshot,
            &mut index,
            &root(),
            "2026-08-18",
            "2026-08-10",
            &query("not present"),
            &["a".repeat(64)],
        )
        .expect("blind spot report");
        assert!(!report.exact_match());
        assert_eq!(report.expected_missing_from_raw, vec!["a".repeat(64)]);
        assert_eq!(report.expected_missing_from_index, vec!["a".repeat(64)]);
        assert_eq!(report.raw_answer.state_label, "Unknown/Blocked");
        assert_eq!(report.rebuilt_index_answer.state_label, "Unknown/Blocked");
    }

    #[test]
    fn stale_index_and_invalid_expected_citations_fail_closed() {
        let first = snapshot(None);
        let second = snapshot(Some(("new.md", "# New evidence\n")));
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        index.rebuild(&first).expect("rebuild");
        assert_eq!(
            evaluate_obsidian_question_parity(
                &second,
                &mut index,
                &root(),
                "2026-08-18",
                "2026-08-10",
                &query("new evidence"),
                &[],
            ),
            Err(ObsidianRetrievalIntegrationError::StaleIndex)
        );
        assert_eq!(
            evaluate_obsidian_question_parity(
                &first,
                &mut index,
                &root(),
                "2026-08-18",
                "2026-08-10",
                &query("release owner"),
                &["not-a-digest".to_owned()],
            ),
            Err(ObsidianRetrievalIntegrationError::InvalidInput)
        );
    }
}
