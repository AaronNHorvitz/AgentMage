# T-303 / T-306: endpoint owner fault handling

Date: 2026-11-08. Tier: core. Immutable predecessor:
`cdb63cad1c5cd6f3924f72c65c64377d69c1d8e4`.
Both parents remain OPEN. No row or sub-row is added or closed. Independent
core review of this fault-handling increment is required.

## Change and authority

The retained owner's exact-commit independent PASS at
`92c919078ec131c6074e081d4c05137b59943834` listed three low findings.
This increment addresses them in the same rows before service integration.
The original report is unchanged and accepts no subsequent change.

For L1, the owner marks a run faulted before entering the original coordinator.
A normal return clears the mark only on success or an error for which the
kernel's original reconciliation latch remains unset. A dependency panic leaves
the mark set even if the host catches the unwind and keeps using the owner.
Events remain inspectable; release drops the original ports/context while
retaining the spent identity. Retrying that run enters no dependency. No panic
is caught by production code and no synthetic terminal or recovery is emitted.
The policy deliberately also faults a run when a dependency panics before an
effect: absence of an effect receipt cannot prove safe reuse after an unwind.

For L2, missing or mismatched endpoint envelopes after the shared coordinator
has returned a terminal outcome now set its original reconciliation latch.
The owner therefore retains a faulted, releasable entry on that error. Restoring
the envelope cannot undo the latch. Native outcome handling and ordinary
rejected approval input are unchanged.

For L3, the existing stateful-error test now fills the other three slots while
retaining the faulted entry, observes refusal of a fifth admission, releases
the faulted run and successfully admits that fifth request. Its original run
identity stays spent. The retained event prefix and execution count stay fixed.

## Verification

Two new test methods exercise real coordinator boundaries with synthetic
dependencies. The owner socket test catches a clock panic during approval
validation, before tool execution, and a verifier panic after one approved
snapshot read and the second model response. It disarms the clock, retries
with and without approval and observes owner refusal, unchanged event prefixes,
unchanged dependency counters and no further socket connection. Release succeeds.
The kernel test injects missing and mismatched envelopes after an actual terminal
publication, observes the reconciliation latch, restores the original envelope
and verifies continued refusal with unchanged events and dispatch counters.
The existing rejected-approval, repeated-terminal and lifetime-capacity tests
remain controls. No hidden task, scorer, threshold or ignore is changed.

The complete unfiltered ordinary gate ran all four affected crates and all
43 targets:

```text
python3 -B scripts/local_agent_fast_gate.py --base cdb63cad1c5cd6f3924f72c65c64377d69c1d8e4 --timeout 3600
```

Format and strict Clippy PASS. The source gate remains **FAIL** on deferred
T-301.1: 2,605 passed, 72 unchanged failed, 71 unchanged ignored, zero filtered.
Every prior passing identity survives; the added set is exactly the two new
methods. Failed and ignored identities and normalized failure diagnostics match
the retained baselines. The audit uses all 43 targets from the complete owner
run, replacing all 18 host/API targets with the subsequent context-repair run;
that intervening commit span changes no kernel/platform Rust source. No target
is excluded for its outcome. All four changed Rust source hashes stayed fixed.
The six command durations, including shared reservations, were 0.10, 0.10, 1.35,
105.42, 22.54 and 315.20 seconds, totaling 444.71 seconds. Two build jobs and four
test threads were used through the gate's own per-Cargo reservations.

The earlier focused owner run passed all seven selected methods. It is only a
development check; the complete gate above qualifies the final source. No source
compilation or test failure occurred during development. The old fault test
gains capacity assertions without losing any original assertion. Context,
transport and original native test bodies and all ledger table rows are
unchanged. Final documentation/report and public-hygiene checks run separately
without repeating the Rust gate. Raw logs, exact inputs and review helpers
are retained privately.

Final source-gate raw log SHA-256:
`dc001a6418068d0718e810a714a01f31b32cd9c3241e8abef82e238b0c2203a9`.

The predecessor's [independent context re-review](../reviews/cdb63cad1c5c-local-coding-readable-context.txt)
arrived as the source gate finished: Verdict: PASS for exact commit
`cdb63cad1c5cd6f3924f72c65c64377d69c1d8e4`, no high/medium finding.
It resolves the prior context M1 and accepts its earlier L1 assertions. New
low L1 (explicit null output-key assertion) and L2 (remaining-capacity boundary
test) remain backlog in these same rows. The report is retained byte-identically;
its PASS accepts only the context repair, not these subsequent owner changes.

## Open work

This remains synchronous in-process ownership, without authenticated HTTP
composition, service-issued run identity allocation, actual scoped filesystem
and grants, background scheduling or durable recovery. Controlled edit/test/
retry, approved Git and parent measurements remain open. Socket models, sources,
tools, permissions and verifiers here are synthetic; this is no real-model score
or native filesystem acceptance. No GPU/model/evaluation/held-out/hidden/prompt/dependency/SBOM/status/
scoreboard/push/release change is made.
