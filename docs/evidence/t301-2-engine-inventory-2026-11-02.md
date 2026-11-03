# T-301.2: Engine inventory, 2026-11-02

Status: builder inventory under [Decision 0153](../decisions/0153-engine-and-local-coding-priority.md),
awaiting independent exact-commit review. No import, application, model or sandbox
qualification follows from a successful build. Nothing is deleted. The original
T-301 three-review disposition and deferred owner-action T-301.1 remain intact.

## Classification

**works** means the stated bounded component duty was exercised; it never means
all planned capabilities work. **partial** means executable code exists but a
required integration, native execution or real-model check is missing or fails.
**scaffold** means the entry point only declares a boundary or a placeholder.
Build/test results below use default features on Linux unless explicitly stated.
Optional targets are listed even when that build does not select them.

The tables cover all ten Cargo workspace packages and their fourteen binaries,
plus the excluded experimental crate and its binary. Cargo metadata, checked
against every tracked Cargo manifest, determines coverage; hand-written manifest
entries alone miss auto-discovered binaries. Each binary gets its own row below.

## Crates

| Crate | Manifest | State | Observed capability and evidence | Carry forward |
| --- | --- | --- | --- | --- |
| `agentmage-kernel-contracts` | [manifest](../../kernel/contracts/Cargo.toml) | works | Shared typed permission, tool, identity and runtime contracts; 86 default-feature tests passed in the [earlier baseline](t301-baseline-2026-10-25.md). No effects or end-user workflow claimed. | Reuse authority contracts; maintain runtime ownership. |
| `agentmage-kernel-engine` | [manifest](../../kernel/engine/Cargo.toml) | partial | Agent loop, grants/approval, validation, context and durable stores have component tests (1,540 passed, 8 ignored in the earlier baseline). Broad modules include contract-only integrations. Historical real coding is limited to the frozen development campaign below. | Reuse coding loop, context, permissions, records and store. Do not make unused integration modules prerequisites. |
| `agentmage-capability-read-only` | [manifest](../../capabilities/read-only/Cargo.toml) | partial | Bounded read/search payloads and worker; 27 baseline tests passed. Native sandbox invocation is not established by payload tests; T-301.1 affects host execution. | Reuse scoped file definitions/payloads and retain confinement. Fixture binary stays development-only. |
| `agentmage-capability-repository-map` | [manifest](../../capabilities/repository-map/Cargo.toml) | partial | Bounded structural maps, syntax parsing, cache and strict edit contracts; 73 baseline tests passed. This does not show model-selected retrieval or measured coding improvement. | Reuse map/retrieval and edit validation for T-303/T-306 after T-311 baseline. |
| `agentmage-capability-knowledge` | [manifest](../../capabilities/knowledge/Cargo.toml) | partial | Local index, document parsing including PDF, memory and citation components; 302 baseline tests passed. Mission import/question/re-import/removal over its API is not integrated. | Retain document/index modules for T-304; unrelated connector modules are background. |
| `agentmage-platform-linux-inference` | [manifest](../../platforms/linux-inference/Cargo.toml) | partial | Native driver, bounded ownership/resource components and text protocol. The 19 protocol tests and independent [T-302.1 PASS](../reviews/8bbe1ca85871-local-agent-t302-1-carriage-return-repair.txt) cover codecs only. Credential-owning configurable HTTP transport, listing and a new real chat remain open. | Reuse the protocol and applicable driver/lease boundaries. Container and acquisition components are not minimum coding dependencies. |
| `agentmage-platform-linux` | [manifest](../../platforms/linux/Cargo.toml) | partial | Native file/command/worktree/IPC/store/sandbox adapters exist. Earlier baseline: 187 passed, 19 failed, 51 ignored; native trust refusals remain failures. Optional public research worker has no production activation. | Reuse confined adapters without relaxing ownership/authority checks. Defer unrelated platform and research integration. |
| `agentmage-host` | [manifest](../../shells/host/Cargo.toml) | partial | CLI and development host composition, authenticated IPC and fixture-backed document process tests exist. Earlier baseline: 532 passed, 53 failed, 11 ignored. Current versioned mission HTTP API/MCP/CLI composition is absent. | Reuse focused composition and approval paths. Do not import desktop scaffolding as an engine requirement. |
| `agentmage-platform-windows` | [manifest](../../platforms/windows/Cargo.toml) | partial | Twelve baseline contract tests passed on Linux; native identity code is conditional. Linux tests do not demonstrate Windows operation. | Preserve source; exclude from Linux engine dependency requirements. Other-platform work is deferred. |
| `agentmage-xtask` | [manifest](../../release/xtask/Cargo.toml) | partial | Candidate packaging/signing command orchestration; four baseline tests passed. No new release/package was generated or qualified. | Preserve development tooling outside the engine runtime dependency graph. |
| `agentmage-experimental-model-lab` | [manifest](../../experimental/model-lab/Cargo.toml) | partial | Excluded from the workspace; library validates experimental authority/artifacts and promotion records, while its binary only prints a placeholder. No real model execution or agent integration. | Preserve, unused by the engine. |

