# T-303/T-306: frozen raw-source one-hundred-thirty-eight-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `224447d902067907dc367726ac8402e2cc725a95`.

Three unchanged-controller pairs add six development attempts. The repeat is
**138/180 attempted, 42 remaining**. Cumulative strict ungraded proposals:
gemma4-31b 18/24; qwen35-27b 22/24; gpt-oss 7/24; muse 13/22; gemma4-e4b 6/22; gemma4-26b-a4b 12/22. First-attempt and within-budget task passes remain unavailable.
Finish all 30 development tasks on all six profiles before tuning or capability
work. No model role is selected and no row closes.

| New profile attempts | Strict ungraded proposals | Observed backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 14036 / 1566 | 5633–8403 / 24333–27103 | 89.621 | 2.307 |
| qwen35-27b | 2/2 | 12181 / 1561 | 4907–7274 / 25462–27829 | 65.505 | 1.755 |
| gpt-oss | 2/2 | 11376 / 9047 | 4598–6778 / 25958–28138 | 61.025 | 1.440 |

gemma4-31b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

qwen35-27b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gpt-oss: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

Earlier transport timeouts remain preserved: muse dev-008, muse dev-011, muse dev-012. No terminal usage is invented and none is rerun. New attempts are reported separately.

No new transport timeout was observed.

This launch resumed the unchanged controller once. No attempt was rerun.
The earlier queue interruption and packaging correction remain
retained in the complete prior capsule and its exact external finalizer.

Controller wall time, including queues and two released 30-second pauses:
400.322 seconds. Reserved-job body time excludes queues and is neither GPU
kernel time nor full reservation duration. Preparation, reporting, continuous
campaign time and complete end-to-end latency are not measured here.

No measured variable changes. [Decision 0213](../decisions/0213-raw-source-presentation.md)'s
raw UTF-8 source, fixed instruction, all 30 excerpts, strict checker, schema,
profiles, model/runtime, thinking request, 32,768-token served context, output
formula `min(32768, 32768 - rendered input tokens - 32)` and 300-second deadlines
stay frozen. The wire request disables thinking; response fields alone do not
establish an absence of internal reasoning. All previous attempts are preserved.

The Rust context selector, credential-owning chat HTTP API, local inference and
strict checker run. Native executable admission and original execution/grading
controls remain unavailable. Native effects, project tests, feedback and retries
cannot run. No candidate, hidden or held-out code executes. These are ungraded
proposals from a partial path, with no solved-task claim. T-303/T-306 stay open.

The unchanged controller runs against the complete 9,760-file read-only source
snapshot; all 2,290 repository and 62 private bindings remain exact. The launcher
checks the frozen source against Git and explicitly allows only the fifty
reporting paths changed since it, binding the reporting HEAD separately. The launcher
retains the optional slot-four through slot-six threshold environment fields;
all are unset, and no environment setting changes. Worker,
collector, controller, policy and measured inputs remain byte-identical.

Each pair independently reacquires the host-provided build and GPU wrappers with
a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the same
22,528 MiB sampled GPU guard. Both reservations release between pairs. The host build wrapper still offers up to six slots, with slots three through six
guarded by 30/36/38/40 GiB of available RAM; its checksum is recorded. The
GPU wrapper, measured inputs and lane limits remain unchanged. No foreign
process is signalled, no limit is raised and nothing is downloaded. Settings are
revoked; API and model are reaped; keys and sockets are absent. Six new token
retirement receipts are persisted and checked before unlinking; all one-hundred-thirty-two
preceding receipts remain preserved.

The host build wrapper changed at 2026-11-19 12:47:28 UTC. The launch
SHA256 is `4061f0e6d5741a9798882066897ee6852df563677726e9ce36e9c25b3bce27a6`.
Those retained bytes authenticate the wrapper at launch, not every later pair's
wrapper invocation. The checkpoint66 review's low finding remains in T-309: record
wrapper provenance per pair. The frozen controller remains unchanged.

The private review capsule retains the complete preceding capsule under
`prior/`, with all its original names, with source archive, input objects, provenance indexes and raw
records. Every retained byte is authenticated; immutable hardlinks avoid copying
large artifacts. Portable reproduction executes only the authenticated effect-free
helper: 30 contexts, 161 earlier strict checks, 128 current strict checks and
10 authenticated generation refusals across all 138 attempt records. It executes
neither candidate code nor inference. The preceding capsule retains its original
reproduction failure and corrected accounting without changing measured inputs.

Reporting-only fast gate, Markdown lint/render, privacy, licence, preservation,
raw audit, cleanup, capsule verification and portable reproduction are checked
for this increment. No source suite reruns: the earlier source gate remains
FAIL, with 1,034 passed, 53 inherited failures, 11 ignored, zero filtered, across
30 targets. Two inherited Python failures and supply-chain limits remain. No
SBOM, long acceptance gate or release runs here.

All 512 ledger table lines, 49 historical path blobs, inventory, Decision 0051,
status model, independence paragraph and preceding scoreboard bytes remain
unchanged. At reporting preparation, the preceding checkpoint review is in flight;
no verdict is inferred. A verdict arriving after sealing is recorded separately. This checkpoint requires independent
core review. Existing T-309 lows remain backlog; no mid-repeat tuning occurs.
