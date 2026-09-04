# Safe Additional File Parsers

## Boundary

Sprint 66 adds six caller-supplied, effect-free parser surfaces: saved HTML, bounded XML, a closed
YAML configuration subset, Jupyter notebook JSON, structured logs, and ZIP-compatible archive
inventory. Every result binds an exact source digest, closed format, parser profile, ordered records,
and zero filesystem, network, or execution effects. No parser discovers a path, creates scratch,
loads a URL, expands an entity, invokes a constructor, executes a notebook cell, evaluates a log, or
opens archive member payloads.

The default profile admits at most 8 MiB of source, 100,000 emitted items, 64 KiB per textual line or
cell source, 8,192 archive entries, and 128 MiB of declared archive expansion. Callers may narrow
those ceilings but cannot broaden them beyond the compiled maximums.

## Saved HTML And XML

The markup scanner returns exact half-open byte ranges for inert text between tags. Saved HTML
retains canonical unsupported codes for scripts, frames, JavaScript URLs, and external references;
none is followed or run. XML rejects document types and entity declarations, preventing external or
recursive entity expansion. Entities are never expanded implicitly.

## Notebooks

The notebook parser accepts version 4 JSON and the closed Markdown, code, and raw cell kinds. It
retains cell order, bounded source text, exact source ranges, source and metadata hashes, declared
execution counts, and content-addressed output summaries with canonical MIME keys. Code and outputs
remain inert; output images or HTML are not rendered and remote references are not resolved.

## YAML

The YAML parser accepts indentation-based mappings with scalar values. It rejects executable tags,
explicit tagged values, aliases, anchors, and merge keys. Key paths containing password, secret,
token, API-key, private-key, or credential identities retain only the exact value hash and a redacted
state. The raw secret is absent from the result.

## Structured Logs

The log parser preserves zero-based event sequence and exact line ranges for JSON Lines objects,
timestamp-prefixed text, stack-trace continuations, and plain text. JSON scalar lines fail instead of
silently changing the record shape. Timestamps are observations, not scheduling or execution
authority.

## Archive Inventory

Archive inventory accepts stored or deflated ZIP metadata only. It checks encryption, member paths,
duplicates, entry counts, declared expansion totals, compression ratios, and integer bounds without
reading member content. Nested archive names are inventoried and quarantined; they are never opened.
No scratch directory or persistence effect is created.

## Deferred Formats

Apple Pages, binary office, encrypted archive, proprietary notebook, rich-media project, and
embedded-executable formats remain disabled under `format.separate-promotion-required`. Contract or
synthetic evidence cannot promote them. Native parity, independent fuzzing, transcription metrics,
and later common-receipt integration remain separate evidence obligations.
