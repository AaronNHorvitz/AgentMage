# Thinking-disabled A4B development repeat — 2026-11-13

All 30 frozen development tasks ran at source `a896f055256e96baec09966c71c5d0a7f1714d96`:
**16/30 strict ungraded proposals**, 11 format refusals and 3 length stops.
First-attempt and within-budget task passes remain **unavailable**.
T-303/T-306 remain open; independent core review is required for this report.

## Measured path and results

The available path selects context from the task statement and editable frozen
base file with the existing Rust helper, calls the credential-owning chat API,
and checks the answer with the unchanged Rust strict checker. This measures no
full repository retrieval, native effects, tests, feedback or retries. Native
executable artifact admission and the original grading-control receipt remain
blockers. No original control is reconstructed. No candidate, hidden or held-out
test ran. Accepted proposals remain isolated, unapplied and untested.

| Outcome / cause | Tasks |
| --- | --- |
| `ungraded-proposal` | 16 |
| `strict-edit-refused:code.proposal.format` | 11 |
| `generation-refused:complete:length` | 3 |

Request/check wall time totals **1289.452 seconds**.
The reported reserved-work timers total **1789.198
seconds / 29.820 minutes**. They start after frozen-input
validation, include model/runtime verification, loading, calls and cleanup,
and exclude queues, wrapper/pre-entry overhead and session gaps. Active GPU
kernel time and continuous end-to-end elapsed time across sessions are unmeasured.
These are reserved-work minutes, not active GPU utilization.

Backend counters and captured endpoint usage agree on **215953 input / 111638
output tokens**, covering **30/30** attempts. They remain provider/backend reports.
All 27 other backend stops are below token limits; the three length stops reach
their output allowances. No incomplete empty-answer response occurs in this
repeat. Proposals are not solved tasks, and these observations qualify no model
role or coding profile. The driver tuple remains unqualified.

## Changed variable and fixed inputs

The changed variable is trusted API `thinking_mode: disabled`, encoded as
`chat_template_kwargs: {"enable_thinking": false}` in every upstream request.
The server retains its default empty template arguments. Complete framing,
JSON-object output, prompts, strict edits, temperature 0, seed 42, four threads,
Q8 key/value caches, batch 256 and microbatch 128 remain fixed. Every captured
request matches the recorded context and budget. Context remains 32,768 tokens;
output is `min(32768, 32768 - rendered input tokens - 32)`, ranging
23002–29511. Input ranges 3225–9734; **does not fit: 0**. These are the largest
allowances in this prepared profile and transport, not a physical-context claim.
Idle and total deadlines remain 300 seconds.

The API SHA-256 is
`ba623b925414e0b390533b18c59b060ad7058c8996d44741420a29b433d05ac1`;
the unchanged helper SHA-256 is
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
The source freeze SHA-256 is
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd`,
binding 2,283 repository and 20 private inputs. All bindings pass before each
job and in the final audit. The actual copied A4B profile also matches its frozen
original byte for byte; both paths are retained. Model SHA-256 is
`a07f72221e8e3f77455ab0d7f7652d01a9f63c262b954aa6932a53275a0e895a`
(17,035,039,872 bytes). No model is downloaded or adopted for a role.

The [fixed first pair](t303-thinking-control-2026-11-13.md) is included in these
30 attempts, never added. Earlier server-option diagnostics and the failed
zero-task startup are separate and excluded from these totals. The previous
controller is absent in this session's PID namespace; its next-pair log contained
only a build-queue wait and no attempt directory existed. The resumed controller
preserves that script/log, changes only the new reservation-log suffix and skips
the completed pair. Its history accounts for the remaining 28 tasks exactly once.
Each pair uses a separate 900-second GPU job through both resource wrappers;
the resumed controller pauses 30 seconds outside both reservations after each job.
Queues are waited normally; no other worker or measurement queue is interrupted.

## Independent review and metadata erratum

The retained [source and first-pair review](../reviews/8c14051a2d1b-trusted-endpoint-thinking-control.txt)
returns **Verdict: PASS** for exact commit
`8c14051a2d1be63fb8382c9153df91a6d93f71d5`, with no high or medium finding.
It closes no row and does not review these later attempts. Its full source gate
reproduces 1,167 passed, 53 inherited failures and 19 ignored over 30 targets;
formatting and Clippy pass. **The source gate remains failing.** This reporting
increment changes no source, prompt, task, threshold, permission or SBOM.

Erratum for the review's low metadata finding: inherited private policy fields
`chat_transport_binary_source` and `comparison` still name the preceding source
and text-to-JSON-object experiment. For this repeat, the actual API source is
`a896f055256e96baec09966c71c5d0a7f1714d96` and the changed variable is the trusted thinking
option above, as `source_commit`, `source_product_commit`, `changed_variable`,
the API hash and all captured bodies establish. The stale labels affect no
number. Frozen policy and sealed checkpoints remain unchanged; later policy-label
hygiene remains T-309 backlog. This erratum applies to all 15 batch policy copies.

## Reproduction and retention

The original helper reproduces all 30 source selections and strict-check results,
including refusals, without candidate execution. All 15 jobs revoke settings and
reap their APIs/models; model keys and sockets are absent. The exact-argument
process scan finds no campaign owner in the current PID namespace and makes no
global-idle claim. The 28 remaining API token files are removed; the first pair's
two were already retired. Sampled total GPU peaks range 19534–20307 MiB.

The read/hash-only verifier passes **2796 files**. It extracts nothing,
executes no archived code, binds all frozen inputs and six changed Rust blobs to
the exact source commit, and independently recomputes requests, response captures,
backend counters, strict outcomes, selected candidate hashes and totals. Only
selected candidate source files are retained, never executable candidate trees.

- `thinking-disabled-a4b-20261113-inputs.tar` SHA-256:
  `0a361699473eea0bd91bc9df93e6e20106221f946e9d6065ac15647e35b99f55`.
- `thinking-disabled-a4b-20261113-manifest.json` SHA-256:
  `b3c1e23eabd12a6bf4e656593e1cba6f1adb79fb65348869791beec713dd33e0`.

The reporting fast gate, direct three-file Markdown lint and actual table
rendering pass. All 507 ledger table lines and the prior scoreboard prefix are
preserved. Added-content privacy/scope checks, all original frozen bindings and
all 11 artifacts of the preceding source-review capsule pass unchanged.

Strict format refusal (11) is the largest refusal cause in this profile. The other
five profiles have no fresh post-change result here; finish required development
measurements before further tuning. No hidden/held-out run, task-quality claim,
model qualification, row closure, release or owner readiness follows.
