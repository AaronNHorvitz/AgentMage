# Decision 0113: Review Fixes for Inspection, Job Control and CLI Views

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088 and 0107 through 0112; current owner restart |
| Scope | Findings V1 through V15 of the independent review of `1458cef4` |

## Findings

An independent read-only review of `28dd0e44..1458cef4` passed. It had two medium
and thirteen low findings. V1: refused client requests shared one bound with the
owner's observations, so a flood of stale requests could stop the owner from ever
recording a terminal state. V2: the hunk display still showed grapheme-extending
marks, such as variation selectors, raw after the first position, so it was weaker
than the whole-file preview. The low findings cover these areas:

- A first composition that omitted a retained constraint for budget (V3).
- A job ledger replay that accepted a truncated chain (V4).
- A renderer that panicked on a malformed digest (V5).
- Progress that still asked for a decision after cancellation (V6).
- A CLI that could project a silently truncated stream (V7).
- Tests that did not exercise what AMR-04.8 and AMR-04.8.1 list (V8).
- A creation review shown for an occupied target (V9).
- A test-ownership scanner that missed `name/mod.rs` modules (V10).
- Retained stage fields that bypassed redaction and the name check (V11).
- Stage sources that were never verified, and a failure log without its cause (V12).
- Obligations that no open ledger row tracked (V13).
- A Sprint 50 review that was stale and that no aggregate check detected (V14).
- A review reader that checked a path and then read it (V15).

## Decision

V1. The job ledger keeps two entries that only the owner may use: enough to start a
queued job and record its terminal event, the most owner observations that can follow
without a client request. Client requests stop at the bound less that reserve.
Refused requests have their own bound of 1,024 entries. A further refusal is answered
with `Full` and is neither recorded nor remembered. It can never be applied later on
a retry, because the revision it names can only fall further behind. A refusal flood
therefore leaves accepted requests and every owner observation recordable. A job that
reaches the client bound while suspended stays suspended; that bound is the limit of
one job's control history.

V2. The hunk display escapes each character on its own, as `{:?}` does at every
position. Every grapheme-extending mark is escaped wherever it occurs. Only tab and the
two ASCII quotes are shown as themselves. A test now requires each displayed line to
equal the preview's `{:?}` of that line with only the tab and double-quote escapes
undone. Decisions 0107 and 0109 are corrected.

V3. The coding context builder refuses every composition, including the first, that
omits a retained constraint for budget. Such a composition ends as resource
exhaustion, before any measurement.

V4. Replay takes the retained head, the entry count and last entry digest, and refuses
a chain that does not end there. The chain proves that entries are consistent with
each other. It does not prove who wrote them or that none are missing after the head.
The AMR-04.6 row now requires the store to authenticate writers, to keep the head in
the same transaction as each append, and to scope request identities by the
authenticated client.

V5. The context inspection renderer shows a digest prefix only for a well-formed
digest and never slices a malformed one.

V6. Progress reports a cancellation request before a pending approval. It clears
pending approvals when the owner observes cancellation, and it reports none once
cancellation is requested.

V7. The CLI's event sink marks a run whose events exceeded its capacity. It then
prints the existing "run progress unavailable" output, because a prefix of a valid
stream verifies and would claim that an ended run had not ended.

V8. Engine tests now cover the running activity and every terminal state's text. CLI
tests present sealed streams through the real sink for two consecutive runs in both
formats, and cover an unverifiable stream and a truncated run.

V9 and V15. The review reader reports one of three states: absent, bytes, or refused.
It opens each component relative to the held parent with `O_NOFOLLOW`, and opens the
final component with `O_NONBLOCK` and `O_NOCTTY` as well. It checks the type and
length of the opened file and reads at most the bound plus one byte. A creation review
is shown only when the target is absent. A link, directory, special file or oversized
file is reported as unavailable.

V10. A default module declaration refers to both `name.rs` and `name/mod.rs`, and a
module found at both paths is ambiguous, as it is to the compiler.

V11 and V12. `scripts/verification_batch.py` writes stage schema 2. Before anything is
written, it checks every written byte for the user and host names in any letter case.
It requires numeric exit codes and durations, and repository-relative source keys with
SHA-256 values. Every schema 2 stage binds the tool itself among its sources. Each
source digest must equal the file at the stage's source revision, unless the stage
declares the path as changed in a named later commit that holds exactly those bytes.
A review pin that must be edited before its gate can run is such a case. Schema 1
stages stay historical: their logs are verified and their sources are not. Tool
changes are committed as source commits before the source-check stage. The
configuration result producer now copies a failing child command's output into its
log.

V13. New open rows track the remaining actual-process and view obligations:
AMR-04.4.1 for a CLI view of the context inspection, AMR-04.9 for actual-process
display of the hunk review and run progress on a native host, and AMR-04.2.3 for the
native proof that a hunk selection preserves human edits and rejected hunks.

V14. Every evidence pass runs `scripts/sprint_50_runtime_contract_review.py` in its
source-check stage. The batch 5 record now states that its renewal also absorbed the
earlier `runtime_loop.rs` change from `64826784`.

## Verification boundary

Each source fix has a regression test. The V2, V3 and V10 tests failed against the
unfixed source. No model, GPU or native workflow was used. Independent re-review of
this batch is requested and remains open.
