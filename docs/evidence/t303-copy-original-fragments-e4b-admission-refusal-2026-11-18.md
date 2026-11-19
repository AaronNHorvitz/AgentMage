# T-303/T-306: E4B resumption refused before GPU admission

Date: 2026-11-18 (local); reservation attempt: 2026-11-19 UTC.
Reporting base: `b6cae1f15ceb7d6d13f12f915f2a06b9e7c5f4d2`.
Frozen prompt/checker source: `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Independent core review required; no row closes.

| Observation | Result |
| --- | --- |
| Planned work | E4B dev-008 through dev-011, two fresh two-task jobs |
| GPU admission | Refused by the existing wrapper before its model command |
| New inference attempts / completed jobs | 0 / 0 |
| New task passes, first attempt / retries | Unavailable / unavailable |
| New model tokens / model GPU minutes | None; no model process started |
| Frozen repeat progress | Unchanged: 157/180 attempted; 23 E4B attempts remain |

The original preceding verifier passed before resumption: all twenty dependency
capsules, the preceding increment, complete committed diffs, raw records and
live frozen state authenticated. Its result explicitly checked the postcommit
receipt. Twenty-one dependencies are retained for this increment. The complete
9,706-file Git-backed source snapshot passed preparation and import checks and
was mounted read-only for the controller. Preparation released its build slot
before the controller began.

The original controller cannot skip its failed dev-007 job. A separate explicit
resumption controller therefore selected index 7, without modifying the frozen
worker, collector, helper or policy. Its first job requested the existing build
and GPU wrappers with a 900-second deadline, low priority, nice 19, 16 GiB RAM
and four CPUs. The GPU wrapper returned exit 1 with:

```text
GPU baseline exceeds 4 GiB or hardware is ambiguous.
```

The resumption controller consequently exited 1. The refusal occurred before
the wrapper invoked the original worker: neither planned job directory exists,
no new candidate exists and no model process started. This is a resource
admission refusal, not a model failure, a task attempt or a does-not-fit result.
No task score, token counter or GPU reservation duration is inferred from it.

A separate subsequent read-only status sample at
`2026-11-19T02:38:08.362103+00:00` is retained in the private raw record; its
timestamp is authoritative there. It reported automatic scheduling and
18,179 MiB used of 24,564 MiB. That later sample is not a measurement of the
exact refused baseline, and its memory use is not attributed to any process.
No other worker or GPU process was stopped. Resource limits remain unchanged.

The preserved dev-007 interruption still counts as the seventh E4B attempt;
dev-008 in that old failed job still contains only its original context file.
No completed or interrupted inference was replayed. All 157 previous attempts,
their raw bytes, the failed job, measured settings, 2,283 repository and 44
private input bindings remain unchanged. The new reservation refusal log is
retained separately and must never be overwritten.

The measured path remains partial: Rust context selection, the credential-owning
chat HTTP API, local inference and strict edit checking. Native executable
admission and original execution/grading-control receipts are unavailable.
Effects, project tests, feedback and retries cannot run; no candidate, hidden
or held-out test executes. T-303/T-306 stay open and no model role qualifies.

Cleanup confirms all 79 preceding jobs retain revoked settings, reaped
API/model processes and absent keys, sockets and API tokens. No owned
controller, worker or API process remains. Raw refusal, launch, resource,
binding, dependency and preservation checks accompany the review capsule.
Reporting fast gate, Markdown lint/render, privacy and licence checks pass.
Source is unchanged, so source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields
and previous scoreboard bytes are preserved. No SBOM regeneration, full
acceptance, promotion or release runs. Existing T-309 backlog remains open.

Next: resume at dev-008 only when the unchanged wrapper admits a resource job.
Use a fresh reservation log and launcher receipt; preserve this refusal and the
original failed job. Finish the remaining 23 attempts before tuning or capability
work. The preceding exact-commit review was in flight at session start; no
verdict is inferred. Only the independent reviewer accepts.

The preceding exact-commit review subsequently returned PASS with no high or
medium findings. Its unchanged report is retained separately. Two low
presentation findings join T-309: distinguish the 3/6 completed-attempt proposal
figure from the 3/7 total including the interruption, and describe the old
dev-008 failure as prompt rendering after context construction completed.
These are clarifications of the prior record, not new scores. No row closes.
