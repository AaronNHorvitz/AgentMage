# T-303/T-306: raw-source repeat blocked in the shared build queue

Date: 2026-11-19.
Reporting base: `c1bb214085bf9eca42b0a45599224fa2a77352fb`.
Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.

The unchanged controller was launched for three two-task jobs through the
required build and GPU wrappers. Its first job never acquired a build
reservation. All three shared build slots were observed occupied. After more than 30 minutes, the
builder ended this scheduling session by signalling only its own fresh waiting
process group; the launcher reaped it with signal 15. Queue-inclusive launch
time was **1940.391207 seconds**. This includes waiter retirement and is not
GPU time, model latency or a new campaign deadline.

**Zero new attempts.** The GPU wrapper, model server, API and task worker never
started. No model context or output budget was served. No new token count,
proposal, task pass, retry score or GPU-minute measurement exists. The frozen
repeat remains **6/180 attempted, 174 remaining**, as in the
[preceding checkpoint](t303-raw-source-checkpoint6-2026-11-19.md).

The wait log is preserved with its digest and original path in a retirement
receipt. Only this unstarted reservation log was archived so the unchanged
controller can recreate its exclusive output file when resumed. No attempt is
replayed or removed. An initial retirement guard refused before any signal
because it mistook a queued command argument for a running worker; both method
versions and the correction are retained privately.

Read-only authentication confirms all 2,290 repository and 62 private bindings,
the complete 9,760-file source snapshot, unchanged worker/controller/collector/
policy, and every preceding job and candidate byte. No owned waiting or worker
process remains; previous settings stay revoked and keys, sockets and tokens
remain absent. No foreign process was signalled. Wrappers, priorities, resource
limits, profiles, prompts, strict checker and grading boundaries are unchanged.
No model download, credential access or source edit occurred.

Resume the same frozen development repeat when shared build capacity is
available: all 30 tasks on all six profiles before tuning or capability work.
First-attempt and within-budget task passes remain unavailable because native
executable admission and original execution/grading controls are unavailable.
No candidate, project, hidden or held-out tests execute here. T-303/T-306 remain
open; independent core review is required for this reporting increment.

The reporting-only fast gate, Markdown render, privacy, licence, preservation,
queue/input audit and sealed-record authentication are checked. No heavy build,
source suite, portable model-helper reproduction, SBOM or long acceptance gate
runs in this session. Earlier source failures and supply-chain limits remain;
this record does not renew their verification. The 512 ledger table lines,
49 historical path blobs, inventory, Decision 0051, status model, independence
paragraph and previous scoreboard bytes remain unchanged.
