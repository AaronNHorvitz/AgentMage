# T-303/T-306: frozen raw-source forty-eight-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `979223db8ba401435d4fea78e4ff85b20df74428`.

Three unchanged-controller pairs add six development attempts. The repeat is
**48/180 attempted, 132 remaining**. Cumulative strict ungraded proposals:
gemma4-31b 6/8; qwen35-27b 8/8; gpt-oss 1/8; muse 4/8; gemma4-e4b 0/8; gemma4-26b-a4b 4/8. First-attempt and within-budget task passes remain unavailable.
Finish all 30 development tasks on all six profiles before tuning or capability
work. No model role is selected and no row closes.

| New profile attempts | Strict ungraded proposals | Observed backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| muse | 0/2 | 7521 / 10558 | 7322–7521 / 25215–25414 | 575.105 | 10.459 |
| gemma4-e4b | 0/2 | 18362 / 16596 | 9066–9296 / 23440–23670 | 162.097 | 2.910 |
| gemma4-26b-a4b | 1/2 | 18370 / 6619 | 9070–9300 / 23436–23666 | 89.218 | 2.145 |

muse: causes `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.format": 1}`; backend terminations `{"stopped-below-token-limits": 1, "transport-timeout-no-terminal-counters": 1}`; response fields `{"content:nonempty": 1, "reasoning:absent-or-empty": 1, "reasoning_content:nonempty": 1}`. Backend counters cover 1/2 attempts; does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

Muse dev-008 reaches the unchanged 300-second transport deadline. Its captured
terminal event is a partial timeout error; no complete upstream response or
terminal backend counters exist. Muse token totals cover only 1/2 new attempts,
not the whole pair. Generation progress is preserved without inventing usage.
No attempt is rerun and the timeout remains a generation refusal.

Controller wall time, including queues and two released 30-second pauses:
3083.054 seconds. Reserved-job body time excludes queues and is neither GPU
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
checks the frozen source against Git and explicitly allows only the twenty-one
reporting paths changed since it, binding the reporting HEAD separately. Worker,
collector, controller, policy and measured inputs remain byte-identical.

Each pair independently reacquires the host-provided build and GPU wrappers with
a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the same
22,528 MiB sampled GPU guard. Both reservations release between pairs. The host build wrapper continues to offer a
fourth slot guarded by 36 GiB of available RAM; its checksum is recorded. The
GPU wrapper, measured inputs and lane limits remain unchanged. No foreign
process is signalled, no limit is raised and nothing is downloaded. Settings are
revoked; API and model are reaped; keys and sockets are absent. Six new token
retirement receipts are persisted and checked before unlinking; all forty-two
preceding receipts remain preserved.

The private review capsule retains the complete preceding capsule under its
original names, with source archive, input objects, provenance indexes and raw
records. Every retained byte is authenticated; immutable hardlinks avoid copying
large artifacts. Portable reproduction executes only the authenticated effect-free
helper: 30 contexts, 161 earlier strict checks, 45 current strict checks and
3 authenticated generation refusals across all 48 attempt records. It executes
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
unchanged. The preceding reporting commit was under independent core review at
session start; its subsequent PASS is retained unchanged. That verdict accepts
only the preceding reporting increment. This checkpoint requires independent
core review. Existing T-309 lows remain backlog; no mid-repeat tuning occurs.
