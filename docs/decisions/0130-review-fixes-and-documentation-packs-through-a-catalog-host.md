# Decision 0130: Review Fixes for Batches 15 and 16, and Documentation Packs Through a Catalog Host

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0124, 0126, 0128 and 0129; current owner restart |
| Scope | Findings F1 to F3 and notes N2 and N4 of the independent review of `bc1e2d39`; AMR-05.9.5 split into AMR-05.9.5.1 to AMR-05.9.5.3; AMR-05.9.5.1 |

## Findings

An independent read-only review of `e5f34910..bc1e2d39` passed with three low
findings and nine notes.

- F1: the development CLI kept a route receipt whose selected route was any
  strict-local route. It did not check that the route was the profile the
  person selected, that the candidate digest was that profile's canonical
  digest, or that the policy digest was the development host's fixed policy.
- F2: two rules of Decision 0129 had no test that failed when either was
  broken. A run started from an event cursor after a restart must mark its
  stored job control chain incomplete. A session dropped while its run is not
  terminal, for example a suspended run, must leave its chains open.
- F3: the rule that only the exact word `yes` publishes a support bundle sat in
  a closure no test reached.
- N2: Decision 0128 says the support bundle's model observation is identified by
  the profile digest. The code identifies it by the SHA-256 of the profile
  identity string.
- N4: a run recorded by a host before schema 22 has no stored chains. A later
  build that resumes it after a restart fails composition when it attaches to
  the missing chain.

The review's other notes need no change. Its F3 reasoning says the platform
reader does not trim the line. The reader does remove the terminator and
surrounding whitespace, as the run approvals rely on. The exact-word rule
applies to that trimmed line.

AMR-05.9.5 joins three different integrations: documentation packs, a
language server adapter and recipe plans. They have different owners and
prerequisites, so one row cannot close them together.

The development host builds its repository composition before it serves IPC.
That composition needs a trusted native Git executable. Reading an ended run's
stored histories (Decision 0129) needs none of it, yet it is refused wherever
native Git is untrusted. Documentation packs need no workspace composition
either.

## Decision

### Review fixes

F1. The CLI also requires that the receipt's selected route is the run's
profile identity. The selected audit's candidate digest must be the SHA-256 of
that profile's canonical encoding. The receipt's policy digest must be the
development host's fixed local-only policy. The dropped-receipt test adds a
resealed receipt for each: another route identity, another candidate digest
and another policy digest.

F2. The suspension fixture's factory hands over the job ledgers and run action
histories of the store it opens for each composition, as the development
factory does. Three service tests use it:

- The applied suspension test asserts that each composition opened the store.
  It records a request after the outcome through the continuation's own
  handle. After release, the stored job control chain is closed and complete
  and holds every decided request.
- A run suspended when its host ends keeps its stored chain open and complete.
- A run whose host ended while it ran is started again from its checkpoint by a
  new service. The stored chain is then marked incomplete, the run declares no
  job control history, and the chain is closed after release.

F3. The answer to a support bundle preview is a separate function of the read
line. A test shows that `yes` confirms and that cancellation cancels. Every
other spelling, a prefix, a repetition, a lone letter, an empty line, end of
input, a too-long line and a read failure decline.

N2. This decision corrects the wording: the model observation is identified by
the SHA-256 of the profile identity. Nothing changes in code.

N4. A run resumed after a restart whose chain the ended host never stored now
creates the chain marked incomplete, so it never claims entries it missed. The
route chain is stored before each model is built, so after a restart it
attaches without marking (a new start kind, after a restart with every entry
stored first); when it is missing it too is created incomplete. A persisted
recorder test covers both.

### AMR-05.9.5 split

AMR-05.9.5 is split into these rows, which must all close before it does:

| Row | Integration through the coding host and development CLI |
| --- | --- |
| AMR-05.9.5.1 | Import, inspect, refresh, delete and search documentation packs through a host that owns them in its operational store |
| AMR-05.9.5.2 | Language server observations through a host adapter under ordinary tool authority |
| AMR-05.9.5.3 | Recipe plans instantiated against the workspace's validation templates and handed to the ordinary change approval |

### AMR-05.9.5.1: documentation packs through a catalog host

Catalog host. The development host gains a second closed operation, the
catalog host. The CLI launches it with the same state root, disposable root
and marked workspace, and it validates the same activation. It activates the
platform adapter, but it builds no repository composition, model, tool or
runtime. It serves the authenticated IPC session of its parent like the
development host. It answers only catalog operations: documentation packs and
the ended-run read of Decision 0129. Every run operation is refused. It opens
the operational store for each operation and closes it afterwards.

The CLI's `--ended-run` now goes through the catalog host, so it no longer
needs the repository composition. A catalog operation takes no scenario, model,
support bundle or run option. The development host's own ended-run read is
removed, so the read has one owner.

Store. Operational store migration 23 adds four tables for one documentation
catalog:

- a head with its revision, the last day a change used and the SHA-256 of the
  catalog state;
- one row per kept version, holding the manifest bytes, the manifest digest,
  the day it was superseded (none while it is current) and how many deletion
  rows existed when it was stored;
- one row per file of a kept version, holding its path, digest and bytes;
- one row per deletion, holding the pack, version, manifest digest, day and
  reason.

