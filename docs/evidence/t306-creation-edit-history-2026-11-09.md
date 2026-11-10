# T-306: Preserve creation review refusal across later edit failures

This same-row repair implements the remaining creation-history detail under
Decision 0186, on immutable base
`8ca2251f23814bc2bb6f41cc2bdd1d7717a2661c`. That base is under independent
core review at session start. T-303/T-306 remain open; no new or closed row
or sub-row. Independent exact-commit core review is required.

During verification, the [base-preimage report](../reviews/8ca2251f2381-local-coding-base-preimages.txt)
returns **Verdict: PASS** for exact commit
`8ca2251f23814bc2bb6f41cc2bdd1d7717a2661c`, with no high or medium finding.
Low L1 existing-file mode binding and L2 whole-tree availability limits remain
same-row backlog. This report is also retained byte-identically.

The retained [review-refusal report](../reviews/81e600d0df9e-local-coding-review-refusals.txt)
has **Verdict: PASS** for exact commit
`81e600d0df9eb479452048c09cbbaba3aafe915f`, with no high or medium finding.
Its low L1 residual identifies published creation lineage lost by a later
failed manual edit or unpublished runtime edit. The reservation-timeout process
note remains same-row backlog. The report is retained byte-identically.

Before retiring native write history, both edit dispatch paths mark creation
review incomplete whenever that history covers a recorded creation. This also
covers edits of other paths: the approval history is shared. Failed or lost
completion, rejected completion metadata and unwinding cannot clear the guard.
Only checked committed completion with the preceding proof retained and the
combined history available restores the prior guard value. An earlier gap
remains sticky across later successful edits, creations and runtime handoffs.
The refusal is `code.commit.creation-provenance`.

Four new regression methods exercise actual confined native writes with
synthetic Git creation and terminal-publication observations. They cover manual
preimage drift; unpublished edits of created and unrelated paths; substituted
transaction, receipt and issued-grant metadata; and denial, expiry, stale
choices and abandonment followed by successful manual and runtime retries.
Later creation and handoff cannot hide a prior gap. Successful retries retain
the absent-before projection, including when an unrelated path is then edited.
An existing missing-proof regression additionally requires the sticky guard.
Original source, index and untracked sentinels stay intact.

The first complete gate fails with 1,312 passes, 73 failures and 64 existing
ignores across 26 unfiltered Rust targets. The extra failure is a new assertion
accidentally placed in the successful post-transfer control, where no creation
lineage remains. The assertion now guards the intended existing missing-proof
regression. All four new methods pass in that first run. Its raw log SHA-256 is
`86de6d39f93b6656c2289467e2c52c6037385fef2cc51ebd0cb2ce5c2b6e0583`.
The complete affected gate is rerun after that test-only correction. An earlier
queued diagnostic is cancelled before obtaining its reservation, exit 143;
no test or build starts in that waiter. Lightweight formatting completes before
each complete run. All compilation and suites use the shared reservation.

The corrected complete affected gate remains **FAIL**: **1,313 Rust passes,
72 inherited failures and 64 existing ignores** across **26 unfiltered
targets**, zero measured or filtered tests. Every target's failure and ignore
identities matches the retained base exactly, with four additional Linux passes.
All four new methods and four affected existing methods pass, including the
successful post-transfer control. Formatting and Clippy pass. All four code
files retain their verification-start hashes throughout this second complete
run. Exact comparison is data only; it does not make the gate pass or accept
either parent. The inherited native/runtime failures remain open.

The corrected gate takes **310.13 seconds**, including shared reservations:
formatting 104.67 seconds, Clippy 2.25 and Rust 201.42. No timeout occurs. These
are verification timings, not model performance. The raw log SHA-256 is
`ae39672f325723f8724f7b4f3e9a155f7b23b9dd767dee5f2e5b180bb47e2b99`.
Final lightweight checks cover completed prose, privacy, unchanged task rows
and both byte-identical review reports. All owned checks complete.

The immutable-base, complete original history, sealed validation and fresh Git
checks remain required. No candidate executor, approval, route or effect is
introduced by this repair. It does not establish reviewer/artifact provenance,
candidate execution, approved Git, native/model measurements or durable recovery.

No model or GPU process starts. Frozen, hidden and held-out inputs, scorers,
prompts, thresholds, dependencies, supply-chain outputs, lifecycle and scoreboard
remain unchanged. No public release or owner readiness is claimed.
