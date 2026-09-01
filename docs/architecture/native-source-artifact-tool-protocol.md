# Native Source-Artifact Tool Protocol

Status: Story 16.2 protocol, extended by the Story 22.3 production text/log adapter

## Boundary

The source-artifact tool family exposes seven exact `1.0.0` identities: `artifact.list`,
`artifact.metadata`, `artifact.read`, `artifact.range`, `artifact.sections`, and
`artifact.search`, plus `artifact.get_log_errors`. Each definition is registered in the common native `ToolRegistry`, declares only
`WorkspaceRead`, requires one single-use exact projection grant, and binds the same closed input and
output schema bytes. Registration supplies validation and discovery, not an executor or authority.

The protocol accepts stable source, section, coordinate range, freshness, output, call, and limit
identities. Byte, line, page, sheet, cell, and structural-section locators remain distinct tagged
types. Every non-list operation requires the exact current manifest digest. List results retain
restricted manifest metadata instead of silently omitting a known source, but restricted or
confidential content is never returned.

## Limits and terminal states

One call is bounded to 1,000 items, 16 MiB of inspected source, 1,000,000 coordinate units, 2 MiB of
serialized items, 15 seconds, 32 MiB of declared working memory, 1,000 logical tasks, recursion depth
eight, and a 64 KiB request. Callers may select smaller positive ceilings. Malformed, duplicate-key,
unknown-field, unsupported-version, unauthorized, over-budget, and repeated calls fail before fake
execution. A launched call ends in exactly one receipt with a visible succeeded, no-result, denied,
stale, restricted, unsupported, out-of-range, cancelled, timed-out, failed, or truncated state.

Receipts bind tool, call, source, provenance, freshness, output identity, limits, and outcome. They
also state that no parser was launched, no network was accessed, and no workspace mutation occurred.
Large inline results are truncated visibly; when the caller's byte ceiling permits, a content-bound
large-result reference replaces a result that cannot fit.

## Fake backend and client parity

`FakeArtifactBackend` owns deterministic in-memory manifest, classification, extraction-state,
section, provenance, freshness, redaction, and fragment projections. It implements byte, line, page,
sheet, cell, section, and lexical-search behavior without a source store, parser, filesystem path,
model call, MCP channel, process, or network adapter. Results are marked
`production_execution: false`; they are protocol-review evidence, not production extraction.

The common registry feeds the same inert definitions to native Chat, CLI, headless, and coding
profiles. Protocol tests normalize caller-selected call/output identities and require identical
correctness data across reruns. Client wrappers cannot add an alternate source or parser channel.

## Production text/log adapter and reserved extensions

Story 22.3 supplies `NativeSourceArtifactBackend`, a read-only projection from the runtime-owned
prepared-source service into this same dispatcher. Production receipts set
`production_execution: true`; parsing still occurs before tool dispatch, and parser, network, and
workspace-effect flags remain false. `artifact.get_log_errors` is now registered because its
deterministic text/log cluster goldens pass. `ArtifactExtractorExtensions` continues to reserve only
the structured-document page and sheet ports. `artifact.get_page` and `artifact.get_sheet` remain
absent pending their owning format stories.

## Current evidence boundary

Local evidence covers the closed catalog, schema-bound common registration, deterministic fake
results, all coordinate types, freshness and classification refusals, redaction, output bounds,
large-result references, replay, cancellation, timeout, crash, and parser/network/workspace canaries.
The original Story 16.2 retained evidence remains a protocol/fake-backend claim. Story 22.3 evidence
separately binds production text/log preparation and dispatch. Neither evidence set claims an
installed native worker, structured-document extraction, or cross-platform sandbox campaign.
