# Bounded original validation history

T-303/T-306 remain open. This is a process-retained command-series check under
Decision 0173, with no row/sub-row added or closed. Independent core review is
required. The preceding completion-turn medium repair remains under review.

## Behavior and authority

The original native validation owner retains at most three completed attempts,
including their original prepared commands, terminal command receipts and
normalized validation receipts. It also retains each original separately
approved rerun plan. Before dispatch it spends the same attempt and records the
edge, clearing the latest result. A lost, corrupted or uncertain successor leaves
the completed prefix inspectable and cannot establish a linked pass.

The kernel check revalidates every attempt against the same sealed template,
original command preparation and command receipt. It requires distinct
validation/command attempt identities, authority transactions, operations and
approval digests. Every prior result must satisfy the original failed-test rerun
rules; its exact plan binds the immediately following attempt identity and
approval. Missing, reordered, substituted or replayed links refuse. The last
receipt must be a full pass. No failed receipt is rewritten or omitted.

Native validation results and inspection expose `linked_pass_verified`.
Inspection also exposes the retained validation receipts as `history` and
original `rerun_plans`. These are process-only, bounded by the original
three-attempt ceiling, and grant no execution, replay or restoration authority.
All command/validation approvals, confinement and parser rules are unchanged.

This is not current-source verification, independent task grading, suite
coverage, or runtime completion acceptance. `current_workspace_verified` and
`test_suite_verified` stay false. The existing coding completion verifier still
rejects failed tool history and the executable development verifier still
refuses certification. Current-source binding and their acceptance integration
remain open. Native/model workflow, durable recovery, approved Git and parent
measurements remain open too.

## Verification

The complete scoped fast gate over all four affected crates remains **FAIL**:
2,833 passed, exactly 72 inherited failures and none new, 71 existing ignored,
zero measured or filtered across 46 unfiltered targets. The failure identities
exactly match the unchanged T-301 baseline. Formatting and Clippy pass.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 22.34 |
| Clippy, all affected targets | 0 | 26.60 |
| Tests, all affected targets, no fail-fast | 101 | 411.27 |
| Complete scoped gate | 1 | 461.89 |

All four new test methods pass, along with the two expanded native history
methods. Kernel cases exercise single-pass and two/three-attempt linked passes,
compilation/assertion failures, missing/reordered/substituted links, altered
hashes, re-sealed invalid records, repeated approvals/identities, partial
coverage and exhaustion. Native cases retain the exact failed and passed
receipts and rerun plan, preserve a failed prefix after a corrupted successor,
retain three failed attempts, and lose all process-only history on restart.
The focused tests also pass; their filtered counts are not the full gate.

All four Rust input hashes stayed fixed through the complete run and final
prose checks. The raw full log SHA-256 is
`3b4ae2d2d9148c53c4bb27a9e0c6d2090b612eadbe65c9198a10c8698da183bb`.
The private reproduction bundle retains the exact plan, raw log, source freeze,
result and baseline comparison, with commands that reserve each Cargo operation
through the shared build slot. It uses two build jobs and four test threads;
no resource limit is raised. The comparison is data checking, not a passing gate.

Kernel fixtures synthesize command receipts; native component fixtures use the
existing private synthetic executor and held roots. No native project-test
process, model or coding score is claimed. The unchanged executable-trust refusal
still prevents a measured native workflow in this lane. No GPU/model process or
download was started. No held-out/hidden test, task definition, scorer, threshold,
dependency, SBOM, status model, release gate or scoreboard is changed.
