# Story 50.2 Runtime Load Results

## Scope

This campaign measures the source-level shared runtime on one recorded Fedora
Linux host. It combines an explicit reference-hardware event and journal load
with focused resource, output, artifact, cancellation, client, and recovery
tests. It does not convert source-level evidence into installed-platform or
release evidence.

## Fixed Workload

- 8,192 deferred progress events inside one 8,196-event terminal chain.
- One fast subscriber and one intentionally lagging single-event subscriber.
- Bounded encrypted journal batches followed by exact full-history replay.
- Sixteen encrypted-store close, reopen, and complete-history verification cycles.
- Focused runtime resource, event pressure, artifact-backed output, artifact
  lifecycle, cancellation, client cancellation, and recovery test groups.

The machine-readable profile is
[`linux-reference-load-profile.json`](../../fixtures/runtime-hardening/v1/linux-reference-load-profile.json).
The ignored Rust reference test runs only through the evidence command so
ordinary unit tests do not inherit hardware-specific timing thresholds.

## Command

```text
python3 scripts/runtime_hardening_load.py
```

The generator requires a clean source tree, invokes only its closed no-shell
Cargo command list, records process output and resource use, rejects an occupied
evidence destination, and binds its report to the exact Git commit and source
digests.

## Current Boundary

This campaign cannot close Sub-task 50.2.3.3 by itself. The current journal
writer has bounded deferred batches, but SQLite flushes still execute on the
coordinator thread. A dedicated writer path and injected slow-disk evidence are
still required before AgentMage can claim model-stream continuity and responsive
cancellation while persistence is blocked. Installed native Chat and
authenticated CLI measurements also remain open.
