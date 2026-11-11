# T-315 memory-linking study: planning record — 2026-11-11

This records the owner's later study request, not an implemented memory
algorithm or a completed row. [Decision 0204](../decisions/0204-open-study-memory-linking.md)
states the open question and the three requirements. The
[roadmap](../../CAPABILITY-ROADMAP.md#open-study-how-agent-memories-are-linked)
links the study from CAP-29 and integration rule 5; the
[PRD](../../PRD.md#12-canonical-data-and-storage) records it beside memory's
storage authority. [T-315](../../TASKS.md) remains unchecked and starts no
earlier than the first CAP-29 work.

The requirements cover versioned, replaceable linking with per-link provenance,
coexisting methods and set-wide disabling without rewriting data; replaceable
source-backed memory writing and context selection with method identities in
run evidence; and fixed-task comparisons of needed facts reached, wrong facts
retrieved, cost and task outcomes before adoption. Existing thresholds and
held-out restrictions remain unchanged. No method is selected or implemented.
No runtime source, prompt, permission, evaluation task or hidden test changes.
The coding-agent and measure-first priorities remain in force.

Validation uses the documentation fast gate from the preceding measurement
commit, plus direct Markdown lint on every changed document. Both pass.
The fast-gate CLI lists the requested files under “Linting”; its summary
“0 issues in 0 files” counts files with issues. The direct library invocation
also returns the four initial file names with zero issues. Final checks including
this evidence cover all five documents, with zero issues in the direct check.
No product tests, model qualification or task-pass result is claimed by these
documentation checks. The retained [independent planning review](../reviews/a72a492830b6-memory-linking-study-plan.txt)
returns PASS for exact commit `a72a492830b627dc3ae85fa4bacb01d020a0277c`.
Its three low wording findings are corrected: set-wide disabling rewrites no
stored fact or link, graph retrieval's benefit remains conditional on measurement,
and the lint summary is described accurately. This planning batch does not
close T-315, T-303 or T-306.
