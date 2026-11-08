# T-303 / T-306: retained configured endpoint run ownership

Date: 2026-11-08. Tier: core. Immutable predecessor:
`aa1705bf758ae91e65f8421ea74802f1c7e6b4f5`.
Both parents remain OPEN. No row or sub-row is added or closed. Independent
core review is required; the predecessor is under separate review.

## Change and authority

`CodingEndpointRuns` owns complete original configured-endpoint coordinators,
including selected source context, transport/approval borrows, permission/tool
and verifier ports, clock, events and cumulative resource accounting. Admission
is inert. Only the exact sealed request identifies a retained instance. Lookup
never composes a replacement. The non-cloneable owner exposes no live coordinator
extraction or port replacement. Keep that same owner in the hosting service.

Up to four entries hold coordinators; completed and faulted entries retain their
slots until release. Release refuses a running/approval-waiting instance and
keeps the spent identity/digest after dropping the original ports and context.
At most 256 identities may be admitted in one owner's lifetime, with no eviction.
Invalid construction before admission has no effects and can be corrected.

A read-only accessor exposes the kernel's original reconciliation latch without
changing it. The owner preserves rejected approval input and its original
challenge. An error after stateful advancement latches the entry closed before
returning, retaining the event prefix for inspection. Further advances cannot
dispatch. A prefix without a terminal is never declared complete or recoverable.
Export needed events before release; storage and secure erasure are not claimed.

The configured transport predecessor's independent core re-review is retained
byte-for-byte. It has Verdict: PASS for exact commit
`f1ac4d326c42b9614c0bc148e2bbcf2af0ba1d3a`, no high/medium finding,
and resolves predecessor M1. Low L1/L2/L3 remain backlog in these same rows.
That verdict accepts only its non-closing increment, not this owner or a parent.

## Verification

Five new owner methods pass, including two actual socket turns with source
snapshots whose caller buffers have been dropped, unchanged repeated approval
and terminal reads, rejected approval followed by a valid approval, no tool
before consent, exact endpoint answer provenance, release and spent identity
refusal, changed/unsealed/foreign request refusal without mutation, stateful
verifier fault with no further dispatch, all four held slots, all 256 lifetime
identities, inert failed admission and original cancellation before dispatch.
The tests use synthetic source, permission/tool and verifier fixtures.

The complete unfiltered gate ran all four affected crates and all 43 targets:

```text
python3 -B scripts/local_agent_fast_gate.py --base aa1705bf758ae91e65f8421ea74802f1c7e6b4f5 --timeout 3600
```

Format and strict Clippy PASS. The source gate remains **FAIL** on the deferred
T-301.1 baseline: 2,601 passed, 72 unchanged failed, 71 unchanged ignored,
zero filtered. Every prior passing identity is retained, exactly five new owner
methods pass, and exact failed/ignored identities and normalized failure
diagnostics match retained predecessor logs. Cargo metadata selects every target
of the four affected packages; inference-only targets are outside this source
scope. No target is omitted for its result. The five Rust source hashes stayed
fixed throughout. The seven command durations were
0.10, 0.10, 0.10, 1.35, 2.15, 71.80, 441.99 seconds, totaling
517.59 seconds including reservations. The gate used its own per-Cargo
shared build reservations, two build jobs and four test threads.

The first focused development compilation failed on missing generic bounds and
test imports. Those were corrected without changing existing tests. A subsequent
focused run passed all five new methods and one existing owner-stop control.
That filtered development run is not qualification; only the complete source
gate above qualifies the final Rust inputs. No original test body, validator,
threshold or ignore changed. The final audit also checks unchanged original
transport/context implementations, the getter-only kernel diff, unchanged
ledger table rows and the byte-identical independent report. Documentation and
public hygiene receive final lightweight checks without another Rust gate.
Raw logs, exact source copies and review helpers are retained privately.

Final source-gate raw log SHA-256:
`3b0951492a485e20e4f22a1f75f5d7d2003db04bda43fcf4e8d990d8fdc6ae3c`.

## Open work

This is synchronous in-process ownership within one retained service owner.
It is not yet integrated with authenticated HTTP, background worker scheduling,
host-wide fresh identity allocation or endpoint CLI/MCP projections. A new
owner cannot recover a destroyed run or restore its clock, budget or grants.
Actual scoped filesystem/grant composition, controlled edit/test/retry, durable
recovery, approved Git and parent measurements remain open. Socket models,
source snapshots, permission/tool owners and verifiers in the tests are synthetic.
They establish no real-model score or live native filesystem authority.

No GPU, real-model, evaluation, hidden/held-out, prompt, scorer, threshold,
ignore, dependency, SBOM, status-model, scoreboard, push or release change.