## Binaries

These classifications cover the actual entry point, not similarly named library
implementations. A self-description is never inference, and a help message is
never a coding run. Source links allow inspection of each scope.

| Binary | Source | State | Evidence and limitation | Carry forward |
| --- | --- | --- | --- | --- |
| `agent` | [entry](../../shells/host/src/bin/agent.rs) | partial | Help/version and explicitly activated development coding path; ordinary execution lacks production transport/activation. Historical campaign used the development path. | Adapt as a thin client of the engine HTTP API. |
| `agentmage` | [entry](../../shells/host/src/bin/agentmage.rs) | partial | Second CLI entry with the same limited development composition; help is not an integrated API client. | Consolidate thin CLI behavior without deleting existing entry points. |
| `agentmage-host` | [entry](../../shells/host/src/main.rs) | partial | Bootstrap/package verification and explicit coding/catalog/evidence development hosts; production mission API is absent. | Reuse selected host composition for the engine service. |
| `agentmage-standalone` | [entry](../../shells/host/src/bin/agentmage-standalone.rs) | partial | Default launch exits 3, `standalone.presentation.unavailable`. Development stdio can drive the term-match fixture host; five earlier process tests passed. No desktop is delivered. | Preserve as development evidence only; no new desktop work. |
| `agentmage-demo-documents` | [entry](../../shells/host/src/bin/agentmage-demo-documents.rs) | partial | JSON-lines local document adapter with bounded reads, no model/network authority. It is not the versioned engine API. | Reuse document handling where appropriate; keep demonstration separate. |
| `agentmage-read-only-worker` | [entry](../../capabilities/read-only/src/bin/agentmage-read-only-worker.rs) | partial | Fixed-path request/snapshot payload process; expects its confined parent. Payload component tests do not prove a fresh sandbox launch. | Reuse through admitted scoped execution only. |
| `agentmage-read-only-lifecycle-fixture` | [entry](../../capabilities/read-only/src/bin/agentmage-read-only-lifecycle-fixture.rs) | partial | Synthetic crash/cancel/timeout lifecycle worker; requires `lifecycle-fixture`, absent from default-feature build. No real tool/backend evidence. | Development tests only; never product packaging. |
| `agentmage-native-inference` | [entry](../../platforms/linux-inference/src/main.rs) | scaffold | Entry accepts only `--self-check`; it does not serve a model. Library driver functionality is separate. | Preserve diagnostic entry; reuse library driver rather than mistake this for a model server. |
| `agentmage-docker-guard` | [entry](../../platforms/linux-inference/src/docker_guard_main.rs) | partial | Self-check or one guarded session with exact bootstrap; no current live container session exercised. | Preserve, unused by minimum local coding path. |
| `agentmage-docker-topology-collector` | [entry](../../platforms/linux-inference/src/docker_topology_collector_main.rs) | partial | Self-check, observation and validation entry points; no current live topology exercised. | Preserve, unused by minimum local coding path. |
| `agentmage-model-installer` | [entry](../../platforms/linux-inference/src/model_installer_main.rs) | partial | Entry accepts self-check and preflight input, with no normal installation/inference authority. Broader lifecycle code is in the library; no installation/download performed here. | Preserve; no acquisition dependency for owner-provided local models. |
| `agentmage-public-research-worker` | [entry](../../platforms/linux/src/bin/agentmage-public-research-worker.rs) | partial | Optional `public-research-worker` feature; exact request and environment contract. No parent production activation; default-feature build does not compile it. | Defer to T-305, retain all confinement/network rules. |
| `native_evidence` | [entry](../../platforms/windows/src/bin/native_evidence.rs) | partial | Native Windows identity observer; Linux entry only refuses with exit 2. | Preserve, unused by Linux engine. |
| `agentmage-xtask` | [entry](../../release/xtask/src/main.rs) | partial | Packaging/signing orchestration; argument/unit tests do not demonstrate a release kit. | Development tool only. No release authorized. |
| `agentmage-experimental-model-lab` | [entry](../../experimental/model-lab/src/main.rs) | scaffold | Prints a placeholder; separate excluded crate. | Preserve, unused. |

