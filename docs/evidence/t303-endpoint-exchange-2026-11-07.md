# T-303/T-306: shared endpoint exchange records

Date: 2026-11-07. Non-closing core work under Decisions 0153 and 0154.
Both parents remain open. Independent exact-commit review is required.

## Change

The credential-owning transport now uses the shared `EndpointModelRequest`,
including `EndpointModelContext`. `EndpointChatBinding` remains an alias; its
existing serialized fields keep their names and values. The additive context
records the ordered JSON message array's count, exact serialized byte length
and SHA-256, including roles and escaping. It describes neither rendered prompt
tokens nor model identity. The temporary message encoding is zeroized on drop.

Single-use `execute_recorded` returns `EndpointModelResult` only after the
existing complete HTTP and codec checks. The result carries the full prepared
request, truthful stop/length finish, digest and UTF-8 length of concatenated
decoded text, optional provider counts and local monotonic dispatch duration.
The old `execute`, chat and usage interfaces retain their return types and use
the same dispatch. The native exact-profile port and all its checks are unchanged.

The proposal API consumes these records for snapshot, workspace and repair
dispatch. Completed terminal events expose `runtime_model_result`; its request
equals both events' `model_binding`. Invalid strict edits and output-limit
finishes still retain completed inference but supply no accepted proposal.
Malformed streams, late bytes, cancellation and incomplete replies return no
completed result or accounting. Lost downstream replies grant no replay.

The duration includes socket waits and synchronous text callbacks, excludes
preparation/approval and is neither GPU nor server inference time. Resident and
accelerator memory remain null, as do verified input/output tokens. Missing,
zero and nonzero provider reports remain distinct. These are descriptive,
serializable contracts without deserialization or execution authority, and
cannot satisfy the coordinator's exact native profile/resource contracts.

The preceding repair-preparation [independent core review](../reviews/f92cea7f3183-local-coding-repair-preparation.txt)
arrived during this source gate and is retained byte-identically. It has
**Verdict: PASS** for exact commit `f92cea7f3183433161157294e0f1f033446bf958`
and no high/medium finding. Its low L1, route-level preparation-refusal coverage
with a native failed-test receipt, remains backlog in these same parent rows.
It accepts its own pinned increment only; neither parent closes.

## Verification

The unmodified scoped source gate selected ten affected workspace crates.
Six commands ran under one shared build reservation in **390.76 seconds**,
excluding the reservation wait. Formatting passed in 2.50 seconds and Clippy
with warnings denied passed in 48.84 seconds. Complete all-target tests ran in
338.12 seconds: **76 targets, 3,230 passed, 72 failed, 79 ignored, 0 filtered**.
The source gate remains **FAIL**: Cargo returns 101 and the gate returns 1.
No failure allowlist, skip, filter or threshold change converts this into a pass.

All 72 failure identities and normalized diagnostics match the retained complete
predecessor source log: 53 host and 19 Linux-platform failures. All 79 ignore
identities also match. The predecessor selected eleven crates after its lockfile
change; this batch changes no Cargo input and excludes the unrelated build-tool
crate's four passing tests and one target. This batch adds one passing test
method, accounting for the change from 3,233 to 3,230 passes. The native
executable-trust owner-action blocker remains open.

The new nine-case actual-socket regression passes unfiltered: nonzero, zero and
missing reports, a completed output-limit finish, malformed usage, truncated
completion, late bytes, cancellation and a rejected HTTP status. Successful
records match the prepared request, decoded UTF-8 text hash/length and provider
report; all verified token and server memory fields remain null. Dispatch time
is bounded between the sum of measured synchronous callbacks and the caller's
surrounding monotonic interval. A length finish retains its distinct disposition.
Every failure returns no completed result.

The strengthened prepared-request fixture independently checks the raw body and
escaped message array while proving that subsequent caller-text mutation cannot
change transmission. It checks response digests, limits, null resources and
credential/prompt redaction with usage both enabled and disabled. Existing
legacy chat/usage tests also pass through the same recorded dispatch.

API fixtures verify that the complete result request equals the initial and
terminal binding and hash the actual returned text events. Partial, cancelled,
malformed and late streams carry no result. The snapshot fixture independently
hashes the raw upstream message array. The four-case repair fixture also hashes
its raw three-message array, including escaped untrusted test feedback, and
checks response/result identity for valid/invalid edits with usage on/off.
Existing preparation/consumption-order regressions pass unchanged in scope;
their receipt callback remains synthetic, as disclosed by the retained review.

The predecessor's review arrived after source checks began. Final light checks
add its unchanged report-format check and verify the final documentation, exact
twelve-path scope, added-content privacy, no new ignores, unchanged ledger table
rows, Cargo inputs, all eight Rust source hashes and the source Cargo commands.
The reviewer source plan has seven commands rather than the builder's six solely
because it includes that review-format check. These light checks run no build or
full suite. Raw logs, source freeze, plans and comparison helpers remain in
private lane state.

## Limits

All socket peers and source content in these checks are synthetic. No native
workflow, local-model quality, GPU time or coding-score claim follows. Request
fingerprints do not prove receipt and cannot identify a unique attempt or grant
replay. Message-array hashes do not verify a server's rendering or tokenization.
Completion observations do not establish a successful edit, test or task.

Endpoint admission and resource-ledger integration in the existing coordinator,
native fail-and-repair execution, durable ownership, approved commit/push and
parent measurements remain open. No new ledger row or closure, hidden/frozen
test input, held-out set, scorer, threshold, ignore, dependency, sandbox, grant,
SBOM or implementation-status change is included. No model download or public
release is authorized by this batch.
