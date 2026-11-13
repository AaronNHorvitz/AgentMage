# GPT-OSS JSON-object checkpoint — 2026-11-13

**Incomplete: 4/30 development tasks attempted; 26 remain.** The measured source is
`40cad6472560e47314a55ddb1f45e2c4c22b1792`. There are **0/4 strict ungraded
proposals**, not a task pass rate. First-attempt and within-budget task passes
remain unavailable. T-303/T-306 remain open; no row closes.

The available partial path is Rust source selection → credential-owning chat API
→ unchanged strict edit checker. Selection receives the statement and editable
frozen base file; full repository retrieval is not measured. Native executable
artifact admission and the missing original grading-control receipt still block
edits, tests and retries through the native coding path. No candidate code,
project test, hidden test, feedback, retry or held-out task ran.

## Fixed inputs and measured results

The changed variable from the preceding complete-response repeat remains the
trusted JSON-object output request. All four captured upstream requests contain
`response_format: {"type":"json_object"}`. Complete framing, source selection,
prompts, checker, profiles, byte ceilings and 300-second idle/total deadlines
remain unchanged. Context is 32,768; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging 24,497–29,748 for
these four tasks. Input ranges 2,988–8,239; does not fit: **0/4 attempted**.
These are the largest allowances in the prepared 32K profile and transport
policy, not a claim about every physically possible context.

The existing `gpt-oss` profile retains four threads, temperature 0, seed 42,
Q8 key/value caches, batch 256, microbatch 128 and `reasoning_effort: medium`.
The model SHA-256 is
`27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`
(12,109,566,624 bytes). The changed driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, unqualified for coding.
No download or model role adoption occurs.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8176 | 24560 | 6640 | `strict-edit-refused:code.proposal.edit` | 38.246 |
| dev-002 | 8239 | 24497 | 24497 | `generation-refused:error:chat.response.incomplete` | 122.840 |
| dev-003 | 4349 | 28387 | 28387 | `generation-refused:error:chat.response.incomplete` | 146.869 |
| dev-004 | 2988 | 29748 | 29748 | `generation-refused:error:chat.response.incomplete` | 144.963 |

Final backend counters cover **4/4 attempts: 23,752 input / 89,272 output
tokens**, including non-answer output. Each incomplete response exhausted its
output allowance. Backend termination counters show three output caps and one
stop below the token limits; they do not establish the cause of the long output.
Request/check time totals **452.918 seconds**. Two separate two-task reservations
total **480.559 seconds / 8.009 GPU job minutes**, including verification, load
and cleanup. Queues, wrapper overhead and the 30-second pauses outside both
reservations are excluded. This is reserved job wall, not active GPU kernel time.
Both resource wrappers were used; the GPU wrapper caps each job at 900 seconds.

## Queue checkpoint and verification

Before tasks 5–6, the controller waited **404.835 seconds** for the shared build
reservation without entering the GPU wrapper or creating that pair's directory.
A guarded checkpoint stopped only the owned controller/build-wait process group,
verified its exact arguments and completed reports, then terminated and reaped
that queued group. The controller's resulting queue-cancellation traceback is
retained. No running model was interrupted, no completed attempt was replayed,
and queue cancellation is neither a model failure nor GPU work. The earlier
initial zero-attempt queue checkpoint also remains retained. Resume at pair 04
(tasks 5–6), preserving pairs 00 and 02; the measurement is not complete.

The partial source/record audit verifies all 2,283 repository and 16 private input
bindings, the four task identities, exact requests, candidate/base hashes, strict
checks, counters and costs. The unchanged helper reproduces all four contexts
and available strict checks without executing candidates or hidden tests.
Cleanup passes for both pairs: settings revoked, API processes reaped, model keys
and sockets absent, no resource-guard errors and no matching campaign processes.
The earlier three profiles' 33 sealed artifacts remain unchanged.

The read/hash-only archive verifier passes **5,322 files**. It extracts nothing
and imports or executes no archived code. Four retired API token files are
excluded; no model weights are packaged. Raw answers and candidates stay private.
The first archive preparation guard refused the queue-cancellation traceback;
its log check was corrected before any archive was created. That failed
preparation did not rerun a model attempt; the corrected verifier is retained.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-gpt-partial-20261113-inputs.tar.gz` | `ea66c2bf6028adb270d9a605f05cec5e3b2ecdca049ab6f1c45e0dc90fb74467` |
| `json-object-gpt-partial-20261113-manifest.json` | `e99679ed87a0ed8657fbec12ebe75601579fd8a2c708ee192df485637eda8f06` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

This reporting-only increment changes no source, prompt, profile, task, threshold,
permission or supply-chain output. The reporting fast gate is run separately;
the last affected-source gate's 53 inherited failures remain unresolved. Resume
the remaining GPT-OSS tasks, then Muse and A4B, before tuning except high/medium
review fixes. Independent core review of this checkpoint report is required.
No task-quality improvement, native qualification, release or readiness is claimed.
