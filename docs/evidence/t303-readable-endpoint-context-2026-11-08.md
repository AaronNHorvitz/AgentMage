# T-303 / T-306: readable verified endpoint observations

Date: 2026-11-08. Tier: core. Immutable predecessor:
`92c919078ec131c6074e081d4c05137b59943834`.
Both parents remain OPEN. No row or sub-row is added or closed.
Independent core re-review is required.

## Review finding and repair

The concrete context's independent review has Verdict: FINDINGS for exact
commit `aa1705bf758ae91e65f8421ea74802f1c7e6b4f5`. The report is retained
byte-for-byte. Medium M1 observes that default serialization exposes opaque
payloads as decimal byte arrays, making read results unusable to a coding model.
The retained-owner review arrived during validation: Verdict: PASS for exact
commit `92c919078ec131c6074e081d4c05137b59943834`, with no high/medium
finding. Its report is also retained unchanged. Low L1 (caught-panic owner
reuse), L2 (terminal-envelope invariant error latch) and L3 (faulted-slot
capacity regression) remain backlog in the same rows; neither parent closes.

The native context's digest check and UTF-8/JSON output decoding are extracted
into one shared helper without changing its behavior. Endpoint calls, results
and refused calls now use the same projection. Every payload keeps its schema,
media type and digest, with readable `content` replacing `bytes`. Absent result
outputs remain null. Original contract records and their hashes remain intact.
Non-UTF-8 data, digest mismatches and malformed `application/json` refuse with
Invalid and latch preparation closed. Writer capacity errors remain
ResourceExhausted. One observation is projected at a time into the existing
bounded streaming writer; no second complete history is built or truncated.
Raw payload size is also capped by remaining content capacity before decoding;
oversize refuses before digest/format checks or temporary JSON allocation.

The review's low L1 is also addressed: the overflow regression now directly
excludes ModelRequested, PermissionRequested and PermissionDecided events as
well as ModelCompleted, execution and accepted sockets. Earlier independent
low findings remain backlog in the same rows.

## Verification

The full unfiltered ordinary fast gate ran both affected crates and all 18
targets, with the final three Rust source hashes frozen throughout:

```text
python3 -B scripts/local_agent_fast_gate.py --base 92c919078ec131c6074e081d4c05137b59943834 --timeout 3600
```

Format and strict Clippy PASS. The source gate remains **FAIL** on the deferred
T-301.1 baseline: 716 passed, 53 unchanged failed, 11 unchanged ignored and zero
filtered. Every prior passing identity survives, along with identical failed/
ignored identities and normalized failure diagnostics from the predecessor's
complete gate. Exactly two new methods pass. Complete target selection follows
the ordinary gate's affected host/API packages, with no target omitted for its
result. The seven command durations were 0.10, 0.10, 0.10, 1.15, 0.85, 27.50
and 127.81 seconds, totaling 157.61 seconds including reservations. The gate
owns per-Cargo build reservations; two build jobs and four test threads were used.

The new methods cover corrupt digests, invalid UTF-8 under JSON and plain-text
media types, malformed JSON and raw size refusal across completed-call arguments,
result outputs and refused-call arguments. Every refusal latches closed and
leaves original serialized contracts unchanged. Plain UTF-8, quotes, non-ASCII
text, embedded instruction-like text and absent outputs retain their intended
user-data representation. The strengthened existing history test compares all
metadata fields independently of the projection helper and tests exact content/
encoded limits and one-byte-under refusals with actual observations. The socket
regression checks the exact source string delivered on turn two. The overflow
regression directly checks absence of model/permission request events.

An earlier filtered development run passed 14 context/owner methods, before
raw-size preflight cases were added to one new method. It is not final source
qualification. Only the complete frozen-source run above qualifies final source.
Original native context test bodies, endpoint transport, retained owner code/
tests and all ledger table rows are unchanged. Both retained reports match their
independent originals byte-for-byte. Final lightweight checks cover the completed
documents, both reports and public hygiene without repeating the Rust gate.
Raw logs, source copies/hashes, comparison audit and review helpers are private.

Final source-gate raw log SHA-256:
`6008cf93f0ef2a39bbe7c58f2f9558aa99ecaaa8a0bbb4839aededd05663ce60`.

## Limits

Sources, model responses, tools, permissions and completion verifiers in the
socket tests are synthetic. This is a repair to the existing library composition,
not a real-model measurement or a new HTTP workflow. Actual scoped filesystem/
grant integration, native edit/test/retry, durable recovery, approved Git and
parent measurements remain open. No evaluation, hidden or held-out content,
scorer, threshold, ignore, dependency, SBOM, status model, scoreboard, GPU,
model download, push or release is changed.
