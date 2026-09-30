# Decision 0126: Offline Documentation Packs, a Language Server Codec and Recipes

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088 and 0124; the implementation amendment; current owner restart |
| Scope | AMR-05.6 (CAP-22), AMR-05.7 (CAP-26) and AMR-05.8 (CAP-31) as components; AMR-05.9 split into AMR-05.9.1 to AMR-05.9.5 |

## Findings

Decision 0124 left three AMR-05 components without a module.

- CAP-22. The knowledge crate retrieves source-traceable fragments with a
  fixed ranking, but nothing imports, versions, licenses or retires
  documentation that is not in a workspace.
- CAP-26. The repository map seals untrusted language service observations,
  but nothing reads a language server's messages. Positions arrive as line and
  character offsets in a negotiated unit and paths as URIs, so turning them into
  byte ranges of the granted files is where a mistake would widen scope.
- CAP-31. Declarative skills, change plans and validation templates exist, but
  nothing describes a repeatable change with typed parameters, a bounded scope,
  prerequisites, verification and a rollback boundary. Automatic dependency
  upgrades and migrations stay forbidden (coding skills), so a recipe can only
  propose.

## Decision

### AMR-05.6: offline documentation packs (CAP-22)

A pack version has a sealed manifest: a plain identity, a semantic version, a
title, a publisher, a license identifier, the date its content was obtained, a
freshness window, and its Markdown or plain text files by relative path, length
and digest. The manifest file is parsed closed.

An import supplies the file bytes already on the machine. It succeeds only
when all of the following hold:

- the manifest verifies and the person's allow-list holds the license;
- every file matches its length and digest and is text without control
  characters;
- the intent names the pack's state: `new` when no version is current, or
  `refresh` naming the current version, which the import supersedes;
- the version is newer than every kept version, and pack and catalog bounds
  hold.

Importing the same version again, or another manifest with the same version,
is refused. No module here has a network client; a refresh is the import of a
newer version the person already holds. Inspection reports each kept version's
state and whether it is fresh or stale on the inspected day. Retention deletes a
superseded version after the person's retention period and never deletes the
current one. A person may delete one version or a whole pack. Deleting the
current version leaves the pack without one; no older version becomes current.
A deletion removes content and index and keeps a content-free record. Dates
never go backwards.

Indexing turns every kept version into documents for the deterministic
knowledge retrieval. Each version gets its own workspace identity. Fragments
are headings or paragraphs of at most 4 KiB, cut at line ends and, for a longer
line, at character boundaries. A superseded version is marked historical, so
current searches return only the current version. A fragment the secret screen
flags is withheld and counted in the receipt.

### AMR-05.7: language server codec (CAP-26)

The codec frames and classifies a language server's JSON-RPC 2.0 messages and
seals diagnostics, references and rename previews as untrusted observations of
the existing language service contract.

- Frames. A header block of at most 1 KiB holds one decimal content length
  and, optionally, a content type whose only parameter declares UTF-8. The body
  is at most 4 MiB. Any other header, a repeated or zero-padded length, a stray
  line break or an oversized block fails the decoder, and it refuses every later
  byte, so the owner stops the server.
- Messages. A message is exactly a response, a notification or a server
  request. Unknown members, repeated keys, a null, negative or fractional
  identity and an error without a message are refused. An error's message is
  never read. A server request is only classified; the owner answers it with a
  refusal unless a qualified adapter handles it under ordinary tool authority.
- Paths. A URI must lie under the workspace root's URI, which the adapter
  supplies and nothing records. Its segments are decoded strictly and must form
  a canonical workspace path; traversal, encoded separators, queries and
  fragments fail.
- Positions. Positions are converted in the negotiated unit (UTF-8, UTF-16 or
  scalar values) against the exact granted bytes, whose digests must match the
  request. A position inside a character or past the last line fails. A
  character offset past the end of its line means the line's end, as the
  message protocol defines.
