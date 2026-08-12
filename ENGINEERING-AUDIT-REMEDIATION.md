# AgentMage Engineering Audit Remediation Ledger

Status: **Non-normative working document**

Baseline reviewed: `abe664bde138d50fd52b3973cb70c7b5e93f5007`

Created: 2026-08-11

Current authorized phase: **Phase 4 - Authority Transaction and Taxonomy (pending user gate review)**

## 1. Purpose and Authority Boundary

This document consolidates independently reviewed engineering findings into a
controlled stabilization ledger. It exists to support review and sequencing. It
does not modify product scope, architecture, release requirements, acceptance
tests, task status, or historical evidence.

The following documents retain their established authority:

1. `PRD.md` controls product intent and release scope.
2. `Agent-Scaffolding-Inventory.md` controls stable requirements and acceptance
   identifiers.
3. `SECURITY-REVIEW.md` controls security requirements and reviewer protocols.
4. `IMPLEMENTATION-PLAN.md` controls the derived high-level sequence.
5. `TASKS.md` controls granular execution and gate order.
6. Accepted decision records control their stated amendments.

Nothing in this ledger is accepted implementation work merely because it is
written here. Promotion into a normative authority requires an explicit user
decision, impact analysis, exact edits to the owning authority, and applicable
tests. Historical requirements and evidence remain preserved.

### 1.1 Phase 1 restrictions

Phase 1 authorizes only this file. During Phase 1:

- No product source may change.
- No normative document may change.
- No requirement, task, acceptance criterion, or decision may change.
- No generated artifact or historical evidence may be refreshed.
- No accepted epic may be deleted, demoted, renumbered, or weakened.
- No commit, push, merge, or transition to Phase 2 is authorized.

## 2. Audit Basis and Interpretation

Two external read-only engineering audits examined the same baseline commit.
Their source reports remain outside this repository. This ledger distills their
findings rather than copying either report or remediation plan verbatim.

Important findings were independently checked against the baseline source,
manifests, machine-readable architecture, CI, tasks, requirements, and committed
evidence. Builds and tests were not rerun as part of this document-only phase.

### 2.1 Finding classifications

| Class | Meaning |
| --- | --- |
| `CONFIRMED-GAP` | Required integration or implementation is visibly absent. |
| `CONFIRMED-DEFECT` | Current code or data representations conflict with a stated invariant. |
| `TRUTH-DEFECT` | Current documentation or machine state overstates or contradicts reality. |
| `VALIDATION-REQUIRED` | Static evidence indicates risk that requires a reproducing test. |
| `DECISION-REQUIRED` | Multiple safe designs exist and one must be explicitly selected. |
| `SEQUENCING` | Recommendation about order, not a product-scope change. |

`CONFIRMED` in this ledger means verified statically at the recorded baseline.
It does not claim a dynamically reproduced exploit, outage, race, or release
failure unless a cited retained artifact already proves that narrower fact.

### 2.2 Priority definitions

| Priority | Meaning |
| --- | --- |
| `P0` | Resolve before enabling any real tool effect or claiming an integrated product path. |
| `P1` | Resolve before connected authority, writes, release qualification, or broad expansion. |
| `P2` | Resolve before packaging and first-GA platform claims. |

## 3. Verified Baseline Facts

The following facts anchor this ledger:

- The active branch is `agent/expand-delivery-windows-ga` at `abe664b`.
- The branch had a clean worktree when Phase 1 began.
- The host assembles component identity strings but has no runtime workflow.
- The read-only capability pack exposes identity functions but no tools.
- The tool interface has no execution method.
- Every valid pre-grant dispatch terminates with `grant_required`.
- The in-memory grant issuer is not composed with a production dispatcher.
- The Linux authorized-workspace constructor remains private pending a user
  workspace-selection boundary.
- No production Linux implementation of the aggregate `PlatformAdapter` exists.
- The Linux sandbox accepts an authorized workspace and a path, receives no
  consumed grant or held target, and mounts the workspace root read-only.
- Operational grants, receipts, nonces, revisions, and recovery state have no
  canonical encrypted transactional store.
- The Visual Studio Code package is not a loadable extension or model provider.
- The release `xtask` executable has an empty `main`.
- Windows is required by the accepted first-GA plan but is absent from product
  code, closed platform enums, workspace membership, CI, packaging, and the
  machine-readable build and module architecture.
- Continuous integration runs documentation validation only.
- Eleven native Linux security tests are ignored by the default product test
  command because they require live platform facilities.
- Both evaluated Gemma candidates are rejected and disabled, while the README
  labels Gemma 4 E4B as the first enabled model.
- Four machine architecture components use `shipped: true` despite scaffolded,
  blocked, or unintegrated states.
- All 227 registered requirements are `planned`.
- All 227 traceability records report `not_yet_produced` evidence with empty
  paths, including acceptance tests named by committed sprint evidence.
- `TASKS.md` contains 226 checked list items out of 4,143 list checkboxes, but no
  accepted sprint is represented as a completed product gate.
- The Python evidence layer contains 122 scripts and 50,132 lines with extensive
  duplication of canonical JSON, hashing, reading, atomic writing, report
  building, and report validation.
- No real Rust or JVM product-boundary fuzz target exists.
- Clean-build currentness binds a curated input closure rather than the complete
  tracked product tree, and its post-bootstrap commands rely on client offline
  modes rather than container-level network isolation.
- Historical evidence is intentionally retained, but the currentness model
  treats later policy edits as reasons for broad retained-evidence failure.

These facts establish substantial contract and Linux security work. They do not
support a claim that an integrated end-user AgentMage application currently
runs.

## 4. Stabilization Principles

Every later phase should preserve these constraints unless an explicit decision
changes one:

1. Preserve all accepted epics, stable identifiers, and historical evidence.
2. Freeze additional scope growth during stabilization.
3. Correct state descriptions without deleting product intent.
4. Make authority structurally unavoidable rather than conventionally expected.
5. Give workers the minimum exact object authority needed for one operation.
6. Persist security state before an operation can become externally effective.
7. Keep immutable historical evidence distinct from current release evidence.
8. Derive current truth from code, manifests, tests, and exact artifacts.
9. Represent platform lanes independently before composing a release decision.
10. Implement one complete user-visible path before broad capability expansion.
11. Use bounded commits with tests and an explicit user gate between phases.
12. Never claim native platform verification from another platform's evidence.

## 5. Remediation Overview

The `RM-*` identifiers below are local ledger handles only. They are not product
requirements or accepted task identifiers.

| ID | Priority | Class | Proposed phase | Summary |
| --- | --- | --- | --- | --- |
| `RM-001` | P0 | TRUTH-DEFECT | 2 | Establish one honest implementation-status vocabulary. |
| `RM-002` | P0 | TRUTH-DEFECT | 2 | Reconcile platform, model, decision, and release truth. |
| `RM-003` | P0 | SEQUENCING | 2 | Freeze expansion without deleting accepted scope. |
| `RM-004` | P0 | CONFIRMED-GAP | 3 | Add product CI and baseline characterization. |
| `RM-005` | P1 | CONFIRMED-DEFECT | 3 | Bind clean-build evidence to complete source identity. |
| `RM-006` | P0 | DECISION-REQUIRED | 4 | Define one kernel-owned authority transaction. |
| `RM-007` | P0 | CONFIRMED-DEFECT | 4 | Unify capability and operation taxonomies. |
| `RM-008` | P0 | CONFIRMED-DEFECT | 5 | Make effect bypass structurally unavailable. |
| `RM-009` | P0 | CONFIRMED-DEFECT | 6 | Canonicalize grant targets through validated path types. |
| `RM-010` | P0 | CONFIRMED-DEFECT | 6 | Restrict workers to exact held objects. |
| `RM-011` | P0 | CONFIRMED-GAP | 7 | Implement canonical encrypted transactional state. |
| `RM-012` | P0 | CONFIRMED-GAP | 7 | Define crash, replay, and uncertain-effect recovery. |
| `RM-013` | P1 | CONFIRMED-GAP | 8 | Implement a real Linux aggregate platform adapter. |
| `RM-014` | P1 | CONFIRMED-DEFECT | 8 | Anchor platform activation in independent release trust. |
| `RM-015` | P1 | CONFIRMED-DEFECT | 8 | Separate configuration policy from native filesystem effects. |
| `RM-016` | P1 | VALIDATION-REQUIRED | 8 | Harden configuration loading and atomic replacement. |
| `RM-017` | P1 | VALIDATION-REQUIRED | 8 | Complete Linux IPC, storage, and process lifecycle safety. |
| `RM-018` | P0 | CONFIRMED-GAP | 9 | Deliver one real Linux VS Code read-and-receipt slice. |
| `RM-019` | P1 | CONFIRMED-DEFECT | 10 | Separate historical and current evidence validation. |
| `RM-020` | P1 | CONFIRMED-DEFECT | 10 | Discover evidence in traceability instead of hard-coding absence. |
| `RM-021` | P1 | TRUTH-DEFECT | 10 | Introduce lane-scoped product and release gates. |
| `RM-022` | P1 | TRUTH-DEFECT | 10 | Label automated, native, and human review provenance exactly. |
| `RM-023` | P1 | CONFIRMED-DEFECT | 10 | Consolidate high-leverage evidence primitives. |
| `RM-024` | P1 | CONFIRMED-GAP | 10 | Add real fuzzing and meaningful concurrency tests. |
| `RM-025` | P1 | TRUTH-DEFECT | 10 | Reconcile model catalog, admission, and runtime state. |
| `RM-026` | P2 | CONFIRMED-GAP | 11 | Implement and test production Linux and VSIX packaging. |
| `RM-027` | P2 | CONFIRMED-GAP | 11 | Implement Windows as a versioned independent platform increment. |

## 6. Detailed Remediation Items

### RM-001: Honest Implementation-Status Vocabulary

**Priority:** P0
**Class:** TRUTH-DEFECT
**Dependencies:** None

