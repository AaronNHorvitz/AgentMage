# Canonical research report component, 2026-09-27

This component implements the report projection in [Decision 0085](../decisions/0085-canonical-research-report-projection.md).
It does not close AMR-03.1.2 or enable ordinary host/provider research. The source
base is `eef257aadc077a259b676e0b24232013162ac997`; exact candidate inputs and
private log identities are in the [component manifest](research-report-component-2026-09-27.json).

## Implemented boundary

The existing canonical authority owner freshly checks every complete source bundle,
terminal transaction, receipt, grant, runtime history, native producer identity and
artifact lifecycle while holding its existing store mutex. Reports carry exact
source-body hashes, UTF-8 byte spans, original retrieval times and current accounting
identity. Observed claims establish excerpt presence only. Model interpretations
retain limitations; proposed conflicts and unanswered questions remain visible.
Invalid evidence refuses the report, including when a draft requests partial output.

The closed draft and output have explicit size/count bounds. Repeated source bodies
and structured locations share quotation allowances. Detected secrets are refused.
Cancellation and original deadline expiry remain explicit on reopening, without
replay, reservation refunds or time resets. Serialized reports are inert content;
later use still requires a canonical source read. There is no report publication
receipt, new grant issuer, database, network client or workflow-completion label.

The optional worker records URL spellings at its existing request loop. Explicit
wire version two nests the unchanged version-one observation. The original frame
decoder remains supported, with no invented URL for older sources. A frozen original
frame digest independently checks byte compatibility. Representation restrictions
in the kernel do not replace the worker's pinned URL library or native provenance.

## Executed checks

All Rust builds and suites ran through the lane's shared build reservation, with
one Cargo job and single-threaded tests. Dependencies remained locked and subsequent
commands used the offline cache.

| Check | Result |
| --- | --- |
| Full kernel engine library | 1,200 passed; 7 ignored native/external cases remain ignored. |
| Engine documentation tests | 13 passed, including refusal to deserialize or clone a canonical report handle. |
| Host report error mapping | One targeted regression passed; wrapped integrity/storage errors retain their existing disposition. |
| Optional public-research worker | 21 passed; three namespace cases ignored by the ordinary test invocation. |
| Separate namespace transport fixture | Two DNS/pinned-connection tests and one TLS matrix passed; synthetic CA and isolated namespaces; owned server reaped. |
| Strict workspace and optional-worker Clippy | Both passed with warnings denied. |
| Python diagnostic and boundary suites | 90 passed across eight suites. |
| Direct source/effect/dependency audits | Passed, including default offline closure and explicit optional worker inventory. |
| Formatting and whitespace checks | Passed. |

The namespace fixture executed before the final fixed-digest assertion was added
to the engine's framing test. Transport production code and the fixture helper did
not change between those checks. Its two DNS tests and TLS matrix are real isolated
transport exercises using synthetic destinations/certificates, not public-provider,
native worker admission or model qualification.

Canonical report regressions cover old/new response versions, receipt aliases,
forged source/context/native identities, clock rollback, missing/released/corrupt
artifacts, poison propagation, Unicode boundaries, aggregate quotation limits,
secret refusal, limited interpretation, conflicts and partial reports. They also
cover cancellation recorded before a budget append and cancellation/expiry after
reopening. The transport test checks actual request-library query and redirect URL
spellings. These are component tests through real owners with synthetic effects.

## Failures retained and corrected

The first offline attempt failed on an unavailable cached dependency. A locked fetch
completed without changing manifests or acquiring a model. An initial compilation
error in digest formatting was corrected. A new compatibility test initially compared
different JSON map orders; the expectation was corrected and then independently
pinned to original frame bytes. The strict source inventory rejected two new inert
URI templates; only those exact per-file strings were enrolled. No network API or
directory exemption was added. Workspace Clippy caught a missing exhaustive host
error match; the match and its regression were added before final checks.

Two inherited authority digests were stale after the owner's IP-policy editorial
updates to `ENGINEERING-RUNTIME.md` and `RUNTIME-BOUNDARIES.md`. Their existing exact
authority sets were retained and only the changed digests were renewed. This does
not promote either accepted plan to an implementation claim.

## Uncompleted acceptance

The source checkpoint is `50f0dbb9728a68657c404f0f00cd53feef5e192a`. The actual
rebuilt CLI/host recheck at that source again exited 5 before tools, with an empty
event stream and a clean fixture. Its binary identities and invocation receipt are
retained in the component manifest. No source changed during evidence renewal.

The actual coding CLI/host launch remains separately documented in the
[restart reconciliation](lane-reconciliation-2026-09-27.md) and
[local launch instructions](../LOCAL-TESTING.md). The implementation lane cannot
provide the native executable ownership and user-systemd bus required for that
workflow. Its confinement checks were preserved. No fresh successful coding
edit/test/repair, denial/cancellation workflow, real-model run, public-provider
campaign, manual user test, independent acceptance or release is claimed here.

Native adversarial acceptance for the current worker, canonical report publication
and consumption, configured provider composition, research-to-patch execution and
their independent gates remain open. No task checkbox or production status changes.

## Batch evidence renewal

The reserved core driver exited zero after exactly one full SBOM write. It passed
547-document lint, runtime schema checks, 186 Python regressions and the affected
architecture, contract, component, configuration and security-map builders. The
reserved storage continuation exited zero after executing its migration, lifecycle,
atomicity, crash, resume, security and local acceptance checks, plus 82 builder
regressions. Counts overlap other checks and are not a unique-case total. Exact
driver log identities are in the component manifest.

The publication scan caught one private repository path in a generated Cargo
transcript. Its complete original is retained privately; the public transcript
replaces only that exact root with `<repo>`. Reverse substitution verified that
every other byte is unchanged. The manifest records both digests and the single
replacement. The transcript validator and dependent whole-file bindings were
renewed, with 20 targeted regressions passing. No result, error, input set or gate
was removed. The SBOM was not rewritten.

Inspection found unchanged acceptance, limitation and platform-status fields.
The builders retain fixed historical `generated_on` values; this renewal's actual
execution is identified by the source and log records here. The precommit direct
inventory examined 751 JSON artifacts and 704 named whole-file bindings to changed
inputs: 105 current, three newly stale and 596 previously stale or historical.
It excludes transitive, unnamed and 1,602 line-span bindings. This is not a global
freshness claim. Historical native/model/platform records retain their original
pins and are not current acceptance of this increment.

The three newly stale bindings were in the Story 11.2 automated gate. After the
evidence commit `f05da1718f6baa9cac36c7b5a4706f7bf615fd04`, its immutable review
pin advanced to that commit and tree under AGENTS section 7. The reserved gate
build, validation, eight regressions and unchanged-SBOM check passed. Its path
set, dependency blockers and blocked story status remain unchanged. This routine
automation does not provide independent model review or satisfy a human-only gate.

The final direct inventory has 121 current and 583 previously stale or historical
bindings, with no newly stale binding in its bounded scope. Its exclusions remain
unchanged. This does not establish freshness of the whole historical evidence tree.
