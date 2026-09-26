# Fresh Research Dispatch — Verified Component Scope

Date: 2026-09-26. Source parent: `96a473f4bfc56bf06e0d14e229ac6aff76c0828e`.
Scope: AMR-03.1.1 and the authority prerequisite of AMR-02.3.2. This is a
verified component increment, not an enabled research tool. The containing commit
pins its sources; the retained commands ran against the inspected working tree.

## Implemented restrictions

The existing durable authority owner performs research preflight after flushing queued
events and inside its existing store lock. Ordinary effects use the same coordinator
through a private no-op preflight; no public arbitrary callback, second execution loop
or new store is exposed.

The additional research path consumes its opaque spent reservation. It checks the
original task/session/run/policy, exact current accounting head, complete live plan,
canonical prior tool request, issued-grant hash and actual start-event bindings.
Clock observations can only restrict the original budget; they never reserve a second
operation, refund a failed attempt or reset the deadline. Persistence/integrity failure
poisons the runtime. Capacity refuses further dispatch even when cancellation could not
be appended; such a failure is not reported as persisted cancellation.

A private adapter passes separate borrowed freshness and consumed-grant proofs to the
research driver. Original grant issuance, consumption and exclusive expiry are retained
in the existing permit. Exact native binding additionally rejects a prepared request whose
deadline extends beyond that grant, including when the current launch time is still valid.
Preparing a shorter request requires separate approval of its own bytes; approved packets
are never rewritten to fit a later grant.

The optional worker now recognizes only its fixed guest working directory and canonical
locale/timezone values. DNS lookup uses an absolute name while the original approved
HTTP host and TLS hostname remain unchanged. These are integration corrections, not
permission to launch the worker or a change to the default offline dependency closure.

Engineering status: **Accepted under owner delegation, 2026-09-20**.
Authority: Decision 0054, within Decisions 0081 and 0084. Verification is separate.

## Retained verification and failures

The first focused command compiled successfully, passed 70 research tests and failed
three new canonical-owner fixtures before dispatch. Their reused artifact helper retained
a dummy policy digest while their grants used a real policy digest. The full-plan ownership
check correctly refused the mismatch. The fixture was corrected to publish under its
actual policy; no production validator was relaxed. The complete failed driver log is
retained privately with SHA-256
`9ab15fc1cb8547441f967d2da02dcee82226dc9f355b466132887c2966352fea`.

The second command again passed 70 tests and failed the three new fixtures, this time
because the fixture incorrectly classified progress events as correctness events. The
existing event validator rejected them. Its complete log SHA-256 is
`d0ce4c4d7d6ea7cfe9bec480ea1060e80e23bf8a669fb5138d6b918be8c0a5df`.
The fixtures now use the canonical classification and explicitly distinguish queued
progress from events flushed before a deliberate fault-injection reopen.

Source inspection also found that operation filtering would skip run-level cancellation.
The new preflight now checks durable cancellation before filtering by operation; added
cases cover requested/observed cancellation before budget cancellation and after reopen.
These regressions passed; this is not a claimed observed production incident.

The third driver exited 0: 74 focused research tests, 22 authority-transaction tests,
four borrowed-proof compile-fail tests, 19 optional-worker tests and 25 structural
boundary regressions passed. Counts overlap; they are not 144 distinct runtime cases.
Strict all-target kernel/host/Linux Clippy with the optional worker, effect-boundary,
runtime-ownership, module-inventory, formatting and diff checks also passed. The full
driver log SHA-256 is
`cd4b4bb24d023ba29abad945059e58bc9968c76f9c8699d40c4c50f4009d2acf`.
The synthetic recording driver does no DNS, HTTP or native process launch. Passing it
establishes only the tested canonical owner behavior, not retrieval or model success.

The broader driver passed the kernel library (1,135 tests, seven ignored), host library
(308 tests, eight ignored), default Linux library (147 tests, 45 ignored), 67 kernel
integration tests across 18 targets and eight compile-fail documentation tests. It built
the actual CLI, host, read worker and optional research worker. Six offline environment
diagnostics passed. It then exited 1 with three failures among 93 Python tests: the new
negative-canary module was named differently from the existing terminal `cfg(test) mod
tests` convention, so the unchanged source auditor scanned its proxy and URL canaries as
production. Its retained full log SHA-256 is
`ab1a37f07859dbf2d5dad8ed8e57ac94ccca3e639d027a330b613ccfab112dca`.

