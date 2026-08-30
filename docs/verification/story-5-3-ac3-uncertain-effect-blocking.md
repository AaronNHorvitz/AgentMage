# Story 5.3 AC3 uncertain-effect blocking

Story acceptance criterion `5.3.AC3` passes for the current retry-identity, reconciliation, and
synchronized in-runtime execution-gate scope. Non-idempotent, destructive, external, and unknown
effects cannot open an automatic successor. An uncertain effect also remains blocked until exact
safe reconciliation and a separate current user approval are both present.

Every successor uses fresh call, tool-call, attempt, grant, approval, receipt, and verification
identities. Reuse of any prior identity or authority object fails before dispatch. A 16-contender
race admits one exact attempt and invokes one synthetic content-free callback; its uncertain result
remains uncertain, cannot become success, and cannot replay. Blocked, denied, failed, and uncertain
terminal outcomes remain distinct and provide a deterministic safe next action.

This closes the current pure retry and synchronized in-runtime gate criterion. It does not execute
a native tool, provider, model, or network effect. Cross-process crash durability,
installed-product and cross-platform evidence, independent review, Story, Sprint, packaging, and
release completion remain separate gates.
