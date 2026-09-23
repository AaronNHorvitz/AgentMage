# Native Coding Campaign Protocol — 2026-09-23

Status: prospective development-evaluation protocol; no model qualified.
Execution result: Muse campaign13 passed all eight cases on one unchanged tuple;
GPT-OSS did not qualify. See the separate
[results record](coding-harness-campaign13-results-2026-09-23.md). The criteria
below were fixed prospectively and are not changed by those results.
Authority: Decisions 0054/0061/0062 and the owner's real-model continuation.
The private progress log declared this four-case/two-repetition protocol before
the first complete real-model diagnostic success. This document makes it reviewable.

## Exact candidates and admission boundary

Use only the two disabled 32K development tuples in
`model-profiles/exact-profile-catalog.json`, with the verified artifacts and
resource policy in `model-profiles/development/coding-model-lab.json`.
Use the pinned packaged llama.cpp b10423 Vulkan executable. Record every profile,
artifact, runtime, codec, template, tokenizer, decoding and binary identity from
the actual native request. Production activation and model gates remain closed.
GPT-OSS's maintainer conversion is not a first-party OpenAI artifact, and its
separate conversion/admission requirements are not satisfied by this campaign.

No download, replacement model, direct model-server probe, scripted reply,
reduced context, relaxed tool schema, broad grant or completion override counts.
Muse is the first diagnostic candidate; evaluate GPT-OSS separately through the
same cases. Existing failing diagnostics remain part of the overall record.

## Cases and fixed pass criteria

Run each case twice in fresh private repositories, sequentially, one model slot
at a time. Use the existing wrapper's synthetic fixtures and actual CLI/host;
do not supply the repair in the objective or alter tests to accept an answer.

| Case | Fixture/scenario | Required result |
|---|---|---|
| Repair | `repair` / `failed-test-repair` | Native failed validation, bounded code correction, fresh complete passing validation, current diff/status and verifier `SUCCESS` |
| New file | `new-file` / `new-file` | Native failed validation, exact absent-file creation with current parent observation, fresh complete passing validation, diff/status and verifier `SUCCESS` |
| Multi-file | `multi-file` / `multi-file` | Native failed validation, required bounded changes to both authorized files, fresh complete passing validation, diff/status and verifier `SUCCESS` |
| Stable | `stable` / `no-op` | Native complete passing validation, current diff/status, no workspace edits and verifier `NO_OP` |

Before writes, inspect the relevant current source or destination evidence using
the available native tools. The only command execution is the exact registered
`fixture.python-validation@1.0.0` template. No package installation, shell
substitution, Git commit or external service is needed by these fixtures.

A case passes only with the verifier outcome, expected filesystem changes,
retained native model responses and chronological tool receipts, CLI-verified
full artifacts, unchanged binaries and passing resource guard. Count every
invalid proposal, rejection, unfinished run and human intervention. Eight of
eight cases must pass on the same source/binary/profile tuple for a successful
bounded development campaign; this is not broader model admission or release.

## Bounds, failures and evidence

Keep 32768 served context, 4096 output reserve, four threads, one slot, 200% CPU,
low priority, 45-minute runtime, MemoryHigh 5G, MemoryMax 6G, swap max 512M,
at least 16 GiB available RAM and the existing start/sampled GPU guards. These
are unchanged diagnostic profiles, not new generation-budget experiments.

Retain full logs under the owner-only coding state directory, including exact
commands, elapsed time, resource samples, source/worktree/binary pins, prompts,
raw responses, tool/result bindings and all failures. The read-only
`scripts/coding_harness_model_evidence.py` checks the retained chain; it neither
executes effects nor grants model admission. Its reports supplement, not replace,
the existing canonical artifacts and verifier.

Do not repeat an unchanged failed integration run. Diagnose it, add a regression
and verify the correction before a new source-pinned attempt. A changed tuple
requires a new complete campaign; retain the earlier campaign as unsuccessful.
If a case remains unsuccessful without a fixable integration defect, record the
candidate's separate not-qualified disposition and continue other useful scoped
cases. Never combine favorable attempts from different tuples into eight passes.

Independent review of the connected implementation is still mandatory. The
implementing session cannot sign Task 50.2.4.7, `M-HARNESS-DAILY`, production
model admission, supported-platform or release completion.

## Prospective Decision 0074 correction accounting

Before its first native campaign, Decision 0074 enables one explicitly recorded
protocol/argument correction through the existing next-turn path and existing
one-parser-failure budget. Rejected bytes/arguments stay invalid and never count
as tools or evidence. A second parser rejection exhausts the run; all ordinary
limits and eight-of-eight same-tuple case criteria above remain unchanged.

The collector now checks a complete rejected response's original validated
resource/usage result, exact raw digest, canonical result bytes, matching failure
and turn-closure events, absence of proposal/permission/effects in that turn,
and exact later host-labelled feedback. It reports rejection counts separately.
An unbound/hidden rejection cannot pass. The terminal verifier, full artifacts,
real failed-test/edit/passing-test order and resource checks remain mandatory.
No old failed run is reclassified; the changed implementation starts a new campaign.
