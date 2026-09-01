# Verified Workflow Supervisor

## Status and scope

Story 23.6 adds one interface-neutral supervisor for immutable, dependency-ordered workflows. It
does not add another model, tool, approval, verifier, recovery, or event loop. The engine selects
one dependency-ready step and accepts only the terminal record produced by a fresh instance of the
existing reusable runtime coordinator.

The implementation is split across two narrow boundaries:

- `kernel/engine/src/verified_workflow_supervisor.rs` admits the canonical workflow and step-policy
  bindings, selects ready work, validates coordinator evidence, accounts budgets, decides whether
  an exact read-only retry class permits a fresh attempt, and seals safe-boundary checkpoints.
- `shells/host/src/workflow_supervisor.rs` composes a fresh common coding coordinator, advances it
  only through `drive_coding_client`, and classifies its verifier-owned terminal outcome. The
  adapter has no direct model, tool-dispatch, grant, receipt, storage, or effect API.

## Completion and recovery boundary

A step completes only when all of the following are current and mutually consistent:

1. the immutable workflow definition and every exact step policy pass closed admission;
2. all declared dependencies are already present in the verified completion prefix;
3. the factory reports a current preflight bound to the exact admitted policy digest;
4. the runtime request, complete hash-chained event sequence, and canonical outcome verify;
5. the terminal event binds the verifier-owned terminal state and outcome digest;
6. the outcome is `SUCCESS` or `NO_OP`, has no failure classification, and stays within the exact
   step budgets.

Model prose, reasoning text, confidence/classifier scores, transcripts, and client rendering are
not supervisor inputs. They therefore cannot create authority, select a retry, or convert an
incomplete outcome into success.

Only a canonical transient failure under an automatically retryable effect/retry pair may start a
fresh attempt. The successor receives a new attempt ordinal and the prior attempt identity for
lineage only. Attempt, coordinator-run, tool-call, grant, and receipt identities must all be fresh;
reuse fails closed. Uncertain effects always require reconciliation and are never replayed.

An interruption before the next attempt begins produces a content-free diagnosis plus a sealed
checkpoint. The checkpoint binds the definition, ordered policy digests, verified completion
prefix, immutable attempt lineage, and exact aggregate budget ledger. Resume verifies the digest
and identity ledger, then begins at the first unfinished dependency-ready step. Executable calls,
arguments, model text, and consumed authority are not retained for replay.

## Deterministic proof surface

The engine fixture executes three dependent read-only steps and creates a distinct runtime artifact
for each step. Each attempt contains a legal model/proposal-to-tool event path, explicit permission
decision, fresh grant, one terminal receipt, current validation evidence, and a canonical terminal
outcome. Additional fixtures prove:

- one transient read failure followed by exactly one fresh successful attempt;
- refusal of reused attempt, run, tool-call, grant, and receipt identities;
- uncertain-effect, cancellation, resource, stale-preflight, oversize, false-completion, and
  repeated-state terminal behavior without false success;
- safe interruption and restart without replay, plus tampered-checkpoint refusal; and
- identical canonical request, events, outcome, classification, and accounting through discard,
  counting, and capturing presentation sinks driven by the common coding-client loop.

The Story 22.5 production prepared-source vertical slice and Story 23.4 native multi-tool fixture
remain the source of real fake-model/context/native-artifact-tool/verifier composition evidence.
Story 23.6 supervises those same coordinator contracts rather than duplicating their internals.

## Authority and interface independence

The supervisor can choose only an admitted ready step and return a typed result. The trusted host
factory owns current context, preflight, policy, tool catalog, coordinator construction, and any
protected approval port. The common coordinator remains the sole owner of proposal decoding,
deterministic repair, call validation, grant consumption, native dispatch, receipts, artifacts,
verification, checkpoint publication, recovery events, and terminal state.

Chat, CLI, and headless callers may use different event sinks or renderers. Those sinks receive
already verified events and have no path back into the supervisor decision. Presentation failure
may stop that client, but presentation mutation cannot alter the canonical run.

## Retained evidence and limits

The source-bound local report is retained at
`artifacts/sprints/sprint-23/story-23.6/workflow-supervisor-report.json`; its raw focused test log is
retained beside it. The report deliberately makes no installed-package, qualified-production-model,
independent-review, Windows, or macOS claim. macOS validation is deferred under the current run
instructions and must be performed later against the exact final candidate commit.
