# Markdown Artifacts and Round Trip

## Scope

The Sprint 57 boundary parses and previews exact user-owned Markdown, performs deterministic local
quality review, generates seven evidence-aware artifact classes, and verifies reopened bytes,
supported semantic structure, and deterministic content-free local render signatures. It has no
ambient file, network, shell, delivery, approval, or execution authority.

```mermaid
flowchart LR
  S[Approved source bytes] --> P[Byte-preserving parser]
  P --> Q[Deterministic quality review]
  P --> E[Exact scoped edit preview]
  C[Evidence states and citations] --> G[Local artifact generator]
  Q --> G
  E --> D[Exact patch draft]
  G --> C[Exact create draft]
  D --> R[Separately approved controlled writer]
  C --> R
  R --> O[Reopened bytes]
  P --> V[Round-trip verifier]
  O --> V
  P --> L[Content-free local structure renderer]
  L --> V
  V --> X[Byte semantic and rendered receipt]
```

The registered host coordinator admits all seven hash-bound artifact skills and binds source
quality review, citation-aware generation, an optional exact edit preview, reopened bytes, and
host-computed local render signatures. Sticky cancellation and missing dependencies fail before
content evaluation. Exact generated and edited bytes may enter the controlled-filesystem contract
only as unapproved create or patch drafts bound to held observations; the coordinator creates no
grant, approval, or effect. Native Chat, interactive CLI, JSON, SDK, and ACP carry only digests,
identifiers, a relative output identity, and an edit-presence bit through the same coordinator.
Source bytes, policies, citations, edit details, rendered structures, and held filesystem context
remain host-owned. Generation returns bytes as a proposal. Display links remain non-authoritative.

## Supported Structure

The parser records headings, paragraphs, ordered and unordered lists, task items, tables, fenced
code and fence content, inline links, wiki links, text blocks, frontmatter, LF or CRLF line endings,
and exact source ranges. Unsupported syntax is retained byte-for-byte with a visible fidelity
warning. Exact edits bind the original digest, target element, source range, replacement bytes,
line-ending policy, and sealed preview digest.

Code-fence content and raw-note regions are protected. Unrelated blocks are not normalized.
Comments, whitespace, frontmatter order, and unsupported source remain untouched unless the user
reviews an exact scoped replacement.

## Quality and Adversarial Content

Quality review is deterministic and configured by an exact profile. It reports CommonMark heading
spacing, local links missing from an approved inventory, duplicate headings, malformed tables,
unsupported structure, long sentences or paragraphs, and uppercase acronyms absent from the exact
approved acronym inventory. Unknown acronyms are never expanded by inference.

HTML, scripts, event handlers, dangerous URI schemes, remote image targets, hidden text, comments,
secret canaries, and instruction-like text are source data. They are never fetched or executed.
Dangerous URI, executable HTML, and canary findings block generated disclosure until reviewed.

## Evidence and Completion

Every confirmed, inferred, historical, or disputed generated statement resolves to one or more
exact citation identities. An unknown statement may remain explicitly uncited. Each citation binds
a validated workspace path, one-based inclusive line range, and source digest.

Local round-trip completion requires all three independent checks:

1. Exact source bytes are identical after reopen.
2. Supported semantic element signatures are identical.
3. Host-computed local rendered-block signatures are identical.

The local renderer hashes each ordered parser element's structural kind, heading level, and bounded
label under a domain-separated version. It returns only ordinals, closed kind names, heading levels,
and SHA-256 values. It does not interpret raw HTML, execute content, resolve assets, or launch an
external process.

Any failed dimension produces a stable limitation and prevents local completion. These checks do
do not prove installed product integration, assistive-technology accessibility, cross-platform
behavior, trusted package execution, independent review, or the deferred manual fuzz campaign.
