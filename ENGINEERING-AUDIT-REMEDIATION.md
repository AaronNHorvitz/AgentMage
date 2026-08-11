# AgentMage Engineering Audit Remediation Ledger

Status: **Non-normative working document**

Baseline reviewed: `abe664bde138d50fd52b3973cb70c7b5e93f5007`

Created: 2026-08-11

Current authorized phase: **Phase 2 - Truth and Status Reconciliation (pending user gate review)**

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
locally as `f71a1ce`. Phase 2 alone is currently authorized; Phases 3 through 12
remain unapproved.

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

Phase 2 is authorized only for RM-001 through RM-003 and the documentation-only
portion of RM-025. Its edits remain subject to the Phase 2 gate and user review;
authorization to edit is not authorization to commit, push, or enter Phase 3.

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
it as current. Separating historical validity from current applicability remains
assigned to RM-019 in Phase 10. This known aggregate result requires explicit
user disposition at the Phase 2 gate; it is not silently waived.
