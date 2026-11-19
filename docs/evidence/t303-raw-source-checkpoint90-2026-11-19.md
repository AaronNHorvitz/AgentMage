# T-303/T-306: frozen raw-source ninety-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `297b433d12a4b4bb54e37b297942c9cca167d3d2`.

Three unchanged-controller pairs add six development attempts. The repeat is
**90/180 attempted, 90 remaining**. Cumulative strict ungraded proposals:
gemma4-31b 12/16; qwen35-27b 15/16; gpt-oss 2/16; muse 7/14; gemma4-e4b 3/14; gemma4-26b-a4b 6/14. First-attempt and within-budget task passes remain unavailable.
Finish all 30 development tasks on all six profiles before tuning or capability
work. No model role is selected and no row closes.

| New profile attempts | Strict ungraded proposals | Observed backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 17936 / 1673 | 8808–9128 / 23608–23928 | 107.397 | 2.752 |
| qwen35-27b | 2/2 | 16169 / 1721 | 7959–8210 / 24526–24777 | 68.092 | 1.710 |
| gpt-oss | 0/2 | 14738 / 29402 | 7240–7498 / 25238–25496 | 179.052 | 3.308 |

gemma4-31b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

qwen35-27b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 1}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 1}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

The earlier Muse dev-008 transport timeout remains preserved with no terminal
usage. Its missing counters are not invented and the attempt is not rerun.
The new attempts are reported separately from that earlier timeout.

No new transport timeout was observed.

The new gpt-oss dev-016 attempt reached its 25,496-token output cap with
partial content and reasoning. Its terminal event is `complete:length`; the
collector refused it, preserving the base fallback with no answer or strict
check. Portable reproduction authenticates that exact additional refusal case,
including raw response bytes, token count and terminal event. Earlier refusal
checks remain intact; no proposal checker, campaign input or threshold changes.

Controller wall time, including queues and two released 30-second pauses:
533.204 seconds. Reserved-job body time excludes queues and is neither GPU
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
checks the frozen source against Git and explicitly allows only the thirty-five
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
retirement receipts are persisted and checked before unlinking; all eighty-four
preceding receipts remain preserved.

The host build wrapper changed at 2026-11-19 12:47:28 UTC. Its new launch
SHA256 is `4061f0e6d5741a9798882066897ee6852df563677726e9ce36e9c25b3bce27a6`.
Those retained bytes authenticate the wrapper at launch, not every later pair's
wrapper invocation. The checkpoint66 review's low finding remains in T-309: record
wrapper provenance per pair. The frozen controller remains unchanged.

The private review capsule retains the complete preceding capsule under
`prior/`, with all its original names, with source archive, input objects, provenance indexes and raw
records. Every retained byte is authenticated; immutable hardlinks avoid copying
large artifacts. Portable reproduction executes only the authenticated effect-free
helper: 30 contexts, 161 earlier strict checks, 83 current strict checks and
7 authenticated generation refusals across all 90 attempt records. It executes
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
unchanged. The preceding reporting commit is under independent core review;
no verdict is inferred. This checkpoint requires independent
core review. Existing T-309 lows remain backlog; no mid-repeat tuning occurs.
