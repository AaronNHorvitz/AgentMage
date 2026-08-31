# Tool Preflight, Attempt, and Verification Composition

Status: Story 16.3 local deterministic composition contract

## Closed registries

`PreflightRegistry` registers exact `1.0.0` probes for workspace, Git, executable, service,
network policy, platform, storage, credential-reference, and resource facts. Every definition fixes
freshness and evidence-size ceilings and treats ambiguity as denial. Observations bind the exact
call, canonical target set, and preimage set through one subject digest; they also carry an evidence
digest, bounded size, trusted observation/expiry times, outcome, and content-free reason.

`ToolCompositionRegistry` derives exactly one mapping for every definition in the common native
`ToolRegistry`. Each mapping binds effect class, ordered required preflights, approval rule,
deterministic verifier, retry policy, terminal diagnostic policy, and its own digest. The host checks
closure for both the 17-tool read-only/artifact catalog and the 21-tool coding catalog. Network,
command, Git, and credential-sensitive operations receive the additional probes implied by their
declared operation; no tool can choose its own probe set.

## Preparation and dispatch

`prepare_tool_attempt` validates the common tool call and capability arguments, canonicalizes exact
targets, preimages, expected effects, and postconditions, checks every current observation, evaluates
policy/approval/grant availability, and records the call in the existing `ToolAttemptGuard`. Its
result binds all of those facts into one operation-attempt digest without consuming authority.

Immediately before consumption, `ToolDispatchRevalidation` must still match the prepared policy and
preflight digests and report current policy, approval, and grant state. Drift fails before launch
commitment. `ToolExecutionLedger` then commits the unique attempt identity, the grant port consumes
exactly once, and only then may the worker port receive the prepared attempt and consumed-grant
digest. A failed consumption or duplicate launch commitment starts no worker.

The worker report binds its disposition, optional worker receipt, complete result digest, artifact
identities, changed-state identities, and cleanup state. The deterministic verifier evaluates exact
postconditions without model interpretation. The composition layer emits a separate terminal
receipt for every launch commitment. Completion is true only for success or verified no-op with a
worker receipt, passed verification, and complete cleanup; partial, denied, cancelled, failed,
uncertain, blocked, malformed, unreceipted, unverified, or uncleared results cannot become success.

## Recovery and replay

The execution ledger restores only sorted unique launch commitments and digest-valid terminal
records. A committed attempt with no terminal record reconciles once to an explicit uncertain
terminal receipt without relaunch. Reuse of the same prepared attempt is refused whether the prior
run finished, crashed, or failed during just-in-time grant consumption. Tests cover crashes before,
during, and after worker execution and after receipt, artifact, verification, and checkpoint
boundaries. Tampered restored terminal records fail closed.

## Boundary

The module composes existing common registries, repeated-call protection, exact grant-consumption
ports, worker ports, and deterministic verifier ports. It does not create grants, approvals,
credentials, worker authority, or model authority. Installed-worker and native platform execution
remain governed by the platform adapters and their separate evidence campaigns.