**Risk addressed:** Words such as `shipped`, `enabled`, `verified`, and `pass`
currently describe unlike states. Readers and validators can mistake scaffolds,
isolated implementations, or retained evidence for integrated release behavior.

**Likely affected surfaces:** README orientation, machine architecture, model
catalog, requirement registry, traceability, gate summaries, release manifests,
and generated status views.

**Proposed work:**

- Define closed states for `planned`, `designed`, `scaffolded`, `implemented`,
  `integrated`, `contract-tested`, `native-tested`, `packaged`,
  `release-verified`, `shipped`, `blocked`, `rejected`, and `superseded`.
- Define legal transitions and evidence required for each transition.
- Keep historical records at their recorded state and revision.
- Prevent generated summaries from promoting a state without owning evidence.

**Required verification:** Schema tests, illegal-transition mutation tests,
machine-to-document consistency tests, and a repository-wide overclaim scan.

**Acceptance criteria:**

- Given any component, when its status is rendered, then exactly one defined
  state and its evidence basis are shown.
- Given a scaffold-only component, when status validation runs, then it cannot be
  labeled integrated, release-verified, or shipped.
- Given historical evidence, when current status changes, then the historical
  record remains immutable and visibly revision-bound.

### RM-002: Platform, Model, Decision, and Release Truth

**Priority:** P0
**Class:** TRUTH-DEFECT
**Dependencies:** RM-001

**Risk addressed:** The README contains retained Mac-primary wording, an enabled
Gemma claim, and first-GA language that conflicts with current implementation and
admission state. Machine validators still encode the pre-Windows architecture.

**Likely affected surfaces:** `README.md`, `PRD.md`, top execution rules in
`TASKS.md`, model profiles, build matrices, module inventories, platform enums,
packaging metadata, and accepted Decisions 0008 through 0011.

**Proposed work:**

- Describe Gemma 4 E4B and its fallback as rejected/disabled at the current
  evidence revision while retaining them as named candidates.
- Correct M5-primary language without deleting the retained Mac lane.
- Represent Windows as required and not implemented, without claiming support.
- Reconcile decision references through Decision 0011.
- Replace scaffold `shipped: true` values with the accepted status vocabulary.

**Required verification:** Cross-document authority checks, model-disposition
binding, platform-matrix validation, decision-reference validation, and negative
tests for unsupported platform and model claims.

**Acceptance criteria:**

- Given the current rejected model records, when orientation is rendered, then
  no model is described as enabled.
- Given Windows has no implementation, when platform status is rendered, then it
  is required/planned and never shipped or verified.
- Given the retained Mac lane, when first-GA scope is rendered, then Mac evidence
  does not block Fedora, Ubuntu, or Windows lane progress.

### RM-003: Stabilization Scope Control

**Priority:** P0
**Class:** SEQUENCING
**Dependencies:** None

**Risk addressed:** Additional capability expansion can further widen the gap
between accepted intent and integrated behavior.

**Likely affected surfaces:** Future decisions, implementation planning, task
promotion, progress reporting, and change-review templates.

**Proposed work:**

- Freeze new capability families until the stabilization gate passes.
- Preserve all 17 accepted epics and 227 stable requirements.
- Require an impact statement before any exception to the freeze.
- Measure progress using integrated verified paths, not document volume.

**Required verification:** Additions-only checks, explicit exception tests, and a
scope-impact checklist covering architecture, security, tests, platforms,
evidence, and release gates.

**Acceptance criteria:**

- Given an existing accepted requirement, when stabilization begins, then its
  identity and intent remain preserved.
- Given a proposed new capability, when no approved exception exists, then it is
  not added to first-GA scope.
- Given progress reporting, when a scaffold is added, then it does not count as
  an integrated user-visible path.

### RM-004: Product CI and Baseline Characterization

**Priority:** P0
**Class:** CONFIRMED-GAP
**Dependencies:** RM-001

**Risk addressed:** A green branch currently proves documentation controls but
does not prove that product code compiles or passes its default tests.

**Likely affected surfaces:** `.github/workflows`, `package.json`, Cargo commands,
VS Code shell commands, native-test runners, and branch-protection guidance.

**Proposed work:**

- Add separate format, lint, build, unit/contract, and shell jobs.
- Use the repository-declared Node, npm, and Rust versions.
- Retain documentation CI as an independent job.
- Record the eleven ignored Linux tests as a distinct native lane.
- Add deterministic failure summaries without private host information.

**Required verification:** Deliberate Rust, TypeScript, formatting, lint, and test
mutations; toolchain-version mismatch tests; and CI configuration validation.

**Acceptance criteria:**

- Given a Rust or TypeScript compilation failure, when CI runs, then the product
  gate fails even if documentation is valid.
- Given a documentation-only failure, when CI runs, then its separate job fails
  without disguising product results.
- Given a native-only test, when generic CI runs, then it is reported as pending
  native evidence rather than silently counted as passed.

### RM-005: Complete Clean-Build Source Binding

**Priority:** P1
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-004

**Risk addressed:** The retained clean-build validator hashes a curated input
list. A change to an omitted implementation file can avoid currentness failure.
Client offline flags also do not prove network-hermetic execution.

**Likely affected surfaces:** clean-build policy, build scripts, source identity,
container invocation, retained reports, and release evidence.

**Proposed work:**

- Bind evidence to an exact complete Git tree and reviewed revision.
- Define explicit, tested exclusions only for non-product transient output.
- Require report source identity to equal the reviewed source identity.
- Disable container networking after dependency bootstrap.
- Separate historical report verification from current-release applicability.

**Required verification:** Omitted-source mutation, tree mismatch, dirty-tree,
untracked-input, symlink, network-attempt, and historical-replay tests.

**Acceptance criteria:**

- Given any tracked product-source mutation, when currentness validation runs,
  then retained clean-build evidence becomes inapplicable.
- Given post-bootstrap build execution, when a process attempts network access,
  then the container boundary denies it.
- Given a historical report, when replayed against its recorded tree, then it can
  remain valid historically without becoming current release evidence.

### RM-006: Kernel-Owned Authority Transaction

**Priority:** P0
**Class:** DECISION-REQUIRED
**Dependencies:** RM-001, RM-007

**Risk addressed:** Tool registration, approval, grants, path resolution,
execution, reconciliation, and receipts currently exist as disconnected islands.

**Likely affected surfaces:** kernel engine, contracts, tool dispatcher, policy,
grant issuer, platform adapters, workers, durable store, receipts, and host.

**Proposed work:** Define one transaction owner for registered tool identity,
canonical operation, current policy, approval identity, exact grant, held target,
final consumption, attempt creation, worker launch, timeout/cancellation,
reconciliation, receipt publication, and durable terminal state.

**Required decisions:** Transaction boundary, durable write ordering, worker
launch protocol, uncertain-effect state, cancellation semantics, and public API
ownership.

**Required verification:** Bypass compile tests, replay, stale policy, stale
preimage, crash-before-consume, crash-after-consume, launch failure, timeout,
cancellation, duplicate result, and receipt-chain tests.

**Acceptance criteria:**

- Given an effect request, when no current exact grant exists, then no worker can
  be launched through a public product API.
- Given successful grant consumption, when launch fails, then durable state
  records one non-replayable terminal or explicitly recoverable outcome.
- Given any shell or capability pack, when it attempts direct effect access,
  then the type and dependency structure prevent compilation or admission.

### RM-007: Canonical Capability and Operation Taxonomy

**Priority:** P0
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-001

**Risk addressed:** The PRD authority classes, `GrantOperation`, free-form tool
effect strings, tool IDs, and configuration side-effect classes have no versioned
mapping.

**Likely affected surfaces:** contracts, configuration schema, policy, grant
templates, tool definitions, provider contracts, receipts, and UI previews.

**Proposed work:**

- Select one closed canonical operation identity and one orthogonal authority
  class model.
- Define versioned mappings for tools, grants, profiles, and providers.
- Reject unknown, ambiguous, or many-to-one escalation mappings.
- Migrate retained fixtures without rewriting historical records.

**Required verification:** Exhaustive mapping tests, unknown-value tests,
round-trip tests, downgrade/version-skew tests, and escalation mutations.

**Acceptance criteria:**

- Given a registered tool, when policy evaluates it, then the tool, grant,
  configuration, preview, and receipt resolve to one exact operation identity.
- Given an unmapped string, when configuration or a tool definition loads, then
  it fails closed.
- Given a version transition, when an old record is read, then migration is
  explicit and cannot broaden authority.

### RM-008: Structural Effect Mediation

**Priority:** P0
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-006

**Risk addressed:** Public sandbox, secret, configuration, network, and future
provider APIs do not require proof of the kernel authority transaction.

**Likely affected surfaces:** crate dependencies, public constructors, sealed
tokens, platform APIs, host dependencies, capability-pack interfaces, and tests.

**Proposed work:**

- Move effect-bearing entry points behind kernel-owned mediated interfaces.
- Use unforgeable, one-attempt typed authorization where an internal boundary
  must cross crates or processes.
- Keep observation-only APIs separately available where safe.
- Prohibit shells from depending directly on effect-bearing platform internals.

**Required verification:** Compile-fail boundary tests, dependency-graph tests,
forged-token tests, duplicate-use tests, and public-API inventory validation.

**Acceptance criteria:**

- Given host composition code, when it attempts a raw platform effect, then no
  public callable path exists.
- Given a valid one-attempt token, when it is consumed once, then reuse fails.
- Given an observation-only call, when it runs, then it cannot be converted into
  effect authority.

### RM-009: Canonical Grant Targets

**Priority:** P0
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-006, RM-007

**Risk addressed:** `GrantTarget` accepts raw strings under weaker rules than the
canonical `WorkspacePath` boundary.

**Likely affected surfaces:** grant contracts, approval, policy, fixtures,
serialization, path adapters, preview digests, and migrations.

