# Public Research and Citations

Public research starts with an explicit query, optional sorted domain allowlist, recency window,
allowed source classes, result ceiling, and aggregate-byte ceiling. Preparing a request performs no
network action. A separately owned provider may return only public HTTPS results for later
verification.

The first AMR increment rejects detected secrets in queries, requires exact canonical domain
names (subdomains need their own entry), and includes result metadata and JSON encoding in the
aggregate byte limit. The URL syntax prefilter is not DNS, TLS, redirect or SSRF validation.
The separate Rust research-budget restriction set freezes quick/deep limits under Decision
0082 and refuses offline, undisclosed queries, private/mixed DNS answers, ambient proxies,
replay, cancellation and exhausted budgets. Neither component contacts a provider or grants
network authority; live research remains open pending the mediated effect composition.

An optional Rust worker now implements the public HTTPS transport component under
Decision 0084. It uses the pinned URL/HTTP/TLS libraries, complete public-address checks,
numeric-address connections with the original TLS hostname, explicit same-origin redirects,
and bounded plaintext/ciphertext reads. It has no proxy, cookie, credential, decompression,
connection-pool or automatic-redirect path. Retrieved instructions remain inert bytes.

The `agentmage-platform-linux/public-research-worker` feature is absent from the default
production dependency graph and is not enabled by the host. Its exact connected graph is
separately inventoried, while complete manifest, lockfile and SBOM checks remain in force.
The worker expects a sealed request projection and native confinement; it is not a supported
standalone fetch command. The consumed-permit binding is a separate kernel check, not a
substitute for durable budget reservations or admitted native launch. Parser fixtures do not
establish real TLS, process cleanup, live provider or model qualification. See the
[component verification record](../verification/research-worker-component-2026-09-26.md).

Canonical research accounting now has a bounded encrypted-store component: full retained
plans, original deadlines, spent attempts and terminal cancellation survive reopen. Failed
attempts are not refunded. Read-only progress inspection cannot reset or consume the budget.
A retained reservation is not a grant or a fresh native-dispatch proof; the host still does
not enable research execution. See the separate
[durability record](../verification/research-durability-component-2026-09-26.md) for actual
tests, failures and remaining native/provider integration.

Verified results prefer primary documentation, original research, and authoritative records ahead
of secondary analysis. Every claim-level citation retains its title, direct URL, publisher,
publication time when known, access point, excerpt digest, freshness result, and a 25-word quotation
limit. Inferences remain visibly labeled. Excerpt content is not retained in the citation record.

Credentials, authenticated profiles, downloads, clicks, publication, and workspace effects are not
part of this contract. Unknown publication time is not fresh. Redirects, copied URLs, and provider
ranking do not grant authority or prove a claim. Browser execution and privacy review remain
separate required evidence before support is claimed.
