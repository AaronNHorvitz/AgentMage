# Ordered Runtime Lineage and Inspector

Status: Story 21.4 local runtime contract

## Canonical lineage

The canonical `RuntimeEvent` chain is the only execution-lineage authority. It now represents
capture, parse, context, route, model, proposal, policy, approval, tool, effect, verification,
recovery, and terminal facts. `RouteSelected`, `ProposalObserved`, and `EffectObserved` close the
three earlier gaps. They are correctness events, use stable identities and SHA-256 digests, and
must occur in their legal turn, model, and attempt positions. They inherit the run, session, task,
policy, correlation, causation, sequence, timestamp, and previous-digest bindings checked for every
event.

No lineage field accepts prompt text, token text, hidden reasoning, credentials, endpoint values,
paths, arbitrary rationale, or tool output. Content remains in separately classified immutable
artifacts and appears only through a bounded `RuntimePayloadReference`. Route records disclose the
closed endpoint privacy class, not its address. Diagnostics disclose a closed safe-next-action and
a referenced record, not free-form model advice.

The shared runtime publisher and journal remain responsible for bounded queues, nonblocking
subscribers, slow-client eviction, resumable digest-bound cursors, cancellation durability, and
ordered SQLCipher-backed correctness history. The inspector adds no bus, queue, persistence path,
or execution authority.

## Read-only inspector

`build_execution_inspector` first verifies every canonical event and then reconstructs one bounded
view of at most 4,096 events. The view exposes the current turn and tool operation, latest route and
endpoint class, ordered proposal, approval, tool, effect, and verification identities, content-free
blocker codes, distinct payload references, terminal diagnosis, closed safe next action, exact
resume cursor, and one attributable content-free fact per event.

The resulting view is deterministic and digest-bound. `verify_execution_inspector` always rebuilds
it from the canonical event chain; a stale or modified materialized view returns a digest mismatch.
Consequently a UI, restart path, or client cache can render the inspector but cannot change runtime
state, invent a blocker, authorize a tool, or override journal truth.

## Identity-bound performance qualification

`RuntimePerformanceObservation` requires one exact fixture identity and fields for capture,
parsing, retrieval, queue, first-event, first-token, generation, tool, verification, and total
latency; event throughput; process memory; optional accelerator memory; disk growth; process count;
artifact bytes; and peak event backlog. `RuntimePerformanceBudget` supplies a threshold for every
field. Qualification is deterministic, content-free, digest-bound, and non-authoritative. Every
exceeded field is reported; malformed accelerator applicability or impossible first-event/token
timing fails closed.

The retained local fixture reconstructs the complete 26-event lineage 100 times and records total
time, throughput, p50/p95/p99 reconstruction latency, and serialized inspector bytes. The remaining
resource fields are exact bounded fixture facts: no accelerator, no disk write, one process,
in-memory event-envelope footprint, serialized artifact size, and the complete fixture as the peak
backlog. Zero-valued capture, parsing, queue, generation, and tool timings mean those stages are not
performed by this inspector-only fixture; they are explicit observations rather than inferred
product latency. This evidence qualifies the local inspector contract only, not installed-product
or supported-platform performance.

## Gate scope

The Story 21.4 evidence reuses the canonical journal's saturation, slow-client, cursor-drift,
delayed-store cancellation, and restart tests. It also retains closed-schema checks, redaction
scans, replay-equivalent inspector results, all-threshold failure tests, strict Clippy, and the
measured local distribution.

The roadmap's reusable-gate ownership table assigns full `RV-56` (Engineering Capability Registry)
to Story 95.3. Story 21.4 therefore does not claim that unrelated future gate. Its local
lineage/inspector/performance slices are complete; installed-runtime measurements, supported-platform
campaigns, and independent qualification remain later or external release evidence.
