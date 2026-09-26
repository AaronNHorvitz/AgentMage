# Decision 0084: Isolated Public Research Transport

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-25 |
| Authority | Decision 0054 and the owned Rust implementation scope in Decision 0081 |
| Scope | AMR-02.3 and the transport prerequisite of AMR-03 |

## Decision

Extend the existing Linux platform adapter with an explicitly opt-in public-research
worker binary and mediated effect driver. Reuse the current authority transaction,
grant consumption, cancellation, process confinement and canonical receipt/artifact
owners. Do not add another execution loop, scheduler, database or browser engine.
The ordinary coding, read-only and command workers remain offline and unchanged.

The feature is absent from the default production dependency closure. Use pinned,
established Rust URL, HTTPS and TLS libraries inside the separate worker, with
unneeded features disabled. A source-only declaration or Cargo feature is neither
runtime activation nor permission to contact a destination. Production and model
admission, independent review, platform and release gates remain separate.

Before enabling this path, extend the existing closed source/dependency inventory to
distinguish the default offline closure from the explicit connected-worker closure.
Preserve the complete Cargo manifests, lockfile, first-party trees and SBOM bindings.
Do not delete the network dependency denylist, admit network APIs in the kernel or
ordinary tools, exempt a directory from scanning, or replace the full SBOM with a
partial manifest. Both closures require exact reviewed inventories. Negative tests
must reject accidental default activation, changed feature topology, undeclared
dependencies and socket/client imports outside the worker boundary.

## One consumed operation, one owned worker

The parent receives a nonforgeable `EffectAuthorization` only after the existing
coordinator consumes the exact grant. Before spawning, match the operation class,
tool/version, canonical arguments, policy-evaluated network scope, expected effects,
current task/workspace identity and independently prepared request. A research plan
or budget from Decision 0082 cannot mint this authorization.

Pass only a closed versioned request and its exact resource/disclosure restrictions
through private operation-scoped input. The worker receives no workspace handle,
canonical store, model, credential reference, browser state, cookies or ambient
environment. Verify its executable/runtime identities and confinement before use.
Its only admitted operation is a bounded public HTTPS GET; no arbitrary method,
request body, caller-selected headers, executable download or tool dispatch is exposed.

Keep explicit operation/task identities, deadline, cancellation, schema rejection and
typed redacted outcomes in the producer/consumer fixtures. Persist the task budget
reservation through the existing owner before dispatch. Failures and uncertain
external attempts consume their reservation; restart never silently retries them.
Cancel or expiry stops only the owned worker and verifies cleanup. A cancelled or
unknown in-flight request is not reported as an undone external disclosure.

## Destination and response checks

Use an established URL parser and accept only unambiguous HTTPS on port 443, exact
canonical public DNS names and the independently approved destination set. Reject
userinfo, unsafe syntax and detected secret material before disclosure. Query privacy
and model inference routing are separate permissions. New source links require new
operations; retrieved text cannot expand destinations or authorize local effects.

Resolve the complete bounded answer set inside the worker. Reject empty, oversized,
mixed-private, loopback, link-local and special-purpose results using the existing
conservative classifier. Pin connection attempts to the admitted addresses while
retaining the original hostname for certificate validation. A second ambient DNS
lookup or an unchecked fallback is not allowed. Default library DNS truncation or
an unowned timeout thread cannot establish this boundary.

Disable ambient proxies, automatic redirects, cookies, credentials and decompression.
Perform each allowed redirect explicitly with fresh destination/DNS/peer/TLS checks
and the original aggregate budget/deadline. The first worker only permits same-origin
redirects; cross-origin redirects are refused even when another host appears in a
broader research plan. A zero-hop restriction remains valid. Connection reuse must not
carry authority between operations or bypass revalidation.

Bound status handling, header bytes, body bytes, redirects, media types and elapsed
time. Reject unsupported encodings, archives, attachment/executable downloads and
ambiguous responses. Retrieved content is inert untrusted data; scripts are never
executed. Its observed URL/hops, retrieval time, media type and complete body identity
must bind to the operation receipt before canonical retention and claim-level reporting.
Provider metadata or a model-supplied citation hash alone is not retrieval evidence.

## Verification and remaining gates

Implement the closed packet and authorization fixtures first, then the isolated
transport and adversarial process tests, then host/provider composition. Cover missing,
expired, revoked and mismatched authority; secret/injection canaries; private/mixed DNS,
rebinding, proxy and redirect refusal; TLS-host mismatch; response/resource ceilings;
clock drift, cancellation, uncertain outcomes and no-replay recovery. Check default
offline behavior after enabling and disabling the optional capability.

Deterministic fixtures and CPU-only public-provider observations do not qualify a
local model. Actual research-to-code qualification requires the same CLI/host/tools,
canonical reports and verifier with the separately admitted exact model in a scheduled
GPU phase. Retain every failure. No new account, paid provider, model/runtime download,
license change, global service or publication/release authority is supplied here.

Library evaluation includes ureq's documented
[custom transport and resolver](https://docs.rs/ureq/3.4.2/ureq/struct.Agent.html#method.with_parts).
The final selected dependency versions and enabled features belong in the exact
manifest/lock inventories and tests, not in an assumption of compatibility with HTTP.

## First component implementation, 2026-09-26

Status: **Accepted under owner delegation, 2026-09-20**. Authority: Decision 0054,
within the unchanged scope of this decision. Pin `ureq` 3.4.2 (defaults disabled,
`rustls`) and `url` 2.5.8 (defaults disabled, `std`) in the optional binary's closed
dependency class. Preserve distinct default and connected normal/build feature
contexts and the full lock/archive/license catalog. No dependency download or
repository license change is involved in this increment.

Use fixed `/input/request` sealed read-only input rather than stdin because the
existing native sandbox uses stdin to install seccomp. Retain the original worker
deadline for all hops, count informational headers against the header ceiling, and
bound TLS ciphertext separately from plaintext/body accounting. These choices do
not admit the worker or change the ordinary offline runner. The
[component record](../verification/research-worker-component-2026-09-26.md) separates
parser fixtures from the remaining native/TLS/provider/model gates.

## Durable reservation implementation unit

Status: **Accepted under owner delegation, 2026-09-20**. Authority: Decision 0054,
within AMR-03.1.1 and this decision's unchanged security boundary. Store research budget
roots, bounded append-only revisions and their current head in the existing encrypted
operational store, with normal hash-bound schema migration and integrity/recovery checks.
Do not introduce a separate database, runtime event sequence or artifact owner. Retain
the full prepared plan through the existing report-artifact path and recheck its complete
payload, ownership and lifecycle before admitting an attempt.

At most 128 accounting revisions may belong to one task budget; reaching this additional
ceiling denies further attempts rather than resetting accounting. Original start/deadline,
trusted clock high-water, spent operations and terminal cancellation/expiry survive reopen.
Quota refusals which change accounting must commit that change before returning. A failed
durable commit cannot issue a native reservation proof. Such a proof is single-use and
only an additional restriction: the exact consumed grant, native admission/confinement and
verified owned cleanup remain separately mandatory. These choices do not activate research
tools or close any component, model, independent-review or release gate.

Move the existing pure research-plan/search/citation schemas and validators into the
kernel's shared contract implementation, retaining host compatibility exports. This lets
the canonical owner decode the exact full prepared plan using the same implementation,
without a reverse kernel-to-host dependency, duplicate validator or opaque hash-only plan.
Transport stays in the separately inventoried Linux worker; these shared types perform no I/O.
