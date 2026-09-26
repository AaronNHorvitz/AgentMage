# Expanded Rust Roadmap: Source and Evidence Reconciliation

## Baseline and authority

Decision 0081 and the owner's explicit restart supersede the prior narrow implementation
assignment. Base commit: `e06d8f34c5e4c4165a55bd482051c61e3c057627`, branch
`demo/fedora-local-docs`; all 93 prior local commits remain intact. The operator supplied
12 modified documents and three new roadmap/amendment/decision documents, not runtime code.
Their exact changes were read before this implementation increment. Privacy edits retain
historical meaning but do not retain the edited files' previous hashes.

The implementation has no source differences in `kernel`, `capabilities`, `platforms`,
`shells`, `Cargo.toml` or `Cargo.lock` between the native campaign pin
`ad28b5ca7862a483bb4b9bf74c199d5f994ae43f` and the restart base. The following files
were rehashed before new source edits; all matched the previous handoff:

| Identity | SHA-256 |
|---|---|
| Actual CLI binary | `7e1185bf2197a30cbf7665c18c8335c6d07a87ad5260c22581fb5115f672e42a` |
| Ordinary host binary | `935c0404504a3e946a3a2ebe9d15ea7b85ef43a7ee12e2635a60732dcc367799` |
| Native read-only worker binary | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |
| Retained Muse campaign13 summary | `62a943cb551be20995a1add1fdbdbaa44d0365883d466a200f57ea891d33a1fd` |

The eight Muse cases remain bounded development evidence at their exact source/model/runtime/
codec/profile, not qualification of this next source revision. GPT-OSS remains separately
not qualified on its tested profile; no unchanged failed campaign is scheduled. The original
rejections, fixes, failures, raw streams and receipts are preserved. See the
[campaign results](coding-harness-campaign13-results-2026-09-23.md), not the superseded
September 22 first-call failure disposition, for the later Muse development result.

## Main consolidation disposition

The owner subsequently consolidated the committed history onto `main` at
`aa3134b224e887fbf5962fbf724b8d3d54cbe8ec`. The restart base and binary hashes above
remain historical measurements, not a new qualification of that consolidation.
Operator commits `9191f766` and `aa3134b2` add Decision 0083, the main-first workflow
and the matching launcher/test branch policy. The exact changes were inspected;
unfinished admission, research and lease source edits were preserved separately.

Future commits request publication through the designated publisher and therefore require
coherent increments, applicable verification and explicit limitations. No unfinished source
is committed merely to obtain a clean campaign pin. The additional planning/README/AGENTS
changes also require current-input freshness checking in the batched pass; the 91-binding
inventory below describes the earlier restart diff only. Consolidation does not renew a
historical hash binding, independent review, model admission or release evidence.

## Reuse and exact gaps

| Existing owner | Reused behavior | New acceptance still needed |
|---|---|---|
| CLI, ordinary host, private IPC, shared coordinator | Actual development launch, live events, exact approvals, cancellation and follow-ups | Production activation and broader client/host contracts remain gated |
| Native coding tools and Linux effect owner | Confined reads, exact patches/create, registered command/validation, conflict-aware rollback | New research effects require explicit independent network authority |
| Canonical journal/artifacts/checkpoints | Durable coding resume, drift refusal, verified full outputs, source-backed context | Research records must enter these owners, not a second database |
| Exact Muse/GPT-OSS codecs and native driver | Actual served 32K preflight, family tools/reasoning, bounded output and cleanup | Cross-host inference ownership was absent in the Rust driver |
| `public_research` and `web_research_safety` | Inert bounded request, citation and disclosure validation | No real search-provider transport, mediated DNS/redirect worker or research-to-patch composition exists at the restart base |
| Existing network policy and grants | Closed `NetworkAccess` operation and strict-local refusal | A research policy object must not itself mint an effect permit |

The `later-network` example is still `deny-all`, has no allowed endpoints and disables shell
network access. It is not a configured research provider or an outbound admission artifact.
Account/provider availability and real research qualification must be reported separately from
the implementation work that can proceed locally. No account, credential or paid provider is
obtained by this reconciliation.

The root license and Cargo SPDX metadata already disagree at the restart base. No metadata or
license text is changed here; the owner's separate licensing disposition is still required.

## Explicit freshness disposition

A read-only scan of named SHA-256 bindings in JSON under `artifacts`, `docs/verification`,
`architecture`, `release` and `supply-chain` found 91 distinct bindings to the changed Markdown
files. Comparing each stored digest both to the restart base and the edited file distinguishes
**five newly stale bindings** from **86 already stale bindings**. This is a direct-binding
inventory, not a transitive acceptance audit or a claim that other evidence is fresh.

