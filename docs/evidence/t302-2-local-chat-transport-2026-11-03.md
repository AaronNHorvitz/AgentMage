# T-302.2: Approved local chat transport, 2026-11-03

Builder evidence under Decision 0153. Independent exact-commit core review is
pending; T-302.2 stays unchecked. The [transport guide](../guides/local-chat-transport.md)
describes the public library contract and its consumer obligations.

## Implemented scope

The Linux inference crate now makes actual HTTP requests to explicitly approved
loopback endpoints. Configuration remains provider-independent within the
supported local text-chat API: numeric loopback HTTP, explicit port and API base
path. Remote endpoints and TLS remain unsupported. No model or server is bundled.

- An owned, zeroizing bearer credential belongs to one transport instance.
  Requests compare approval with the exact numeric address and API base before
  I/O. Keys and request headers are erased on drop, including error returns.
  There is no ambient credential, DNS, proxy, redirect, cookie or retry mechanism.
- Model listing doubles as an explicit connection test. Optional first-run
  detection tests at most eight approved candidates using a separate probe-only
  permission, without credentials or conversation data. Discovery never grants
  conversation permission or automatically selects an endpoint.
- Streaming text reaches the caller before the server finishes. The caller
  receives success only after valid protocol completion, HTTP framing and actual
  socket EOF. Late bytes, truncation, cancellation and timeout make earlier text
  partial. Output remains untrusted plain text, never tool or settings authority.
- Total and inactivity deadlines, a cancellation tick, and fixed wire, header,
  body, model-list, event and text ceilings bound requests. Fixed error codes and
  plain-language guidance contain no endpoint, upstream body, key or prompt.
  Each call closes its socket and starts no background worker or child process.

These are trusted in-process application contracts. The library's approval
constructors are not authentication for an untrusted API client. T-302.3 must
provide authenticated settings and approval controls, origin protections and
bounded streaming. Protected credential persistence also remains application
work; this transport reads no credential file or environment variable.

## Contract verification

The reserved command completed successfully:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 cargo test --locked --offline -p agentmage-platform-linux-inference --test local_chat_transport
```

All **17 tests passed**, zero failed, ignored or filtered. Cargo reported 6.45 s
for compilation and 0.47 s for the tests; supervised wall time including the
shared-slot wait was 266.65 s. The preceding session's two 600.08 s reservation
timeouts started no compiler or test and remain failed attempts to obtain the
resource, not test results. No reservation or resource limit was bypassed.

The contracts run the transport against synthetic servers on actual ephemeral
loopback sockets. They exercise credential routing, refusal before I/O for a
different address/path or pre-cancellation, invalid keys/limits, model listing
and bounded IDs, uncredentialed discovery, IPv6 and close-delimited responses,
bytewise chunked UTF-8 streaming before completion, error/status redaction and
redirect refusal, malformed/ambiguous headers and framing, oversized responses
and chunk-overhead accounting, premature EOF, late bytes, stalled physical EOF,
total/idle deadlines, cancellation from another thread and a text callback,
socket closure, and ambient proxy refusal in a child test process.

The synthetic contract servers are not model inference. No prompt, agent loop,
sandbox, repository workflow or model admission was tuned or qualified here.

## Complete touched-crate gate

The unchanged fast gate ran from batch base
`8e931557af05261085821581b82b44d8e594c3e6`, including untracked additions,
with its normal shared reservation, two build jobs and four test threads:

```sh
python3 scripts/local_agent_fast_gate.py --base 8e931557af05261085821581b82b44d8e594c3e6
```

Its result is **FAIL**, exit 1; Cargo tests exited 101. All 53 sorted failed
identities exactly equal the [host baseline](t301-baseline-failures-2026-10-25.md#host-library-53),
with no new or missing failure. This comparison is not an allowlist or a pass.
The lane still presents trusted system executables as owned by UID 65534;
T-301.1 remains a deferred owner-action blocker, not a transport prerequisite.
No trust check, test, threshold, feature default or ignore marker changed.

| Package, all targets | Passed | Failed | Existing ignored | Filtered |
| --- | --- | --- | --- | --- |
| Inference | 202 | 0 | 8 | 0 |
| Host reverse dependent | 532 | 53 | 11 | 0 |
| Total | 734 | 53 | 19 | 0 |

| Gate step | Exit | Seconds including reservation wait |
| --- | --- | --- |
| Range whitespace | 0 | 0.10 |
| Untracked whitespace | 0 | 0.10 |
| Markdown lint, two guides | 0 | 0.20 |
| Touched-package formatting | 0 | 0.85 |
| All-target warnings-denied Clippy | 0 | 36.46 |
| All-target tests, no fail fast | 101 | 101.75 |

All 17 transport tests also passed inside this complete gate. The existing
standalone process fixtures remain synthetic evidence of their own paths; they
do not use this transport or a real model. Private raw logs, timings, exact
failed-identity comparison and tested-source hashes are retained in lane state.
Final evidence and queue prose received a subsequent Markdown/whitespace check;
the Rust source is unchanged from the tested bytes.

With Cargo idle, the reserved policy/link validator passed for 687 Markdown
files and all policy invariants, in 323.89 s including the shared-slot wait.
This is the ordinary validator, not T-310's long documentation gate. A malformed
private Markdown scratch document separately produced two lint errors and exit
1, confirming the linter rejects invalid input. It is not a product fixture.

## Preservation and remaining work

No dependency was added or updated. This is original implementation in the
project's own terms; no external product implementation or documentation was
consulted, and no model was acquired. Existing licence/notice reconciliation
from the [inventory](t301-2-engine-inventory-2026-11-02.md#import-boundary-and-licences)
remains unfinished; this batch does not claim import-wide licence readiness.

The historical ledger below the active mission queue, status model, Cargo
manifests/lock and supply-chain outputs remain unchanged. Source edits were
completed before measurement; no per-edit SBOM or historical evidence cascade
was regenerated and no evidence binding was narrowed. The unchanged long
documentation gate remains scheduled once at T-310.

T-302 still needs the authenticated versioned local API and one recorded real
streaming model conversation through the GPU slot. No GPU/model process,
off-machine service, model download, install kit, public release or owner-test
readiness was exercised. No mission row closes on this builder evidence.
