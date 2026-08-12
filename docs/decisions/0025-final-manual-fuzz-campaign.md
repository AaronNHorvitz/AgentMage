# Decision 0025: Final Manual Fuzz Campaign

| Field | Value |
|---|---|
| Status | Accepted sequencing decision |
| Date | 2026-08-12 |
| Scope | Real fuzz-engine execution for product parsers, IPC decoders, path boundaries, model-output decoders, and reviewed FFI boundaries |
| Defers | Execution of the open `RM-024` real-fuzz campaign until the final pre-release verification stage |
| Does not defer | Unit, property, integration, adversarial, malformed-input, resource, cancellation, cross-target, or native-platform tests |
| Blocks | Every affected `RV-15` result, Sprint 166, and final `G-GA` until the campaign and required reruns pass |
| Preserves | Decisions 0001 through 0024, all fuzz requirements and corpora, truthful evidence states, and the no-waiver release rule |

## Context

Decision 0019 separated real product-boundary fuzzing from synthetic corpus and
pipeline evidence because the real engine requires a deliberate, manually
supervised security-validation session. Numbered development has resumed, but
the user needs to reserve a substantial uninterrupted period for that campaign.

Running isolated fuzz targets while their parsers, IPC frames, platform
adapters, and FFI surfaces are still changing would also make much of the
result stale before release. This is a sequencing decision only. It cannot turn
missing fuzz evidence into a pass or reduce any acceptance threshold.

## Decision

1. Real fuzz-engine execution remains required and open under `RM-024`.
2. The complete real-fuzz campaign runs after first-GA implementation surfaces
   are frozen and before Sprint 166 can close.
3. Development continues with all non-fuzz verification required by each
   bounded change, including deterministic malformed-input suites, property
   tests, adversarial tests, resource limits, cancellation, cross-target
   compilation, and genuine native-platform execution.
4. An earlier sprint that owns an `RV-15` case remains incomplete when its real
   fuzz evidence is deferred. Synthetic, seeded, replay, or property-test output
   must not be labeled as real fuzz-engine, sanitizer, or coverage evidence.
5. The final campaign binds exact source, toolchain, harness, target, corpus,
   seed, duration, sanitizer configuration, coverage output, crash artifacts,
   minimization results, environment identity, and reviewer disposition.
6. Every confirmed defect is fixed through its owning boundary, receives a
   regression test, and triggers impact-based reruns. Material fixes reopen
   affected evidence and may require another full fuzz pass.
7. Final `G-GA` remains blocked until every promoted fuzz target passes its
   declared campaign, all crashes and hangs are dispositioned, and an
   independent reviewer reconciles raw output with the release summary.

## Verification

- Documentation validation must keep `RM-024` and affected `RV-15` work open.
- The final Sprint 166 plan must contain an explicit real-fuzz campaign task.
- Release-status computation must reject missing, stale, synthetic-only,
  unreviewed, or unresolved fuzz evidence.
- No ordinary product check may claim to have run the deferred engine.

## Consequences

- Product implementation can continue without repeatedly triggering the manual
  security campaign during active boundary development.
- The end-of-development verification period becomes longer and may uncover
  changes that reopen earlier evidence.
- This decision changes execution timing, not scope, severity, coverage, or the
  release threshold.
