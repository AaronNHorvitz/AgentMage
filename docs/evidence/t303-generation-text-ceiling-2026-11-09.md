# T-303/T-306: strict generation text envelope, 2026-11-09

Status: builder implementation; independent core review required. T-303 and
T-306 remain open. No row or sub-row is added or closed.

## Change

Strict generation freezes a transport text ceiling equal to the smaller of
64 KiB and the sealed run output ceiling before the kernel phase checks it.
Small runs no longer need a 1 MiB text reservation merely to generate a strict
edit. The original prepared transport can consume itself to narrow its current
text ceiling; zero or an increase refuses before connecting. The original body,
context fingerprints, endpoint, credential owner, requested tokens, wire/body
limits and continuous deadlines are retained. Ordinary chat defaults stay intact.

The transport counts UTF-8 bytes across decoded fragments. It refuses a fragment
that would exceed the frozen ceiling before forwarding it and closes that
exchange without a completed result. Earlier text remains provisional. The
original kernel phase independently checks its own stream count and descriptor.
The strict answer collector retains its additional bound. A rejected exchange
keeps its admitted attempt and input observations, with completed counts absent.
Native reads, full preimages, workspace binding, exact approval and the original
atomic writer retain their previous boundaries.

This addresses strict-generation review low L2 and strengthens low L1's control
assertions. The observed callback refusal is exactly the transport's typed
`Dependency(OperationStopped(Cancelled))`, rather than the review's suggested
phase-level `Stopped(Cancelled)`: the transport checks the same control after
the callback and its error returns before the phase's later check. The test
preserves that original failure identity. Preview refusal is exactly `Incomplete`
after a cancelled exchange and `Preparation(Cancelled)` after completed EOF
followed by cancellation. No cancellation behavior is changed to fit an assertion.

## Verification

Focused development checks pass: six strict-generation methods (680 filtered)
and the complete transport integration target (31 passed, none filtered).
Including build reservations, these commands take 12.87 and 4.19 seconds.
They are development checks, separate from the ordinary unfiltered scoped gate.

Two new transport methods cover borrowed/shared ownership, inert zero/increase
refusal, equal and minimum ceilings, unchanged body and descriptor fields,
UTF-8 byte counts across fragments, exact-limit completion and overflow refusal
before the callback, with optional usage both enabled and disabled. A new host
method exercises strict generation and original native previews at run ceilings
of 4 KiB, 64 KiB and 2 MiB, in both ownership forms, at and above each effective
text limit. Overflow preserves the prior preview and source bytes. Exact-limit
answers prepare a native diff without writing. Existing protected-approval,
control, scope, stale-source and malformed-answer cases remain checked.

The ordinary full scoped fast gate exits **1 / FAIL**: 29 targets, 1,070
passed, exactly 53 inherited host failures, 19 existing ignored and zero
measured or filtered. Every failing identity matches the unchanged T-301 host
baseline, with none new or missing. All three added methods, the exact control
assertions and the existing protected native HTTP edit method pass. The data
comparison is not a passing gate or an independent acceptance verdict.

The gate uses immutable predecessor
`37207e27bb0dfd5b68cf18f80f9e50570742453d` as its base:

```sh
python3 -B scripts/local_agent_fast_gate.py --base 37207e27bb0dfd5b68cf18f80f9e50570742453d --timeout 600
```

It selects host, local API and Linux inference formatting, all-target Clippy
and all-target tests without filters. Every Cargo invocation reserves its own
shared build slot; two build jobs and four test threads are retained.
Formatting exits zero in 13.08 seconds, Clippy zero in 8.82 seconds, and tests
101 in 134.62 seconds. Total elapsed time including reservations is 157.06
seconds. The four Rust input hashes are unchanged throughout. Final
documentation checks run separately after recording these results; the final
eight-path plan adds only the ledger, this evidence and the retained predecessor
review to the frozen five-path source plan, with identical Cargo commands. No
Rust changes follow the gate.

Raw full-gate log SHA-256:
`e04d186d71891371e06c2f28f2450373e2838fde8534479a53a7ff93877491a0`.
The owned full-run temporary namespace retains 22 entries, including four
inherited host-test files totaling 36 bytes. New fixture roots are absent after
their cleanup. The focused namespace is empty and removed. No unrelated
namespace, worker or resource ceiling is changed. The unchanged Linux unit
package and its nineteen inherited failures are not selected or rerun.

## Scope

Endpoint answers and worktree-creation provenance are synthetic; the native
reader, strict parser and preview actually execute. This is not a real-model
coding measurement, native Git creation, test-worker success or task score.
There is no new executor, permission, HTTP route, retry or durable recovery.
Cumulative generation/coordinator accounting, general edit/test/retry, positive
native command-factory admission, approved Git and parent measurements remain
open. Earlier schema-ID and command-preflight lows remain same-row backlog.
No hidden/held-out test, scorer, task pin, threshold, dependency, SBOM, status
model or scoreboard changes. The long acceptance gate and GPU inference do not
run in this batch. The preceding workspace-label repair's immutable commit is
preserved.

## Predecessor verdict received after implementation commit

The [independent predecessor review](../reviews/37207e27bb0d-local-coding-preview-workspace.txt)
has **Verdict: PASS** for exact commit
`37207e27bb0dfd5b68cf18f80f9e50570742453d`, no high/medium finding and no parent
closure. Its bytes are retained unchanged. Its low L1 documentation finding is
addressed in the guide: all created worktree owners currently share the same
workspace label, so this metadata check cannot distinguish two created owners.
The trusted caller must still supply the original live owner; held-root checks,
complete creation-receipt binding and exact approvals remain authoritative.
Per-owner binding before reads remains future work, not a claim of this repair.

This verdict arrived after implementation commit
`a5923bebfa9fa1b60ecc2cac4ff2613d2c811803`. Retaining it and clarifying the
shared label changes documentation only, on top of that commit. The final plan
retains the exact same Cargo commands and four Rust hashes from the full run.
Documentation, review-text, whitespace and public-hygiene checks are repeated.
The new text-ceiling repair still requires its own independent core review.
