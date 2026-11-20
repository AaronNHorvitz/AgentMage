# T-303/T-306: frozen raw-source one-hundred-eighty-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `1d697fddaa4895c04964654c7fb89badeee953f8`.

Fifteen unchanged-controller pairs add thirty development attempts. The repeat is
**180/180 attempted, 0 remaining**. Cumulative strict ungraded proposals:
gemma4-31b 23/30; qwen35-27b 28/30; gpt-oss 8/30; muse 20/30; gemma4-e4b 8/30; gemma4-26b-a4b 16/30. First-attempt and within-budget task passes remain unavailable.
All 30 development tasks on all six profiles have now been attempted.
The next work investigates the largest diagnosed refusal cause before further tuning or capability work. No model role is selected and no row closes.

| New profile attempts | Strict ungraded proposals | Observed backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 3/4 | 30338 / 4364 | 3558–9118 / 23618–29178 | 244.028 | 6.171 |
| qwen35-27b | 4/4 | 26745 / 3047 | 3321–7951 / 24785–29415 | 131.872 | 3.505 |
| gpt-oss | 0/4 | 24744 / 67668 | 3061–7359 / 25377–29675 | 379.068 | 7.153 |
| muse | 5/6 | 32531 / 35031 | 3004–7376 / 25360–29732 | 839.081 | 15.907 |
| gemma4-e4b | 2/6 | 39974 / 40127 | 3554–9114 / 23622–29182 | 439.857 | 7.966 |
| gemma4-26b-a4b | 3/6 | 39998 / 7977 | 3558–9118 / 23618–29178 | 252.710 | 7.300 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`. Backend counters cover 4/4 attempts; does not fit: 0.

qwen35-27b: causes `{"ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`. Backend counters cover 4/4 attempts; does not fit: 0.

gpt-oss: causes `{"generation-refused:error:chat.response.incomplete": 2, "strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"output-cap": 2, "stopped-below-token-limits": 2}`; response fields `{"content:absent-or-empty": 2, "content:nonempty": 2, "reasoning:absent-or-empty": 4, "reasoning_content:nonempty": 4}`. Backend counters cover 4/4 attempts; does not fit: 0.

muse: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 5}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:nonempty": 6}`. Backend counters cover 6/6 attempts; does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 2}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 5}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`. Backend counters cover 6/6 attempts; does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`. Backend counters cover 6/6 attempts; does not fit: 0.

Read-only diagnosis across all 180 attempts: `{"empty-old": 1, "generation-refused:complete:length": 4, "generation-refused:error:chat.response.incomplete": 8, "generation-refused:error:chat.transport.timeout": 3, "old-absent": 36, "old-ambiguous": 2, "old-not-visible": 1, "overlapping-edits": 9, "strict-edit-refused:code.proposal.format": 4, "unchanged-edit": 9, "valid-edit-conditions": 103}`. The largest refusal cause is `old-absent` (36 attempts); it determines the next investigation under MEASURE FIRST. The retained earlier classifier passes its 22 diagnostic controls and agrees with the strict checker on edit-condition acceptance/refusal. No source or answer is repaired; these are not task grades.

New content-bearing output-cap refusals: gemma4-e4b dev-025 input 4563, output 28173. Exact raw responses and terminal usage are retained and reproduced; none is repaired or rerun.

Previously recorded E4B and A4B dev-024 returned content with `finish_reason: length` at their
unchanged output budgets: E4B 27,107 output / 5,629 input tokens; A4B 27,103
output / 5,633 input tokens. The collector refused both before constructing
an edit or running the strict checker. Exact responses and records remain
retained; no repair or rerun occurred. Portable reproduction checks both
observed capped responses separately from the earlier gpt-oss cap.

Earlier transport timeouts remain preserved: muse dev-008, muse dev-011, muse dev-012. No terminal usage is invented and none is rerun. New attempts are reported separately.

No new transport timeout was observed.

This launch resumed the unchanged controller once. No attempt was rerun.
The earlier queue interruption and packaging correction remain
retained in the complete prior capsule and its exact external finalizer.

Controller wall time, including queues and fourteen released 30-second pauses:
6601.979 seconds. Reserved-job body time excludes queues and is neither GPU
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
checks the frozen source against Git and explicitly allows only the fifty-six
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
revoked; API and model are reaped; keys and sockets are absent. Thirty new token
retirement receipts are persisted and checked before unlinking; all one-hundred-fifty
preceding receipts remain preserved.

The host build wrapper changed at 2026-11-19 12:47:28 UTC. The launch
SHA256 is `4061f0e6d5741a9798882066897ee6852df563677726e9ce36e9c25b3bce27a6`.
Those retained bytes authenticate the wrapper at launch, not every later pair's
wrapper invocation. The checkpoint66 review's low finding remains in T-309: record
wrapper provenance per pair. The frozen controller remains unchanged.

The private review capsule retains the complete preceding capsule under
`prior/`, with all its original names, with source archive, input objects, provenance indexes and raw
records. Every retained byte is authenticated; immutable hardlinks avoid copying
large artifacts. Verification scans every retained path and byte once, then checks
all nested manifests' exact inventories, hashes and lengths against those observations.
Hardlinked files reuse a digest only while their full file identity agrees;
all paths are checked again for drift. Eleven synthetic controls agree with the
preceding verifier on valid input and refusals. No input binding is narrowed.
The previous exact external finalizer, original postcommit envelope and private
late-verdict timing correction script and receipt remain retained unchanged. Portable reproduction executes only the authenticated effect-free
helper: 30 contexts, 161 earlier strict checks, 165 current strict checks and
15 authenticated generation refusals across all 180 attempt records. It executes
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
unchanged. At reporting preparation, the preceding checkpoint independent PASS is
retained unchanged; no row closes. This checkpoint requires independent
core review. Existing T-309 lows remain backlog; no mid-repeat tuning occurs.
