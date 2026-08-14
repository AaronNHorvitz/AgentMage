# Knowledge Capability

This capability defines the authority-free human knowledge domain used by the plain-folder and
Obsidian adapters. User-owned Markdown is canonical. Implementations may derive disposable indexes
and exports, but those representations cannot authorize, replace, or silently modify a canonical
record.

The capability exposes record inspection and write-preview contracts only. Its Obsidian boundary
admits a content-free selected local vault scope, consumes immutable authorized Markdown
snapshots, parses bounded frontmatter and note structure, and derives a deterministic wiki-link
and backlink graph. It does not open a host path, require Obsidian, execute note instructions, or
silently resolve ambiguous links.

The capability has no filesystem, platform, operational-store, grant, shell, model-runtime, or
network dependency. Its disposable in-memory Obsidian index stores source-traceable parsed
elements, links, coverage, conflicts, and verified attachment metadata outside the vault. Atomic
rebuild and exact watcher-event batches can replace only that derived projection. Bounded current
note readers, lexical queries, relationship traversal, stale detection, and per-file section
previews emit content-free receipts and preserve source authority.

The capability does not create an operating-system watcher or open a vault path; a trusted local
adapter must supply observed snapshots and events. Every canonical note write remains outside the
Sprint 27 and Sprint 28 boundaries.

The v0.2 task projection adds closed status and priority values, owner, project, blocker, next
action, deferral, stable dependencies, source links, evidence, deterministic duplicate warnings,
and non-authoritative views. Every transition requires new content-addressed evidence and produces
only an exact canonical-record preview; this crate still exposes no apply method.

Declarative skills are hash-bound UTF-8 packages containing only prompts, schemas, examples, and
templates. Exact manifests retain source, signer or provenance, license, version, compatibility,
purpose, requested read-only scope, precedence, trust state, and complete file identities. Skills
cannot access filesystems, shells, secrets, networks, connectors, approvals, grants, tool
registration, execution, workspace expansion, memory promotion, or writes. Conflicting instruction
keys are visible and omitted from bounded context.

The built-in pack contains Daily Setup, Daily Briefing, Issue Intake, Handoff, Meeting Cleanup,
Repository Learning, Plain-Workspace Steward, and Obsidian Vault Steward. Each workflow returns the
same interface-neutral source and evidence contract for native Chat or a later compatible CLI,
reports zero proposed writes, and leaves all source files unchanged.
