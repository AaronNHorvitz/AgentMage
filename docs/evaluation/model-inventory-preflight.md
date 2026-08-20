# Reference-Machine Model Inventory Preflight

## Scope

This runbook drives Sprint 14 Sub-task 14.2.2.2. It runs a non-acquiring
preflight for every entry in the frozen candidate inventory against every
declared reference-machine envelope. It never downloads bytes, starts a runtime,
opens network egress, activates a candidate, or promotes a family, sibling, or
lineage result across profiles.

## Inputs

- `model-profiles/catalogs/2026-08-14/candidate-inventory.json` - the frozen
  first-party candidate inventory produced by
  `scripts/model_candidate_inventory.py`.
- `model-profiles/catalogs/2026-08-14/candidate-role-suite-matrix.json` - the
  bound role and suite matrix that reconciles one-to-one with the inventory.
- `evidence/model-inventory-preflight/reference-machine-envelopes.json` - the
  declared architecture, operating system, accelerator, runtime, artifact
  format, memory, storage, context, modality, and expected-working-set ceilings
  for each reference machine.

## Reference Machines

- `fedora-kinoite-44-x86_64` - first-GA primary development and evaluation host.
- `ubuntu-24.04-x86_64` - first-GA secondary Linux host.
- `windows-11-x64` - first-GA Windows host.
- `macbook-pro-m5-arm64` - retained post-GA Apple Silicon host.

Each envelope publishes exact CPU architecture, operating system, accelerator
class and device memory, supported artifact formats, supported runtime builds,
memory, storage, context-token ceiling, modality coverage, and expected working
set. A candidate cannot silently exceed a listed ceiling or borrow another
envelope's fit.

## Preflight Axes

For every `(candidate, envelope)` pair the script evaluates the following axes:

- `architecture` - envelope CPU architecture against artifact-neutral source
  entries.
- `runtime` - envelope runtime coverage for the candidate's advertised artifact
  formats.
- `format` - overlap between the envelope's supported artifact formats and the
  candidate's advertised formats.
- `acceleration` - envelope accelerator class and device memory; measured fit
  requires an exact profile so the axis is recorded as `UNRESOLVED`.
- `disk` - envelope storage ceiling against the exact artifact byte size, which
  is not selected at the source-entry layer.
- `memory` - envelope memory ceiling against the exact resident memory of the
  admitted quantization and runtime, which is not measured at this layer.
- `context` - envelope context-token ceiling against the candidate's advertised
  context, which is not declared at the source-entry layer.
- `modality` - candidate modalities against the envelope's declared modality
  support.
- `expected_working_set` - envelope working-set ceiling against the measured
  runtime working set, which requires an admitted exact profile.

An axis records `PASS`, `BLOCKED-HARDWARE`, or `UNRESOLVED`. A `BLOCKED-HARDWARE`
axis produces a `BLOCKED-HARDWARE` entry status. Otherwise every non-`INELIGIBLE`
source entry produces a `BLOCKED` entry status with reasons naming the missing
exact artifact bytes and unmeasured working set. `INELIGIBLE` source entries
remain `INELIGIBLE` per envelope and cannot be promoted.

## Commands

Rebuild and validate the preflight report against the frozen inputs:

```bash
python3 scripts/model_inventory_preflight.py --write
```

Verify the current report without rewriting it:

```bash
python3 scripts/model_inventory_preflight.py
```

Both commands print a summary line and exit non-zero on any reconciliation,
duplicate-identity, order, borrowing, activation, acquisition, or role
escalation failure.

## Evidence

- `evidence/model-inventory-preflight/reference-machine-envelopes.json` -
  declared envelope ceilings.
- `evidence/model-inventory-preflight/preflight-report.json` - reproducible
  per-envelope preflight decisions built from the frozen inputs on demand.
- `tests/test_model_inventory_preflight.py` - reconciliation, non-acquisition,
  role isolation, and borrowing tests.

The report is deterministic from the frozen inputs. A stale or absent report is
rebuilt by re-running the preflight command; a report that fails validation
against the current frozen inputs must be regenerated rather than edited.
