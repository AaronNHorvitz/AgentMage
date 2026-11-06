# T-311.18: Text-proposal baseline freeze

Builder record based on `1104abe2`, before any model task request. T-311.18 and
parent T-311 remain open. The independent review of `d746e28e` closed only
expanded T-311.17; its five low findings are handled within this baseline row.
No new per-finding row is introduced.

## Running collector

The [fixed collector](../guides/coding-baseline.md) uses the existing version 1
chat API. Synthetic streams exercise collection, exact replacement application,
partial/length/format refusals, base fallbacks and bound aggregation. Its prompt
whitelists only the statement, relative editable path and complete base source.
No reference, hidden test, history or feedback is sent. No model text is executed
by the collector; snapshots require the independent grader.

This is a separately labelled chat-only text-proposal measurement, not execution
of the older native coding CLI or evidence that an agent tool loop works. That
CLI retains the deferred host executable-trust blocker. Neither its code nor the
API, transport, model codecs, permission rules or runtime activation changed.
The current chat path has no retries; one attempt is the entire frozen budget.
The fixed policy requests 4,096 output tokens and uses the existing 8K chat
context, not the separate 32K coding-preparation admission. Full-source context
refusals and all other failures remain in the denominator.

## Review backlog disposition

The retained [suite review](../reviews/d746e28e2438-local-coding-t311-suite-admission.txt)
has no high or medium finding. Its lows are addressed as follows:

- L1: worker coverage, admission limitations and the guide now explicitly name
  `candidate-writable-temporary-fixtures`. The bridge and trusted native fixtures
  share writable temporary files, including files a native fixture may execute.
  Separate temporary mounts/read-only native fixtures remain backlog; this batch
  does not claim to implement that isolation.
- L2: trusted worker records distinguish sticky fixture assertions, adapter
  refusals and wire faults. A caught fixture assertion is `test_failure` even
  when ordinary unittest failure counts are zero. An observed adapter refusal
  is `adapter_policy` and admission retains it as ungradable. Malformed frames
  and unexplained bridge exits remain `protocol`; classification describes
  observed boundaries, not an inferred internal cause of an exited candidate.
- L3: the historical scorer guide now scopes its generated refusal to that
  scorer and points to independent admission and inherited lane-quota limits.
- L4: a newly authored synthetic held-out admission test executes the real worker
  path and original synthetic assertions. It loads no actual held-out descriptor
  or hidden module and is not a held-out task/model score.
- L5: admission copies the known dev-003 process-observation race into every
  attempt row, including refused attempts. Such failures still earn no pass.

The original hidden worker subject, task definitions, statements, source/test
pins, selections, counts and thresholds are unchanged. The new worker tests are
separate synthetic fixtures; no hidden test was edited.

## Executed verification

All runtime verification used the shared build reservation, without changing a
quota or taking another worker's resources. The initial reservation wait is not
a code failure or test execution. No GPU, model or download was used here.

| Check | Observed result |
| --- | --- |
| Three targeted data-only checks | PASS, 0.092 seconds |
| Four complete focused modules | PASS, 38 tests, 8.441 seconds |
| Ordinary scoped fast gate against `1104abe2` | PASS, 62 tests, 8.689 seconds; full command 10.433 seconds |
| Complete frozen suite, `verify --purpose row-close` | PASS, all 80 discriminating pairs / 160 attempts; command 179.972 seconds |
| Frozen development and held-out commitments | Unchanged; 30 development / ten held-out tasks |
| Whole-helper source check | Matching; `source_changed: false`, `input_refused: false` |
| Independent receipt validation | PASS with current whole-helper source hashes and five bound control logs |
| Direct report/log audit | PASS, 160 reports / 420 direct log bindings |

The suite includes all historical and independent development and held-out
controls. Control runs validate the grader, not a model. No model score, token
count, GPU time, native agent result or T-311 completion is asserted by them.

| Retained artifact | SHA-256 |
| --- | --- |
| Collector source | `4ba8f4c9d098bdc9fb7e200e112d45d6683eab701aa875990dab3e62b05f61af` |
| Canonical fixed collector policy | `4fa9592e50311ebb4be9599f85deaf8938caa89f0c30b1b128583f84adb3b1ae` |
| Complete control receipt | `06d419f55deb10cbc62a56ca2a1c4c821f42cb4f23469af087f545dc474e5da1` |
| Unchanged worker subject | `bbda63ba2eed86834fdcdc379dcfc5f9ccc9faa4d94c31c8c976bfe0a344ecd6` |

Raw commands, verification logs and reports remain private for independent
review. Model collection must use this frozen source and policy, retain every
attempt, release the GPU between bounded batches and publish only actual
measurements. Held-out model collection is reserved for the row-closing
scoreboard. No supply-chain regeneration, long gate, release or owner-ready
claim is included in this batch.
