# Guided endpoint terminal continuation — builder evidence

Date: 2026-11-09. Rows: T-303 / T-306, both open. Tier: core.
Base: `d580df2cdf3e53cf87c9e81448b863cb97df4d15`.
No row or sub-row is added or closed. Independent core review is required.

## Delivered behavior

The existing terminal client now accepts `endpoint-run <seconds> <run-id>
<request-digest>`. It pins the existing service request and schedules its
unchanged advance route until a terminal response, a declined confirmation or
an error. Every tool operation requires an exact controlling-terminal approval,
and every advance separately requires model disclosure consent. A local tool
approval is not transmitted if the subsequent disclosure is declined.

The original coordinator retains all authority, deadlines, budgets and
completion checks. The terminal supplies no tools, permission policy, model
transport, new owner, automatic approval or HTTP retry. It stops on duplicate
approval identities and more challenges than the admitted tool budget, and
bounds requests to that budget plus one terminal observation. Interruptions,
errors and deadlines do not send cancellation or release. Already dispatched
operations may still complete and require inspection. A blocked synchronous
stdout write can delay stop; output consumers must keep draining.

A fresh client can continue a still-held service run after inspection and fresh
consent. No saved record reconstructs execution. A terminal envelope retains
`service-observation-only`; client exit zero means that observation arrived,
including a failed or cancelled state, rather than task success. See
[Decision 0180](../decisions/0180-guided-endpoint-terminal-continuation.md) and
[the terminal guide](../guides/runtime-endpoint-runs.md#guided-terminal-continuation).

## Verification

The complete affected local API gate passes, using the existing fast gate with
`--base d580df2cdf3e53cf87c9e81448b863cb97df4d15`. Every Cargo command reserves a
shared build slot individually, with two build jobs and four test threads.
Only the retained Node installation is added to PATH for Markdown linting.

| Check | Exit | Seconds, including reservation waits |
| --- | --- | --- |
| Whitespace | 0 | 0.10 |
| New-file whitespace | 0 | 0.10 |
| Retained review format | 0 | 0.10 |
| Markdown, four files | 0 | 1.35 |
| Affected-package formatting | 0 | 629.13 |
| Affected-package Clippy, all targets, warnings denied | 0 | 39.32 |
| Affected-package tests, all targets, no fail-fast | 0 | 51.00 |

Total gate wall time is 721.17 seconds, predominantly reservation waiting.
The test run passes **248 tests across eleven unfiltered targets**, with zero
failures, ignores, measured tests or filtered tests. All ten named regression
checks pass. All five changed Rust-file hashes remain fixed throughout the gate.
Existing test lines and ledger rows are preserved; the retained report matches
its original byte for byte. Earlier host/Linux failures are outside this API
selection and are not cleared by it. Raw gate-log SHA-256:
`f9e92d6a324ba72d40cc87e3c00401c56f5b32b4ee176ec74f3b8eaa50ba9ef7`.

The private review bundle contains the exact changed-path and Cargo-command
selection, per-target results, source hashes, raw log, and reproduction checks.
Final prose is checked again after inserting these observed results.

Actual controlling PTYs with an authenticated recording HTTP peer exercise
separate decisions for two successive operations, repeated challenges, tool
budgets of zero and one, declined disclosure before and after local approval,
declined tool consent, SIGINT/SIGTERM/SIGHUP, prompt deadline, invalid limits,
nonterminal input, foreign run/request/task identity, malformed terminal
responses, refusals and lost replies. An approval already sent is never
resent, including stalled response headers/bodies interrupted by deadline or
signal. These peer tests establish client wire behavior, not native execution.

A separate actual retained HTTP service and synthetic streaming endpoint test
runs three fresh terminal executables. The first shares context and declines
the tool. The second receives that same pending challenge and confirms the
operation and next disclosure separately. The synthetic tool executes once;
the original deterministic verifier rejects the completion. The recorded state
is `FAILED`, with two model calls and one tool call. The third client observes
the same outcome without another execution or model request. The retained
canonical event record passes the kernel sequence validator, then the test
explicitly releases the run. No native filesystem tool, native Git, real model,
project test execution or coding score is inferred from this synthetic run.

The initial focused peer run had two passes and one failure: a new expected
response object included `selection: null`, although the existing serializer
omits that absent field. Only the new expectation was corrected. No existing
assertion, serializer or threshold was changed. The live retained-service
focused test passed. Private observations retain this history.

## Independent review and remaining work

The preceding [private-record review](../reviews/d580df2cdf3e-local-coding-private-records.txt)
has **Verdict: PASS** for exact commit
`d580df2cdf3e53cf87c9e81448b863cb97df4d15`, with no high or medium finding.
It closes no row. Its low L1 (saved-transfer archives can be inside either
checkout) and low L2 (standalone collision tests do not pin prompt count)
remain same-row backlog, along with inherited lows. This implementation is
pending its own independent core review; the builder supplies no verdict.

Native executable trust remains an owner-action blocker. Native/model coding
measurement, approved signed Git, durable execution recovery and inherited
host/Linux failures remain open. This client increment changes no runtime
prompt, frozen or hidden test, held-out set, scorer, threshold, dependency,
SBOM, status model, scoreboard, long gate or release. No model/GPU work,
download, resource increase, history rewrite or push occurs.
