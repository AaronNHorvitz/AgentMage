# Native Source-Artifact Tool Protocol

Status: Story 16.2 local protocol and fake-backend contract

## Boundary

The source-artifact tool family exposes six exact `1.0.0` identities: `artifact.list`,
`artifact.metadata`, `artifact.read`, `artifact.range`, `artifact.sections`, and
`artifact.search`. Each definition is registered in the common native `ToolRegistry`, declares only
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

## Reserved extensions

`ArtifactExtractorExtensions` reserves typed ports for page, sheet, and extraction-error retrieval,
including exact source, freshness, coordinate, and limits. The associated identities
`artifact.get_page`, `artifact.get_sheet`, and `artifact.get_log_errors` are intentionally absent
from `ArtifactToolKind::ALL` and the native registry. A later owning story must supply an admitted
extractor and native implementation before registering any of them.

## Current evidence boundary

Local evidence covers the closed catalog, schema-bound common registration, deterministic fake
results, all coordinate types, freshness and classification refusals, redaction, output bounds,
large-result references, replay, cancellation, timeout, crash, and parser/network/workspace canaries.
This does not claim a production artifact store or parser, installed native worker execution, or
cross-platform sandbox evidence. Those remain with the later native source-artifact implementation
and release campaigns.
