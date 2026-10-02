# Decision 0141: The Public Research Worker Contract, Version 1

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-02 |
| Date | 2026-10-02 |
| Authority | Decisions 0084, 0097, 0106, 0135 and 0139; current owner restart |
| Scope | AMR-02.3.1 |

## Findings

AMR-02.3.1 asks for three things:

- freeze the closed request and outcome of the public research worker,
  with producer and consumer fixtures;
- bind the exact arguments, disclosure and network scope to the consumed
  effect permit;
- keep the default offline dependency closure beside an explicitly
  inventoried optional worker.

Its prerequisites are AMR-02.2, which batch 24 closed, and Decision 0084.
The confined worker itself and its native adversarial matrix are AMR-02.3.2.

Most of the row exists:

- The request is one closed packet. `PreparedPublicGet` prepares it from the
  plan's restrictions, and the worker decodes it from a sealed read-only
  input. Its identity is the SHA-256 of its exact bytes.
- A success is one frame: a big-endian length, closed JSON metadata, then
  the exact body. The metadata is version 1, or version 2 with the URLs the
  worker reports. The parent decodes it against the packet.
- The binding is complete:
  - the call validator binds the tool, the argument bytes and their digest,
    and the decoded request, to the packet;
  - the issued grant names the packet's digest as its only effect;
  - the start requires the policy to evaluate the packet's own destination;
  - the dispatch adapter checks the packet again.
- The dependency closure is inventoried and enforced. A policy records the
  default and connected contexts, and its check and tests run in the
  repository's audits.

Three parts are missing:

- No fixture freezes the bytes, and no document states the contract.
- The worker reports a failure as one of eight codes on standard error and
  exits with status 5. That code is the worker's own type, and the parent
  never reads it: any standard error output becomes one generic reason.
  Decision 0084 asks for typed redacted outcomes in the fixtures.
- Decoding is not exact. The packet and the frame metadata decode through
  closed types that refuse unknown and repeated members. A packet or
  metadata written in another member order, or with other whitespace,
  still decodes. Its digest then differs from the canonical encoding of
  the same request.

## Decision

### Exact bytes in both directions

The packet and the frame metadata are each accepted only as the exact
canonical encoding of their decoded value: the closed types, written as
compact JSON in their declared member order. The worker refuses any other
packet bytes as invalid input. The parent refuses any other frame metadata
as invalid. The body after the metadata is unchanged: it is the exact
retrieved text, bounded and checked as before.

Every packet and frame the engine writes is already canonical, so no stored
packet or frame changes meaning.

### A closed failure outcome

`PublicGetWorkerFailure` in the engine names the eight failures:
environment, input, destination, transport, response, limit, deadline and
output. Each has one stable code. The worker uses this type. A failed
worker writes exactly its code and one newline to standard error, and exits
with status 5.

The parent reads such a report only as a diagnostic. When the worker exited
with status 5 and wrote exactly one known code and its newline, the
failure's reason is that code. Any other output keeps the generic reason.
The authority outcome stays as conservative as before: a worker that ran
may have disclosed its request, so the attempt stays uncertain whatever the
worker reports. The report never proves that nothing was sent.

### The contract and its fixtures

`docs/architecture/public-research-worker-contract-v1.md` states the
contract: the roles, the request, the two frame versions, the failure
report, the binding to the consumed permit, the limits and the consumer
obligations.

`fixtures/public-research-worker/v1/` holds these files:

- two exact packets, a visit and a disclosed search;
- three exact frames: version 1 and version 2 for the visit, with one
  same-origin redirect in the second, and version 1 for the search;
- the eight failure reports;
- the binding of one call to its permit: the argument digest, the packet
  digest, the grant's effect details and the policy's network scope;
- a manifest with each file's SHA-256.

The engine's types and functions build every fixture. An ignored engine test
prints them, and `scripts/public_research_worker_fixtures.py` compares the
printed files with the committed ones, or rewrites them with `--write`.

Engine consumer tests check these properties:

- each fixture is exactly what the types build;
- each decodes exactly;
- changed member order, whitespace, an added or repeated member, and changed
  numbers or escapes are each refused;
- each failure report decodes, and a report with other bytes or another
  exit status does not;
- the binding verifies through the call validator and the issuance's own
  derivations, and a changed argument byte is refused.

A Linux test checks the parent's reading of a report.
`tests/test_public_research_worker_contract.py` checks the manifest, parses
the frames and checks that the document names every fixture. The batch's
source stage also runs the existing dependency closure check and its tests.

### The packet's policy digest

The packet's `policy_sha256` names the plan's research restrictions. It is
not the governing policy that evaluates the grant. The permit binds the
governing policy at issuance and again at the start. The contract states the
difference, and the packet's digest is not compared with the permit's.

## Limits

- Component row only. No worker runs here. The confined worker, its DNS,
  TLS, redirect and rebinding matrix, its admission and owned cleanup stay
  with AMR-02.3.2 on a native Linux host.
- No host, provider or model uses the contract. Decision 0084 still forbids
  activation before the native matrix.
- No dependency, feature or lockfile changes.

## Consequences

- `kernel/engine/src/research_fetch.rs` and
  `kernel/engine/src/research_response.rs` decode exactly, and the response
  module gains the failure type and the contract test module.
- The Linux worker uses the engine's failure type and writes its exact
  report; the parent reads a report as the failure's reason.
- New contract document, fixtures, script and Python test.
- TASKS.md: AMR-02.3.1 is recorded with its evidence when the batch is
  verified, as a component row.
