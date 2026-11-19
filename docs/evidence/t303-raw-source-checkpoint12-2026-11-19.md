# T-303/T-306: frozen raw-source twelve-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `208a631a9537050591ee631fe451f6915d1b25f8`.

Three unchanged-controller pairs add six development attempts. The repeat is
**12/180 attempted, 168 remaining**. Cumulative strict ungraded proposals:
gemma4-31b 2/2; qwen35-27b 2/2; gpt-oss 0/2; muse 2/2; gemma4-e4b 0/2; gemma4-26b-a4b 1/2. First-attempt and within-budget task passes remain unavailable.
Complete all 30 development tasks on all six profiles before further tuning
or capability work. No model role is selected and no row closes.

| New profile attempts | Strict ungraded proposals | Backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gpt-oss | 0/2 | 14837 / 32215 | 7292–7545 / 25191–25444 | 163.163 | 3.015 |
| muse | 2/2 | 14866 / 11551 | 7287–7579 / 25157–25449 | 274.721 | 4.995 |
| gemma4-e4b | 0/2 | 18384 / 3366 | 9081–9303 / 23433–23655 | 57.244 | 1.165 |

gpt-oss: causes `{"strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

muse: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

Controller wall time, including queues and two released 30-second pauses:
680.243 seconds. Reserved-job body time excludes queues; it is neither GPU
kernel time nor full reservation duration. Preparation, reporting, continuous
campaign time and complete end-to-end latency are not measured here.

No measured variable changes. The raw UTF-8 source presentation in
[Decision 0213](../decisions/0213-raw-source-presentation.md), fixed instruction,
all 30 development excerpts, exact strict checker, schema, model/runtime,
profiles, thinking request, 32,768-token served context, output formula
`min(32768, 32768 - rendered input tokens - 32)` and 300-second deadlines stay
frozen. The wire request disables thinking; observed response fields do not
establish an absence of internal reasoning. All previous attempts are preserved.

The existing Rust context selector, credential-owning chat HTTP API, local
inference and strict checker run. Native executable admission and original
execution/grading controls remain unavailable. Native effects, project tests,
feedback and retries cannot run. No candidate, hidden or held-out code executes.
These are ungraded proposals from a partial path, with no solved-task claim.
T-303/T-306 remain open.

The unchanged controller runs against the complete 9,760-file read-only source
snapshot, authenticating all 2,290 repository and 62 private bindings. The
launcher checks the frozen Git source and the exact eight reporting-only paths
changed since that source; it records the current reporting HEAD separately.
Worker, collector, controller, policy and measured inputs remain byte-identical.

Each pair independently reacquires the unchanged build and GPU wrappers with
a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the same
22,528 MiB sampled GPU guard. Both reservations release between pairs. No
foreign process is signalled, no limit is raised and nothing is downloaded.
Settings are revoked; API and model processes are reaped; keys and sockets are
absent. Six new token retirement receipts are persisted and checked before
unlinking. All six preceding receipts remain preserved.

The review capsule retains the complete preceding checkpoint and parent-review
capsules, source archive, all input objects, original names, provenance indexes
and raw records. Every retained byte is authenticated. Portable reproduction
executes only the authenticated effect-free helper to reproduce 30 contexts,
161 earlier strict checks and all 12 current checks; it never executes candidate
code or inference.

The reporting-only fast gate, Markdown render, privacy, licence, preservation,
raw-attempt audit, cleanup and portable verification/reproduction are checked
for this increment. No source suite is rerun: the preceding source gate remains
FAIL, with 1,034 passed, 53 inherited failures, 11 ignored, zero filtered,
across 30 targets. Two inherited Python failures and supply-chain limits remain.
No SBOM or long acceptance gate runs here.

All 512 ledger table lines, 49 historical path blobs, inventory, Decision 0051,
status model, independence paragraph and preceding scoreboard bytes remain
unchanged. The preceding reporting increment and queue-block record both received
independent PASS verdicts with no high or medium finding. Their reports are
retained unchanged. The reporting review's low stale-status wording finding
is corrected; the queue review's low receipt-derivation finding joins T-309.
This checkpoint needs its own independent
core review. Existing T-309 lows remain backlog; no tuning occurs mid-repeat.
