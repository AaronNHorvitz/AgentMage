# Inline-schema 31B resumed development measurement — 2026-11-16 UTC

**6/30 frozen development tasks attempted; 2 strict ungraded proposals.**
First-attempt and within-budget task passes remain **unavailable**.
The remaining 24 31B tasks and three full profiles (114 attempts) remain before tuning or new capabilities.
T-303/T-306 remain open; independent core review is required.

Measured source: `72c002a42957340de0b6fa0eb4d48385db2c7e62`. The available path is
existing Rust context selection → credential-owning chat API → original Rust
strict edit checker. Native executable admission and the original grading-control
receipt remain unavailable; full repository retrieval, native effects, tests,
feedback and retries cannot run. No control is reconstructed. No candidate,
hidden or held-out test ran. Proposals are unapplied and untested.

| Outcome / cause | Attempts |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 4 |
| `ungraded-proposal` | 2 |
| Does not fit | 0 |

Request/check time totals **305.215 seconds**. Completed-job reserved work
is **457.192 seconds / 7.620 minutes**, after frozen-input validation,
including model verification, loading, requests and cleanup. Queues, wrapper
and pre-entry overhead and released pauses are excluded. Active GPU kernel
time and continuous end-to-end elapsed time are unmeasured. Backend counters
cover **6/6**: **44823 input / 5522 output tokens**. Backend terminations:
`{"stopped-below-token-limits": 6}`. Captures:
`{"complete-choice-json": 6}`. Reasoning metadata:
`{"unavailable": 6}`; missing metadata is never zero.

Changed variable from the preceding 31B repeat: trusted named inline
`json_schema` requests with `strict: true`. Captured upstream bodies verify the
actual encoding. Every attempted context and message matches the preceding
repeat byte for byte. Model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option and 300-second total/idle deadlines
remain fixed. Context is 32,768 tokens. Output follows
`min(32768, 32768 - rendered input tokens - 32)`, ranging **23170–29511**;
preflight input ranges **3225–9566**. These are the prepared profile and transport
allowances. No answer is repaired or fallback added. Strict acceptance is not
proof of a correct solution, improvement or model-role qualification.

The resume launcher authenticates the sealed four-task prefix and prior queued
resume cleanup, every retained raw
capture and candidate file, 2,283 repository and 26 private frozen bindings,
and exact prior-owner absence. The four completed attempts are carried without
replay; only previously unattempted tasks run. Sources, profiles, controller
and batch remain fixed; only the controller's reservation-log parent changes.
Two earlier queued resumes stopped before GPU entry or any new attempt:
r3 after 1,278.079 seconds and r4 after 1,519.535 seconds. Their authenticated
stop and cleanup receipts remain in the sealed predecessor; neither counts
as a model failure. The later r6 queued stop waited 1,280.475 seconds,
also with zero GPU entries and zero new attempts; its stop, cleanup and
preservation receipts are authenticated before this resume.
The host-supplied build wrapper now offers a third slot when memory allows;
its bytes and the GPU wrapper are pinned at launch. Lane settings are unchanged.
Each two-task job uses both shared wrappers, a 900-second execution cap,
host low priority, nice 19, 16 GiB/four CPUs and 30-second released pauses.
Wrapper exits are recorded; no reservation or foreign process is bypassed.

All 3 completed jobs revoke settings and reap API/model owners; model keys
and sockets are absent. Cleanup receipts record **6 API token files retired in total**:
4 at the prior checkpoint and 2 in this resume.
The disjoint lists are authenticated and counted directly. Exact-argument scanning covers this PID
namespace only. Sampled total GPU peaks range
**20582–21162 MiB**.

Source/outcome audit, capture/context/profile comparison and original-helper
reproduction pass for all 6 attempted tasks. Only the original native context
and strict checker run on saved answers; generated code and hidden tests never
execute. The review capsule retains raw requests/responses, contexts, records,
candidates, all frozen inputs, prior comparison metadata and exact methods and
invocations. Its read-only verifier imports no archived module. Earlier capsules
remain immutable and separately hash-bound; binary provenance is unchanged.

Reporting fast gate, direct Markdown lint, table rendering, public-text hygiene,
ledger-row preservation and prior scoreboard-prefix checks pass. The source gate
is not rerun for reporting and remains **FAIL**: 1,169 passed, 53 inherited
failures and 19 ignored, as recorded in the
[source review](../reviews/ac1c30201171-trusted-inline-output-schema.txt).
No row closes; no release or owner-ready claim is made.

The next pair was stopped after 280.559 seconds waiting for shared build capacity, before GPU entry or a new attempt. Its exact own session was authenticated, signalled and rescanned empty. Queue time adds no model failure or retry. The 24 unattempted 31B tasks remain pending.