Triggers enforce these rules:

- A version may change only from current to superseded on a given day.
- A file never changes.
- A version is stored only after every deletion so far, and it and its files
  are deleted only with a later deletion row naming its pack, version and
  manifest digest.
- Deletions are append-only, in sequence.
- The head is never deleted, its revision moves forward by exactly one, and its
  last day never goes backwards.

The kernel store module stays independent of the knowledge crate. It stores
opaque manifests and file bytes and checks every file digest. Its operations
are:

- `load`, which reads the whole state, checks every file against its digest and
  checks the state against the head digest;
- `commit`, which takes the revision it read and the complete next state.

The head digest covers each version's key, manifest digest, the digest of its
manifest bytes, its supersession day, its deletion count, and each file's path,
digest and length, and every deletion row.

The commit inserts new versions, marks superseded ones, deletes versions with
their deletion rows, appends deletions and advances the head, all in one
immediate transaction. It refuses any other change: a changed or rewritten
version, a version added already superseded, a deletion without its row, a
deletion row for a version that stays, an edited or reordered deletion, or a
day that goes backwards. Every store open checks the metadata against the head
without reading file bytes, and every load also reads and checks each file, so
a tampered row poisons the store.

The knowledge crate exports a catalog's state and restores a catalog from one.
The restore re-verifies every manifest seal, file length, digest and text
rule, the version order, that at most one version is current and that it is
the newest, every date, and the bounds. The host restores the catalog, applies
one operation and commits the next state.

Policy. Each operation first applies retention for the host's current UTC
date. Superseded versions are kept for 30 days and then deleted. Retention and
person deletions are reported. The next state is committed only when retention
or the operation changed the catalog, so reading changes nothing. An import
accepts only the licenses the person names for that import with
`--allow-license`, at most eight. A date earlier than the catalog's last change
is refused, so a clock that moved back changes nothing.

Transport. IPC wire 12 adds one documentation pack request with these
operations:

- `import-begin` takes the sealed manifest, the licenses allowed and an
  optional version to refresh. The host verifies the manifest and stages the
  import.
- `import-chunk` takes text at an offset of one listed file, at most 1 MiB, so
  that escaping keeps it inside one frame. It must continue the file exactly and
  never pass its length; a refused chunk discards the staged import.
- `import-commit` checks every file and imports the version.
- `list`, `inspect`, `delete` and `search`.

One staged import exists at a time, and a new begin or the end of the session
discards it. A refusal is a closed content-free code: the component's codes,
plus the staging and transport codes.

CLI. These replace the objective, are exclusive with each other and with
`--ended-run`, and launch only the catalog host:

- `--doc-pack-import DIRECTORY --allow-license ID...`, with
  `[--refresh-version MAJOR.MINOR.PATCH]`. The CLI reads `manifest.json` and
  each listed file from the directory, refusing links, other file types and
  files whose length differs. It then sends the manifest and every file in
  chunks.
- `--doc-pack-list`.
- `--doc-pack-inspect PACK_ID`.
- `--doc-pack-delete PACK_ID[@MAJOR.MINOR.PATCH]`.
- `--doc-pack-search TERMS`, with `[--doc-pack PACK_ID]` and
  `[--include-history]`. Every term must match. At most 20 hits are returned,
  with their pack, version, path, lines, the bounded cited text escaped for the
  terminal, and the citation digest.

Results go to standard output as text lines or JSON rows, and refusals go to
standard error. A refused operation exits with the closed class of its
refusal: malformed input 2, a policy refusal or an unknown pack 4, an
unavailable store or clock 5, a bound 7. The CLI keeps the staged answer and
each chunk answer only when they acknowledge exactly what it sent, and an
import receipt only when it names the manifest the CLI sent, recomputes and
used no network. A cancellation between requests stops sending, and the host
discards the staged import when the session ends.

The development harness gains `doc-pack` for these operations and
`doc-pack-sample`, which writes a small sealed pack of synthetic text for
trying them; `ended-run` no longer passes a scenario.

## Limits

The search is the deterministic knowledge retrieval over the packs, run through
the CLI. No coding run's model sees a pack yet. The packs come from files the
person already holds; nothing is downloaded. The store's key authenticates
writers. Within that trust boundary the head digest detects a torn or
partial change, not a rewrite by a key holder. The catalog host is a development
operation of the disposable activation, not a released service. Language server
and recipe integration remain AMR-05.9.5.2 and AMR-05.9.5.3. Native actual-process
proof of the coding host remains AMR-05.10, and independent review remains open.

## Consequences

- `capabilities/knowledge`: catalog state export and restore, with tests, and
  closed parsing of the wire types.
- `kernel/engine`: migration `0023-documentation-packs.sql`, the
  `doc_pack_store` module with tests, `SCHEMA_VERSION` 23, verification at
  open, the schema 23 fixture and migration tests. The Sprint 11 store
  producers and their documents name schema 23.
- `platforms/linux`: launching the catalog host.
- `shells/host`: the catalog host and its service, the documentation pack
  owner, wire 12, the CLI options and rendering, and the review fixes.
- TASKS.md: AMR-05.9.5 is split, and AMR-05.9.5.1 is recorded with its evidence
  when the batch is verified.