**Proposed work:**

- Replace or version raw path components with validated path-component types.
- Bind target scope to workspace authorization and platform identity.
- Bind operation grants to held object identity and required preimage.
- Preserve content-free serialization and bounded diagnostics.

**Required verification:** Existing path corpus, Unicode normalization, encoded
separator, colon, rooted path, ambiguous suffix, workspace mismatch, stale
authorization, and serialization-invariant tests.

**Acceptance criteria:**

- Given a target rejected by `WorkspacePath`, when grant issuance is attempted,
  then issuance rejects the same target.
- Given a valid grant target, when it reaches the platform boundary, then no
  lossy or weaker reparsing occurs.
- Given a changed authorization or object identity, when consumption occurs,
  then the grant fails stale before launch.

### RM-010: Exact-Object Worker Isolation

**Priority:** P0
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-008, RM-009

**Risk addressed:** A read of one file currently mounts the entire authorized
workspace into the worker.

**Likely affected surfaces:** Linux sandbox runner, held objects, worker protocol,
mount construction, directory enumeration, exclusions, tests, and receipts.

**Proposed work:**

- Pass only descriptor-held exact objects required by the consumed operation.
- Make exclusion sets enforceable at the worker boundary.
- Use bounded scratch independently from canonical workspace objects.
- Prevent workers from resolving sibling or parent workspace content.

**Required verification:** Hostile-worker adjacent-file reads, enumeration,
descriptor substitution, symlink, mount change, proc-fd discovery, exclusion,
scratch escape, and cleanup tests.

**Acceptance criteria:**

- Given authority for one file, when the worker attempts to open a sibling, then
  the operating-system boundary denies access.
- Given one held directory with exclusions, when enumeration runs, then excluded
  entries are unavailable rather than merely filtered from output.
- Given a stale held object, when launch is attempted, then revalidation fails
  before the worker starts.

### RM-011: Canonical Encrypted Transactional Store

**Priority:** P0
**Class:** CONFIRMED-GAP
**Dependencies:** RM-006, RM-013

**Risk addressed:** In-memory grant and operation state cannot preserve replay
protection, receipts, or uncertain outcomes across restart.

**Likely affected surfaces:** new storage boundary, migrations, grants, attempts,
receipts, sessions, checkpoints, recovery, cryptographic key broker, and removal.

**Proposed work:**

- Implement the approved encrypted SQLite/SQLCipher-compatible authority.
- Establish one transactional writer and explicit migrations.
- Persist grants, nonces, attempts, receipts, revision chains, task state,
  cancellation, checkpoints, and uncertain outcomes.
- Keep live state off cloud-synchronized and network storage.

**Required verification:** Encryption-at-rest inspection, key-unavailable startup,
migration, corruption, foreign-key, concurrent writer, atomicity, rollback,
backup/restore, uninstall, and wrong-storage-class tests.

**Acceptance criteria:**

- Given a consumed grant, when the process restarts, then replay remains denied.
- Given interrupted state publication, when recovery opens the store, then it
  selects a valid prior, terminal, or explicit uncertain state.
- Given unavailable encryption authority, when startup runs, then operational
  state is not opened or silently downgraded.

### RM-012: Crash and Uncertain-Effect Recovery

**Priority:** P0
**Class:** CONFIRMED-GAP
**Dependencies:** RM-011

**Risk addressed:** No composed journal currently distinguishes no effect,
completed effect, or an effect whose outcome cannot be proven.

**Likely affected surfaces:** authority transaction, operation journal, worker
protocol, provider adapters, receipts, restart controller, UI, and diagnostics.

**Proposed work:** Define durable states and recovery behavior for every boundary
between approval, consumption, attempt creation, launch, effect, result,
verification, and receipt publication.

**Required verification:** Deterministic fault injection at every transition,
power-loss simulation, duplicate delivery, lost response, stale recovery owner,
manual resolution, and no-automatic-retry tests.

**Acceptance criteria:**

- Given a crash before an effect, when recovery runs, then the system does not
  fabricate a completed receipt.
- Given a crash after a possible effect, when outcome cannot be proven, then the
  operation becomes `uncertain` and is not retried automatically.
- Given a verified completed effect without a published receipt, when recovery
  runs, then one canonical receipt is published exactly once.

### RM-013: Production Linux Aggregate Adapter

**Priority:** P1
**Class:** CONFIRMED-GAP
**Dependencies:** RM-006, RM-011

**Risk addressed:** Linux path, sandbox, IPC, secret, storage, and inventory
mechanisms are not composed behind the aggregate startup contract.

**Likely affected surfaces:** Linux platform crate, platform contracts, startup,
workspace picker, package manifest, host composition, and native tests.

**Proposed work:** Implement one production adapter that owns verified startup,
workspace authorization, path resolution, tool confinement, process limits,
secrets, storage, local runtime identity, packaging, update state, and offline
enforcement.

**Required verification:** Every platform-capability probe, workspace selection,
foreign-handle, changed mechanism, unavailable dependency, restart, and native
conformance test.

**Acceptance criteria:**

- Given a valid installed Linux release, when startup runs, then every required
  capability is independently verified before workspace access.
- Given any required capability failure, when activation runs, then startup fails
  closed with bounded diagnostics.
- Given a selected workspace, when authorization completes, then only the
  production adapter can construct its authorized handle.

### RM-014: Independent Platform Trust Anchors

**Priority:** P1
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-013

**Risk addressed:** The startup interface obtains expected manifest identity,
observed runtime identity, capability status, and mechanism digest from the same
adapter. The kernel does not compare mechanism digests with independent expected
values.

**Likely affected surfaces:** signed release manifest, startup loader, platform
contract, capability observations, package verification, and native adapters.

**Proposed work:** Load trusted expected identities through a release-verification
boundary independent from native capability observations. Compare every observed
mechanism with its signed expected identity.

**Required verification:** Manifest substitution, mechanism digest mutation,
package mutation, rollback, expired signer, wrong platform, adapter self-report,
and trusted-loader failure tests.

**Acceptance criteria:**

- Given an adapter that reports `Verified` with an arbitrary mechanism digest,
  when activation runs, then it is rejected.
- Given a modified package or mechanism, when the manifest is unchanged, then
  startup fails before workspace access.
- Given unavailable trusted manifest authority, when startup runs, then no
  adapter can self-authorize.

### RM-015: Configuration and Platform Boundary Separation

**Priority:** P1
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-007, RM-013

**Risk addressed:** The kernel configuration module duplicates tool/grant concepts
and performs direct platform-specific filesystem mutation.

**Likely affected surfaces:** configuration schema and loader, kernel engine,
platform storage interfaces, profile translation, migrations, and tests.

**Proposed work:**

- Retain pure closed-schema, comparison, and authority-subset logic in the kernel.
- Move native loading, ownership, permissions, replacement, sync, and recovery to
  platform-owned storage.
- Translate configuration into canonical tool, grant, policy, and runtime types.
- Remove private duplicate semantics once migration tests protect behavior.

**Required verification:** Cross-platform contract tests, translation round trips,
authority-subset mutations, unsupported-platform failures, and migration tests.

**Acceptance criteria:**

- Given validated configuration, when runtime activation occurs, then registered
  tools and policy derive from canonical public contract types.
- Given kernel-only code, when built for a supported platform, then it contains
  no direct Unix filesystem dependency.
- Given a profile mutation, when translated, then authority cannot broaden
  through a representation mismatch.

### RM-016: Configuration File and Replacement Hardening

**Priority:** P1
**Class:** VALIDATION-REQUIRED
**Dependencies:** RM-015

**Risk addressed:** Current path-based reads and check-then-replace sequences may
permit symlink, ownership, permission, identity, or race failures.

**Likely affected surfaces:** platform configuration store, descriptor-relative
operations, backup, migration, rollback, permissions, and durability.

**Proposed work:** Use descriptor-relative no-follow loading, exact owner and mode
checks, regular-file checks, held preimages, same-directory atomic publication,
directory synchronization, and identity-bound rollback.

**Required verification:** Symlink swaps, hard-link changes, rename races, owner
and mode changes, cross-filesystem targets, stale backups, interrupted sync,
concurrent writers, and platform-specific durability tests.

**Acceptance criteria:**

- Given a symlink or foreign-owned configuration, when loading occurs, then it is
  rejected before parsing.
- Given a target identity change between validation and publication, when apply
  runs, then replacement fails without overwriting the changed target.
- Given a completed receipt, when power-loss recovery runs, then the selected
  configuration and retained backup agree with that receipt.

### RM-017: Linux IPC, Storage, and Process Lifecycle Hardening

**Priority:** P1
**Class:** VALIDATION-REQUIRED
**Dependencies:** RM-013

**Risk addressed:** The Unix listener does not own cleanup of its socket path;
strict-local roots do not record owner/mode admission; process inventory combines
multiple `/proc` observations without stable process identity binding.

**Likely affected surfaces:** Linux IPC, strict-local inspector, process inventory,
startup cleanup, shutdown, diagnostics, and native tests.

**Proposed work:**

- Give the listener an identity-bound cleanup lifecycle and safe stale-socket
  recovery.
- Require appropriate current-user ownership and private modes for strict-local
  operational roots.
- Bind process observations through pidfd/start-time identity where available.
- Label unavoidable observation races as uncertain rather than authoritative.

**Required verification:** Clean shutdown, crash restart, attacker-created socket,
socket replacement, owner/mode mutations, PID reuse, disappearing process,
partial `/proc` reads, and unsupported-kernel tests.

**Acceptance criteria:**

- Given orderly shutdown, when the listener drops, then only its own unchanged
  socket identity is removed.
- Given a stale or attacker-controlled socket, when startup runs, then it does not
  unlink or trust the object without identity-safe recovery.
- Given PID reuse during inventory, when observations disagree, then the record
  is rejected or marked uncertain.

### RM-018: Linux VS Code Read-and-Receipt Vertical Slice

