// SPDX-License-Identifier: Apache-2.0
//! Strict batch-size boundaries on caller-supplied snapshots; no execution.

use agentmage_capability_repository_map::coding_proposal::{
    CodingProposalError, CodingSource, check_coding_proposal, prepare_coding_proposal,
};
use serde_json::json;
use sha2::{Digest, Sha256};

const LIMIT: usize = 128 * 1024;

fn padded(path: &str, first_line: &str, bytes: usize) -> CodingSource {
    let mut content = format!("{first_line}\n");
    let remaining = bytes - content.len();
    content.push_str(&"p\n".repeat(remaining / 2));
    if remaining % 2 == 1 {
        content.push('p');
    }
    assert_eq!(content.len(), bytes);
    CodingSource {
        path: path.into(),
        content,
    }
}

fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn batch_growth_and_shrinkage_use_final_bytes_independent_of_order_and_path_names() {
    // Four removed ASCII bytes fund four added UTF-8 bytes. Sources start at
    // the unchanged aggregate ceiling; each resulting file remains below it.
    for (grow, shrink) in [("a.txt", "z.txt"), ("z.txt", "a.txt")] {
        let original = [
            padded(grow, "alpha", LIMIT / 2),
            padded(shrink, "trimβ", LIMIT / 2),
        ];
        for reverse_sources in [false, true] {
            let mut sources = original.clone();
            if reverse_sources {
                sources.reverse();
            }
            let context = prepare_coding_proposal("alpha trim", &sources, 4096).unwrap();
            for reverse_edits in [false, true] {
                for suffix in [" é", " é!"] {
                    let mut edits = vec![
                        json!({"path":grow,"old":"alpha","new":format!("alpha{suffix}")}),
                        json!({"path":shrink,"old":"trimβ","new":"β"}),
                    ];
                    if reverse_edits {
                        edits.reverse();
                    }
                    let before = serde_json::to_vec(&sources).unwrap();
                    let changes = check_coding_proposal(
                        &context,
                        &sources,
                        &json!({"edits":edits}).to_string(),
                    )
                    .unwrap();
                    assert_eq!(changes.len(), 2);
                    assert!(changes[0].path < changes[1].path);
                    assert_eq!(
                        changes.iter().map(|c| c.content.len()).sum::<usize>(),
                        LIMIT + suffix.len() - 4
                    );
                    for change in changes {
                        let source = original.iter().find(|s| s.path == change.path).unwrap();
                        let expected = if change.path == grow {
                            source
                                .content
                                .replacen("alpha", &format!("alpha{suffix}"), 1)
                        } else {
                            source.content.replacen("trimβ", "β", 1)
                        };
                        assert_eq!(change.content, expected);
                        assert_eq!(change.before_sha256, hash(&source.content));
                        assert_eq!(change.after_sha256, hash(&expected));
                    }
                    assert_eq!(serde_json::to_vec(&sources).unwrap(), before);
                }
            }
        }
    }
}

#[test]
fn final_batch_overflow_counts_unchanged_files_and_rejects_the_whole_proposal() {
    // Shrink an earlier file, grow a later file, and retain an unedited third
    // file. Checking only changed files would silently omit half the snapshot.
    let sources = [
        padded("a.txt", "trimβ", LIMIT / 4),
        padded("z.txt", "alpha", LIMIT / 4),
        padded("untouched.txt", "untouched", LIMIT / 2),
    ];
    let context = prepare_coding_proposal("alpha trim", &sources, 4096).unwrap();
    let before = serde_json::to_vec(&sources).unwrap();
    for reverse in [false, true] {
        let mut edits = vec![
            json!({"path":"a.txt","old":"trimβ","new":"β"}),
            json!({"path":"z.txt","old":"alpha","new":"alpha é!!"}),
        ];
        if reverse {
            edits.reverse();
        }
        // Five added bytes minus four removed bytes is exactly one too many.
        assert!(matches!(
            check_coding_proposal(&context, &sources, &json!({"edits":edits}).to_string()),
            Err(CodingProposalError::Edit)
        ));
    }
    // A balanced valid batch must still reject when its final edit is invalid.
    for bad in [
        json!({"path":"missing.txt","old":"alpha","new":"x"}),
        json!({"path":"z.txt","old":"alpha","new":"different"}),
    ] {
        let edits = json!({"edits":[{"path":"a.txt","old":"trimβ","new":"β"}, {"path":"z.txt","old":"alpha","new":"alpha é!"}, bad]});
        assert!(matches!(
            check_coding_proposal(&context, &sources, &edits.to_string()),
            Err(CodingProposalError::Edit)
        ));
    }
    assert_eq!(serde_json::to_vec(&sources).unwrap(), before);
}
