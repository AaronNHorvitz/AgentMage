# Decision 0132: Review Fixes for Batch 17, and Extension Revocation Through the Catalog Host

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0088, 0124, 0130 and 0131; current owner restart |
| Scope | Findings F1 to F3 and notes N3 and N6 of the independent review of `935cfdd8`; AMR-05.9.4.2 |

## Findings

An independent read-only review of `bc1e2d39..935cfdd8` passed with three low
findings and thirteen notes.

- F1: the CLI read a pack file that held a control character other than a line
  feed, carriage return or tab, launched the catalog host and sent the file in
  1 MiB chunks. The host refuses such text at commit, but its JSON encoding is
  up to six bytes per character, so the client's frame bound refused the
  chunk first. The client then exited as a runtime failure, after a host was
  launched for a pack that could never import. The comment that JSON escaping
  "at most doubles" a chunk holds only for text the host accepts.
- F2: the store keeps at most 100,000 deletion records and never prunes them.
  Once the history is full, retention that falls due cannot commit, and every
  operation, reads included, is refused with `doc-pack.store-limit` from then
  on.
- F3: three rules had no test that failed when the rule alone was broken: a
  refused begin discarding an import staged before it, the host's own 1 MiB
  chunk bound, and the restore rule that no version was obtained after the
  last change.
- N3: the store module says its triggers enforce the same transitions as the
  commit. Three rows the commit never writes are admitted by the triggers and
  caught only by the head digest at the next open.
- N6: the commit's own rule that the day never goes backwards was masked: the
  suite's case also broke the deletion-date rule, and a head dated backwards
  alone is refused by a trigger with another error.

The review's other notes need no change.

AMR-05.9.4.2 needs three things the host does not have: a durable catalog of
installed extensions, trusted issuer keys and the last accepted revocation
list. The owner states of Decision 0131 can keep all three as one more owner,
without a migration. The capability package component already verifies a
package (seal, source digest, license, signer, dependency locks,
compatibility and signature), verifies a revocation list against trusted
issuer keys and the last accepted list, and marks revoked catalog entries
inactive. Nothing in the host uses them.

## Decision

### Review fixes

F1. The knowledge component names its text rule once,
`doc_pack_text_allowed`, and checks it at import. The CLI's pack reader
applies the same rule to every file and refuses other text with
`doc-pack.content-invalid` before any host is launched. The chunk bound's
comment now says that escaping at most doubles documentation text. A wire test
shows that the largest chunk of such text, made only of characters JSON
escapes, fits one frame, and that a chunk of other control characters would
not. The pack reader test gains a file with a control character.

F2. Each kept version becomes at most one deletion record, by retention or by
the person. The documentation pack owner therefore refuses an import, with
`doc-pack.store-limit`, unless the deletion records and the kept versions
together, the new version included, stay within the store's deletion bound.
Deletion and retention never change that total, so retention can always
commit and reads always answer. A full history still refuses imports; the
catalog then needs a later decision that compacts records. A test lowers the
owner's bound to three and shows a refused import, retention, a deletion and
reads.

F3. Three tests are added. A refused begin after a staged import and one
accepted chunk leaves nothing staged. A chunk longer than the host's bound,
for a file long enough to take it, is refused. A state whose only version was
obtained a day after the last change is refused at restore.

N3. The store module now says which transitions the triggers forbid and which
rows only the head digest catches.

N6. A commit that only dates the head backwards, with nothing else changed,
is refused with `InvalidChange` by the commit itself.

### Extension scopes

Extensions are kept per workspace scope, named by a portable workspace label
as for memory. Each scope holds:

- the keys the person trusts in it, each with one role: a package signer or a
  revocation issuer. A key is an Ed25519 public key with an identity;
- the extensions installed in it;
- the revocation list it accepted last, with its signature.

Nothing crosses scopes. A key, an installation or a list in one scope never
changes another scope.

### Extension catalog state

The catalog host keeps one extension catalog for its state root as the owner
state `extension-catalog`. Each operation decodes the stored state, applies
one change and commits the next state under the revision it read. Decoding is
closed and verifies everything again:

- every key is a valid Ed25519 public key, and keys are unique;
- every installed package passes the component's package verification again,
  against the scope's signer key, the license and the source digest kept with
  it, its dependency locks and the host's compatibility;
- the accepted list passes the component's list verification again, against
  the scope's issuer key.

A state that fails any check is refused as a store integrity failure.

An installed extension is active unless the scope's accepted list revokes its
package, its exact manifest or its signer key, or it depends on an inactive
extension. Its catalog entries, one per tool and skill, follow the
component's catalog rules. Two extensions of one scope may not offer the same
tool or skill.

### Operations

- Trust. The person adds a key to a scope from a trust statement file: its
  role, identity and public key. A key already trusted in the scope, or an
  identity already used for that role there, is refused.
- Distrust. The person removes a key by its digest. A signer key of an
  installed extension, or the issuer key of the accepted list, is refused as
  in use.
- Install. The person installs one signed package from a directory into a
  scope and names the one license allowed for it. The CLI reads the package
  file and the source file, and sends the package with the source digest it
  observed. The host checks that the package is not installed in the scope,
  that a trusted signer key of the scope matches the manifest's signer
  identity and key digest, and that each dependency is installed and active
  in the scope at its exact version and manifest digest. It then runs the
  component's verification, with the scope's revocations, and the catalog
  rules. The person's invocation approves the installation; its decision
  digest is the SHA-256 of the request.
- Uninstall. One extension leaves a scope. An extension another one depends
  on is refused.
- Apply revocations. The person applies a signed revocation list file to a
  scope. A list whose issuer key the scope does not trust is refused as
  untrusted. A list that fails its own verification (shape, digest or
  signature) is refused as invalid. A list older than, forked from or of
  another identity than the scope's accepted list is refused as stale. The
  same list again changes nothing. Otherwise the scope keeps the list, and the
  answer names every extension of the scope the list made inactive.
- List. Every scope, or one: its keys by role, identity and digest; each
  extension with its version, manifest digest, signer, license, tools,
  skills, requested side effects, whether it is active and why; and the
  accepted list's identity, sequence and digest.

Every refusal is a closed content-free code and changes nothing.

Transport. IPC wire 14 adds extension requests and answers. The CLI's
`--extension-trust FILE`, `--extension-distrust KEY_SHA256`,
`--extension-install DIRECTORY` with `--extension-allow-license LICENSE`,
`--extension-uninstall PACKAGE_ID`, `--extension-revocations FILE` and
`--extension-list` are catalog operations that launch only the catalog host.
Each needs `--extension-workspace LABEL`, which is optional only for the list.
The CLI keeps an answer only when it acknowledges the sent request. Files are
read only from absolute paths, without following links, as bounded regular
files.

The harness gains `extension`, and `extension-sample`, which copies a
committed synthetic sample into a new private directory. The sample holds
trust statements, three signed packages, one of which depends on another,
and signed lists: two in sequence, a fork, a list from a key the sample does
not trust, and a list whose signature does not verify. Its keys are derived
from fixed, published seeds. They are sample keys and must never be trusted
outside a disposable demonstration root. A unit test regenerates every sample
file from those seeds and compares the bytes.

## Limits

An installed extension provides nothing to a coding run yet: no runtime reads
the extension catalog, and nothing an extension declares is granted or run.
The host keeps no package source and trusts the source digest that the
person's own authenticated CLI observed. Updates and rollback of an installed
extension are not offered. Native proof of the coding host remains AMR-05.10,
and independent review remains open.

## Consequences

- `capabilities/knowledge`: `doc_pack_text_allowed`, `kept_version_count` and
  the restore test.
- `kernel/engine`: the owner state name `extension-catalog`, the store's
  module wording and the day test.
- `shells/host`: the pack reader's text rule, the owner's history bound and
  tests; the extension owner of the catalog host, its sample, wire 14, the CLI
  options and rendering.
- `scripts/coding_harness.py`: `extension` and `extension-sample`.
- TASKS.md: AMR-05.9.4.2 is recorded with its evidence when the batch is
  verified.