**Priority:** P0
**Class:** CONFIRMED-GAP
**Dependencies:** RM-004, RM-006 through RM-017

**Risk addressed:** No user-facing product path currently composes the existing
security foundation.

**Likely affected surfaces:** VS Code extension, authenticated IPC, host,
configuration, workspace selection, kernel transaction, read capability, Linux
worker, durable store, receipts, diagnostics, and packaging harness.

**Proposed work:** Deliver one exact workflow: activate the extension, connect to
the verified host, select one workspace, preview one bounded read, approve it,
consume one grant, expose one exact object to one worker, return a structured
result, persist one receipt, and render an answer or denial.

**Required verification:** Clean startup, denial, malformed request, stale source,
approval mismatch, replay, adjacent-file attack, timeout, cancellation, worker
crash, host crash, restart, receipt verification, and extension deactivation.

**Acceptance criteria:**

- Given a clean supported Linux installation, when a user approves one file read
  in VS Code Chat, then the real product surface returns cited bounded content and
  one durable receipt.
- Given no approval or a stale grant, when the same request reaches the host,
  then no worker starts and a bounded denial is rendered.
- Given restart after a completed attempt, when the session resumes, then the
  operation is not repeated and the receipt remains verifiable.
- Given an adversarial worker, when it requests adjacent content, then the native
  sandbox denies access.

### RM-019: Historical and Current Evidence Architecture

**Priority:** P1
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-001, RM-005

**Risk addressed:** Frozen evidence is broadly invalidated by later policy edits
because historical validity and current applicability are conflated.

**Likely affected surfaces:** evidence schemas, source pins, validation scripts,
supersession, release gates, policy identities, and retained artifacts.

**Proposed work:**

- Verify historical evidence against its recorded commit, tree, policy, paths,
  signatures, and environment.
- Evaluate current applicability separately.
- Compute direct and transitive impact from changed owned inputs.
- Require explicit supersession rather than silent regeneration.

**Required verification:** Historical replay, current-policy change, unrelated
change, transitive dependency, signature mutation, supersession cycle, missing
source, and stale-current-gate tests.

**Acceptance criteria:**

- Given unchanged historical inputs, when current policy later changes, then the
  old artifact remains historically verifiable.
- Given a changed owning input, when current applicability is evaluated, then the
  affected artifact and transitive dependents become stale.
- Given superseding evidence, when selected, then both records remain immutable
  and their relationship is explicit.

### RM-020: Artifact-Discovering Traceability

**Priority:** P1
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-019

**Risk addressed:** The traceability generator unconditionally emits empty paths
and `not_yet_produced`, even when accepted artifacts name an acceptance test.

**Likely affected surfaces:** requirement registry, traceability generator,
artifact manifests, schemas, status calculation, and reports.

**Proposed work:** Discover artifacts through exact identifiers and validated
manifests. Derive states for absent, produced, stale, blocked, superseded,
historical-only, current, and rejected evidence.

**Required verification:** Existing `AT-AUTH-001` discovery, duplicate claim,
wrong requirement, forged path, stale source, missing artifact, supersession, and
outside-release tests.

**Acceptance criteria:**

- Given a valid current artifact naming an acceptance test, when traceability is
  generated, then its exact path and state are recorded.
- Given an artifact with no valid binding, when discovery runs, then it cannot
  create requirement completion.
- Given outside-release requirements, when reported, then their expected absence
  is distinguished from missing required evidence.

### RM-021: Lane-Scoped Gates

**Priority:** P1
**Class:** TRUTH-DEFECT
**Dependencies:** RM-001, RM-019

**Risk addressed:** Global `BLOCKED-MACOS` results hide completed shared/Linux
increments even though Mac no longer blocks first GA.

**Likely affected surfaces:** gate schemas, task summaries, platform matrices,
release composition, artifacts, and progress reports.

**Proposed work:** Represent shared, Fedora, Ubuntu, Windows, and retained Mac
results independently. Compose only the lanes applicable to a named milestone or
release.

**Required verification:** Mixed pass/block states, unsupported lane, no evidence
substitution, retained lane, release composition, and status-rendering tests.

**Acceptance criteria:**

- Given passing shared/Fedora work and missing Mac evidence, when the first-GA
  gate is evaluated, then Mac does not erase or substitute those results.
- Given required Windows evidence is absent, when v1.0 GA is evaluated, then the
  Windows lane blocks that release only.
- Given a retained Mac milestone, when it is evaluated, then Linux evidence
  cannot satisfy its native controls.

### RM-022: Exact Review Provenance

**Priority:** P1
**Class:** TRUTH-DEFECT
**Dependencies:** RM-001, RM-019

**Risk addressed:** Automated report aggregation can be described as independent
review even when no separate human or independent execution occurred.

**Likely affected surfaces:** evidence schemas, review records, gate text,
reviewer identity, reports, and release criteria.

**Proposed work:** Define separate classes for deterministic self-check,
automated aggregation, independent automated analysis, native execution,
independent human review, and external review. Permit blocked reviews to retain
findings and remediation state.

**Required verification:** Self-review mislabeling, missing reviewer, reused
process identity, nonzero findings, blocked review, re-review, and provenance
rendering tests.

**Acceptance criteria:**

- Given a script that builds and checks its own report, when provenance is
  recorded, then it is labeled self-check or aggregation, not human review.
- Given a blocked review with findings, when validated, then findings and owners
  remain visible rather than being forced to zero.
- Given a release control requiring human review, when only automation exists,
  then the control remains unsatisfied.

### RM-023: Targeted Evidence-Tool Consolidation

**Priority:** P1
**Class:** CONFIRMED-DEFECT
**Dependencies:** RM-019, RM-020

**Risk addressed:** Duplicated serialization, hashing, atomic-write, report, and
source-identity logic creates inconsistent behavior and broad change cost.

**Likely affected surfaces:** Python evidence scripts, shared validation library,
tests, schemas, and command entry points.

**Proposed work:** Extract a small reviewed library for canonical JSON, bounded
reads, hashing, atomic writes, source identity, artifact discovery, report
building, validation, and redacted diagnostics. Migrate only owning high-value
scripts in bounded groups.

**Required verification:** Golden output, cross-script compatibility, atomic
failure, oversized input, symlink, path escape, deterministic ordering, and
historical report tests.

**Acceptance criteria:**

- Given migrated scripts, when run on unchanged inputs, then authoritative output
  is byte-identical or explicitly version-migrated.
- Given a shared primitive failure, when any caller runs, then behavior and
  diagnostics are consistent.
- Given an unmigrated historical script, when retained evidence is verified, then
  consolidation does not rewrite its historical output.

### RM-024: Real Fuzzing and Concurrency Assurance

**Priority:** P1
**Class:** CONFIRMED-GAP
**Dependencies:** RM-006 through RM-010

**Risk addressed:** Current fuzzing is policy, registry, corpus, schema, and
synthetic pipeline scaffolding rather than executed product-boundary fuzzing.

**Likely affected surfaces:** fuzz workspace, path/grant/IPC/config parsers,
worker protocol, corpora, CI, sanitizer evidence, and concurrency tests.

**Proposed work:** Add real fuzz targets beginning with canonical path, grant,
tool-call, and IPC boundaries. Consume existing dictionaries and regressions.
Separate synthetic pipeline fixtures from real engine output. Add concurrency
tests that do not serialize the contested operation in the test harness.

**Required verification:** Seed replay, malformed corpus, timeout, memory limit,
sanitizer finding, minimized regression, duplicate consume, cancellation race,
and evidence provenance tests.

**Acceptance criteria:**

- Given a registered target, when fuzz execution is claimed, then a real harness
  ran against the product boundary and records toolchain/corpus identity.
- Given a synthetic fixture, when reported, then it cannot be mistaken for real
  sanitizer, coverage, or fuzz-engine output.
- Given concurrent grant attempts, when tested, then synchronization belongs to
  product code rather than a mutex that serializes the test itself.

### RM-025: Model Catalog and Runtime Truth

**Priority:** P1
**Class:** TRUTH-DEFECT
**Dependencies:** RM-001, RM-002, RM-013

**Risk addressed:** The orientation promises an enabled Gemma profile while both
evaluated candidates are rejected/disabled and no production runtime is composed.

**Likely affected surfaces:** README, model catalog, admission state, runtime
contract, installer, startup, UI picker, fallback handling, and evidence.

**Proposed work:** Show no enabled model at the baseline. Preserve named
candidates and their exact dispositions. Require a new admitted artifact/runtime
pair before activation. Never silently substitute a fallback.

**Required verification:** Rejected candidate, blocked candidate, valid admission,
artifact mutation, runtime mutation, unsupported platform, fallback refusal,
disabled picker, and offline-runtime tests.

**Acceptance criteria:**

- Given the current dispositions, when AgentMage starts, then no Gemma profile is
  offered as enabled.
- Given a future admitted profile, when any bound artifact or runtime changes,
  then activation fails until re-admission.
- Given a rejected selected model, when inference is requested, then AgentMage
  stops visibly without automatic substitution.

### RM-026: Production Linux and VSIX Packaging

**Priority:** P2
**Class:** CONFIRMED-GAP
**Dependencies:** RM-018 through RM-025

**Risk addressed:** Packaging directories contain prose and synthetic manifests,
not production RPM, DEB, or VSIX definitions and lifecycle evidence.

**Likely affected surfaces:** RPM, DEB, VSIX, signing, manifests, installer,
updater, rollback, uninstall, diagnostics, and clean test images.

**Proposed work:** Package the verified vertical slice for Fedora, Ubuntu, and
VS Code. Test installation, launch, upgrade, rollback, uninstall, data handling,
signature verification, emergency disablement, and removal.

**Required verification:** Clean-machine install, non-admin operation, signature
mutation, partial install, upgrade failure, downgrade, rollback, uninstall,
preserve/remove data choice, and reinstall tests.

