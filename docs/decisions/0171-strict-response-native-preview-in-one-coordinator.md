# Decision 0171: Strict responses prepare native previews in one coordinator

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0168 and 0170 |
| Scope | Trusted process-local strict-response interpretation and native approval |

Retain a process-local, single-use interpretation callback in a prepared endpoint
turn. Ordinary turns keep their existing tool-or-completion interpretation.
Only trusted turn construction selects this callback; no model or wire field
can construct it. The callback receives the normally completed, accounted model
text, original sealed request and coordinator-derived call identities. It runs
under the original borrowed phase control, with checks before and after it.
Incomplete, overflowing, stopped or token-limited answers never enter it.
Tool/event capacity is checked before callback entry. The callback may prepare
an inert preview under prior read consent; it must not execute an effect.

The result is only a tool identity, version and argument values. The existing
coordinator still owns the payload schema, encoding, digest, identities, input
charge, registry checks, protected challenge, grants, dispatch and verification.
The original response descriptor and accounting remain unchanged. Callback
refusal terminates without interpreting the text as a completion or falling back
to a model-authored tool envelope. A callback cannot certify task completion.

The host supplies a pair of private references to one original live repository.
The turn side retains the narrowed startup and strict source context. After
checking the complete original run, both original controls and the epoch clock,
it reuses the strict parser and fresh source reader to prepare the complete
native diff. It binds that diff to the original native edit boundary using the
coordinator's exact identities. The tool side delegates permission evaluation,
resolution and execution to that same boundary without substitution. A pending
effect cannot be replaced. Only an observed successful native execution returns
the same repository to preview preparation; failed, uncertain and spent native
boundaries remain held. Lock poisoning refuses; unwind never restores authority.

Keep one coordinator, cumulative resource ledger, continuous control and shared
epoch clock for the entire sequence, including approvals and later strict
turns. Do not use terminal owner return and a freshly admitted run as cumulative
continuation. The caller still owns read/disclosure consent, current snapshot
selection, original repository provenance, shared clock semantics and independent
completion checks. The shared workspace label alone cannot distinguish created
owners. There is no path adoption, durable recovery or automatic service factory
selection. General edit/test/retry, approved Git and coding measurements remain
open. T-303/T-306 stay open pending their independent acceptance and measurements.
