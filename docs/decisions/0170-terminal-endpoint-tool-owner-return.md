# Decision 0170: Return the original terminal endpoint tool owner

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-08 |
| Authority | Local agent mission; T-303/T-306; Decisions 0168 and 0169 |
| Scope | Trusted process-local continuation after a terminal endpoint run |

An endpoint coordinator owns its native tool boundary exclusively. Ordinary
release discards it. A trusted host also needs to recover that original owner
to prepare a later edit, test or original-checkout diff without adopting a
worktree from a path or recreating its authority from receipts.

Allow consuming a terminal endpoint coordinator to return its original tool
boundary. Require the retained whole terminal envelope and shared outcome to
agree, the final event to name that terminal state and digest, no pending
approval or suspension, no reconciliation requirement and no uncertain terminal
state. Refusal returns the same coordinator unchanged, without entering a
dependency. This operation neither retries an effect nor reconciles uncertainty.

The retained host registry checks the exact original sealed request. Successful
return leaves the original identity and digest permanently spent, as ordinary
release does. Live approvals, faults and uncertain effects remain inspectable
after refusal. Export canonical events first: successful return discards the
old coordinator, context, model connection, verifier and budget ledger. A later
run needs its own admission and exact approvals; no old run is resumed.

The prepared edit/command boundary returns the same original repository through
the existing native abandonment methods. Successful edit lineage and validation
attempts remain with that owner. Pending previews and issued handles are
abandoned; durable grants retain their ordinary expiry. There is no new wire
route, durable recovery, path adoption, automatic cleanup or approval inheritance.

The dedicated HTTP service offers trusted startup an opt-in tool-return entry
point. In that service only, the existing exact control-authorized release
requires the same safe terminal checks and retains the boundary until service
shutdown joins every worker. Observation credentials cannot release it. Repeated
or substituted releases refuse; uncertain/faulted releases preserve inspection.
Stopping without a successful explicit release returns no tool. Service errors
can drop process-held state. The caller must export events before release and
abandon old native previews before separate later admission. No serialized port,
path adoption, renewed grant, budget or old run is returned. The boundary keeps
its existing native authority. Ordinary service entry
points retain their discard behavior.

Tests exercise terminal ownership and fresh approvals with synthetic endpoint
responses, plus original native edits and synthetic command observations with
real authority receipts. They establish no complete native Git/model workflow,
independent task grade or coding score. Executable controlled startup, general
model-driven coding, durable recovery, approved Git and parent measurements
remain open. T-303/T-306 remain open; no row or sub-row is added or closed.
Independent core review is required.
