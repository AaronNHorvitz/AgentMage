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

The retained
[`report.json`](../../artifacts/sprints/sprint-50/story-50.2-runtime-load/report.json)
is bound to source commit `268ea924efb88c11eef24b0eee03cce9febb4548`. All
eight commands and 39 focused tests passed without a failed, ignored, or
measured test result.

| Measurement | Recorded result | Declared ceiling or floor |
|---|---:|---:|
| Canonical events | 8,196 | exactly 8,196 |
| Journal elapsed time | 2,356 ms | at most 30,000 ms |
| Journal throughput | 3,478 events/s | at least 250 events/s |
| Publisher elapsed time | 254 ms | at most 10,000 ms |
| Publisher throughput | 32,267 events/s | at least 1,000 events/s |
| Maximum queued events | 127 | at most 1,024 |
| Maximum queued bytes | 90,932 | at most 4,194,304 |
| Resident memory | 45,952 KiB | at most 1,048,576 KiB |
| Retained encrypted-store bytes | 10,813,336 | at most 134,217,728 |
| Verified reopen cycles | 16 | exactly 16 |
| Reopen elapsed time | 13,867 ms | at most 60,000 ms |

The run used Fedora Linux `7.1.6-201.fc44.x86_64` on an x86-64 Intel Core
i9-13900KF host with 32 logical CPUs and 65,570,268 KiB of reported memory.
The report disposition is deliberately `PARTIAL-PASS`: its measured campaign
passed, while its declared architecture and installed-interface limitations
remain open.

This campaign cannot close Sub-task 50.2.3.3 by itself. The current journal
writer has bounded deferred batches, but SQLite flushes still execute on the
coordinator thread. A dedicated writer path and injected slow-disk evidence are
still required before AgentMage can claim model-stream continuity and responsive
cancellation while persistence is blocked. Installed native Chat and
authenticated CLI measurements also remain open.
