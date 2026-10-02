# Canonical Research Source Consumption

Engineering choice: **Accepted under owner delegation, 2026-09-20**.
Authority: Decision 0054, within Decision 0084 and AMR-03.1.2.
Implementation and verification are in progress; this document is not acceptance.

## Existing owners

`DurableAuthorityRuntime` remains the entry point. Its operational store, authority
coordinator, grant issuer, runtime journal and artifact payload owner remain the only
canonical owners. Research adds no database, scheduler, permission issuer or query
language. The ordinary coding profile remains offline; this consumer enables no tool.

A provider citation, matching digest or syntactically consistent native-result record
is insufficient to establish retrieval. A consumer must resolve actual canonical
objects, not accept caller-supplied copies of a transaction, receipt or consumed grant.
Expected worker, runtime-manifest and confinement identities come independently from
trusted admitted composition. Finding those identities inside retained material is not
admission, and the canonical consumer cannot qualify a native producer by itself.

## Grant issuance for an admitted run

Decision 0139 adds `issue_public_get_grant` to `DurableAuthorityRuntime`. The existing
grant issuer derives one exact single-use network grant from the session parent whose
preview is the digest of the plan's full canonical bytes. The trusted host attests that
this is the plan the person confirmed (Decision 0140, note N1). A task-authorized plan
needs nothing more for a
request inside it. An ask plan also needs the person's approval of the exact request,
whose preview is the packet's digest; `prepare_public_get` returns that packet without
recording anything.

Before anything is committed, the budget owner checks the request without spending.
The plan must not be offline, and the budget must not be cancelled. The packet is
prepared from the retained plan's restrictions and original clock. The target must
classify as a disclosed query or a visit, and a dry-run reservation must fit. The full
plan must still be live. The call must match its registered tool and the packet
exactly. An action that already holds a grant never receives another. The governing
policy evaluates the exact child grant against the packet's own destination. The grant
names the packet as its only effect and expires with the packet or the parent,
whichever comes first. The owner commits the parent's new revision, the child and the
caller's permission event atomically. A refusal commits nothing and spends nothing.

Issuance is not a reservation or a dispatch proof. At the start, the authority
transaction also requires its policy context to name the packet's own destination.
No host, worker or provider issues these grants yet; Decision 0084 still governs
activation. The owner bounds the call's argument bytes and checks their digest before
it decodes them (Decision 0140, note N2).

## Persistence through the coordinator

Under Decision 0140 (AMR-03.1.1) the plan and each reservation persist through the
real coordinator and the real owners before the effect starts. The coordinator
publishes the plan and has its budget opened when the run starts, and the trusted port
reserves each request before it begins the effect. When an admitted run observes a
cancellation, the coordinator has the port cancel the task budget before it seals the
outcome, and it accepts only the owner's description of a cancelled budget for the
published plan and scope. Otherwise the outcome names
`runtime.research.budget_cancellation_unconfirmed`.

Composed tests interrupt the trusted glue after a reservation and after a start, and
reopen the store. The budget comes back with the same revision, head, counts and
original clocks. A spent request is never reserved or dispatched again, and an
interrupted effect is closed by recovery without a published result. A worker that
fails, times out or becomes uncertain keeps its spent visit. An expiry or a
cancellation is retained and survives reopening. A second run of the same task, or a
second owner of the same store, is refused. The native worker stays synthetic.

While an effect's terminal is pending, the owner refuses every other call with
`DurableAuthorityError::Poisoned`, the same error it returns for a poisoned store. A host
must not read that error alone as damage: the pending effect is closed by its own terminal
commit or, after an interruption, by recovery when the store is reopened (Decision 0142,
note N1 of the review of `a8fd53e3`).

## Reports through the coordinator

Under Decision 0142 (AMR-03.1.4) an admitted run that completes with a report draft has
it retained through `publish_research_report_draft` before its terminal. The trusted
host decodes the draft, supplies the independently admitted native identity and asks
the owner to check it at the coordinator's manifest instant. The owner's existing checks
decide: every source must be a complete public GET bundle of this run, every excerpt
the exact byte range of its source body within the quotation bounds, the run's own
accounting readable, the run not yet ended, and the manifest's classification,
retention and policy must cover every source. A refusal retains nothing. The coordinator
records the creation event only after the owner retained the draft.

That creation event stays in the coordinator's memory until the run's terminal is
flushed. A run that ends with a dependency error or an interruption after the owner
retained the draft and before its terminal, for example a clock failure at the next
phase check, leaves a draft whose manifest and payload are retained without a creation
event. An admitted run cannot be resumed, and `read_retained_research_report` refuses
every read of such a draft as a binding failure, so it is permanently unreadable; it
stays in the payload store until retention removes it. A host should expect such a draft
after an interrupted completion and must not present it as a report (Decision 0143,
review F2 of `8842c770`).

Composed tests read the retained draft back through `read_retained_research_report`:
checked, partial with unresolved questions, expired at the plan's elapsed limit, and
cancelled after a cancellation that arrived once the draft was retained. Source drift
refuses every read, and reopening reads the same report without reserving or
dispatching any request again. The owner refuses, and retains nothing for, a draft
that cites the plan, a forged bundle reference, an excerpt absent from its source, an
excerpt bound to another body, or more sources than the plan's visits. A draft that is
not the exact encoding of a valid draft never reaches the owner. Publication proves
only what was checked when the draft was retained; the claims stay the model's.

## Acyclic artifact publication

