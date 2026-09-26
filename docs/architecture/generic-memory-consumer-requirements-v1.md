# AgentMage generic-memory consumer requirements — candidate 1

Revision: 2026-09-26, candidate 1. Registered under AMR-03.1.3 in [TASKS.md](../../TASKS.md).
This is a consumer requirement and unexecuted scenario specification, not installed production
code, an accepted producer contract, evidence of a running counterpart, a backend cutover or
a new query language. No other repository was inspected. Independent acceptance remains open.

## Authority and dependency position

The owner's September 26 direction permits independently pinning AgentMage's consumer side.
Decision 0081 assigns execution/permissions to AgentMage and makes memory initially an optional
rebuildable derived index. CAP-20's citation/source boundary is P0; CAP-29 runtime memory
integration remains P1, dependent on AMR-02/AMR-04. These requirements prepare that boundary;
they neither block standalone coding/research on an absent service nor advance P1 ahead of P0.

Engineering choice status: **Accepted under owner delegation, 2026-09-20**. Authority:
[Decision 0054](../decisions/0054-standing-owner-delegation.md),
[Decision 0081](../decisions/0081-rust-capability-roadmap-and-staged-delivery.md) and the owner's
September 26 consumer-side assignment. Use backend-neutral requirements with synthetic scenario
fixtures; no Rust dependency, service launch, shared database, copied producer implementation,
credential exchange, source upload, migration or independent-review assertion. This status
applies only to AgentMage's choice of its own constraints, never to counterpart approval.

## Inspected baseline and reusable owners

Baseline HEAD: `5b731b36d646aa22baa78f310544ea6ebce668e3`. The following inspected files are
unchanged against that baseline unless stated separately. SHA-256 pins:

| Path | Digest |
| --- | --- |
| docs/architecture/uste-memory-integration-plan.md | 52ab715135976fcd879c17e1725e9d0fd92e4530f6445fc7e95fecd62e6e06c7 |
| docs/architecture/source-backed-memory-boundary.md | 3596ca70d31759ed1cb885199307cebb81375cf95da82bb2a6b7af3529954792 |
| capabilities/knowledge/src/memory.rs | f880bf3998de8d97cb8e6e0484126985173f186c52da900f82da46f7b8ae8c92 |
| capabilities/knowledge/src/memory_working.rs | 6d2c6a50a157d38fb56805796ddbc6ce650ce26cc08a4434a49c3e750d68ebf7 |
| capabilities/knowledge/src/memory_lifecycle.rs | c9ad2cbbd3c23eb60e7e87cdfae77aa41ce7ba3a56c126b5005088398be8472c |
| kernel/engine/src/source_preparation.rs | 9facf973fb5a8e10e8c8c09c3edc1f900ad5a86fc90a55229d987d97aa5ce5c1 |
| kernel/engine/src/source_runtime_context.rs | bf2c734e5052dbd861d2f4d7606e320ba36e38cbe11372223c110785423e68de |

Reuse `MemoryScope`, `MemoryType`, `MemoryItemStatus`, `MemoryItem`, `MemoryLoadQuery`,
`select_memory`, `EvidenceReference`, existing source preparation/full-artifact owners and
the served-model context budget. `select_memory` filters caller-supplied items; it does not
authenticate a backend reply or validate current source permissions. Its optional project
and conversation selectors are filters, not authority to broaden a granted scope. The
consumer adapter must verify those boundaries before calling existing selection/rendering.
An `EvidenceReference` or provider-reported hash alone is not proof of readable source bytes.

The old integration proposal's counterpart dates/statuses are historical planning statements,
not observations of current USTE. This draft neither updates them from speculation nor fetches
counterpart content. Any offered producer contract must be supplied as an exact artifact and
reviewed independently before AgentMage records compatibility.

## Required consumer invariants

1. **Authority stays local.** Consumer derives workspace/project/conversation, task/run,
   governing policy and read/disclosure scope from current trusted state, not model or provider
   fields. An index cannot mint grants, approve facts, execute procedures or register tools.
   Approved memory still cannot authorize a future command. Durable promotion/correction keeps
   existing exact human decision requirements; derived ingestion only mirrors already-approved
   versions, not raw working context or full conversations.
2. **Closed negotiated operation.** Pin producer build, schema/version, supported capability
   set and consumer mapping. Refuse unknown required features/versions, unbounded frames,
   duplicate fields/IDs or malformed identities before using content. No arbitrary SQL,
   graph/query language, script, path lookup, backend database handle or ambient network route.
   Fixed existing literal terms and explicit filters suffice for the initial experiment.
3. **Exact identity.** Bind each reply to request identity/digest, authenticated peer identity,
   scope, policy, original deadline, producer epoch and snapshot/index generation. A random
   string or self-reported digest is not authentication. Same request key with different
   immutable content is a conflict. Epoch rollover/rebuild/restore invalidates old cursors;
   stale or foreign continuation cannot widen scope or resurrect deleted evidence.
