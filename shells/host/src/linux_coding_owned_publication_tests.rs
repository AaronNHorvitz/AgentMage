// SPDX-License-Identifier: MIT OR Apache-2.0
//! Component dispatch refusal, without fabricating a native worktree owner.

use super::{RuntimePortFailure, with_owned_terminal_publication};

#[test]
fn owned_creation_without_terminal_context_never_enters_effect_dispatch() {
    let mut grant_consumed = false;
    let mut available_paths = Vec::new();
    let result = with_owned_terminal_publication(true, false, || {
        // Synthetic sentinels for later operations, not native grant/lineage
        // observations. The real entry wrapper encloses the entire native body.
        grant_consumed = true;
        available_paths.push("new.py");
        Ok(())
    });
    assert_eq!(result, Err(RuntimePortFailure::Invalid));
    assert!(!grant_consumed);
    assert!(available_paths.is_empty());
}

#[test]
fn publication_admission_preserves_unowned_dispatch_and_propagates_failures() {
    for (owned, context) in [(false, false), (false, true), (true, true)] {
        let mut calls = 0;
        let result: Result<(), _> = with_owned_terminal_publication(owned, context, || {
            calls += 1;
            Err(RuntimePortFailure::Uncertain)
        });
        assert_eq!(calls, 1);
        assert_eq!(result, Err(RuntimePortFailure::Uncertain));
    }
}
