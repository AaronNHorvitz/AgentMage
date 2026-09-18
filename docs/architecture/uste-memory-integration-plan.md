# USTE memory integration proposal

Date: 2026-09-18 (original proposal: 2026-09-16)

Status: owner-requested planning document, not an implemented backend, accepted runtime
cutover, release-gate closure or authorization to bypass the existing execution order.
AgentMage source baseline inspected: `c99c8254a4795d2a076c36f700e2daf8e2753bc4`.
USTE was documentation-only at the original proposal date. Its original design baseline is
[`be5f7a489f5fcaf4d4eb9c08f9860b9a06f68170`](https://github.com/AaronNHorvitz/USTE/commit/be5f7a489f5fcaf4d4eb9c08f9860b9a06f68170)
(design draft 1.2), not a tested runtime dependency.

The updated delivery plan is pinned to USTE
[`1ad74fa`](https://github.com/AaronNHorvitz/USTE/commit/1ad74fa650dcdb1e2562d765ca30e47d310272cd):
[M1 milestone](https://github.com/AaronNHorvitz/USTE/blob/1ad74fa650dcdb1e2562d765ca30e47d310272cd/docs/memory-first-milestone.md)
and [Decision 0054](https://github.com/AaronNHorvitz/USTE/blob/1ad74fa650dcdb1e2562d765ca30e47d310272cd/docs/decisions/0054-memory-first-delivery.md).
This is a planning reference, not a qualified runtime pin. The Rust foundation now exists;
M1 acceptance and AgentMage integration are still open.

## Purpose and authority

Explore USTE as a local Rust-native spatial-temporal graph and content backend for AgentMage
memory. Preserve AgentMage's source-backed memory policy, approval boundary, privacy and
strict-local behavior. Do not replace a working demo or authoritative memory path merely
because a backend architecture has been specified.

Current authorities remain [TASKS](../../TASKS.md),
[status model](../../architecture/status-model.json), accepted Decisions and the
[source-backed memory boundary](source-backed-memory-boundary.md).
This proposal does not enable a model, platform, connector, network route or persistent
memory capability. Its integration tasks must be registered into the existing roadmap under
the accepted execution rules before implementation. No new parallel sprint plan is activated.

USTE remains independently named and domain-neutral. Its
[implementation plan](https://github.com/AaronNHorvitz/USTE/blob/main/docs/implementation-plan.md),
[spatial contract](https://github.com/AaronNHorvitz/USTE/blob/main/docs/spatial-world-model.md)
and [time contract](https://github.com/AaronNHorvitz/USTE/blob/main/docs/time-and-ordering.md)
describe intended capabilities, not availability. Integration must later pin an exact tested
USTE commit/version, feature set, disk format and dependency/license inventory; a moving branch
link is not an admissible release dependency.

## Existing components to preserve

- [Memory candidates and policy](../../capabilities/knowledge/src/memory.rs): scoped,
  source-backed candidates and exact user decisions; model proposals do not grant authority.
- [Working memory and selection](../../capabilities/knowledge/src/memory_working.rs): bounded
  temporary context, compaction proposals and selective retrieval.
- [Memory lifecycle](../../capabilities/knowledge/src/memory_lifecycle.rs): catalog transitions,
  correction/supersession, retention and Markdown projections.
- [Portable memory](../../capabilities/knowledge/src/memory_portable.rs): authenticated,
  closed-schema export/import; not automatic filesystem or credential authority.
- [Host file planning](../../shells/host/src/memory_file_runtime.rs): controlled publication,
  backup/restore and conflict plans through existing kernel/platform grants.

These source components are not proof that durable USTE-backed memory is integrated into the
current local demo. Existing production status and demo evidence remain unchanged.

## Mapping the five memory functions

| Function | AgentMage responsibility | Proposed USTE representation |
|---|---|---|
| Working | Prompt/token budgets, transient session context, explicit compaction proposals | Optional authorized checkpoints only; a database does not enlarge a model context window |
| Episodic | Consent, scope and redaction for selected interactions/tool observations | Immutable events/artifacts with source times, recorded revision, evidence and retention |
| Semantic | Candidate policy, exact human acceptance, confidence and contradiction decisions | Entities, assertions, relationships, evidence and bitemporal corrections |
| Procedural | Approve a method separately from permission to execute it | Versioned procedure artifacts with prerequisites, outcomes and evidence; inert on retrieval |
| Forgetting | User decisions, holds, expiry/purge policy and visible completion receipts | Dependency-aware revocation/purge across originals, graph, indexes, summaries and branches |

Preference remains an explicit existing memory type; it can map to scoped assertions without
silently changing the public taxonomy. Episodic memory is not a blanket instruction to retain
full chat logs, secrets or every tool result. Capture only policy-admitted content. Encrypted
storage does not make prohibited memory eligible. Spatial memory is optional enrichment:
an entity may have location/movement evidence, but most facts need no position or physics.

## Trust and process boundary

The proposed initial transport is an authenticated local USTE service supervised through
AgentMage's existing process/IPC controls. Exact transport, credentials, key ownership and
startup/recovery design require an accepted integration decision. An embedded Rust backend
may be evaluated separately; it must not leak unrestricted database handles to model code.

The model proposes a bounded request. AgentMage authenticates scope, applies memory policy
and required human decisions, then its trusted adapter invokes USTE. USTE independently
checks its namespace/capability and transaction constraints. Neither a retrieved instruction,
an arbitrary actor string nor a simulation branch can approve a memory write.

Preserve exact workspace/project/conversation scope; use deny-by-default mappings and never
derive identity from paths or inferred name matches. Foreign records, counts, graph paths,
nearest results and source existence remain inaccessible. Physics, geolocation and unrelated
workers stay disabled for ordinary memory retrieval. No automatic download or cloud fallback.

USTE's Rust-only engine policy does not make AgentMage's entire model/runtime stack Rust-only.
Inspect the actual resolved build and keep optional/native worker profiles accurately labeled.
USTE's MIT OR Apache-2.0 licensing and AgentMage's current repository license remain separate;
review exact dependency/distribution terms before packaging and do not copy AgentMage source
into USTE as though it automatically inherited USTE's license.

## Proposed adapter contract

These are conceptual operations, not existing Rust symbols or callable endpoints:

| Operation | Required inputs and behavior |
|---|---|
| Reconcile source | Exact approved memory/source version, scope, provenance, policy and idempotency key |
| Query memory | Trusted scope, memory types, knowledge revision, relevance/graph/time/optional spatial filters and hard budgets |
| Resolve evidence | Immutable source version plus locator; current permission and byte/coverage checks |
| Apply approved transition | Digest-bound AgentMage decision, exact prior version, lifecycle change and durable outcome lookup |
| Reconcile revocation | Policy/deletion epoch, affected identities and dependencies; block stale reads immediately |
| Inspect health | Supported version/features, lock state, index watermark, retention boundary and recovery status without sensitive payloads |

Results map back into existing memory hits: identity/type/status, scope, candidate and decision
digests, source evidence, sensitivity, confidence, verification/supersession, ranking reasons
and visible limitations. Include USTE revision and sync watermark. An incompatible schema,
unknown outcome, stale policy, missing history or unavailable key is an explicit state.

USTE may prefilter by bytes/results, but AgentMage's served model tokenizer and full rendered
prompt remain authoritative for context admission. Account for system instructions, history,
tools, retrieved snippets and output reserve together. Retrieval is not automatic context
compaction, and neither database capacity nor a high hit count proves useful recall.

## Read lifecycle and accuracy

1. Bind workspace/project/conversation and currently valid source permissions.
2. Query only policy-eligible approved/held memory with bounded work and explicit temporal intent.
3. Reconcile returned IDs/version digests against the current authority and sync watermark.
4. Recheck evidence access and stale/superseded state; refuse or clearly omit unavailable data.
5. Pack cited snippets into the served-model context budget; report omissions and conflicts.
6. Treat the answer and extracted updates as proposals, not automatic durable memories.

UTC is the comparison timeline; display source-local or user-local time with the named zone
and offset. Event time, file metadata, when AgentMage learned a fact and when a derivation
became available stay distinct. Historical questions cannot use corrections learned later.
Location results distinguish observed, estimated and simulated state, including uncertainty.
Physics output may inform a hypothetical plan but cannot become a remembered observed fact.

## Staged integration and migration

### Delivery priority — bounded memory pilot before full world-model support

The owner-approved USTE M1 plan separates a small, experimental memory backend from its full
spatial/physics and disk-scale release. USTE tasks T-63–T-68 cover a verified bounded baseline,
durable source-backed writes, cited graph/lexical retrieval, correction/revocation, restart,
and a generic offline Rust consumer harness. All are open planning tasks, not working features.
Its existing R1/R2/R3/R4 gates and benchmarks remain unchanged.

For AgentMage, prioritize the proposed ordinary-memory path through UM-06 before optional
UM-09 spatial/physics work once these tasks have been admitted to the authoritative roadmap.
This proposal does not reorder the existing first-release critical path, register new stories,
enable persistent memory, or authorize a runtime cutover.

The earlier backend checkpoint can qualify Stage B without completing all of USTE R2, but
only after every required M1 case and AgentMage's own admission/security tests pass.
Start with synthetic fixtures and shadow retrieval; keep the existing source store authoritative.
An embedded Rust harness is not proof that the proposed authenticated service transport works.
Require separate process/IPC/key validation if that transport is selected.

Use the exact tested build and declared caps, not a moving branch or unverified recovery edit.
Full state, history, journal/retry metadata, source storage and recovery peaks must fit the
profile; refuse out-of-budget ingestion instead of risking desktop memory exhaustion.
Basic revocation and fail-closed rebuild are mandatory now. Complete physical purge,
backup/restore, sensitive production data and authoritative migration remain later gates.
A USTE-only demonstration does not complete UM-04–UM-06 or prove AgentMage integration.

### Stage A — Contract fixtures, no runtime dependency

Register the proposal with the authoritative roadmap and make mapping/reference fixtures.
Use a fake adapter to test policy, failure states and round-trip preservation. No Cargo
dependency, running service or new memory authority is required for this step.

### Stage B — Experimental derived index

Entry: USTE M1/T-68 passes its bounded memory acceptance cases (or a later fully qualified
superset); AgentMage's applicable admission, process/IPC and evidence gates are satisfied.
The initial experiment uses synthetic data. Existing AgentMage memory remains authoritative.
Import only approved exact versions into an opt-in rebuildable index. Journal/outbox design
must make source changes retryable with stable IDs, explicit sync watermarks and reconciliation;
do not pretend two stores share an atomic transaction. Index lag must never admit revoked or
superseded facts: validate against the authority or refuse the affected query.

Shadow-query against existing selective loading before routing user retrieval. Compare results,
citations, access decisions, latency and prompt budget. Failure disables the index without
data loss. Any fallback is only to an existing authorized local path and is visible to users.

### Stage C — Optional authoritative backend

Entry: USTE R3 lifecycle/migration/recovery requirements and AgentMage's accepted cutover
decision, new threat review and appropriate gates. Production qualification additionally
requires USTE R4 and AgentMage's own release evidence; neither substitutes for the other.

Export approved scoped data into encrypted staging; verify counts, IDs, decision/source
digests, relationships, retained history and deletion epoch. Quiesce writes or replay a
verified bounded change tail. Test on a new store, compare semantic state, then switch one
authority pointer only after verification. Never silently dual-write two authorities.
Markdown becomes a projection if that is the approved design; hand edits remain proposals.

Before cutover, rollback disables the derived index. After authoritative writes, rollback
requires a tested lossless reverse migration or a declared non-reversible boundary. Old
backups cannot resurrect deleted memories. Preserve access revocation while physical cleanup
or backup expiry is pending. Reconcile keys, copies, holds and minimum audit receipts explicitly.

### Stage D — Optional spatial and simulation memory

Admit explicit world/location sources and privacy rules. Test mixed document/location queries
and source-backed trajectories. Only then enable authorized hypothetical simulations under
resource limits. No location inference, continuous tracking or physical action is implied by
turning on ordinary semantic memory. Navigation here means querying a supplied graph, not
granting AgentMage permission to drive a robot or operate a trading account.

## Proposed work packages

All rows are open planning items. `UM-*` identifiers are local to this proposal, not registered
AgentMage story IDs. Map them into existing incomplete owners before execution; preserve
existing completed evidence and the first-release critical path.

| Status / ID | Work package | Depends on | Required completion evidence |
|---|---|---|---|
| [ ] UM-01 | Register backend decision, scope owners and threat model | Existing roadmap admission | Accepted decision, mapped authoritative tasks, no status inflation |
| [ ] UM-02 | Define schema/identity/approval/lifecycle mapping and version policy | UM-01 | Exact round-trip fixtures for all memory types, conflicts and forbidden inputs |
| [ ] UM-03 | Fake adapter and bounded query/prompt contract | UM-02 | Positive/negative/error/boundary/no-side-effect tests; no context overflow |
| [ ] UM-04 | Pin USTE and design authenticated service/key/process lifecycle | UM-03; verified USTE M1/T-68 or qualified superset | Dependency/license/unsafe review, offline launch/lock/crash/version refusal tests |
| [ ] UM-05 | Derived index ingestion, outbox/checkpoints and reconciliation | UM-04 | No loss/duplicates after retry; revocation and stale watermark denial |
| [ ] UM-06 | Cited retrieval, source resolution and shadow comparison | UM-05 | Correct scope/citations, conflict visibility, relevance baseline and measured latency |
| [ ] UM-07 | Retention/purge/backup/restore across both systems | UM-06; USTE R3 | Dependency deletion, no stale resurrection, exact receipts and key-loss behavior |
| [ ] UM-08 | Optional authoritative migration and rollback rehearsal | UM-07; accepted cutover decision | Counts/digests/history parity; interrupted migration and post-write rollback tests |
| [ ] UM-09 | Optional spatial/motion/physics memory | UM-06; relevant USTE spatial/physics gates | Opt-in location privacy, time semantics and observed/predicted separation |
| [ ] UM-10 | Integrated security/performance/release evidence | UM-07; UM-08/09 only if shipped | Current source-bound reports, independent required reviews and actual installed trials |

## Acceptance and measurements

Stage B uses synthetic data: approve a fact from a document; restart; retrieve it with an exact citation;
correct it with a new approved version; answer current and historical questions correctly;
retain a contradiction visibly; deny a second workspace; revoke a source; restart and refuse
stale indexed results. Rebuild from the current authority without losing source records.
Stage C/UM-07 must additionally prove purge of source and derivatives and rejection of stale
restore; a Stage B read-exclusion receipt must never claim physical erasure.
Add malicious document instructions, disconnected service, disk
full, malformed responses, wrong keys, changed source digests and budget exhaustion.

For the optional spatial slice, attach a source to a moving object; query its observed location
at a specified time; display UTC/local time; run a clearly hypothetical branch; revoke the
source and prove that related positions, routes and predictions no longer leak it.

Before user-facing enablement, fix a corpus, hardware/model profile and numeric budgets for
p50/p95/p99 retrieval, restart recovery, sustained ingestion, RSS, prompt occupancy, citation
accuracy, relevance and deletion completion. Run with encryption, authorization and durability
enabled alongside local inference. Compare with existing retrieval; make no unsupported token
savings, trading performance, security certification or full-memory-completeness claim.

## Documentation-only change boundary

This proposal changes no Rust code, dependency, schema, accepted Decision, task checkbox,
status model or runtime configuration. Implementation must batch capability changes and then
renew affected evidence once under [AGENTS.md](../../AGENTS.md); it must not weaken bindings
or substitute an illustrative demo for release evidence.
