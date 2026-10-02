# AgentMage public research worker contract — version 1

Revision: 2026-10-02, version 1. Registered under AMR-02.3.1 in [TASKS.md](../../TASKS.md);
design in [Decision 0141](../decisions/0141-public-research-worker-contract.md), within
[Decision 0084](../decisions/0084-isolated-public-research-transport.md).

This contract states what passes between the engine and the optional public research worker:
the request the parent hands the worker, the frame the worker returns on success, the report it
returns on failure, and how one exact call binds to the permit that the authority owner consumes
before the worker starts. It is unexecuted here. No worker runs in this repository's checks, and
no host, provider or model uses the worker. Its native qualification is AMR-02.3.2, which needs a
native Linux host.

## Roles

| Role | Position |
| --- | --- |
| Parent | The engine's authority owner and the Linux effect driver. It prepares the packet, consumes the exact grant, reserves the budget, starts one confined worker and decodes what the worker returns. |
| Worker | The optional `agentmage-public-research-worker` binary. It reads one packet, performs one bounded public HTTPS GET with its own URL, DNS, TLS and redirect checks, and writes one frame or one failure report. |

The worker holds no grant, workspace, store, model, credential or browser state. Nothing it
writes grants anything: the parent decides what an attempt means.

## Versions

| Item | Version | Where it is defined |
| --- | --- | --- |
| This contract | 1 | `fixtures/public-research-worker/v1/manifest.json` |
| Request packet | schema 1 | `kernel/engine/src/research_fetch.rs` (`PublicGetWorkerPacket`) |
| Frame metadata | schema 1, or outer schema 2 | `kernel/engine/src/research_response.rs` (`PublicGetResponse`) |
| Failure report | 1 | `kernel/engine/src/research_response.rs` (`PublicGetWorkerFailure`) |

A consumer treats any other version as unavailable.

## The request

The parent passes one packet as a sealed read-only file at `/input/request`, never on standard
input, because the sandbox uses standard input to install its system call filter.

- The packet is one JSON object of at most 16 KiB with exactly these members, in this order:
  `schema_version` (1), `task_id`, `policy_sha256`, `prepared_at_epoch_ms`,
  `deadline_epoch_ms` and `request`.
- `request` holds `schema_version` (1), `operation_id`, `target` (`domain`, `path` and the
  ordered `query` fields), `maximum_response_bytes`, `redirect_limit` and `timeout_ms`.
- The deadline is the preparation time plus the timeout, and it lies inside the task's original
  deadline. The worker refuses a packet outside its interval.
- `policy_sha256` names the plan's research restrictions. It is not the governing policy that
  evaluates the grant; the permit binds that policy at issuance and again at the start.
- The packet's identity is the SHA-256 of its exact bytes.

## Exact bytes

Each packet and each frame's metadata is accepted only as the exact canonical encoding of its
decoded value: compact JSON in the declared member order, with no other whitespace, escapes or
number forms. A member the types do not name, or a member named twice, is refused as well. The
worker refuses any other packet bytes with the input failure, and the parent refuses any other
frame metadata. One request therefore has one identity, and one response has one frame.

## The frame

On success the worker writes exactly one frame to standard output and exits with status 0 and
no standard error output.

- The frame is a four-byte big-endian length, then that many bytes of JSON metadata, then the
  exact body.
- Version 1 metadata is the observation: `schema_version` (1), `request_sha256`,
  `operation_id`, `started_epoch_ms`, `completed_epoch_ms`, `hops`, `media` and `body_sha256`.
  Each hop holds its `target`, `status`, `header_bytes` and `body_bytes`.
- Version 2 metadata is `schema_version` (2), the version 1 `observation`, and
  `worker_reported_urls`: one URL per hop, as the worker's URL library wrote it.
- The first hop is the packet's own target. Each earlier hop is a same-origin redirect, and the
  last hop is a 200 response whose body is the frame's body.
- The body is inert UTF-8 text, plain, HTML or JSON, at most the packet's response bound. It is
  never executed or rendered, and it grants nothing.

The parent decodes a frame only against the packet it prepared. It checks the request digest,
operation, times, hops, bounds and body digest, and that the worker's process and cleanup were
verified. Consistency is not attestation: a frame proves nothing about a connection by itself.

## The failure report

A worker that does not complete writes exactly one report to standard error, the failure's
code and one newline, writes no frame, and exits with status 5. The codes are closed:

| Failure | Code |
| --- | --- |
| The arguments, environment or confinement are not the admitted ones | `research.worker.environment-denied` |
| The request input is unreadable, oversized or not an exact packet | `research.worker.input-denied` |
| The target, its URL or its DNS answer is outside the public scope | `research.worker.destination-denied` |
| Connecting, TLS or the HTTP exchange failed | `research.worker.transport-failed` |
| The response's status, headers, media type or redirect is refused | `research.worker.response-denied` |
| A header, body, redirect or ciphertext bound was reached | `research.worker.limit` |
| The packet's deadline passed, or the clock was unusable | `research.worker.deadline` |
| The frame could not be written | `research.worker.output-failed` |

The parent reads a report only as a diagnostic. When the worker exited with status 5 and wrote
exactly one known code and its newline, the attempt's failure reason is that code; any other
output keeps the generic reason `native-nonsuccess`. The attempt's authority outcome does not
depend on the report: a worker that ran may have disclosed its request, so the attempt stays
uncertain. A report never proves that nothing was sent.

## The binding to the permit

One exact call binds to the permit the authority owner consumes before the worker starts:

- the registered tool declares exactly one network effect under a single-use grant;
- the call's argument bytes are the exact encoding of the packet's `request`, under their digest,
  and the call's identity is the request's `operation_id`;
- the issued grant's preview and its only expected effect name the packet's digest;
- the start's policy evaluation names the packet's own destination, `https:<domain>:443`;
- the dispatch adapter checks the packet's task, operation and digest once more before the
  worker starts.

The `binding-visit.json` fixture shows this chain for the visit packet.

## Consumer obligations

1. Accept only the exact canonical bytes of a packet or frame, and treat an unknown version as
   unavailable.
2. Decode a frame only against the packet it was prepared for.
3. Read a failure report only as a diagnostic; never read it, or a missing frame, as proof that
   nothing was disclosed.
4. Never read a frame as a grant, a citation or permission to visit a link it mentions.
5. Treat the body as untrusted content.

## Fixtures

Each record has fixtures in `fixtures/public-research-worker/v1/`.

| File | Record | Shows |
| --- | --- | --- |
| `request-visit.json` | request packet | a visit to one page, with one redirect allowed |
| `request-search.json` | request packet | a search that carries the plan's disclosed query |
| `response-visit-v1.frame` | frame, schema 1 | the visit, one hop |
| `response-visit-v2.frame` | frame, schema 2 | the visit after one same-origin redirect, with the worker's URLs |
| `response-search-v1.frame` | frame, schema 1 | the search's JSON result |
| `failures.json` | failure reports | the exit status and each code's exact report |
| `binding-visit.json` | permit binding | the visit call's arguments, digests, grant and network scope |
| `manifest.json` | — | the contract version and each file's record, schema and SHA-256 |

The fixtures are synthetic. The packets come from a plan with two destinations and one disclosed
query, and the frames from fixed observations and bodies. They show each record's exact bytes,
not a retrieval.

The engine test module `research_response::contract_tests` builds every fixture from the
engine's types and functions and compares it byte for byte. It decodes each one exactly and
refuses other encodings of the same values, added members and repeated members. It decodes each
failure report and refuses any other bytes or exit status, and it verifies the binding through
the shared call validator. A Linux test checks the parent's reading of a report.
`tests/test_public_research_worker_contract.py` checks the manifest's digests, parses the frames
and checks that this document names every fixture and code. After a deliberate change to a
record, `python3 scripts/public_research_worker_fixtures.py --write` regenerates the files from
the engine's types; without `--write` it only compares.

## Dependency closure

The worker is absent from the default build. Its URL, HTTPS and TLS libraries are optional and
enabled only by the `public-research-worker` feature of the Linux crate.
`security/public-research-dependency-policy.json` records the default and connected closures, and
`python3 scripts/research_dependency_closure.py` checks them against the lockfile and the
feature topology. The worker's modules belong to its binary alone.

## Change control

A change to the encoding or meaning of any record raises that record's version and this
contract's version, and adds a new fixture directory beside `v1`. Version 1 stays as it is.

## Limits

- Unexecuted: no worker ran for these fixtures, and no frame came from a network.
- The confined worker's native qualification, with its DNS, TLS, redirect and rebinding matrix,
  admission and owned cleanup, is AMR-02.3.2 and needs a native Linux host.
- No host, provider or model uses this contract yet; Decision 0084 forbids activation before the
  native matrix.
