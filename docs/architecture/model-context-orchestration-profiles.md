# Model Context and Orchestration Profiles

AgentMage compiles one model-specific context plan before Story 22.1 composes individual context
items. The plan is bound to the exact model manifest, runtime, tokenizer, and authoritative token
counter. Byte counts and heuristic token estimates may be useful diagnostics, but they are not
accepted by this admission boundary.

The total context window is partitioned into system/tool definitions, user input, source artifacts,
retrieved context, workflow/recovery reserve, output reserve, and a non-spendable safety margin.
Checked arithmetic rejects fixed overcommit. Authoritative source context is allocated first and
cannot fall below its declared useful minimum; an oversized source allocation is visibly truncated.
Retrieved context is then included, visibly summarized, or visibly omitted. Every non-complete
disposition carries a stable reason code, and all remaining tokens are recorded as unallocated.

The orchestration shape may change only presentation and planning shape: plan horizon, visible
descriptive tool subset, observation size, one proposal-parser repair allowance, recovery
scaffolding, and diagnostic verbosity. The compiled profile separately hash-binds policy, grants,
approvals, side-effect classification, retry eligibility, verifier, budgets, and completion
authority. Those eight controls must remain identical across candidates.

A profile needs current independent evidence for every declared role/workflow tuple. Evidence is
never inferred across a model family. Compilation never enables automatic fallback. Even complete
orchestration evidence leaves a candidate disabled unless its exact underlying model profile is
already enabled and has an approved or explicitly degraded lifecycle.

The closed machine-readable contract is
[`orchestration-profile.schema.json`](../../schemas/model/orchestration-profile.schema.json). A
context-plan or orchestration schema change changes its digest and requires new exact evidence;
there is no permissive migration or borrowed compatibility. Manual selection can choose only an
already enabled exact model/orchestration tuple and does not alter any invariant control.
