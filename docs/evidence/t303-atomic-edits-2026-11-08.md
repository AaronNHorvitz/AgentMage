# T-303/T-306: strict edit batch bounds

Date: 2026-11-08. Tier: core. Immutable predecessor:
`32c91cae33cb6d7de708ddc10b3979c71c9f25b4`.
This fixes the existing strict proposal path within the open parent rows.
No row is added or closed; independent exact-commit core review is required.

## Behavior

A valid multi-file proposal could be refused at the 128 KiB snapshot ceiling
when an alphabetically earlier file grew before a later file shrank. The
checker applied all edits to original snapshots together but checked aggregate
size after each file. Renaming the same files could change acceptance.

The checker now plans final file sizes and the complete final aggregate before
allocating candidate content. Unchanged files still count. Shrinkage and growth
in the same proposal balance independently of file names, source order or edit
order. The existing per-file and aggregate 128 KiB limits remain unchanged.
One-byte final overflow still rejects the entire proposal. Strict JSON, exact
visible unique preimages, overlap, stale snapshots and net-no-op checks remain.

All existing proposal consumers share this checker, including snapshot, selected
workspace, owned edit and failed-test repair paths. This changes in-memory
candidate validation only. File reads, disclosure, diff approval, native writes,
permissions, grants, confinement and retry budgets retain their original paths.
It does not connect the configured endpoint to controlled native edit/test/retry.
Native/model success, durable execution recovery, approved Git, parent
measurements and both parent rows remain open.

## Verification

Before any Rust edit, the complete affected-package baseline ran through the
shared build reservation: **26 targets, 850 passed, 53 failed, 11 ignored,
zero filtered**. The failures belong to the existing host target; they remain
failures under deferred T-301.1, with no skip or trust-boundary waiver.

The initial new test compilation failed because its digest assertions used
hexadecimal formatting unsupported by the pinned digest type. Only those new
assertions changed to the existing byte-wise formatting; raw compile failures
are retained and are not reproduction evidence. With the original checker still
unchanged, both focused regressions then failed as expected: the snapshot
positive case refused with `Edit`, and the actual HTTP proposal route emitted
an edit refusal for the valid final-size batch. The snapshot overflow negative
control passed. Raw reproduction logs are retained privately.

Two new snapshot methods cover both lexical growth orders, both source orders,
both edit orders, UTF-8 byte accounting, exact-limit and below-limit acceptance,
one-byte overflow including an unchanged file, invalid final edits, overlaps,
original/result hashes and unchanged inputs. One new HTTP integration method
streams the valid and overflowing batches through the configured transport in
both lexical orders. It checks actual disclosed excerpts, full resulting content
and hashes, complete inference accounting, and proposal-only authority. Its
endpoint is synthetic; it does not run a model, worker, test command or write.

The complete ordinary final affected-package gate ran **27 targets, 853 passed,
53 failed, 11 ignored, zero filtered**. All 850 baseline passing identities
remain, plus exactly the three new methods. Every failing and ignored identity
is preserved, and all 53 normalized failure diagnostics match. Existing test
bodies and ledger table rows are unchanged. The full gate remains **FAIL**:
Cargo tests exit 101 and the gate exits 1 on the original host failures.

Formatting, strict Clippy, changed-document lint, retained-review format,
whitespace and privacy checks pass. Seven gate commands took **194.34 seconds**,
including their individual build reservations. All three Rust input hashes
stayed frozen throughout the complete gate; no Rust source changed afterward.
Final documentation facts receive separate lightweight checks. Raw logs, source
snapshots and identity/diagnostic comparisons remain private. This is a verified
strict-validator repair with synthetic HTTP evidence, not a successful native
coding workflow, a whole-workspace pass, or a model-quality measurement.

No evaluation inputs, hidden/held-out assertions, scoring thresholds, dependency,
model/GPU work, SBOM, status model, scoreboard, release or push changes.

## Independent predecessor review

The terminal event-record [independent review](../reviews/32c91cae33cb-local-coding-endpoint-record.txt)
arrived during this session with **Verdict: PASS** for the predecessor's exact
commit and no high/medium finding. It is retained unchanged. Low L1 (independent
event-count ceiling coverage) and L2 (offline refusal wording) remain same-row
backlog. Its reviewer records Markdown lint exit 127 because Node was absent;
that failure remains in the raw report. Its PASS accepts only that predecessor
increment and closes neither parent nor this fix.
