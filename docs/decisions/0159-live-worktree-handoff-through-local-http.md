# Decision 0159: Live worktree handoff through the local HTTP service

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-07 |
| Authority | Decisions 0153, 0154 and 0158; T-303/T-306 |
| Scope | Process-held ownership lifecycle through the existing local coding API |

Let the manual coding HTTP service transfer its actual successfully created
worktree owner into the existing native coordinator factory in the same process.
Require the control credential, explicit inspection and handoff consent, exact
held root identity, independently valid development activation and the unchanged
closed model/scenario selections. Do not serialize ownership or recover it from
a path, marker or receipt. Return the manual owner intact when handoff refuses.
Keep the manual and runtime authority stores separate.

Serve that coordinator through the existing versioned runtime routes. Retain
ordinary model-context and per-tool approvals. Exclude simultaneous manual
repository, chat and workspace operations while attached. Derive one fixed
embedded control-client scope inside the host; never accept a peer scope in
HTTP input. The canonical job ledger still decides each control request.

Require explicit release of every prepared and active run, followed by explicit
recovery consent, before returning the original manual owner. A terminal run
still requires release. Concurrent handlers refuse recovery. Drop the empty
factory before recovering the lease. Subsequent original application requires
fresh read consent, a complete current diff and a separate single-use decision.

An embedded service must not inherit the separate-process host's reliance on
process teardown to stop a busy worker. On shutdown, signal the existing passive
cancellation probe, continue draining bounded event/result channels, and join all
workers before returning. Preserve an earlier accepted cancellation. Do not
invent a terminal outcome or acknowledged job-control cancellation; incomplete
canonical records remain incomplete. Native execution deadlines still apply;
shutdown cannot report completion while a worker continues.

The separate-process launcher keeps its existing behavior. Durable ownership,
new-file transfer, arbitrary configured endpoint composition, native positive
workflow/model observations, approved commit/push, MCP and parent measurements
remain open. Tests of scripted coordinators and native atomic writers do not
qualify a model or close either mission row. No evaluation input, score,
threshold, source binding or release authority changes.
