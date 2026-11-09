# T-303/T-306: endpoint controlled-write composition

Builder record for a non-closing core increment. Independent core review is
required. No row or sub-row is added or closed.

## Result and boundary

The original configured-endpoint coordinator and host context can now compose
controlled-write requests with trusted original scoped tool owners. They retain
the original exact decisions, complete resource ceilings, catalog restrictions,
state-change checks, independent completion verifier and endpoint claim/envelope
binding. The read-only launch, authority policy, native file owner and observation
verifier remain read-only. No worker manifest, sandbox, dependency, model,
evaluation task, hidden test, threshold or supply-chain output changes.

The kernel's former blanket controlled-mode refusal is replaced by execution
through its existing controlled-write rules. The old durable-mode refusal is
retained. Read-only catalogs still refuse writes and commands. The separate
standalone model-turn component remains read-only. No native model profile,
second loop, executor, permission system or replay authority is introduced.

## Verification

The final ordinary scoped fast gate remains **FAIL**, exit 1. Its complete test
command exits 101: **2,717 passed, 72 failed, 71 ignored, zero filtered**, across
45 test targets. All 72 failure identities exactly equal the unchanged
[T-301 baseline](t301-baseline-failures-2026-10-25.md): 53 host library and
19 Linux library failures. There are no additional failure identities. This is
an identity comparison, not a waiver or proof of every failure's cause.
The executable ownership blocker remains observed; the inherited ignored tests
remain ignored. No test selection, trust check or threshold is changed.
The [development guide](../guides/local-agent-development.md) requires keeping
this result FAIL while deferred T-301.1 blocks no mission row. No native host
gate or native sandbox acceptance is claimed.

The final run contains all **10 new methods passing**: eight kernel coordinator
methods and two host context methods. All 12 predecessor endpoint-coordinator
method identities also pass. The full kernel library reports 1,552 passed,
zero failed and eight inherited ignored tests. Whitespace, new-file whitespace,
retained-review format, Markdown, Rust formatting and Clippy all exit zero.
The seven command exits are `0, 0, 0, 0, 0, 0, 101`; overall elapsed time is
358.88 seconds including shared reservations. Formatting takes 25.20 seconds,
Clippy 9.87 seconds and the full tests 322.07 seconds including reservations.
All six changed Rust inputs were frozen before this run and remain byte-identical.
There is no source edit after it. Current lightweight checks run again after
this evidence and guide update.

Run the unchanged ordinary command with the installed Node runtime on PATH:

```sh
python3 -B scripts/local_agent_fast_gate.py --base 10036f5e293ce7b543425cbedfbce016c09403dd --timeout 600
```

Its Rust checks include all targets in the kernel, host, Linux and local API
packages, with no test filter and no fail-fast. Each Cargo operation uses the
shared build reservation separately; no GPU work occurs.

The new coordinator cases exercise a synthetic write followed by a separately
approved synthetic command, rejection of the earlier write decision for the
command, ten changed-decision refusals, cancellation and expiry for both operation
kinds, malformed/failed/uncertain effect observations, seven substituted verifier
candidates and completion without observations, read-only and unknown-tool
refusal, every missing required resource ceiling, cursor refusal and all 19
other canonical operation classes. An expired response is rejected before
advancement; a response-free advance then observes the original expired clock.
A malformed effect result latches reconciliation without inventing a terminal
receipt. Neither path dispatches an automatic retry.

The host cases use the original context and credential-owning socket transport
with a synthetic upstream and snapshot read owner. They also check the original
registered write schemas, changed run binding, missing process budget, durable
mode and cursor refusal. Synthetic effect ports change counters only: they
establish no native writes, test execution, worktree ownership or model quality.

Development failures are retained: the first kernel build omitted an event-ID
import; the next focused run passed 18 and failed two tests whose expectations
incorrectly demanded terminal results for expired approvals and malformed
receipts. Those tests now assert the original explicit refusal and reconciliation
latch; production validation was unchanged. The next focused run passed all 20
methods. The first complete gate then caught a missing host-test import and
failed compilation. Only the later complete run above describes the final source.
No failed or filtered development run is presented as a passing complete gate.

## Review and remaining work

The predecessor's [independent core review](../reviews/10036f5e293c-local-coding-runtime-budget.txt)
has **Verdict: PASS** for exact commit
`10036f5e293ce7b543425cbedfbce016c09403dd`, with no high/medium finding.
Its low L1 (reserve a release slice) and L2 (direct post-stop allowlist probes)
remain same-row backlog with earlier lows. The report is retained byte-for-byte.
It does not review this subsequent implementation.

Raw logs, plans, source hashes, tool provenance and the failure-identity comparison
are retained privately and named in the review request. Their data audit verifies
recorded results; its own successful exit never changes the gate's FAIL result.
No model, hidden/held-out task, scorer, threshold, baseline or scoreboard is edited.
There is no development-suite rerun or new score: native controlled-write
activation is still absent, and the chat-only baseline cannot measure this path.
Native endpoint edit/test/retry, durable recovery, approved Git and parent
measurements remain open. T-303/T-306 stay open. No owner-ready or release claim.