**Acceptance criteria:**

- Given a supported clean Linux image, when signed packages are installed, then
  the verified vertical slice launches without developer toolchains.
- Given a package or manifest mutation, when launch occurs, then startup fails
  closed.
- Given uninstall, when the user chooses removal, then binaries, integration,
  credentials, and authorized state are removed according to the recorded choice.

### RM-027: Windows as a Versioned Platform Increment

**Priority:** P2
**Class:** CONFIRMED-GAP
**Dependencies:** Stable shared contracts from RM-006 through RM-018

**Risk addressed:** Windows is accepted first-GA scope but absent from the
current implementation and machine architecture. Adding it ad hoc could weaken
shared invariants or duplicate Linux assumptions.

**Likely affected surfaces:** platform and path contract versions, Windows crate
or service, named pipes, ACLs, handle-relative paths, restricted workers, Job
Objects, DPAPI, runtime, package, update, CI, native tests, and release gates.

**Proposed work:** Add Windows through an explicit contract version and independent
lane. Preserve Linux behavior. Define user identity, package identity, named-pipe
authentication, NTFS handle-relative scope, reparse-point policy, worker token,
Job Object limits, key protection, storage durability, native model runtime,
installation, updates, rollback, and removal.

**Required verification:** Windows build and unit tests, native non-admin tests,
ACL mutation, pipe impersonation, PID reuse, reparse points, hard links, case and
Unicode ambiguity, network-share rejection, DPAPI identity, worker escape,
resource exhaustion, package lifecycle, and cross-platform contract fixtures.

**Acceptance criteria:**

- Given the versioned shared contract, when Linux is rebuilt, then Windows
  enrollment does not weaken or change existing Linux guarantees.
- Given a standard Windows 11 user, when the native vertical slice runs, then its
  authority, path, IPC, storage, runtime, and receipt invariants match the shared
  contract through Windows-native enforcement.
- Given no native Windows evidence, when v1.0 is evaluated, then Windows remains
  visibly blocked and no other platform substitutes for it.

## 7. Approval-Gated Stabilization Phases

The phases below remain approval-gated. Phase 1 was approved and committed
locally as `f71a1ce`. Phase 2 was approved and committed locally as `9458b0e`.
Phase 3 was approved and committed locally as `e7705f0`. Phase 4 alone is
currently authorized; Phases 5 through 12 remain unapproved.

### Phase 1: Consolidated Remediation Ledger

**Authorized output:** This file only.

**Gate:** One new non-normative file; no other diff; no commit or push; user
reviews the ledger before Phase 2.

### Phase 2: Truth and Status Reconciliation

**Candidate items:** RM-001 through RM-003 and the documentation-only portion of
RM-025.

**Gate:** Accepted scope preserved; model/platform/status contradictions removed;
historical evidence unchanged; all documentation checks pass.

### Phase 3: Baseline Characterization and Product CI

**Candidate items:** RM-004 and RM-005.

**Gate:** Product compilation and tests are independent protected signals; exact
baseline failures and native-test gaps are visible; documentation remains a
separate signal.

### Phase 4: Authority Transaction and Taxonomy

**Candidate items:** RM-006 and RM-007.

**Gate:** Explicit architecture decision approved; one canonical operation model;
transaction states, owner, ordering, and failure semantics specified and tested.

### Phase 5: Structural Effect Mediation

**Candidate item:** RM-008.

**Gate:** Public APIs and crate dependencies make shell/capability bypass
unavailable; compile-fail and dependency tests pass.

### Phase 6: Canonical Targets and Exact Isolation

**Candidate items:** RM-009 and RM-010.

**Gate:** Grant and path semantics are identical; a hostile worker cannot access
adjacent or excluded content; stale held objects fail before launch.

### Phase 7: Durable State and Recovery

**Candidate items:** RM-011 and RM-012.

**Gate:** Encrypted transactional state survives restart; replay is denied; every
fault point yields a valid prior, terminal, or explicit uncertain state.

### Phase 8: Linux Platform and Lifecycle Hardening

**Candidate items:** RM-013 through RM-017.

**Gate:** One independently verified production Linux adapter exists; native
configuration, IPC, storage, and process lifecycle tests pass.

### Phase 9: Linux VS Code Vertical Slice

**Candidate item:** RM-018.

**Gate:** The real VS Code product surface performs one approved exact read and
publishes one durable receipt; denial, attack, cancellation, crash, and restart
tests pass.

### Phase 10: Evidence, Traceability, Gates, and Fuzzing

**Candidate items:** RM-019 through RM-025.

**Gate:** Historical/current evidence is distinct; traceability discovers real
artifacts; platform lanes compose honestly; real fuzz targets run; model state is
accurate.

### Phase 11: Packaging and Windows Increment

**Candidate items:** RM-026 and RM-027.

**Gate:** Linux and VSIX package lifecycles pass. Windows code and evidence pass
the independently approved boundary available on a real Windows runner. Without
native Windows evidence, the Windows portion remains blocked rather than waived.

### Phase 12: Independent Stabilization Audit and Resumption Gate

**Candidate work:** Re-run architecture, security, product, native, packaging,
evidence, traceability, fuzz, and overclaim reviews against the exact final tree.

**Gate:** No unresolved P0 finding; accepted P1/P2 residual risks explicitly
owned; authoritative documents aligned; repository clean; user explicitly
approves resuming the original numbered feature roadmap.

## 8. Gate Protocol Between Phases

At the end of every authorized phase, the implementing assistant must stop and
report:

1. Exact files changed.
2. Exact tests and checks run.
3. Pass, fail, blocked, skipped, and unavailable results.
4. Remaining findings and newly discovered risks.
5. Any deviation from the approved phase scope.
6. Worktree, branch, commit, upstream, and push state.
7. A proposed local commit subject, when a commit is appropriate.

The assistant must then ask whether the user approves completing or committing
that phase and proceeding to the next named phase. Silence, an unrelated request,
or approval of one artifact is not approval to enter the next phase. Push and
merge always require separate explicit authorization.

## 9. Cross-Cutting Risks to Monitor

| Risk | Control |
| --- | --- |
| Scope deletion disguised as stabilization | Additions-only checks and explicit user review. |
| Historical evidence rewritten for current green status | Immutable revision-bound validation and supersession. |
| Large refactor obscures security regressions | Bounded phase commits and characterization tests. |
| Linux implementation accidentally defines Windows semantics | Versioned shared contracts and independent native lanes. |
| Status vocabulary becomes another duplicated taxonomy | One schema, one transition model, generated views. |
| New persistence becomes ambient authority | One writer, encrypted store, strict-local admission, mediated APIs. |
| Exact-object sandbox breaks legitimate bounded reads | Positive fixture matrix plus adversarial adjacent-object tests. |
| CI gives false confidence without native facilities | Separate generic, Fedora-native, Ubuntu-native, and Windows-native results. |
| Evidence consolidation rewrites retained output | Golden compatibility and historical replay tests. |
| Vertical slice becomes a shortcut around architecture | Transaction, path, persistence, and platform gates must precede it. |

## 10. Decisions Required Before Relevant Implementation

The following questions remain deliberately unresolved in Phase 1:

1. What exact type owns the authority transaction?
2. What durable ordering couples grant consumption, attempt creation, effect,
   reconciliation, and receipt publication?
3. What versioned operation and authority taxonomy replaces the current
   incompatible representations?
4. Does a grant serialize a validated canonical path, a held-object reference, or
   both at different lifecycle stages?
5. How are exact regular files and bounded directories exposed to Linux workers
   without revealing the workspace root?
6. Which signed release object independently anchors expected platform mechanism
   identities?
7. Which SQLCipher-compatible binding and key-broker boundary satisfy supported
   Linux and future Windows requirements?
8. Which native CI runners are available for Fedora and Windows evidence?
9. What source-identity and supersession format governs historical evidence?
10. What is the approved migration path from duplicated configuration and
    evidence representations?

Each question requires a decision record or an amendment to an existing owning
decision before dependent production implementation begins.

## 11. Definition of Stabilization Complete

Stabilization is complete only when all of the following are true:

- The repository describes its current state without model, platform, package,
  gate, review, or component overclaims.
- A real supported Linux product path runs from VS Code Chat through the kernel
  and native worker to a durable result and receipt.
- No public effect path bypasses the kernel authority transaction.
- Grants, canonical paths, held objects, sandbox exposure, and receipts describe
  the same exact operation.
- Restart cannot replay consumed authority or hide an uncertain effect.
- Product CI compiles and tests the exact source tree.
- Native tests are represented independently from generic contract tests.
- Historical evidence remains immutable and current evidence is impact-aware.
- Traceability discovers and validates actual artifacts.
- Real fuzzing covers promoted high-risk boundaries.
- Linux packages complete their lifecycle tests.
- Windows is either independently implemented and natively verified for the
  accepted first-GA gate or remains explicitly blocked without substitution.
- An independent final audit finds no unresolved P0 issue.
- The user explicitly authorizes resuming the original roadmap.

## 12. Phase 1 Record

Phase 1 creates this ledger as a review artifact. It intentionally performs no
remediation, changes no accepted authority, refreshes no evidence, and makes no
release claim. The user approved the ledger and its local commit
`f71a1ce` before separately authorizing Phase 2. No push occurred.

## 13. Phase 2 Record

Phase 2 was authorized only for RM-001 through RM-003 and the documentation-only
portion of RM-025. The user approved its local commit as `9458b0e`; no push
occurred.

The candidate Phase 2 implementation establishes Decision 0012 and one
machine-readable status model, reconciles current orientation, model, platform,
decision, and roadmap-pause truth, replaces machine `shipped` booleans with
status references, and refreshes only current generated planning registries.
The 17 epics, 169 sprints, 227 requirements, 1,237 additions-only checklist
entries, 227 explicit empty-evidence traceability states, retained model records,
and historical Decisions 0001 through 0011 remain preserved.