| Newly stale artifact | Changed bound input | Disposition |
|---|---|---|
| `architecture/schema-evolution-and-rollback.json` | `ENGINEERING-RUNTIME.md` | Historical binding; revalidate applicable schema contract in the batched pass |
| `artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json` | `TASKS.md` | Current boundary check requires renewal after the source batch |
| `artifacts/sprints/sprint-48/local-evidence-report.json` | `README.md` | Prior collector remains historical, not current amended-document acceptance |
| `artifacts/sprints/sprint-9/story-9.1/linux-native-inference-boundary.json` | `IMPLEMENTATION-PLAN.md` | Previous boundary receipt is not silently rebound; applicable checks need new results |
| `docs/verification/remaining-plan-blocker-audit.json` | Decision 0061 | Previous narrow-scope audit is superseded for sequencing, not erased or treated as new proof |

The scan recognizes path-keyed hashes and explicit path/hash records; unnamed digests and
transitive dependants are not asserted covered. The 86 previously stale bindings are not newly
introduced failures and must not be silently reaccepted. Edited historical privacy/handoff
documents are not test receipts even where no direct binding was found. Native campaign
artifacts remain unmodified at their original pin. Any new Cargo-member source edit also
requires the one cumulative SBOM/evidence pass after the work unit, never a per-file cascade.

### Post-consolidation working-source check

A subsequent read-only inventory at `aa3134b2` plus the in-flight AMR changes examined
748 JSON files against 21 changed Markdown inputs. Its explicitly recognized named
whole-file bindings total 91: six match the restart base but not the current input,
and 85 match neither. No recognized binding matches its current changed input.
The additional newly stale input is `docs/guides/public-research.md` in
`artifacts/sprints/sprint-82/local-evidence-report.json`. The preceding five newly
stale inputs are still stale. These are snapshot-specific inventories, not a monotonic
count of all evidence dependencies or a claim that the two scans recognized identical sets.

All JSON files in that scan parsed. It explicitly excluded 1,602 line-span bindings:
comparing a selected-row digest to a whole-file hash would be invalid. Unnamed digests,
line-span freshness and transitive dependants still require their registered checks in
the cumulative pass. The inventory renewed no historical artifact or review pin.

Lightweight source/contract and isolated launcher checks passed: 40 tests across
`tests.test_public_research_contract`, `tests.test_start_coding_codex`,
`tests.test_strict_local_source_audit` and `tests.test_effect_boundary`, followed by
one new source-boundary regression. The latter rejects TCP, UDP, DNS-resolution and
HTTP-client APIs in the research address-classification module; permitting address
values does not permit socket effects. These checks ran sequentially in the existing
memory-capped scope after the available-RAM check. They do not compile Rust, start
inference or establish executable research success.

Read-only dependency-class, dependency-direction, runtime-ownership, module-inventory
and status-model validators also pass on this working source. Their existing closed
policies were not relaxed to obtain those results. In particular, status validation
preserves the pre-release/open-gate declarations; it is not an implementation gate pass.

The focused Rust lease test initially waited behind the mandatory shared build reservation.
On resumption its retained result was inspected: compilation succeeded and all eight
selected tests passed. Two ignored entries are subprocess helpers exercised by the parent
tests, not omitted acceptance cases. The completed broader CPU batch is recorded below;
the focused result alone does not prove executable recovery. No SBOM renewal or model
requalification is claimed. The first GPU phase was handed off after the scheduling wait,
with no new model process launched. A native regression on the changed source requires
a later operator-scheduled GPU round.

The freshly compiled test binary also passed four focused CPU-only resource-admission
tests and the legacy 8K compatibility test, run sequentially in the capped scope. The
observer-execution fixture invokes only `/usr/bin/true`; no actual GPU observation or
model launch occurred. These five tests are component evidence, not a replacement for
the broad suite or a native model campaign.

### September 25 CPU-batch reconciliation

The retained September 24 CPU batch has terminated; its earlier build-admission wait is
resolved. Inspection of the actual log establishes 1,513 passing Rust tests: 119 in Linux
inference, 1,063 in the kernel engine, 316 in the host library and 15 in the host binary.
The suite's ignored cases remain ignored; these totals do not promote their acceptance.
Host and native capability binaries built successfully with the locked, offline dependency
set, one Cargo job and one test thread in the existing capped scope.

