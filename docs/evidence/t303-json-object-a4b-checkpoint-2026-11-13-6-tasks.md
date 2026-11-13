# A4B JSON-object checkpoint — 2026-11-13 (6/30, incomplete)

**6/30 frozen development tasks attempted; 24 remain.** At source
`40cad6472560e47314a55ddb1f45e2c4c22b1792`, `gemma4-26b-a4b` produced 0/6 strict
ungraded proposals. Task passes, tests and retries remain unavailable.
T-303/T-306 remain open. This is a checkpoint of a partial coding path,
not a completed development measurement.

## Path and settings

The existing Rust source selector receives each task statement and editable
frozen base file. Credential-owning chat returns untrusted text to the unchanged
strict edit checker. Full repository retrieval, native execution, tests and
retries remain unavailable. Native executable admission and the missing original
grading controls remain blockers. No candidate, hidden or held-out test ran.

Changed variable from the preceding complete-response repeat: trusted JSON-object
output, captured as `response_format: {"type":"json_object"}` in all six exact
upstream requests. Complete framing, prompts, source selection, strict checker,
profiles, byte ceilings and 300-second idle/total deadlines stay fixed.
Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, here 23172–29513.
**Does not fit: 0/6 attempted.** No claim is made about the remaining inputs.
These are the largest allowances in the prepared 32K profile and transport
policy, not a claim about every physically possible context.

Preflight verifies 2,283 repository and 16 private inputs. Before launch it also
verified all 219 earlier retained artifacts, including the first-pair checkpoint. The API SHA-256 remains
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`;
the helper remains
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
The model SHA-256 is `a07f72221e8e3f77455ab0d7f7652d01a9f63c262b954aa6932a53275a0e895a`
(17,035,039,872 bytes). Its prepared profile retains
four threads, temperature 0, seed 42, Q8 caches, batch 256, microbatch 128 and
empty template arguments. The driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, changed since preparation
and unqualified for coding. No model was downloaded or adopted for a role.
The [independent source review](../reviews/de8c204cbc52-json-object-model-output.txt)
passes its exact source increment only; its low finding stays T-309 backlog.

## Outcomes and resumable boundary

Causes: 6 `generation-refused:error:chat.response.incomplete`. First-attempt and within-budget task passes are **unavailable**.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9502 | 23234 | 23234 | `generation-refused:error:chat.response.incomplete` | 200.139 |
| dev-002 | 9564 | 23172 | 23172 | `generation-refused:error:chat.response.incomplete` | 173.211 |
| dev-003 | 4752 | 27984 | 27984 | `generation-refused:error:chat.response.incomplete` | 232.599 |
| dev-004 | 3223 | 29513 | 29513 | `generation-refused:error:chat.response.incomplete` | 216.040 |
| dev-005 | 8460 | 24276 | 24276 | `generation-refused:error:chat.response.incomplete` | 205.712 |
| dev-006 | 9310 | 23426 | 23426 | `generation-refused:error:chat.response.incomplete` | 175.147 |

Request/check time: **1202.848 seconds**.
Three two-task reservations: **1282.759 seconds /
21.379 GPU job minutes**, including verification,
load and cleanup. Both shared resource wrappers ran, with a 900-second GPU cap.
Queue waits and wrapper overhead are excluded; active GPU kernel time is
unmeasured. Final backend counters cover 6/6 attempts:
**44,811 input / 151,605
output tokens**, including non-answer output. Missing counters are not zero.
Backend terminations: 6 attempts reached the output allowance. Transport outcomes alone do not establish
root cause. API usage was not requested.

The first controller completed tasks 1–2 before its earlier checkpoint. The
resume controller skipped that completed pair and finished tasks 3–6, each
exactly once, using fresh reservation logs. After the shared build queue cleared,
a graceful checkpoint signal targeted only the verified resume-controller PID
while tasks 5–6 were running. Its handler let that pair and cleanup finish
normally, then stopped before tasks 7–8. No model or worker was signalled,
no queue was cancelled, and no attempt was replayed. Both wrappers were released
after every pair. Resume tasks 7–8 with a fresh controller log. This report
includes the earlier two-task prefix; do not add the checkpoints together.

## Verification and retention

Source/record audit, original-helper context/check reproduction and cleanup pass.
Settings are revoked, the API is reaped, model key and socket are absent, and a
fresh process scan finds no campaign controller, model, API or worker. The
bound scan script and receipt explicitly record every matching method and pattern. The
separate read/hash-only archive verifier passes **7,165 files**,
reconstructing all six outcomes, requests, budgets, counters, times and frozen
bindings. It extracts nothing and executes no archived code. Six retired API
tokens and all weights are excluded. Answers and candidates remain private.
All 219 previous artifact bindings remain intact. Full-report helpers are
prepared but inactive; their presence does not establish measurement completion.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-a4b-checkpoint6-20261113-inputs.tar.gz` | `eb3f0ccd8b349c3d783acdfcd427e153d44d1e3a669f77d322e0c1567fae652e` |
| `json-object-a4b-checkpoint6-20261113-manifest.json` | `3b7f8abf08747a4c47634a26de40ead77d712be6fa3fdaf48df7fe12fe8eb4fe` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

Reporting-only fast gate, direct Markdown lint and scope/privacy checks are
recorded with the independent core review request. The last affected-source
gate's 53 inherited failures remain disclosed; it was not rerun for this report.
No source, prompt, profile, task, threshold, permission, supply-chain output or
ledger row changes. Finish the remaining 24 A4B tasks before tuning except
high/medium review fixes. No task-quality improvement, qualification, row
closure, release or owner readiness is claimed.
