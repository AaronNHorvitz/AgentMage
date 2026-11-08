# T-303 / T-306: concrete endpoint coding context

Date: 2026-11-08. Tier: core. Immutable predecessor:
`f1ac4d326c42b9614c0bc148e2bbcf2af0ba1d3a`.
Both parents remain OPEN; no row or sub-row is added. Independent core review
is required. The predecessor is under separate review; this is no self-approval.

## Change and authority

`CodingEndpointContext` replaces the need for a test-only context builder at
library composition. Its convenience constructor joins the original configured
transport to the same sealed read-only run, max-output request and existing
coding completion schema. This is not another model or orchestration loop.

The caller supplies consented source snapshots. The existing deterministic
source selector keeps file digests/counts and exact visible byte ranges, using
its unchanged 2–32 KiB allocation. No path, process, credential or network is
opened by context preparation. Selected source coverage is not a claim about
current filesystem bytes; the caller owns freshness and disclosure consent.

Complete original model-visible definitions and input schemas are obtained
through the existing native catalog projection. Shared schemas are serialized
once, without removing any original definition or schema. The system contract
asks for the existing endpoint plain-JSON tool proposal or coding completion
candidate. Source data stays in a separate user message. Exact task and current
calls, results, evidence and refusals are serialized from the original supplied
coordinator observations. Model prose grants no authority and verifies no check.

The context pins the sealed request digest and consecutive turn ordinals.
Foreign/unsealed requests, mismatched call/result bindings and repeated/skipped
turns refuse; a refused preparation stays closed. The borrowed control is
checked before/after preparation. The dynamic writer stops before crossing the
content limit; the existing kernel packet checks complete message-array bytes
including escaping. Overflow refuses all context, with no history truncation.
Body, cumulative resource, transport and permission checks remain downstream.
No native profile, local token count or server capacity is invented.

Application context buffers are transient ordinary process memory; this change
does not claim secure erasure. The existing transport's frozen-body erasure is
unchanged. A fresh builder restores no ownership, clock, budget or grant.

## Verification

Seven new methods pass in the focused development run and in the complete
unfiltered source gate. They cover original definition/schema preservation,
exact source spans and untrusted roles; foreign/unsealed requests and repeated
or skipped turns; catalog/path/message-count/source-budget refusals; complete
current result/refusal inclusion and call/result mismatch; exact UTF-8 content
and escaped message-array ceilings and all owner-stop classes; two real socket
turns through the existing approval/tool/verifier loop; and context overflow
ending as Exhausted before socket or permission entry. The socket completion
uses the existing coding-candidate shape and a synthetic fixture verifier.
Terminal re-reading is idempotent. These are component results, not live coding.

The full gate ran the host and local API, all targets, without filters:

```text
python3 -B scripts/local_agent_fast_gate.py --base f1ac4d326c42b9614c0bc148e2bbcf2af0ba1d3a --timeout 3600
```

The gate used its own per-Cargo shared reservations, two build jobs and four test
threads. Format and strict Clippy PASS. The source gate is **FAIL** on the
deferred baseline: 709 passed, 53 unchanged failed, 11 unchanged ignored,
zero filtered, 18 targets. Comparison with the predecessor's complete source
gate preserves every old passing identity, adds exactly seven context methods,
and preserves exact failed/ignored identities and normalized diagnostics. All
five Rust input hashes stayed fixed throughout. The six command durations were
0.10, 0.15, 1.20, 0.85, 27.80 and 124.23 seconds: 154.33 seconds including waits.
The private final audit reruns only lightweight checks, not another Rust gate;
Markdown, hygiene, unchanged ledger table rows and native fixture/transport
preservation PASS. Retained raw logs and exact source copies are identified by
the private review-input manifest.

Development failures remain retained: the first build found unsupported digest
formatting; the next run passed five methods and found two new fixture errors.
The socket fixture reused a reader requiring its old synthetic context field;
a separate reader now keeps the same HTTP/auth/model/usage/size checks and
inspects the actual new context. Overflow correctly yields Exhausted, not
Failed. The subsequent focused run passed all seven methods. No original test
body, validator, threshold, ignore or production terminal behavior was changed
to resolve these fixture errors.

## Open work

Exclusive live run ownership, actual scoped filesystem/grant composition,
endpoint HTTP/CLI/MCP projection, controlled writes, edit/test/retry, durable
recovery, approved Git and both parent measurements remain open. Socket models,
permission owners, snapshot tools and verifiers in this batch are synthetic;
they are not a local-model coding result or live native filesystem authority.

No GPU, real-model, evaluation, hidden/held-out, scorer, threshold, ignore,
dependency, SBOM, status-model, push, release or scoreboard change is made.
The new context instruction has not been measured on development tasks; the
agent campaign remains required before parent closure. No held-out input was
read or tuned.

Final source-gate raw log SHA-256:
`1f16ee7c31047ea8578d3969d2aceab412886d379764f2f1cb01f84db9b7cfa0`.