The next command, strict Clippy over all targets of those three crates, failed at
`kernel/engine/src/research_budget.rs:398` with `clippy::nonminimal_bool`. The raw failed
attempt is retained. The correction applies De Morgan's law to the existing IPv4 deny
predicate, preserving all rejected ranges without suppressing the lint. The subsequent
Python CLI tests, executable matrix and recovery checks did not run in that failed batch.
Their new attempt is separate; no success is inferred from the prior unit-test totals.
Native model qualification on this source still requires a future scheduled GPU phase.

The subsequent developer-tool checkpoint `14e33a45` adds bounded content identities for
non-ignored untracked files to the existing harness diagnostic. HEAD, the complete tracked
diff and all binary bindings remain intact. Seventeen harness/collector fixture tests pass;
the initial FIFO-fixture failure is retained and was corrected to exercise replacement
after Git enumeration. Historical native campaigns are not rebound to this changed wrapper.

Later dependency-ready source work adds the validated task identity to the nonforgeable
effect permit and a closed, inert public-GET packet under Decision 0084. Producer preparation
and worker decoding are different types; neither is permission, transport proof or a resumed
budget. No network client or worker dispatch is enabled. Formatting, the strict-local source
audit and the effect-boundary audit pass. The latter first rejected an authority-type name
used only in a documentation comment; the comment was clarified without altering code or
the consumer allowlist. At that checkpoint these fixtures awaited execution and were not included
in the preceding 1,513-test result. Targeted Markdown lint passes after removal of one extra
blank line in the AMR table; its failed attempt is retained too.

The same source unit now includes an inert worker-response frame: bounded closed JSON
metadata followed by the exact unencoded body. Full-body identity, request/operation,
initial target, same-origin redirect accounting and original time/byte limits are checked.
Descriptive observations remain distinct from authorized worker provenance or verified
citations. Its four Rust fixtures were then pending compilation, not part of the old totals.
A fresh capped run of the public-research, strict-local-source, effect-boundary and launcher
Python suites passed all 41 tests after this addition. No HTTP dependency, network launch,
SBOM renewal or new model result is supplied by those lightweight checks.

### September 26 completion of the retained remainder

The previously queued remainder completed; its shared-build wait is resolved. The original
process handle is closed. Its terminal envelope was truncated during collection, so a numeric
outer-process exit status is not independently retained. The unchanged fail-fast shell driver
reached its final successful supply-chain validation; individual process exits, raw logs and
case reports are retained. No campaign was rerun to replace the missing outer status.

Inspection establishes the following results on `14e33a4565e34663fd84fe0939b3ccc5bc2e633d`
plus tracked diff SHA-256 `db225e343c081a899f6f76f30cac6dd59cd59cd18690abd1b84459eceeab7bad`
and untracked inventory SHA-256
`1a45d404935454869ef44373e84171d0f8404e011bad916eea825fc0ec369159`:

| Check | Actual result |
|---|---|
| `cargo test -p agentmage-kernel-engine --lib research_budget --locked --offline` | 8 passed |
| Strict Clippy, all targets of Linux inference, kernel and host | Passed with `-D warnings` |
| Host and read-only capability binaries, locked/offline | Built successfully |
| CLI binary, coding harness, public research, strict-local source, effect-boundary and launcher Python suites | 58 passed |
| Strict-local source, effect-boundary and dependency-class validators | Passed |
| Actual CLI/host/native-tool scripted acceptance matrix | 22 cases passed, including expected nonzero exits |
| Safe restart and worktree-drift refusal | Both passed; no prior-effect replay, human edit preserved |
| Protocol-rejection checkpoint restart | Passed; no rejected-turn authority, six fresh tool effects, verified repair |
| Kernel authority, GET request, GET response and host research fixtures | 17 + 4 + 4 + 12 passed |
| GET producer/consumer type separation | One compile-fail doctest passed |
| One cumulative SBOM write, supply-chain fixtures and final validation | Validated; 11 fixtures passed |

Rust tests used one test thread and one Cargo job. The complete remainder used the shared
build reservation and existing RAM-checked memory-capped scopes. These totals are not added
to the earlier 1,513 tests: some repeat earlier coverage. Ignored cases remain ignored.
All 22 raw stdout/stderr identities and per-process exit statuses were inspected against the
summary; recovery reports, drivers, binaries and tested source identities were also rehashed.
The initial read-only inspector incorrectly compared two different report schemas; its failure
is retained, and the corrected inspector passed without changing any campaign evidence.

