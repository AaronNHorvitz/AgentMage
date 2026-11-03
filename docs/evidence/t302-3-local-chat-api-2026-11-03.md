# T-302.3: Authenticated local chat API, 2026-11-03

Builder evidence for the batch based on
`d228d9c87d083f60837862f45d93b36b559d5241`. T-302.3 remains **open** pending an
independent exact-commit core review. The preceding commit retained the unchanged
[T-302.2 PASS](../reviews/e118db465d64-local-agent-t302-2-transport.txt) and closed
only that transport component. No builder review closes a row.

## Running scope

The Linux inference crate now exports `local_chat_api` and auto-discovers the
`agentmage-local-api` executable. It serves a versioned authenticated loopback
HTTP API over the existing approved transport. The
[API guide](../guides/local-chat-api.md) gives build/start commands, routes,
JSON contracts, bounds, credential ownership and remaining limitations.
The executable was spawned and queried by a process contract test, rather than
only compiled as a zero-test binary harness.

Two separately generated API tokens distinguish trusted settings controls from
ordinary conversation clients. Only the authenticated settings path, with an
explicit true conversation choice, mints endpoint approval. Probing is separately
authorized, anonymous and informational. Chat clients cannot choose endpoints,
override destinations or turn successful discovery into permission. This is the
T-302.2 review's L2 acceptance input. It does not authenticate a physical human
or isolate hostile processes under the same operating-system account.

The API supports connection testing/model listing, text chat, cancellation,
settings inspection/revocation and a bounded activity feed. Requests use strict
length framing, exact numeric Host, bearer authentication and JSON. Browser
origins/cookies/fetch metadata, forwarded headers, input chunking, compression,
duplicate headers/fields and oversized requests fail. Four fixed workers and
one upstream operation bound concurrency. Request deadlines are absolute;
response writes are bounded. Cancellation remains serviceable during inference,
and settings cannot replace an active operation's authority.

Chat emits incremental JSON text events before upstream completion. Only verified
transport completion emits a terminal success event. Upstream failure gives a
fixed content-free error and marks earlier text partial. JSON framing does not
make untrusted text safe for terminal/markup interpretation; clients have an
explicit display obligation for answers **and model IDs**. The guide discloses
residual model-port trust and probe timeout behavior from review lows L1/L3/L4;
their backlog verification remains with T-309.

Private owner-only single-link token files are loaded through a held private
directory, with no symlink following for the directory or token leaf. Setup
never overwrites existing tokens. The model key/configuration stay in memory
and require a new explicit choice on restart. Metadata activity retains the
latest 128 ordered events, settings revision and exact configured endpoint,
with no keys, tokens or conversation text. It is process-only inspection, **not**
durable run recording, interruption/resume or replay; those remain T-303 and must
use the canonical store. No tool or command route exists in this API batch.

## Observed checks

The complete per-change gate ran after the final Rust edit:

```sh
python3 scripts/local_agent_fast_gate.py --base d228d9c87d083f60837862f45d93b36b559d5241 --timeout 600
```

It selected the inference crate and reverse-dependent host, with the normal
shared reservation, two build jobs, four test threads, locked/offline Cargo,
all targets and no fail fast. There were no new skips, filters, failure
allowlists, dependency/feature changes or weakened trust checks.

| Gate step | Exit | Seconds including reservation wait |
| --- | --- | --- |
| Range whitespace | 0 | 0.10 |
| Untracked whitespace | 0 | 0.10 |
| Markdown lint, two guides | 0 | 0.20 |
| Touched-package formatting | 0 | 0.85 |
| All-target warnings-denied Clippy | 0 | 34.51 |
| All-target unit/integration/binary tests | 101 | 110.58 |

| Package, all targets | Passed | Failed | Existing ignored | Filtered |
| --- | --- | --- | --- | --- |
| Inference | 217 | 0 | 8 | 0 |
| Host reverse dependent | 532 | 53 | 11 | 0 |
| Total | 749 | 53 | 19 | 0 |

The gate is **FAIL**, exit 1. All 53 sorted failed identities exactly match the
[host baseline](t301-baseline-failures-2026-10-25.md#host-library-53), with no new
or missing identity. This comparison is not a pass or an allowlist. T-301.1's
native executable-trust environment blocker remains deferred and blocks no
mission work under Decision 0153. No native sandbox acceptance is established.

All **15 API contract tests** passed with zero failed, ignored or filtered in
4.20 seconds inside that complete gate. All 17 existing transport tests also
passed. The API contracts exercise actual ephemeral API/upstream sockets:

- Incorrect token, Origin, Cookie, fetch metadata, forwarded headers and wrong
  Host refusal; client-token refusal for every settings/probe path.
- Successful probe without credential forwarding, followed by refused chat and
  listing until a separate explicit authenticated endpoint choice.
- Exact configured credential routing, secret-free settings inspection and
  revocation without subsequent upstream permission.
- First text event observed while the upstream waits for the test's acknowledgement;
  JSON framing of markup, newline, terminal-control and bidirectional text.
- Truncated/late-invalid streams and upstream authentication errors emit no
  completion; error bodies, upstream reasons and credentials are not echoed.
- Concurrent cancellation during a stalled chat; refusal of competing upstream
  operations and settings races; orderly shutdown closes upstream and joins workers.
- Duplicate/ambiguous headers, invalid/oversized requests, unsupported encoding,
  remote endpoints, invalid keys, unknown chat fields and invalid roles/limits.
- Four slow-header clients and an incomplete body hit absolute timeouts;
  workers recover for subsequent authenticated requests.
- Activity captures exact choices/results without conversations or secrets;
  retention stays at 128, with sequence gaps visible after eviction.
- Random private token setup never overwrites; loading rejects loose modes,
  symlink files, hard links and oversized tokens; the real executable initializes
  credentials, starts and answers authenticated status.

The synthetic upstreams are **not models**. Earlier incremental contract runs
passed 13 and then 14 tests; the final all-target run above covers the final
15-test source. No model, GPU, download, tuning, real inference or coding-score
claim follows from these measurements. T-302.4 still needs the GPU-slot real chat.

The reserved ordinary documentation validator passed all policy invariants for
688 Markdown files in 138.63 seconds including reservation wait. Its Markdown
enumeration preceded this final evidence file. The four final changed Markdown
files then passed scoped link, sensitive-text, claim and identifier checks, plus
Markdown lint and whitespace checks. This is not the long documentation gate.

Private lane logs retain exact timings, outputs, failure comparison and tested
source hashes. The final queue/evidence prose receives scoped Markdown and
whitespace checks after the Rust gate; Rust source remains at its tested hashes.

## Preservation

No dependency or Cargo manifest/lock was changed. This is original work in the
project's own terms, without external product code, documentation or designs.
Current Apache-2.0 headers match existing package metadata; import-wide
MIT-or-Apache licence reconciliation remains an inventory follow-up.

The historical ledger below the mission queue, architecture status model,
runtime permissions, sandbox and supply-chain outputs are unchanged. No SBOM
or old evidence cascade was regenerated, and no evidence input set was narrowed.
The long documentation gate remains scheduled once at T-310. MCP, task CLI,
durable agent runs, coding evaluation, install kit, local owner testing and
public release are not claimed by this API evidence.
