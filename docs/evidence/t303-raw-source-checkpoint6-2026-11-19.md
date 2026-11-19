# T-303/T-306: frozen raw-source six-attempt checkpoint

Frozen source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.
Reporting base: `d9bbbd25fb0b3be4d21c731a0078a5cd7762a262`.

This checkpoint adds four development attempts across two profiles. The frozen
repeat is **6/180 attempted, 174 remaining**. Cumulative strict ungraded
proposals: gemma4-31b 2/2; qwen35-27b 2/2; gemma4-26b-a4b 1/2. First-attempt and within-budget task passes remain
unavailable. Complete all 30 development tasks on all six profiles before
further tuning or capability work; no model role is selected here.

| New profile attempts | Strict ungraded proposals | Backend input / output tokens | Rendered input / requested output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 18392 / 3005 | 9085–9307 / 23429–23651 | 119.055 | 2.645 |
| qwen35-27b | 2/2 | 16057 / 1826 | 7891–8166 / 24570–24845 | 63.291 | 1.484 |

gemma4-31b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

qwen35-27b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

Controller wall time, including queues and the released 30-second pause:
910.410 seconds. Reserved-job body time excludes queues; it is neither GPU
kernel time nor full reservation duration. Snapshot preparation, reporting,
continuous campaign time and complete end-to-end latency are not measured here.

No measured variable changes: the raw UTF-8 source presentation in
[Decision 0213](../decisions/0213-raw-source-presentation.md), fixed instruction,
all 30 development excerpts, exact checker, schema, profiles, model/runtime,
thinking request, 32,768-token served context, output formula
`min(32768, 32768 - rendered input tokens - 32)` and 300-second deadlines stay
frozen. The wire request disables thinking; observed response fields do not
establish an absence of internal reasoning. Earlier attempts remain unchanged.

The existing Rust context selector, credential-owning chat HTTP API, local
inference and strict checker run. Native executable admission and the original
execution/grading controls remain unavailable. Native effects, project tests,
feedback and retries cannot run. No candidate, hidden or held-out code executes.
These proposals are ungraded; the checkpoint is a partial-path measurement,
not solved tasks or an executable coding workflow. T-303/T-306 stay open.

The unchanged controller runs against the same complete 9,760-file read-only
source snapshot and authenticates all 2290 repository and
62 private bindings. The reporting HEAD is recorded separately
from the frozen source. An initial launcher check incorrectly compared the
frozen TASKS.md bytes to newer reporting bytes; it refused before acquiring a
reservation or starting inference. That launcher and failure log are retained.
The corrected launcher checks the frozen Git source, the complete snapshot and
the exact four reporting-only paths changed since the source. Worker,
collector, controller and model inputs remain byte-identical.

Each pair independently reacquires the unchanged build and GPU wrappers with
a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the same
22,528 MiB sampled GPU guard. Both reservations release between pairs. No
foreign process is signalled, no limit is raised and nothing is downloaded.
Settings are revoked; API and model processes are reaped; keys and sockets are
absent. Four new token retirement receipts are persisted and checked before
unlinking. The preceding two receipts and all old attempts remain preserved.

The private review capsule retains the complete preceding capsule, all of its
input objects, source archive and provenance indexes, plus the complete current
campaign records. Every original binding name remains available; the capsule
verification hashes every retained byte. Its portable reproduction executes
only the authenticated effect-free helper to reproduce 30 contexts, 161 earlier
strict checks and all six current checks; it never executes candidate code.

The reporting-only fast gate, Markdown render, privacy, licence, preservation,
raw-attempt audit, cleanup and portable verification/reproduction are checked
for this increment. No source check is presented as newly green: the preceding
full source gate remains FAIL, with 1,034 passed, 53 inherited failures,
11 ignored, zero filtered, across 30 targets. The two inherited Python failures
and supply-chain limits remain open. No SBOM or long acceptance gate runs here.

All 512 ledger table lines, 49 historical path blobs, inventory, Decision 0051,
status model, independence paragraph and preceding scoreboard bytes remain
unchanged. The preceding source/measurement commit was under independent core
review at session start. Its verdict is not inferred. This checkpoint requires
its own independent core review; no row closes. Existing T-309 backlog remains.
