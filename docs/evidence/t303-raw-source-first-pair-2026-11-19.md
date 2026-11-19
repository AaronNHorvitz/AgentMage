# T-303/T-306: first frozen raw-source measurement

Source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.

The source presentation change in [Decision 0213](../decisions/0213-raw-source-presentation.md)
is frozen. This checkpoint records the first two A4B development attempts:
**1/2 strict ungraded proposals**, with task passes and
retries unavailable. The six-profile repeat is **2/180 attempted, 178 remaining**.
Do not tune or start another capability before completing that repeat.

| Measure | Observed |
| --- | --- |
| Served context | 32,768 tokens |
| Output formula | `min(32768, 32768 - rendered input tokens - 32)` |
| Rendered input tokens | 9085–9307 |
| Requested output tokens | 23429–23651 |
| Backend input / output tokens | 18392 / 4489 |
| Backend counter coverage | 2/2 |
| First-attempt / within-budget task passes | Unavailable / unavailable |
| Request and strict-check time | 60.782 seconds |
| Reserved-job body time | 97.639 seconds / 1.627 GPU-slot minutes |
| Controller wall time, including queues | 98.811 seconds |

Causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`.
Backend terminations: `{"stopped-below-token-limits": 2}`.
Response fields: `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`.
Does not fit: 0.

The changed variable is raw UTF-8 source presentation, replacing source-level
JSON-escaped strings. The fixed instruction, all 30 development excerpt
selections, output schema, thinking request, profiles, model/runtime, context
and output formula, and 300-second deadlines are unchanged. The wire request
disables thinking; response-field observations do not establish the absence of
internal reasoning. All 161 available previous strict checks reproduce exactly.

Only the existing Rust context selector, credential-owning chat HTTP API,
local inference and strict checker run. Native executable admission and original
execution/grading controls remain unavailable. Native effects, project tests,
feedback and retries cannot run. No candidate, hidden or held-out code executes.
Proposals are ungraded and no model role qualifies. This is a partial path,
not a complete coding loop or a solved-task score. T-303/T-306 remain open.

The worker, collector and controller change only their private namespace from
the preceding campaign. Their entry points are unchanged; the helper is rebuilt
against the frozen source. All 2290 repository and 62 private
input bindings are retained and authenticated, including every preceding input
name. Source changes are explicit. The complete 9760-file Git source
snapshot runs read-only; preparation releases its build slot before the
controller starts. No old attempt or candidate is overwritten or replayed.

The pair reacquires both unchanged wrappers with a 900-second job deadline,
low priority, nice 19, 16 GiB RAM, four CPUs and the existing 22,528 MiB sampled
GPU guard. Body time excludes queues and is neither GPU kernel time nor the
full reservation duration. Controller time excludes snapshot preparation,
preflight and reporting. Continuous campaign time and end-to-end latency remain
unmeasured. No foreign process is signalled and no resource limit is raised.

Settings are revoked, the API and model reaped, keys and sockets absent and the
owned-process scan empty. Both token retirement receipts are persisted and
counted before unlinking the tokens, applying the preceding low cleanup note.
Raw bodies, response captures, backend counters, source/context hashes and
strict-check outputs are retained in a sealed private review capsule. Its
reproduction method runs only the authenticated effect-free Rust helper.

Final format, Clippy, Markdown lint/render, privacy, licence, preservation,
freeze, raw-attempt audit and cleanup checks pass. The full source gate remains
**FAIL**: 1,034 passed, 53 inherited failures, 11 ignored, zero
filtered, across 30 targets. Every failure identity matches the retained
preceding source run. No failure is filtered, waived or called green. The
licence batch's two inherited Python failures and supply-chain limits remain
open; no SBOM regeneration or long acceptance gate runs here.

All 512 ledger table lines, 49 historical path blobs, inventory, Decision 0051,
status model and prior scoreboard bytes are preserved. The independence
paragraph is unchanged. Both preceding reviews are retained unchanged; their
PASS verdicts accept their own exact commits only. Independent core review is
required for this source and measurement batch. No row closes.
