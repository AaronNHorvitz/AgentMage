# Story 50.2 Component-Removal Results

## Scope

This campaign defines the local source evidence for independently removing the
interactive CLI, native Chat adapter, future workflow caller test adapter, and
optional local runtime projections. Each scenario keeps the caller-neutral
transport and reusable coordinator unchanged, runs the surviving Rust library
tests, applies strict Clippy, and inspects Rust dep-info rather than inferring
source absence from a successful build alone.

## Closed command

Run the campaign only from a clean committed source revision:

```text
python3 scripts/runtime_component_removal.py
```

Verify retained evidence without compiling or executing the Rust suites:

```text
python3 scripts/runtime_component_removal.py --verify
```

The profile at
`fixtures/runtime-hardening/v1/component-removal-profile.json` owns the exact
feature sets, removed and required source paths, offline no-shell Cargo argv,
minimum passing-test counts, exact ignored-test counts, and limitations.

## Required interpretation

A successful local campaign proves only that each named component is absent
from its isolated Rust compilation, the declared shared sources remain, the
remaining unit suite passes, and strict linting is clean on the recorded host.
It does not prove installed-package residue removal, process or socket absence,
persistent-data cleanup, credential lifecycle, supported-platform parity,
accessibility, independent review, or deferred manual fuzzing. Ignored tests
remain visible and are never converted into passes.

## Current disposition

`LOCAL-SOURCE-PASS` on the recorded Fedora x86-64 host for source commit
`33c619402260d0fc735b36134979ac1541c414fc`.

| Isolated removal | Remaining tests | Ignored | Compiled-source result |
|---|---:|---:|---|
| Interactive CLI | 110 passed | 3 | `cli.rs` and `cli_runtime.rs` absent; native Chat, workflow caller, Linux read shell, coding runtime, harness, and shared transport present |
| Native Chat adapter | 116 passed | 3 | `native_chat_runtime.rs` absent; CLI, workflow caller, Linux read shell, coding runtime, harness, and shared transport present |
| Future workflow caller | 113 passed | 3 | `workflow_caller.rs` and `workflow_assignment.rs` absent; CLI, native Chat, Linux read shell, coding runtime, harness, and shared transport present |
| Optional runtime projections | 584 passed | 2 | `runtime_projection.rs` absent; coordinator, loop, journal, artifacts, lifecycle, and recovery present |

All four strict Clippy commands pass with warnings denied. The 923 passing test
executions are scenario totals, not unique test identities; each scenario is a
separate reduced-feature build. Eleven ignored executions remain explicit.

The retained [`report.json`](../../artifacts/sprints/sprint-50/story-50.2-component-removal/report.json)
binds the exact source blobs, environment, fixed offline argv, test and Clippy
logs, and compiled-source manifests by SHA-256. The verifier and evidence
mutation tests reject source, command, result, log, manifest, count, or
limitation drift.

This closes only the local source-removal sub-task. Installed package and data
residue, process and socket cleanup, supported-platform execution,
accessibility, external review, and manual fuzzing remain outside this result.
