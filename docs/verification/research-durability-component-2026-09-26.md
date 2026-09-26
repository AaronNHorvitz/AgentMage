# Durable Research Accounting — Component Verification

Date: 2026-09-26. Source parent: `5b731b36d646aa22baa78f310544ea6ebce668e3`.
Scope: the canonical-store component of AMR-03.1.1 under Decisions 0082 and 0084.
This is not an enabled native research tool, provider or model qualification.

## Implemented boundary

The existing encrypted operational store owns immutable task-budget roots, bounded
append-only accounting revisions and their current heads. Schema 19 adds these three
tables without replacing a store, coordinator, event sequence or artifact owner. Old
migrations and the schema-18 fixture remain intact. A task cannot acquire a fresh budget
by reopening the same store or changing its original run, session or policy.

The full prepared plan is retained through the existing report-artifact owner. Before
opening a budget or returning a request reservation, the implementation verifies the
complete artifact payload, exact owning context, lifecycle and canonical publication
event. It reuses the original shared plan decoder, moved into the kernel with host
compatibility exports, rather than duplicating validation or trusting a preview hash.

Every successful reservation commits before returning an opaque accounting object.
Failed, dropped and uncertain attempts receive no refund or transparent replay. Failed
quota observations retain clock advancement; observed expiry, clock rollback and
cancellation stay terminal after restart. The original start and restriction set cannot
be replaced during recovery. The additional 128-revision ceiling fails closed.

A read-only resume projection exposes original counters and a verified head identity;
polling it consumes no revisions and is not a current-clock or admission attestation.
Backup and fresh restore use the existing encrypted owner. Derived exports include all
three research families as identity hashes and retained record hashes, with no raw plan,
query, destination or task identity. They remain nonexecutable and cannot restore authority.

## Observed checks and retained failures

The first compile rejected unsupported SQLite `u64` conversions. Bounded revision fields
now use supported `u16` values under the unchanged 128-revision ceiling. The next run
passed 62 focused tests and exposed a diagnostic mismatch: concurrent ownership was
refused during initial database configuration but mapped to a generic open failure.
That path now uses the existing busy/locked error classifier; the exact concurrency
assertion is retained and the older broader assertion is tightened.

The next run passed 63 focused research tests and 46 operational-store tests, with one
existing ignored store test unchanged. It then rejected an incomplete host error match
for the new journal error variant. The corrected mapping distinguishes uncertainty,
cancellation and exhausted resources; its regression exercises those outcomes.

The remaining focused check exited 0: three host compatibility/error tests, strict
all-target kernel/host Clippy, 26 Python contract/evidence regressions, dependency-direction,
runtime-ownership, module-inventory, formatting and diff checks passed. Counts from these
runs overlap and must not be added together as unique coverage.

New canonical fixtures cover full-plan schema rejection and changed restrictions, missing
publication events, payload loss/corruption/release, original deadlines, failed-quota clock
rollback, cancellation, replay, concurrent ownership, failed atomic persistence, bounded
journal pressure, backup/restore and content-free export. Separate encrypted migration
fixtures verify 18-to-19 preservation and rollback of tables, history and version on failure.
Payload storage in the journal fixtures is explicitly fake; the encrypted database and
canonical run/artifact metadata owner are real. These tests perform no network or inference.

| Retained private evidence | SHA-256 |
|---|---|
| Initial compile, exit 101 | `60c641182de2f07fc83649a5e951394be5f851f1a6def65f6d24114dc9fdac23` |
| Concurrency diagnostic failure, exit 101 | `863bba1e06c618820b73c6f843f0ba17104740bd552c160f5b2bf4f7d12f6c02` |
| Passing kernel/store checks, then host compile failure, exit 101 | `7c3194ff3c21fa5228261b6c76f215e1dc9d51b6b032001c9b2ced39063666a8` |
| Focused remainder, exit 0 | `a143a276d4a19b5b335615f863347a7860ba0afc8c34448e8a426bb5cacb0a6b` |

Raw logs and source snapshots remain private because they contain machine-local build
paths. Their hashes identify retained evidence, not public download links.

### Broader executable regression

The follow-on driver and collected terminal result both exited 0. The complete kernel
library passed 1,115 tests with seven existing tests ignored; the host passed 308 with
eight ignored; the default Linux adapter passed 147 with 45 ignored. The ignored set was
not promoted to passing coverage. The packet-to-prepared-authority compile-fail test,
actual CLI/host/read-worker build, 59 Python regressions and three boundary/dependency
audits also passed.

All 22 actual-process scripted coding cases passed against the schema-19 binaries. Cases
include repair, new/multiple files, no-op, rollback, denial, output pressure, cancellation,
stale/replayed approval, invalid activation, protocol correction, native command failure,
expired cursors, cancellation races, artifact-integrity refusal and human-edit-preserving
rollback conflict. Expected nonzero rejection exits are retained as expected refusals,
not suppressed failures. These are explicitly scripted model proposals through the real
CLI, host and native tools, not real-model success or native research qualification.