4. **Derived-only ingestion.** A source change and optional index update do not share an atomic
   transaction. Reuse canonical outbox/reconciliation ownership where available; define it
   explicitly before wiring writes. Duplicate exact operations resolve to their original
   durable outcome; timeouts after possible submission are uncertain, never presumed undone.
   Automatic retries may not create a second write or reset deadline/budget. No second runtime
   journal, store or coordinator may be introduced by the adapter.
5. **Current authority wins.** Verify candidate/item digest, accepted decision, exact source
   versions, lifecycle and revocation/deletion epoch at delivery time against the existing
   authority. An index watermark is descriptive, not permission. Missing current validation
   means refuse the affected query, not serve a stale "best effort" hit. Index absence may
   visibly fall back only to an already-authorized local retrieval path under the same bounds.
6. **Full evidence chain.** Resolve sources through the existing source/artifact owner, verify
   content hash and bounded exact locator/excerpt against full bytes, retain source version,
   recorded/observed times and readable-reference identity. Corrupt/missing/released/expired
   source, mismatched excerpt/range or unsupported locator cannot become a grounded claim.
   Source text stays untrusted data, including instructions, links and forged authority.
7. **Temporal honesty.** Current approved/held items are ordinary retrieval; historical and
   as-known-at modes must be separately explicit and supported. Do not equate existing
   `include_historical` with proven bitemporal queries. A correction learned later must not
   enter an earlier as-known-at answer. Conflicts are shown, not resolved by ranking. Consumer
   displays unsupported temporal requests rather than fabricating unavailable history.
8. **No side channels in reports.** Foreign existence/counts/IDs/scores/errors do not enter
   model context, progress or diagnostics. Log request/outcome classes and safe opaque bindings,
   not raw queries, excerpts, credentials, operator paths or private consumer identities.
   Small-domain content/query hashes may disclose information; do not call them anonymization
   or publish them automatically. Rejected payload content must not appear in debug output.
9. **Bounded context and work.** Freeze synthetic initial profile at no more than 16 results,
   64 KiB combined UTF-8 excerpt bytes, 1 MiB total decoded reply including metadata and
   5 seconds end-to-end per query (all subordinate to existing task/operation ceilings).
   Reject pre-decode oversize; actual served-model tokenizer/full rendered prompt and reserved
   output remain authoritative. Include tool schemas, pinned instructions, history, wrappers
   and provenance in accounting. No silent context truncation or claim of fully read sources.
   The numbers are consumer maxima, not measured performance or a request to grow machine caps.
10. **Cancellation and recovery.** Original deadline and request identity survive restart.
    Do not restart timed-out work with a fresh budget. Cancellation prevents later response
    delivery and index results cannot reenter after revocation. Use existing process/lease
    owners; clean up only owned children. Reopen returns explicit current generation, lag and
    failure state; restore behind deletion high-water fails closed before reads.
11. **Deletion truth.** Immediate read exclusion is distinct from index deletion, source purge,
    derivative purge, backup expiry and cryptographic erasure. Return the actual achieved
    boundary; never claim physical erasure for a tombstone. An index can be rebuilt from current
    authority without source loss; disabling it must not remove canonical memory.

## Producer artifact needed later

Exact immutable contract/schema and fixture hashes; build/revision and licensed dependency
inventory; bounded read/resolve/reconcile/revocation operations; generation/epoch/watermark
semantics; authentication and process ownership; cancellation, uncertain writes, durable outcome
lookup and idempotency; explicit unsupported features; reopen/deletion and corruption outcomes.
AgentMage will map these independently to the invariants above and run consumer fixtures. An
offered candidate, producer self-test or USTE-only demo does not establish combined integration.

## Fixture handoff and admission

The companion [candidate fixtures](../../fixtures/memory-consumer/v1/conformance-candidate.json)
list 34 deterministic synthetic expectations. It is a fixture specification, NOT passing test evidence or the producer wire
schema. A later Rust harness must execute each through the actual consumer validator and its
existing owners, with a labelled fake producer for component checks. Real counterpart/service,
admission/security, interruption/recovery and source-backed model-context checks remain separate.
No M-HARNESS-DAILY, independent review, model, platform or release milestone closes here.

## Specification validation and freshness

Four [specification checks](../../tests/test_memory_consumer_candidate.py) validate the
candidate flags, exact 34-case closure, fixed resource maxima, seven AgentMage historical
source pins and open task/acceptance wording. They do not execute the scenarios, a Rust
adapter, a producer service or model context retrieval. All four specification tests passed;
the retained private log SHA-256 is
`16fcc35a7255db25dc555eaa957adab4228b9026766056495aa36d7de017665d`.
The requested documentation-lint command also exited successfully.

This new document and task row are planning inputs, not renewed acceptance. Existing memory,
coding, privacy and integration receipts remain evidence only for their recorded source pins.
No historical producer status is silently reaccepted as current, and no old evidence binding
is removed or narrowed. Any later gate using the changed inputs must explicitly revalidate them.
