# Retained endpoint terminal client

Date: 2026-11-08. Rows: T-303/T-306, non-closing. Tier: core.
Predecessor: `7defa427d3179554920086b1b5f3e8c2875aa7e5`, immutable. Its
[independent core review](../reviews/7defa427d317-local-coding-endpoint-http.txt)
arrived during this batch with **Verdict: PASS**, no high/medium finding, and is
retained byte-identically. This accepts only that HTTP increment. Low L1's
per-advance disclosure extent is clarified in the terminal guide, pending
independent review here; L2's HTTP dependency-fault and explicit Host/Origin
regression coverage remains backlog in these same rows. No new row or sub-row.

## Delivered behavior

The existing `agentmage-code` executable now consumes the retained endpoint
HTTP routes. It inspects the sealed request, advances with fresh explicit
model-disclosure consent, responds to exact protected approval challenges,
inspects individual canonical events, latches cancellation and separately
confirms release. It uses the existing bounded loopback-only HTTP client and
control credential. It creates no new coordinator, authority, tool, model
transport, run identity or background worker.

Each effectful command pins the service's original request identity and digest.
Before dispatch it verifies the fetched request and checks its run, digest,
ephemeral mode, no cursor and fixed process-only configuration. Tool approval
checks the complete challenge digest, run/task and visible tool identity, shows the
protected arguments in readable JSON and requires exact terminal confirmation.
The host still validates the original pending challenge and its current expiry.
Separate disclosure consent follows even for denial: a continued run can send
tool observations to its selected model. Declining either prompt sends no
approval. Piped consent is refused before connecting. Cancellation discloses
nothing to a model and requires no terminal. Release has a distinct confirmation
warning about process-only record loss and spent identities.

The client preserves the endpoint envelope and explicitly labels the step as a
service observation. It checks response shape and identity, without claiming an
accepted completion proof or independent success verification. Event inspection
checks individual canonical digests and exact run/session/task/policy/sequence
bindings, bounds each page to one event and checks index consistency. It labels
that limited verification and never treats an index as a recovery cursor.
All output uses the existing terminal escaping. No refusal or lost reply is
retried; no implicit cancellation or release masks uncertainty.

## Verification

The complete ordinary fast gate selected only `agentmage-local-api`, the changed
crate. It passed formatting, strict Clippy, documentation and all nine unfiltered
lib/bin/integration targets: **135 passed, zero failed, zero ignored, zero
filtered**. All 127 predecessor passes remain, plus seven terminal methods and
one actual retained-HTTP consumer method. No target is selected by its result.
The unchanged broader native source-gate limitations and deferred T-301.1 remain;
this API gate is not whole-repository or native-workflow acceptance.

The six commands took 555.56 seconds including shared reservations. The format
step spent 509.00 seconds including its shared reservation; strict Clippy took
22.10 seconds and tests 22.76 seconds, likewise including reservations. All six
returned zero. Five Rust source/test hashes were fixed before the gate and
remained unchanged through it. No source edit or additional Cargo gate followed.
The predecessor review arrived after the plan froze; its text lint and final
current documentation/hygiene checks are performed separately without repeating
Cargo verification.

The initial focused development run had five passes and one failure: the new
fixture expected an explicit null hunk selection, whereas the existing approval
contract omits it when absent. The new fixture was corrected to that unchanged
contract; old assertions are preserved. The later focused run passed all seven
terminal methods. Its name filter selected no HTTP methods; the full gate above
ran the entire HTTP target unfiltered, including the new executable consumer.
Both development logs are retained.

Seven methods execute the binary under real terminals against a recording
synthetic HTTP peer. They assert exact control credentials, routes and bodies;
fresh model consent; independent tool/disclosure confirmations; no posted
approval after either refusal; explicit denial; pipe refusal; tampered and foreign
request/challenge rejection; rejection of ambiguously repeated JSON arguments;
no retry after a refused or lost reply; cancellation/release distinctions;
paging bounds; and truthful terminal observation identity. The eighth method
executes the same binary against the actual retained HTTP owner, reads its
request, cancels it and checks every nonempty canonical cancellation-event page.
It asserts zero tool executions and zero upstream connections. Synthetic ports
remain explicit; none of these results is a native workflow or model score.

Private raw logs, source copies, plans, audits and final input copies are kept
in the lane validation record for scope `local-coding-endpoint-terminal`.
`run.py` executes the ordinary gate; `audit.py` compares all nine API targets to
the predecessor's complete gate; `final.py` checks current documentation,
unchanged ledger table rows, review identity, public hygiene and exact file
copies. No source outcome is waived.

| Frozen Rust input | SHA-256 |
| --- | --- |
| `platforms/local-api/src/coding_cli.rs` | `f5f1fb20c667c673ba331b490a1672698e6047d7da7e5675248f153a95f5dd99` |
| `platforms/local-api/src/coding_cli_endpoint.rs` | `05a6856bc52a98040a61fa05273942fce7071a9749593cb9c72ea80d7042bb67` |
| `platforms/local-api/tests/coding_endpoint_http.rs` | `79523791bdb2ddbbec205123c6a79c0cd5bb637f890a0522f90e98c958f84d0a` |
| `platforms/local-api/tests/coding_runtime_cli.rs` | `c83395ffe5e927a7703439d4e5815bfd027285b6f5ab1dcf8288c047f0a5bf3a` |
| `platforms/local-api/tests/support/coding_endpoint_cli.rs` | `885b02f8d8e0573b83ab32737d71a32926ea44b573c382885b2a74dd8e0eba4c` |

## Remaining work

Native filesystem/grant composition and executable service activation are still
open. The terminal client does not make the library service installable. The
tests use synthetic endpoint/tool/permission observations; they are not coding
scores or a native filesystem/grant run. MCP endpoint projection, native
edit/test/retry, durable recovery, approved Git and parent measurements remain
open. T-303/T-306 stay open until exact-commit independent acceptance. Existing
low findings remain backlog in these same rows. No evaluation suite, hidden
test, held-out set, scorer, threshold, ignore list, dependency, native admission,
model, GPU, supply-chain output, status model, scoreboard, push or release changes.
