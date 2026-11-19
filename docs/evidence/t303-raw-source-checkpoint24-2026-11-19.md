# T-303/T-306: frozen raw-source twenty-four-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `f278e0b36765f5cb097da4c2efdb0c30e421d6cb`.

Three unchanged-controller pairs add six development attempts. The repeat is
**24/180 attempted, 156 remaining**. Cumulative strict ungraded proposals:
gemma4-31b 4/4; qwen35-27b 4/4; gpt-oss 0/4; muse 2/4; gemma4-e4b 0/4; gemma4-26b-a4b 2/4. First-attempt and within-budget task passes remain unavailable.
Finish all 30 development tasks on all six profiles before tuning or capability
work. No model role is selected and no row closes.

| New profile attempts | Strict ungraded proposals | Backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| muse | 0/2 | 6464 / 19082 | 2647–3817 / 28919–30089 | 431.871 | 7.782 |
| gemma4-e4b | 0/2 | 7476 / 14800 | 3040–4436 / 28300–29696 | 120.899 | 2.164 |
| gemma4-26b-a4b | 1/2 | 7484 / 10810 | 3044–4440 / 28296–29692 | 101.487 | 2.258 |

muse: causes `{"strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

Controller wall time, including queues and two released 30-second pauses:
1933.238 seconds. Reserved-job body time excludes queues and is neither GPU
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
checks the frozen source against Git and explicitly allows only the thirteen
reporting paths changed since it, binding the reporting HEAD separately. Worker,
collector, controller, policy and measured inputs remain byte-identical.

Each pair independently reacquires the unchanged build and GPU wrappers with
a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the same
22,528 MiB sampled GPU guard. Both reservations release between pairs. No foreign
process is signalled, no limit is raised and nothing is downloaded. Settings are
revoked; API and model are reaped; keys and sockets are absent. Six new token
retirement receipts are persisted and checked before unlinking; all eighteen
preceding receipts remain preserved.

The private review capsule retains the complete preceding capsule under its
original names, with source archive, input objects, provenance indexes and raw
records. Every retained byte is authenticated; immutable hardlinks avoid copying
large artifacts. Portable reproduction executes only the authenticated effect-free
helper: 30 contexts, 161 earlier strict checks and all 24 current checks. It
executes neither candidate code nor inference.

Reporting-only fast gate, Markdown lint/render, privacy, licence, preservation,
raw audit, cleanup, capsule verification and portable reproduction are checked
for this increment. No source suite reruns: the earlier source gate remains
FAIL, with 1,034 passed, 53 inherited failures, 11 ignored, zero filtered, across
30 targets. Two inherited Python failures and supply-chain limits remain. No
SBOM, long acceptance gate or release runs here.

All 512 ledger table lines, 49 historical path blobs, inventory, Decision 0051,
status model, independence paragraph and preceding scoreboard bytes remain
unchanged. The preceding reporting commit was under independent core review at
session start; its subsequent PASS has no critical, high, medium or new low
findings. Its report is retained unchanged. That verdict accepts the preceding
reporting increment only. This checkpoint requires independent core review.
Existing T-309 lows remain backlog; no mid-repeat tuning occurs.
