# Story 5.3 AC1 runtime-owned transitions

Story acceptance criterion `5.3.AC1` passes for the current deterministic workflow-contract and
in-runtime admission scope. The closed 18-state workflow table permits only declared legal edges,
requires immutable workflow identity and monotonic sequence, and makes all eight terminal states
absorbing.

An admitted step binds its graph position, preflights, effect and retry classes, approval policy,
idempotency rule, verifier set, and every execution budget to integrity-checked policy. A fresh
attempt is admitted only through the synchronized runtime boundary after current preflight, exact
approval, single-use grant, fresh identities, and remaining budget all agree. Parser repair, model
repair, step attempt, replan, repeated-state, and total-work limits remain independent and fail
without partial charge. The retained adversarial campaign records zero dispatches for stale
preflight, approval bypass, and grant-reuse candidates.

This closes the current pure contract and synchronized in-runtime admission criterion with
synthetic data. It does not execute a native tool, provider, model, or network effect. Durable
cross-process workflow execution, installed-product and cross-platform evidence, independent
review, Story, Sprint, packaging, and release completion remain separate gates.
