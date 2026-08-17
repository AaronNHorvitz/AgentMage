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
python3 scripts/runtime_hardening_load.py \
  --output artifacts/sprints/sprint-50/story-50.2-runtime-load-worker
```

The generator requires a clean source tree, invokes only its closed no-shell
Cargo command list, records process output and resource use, rejects an occupied
evidence destination, and binds its report to the exact Git commit and source
digests. The command above is the historical generation command; its immutable
destination is now occupied by the retained result and is intentionally not
overwritten by a rerun.

## Prior Retained Boundary

The currently retained historical
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
The report disposition is deliberately `PARTIAL-PASS`. It predates the
dedicated journal worker and remains a historical lower-level writer baseline,
not current worker-backed evidence.

## Current Worker-Backed Boundary

The retained
[`report.json`](../../artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json)
is bound to source commit `f8d521c4dc4bb9b2053447a61aaac04028b4906b`.
All nine commands and 42 focused tests passed without a failed, ignored, or
measured test result. The main workload runs through `RuntimeJournalWorker`, and
the dedicated isolation group covers slow-store progress admission, independent
client publication, explicit producer saturation, exact retry, sticky storage
failure, normal shutdown, and verified reopen.

| Measurement | Recorded result | Declared ceiling or floor |
|---|---:|---:|
| Canonical events | 8,196 | exactly 8,196 |
| Journal elapsed time | 2,304 ms | at most 30,000 ms |
| Journal throughput | 3,557 events/s | at least 250 events/s |
| Publisher elapsed time | 258 ms | at most 10,000 ms |
| Publisher throughput | 31,767 events/s | at least 1,000 events/s |
| Queue saturation recoveries | 7 | each rejected event is flushed and retried once |
| Maximum queued events | 1,024 | at most 1,024 |
| Maximum queued bytes | 733,184 | at most 4,194,304 |
| Resident memory | 53,996 KiB | at most 1,048,576 KiB |
| Retained encrypted-store bytes | 10,813,336 | at most 134,217,728 |
| Verified reopen cycles | 16 | exactly 16 |
| Reopen elapsed time | 13,935 ms | at most 60,000 ms |

The current run used Fedora Linux `7.1.6-201.fc44.x86_64` on an x86-64 Intel
Core i9-13900KF host with 32 logical CPUs and 65,570,268 KiB of reported memory.
Its disposition remains deliberately `PARTIAL-PASS`. Holding the sole SQLCipher
connection lock is a deterministic slow-store surrogate, not a real filesystem,
device, power-loss, or installed-client campaign. Integrated model-stream and
cancellation timing during a real disk stall, installed native Chat and
authenticated CLI measurements, supported-platform evidence, and independent
review remain open. Sub-task 50.2.3.3 is not closed by this source-level result.