The test module now follows that existing convention. A new regression verifies that the
same strings placed in production are still refused; no production allowlist or scanner
exception was added. The private-preflight visibility guard also rejects crate/super
exposure. A remainder-only command exited 0: all 94 Python regressions, 19 worker tests,
optional-worker strict all-target Clippy/build, six offline diagnostics, source/effect/
dependency/status checks, formatting and documentation checks passed. Unchanged broad
Rust library suites were not redundantly rerun for the test-only rename and checker edits.
The full remainder log SHA-256 is
`8c780f431c26130c18add9fde84235a10cd17e24a6df84708f52304148d50f8b`.

All 22 actual-process scripted CLI/host cases passed, including genuine failing-test
repair, file creation, multi-file edits, rejection correction, cancellation, approval races,
stale/replayed approvals, cursor expiry, output overflow, false completion, native command
failure, artifact tampering and conflict-aware rollback. The complete private acceptance
report SHA-256 is
`7a2904dfaa7ac81499c6d78287e64099c9865aeaf6c1e7b7c3e8bb02685bad20`.
Its profile is `scripted-executable-fixture-32k-v1`, qualification
`executable-scripted-only`. These are not real-model or native research successes.

| Built executable | SHA-256 |
| --- | --- |
| CLI | `a4d28562b5663432ddce42432d476e06dc00bf7ac43c1c26e2386f63ff69e284` |
| Host | `a4ad2faa0ee81093364eb2c0fee50e42fe170d33054c95f28a2c38d8e16f819a` |
| Read worker | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |
| Optional research worker | `aae7b6063059119d1db2637656efb98b240229cc28ccd8fa0e2b23b998345e3f` |

The six environment diagnostics unshare networking and deny socket/connect syscalls.
They supply no request: the exact composed locale/working directory reaches input refusal;
missing/foreign working directory, ambient PATH and extra arguments fail earlier. Their
diagnostic-only read-only runtime mount is not the production minimal confinement manifest.
They establish the observed environment correction, not outbound/TLS admission or cleanup.

## Remaining work and freshness

Native confinement, bounded process supervision and verified owned cleanup must precede
host/provider activation. Receipt-backed complete source artifacts, resumable reports and
the actual configured-provider/local-model research-to-code campaign remain open.
Historical eight-case Muse evidence is not qualification for this source; GPT-OSS remains
unqualified. Independent review, human acceptance, platform and release gates remain open.

No task checkbox or milestone is promoted. Source edits invalidate affected whole-file
bindings; historical reports remain evidence of their original inputs, not current approval.
The source batch is complete. One applicable SBOM/dependent-evidence refresh follows these
checks without narrowing bindings. Historical 34-source/workflow-family lifecycle and
224-case crash fixtures retain their bounded scopes: renewing those leaf checks does not
establish exhaustive research-family coverage or a whole-current-store gate. Committed-blob
reports need their exact subsequent source pin; historical native/model acceptance is not
silently renewed.

The single refresh driver exited 0. Supply-chain generation/check and its 11 tests,
the contract boundary and index, configuration/dependency reports, kernel boundary
reports, and applicable source/workflow/storage leaf reports and their tests passed.
The inspected changes preserve their existing claims and input sets; only bindings,
source byte lengths, source revision metadata and measured test output changed.
The full retained driver log SHA-256 is
`e514038dbc01e4cd277153d870fe4765f1bbc34700b86f63bd1e7b636b39e1bb`.

A read-only inventory parsed 748 JSON files and found 623 named whole-file bindings
to tracked changed inputs: 75 current, ten newly stale, and 538 previously stale or
historical. This is not a transitive freshness audit or acceptance metric. The ten
newly stale bindings belong to the schema-3 snapshot, operational-store unit report,
Story 4.1 security map and Sprint 82 local report. Their committed-source follow-on
must use this increment's actual commit. Whole-current-store aggregates and historical
native/model gates retain the separately stated limitations; no automatic reacceptance
is claimed for them.
