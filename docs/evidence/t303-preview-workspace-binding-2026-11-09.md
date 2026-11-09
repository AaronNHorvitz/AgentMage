# T-303/T-306: preview workspace binding, 2026-11-09

Status: builder implementation; independent core review required. Both parent
rows remain open. No row or sub-row is added or closed.

Preview preparation checks the sealed workspace label against the original
repository's held created-worktree label before source selection, reads or
preview replacement. Foreign labels and absent creation owners refuse `Scope`.
This addresses strict-preparation review low L1. Generated-answer preview uses
the same check and retains the original generation result and usage on refusal.

The label accessor is content-free, performs no filesystem access and supplies
no authority. Matching labels do not identify a physical root or adopt an owner;
the original native reader still checks held roots and selected paths, and
native binding still checks the complete request and creation receipt. Exact
approvals, kernel grants and atomic writes remain unchanged.

Three added regression methods cover valid snapshots under a foreign workspace,
refusal before missing-file selection, unchanged prior previews, absent creation
ownership, same-path root replacement and generated observations surviving
foreign-workspace refusal. Existing synthetic generation fixtures now seal the
same label as their original native owner, with narrowing ledgers bound to that
label. Exclusion, positive generation and native approval assertions remain.
Creation and endpoint responses are synthetic; no native Git creation, actual
model, task grade, coding score or complete coding workflow is claimed.

## Verification

The ordinary unfiltered scoped fast gate against immutable base
`d2ce9c60e20d96f1b932c875d862d42b3d3d2e85` exits **1 / FAIL**:
25 targets, 1,169 passed, 72 failed, 63 existing ignored, zero measured or
filtered. All 72 failure identities equal the committed T-301 host/Linux
baseline, with none new or missing. This comparison is data only, not acceptance.
All three new methods, the existing positive generation and exclusion methods,
and the protected native HTTP edit method pass. No test, ignore or threshold
was weakened.

Formatting exits zero (199.78 seconds including reservation), Clippy exits zero
(8.77 seconds), and tests exit 101 (190.14 seconds). Total elapsed time including
reservations is 399.21 seconds. Each Cargo command reserves its own shared build
slot, using two build jobs and four test threads. The four Rust input hashes
stay unchanged for the full run. Raw-log SHA-256:
`50f027d0f3d139eab115a70c73a2ed8045eb9c3560ebdf042579a90a78790185`.

The owned temporary namespace retains 22 entries, including 4 inherited host
fixture files totaling 36 bytes. New workspace/generation fixture roots are
absent; no unrelated directory is cleaned. No GPU, download, long documentation
gate or full-workspace acceptance is run. Final documentation/review-text,
whitespace and hygiene checks are repeated after recording these results;
the same four source hashes and identical Cargo commands remain bound to this
run. No additional source gate is claimed for the documentation-only update.

## Review received during this batch

The independent strict-generation review has **Verdict: PASS** for exact base
commit, no high/medium finding, and is retained byte-identically. Both parents
remain open. Its low L1 exact cancellation assertions remain same-row backlog;
low L2's fixed 1 MiB transport envelope is disclosed in the guide, with smaller
prepared envelopes remaining backlog. Earlier schema-ID and command-resource
lows remain backlog. The present workspace-label repair needs its own review.
Cumulative generation accounting, general edit/test/retry, native command-factory
success, recovery, approved Git and parent measurements remain open. No hidden
tests, held-out tasks, scorer, threshold, dependency, SBOM, status or score changes.
