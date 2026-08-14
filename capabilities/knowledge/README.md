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
network dependency. Obsidian indexing, watching, and every canonical note write remain outside the
Sprint 27 parser boundary.
