# Story 11.2 storage crash-boundary evidence

Sub-task 11.2.4.1 is locally complete on Linux. The canonical operational-store subprocess crash
campaign now forces process termination immediately before and after manifest, extraction, lexical
index, workflow attempt, workflow receipt, verification, recovery-decision, checkpoint, retention
expiry, and deletion commits. The expanded campaign retains the existing transaction, session
checkpoint, migration, key retrieval, backup, and restore boundaries as regression coverage.

All 16 boundaries run at both crash positions with seven deterministic seeds per position, for 224
forced-stop cases and 32 exact boundary-position cells. After each abrupt child exit, recovery
observes either the old or new commit state, completes an absent transition once, rejects duplicate
publication, and finishes with exactly one target record. The child must terminate with the fixed
crash exit code; panic, ordinary failure, or an unexpectedly completed process fails the campaign.

This evidence does not yet claim every cross-table old/new-state invariant in Sub-task 11.2.4.2,
the retained trace and RV mappings in 11.2.4.3, Story or Sprint completion, platform coverage,
packaging readiness, or release readiness. Exact commands and hashes are retained in
`artifacts/sprints/sprint-11/story-11.2/storage-crash-boundary-report.json`; combined output is
retained beside it in `storage-crash-boundary-results.log`.
