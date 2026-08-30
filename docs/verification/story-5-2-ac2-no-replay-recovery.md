# Story 5.2 AC2 no-replay recovery

Story acceptance criterion `5.2.AC2` passes for the current pure retry-admission and synchronized
in-runtime execution-gate scope. A criterion-specific product regression attempts to reuse every
prior identity family: operation attempt, call, tool call, grant, approval, receipt, and idempotency
key. Each case is denied before admission. No old call or authority object becomes a successor.

Automatic recovery remains unavailable for non-idempotent, destructive, external, and unknown
effects. An uncertain recovery decision is not a retry decision. Conditional recovery admits only
current evidence proving a fresh attempt safe; both an uncertain prior effect and an already-present
desired state block another attempt. The retained 16-racer product test invokes one synthetic,
content-free callback exactly once, keeps its uncertain outcome sticky, and denies the replay.

This closes the current no-replay recovery criterion without executing a native tool, provider, or
model. Cross-process crash durability, cross-platform installed-product evidence, independent
review, Story, Sprint, packaging, and release completion remain separate gates.