## Real-model boundary

The [historical command-control campaign](../verification/coding-harness-campaign16-results-2026-09-27.md)
records eight actual development coding runs at frozen commit
`b232aa2b9f38d227d896cb5f616c38fac48dea2e`: four synthetic scenarios, twice,
using a real local model, actual CLI/host/IPC/tools and verifier. Six mutation
runs passed fresh tests after edits; two stable cases returned no-op. This is
retained historical evidence, not a fresh run of today's tree, not 30 real
repository tasks and not a held-out score. It qualifies no other model.

The standalone document process tests instead use a term matcher. T-302.1 tests
only bytes and synthetic streams. No current-tree mission chat, HTTP/MCP service,
confined coding workflow or T-311 baseline has run with a real model in this
inventory batch. No GPU process was started and no tokens, GPU minutes or coding
success percentage are inferred. T-302 transport/API/real-run work precedes T-311;
no prompt or loop tuning is done before that baseline.

## Import boundary and licences

The smallest current reuse path is contracts plus the relevant engine, read-only,
repository-map, Linux/inference and host modules. Knowledge is later document
work. The host currently directly depends on knowledge and broad Linux/engine
features: this inventory does **not** claim a small standalone dependency graph
already exists. Extract or feature-bound required composition during implementation;
never satisfy minimum coding work by activating unrelated scaffolding. Keep the
existing authority owner and do not replace confinement with a bare command runner.

All ten workspace manifests currently inherit `Apache-2.0`; the excluded lab also
says `Apache-2.0`. The root [LICENSE](../../LICENSE) and [NOTICE](../../NOTICE)
state Business Source License 1.1; the notice explicitly says package metadata is
an interim discrepancy. The contracts directory additionally has its own Apache
licence text. [Decision 0051](../decisions/0051-business-source-license.md)
records the existing interim state; Decision 0153 records the new import direction. These facts do not establish `MIT OR Apache-2.0` import readiness.
No licence, copyright or source header was rewritten by this inventory.

Decision 0153 requires accurate dual-licence notices for carried-forward original
code at import. Before import, reconcile the authoritative notices and metadata,
verify ownership/provenance of selected source and fixtures, and retain declared
permissive dependency notices. Existing declarations and a successful build are
insufficient to certify an entire crate's ownership or dependency licence closure.
This is open work under the owner-authorized import direction; no legal clearance
or completed audit is asserted. Unrelated original source remains preserved.

## Current verification

The measured Rust source is byte-identical to `8bbe1ca858719c671b2a6ade15db04c98e7f7b34`;
the mission reconciliation is commit `381fd1ee`. Only queue/documentation/review
retention changes are in this batch. Manifests, lockfiles, runtime source, status
model, historical ledger and supply-chain outputs are unchanged.

The completed baseline ran through one shared build reservation, offline and
locked, with two build jobs and four test threads. Seconds below measure each
command after reservation acquisition. The supervised completed run took 325.91
seconds including reservation/cleanup. An earlier attempt was interrupted after
the reservation became available and compilation had started; it establishes no
result. Its logs are retained separately. The completed run below was a fresh
rerun; no failure was relabeled or skipped.