Current documentation, status, architecture, registry, policy, traceability,
coverage, additions-only, schema, and product checks pass. The legacy aggregate
gate remains fail-closed when it reaches revision-bound retained evidence; for
example, the Story 4.1 security checker reports that its retained evidence
closure is stale. Phase 2 does not refresh that historical artifact or represent
it as current. Separating historical validity from current applicability across
the complete evidence system remains assigned to RM-019 in Phase 10. This known
aggregate result was not silently waived or represented as passing.

## 14. Phase 3 Record

Phase 3 was authorized only for RM-004 and RM-005. The user approved its local
commit as `e7705f0`; no push occurred. That approval separately authorized entry
into Phase 4, but it did not authorize a Phase 4 commit, push, or entry into
Phase 5.

The candidate RM-004 implementation adds a machine-readable product CI policy,
a policy validator and bounded lane runner, an independent product workflow,
and mutation tests. Format, lint, build, Rust unit/contract, and Visual Studio
Code shell results are separate signals from documentation. The generic runner
verifies the exact inventory of eleven native Linux tests while reporting them
as pending native execution, never as passed.

The candidate RM-005 implementation upgrades future clean-build evidence to
schema v2 and binds it to the exact commit, complete recursive Git tree,
committed-archive SHA-256, and an in-image verified canonical content digest. It
rejects dirty, untracked, unapproved ignored,
symbolic-link, and Git-link source states. Dependency bootstrap occurs during
the image build; verification runs in a rootless, unprivileged, read-only
container with `--network=none`. The retained schema-v1 report remains unchanged
and is explicitly historical. This clean-build-specific separation does not
claim to complete the repository-wide evidence supersession work in RM-019.

The repository's clean Linux report cannot be refreshed before the Phase 3 diff
is committed because the new source-binding contract correctly rejects the
current dirty and untracked worktree. End-to-end verification instead committed
the exact candidate diff inside a disposable repository. Its schema-v2 Fedora
44 and Ubuntu 26.04 runs both passed with `current-reviewed-source`
applicability, including in-image source-content verification, all locked
product commands, post-bootstrap network denial, and disposable supply-chain
generation and validation. The retained repository report remains unchanged.
A fresh repository report remains a post-commit action and requires separate
authorization before replacement.

Phase 3 verification produced the following candidate-gate results:

- `npm run product:check` passed format, strict lint, Rust and TypeScript build,
  153 default Rust tests, and the Visual Studio Code shell test; eleven native
  Linux tests remained explicitly ignored by the default Rust lane.
- The 25 focused product-CI and clean-build policy, mutation, source-identity,
  permission, and historical-replay tests passed.
- Product-CI contract validation, native-test inventory, retained schema-v1
  clean-build replay, Markdown, Mermaid, current-document policy validation,
  planning schema validation, and workflow YAML parsing passed.
- The disposable schema-v2 clean-build run passed both required Linux platforms
  and reported `current-reviewed-source` for the exact disposable candidate
  commit. No report or retained artifact was copied back to this repository.
- The broad historical Python suite ran 888 tests and remained fail-closed with
  22 failures and 65 errors. The reported chains are stale retained supply-chain,
  model-policy, component-inventory, vulnerability, security-map, and sprint-gate
  artifacts. `requirements:check` stopped at the same stale supply-chain
  provenance, SBOM, and dependency-hash outputs. Those artifacts were not
  refreshed, rewritten, or represented as current during Phase 3.
- `git diff --check` passed, the retained `artifacts/` tree has no diff, the
  branch remains local with no upstream, and no push occurred.

## 15. Phase 4 Record

Phase 4 was authorized only for RM-006 and RM-007. The user approved amended
Decision 0013 and the Phase 4 local commit. The candidate was committed locally
as `4136253`; no push occurred. That approval separately authorized entry into
Phase 5, but it did not authorize a Phase 5 commit, push, or entry into Phase 6.

The candidate RM-007 implementation establishes taxonomy version 1 with 22
closed operations and eight non-inheriting authority classes. A private-field
`OperationBinding` derives the one permitted class for each operation and
rejects class mismatch, unsupported versions, unknown values, wildcards, and
ambiguous legacy mappings. Configuration, tool definitions, required grant
templates, approvals, capability grants, expected effects, policies, work
packets, transaction records, and receipts now use the same closed taxonomy;
every effect-bearing record uses one exact `OperationBinding`, while descriptive
work packets use the corresponding closed authority-class vocabulary.
Because these are incompatible required-field and wire-shape changes, the live
kernel contract family and live agent-configuration bundle advance to schema
version 2. The frozen Story 4.1 version-1 package and historical configuration
migration fixtures remain the version-bound records for their retained
evidence; current parsing does not silently reinterpret either as version 2.
Configuration evidence, result, diff, rollback, profile-catalog, and other
record families retain their independent version-1 schemas.

The amended candidate also makes repository safety structural before Git
implementation begins. Clone, namespaced fetch, owned-worktree creation and
removal, compare-and-swap branch fast-forward, commit, and push are distinct
operations. Generic pull, merge, rebase, reset, clean, checkout-discard, stash,
tag or note mutation, branch deletion, remote configuration, hook execution,
force, mirror, and arbitrary ref update remain unrepresentable. The normative
repository-safety contract requires exact pre/post repository manifests,
isolated task worktrees, temporary commit indexes, authenticated credential
brokering, explicit refspecs, protected user/Git state, hostile-config denial,
remote-currentness checks, and no retry while a push result is uncertain.
The amendment adds `SR-GIT-001` through `SR-GIT-012` and quantitative reviewer
protocol `RV-49`, with first-execution ownership distributed across Sprints 42,
47, 71, 85-86, and 106 and integrated reruns assigned to Sprints 126 and 166.
Legacy additions-only checklist statements remain textually preserved; Decision
0013 explicitly supersedes unsafe literal `fetch --prune` and generic-pull
interpretations with namespaced fetch and a distinct compare-and-swap branch
fast-forward.

The candidate RM-006 implementation establishes a kernel-owned in-memory
authority transaction with fixed prepared, grant-consumed, attempt-recorded,
launch-committed, reconciling, and terminal states. It binds transaction,
attempt, approval, grant, operation, tool-call, action, task, session, and
correlation identities. Current policy and grant state are checked immediately
before atomic consumption; a non-replayable attempt and launch commitment are
recorded before crossing the worker boundary; and terminal receipts bind the
same authority identities in a hash chain.
Cancellation, launch failure, timeout, duplicate results, crashes, recovery,
and uncertain effects have explicit fail-closed semantics.

Phase 4 intentionally keeps the worker driver and transaction runner private.
Tests exercise the ordering model without creating a public effect path. Phase
5 still owns structural effect mediation; Phase 6 owns held-target isolation;
and Phase 7 owns encrypted crash-durable transactional persistence. The current
coordinator is in-memory and does not support an integrated product workflow.

Historical Story 4.1 and Story 5.1 packages, references, fixtures, and generated
reports remain unchanged and continue to describe their recorded revisions.
Accepted Decision 0013 and commit `4136253` describe the Phase 4 result. No old
artifact was refreshed or represented as current Phase 4 evidence.

Phase 4 verification produced the following candidate-gate results:

- `npm run product:check` passed Rust and TypeScript formatting, Clippy with
  warnings denied, ESLint, the strict-local source audit, all workspace builds,
  all enabled default Rust tests, the no-public-launch compile-fail test, and the
  Visual Studio Code shell test. Eleven environment-dependent native Linux tests
  remain explicitly ignored rather than represented as passing.
- `cargo test --workspace --all-targets --locked` passed every enabled unit and
  integration test. The seven authority-transaction tests cover the complete
  declared state matrix, pre-launch denial, exact success, launch failure,
  replay, cancellation, timeout, duplicate results, six crash points, receipt
  chaining, and idempotent terminal recovery.
- The focused configuration suite passed all 26 loader, migration, authority,
  rollback, and startup tests. The planning-schema suite passed all 30 live
  schema, profile, mutation, and semantic checks; its one retained-report check
  failed closed because the historical configuration schema report is stale.
- Markdown lint passed all 76 files, all 37 Mermaid blocks parsed, documentation
  and policy validation passed, and architecture and product-CI contracts
  passed.
- `requirements:check` passed registry, additions-only, architecture, module,
  dependency, injection, resolution, build-contract, and dependency-class checks
  before stopping at stale supply-chain provenance, SBOM, and dependency hashes.
- The broad historical Python suite ran 886 tests in 565.770 seconds and
  remained fail-closed with 35 failures and 78 errors. The failures cluster in
  revision-bound configuration, grant, kernel, path, supply-chain, model,
  component-inventory, vulnerability, update, security-map, and story/sprint
  evidence. Moving `GrantOperation` into the canonical taxonomy also makes the
  historical Story 5.1 source parser report its old expected location missing;
  the current Rust taxonomy and transaction tests pass. Separating historical
  validity from current applicability remains assigned to RM-019 in Phase 10.
- `git diff --check` passed. The retained `artifacts/` tree, version-1 grant
  fixtures, and frozen Story 4.1/5.1 references had no diff. The branch remained
  local with no upstream; the approved Phase 4 commit was local and no push
  occurred.

## 16. Phase 5 Record

Phase 5 was authorized only for RM-008. The user approved entry into this phase
after approving amended Decision 0013 and the Phase 4 local commit. The user
subsequently approved Decision 0014, the Phase 5 local commit, and entry into
Phase 6. That approval did not authorize a push or a Phase 6 commit.

The candidate makes the kernel transaction the sole issuer of a cross-crate
effect permit. `AuthorityTransactionRequest` has private fields and one
production constructor that rejects call/context identity drift before state
retention. `EffectAuthorization` has private fields, borrows the exact request
and consumed-grant digest, has no public constructor, clone, copy, or wire
format, and is consumed by value through `EffectDriver::execute`. The public
coordinator entry point issues it only after registry validation, current policy
evaluation, atomic grant consumption, attempt recording, and launch commitment.

