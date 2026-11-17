# Copy original fragments: completed A4B development repeat — 2026-11-17 UTC

The frozen A4B repeat records **17/30 strict ungraded proposals**.
This increment adds 20 attempts, dev-011 through dev-029, then dev-rust-001, preserving the first
ten attempts without replay. **First-attempt and within-budget task passes
remain unavailable.** Five profiles still need their 30-task repeat:
**150/180 attempts remain** before further tuning or capability work.
T-303/T-306 remain open; this non-closing measurement needs independent core review.

Prompt/checker source is unchanged:
`35217c60a56a5d3e065148eb9a6aea898b5e1b16`, under
[Decision 0212](../decisions/0212-copy-original-fragments-before-replacement.md).
The [previous checkpoint](t303-copy-original-fragments-checkpoint10-2026-11-17.md)
was under independent review at session start. Its
[unchanged report](../reviews/35afab91240a-copy-original-fragment-checkpoint10.txt) has since returned **Verdict: PASS**
for exact `35afab91240af28c2099fe61dd99851d177abfff`, with no new findings and no row closure.
That verdict does not cover this increment. Earlier low notes about helper
build provenance and native executable admission wording remain T-309 backlog;
the helper remains frozen, as required for this repeat.

## Measurements

| Observed value | Cumulative A4B result |
| --- | --- |
| Development tasks | dev-001 through dev-029, plus dev-rust-001; one attempt each |
| Served context | 32,768 tokens |
| Rendered input preflight | 3229–9738 tokens |
| Requested output budget | 22998–29507 tokens |
| Strict ungraded proposals | 17/30 |
| First-attempt / within-budget task passes | Unavailable / Unavailable |
| Request and checker time | 1260.250 seconds |
| Completed-job reserved work | 1827.192 seconds / 30.453 GPU-slot work minutes |
| Backend counters | 216073 input / 89292 output; 30/30 coverage |

Causes, using original checker identifiers: `{"strict-edit-refused:code.proposal.edit": 12, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 17}`.
Does not fit: 0.
Backend terminations: `{"stopped-below-token-limits": 30}`.
Captured HTTP statuses: `{"200": 30}`.
Captured response fields: `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`.
These are observed fields, not evidence about internal reasoning. A strict
proposal is untested data; a refusal is neither repaired nor counted as a pass.

No measured variable changes on resume. The campaign's changed variable remains
the fixed original-fragment construction instruction. Context, source excerpts,
model/profile, runtime, inline schema, complete response framing, disabled-thinking
request and 300-second total/idle deadlines stay frozen. Output stays
`min(32768, 32768 - rendered input tokens - 32)`. All 30 development contexts
and all saved answers reproduce with the unchanged authenticated helper.

The available path is current Rust context selection → authenticated
credential-owning chat HTTP API binary at
`72c002a42957340de0b6fa0eb4d48385db2c7e62` → current Rust strict checker.
Native executable admission and the original grading controls remain unavailable.
Repository exploration, native effects, project tests, feedback and retries
cannot run. No control is reconstructed or bypassed. No candidate, hidden or
held-out test executes. This is a partial-path measurement, not a complete
coding-loop result or model qualification.

## Resources, preservation and checks

This segment runs ten two-task jobs, each acquiring both shared wrappers and
releasing them before the next pair. Low build priority, nice 19, 16 GiB RAM,
four CPUs and the 900-second per-job GPU cap are unchanged. The complete
9,706-file source snapshot passes its off-GPU preflight and is mounted read-only
for inference. All 2,283 repository and 44 private frozen bindings remain intact.

This segment adds 752.070 request/check seconds and 1167.491
completed-job reserved-work seconds. Its controller interval is 2754.075
seconds, including queues and released pauses. The three recorded controller
intervals total 3697.668 seconds. Between-session gaps, outer
input authentication, preparation and reporting are outside that total.
Completed-job work excludes pre-entry validation, wrapper overhead and queues.
GPU kernel time, complete reservation duration and total end-to-end latency
remain unmeasured.

All ten new jobs complete normally. Settings are revoked; API and model owners
are reaped; keys and sockets are absent; 20 new API tokens are retired; and the
exact-owner process scan is empty. Earlier token-retirement receipts are unchanged.
No foreign process is signalled, no model downloaded and no external service used.

The reporting fast gate, direct Markdown/table rendering, private-path and
licence checks, raw request/response/counter audit, helper reproduction, complete
prior-capsule authentication and preservation checks pass. Source tests are not
rerun because no runtime source changes. Their last recorded gate remains
**FAIL: 1,030 passed, 53 inherited failures, 11 ignored, zero filtered**.
The [licence evidence](readme-and-dual-licence-2026-11-16.md) continues to disclose
two inherited Python failures and supply-chain limits. No full acceptance gate
or SBOM regeneration runs.

The earlier scoreboard is a byte-exact prefix; all 512 ledger table lines,
49 historical private-path blobs, inventory, Decision 0051, status model and
frozen evaluation inputs are preserved. No row, lifecycle, platform, model role,
release or owner-readiness status is promoted. Resume only the first absent
frozen pair on the next profile; complete all 180 before tuning or new capabilities.

The first report-bundle verification failed on a reporting-only assumption that
all task IDs were numeric. The frozen final task is `dev-rust-001`. The corrected
bundle takes IDs from frozen records and includes that task's candidate data.
The failed bundle is preserved; no inference or evaluation input is rerun or changed.
