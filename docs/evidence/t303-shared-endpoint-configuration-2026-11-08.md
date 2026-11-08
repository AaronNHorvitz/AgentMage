# T-303/T-306: retained endpoint configuration

Date: 2026-11-08. Tier: core. Predecessor:
`08a82f80297130cce0fc201b702a0749b518a563`.
Both mission parents remain open; no row or sub-row is added or closed.
The predecessor is under independent review; this increment neither rewrites
it nor supplies an independent verdict.

## Change

The endpoint coordinator previously borrowed its configured transport and
approval for its complete lifetime. The API owns replaceable configuration
through `Arc`, so retaining such a coordinator in the service required another
configuration ownership form. `CodingEndpointConnection` now supports the
original borrowed objects and shared ownership of those same immutable objects.
Admission, run-bound context, model selection and the original coordinator use
one path. The API retains its original transport and approval individually
through `Arc`; its existing settings handlers, consent, operation lease and
manual request behavior are unchanged.

A shared prepared request holds the exact original credential-owning transport.
Preparation delegates to ordinary `prepare_chat` before transferring the frozen
binding and zeroizing body into shared ownership. It performs no I/O. Approval,
encoding, limits, hashing, cancellation and dispatch have no alternate logic.
Credentials are never cloned or copied into the kernel. The owning coordinator,
request digest, context, clock, events, budgets, exact approval challenge and
spent identity remain the original retained instance. Releasing it drops its
configuration references; no permanent allocation or background worker exists.

This is a prerequisite for HTTP composition, not an enabled HTTP run route.
There is no new file, tool, permission, command, endpoint or resume authority.
The eventual service must enforce current settings revocation and workspace
selection before dispatch; retaining old consent does not establish such a
policy. Actual native scoped filesystem/grants, service-issued identity,
edit/test/retry, durable recovery, approved Git and measurements remain open.

## Verification

Three new test methods use actual loopback sockets and synthetic models and
source/tool/grant fixtures. They establish no real-model or coding score.

- Shared transport: both usage settings produce the same binding as ordinary
  preparation; the original body, API path and credential reach the socket after
  caller settings/prompt references are dropped. The request moves to another
  thread without a borrowed configuration. Observations exclude private text
  and keys. A weak reference proves release after dispatch, with no extra socket.
- Shared transport refusals: wrong destination approval and invalid model/token
  requests refuse without I/O or retained references. All four run-control stops,
  transport cancellation and unused-request drop preserve no-connect behavior
  and release the retained configuration.
- Shared owner: an explicitly static owner survives caller configuration drop,
  waits for exact approval without tool execution, rejects a replacement sealed
  destination without changing events, then performs one approved synthetic read
  and a second model turn through the same original transport. The unchanged
  fixture verifier establishes a no-op terminal. Repeated terminal reads do no
  work; release drops transport and approval, and the identity remains spent.

Initial development compilation exposed three old admission literals needing
the new connection field. The first owner execution also exposed an incorrect
new fixture summary; it was corrected to satisfy the unchanged fixed verifier.
The focused transport methods and corrected owner method pass. A broad `shared_`
development filter also includes an existing native-manifest failure; it is not
a passing acceptance gate. Final unfiltered verification is recorded below.

## Final unfiltered gate

The ordinary fast gate over all three affected crates and all their targets
finished with **936 passed, 53 unchanged failures, 19 unchanged ignores and zero
filtered**, across 27 targets. All 933 prior passes remain; exactly the three
new methods pass. Failed and ignored identities and normalized failure
diagnostics match the prior complete logs. No target was omitted for its result.
The source gate remains **FAIL** on the deferred T-301.1 native-manifest
blocker; nothing is waived or presented as an overall passing source gate.

Formatting and strict Clippy pass. All six commands completed in 213.87 seconds
including shared build reservations (0.10, 0.10, 1.30, 78.30, 20.70, 113.37).
The eight changed Rust files stayed hash-identical throughout the complete gate.
The final lightweight checks rerun only the audit, documentation and hygiene,
without repeating source compilation or tests.

Baselines are all 18 host/API targets from the immediate predecessor's full
43-target gate, plus all nine inference targets from the full 52-target gate at
`f1ac4d326c42b9614c0bc148e2bbcf2af0ba1d3a`. The entire inference crate is
unchanged between that tested commit and this increment's predecessor. Private
validation retains the complete raw logs, exact source copies, command plan,
source hashes, target identities, diagnostic comparison and review helpers.

## Limits and independent review

No evaluation task, hidden/held-out test, scorer, threshold, dependency, native
admission, ignore, model, GPU, scoreboard, SBOM, status or release change.
Existing evidence remains bound to its historical inputs. No supply-chain
regeneration occurs during this source batch. Independent core review of the
exact final commit is required; implementation tests cannot close either row.
