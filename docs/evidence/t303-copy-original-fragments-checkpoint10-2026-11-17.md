# Copy original fragments: ten-attempt checkpoint — 2026-11-17 UTC

The frozen A4B repeat now records **6/10 strict ungraded proposals**,
including eight new development attempts. **First-attempt and within-budget
task passes remain unavailable**. There are 20 A4B and 150 other-profile
attempts left: **170/180 remain** before further tuning or capability work.
T-303/T-306 stay open; this reporting increment requires independent core review.

The prompt/checker source stays
`35217c60a56a5d3e065148eb9a6aea898b5e1b16` under
[Decision 0212](../decisions/0212-copy-original-fragments-before-replacement.md).
The preceding [first-pair evidence](t303-copy-original-fragments-2026-11-17.md)
and every saved first-pair byte are preserved. That source-and-first-pair commit
was in independent review when this session started. This increment closes no row. The independent review has since returned **Verdict: PASS**,
with no high or medium finding; its [unchanged report](../reviews/b7ab16d0c628-copy-original-fragment-generation.txt)
accepts that earlier increment only. Two low notes stay in T-309: record the
helper build's frozen source provenance, and describe inherited failures as
native executable admission failures (one is Git artifact verification). The
helper stays frozen during this repeat; the reviewer independently rebuilt it
against the pinned source and reproduced the same 30 contexts and 168 answers.

## Observations

| Observed value | Cumulative A4B result |
| --- | --- |
| Development tasks | dev-001 through dev-010; each attempted once |
| Served context | 32,768 tokens |
| Rendered input preflight | 3229–9738 tokens |
| Requested output budget | 22998–29507 tokens |
| Strict ungraded proposals | 6/10 |
| First-attempt / within-budget task passes | Unavailable / Unavailable |
| Request and checker time | 508.180 seconds |
| Completed-job reserved work | 659.701 seconds / 10.995 GPU-slot work minutes |
| Backend counters | 71701 input / 45453 output; 10/10 coverage |

Failure causes, with original strict-checker identifiers:
`{"strict-edit-refused:code.proposal.edit": 3, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 6}`. Does not fit: 0.
Backend terminations: `{"stopped-below-token-limits": 10}`.
Captured HTTP statuses: `{"200": 10}`.
Captured response fields: `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`.
These fields make no claim about internal reasoning. Refused answers are not
repaired, retried, dropped or counted as solved tasks.

No measured setting changes on resume. The campaign's changed variable remains
the fixed instruction asking the model to copy original text before replacement.
Output stays `min(32768, 32768 - rendered input tokens - 32)`; the inline schema,
32K served context, disabled-thinking request, complete response framing,
model/profile, runtime and 300-second total/idle deadlines remain fixed.
All 30 development contexts reproduce against the authenticated helper;
all saved answers reproduce their original strict-checker result.

The available path is current Rust context selection → the authenticated
credential-owning chat HTTP API binary at
`72c002a42957340de0b6fa0eb4d48385db2c7e62` → current Rust strict checker.
Native executable admission and the original grading controls remain unavailable.
Repository exploration, native effects, project tests, feedback and retries
cannot run here. No control is reconstructed or bypassed. No candidate, hidden
or held-out test runs; proposed files remain untested data.

## Resource and evidence limits

This resume runs four two-task jobs, independently taking and releasing both
shared wrappers between pairs. It uses low build priority, nice 19, 16 GiB RAM,
four CPUs and a 900-second cap per GPU job. The complete 9,706-file source
snapshot passes its off-GPU preflight and is mounted read-only with Git metadata.
All 2,283 original repository input names and 44 private bindings authenticate;
none is removed or narrowed. The first two attempts are preserved and never
replayed. No model is downloaded and no external service is used.

This segment adds 447.615 request/check seconds and
562.634 completed-job reserved-work seconds. Its controller launch-to-exit
interval is 845.456 seconds, including queues and the released pauses.
The two recorded controller intervals total 943.593
seconds; the gap between sessions, outer authentication, preparation and
reporting are outside that total. Completed-job work excludes pre-entry binding
validation, wrapper overhead, queues and preparation. Actual GPU kernel time,
complete reservation duration and total end-to-end latency remain unmeasured.

All four jobs finish normally. Settings are revoked, API and model owners reaped,
keys and sockets absent, eight new API tokens retired, and the exact-owner
process scan is empty. No foreign process is signalled. The earlier two-token
retirement receipt remains unchanged.

Reporting fast gate, direct Markdown/table rendering, privacy/licence checks,
full original-capsule authentication, raw wire/counter audit, helper reproduction
and preservation checks pass. No runtime source changes in this increment, so
source tests are not rerun. Their last recorded gate remains **FAIL: 1,030 passed,
53 inherited failures, 11 ignored, zero filtered**; none is called green.
The licence batch's two inherited Python failures and supply-chain limits
remain disclosed in its [evidence](readme-and-dual-licence-2026-11-16.md).

The previous scoreboard is a byte-exact prefix. All 512 ledger table lines,
49 historical private-path blobs, the historical inventory, Decision 0051,
status model and frozen evaluation inputs are preserved. No SBOM or full
acceptance regeneration runs, and no platform, model role, lifecycle, release
or owner-readiness status is promoted. Resume only the first absent frozen pair.
