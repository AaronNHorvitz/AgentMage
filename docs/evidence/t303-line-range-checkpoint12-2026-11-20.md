# T-303/T-306: frozen line-range twelve-attempt checkpoint

Frozen source and reporting base: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.

[Decision 0214](../decisions/0214-original-line-range-proposals.md)'s protocol is
frozen for a fresh development repeat. Two tasks on each of six profiles record
**12/180 attempted, 168 remaining**. Strict ungraded proposals: gemma4-31b 0/2; qwen35-27b 2/2; gpt-oss 1/2; muse 1/2; gemma4-e4b 0/2; gemma4-26b-a4b 1/2.
First-attempt and within-budget task passes remain unavailable. No model role
is selected and T-303/T-306 stay open. Complete all 30 tasks on all six profiles
before further tuning or capability work.

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 0/2 | 9207–9611 / 23125–23529 | 18818 / 2325 | 132.102 | 3.262 |
| qwen35-27b | 2/2 | 7990–8418 / 24318–24746 | 16408 / 2694 | 90.422 | 2.165 |
| gpt-oss | 1/2 | 7368–7737 / 24999–25368 | 15105 / 36961 | 199.545 | 3.600 |
| muse | 1/2 | 7364–7771 / 24965–25372 | 15135 / 19625 | 450.301 | 7.935 |
| gemma4-e4b | 0/2 | 9203–9607 / 23129–23533 | 18810 / 33055 | 301.528 | 5.263 |
| gemma4-26b-a4b | 1/2 | 9207–9611 / 23125–23529 | 18818 / 4084 | 131.606 | 3.229 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

qwen35-27b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gpt-oss: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

muse: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 1}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 1}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Backend counters cover 2/2 attempts; does not fit: 0.

The gemma4-e4b dev-002 response reached its 23,129-token output budget with
partial content. It was refused before the strict checker, without repair or
rerun; the candidate remains the unchanged base. The complete response and
usage are retained.

The changed variable is the complete original-line-range protocol: fixed
instruction, version 2 source headers, strict parser and trusted inline
`coding_lines_v1` schema. Compared with the prior raw-source repeat, excerpt
sets change on dev-002, dev-006, dev-007, dev-008, dev-013, dev-015, dev-016, dev-019 and dev-027. The complete
30-task preflight and actual contexts are retained. Raw source bodies remain
exact; no source or answer is repaired. A line address still has to satisfy the
same unique, visible, nonoverlapping exact-preimage checks.

Each profile serves a 32,768-token context. Per-task output is
`min(32768, 32768 - rendered input tokens - 32)`; the actual rendered token
counts, budgets and does-not-fit outcomes are recorded. Model/runtime profiles,
temperature, seed, 300-second deadlines and disabled-thinking wire request stay
unchanged. Observed response fields do not establish an absence of internal
reasoning. API token usage is not requested; observed backend counters are
reported separately. These are first generation attempts, with zero retries.

The worker, collector and controller change only the private namespace and the
bound output schema; an AST comparison verifies this. The same effect-free
helper entry is rebuilt against the frozen production library. The fresh
campaign retains every predecessor repository/private binding name, with all
2355 repository and 78 private inputs authenticated.
Its complete 9823-file Git source view is read-only.
All prior campaigns, candidates, capsules and source snapshots stay unchanged.

Only Rust retrieval, the credential-owning chat HTTP API, local inference and
the strict checker run. Native executable admission and original execution and
grading controls remain unavailable; none is reconstructed. Native effects,
project tests, feedback and retries cannot run. No candidate, hidden or held-out
code executes. Strict proposals are not solved tasks or coding admission.

Six bounded jobs independently acquire both host wrappers, with a 900-second
deadline per pair, low priority, nice 19, 16 GiB RAM, four CPUs and the existing
22,528 MiB sampled GPU guard. Both reservations release after every pair; five
30-second pauses also release them. Controller wall time including queues and
pauses is 1839.726 seconds. Body time excludes queues and is neither kernel
time nor full-reservation duration. Preparation, reporting and complete
end-to-end latency are unmeasured. No limit is raised, no foreign process is
signalled and no model is downloaded.

Wrapper bytes and optional threshold environment fields are retained at launch.
The host build wrapper offers up to six slots with the existing available-RAM
checks. Per-pair wrapper provenance remains the disclosed T-309 backlog item;
launch provenance does not prove every later wrapper invocation.

Settings are revoked; all API/model processes are reaped; keys and sockets are
absent. All twelve token retirement receipts are persisted and verified before
unlinking. Raw requests/responses, backend logs, full candidate bytes as data,
contexts, source hashes, settings, model identity and cleanup records remain in
the complete sealed review capsule. Its portable reproduction authenticates the
whole inventory and source archive, reruns 30 contexts, 165 prior checks and
185 range equivalences, then checks each new answer or generation refusal.
An interim live audit failed because a running job had not yet written its
final report. That failed check is retained; the final audit runs after all
six pairs finish. It changes no model input or attempt.
It executes only the authenticated effect-free helper, never candidate code.

The reporting fast gate, Markdown lint/render, privacy, licence, preservation,
raw-attempt audit, cleanup and capsule reproduction are checked for this batch.
The initial reporting gate found one extra blank line in TASKS.md; that formatting
error was corrected and the final gate rerun. The initial failed log is retained.
There are no repository source changes. The preceding full source gate remains
**FAIL**: 1,043 passed, 53 inherited failures, 11 ignored, zero filtered, across
31 targets. No failure is waived or called green. Earlier disclosed Python and
supply-chain limits remain. No SBOM regeneration, long acceptance gate, release
or Git push occurs.

All 512 ledger table lines, 49 historical path blobs, README, inventory,
Decision 0051, status model and supply-chain outputs remain unchanged. Earlier
scoreboard bytes are an exact prefix. This checkpoint requires independent
core review and closes no row.