| Command (Cargo commands use `--locked --offline`) | Exit | Seconds |
| --- | --- | --- |
| `cargo build --workspace --all-targets` | 0 | 44.161 |
| `cargo fmt --all -- --check` (no lock/offline flags needed) | 0 | 2.388 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 43.072 |
| `cargo test --workspace --all-targets --no-fail-fast` | 101 | 233.562 |
| `cargo build --manifest-path experimental/model-lab/Cargo.toml --all-targets` | 0 | 0.239 |
| `cargo test --manifest-path experimental/model-lab/Cargo.toml --all-targets` | 0 | 0.022 |
| `python3 -m unittest tests.test_local_agent_fast_gate` | 0 | 2.288 |
| Thirteen bounded binary entry-point probes | 0 | 0.049 |

The fresh workspace result is **2,948 passed / 72 failed / 78 existing ignored /
0 filtered**, across 63 test-result summaries. The 72 distinct failed identities
are exactly equal to the retained [baseline failure appendix](t301-baseline-failures-2026-10-25.md):
53 host library and 19 Linux library failures, none added or missing. Counts in
the crate table agree with the earlier baseline except inference now has 185
passes and 8 ignored, including all 19 protocol tests passing. The full Rust
baseline remains **FAIL**. Identity equality is classification, never a waiver.
No tests, thresholds, root-ownership checks or ignored-test declarations changed.
The five standalone fixture-process tests passed; they still use no real model.
Doctests and optional feature combinations were not selected by these commands.

The excluded lab's six component tests passed; its placeholder binary still does
no model work. All 23 existing fast-gate self-tests passed, including their
intentional missing-executable and timeout probes. The optional lifecycle fixture
and public research binary were not compiled or executed by the default-feature
build. Nothing here qualifies their feature combinations or production activation.

The fresh binaries were probed only within these narrow scopes:

| Probe | Observed result |
| --- | --- |
| `agent --help`, `agentmage --help` | Both exit 0 and show help. |
| Host with an invalid operation | Exit 1; no service started. |
| Standalone without arguments | Exit 3, presentation unavailable. |
| Document demonstration with `{}` on stdin | Exit 0 with a JSON error response; no document imported. |
| Read-only worker without arguments | Exit 1; no workspace request dispatched. |
| Native inference, container guard, topology collector and model installer `--self-check` | Each exits 0 with a JSON description; no inference, live container, observation or installation. |
| `native_evidence` on Linux | Exit 2, requires Windows. |
| Packaging tool without arguments | Exit 1; no package/signature generated. |
| Experimental lab binary | Exit 0, prints its placeholder message. |

The structural metadata queries took 0.022 seconds for the workspace and 0.021
seconds for the excluded crate. An exact-set comparison against every tracked
Cargo manifest and both tables passed: eleven packages, fifteen binary targets,
matching manifest/source links, no missing or duplicate entry. Metadata is a
structural check and does not compile or run a backend.

The scoped fast gate for the full documentation/review-retention batch is **PASS**:
whitespace, immutable review format and Markdown lint. It correctly selects no
Rust crate for documentation changes. Existing link, sensitive-text and claim
checks passed on the changed documents. Historical-ledger/source/global-output
preservation and unchanged independent-review bytes also passed. The retained
review SHA-256 is `226231790105c13896ce562357884e7a01b43bb0bc9b088bcf90d5754367b482`.

The raw completed workspace-test log SHA-256 is
`f633a32b8642f0aab5ded04c3056074e37e354c126bcd061637a2ca83dd1d021`.
The historical campaign summary hash was independently recomputed as
`659c47a9addc272f1fc9f0cc83c81febc66940b064ec5977137a48405245a962`,
matching its published record; the real backend was not rerun. Private logs,
per-command timings, entry-probe outputs and comparison helpers remain in the
lane; no raw host paths, transcripts or credentials are published.

No native-host execution facility is exposed here. The shared wrapper reserves
resources inside the lane; it does not escape its namespace. System Git and
sandbox executables still appear owned by UID 65534, not root. Therefore no
native-host gate PASS or working native sandbox is claimed. T-301.1 remains
deferred and blocks no new work under Decision 0153. No model/GPU process,
download, supply-chain regeneration, long documentation gate or release ran.
T-301.2 awaits its own independent review; this record closes no row.
