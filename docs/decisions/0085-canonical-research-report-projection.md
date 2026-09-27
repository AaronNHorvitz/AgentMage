# Decision 0085: Canonical Research Report Projection

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decision 0054, Decision 0081 and the owner's current implementation restart |
| Scope | AMR-03.1.2 source-linked reports and their worker URL prerequisite |

## Decision

Assemble reports through `DurableAuthorityRuntime`, holding its existing canonical
store mutex while freshly reading every declared complete source bundle. Reuse the
current transaction, receipt, grant, journal, payload and lifecycle checks. Returned
source handles and provider snippets cannot substitute for that fresh read. Do not
introduce another store, execution loop, grant issuer or network client.

Admit bounded closed drafts with exact UTF-8 byte spans, source-body digests and
claim links. An observed claim establishes only the presence of its exact excerpt.
Interpretation retains the existing model-inference label and an explicit limitation.
Proposed contradictions remain unresolved; no model-selected deterministic derivation,
publisher-truth, independent-corroboration or workflow-completion label is accepted.
Missing or invalid canonical evidence refuses the report instead of becoming a
partial-success excuse. Declared unanswered questions can produce a partial report.

Preserve downward-only original task limits, with additional ceilings of 20 sources,
64 claims, 128 spans, 32 conflicts and 32 unanswered questions. Bound draft JSON to
128 KiB and output JSON to 256 KiB. Each excerpt is at most 1,024 bytes; aggregate
declared quotations are at most 25 whitespace-delimited words per complete body and
per structured location. Copies and repeated excerpt fields share their allowances.
This is an excerpt quota, not a general plagiarism detector or a semantic fact checker.
Detect secret material before exposing report text. Failures are content-free.

Report reads retain original retrieval clocks, current read time and accounting-head
identity. Publication date, publisher identity, cache freshness and source independence
are not inferred. Cancellation and original deadline expiry remain explicit; reading
retained evidence cannot refund a reservation, extend a deadline or replay an effect.
Serialization is inert content for the existing artifact owner, not a publication receipt.
Later consumption must recheck original sources and their lifecycle.

Add explicit response wire version two for ordered URL spellings captured from the
worker's existing pinned URL library at its actual request loop. Nest the unchanged
version-one observation. Keep the original frame readable with no invented URL, and
preserve the 64 KiB metadata ceiling and full frame/native-result hash bindings.
The kernel applies representation restrictions, not a second URL/query parser.
Syntactically consistent strings alone establish neither connection provenance nor
permission to follow a link. Current producer admission remains mandatory.
Enroll only the exact inert `https://{}{}` representation in the source inventory
for the response representation check and its canonical fixture. Keep all network
API and undeclared-URI denials intact.

## Verification boundary

Exercise old/new framing, actual worker-loop query/redirect projection, canonical
receipt and artifact substitution, current clocks, released or corrupt source,
UTF-8 boundaries, quotas, secret refusal, interpretation/conflicts, cancellation and
reopening. Separate synthetic transport and canonical-store cases from native
transport and real-model research. No ordinary host/provider research tool is enabled
by this component. Native adversarial, provider/model, independent review, human,
platform and release acceptance remain distinct open gates.

The restart lane's missing root-trusted native executables and user-systemd bus are
external requirements for live native tests. Retain the failed launch and improve
diagnostics; preserve the trust and confinement requirements. A ready fixture or an
executable's presence is not proof that a native workflow can launch in that lane.