Retain six ordinary Report artifacts through the existing full-payload owner:

| Object | Complete content | Binding |
| --- | --- | --- |
| Call | Original canonical ToolCall, including exact argument bytes | Actual requested call and issued grant |
| Packet | Original bounded worker request | Original reservation and expected effect |
| Material | Redacted native result material | Terminal authority transaction result digest |
| Frame | Complete worker response framing and source body | Material's exact frame and body identities |
| Tool result | Original canonical ToolResult and three output references | Actual ToolCompleted result digest |
| Bundle | Transaction identity, reservation identity and five references | Canonical artifact publication and read restrictions |

The tool output contains packet, material and frame references. It does not contain
its own reference or the bundle reference. The bundle names the complete tool result
but is never added back into that result, avoiding a digest cycle.

The transaction result digest and ToolCompleted result digest deliberately differ:
one binds native material, the other the complete tool result. The start event binds
the issued grant revision; the authority transaction separately binds the consumed
revision. Neither distinction may be erased to make a fixture pass.

After execution yields a receipt, trusted composition may prepare exact manifests and
references in memory. The existing pending-terminal guard still forbids publication
or reads until terminal durability finishes. Publish complete bytes and matching
ArtifactCreated events afterward, in the original run, turn and operation. A crash
between these steps leaves an incomplete, unreadable bundle; it does not permit an
automatic network replay or a hash-only replacement for missing source bytes.

## Completion preparation

Decision 0097 adds closed normalization of an already successful native result.
It cross-checks the registered call, terminal transaction description, exact receipt,
original packet and reservation, independently expected native identities and observed
parent interval. The shared pre-effect call validator preserves approved JSON bytes.
These descriptions remain untrusted as provenance until the canonical reader resolves
and verifies the actual owners' records.

The helper appends source members, the complete ToolResult and the bundle through the
same borrowed artifact builder introduced by Decision 0096. It uses no identifier
allocator or clock. Returned references must describe the exact complete candidates,
with distinct identities. Sealing happens once, after every dependent reference exists;
the exact terminal must follow the original start and verified cleanup. A successful
network disclosure is Changed. Caller failure must propagate without effect replay,
partial acceptance, result replacement or another sealing attempt.

The research begin operation also exposes the existing durable-start observer through
its original private preflight/checkpoint owner. The compatibility entry supplies a
no-op observer. Observation follows durable consumption and start publication, and
precedes the driver. Observation failure preserves spent accounting and reconciles on
reopen without replay.

No ordinary runtime mode admits NetworkAccess. The normalizer is an inactive preparation
prerequisite; synthetic builder/store tests do not demonstrate coordinator admission,
native transport, a provider path or real-model research. The native adversarial and
independent review prerequisites of Decision 0084 still govern activation.

## Read-time requirements

Resolve the bundle under the exact session, task and policy, then verify:

- The real terminal transaction and receipt agree on attempt, approval, grant,
  correlation, action, call and operation, with success and no uncertain effect.
- The original call matches the registered tool/schema and actual grant history.
- The complete runtime chain contains its requested, started and completed events
  in order, with the exact operation/turn/correlation and both result bindings.
- The original reservation belongs to the task's fully verified bounded history;
  a newer head must not silently replace the reservation identity.
- The full prepared plan and every source artifact remain available under current
  ownership, lifecycle, retention, size and digest checks. Previews cannot substitute.
- Native material matches the original packet, full frame, independently expected
  producer identities and canonical parent interval.
- Every referenced artifact has the corresponding exact full publication event.

Envelope decoding remains closed and bounded. The binary frame uses its own inert
media type; JSON envelopes do not claim that binary framing is JSON. A returned
source is a point-in-time canonical observation, not a reusable read permission,
dispatch proof, statement of freshness or authority to execute retrieved text.

Before loading the complete event chain, the reader checks a separate work ceiling:
16,384 events and 8 MiB of serialized history per run. An oversized history returns
resource exhaustion; it never accepts a truncated prefix or discards retained events.
The existing store mutex covers both the bounded check and full verification. Its
exclusive SQLCipher connection refuses a competing owner before reading state;
this consumer does not introduce a second connection or relax that restriction.
This is a conservative read-work limit, not a change to retention or shared-machine caps.

## Two clocks, no renewed authority

Decode the original packet against the trusted ToolStarted time. Enforce artifact
expiry and access at the current trusted read time. Reconstruct restrictions from the
original plan and budget start, never by editing a deadline or resetting counters.
The read clock cannot precede the latest verified event in the owning run or the
budget's clock high-water mark. Reads themselves do not persist a new clock or
claim a globally trusted time source.

Later reservations, task-budget cancellation and a completed run may leave previously
retained source readable. Historical reads must not consume a revision, refund a
failed attempt, clear cancellation or yield fresh dispatch material. A released or
expired source is unavailable even if an old report or an earlier read mentioned it.

## Verification boundaries

Synthetic driver/payload fixtures exercise the actual canonical coordinator and
encrypted metadata owner, not native HTTPS or model execution. Required cases include
coherent but forged records, wrong producer pins, foreign owners, failed/uncertain
transactions, issued/consumed confusion, missing publication, source drift, lifecycle
changes, journal corruption, reopen and terminal-run reads without replay.

Native connected-worker admission, provider composition, claim-linked partial reports,
contradictions and actual model research-to-code campaigns remain separate work.
Independent review and human, platform and release acceptance remain open. No task
checkbox or historical qualification changes solely because this design is recorded.
