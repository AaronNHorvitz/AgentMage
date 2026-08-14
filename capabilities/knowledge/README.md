# Knowledge Capability

This capability defines the authority-free human knowledge domain used by the plain-folder and
Obsidian adapters. User-owned Markdown is canonical. Implementations may derive disposable indexes
and exports, but those representations cannot authorize, replace, or silently modify a canonical
record.

The capability exposes record inspection and write-preview contracts only. It has no filesystem,
platform, operational-store, grant, shell, model-runtime, or network dependency.
