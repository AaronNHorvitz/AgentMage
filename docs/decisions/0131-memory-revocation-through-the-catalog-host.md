# Decision 0131: Memory Revocation Through the Catalog Host

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0088, 0124, 0126 and 0130; current owner restart |
| Scope | AMR-05.9.4 split into AMR-05.9.4.1 and AMR-05.9.4.2; AMR-05.9.4.1 |

## Findings

AMR-05.9.4 asks for two things through production transport and the CLI: the
revocation of memory items and sources, and the application of signed extension
revocation lists, each with per-scope isolation.

- Memory. The knowledge component has a candidate policy, an explicit person's
  decision that turns an eligible candidate into an item, and a catalog whose
  transitions include revoking one item and revoking, in one transition, every
  item of one workspace that cites a source (Decision 0124). It has an encrypted
  portable form. Nothing in the host creates an item or keeps a catalog.
- Extensions. The revocation list is signed, sequenced and checked against a
  trusted issuer key and the last list accepted. The host has no catalog of
  installed extensions to deactivate, no trusted issuer key and no durable record
  of the last list accepted. Building those is package installation work, which
  this row does not cover.
- Each durable owner so far added its own operational store migration. Every
  migration renews the whole Sprint 11 store evidence wave.

## Decision

### Split

| Row | Integration |
| --- | --- |
| AMR-05.9.4.1 | Memory items and sources revoked through the catalog host and the CLI, with per-workspace isolation |
| AMR-05.9.4.2 | Signed extension revocation lists applied through the host and the CLI. This needs a durable catalog of installed extensions, trusted issuer keys and the last accepted list |

AMR-05.9.4 closes when both close.

### Owner states

Operational store migration 24 adds one table of owner states. Each row holds
an owner's identity, a revision, the owner's state bytes and their SHA-256.
Triggers enforce three rules:

- a first state has revision one;
- each later state raises the revision by exactly one and keeps the identity;
- a state is never deleted.

The kernel store module names its owners in a closed list; the first is the
memory catalog. `load` returns an owner's state, or none before its first commit,
and checks the digest. `commit` takes the revision the owner read and the next
state, and refuses any other revision as stale. Every store open checks every
digest, so a changed state poisons the store. The store does not interpret a
state; its owner decodes and re-verifies it. A later owner adds a name, not a
migration.

### Memory catalog state

The knowledge component encodes a memory catalog in the plaintext of its
portable form. Decoding applies the same rules as an import: closed parsing,
every item valid, portable identifiers, no restricted data or machine paths,
and the catalog digest must match its items.

### Memory through the catalog host

The catalog host keeps one memory catalog for its state root. Each operation
decodes the stored state, applies one transition and commits the next state
under the revision it read. The operations are:

- Remember. The person states one memory: its text, its type (semantic,
  preference, procedural or episodic), a workspace label and a kept
  documentation pack file it cites as `PACK@VERSION:PATH`. The host resolves the
  cited file in its documentation catalog. The evidence names the source
  `doc-pack:PACK:VERSION`, the object `path:` followed by the path's components
  joined by colons, and the file's digest. A path that is not portable is
  refused. The existing policy evaluates the candidate. Only an eligible
  candidate becomes an item, approved by the person's invocation. The decision
  digest is the SHA-256 of the canonical request, and the host's clock gives
  the times. A refused candidate gives the policy's reason code and stores
  nothing.
- List. Every item, or every item of one workspace: identity, type, status,
  workspace, cited sources and times. The text of an item is shown only to the
  person who asked, escaped for the terminal.
- Revoke. One item. Its text is kept for inspection and later deletion, and it is
  never loaded again.
- Revoke source. Every revocable item of one workspace that cites a source, or
  one object of it, in one transition. Items of other workspaces are never
  touched. A source no item cites is refused as not found.
- Delete. One item becomes a tombstone without its text.

Every refusal is a closed content-free code and changes nothing.

Transport. IPC wire 13 adds memory requests and answers. The CLI's
`--memory-remember TEXT` (with `--memory-workspace`, `--memory-cite` and an
optional `--memory-type`), `--memory-list` (with an optional
`--memory-workspace`), `--memory-revoke ID`, `--memory-revoke-source SOURCE`
(with `--memory-workspace` and an optional `--memory-object`) and
`--memory-delete ID` are catalog operations: each launches only the catalog
host, alone. The harness gains `memory`.

## Limits

No coding run reads memory yet. Items are created only by the person's explicit
invocation and cite only documentation pack files. The workspace is a label the
person names, not a held workspace. Extension revocation is AMR-05.9.4.2.
Native proof of the coding host remains AMR-05.10, and independent review
remains open.

## Consequences

- `capabilities/knowledge`: memory catalog state encoding and decoding, with
  tests.
- `kernel/engine`: migration `0024-owner-states.sql`, the `owner_state_store`
  module with tests, `SCHEMA_VERSION` 24, verification at open, the schema 24
  fixture and migration tests. The Sprint 11 store producers and their documents
  name schema 24.
- `shells/host`: the memory owner of the catalog host, wire 13, the CLI options
  and rendering.
- TASKS.md: AMR-05.9.4 is split, and AMR-05.9.4.1 is recorded with its evidence
  when the batch is verified.
