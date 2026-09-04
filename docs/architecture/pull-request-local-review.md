# Pull-Request Local Review

Sprint 74 layers a pure review projection over the existing owned-worktree boundary. A binding
fixes repository/worktree identity, exact base, head and merge-base commits, optional fork,
changed-path/dependency/instruction/test digests, added-commit count, base movement, and verified
preservation of the active checkout and remote state.

Findings cover correctness, security, data, accessibility, performance, dependencies, and tests.
Every finding binds path, line range, commit and evidence and is labeled current, stale, moved,
resolved, superseded, conflicting, uncertain, or locally unverifiable. Formatting-only, duplicate,
low-confidence, deterministic-check-enforced, and non-current findings are suppressed. Suggested
fixes contain full diff, tests, risks, and rollback only as unapplied controlled-write proposals.
Submission, branch update, commit, push, merge, and release are fixed disabled.

This module neither fetches nor creates a worktree and does not run tests or a sandbox. Exact native
PR fetch/checkout, local review execution, hosted refresh, and no-publication observation remain
external campaigns.
