# Decision 0101: Runtime Phase Deadline Admission

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-28 |
| Authority | Decisions 0054, 0061, 0081, 0088, 0099 and 0100; current owner restart |
| Scope | AMR-01/AMR-03 and Tasks 48.2.4.4, 48.2.5.1 and 48.2.5.3 prerequisites |

## Finding

Source inspection shows that the coordinator checks elapsed run time at the
start of a turn, while every model request receives the entire original timeout.
Context construction and token binding, model results, permission evaluation,
pending approval resolution and verification can proceed between those checks.
A protected challenge can still be valid after the shorter run budget expires.
These are distinct lifetimes; a valid response cannot extend the run budget.
Nine original deterministic clock regressions executed against unchanged
production source: one ordinary-success control passed and eight cases failed.
They observed renewed model budgets, work after context/model/permission delays,
an unexpired approval launching after run expiry, a pending poll that did not
terminate and late verification becoming success. These are component fixtures,
not native process or model qualification.

## Decision

Use the existing coordinator's original run start and trusted monotonic clock
to admit subsequent phases, rejecting observed clock regression. Keep the start
restored from canonical history;
turns, reconnects and approval responses do not renew it. Bind model requests
to the positive remaining time observed at request construction. Preserve that
exact request and its published digest, then check the run again before dispatch
and before accepting a returned proposal for further work.

Record the request construction clock sample as its canonical `ModelRequested`
event time. Checkpoint recovery reconstructs the exact remaining-time request
from that event and the original run start. For an older journal, retain support
for its exact original-full-timeout request digest. Neither form admits an
arbitrary replacement timeout or resets the run start. A digest must match all
the original request fields; no request or historical event is rewritten.

Check cancellation and elapsed time around context construction and each exact
token-binding callback. Preserve an observed cancellation's original identity
through the callback return. Foreign identity and failed observation remain
failures; expiry never fabricates a cancellation. A callback or port that failed
for another reason does not become successful because a deadline also elapsed.

Stop an expired pending run before authority resolution or worker launch, including
when polled without a response. This happens on the next coordinator advancement;
it does not install an idle timer or make client status projection advance a run.
Keep malformed, foreign and stale response
validation ahead of advancement so rejected client input cannot consume a valid
pending challenge. Recheck after permission evaluation or resolution before an
allowed effect starts. Keep the exact grant, preview, tool definition, ownership
and consumption rules; never shorten a signed operation by silently changing its
already approved contract. Issued authority is neither refunded nor replayed.

Recheck successful verification before declaring success. Preserve canonical
model terminal events, resource accounting, actual tool receipts, uncertain
effects, terminal persistence and cleanup. Stopping a later phase cannot erase
work that already occurred or manufacture a receipt for work that never started.
Use the existing exhausted/cancelled/failed outcomes and event stream. Add no
scheduler, model owner, store, permission class or wire contract.

The existing event grammar cannot close a turn while its tool or approval remains
pending. Do not invent a denial, cancellation, tool result or completed turn to
hide this state. Permit the existing exhausted run terminal to end a turn whose
single requested tool has never started, with no active model or route and only
that operation's optional unresolved approval. Keep started operations and
incomplete effect observations ineligible for this path. The terminal prevents
any successor dispatch; previously published requests and decisions remain in
history. Add positive and adversarial event-sequence tests for this narrow
pre-execution exhaustion transition. Other terminal transitions retain their
existing requirements.

This is the coordinator prerequisite for controlled model preparation on the
existing live worker. It does not preempt blocking context, model, tool, verifier
or persistence calls. Event publication itself consumes time after a request is
bound. An in-flight absolute deadline across manifest hashing, loading, token
binding and native preflight still requires control propagation through those
existing owners. Decision 0100's socket deadline does not supply that integration.
No total hard wall-clock ceiling or native process termination follows here.

## Verification boundary

Retain pre-fix results using original deterministic ports and explicit trusted-clock
advances. Cover remaining model budgets, context/token callbacks, late completion
and tool proposals, permission evaluation/resolution, pending polls and valid
responses in ephemeral and durable modes, late verification and ordinary success.
Add cancellation identity, failed-probe and race cases while preserving actual
receipts and exact event/outcome validation. These fixtures do not qualify an
executable workflow, model or native cancellation campaign.

The first fixed run passed 18 of 19 focused tests and stopped on cancellation
event validation. Its producer used a run-level cancellation code in a model
failure event. Correct the producer to the existing model cancellation code;
preserve the diagnostic allowlist and retain that failed run.
The corrected focused run passed all 19 tests; the full engine run then found
one rejected-proposal recovery failure because its validator still reconstructed
the old full-timeout request. Retain the failure and verify the construction-time
binding and legacy-digest compatibility before accepting this batch.

Keep implementation and tests covered by existing whole-file input bindings.
Finish the source batch before one applicable evidence regeneration. Preserve
all recorded failures, independent review, human-only gates and the full roadmap.
