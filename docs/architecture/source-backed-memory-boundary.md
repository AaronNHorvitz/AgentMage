# Source-Backed Memory Boundary

## Purpose

Sprint 31 separates temporary context, source-backed durable memory, human decisions, and
human-readable files. A model may propose a candidate, but cannot remember, correct, supersede,
decay, hold, delete, or write a memory item on its own authority.

```mermaid
flowchart LR
    S["Bounded source evidence"] --> C["Memory candidate"]
    C --> P["Secret, sensitivity, confidence, retention, and contradiction policy"]
    P -->|"temporary"| W["Bounded WORKING.md preview"]
    P -->|"unresolved, contradictory, prohibited"| B["Blocked with visible state"]
    P -->|"eligible"| H["Exact human approve or reject decision"]
    H --> L["Current and historical memory catalog"]
    L --> M["MEMORY.md and topic previews"]
    L --> R["Selective scoped loading"]
    W --> X["Compaction preview"]
    X --> C
    L --> T["Edit, supersede, correct, decay, hold, expire, delete"]
    T --> L
    L --> E["Versioned authenticated portable export"]
    E --> I["Closed-schema verified import into a new catalog"]
    M --> F["Authority-free host filesystem plan"]
    F --> KER["Kernel preview, approval, one-use grant"]
    KER --> BAK["Atomic platform backup and restore"]
    KER --> CON["Exact simultaneous-edit conflict bundle"]
    E --> MIG["Digest-bound controlled export plan"]
    K["Credential and random adapters"] --> E
```

## Candidate Policy

Memory identities are stable and independent of paths. The closed type taxonomy is working,
episodic, semantic, procedural, and preference. Every candidate carries exact workspace,
project, and conversation scope; bounded content; optional normalized fact key; tags and links;
content-addressed evidence; sensitivity; confidence; creation and optional expiry; and visible
inferred-sensitive/model-requested markers.

Policy produces temporary context, durable fact, preference, procedure, episode, unresolved claim,
contradiction, or prohibited state. Secrets, restricted data, and inferred-sensitive content are
prohibited from Markdown memory. Source-free or low-confidence assertions remain unresolved.
Different current content under the same scoped fact key remains a contradiction and cannot be
silently promoted. Working context cannot be durably promoted as-is.

Eligible durable candidates still require an exact human approve/reject decision bound to the
candidate digest and decision-evidence digest. Model-requested promotion changes no policy field.
Rejected candidates retain no proposed content in the resulting tombstone.

## Working and Durable State

`WorkingMemory` enforces explicit entry and UTF-8 byte ceilings and can render its complete set as
a no-write `WORKING.md` preview. Compaction takes explicit source-entry/proposed-candidate pairs,
inherits exact scope/evidence/sensitivity, cannot raise confidence, and emits only ordinary
candidates that must pass policy and human review. It neither creates durable state nor clears the
working set.

The in-memory catalog retains current and historical items under atomic clone/validate/publish
transitions. Edit, correction, and supersession require an independently approved replacement
identity with the same scope and fact key. The original becomes `Superseded` and links to the
replacement. Confidence decay can only lower confidence. Holds resist policy expiry. Deletion
removes content, tags, and links while retaining a content-free tombstone and evidence/decision
identity. Failed transitions publish no revision.

## Human-Readable Previews and Retrieval

The catalog deterministically previews a compact linked `MEMORY.md` plus one portable
`Memory/memory-*.md` topic file per identity. Topic frontmatter exposes type, status, confidence,
dates, tags, links, and candidate/decision digests. Current and historical sections remain
separate. Preview objects have fixed false write markers and no apply method.

Selective loading requires an exact workspace and can further require project, conversation,
memory type, any-match tags, links, evidence IDs, and all-match literal relevance terms. Only
approved/held items enter by default. Stable ranking uses visible match reasons, confidence, last
verification, and identity under result and byte budgets. Every hit includes source evidence,
sensitivity, status, last verification, and supersession. The result explicitly records that no
cross-workspace content entered.

## Portable Encrypted Export

The portable boundary serializes the complete current and historical catalog into a closed
version-1 envelope and encrypts it in place with XChaCha20-Poly1305. HKDF-SHA-256 derives a
domain-separated file key from a nonzero 256-bit key supplied by a trusted credential adapter and
a fresh 256-bit public salt. The trusted caller also supplies a fresh 192-bit nonce; this pure
capability does not acquire randomness, open a credential store, or retain the key in debug output.

The authenticated header binds format version, salt, nonce, exact plaintext length, and plaintext
digest. The envelope binds catalog revision, a full catalog digest, and every item field including
scope, content, tags, links, evidence, sensitivity, lifecycle dates, decision identities, and
supersession. Import verifies length, version, AEAD, plaintext digest, closed JSON schema, portable
identities, link closure, lifecycle states, and recomputed catalog digest before constructing a new
in-memory catalog. Wrong keys, changed bytes, truncation, version drift, duplicate identities,
invalid transitions, and digest drift fail without a partial catalog.

Export refuses restricted data, credential candidates, absolute host-path evidence, nonportable
identities, and invalid evidence metadata. Ciphertext proposals and import receipts expose no
filesystem apply method and retain fixed false machine-path, credential, and file-write markers.
Fresh salt and nonce generation, key storage, and durable file application remain responsibilities
of later trusted platform adapters.

## Installed File Recovery

The host planner accepts only a complete no-write `MemoryMarkdownBundle` whose index, ordered topic
paths, content hashes, and bundle digest recompute exactly. It returns only the existing controlled
filesystem drafts for publication, exact last-good copies, complete-file restoration, conflict
preservation, and opaque encrypted export. The plan owns no file, model, process, or network handle
and records that no automatic decision occurred.

Every update supplies the bundle identity the caller observed. A mismatch can produce only a
complete proposal under a held `Conflicts` destination; it cannot target the current projection.
Last-good copies and restoration remain distinct controlled transactions over exact held preimages.
The existing kernel renders and approves every operation, consumes a one-use grant, invokes the
atomic platform driver, verifies receipts, and owns crash recovery. Portable encrypted exports are
bounded opaque bytes under an exact digest. Path mismatch, wrong held-object kind, unrelated work,
digest drift, and oversized files fail before authority exists.

## Open Boundary

This sprint slice now composes approved `MEMORY.md` topic-bundle publication, last-good backup and
recovery, simultaneous-edit conflict preservation, and exact encrypted-export transfer through the
existing kernel/platform transaction boundary. `WORKING.md` application, credential-store
integration, and trusted entropy acquisition are not added. Upstream Sprint 30 closure and renewal
of the source-bound aggregate under the inherited strict-local host prerequisite remain required
before Sprint 31 can pass.
