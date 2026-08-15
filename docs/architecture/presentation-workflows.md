# Presentation Workflows

## Boundary

Sprint 64 admits two local, effect-free paths:

1. bounded direct inspection of caller-authorized PowerPoint Open XML bytes; and
2. deterministic in-memory generation or full-regeneration editing of a closed slide grammar.

Neither path renders slides, follows links, resolves remote media, runs macros, opens embedded
objects, invokes an office application, writes a file, or expands the caller's workspace grant.
The source bytes remain authoritative and every proposed output is hash-bound.

## Inspection

`inspect_pptx` validates ZIP paths, compression and expansion limits, required relationships,
slide ordering, XML structure, and a closed parser profile. It retains:

- ordered slides and objects with package-part and object hashes;
- speaker-note text;
- inert links with exact target hashes and owning object identities;
- package-local images with byte counts, hashes, and owning object identities;
- captions, alternative text, and layout identities; and
- content-minimized findings for macros, embedded executable content, external media, active
  actions, and unsupported content.

Any relationship escape, malformed required part, duplicate object identity, encrypted entry, or
resource-limit breach fails closed. Blocking content makes the inspection unsafe for reuse.

## Generation

The generated slide grammar contains inert text, bullet lists, tables, integer column charts, and
small node-edge diagrams. Tables, charts, and diagrams carry SHA-256 identities recomputed from
their exact ordered data. Callers provide a stable deck identity, workspace-relative `.pptx` path,
fixed UTC metadata timestamp, stable slide identities, and complete slide specifications.

Generation uses fixed 16:9 geometry and deterministic ZIP metadata. The package contains a basic
master, blank layout, theme, speaker notes, tables, charts, and fixed document properties. It is
reopened through the same bounded inspector before being returned. The package is always a
proposal: persistence remains a separately authorized effect.

## Editing And Preview

Editing accepts only strictly ordered complete slide replacements bound to the exact source byte
hash. It performs full deterministic regeneration and verifies that every untouched slide keeps
the same structural preview identity. Source bytes are never modified.

Each slide preview records fixed geometry, ordered object kind, exact content hash, and optional
data-source hash. The preview is structural evidence, not a rendered image. Every preview therefore
sets `native_render_required` to `true`; a renderer or accessibility review cannot be inferred from
successful package inspection.

## Runtime Records

The closed runtime records are:

- `presentation-inspection`;
- `generated-presentation`; and
- `edited-presentation`.

Schema semantics recompute package hashes, source/path bindings, slide and object order, data-source
hashes, structural preview hashes, safe-reuse state, edit changes, and no-effect claims.

## Current Limits

Legacy binary presentation formats, encrypted packages, arbitrary layouts, animation, audio/video,
SmartArt fidelity, external media retrieval, embedded object execution, native rendering, pixel
comparison, native accessibility inspection, and cross-platform office-suite acceptance are not
admitted. Sprint 65 owns image redaction and visual-diff contracts. Native pixel and accessibility
evidence remains a blocker for the complete Sprint 64 gate.
