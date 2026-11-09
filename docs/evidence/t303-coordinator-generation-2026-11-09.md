# T-303/T-306: strict request coordinator handoff, 2026-11-09

Status: builder implementation; independent core review required. T-303 and
T-306 remain open. No row or sub-row is added or closed.

## Change

A trusted endpoint turn port can consume an unexecuted strict generation into
the existing coordinator's prepared-turn interface. Conversion verifies the
complete sealed run, exact original startup digest, turn range and current
borrowed control before returning the original frozen transport and independently
framed context. It does not dispatch, read a file or consume a model attempt.
Borrowed and retained shared connections keep their original credential owners.
Transport errors use the existing host adapter's content-free mapping.

The original coordinator frames and executes this request with its own
cumulative ledger, events and continuous control. The unused standalone phase
is consumed; no phase usage is imported or accepted as cumulative accounting.
The existing kernel still enforces an independent per-phase envelope. No new
ledger, model loop, executor, authority, route or recovery mechanism is added.

The answer remains untrusted model text. Conversion does not parse edits,
prepare a native preview or establish completion. Source freshness/disclosure
consent and the choice of completion schema and verifier stay with the trusted
composition. Native previews and effects retain separate existing boundaries.
The ordinary retained service factory is unchanged; selection of strict turns
and the general edit/test/retry schedule are still open.

## Verification

The two added regression methods pass in the focused development check (686
filtered, 3.45 seconds of test execution) and again in the ordinary unfiltered
scoped gate. The conversion refusal method checks tampered and foreign seals,
zero/over-limit turn ordinals and cancelled control without connecting.

The coordinator fixture covers eight cases with both borrowed and shared
connections. Its first synthetic answer requests an already prepared native
edit. A stale approval refuses without advancing events or writing; the exact
protected approval performs the original native atomic edit once. Only the
unchanged second file is disclosed in both frozen strict requests. In the
two-dispatch case, raw edit text reaches the original independent verifier,
which refuses completion: no second edit occurs. This does not implement
generation-to-preview scheduling.

Other cases refuse the second dispatch after cumulative output/time envelopes
or model-call exhaustion, original clock expiry or exact cancellation. An
incomplete second response and a text overflow retain both admitted attempts
and frozen input charges without inventing completed output. Assertions compare
exact cumulative model calls, body plus tool-argument bytes and completed text
plus native-result bytes. The original native result survives all terminals;
terminal rereads preserve the same events, ledger and source bytes. Every event
passes the existing sequence verifier. Native creation provenance and endpoint
answers are synthetic; the file transaction actually runs.

Three earlier development checks fail: initial test-only type/trait errors,
an incorrect fixture assumption that permission denial permits another turn,
and an identifier-formatting error while composing the native fixture. The
fixture now follows the unchanged terminal-denial behavior and uses an exactly
approved native edit before the strict turn. All earlier logs are retained;
no production permission or failure behavior is changed to fit an assertion.

The complete scoped fast gate remains **FAIL / exit 1**: 20 targets, 852
passed, exactly 53 inherited host failures with none new or missing, 11 existing
ignored and zero measured or filtered. The failure identities equal the
unchanged T-301 host baseline. Formatting exits zero in 0.95 seconds, all-target
Clippy zero in 11.37 seconds, and all-target tests 101 in 148.46 seconds. Total
elapsed time including reservations is 162.46 seconds. Four Rust input hashes
remain unchanged throughout. The comparison of inherited failures is data only,
not a passing gate or an independent acceptance verdict.

```sh
python3 -B scripts/local_agent_fast_gate.py --base c425b77ec89ec989b807ce42fc75fe70c1965bf2 --timeout 600
```

Each Cargo command reserves its own shared build slot, with two build jobs and
four test threads. Host and local API formatting, all-target lint and all-target
tests run without filters. The unchanged Linux unit and inference packages are
not selected. The eight-path plan includes the retained predecessor review.
Only documentation results are updated after the source gate; final whitespace,
Markdown, review-text and public-hygiene checks run separately.

Full-gate raw log SHA-256:
`4cd4cc4b98d97cfe2084cb549977d5046820f7baf143e3836ab1bd83d6b2aae6`.

## Scope

Fixtures use synthetic endpoint answers and worktree creation provenance. The
original socket transport, reusable coordinator and exactly approved native
file edit actually execute. Native Git creation, model-driven edit/test retries, approved
Git, durable recovery, parent measurements and both parent closures remain open.
No development task grade, held-out run or score is claimed. Hidden tests,
scorers, task pins, thresholds, dependency/SBOM inputs, status model and scoreboard
are unchanged. The long acceptance gate and GPU inference do not run in this batch.
The in-flight predecessor commit and its history remain immutable.

## Predecessor verdict

The [independent text-ceiling review](../reviews/c425b77ec89e-local-coding-generation-ceiling.txt)
arrived during implementation. It has **Verdict: PASS** for exact commit
`c425b77ec89ec989b807ce42fc75fe70c1965bf2` with no high/medium finding; its
bytes are retained unchanged. Low L1 (assert the host overflow fixture's exact
delivered prefix) is same-row test backlog. Earlier schema-ID and command
resource-preflight lows remain backlog. Both parents remain open; this verdict
does not accept the new coordinator handoff.