- Observations. Diagnostics keep their range and a digest of their severity,
  code, source and message; related information, tags and data are not kept.
  References keep their location. A rename preview keeps text edits with the
  digest of each replacement. File creation, renaming and deletion, both edit
  forms at once, and overlapping edits or two inserts at one position are
  rejected as a whole. An empty answer is sealed as unavailable, never as a
  complete proof of absence. A cancelled request is sealed as cancelled. More
  items than the descriptor allows give a partial, truncated observation.

The codec also encodes references and rename requests at the request's
position. A rename request needs the new name whose digest the request carries.
The codec starts no process, reads no file and applies no edit. Server launches,
workspace edits and dynamic server configuration stay under ordinary tool
authority, and any adapter is qualified separately (AMR-05.10). The codec
claims no conformance to any server; its fixtures are written by hand in this
project's terms.

### AMR-05.8: engineering recipes (CAP-31)

A recipe manifest is sealed and parsed closed. It holds:

- a plain identity, a version, a closed kind (dependency update, migration,
  security repair, documentation or tests) and a title;
- typed parameters: a choice from a sorted set, a bounded integer, a
  workspace path or a version;
- workspace path prefixes as its scope, and the most files one plan may change;
- sorted prerequisites (a clean worktree, locked dependencies or a named tool);
- the validation kinds that verify it;
- a rollback boundary: only the plan's own writes, or irreversible with a
  reason code;
- whether applying needs a separate network grant.

Each kind declares what its change needs:

- a dependency update needs locked dependencies and a build or test
  validation;
- a migration needs a clean worktree and a test validation;
- a security repair needs a clean worktree and a security or test validation;
- a tests recipe needs a test validation;
- documentation never needs the network.

Instantiation checks every value against its declared parameter. It binds every
registered validation template of each declared kind from the workspace's
registry, and fails when a kind has none. The result is a plan with an exact
digest and a fixed proposal marker. A change checked against the plan must stay
inside its scope and file bound. The plan grants nothing: its writes, commands
and any network access go through the ordinary change approval, validation and
grant paths.

### AMR-05.9 split

AMR-05.9 is split into these rows, which must all close before it does:

| Row | Integration through the coding host and development CLI |
| --- | --- |
| AMR-05.9.1 | An action history entry for each authorized effect of a run, kept by the effect owner; a CLI view after each run and a redacted export on request |
| AMR-05.9.2 | Local-only routing by default: the development host's route receipt records the mode and data classes, and no remote route is available without its own grant |
| AMR-05.9.3 | Support bundles previewed and published through the CLI only with the one-use export approval |
| AMR-05.9.4 | Memory and extension revocation through production transport, with per-scope isolation |
| AMR-05.9.5 | Documentation pack import, inspection, refresh and deletion; language server observations through a host adapter; recipe plans handed to the ordinary change approval |

The review's note N4 (the router's time and counters are caller-supplied)
becomes a duty of AMR-05.9.2: the host sources them from the grant's owner.

## Verification boundary

Unit tests cover each rule above on hand-written fixtures:

- packs: every refusal with the catalog unchanged, refresh and supersession,
  freshness on given days including a leap year, retention and deletion without
  content, fragment bounds, the secret screen and current versus historical
  searches;
- codec: frames split and joined across reads, every header refusal and the
  failed decoder, message classification and refusals, UTF-8, UTF-16 and scalar
  positions over a CRLF line and non-ASCII characters, clamping, root and
  traversal refusals, unsupplied files, the three observation kinds, empty,
  failed, cancelled and foreign answers, rename refusals, request encoding and
  the server-request refusal;
- recipes: closed admission and the seal, each kind's rule, typed and bounded
  parameters, plan binding and digest, unavailable verification and change
  scope.

These are component rows. No coding host, CLI, native process, language
server, network, provider, model or GPU uses them yet (AMR-05.9 and AMR-05.10).
Independent review is requested and remains open.