| Retained evidence | SHA-256 |
|---|---|
| Remainder raw log | `a5c90ce0cc7c0288804ff4c7c45d6fa3f92aba2ac5c93e139a6542bbeba980cb` |
| 22-case executable-scripted report | `5a4af1e6cad0bd94e8648b330762718a83852f745704995faf6a218a8dbe7fdf` |
| Safe/drift resume report | `0a08c159df2b9b61ceaf3a6e0196c0bb2de32326c3f7a072fa76c6b7b0e94ac7` |
| Protocol-rejection resume report | `766b567948783012fe6e31784a4c5541ecb0016a5cc60f7b4e9e09fe7a458013` |
| Tested CLI | `e7d57b177a6948fdf78355ecb27c874728814dd047f63fdedbfdebec0a49db66` |
| Tested ordinary host | `147bd953535b1c8c62453931dee08e28b921309bd0c3e129f7deb98b5093b7bb` |
| Tested read-only worker | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |

Raw diagnostics remain private because they contain machine-local paths and streams; these
digests identify retained evidence, not publicly downloadable artifacts. The source comparison
excluded only the three subsequently generated supply-chain outputs. Final documentation and
applicable evidence renewal are later changes, not silently inserted into the tested diff.

The renewed SBOM changes only the complete first-party kernel, host and Linux-inference
tree identities and their containing manifest digests. No dependency, lockfile, license or
hash-input scope changed. Historical reports bound to previous trees remain historical and
must not be treated as current acceptance. Decision 0080's withdrawn demo disposition remains
in force. The schema-evolution authority pin is renewed after inspecting the sole underlying
document change: consumer-neutral terminology, with no schema or migration-policy change.
Applicable current contract checks are renewed separately; native/package and independent
review receipts are not renewed by this scripted campaign.

The current unit implements admission/lease and inert research prerequisites, not a live
network worker, configured provider or research-to-code workflow. Those remain owned next
implementation work. The first GPU phase was already handed off; fresh inference requires
a new scheduled phase and exact admission. Muse's eight-case result remains historical bounded
development success, GPT-OSS remains unqualified, and no model threshold has been lowered.

### Batched prerequisite checkpoint

The subsequent schema-evolution validator and 16 fixtures passed. The existing contract
builder executed 52 schema, 8 dependency, 15 ownership, 13 API-surface and 33 status/boundary
tests; the renewed index passed its 12 fixtures. Configuration-startup and component-inventory
builders and seven fixtures each passed. No source edit or second SBOM write intervened.

The first checkpoint exited 1 because the security mapper refused five stale configuration
reports. Their unchanged builders then executed the registered loader, authority-mutation,
migration-recovery, result-binding and schema-failure tests plus strict Clippy before writing
new reports. Their fixture suites passed 5, 7, 5, 5 and 5 tests respectively. The security map
and six fixtures, status validation, six-file Markdown lint and diff check then passed; the
remainder driver and its collected process result both record exit 0. Failed and successful
attempts remain separate:

| Checkpoint log | SHA-256 |
|---|---|
| Initial checkpoint, including stale-input rejection | `1880007f960365e5cd2689eb06080e787ecef000b294f323076e1d6072e00e70` |
| Corrected prerequisite/remainder pass | `06b26280e8c6814663f17f08ec9405ad33a94a2cb76fae788e26970095e5e000` |

A final named whole-file scan of 748 JSON files found 452 bindings to the changed Markdown
and three cumulative SBOM outputs: 10 current, six newly stale and 436 previously stale or
historical. This narrow inventory excludes 1,602 line-span bindings, unnamed hashes and
transitive acceptance. The six newly stale bindings remain explicitly unaccepted:

| Retained artifact | Changed input and disposition |
|---|---|
| Sprint 48 local evidence report | README; historical collector, not fresh executable acceptance |
| Story 7.1 security evidence map | Provenance and SBOM; two bindings, no current platform/security promotion |
| Sprint 82 local evidence report | Public-research guide; no live research qualification |
| Story 9.1 Linux native-inference boundary | Implementation plan; fresh native/platform qualification still required |
| Remaining-plan blocker audit | Decision 0061; old sequencing audit superseded, not rewritten as new evidence |

The renewed contract and configuration reports keep their existing partial-scope and open
product-gate declarations. They do not renew those historical artifacts, independent review,
model campaigns, supported-platform or release gates. AMR packages remain open until their
full acceptance cases pass; this is a verified prerequisite increment, not roadmap completion.

## Open gates

AMR package rows remain open. Existing model admission, external independent review, human
acceptance, production transport, supported-platform and release gates are not promoted.
The prior pinned independent-review package remains an outstanding historical candidate;
the new source increment will require a new pinned package, not self-approval or an automated
review-pin update presented as independent review.