Raw configuration migration, replacement, and rollback methods are now
crate-private and are exposed only through `ConfigurationEffectDriver`.
`LinuxSandboxRunner::run` is private and is exposed only through
`LinuxSandboxEffectDriver`. Linux Secret Service probe, store, lookup, and clear
methods are private and are exposed only through `LinuxSecretEffectDriver`.
The Unix listener and its bind/accept methods are module-private and are not
exported; no public product socket-creation path exists in this phase. Safe
path, configuration, manifest, inventory, and bounded-result observation APIs
remain separate and have no conversion into an effect permit.

The dependency graph now permits an effect-bearing platform adapter to import
the narrow kernel mediation API while continuing to prohibit every
kernel-to-platform, kernel-to-capability, and kernel-to-shell edge. The Linux
adapter materializes that inward edge. The macOS edge is declared but remains
`blocked-macos`. Shell and read-only capability source remain unable to consume
the permit; the read-only capability continues to import contracts only.

The new effect-boundary validator checks the permit shape, exact permit-user
inventory, raw API visibility, internal manifest edges, absence of a public
Unix listener, and direct shell/capability process, socket, and common
filesystem-mutation patterns. Mutation tests prove that reopening sandbox
execution, making the permit cloneable, adding an unregistered permit consumer,
converting an observation module into an authority consumer, adding a shell
process launch, removing the platform mediation dependency, or exporting the
listener all fail the guard.

This phase establishes structural mediation only. It does not claim exact
grant-target parity, exact held-object mounts, durable authority transactions,
restart recovery, macOS implementation, or an integrated product workflow.
Those boundaries remain assigned to Phases 6 and 7 or their existing platform
work. A trusted admitted driver can still report a false result; driver
provenance and activation remain independent build and platform responsibilities.

Phase 5 verification produced the following candidate-gate results:

- `npm run product:check` passed Rust and TypeScript format checks, Clippy with
  warnings denied, ESLint, the strict-local and effect-boundary source audits,
  every workspace build, all enabled default Rust tests, all six compile-fail
  boundary tests, and the Visual Studio Code shell test. Eleven native Linux
  tests remain explicitly ignored under their recorded environment conditions.
- The kernel suite passed 88 unit tests plus its integration suites. The public
  coordinator path executes one valid transaction, and production request
  construction rejects call/context identity drift. Existing grant, policy,
  replay, cancellation, crash, reconciliation, and receipt tests remain green.
- Nine focused effect-boundary policy and mutation tests pass. The eleven
  dependency-policy and live architecture-report tests pass, including the
  eight-edge materialized graph and four explicitly unmaterialized edges.
- Markdown lint passed all 78 files, all 39 Mermaid blocks parsed, and current
  documentation and policy validation passed.
- The retained Story 4.1 kernel architecture report fails closed as stale
  because it records the previous seven-edge graph. Its downstream dispatcher,
  boundary-integration, security-map, and gate artifacts consequently remain
  stale. They were not regenerated, rewritten, or represented as Phase 5
  evidence.
- The broad historical Python suite ran 893 tests in 587.750 seconds and
  remained fail-closed with 48 failures and 98 errors. The reported chains are
  revision-bound architecture, kernel, grant, configuration, supply-chain,
  component-inventory, model, update, vulnerability, security-map, and
  story/sprint evidence. The focused current-source and mediation checks pass;
  repository-wide historical/current evidence supersession remains RM-019.
- `git diff --check` passed and the retained `artifacts/`, `fixtures/`, and
  `references/` trees have no diff. The approved candidate is authorized for a
  local commit on branch `agent/expand-delivery-windows-ga`, which has no
  upstream. No push is authorized.

## 17. Phase 6 Record

Phase 6 was authorized only for RM-009 and RM-010 after the user approved
Decision 0014 and the Phase 5 local commit. The approved Phase 5 candidate was
committed locally as `c7bdeac`; no push occurred. Entry into Phase 6 did not
authorize a Phase 6 commit, a push, or entry into Phase 7.

The candidate replaces raw grant path components with two private-field tagged
target forms. A `workspace_scope` uses the root-capable canonical
`WorkspaceScopePath` and binds workspace authorization, adapter instance, and
platform. A `held_object` uses the non-empty canonical `WorkspacePath` and also
binds object kind, platform object identity, and a required nullable preimage.
Regular files require an exact bounded preimage and directories prohibit one.
Wildcard, empty object, dot, traversal, rooted, separator, encoded separator,
colon, ambiguous suffix, invisible-format, non-normalized Unicode, missing
field, and cross-platform identity forms fail at construction or
deserialization.

Session grants now admit only canonical scope targets and exclusions. Operation
grants admit only exact held-object targets and may inherit only exclusions
contained by the parent scope under the same authorization, adapter, and
platform identities. Approval rendering, policy evaluation, grant issuance,
final consumption, and preimage validation use those same typed targets without
weaker path reparsing. The opaque `EffectAuthorization` borrows the consumed
grant's exact targets, exclusions, and preimages. The Linux effect driver must
match its continuously held object against that permit before the private
runner can be entered.

The Linux worker no longer receives the workspace-root descriptor or an
original workspace object. For a file read, the supervisor copies only the
grant-bound bytes from the continuously held descriptor into an anonymous
file, verifies the exact byte count and SHA-256, revalidates the held object,
and seals the projection against writes, growth, shrinkage, and seal removal.
Bubblewrap copies only that immutable projection to `/input/object`. For a
directory read, the supervisor performs bounded descriptor enumeration,
conservatively removes the first descendant covered by every inherited
exclusion, sorts raw names, and supplies only a sealed NUL-delimited projection.
The source directory, children, file contents, siblings, parents, and excluded
names are absent from the worker namespace. Private bounded scratch remains
separate from canonical objects.

The live contract schema remains version 2 because no durable grant store,
released API, or compatibility promise exists before Phase 7. Raw pre-Phase 6
version-2 targets reject rather than migrate silently. Frozen version-1 Story
4.1 and Story 5.1 fixtures, references, and reports remain unchanged. Decision
0015 recorded the candidate target and worker-boundary contract. The
user subsequently accepted Decision 0015, authorized the Phase 6 local commit,
and authorized entry into Phase 7 on 2026-08-11. No push was authorized.

Phase 6 verification produced the following candidate-gate results:

- `npm run product:check` passed Rust and TypeScript format checks, Clippy with
  warnings denied, ESLint, strict-local and effect-boundary source audits, all
  workspace builds, every enabled default Rust test, all compile-fail boundary
  tests, and the Visual Studio Code shell test.
- `cargo test --workspace --all-targets` passed every enabled unit and
  integration test. Canonical-target tests cover parser parity, complete wire
  requirements, authorization and platform affinity, object identity, and
  preimage drift. Kernel tests prove that only the exact authorized held object
  reaches a driver launch record.
- Default Linux tests prove exact immutable file projection and sealing,
  exclusion-safe bounded directory projection, and stale-object denial before
  process launch.
- All 11 environment-dependent Linux sandbox tests were executed separately on
  Fedora Kinoite 44 and passed. They cover exact-file and directory-projection
  reads, sibling, parent, ambient workspace, and `/proc/self/fd` denial,
  read-only input, scratch isolation, absence of ambient paths, devices,
  processes, and network, fixed environment, `NoNewPrivileges`, seccomp, output
  bounds, and elapsed-runtime enforcement.
- `npm run docs:validate` passed all 79 Markdown files and current policy
  invariants. Ubuntu and macOS execution remain unverified and are not inferred
  from Fedora evidence.
- The broad historical Python evidence suite was not rerun for this candidate.
  Retained historical reports were not refreshed or represented as current;
  their applicability separation remains assigned to RM-019.
- `git diff --check` passed, the index is empty, and the retained `artifacts/`,
  `fixtures/`, and `references/` trees have no diff. Branch
  `agent/expand-delivery-windows-ga` remains local with no upstream. The Phase 6
  candidate is authorized for a local commit and no push is authorized.

## 18. Phase 7 Record

Phase 7 was authorized only for RM-011 and RM-012 after the user approved
Decision 0015 and the Phase 6 local commit. The approved Phase 6 candidate was
committed locally as `bf882d5`; no push occurred. Entry into Phase 7 did not
authorize a Phase 7 commit, a push, or entry into Phase 8.

The user subsequently approved Decision 0016, the Phase 7 local commit, and
entry into Phase 8 on 2026-08-11. That approval did not authorize a push or a
Phase 8 commit.

The candidate adds one SQLCipher database as canonical authority for the
currently implemented grants, anti-replay nonces, authority transactions,
operation attempts, receipts, revision histories, heads, and checkpoints. It
uses pinned `rusqlite` 0.40.2 with bundled SQLCipher and a Linux OpenSSL 3
cryptographic provider. The direct and transitive Cargo packages, SQLCipher's
BSD notice, and the Linux `libcrypto` runtime dependency are represented in the
dependency classes, license catalog, provenance, hashes, and CycloneDX SBOM.

The database key has one fixed 256-bit shape and is available to the kernel
only inside an `OperationalStoreKeyProvider` callback. The Linux implementation
can look up only the fixed `operational-store-key-v1` Secret Service identity
for one profile. It cannot list, return, select, store, or clear general
credentials through that boundary. Missing, malformed, and wrong keys fail
closed; no plaintext fallback exists. SQLCipher logging is disabled before key
validation, key material does not enter process arguments or environment
variables, and public errors and debug representations remain content-free.

Schema version 1 has an executable migration whose exact source text is hashed
and revalidated at every open. The schema uses strict tables, foreign keys,
immutable grant and transaction revisions, unique nonces and attempts, explicit
heads, unique sequenced receipts, generation compare-and-swap, deterministic
full-state digests, and append-only checkpoints. The connection requires an
exclusive single writer, zero busy timeout, WAL, full synchronization, secure
deletion, disabled trusted schema, and memory-only temporary storage. Every
open checks SQLCipher pages, SQLite structure, foreign keys, schema history,
heads, relational identity columns, canonical JSON digests, revision order,
state shapes, nonce and attempt uniqueness, receipt chain, checkpoint head, and
the complete authority-state digest.

