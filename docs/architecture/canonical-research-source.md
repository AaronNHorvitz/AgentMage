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
