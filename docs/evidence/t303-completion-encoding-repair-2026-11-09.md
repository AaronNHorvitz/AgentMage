# Completion-only positional tool refusal

T-303/T-306 remain open. This fixes medium M1-R from the independent core
[completion-turn review](../reviews/1828e7e4eeaa-local-coding-completion-turn.txt)
for exact commit `1828e7e4eeaa264087c90189ff175f8620c18f82`, on top of the
immutable validation-history commit. No row/sub-row is added or closed;
independent core re-review is required.

## Behavior

The prior completion-only guard recognized a top-level `tool_call` member,
but the existing derived tool parser also accepted a positional JSON array.
Such a response could reach permission admission in the kernel and fault the
host completion boundary without a recorded terminal refusal. The independent
review reproduced both cases. No unapproved native effect was observed.

The guard now checks the same tool-proposal parser as the ordinary fallback,
as well as the previous explicit member check. It records
`runtime.proposal.completion-only` for positional tool proposals too. A second
restriction prevents completion-only turns from using the fallback admission
path at all. The attached interpreter remains disabled in either attachment
order. Valid completion candidates still face the original independent
verifier; no verification condition is changed.

The completion prompt also omits the stale tool-schema lookup text (review
low L2). The ordinary-turn positional grammar is unchanged; low L1 remains
backlog on the same rows. Permissions, budgets, native execution, approval,
confinement and credential ownership are unchanged.

## Verification

The complete scoped fast gate remains **FAIL**: 2,833 passed, exactly 72
inherited failures and none new, 71 existing ignored, zero measured or filtered
across 46 unfiltered targets. The failure identities match the unchanged T-301
baseline exactly. Formatting and Clippy pass. All three affected completion
methods pass; no test or verification condition is weakened.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 540.82 |
| Clippy, all affected targets | 0 | 20.39 |
| Tests, all affected targets, no fail-fast | 101 | 400.00 |
| Complete scoped gate | 1 | 962.84 |

The formatting duration includes the shared-slot wait. Four Rust input hashes
stayed fixed through the full run and final prose checks. The raw full log
SHA-256 is `24553d16df31738e342ed55b71890630dd7922b07e210c63a39cc34a51a82bec`.
The private reproduction bundle retains the plan, raw log, source freeze,
result and exact baseline comparison. Each Cargo command reserves the shared
build slot, with two build jobs and four test threads. The comparison is data
checking, not a passing full gate. The separate focused kernel method also
passes; its filtered result is not the complete gate.

The expanded kernel method covers both attachment
orders and object/array combinations at the outer and nested-call levels,
including an invalid object-form argument and an ordinary candidate refused
for missing postcondition evidence. The expanded host method covers all four
encodings for each shipped tool. They require one dispatch, zero tools or
permission events, no output or file change, a recorded refusal and stable
terminal results on another advance. Both host completion methods assert the
stale prompt member is absent.

These are synthetic kernel and host coordinator tests. They do not demonstrate
a successful native project-test process or an actual coding-model workflow.
Current-source verification, runtime acceptance, durable recovery, approved Git
and parent measurements remain open. No hidden/held-out task, scorer,
threshold, model, dependency, SBOM, status model or scoreboard is changed.
No GPU, model process or download is needed for this repair.
