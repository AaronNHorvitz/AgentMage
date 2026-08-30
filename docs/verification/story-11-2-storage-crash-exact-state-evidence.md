# Story 11.2 storage crash exact-state evidence

Sub-task 11.2.4.2 is locally complete on Linux. The 224-case operational-store subprocess campaign
now validates the complete old-or-new state around every new source and workflow commit. Recovery
rejects any target count outside zero or one, completes the old state once, refuses a duplicate,
and verifies the final new state.

Manifest, provenance, and materialization-head rows publish together. Extraction crashes cannot
publish a lexical index; index publication retains its exact cache authority. Workflow attempt
cardinality remains zero or one, and receipt, verification, and recovery publication preserve one
existing attempt. Current-source views contain no non-current materialization. Recovery also proves
that no effect-driver launch marker exists, so reconstructing durable state cannot replay an effect.
The pre-existing checkpoint, retention, deletion, migration, key, backup, restore, and transaction
assertions continue to require their exact old or new state in the same campaign.

This evidence does not claim the retained trace, encrypted-page scan, cleanup evidence, and RV
mappings in Sub-task 11.2.4.3, Story or Sprint completion, platform coverage, packaging readiness,
or release readiness. Exact commands and hashes are retained in
`artifacts/sprints/sprint-11/story-11.2/storage-crash-exact-state-report.json`; combined output is
retained beside it in `storage-crash-exact-state-results.log`.