| Additional private evidence | SHA-256 |
|---|---|
| Broad regression driver, exit 0 | `6835d5486b9ded3dc61e2c6fce7dbfd35e48af7d1c80f0f2754eb181601d2d0b` |
| Inspected 22-case report | `cea16823edfd8a12a7f07993af44283f5400a4c29a3e03105f1d61e4f1880788` |
| Actual CLI binary | `016fcdcf1d69d49c0281ef0e125197ff5464b05b5e5eebbd64df30b8e1a16786` |
| Actual host binary | `636416d7787fc689f9035ab8cd7a41a03cb405828f585151ed84e96f8226603f` |
| Actual read worker | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |

Each case retains its full source/diff and binary identities. Runtime and harness source
remained unchanged during the matrix. One evidence-builder date was corrected during the
campaign; that metadata-only intervention is retained rather than presenting the entire
checkout as one unchanged source snapshot. Binary identities remained unchanged per case.

The additional integration driver exited 0: 67 kernel tests across 18 integration targets,
15 optional-worker tests and 40 Python checks passed, with no ignored cases in those
commands. Status validation, the requested documentation-lint command and diff checks
also exited 0. These results overlap earlier component coverage.

### Cumulative evidence disposition

One SBOM write covered the completed Rust batch. Structural contract, configuration,
dependency and kernel-boundary reports were renewed and checked. The first evidence
driver then exited 1 because a content-deduplication checker still requested the old
version-18 upgrade test. The missing-marker check correctly refused to publish a pass.
Its failure diagnostic is retained; that builder did not emit its internally captured
Cargo output on this path, so complete raw output for that failed step is unavailable.

Four current-source checker references were corrected to the real version-19 test.
Three related leaf reports also received accurate current schema metadata and additional
schema-18, schema-19, migration and research-journal input bindings; no input was removed.
New regressions require rejected raw output to be retained and prohibit report publication
when required test markers are absent. The correction changed developer checkers/tests,
not Rust code, migrations, acceptance thresholds or SBOM inputs. No second SBOM write ran.

The remainder driver exited 0. Its initial 22 checker tests and subsequent leaf checks
passed, including source lifecycle, workflow attempts/publication, migrations, downgrade
refusal, the original 34 source/workflow families, the original 224-case seeded crash
matrix, retained security mapping and durable resume. The final supply-chain validation
passed. Those bounded historical test scopes are not expanded to exhaustive research-family
acceptance. Some older generators retain their original date labels; the rerun date and
source-binding disposition are recorded here rather than inferred from those labels.

| Additional retained private evidence | SHA-256 |
|---|---|
| Integration driver, exit 0 | `b7051d191121f977a3f138320362a79484772c0c8fd92ed6f2c8c1899afa1dd3` |
| Initial cumulative evidence driver, exit 1 | `bdf5d08fc0c2ebe2674fa8f9223ed87a148da9846149e01036e852ee9d27f4d2` |
| Corrected remainder and freshness inventory, exit 0 | `489e0da07a638e144df157e8626a5e816bca653b1c5a86ea72151c1741d0ebe4` |

The post-pass read-only inventory parsed 748 JSON files. Of 729 recognized named whole-file
bindings to changed inputs, 101 were current, 38 newly stale against the source parent,
and 590 previously stale or historical. It excluded 1,602 line-span bindings and makes no
transitive, unnamed-binding or aggregate acceptance claim. The before-SBOM inventory had
a different changed-input set; its counts must not be subtracted as a completeness metric.

The remaining newly stale reports are Story 11.2 acceptance/gate aggregates, the Story 4.1
security map, Sprint 82 and the historical frozen-scope breadth report. Story 11.2's
whole-current-store assertions need research-family coverage reconciliation, not automatic
reacceptance. The frozen-scope report remains historical. Sprint 82, store-unit and
operational-schema evidence need the new committed source pin; the Story 4.1 mapping also
receives its truthful pin after that checkpoint. These are pending follow-on checks, not
current approvals. No aggregate evidence-freshness, model or release claim is made.

## Remaining integration and acceptance

An opaque reservation proves spent accounting, not permission, current cancellation state
or successful retrieval. Native dispatch must consume and revalidate it through the
existing authority owner, with an exact consumed grant, current plan and original deadline.
The owner already holds its store mutex across driver execution; a driver must not relock
that store or create a second execution path. Native atomic cancellation, confinement and
verified owned cleanup remain separate requirements. Query-versus-visit accounting must
come from the trusted registered provider/tool mapping, never a model-selected label.

Host/provider activation, receipt-backed source retention, resumable reports and actual
research-to-code qualification remain open. Historical eight-case Muse results are bounded
development evidence, not proof for this revision or research workflow. GPT-OSS remains
unqualified. No GPU phase, independent review, human acceptance, platform or release gate
is granted by these component checks. No task checkbox is promoted here.
