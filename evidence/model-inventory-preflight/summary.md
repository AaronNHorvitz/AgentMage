# Reference-Machine Model Inventory Preflight Summary

## Scope

Sprint 14 Sub-task 14.2.2.2 evidence for the non-acquiring architecture,
runtime, format, acceleration, disk, memory, context, modality, and
expected-working-set preflight against each declared reference-machine envelope.

## Frozen Inputs

- `model-profiles/catalogs/2026-08-14/candidate-inventory.json`
- `model-profiles/catalogs/2026-08-14/candidate-role-suite-matrix.json`
- `evidence/model-inventory-preflight/reference-machine-envelopes.json`

## Declared Reference Machines

- `fedora-kinoite-44-x86_64`
- `ubuntu-24.04-x86_64`
- `windows-11-x64`
- `macbook-pro-m5-arm64`

## Guaranteed Invariants

- No acquisition, runtime start, or network egress occurs during preflight.
- No preflight decision is borrowed from another exact profile, sibling, or
  family relative.
- Every source `INELIGIBLE` entry remains `INELIGIBLE` per envelope.
- Every source `BLOCKED` entry receives a per-envelope decision of `BLOCKED` or
  `BLOCKED-HARDWARE`; a specialist, safety, embedding, translation, medical,
  research, interpretability, or legacy profile never silently becomes a
  `coding_planner`.
- Every envelope publishes exact architecture, operating system, accelerator,
  supported runtimes, supported artifact formats, memory, storage, context
  ceiling, modality coverage, and expected working set.

## Regeneration

The full per-envelope preflight report is deterministic from the frozen inputs
and is regenerated with `python3 scripts/model_inventory_preflight.py --write`.
The runbook is `docs/evaluation/model-inventory-preflight.md`.