`DurableAuthorityRuntime` is now the sole public effect-launch boundary. The
in-memory issuer and coordinator are deterministic caches with no public launch
method. Prepared state is checkpointed first; grant consumption and the matching
transaction revision are one publication; attempt and launch commitment are
each durable before the effect driver; a reconciled bounded result is durable
before terminal state; and the terminal transaction, applicable uncertain
grant revision, and receipt are one publication. Any persistence ambiguity
poisons the runtime and requires reopen and recovery before another effect.

Restart recovery never invokes a driver. Prepared state closes as failed with
the grant still issued. Grant-consumed and attempt-recorded state close as
failed with the grant still consumed. Launch-committed state closes as
uncertain and is never retried. A complete non-uncertain reconciled result
publishes its one terminal receipt; an uncertain reconciled result remains
uncertain. Terminal recovery is idempotent, and reuse of a retained transaction
or consumed grant remains denied.

A separately keyed SQLCipher online backup is verified before success and can
reconstruct the same canonical receipt state. Database, WAL, shared-memory, and
backup canary scans reject plaintext authority identifiers. Tests also cover
missing and wrong keys, future and tampered migrations, page corruption,
foreign-key orphans, a forced transaction abort with no partial generation or
checkpoint, remote and synchronized storage denial, a second writer, and all
six crash boundaries.

Proposed Decision 0016 and the durable-store architecture reference record this
candidate. The candidate deliberately does not claim complete Sprint 11 domain
tables, retention, legal holds, erasure, export, descriptor-bound Linux
state-root composition, initial key provisioning, key rotation, uninstall
orchestration, application-host composition, Ubuntu execution, macOS, Windows,
or a supported product. Those remain with their existing later phases. In
particular, a full Linux aggregate adapter must bind the inspected root and key
lifecycle before product integration.

Phase 7 verification produced the following candidate-gate results:

- `npm run product:check` passed Rust and TypeScript formatting, Clippy with
  warnings denied, ESLint, strict-local and effect-boundary audits, every
  workspace build, all enabled default Rust tests, compile-fail tests, and the
  Visual Studio Code shell test.
- `cargo test --workspace --all-targets --locked` passed 95 kernel unit tests,
  all kernel integration tests, 31 contract unit tests, all contract integration
  tests, 35 enabled Linux tests, and the remaining workspace tests. Fourteen
  environment-dependent Linux tests remain explicitly ignored rather than
  represented as Phase 7 execution evidence.
- The encrypted restart test persisted and reopened every declared crash point,
  denied replay without another driver call, distinguished verified completion
  from uncertainty, published one receipt, scanned live encrypted artifacts,
  and restored the same state from a different-key backup.
- The effect-boundary validator and all 10 mutation tests pass; exposing the
  in-memory coordinator launch method now fails the guard.
- Dependency-class validation and deterministic supply-chain provenance,
  checksum, license, and CycloneDX validation pass with the new SQLCipher stack.
- `npm run docs:validate` passed all 81 Markdown files and current policy
  invariants, including the new Decision 0016 and architecture Mermaid blocks.
- `npm run requirements:check` passed the registry, additions-only,
  architecture, module, dependency, effect-boundary, and preceding checks before
  failing closed at the retained stale dependency-injection report. That
  historical/current evidence separation remains assigned to RM-019 and was not
  silently refreshed or represented as Phase 7 evidence.
- `git diff --check` passed. No push is authorized; the approved Phase 7
  candidate is authorized only for the local commit preceding Phase 8 work.

## 19. Phase 8 Record

Phase 8 was authorized only for RM-013 through RM-017 after the user approved
Decision 0016, the Phase 7 local commit, and entry into Phase 8. The approved
Phase 7 candidate was committed locally as `46fdd03`; no push occurred. Entry
into Phase 8 did not authorize a Phase 8 commit, a push, or entry into Phase 9.

The candidate separates expected release trust from native platform
observations. Exact bounded schema-version-2 manifest bytes and a detached
Ed25519 signature enter `verify_platform_release` with an independently supplied
public key. The resulting `VerifiedPlatformRelease` binds the Linux family,
architecture, four nonzero runtime identities, and ten ordered nonzero expected
mechanism digests. `PlatformAdapter` can report only observations. Activation
rejects signer, signature, schema, status, order, platform, architecture,
runtime, capability, status, and mechanism substitution before workspace access.
The historical unsigned version-1 fixtures remain preserved but cannot enter
the production verifier; no signed release artifact or signing key is claimed.

One `LinuxPlatformAdapter` now discovers Fedora or Ubuntu and reports the fixed
native mechanism set. Construction conveys no authority. Production workspace
selection, exact-object resolution, configuration opening, authority-state
opening, and operational-key provisioning all require an independently
activated `VerifiedPlatformAdapter<LinuxPlatformAdapter>`. Missing mechanisms
are unavailable, changed or unsafe mechanisms are invalid, and the current
development executable cannot masquerade as a supported package. Fedora and
Ubuntu share the same activation semantics while retaining nonportable signed
runtime identities.

The kernel configuration module now contains only bounded parsing, migration
calculation, canonicalization, authority comparison, diffing, and result
binding. All native path and mutation code moved to `LinuxConfigurationStore`.
It retains a private descriptor-held root; requires the fixed target, backups,
and candidates to be current-user `0600`, single-link regular files on that
filesystem; compares bounded reads before and after; retains immutable
content-addressed backups; and publishes through synchronized same-directory
atomic exchange. Transaction-named candidates safely complete the exact
interrupted state after exchange and before displaced-file cleanup. Changed
preimages, links, modes, identities, backup conflicts, and foreign objects fail
without overwriting the selected target. Mutation remains private behind
`LinuxConfigurationEffectDriver` and one kernel-issued administration permit.

Linux strict-local roots now require current-user ownership and no group or
other permission bits, retain owner and mode in their identity, and provide a
separate I/O descriptor for synchronization. Authority state opens through the
exact `/proc/self/fd/<held-root>/authority.db` form after final-object creation
or verification rejects symbolic links; ordinary paths retain SQLite no-follow.
The aggregate holds and revalidates the root around the SQLCipher runtime. A
focused encrypted-store test proves that this descriptor path is executable,
not merely type-correct.

Initial operational-key provisioning is explicit and separate from startup. It
requires the verified aggregate, a private state root, a private fixed
single-writer lifecycle lock, an empty authority-state/key pairing, operating
system entropy, Secret Service standard input, and exact post-store lookup.
Existing keys are never overwritten, an existing database without its key is
refused, and normal startup cannot provision or delete. Key and state deletion,
uninstall orchestration, and interruption-safe rotation remain unimplemented;
rotation is explicitly unavailable.

The Linux listener now owns the parent and socket identities it creates and
removes only that unchanged socket on drop. Unknown existing paths are retained
and rejected; automated stale recovery is not claimed. Peer authentication binds
UID, PID, process start time, and executable digest into the one-use frame.
Inventory reads start time before collection, retains a pidfd when supported,
captures status, executable, descriptors, and sockets, and rejects a changed or
disappeared process after collection. Unsupported pidfd kernels expose the
explicit `StartTimeOnly` binding rather than claiming pidfd strength.

Accepted Decision 0017 and the platform, lifecycle, mediation, strict-local, and
durable-store architecture references record this candidate. The candidate
deliberately does not claim a production signer, signed manifest, package,
installer, updater, local model, application-host workflow, Visual Studio Code
workflow, Ubuntu-native execution, macOS, Windows, key rotation, automatic stale
socket recovery, or a supported product. Those boundaries remain assigned to
later phases.

Phase 8 verification produced the following candidate-gate results:

- `npm run product:check` passed formatting, warnings-denied Clippy, ESLint,
  strict-local and effect-boundary audits, every workspace build, all enabled
  Rust and TypeScript tests, compile-fail authority tests, and the shell
  scaffold test.
- `cargo test --workspace --all-targets --locked` passed every enabled unit and
  integration test. The focused totals include 96 kernel unit tests and 50
  enabled Linux tests. Fifteen environment-dependent Linux tests remain
  explicitly ignored by the default gate rather than silently represented as
  ordinary execution evidence.
- All 11 ignored Fedora Bubblewrap, systemd, seccomp, projection, network,
  resource, and stale-object tests passed when explicitly executed on Fedora
  Kinoite 44. All three ignored live Secret Service tests also passed, including
  the new fixed operational-key provision, verify, overwrite-refusal, and
  cleanup path.
- The isolated bind-mount replacement test could not execute its attack setup
  because this desktop session lacks mount privilege. Its explicit run failed
  at `mount` before the mutation and remains ignored and unavailable, not passed.
- The effect-boundary validator and all 12 mutation tests passed. They now reject
  kernel configuration filesystem dependencies and any reopened public native
  configuration-mutation surface.
- Deterministic supply-chain provenance, dependency hashes, license inventory,
  and CycloneDX validation passed after refresh. No new external package entered
  the locked graph.
- `npm run docs:validate` passed all 84 Markdown files and current policy
  invariants, including the version-2 signed-manifest contract and three updated
  architecture diagrams.
- `npm run requirements:check` passed the registry, additions-only,
  architecture, module, dependency, and effect-boundary checks before failing
  closed at the retained stale dependency-injection report. Historical/current
  evidence separation remains RM-019 and was not refreshed or represented as
  Phase 8 evidence.
- `git diff --check` passed. The user subsequently approved Decision 0017, the
  Phase 8 local commit, and entry into Phase 9 on 2026-08-11. The branch has no
  upstream; no push or Phase 9 commit was authorized by that approval.
