# Decision 0124: AMR-05 Decomposition and First Components

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088 and 0110; the implementation amendment; current owner restart |
| Scope | AMR-05 split into component and integration rows; CAP-07 route grants, CAP-29 memory revocation, CAP-42 action history, CAP-44 extension revocation and CAP-47 support bundles as components |

## Findings

AMR-05 groups eight P1 capabilities. None of them reaches its acceptance through the
coding host or the development CLI today, and each has some existing source.

- CAP-07. The kernel router selects one qualified route and allows a fallback only under
  an explicit policy. A remote route needs only a request flag and a per-route cost flag.
  Nothing names the provider, the data sent or a budget, and nothing lets a person say
  "local only". Nothing outside the router's own tests calls it.
- CAP-29. The memory catalog records insertion, supersession, correction, decay, hold,
  expiry and deletion. It has no revocation. When a source is found wrong or withdrawn,
  the facts drawn from it can only be deleted one by one, and a historical load still
  returns them until then.
- CAP-44. A capability package is admitted only with a trusted signer, a matching key
  digest and a strict signature. Nothing can revoke a package, a manifest or a signer key,
  so a compromised key or a bad release stays admissible.
- CAP-42. Several modules record their own audit facts, but no single record ties the
  authorization, the effect's identity, the outcome and the evidence together, with
  redaction, retention and an export.
- CAP-47. The host exports a content-free doctor report through a one-use approval. It
  cannot export component versions or evidence digests, which support needs.
- CAP-22, CAP-26 and CAP-31 have no module yet. Offline documentation packs can reuse the
  source lifecycle and the knowledge index. Language intelligence has sealed language
  service observations but no server codec. Recipes are closest to the declarative skill
  and change-plan contracts.

## Decision

Split AMR-05 into these rows and record every identity in the work selector:

| Row | Capability | Kind |
| --- | --- | --- |
| AMR-05.1 | CAP-07 | Kernel component: route grants and local-only mode |
| AMR-05.2 | CAP-29 | Component: revocation of memory items and sources |
| AMR-05.3 | CAP-44 | Host component: signed revocation of packages, manifests and signer keys |
| AMR-05.4 | CAP-42 | Kernel component: action history with redaction, retention and export |
| AMR-05.5 | CAP-47 | Host component: support bundles through the one-use export approval |
| AMR-05.6 | CAP-22 | Component: offline documentation packs |
| AMR-05.7 | CAP-26 | Component: language server codec and sealed observations |
| AMR-05.8 | CAP-31 | Component: versioned recipe manifests |
| AMR-05.9 | all | Integration through the coding host and development CLI |
| AMR-05.10 | all | Actual-process proof on a native Linux host and qualified adapters |

Each component row extends existing rows (Stories 13.6, 15.2 and 31.1, Sprints 78, 79,
157 and 159) instead of replacing them. Their checked rows stay as they are. The parent
closes only when every row closes. AMR-05.1 to AMR-05.5 are implemented now.

### AMR-05.1: route grants and local-only mode (CAP-07)

A routing request names its mode and every data class it sends: conversation,
workspace excerpts, tool outputs, retrieved sources or memory. In local-only mode no
remote route is eligible, whatever grants exist. In hybrid mode a remote route is
eligible only under a grant of its own that:

- names that exact route and candidate, and the provider shown to the person;
- lists every data class the request sends;
- has not expired and has budget left for one more request and its input tokens, as
  counted by the grant's owner;
- allows fallback, when the route would replace a failed route.

A grant carries the digest of its canonical form, and a tampered, duplicated or empty
grant fails the request. The receipt records the mode, the data classes sent, and the
grant digest and provider when the route is remote. Local routes are still preferred,
and fallback still needs the existing explicit policy, so nothing substitutes a remote
route silently.

### AMR-05.2: memory revocation (CAP-29)

Revocation is a new explicit transition. It withdraws one item, or in one transition
every item of one workspace that cites a revoked source or source object. Items of other
workspaces are never touched, and a revocation that reaches no item changes nothing. A
revoked item keeps its content so the person can inspect it and delete it later. It no
longer counts as current, and no load returns it, not even as history. The portable
export carries the revoked state, so an import never makes the item loadable again.
Older builds refuse an export that contains a revoked item. A later candidate that
cites the same source needs its own explicit decision; the catalog keeps no standing
source ban.

### AMR-05.3: extension revocation (CAP-44)

A revocation list is sealed, signed in its own domain and sequenced. It revokes a
package, an exact manifest or a signer key. The host verifies the list's shape, digest,
trusted issuer key and signature. It refuses a list older than, or forked from, the last
list it accepted, and it keeps that list's checkpoint. A host that has accepted a list can
no longer claim it has none. Package admission refuses a revoked package before checking
its signature, and the admission record binds the list it was checked against. For a
package revoked after it was enabled, the catalog marks every entry of the package
inactive and says why. It also marks any active entry whose manifest was not supplied,
so an unchecked entry never stays active.

### AMR-05.4: action history (CAP-42)

An action history entry records:

- the closed action kind and the owner's action identity;
- its authorization, a grant or a person's decision. Only a denied or cancelled action
  may name none;
- the digest of the effect's identity, the closed outcome and a stable reason code;
- the evidence digests and a retention deadline.

The entries form a hash chain that a restarted owner replays exactly. Identifiers are
kept only when they are plain identifiers that the shared secret detector does not
flag. Anything else is replaced by a redaction marker, which leaves no digest, and the
entry counts it. A raw prompt, a path or a token therefore never enters the history.
When an entry's retention deadline passes, only its place in the chain remains. A manual
export covers a requested range and is scanned once more before it is returned. It has
no path and writes nothing.

### AMR-05.5: support bundles (CAP-47)

A support bundle holds the local doctor report, component versions and evidence records
named by kind and digest. Every name must be a plain identifier that the shared secret
detector does not flag. The bundle says it is never uploaded. It is previewed and
published only through the existing one-use, expiring export approval, into a private
local directory. The module has no network client.

## Verification boundary

Unit tests cover each rule above:

- routing: local-only mode, grant mismatch, expiry, data classes, both budget limits,
  arithmetic overflow and fallback, and tampered, duplicate and empty grants;
- memory: item and source revocation, workspace isolation, the object selector, loads
  including history, and the portable round trip;
- extensions: list trust, signature, rollback and forks, each revocation kind at
  admission, and catalog deactivation;
- action history: redaction with secret and prompt canaries that leave neither value
  nor digest, the authorization rule, retention, replay tampering and export ranges;
- support bundles: publication only after approval, and refusal of secrets, paths,
  disorder and uploads.

These are component rows. No coding host, CLI or native process uses them yet
(AMR-05.9 and AMR-05.10). No test uses a network, a provider, a model or the GPU.
Independent review is requested and remains open.
