# Inline-schema 27B development measurement — 2026-11-16 UTC

**30/30 frozen development attempts; 28 strict ungraded proposals.**
First-attempt and within-budget task passes remain **unavailable**.
60 attempts remain before tuning or new capabilities.
T-303/T-306 stay open; this reporting batch requires independent core review.

Measured source: `72c002a42957340de0b6fa0eb4d48385db2c7e62`. The available path is existing Rust
context selection → credential-owning chat HTTP API → original strict edit
checker. Native executable admission and original grading controls remain
unavailable. Full repository retrieval, native effects, project tests, feedback
and retries cannot run. No control is reconstructed. No candidate, hidden or
held-out test ran; proposals remain unapplied and untested.

| Outcome / cause | Attempts |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 2 |
| `ungraded-proposal` | 28 |
| Does not fit | 0 |

Request/check time: **1260.537 seconds**. Completed-job reserved work:
**1666.284 seconds / 27.771 minutes**, including model verification,
loading, requests and cleanup after frozen-input validation. Queues, wrapper
overhead, pre-entry validation and released pauses are excluded. The recorded
controller launch-to-exit UTC interval is **3800.162 seconds**, including
queues and released pauses; authentication, preflight and reporting are outside
that interval. GPU kernel time and complete end-to-end latency are unmeasured. Backend counters
cover **30/30**, with **189334 input / 37798 output tokens**.
Backend terminations: `{"stopped-below-token-limits": 30}`.
Captures: `{"complete-choice-json": 30}`. Reasoning metadata:
`{"unavailable": 30}`. Missing metadata is not zero.

Changed variable from this model's preceding repeat: trusted named inline
`json_schema` with `strict: true`, verified in upstream request captures.
Contexts and messages match that repeat byte for byte. Model/profile, runtime,
helper, prompts, strict checker, complete framing, requested thinking option
and 300-second total/idle deadlines remain fixed. Context is 32,768 tokens;
output follows `min(32768, 32768 - rendered input tokens - 32)`, ranging
**24137–29804**. Input preflight ranges **2932–8599**. No answer repair or
fallback is added. Strict acceptance does not establish a correct solution,
improvement, model admission or role qualification.

## Frozen source and preparation

The licence batch changed 195 working-tree files covered by the frozen source
manifest. The launcher mounts a complete, authenticated **9,630-file Git
archive** of the measured source read-only at its original repository address,
with the existing Git metadata also mounted read-only. Existing device nodes
are preserved. All **2,283 repository and 26 private frozen hashes** remain
required and pass. No input is removed, narrowed or renewed for changed bytes.
The complete archive is additionally authenticated against the source Git tree.
The original batch, collector, binary, helper and controller logic are unchanged;
only the controller's reservation-log parent changes. This continues the frozen
measurement; it does not verify the relicensed working tree.

Before inference, all **30 development tasks** are materialized through their
original Git bases and the unchanged native context helper. Every context
matches the prior repeat byte for byte. The earlier sealed preflight is
re-authenticated and a fresh preflight reproduces it inside the actual mount.
No generated candidate or hidden test executes in either preflight.

Three failed source-only preparations are retained in full. The first lost
existing device access while queued and stopped before GPU entry. The second
lacked the runtime manifest and used **18.443 seconds** of reserved preparation,
without starting a model. The third lacked Git history and used **44.628
seconds**, starting the model but failing before the first inference request.
All three produced **zero task attempts**. These preparation times are separate
from completed-job work and task outcomes. Exact-owner cleanup, settings
revocation, API/model exits and the third preparation's two retired token files
remain bound in the capsule. No failed setup is silently replayed or scored as
a model failure. The source-only approach stopped after these three failures.

The new full-Git approach passed preflight before GPU entry. Its initial
preparation-script invocation refused an already existing empty Git mountpoint;
that off-GPU check made no launch or resource reservation. The original script
and corrected derivation are retained. No permission or device was added.

The completed 31B capsule, postcommit receipt, saved attempts and live candidate
bytes are authenticated before this profile transition. Completed attempts are
never replayed. The preceding 27B archive and comparison manifest are
hash-bound; its original report comes from the archive, not a live mutable copy.

## Resources and verification

Each two-task job uses both shared wrappers, low priority, nice 19, 16 GiB/four
CPUs, a 900-second GPU reservation and 30-second released pauses. Optional
wrapper overrides are recorded absent. All 15 completed jobs revoke settings,
reap API/model owners and leave model keys and sockets absent. Cleanup counts
**30 retired API token files** from the actual list, separate from the failed
preparation's two. Exact-argument scanning covers this PID namespace only.
Sampled total GPU peaks range **17987–18504 MiB**. No foreign process is signalled.

Source/outcome audit, capture/context/profile comparison and original-helper
reproduction pass. Only the original context and strict checker process saved
answers; generated code and hidden tests never execute. The sealed review
capsule retains raw requests/responses, candidates, contexts, records, all
frozen inputs, the complete source archive, three failure archives, comparison
receipts and exact methods/invocations. The verifier reads and hashes these
files and Git objects; it imports no archived module. It retains all **19,241
candidate projection files**, checks their exact original projection membership
and verifies every auxiliary file against its frozen Git base.

The initial packaging script refused a complete projection because an unrun
preparation method assumed a one-file directory. Its partial copy and original
method are preserved; the copy is hash-bound as a subset of the final capsule.
The corrected packaging retains the complete projections. No model answer,
candidate, task, input or measurement result changed, and nothing was replayed.

Reporting gate, Markdown lint/rendering, public-text hygiene and preservation
checks pass. The earlier scoreboard is an exact prefix and all **509 ledger
table lines** stay unchanged. The full source/acceptance gates are not rerun.
Inherited source **FAIL: 1,169 passed / 53 failed / 19 ignored** remains disclosed
in the [source review](../reviews/ac1c30201171-trusted-inline-output-schema.txt).
The licence batch records two inherited Python gate failures and its
supply-chain catch-up limits in its
[evidence](readme-and-dual-licence-2026-11-16.md).
No row closes; no release or owner-test-ready claim is made.
