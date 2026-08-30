# Story 11.2 Source Content-Deduplication Evidence

This record covers Sub-task 11.2.1.2 only. Equal source bytes reuse the existing migration-0007
`runtime_payloads` content-addressed object while each logical source owns a distinct
`runtime_artifacts` reference and source manifest.

## Identity boundary

Migration `0013` adds no payload table. It prevents two source manifests or retention rows from
claiming the same physical artifact identity, binds origin and reference request/authority values to
the manifest, binds provenance classification and freshness to that manifest, and binds persisted
retention to the same source request, authority, physical artifact, payload digest, and byte size.
Origin, reference, manifest, provenance, and active persisted-retention identity fields cannot be
rewritten in place.

## Focused proof

The runtime publication test proves equal bytes occupy one content-addressed object while two
logical artifact references retain independent ownership and reference state. The encrypted
operational-store test layers two sources with distinct origin, classification, authority,
freshness, and retention identities over that one payload, then rejects physical-reference reuse
and every tested identity rewrite. Schema-1 upgrade history, seeded migration crash recovery, strict
Clippy, and evidence mutation tests also pass.

## Deliberately open scope

This increment does not claim refresh, transitive invalidation, expiry, deletion, hold, or
garbage-collection transactions. Those remain assigned to Sub-task 11.2.1.3. Typed source
publication, the remaining Story 11.2 work, Sprint 11 completion, platform acceptance, and release
readiness are not claimed.
