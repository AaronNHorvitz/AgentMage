// SPDX-License-Identifier: Apache-2.0
//! Bounded inert composition of original write projections, with no new approval.

use super::*;

/// Composes two to sixty-four ordered original transactions into one net diff.
/// Original approvals and applications remain in `write_history`; the composite
/// has neither a singular approval nor any write or Git authority. Structural
/// integrity alone never authenticates these records as native execution.
pub fn capture_reviewable_write_history(
    history: Vec<ReviewableChangeSet>,
) -> Result<ReviewableChangeSet, ReviewPacketError> {
    use ReviewPacketError::ChangeSetMismatch;
    if !(2..=64).contains(&history.len()) {
        return Err(ChangeSetMismatch);
    }
    let scope = history[0].scope;
    let mut bytes = 0_usize;
    let mut files = BTreeMap::<String, (ReviewChangedFile, Vec<String>)>::new();
    let mut applications = BTreeSet::new();
    let mut hooks = BTreeSet::new();
    for transaction in &history {
        // Histories are flat. Refuse before recursive verification or copying.
        if !transaction.write_history.is_empty()
            || transaction.scope != scope
            || !verify_single_change_set(transaction)
            // A retry can apply the same content/intent again after a rollback.
            // Its checked application must be new; content identity alone does
            // not identify an execution. Reusing any application still refuses.
            || !applications.insert(&transaction.write_application_receipt_sha256)
        {
            return Err(ChangeSetMismatch);
        }
        hooks.extend(transaction.review_hooks.iter().copied());
        for file in &transaction.files {
            bytes = bytes
                .checked_add(file.complete_diff.len())
                .filter(|bytes| *bytes <= MAX_DIFF_BYTES)
                .ok_or(ChangeSetMismatch)?;
            // Exact complete diffs contain five physical lines: embedded newlines
            // are escaped. Validate the fixed headers before composing endpoints.
            let lines = diff_lines(file).ok_or(ChangeSetMismatch)?;
            match files.get_mut(&file.path) {
                None => {
                    files.insert(
                        file.path.clone(),
                        (file.clone(), vec![file.operation_sha256.clone()]),
                    );
                }
                Some((net, operations)) => {
                    if net.postimage_sha256 != file.preimage_sha256
                        || net.artifact_class != file.artifact_class
                        || net.generated != file.generated
                        || net.purpose != file.purpose
                    {
                        return Err(ChangeSetMismatch);
                    }
                    let first = diff_lines(net).ok_or(ChangeSetMismatch)?;
                    net.complete_diff = format!(
                        "{}\n{}\n{}\n{}\n{}\n",
                        first[0], lines[1], lines[2], first[3], lines[4]
                    );
                    net.postimage_sha256 = file.postimage_sha256.clone();
                    operations.push(file.operation_sha256.clone());
                }
            }
        }
    }
    if files.is_empty() || files.len() > MAX_CHANGED_FILES {
        return Err(ChangeSetMismatch);
    }
    let files = files
        .into_values()
        .enumerate()
        .map(|(i, (mut file, operations))| {
            file.operation_id = format!("history-{i}");
            file.operation_sha256 = sha256_json(&operations)?;
            // Net-zero paths remain unsupported, rather than presenting them as edits.
            if file.preimage_sha256 == file.postimage_sha256 {
                return Err(ChangeSetMismatch);
            }
            Ok(file)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ReviewableChangeSet {
        change_set_id: "write-history".into(),
        change_set_sha256: sha256_json(&history)?,
        intent_sha256: sha256_json(&history.iter().map(|h| &h.intent_sha256).collect::<Vec<_>>())?,
        plan_sha256: sha256_json(&history.iter().map(|h| &h.plan_sha256).collect::<Vec<_>>())?,
        write_approval_receipt_sha256: None,
        write_application_receipt_sha256: None,
        write_history: history,
        // Preserve the shared classification; differing breadth refuses.
        scope,
        review_hooks: hooks.into_iter().collect(),
        files_sha256: sha256_json(&files)?,
        files,
        write_authority: false,
    })
}

fn diff_lines(file: &ReviewChangedFile) -> Option<Vec<&str>> {
    let lines = file.complete_diff.lines().collect::<Vec<_>>();
    (lines.len() == 5
        && lines[0] == format!("--- {} sha256:{}", file.path, file.preimage_sha256)
        && lines[1] == format!("+++ {} sha256:{}", file.path, file.postimage_sha256)
        && lines[2] == "@@ exact-complete-postimage @@"
        && lines[3].starts_with("-\"")
        && lines[3].ends_with('"')
        && lines[4].starts_with("+\"")
        && lines[4].ends_with('"'))
    .then_some(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(id: &str, path: &str, before: &str, after: &str) -> ReviewableChangeSet {
        let hash = |s: &str| sha256_hex(s.as_bytes());
        let file = ReviewChangedFile {
            operation_id: "edit-0".into(),
            path: path.into(),
            preimage_sha256: hash(before),
            postimage_sha256: hash(after),
            artifact_class: ReviewArtifactClass::Code,
            purpose: CommitChangePurpose::Behavior,
            generated: false,
            complete_diff: format!(
                "--- {path} sha256:{}\n+++ {path} sha256:{}\n@@ exact-complete-postimage @@\n-{before:?}\n+{after:?}\n",
                hash(before),
                hash(after)
            ),
            operation_sha256: hash(id),
        };
        ReviewableChangeSet {
            change_set_id: id.into(),
            change_set_sha256: hash(id),
            intent_sha256: hash("intent"),
            plan_sha256: hash("plan"),
            write_approval_receipt_sha256: Some(hash(&format!("approval-{id}"))),
            write_application_receipt_sha256: Some(hash(&format!("application-{id}"))),
            write_history: vec![],
            scope: WriteChangeScope::Minimal,
            review_hooks: vec![WriteReviewHook::Security],
            files_sha256: sha256_json(&vec![file.clone()]).unwrap(),
            files: vec![file],
            write_authority: false,
        }
    }

    #[test]
    fn write_history_composes_exact_endpoints_and_keeps_original_receipts() {
        let a = leaf("first", "source.rs", "original\n", "retry\n");
        let b = leaf("second", "other.rs", "other", "changed");
        let c = leaf("third", "source.rs", "retry\n", "final\n");
        let history = vec![a.clone(), b, c.clone()];
        let net = capture_reviewable_write_history(history.clone()).unwrap();
        assert!(verify_reviewable_change_set(&net));
        assert_eq!(net.write_history, history);
        assert_eq!(net.write_approval_receipt_sha256, None);
        assert_eq!(net.write_application_receipt_sha256, None);
        assert!(!net.write_authority);
        assert_eq!(net.files.len(), 2);
        let file = &net.files[1];
        assert_eq!(file.operation_id, "history-1");
        assert_eq!(file.preimage_sha256, a.files[0].preimage_sha256);
        assert_eq!(file.postimage_sha256, c.files[0].postimage_sha256);
        assert!(
            file.complete_diff
                .contains("-\"original\\n\"\n+\"final\\n\"")
        );
        assert!(!file.complete_diff.contains("retry"));
        for which in ["file", "history", "order", "approval", "digest"] {
            let mut bad = net.clone();
            match which {
                "file" => bad.files[1].complete_diff.push_str("forged"),
                "history" => {
                    bad.write_history.remove(0);
                }
                "order" => bad.write_history.swap(0, 2),
                "approval" => {
                    bad.write_approval_receipt_sha256 = a.write_approval_receipt_sha256.clone()
                }
                "digest" => bad.change_set_sha256 = a.change_set_sha256.clone(),
                _ => unreachable!(),
            }
            assert!(!verify_reviewable_change_set(&bad), "{which}");
        }
    }

    #[test]
    fn write_history_refuses_gaps_duplicates_nesting_bounds_and_incompatible_files() {
        let a = leaf("first", "source.rs", "original", "retry");
        let b = leaf("second", "source.rs", "retry", "final");
        assert!(capture_reviewable_write_history(vec![]).is_err());
        assert!(capture_reviewable_write_history(vec![a.clone()]).is_err());
        assert!(capture_reviewable_write_history(vec![a.clone(); 65]).is_err());
        assert!(capture_reviewable_write_history(vec![a.clone(), a.clone()]).is_err());
        assert!(capture_reviewable_write_history(vec![b.clone(), a.clone()]).is_err());
        let net = capture_reviewable_write_history(vec![a.clone(), b.clone()]).unwrap();
        assert!(capture_reviewable_write_history(vec![net, b.clone()]).is_err());
        for which in ["gap", "purpose", "scope", "diff", "net-zero", "budget"] {
            let mut bad = b.clone();
            match which {
                "gap" => bad = leaf("gap", "source.rs", "unknown", "final"),
                "purpose" => bad.files[0].purpose = CommitChangePurpose::Formatting,
                "scope" => bad.scope = WriteChangeScope::Refactor,
                "diff" => bad.files[0].complete_diff.push_str("extra\n"),
                "net-zero" => bad = leaf("zero", "source.rs", "retry", "original"),
                "budget" => bad = leaf("large", "source.rs", "retry", &"x".repeat(MAX_DIFF_BYTES)),
                _ => unreachable!(),
            }
            bad.files_sha256 = sha256_json(&bad.files).unwrap();
            assert!(
                capture_reviewable_write_history(vec![a.clone(), bad]).is_err(),
                "{which}"
            );
        }
    }

    #[test]
    fn write_history_accepts_sixty_four_and_bounds_cumulative_diff_bytes() {
        let history = (0..64)
            .map(|i| {
                leaf(
                    &format!("transaction-{i}"),
                    "source.rs",
                    &format!("version-{i}"),
                    &format!("version-{}", i + 1),
                )
            })
            .collect();
        let net = capture_reviewable_write_history(history).unwrap();
        assert_eq!(net.write_history.len(), 64);
        assert!(verify_reviewable_change_set(&net));
        // Each leaf is independently valid and smaller than the limit; their
        // complete history exceeds it. No truncated history is returned.
        let large = "x".repeat(MAX_DIFF_BYTES / 2);
        let a = leaf("large-a", "source.rs", "original", &large);
        let b = leaf("large-b", "source.rs", &large, "final");
        assert!(verify_single_change_set(&a));
        assert!(verify_single_change_set(&b));
        assert!(capture_reviewable_write_history(vec![a, b]).is_err());
    }

    #[test]
    fn write_history_retry_cycle_distinguishes_content_from_application_identity() {
        let first = leaf("same-content", "source.rs", "original", "changed");
        let rollback = leaf("rollback", "source.rs", "changed", "original");
        let mut retry = first.clone();
        retry.write_approval_receipt_sha256 = Some(sha256_hex(b"new original approval"));
        retry.write_application_receipt_sha256 = Some(sha256_hex(b"new checked application"));
        let net =
            capture_reviewable_write_history(vec![first.clone(), rollback.clone(), retry.clone()])
                .unwrap();
        assert!(verify_reviewable_change_set(&net));
        assert_eq!(net.write_history, [first.clone(), rollback.clone(), retry]);
        assert_eq!(net.files[0].preimage_sha256, sha256_hex(b"original"));
        assert_eq!(net.files[0].postimage_sha256, sha256_hex(b"changed"));
        assert_eq!(net.files[0].complete_diff, first.files[0].complete_diff);
        assert!(net.write_approval_receipt_sha256.is_none());
        assert!(net.write_application_receipt_sha256.is_none());
        assert!(!net.write_authority);

        println!(
            "retry_cycle_projection_json={}",
            serde_json::to_string(&net).unwrap()
        );

        // An exact replay is still a duplicate, even with a distinct change-set
        // digest or approval. Resealing the surrounding history cannot hide it.
        for substitute_content_identity in [false, true] {
            let mut replay = first.clone();
            if substitute_content_identity {
                replay.change_set_sha256 = sha256_hex(b"substituted content identity");
                replay.write_approval_receipt_sha256 = Some(sha256_hex(b"other approval"));
            }
            assert!(
                capture_reviewable_write_history(vec![first.clone(), rollback.clone(), replay])
                    .is_err()
            );
        }
        // The temporary return to the original bytes still cannot be presented
        // as a completed net change. It may remain inside a later nonzero history.
        assert!(capture_reviewable_write_history(vec![first, rollback]).is_err());
    }
}
