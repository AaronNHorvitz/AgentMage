# Decision 0096: Runtime Artifact Preparation

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0081 and the current owner restart |
| Scope | AMR-03 canonical artifact preparation prerequisite |

## Decision

Extend the existing coordinator's synchronous terminal-event builder with pure,
bounded artifact preparation. The trusted host may append complete candidates
after observing the durably consumed start, before sealing its terminal result.
Preparation returns descriptive references, without publishing bytes, metadata or
events, granting authority, or adding another identifier allocator or store.

Use the coordinator's existing artifact ordinal, classification, retention and
resource ledger. Every nonempty append consumes the same output, artifact and disk
budgets; the existing per-operation candidate ceiling applies to the complete set.
The first preparation samples the trusted terminal observation clock once. All
prepared manifests and the terminal event use that observed time. This is the
observation after native completion, not a future time guessed by the host or a
claim about the later persistence instant. Original effect deadlines and native
cleanup requirements are unchanged.

The same receipt identity and digest must accompany every append. Sealing requires
the final execution's complete candidate sequence to match every prepared byte,
kind and media type, with the same receipt. It also checks remaining output and
event capacity. Invalid preparation, late preparation, a failed append or repeated
sealing is sticky; the host cannot recover by ignoring an error or switching to an
unprepared result. Existing callback/returned-result checks from Decision 0090 and
advancement reconciliation from Decision 0091 remain mandatory.

After the exact canonical terminal commit, publish the prepared candidates in
their original order through the existing artifact port, then route ordinary
output. Each publication keeps payload-before-metadata-before-event ordering.
This is not an atomic multi-artifact transaction: a failed publication or event
append can leave a partial bundle, which must remain unusable as complete research
evidence. Failure requires reconciliation and never automatically replays an effect.
Unprepared operations retain their current output ordering and accounting.

The builder is borrowed only for the current correctness call. Public references
are descriptions, not reusable publication or effect permits. Frozen wire contracts
and the canonical research reader are unchanged. Ordinary modes continue refusing
network operations. This component prerequisite does not enable research tools or
replace Decision 0084's native adversarial qualification before host activation.

## Verification boundary

Exercise dependent references, exact terminal timing, complete resource accounting,
receipt and candidate substitutions, append/seal ordering, ignored failures, absent
durable artifact ownership, publication and journal failures, and no replay after
failed advancement. Preserve existing correctness-boundary tests. Synthetic prepared
artifacts are separate from native retrieval, provider, model, manual-user,
independent-review and release acceptance; no such gate closes with this decision.

Bind the complete new production module, its tests and the adapted host boundary
in the existing source evidence. The automated coordinator boundary checks inspect
both the original module and its preparation child. Moving code must not narrow
the no-client, no-transport, no-native-effect or no-storage checks. Add the prepared
artifact matrix to the bounded campaign without removing existing cases or changing
its time, memory or performance thresholds.
