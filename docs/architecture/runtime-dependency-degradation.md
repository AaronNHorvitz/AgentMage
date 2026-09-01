# Runtime Dependency Degradation

The Rust kernel evaluates one closed dependency inventory before an affected workflow executes.
Every runtime, parser, model, codec, endpoint, tool, verifier, store, client feature, and optional
capability has an exact identity, version, policy envelope, and one of three workflow-local classes:
required, optional, or substitutable.

Required loss blocks execution. Optional loss produces a visible reduced-capability disposition and
may proceed only because the dependency was explicitly classified optional. Disabled and
quarantined states remain visible even when the affected optional capability can be omitted.
Substitutable loss is `unavailable` unless a current explicit replacement decision passes every
qualification check.

## Substitution boundary

A replacement is admitted only when all of the following are true:

- the original dependency was declared substitutable and binds the exact substitution-policy hash;
- the replacement is already present in the same closed inventory and belongs to the same family;
- its exact version and identity digest are currently available;
- the qualification generation is nonzero and equals the current observation;
- the decision is qualified, visible, and carries a stable reason code;
- its data and completion policies are identical, its authority classes are a subset, and its
  security and verification strengths are equal or greater.

This is an explicit fresh selection, not fallback. Health cannot select a route, an optional
dependency cannot silently become required, and a replacement cannot broaden authority or weaken
completion. Required or optional dependencies reject substitution records entirely.

## Closed results

Every registered dependency produces one ordered result: `ready`, `blocked`, `degraded`,
`unavailable`, `disabled`, or `quarantined`, plus a content-free reason and explicit execution
permission. Missing, extra, duplicate, unknown, malformed, or unaccounted observations fail the
evaluation rather than disappearing from the result.

The evaluator is interface-neutral. Chat, Verified Chat, CLI, headless, and later workflow callers
must present its result but cannot reinterpret it. An allowed optional degradation never grants a
tool, changes a route, relaxes verification, or asserts task completion.
