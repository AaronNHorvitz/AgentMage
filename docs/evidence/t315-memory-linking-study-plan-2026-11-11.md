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
The fast-gate CLI initially reports four requested files but zero linted files
in its summary; the direct library invocation explicitly returns those four
file names with zero issues. Final checks including this evidence cover all
five documents, with zero issues in the direct check. No product tests,
model qualification or task-pass result
is claimed by these documentation checks. Independent review is still required;
this planning batch does not close T-315, T-303 or T-306.
