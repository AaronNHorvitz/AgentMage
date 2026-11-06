# T-303 and T-306: schema consumer gate repair

The [independent project-test review](../reviews/cd3179cfbc94-local-coding-project-tests.txt)
has **Verdict: FINDINGS** for exact commit
`cd3179cfbc94d2933a2aba6000019baa3439cada`. It is retained byte-for-byte.
Medium M1 found that changing only the validation-receipt schema selected its
JavaScript tests without the host Rust consumer or producer-fixture checks.
This repairs that finding in the existing non-closing batch; no new task row or
parent closure is claimed. Separate exact-commit core re-review remains required.

## Change

The gate now seeds `agentmage-host` into package selection before expanding
reverse dependencies when the exact receipt schema or a registered current
producer fixture changes. It runs formatting, all-target Clippy and complete
unfiltered source tests for that selection. A schema-only change additionally
runs the existing producer manifest unit tests and reserved read-only production
fixture comparison, as fixture-only changes already did. JavaScript schema tests
remain required. Unknown schemas, JavaScript and fixture files still refuse.

The regression uses a schema-only path and a synthetic dependency graph with two
levels of host consumers. It requires all three Rust operations to cover the host
and both consumers, excludes unrelated crates, and requires manifest tests and
read-only reserved reproduction. Before the repair it fails because no Rust
operation is planned; after the repair it passes. Changing only the JavaScript
test still runs its own tests without unnecessarily selecting Rust consumers.

The [development guide](../guides/local-agent-development.md) and preceding
evidence now identify the embedded Rust schema consumer explicitly. The earlier
batch's successful producer reproduction did not prove schema-only selection.
No runtime, schema, producer fixture, model prompt, frozen evaluator, hidden test,
threshold, dependency or supply-chain artifact changed. Nothing is regenerated.

## Verification

The repair's per-change gate passes. A separate execution of the actual
schema-only plan runs all eight planned steps: the first seven pass; complete
host source tests retain their documented executable-trust failures. The two
results have different scopes and neither substitutes for the other.

| Check | Observed result |
| --- | --- |
| Schema-only regression before repair | Fails: no Rust operation exists in the plan |
| Gate and producer Python unit tests | PASS: 36 tests |
| Repair per-change gate | PASS: all six steps, including four changed Markdown files, unchanged retained-review format and 27 gate tests |
| Schema-only JavaScript consumer tests | PASS: 78 tests, no failed/skipped/cancelled/pending test |
| Schema-only producer manifest tests | PASS: 9 tests |
| Reserved read-only producer comparison | PASS: nine current files match; 0.45 seconds including reservation |
| Reserved host formatting / all-target Clippy | PASS: 0.70 / 18.33 seconds including reservation |
| Reserved complete host source tests | FAIL: 533 passed, 53 failed, 11 existing ignored, zero filtered; 76.70 seconds including reservation |
| Host failure identity comparison | Exactly the 53 failures in the retained host baseline, no new or missing identity |
| Documentation policy | PASS: 768 Markdown files and all policy invariants |
| Runtime, schema, fixture and frozen evaluation preservation | Unchanged; only the gate, its regression, documentation and retained review change |

The host failure identity digest is
`dbfcc2c2f86bedf8218c0e72145e1ab4ad5b78a1c2c01bd83a8f09354f909725`,
using sorted unique names with a final newline. The baseline is the unchanged
`batch35-source-2/11.log` under the retained standalone host verification logs.
The real workspace has no reverse dependents of the host; transitive selection
is exercised by the synthetic graph regression. Complete source checks run
without filters, new ignores, allowlists or weakened trust checks. No Linux
crate test target is selected for a schema-only change. The old long acceptance
gate is reserved for T-310.

Raw logs, schema-only plan and audit results are retained in the private review
scratch directory named in the request. No source changed after these checks.

## Remaining work

Low L1 from this review stays backlog before parent acceptance: unsupported
honest-runner output and compile failures can become non-retryable malformed
results. No parser or rerun policy changes in this repair. Prior native positive
HTTP coverage, shared error prefixes, refused-edit latest-result retention and
the same-user worktree identity window remain backlog. Model failure-to-repair,
native positive execution, durable recovery/cancellation, original-repository
apply, approved commit/push, MCP and measured parent acceptance remain open.
No model, GPU, development or held-out campaign ran; no new score is claimed.
