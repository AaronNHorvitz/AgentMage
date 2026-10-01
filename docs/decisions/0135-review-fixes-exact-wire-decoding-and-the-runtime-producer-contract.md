# Decision 0135: Review Fixes for Batch 19, Exact Wire Decoding, and the Runtime Producer Contract

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0124, 0127, 0129, 0133 and 0134; current owner restart |
| Scope | Findings F1 and F2 and notes N2 and N5 of the independent review of `84e531fb`; AMR-06.1 |

## Findings

An independent read-only review of `e197fe95..84e531fb` passed with two low
findings and twelve notes.

- F1: a recipe's parameter type, prerequisite and rollback are enums tagged
  by one member. A variant that names only its tag accepted, and dropped,
  any other member beside the tag. The manifest file parser refuses such a
  member, because it compares its re-encoding with the file. The prepare
  request and the declared plan on the wire are decoded by the types alone,
  so a recipe or a plan with such a member decoded, and the plan still
  verified. Nothing was granted or changed: the manifest seal and the plan
  digest are over the types, and the dropped member could alter neither.
  The claim that recipe plans decode closed was too broad.
- F2: the client's check of a declared plan compares the plan's recipe
  identity, version, kind and manifest digest with what was sent. No test
  failed when one of those four comparisons alone was removed.
- N2: the plan's text form prints each validation identity and template
  digest as the host declared them, without checking their shape.
- N5: the plan's file bound counts each distinct path when it is proposed,
  so in-scope proposals the person declines still use the bound. The
  testing guide did not say so.

The review's other notes need no change. Notes N1 and N8 to N12 describe
equivalent mutations, the absence of a fuzz target, inherent limits and
behavior that is already correct; notes N3, N4 and N7 concern how the
request and the inventory count stages and records; note N6 is already a
Limit of Decision 0133.

AMR-06.1 is dependency-ready (Decision 0134). The runtime already produces
the records a coordinator would read, as host wire types, but no document
says which of them a consumer may rely on, and no fixture holds them apart
from the tests.

## Decision

### Review fixes

F1, in the types. Each variant that names only its tag becomes a variant
with an empty member list: the parameter types `path` and `version`, the
prerequisites `clean_worktree` and `locked_dependencies` and the rollback
`revert_writes_only`. Encoding is unchanged, byte for byte, so manifest
seals, plan digests and the committed samples are unchanged. Decoding now
refuses any member beside the tag wherever the types are decoded. The
action history's `unauthorized` authorization already has this form.

F1, on the wire. Every frame of the runtime transport is decoded exactly,
in both directions: the frame decodes into its types, and its re-encoding
must equal the frame as received. A member the types do not name is
refused at any position, whatever the type's own attributes allow. This is
the rule the manifest file parser already applies. A host answers a frame
that is not exact as a denied request; a client treats such an answer as
evidence the runtime cannot be trusted for this step. No member or
encoding changed, so the wire version stays 15. (Corrected under Decision
0136: as first implemented, the comparison refused a run request whose
32-bit decoding values are not short binary fractions, so not every
conforming frame decoded; and a member repeated inside a map passed.)

F2. The test of the declared plan gains a resealed plan that differs only
in its recipe identity, only in its version, only in its kind and only in
its manifest digest. Each is refused, and each fails the test when its
comparison alone is removed.

N2. The client keeps a declared plan only when every validation it binds
names a plain identity and a lowercase SHA-256 digest, and its registry
digest is a lowercase SHA-256 digest. The text form therefore prints only
such values.

N5. The testing guide says that the bound counts proposed paths, approved
or not.

### The runtime producer contract (AMR-06.1)

`docs/architecture/runtime-producer-contract-v1.md` states, for the
Coordinator, Memory and Host roles, which runtime records a consumer may
rely on:

- the sealed run request;
- the run declarations, schema 4: recoverability, context views, the
  effect, job control and route histories, the route receipt and the
  recipe plan;
- job status, and the answer to a job control request;
- the stored action histories of an ended run.

For each record it names the version, the members a consumer may rely on,
what an absent member means, what the digest or seal covers, and the
existing function that verifies it. It states how records travel: an
authenticated local IPC peer, a versioned envelope, frames of at most
4 MiB, decoded exactly. It states the consumer's obligations: verify
before use, treat an unknown version as unavailable, and never read a
record as a grant.

`fixtures/runtime-producer/v1/` holds one synthetic instance of each record
and a manifest of their digests. A host unit test builds every fixture from
the runtime's own types and functions, compares each with the committed
file byte for byte, decodes each exactly and runs the verification a
client runs. A Python test checks the manifest's digests and that the
document names every fixture and version, and nothing private.

This is an unexecuted producer specification. It is not a consumer's
contract, a counterpart's approval or an integration proof. It names roles
only. A consumer's own contract is pinned separately, when offered, and
checked against these fixtures. The transport, its version and the records
themselves are unchanged by it.

## Limits

- The fixtures are synthetic: the request comes from the controlled-write
  coding-run fixture (corrected under Decision 0136) and the histories from
  test recorders. They show the shape and verification of each record, not
  a coding run.
- Exact decoding refuses members the types do not name. It does not make a
  record authoritative; each consumer still verifies what it uses.
- Rows AMR-06.2 to AMR-06.10 stay open until their consumer contracts,
  grants, accounts or native hosts exist.

## Consequences

- `kernel/engine`: the five recipe variants and their tests.
- `shells/host`: exact frame decoding in the runtime IPC and its tests; the
  declared plan's validation shape check; the recipe tests; the producer
  contract test module.
- `docs/architecture/runtime-producer-contract-v1.md`,
  `fixtures/runtime-producer/v1/` and
  `tests/test_runtime_producer_contract.py`.
- `docs/LOCAL-TESTING.md`: the file bound sentence.
- TASKS.md: AMR-06.1 is recorded with its evidence when the batch is
  verified.
