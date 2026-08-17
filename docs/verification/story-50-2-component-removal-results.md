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

The campaign definition and mutation tests are present. Retained execution
results are added only after this definition is committed and the campaign is
run from that clean source revision.
