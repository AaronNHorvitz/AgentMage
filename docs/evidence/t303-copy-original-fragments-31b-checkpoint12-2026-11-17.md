# Copy original fragments: twelve 31B development attempts — 2026-11-17 UTC

Eight new frozen development attempts, dev-005 through dev-012, yield
**7/8 strict ungraded proposals**, bringing 31B to
**9/12**. First-attempt and within-budget task passes
remain unavailable. All 30 A4B and four earlier 31B attempts are preserved,
without replay: **42/180 recorded; 138 remain** before tuning or capability work.
T-303/T-306 remain open. Independent core review is required; no row closes.

The [unchanged previous checkpoint review](../reviews/d6eb2abf29f8-copy-original-fragment-31b-development-checkpoint4.txt) returns **Verdict: PASS** for exact `d6eb2abf29f8391f072171d97162de716cadd987`. It accepts that checkpoint only and closes no row. Its low L1 concerns verifier access to live builder state; it joins T-309. This increment reports that state as unavailable when not mounted, while checking every sealed byte. No verdict is inferred for this increment.
Earlier low findings remain open, including helper build provenance, native
executable admission wording and reporting correction provenance. This batch
retains its original reporting methods, every adapted method and the adaptation
record. No inference helper or measured input changes.

## Measurements

| Observed value | Cumulative 31B | Eight new attempts |
| --- | --- | --- |
| Development tasks | dev-001 through dev-012 | dev-005 through dev-012 |
| Strict ungraded proposals | 9/12 | 7/8 |
| First-attempt / within-budget task passes | Unavailable / Unavailable | Unavailable / Unavailable |
| Request and checker seconds | 868.127 | 559.031 |
| Completed-job reserved work seconds | 1152.523 | 768.460 |
| GPU-slot work minutes | 19.209 | 12.808 |
| Backend input / output tokens | 81642 / 22072 | 54577 / 13112 |
| Backend counter coverage | 12/12 | 8/8 |

Prompt/checker source: `35217c60a56a5d3e065148eb9a6aea898b5e1b16`. Served context: 32,768 tokens.
Rendered input: 3229–9738; requested output: 22998–29507 tokens.
Output remains `min(32768, 32768 - rendered input tokens - 32)`.
Cumulative causes: `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 9}`.
New causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 7}`.
Does not fit: 0.
Cumulative backend terminations: `{"stopped-below-token-limits": 12}`; HTTP statuses: `{"200": 12}`.
Observed response fields: `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:absent-or-empty": 12}`. These fields establish no internal
reasoning behavior. Refusals are not repaired or counted as passes.

The campaign's changed variable remains the fixed original-fragment instruction
in [Decision 0212](../decisions/0212-copy-original-fragments-before-replacement.md).
No variable changes on resume. Context, excerpts, schema, framing, model settings,
disabled-thinking request and 300-second total/idle deadlines remain frozen.
The authenticated unchanged helper reproduces all 30 original contexts and the
eight new saved answers. All original 2,283 repository and 44 private bindings
are checked, with no narrowed input set.

The measured path remains Rust context selection → authenticated credential-owning
chat HTTP API binary at `72c002a42957340de0b6fa0eb4d48385db2c7e62` → Rust strict
checker. Native executable admission and original grading controls are unavailable.
Repository exploration, native effects, project tests, feedback and retries cannot
run. No original control is reconstructed or bypassed. No candidate, hidden or
held-out test executes. This partial path gives no complete coding-loop pass rate
or qualified model role.

## Resources and verification

Four two-task jobs each acquire both shared wrappers and release them between
pairs. Low priority, nice 19, 16 GiB RAM, four CPUs and the 900-second job cap
remain in force. The full 9,706-file source snapshot passes its off-GPU preflight
and is mounted read-only. All four previous capsules and their complete committed
diff receipts authenticate before inference; every sealed file remains intact.
Live builder files are checked here. The verifier reports live state as
unavailable when that directory is not mounted in the reviewer sandbox; sealed
inputs still check unconditionally, and any accessible live mismatch fails.

The new controller interval is 2337.847 seconds, including queues and
released pauses. Both 31B controller intervals total 2754.318
seconds, excluding the between-session gap, outer authentication, preparation
and reporting. Completed-job work excludes queues, pre-entry validation and
wrapper overhead. GPU kernel time, full reservation duration and total end-to-end
latency remain unmeasured.

All four new jobs complete normally. Settings are revoked, API and model owners
reaped, keys and sockets absent, eight new API tokens retired, and the exact-owner
process scan is empty. Earlier raw bytes and retirement receipts are unchanged.
No owned GPU job remains; no foreign process is signalled, model downloaded or
external service used.

Raw request/response/checker/counter audit, reporting fast gate, direct Markdown
lint and table rendering, private-path and licence checks, helper reproduction,
full dependency authentication and preservation checks pass. No runtime source
changes; source tests are not rerun. The last source gate remains **FAIL: 1,030
passed, 53 inherited failures, 11 ignored, zero filtered**. The two inherited
Python failures and supply-chain limits remain disclosed in the
[licence evidence](readme-and-dual-licence-2026-11-16.md). No SBOM regeneration
or full acceptance gate runs.

Packaging initially stopped on an archived-request filename mismatch after
measurement and reporting checks passed. The corrected package retains the failed
staging bytes and every edited method; no attempt is rerun.

All 512 ledger table lines, 49 historical private-path blobs, original inventory,
Decision 0051, status model and frozen evaluation inputs are preserved. The old
scoreboard is an exact byte prefix. No row, lifecycle, platform, model role,
release or owner-readiness status is promoted. Resume the first absent 31B pair
at offset 12 and finish all 180 attempts before tuning or new capabilities.
