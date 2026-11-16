// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic inventory workers exercise deadline/cancellation forwarding.

use super::*;
use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
use std::time::Duration;

#[test]
fn inventory_deadline_spans_all_reads_including_detached_head_and_final_result() {
    for stop in [
        None,
        Some(0),
        Some(1),
        Some(2),
        Some(3),
        Some(4),
        Some(5),
        Some(6),
        Some(7),
    ] {
        for with_token in [false, true] {
            let token = CancellationToken::root(
                BoundaryKind::Shell,
                TaskId::from_raw("inventory-deadline"),
                CorrelationId::from_raw("inventory-deadline"),
            );
            let cancellation = with_token.then_some(&token);
            let deadline = Instant::now()
                + if stop == Some(0) {
                    Duration::ZERO
                } else {
                    Duration::from_millis(100)
                };
            let mut calls = 0;
            let result = collect_inventory_with(
                "repository",
                cancellation,
                Some(deadline),
                |args, _, forwarded, until| {
                    calls += 1;
                    assert_eq!(until, Some(deadline));
                    assert_eq!(
                        forwarded.map(|t| t as *const _),
                        cancellation.map(|t| t as *const _)
                    );
                    if stop == Some(calls) {
                        while let Some(remaining) = deadline.checked_duration_since(Instant::now())
                        {
                            std::thread::sleep(remaining);
                        }
                    }
                    Ok(match args {
                        ["rev-parse", "--show-object-format"] => Some(b"sha1\n".to_vec()),
                        ["rev-parse", "--verify", "HEAD"] => {
                            Some(format!("{}\n", "a".repeat(40)).into_bytes())
                        }
                        ["symbolic-ref", "--quiet", "HEAD"] => None,
                        ["ls-files", ..] | ["ls-tree", ..] => Some(Vec::new()),
                        _ => panic!("unexpected inventory worker"),
                    })
                },
            );
            assert_eq!(calls, stop.unwrap_or(7));
            if stop.is_some() {
                assert_eq!(
                    result.unwrap_err().kind(),
                    LinuxRepositoryErrorKind::ObservationFailed
                );
            } else {
                let result = result.unwrap();
                assert_eq!(result.repository_sha256, "repository");
                assert_eq!(result.object_format, "sha1");
                assert_eq!(result.commit_id, "a".repeat(40));
                assert_eq!(result.branch, None);
                assert!(result.entries.is_empty());
                assert_eq!(
                    result.git_evidence_sha256,
                    framed_digest(&[
                        b"sha1",
                        "a".repeat(40).as_bytes(),
                        b"detached",
                        b"",
                        b"",
                        b"",
                        b""
                    ])
                    .unwrap()
                );
            }
            assert!(!token.is_cancelled());
        }
    }
}
