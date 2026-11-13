# Muse JSON-object checkpoint — 2026-11-13 (incomplete)

**2/30 frozen development tasks attempted; 28 remain.** At source
`40cad6472560e47314a55ddb1f45e2c4c22b1792`, Muse produced 1/2 strict ungraded
proposals. Task passes, tests and retries remain unavailable. T-303/T-306 stay
open. This is a partial-path checkpoint, not a completed development measurement.

## Measured path and settings

Rust source selection receives each statement and its editable frozen base file,
then uses credential-owning chat and the unchanged strict edit checker. Full
repository retrieval is not measured. Native executable artifact admission and
the missing original grading-control receipt still block execution, tests and
retries. No candidate code, hidden or held-out test ran.

Changed variable from the preceding complete-response measurement: trusted
JSON-object output request, captured as `response_format: {"type":"json_object"}`
in both upstream bodies. Complete framing, prompts, selection, checker, byte
ceilings and 300-second idle/total deadlines stay fixed. Context: 32,768 tokens.
Output: `min(32768, 32768 - rendered input tokens - 32)`, here 24509–24616.
Both attempted inputs fit; **does not fit: 0/2 attempted**, with no claim about
the remaining 28. These are the largest allowances in the prepared 32K profile
and transport policy, not every physically possible context.

All 2,283 repository and 16 private input hashes were verified before the job.
The API SHA-256 is
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`;
the unchanged helper SHA-256 is
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
Muse SHA-256: `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`
(16,756,683,904 bytes). Profile: four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128,
`reasoning_strength: medium`. The driver remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, changed since preparation
and unqualified for coding. No download or role adoption occurs.
The [source review](../reviews/de8c204cbc52-json-object-model-output.txt) passes
exact source increment `de8c204cbc52c7b090f8039a4bfaf15666854852` only;
its low preflight finding stays T-309 backlog.

## Results and queue checkpoint

Causes: 1 `strict-edit-refused:code.proposal.edit`, 1 `ungraded-proposal`. First-attempt and within-budget task passes are **unavailable**.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8120 | 24616 | 8203 | `strict-edit-refused:code.proposal.edit` | 191.646 |
| dev-002 | 8227 | 24509 | 5864 | `ungraded-proposal` | 135.279 |

Request/check time: **326.925 seconds**.
One two-task reservation: **353.972 seconds /
5.900 GPU job minutes**, including verification,
load and cleanup. Both resource wrappers were used, with a 900-second GPU cap.
Queue waits, wrapper overhead and the 30-second pause outside both reservations
are excluded; active GPU kernel time is not measured. Final backend counters
cover 2/2 attempts: **16,347 input /
14,067 output tokens**, including non-answer output.
API usage was not requested. Backend terminations: `{'stopped-below-token-limits': 2}`.

The next pair waited **733.337 seconds** for
a shared build reservation, then checkpointed before GPU-wrapper entry, job
directory creation or any new attempt. The owned process group was paused and
its command lines rechecked before termination. No other worker was touched.
This wait is neither a model failure nor GPU execution time. Resume tasks 3–4
with a fresh controller log; preserve both completed attempts without replay.

## Verification and retention

Source/record audit, original-helper context/check reproduction and cleanup pass.
Settings were revoked, the API reaped, model key and socket removed, and no
campaign model/API/controller process remains. The separate read/hash-only
archive verifier passes **3,945 files**, reconstructing both
outcomes, requests, budgets, counters, times and frozen bindings. It extracts
nothing and executes no archived code. Two retired API tokens and all weights
are excluded; raw answers and candidates remain private. All 68 earlier sealed
artifact bindings are preserved. Prepared full-report helpers are inactive;
their presence is not a claim that the remaining tasks ran.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-muse-checkpoint-20261113-inputs.tar.gz` | `66b85753eb86fcec6736ff7f7e621a43ffff82d53f393dd37cb703895955cdf8` |
| `json-object-muse-checkpoint-20261113-manifest.json` | `ff9720425b1e2fbf0ee57844bce519f678ce3ec2a55ecb5909365ca79d496ad5` |
| Frozen inputs | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

Reporting-only fast gate, direct Markdown lint and scope/privacy checks are
recorded with the core review request. The last affected-source gate retained
53 inherited failures and is not rerun for this report. No source, prompt,
profile, task, threshold, permission, supply-chain output or ledger row changes.
The remaining Muse tasks and A4B repeat precede tuning except high/medium fixes.
No task-quality improvement, model qualification, row closure, release or
owner-test readiness is claimed. Independent review is required.
