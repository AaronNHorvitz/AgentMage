# Story 5.2 AC1 deterministic retry disposition

Story acceptance criterion `5.2.AC1` passes for the current pure contract, registry, and
admission-policy scope. One product regression traverses all 22 registered operations and all 14
closed workflow failure classes: every one of the 308 pairs produces the same operation-derived
effect class, failure-derived conservative disposition, and closed retry-class set on repeated
evaluation.

Effect and failure classifications are exhaustive Rust enums. Custom, wildcard, inherited,
model-created, unknown, and empty wire values fail deserialization, while the tool registry derives
the effect class from the exact registered operation rather than caller or model output. The
retained 79-mutation admission campaign includes every effect/retry and failure/uncertainty field
and records zero dispatches after mutation.

This closes the current deterministic policy criterion without executing a model or tool effect.
Native provider execution, cross-platform installed-product evidence, independent review, Story,
Sprint, packaging, and release completion remain separate gates.
