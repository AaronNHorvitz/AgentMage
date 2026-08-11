# AgentMage - Story-Based Sprint Plan

| Field | Value |
|---|---|
| Status | Implementation in progress; delivery-system and Windows first-GA scope accepted |
| Cadence | Ordered dependency and evidence gates; no calendar duration or delivery estimate is implied |
| Scope | Complete AgentMage roadmap from foundation through v1.0 GA delivery-system closure |
| Project boundary | Independently developed by Aaron N. Horvitz on personal time and personally controlled equipment; not employer-sponsored or commissioned; intended for public distribution |
| Product authority | `PRD.md` |
| Detailed requirement authority | `Agent-Scaffolding-Inventory.md` |
| Product-security engineering baseline | `SECURITY-REVIEW.md` |
| Product vulnerability and support policy | `SECURITY.md` |
| Model admission authority | `MODEL-PROVENANCE-POLICY.md` |
| Runtime/process/socket boundary | `RUNTIME-BOUNDARIES.md` |
| Connected-delivery boundary | `DELIVERY-SYSTEM.md` |
| Windows 11 boundary | `WINDOWS-BOUNDARIES.md` |
| High-level implementation guide | `IMPLEMENTATION-PLAN.md` (derived; does not override requirements or task gates) |
| Execution rule | Work proceeds in numbered order under Decisions 0003 and 0008; Mac items remain `BLOCKED-MACOS`, but first-GA shared/Linux/Windows work may continue when technically independent and no platform evidence is substituted |

## Planning Hierarchy and Numbering

- `Epic E` is a release or major product increment and is not itself story-sized.
- `Sprint N` is one dependency-bounded planning and evidence gate with one or more reviewable stories; a story that becomes too broad is split through an appended planning decision before implementation continues.
- `Story N.S` is one user-, maintainer-, or reviewer-facing value delivered by Sprint N.
- `Task N.1.T` groups implementation, artifacts, or verification work under the story.
- `Sub-task N.1.T.U` is the smallest planned independently checkable work item.
- `Story AC N.1.ACk` is a Given/When/Then acceptance criterion for the story.
- `Sprint AC N.ACk` is a blocking sprint-level completion condition.
- Legacy `S-NNN-Ixx` and named test IDs remain in the text for traceability; the new hierarchy is the execution numbering.
- New work is appended; accepted or completed identifiers are never renumbered.

## Planning Flags and Resolutions

| Flag | Classification | Resolution |
|---|---|---|
| The phrase `tasks weeks each` did not identify a number and an earlier interpretation introduced unsupported calendar estimates. | Resolved ambiguity | Sprints are dependency and evidence gates with no calendar duration; release dates and effort estimates require a separate explicit decision. |
| Release labels such as v0.1, v0.2, and v1+ contain many independent outcomes. | Epic | Retained as numbered epics; only bounded stories receive sprint commitments. |
| Words such as complete, deep, safe, full, compatible, and review-ready can be subjective. | Vague | Bound each occurrence to source coverage, named tests, required artifacts, Given/When/Then criteria, raw evidence, and a binary PASS/BLOCKED gate. |
| Broad legacy increments combine multiple user outcomes. | Epic candidate | Split the flagged increments into sequential stories listed below; no legacy job, artifact, test, or gate was removed. |
| `DEFER` items are intentionally not implementation-ready. | Unplaceable as active work | Keep them as tested exclusions until a user-approved decision promotes them into a stable backlog and new numbered stories. |
| Customer-only decisions such as managed-device installation, allowed data, privacy, retention, accessibility acceptance, and AI-tool approval are outside product authority. | Reviewer-owned | Place evidence-production work in sprints, but reserve the actual determination for the device owner or deploying organization. |
| The delivery/security audit identified missing v0.1 policy, runtime, fuzzing, incident, accessibility, support, diagnostics, and handoff work. | Accepted planning decision | Record the independently assessed decisions in `docs/decisions/0001-product-security-and-runtime-baseline.md`; add bounded stories without deleting or renumbering prior work. |
| Required MacBook Pro M5 hardware is unavailable while independent shared and Linux work remains executable. | Accepted sequencing decision | Apply `docs/decisions/0003-blocked-platform-lane-continuation.md`: retain every Mac item and gate as `BLOCKED-MACOS`, prohibit substitution or release claims, and continue only dependency-independent work in numeric order. |
| The product expanded to a delivery control plane and Windows 11 became a first-release requirement. | Accepted scope decision | Apply `docs/decisions/0008-first-ga-delivery-system-and-windows.md`: preserve Sprints 0-102, classify them as internal/inherited milestones, append the delivery and Windows work, and close the first supported release only at `G-GA`. |

### Blocked Platform Lane

- `BLOCKED-MACOS` is a truthful incomplete state, never a pass, waiver, test skip, or supported-platform claim.
- The first unchecked item remains authoritative within each platform lane. When an item requires unavailable Mac execution or artifacts, record the blocker and continue to the next numbered item whose inputs are independent of that result.
- Platform-neutral contracts must preserve macOS requirements even when only Linux execution is currently possible.
- Mac stories, sprints, epics, and support claims remain unchecked whenever their closure depends on one or more `BLOCKED-MACOS` items. Under Decision 0008, Mac is not a v1.0 `G-GA` dependency.
- Linux evidence, mocks, cross-compilation, and static checks may prove their own declared scope only; they never satisfy a Mac checkbox or cross-platform gate.
- Mac work resumes at the earliest blocked identifier when the physical hardware or untouched evidence becomes available.

### Epic Candidates Resolved

| Legacy increment | New sequential stories | Resolution |
|---|---|---|
| `S-018` | Sprints 18-19 | Pinned Repository Structure; Repository Map Coverage and Source Resolution |
| `S-019` | Sprints 20-21 | Evidence-State Assignment; Citation Freshness and Tamper-Evident Receipts |
| `S-021` | Sprints 23-25 | Native Visual Studio Code Chat Experience; Manual Codex Handoff Boundary; v0.1 Cross-Platform Release |
| `S-023` | Sprints 27-28 | Obsidian Note Parsing; Vault Indexing, Links, and Recovery |
| `S-027` | Sprints 32-33 | Conversation Search and Branching; Private Archives and Evidence Bundles |
| `S-029` | Sprints 35-36 | Exact-Preimage Write Approval; Atomic Write Application and Rollback |
| `S-048` | Sprints 55-56 | Meeting Records and Continuity; Document, Correspondence, and Filing Control |
| `S-050` | Sprints 58-59 | Word Extraction and Structural Preservation; Word Generation and Visual Verification |
| `S-051` | Sprints 60-61 | PDF Extraction and Page Citations; PDF Generation, Redaction, and Visual Verification |
| `S-052` | Sprints 62-63 | Spreadsheet, CSV, and JSON Parsing; Reconciliation, Safe Output, and Verification |
| `S-053` | Sprints 64-65 | Presentation Workflows; Images, Redaction, and Visual Verification |
| `S-054` | Sprints 66-67 | Safe Additional File Parsers; Local Audio Transcription and Common Receipts |
| `S-063` | Sprints 76-77 | Desktop Conversation and Workspace Experience; Desktop Status, Recovery, and Packaging |
| `S-064` | Sprints 78-79 | Capability Package Trust and Lifecycle; Hooks, Safe Mode, and Package Recovery |
| `S-065` | Sprints 80-81 | Read-Only MCP Identity and Manifests; MCP Request Mediation and Failure Isolation |
| `S-066` | Sprints 82-84 | Public Research and Citations; Sandboxed Browser Inspection; Confirmed Computer Use |
| `S-067` | Sprints 85-86 | GitHub Mutation Preview and Authority; GitHub Idempotency, Recovery, and Prohibited Operations |
| `S-068` | Sprints 87-88 | Connector Governance and Isolation; Approval-Gated Connector Writes and Recovery |
| `S-069` | Sprints 89-90 | Queue, Lease, and Retry Semantics; Read-Only Schedules, Notifications, and Receipts |
| `S-071` | Sprints 92-93 | Agent Definitions and Registry; Agent Validation, Dry Runs, and Enablement |
| `S-072` | Sprints 94-95 | Child Authority and Isolation; Agent Coordination, Review, and Direction |
| `S-073` | Sprints 96-98 | Signed Updates and Supply-Chain Maintenance; Backup and Migration; Safe Mode, Diagnostics, and Operational Recovery |
| `S-074` | Sprints 99-100 | Cross-Interface Authority and Isolation; v1+ Privacy, Recovery, and Release Evidence |
| `S-075` | Sprints 101-102 | Requirement and Deferred-Scope Closure; Final Product Verification and Release Decision |

No vague or epic-sized item remains silently assigned as a single active story. If implementation discovery proves that a numbered story cannot remain independently reviewable, the story is blocked and split into newly appended story/sprint identifiers before work continues.

## Universal Story Definition of Done

Controls `G-DOD-01` through `G-DOD-13` apply to every story. Decision 0008 adds `G-DOD-14` through `G-DOD-16` prospectively to Sprints 103-126 and to any future story that introduces a connected operation or adapter. Those controls do not retroactively rewrite or reopen immutable evidence for completed local-only stories; a later change to one of those stories that introduces connected authority must satisfy the added controls.

- [ ] **G-DOD-01:** Scope, dependencies, risks, exclusions, source requirements, and applicable `SR-*` controls are recorded before implementation.
- [ ] **G-DOD-02:** Every implementation sub-task has positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect unit cases where applicable.
- [ ] **G-DOD-03:** Public contracts and schemas reject missing, extra, malformed, oversized, stale, and unsupported-version input predictably.
- [ ] **G-DOD-04:** Authority-bearing behavior uses the kernel and an exact current `CapabilityGrant`; models, prompts, shells, plugins, connectors, and child agents carry no ambient authority.
- [ ] **G-DOD-05:** Security-sensitive behavior fails closed when identity, policy, path, sandbox, encryption, network, evidence, or recovery checks are unavailable.
- [ ] **G-DOD-06:** Every tool attempt emits one schema-valid receipt, including denial, cancellation, timeout, uncertain result, and failure.
- [ ] **G-DOD-07:** Persisted data is sensitivity-labeled, minimized, encrypted when required, assigned retention, and excluded from logs/model context unless explicitly authorized.
- [ ] **G-DOD-08:** User-owned files and unrelated working-tree changes remain preserved.
- [ ] **G-DOD-09:** Raw results, normalized results, environment identity, fixture hashes, tool versions, summaries, failures, skips, retries, suppressions, and limitations reconcile exactly.
- [ ] **G-DOD-10:** Supported-platform checks pass on the story's declared matrix; Fedora, Ubuntu, Windows, and retained Apple Silicon evidence remain separate and no platform silently weakens a common contract.
- [ ] **G-DOD-11:** Required documentation, manifests, software/model/crypto bills of materials, threat cases, recovery guidance, and evidence indexes are current.
- [ ] **G-DOD-12:** Critical trust boundaries receive the required independent review with reviewer, commit, findings, disposition, and re-review recorded.
- [ ] **G-DOD-13:** Gate status is PASS or BLOCKED; no failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed blocking check is represented as passing.
- [ ] **G-DOD-14:** Every connected operation binds exact provider, host, tenant, account, project or environment, capability class, credential reference, preconditions, effect, limits, and support-matrix tuple.
- [ ] **G-DOD-15:** Every external effect has an exact preview, consumed single-use grant, idempotency or reconciliation strategy, verified postcondition, receipt, cancellation path, and rollback or compensation plan.
- [ ] **G-DOD-16:** Every promoted adapter passes its versioned conformance level and can be removed without damaging strict-local behavior or another adapter.

## Test and Evidence Contract

Every numbered implementation sub-task inherits five issue-local cases: `UT-POS`, `UT-NEG`, `UT-BND`, `UT-ERR`, and `UT-SFX`. Each case records setup, fixture identity/hash, action, expected state/value, prohibited side effects, receipt/audit assertions, cleanup, and evidence path. Property tests record seeds and shrink results; fuzz tests record corpus, duration, tool version, coverage, crashes, and sanitizer results; model tests record model/runtime manifest and decoding profile.

Evidence is stored under `artifacts/sprints/sprint-N/<story-or-test-id>/` as raw machine-readable output plus a concise generated summary. A test marked not applicable requires a written rationale and reviewer approval. A summary is never the authority over contradictory raw evidence.

These controls support public product-security review, independent verification, customer evaluation, and optional managed-device assessment. They do not claim external certification or deployment approval that has not been independently granted, and they do not transfer project ownership.

### v0.1 Reviewer Protocol First-Execution Ownership

The owning sprint performs the first complete execution possible for its boundary and retains raw evidence. Sprint 25 reruns every applicable protocol against the integrated release candidate; it does not become the first owner merely because it assembles the release evidence bundle.

| Protocol | First-execution owner | Required rerun or extension |
|---|---|---|
| `RV-01` Release identity and integrity | Sprint 3 for build inputs; Sprint 25 for signed packages | Every release candidate |
| `RV-02` Clean standard-user installation | Sprint 8 on macOS; Sprint 9 on Fedora/Ubuntu | Sprint 25 on all release platforms |
| `RV-03` Sandbox and ambient-access resistance | Sprint 8 on macOS; Sprint 9 on Fedora/Ubuntu | Every changed platform boundary and Sprint 25 |
| `RV-04` Path and race safety | Sprint 6 | Every changed path/tool boundary and Sprint 25 |
| `RV-05` IPC identity and replay | Sprint 8 | Linux extension in Sprint 9; Sprint 25 |
| `RV-06` Offline and egress proof | Sprint 10 | Every runtime adapter change and Sprint 25 |
| `RV-07` Acquisition separation | Sprint 14 | Every model/runtime artifact change and Sprint 25 |
| `RV-08` Data minimization and secret leakage | Sprint 11 | Every persistence/export surface and Sprint 25 |
| `RV-09` Cryptography and key handling | Sprint 11 | Every provider/platform change and Sprint 25 |
| `RV-10` Retention, backup, and sanitization | Sprint 11 | Every durable store and Sprint 25 |
| `RV-11` Prompt injection and authority escalation | Sprint 17 | Every new untrusted-content or tool surface and Sprint 25 |
| `RV-12` Grant mutation and replay | Sprint 5 | Every authority-bearing capability and Sprint 25 |
| `RV-13` Model and runtime provenance | Sprint 13 | Every model, quantization, runtime, image, or adapter change and Sprint 25 |
| `RV-14` Model quality and evidence integrity | Sprint 13, extended in Sprint 15 | Every model/profile change and Sprint 25 |
| `RV-15` Fuzzing and malformed input | Sprint 2 harness/corpus foundation | Continuous at every parser, IPC, model-output, path, and FFI boundary; Sprint 25 |
| `RV-16` Resource exhaustion and cancellation | Sprint 15 | Every model/runtime/tool resource-policy change and Sprint 25 |
| `RV-17` Crash recovery and state integrity | Sprint 22 | Every durable-state transition and Sprint 25 |
| `RV-18` Audit completeness and redaction | Sprint 21 | Every event/schema/export change and Sprint 25 |
| `RV-19` Vulnerability and supply-chain review | Sprint 3 for source/build inputs | Sprint 25 for shipped packages and models |
| `RV-20` Accessibility | Sprint 23 | Every user-facing surface and Sprint 25 |
| `RV-21` Incident tabletop | Sprint 25 | Every material incident/runbook change |
| `RV-22` Update, rollback, and end of support | Sprint 25 for signed manual patch delivery | Sprint 96 when automatic update capability is introduced |
| `RV-23` Provider manifest and conformance | Sprint 103 | Every adapter or provider-version promotion and Sprint 126 |
| `RV-24` Credential, host, tenant, and account isolation | Sprint 104 | Every credential or identity-path change and Sprint 126 |
| `RV-25` External effect, idempotency, and reconciliation | Sprint 105 | Every remote-write, execute, deploy, secret, or admin operation and Sprint 126 |
| `RV-26` Event, webhook, and polling integrity | Sprint 105 | Every event adapter or polling change and Sprint 126 |
| `RV-27` Delivery graph and cross-system identity | Sprint 103 | Every delivery-object or correlation change and Sprint 126 |
| `RV-28` Deployment, infrastructure, and rollback safety | Sprint 112 | Every deployment, infrastructure, flag, migration, or rollback change and Sprint 126 |
| `RV-29` Provider failure, version skew, and resource exhaustion | Sprint 123 | Every provider matrix expansion and Sprint 126 |
| `RV-30` Adapter removal and strict-local restoration | Sprint 124 | Every connected-pack lifecycle change and Sprint 126 |

## Epic Roadmap

| Epic | Product increment | Sprint range |
|---|---|---|
| Epic 0 | Foundation | Sprints 0-3 |
| Epic 1 | v0.1 - Read-Only Local Evidence Assistant | Sprints 4-25 |
| Epic 2 | v0.2 - Knowledge, Obsidian, and Memory | Sprints 26-34 |
| Epic 3 | v0.3 - Controlled Writes | Sprints 35-40 |
| Epic 4 | v0.4 - Coding and Complete Local CLI | Sprints 41-50 |
| Epic 5 | v0.5 - Manual Frontier Consultation | Sprints 51-53 |
| Epic 6 | v0.6 - Administrative and Document Work | Sprints 54-69 |
| Epic 7 | v0.7 - Read-Only GitHub and Connectors | Sprints 70-75 |
| Epic 8 | v1+ - Desktop, Extensions, Actions, Scheduling, and Agents | Sprints 76-100 |
| Epic 9 | Inherited-Roadmap Closure Checkpoint | Sprints 101-102 |
| Epic 10 | Provider-Neutral Delivery System and Windows 11 | Sprints 103-125 |
| Epic 11 | v1.0 GA Verification and Release Decision | Sprint 126 |

## [ ] Epic 0 - Foundation

### [ ] Sprint 0 - Canonical Scope and Traceability Baseline

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-000`.

**Sprint goal:** Make the project documents mechanically consistent and establish one traceable source for every requirement.

**Source coverage:** README; PRD Sections 1-5 and 18-20; inventory rules, architecture, stable backlog, release sequence, Sections 31C, 33, 35, 35A, 35B, and 35C.

**Dependencies:** None.

#### [x] Story 0.1 - Canonical Scope and Traceability Baseline

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need canonical scope and traceability baseline so that AgentMage delivers the following bounded outcome: Make the project documents mechanically consistent and establish one traceable source for every requirement.

##### Tasks and Sub-tasks

- [x] **Task 0.1.1 - Implement the bounded story**
  - [x] **Sub-task 0.1.1.1** (legacy `S-000-I01`): Create a machine-readable requirement registry for every `AM-*`, `AT-*`, and `CR-*` identifier.
  - [x] **Sub-task 0.1.1.2** (legacy `S-000-I02`): Record each requirement's title, source document, source heading, release, dependencies, disposition, acceptance tests, and current status.
  - [x] **Sub-task 0.1.1.3** (legacy `S-000-I03`): Define the conflict rule that preserves the narrower safety boundary or release scope until an approved decision resolves the conflict.
  - [x] **Sub-task 0.1.1.4** (legacy `S-000-I04`): Define decision-record, risk-register, change-log, release-manifest, and requirement-supersession schemas.
  - [x] **Sub-task 0.1.1.5** (legacy `S-000-I05`): Create a coverage check that reports duplicate identifiers, unresolved dependencies, missing acceptance tests, release mismatches, and unmapped normative statements.
  - [x] **Sub-task 0.1.1.6** (legacy `S-000-I06`): Encode all explicit exclusions, rejected defaults, and deferred items as testable policy expectations rather than prose-only notes.
  - [x] **Sub-task 0.1.1.7** (legacy `S-000-I07`): Add an additions-only inventory check that detects removed or weakened canonical requirements.
  - [x] **Sub-task 0.1.1.8** (legacy `S-000-I08`): Create a public product-security reference register that records publisher, title, publication and effective dates, source URL, retrieval date, SHA-256, current or superseded status, and the approved local-snapshot decision for every cited source without implying external certification or endorsement.

- [x] **Task 0.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 0.1.2.1:** Versioned requirement registry.
  - [x] **Sub-task 0.1.2.2:** Cross-document traceability report.
  - [x] **Sub-task 0.1.2.3:** Decision and supersession templates.
  - [x] **Sub-task 0.1.2.4:** Machine-readable exclusion register.
  - [x] **Sub-task 0.1.2.5:** Versioned public-authority register and snapshot-retention policy.

- [x] **Task 0.1.3 - Verify and close the story**
  - [x] **Sub-task 0.1.3.1:** `S-000-UT01` loads valid and malformed registries; assert canonical ordering, exact source anchors, duplicate rejection, unresolved-reference diagnostics, and zero file mutation.
  - [x] **Sub-task 0.1.3.2:** `S-000-UT02` feeds conflicting requirements in both document orders; assert the narrower safety/release rule wins deterministically and the conflict remains visible.
  - [x] **Sub-task 0.1.3.3:** `S-000-UT03` changes, removes, supersedes, redirects, or substitutes a registered public product-security reference; assert freshness/integrity failure, impact-review creation, and no silent replacement of the pinned review baseline.
  - [x] **Sub-task 0.1.3.4:** `S-000-ST01` removes, renames, weakens, or reclassifies each fixture requirement; assert additions-only and exclusion checks block closure and identify the exact changed statement.
  - [x] **Sub-task 0.1.3.5:** `S-000-IT01` rebuilds traceability from a clean checkout; assert every normative statement resolves to requirement, issue, test, release, status, and evidence fields with no orphan.
  - [x] **Sub-task 0.1.3.6 - Product security evidence:** Map `SR-GOV-001` through `SR-GOV-010`, `SR-SUP-001`, `SR-TST-001`, and `SR-TST-010`; retain the registry, conflict report, exclusion diff, raw checker output, hashes, and reviewer disposition.

##### Story Acceptance Criteria

- [x] **Story AC 0.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then registry generation is byte-identical across two clean runs, and every duplicate, orphan, contradiction, missing test, and release mismatch in the seeded corpus is detected with no false pass.
- [x] **Story AC 0.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a reviewer can navigate any v0.1 requirement to its source heading, implementation sprint, acceptance test, exclusion state, and current evidence without manual reconstruction.
- [x] **Story AC 0.1.AC3:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every public product-security reference is either linked as a current live source or retained as an approved hash-pinned snapshot with provenance; neither state is represented as publisher certification, endorsement, or approval of AgentMage.

#### [x] Story 0.2 - Public Policy, License, and Documentation Controls

**User-facing value:** As a user, maintainer, or reviewer, I need the project's license, vulnerability process, model-admission rules, runtime boundaries, and documentation checks to be explicit before implementation so that foundational assumptions cannot drift silently.

##### Tasks and Sub-tasks

- [x] **Task 0.2.1 - Establish public policy and decision artifacts**
  - [x] **Sub-task 0.2.1.1:** Publish the Apache License 2.0 as the repository license and identify it consistently in product documents and package metadata.
  - [x] **Sub-task 0.2.1.2:** Publish `SECURITY.md` with supported-version status, private vulnerability reporting, triage, signed manual patch delivery, emergency local disablement, and end-of-support behavior.
  - [x] **Sub-task 0.2.1.3:** Publish `MODEL-PROVENANCE-POLICY.md` with supplier/control, license, lineage, derivative, immutable identity, runtime, evaluation, admission, revocation, and fallback requirements.
  - [x] **Sub-task 0.2.1.4:** Publish `RUNTIME-BOUNDARIES.md` with processes, privileges, sockets, data flows, runtime adapters, lifecycle, and fail-closed conditions for macOS, Fedora, and Ubuntu.
  - [x] **Sub-task 0.2.1.5:** Record the accepted baseline in `docs/decisions/0001-product-security-and-runtime-baseline.md` without importing external audit text into the repository.

- [x] **Task 0.2.2 - Enforce documentation integrity in continuous integration**
  - [x] **Sub-task 0.2.2.1:** Add Markdown linting and Mermaid parsing with pinned tool versions and immutable workflow action revisions.
  - [x] **Sub-task 0.2.2.2:** Add local-link, secret-pattern, prohibited-claim, stable-identifier, required-document, and cross-document consistency checks.
  - [x] **Sub-task 0.2.2.3:** Add deliberate ignore rules for credentials, local model artifacts, databases, logs, build output, caches, and generated evidence without ignoring canonical planning documents.
  - [x] **Sub-task 0.2.2.4:** Document and run one local command that reproduces the documentation gate from a clean checkout.

- [x] **Task 0.2.3 - Verify and close the story**
  - [x] **Sub-task 0.2.3.1:** Seed broken links, malformed Mermaid, unresolved stable IDs, a prohibited deployment claim, and synthetic secret signatures independently; assert every case blocks while the unmodified repository passes.
  - [x] **Sub-task 0.2.3.2:** Compare all canonical documents against the accepted decision; assert license, model, runtime, platform, interface, handoff, support, and release boundaries agree.
  - [x] **Sub-task 0.2.3.3 - Product security evidence:** Map `SR-GOV-003`/`SR-GOV-004`/`SR-GOV-006`/`SR-GOV-009`/`SR-GOV-010`, `SR-SUP-001`/`SR-SUP-003`/`SR-SUP-006`/`SR-SUP-010`, and `SR-TST-001`; retain policy hashes, lint output, mutation-test output, workflow identity, and decision record.

##### Story Acceptance Criteria

- [x] **Story AC 0.2.AC1:** Given a clean checkout, when the documented local documentation command and continuous-integration workflow run, then Markdown, Mermaid, links, secrets, claims, identifiers, and cross-document assertions all pass with identical blocking semantics.
- [x] **Story AC 0.2.AC2:** Given any release artifact or model profile, when its license, support state, provenance, runtime, or vulnerability path is reviewed, then one current public document states the controlling rule and links to its evidence owner.
- [x] **Story AC 0.2.AC3:** Given seeded policy drift or sensitive material, when documentation validation runs, then the exact file and rule are reported without printing the sensitive value and the gate fails.

#### [ ] Story 0.3 - Gemma 4 E4B and Runtime Feasibility Decision

**User-facing value:** As a user and maintainer, I need evidence that Gemma 4 E4B can perform the v0.1 workload through the declared local runtimes before the architecture depends on it.

##### Tasks and Sub-tasks

- [x] **Task 0.3.1 - Admit candidate artifacts for evaluation**
  - [x] **Sub-task 0.3.1.1:** Verify the first-party Gemma 4 E4B model card, Apache-2.0 disposition, publisher/control, lineage, tokenizer, context contract, and known limitations under `MODEL-PROVENANCE-POLICY.md`.
  - [x] **Sub-task 0.3.1.2:** Resolve and record immutable source, GGUF, conversion/quantization, native runtime-build, Docker engine-image, and `ai/gemma4:e4b` model-image digests; never use a mutable tag as release identity.
  - [x] **Sub-task 0.3.1.3:** Create one versioned corpus for native macOS, native Linux, and Docker Model Runner adapters covering ordinary chat, repository tasks, citations, tool schemas, malformed output recovery, cancellation, context limits, latency, memory, and zero egress.

- [ ] **Task 0.3.2 - Execute the feasibility spike and decide**
  - [x] **Sub-task 0.3.2.1:** Run the corpus on Fedora native `llama.cpp` and the isolated Docker Model Runner compatibility adapter with explicit context and resource settings. Evidence is retained in `artifacts/sprints/sprint-0/story-0.3` and `artifacts/sprints/sprint-0/story-0.3-dmr`; the exact DMR image ran under rootless Podman compatibility deployment, not Docker Engine.
  - [ ] **Sub-task 0.3.2.2:** Run the same corpus on the MacBook Pro M5 native `llama.cpp`/Metal reference when that hardware is available; do not substitute Linux evidence for the Mac result. The exact Apple Silicon `llama.cpp b10333` archive and critical binaries are fail-closed by `model-profiles/runtimes/llama-cpp-b10333-macos-arm64.json`; `scripts/macos_model_feasibility.py` and `docs/evaluation/macos-model-feasibility.md` provide the tested M5-only execution and verification path. `scripts/story_0_3_macos_evidence.py` validates and admits the transferred raw result without rewriting it, sanitizes only the machine-local evaluation root in the public log, and preserves hashes to the untouched external source. This item remains incomplete until the physical Mac run is imported, independently recomputed, and admitted as immutable evidence.
  - [x] **Sub-task 0.3.2.3:** Record PASS, BLOCKED, or REJECTED for E4B with raw scores, failures, resource fit, runtime differences, and remediation; do not tune thresholds after viewing results without a versioned decision. The hash-bound `model-profiles/candidates/gemma-4-e4b/feasibility-disposition.json` records `REJECTED`, preserves the fixed corpus thresholds and raw Linux evidence, leaves unavailable macOS evidence unsubstituted, and triggers no automatic or authorized fallback activation.
  - [x] **Sub-task 0.3.2.4:** If E4B is rejected, evaluate the disabled Gemma 4 12B Unified fallback through the complete admission and corpus gate; never switch automatically or silently. The hash-bound `model-profiles/candidates/gemma-4-12b-unified/feasibility-disposition.json` records `REJECTED`, preserves both complete 82-trial Fedora results and their fixed thresholds in `artifacts/sprints/sprint-0/story-0.3-fallback`, retains unresolved source-lineage and reproducible-conversion blockers, leaves the unavailable MacBook Pro M5 path unsubstituted, and keeps activation and automatic switching prohibited.

- [x] **Task 0.3.3 - Verify and close the story**
  - [x] **Sub-task 0.3.3.1:** Substitute one license, lineage field, GGUF hash, OCI digest, runtime build, tokenizer, and context setting at a time; assert quarantine or visible refusal before inference. The immutable `artifacts/sprints/sprint-0/story-0.3-substitution` bundle records 16 of 16 passing one-field negative scenarios across E4B and Gemma 4 12B Unified: license, lineage, tokenizer, context, GGUF, model OCI digest, runtime OCI digest, and native runtime build each produced a visible `MODEL-IDENTITY-SUBSTITUTION` quarantine before admission-decision evaluation, with zero inference invocations.
  - [x] **Sub-task 0.3.3.2:** Probe Docker Model Runner from a LAN peer, unrelated same-user process, tool container, and separate namespace; assert only the guarded kernel adapter path succeeds. The immutable `artifacts/sprints/sprint-0/story-0.3-reachability` bundle records a passing guarded-topology feasibility matrix: the designated authenticated path returned HTTP 200; an unrelated same-user process and label-disabled tool container received HTTP 401; a separately mapped namespace was denied; and a rootless non-loopback private-network peer received connection refusal. The private DMR namespace had only loopback, zero routes, zero TCP listeners, and zero interface bytes. The peer was not a second physical workstation, the evaluation-only Python guard was not the future signed kernel, and the exact image filesystem was not run as an OCI workload in this prototype, so production DMR support remains `BLOCKED` pending the later release gates.
  - [x] **Sub-task 0.3.3.3 - Product security evidence:** Perform the feasibility portions of `RV-06`, `RV-13`, `RV-14`, and `RV-16`; retain manifests, immutable identities, corpus, raw adapter results, network probes, and the model/runtime decision. The immutable `artifacts/sprints/sprint-0/story-0.3-security` bundle hash-binds the controlling policies, fixed corpus, source and artifact records, rejected model decisions, four raw Linux adapter results, substitution refusals, and guarded DMR reachability evidence. All four feasibility portions are complete, while every full protocol remains `NOT_COMPLETE`: both evaluated models remain rejected and disabled, production DMR support remains blocked, the required MacBook Pro M5 run remains unavailable and unsubstituted, and neither independent review nor release approval is claimed.

##### Story Acceptance Criteria

- [ ] **Story AC 0.3.AC1:** Given every candidate artifact and runtime, when admission and substitution tests run, then only exact immutable policy-approved identities reach inference and every unknown or altered input fails visibly.
- [ ] **Story AC 0.3.AC2:** Given the fixed cross-adapter corpus, when E4B runs on each available reference path, then results expose quality, context, latency, memory, cancellation, and isolation differences and produce a recorded decision without automatic fallback.
- [ ] **Story AC 0.3.AC3:** Given Docker Model Runner's unauthenticated API, when reachability and zero-egress probes run, then no undeclared process, container, namespace, or non-loopback peer can reach it and no AgentMage data leaves the workstation.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 0.AC1:** Every v0.1 backlog dependency and acceptance-test reference resolves exactly once.
- [ ] **Sprint AC 0.AC2:** Every PRD v0.1 requirement maps to at least one stable backlog row.
- [ ] **Sprint AC 0.AC3:** Every competitive recommendation maps to an inventory section and release gate.
- [ ] **Sprint AC 0.AC4:** README, PRD, implementation plan, inventory, tasks, license, security/model/runtime policies, and accepted decisions agree on platforms, model, runtimes, interface, data authority, release sequence, and exclusions.
- [ ] **Sprint AC 0.AC5:** Removing or weakening a fixture requirement causes the traceability check to fail.

**Gate decision:** Sprint 0 is `BLOCKED-MACOS` until Stories 0.1 through 0.3, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Decision 0003 permits dependency-independent shared and Linux development to continue without representing this sprint or any affected downstream gate as PASS.
### [ ] Sprint 1 - Repository and Package Architecture

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-001`.

**Sprint goal:** Create the from-scratch repository structure with enforced one-way dependencies among kernel, platform adapters, capability packs, and shells.

**Source coverage:** PRD Section 5; inventory Product Architecture; `AM-KRN-001`; Sections 29 and 30.

**Dependencies:** Sprint 0; legacy dependency record: Sprint 0 (legacy S-000).

#### [ ] Story 1.1 - Repository and Package Architecture

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need repository and package architecture so that AgentMage delivers the following bounded outcome: Create the from-scratch repository structure with enforced one-way dependencies among kernel, platform adapters, capability packs, and shells.

##### Tasks and Sub-tasks

- [ ] **Task 1.1.1 - Implement the bounded story**
  - [x] **Sub-task 1.1.1.1** (legacy `S-001-I01`): Select and record implementation languages and build systems through an architecture decision supported on Apple Silicon macOS, Fedora, and Ubuntu. Evidence: accepted [`Decision 0004`](docs/decisions/0004-language-and-build-system-architecture.md), machine-readable [`language-build-matrix.json`](architecture/language-build-matrix.json), fail-closed validator, and mutation tests. This records architecture support only; macOS implementation and execution remain `BLOCKED-MACOS` under Decision 0003.
  - [x] **Sub-task 1.1.1.2** (legacy `S-001-I02`): Create top-level modules for kernel contracts, platform adapters, capability packs, shells, fixtures, packaging, documentation, and release tooling. Evidence: machine-readable [`module-inventory.json`](architecture/module-inventory.json), tracked module markers, a fail-closed path and ownership validator, and mutation tests. The macOS adapter and package are structural placeholders and remain `BLOCKED-MACOS`.
  - [x] **Sub-task 1.1.1.3** (legacy `S-001-I03`): Define dependency direction rules that forbid kernel imports from capability packs or shells. Evidence: accepted machine-readable [`dependency-rules.json`](architecture/dependency-rules.json), [compile and assembly diagrams](docs/architecture/dependency-direction.md), exact allowlist and cycle validator, and separate reverse-edge mutation tests. Actual manifest enforcement begins in Sub-task 1.1.1.4.
  - [x] **Sub-task 1.1.1.4** (legacy `S-001-I04`): Create package manifests, lock files, formatting rules, lint rules, test discovery, and reproducible development commands. Evidence: pinned Cargo, npm, TypeScript, and interface-only Swift package manifests; committed lock files; Rustfmt, Clippy, Prettier, ESLint, and strict compiler rules; passing shared/Linux build and test commands; machine-readable [`build-contract.json`](architecture/build-contract.json); and manifest mutation tests. macOS build/test commands are configured but remain `BLOCKED-MACOS` and unexecuted.
  - [x] **Sub-task 1.1.1.5** (legacy `S-001-I05`): Separate production dependencies, development dependencies, platform packaging dependencies, and optional later-capability dependencies. Evidence: machine-readable [`dependency-classes.json`](architecture/dependency-classes.json), exact manifest and lock reconciliation, empty isolated packaging dependency classes, disabled later-capability dependencies, and class-escape mutation tests.
  - [x] **Sub-task 1.1.1.6** (legacy `S-001-I06`): Add license, provenance, hash, and software-bill-of-materials generation for dependencies. Evidence: Apache-2.0 [`LICENSE`](LICENSE), deterministic [`dependency-provenance.json`](supply-chain/dependency-provenance.json), [`dependency-hashes.sha256`](supply-chain/dependency-hashes.sha256), CycloneDX 1.6 [`sbom.cdx.json`](supply-chain/sbom.cdx.json), offline generator/checker, and license/integrity/closure/status mutation tests. Missing upstream license metadata remains visible as `NOASSERTION`, not approved.
  - [x] **Sub-task 1.1.1.7** (legacy `S-001-I07`): Create a no-install diagnostic that reports missing optional components without changing the machine. Evidence: machine-readable [`optional-component-inventory.json`](architecture/optional-component-inventory.json), read-only [`no_install_diagnostic.py`](scripts/no_install_diagnostic.py), report/contract commands, fake-path and before/after mutation tests, and checks that presence remains unverified and no path or environment value is emitted. This is an internal harness, not the later native-Chat `agentmage doctor` interface.

- [x] **Task 1.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 1.1.2.1:** Repository skeleton with buildable empty modules. Evidence: [`artifact-index.json`](artifacts/sprints/sprint-1/story-1.1/artifact-index.json) binds the module inventory and package manifests; shared/Linux modules build, while the interface-only Swift module remains `BLOCKED-MACOS` and unverified.
  - [x] **Sub-task 1.1.2.2:** Architecture decision and dependency diagram. Evidence: the artifact index binds Decision 0004 and the compile/assembly diagrams by SHA-256.
  - [x] **Sub-task 1.1.2.3:** Locked dependency graph and initial software bill of materials. Evidence: the artifact index binds all three lock files, deterministic dependency provenance and hashes, and the CycloneDX 1.6 SBOM.
  - [x] **Sub-task 1.1.2.4:** Clean build, lint, format-check, and test commands. Evidence: the artifact index binds the build contract, pinned toolchain, and exact `product:build`, `product:lint`, `product:format-check`, and `product:test` commands.

- [ ] **Task 1.1.3 - Verify and close the story**
  - [x] **Sub-task 1.1.3.1:** `S-001-UT01` evaluates the dependency graph with each prohibited reverse edge injected separately; assert a precise build or architecture-check failure before packaging. Evidence: [`dependency-injection-report.json`](artifacts/sprints/sprint-1/story-1.1/dependency-injection-report.json) injects all 32 prohibited edges among source-bearing modules independently and records the exact source/target rejection from the fail-closed architecture checker.
  - [x] **Sub-task 1.1.3.2:** `S-001-UT02` resolves locked manifests twice in clean environments and with one substituted, missing, revoked, and wrong-platform dependency; assert identical approved graphs and fail-closed substitutions. Evidence: [`locked-resolution-report.json`](artifacts/sprints/sprint-1/story-1.1/locked-resolution-report.json) records two identical clean offline Cargo/npm resolutions with unchanged locks and four precise fail-closed mutation results. Swift resolution remains `BLOCKED-MACOS`; Linux results make no Mac support claim.
  - [x] **Sub-task 1.1.3.3:** `S-001-ST01` scans source, build output, packages, and SBOMs for undeclared binaries, licenses, secrets, dynamic loaders, and privileged assumptions; assert every seeded violation is reported. Evidence: [`artifact-scan-report.json`](artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json) scans real source/SBOM surfaces, clean synthetic build/package surfaces, and detects all five isolated seeded violations. The excluded development-only `khroma` license gap remains visible as open review; native macOS binary/package scans remain `BLOCKED-MACOS`.
  - [ ] **Sub-task 1.1.3.4:** `S-001-IT01` executes documented build, lint, format, test, SBOM, and diagnostic commands as a clean standard user on each reference platform; assert no ambient development dependency is used. Linux evidence: [`clean-build-report.json`](artifacts/sprints/sprint-1/story-1.1/clean-build-report.json) records passing Fedora 44 and Ubuntu 26.04 runs from the same committed input closure, each as UID/GID 10001 in a rootless digest-pinned container with dropped capabilities, read-only source/root filesystems, fresh writable state, exact toolchains, clean dependency installation, ten passing commands, unchanged generated supply-chain artifacts, and no detected ambient dependency. The required macOS run remains `BLOCKED-MACOS`, so this cross-platform sub-task remains unchecked.
  - [x] **Sub-task 1.1.3.5 - Product security evidence:** Map `SR-PLT-001`, `SR-PLT-010` through `SR-PLT-012`, `SR-SUP-002` through `SR-SUP-006`, `SR-SUP-011`, and `SR-TST-003`; retain clean-build logs, graph reports, SBOMs, licenses, hashes, and environment manifests. Evidence: [`security-evidence-map.json`](artifacts/sprints/sprint-1/story-1.1/security-evidence-map.json) maps all 11 requirements exactly once and hash-binds the retained clean-build logs, dependency graphs, lockfiles, SBOM, license, provenance, scans, environment manifests, policies, and decisions. The map marks every product-wide requirement `not-complete`, preserves the open development-only `khroma` license review, records no release claim, and retains macOS as `BLOCKED-MACOS`; this checkbox closes the evidence-mapping work only.

##### Story Acceptance Criteria

- [ ] **Story AC 1.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the empty product builds reproducibly from pinned inputs, contains only manifest-declared components, and preserves one-way kernel/platform/capability/shell dependency boundaries.
- [ ] **Story AC 1.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then removing any optional component degrades only its declared capability; removing a required security primitive blocks startup with a non-secret diagnostic.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 1.AC1:** A clean checkout builds and runs the empty test suite on all reference platforms.
- [x] **Sprint AC 1.AC2:** Static dependency tests prove the kernel imports no shell or capability pack. Evidence: the accepted dependency rules, compile/assembly diagrams, exact graph validator, and [`dependency-injection-report.json`](artifacts/sprints/sprint-1/story-1.1/dependency-injection-report.json) reject all 32 independently injected prohibited edges.
- [x] **Sprint AC 1.AC3:** Dependency resolution is reproducible from lock files and approved artifacts. Evidence: [`locked-resolution-report.json`](artifacts/sprints/sprint-1/story-1.1/locked-resolution-report.json) records identical clean Cargo/npm graphs from two offline locked resolutions and fail-closed substituted, missing, revoked, and wrong-platform inputs; Swift remains `BLOCKED-MACOS` without weakening the demonstrated shared/Linux dependency result.
- [ ] **Sprint AC 1.AC4:** No required Mac end-user dependency assumes Homebrew, Rosetta, ambient Python, ambient Git, Xcode command-line tools, Docker Desktop, or administrator access.
- [ ] **Sprint AC 1.AC5:** Missing optional dependencies degrade only their declared capabilities.

**Gate decision:** Sprint 1 is PASS only when Story 1.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 2 - Test Harness and Synthetic Corpus

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-002`.

**Sprint goal:** Build the safe fixture and evidence framework required to test every later capability before real user data is touched.

**Source coverage:** inventory Sections 31, 31A, 31B, and 31C.

**Dependencies:** Sprint 1; legacy dependency record: Sprint 1 (legacy S-001).

#### [ ] Story 2.1 - Test Harness and Synthetic Corpus

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need test harness and synthetic corpus so that AgentMage delivers the following bounded outcome: Build the safe fixture and evidence framework required to test every later capability before real user data is touched.

##### Tasks and Sub-tasks

- [x] **Task 2.1.1 - Implement the bounded story**
  - [x] **Sub-task 2.1.1.1** (legacy `S-002-I01`): Create fixture generators for Markdown workspaces, Obsidian vaults, Git repositories, supported parser languages, unsupported languages, and malformed inputs. Evidence: the fixture-only [`generator-profile.json`](fixtures/generator-profile.json), deterministic standard-library [`fixture_generator.py`](scripts/fixture_generator.py), focused mutation/reproducibility tests, and [`fixture-generator-report.json`](artifacts/sprints/sprint-2/story-2.1/fixture-generator-report.json) cover all six required families without external commands, network use, executable files, existing-destination overwrite, private data, real credentials, remote references, product parser-admission claims, or macOS support claims. Its ephemeral preview is now fulfilled by the separately hash-bound corpus v1 artifact in Sub-task 2.1.2.1.
  - [x] **Sub-task 2.1.1.2** (legacy `S-002-I02`): Create path traversal, symlink escape, case collision, Unicode normalization, mount-change, file-replacement, and stale-bookmark fixtures. Evidence: the portable synthetic [`path-fixture-profile.json`](fixtures/path-fixture-profile.json), side-effect-free [`path_fixture_generator.py`](scripts/path_fixture_generator.py), focused tests, and [`path-fixture-report.json`](artifacts/sprints/sprint-2/story-2.1/path-fixture-report.json) cover all seven scenario families. Real symlinks remain contained within a generated synthetic sandbox; corpus v1 stores only inert symlink declarations. Case/Unicode collisions use portable logical names; mount changes and stale bookmarks use fake identities with no privileged operation or genuine bookmark; no product path-safety or macOS execution claim is made.
  - [x] **Sub-task 2.1.1.3** (legacy `S-002-I03`): Create secret canaries, prompt-injection content, conflicting instructions, malformed model calls, grant replay, and approval-bypass fixtures. Evidence: the inert synthetic [`adversarial-fixture-profile.json`](fixtures/adversarial-fixture-profile.json), deterministic [`adversarial_fixture_generator.py`](scripts/adversarial_fixture_generator.py), focused tests, and [`adversarial-fixture-report.json`](artifacts/sprints/sprint-2/story-2.1/adversarial-fixture-report.json) cover all six families. Canary values are unmistakably synthetic and retained only in generated/versioned fixtures; evidence stores hashes/counts, malformed calls never execute, grant/approval records explicitly state that no authority was minted, and no product-security claim is made.
  - [x] **Sub-task 2.1.1.4** (legacy `S-002-I04`): Create deterministic fake model, fake tool, fake inference runtime, fake connector, fake clock, fake secret store, and crash injector adapters. Evidence: the test-only [`fake-adapter-contract.json`](fixtures/fake-adapter-contract.json), typed deterministic [`fake_adapters.py`](fixtures/fake_adapters.py), focused tests, and [`fake-adapter-report.json`](artifacts/sprints/sprint-2/story-2.1/fake-adapter-report.json) define all seven adapters and eight success/failure modes. The fakes use no network, process execution, real workspace, persistent state, or real secret; traces hash requests and redact synthetic secret values; no product-adapter or platform-support claim is made.
  - [x] **Sub-task 2.1.1.5** (legacy `S-002-I05`): Create document fixtures for text, JSON, Word, Portable Document Format, spreadsheets, presentations, images, archives, notebooks, and logs. Evidence: the inert [`document-fixture-profile.json`](fixtures/document-fixture-profile.json), deterministic standard-library [`document_fixture_generator.py`](scripts/document_fixture_generator.py), structural/security tests, and hash-only [`document-fixture-report.json`](artifacts/sprints/sprint-2/story-2.1/document-fixture-report.json) cover all ten required families. OOXML, PDF, PNG, ZIP, and notebook structures are real and reproducible; fixtures contain no macros, scripts, formulas, external relationships, executable archive entries, notebook execution history, private paths, or real credentials. The ephemeral preview is fulfilled by corpus v1 and makes no product parser or macOS claim.
  - [x] **Sub-task 2.1.1.6** (legacy `S-002-I06`): Create expected-output manifests with content hashes, source ranges, evidence states, receipts, and prohibited side effects. Evidence: [`expected-output-profile.json`](fixtures/expected-output-profile.json), deterministic [`expected_output_manifests.py`](scripts/expected_output_manifests.py), focused corruption/reproducibility tests, and hash-only [`expected-output-report.json`](artifacts/sprints/sprint-2/story-2.1/expected-output-report.json) define one synthetic expectation for each Observed, Derived, Inferred, and Unknown/Blocked state. Citations bind whole-content hashes to zero-based half-open UTF-8 byte ranges and one-based inclusive line ranges; derivations name their method and inputs; inferences name synthetic model/runtime/template identities; denials expose a reason; receipts bind request/output hashes in a deterministic chain; all six prohibited side effects have expected count zero. Corpus v1 now versions all four golden manifests; no product receipt implementation, authority, or macOS claim is made.
  - [x] **Sub-task 2.1.1.7** (legacy `S-002-I07`): Build a platform result recorder that captures environment identity without secrets. Evidence: the deny-by-default [`platform-result-profile.json`](fixtures/platform-result-profile.json), standard-library [`platform_result_recorder.py`](scripts/platform_result_recorder.py), injection/tamper tests, and deterministic [`platform-result-recorder-report.json`](artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json) persist only eleven allowlisted Linux environment fields and exact build, fixture-set, model, runtime, and policy identities/hashes. Thirteen identifying or sensitive field classes are prohibited; ambient environment-variable and secret-store values are never retained; records preserve pass/fail state and carry a canonical self-hash. Live Fedora collection passed without persisting the current host record. macOS collection/execution remains `BLOCKED-MACOS`, and no macOS support claim is made.
  - [x] **Sub-task 2.1.1.8** (legacy `S-002-I08`): Build test sharding, retry classification, flaky-test detection, bounded output, and immutable result-bundle generation. Evidence: [`test-result-bundle-profile.json`](fixtures/test-result-bundle-profile.json), deterministic [`test_result_bundle.py`](scripts/test_result_bundle.py), focused reconciliation/tamper tests, and hash-only [`test-result-bundle-report.json`](artifacts/sprints/sprint-2/story-2.1/test-result-bundle-report.json) assign every test by SHA-256 modulo, preserve all nine synthetic results, classify retries/flakes/quarantine/skip/cancellation/failure/timeout exactly, and retain UTF-8 excerpts within 64 bytes while hashing raw output and redacting synthetic canaries. Per-result, per-shard, platform-context, and whole-bundle hashes make mutation detectable; summaries are recomputed from raw normalized results and retain five final non-pass cases. The claim is tamper evidence by content addressing, not filesystem immutability; the preview is not persisted, executes no tests, and leaves macOS `BLOCKED-MACOS`.

- [x] **Task 2.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 2.1.2.1:** Versioned synthetic corpus and golden manifests. Evidence: [`corpus-profile.json`](fixtures/corpus-profile.json), deterministic [`versioned_corpus.py`](scripts/versioned_corpus.py), the checked-in 1.0.0 [`agentmage-synthetic-corpus-v1.zip`](fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip), external self-hashed [`manifest.json`](fixtures/corpus/v1/manifest.json), corruption/reproducibility tests, and [`versioned-corpus-report.json`](artifacts/sprints/sprint-2/story-2.1/versioned-corpus-report.json) bind 77 entries across base, path, adversarial, document, and golden families. The ZIP uses stored, unencrypted, non-executable regular entries with no absolute/traversal/duplicate/symlink names; two symlink escapes remain inert declarations for temporary-sandbox materialization. The manifest binds every byte plus all source profiles, generators, and assembler; four goldens cover every evidence state. No product support or macOS execution claim is made, and macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.2.2:** Shared acceptance-test runner. Evidence: [`acceptance-runner-profile.json`](fixtures/acceptance-runner-profile.json), platform-neutral [`shared_acceptance_runner.py`](scripts/shared_acceptance_runner.py), focused registration/failure/skip/error/cancellation/redaction/tamper tests, and hash-only [`shared-acceptance-runner-report.json`](artifacts/sprints/sprint-2/story-2.1/shared-acceptance-runner-report.json) run eight deterministic checks in declared order over corpus integrity, golden receipts, base and fault-adapter modes, inert path declarations, document safety, platform redaction, and non-pass result reconciliation. The runner bounds diagnostics to 256 UTF-8 bytes, redacts synthetic canaries/private paths, retains every case with typed status, rejects missing/extra handlers, self-hashes every result/run, executes no external command or network action, touches no user data, and uses explicit logical rather than wall-clock timing. The canonical synthetic run passes 8/8 without claiming product acceptance; macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.2.3:** Platform result schema and fixture provenance ledger. Evidence: strict Draft 2020-12 [`platform-result.schema.json`](schemas/testing/platform-result.schema.json) and [`fixture-provenance-ledger.schema.json`](schemas/testing/fixture-provenance-ledger.schema.json), deterministic [`fixture_provenance.py`](scripts/fixture_provenance.py), the checked-in self-hashed [`provenance-ledger.json`](fixtures/corpus/v1/provenance-ledger.json), schema and mutation tests, and [`fixture-provenance-report.json`](artifacts/sprints/sprint-2/story-2.1/fixture-provenance-report.json). The ledger closes over all 11 root fixture contracts, five corpus source families, five support contracts, four evidence-state goldens, 35 unique artifact nodes, and 34 typed acyclic edges; it verifies repository-relative paths and every artifact hash while rejecting stale/missing/duplicate sources, dangling or cyclic lineage, private paths, secret-shaped values, raw environment retention, weakened controls, and unsupported platform claims. Both synthetic Linux platform records validate against the formal schema. No product support claim is made, and macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.2.4:** Crash, network, resource, and adversarial test adapters. Evidence: [`fault-test-adapter-profile.json`](fixtures/fault-test-adapter-profile.json), typed side-effect-free [`fault_test_adapters.py`](fixtures/fault_test_adapters.py), deterministic [`fault_test_adapter_evidence.py`](scripts/fault_test_adapter_evidence.py), 16 focused behavioral and mutation tests, and hash-only [`fault-test-adapter-report.json`](artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json). The 29-event canonical trace covers five crash/commit states, eight allowlist and transport outcomes, all six bounded resource dimensions plus an admitted case, five corpus-bound adversarial dispositions, and pre-cancellation for every adapter. Operations use only synthetic `.invalid` endpoints and integer resource accounting; they execute no payload, socket, process, filesystem write, or real exhaustion, retain no raw payload, bound events/details, expose typed commit uncertainty, and enforce idempotent cleanup and post-close rejection. The shared runner passes the added fault-adapter closure check, provenance binds the new contract, no product support claim is made, and macOS remains `BLOCKED-MACOS`.

- [x] **Task 2.1.3 - Verify and close the story**
  - [x] **Sub-task 2.1.3.1:** `S-002-UT01` regenerates every corpus item from a pinned seed; assert expected hashes, metadata, provenance, and golden outputs are stable and corruption is detected before use. Evidence: all five source profiles and the corpus profile carry validated pinned synthetic seeds; [`story_2_1_verification.py`](scripts/story_2_1_verification.py), focused tamper tests, and [`corpus-reproducibility-report.json`](artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json) independently regenerate the 77-entry corpus twice in clean temporary destinations, verify byte-identical archives/manifests against checked artifacts, bind five source/provenance families and all four evidence-state goldens, and detect seeded archive and manifest corruption before use without persisting it. No current-host or private user data is used, no product support claim is made, and macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.3.2:** `S-002-UT02` drives every fake adapter through success, denial, malformed result, cancellation, timeout, crash-before-commit, crash-after-commit, and uncertain-result states; assert typed outcomes and cleanup. Evidence: the shared [`fake-adapter-contract.json`](fixtures/fake-adapter-contract.json) now specifies idempotent cleanup and post-close rejection, all seven adapters implement that contract, and the fake clock exposes a test-only mode probe without wall-clock access. [`story_2_1_verification.py`](scripts/story_2_1_verification.py), focused lifecycle/tamper tests, and [`adapter-mode-verification-report.json`](artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json) execute the complete 7-by-8 matrix, retain 56 typed outcomes and per-adapter trace hashes, require 56 post-close rejections, stop the runtime, clear the synthetic secret store, reset the clock, and retain no secret value or real side effect. No product support claim is made, and macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.3.3:** `S-002-ST01` searches all fixtures and generated artifacts for real-looking credentials, private paths, hidden executables, active formulas, remote references, and network behavior; assert zero prohibited content. Evidence: recursive [`fixture_security_scan.py`](scripts/fixture_security_scan.py), eight focused scope/detector/tamper tests, and [`fixture-security-scan-report.json`](artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json) inspect every controlled fixture and Sprint 2 artifact plus all 77 corpus entries and four nested ZIP/Office containers. The scanner checks credential shapes, private paths, executable names/modes/magics, PDF actions, notebook code, Office formulas/external relationships, non-reserved URLs, and network-capable imports/tokens; intentional labeled synthetic canaries and `.invalid` endpoints remain permitted. Six seeded detector categories are all caught, canonical findings are zero, no network call or sensitive value is retained, no product support claim is made, and macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.3.4:** `S-002-IT01` recomputes a summary from sharded raw results containing failure, skip, retry, flaky, and quarantine fixtures; assert none are omitted or converted to pass. Evidence: an independent reducer in [`story_2_1_verification.py`](scripts/story_2_1_verification.py), focused omission/duplication/pass-conversion tests, and [`summary-reconciliation-report.json`](artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json) flatten and reconcile all three shards without calling the producer summary function. The content-addressed comparison binds nine unique results, five final non-pass cases, skips, six retry attempts, two flakes, quarantine, cancellation, timeout, redaction, and truncation; producer and independently recomputed summaries match exactly while mutations fail closed. The verifier persists no raw result or output, makes no product acceptance claim, and leaves macOS `BLOCKED-MACOS`.
  - [x] **Sub-task 2.1.3.5 - Product security evidence:** Map `SR-TST-001` through `SR-TST-006`, `SR-TST-010`, `SR-DAT-002`, `SR-OPS-003`, and `SR-AI-005`; retain corpus manifest, generator seeds, adapter traces, corruption results, and signed summary comparison. Evidence: [`story_2_1_security_evidence.py`](scripts/story_2_1_security_evidence.py), nine focused signature/mapping/tamper/boundary tests, independently scanned final-envelope artifacts, and [`security-evidence-map.json`](artifacts/sprints/sprint-2/story-2.1/security-evidence-map.json) map all ten requirements exactly once and hash-bind the retained corpus, profiles, adapter and corruption evidence, fixture scan, and summary reconciliation. The independently recomputed [`summary-comparison.json`](artifacts/sprints/sprint-2/story-2.1/summary-comparison.json) has a verified detached Ed25519 signature and retains only public verification material; the ephemeral private key was not retained. Every product-wide requirement remains `not-complete`, no release or product-acceptance claim is made, and macOS remains `BLOCKED-MACOS` with evidence substitution prohibited.

##### Story Acceptance Criteria

- [x] **Story AC 2.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every later input class has a versioned normal, boundary, malformed, hostile, oversized, cancellation, and recovery fixture with declared prohibited side effects. Evidence: the versioned [`later-input-class-fixtures-v1.json`](fixtures/story-2.1/later-input-class-fixtures-v1.json), deterministic generator/validator, seven mutation tests, and [`input-class-fixture-matrix-report.json`](artifacts/sprints/sprint-2/story-2.1/input-class-fixture-matrix-report.json) cover the complete 12-by-7 cross-product of all input classes named for Story 2.2. All 84 bounded synthetic fixtures declare seven prohibited side-effect counts at zero; oversized, cancellation, and recovery cases are logical bounded seeds and do not exhaust host resources or claim concrete product-parser support.
- [x] **Story AC 2.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then an independent runner can reproduce corpus identity and test summaries without access to private user data or the original development machine. Evidence: [`story_2_1_gate.py`](scripts/story_2_1_gate.py), seven gate/tamper/overclaim tests, and [`story-gate-report.json`](artifacts/sprints/sprint-2/story-2.1/story-gate-report.json) independently rebuild two byte-identical 77-entry corpora in clean temporary destinations and recompute the nine-result sharded summary exactly from repository-relative synthetic inputs. The verifier binds reviewed commit `bee933d341bbeeb74f307ce9940c304f8c989b17`, requires no private or original-machine data, and records no external-human review claim.

**Story gate evidence:** Both story criteria and all shared/Linux foundation work pass. The story remains `BLOCKED-MACOS` under `G-DOD-10`; [`story-gate-report.json`](artifacts/sprints/sprint-2/story-2.1/story-gate-report.json) prohibits Linux evidence substitution, records no macOS execution or support claim, and therefore leaves the Story 2.1 checkbox open.

#### [ ] Story 2.2 - Continuous Trust-Boundary Fuzzing Foundation

**User-facing value:** As a user and reviewer, I need malformed-input testing to begin with the first parsers and protocols so that security defects are found while each boundary is still small.

##### Tasks and Sub-tasks

- [x] **Task 2.2.1 - Build reusable fuzz infrastructure**
  - [x] **Sub-task 2.2.1.1:** Define fuzz-target contracts for manifests, configuration, IPC, model output, capability grants, paths, text encodings, Git objects, repository parsers, SQLite imports, archives, and every FFI boundary. Evidence: versioned [`target-registry.json`](fuzzing/target-registry.json), deterministic [`fuzz_target_registry.py`](scripts/fuzz_target_registry.py), seven focused closure/discovery/mutation tests, and [`target-registry-report.json`](artifacts/sprints/sprint-2/story-2.2/target-registry-report.json) register exactly one bounded baseline contract for each of the 12 required trust-boundary classes and bind every target to all seven Story 2.1 seed scenarios. Each contract prohibits network, external processes, user-file writes, ambient authority, private data, and real credentials; crashes, timeouts, resource exhaustion, secret exposure, authorization bypass, and path escape are non-pass outcomes. Source discovery confirms zero active FFI boundaries in the current tree and blocks future unregistered Rust/C/native boundaries. These are fake-boundary contracts only, with no product-boundary execution or macOS support claim.
  - [x] **Sub-task 2.2.1.2:** Record pinned engines, sanitizers, dictionaries, seed corpora, maximum input/resource limits, minimum local and continuous-integration durations, crash deduplication, and regression-corpus retention. Evidence: [`toolchain-policy.json`](fuzzing/toolchain-policy.json), six hash-bound libFuzzer dictionaries, deterministic [`fuzz_toolchain_policy.py`](scripts/fuzz_toolchain_policy.py), seven focused pin/budget/deduplication/retention mutation tests, and [`toolchain-policy-report.json`](artifacts/sprints/sprint-2/story-2.2/toolchain-policy-report.json) pin `cargo-fuzz` 0.13.2 with its crate hash, Rust `nightly-2026-08-01` with its channel-manifest hash, Jazzer.js 4.0.0 with npm integrity, and the dependency-free fake-boundary engine. The policy requires AddressSanitizer, Linux leak checks, native-FFI undefined-behavior checks, and Jazzer.js command/path/request bug detectors; binds three seed-corpus artifacts; caps input, time, memory, output, corpus, and concurrency; sets 60-second local, 300-second changed-target CI, and 3,600-second scheduled minima; defines canonical crash signatures, mandatory bounded minimization, and reviewed regression retention. Third-party tools are recorded but not installed, acquisition remains separate from offline execution, no product fuzz run is claimed, and macOS remains `BLOCKED-MACOS`.
  - [x] **Sub-task 2.2.1.3:** Add a standard result schema for seed, coverage, sanitizer state, crash signature, minimized reproducer, timeout/resource event, owner, severity, disposition, and evidence hash. Evidence: strict Draft 2020-12 [`fuzz-result.schema.json`](schemas/testing/fuzz-result.schema.json), canonical self-hashed [`fuzz-result.valid.json`](schemas/testing/examples/fuzz-result.valid.json), deterministic [`fuzz_result_contract.py`](scripts/fuzz_result_contract.py), six focused semantic mutation tests, the shared 17-test AJV schema suite, and [`fuzz-result-schema-report.json`](artifacts/sprints/sprint-2/story-2.2/fuzz-result-schema-report.json). The closed schema requires exact target/registry/harness/corpus/dictionary/build/policy/platform/engine identities, run seed and limits, coverage, sanitizer state, typed non-pass failure and crash signature, bounded minimized reproducer, timeout/resource state, ownership/severity/disposition, evidence hashes, and redaction. Pass records cannot retain a failure, non-pass records cannot omit one, unknown fields fail, raw payloads/private paths/canary values are prohibited, and no product fuzz or macOS support claim is made.
  - [x] **Sub-task 2.2.1.4:** Require every future trust-boundary story to register and execute its targets before its sprint gate can pass. Evidence: versioned [`story-gate-policy.json`](fuzzing/story-gate-policy.json), executable [`fuzz_story_gate.py`](scripts/fuzz_story_gate.py), eight focused gate-pass/block/waiver/overclaim tests, and [`gate-policy-report.json`](artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json) map all 12 targets to 13 owner-sprint gates through 14 assignments and require exact registration, registry hash, corpus, resource policy, schema-valid result, pass status, regression path, and successful replay evidence. The evaluator blocks missing or extra evidence, unimplemented boundaries, stale identities, invalid schemas, every non-pass status, and failed regression replay; ordinary development credentials cannot waive it. All future gates are currently and correctly blocked because their product boundaries are not implemented, no product execution is claimed, macOS remains `BLOCKED-MACOS`, and `npm run requirements:check` now runs the policy validator on every documentation/CI check.

- [x] **Task 2.2.2 - Verify and close the story**
  - [x] **Sub-task 2.2.2.1:** Seed crashing, hanging, resource-exhausting, path-escaping, secret-leaking, and authorization-bypassing fixtures; assert deterministic detection, minimization, ownership, and gate failure. Evidence: versioned [`security-failures-v1.json`](fuzzing/seeds/security-failures-v1.json), deterministic [`seeded_fuzz_failures.py`](scripts/seeded_fuzz_failures.py), seven focused detection/minimization/ownership/redaction/tamper tests, six formal-schema validations, six hash-named regression reproducers, and [`seeded-failure-report.json`](artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json). The bounded fake-boundary campaign detects all six required classes and emits six typed non-pass results, deterministic crash signatures, minimized reproducers, explicit owners/severities, `quarantined-blocking` dispositions, and forced gate blocks. Hangs and resource exhaustion use logical counters without exhausting the host; the secret canary is derived only in memory and does not appear in reports or persisted reproducers. No network, private data, real credential, product-boundary execution, or macOS support claim is made.
  - [x] **Sub-task 2.2.2.2:** Run the empty/fake-boundary baseline twice from clean environments; assert corpus and result reconciliation while failed or timed-out cases never become passes. Evidence: deterministic [`fuzz_baseline_reconciliation.py`](scripts/fuzz_baseline_reconciliation.py), seven clean-root/reconciliation/mutation tests, and hash-only [`baseline-reconciliation-report.json`](artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json) execute the six-seed fake campaign twice from independently created empty temporary roots containing only 13 hash-allowlisted inputs. Both runs retain identical input, campaign, normalized-result, coverage, evidence, regression, and reconciliation hashes; all twelve observed result instances remain non-pass and both logical timeout instances remain hangs. Inputs are unchanged, temporary roots and raw payloads are not retained, and no network, private data, original-machine dependency, product-boundary execution, or macOS support claim is made.
  - [x] **Sub-task 2.2.2.3 - Product security evidence:** Establish `RV-15`; map `SR-SUP-009`, `SR-TST-002`, `SR-TST-004`, `SR-TST-006`, and `SR-TST-012`; retain target registry, pinned tools, seed corpus, seeded-failure results, minimized reproducers, and reviewer disposition. Evidence: deterministic [`story_2_2_security_evidence.py`](scripts/story_2_2_security_evidence.py), seven mapping/tamper/overclaim tests, [`rv-15-control-map.json`](artifacts/sprints/sprint-2/story-2.2/rv-15-control-map.json), [`security-evidence-map.json`](artifacts/sprints/sprint-2/story-2.2/security-evidence-map.json), and [`reviewer-disposition.json`](artifacts/sprints/sprint-2/story-2.2/reviewer-disposition.json) map all five requirements exactly once and hash-bind the target/tool/gate policies, formal result contract, seed corpus, six reports, and all six minimized reproducers. The RV-15 fuzz-foundation execution is complete with retained blocking fixtures, while the full protocol and every product requirement remain `NOT_COMPLETE` pending concrete owner-boundary campaigns. Independent review and release approval remain pending, no Linux/fake evidence substitutes for macOS, and no product-boundary or release claim is made.

##### Story Acceptance Criteria

- [x] **Story AC 2.2.AC1:** Given a newly registered trust boundary, when its story gate is evaluated, then a versioned fuzz target, corpus, resource budget, raw result, and regression path are required or the gate blocks. Evidence: [`story-gate-report.json`](artifacts/sprints/sprint-2/story-2.2/story-gate-report.json) independently verifies 12 target contracts and 13 owner gates; exact target registration, corpus, policy, schema-valid raw result, pass status, regression path, and replay fields are mandatory, missing/stale evidence blocks, and ordinary development credentials cannot waive the gate.
- [x] **Story AC 2.2.AC2:** Given each seeded security failure, when the fuzz harness runs, then it produces a bounded minimized reproducer and a non-pass disposition without exposing secret-canary values. Evidence: the independent [`story-gate-report.json`](artifacts/sprints/sprint-2/story-2.2/story-gate-report.json) confirms all six seeded failures remain non-pass, all six minimized reproducers are retained, all six target gates block, and no secret-canary value is recorded.
- [x] **Story AC 2.2.AC3:** Given two clean runs with identical inputs and tools, when summaries are generated, then target identity, crash classification, coverage fields, and evidence hashes reconcile exactly. Evidence: the independent [`story-gate-report.json`](artifacts/sprints/sprint-2/story-2.2/story-gate-report.json) confirms both clean runs and all ten input, campaign, result, regression, target, classification, coverage, evidence, non-pass, and timeout-preservation comparisons reconcile exactly without the original development machine.

**Story gate evidence:** All three story criteria and all shared/Linux foundation work pass. The story remains `BLOCKED-MACOS` under `G-DOD-10`; [`story-gate-report.json`](artifacts/sprints/sprint-2/story-2.2/story-gate-report.json) independently reviews pushed evidence commit `5c342303a5c30078d0ac6e7370d77bdf5152e51c`, prohibits Linux or fake-boundary evidence substitution, records no product-boundary execution, macOS support, product acceptance, external-human review, or release claim, and therefore leaves the Story 2.2 checkbox open.

#### Sprint Acceptance Criteria

- [x] **Sprint AC 2.AC1:** Recreating the corpus yields identical expected hashes where determinism is required. Evidence: [`sprint-gate-report.json`](artifacts/sprints/sprint-2/sprint-gate-report.json) independently confirms two byte-identical corpus regenerations against checked artifacts and two clean fake-boundary runs with all ten reconciliation comparisons exact.
- [x] **Sprint AC 2.AC2:** A deliberately corrupted fixture is detected before test execution. Evidence: [`sprint-gate-report.json`](artifacts/sprints/sprint-2/sprint-gate-report.json) confirms both archive and manifest mutations are detected before use, with no corrupt artifact persisted.
- [x] **Sprint AC 2.AC3:** Fake tools and models can simulate success, denial, malformed output, cancellation, timeout, crash, and uncertain completion. Evidence: [`sprint-gate-report.json`](artifacts/sprints/sprint-2/sprint-gate-report.json) verifies the complete seven-adapter by eight-mode matrix: 56 typed cases, all expected outcomes matched, all cleanup passed, and all 56 post-close calls rejected.
- [x] **Sprint AC 2.AC4:** Test results identify the exact fixture, platform, build, model, runtime, and policy versions. Evidence: [`sprint-gate-report.json`](artifacts/sprints/sprint-2/sprint-gate-report.json) retains both synthetic Linux result identities with exact fixture, distribution/version/architecture, build, model, runtime, policy, record, and content hashes while recording no ambient environment values.
- [x] **Sprint AC 2.AC5:** No fixture contains a real credential or private user file. Evidence: [`sprint-gate-report.json`](artifacts/sprints/sprint-2/sprint-gate-report.json) independently requires the recursive fixture/security scan to retain zero blocking findings and no raw sensitive values or network calls while all six seeded prohibited-content categories remain detectable.

**Gate decision:** Sprint 2 is PASS only when Stories 2.1 and 2.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

**Current gate evidence:** All five sprint criteria and the shared/Linux foundation pass. Sprint 2 remains `BLOCKED-MACOS`; [`sprint-gate-report.json`](artifacts/sprints/sprint-2/sprint-gate-report.json) aggregates both story gates from pushed evidence commit `d628b91caefacd15256fd4ec3008e4a208070004`, leaves both story and sprint checkboxes open, preserves `G-DOD-10` as the sole shared gate blocker, and prohibits macOS evidence substitution or unsupported product/release claims.
### [ ] Sprint 3 - Configuration, Versioning, and Build Integrity

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-003`.

**Sprint goal:** Make configuration explicit, versioned, fail-closed, and incapable of silently broadening authority.

**Source coverage:** `AM-CFG-001`; inventory Sections 29 and 30; `AT-CFG-001`.

**Dependencies:** Sprint 2; legacy dependency record: Sprint 1 (legacy S-001), Sprint 2 (legacy S-002).

#### [ ] Story 3.1 - Configuration, Versioning, and Build Integrity

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need configuration, versioning, and build integrity so that AgentMage delivers the following bounded outcome: Make configuration explicit, versioned, fail-closed, and incapable of silently broadening authority.

##### Tasks and Sub-tasks

- [x] **Task 3.1.1 - Implement the bounded story**
  - [x] **Sub-task 3.1.1.1** (legacy `S-003-I01`): Define versioned schemas for core, platform, model, workspace, tool, permission, budget, logging, retention, skill, and shell configuration. Evidence: eleven closed Draft 2020-12 section schemas under [`schemas/configuration`](schemas/configuration), their shared definitions and closed [`agent-configuration.schema.json`](schemas/configuration/agent-configuration.schema.json) bundle, a safe synthetic [`agent-configuration.valid.json`](schemas/configuration/examples/agent-configuration.valid.json), strict AJV validation in [`validate_planning_schemas.mjs`](scripts/validate_planning_schemas.mjs), seven focused schema/coercion/authority/resource mutation tests, and [`configuration-schema-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-schema-report.json). All 12 canonical section/bundle records validate, while 48 missing-version, unknown-field, wrong-type, and unsupported-version mutations fail closed without coercion or defaults. Version 1 requires fail-closed startup, default deny, restrict-only inheritance, standard-user execution, local model/shell transport, no automatic routing or model-to-tool channel, and redacted/minimized logging. This defines configuration contracts only; no product loader or macOS execution/support claim is made.
  - [x] **Sub-task 3.1.1.2** (legacy `S-003-I02`): Create development, synthetic-test, strict-local read-only, knowledge, write, coding, and later network profiles without enabling later profiles early. Evidence: the closed [`profile-catalog.schema.json`](schemas/configuration/profile-catalog.schema.json), seven complete generated configurations and [`catalog.json`](configuration/profiles/catalog.json), deterministic [`configuration_profiles.py`](scripts/configuration_profiles.py), seven focused identity/phase/authority/network/overclaim tests, two formal AJV profile tests, and [`profile-catalog-report.json`](artifacts/sprints/sprint-3/story-3.1/profile-catalog-report.json). Development, synthetic-test, and strict-local read-only are current-foundation definitions; knowledge, write, coding, and later-network are `future-disabled` behind explicit release and loader gates. All seven have product registration false, all four future definitions retain no more than the read-only effective baseline, and even the network placeholder keeps permission, model, and shell network access disabled. The library loader added in Sub-task 3.1.1.3 is not registered at product startup, no profile is activated, and no macOS execution/support claim is made.
  - [x] **Sub-task 3.1.1.3** (legacy `S-003-I03`): Implement schema validation, unknown-field rejection, safe defaults, migration, configuration diff, backup, and rollback. Evidence: [`configuration.rs`](kernel/engine/src/configuration.rs) provides a bounded duplicate-aware JSON parser, closed typed version 1 contract validation, explicit strict-local read-only defaults, deterministic version 0-to-1 migration, hash-only redacted differences, private content-addressed backups, atomic replacement, and preimage-checked rollback. Twelve focused Rust tests cover canonical loading; unknown, duplicate, missing, unsupported, oversized, out-of-schema, and authority-broadening input; the complete schema-failure matrix; deterministic non-broadening migration; published migration fixtures; redacted diff classification; exact backup/rollback identity; invalid apply; and stale rollback preservation. The offline [`configuration_loader_evidence.py`](scripts/configuration_loader_evidence.py) gate and five report-integrity tests bind those results to exact source/profile/fixture hashes in [`configuration-loader-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-loader-report.json). The module is compiled and linted on Fedora and Ubuntu but is not registered at product startup, activates no profile, performs no network operation, persists no raw configuration evidence, and makes no macOS execution/support claim.
  - [x] **Sub-task 3.1.1.4** (legacy `S-003-I04`): Prevent configuration files, environment overrides, child settings, repository files, and model output from broadening permissions. Evidence: [`RestrictedConfigurationSource`](kernel/engine/src/configuration.rs) closes the untrusted source set to configuration-file, environment, child-profile, repository, and model-output candidates. `ConfigurationManager::verify_parent_profile` admits a parent authority only after strict Ed25519 verification over its domain, record type, schema version, profile identifier, and canonical configuration SHA-256; `load_restricted_candidate` then requires each complete candidate to pass the versioned loader and remain a subset of that verified parent. The comparison covers platform identity; model activation/runtime/limits; workspace roots/access/symlinks; tool identity/enablement/capabilities; permission capabilities/network/grants; resource budgets; logging verbosity/size/redaction; retention/backup; skill catalogs/limits/capabilities; and shell identity/execution/network. Eight focused Rust tests prove valid restrictions succeed through all five channels while capability, root, model, tool, platform, budget, logging, retention, malformed-input, signature, key, replay, and non-capability diff broadening fail closed without raw candidate disclosure or parent mutation. The offline [`configuration_authority_evidence.py`](scripts/configuration_authority_evidence.py) gate and five report-integrity tests bind the exact test/source/channel/dimension closures in [`configuration-authority-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-authority-report.json); Sub-task 3.1.3.2 supplies exhaustive mutation evidence for 97 permission-bearing leaves and 485 channel attempts. No environment value is read, no raw candidate or private path is persisted, no profile is registered or activated, the test signing key is synthetic, and no production trust root or macOS execution/support claim is made.
  - [x] **Sub-task 3.1.1.5** (legacy `S-003-I05`): Record a configuration hash in each session and release result. Evidence: [`ConfigurationBoundResult`](kernel/engine/src/configuration.rs) can be constructed only from a validated `LoadedConfiguration` and binds each closed `session` or `release` result kind to its result identifier, external payload SHA-256, exact configuration profile identifier and canonical SHA-256, plus a deterministic record SHA-256 without persisting raw configuration. The closed [`configuration-result.schema.json`](schemas/configuration/configuration-result.schema.json), two canonical fixtures, and planning [`release-manifest.schema.json`](schemas/planning/release-manifest.schema.json) require that identity at both result-contract and release-manifest boundaries. Four focused Rust tests cover exact identity propagation, deterministic minimized serialization, configuration/payload separation, and malformed identity/hash rejection; 29 Node schema tests include valid session/release records and missing, unknown, invalid-kind, invalid-hash, and release-manifest identity failures. The offline [`configuration_result_evidence.py`](scripts/configuration_result_evidence.py) gate and five report-integrity tests bind those contracts, exact tests, source hashes, and unsupported-claim closures in [`configuration-result-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json). Fedora and Ubuntu clean builds pass for implementation commit `467b505`; actual product session/release execution and profile activation remain owned by later tasks, and no macOS execution/support claim is made.
  - [x] **Sub-task 3.1.1.6** (legacy `S-003-I06`): Inventory each approved interpreter, executable, package, runtime, and optional dependency with version and hash. Evidence: the enforced [`component-inventory-policy.json`](architecture/component-inventory-policy.json) and deterministic [`component_inventory.py`](scripts/component_inventory.py) reconcile rather than duplicate the clean-build executable evidence, locked dependency provenance/SBOM, dependency classes, optional-component inventory, and native/Docker/macOS runtime-candidate records. [`component-inventory-report.json`](artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json) binds 427 approved build/dependency inputs: seven versioned and SHA-256-hashed executables on each of Fedora and Ubuntu, two digest-bound clean-build environments, and all 411 Cargo, npm, workspace, and blocked-macOS Swift package records with versions and native cryptographic hashes. No approved record lacks a version or hash. Seven component-inventory tests plus 13 clean-build policy/evidence tests reject missing executable identities, package closure drift, early runtime/optional approval, stale source hashes, and Linux-to-macOS claim substitution. Three exact native/Docker/macOS model runtime identities remain `candidate-not-approved`; nine optional, prerequisite, development, and macOS-build executables remain `presence-only-not-approved`; and approved product-runtime and optional-dependency sets remain empty. Fedora and Ubuntu clean builds pass for implementation commit `c932419`; no product runtime/profile is activated and no macOS execution/support claim is made.
  - [x] **Sub-task 3.1.1.7** (legacy `S-003-I07`): Add signed-update and rollback design records while keeping automatic update checks disabled. Evidence: accepted design Decisions [`0005`](docs/decisions/0005-signed-manual-update-design.md) and [`0006`](docs/decisions/0006-update-rollback-design.md), their closed machine-readable [`signed-update-design.json`](architecture/signed-update-design.json) and [`rollback-design.json`](architecture/rollback-design.json) authorities, and deterministic validation in [`update_design.py`](scripts/update_design.py). The update design permits only user-selected local bundles acquired outside the AgentMage runtime; requires detached signatures, explicit algorithm/key/threshold metadata, a local trust root, pre-extraction verification, immutable release/configuration/component/BOM identities, increasing release sequence, compatibility and impact records, atomic staging/activation, durable transitions, and preservation of the prior release until post-activation verification. Automatic checks, background downloads, remote triggers/control, transaction network access, and downgrade paths remain prohibited. Rollback is a new exact-precondition local approval transaction that reverifies target signatures, hashes, support, revocation, platform, backups, and schema compatibility; separates binary/configuration rollback from data recovery; forbids blind restore and later-user-data overwrite; and blocks stale, revoked, unsupported, mismatched, or irreversible targets. Seven mutation/report-integrity tests reject hidden network authority, weakened signature/identity controls, unsafe rollback paths, unknown fields, implementation overclaims, stale evidence, and macOS substitution. [`update-rollback-design-report.json`](artifacts/sprints/sprint-3/story-3.1/update-rollback-design-report.json) binds both decisions and their sources. Fedora and Ubuntu clean builds pass for design commit `2358b91`; no verifier, stager, activation/rollback implementation, production signer, package-level `RV-22` result, or macOS execution/support claim is made.

- [x] **Task 3.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 3.1.2.1:** Configuration schemas and migration fixtures. Evidence: five published fixtures under [`fixtures/configuration/migration`](fixtures/configuration/migration) provide one valid version 0 input, its exact version 1 output, and three invalid unsupported-version, reserved-version, and missing-section inputs. The real Rust loader test executes every fixture, proves the valid migration emits the expected canonical bytes and hash through 13 deterministic operations, and proves all invalid inputs fail closed.
  - [x] **Sub-task 3.1.2.2:** Profile catalog with explicit capability deltas. Evidence: [`capability-deltas.json`](configuration/profiles/capability-deltas.json) compares all seven profiles with the strict-local read-only baseline, separates effective authority from future planned capability, and records zero effective authority broadening, network enablement, or product registration.
  - [x] **Sub-task 3.1.2.3:** Dependency and executable inventory. Evidence: the shared [`component-inventory-report.json`](artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json) remains the single authoritative artifact and binds 427 approved Fedora/Ubuntu build and dependency inputs with versions and hashes. Three model runtimes remain candidate-not-approved, nine optional or prerequisite components remain presence-only-not-approved, approved product-runtime and optional-dependency sets remain empty, and no macOS result is inferred.
  - [x] **Sub-task 3.1.2.4:** Configuration diff and rollback report format. Evidence: the closed [`configuration-diff.schema.json`](schemas/configuration/configuration-diff.schema.json) and [`configuration-rollback-report.schema.json`](schemas/configuration/configuration-rollback-report.schema.json), plus canonical examples, require stable path ordering, hash-only values, exact preimage/backup identities, atomic transition state, retained private backup state, and explicit rollback outcome without persisting raw values or private paths. [`configuration-review-artifacts-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json) binds all four deliverables and records five migration fixtures, seven profile deltas, two report formats, and zero effective profile broadening. Fedora and Ubuntu clean builds pass for implementation commit `f1fae4e`; no product loader/profile activation or macOS execution/support claim is made.

- [x] **Task 3.1.3 - Verify and close the story**
  - [x] **Sub-task 3.1.3.1:** `S-003-UT01` validates each schema with missing, extra, wrong-type, oversized, unsupported-version, and ambiguous fields; assert one stable diagnostic and no partial startup. Evidence: [`every_configuration_schema_failure_class_is_stable_and_side_effect_free`](kernel/engine/src/configuration.rs) executes the six failure classes against all 11 section schemas and the complete bundle for 72 fail-closed cases. Every case is repeated to prove the bounded code/diagnostic is stable, then submitted to the atomic apply boundary to prove it cannot alter the active configuration or create a backup/temporary file; the manager remains usable with the strict-local read-only default after all rejections. The dedicated offline [`configuration_schema_failure_evidence.py`](scripts/configuration_schema_failure_evidence.py) gate, five report-integrity tests, and [`configuration-schema-failure-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json) close the exact schema, failure-class, test, source-hash, no-side-effect, and unsupported-claim sets. Fedora and Ubuntu clean builds pass for implementation commit `4d1f455`; product startup registration/profile activation and macOS execution/support remain unclaimed.
  - [x] **Sub-task 3.1.3.2:** `S-003-UT02` mutates every permission-bearing value through files, environment, repository settings, child profiles, and model output; assert effective authority never exceeds the signed parent profile. Evidence: the complete [`permission-bearing-values.json`](configuration/permission-bearing-values.json) registry identifies 97 unique permission-bearing leaves across 67 authority dimensions, including 59 schema-valid broadenings and 38 closed-contract violations, and explicitly excludes three non-authority identity/version fields with rationale. [`every_permission_bearing_value_is_rejected_through_every_untrusted_channel`](kernel/engine/src/configuration.rs) executes every mutation through all five closed untrusted channels for 485 total attempts and accepts zero broadenings; the configuration-file channel reads an actual local candidate file, schema-valid mutations first prove they satisfy the closed contract, and safety-invalid mutations fail before authority comparison. [`parent_profile_signature_verification_rejects_tampering_and_wrong_keys`](kernel/engine/src/configuration.rs) proves an Ed25519-signed parent cannot be forged with a changed signature or key or replayed against a different configuration. The dedicated offline [`configuration_authority_mutation_evidence.py`](scripts/configuration_authority_mutation_evidence.py) gate, six report-integrity tests, and [`configuration-authority-mutation-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json) bind the exact registry, channel, test, result, and source-hash closure. Fedora and Ubuntu clean builds pass for implementation commit `c932419`; the fixed test key is synthetic, and no production trust root, product loader/profile activation, or macOS execution/support claim is made.
  - [x] **Sub-task 3.1.3.3:** `S-003-RT01` interrupts migration before and after each durable transition; assert either the prior valid configuration or complete migrated configuration is selected and rollback is repeatable. Evidence: [`migrate_path_v0`](kernel/engine/src/configuration.rs) retains the byte-exact version 0 preimage in a content-addressed private regular-file backup, prepares a canonical version 1 candidate, and publishes it by same-directory atomic rename with file and parent-directory synchronization; [`rollback_migration`](kernel/engine/src/configuration.rs) requires the exact migrated preimage, restores the exact legacy bytes, retains the backup, and treats an identical retry as successful. [`migration_interruptions_select_valid_state_and_rollback_is_repeatable`](kernel/engine/src/configuration.rs) injects interruption immediately before and after the backup, candidate, and target durable transitions for six scenarios, proves only the exact prior version 0 or complete canonical version 1 state is selected, resumes safely around an already-prepared candidate, and executes rollback twice in every scenario. The same test changes the target immediately before publication and proves the concurrent preimage is preserved with a conflict rather than overwritten. The dedicated offline [`configuration_migration_recovery_evidence.py`](scripts/configuration_migration_recovery_evidence.py) gate, five report-integrity tests, and [`configuration-migration-recovery-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json) bind the exact transition, boundary, test, command, source-hash, durability, and unsupported-claim closures. Fedora and Ubuntu clean builds pass for evidence revision `a10ad6f`; product startup migration/profile activation and macOS execution/support remain unclaimed.
  - [x] **Sub-task 3.1.3.4:** `S-003-IT01` starts every release profile from a clean environment; assert only declared capabilities register and configuration, dependency, executable, and policy hashes match the result bundle. Evidence: [`verify_profile_startup`](kernel/engine/src/configuration.rs) invokes the startup-verification boundary only for a validated version 1 configuration, an empty regular-directory root, an exact sorted catalog capability declaration, bounded hash-only build evidence, and a profile that remains unregistered. [`every_profile_startup_from_clean_environment_matches_declared_authority_and_evidence`](kernel/engine/src/configuration.rs) executes all seven catalog profiles in separate fresh roots, including three current-foundation and four future-disabled definitions, and proves every result is deterministic, self-hashed, path-free, leaves the root untouched, and binds the exact canonical configuration, [`dependency-provenance.json`](supply-chain/dependency-provenance.json), Linux [`clean-build-report.json`](artifacts/sprints/sprint-1/story-1.1/clean-build-report.json), and [`catalog.json`](configuration/profiles/catalog.json) policy identities. Because all seven authoritative entries retain `product_registration: false`, every invocation returns `blocked-as-declared`, zero capabilities register, and dirty-root, early-registration, mismatched-capability, and missing-evidence attempts fail closed. The dedicated offline [`configuration_startup_evidence.py`](scripts/configuration_startup_evidence.py) gate parses the seven minimized Rust result bundles, independently checks their catalog authority and artifact/record hashes, and binds the exact test, command, source, and unsupported-claim closure in [`configuration-startup-report.json`](artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json); six report-integrity tests reject missing execution, profile/result mutation, identity drift, dirty-environment claims, product/release overclaims, and macOS substitution. Fedora and Ubuntu clean builds pass for implementation commit `1d4ea23`; product startup activation, release readiness, and macOS execution/support remain unclaimed.
  - [x] **Sub-task 3.1.3.5 - Product security evidence:** Map `SR-GOV-008` through `SR-GOV-010`, `SR-ACC-001`, `SR-DAT-007`, `SR-SUP-003`, `SR-SUP-013`, `SR-OPS-002`, and `SR-TST-005`; retain schemas, migration matrix, profile diffs, startup traces, and rollback hashes. Evidence: deterministic [`story_3_1_security_evidence.py`](scripts/story_3_1_security_evidence.py), six focused mapping/artifact/omission/overclaim tests, and [`security-evidence-map.json`](artifacts/sprints/sprint-3/story-3.1/security-evidence-map.json) map all nine requirements exactly once and hash-bind 22 retained schema, profile, permission, dependency, clean-build, configuration, migration, startup, component, rollback-design, validator, and test artifacts. `SR-GOV-008` and `SR-SUP-003` are demonstrated within the bounded story scope; the other seven requirements retain explicit product-wide work, and all nine remain product-incomplete. The map distinguishes the six real migration interruption scenarios from the later 100-resume product threshold, configuration key exclusion from later platform key-service implementation, configuration authority rejection from the later typed `CapabilityGrant` executor, and candidate blocking from a later implemented revoked-component release gate. No private user data, raw configuration, network operation, product startup activation, release, or macOS support is claimed; macOS evidence substitution remains prohibited. Fedora and Ubuntu clean builds pass for implementation commit `a8483c6`.

##### Story Acceptance Criteria

- [x] **Story AC 3.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then all configuration inputs are versioned and fail closed, and no unknown, stale, malformed, or permission-broadening setting reaches capability registration. Evidence: the 72-case schema-failure matrix, 485-attempt signed-parent authority matrix, and seven-profile clean-startup matrix reject every invalid or broadened input before registration and record zero registered capabilities, partial startups, accepted broadenings, or raw-value disclosures.
- [x] **Story AC 3.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then migration, diff, backup, and rollback preserve semantics and redact secrets while producing reproducible before/after evidence. Evidence: deterministic version 0-to-1 fixtures and redacted hash-only diffs are joined by six before/after durable-transition interruptions, exact content-addressed private backups, old-or-complete-new state selection, concurrent-preimage protection, and repeatable exact rollback evidence.

**Story gate evidence:** Both story acceptance criteria and all shared/Linux work pass. Story 3.1 remains `BLOCKED-MACOS` under `G-DOD-10`: [`story-gate-report.json`](artifacts/sprints/sprint-3/story-3.1/story-gate-report.json) independently reviews commit `af13ebaa8832a3db36f270337e616c50b5fe180b` and tree `f4872857e2e0f654c9846afc215beaea523dea36`, verifies the byte identity of 16 implementation and evidence artifacts, and preserves macOS execution as the only blocker. No Linux result substitutes for Mac evidence, and the story checkbox remains open.

#### [ ] Story 3.2 - Vulnerability Support and Signed Manual Patch Policy

**User-facing value:** As a user or reviewer, I need a clear way to report vulnerabilities and receive authentic fixes so that a local-only product remains supportable without hidden network services or automatic update checks.

##### Tasks and Sub-tasks

- [x] **Task 3.2.1 - Implement the support contract**
  - [x] **Sub-task 3.2.1.1:** Implement supported-version, severity, ownership, response-target, embargo, disclosure, end-of-support, and maintainer-contact records consistent with `SECURITY.md`. Evidence: the closed Draft 2020-12 [`vulnerability-support-policy.schema.json`](schemas/support/vulnerability-support-policy.schema.json) and authoritative [`vulnerability-support-policy.json`](support/vulnerability-support-policy.json) define all eight required domains without claiming a supported production binary. The record retains zero supported releases; immutable signed-manifest support authority; the 90-day superseded pre-1.0 rule; four ordered severity levels; capacity-dependent acknowledgement, initial-disposition, and update targets that are explicitly not an SLA; private-by-default embargo; bounded disclosure and evidence-based closure; end-of-support/revocation states; private GitHub intake with a request-only public fallback; and zero automatic update, background network, telemetry, remote-control, or remote-kill-switch authority. Seven Python integrity/mutation tests and three AJV schema tests reject missing, unknown, broadened, unsafe-contact, deadline, disclosure, closure, network, product, release, and macOS claims. [`vulnerability-support-policy-report.json`](artifacts/sprints/sprint-3/story-3.2/vulnerability-support-policy-report.json) hash-binds the policy sources. [`support-policy-platform-report.json`](artifacts/sprints/sprint-3/story-3.2/support-policy-platform-report.json) binds implementation commit `27d8cdd5581e5645e317277c33f1370b2ca2c540` and records passing isolated Fedora 44 and Ubuntu 26.04 validation as UID/GID 10001 with a read-only root/source, all capabilities dropped, no new privileges, bounded temporary storage, and no network. No private data, product support, release, or macOS support is claimed; Mac execution remains `BLOCKED-MACOS` without evidence substitution.
  - [x] **Sub-task 3.2.1.2:** Define signed manual patch metadata with release identity, signer, checksums, provenance, prerequisites, schema impact, rollback path, revocation, and support state; keep silent update checks and remote control absent. Evidence: implementation commit `01c8ee2` adds the closed Draft 2020-12 [`signed-manual-patch-metadata.schema.json`](schemas/support/signed-manual-patch-metadata.schema.json), a clearly non-releasable synthetic [`signed-manual-patch-metadata.valid.json`](schemas/support/examples/signed-manual-patch-metadata.valid.json), deterministic semantic validation, seven Python integrity/mutation tests, four AJV schema tests, and [`manual-patch-metadata-report.json`](artifacts/sprints/sprint-3/story-3.2/manual-patch-metadata-report.json). The contract binds an increasing release sequence and exact current-release preimage; Ed25519 trust-root, authorized-signer, threshold, and detached-signature identities; ten unique package/manifest/provenance/SBOM/cryptographic-BOM/model-BOM/component/configuration/capability/authority checksums; source/builder/build-recipe provenance; platform, trust-root, storage, and standard-user prerequisites; schema/migration impact; exact supported non-revoked rollback target; local revocation metadata; support state; advisory identity; and explicit durable user-approved activation controls. Downgrade, unauthorized/insufficient signer, checksum aliasing, provenance drift, unsafe schema/rollback, revoked target, unsupported state, hidden network/download/remote authority, and product/release/macOS overclaims all fail closed. The metadata fixture does not implement or approve a production signer, verifier, activation transaction, package, release, or Mac support; cryptographic and transaction execution remain owned by Sub-task 3.2.2.1.
  - [x] **Sub-task 3.2.1.3:** Define a local emergency-disable mechanism that can block a compromised model, runtime, component, capability, or version through a user-installed signed policy update without transmitting workstation data. Evidence: implementation commit `7f9d505` adds accepted contract Decision [`0007`](docs/decisions/0007-local-emergency-disablement.md), the closed Draft 2020-12 [`emergency-disable-policy.schema.json`](schemas/support/emergency-disable-policy.schema.json), a non-activatable synthetic [`emergency-disable-policy.valid.json`](schemas/support/examples/emergency-disable-policy.valid.json), deterministic five-subject evaluation, seven Python integrity/mutation tests, four AJV schema tests, and [`emergency-disable-policy-report.json`](artifacts/sprints/sprint-3/story-3.2/emergency-disable-policy-report.json). A user-selected local bundle acquired outside the runtime binds a monotonically increasing policy sequence, validity window, payload hash, local trust root, authorized Ed25519 signer threshold, detached signatures, exact block entries, durable staging, atomic activation, retained prior valid policy, minimized decision receipts, and explicit newer-policy recovery. Model artifact, runtime, component, capability, and release-version exact matches all block before ordinary authority evaluation; declared hash mismatches and unknown subjects do not false-match or create authority. Invalid candidates preserve the active policy, an unreadable active policy requires local safe mode, and expiration, deletion, downgrade, or remote messages cannot silently unblock a subject. The schema and semantic gates reject unsafe signer, validity, sequence, network, remote, receipt, recovery, authority, product, release, and macOS mutations. No workstation data or network channel is used. Production signature verification, trust-root installation, atomic policy installation, startup integration, product activation, and Mac support remain explicitly unimplemented and are exercised by later Story 3.2 verification work.
  - [x] **Sub-task 3.2.1.4:** Define safe diagnostics and evidence-preservation guidance for vulnerability reports, including explicit exclusion of prompts, user files, credentials, keys, and unrelated paths. Evidence: implementation commit `b62bc9b` adds public [`vulnerability-reporting.md`](docs/support/vulnerability-reporting.md) guidance; authoritative [`vulnerability-report-evidence-policy.json`](support/vulnerability-report-evidence-policy.json); closed policy and [`vulnerability-diagnostic-bundle.schema.json`](schemas/support/vulnerability-diagnostic-bundle.schema.json) contracts; a clearly synthetic, sanitized, unsubmitted [`vulnerability-diagnostic-bundle.valid.json`](schemas/support/examples/vulnerability-diagnostic-bundle.valid.json); deterministic field/value scanning; seven Python integrity/mutation tests; five AJV schema tests; and [`vulnerability-report-evidence-report.json`](artifacts/sprints/sprint-3/story-3.2/vulnerability-report-evidence-report.json). The contract permits 14 bounded identity, synthetic-reproduction, assessment, and minimized-reference fields while structurally excluding 23 prompt, conversation, model-response, user-file, workspace/source, credential, password/token/cookie/key/recovery-code, environment, browser, shell-history, raw-configuration/database/memory/log, absolute/home/traversal/unrelated-path, and unrelated-personal-data categories. Five seeded secret/key/path/traversal value classes and all prohibited field names are detected recursively. The original remains under device-owner control; only an explicitly selected, scanned, manually reviewed, post-sanitization-hashed derivative may be transferred manually outside AgentMage. Access is limited to the maintainer and assigned fix owner, closure review is required, retention is capped at 180 days after closure absent separately authorized bounded hold, and a hold creates no collection authority. No private user data or AgentMage network channel is used. Product export, redaction, encryption, transfer, retention scheduling, submitted-report, release, and Mac-support claims remain absent.

- [x] **Task 3.2.2 - Verify and close the story**
  - [x] **Sub-task 3.2.2.1:** Test valid, wrong-signer, downgrade, corrupt, mismatched, interrupted, revoked, unsupported-version, migration-failure, and rollback fixtures against the manual patch contract. Evidence: implementation commit `fa6a415` adds a 10-case synthetic corpus under [`fixtures/support/manual-patch`](fixtures/support/manual-patch), deterministic [`manual_patch_verifier.py`](scripts/manual_patch_verifier.py), seven focused integrity/adversarial tests, and [`manual-patch-verification-report.json`](artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json). The verifier performs real detached Ed25519 verification with the exact hashed OpenSSL executable, validates the hashes of the package and ten required metadata artifacts, binds release sequence/current preimage/signer/trust-root/platform/support/revocation/schema/migration identities, and models durable activation and exact rollback without retaining a private key. The valid case alone can proceed; wrong signer, downgrade, corrupt artifact, platform mismatch, revoked release, unsupported metadata version, and migration failure are blocked; interruption preserves the exact prior package; and failed post-activation verification restores that exact prior package. All ten outcomes pass with zero prior-state failures and zero network authority. The corpus and report are synthetic, no product verifier, activation, production signer, package release, private user data, or Mac support is claimed, and macOS remains `BLOCKED-MACOS` without evidence substitution.
  - [x] **Sub-task 3.2.2.2:** Exercise the reporting workflow from private intake through triage, bounded evidence, remediation decision, signed patch metadata, notification, and closure using synthetic content. Evidence: implementation commit `2e77417` adds the closed seven-state [`workflow.valid.json`](fixtures/support/vulnerability-workflow/workflow.valid.json), deterministic [`vulnerability_support_workflow.py`](scripts/vulnerability_support_workflow.py), seven focused lifecycle/cryptographic/mutation tests, and [`vulnerability-workflow-report.json`](artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json). The exercise traverses the support policy's exact received, acknowledged, triaged, remediation-decided, fix-verified, notification-published, and closed states with exact role ownership, monotonic synthetic timestamps, policy-derived moderate-severity response targets, five hash-bound evidence references, and an append-only seven-receipt SHA-256 chain. It independently validates the three-record sanitized diagnostic bundle with zero findings and re-executes the valid manual-patch case through real detached Ed25519 verification before the synthetic fix-verification stage. Notification channels and all seven required advisory content classes are exercised as synthetic records only; closure requires bounded evidence, no unresolved uncertainty, explicit retention review, the 180-day limit, and no incident hold. Mutation tests reject state/order, role, evidence, policy hash, external publication, network, private-data, production patch/release, Mac, report-overclaim, and receipt-chain drift. No actual private report, reporter contact, external publication, product patch, release, network action, private user data, or Mac execution is claimed; macOS remains `BLOCKED-MACOS` without evidence substitution.
  - [x] **Sub-task 3.2.2.3 - Product security evidence:** Map `SR-GOV-004`, `SR-GOV-010`, `SR-SUP-005`/`SR-SUP-010`/`SR-SUP-011`/`SR-SUP-013`, `SR-OPS-004` through `SR-OPS-007`; retain support records, patch schemas, signature and rollback tests, revocation tests, and tabletop notes. Sprint 25 performs the first complete package-level `RV-22` execution. Evidence: implementation commit `62e5ed3` adds the bounded [`story-3.2-vulnerability-response-tabletop.md`](docs/security/story-3.2-vulnerability-response-tabletop.md), deterministic [`story_3_2_security_evidence.py`](scripts/story_3_2_security_evidence.py), seven focused mapping/closure/overclaim tests, and [`security-evidence-map.json`](artifacts/sprints/sprint-3/story-3.2/security-evidence-map.json). The map covers all ten named requirements exactly once and hash-binds 37 policy, guidance, decision, design, schema, fixture, signed-patch, revocation, workflow, platform, validator, and test artifacts. `SR-SUP-010` is demonstrated within the bounded Story 3.2 scope; the other nine requirements retain explicit product-wide work, and all ten remain product-incomplete. The tabletop preserves exact role and workflow decisions while stating that no actual report, incident, external notification, patch, release, network action, private data, or Mac execution occurred. It explicitly leaves the four-scenario `RV-21`, package-level `RV-22`, product audit/export/clock handling, incident-runbook, evidence-store enforcement, production provenance, release CI, build/startup revocation, and every-platform package run to their owning later stories. Product, release, network, `RV-21`, `RV-22`, and Mac overclaims fail closed; macOS remains `BLOCKED-MACOS` without evidence substitution.

##### Story Acceptance Criteria

- [x] **Story AC 3.2.AC1:** Given a vulnerability report containing only synthetic data, when the support workflow runs, then ownership, severity, bounded evidence, decisions, notifications, and closure are traceable without any automatic network contact by AgentMage. Evidence: the seven-state workflow produces seven hash-chained receipts with exact maintainer/fix-owner roles, policy-derived moderate severity and response targets, three sanitized evidence records with zero findings, remediation and notification decisions, bounded closure and retention review, zero private-data records, zero external actions, and zero AgentMage network calls.
- [x] **Story AC 3.2.AC2:** Given valid and adversarial manual patch metadata, when verification runs, then only an authorized non-downgrade patch for the exact supported release can proceed and every failure preserves the prior safe state. Evidence: all ten required cases reach their exact outcomes; one authorized valid case can proceed, while wrong-signer, downgrade, corruption, platform mismatch, revocation, unsupported metadata, and migration failure block, interruption preserves the exact prior package, and postcheck failure restores it. Real detached Ed25519 verification and all required artifact hashes are exercised with zero prior-state failures.
- [x] **Story AC 3.2.AC3:** Given a revoked component or end-of-support release, when AgentMage starts, then the applicable local capability or version is visibly blocked or constrained according to signed policy and no remote kill switch is required. Evidence: at the declared contract scope, the emergency-disable policy includes `product-startup-admission` before ordinary authority evaluation and emits minimized visible blocked receipts for exact model, runtime, component, capability, and release-version subjects, including revoked and unsupported reasons. No remote kill switch or runtime network authority exists. Actual product-startup hook integration and activation remain explicitly unclaimed and belong to their later implementation stories.

**Story gate evidence:** All three story acceptance criteria and all shared/Linux work pass. Story 3.2 remains `BLOCKED-MACOS` under `G-DOD-10`: [`story-gate-report.json`](artifacts/sprints/sprint-3/story-3.2/story-gate-report.json) independently reviews commit `254408669fbb9538eae9d41f7900653b41c47221` and tree `2482a1e5e47c419adbe41bbc69ce5a5b1a82095d`, verifies the byte identity of 28 implementation and evidence artifacts with zero findings, and preserves macOS execution as the only blocker. The gate keeps product startup integration, actual incident, support, patch, acceptance, release, `RV-21`, and `RV-22` claims absent. No Linux result substitutes for Mac evidence, and the story checkbox remains open.

#### Sprint Acceptance Criteria

- [x] **Sprint AC 3.AC1:** `AT-CFG-001` passes. Evidence: 485 permission-bearing mutations exceed the required 100-case threshold, all 485 are rejected through five untrusted channels, accepted broadening is zero, and 72 schema failures produce stable exact diagnostics.
- [x] **Sprint AC 3.AC2:** Unknown, malformed, stale, or permission-broadening settings stop before startup. Evidence: all 72 schema-failure cases and 485 cross-channel authority mutations reject before registration with zero partial startups and zero accepted broadenings.
- [x] **Sprint AC 3.AC3:** A profile cannot enable a capability excluded from its release. Evidence: all seven profiles, including four future-disabled profiles, start from clean synthetic roots with zero registered capabilities and zero early product registrations.
- [x] **Sprint AC 3.AC4:** Configuration migration preserves semantics and fails safely on unsupported versions. Evidence: six injected interruptions span all three durable transitions, every selected state is the exact prior or complete migrated form, unsupported fixtures reject, concurrent preimage change is preserved, and exact rollback is repeatable.
- [x] **Sprint AC 3.AC5:** Logs and diagnostics contain configuration identity but no secrets. Evidence: session/release result contracts retain only `profile_id` and SHA-256 configuration identity with canonical record hashing, while raw configuration and private paths remain absent; the bounded diagnostic fixture has zero scan findings and zero private-data records.

**Sprint gate evidence:** All five sprint acceptance criteria and all shared/Linux work pass. Sprint 3 remains `BLOCKED-MACOS` under `G-DOD-10`: [`sprint-gate-report.json`](artifacts/sprints/sprint-3/sprint-gate-report.json) independently reviews commit `aac2a33e549c18cf6784e0552a2ebfaa13f2c66e` and tree `fbe3ff4f87eaa0c0bea6a6479c2cc010b8d8d929`, verifies 14 aggregate story-gate and acceptance artifacts with zero findings, and confirms both Stories 3.1 and 3.2 are blocked only by the same missing Mac execution evidence. No Linux result substitutes for Mac evidence; product startup, product acceptance, and release claims remain absent, and the sprint checkbox remains open.

**Gate decision:** Sprint 3 is PASS only when Stories 3.1 and 3.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 1 - v0.1 - Read-Only Local Evidence Assistant

### [ ] Sprint 4 - Kernel Contracts and Typed Boundaries

**Current Story 4.1 evidence correction:** The historical evidence narratives
in this sprint retain the identities and counts that were true at their cited
commits. The current reviewed package supersedes those counts: package source
revision `d399e40` contains 24 closed, non-executable members and three pinned
direct dependencies, has SHA-256
`9809db151076b3bd1376286aa03fe9a9f80aa9281d8dc4e2baf71ac92d511458`, and
passes offline locked tests, strict Clippy, archive-path mutation checks, and
exact source-closure verification. Reference revision `76076d8` documents all
120 public exports and all 15 `VersionedContract` types. Fixture revision
`09f344a` independently reproduces 15 canonical success fixtures, including
`ApprovalRequest` and `CapabilityGrant`, plus the seven persisted failure
fixtures and generated oversized case; its manifest SHA-256 is
`9716abdff573a33a15b25b6c8a5edcf9271861828033f854e7bc0e58b177f1ec`.
`ApprovalRequest` remains non-authoritative, and parsing or constructing a
`CapabilityGrant` does not validate policy or authorize execution; the
source-contract package itself has no executor. Story review commit `f9d3e29`
revalidated this expanded closure without promoting product, release, or macOS
claims.

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-004`.

**Sprint goal:** Implement the interface-independent kernel and the typed contracts shared by every later capability.

**Source coverage:** `AM-KRN-001`; inventory Sections 2, 3, 4, and Product Architecture.

**Dependencies:** Sprint 3; legacy dependency record: `G-FOUNDATION`.

#### [ ] Story 4.1 - Kernel Contracts and Typed Boundaries

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need kernel contracts and typed boundaries so that AgentMage delivers the following bounded outcome: Implement the interface-independent kernel and the typed contracts shared by every later capability.

##### Tasks and Sub-tasks

- [x] **Task 4.1.1 - Implement the bounded story**
  - [x] **Sub-task 4.1.1.1** (legacy `S-004-I01`): Implement typed task, work-packet, validation-issue, plan, action, tool-definition, tool-call, tool-result, evidence-reference, receipt, and error contracts. Evidence: implementation commit `ac7ff45` adds the dependency-free, interface-independent [`agentmage-kernel-contracts`](kernel/contracts/src/lib.rs) family across distinct identity, common payload/validation/error, task/work-packet/plan/action, tool definition/call/result, and evidence/receipt modules. All public records carry explicit schema versions where they cross a boundary; separate identifier newtypes prevent accidental identity interchange; calls bind exact tool versions and correlation identities; evidence references are content-addressed; and tasks, packets, plans, actions, and tool definitions remain descriptive objects with no embedded execution grant. Seven focused unit tests and one public-API integration test construct and link every requested contract family, while the complete workspace format, lint, build, and test gate passes. Strict wire parsing, deterministic serialization, field bounds, canonical receipt hashing, durable storage, and dispatch authority are not claimed here and remain assigned to Sub-tasks 4.1.1.2 through 4.1.1.7 and Sprint 5.
  - [x] **Sub-task 4.1.1.2** (legacy `S-004-I02`): Implement deterministic serializers, parsers, schema versions, and validation error paths. Evidence: implementation commit `c812af5` adds the closed [`serialization.rs`](kernel/contracts/src/serialization.rs) boundary using the already pinned workspace Serde and `serde_json` dependencies. Ten top-level contract types implement one public `VersionedContract` API; every struct rejects unknown and duplicate fields, every enum has an explicit snake-case wire value, and distinct identifier newtypes serialize transparently without losing type separation in Rust. Serialization rejects unsupported versions and emits stable compact bytes in declaration and sequence order with no map-valued contract fields; parsing admits exactly one complete value and rejects missing, unknown, duplicate, malformed, trailing, oversized, unsupported-enum, and unsupported-version input. The shared boundary enforces a 1 MiB encoded limit before parsing, returns boxed typed `ContractError` values through `ContractResult<T>`, supplies an exact `schema_version` error path for version failures, and uses stable redacted codes and messages without retaining or echoing candidate content. Three focused serializer/parser tests plus the expanded public-API integration test verify exact bytes, repeated-byte stability, all ten top-level round trips, rejection classes, size, version, and linked identities; the complete workspace format, lint, build, and test gate passes. Published schema fixtures, compatibility versions, semantic field bounds, and exhaustive mutation coverage remain assigned to Tasks 4.1.2 and 4.1.3.
  - [x] **Sub-task 4.1.1.3** (legacy `S-004-I03`): Implement work-packet field validation, transition validation, revision history, plan adaptation, and completion-evidence validation. Evidence: implementation commit `0ebda0a` completes the bounded [`WorkPacket`](kernel/contracts/src/task.rs) contract with reason, owner, authoritative and completion evidence, non-authoritative mutable/protected file declarations, expected output, acceptance checks, required evidence and capability class, structured budgets and stop conditions, rollback expectations, sensitivity, verification/review dates, lifecycle conditionals, disposition, and supersession identity. The interface-independent [`work_packet.rs`](kernel/engine/src/work_packet.rs) validator applies deterministic identifier, text, list, date, secret-like-value, evidence, budget, stop-condition, overlap, state-conditional, and completion checks with bounded redacted `ValidationIssue` outputs. Its closed eight-state lifecycle exercises all 64 transition pairs; same-state revisions are allowed only for non-terminal work, terminal states cannot reopen, and cancellation or supersession may retain an existing plan. `WorkPacketHistory` admits exact sequential revisions without overwriting earlier typed records or changing packet/task identity; plan adaptation binds a newer packet revision, same task and plan identity, deterministically rebuilds dependencies, and preserves a prior step state only when its description and evidence contract are unchanged. Completion requires exactly one non-empty evidence entry for every acceptance check and presence of every required evidence class. Six focused engine tests verify valid fixtures for all states, deterministic/redacted malformed-field findings, the complete transition matrix, revision preservation and rejection, plan repeatability/adaptation/staleness, and completion failure paths; the expanded contract round trip and complete workspace format, lint, build, and test gate pass. File declarations remain non-authoritative until canonical paths and exact grants are implemented, budget/stop enforcement remains Sub-task 4.1.1.5, and durable encrypted history persistence remains assigned to its storage sprint.
  - [x] **Sub-task 4.1.1.4** (legacy `S-004-I04`): Implement the tool registry and dispatcher interfaces without production tools. Evidence: implementation commit `e296d49` expands the closed tool contracts with schema-bound payloads, review risk, a non-authoritative required-grant template, timeout, elapsed duration, and typed state-change disposition, then adds the kernel-owned [`tooling.rs`](kernel/engine/src/tooling.rs) registry and pre-grant dispatcher. The `Tool` interface exposes only an immutable declarative definition and has no execution method or per-call capability callback. `ToolRegistry` freezes exact identity/version definitions, lists them deterministically, rejects invalid or duplicate registrations, and validates calls against exact schema identity and hash, media type, a 1 MiB argument limit, lowercase payload digest, one complete recursively parsed JSON value, and duplicate-key rejection. Definitions require bounded identities/text/effects, valid schema hashes and versions, three-component numeric tool versions, a non-zero timeout no greater than 300 seconds, a single-use grant template, and an operation exactly equal to the tool identity. `ToolDispatcher` has no executor callback: malformed or unregistered calls return typed failed results, while every valid call returns a typed `tool.dispatch.grant_required` denial with zero elapsed time, no output or evidence, and `not_changed` state. Five focused registry/definition/call/closed-JSON/dispatcher tests and the expanded contract round trip pass with the complete workspace format, lint, build, and test gate. No production tool, schema-specific argument semantics, capability grant, receipt persistence, sandbox execution, or state change is claimed; exact grant validation and atomic consumption remain Sprint 5.
  - [x] **Sub-task 4.1.1.5** (legacy `S-004-I05`): Implement bounded run budgets and explicit stop conditions. Evidence: implementation commit `e2254bb` expands the descriptive budget contract across plan steps, tool calls and depth, model calls, input/output bytes, elapsed milliseconds, memory, disk, and process count, and adds the stateful [`run_control.rs`](kernel/engine/src/run_control.rs) kernel controller. Every valid packet must explicitly retain acceptance, user-decision, policy-denial, error, cancellation, budget-exhaustion, and uncertain-result stops; optional deadline stops remain declaration-gated. `RunController` starts only from a fully valid active packet, admits usage only against an exact declared resource ceiling, treats the limit as inclusive, preserves admitted usage when an attempt would exceed the limit, fails closed on undeclared resources and counter overflow, and makes the first terminal reason sticky and idempotent. Budget exhaustion can arise only from accounting, and acceptance satisfaction can arise only from a newer same-identity completed packet whose acceptance checks and required evidence validate; direct signals for either are prohibited. Five focused tests cover exact boundaries, rejected-attempt immutability, undeclared resources, overflow, first-reason precedence, undeclared and prohibited signals, invalid start contracts, identity/revision drift, and missing or valid completion evidence. The complete workspace format, lint, build, and test gate passes with 43 engine tests. The controller performs deterministic kernel accounting only; operating-system measurements, watchdog enforcement, sandbox termination, durable usage persistence, and a production tool/model loop remain assigned to their platform, storage, and integration stories.
  - [x] **Sub-task 4.1.1.6** (legacy `S-004-I06`): Define cancellation and error propagation across kernel, adapters, tools, models, and shells. Evidence: implementation commit `1a5c87d` adds closed versioned [`CancellationSignal`](kernel/contracts/src/boundary.rs) and [`BoundaryFailure`](kernel/contracts/src/boundary.rs) contracts with distinct cancellation identity, exact task and correlation identity, declared shell/kernel/platform-adapter/tool/model boundaries, stable cancellation reasons, denied/cancelled/timed-out/failed/uncertain outcomes, the original typed error, and an ordered propagation route. The deterministic [`propagation.rs`](kernel/engine/src/propagation.rs) engine permits only declared downward cancellation edges and their exact upward failure counterparts, preserves the first signal observed at each token, propagates parent cancellation to current and later uncancelled descendants, keeps child cancellation descendant-only, validates a fresh signal's claimed origin, and atomically returns the signal actually installed under simultaneous callers. Failure envelopes reject unsupported versions, origin/route mutation, repeated or invalid boundary hops, outcome/error-category disagreement, cancellation on non-cancelled outcomes, and task/correlation drift while preserving the original error and cancellation context through every admitted observer. Seven focused propagation tests cover parent, late-child, child-only, idempotent, simultaneous, malformed, all-outcome, tool/platform/kernel/shell, and model/kernel/shell paths; the public contract test round-trips both new envelopes; 11 contract unit tests, one integration test, 50 engine tests, strict Clippy, all workspace Rust/TypeScript builds, and the VS Code shell test pass through `npm run product:check`. This increment defines an in-process token and typed-envelope protocol only: production shell/model/tool/platform integrations, operating-system process termination, watchdog behavior, durable cancellation recovery, and effect reconciliation remain assigned to their owning stories and are not claimed here.
  - [x] **Sub-task 4.1.1.7** (legacy `S-004-I07`): Enforce that descriptions, plans, prompts, and tool definitions carry no authority. Evidence: implementation commit `df7de07` adds a closed versioned [`Prompt`](kernel/contracts/src/prompt.rs) contract with exact prompt, task, and correlation identities, explicit message provenance roles, bounded whole-contract parsing, and no grant, permission, workspace, model, tool, network, or execution-authority field. The kernel-owned [`authority.rs`](kernel/engine/src/authority.rs) module seals free-form descriptions, tasks, work packets, plans, actions, prompts, tool definitions, and required-grant templates as `NonAuthoritativeArtifact` types that external crates cannot extend; offering any of them as authority has no success path and deterministically returns one redacted typed policy denial without inspecting or echoing content. Closed deserialization rejects an injected `capability_grant` field on plans, prompts, and tool definitions, while an otherwise valid registered definition containing authority-looking description and grant-template claims still reaches only the non-executing dispatcher's exact `tool.dispatch.grant_required` denial. Two focused authority tests, one prompt test, the expanded tool-dispatch test, and the public contract integration test cover sealed type membership, hostile content, redaction, deterministic round trips, unknown authority fields, and dispatch invariance; 12 contract unit tests, one integration test, 53 engine tests, strict Clippy, all workspace Rust/TypeScript builds, and the VS Code shell test pass through `npm run product:check`. This increment does not define, mint, validate, consume, persist, or delegate a real `CapabilityGrant`; the sole positive authority path remains absent until Sprint 5 and every current tool dispatch remains denied.

- [x] **Task 4.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 4.1.2.1:** Versioned kernel contract package. Evidence: producer/verifier commit `8d212b4` makes the contract crate self-contained with byte-identical Apache-2.0 license text and the pinned Rust toolchain, then adds the standard-library-only [`kernel_contract_package.py`](scripts/kernel_contract_package.py) builder/checker and three focused archive-path/manifest mutation tests. Contract correction commit `03e675d` requires all 14 optional wire keys to be explicitly present while still permitting `null`; the integration regression removes each key in turn and asserts `contract.field.missing`. Refresh commit `5527668` publishes the corrected 20,729-byte [`agentmage-kernel-contracts-0.0.0.crate`](artifacts/sprints/sprint-4/story-4.1/agentmage-kernel-contracts-0.0.0.crate) with SHA-256 `84e35c278d7480e0c6e4ee5d9c35fa194a2db75b2b381ae4217c243f8c13d4fd` and its deterministic [`kernel-contract-package-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-contract-package-report.json). The checker admits exactly 17 non-executable members, caps compressed/member/expanded sizes, rejects absolute/traversal/duplicate/symlink/broadened archives, binds every packaged source byte to immutable Git revision `03e675d`, verifies repository and crate license/toolchain equality, checks unpublished Cargo `0.0.0` metadata separately from wire schema version `1`, permits only exact pinned `serde` and `serde_json` dependencies, forbids build scripts and unsafe Rust, and unpacks into an ephemeral directory for offline locked tests plus strict Clippy. The checker is enrolled in `requirements:check`; its report contains only relative paths, hashes, fixed identities, statuses, and limitations. This is a reviewable unpublished source-contract package, not a product release; it contains no positive authority path, Linux package verification is local, and macOS package/build/execution evidence remains `blocked-macos` with no macOS implementation claim.
  - [x] **Sub-task 4.1.2.2:** Contract reference documentation. Evidence: reference commit `5d9c644` adds the independent-client [`kernel-contract-reference.md`](docs/architecture/kernel-contract-reference.md) guide covering the authority boundary, exact JSON wire rules, all 11 redacted parser codes, common/task/work-packet/plan/action/prompt/tool/evidence/receipt/cancellation families, 13 top-level `VersionedContract` types, compatibility policy, implementation checklist, 65-symbol public index, two boundary diagrams, and explicit limitations. The reference pass identified that default Serde behavior admitted omitted `Option<T>` keys; corrective contract commit `03e675d`, refreshed package commit `5527668`, and the complete 14-key omission regression now make the documented closed-field rule true before this artifact was accepted. The standard-library-only [`kernel_contract_reference.py`](scripts/kernel_contract_reference.py) checker parses the frozen package API instead of following later source implicitly, requires all 65 exports, 13 versioned types, 11 error codes, and nine guide sections, rejects named overclaims, and rebuilds warning-free Rustdoc offline from corrected package SHA-256 `84e35c278d7480e0c6e4ee5d9c35fa194a2db75b2b381ae4217c243f8c13d4fd`. Three focused parser/coverage/overclaim tests, Markdown lint, 22 Mermaid validations, policy/link validation across 51 Markdown files, package verification, and the deterministic [`kernel-contract-reference-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-contract-reference-report.json) pass. Follow-up commit `6169622` links the reference directly to the published golden corpus, and evidence refresh commit `b5e79f9` binds the guide/checker/tests to that immutable revision. The checker is enrolled in `requirements:check`; no positive grant/tool execution path is claimed, and all macOS reference/build/execution evidence remains `blocked-macos`.
  - [x] **Sub-task 4.1.2.3:** Golden serialization and compatibility fixtures. Evidence: generator commits `12adb4e` and `46af6d8` add a deterministic exact-env fixture emission path to the complete public contract-family integration test, a standalone Rust client verifier compiled against the frozen package, and the standard-library-only [`kernel_contract_fixtures.py`](scripts/kernel_contract_fixtures.py) builder/checker with three focused mutation and closure tests. Publication commit `4b1f2a7` records 13 canonical valid examples, one for every public `VersionedContract`, in the hash-bound [`version 1 fixture manifest`](fixtures/contracts/v1/manifest.json); seven persisted malformed, missing, unknown, duplicate, trailing, older-version, and newer-version inputs plus their exact redacted error codes in the [`compatibility record`](fixtures/contracts/compatibility.json); and one non-persisted 1,048,577-byte oversized input generated in memory with expected code `contract.size.exceeded`. The manifest SHA-256 is `4bd8db1a10d2a034899dd2733a0b8461d82f2f01ac5d1019257d8906bf595396` and it binds every fixture to frozen package SHA-256 `84e35c278d7480e0c6e4ee5d9c35fa194a2db75b2b381ae4217c243f8c13d4fd` and generator revision `46af6d894fefe8b2c2198a686f6f560f2090871d`. The checker rejects output drift, extra or missing generated files, package/hash/source drift, canonical-byte changes, unexpected parser codes, and any verifier failure; it runs the Rust client offline against the extracted frozen crate and is enrolled in `requirements:check`. The deterministic [`kernel-contract-fixture-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json) records passing Linux round trips and compatibility rejection with no network or execution authority. No schema migration, positive authority path, or macOS execution evidence is claimed; macOS remains `blocked-macos`.
  - [x] **Sub-task 4.1.2.4:** Architecture dependency report. Evidence: implementation commit `703eb13` adds the human-readable [`kernel-dependency-report.md`](docs/architecture/kernel-dependency-report.md), the standard-library-only [`kernel_architecture_report.py`](scripts/kernel_architecture_report.py) manifest analyzer, three focused graph/report mutation tests, and a build-contract regression. The work identified and corrected a stale build-contract expectation that denied the already-approved pinned `serde` and `serde_json` contract dependencies. Evidence commit `bfa5930` publishes the deterministic [`kernel-architecture-dependency-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-architecture-dependency-report.json), SHA-256 `f5a846b9d3592e03d6508ec401bf06fa61fffe3427ec45729a23e4d7ad760180`, bound to immutable source revision `703eb13c680127324a1b910b8bd202e66de1c45b`. The checker derives edges from every Cargo product manifest, the VS Code shell manifest, the Swift manifest, both package lock families, the accepted module inventory, and dependency policy; it distinguishes the ten-edge logical allowlist from seven materialized Linux product edges, proves both graphs acyclic with zero prohibited observed edges, records six direct external Cargo uses and 31 locked external packages, and confirms the VS Code shell has zero runtime npm dependencies and Swift has zero external packages. The three unmaterialized edges are explicit: two remain `blocked-macos`, while the VS Code-to-contract edge remains `protocol-not-yet-generated`. The report is enrolled in `requirements:check`; it verifies compile direction only and does not claim runtime process/data-flow enforcement, external-package security review, a positive authority path, or macOS implementation/execution.

- [x] **Task 4.1.3 - Verify and close the story**
  - [x] **Sub-task 4.1.3.1:** `S-004-UT01` round-trips every contract version and canonical serializer; assert byte stability plus rejection of missing, extra, malformed, oversized, duplicate, and unsupported fields. Evidence: the only supported wire version is schema version `1`. Frozen package tests from contract commits `c812af5` and `03e675d` exercise deterministic canonical serialization, repeated-byte equality, all 13 public `VersionedContract` round trips, exact size boundaries, trailing input, all 14 nullable-but-required key omissions, unknown and duplicate fields, malformed/eof syntax, unsupported enum values, and older/newer schema rejection through the public parser. Generator/verifier commits `12adb4e` and `46af6d8` independently compile a client against frozen package SHA-256 `84e35c278d7480e0c6e4ee5d9c35fa194a2db75b2b381ae4217c243f8c13d4fd`; corpus commit `4b1f2a7` retains 13 hash-bound canonical success fixtures and seven persisted failure fixtures, while producing the 1,048,577-byte oversized case in memory. `npm run kernel-contract-package:check`, `npm run kernel-contract-fixtures:check`, and `npm run product:check` all pass on Linux; the machine-readable [`kernel-contract-fixture-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json) records canonical-byte stability, public-parser round trips, exact compatibility rejection, and no network or positive execution authority. No migration or macOS execution result is inferred from this Linux verification; macOS remains `blocked-macos`.
  - [x] **Sub-task 4.1.3.2:** `S-004-UT02` exercises every legal and illegal work-packet transition, revision, stop condition, budget boundary, and completion claim; assert deterministic state and typed reason. Evidence: verification commit `ed72c9f` expands the kernel-engine matrices without changing production behavior. The work-packet suite evaluates all 64 ordered pairs across eight lifecycle states against the closed 21-legal/43-illegal table, constructs one valid packet in every state, preserves prior revisions on failure, and asserts successful append plus every typed revision failure: invalid initial revision, invalid candidate, changed packet identity, changed task identity, repeated/skipped revision, exhausted counter, and illegal transition. Completion cases prove exact valid claims and reject wrong-state, missing, duplicate, unknown, empty, and missing-required-class evidence with stable codes. The run-control suite iterates all ten `BudgetResource` variants through zero, below-limit, exact-limit, exceeded, undeclared, and counter-overflow paths, asserting rejected attempts never mutate admitted usage and the first terminal reason remains sticky. All eight `StopConditionKind` variants now have an exact path: six declared signals, evidence-bound acceptance completion, and accounting-bound budget exhaustion, with typed rejection of direct acceptance/budget signals and an undeclared deadline. Repeated execution is deterministic; `npm run product:check` passes with 56 kernel-engine tests plus the complete workspace and VS Code shell gates. This verifies in-process deterministic accounting and state only; operating-system resource measurement and macOS execution remain assigned and `blocked-macos`.
  - [x] **Sub-task 4.1.3.3:** `S-004-ST01` attempts dispatch from shell, model, tool, capability pack, forged description, and unregistered caller; assert zero execution without kernel validation and an exact denial receipt. Evidence: implementation/verifier commit `437e10d` adds the closed `ProposalOrigin`, `PreGrantDispatchDisposition`, and `PreGrantDispatchReceipt` kernel types, while explicitly defining origin as untrusted denial provenance that cannot authenticate a caller, grant authority, or select a privileged path. `ToolDispatcher::dispatch` now requires an origin classification, performs exact registry/argument validation, and returns only terminal receipts; it still has no executor callback or success branch. A Rust security test covers shell, model, tool, capability-pack, unregistered-caller, forged-authority-description, and unregistered-tool attempts, preserves exact call/correlation identity, and emits a redacted trace only under the exact opt-in test environment. The standard-library-only [`kernel_dispatch_security.py`](scripts/kernel_dispatch_security.py) checker reruns that test, requires the complete ordered seven-case closure, and rejects outcome/disposition/code drift, state change, elapsed execution time, output, evidence, or a positive path. Evidence commit `153a376` publishes the deterministic [`kernel-dispatch-security-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json), SHA-256 `0870f7595b6182b2ef2c1e94c01c7e9ade114c0ae3146227467988bde3d6fd79`, bound to source revision `437e10d792b9dc01712b873f464580a8c1fc32b4`; `npm run kernel-dispatch-security:check` and `npm run product:check` pass with 57 engine tests. The report is enrolled in `requirements:check`; these are in-process pre-grant receipts, not durable ledger records, and no macOS execution evidence is claimed.
  - [x] **Sub-task 4.1.3.4:** `S-004-IT01` propagates success, denial, cancellation, timeout, and typed failure across fake shell/kernel/model/tool boundaries; assert correlation identity and no lost error context. Evidence: integration/verifier commit `bdf6e39` adds an external [`boundary_workflow.rs`](kernel/engine/tests/boundary_workflow.rs) client test that imports only public contract and propagation APIs. It correctly keeps success on the normal `ToolResult::Succeeded` path rather than misrepresenting it as `BoundaryFailure`, wire-round-trips the result at each fake observer with stable canonical bytes, and carries tool-origin success through platform-adapter/kernel/shell observers with unchanged correlation, output, and no error. Denial and cancellation traverse tool/platform-adapter/kernel/shell; timeout and typed dependency failure traverse model/kernel/shell. At every hop the test round-trips the public schema and asserts unchanged task/correlation IDs, error ID/code/category/field path/causal ID, and cancellation context. The standard-library-only [`kernel_boundary_integration.py`](scripts/kernel_boundary_integration.py) checker reruns the opt-in Rust trace, requires the exact ordered five-outcome closure and routes, and rejects identity, route, outcome, error, cause, or cancellation drift. Evidence commit `59ea919` publishes the deterministic [`kernel-boundary-integration-report.json`](artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json), SHA-256 `8f6af0392e598e32feeeefca2d13da1a0d7f966d581f1277f7fccd36d5a2882d`, bound to source revision `bdf6e394fdf4b6da9a576228ea2bf3c5453da506`; its checker and the full product gate pass. The report is enrolled in `requirements:check`; observers are fake in-process boundaries, and no production adapter, execution, durable transport, or macOS evidence is claimed.
  - [x] **Sub-task 4.1.3.5 - Product security evidence:** Map `SR-GOV-006`, `SR-ACC-001`, `SR-AI-003`, `SR-OPS-001`, `SR-TST-001`, `SR-TST-002`, and `SR-TST-004`; retain schema corpus, compatibility report, architecture graph, and dispatcher-denial traces. Evidence: implementation commit `dd59610` adds the fail-closed [`story_4_1_security_evidence.py`](scripts/story_4_1_security_evidence.py) generator/checker and seven focused exact-map, omission, reordering, hash, path, missing-evidence, and overclaim tests; it also enrolls the checker in `requirements:check`. Evidence commit `ec5265b` publishes the deterministic [`security-evidence-map.json`](artifacts/sprints/sprint-4/story-4.1/security-evidence-map.json), SHA-256 `0076279c1ea99c794d1a8fede0389ad086682666d54e1dc73f8b0dbaf6a43021`, bound to immutable source revision `dd59610`. The map revalidates and hash-binds the frozen contract package, public reference, 13 canonical success fixtures, seven persisted compatibility failures, generated oversized case, architecture graph, seven zero-execution dispatcher traces, and five typed boundary outcome traces. It maps exactly seven controls while marking every product requirement `not-complete`: `SR-AI-003` and `SR-TST-004` are demonstrated only within the Story 4.1 boundary; the other five are partial contributions. The report explicitly makes no positive authority, durable audit, release, product-completion, or fuzzing claim, calls the hostile parser corpus fixed rather than fuzzed, uses no network or private user data, and leaves macOS `blocked-macos` without substituted evidence. All seven focused tests and `npm run product:check` pass; the full `requirements:check` remains blocked before reaching this checker because the protected locked-resolution report is stale, and that earlier reviewed artifact was not mutated.

##### Story Acceptance Criteria

- [x] **Story AC 4.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every public boundary is versioned, deterministic, bounded, and authority-free except the separately defined grant contract. Evidence: the frozen package exposes 13 top-level `VersionedContract` types through one closed parser and canonical serializer, rejects unknown, duplicate, missing, trailing, malformed, oversized, older-version, and newer-version inputs with stable redacted codes, and retains no map-valued wire fields. The 13 canonical fixtures round-trip byte-for-byte through the published API. Tasks, work packets, plans, prompts, tool definitions, descriptions, models, shells, tools, and capability packs are sealed as non-authoritative; all seven origin classes reach only a typed zero-execution dispatcher receipt. A positive `CapabilityGrant` contract is intentionally absent until Sprint 5, so no current public contract can authorize execution.
- [x] **Story AC 4.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a client implementing only the published contracts can reproduce every success and failure fixture without importing internal modules. Evidence: [`fixture_verifier.rs`](fixtures/contracts/fixture_verifier.rs) is compiled offline as a standalone client against only the extracted frozen package SHA-256 `84e35c278d7480e0c6e4ee5d9c35fa194a2db75b2b381ae4217c243f8c13d4fd`. It imports the published contract types and `VersionedContract` API, reproduces all 13 canonical success fixtures exactly, reproduces all seven persisted compatibility failures by exact redacted code, and independently generates and rejects the 1,048,577-byte oversized case. The fixture checker rejects any corpus, manifest, source, package, hash, path-closure, canonical-byte, or error-code drift.

**Story gate evidence:** Both story acceptance criteria and all shared/Linux work pass. Story 4.1 remains `BLOCKED-MACOS` under `G-DOD-10`: implementation commit `3390065` adds the independent [`story_4_1_gate.py`](scripts/story_4_1_gate.py) evaluator and eight focused review-identity, artifact-closure, task-omission, acceptance, architecture, security, universal-control, and overclaim tests; evidence commit `585ea10` publishes [`story-gate-report.json`](artifacts/sprints/sprint-4/story-4.1/story-gate-report.json), SHA-256 `3294ce1ae060e4ae7b995b8784d112f70f4efd367ef4cfb51b766b85d8333a27`. The gate independently reviews commit `26272273c9e4709504cf99da8f2771a2384ea4eb` and tree `9d10ec531c1adb8e5a8dcf73609d8be9940e59a4`, verifies the byte identity of 18 contract, fixture, architecture, authority, dispatch, boundary, and security artifacts with zero findings, passes `AT-ARCH-001` only for the implemented contract-layer scope, and keeps product-wide acceptance, positive authority, release, and macOS-support claims absent. No Linux result substitutes for Mac evidence, and the story checkbox remains open.

#### Sprint Acceptance Criteria

- [x] **Sprint AC 4.AC1:** `AT-ARCH-001` passes for the implemented contract layer. Evidence: the architecture report derives ten logical and seven materialized product edges from every Cargo, npm, and Swift manifest/lock family, proves both graphs acyclic, finds zero prohibited observed edges or reverse/lower-layer authority dependencies, and records all three unmaterialized edges explicitly. The independent story gate joins that static result with five runtime typed-boundary traces and limits the PASS to the implemented contract layer; it makes no product-wide `AT-ARCH-001` claim.
- [x] **Sprint AC 4.AC2:** Every contract rejects missing, extra, malformed, oversized, and unsupported-version fields predictably. Evidence: the frozen parser and independent fixture client reproduce missing, unknown/extra, duplicate, malformed/eof, trailing, older-version, newer-version, and generated 1,048,577-byte oversized failures by stable redacted code. The exhaustive nullable-key regression removes all 14 nullable-but-required keys one at a time and receives `contract.field.missing` in every case.
- [x] **Sprint AC 4.AC3:** Round-trip serialization is deterministic. Evidence: all 13 public `VersionedContract` families serialize to their manifest-hashed canonical schema-version-1 bytes, parse through the public API, and serialize byte-identically on repeated runs; the fixture checker and standalone frozen-package client reject any canonical-byte drift.
- [x] **Sprint AC 4.AC4:** Cancellation and errors remain typed across every stub boundary. Evidence: five external-client integration traces preserve exact task/correlation identities through success, denial, cancellation, timeout, and typed dependency failure. Success remains a normal `ToolResult`; all non-success outcomes remain `BoundaryFailure` records with unchanged error ID, code, category, field path, causal ID, route, and cancellation context at every fake observer.
- [x] **Sprint AC 4.AC5:** No shell, model, or capability pack can execute a tool without the kernel dispatcher. Evidence: all seven shell, model, tool, capability-pack, unregistered-caller, forged-description, and unregistered-tool attempts terminate in exact zero-execution pre-grant receipts. The dispatcher exposes no executor callback or success branch, records no output/evidence/state change/elapsed execution, and the gate rejects any positive-dispatch-path claim.

**Gate decision:** Sprint 4 is PASS only when Story 4.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

**Current gate evidence:** All five sprint criteria and the shared/Linux kernel-contract foundation pass. Implementation commit `59cae1e` adds the independent [`sprint_4_gate.py`](scripts/sprint_4_gate.py) aggregate evaluator and seven focused currentness, criterion, story-blocker, review-closure, checklist-state, mutation, and overclaim tests; evidence commit `2c3c4ed` publishes [`sprint-gate-report.json`](artifacts/sprints/sprint-4/sprint-gate-report.json), SHA-256 `b0169c59acff9c96761bf02b2c45d748b84d11d18203cd1e36cf829ce64a5aae`. The gate independently reviews commit `af282594b2c48710f3e7126814cc5963b5b074c1` and tree `4b316221b9df718b5826f7a469ff3f64724368d8`, aggregates the Story 4.1 gate, verifies all five criteria from raw architecture, fixture, dispatcher, and boundary evidence, records zero findings, and keeps product-wide architecture acceptance, positive authority, release, and external-human-review claims absent. Sprint 4 remains `BLOCKED-MACOS`; `G-DOD-10` is the sole blocking control, Story 4.1 is the sole blocking story, Linux evidence substitution is prohibited, and both Story and Sprint checkboxes remain open.
### [ ] Sprint 5 - Capability Grants and Policy Engine

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-005`.

**Sprint goal:** Make `CapabilityGrant` the only authority-bearing object and prove it is exact, expiring, single-use, and non-broadenable.

**Source coverage:** `AM-AUT-001`; PRD Section 10; inventory Section 5; `AT-AUTH-001`; Section 35A rejected defaults.

**Dependencies:** Sprint 4; legacy dependency record: Sprint 3 (legacy S-003), Sprint 4 (legacy S-004).

#### [ ] Story 5.1 - Capability Grants and Policy Engine

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need capability grants and policy engine so that AgentMage delivers the following bounded outcome: Make `CapabilityGrant` the only authority-bearing object and prove it is exact, expiring, single-use, and non-broadenable.

##### Tasks and Sub-tasks

- [x] **Task 5.1.1 - Implement the bounded story**
  - [x] **Sub-task 5.1.1.1** (legacy `S-005-I01`): Implement the complete versioned grant schema, parent relationship, preview digest, nonce, use limit, preimages, side effects, rollback description, expiry, and status. Evidence: implementation commit `48a2f20` adds the versioned [`CapabilityGrant`](kernel/contracts/src/grant.rs) authority contract and distinct `ActorId`, `GrantId`, `GrantNonce`, and `WorkspaceId` identities; corrective scope commit `02bc026` explicitly distinguishes session-read parents from operation children and binds exclusions and sensitivity rather than hiding them inside a digest. Its 29 required top-level wire keys bind immutable revision and class; exact actor/session/task; required nullable action identity/kind and tool identity/version; one of 15 closed operation classes with no wildcard/custom/approve-all variant; ordered componentized included and excluded workspace-relative target candidates; data sensitivity; exact argument digest; indexed target preimages; indexed operation-typed side effects and canonical-details digests; rollback description; integer issuance/expiration instants; nonce; use limit/count; required nullable parent identity/revision digest; confirmed-preview digest; policy digest; and one of six lifecycle states. Nested targets, preimages, and effects reject unknown fields; all nullable keys remain required on the wire. Two focused grant tests remove every top-level key independently, reject unknown nested absolute-path authority and the unsupported `all` operation, and assert the exact authority shape. The expanded public-family integration test round-trips the grant deterministically, requires all six nullable relationship keys, and keeps its identities linked to the task/action/tool fixture; contract unit tests, the integration test, strict Clippy, and `npm run product:check` pass. Parsing or constructing this public record does not make it valid: session/derived-grant rules, canonical path validation, kernel-only issuance, policy authentication, scope/intersection checks, lifecycle transitions, durable storage, and atomic consumption remain assigned to later sub-tasks and sprints.
  - [x] **Sub-task 5.1.1.2** (legacy `S-005-I02`): Implement session read grants and single-use derived operation grants. Evidence: implementation commit `cbab08d` adds the kernel-engine [`GrantIssuer`](kernel/engine/src/grants.rs), explicit bounded `SessionReadGrantRequest` and `DerivedOperationGrantRequest` inputs, nine closed redacted issue/derivation error classes, canonical grant/revision hashing, exact current-state lookup, nonce retention, and immutable revision-hash history. Session parents can issue only as `session_read`/`workspace_read`: they bind one exact actor/session/task/workspace, non-empty included scope, nested exclusions, sensitivity, preview and policy digests, a positive lifetime no longer than 24 hours, a unique nonce, and one to 4,096 allowed child derivations; action and tool fields are absent and no state change is declared. Derived children inherit actor/session/task/exclusions/sensitivity, must remain within an included parent target and outside every exclusion, must use the exact parent policy, cannot outlive the parent, bind the exact pre-derivation parent revision hash, and are forced to `operation`, revision 1, `issued`, `use_limit = 1`, and `use_count = 0`. Each successful derivation atomically advances the parent revision/count and closes the parent as consumed at its exact limit; all fallible serialization/hash checks complete before state mutation. Duplicate grant IDs, reused nonces, malformed IDs/digests/tool versions, invalid lifetimes, wildcard/traversal/separator path components, changed workspaces/policy, invalid indexes/effects/preimages, excluded or broadened scope, wrong/missing/expired/terminal parents, and exhausted derivations fail without partial insertion or parent mutation. Two focused tests cover exact parent/child fields, inherited scope, retained revision hashes, two-child exhaustion, third-child denial, duplicate ID, nonce replay, invalid lifetime, wildcard scope, excluded target, changed workspace, changed policy, and unchanged state after every denial; strict Clippy and the complete product gate pass with 59 engine tests. Issuer creation is explicit and has no `Default` path; only records retained by the exact kernel-owned issuer instance may later be considered, while arbitrary public grant records remain untrusted candidates. This increment does not implement policy admission, caller authentication, tool dispatch/execution, operation-grant consumption, durable storage, process confinement, or macOS behavior.
  - [x] **Sub-task 5.1.1.3** (legacy `S-005-I03`): Implement the policy engine with deny precedence across actor, task, action, tool, path, argument, preimage, network, credential, and publication scopes. Evidence: implementation commit `139d242` adds the immutable [`PolicyEngine`](kernel/engine/src/policy.rs), a versioned deterministic `PolicyDocument`, exact tool/version bindings, closed allow/deny sets, a kernel-computed canonical policy SHA-256, fixed-order redacted decisions, and explicit external network, credential, and publication scopes. Evaluation admits only an unexpired, issued, one-use operation grant that exactly matches the current record retained by the supplied `GrantIssuer` and the engine's computed policy identity; it then compares actor, session, task, action identity/kind, tool identity/version, operation, ordered workspace targets, canonical-argument digest, indexed preimages, expected side effects, confirmed-preview digest, and required external scope. Every rule is deny-by-default, every allow/deny collision resolves to deny, unexpected external scopes fail closed, and policy construction rejects unsupported revisions, wildcard/NUL/empty scope values, path separators/traversal components, and malformed denied digests. Three focused tests prove deterministic hashing, exact success, issuer-record forgery rejection, fixed first-failure classification after each context mutation, deny precedence for every named policy dimension, exact external-scope requirements, and malformed/broad-document rejection; strict Clippy and the complete `npm run product:check` gate pass with 62 engine tests. This increment evaluates but does not consume a grant, call a tool, authenticate a process boundary, persist policy or grant state, or make any execution decision atomic; those authority boundaries remain open in Sub-task 5.1.1.4 and later work.
  - [x] **Sub-task 5.1.1.4** (legacy `S-005-I04`): Implement atomic validation and consumption immediately before execution. Evidence: implementation commit `6acabb3` adds `GrantIssuer::consume_for_execution`, three closed redacted `GrantConsumeError` outcomes, and a deliberately non-authoritative `GrantConsumptionRecord`. Under one exclusive issuer borrow, the transaction looks up the exact kernel-retained grant, recomputes and verifies its current canonical revision hash, invokes the deny-first `PolicyEngine` against current observations, checks revision/use-count arithmetic, constructs and hashes the terminal revision, and only then mutates issuer state. Success advances revision 1 to 2, increments the one-use operation grant exactly from zero to one, sets status to `consumed`, retains both canonical revision hashes, and returns evidence containing identities/hashes/time/policy but no target, operation, tool, or argument scope that could become alternate authority. Replay sees the retained terminal grant and fails policy before another transition. Three focused tests prove exact one-time consumption, no use-count consumption after changed arguments, changed policy, expiry, or retained-hash corruption, and exactly one successful terminal transition when two threads race through the same mutex-protected issuer; strict Clippy and the complete `npm run product:check` gate pass with 65 engine tests. Sub-task 5.1.1.6 subsequently strengthens stale issued-grant handling by retaining explicit `invalidated` or `expired` terminal revisions rather than leaving the denied grant issued. This is an atomic in-memory kernel transaction and the required final call primitive for the future dispatcher. Encrypted crash-durable grant transactions remain assigned to the canonical operational store, and invocation immediately before an actual isolated worker attempt remains assigned to Sprint 16; neither is claimed by this increment.
  - [x] **Sub-task 5.1.1.5** (legacy `S-005-I05`): Implement approval-request rendering as a non-authority display object. Evidence: implementation commit `449bed0` adds the versioned shared [`ApprovalRequest`](kernel/contracts/src/approval.rs) display contract and deterministic kernel [`render_approval_request`](kernel/engine/src/approval.rs)/`verify_approval_request` boundary. The request exposes the proposed and parent grant identities, observed parent-revision digest, actor/session/task/action class, closed operation, exact registered `ToolCall` including canonical argument bytes and digest, componentized included targets and inherited exclusions, sensitivity, current preimages, expected side effects, rollback/recovery description, issuance/expiration, policy identity, and confirmation digest. Rendering validates exact registry identity/schema/arguments, bounded identifiers and lifetime, componentized same-workspace scope, preimage/effect indexes and digests, rollback text, and closed operation/effect agreement, then computes SHA-256 over canonical request bytes with the confirmation field set to 64 zeroes. Verification repeats the shape/tool checks and rejects any digest mismatch. The contract deliberately has no nonce, use limit/count, lifecycle status, grant class, signature, issue/consume method, or conversion to `CapabilityGrant`; it is sealed as `NonAuthoritativeArtifact`, and `reject_as_authority` always returns the same redacted policy denial. Contract, public-family, deterministic rendering, field-mutation, malformed-call, wildcard-scope, confirmation-tampering, and authority-rejection tests pass; strict Clippy and the complete `npm run product:check` gate pass with 15 contract and 68 engine tests. A valid display proves only internal consistency: authenticated user-decision capture, grant issuance, shell presentation/accessibility, persistence, and worker execution remain assigned to later tasks.
  - [x] **Sub-task 5.1.1.6** (legacy `S-005-I06`): Implement mutation, replay, race, stale-preimage, changed-policy, uncertain-result, and child-scope rejection. Evidence: implementation commit `936e8c4` strengthens the issuer lifecycle so a policy denial against an otherwise-current unused operation grant atomically retains a new terminal revision: elapsed grants become `expired`, while changed arguments, preimages, preview/context, policy, or other exact scope become `invalidated`; use count remains zero and the old/new canonical revision hashes remain available. A consumed attempt may advance exactly once to `uncertain` only when the caller supplies the exact retained consumed-revision digest; the transition retains revision 3 with use count one and never restores authority. Missing, malformed, stale-digest, wrong-state, repeat-uncertain, and replay attempts fail closed through four redacted `GrantLifecycleError` classes. Dedicated child derivation tests reject sibling targets, excluded descendants, cross-scope aggregation, changed workspace, changed policy, and nonce replay without parent mutation, while accepting a correctly narrowed child. Focused tests separately prove argument mutation, stale preimage, changed policy, exact-expiry behavior, retained-hash corruption, consumed replay, two-consumer race, wrong uncertain digest, one exact uncertain transition, repeat uncertainty denial, and terminal non-reusability; strict Clippy and the complete `npm run product:check` gate pass with 70 engine tests. These transitions remain in-memory until the encrypted operational store owns durable atomicity; caller/process authentication, actual worker results, terminal receipts, and platform confinement remain later boundaries.
  - [x] **Sub-task 5.1.1.7** (legacy `S-005-I07`): Encode deny-by-default rules for write, delete, command, network, commit, push, publish, send, upload, deploy, database write, and credential access. Evidence: implementation commit `88614b4` adds the immutable `PolicyEngine::strict_local_read_only` constructor, exact [`StrictLocalReadOnlyScope`](kernel/engine/src/policy.rs), and closed `STRICT_LOCAL_DENIED_OPERATIONS` set. The constructor admits only explicitly named actor/task/action/tool/version/target identities and exact `workspace_read`; it explicitly denies `workspace_write`, `workspace_delete`, `command_execute`, `network_access`, `git_commit`, `git_push`, `publish`, `send`, `upload`, `deploy`, `database_write`, and `credential_access`, leaves network/credential/publication allow sets empty, and denies every additional operation by absence. It returns an immutable engine with a kernel-computed policy digest and has no default or mutable post-construction profile path. A focused test asserts the exact twelve-member denied set and empty external allow scopes, proves exact workspace read succeeds, then issues otherwise-valid grants for all twelve prohibited classes plus database read and model inference and proves each fails at the operation-policy boundary; strict Clippy and the complete `npm run product:check` gate pass with 71 engine tests. The general explicit-policy constructor remains for isolated tests and separately gated future profiles; startup selection and configuration-to-policy binding remain later integration work, so this increment does not enable any prohibited capability.

- [x] **Task 5.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 5.1.2.1:** Grant schema and policy decision table. Evidence: source commit `1424f1f` adds the review-oriented [`grant-policy-reference.md`](docs/architecture/grant-policy-reference.md), standard-library-only [`grant_policy_reference.py`](scripts/grant_policy_reference.py) builder/checker, and three focused checker tests; evidence commit `4e89354` publishes the deterministic [`grant-policy-reference-report.json`](artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json) bound to the full source revision `1424f1ffc9c6c9631b2ea79eccec68cca13d4054`. The reference inventories all 29 required top-level grant fields, nested target/preimage/side-effect shapes, parent/child invariants, all 15 fixed policy scopes and stable denial codes in evaluation order, all 15 closed operations, the one explicitly allowed strict-local operation, 12 explicit hazardous-operation denials, and two operations denied by absence. Its tables document exact allow conditions, deny precedence, lifecycle effects during consumption, atomic-consumption behavior, and the non-authoritative status of approval and evidence records. Focused checker tests, source/report freshness validation, Markdown lint, 23 Mermaid validations, policy/link validation across 53 Markdown files, `git diff --check`, and the complete `npm run product:check` gate pass. The report records shared contracts and the Linux reference as locally verified while retaining `blocked-macos` with no macOS claim; in-memory state, durable storage, authenticated IPC, canonical platform paths, isolated worker execution, and startup policy binding remain explicitly open.
  - [x] **Sub-task 5.1.2.2:** Grant-state transition diagram. Evidence: source commit `9c127fd` adds the shared/Linux [`grant-state-transitions.md`](docs/architecture/grant-state-transitions.md) reference with separate Mermaid state machines for session-read parents and derived operation grants, an exact transition table, explicit non-transitions, invariants, and review checklist; evidence commit `3764f8d` publishes the deterministic [`grant-state-report.json`](artifacts/sprints/sprint-5/story-5.1/grant-state-report.json) bound to full source revision `9c127fd5f34c26df2bcfc4fa19ddcc9768bd56a8`. The standard-library-only [`grant_state_transitions.py`](scripts/grant_state_transitions.py) checker compares the document with the six-value Rust `GrantStatus` enum and the issuer's constrained transition call sites, covering three parent transitions and five operation transitions. Three focused tests prove enum-order, transition-marker, reserved-state, and boundary-disclosure detection. The artifact distinguishes state-preserving denials from revision-producing transitions, records `consumed` as terminal for authority while permitting one digest-bound evidence transition to `uncertain`, and discloses that `revoked` has no current producer and parent expiry during derivation does not rewrite the parent. Focused checker tests, report freshness validation, Markdown lint, 25 Mermaid validations, policy/link validation across 54 Markdown files, and the complete `npm run product:check` gate pass. In-memory state, crash durability, an explicit revocation API, isolated worker execution, final effect receipts, and all macOS implementation/evidence remain open and are not claimed.
  - [x] **Sub-task 5.1.2.3:** Approval preview and denial receipt fixtures. Evidence: typed fixture commit `fd8f4da` adds the deterministic [`grant_review_fixtures.rs`](kernel/engine/tests/grant_review_fixtures.rs) integration generator, standard-library-only [`grant_review_fixtures.py`](scripts/grant_review_fixtures.py) builder/checker, three mutation/closure tests, and a three-object [`grant-review-v1`](fixtures/grants/v1/manifest.json) fixture set. The generator registers one synthetic read tool, constructs the immutable strict-local policy, issues a bounded parent, renders a real 20-field [`ApprovalRequest`](fixtures/grants/v1/approval-preview.workspace-read.json), derives the exact operation grant from its confirmation digest, changes only the current argument digest, observes the actual `policy.deny.argument` decision, and invokes atomic consumption. The denial advances the unused operation to `invalidated` revision 2 with use count zero; a later corrected replay fails at the terminal grant boundary. The generated [`policy-denial.argument-mismatch.json`](fixtures/grants/v1/policy-denial.argument-mismatch.json) records that exact redacted decision/lifecycle result, and the self-hashed [`denial-receipt.argument-mismatch.json`](fixtures/grants/v1/denial-receipt.argument-mismatch.json) uses the shared `Receipt` contract, content-addresses the decision as evidence, preserves correlation/action/tool-call identities, and carries a non-retryable typed policy error. Evidence commit `c47cb1f` publishes the deterministic [`grant-review-fixture-report.json`](artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json) bound to full source revision `fd8f4da2225bcbaec39a76418b6f519310be4fae`. The approval contains zero nonce/use/status/class authority fields; both display and receipt remain non-authoritative. Typed regeneration, exact-byte retention, confirmation/receipt digest verification, fixture mutation rejection, strict Clippy, Markdown/policy validation across 55 documents, and the complete `npm run product:check` gate pass. Authenticated decision capture, a production receipt builder, durable/keyed receipt chaining, isolated execution, effect reconciliation, and macOS implementation/evidence remain explicitly open.
  - [x] **Sub-task 5.1.2.4:** Adversarial grant corpus. Evidence: typed corpus commit `e88bd1e` adds the deterministic [`adversarial_grant_corpus.rs`](kernel/engine/tests/adversarial_grant_corpus.rs) generator/executor, standard-library-only [`adversarial_grant_corpus.py`](scripts/adversarial_grant_corpus.py) builder/checker, three closure/mutation tests, and the exact-byte reproducible [`agentmage-adversarial-grants-v1`](fixtures/grants/adversarial/v1/corpus.json) corpus and [`manifest`](fixtures/grants/adversarial/v1/manifest.json). The corpus contains 560 unique recorded seeds: 40 independent cases for each required actor, session, task, action, tool, path, argument, preimage, side-effect, expiry, nonce, use-count, parent, and preview-digest mutation class. Its 440 current-context or clock mutations invoke final atomic consumption, assert the exact first policy-denial scope, retain zero admitted attempts and use count zero, advance an otherwise-current grant to revision 2 `invalidated` or `expired`, and prove a corrected replay remains denied. Its 120 nonce/use-count/parent mutations construct forged public grant candidates, prove issuer-current identity denies them at `Grant`, and leave the real revision 1 issued record unchanged; these candidates cannot enter the identity-only consumption API. Evidence commit `b164df3` publishes the deterministic [`adversarial-grant-corpus-report.json`](artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json) bound to full source revision `e88bd1e7cdbc001156de5cbbc3c1f5fe05bc0025`. Typed regeneration/execution, exact-byte and manifest checks, duplicate/missing/wrong-scope/wrong-status/admitted-attempt mutation rejection, strict Clippy, Markdown/policy validation across 56 documents, and the complete `npm run product:check` gate pass. The corpus exercises in-memory issuer/policy behavior without a worker; canonical platform path resolution, durable transactions, concrete execution counting, and all macOS implementation/evidence remain open.

- [x] **Task 5.1.3 - Verify and close the story**
  - [x] **Sub-task 5.1.3.1:** `S-005-UT01` mutates actor, session, task, action, tool, path, argument, preimage, side effect, expiry, nonce, use count, parent, and preview digest independently across at least 500 seeded cases; assert zero execution. Evidence: the typed, immutable-source-bound `agentmage-adversarial-grants-v1` implementation and report from Sub-task 5.1.2.4 execute 560 unique deterministic seeds, exactly 40 per named mutation dimension. The Rust integration generator re-creates a fresh exact parent/operation/policy/context for every seed, mutates only the named dimension, and fails the test immediately if final policy or atomic consumption admits the case. All 440 current-observation/clock cases return their exact first denial scope, retain zero admitted attempts and use count zero, terminalize to `invalidated` or `expired`, and reject an exact corrected replay; all 120 forged nonce/use-count/parent candidates fail current-issuer identity and leave real authority unchanged. The product has no executor callback behind this boundary, so zero admitted attempts is also zero possible executions in the tested build. The corpus and report record `admitted_attempt_count: 0`; exact-byte regeneration, three independent Python closure/mutation tests, direct Rust corpus execution, strict Clippy, and the complete product gate pass. This verifies in-memory kernel admission, not a future isolated-worker execution counter, durable transaction, canonical platform path adapter, or macOS behavior.
  - [x] **Sub-task 5.1.3.2:** `S-005-UT02` races two consumers against one valid grant and replays success, denial, timeout, crash, and uncertain results; assert exactly one atomic consumption and no second effect. Evidence: verifier commit `c27e2d7` adds the typed [`grant_race_replay.rs`](kernel/engine/tests/grant_race_replay.rs) two-thread barrier harness, standard-library-only [`grant_race_replay.py`](scripts/grant_race_replay.py) report checker, and three scenario-closure/mutation tests. Each of five fresh exact-grant scenarios races two mutex-serialized final consumers: success, timeout, crash, and uncertain produce exactly one atomic consumption and one race-loser denial at `Grant`; the changed-argument denial scenario correctly produces zero consumption/worker/effect, with the first consumer terminalizing the unused grant and the second denied at `Grant`. Deterministic worker-start/effect probes then model the named result, after which an exact corrected replay is attempted. Success remains revision 2 `consumed`; denial remains revision 2 `invalidated`; unreconciled timeout, crash, and uncertain results each use the exact consumed digest to become revision 3 `uncertain`. Every replay is denied, worker starts never exceed one, and effect count never increases after replay. Evidence commit `2dfbbbb` publishes the deterministic [`grant-race-replay-report.json`](artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json) with all five traces, bound to full source revision `c27e2d7e742a73dc5205f187b6076a334a6abfd8`. Typed races, missing-result/nonterminal-uncertainty/second-consumption/second-worker/second-effect mutation rejection, strict Clippy, documentation policy validation, and the complete product gate pass. Worker/effect counters are test probes rather than a production executor; durable cross-process atomicity, concrete effect reconciliation, and macOS behavior remain open.
  - [x] **Sub-task 5.1.3.3:** `S-005-ST01` asks models, prompts, shells, tools, plugins, approval displays, and simulated children to mint, widen, transfer, or combine authority; assert denial with actor-attributed receipts. Evidence: implementation/verifier commit `c21d8be` extends the sealed [`authority.rs`](kernel/engine/src/authority.rs) boundary with closed `AuthorityProposalSource` and `AuthorityEscalationKind` enums plus a versioned `DescriptiveAuthorityReceipt`. `reject_authority_attempt` accepts only an existing sealed `NonAuthoritativeArtifact`, records typed actor/session/task/source/escalation/artifact attribution, retains the same redacted policy error, sets `authority_admitted = false`, never inspects candidate content, and has no success variant, grant fields, or conversion to `CapabilityGrant`. The typed [`authority_escalation_matrix.rs`](kernel/engine/tests/authority_escalation_matrix.rs) verifier executes the complete 7-by-4 matrix: model, prompt, shell, tool, plugin, approval display, and simulated child each attempt mint, widen, transfer, and combine for 28 total cases. Prompts, descriptions, tool definitions, and approval displays contain distinct hostile authority claims; every receipt round-trips as a closed typed value while retaining none of those claims, nonce/use/status/grant material, or a positive outcome. The standard-library-only [`authority_escalation_matrix.py`](scripts/authority_escalation_matrix.py) checker and three closure/mutation tests enforce exact source/artifact/escalation pairing, one actor/session/task attribution, constant denial/error shape, zero admitted authority, and candidate-content redaction. Evidence commit `356bfdb` publishes the deterministic [`authority-escalation-report.json`](artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json) bound to full source revision `c21d8be2a7429f605ab26f39a19365f91df85469`. Strict Clippy, all typed matrix tests, documentation validation, and the complete product gate pass. Attribution is typed evidence rather than authenticated IPC; receipts remain in-memory without durable keyed integrity, and plugin/child runtimes plus macOS implementation remain open.
  - [x] **Sub-task 5.1.3.4:** `S-005-IT01` changes policy, file preimage, task, and preview between approval and dispatch; assert stale authority is rejected immediately before execution. Evidence: integration/verifier commit `1bebc26` adds the typed [`grant_stale_dispatch.rs`](kernel/engine/tests/grant_stale_dispatch.rs) workflow, standard-library-only [`grant_stale_dispatch.py`](scripts/grant_stale_dispatch.py) report checker, and three closure/mutation tests. Every fresh case registers the synthetic read tool, builds the immutable strict-local policy, issues a bounded parent, renders a real `ApprovalRequest`, derives an operation whose `preview_sha256` exactly equals the rendered confirmation, and then changes only policy revision, current preimage digest, task identity, or current preview digest. The changed value is supplied to `GrantIssuer::consume_for_execution`, the final in-memory boundary before any future worker. The four cases return `policy.deny.grant`, `policy.deny.preimage`, `policy.deny.task`, and `policy.deny.preview` respectively; each atomically retains revision 2 `invalidated` with use count zero, starts zero workers/effects, and denies a subsequent exact corrected replay at `Grant`. Evidence commit `4f9c23a` publishes the deterministic [`grant-stale-dispatch-report.json`](artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json) with all four traces, bound to full source revision `1bebc26f2947f44c03709d6418e0a02f66e80480`. Typed integration execution, missing-case/wrong-scope/consumption/worker/effect/replay-success mutation rejection, strict Clippy, documentation validation, and the complete product gate pass. Final dispatch remains an in-memory transaction without a production worker; authenticated/durable approval and canonical platform preimage observation are later boundaries, and macOS evidence remains open.
  - [x] **Sub-task 5.1.3.5 - Product security evidence:** Map `SR-ACC-001` through `SR-ACC-003`, `SR-ACC-007`, `SR-AI-004`, `SR-AI-005`, `SR-OPS-001`, and `SR-TST-012`; retain mutation seeds/results, race traces, state transitions, and independent boundary review. Evidence: independent-review implementation commits `8795a44`, `21e72fe`, and `3af3c6a` add the standard-library-only [`grant_boundary_review.py`](scripts/grant_boundary_review.py), four focused omission/result/platform/overclaim tests, and package commands; evidence commit `96d4d36` refreshes the deterministic [`grant-boundary-review.json`](artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json), SHA-256 `04447366fbf2967f8f75c7521824e554e7a86e3c0a1b18a256912c7f3665cca5`, against full source revision `3af3c6ab791ed035e7a9abcd94dfacbdd08e6317`. The independent automated review reexecutes and hash-binds all seven source checkers/reports, reconciles 560 mutation seeds, five race/replay scenarios, eight documented transitions, 28 escalation attempts, and four post-approval mutations, records zero admitted mutations/escalations/replays and no findings within Story 5 scope, and preserves eight open product boundaries. It explicitly records that external human review was not performed. Security-map source commits `3af3c6a` and `3772d67` add and narrow [`story_5_1_security_evidence.py`](scripts/story_5_1_security_evidence.py), six focused mapping/closure/mutation/overclaim tests, and requirements-command enrollment; evidence commit `345e2b5` publishes the current [`security-evidence-map.json`](artifacts/sprints/sprint-5/story-5.1/security-evidence-map.json), SHA-256 `dc353fd72d81b3452ddda671a668415881d6242a87104a371f9ea051a8c7c2d0`, bound to full source revision `3772d6766efaa25c8e6ad11c513441fe2a37fc35`. The map retains 31 hashed artifacts and all eight named controls: only `SR-ACC-001` is demonstrated within this story's shared/Linux in-memory boundary; the other seven are partial story evidence, and every product requirement remains `not-complete`. It makes no durable-audit, fuzzing, release, external-human-review, product-completion, or macOS claim. Ten focused review/map tests, both currentness checkers, strict Clippy, and the complete product gate pass; authenticated decisions/IPC, crash-durable transactions, canonical platform paths, isolated workers, durable keyed receipts, effect reconciliation, explicit revocation, release enforcement, prompt-injection coverage, and macOS implementation/evidence remain assigned to later gates.

##### Story Acceptance Criteria

- [x] **Story AC 5.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no operation executes without one current exact grant, and every terminal path consumes or invalidates it according to the documented state machine. Evidence: the aggregate Story gate independently reconciles 560 grant mutations with zero admissions, five race/replay scenarios with zero successful replays, four post-approval mutations with zero worker starts, and all eight implemented parent/operation transitions. Both criteria pass their bounded shared/Linux in-memory scope; no production-executor or durable-transaction claim is made.
- [x] **Story AC 5.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then wildcards, approve-all modes, implicit inheritance, grant aggregation, and model-created authority are structurally unrepresentable or fail closed. Evidence: the aggregate Story gate verifies the closed 15-operation schema, 12 explicit strict-local denials plus two denials by absence, zero authority fields in approval displays, and zero admitted authority across all 28 source-by-escalation cases.

#### Sprint Acceptance Criteria

- [x] **Sprint AC 5.AC1:** `AT-AUTH-001` passes with zero unauthorized executions. Evidence: the Sprint gate reconciles all 560 seeded grant mutations and 28 authority-escalation attempts with zero admissions in the shared/Linux in-memory boundary. No production-executor claim is made.
- [x] **Sprint AC 5.AC2:** Every accepted grant is consumed exactly once. Evidence: success, timeout, crash, and uncertain scenarios each admit one of two racing consumers exactly once; worker/effect probes never exceed one, and every replay remains denied. Durable cross-process atomicity remains open.
- [x] **Sprint AC 5.AC3:** A changed preview, argument, target, preimage, policy, task, scope, expiry, or nonce invalidates authority. Evidence: 14 mutation classes across 560 deterministic seeds admit zero attempts, and all four post-approval policy/preimage/task/preview mutations reach final consumption with zero worker starts.
- [x] **Sprint AC 5.AC4:** Models, tools, shells, plugins, and simulated child agents cannot mint or broaden a grant. Evidence: all seven descriptive source classes attempt mint, widen, transfer, and combine across 28 typed cases; every result is a redacted actor/session/task-attributed denial with zero admitted authority.
- [x] **Sprint AC 5.AC5:** Wildcard and approve-everything configurations remain impossible. Evidence: the grant contract has 15 closed operation variants with no wildcard/custom/approve-all member, strict-local policy explicitly denies 12 hazardous operations and denies two more by absence, and approval displays contain zero authority fields.

**Gate decision:** Sprint 5 is PASS only when Story 5.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

**Current story-gate evidence:** Source commit `dd7c2d0` adds the independent [`story_5_1_gate.py`](scripts/story_5_1_gate.py) aggregate evaluator, package/requirements enrollment, and seven focused acceptance, review, checklist, Definition-of-Done, mutation, and overclaim tests; evidence commit `62b358c` publishes [`story-gate-report.json`](artifacts/sprints/sprint-5/story-5.1/story-gate-report.json), SHA-256 `d0e54d5754a639f39238a0ef2ba65c37a0d7731407dec1a85490b6ed58869c5a`. The gate independently reviews commit `345e2b561d5ed2263004e232d03ee880fcc09ba2` and tree `5894f9f707cedee1c9c7d93f9c0884813319b47c`, retains 24 exact artifact hashes, records zero findings, and passes both Story criteria and every non-platform Definition-of-Done control in the declared shared/Linux scope. Story 5.1 remains `BLOCKED-MACOS`; `G-DOD-10` is the sole blocker, Linux evidence substitution is prohibited, external human review is not claimed, and the Story checkbox remains open.

**Current sprint-gate evidence:** Source commit `8bc070f` adds the independent [`sprint_5_gate.py`](scripts/sprint_5_gate.py) aggregate evaluator, package/requirements enrollment, and seven focused acceptance, story/blocker, checklist, review, Definition-of-Done, mutation, and overclaim tests; evidence commit `76b54a1` publishes [`sprint-gate-report.json`](artifacts/sprints/sprint-5/sprint-gate-report.json), SHA-256 `3bda13960c43acff01a14cb02b637563cfc950e01176e4dd9b8690212582b05d`. The gate independently reviews commit `89e68a38001b0991f964c47a04fc5a33156d9f89` and tree `a99b5cda5f9b195f0afdbbb6ceefe978ec1b3d70`, retains 13 exact artifact hashes, records zero findings, and passes all five Sprint criteria in their bounded shared/Linux scopes. Sprint 5 remains `BLOCKED-MACOS`; `G-DOD-10` and Story 5.1 are the sole blocking control and story, Linux evidence substitution is prohibited, external human review is not claimed, and both the Story and Sprint checkboxes remain open.
### [ ] Sprint 6 - Canonical Workspace Paths

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-006`.

**Sprint goal:** Implement cross-platform workspace-relative path authority with descriptor-relative resolution and display-only absolute links.

**Source coverage:** `AM-PTH-001`; PRD Section 11; inventory Sections 5 and 6; `AT-PATH-001`.

**Dependencies:** Sprint 5; legacy dependency record: Sprint 4 (legacy S-004), Sprint 5 (legacy S-005).

#### [ ] Story 6.1 - Canonical Workspace Paths

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need canonical workspace paths so that AgentMage delivers the following bounded outcome: Implement cross-platform workspace-relative path authority with descriptor-relative resolution and display-only absolute links.

##### Tasks and Sub-tasks

- [ ] **Task 6.1.1 - Implement the bounded story**
  - [x] **Sub-task 6.1.1.1** (legacy `S-006-I01`): Implement `WorkspacePath` with workspace identity and normalized relative components only. Evidence: implementation commit `52d43a9` adds the shared [`WorkspacePath`](kernel/contracts/src/path.rs) contract with private fields, exact bounded `WorkspaceId`, ordered validated [`WorkspacePathComponent`](kernel/contracts/src/path.rs) values, read-only accessors, and custom deserialization that reuses the sole constructor and rejects unknown authority fields. A valid value can contain only one approved workspace identity plus one to 256 relative components; each component is bounded to 255 UTF-8 bytes and required to be Unicode NFC. The content-free [`WorkspacePathError`](kernel/contracts/src/path.rs) exposes only a stable closed failure class and optional component index, never the rejected value or an ambient path. The exact `unicode-normalization 0.1.25` dependency and its two locked transitive crates are recorded in dependency classes, the external license catalog, hashes, provenance, and CycloneDX SBOM. Three unit tests and one external-client integration test prove exact valid construction, deterministic serialization/round trip, private-field revalidation, and rejection of injected absolute-root authority; 18 contract tests, two public integration tests, strict Clippy, dependency-class validation, supply-chain validation, and the complete product gate pass. This increment defines no root handle, absolute path, descriptor, filesystem observation, display link, platform adapter, Linux behavior, or macOS behavior; those remain assigned to the following sub-tasks.
  - [x] **Sub-task 6.1.1.2** (legacy `S-006-I02`): Reject absolute paths, empty components, dot components, traversal, alternate separators, NUL bytes, invalid encoding, and ambiguous normalization. Evidence: implementation commit `72d9448` expands the sole [`WorkspacePath`](kernel/contracts/src/path.rs) constructor/deserializer with a fixed precedence of content-free failures for empty path/component, current-directory and parent traversal, rooted candidates, drive/alternate-stream colon syntax, ASCII and six alternate Unicode separators, encoded dot/slash/backslash octets, NUL/control scalars, oversized component/count limits, trailing-dot/space ambiguity, invisible zero-width/directional formatting, and noncanonical NFC or NFKC forms. Invalid UTF-8 fails in Serde before construction, unknown fields fail closed, and workspace identities cannot carry separators or ambient roots. Four focused unit tests exercise valid Unicode NFC, every named invalid class, malformed bytes, count/size/workspace bounds, exact failure kind/index, and rejected-value redaction; the external-client test separately rejects parent traversal and injected absolute-root authority. Nineteen contract tests, two public integration tests, strict Clippy, the complete product gate, and regenerated supply-chain validation pass. This closes deterministic parser behavior only; the 500-case generated corpus, case/filesystem collision checks, descriptor resolution, and all platform execution evidence remain assigned to later sub-tasks.
  - [x] **Sub-task 6.1.1.3** (legacy `S-006-I03`): Define the platform path-adapter contract around authorized workspace handles. Evidence: implementation commit `f3f0ca7` adds the shared [`PlatformPathAdapter`](kernel/contracts/src/platform_path.rs) contract with adapter-specific associated handle/object types, exact `AdapterInstanceId`, `WorkspaceAuthorizationId`, and `WorkspaceId` affinity, a closed platform family, four read-only resolution intents, and 13 stable content-free failure classes. [`AuthorizedWorkspaceHandle`](kernel/contracts/src/platform_path.rs) exposes only stable authorization/adapter/workspace identities and platform family; [`HeldWorkspaceObject`](kernel/contracts/src/platform_path.rs) exposes only the canonical `WorkspacePath`, authorization and adapter identities, and validated intent. Implementations retain native authorization state and held native objects privately, associated types prevent accidental cross-adapter handle reuse, and errors retain at most the unsafe component index without path or operating-system text. Two internal deterministic-adapter tests and one external-client integration test prove exact successful binding plus workspace-mismatch and foreign-handle denial before any adapter observation. Twenty-one contract unit tests, three public contract integration tests, strict Clippy, formatting, supply-chain validation, and the complete product gate pass. This contract creates no workspace authorization, absolute path, native descriptor, filesystem observation, preimage hash, display link, Linux resolution, or macOS resolution; those remain assigned to following sub-tasks.
  - [x] **Sub-task 6.1.1.4** (legacy `S-006-I04`): Implement descriptor-relative no-follow resolution, held descriptors, file-identity checks, and exact preimage hashes. Evidence: implementation commit `7af1a6c` extends the shared path contract with closed regular-file/directory kinds, domain-separated mount/object identity digests, exact byte-length/SHA-256 [`FilePreimage`](kernel/contracts/src/platform_path.rs) evidence, and redacted hard-link/resource-limit failures. The [`LinuxPathAdapter`](platforms/linux/src/lib.rs) validates adapter/workspace affinity before observation, holds an approved root descriptor, walks each canonical component relative to the previously held directory with `O_PATH | O_NOFOLLOW | O_CLOEXEC`, rejects symlinks, special objects, multiply linked regular files, and cross-device observations, and reopens the final object read-only with no-follow flags while requiring an exact pre/post `fstat` snapshot match. Both the workspace root and final object descriptors remain owned by [`LinuxHeldObject`](platforms/linux/src/lib.rs) through use; regular-file read/hash intents compute a bounded exact SHA-256 with offset-based `pread`, compare metadata before and after hashing, and rehash the held descriptor during later identity validation without following a replaced pathname. Five Linux unit tests prove exact preimage bytes, harmless rename/path replacement safety, same-inode content-mutation detection, intermediate symlink denial, hard-link denial, object-kind denial, resource-limit denial, redacted component attribution, and workspace-affinity denial before a deliberately missing-path observation. The safe pinned `rustix 1.1.4` dependency and five locked all-target transitive packages are recorded in the accepted build graph, dependency classes, external license catalog, checksums, provenance, and CycloneDX SBOM; 27 focused build/dependency/supply-chain tests, 21 contract tests, three public contract integration tests, strict Clippy, and the complete product gate pass. No public ambient-path workspace authorization exists, this walker is not yet selected as the production Linux strategy, and it does not claim `openat2`, bind-mount-safe `RESOLVE_NO_XDEV`, capability probing, fail-closed resolver selection, display links, Ubuntu execution, or macOS behavior; those remain assigned to following sub-tasks.
  - [x] **Sub-task 6.1.1.5** (legacy `S-006-I05`): Implement Linux beneath/no-symlink resolution and a fail-closed fallback. Evidence: implementation commit `8c488e9` adds explicit [`LinuxResolutionStrategy`](platforms/linux/src/lib.rs) evidence and probes the already-held workspace root with safe `rustix::openat2` before any path resolution. On a supported kernel, every component and final reopen is relative to a held directory descriptor and enforces `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS | RESOLVE_NO_XDEV` plus no-follow, close-on-exec, read-only/object-kind flags; the continuously held result records `OpenAt2`. On an unavailable, filtered, or incompatible `openat2` primitive, the sole public automatic mode returns redacted `UnsupportedPrimitive` and performs no weaker path fallback. A private, non-selectable test branch retains the descriptor walk from Sub-task 6.1.1.4 and admits it only when descriptor-based `statx` reports one unchanged `STATX_MNT_ID` for the root and every component; absent mount identity stops with `UnsupportedPrimitive`. Seven Linux unit tests prove automatic selection matches an independent strict `openat2` probe, this Fedora host selects the primary strategy, differing mount IDs reject, verified fallback selection is private and explicit, missing mount evidence fails closed, direct/intermediate symlinks reject, and all held-descriptor/preimage/race protections continue to pass. Strict Clippy, formatting, current supply-chain validation, and the complete product gate pass. This is unprivileged local Fedora execution evidence only: no public workspace-selection constructor, Ubuntu execution, privileged bind-mount attack, packaging support, display-link behavior, or macOS behavior is claimed.
  - [ ] **Sub-task 6.1.1.6** (legacy `S-006-I06`): Implement macOS security-scoped bookmark resolution, alias rejection, stale-bookmark handling, case and Unicode collision checks, and mount-change detection.
  - [x] **Sub-task 6.1.1.7** (legacy `S-006-I07`): Generate absolute file-and-line links only after validation and never accept them back as authority. Evidence: implementation commit `6830c46` adds the distinct one-way [`DisplayFileLink`](kernel/contracts/src/display_link.rs) type with a bounded validated absolute `file:///` URI, optional positive one-based line, held-object identity evidence, a clickable `#L<n>` renderer, and content-free errors. The type deliberately implements no deserializer and no conversion to `WorkspacePath`, `GrantTarget`, or any adapter input; debug output redacts the URI, and the explicit rendering accessor is the sole path-disclosure operation. [`LinuxHeldObject::display_link`](platforms/linux/src/lib.rs) is callable only after successful descriptor resolution, revalidates the held root/object and exact regular-file preimage, obtains the current absolute display path from the held descriptor rather than caller input, percent-encodes raw Unix bytes into an ASCII URI, and binds the same object identity. Two contract tests reject non-file, relative, empty, malformed-percent, query, fragment, zero-line, and unredacted-debug cases. A Linux integration-style unit test resolves a filename containing spaces, `#`, and `%`, verifies exact percent encoding and line rendering, then feeds both the base URI and clickable target into `WorkspacePath` construction and proves typed rejection before adapter observation. Twenty-three contract tests, three public contract integration tests, eight Linux tests, strict Clippy, build/dependency/supply-chain validation, and the complete product gate pass. The explicit UI getter intentionally reveals the validated absolute path for display; no Visual Studio Code rendering/activation, Ubuntu execution, macOS link generation, or authority conversion is claimed.

- [x] **Task 6.1.2 - Produce reviewable artifacts** Evidence: Sub-tasks 6.1.2.1 through 6.1.2.3 publish current, immutable, machine-checked contract, corpus, display-replay, Fedora race-harness, and Ubuntu conformance reports. Each report independently binds its source revision and hashes, rejects platform/release overclaims, and retains explicit macOS and scope-specific unexecuted-environment blockers.
  - [x] **Sub-task 6.1.2.1:** Path contract and platform adapter interfaces. Evidence: source commits `434a10c` and `c3f98a9` add the versioned [`Path Authority Contract`](docs/architecture/path-authority-contract.md), deterministic [`path_contract_artifact.py`](scripts/path_contract_artifact.py) checker, and three claim-tampering/source-closure tests; evidence commits `09923ff`, `b2a96e6`, `e94d268`, and current source-binding refresh `03e0b6a` record the immutable [`path-contract-report.json`](artifacts/sprints/sprint-6/story-6.1/path-contract-report.json) against the exact 15-class error contract, three adapter methods, four handle methods, seven held-object methods, four resolution intents, and all four mandatory Linux `openat2` flags. The checker fails on API drift, a public fallback, display-to-authority conversion, missing strict flags, symbolic revision identity, or unsupported platform/release claims. Fedora is `verified-local`; the separate conformance artifact executes the shared logical fixture on pinned Ubuntu 26.04; macOS is `blocked-macos`; no evidence is substituted across platforms. Focused Python tests, Markdown/Mermaid/policy validation, the complete product gate, and the committed artifact checks pass.
  - [x] **Sub-task 6.1.2.2:** Canonicalization and display-link test corpus. Evidence: canonical corpus commits `1ff13fe`, `3f0c884`, `68c1d04`, and current source-binding refresh `03e0b6a` publish 640 deterministic constructor/deserializer cases with a digest-bound manifest and immutable execution report; display-link commits `945858b`, `6b91f2b`, and current source-binding refresh `03e0b6a` add 128 synthetic validated `file:///` links, both base and rendered `#L` forms, five declared authority surfaces, the real kernel integration matrix, and immutable [`display-link-authority-report.json`](artifacts/sprints/sprint-6/story-6.1/display-link-authority-report.json). The public fixtures contain no real workspace names or content. Deterministic regeneration, strict fixture closure, source-revision/hash binding, focused mutation tests, and runtime execution through the shared contract and kernel gates pass. Case-collision controls preserve distinct spelling on Fedora but make no macOS collision claim; macOS remains `blocked-macos`.
  - [x] **Sub-task 6.1.2.3:** File-identity and race-condition test harness. Evidence: source commits `073a7a6` and `4dc8700` add the deterministic [`path_race_artifact.py`](scripts/path_race_artifact.py) runner, three report-tampering tests, a concurrent Linux stress test that alternates atomically replaced safe regular files with symlinks to an out-of-root synthetic file while performing at least 512 strict resolutions, and an ignored-by-default bind-mount attack executed only under a private `unshare` user/mount namespace. Every successful resolution carries only the expected in-workspace preimage; symlink/race outcomes deny or retain the safe held object, changed link identity fails closed, and a real cross-mount replacement returns `MountChanged`. Evidence commits `b2a96e6`, `e94d268`, and current source-binding refresh `03e0b6a` record six executed Fedora traces spanning symlinks, hard links, rename/replacement, post-resolution mutation, strict strategy/mount-identity drift, concurrent TOCTOU, and isolated bind-mount swap with zero out-of-root access. macOS aliases remain `blocked-macos`; the race suite does not substitute the separate Ubuntu logical-fixture execution for Ubuntu attack execution. Ten Linux tests, the isolated mount test, strict Clippy, focused evidence tests, and the complete product gate pass.

- [ ] **Task 6.1.3 - Verify and close the story**
  - [x] **Sub-task 6.1.3.1:** `S-006-UT01` runs at least 500 generated path cases covering absolute, empty, dot, traversal, alternate separator, NUL, invalid encoding, case, and Unicode ambiguity; assert zero accepted escape. Evidence: source commit `1ff13fe` adds the deterministic public-synthetic [`fixtures/paths/v1/corpus.json`](fixtures/paths/v1/corpus.json), digest-bound manifest, real Rust [`path_corpus.rs`](kernel/contracts/tests/path_corpus.rs) integration test, generator/checker, and three evidence-tampering tests; evidence commits `3f0c884` and current source-binding refresh `68c1d04` record the immutable [`path-corpus-report.json`](artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json). The closed corpus contains 640 cases in ten equal 64-case classes: absolute roots, empty paths/components, dot components, direct/encoded traversal, ASCII/Unicode alternate separators, NUL, malformed UTF-8 JSON, case-collision candidates, noncanonical Unicode, and valid controls. The real constructor/deserializer rejects all 512 escape-shaped inputs with zero admitted escape and accepts 128 bounded controls. The 64 case candidates remain 64 distinct spellings on this case-sensitive Fedora path contract; they are not substituted as macOS collision evidence, which remains assigned to open Sub-task 6.1.1.6. Deterministic regeneration, focused Python mutation tests, the Rust corpus test, strict Clippy, all workspace tests, and the complete product gate pass with zero filesystem observations.
  - [ ] **Sub-task 6.1.3.2:** `S-006-ST01` performs symlink, alias, hard-link where applicable, rename, replacement, mount-swap, and time-of-check/time-of-use attacks while descriptors are held; assert zero out-of-root access. All non-macOS evidence is complete: commits `073a7a6`, `4dc8700`, and `e94d268` pass Fedora symlink, hard-link, rename, replacement, mutation, mount-identity-drift, concurrent TOCTOU, and real bind-mount replacement inside an isolated user/mount namespace with zero out-of-root access. This item remains open only because alias execution requires the open macOS implementation; no result is substituted.
  - [x] **Sub-task 6.1.3.3:** `S-006-UT02` feeds generated display links back to every authority-bearing API; assert type or policy rejection and no filesystem observation. Evidence: source commit `945858b` adds [`display_link_authority.rs`](kernel/engine/tests/display_link_authority.rs), which reconstructs 128 valid display objects and feeds both each base URI and rendered line target to `WorkspacePath::new`, `WorkspacePath` deserialization, `GrantIssuer::issue_session_read`, `PolicyEngine::strict_local_read_only`, and the adapter's type-enforced `&WorkspacePath` boundary. All 1,280 surface/candidate assertions reject; zero grants become current and zero filesystem observations occur. The evidence checker also fails if `DisplayFileLink` becomes deserializable or convertible to authority, the adapter stops requiring `WorkspacePath`, a surface disappears, or platform/release evidence is overclaimed. Evidence commit `6b91f2b` binds the corpus, production sources, executable test, checker, and claim-tampering tests to one immutable revision. The complete product gate passes.
  - [ ] **Sub-task 6.1.3.4:** `S-006-IT01` resolves the same logical fixture through fake, macOS, Fedora, and Ubuntu adapters; assert equivalent policy decisions and platform-specific identity evidence. All non-macOS evidence is complete: source commit `b449117` adds one exact logical fixture, deterministic fake and Fedora adapter tests, and a constrained Ubuntu runner; evidence commit `9f3949c` publishes [`path-platform-conformance.json`](artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json). Fake, Fedora 44, and a pinned Ubuntu 26.04 image all return the same `admit-canonical-read` decision and platform-specific adapter/object identity evidence. Ubuntu executes the committed source archive with Rust 1.95.0, Cargo offline, no network, a read-only root, all capabilities dropped, `no-new-privileges`, bounded memory/PIDs, a non-root user, and exact image/source hashes. This item remains open only because the required macOS adapter execution is unavailable; no result is substituted.
  - [x] **Sub-task 6.1.3.5 - Product security evidence:** Map `SR-PLT-004`, `SR-ACC-004` through `SR-ACC-006`, `SR-TST-002`, `SR-TST-004`, and reviewer protocol `RV-04`; retain generated corpus, race traces, descriptor identities, and independent secure-path review. Evidence: source commits `5c7dc03`, `4dc8700`, and `b7df142` add and update a fail-closed independent [`path_boundary_review.py`](scripts/path_boundary_review.py), exact Story 6.1 security mapper, Ubuntu conformance subject integration, and tests for subject omission, escape admission, stale hashes, requirement completion, fuzzing, review, release, and platform overclaims. Current review commit `4efccff` re-executes all five subject checkers and reconciles 640 generated cases, zero admitted escapes, 1,280 display-authority denials, six Fedora race traces including the isolated bind-mount attack, equivalent fake/Fedora/Ubuntu logical-fixture decisions, and zero out-of-root reads in [`path-boundary-review.json`](artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json). Current security-map commit `c005868` retains 26 hashed public-synthetic/source/evidence records and maps all six named requirements plus `RV-04`; every product requirement remains `not-complete`, fixed generation is explicitly not fuzzing, `RV-04` remains partial on blocked macOS and scope-specific attack tests, external human review is not claimed, and macOS evidence is not substituted. All subject checkers, focused tests, documentation validation, and the complete product gate pass.

##### Story Acceptance Criteria

- [ ] **Story AC 6.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every accepted tool path contains only canonical workspace identity plus normalized relative components and resolves to the same held object through use.
- [ ] **Story AC 6.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every denial identifies the unsafe component without exposing unrelated absolute paths or probing prohibited filesystem locations.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 6.AC1:** `AT-PATH-001` passes on every reference platform.
- [ ] **Sprint AC 6.AC2:** Valid fixture paths resolve unambiguously and invalid paths fail closed.
- [ ] **Sprint AC 6.AC3:** Symlink, alias, rename, mount, case, and Unicode attacks cannot escape the workspace.
- [ ] **Sprint AC 6.AC4:** Display links cannot be replayed as tool arguments.
- [ ] **Sprint AC 6.AC5:** Path denials produce receipts without exposing unrelated absolute paths.

**Gate decision:** Sprint 6 is PASS only when Story 6.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 7 - Platform Adapter Contract and Release Manifests

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-007`.

**Sprint goal:** Freeze and implement the shared platform boundary before product logic depends on operating-system behavior.

**Source coverage:** `AM-PLT-001`; PRD Sections 7 and 9; inventory Section 5A and Section 29; `AT-PLAT-001`.

**Dependencies:** Sprint 6; legacy dependency record: Sprint 3 (legacy S-003), Sprint 4 (legacy S-004), Sprint 6 (legacy S-006).

#### [ ] Story 7.1 - Platform Adapter Contract and Release Manifests

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need platform adapter contract and release manifests so that AgentMage delivers the following bounded outcome: Freeze and implement the shared platform boundary before product logic depends on operating-system behavior.

##### Tasks and Sub-tasks

- [ ] **Task 7.1.1 - Implement the bounded story**
  - [ ] **Sub-task 7.1.1.1** (legacy `S-007-I01`): Define adapters for workspace authorization, secure paths, tool confinement, secret storage, process limits, local inference, model installation, packaging, and updates. Shared boundary complete: implementation commit `7758614` adds platform API version 1, an exact ten-capability closure including network isolation, immutable manifest/runtime identities, content-free observations and failures, and a platform-independent kernel activation wrapper. Native Linux mechanism adapters remain assigned to Sprint 9; macOS implementations remain assigned to Sprint 8, so this cross-platform item stays open.
  - [ ] **Sub-task 7.1.1.2** (legacy `S-007-I02`): Freeze the macOS release manifest fields for platform build, architecture, toolchain, Team ID, bundle IDs, App Group, entitlements, designated requirements, helper hashes, package digest, and Visual Studio Code version.
  - [x] **Sub-task 7.1.1.3** (legacy `S-007-I03`): Freeze Fedora and Ubuntu adapter manifests for sandbox, secret service, inference runtime, paths, process limits, package digest, and supported distributions. Evidence: source commit `f03283d` adds deterministic public-synthetic Fedora 44/x86_64 and Ubuntu 26.04/x86_64 contract fixtures with exact OS, Rust 1.95.0, Visual Studio Code 1.132.0, package, and ten capability-mechanism identities. Each identifies Bubblewrap/seccomp, Linux Secret Service, cgroup v2, strict `openat2`, native `llama.cpp`, verified model installation, RPM or DEB packaging, signed manual update/rollback, and no-connectivity network namespaces. The package values are explicitly synthetic pre-release fixture digests with `release_claim: none`; they cannot satisfy packaging or support gates.
  - [ ] **Sub-task 7.1.1.4** (legacy `S-007-I04`): Implement startup capability detection and fail-closed adapter selection. Shared selection complete: commit `7758614` compares API, platform, architecture, OS build, toolchain, Visual Studio Code, and package identity in fixed order, then requires all ten manifest-bound capability probes before constructing `VerifiedPlatformAdapter`. Twenty missing/invalid capability cases and every runtime-identity mutation refuse without fallback. Native Linux probes remain for Sprint 9 and macOS probes remain for Sprint 8, so the platform-complete item stays open.
  - [x] **Sub-task 7.1.1.5** (legacy `S-007-I05`): Separate maintainer release dependencies and credentials from end-user runtime dependencies. Evidence: commit `f03283d` freezes sorted, machine-checked `maintainer_release`, `end_user_runtime`, and `excluded_end_user` classes in both Linux fixtures and documents the boundary in [`platform-adapter-contract.md`](docs/architecture/platform-adapter-contract.md). Build/signing tools are not end-user requirements; signing credentials are excluded and no credential or private-environment value is retained. Five mutation tests reject dependency overlap, private fields, credential promotion, and release overclaims.
  - [ ] **Sub-task 7.1.1.6** (legacy `S-007-I06`): Implement environment recording for reproducible acceptance results. Non-macOS recorder complete: commit `c414a0f` implements the allowlisted [`platform_result_recorder.py`](scripts/platform_result_recorder.py) with exact OS, distribution, architecture, kernel, Python, encoding, privilege, container, CI, and build/fixture/model/runtime/policy identities while excluding hostnames, users, paths, addresses, environment values, and secret-store values. It supports deterministic synthetic records and validated current Linux collection. Native macOS recording remains unavailable, so the cross-platform item stays open.
  - [ ] **Sub-task 7.1.1.7** (legacy `S-007-I07`): Add contract tests that run identically against macOS, Fedora, Ubuntu, and deterministic fake adapters. All available non-macOS contract targets complete: commit `f03283d` adds one external conformance test that runs identical activation semantics for deterministic fake, Fedora, and Ubuntu identities; evidence commit `ca52373` records local Fedora and pinned no-network Ubuntu execution with three equivalent results. The required macOS adapter run remains blocked and is not substituted.

- [ ] **Task 7.1.2 - Produce reviewable artifacts**
  - [x] **Sub-task 7.1.2.1:** Versioned platform adapter API. Evidence: implementation commit `7758614` and [`platform-adapter-contract.md`](docs/architecture/platform-adapter-contract.md) define API version 1, four closed platform families, two architectures, ten required capabilities, six runtime identity fields, manifest-bound capability observations, 11 content-free startup failures, and the sole verified activation wrapper. Enforced evidence source commit `9826b4a` measures zero operating-system branch tokens in the production kernel selector and fails on API, capability, error, or probe-path drift; evidence commit `ca52373` records the immutable result.
  - [ ] **Sub-task 7.1.2.2:** Reference-platform release manifests. Fedora and Ubuntu contract manifests are complete at `f03283d` and their exact repeated canonical hashes are retained at `ca52373`. They remain pre-release fixtures rather than built package manifests, and the macOS reference manifest remains blocked, so this item stays open.
  - [x] **Sub-task 7.1.2.3:** Maintainer versus end-user dependency matrix. Evidence: commit `f03283d` publishes the documented matrix and two machine-checked platform instances. Each contains six maintainer dependencies, five end-user runtime dependencies, six excluded end-user dependencies, zero conflicting overlap, and explicit credential exclusion; evidence commit `ca52373` retains counts and exact manifest hashes.
  - [x] **Sub-task 7.1.2.4:** Platform capability and startup-failure reports. Evidence: source commits `f03283d` and `9826b4a` add the fail-closed [`platform_manifest_artifact.py`](scripts/platform_manifest_artifact.py) runner and five mutation tests; evidence commit `ca52373` publishes [`platform-contract-report.json`](artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json) against the exact API, both Linux fixtures, two Rust conformance tests, local Fedora execution, and constrained Ubuntu execution. The report explicitly limits results to contract semantics, marks native mechanisms and macOS open, retains no private values, and makes no release claim.

- [ ] **Task 7.1.3 - Verify and close the story**
  - [ ] **Sub-task 7.1.3.1:** `S-007-UT01` runs the shared adapter conformance suite against fake, macOS, Fedora, and Ubuntu implementations; assert identical contract semantics and declared platform-specific evidence. Fake, Fedora, and Ubuntu contract semantics pass at `ca52373`; macOS remains blocked.
  - [ ] **Sub-task 7.1.3.2:** `S-007-UT02` removes or corrupts signing, sandbox, secret-store, path, process-limit, inference, package, and network primitives one at a time; assert startup refusal before workspace access. Commit `7758614` executes unavailable and invalid states for every one of the ten shared capabilities, producing 20 exact refusals with no verified wrapper or workspace observation. Native Linux primitive corruption and macOS signing/entitlement execution remain open.
  - [ ] **Sub-task 7.1.3.3:** `S-007-ST01` substitutes wrong platform, architecture, OS build, toolchain, VS Code build, helper hash, entitlement, and package digest; assert unsupported-state reporting and zero fallback. Commit `7758614` proves wrong API, platform, architecture, OS build, toolchain, Visual Studio Code build, package digest, capability identity, platform evidence, and manifest evidence all refuse without fallback. Helper-hash and entitlement cases require the blocked macOS implementation.
  - [ ] **Sub-task 7.1.3.4:** `S-007-IT01` generates release manifests twice from pinned inputs; assert complete component closure, reproducible fields, and separation of maintainer credentials from user dependencies. Evidence `ca52373` proves each committed Linux fixture canonicalizes twice to one exact digest with complete capability/dependency closure. Release-package generation and the macOS manifest remain open, so canonical fixture replay is not promoted to the full integration claim.
  - [x] **Sub-task 7.1.3.5 - Product security evidence:** Map `SR-GOV-006`, `SR-PLT-009` through `SR-PLT-012`, `SR-SUP-002` through `SR-SUP-005`, and `SR-TST-007`; retain adapter contract results, manifest diffs, failure matrix, and clean-environment identities. Evidence: source commit `0ec6eb6` adds the exact fail-closed mapper and four removal/path/status/overclaim test groups; current evidence commit `34679a6` publishes [`security-evidence-map.json`](artifacts/sprints/sprint-7/story-7.1/security-evidence-map.json) against the refreshed accepted source-dependency SBOM and provenance. It retains 19 hashed records, maps all ten requirements and `RV-01` through `RV-05`, records one demonstrated contract scope and nine partial or blocked scopes, leaves every product requirement `not-complete`, identifies Ubuntu as containerized no-network contract execution, marks macOS blocked, and makes no release or external-review claim.

##### Story Acceptance Criteria

- [ ] **Story AC 7.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then product logic contains no operating-system branch outside the adapter boundary, and every runtime component resolves to one declared platform manifest.
- [ ] **Story AC 7.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then linux evidence cannot satisfy Mac gates, Mac evidence cannot satisfy Linux gates, and all result bundles identify exact hardware/software context without secrets.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 7.AC1:** Each reference platform selects only its declared adapter.
- [ ] **Sprint AC 7.AC2:** Missing signing, sandbox, secret-store, path, resource, or network primitives prevent startup.
- [ ] **Sprint AC 7.AC3:** Capability packs contain no operating-system branch that bypasses an adapter.
- [ ] **Sprint AC 7.AC4:** Platform result records contain enough identity to reproduce a failure without secrets.
- [ ] **Sprint AC 7.AC5:** The platform portion of `AT-PLAT-001` passes before packaging-specific work begins.

**Gate decision:** Sprint 7 is PASS only when Story 7.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 8 - macOS Security Topology and Packaging

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-008`.

**Sprint goal:** Build and prove the signed, notarized, App-Sandboxed Apple Silicon reference topology.

**Source coverage:** `AM-SEC-001`, `AM-SEC-002`; PRD Section 7.1 and Section 9; inventory Section 5A; `AT-PLAT-001`, `AT-SEC-001`, `AT-SBX-001`.

**Dependencies:** Sprint 7; legacy dependency record: Sprint 5 (legacy S-005), Sprint 6 (legacy S-006), Sprint 7 (legacy S-007).

#### [ ] Story 8.1 - macOS Security Topology and Packaging

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need macos security topology and packaging so that AgentMage delivers the following bounded outcome: Build and prove the signed, notarized, App-Sandboxed Apple Silicon reference topology.

##### Tasks and Sub-tasks

- [ ] **Task 8.1.1 - Implement the bounded story**
  - [ ] **Sub-task 8.1.1.1** (legacy `S-008-I01`): Implement the arm64 kernel host with Hardened Runtime and minimal App Sandbox entitlements.
  - [ ] **Sub-task 8.1.1.2** (legacy `S-008-I02`): Implement the signed native Visual Studio Code IPC bridge and authenticated mode-restricted App Group socket.
  - [ ] **Sub-task 8.1.1.3** (legacy `S-008-I03`): Verify audit tokens, designated requirements, bundle and App Group identity, peer identity, and fresh launch challenge.
  - [ ] **Sub-task 8.1.1.4** (legacy `S-008-I04`): Implement the native workspace picker and read-only app-scoped security-scoped bookmark lifecycle.
  - [ ] **Sub-task 8.1.1.5** (legacy `S-008-I05`): Implement the signed stateless XPC tool helper with one consumed grant, one bookmark, isolated scratch, bounded resources, no network entitlement, and no workspace write authority.
  - [ ] **Sub-task 8.1.1.6** (legacy `S-008-I06`): Package the native Metal inference service as a separate untrusted process with no workspace, tool, grant, or credential access.
  - [ ] **Sub-task 8.1.1.7** (legacy `S-008-I07`): Build, sign, notarize, staple, Gatekeeper-check, install, launch, uninstall, and rollback on the isolated release runner.

- [ ] **Task 8.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 8.1.2.1:** Signed reference package and component hash manifest.
  - [ ] **Sub-task 8.1.2.2:** Entitlement, code-signing, notarization, stapling, and Gatekeeper reports.
  - [ ] **Sub-task 8.1.2.3:** IPC authentication and bookmark lifecycle receipts.
  - [ ] **Sub-task 8.1.2.4:** macOS sandbox profile and attack results.

- [ ] **Task 8.1.3 - Verify and close the story**
  - [ ] **Sub-task 8.1.3.1:** `S-008-UT01` validates code identity, audit token, Team ID, bundle/App Group identity, protocol version, fresh challenge, and socket mode with each field absent or wrong; assert handshake refusal.
  - [ ] **Sub-task 8.1.3.2:** `S-008-ST01` attacks host, bridge, XPC helper, and inference service for ambient home, device, process, environment, credential, network, workspace-write, grant, and cross-user access; assert zero unauthorized access.
  - [ ] **Sub-task 8.1.3.3:** `S-008-RT01` exercises stale/revoked bookmarks, helper crash, host crash, interrupted install, failed launch, uninstall, and rollback; assert cleanup and prior-valid-state recovery.
  - [ ] **Sub-task 8.1.3.4:** `S-008-AT01` builds, signs, notarizes, staples, Gatekeeper-checks, installs, launches, uses, and removes the package as a standard user on the pinned Apple Silicon image.
  - [ ] **Sub-task 8.1.3.5 - Product security evidence:** Map `SR-PLT-001` through `SR-PLT-008`, `SR-SUP-002`, `SR-TST-007` through `SR-TST-009`, and `RV-01` through `RV-05`; retain signature/notarization output, entitlements, IPC traces, sandbox results, install video/log, and reviewer record.

##### Story Acceptance Criteria

- [ ] **Story AC 8.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every executable and helper matches the signed manifest and minimal entitlement set; wrong or unmanifested code cannot connect, load, or execute.
- [ ] **Story AC 8.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the full Mac workflow requires no administrator access or excluded ambient development tools and leaves only documented, scanner-detectable remnants after uninstall.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 8.AC1:** macOS portions of `AT-PLAT-001`, `AT-SEC-001`, and `AT-SBX-001` pass.
- [ ] **Sprint AC 8.AC2:** Unsigned, wrongly signed, wrong-bundle, wrong-App-Group, wrong-peer, and replayed bridge attempts are rejected.
- [ ] **Sprint AC 8.AC3:** The tool helper cannot access ambient home paths, network, devices, ungranted roots, or workspace writes.
- [ ] **Sprint AC 8.AC4:** The end-user installation requires none of the explicitly excluded ambient development tools.
- [ ] **Sprint AC 8.AC5:** Signing and notarization credentials appear nowhere in repository data, build output, logs, model context, or package contents.

**Gate decision:** Sprint 8 is PASS only when Story 8.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 9 - Fedora and Ubuntu Security Topology

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-009`.

**Sprint goal:** Build and prove the unprivileged Linux topology with identical shared capability behavior.

**Source coverage:** `AM-SEC-001`, `AM-SEC-002`; PRD Section 7.2 and Section 9; inventory Section 5A; `AT-PLAT-001`, `AT-SEC-001`, `AT-SBX-001`.

**Dependencies:** Sprint 8; legacy dependency record: Sprint 5 (legacy S-005), Sprint 6 (legacy S-006), Sprint 7 (legacy S-007).

#### [ ] Story 9.1 - Fedora and Ubuntu Security Topology

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need fedora and ubuntu security topology so that AgentMage delivers the following bounded outcome: Build and prove the unprivileged Linux topology with identical shared capability behavior.

##### Tasks and Sub-tasks

- [ ] **Task 9.1.1 - Implement the bounded story**
  - [x] **Sub-task 9.1.1.1** (legacy `S-009-I01`): Implement the unprivileged kernel host and authenticated mode-restricted Unix socket with peer-credential and fresh-session checks. Evidence: source commit `f7271cf` implements owner-only Unix sockets, kernel `SO_PEERCRED` user/process identity, held executable SHA-256 identity, fixed protocol framing, fresh random challenge/launch material, HMAC-SHA-256 authentication, one-time atomic consumption, replay denial, and redacted/zeroized credentials. Artifact commit `cd69735` binds all five IPC tests to committed source in `linux-control-verification.json`; no Ubuntu or macOS platform claim is inferred.
  - [x] **Sub-task 9.1.1.2** (legacy `S-009-I02`): Implement fresh Bubblewrap workers with read-only workspace bind, private scratch, minimal executable and environment allowlists, and no network namespace connectivity. Evidence: source commits `490aba3` and `ffd955e` implement one typed canonical read operation per fresh worker, descriptor-bound read-only workspace and executable mounts through systemd `OpenFile=` plus Bubblewrap `--ro-bind-fd`, private scratch/dev/proc, complete namespace separation, fixed environment, root-owned artifact verification, bounded output, and no generic shell or argument surface. Artifact commit `cd69735` records real Fedora 44 tests for canonical read, write denial, ambient-root/device/process denial, environment minimization, network denial, kernel control state, and output exhaustion; Ubuntu execution remains open.
  - [x] **Sub-task 9.1.1.3** (legacy `S-009-I03`): Apply seccomp rules and user cgroup resource limits to every worker. Evidence: source commits `490aba3` and `ffd955e` pin the pure-Rust `seccompiler` policy compiler, pass a fixed classic-BPF deny policy to Bubblewrap, require `NoNewPrivileges`, and launch every worker in a transient user service with exact `MemoryMax`, `MemorySwapMax=0`, `TasksMax`, `CPUQuota`, and `RuntimeMaxSec` bounds. Fedora tests observe `NoNewPrivs: 1` and seccomp mode 2 from inside the worker, terminate an unbounded worker, and reject over-bound output; artifact commit `cd69735` records the tool identities, cgroup v2 host state, tests, and limitations.
  - [x] **Sub-task 9.1.1.4** (legacy `S-009-I04`): Implement descriptor-relative Linux path resolution and fail-closed behavior when required kernel features are absent. Evidence: source commits `7af1a6c`, `8c488e9`, `073a7a6`, `4dc8700`, and `b449117` implement held workspace/file descriptors, strict `openat2`, verified descriptor-walk fallback, mount-identity enforcement, exact preimages, mutation/race denial, isolated bind-mount attack coverage, and Fedora/Ubuntu conformance. Evidence commit `9f3949c` retains the non-macOS platform conformance report; unavailable mount identity and unsafe resolution paths fail closed.
  - [x] **Sub-task 9.1.1.5** (legacy `S-009-I05`): Integrate Linux Secret Service without passing keys through configuration, environment dumps, model context, tools, or logs. Evidence: source commits `bf389e5` and `93e24cd` implement fixed non-secret attributes, pipe-only credential transfer, a cleared and reconstructed session-bus environment, root-owned client identity plus per-operation digest revalidation, bounded output and elapsed time, content-free errors/receipts, zeroized value and sensitive pipe buffers, and no serialization, display, configuration, model, or tool surface for values. Evidence commit `d8f6984` binds the libsecret package and `secret-tool` executable identities, three redaction/bound unit tests, a live no-match probe, and a synthetic store/lookup/clear round trip with verified post-clear absence to committed source; no pre-existing credential was enumerated or changed.
  - [ ] **Sub-task 9.1.1.6** (legacy `S-009-I06`): Package the declared local inference adapter behind the shared runtime contract and isolate it from tool and workspace authority.
  - [ ] **Sub-task 9.1.1.7** (legacy `S-009-I07`): Build, install, launch, uninstall, and recover on clean Fedora and Ubuntu environments.

- [ ] **Task 9.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 9.1.2.1:** Fedora and Ubuntu packages and component manifests.
  - [ ] **Sub-task 9.1.2.2:** Bubblewrap, seccomp, cgroup, IPC, and Secret Service verification reports.
  - [ ] **Sub-task 9.1.2.3:** Linux sandbox attack results.
  - [ ] **Sub-task 9.1.2.4:** Cross-platform adapter parity report.

- [ ] **Task 9.1.3 - Verify and close the story**
  - [x] **Sub-task 9.1.3.1:** `S-009-UT01` authenticates valid and wrong-user, wrong-process, replayed, stale, malformed, and version-mismatched Unix-socket peers; assert only the fresh declared peer proceeds. Evidence: source commit `f7271cf` contains generated-material, exact-peer, real-socket, mutation-matrix, unsafe-parent, existing-socket, and short-frame tests. Artifact commit `cd69735` reruns and binds all five IPC tests to source revision `5278d9bfb943dc90d5042a85da5838e225249483`; every admitted request is fresh, exact, and consumed once.
  - [ ] **Sub-task 9.1.3.2:** `S-009-ST01` attacks Bubblewrap workers for ambient home, device, network, process, environment, secret-store, ungranted-root, and workspace-write access under seccomp and cgroups; assert zero escape.
  - [ ] **Sub-task 9.1.3.3:** `S-009-UT02` disables Bubblewrap, user namespaces, seccomp, cgroups, Secret Service, descriptor-safe paths, and network isolation independently; assert fail-closed startup and no degraded insecure mode.
  - [ ] **Sub-task 9.1.3.4:** `S-009-AT01` installs, launches, exercises, uninstalls, and residue-scans clean supported Fedora and Ubuntu images as unprivileged users using only published steps.
  - [ ] **Sub-task 9.1.3.5 - Product security evidence:** Map `SR-PLT-001`, `SR-PLT-006`, `SR-PLT-007`, `SR-PLT-010`, `SR-PLT-012`, `SR-TST-006` through `SR-TST-009`, and `RV-02` through `RV-05`; retain package manifests, sandbox policies, syscall/resource traces, parity results, and clean-install evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 9.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then fedora and Ubuntu produce equivalent shared policy, receipt, path, cancellation, and evidence outcomes while preserving distinct platform manifests.
- [ ] **Story AC 9.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then each worker receives only one operation's exact read scope, private scratch, executable allowlist, limits, and no network or durable secret material.

#### [ ] Story 9.2 - Linux Native Reference and Docker Compatibility Boundary

**User-facing value:** As a Linux user, I need native `llama.cpp` and Docker Model Runner to follow one AgentMage contract without Docker becoming an undeclared privilege or network boundary.

##### Tasks and Sub-tasks

- [ ] **Task 9.2.1 - Implement explicit Linux runtime topologies**
  - [ ] **Sub-task 9.2.1.1:** Package unprivileged native `llama.cpp` as the Fedora/Ubuntu security-reference adapter with pinned build identity, restricted model store, bounded resources, guarded kernel IPC, and no workspace/tool/grant/credential authority.
  - [ ] **Sub-task 9.2.1.2:** Package Docker Model Runner as an optional compatibility adapter with pinned engine/model OCI digests, explicit install/daemon prerequisites, declared user/group privileges, bounded mounts/resources, and no workspace/tool/grant/credential authority.
  - [ ] **Sub-task 9.2.1.3:** Prevent the extension, tool worker, unrelated process, and arbitrary container from connecting directly to the Docker Model Runner API; route AgentMage inference only through the guarded kernel adapter.
  - [ ] **Sub-task 9.2.1.4:** Refuse Docker mode when daemon privilege, socket ownership, API binding, container reachability, image identity, resource limits, or zero-egress state differs from the approved topology.

- [ ] **Task 9.2.2 - Verify and close the story**
  - [ ] **Sub-task 9.2.2.1:** Inspect process trees, user/group identity, capabilities, namespaces, sockets, mounts, cgroups, images, and firewall state for native and Docker modes on clean Fedora and Ubuntu systems.
  - [ ] **Sub-task 9.2.2.2:** Probe the unauthenticated Docker API from LAN, host, same-user, extension, tool-worker, ordinary-container, and separate-namespace positions; assert only the declared kernel path succeeds.
  - [ ] **Sub-task 9.2.2.3:** Disable each required isolation primitive independently; assert visible startup refusal with no fallback to a weaker adapter.
  - [ ] **Sub-task 9.2.2.4 - Product security evidence:** Extend `RV-02`, `RV-03`, and `RV-05`; map `SR-PLT-001`/`SR-PLT-006`/`SR-PLT-007`/`SR-PLT-010`/`SR-PLT-012`, `SR-NET-003`/`SR-NET-006`; retain process/socket/mount diagrams, immutable identities, reachability matrix, failed-preflight results, and parity report.

##### Story Acceptance Criteria

- [ ] **Story AC 9.2.AC1:** Given native and Docker Linux modes, when their topologies are inspected, then every process, privilege, mount, socket, writable path, resource limit, model artifact, and network rule matches `RUNTIME-BOUNDARIES.md` or startup fails.
- [ ] **Story AC 9.2.AC2:** Given Docker Model Runner's unauthenticated API, when every declared hostile peer probes it, then no peer except the guarded kernel adapter succeeds and no non-loopback listener exists.
- [ ] **Story AC 9.2.AC3:** Given a Linux user without an approved Docker boundary, when AgentMage starts, then native `llama.cpp` remains the reference path and Docker is neither required nor selected silently.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 9.AC1:** Linux portions of `AT-PLAT-001`, `AT-SEC-001`, and `AT-SBX-001` pass on Fedora and Ubuntu.
- [ ] **Sprint AC 9.AC2:** Workers cannot access ambient home paths, network, devices, ungranted roots, or workspace writes.
- [ ] **Sprint AC 9.AC3:** Wrong-user, wrong-process, replayed, and malformed IPC peers are rejected.
- [ ] **Sprint AC 9.AC4:** Failure of Bubblewrap, seccomp, cgroups, Secret Service, path protection, or network controls prevents startup.
- [ ] **Sprint AC 9.AC5:** Shared contract fixtures produce equivalent policy and evidence results across macOS and Linux.

**Gate decision:** Sprint 9 is PASS only when Stories 9.1 and 9.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 10 - Strict-Local Network and Data-Residency Boundary

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-010`.

**Sprint goal:** Make local-only operation observable and enforceable after model acquisition ends.

**Source coverage:** `AM-NET-001`; PRD Section 14; inventory Sections 5A and 5B; `AT-NET-001`, `AT-NET-002`.

**Dependencies:** Sprint 9; legacy dependency record: Sprint 8 (legacy S-008), Sprint 9 (legacy S-009).

#### [ ] Story 10.1 - Strict-Local Network and Data-Residency Boundary

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need strict-local network and data-residency boundary so that AgentMage delivers the following bounded outcome: Make local-only operation observable and enforceable after model acquisition ends.

##### Tasks and Sub-tasks

- [ ] **Task 10.1.1 - Implement the bounded story**
  - [ ] **Sub-task 10.1.1.1** (legacy `S-010-I01`): Deny outbound network access for shells, kernel, tools, converters, indexers, and local inference except the declared guarded local path.
  - [ ] **Sub-task 10.1.1.2** (legacy `S-010-I02`): Prevent Visual Studio Code and tool workers from connecting directly to the raw inference runtime.
  - [ ] **Sub-task 10.1.1.3** (legacy `S-010-I03`): Reject undeclared listeners and non-loopback, local-area-network, or container-accessible bindings.
  - [ ] **Sub-task 10.1.1.4** (legacy `S-010-I04`): Detect cloud-synchronized paths and remote filesystems and reject them as strict-local state roots.
  - [ ] **Sub-task 10.1.1.5** (legacy `S-010-I05`): Inventory every process, socket, port, writable path, tool, and network rule used by a session.
  - [ ] **Sub-task 10.1.1.6** (legacy `S-010-I06`): Detect hidden telemetry, analytics, update checks, remote assets, remote fonts, crash reporting, and model marketplace calls.
  - [ ] **Sub-task 10.1.1.7** (legacy `S-010-I07`): Implement a content-free network-attempt ledger and an offline proof workflow.

- [ ] **Task 10.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 10.1.2.1:** Strict-local policy profile.
  - [ ] **Sub-task 10.1.2.2:** Process, socket, path, and network-boundary report.
  - [ ] **Sub-task 10.1.2.3:** Firewall and packet-capture acceptance harness.
  - [ ] **Sub-task 10.1.2.4:** Cloud-sync and remote-filesystem detection fixtures.

- [ ] **Task 10.1.3 - Verify and close the story**
  - [ ] **Sub-task 10.1.3.1:** `S-010-UT01` classifies loopback, Unix socket, LAN, container, proxy, DNS override, remote mount, and cloud-synchronized path fixtures; assert only the declared authenticated local inference transport is accepted.
  - [ ] **Sub-task 10.1.3.2:** `S-010-ST01` runs every normal workflow under packet capture, DNS/socket tracing, and process attribution for at least 60 minutes; assert zero undeclared outbound attempts and zero bytes.
  - [ ] **Sub-task 10.1.3.3:** `S-010-ST02` injects telemetry, crash upload, remote font/asset, marketplace, update, proxy, and hostile-loopback dependencies; assert build/startup/test failure rather than silent contact.
  - [ ] **Sub-task 10.1.3.4:** `S-010-AT01` disables all external networking after installation and executes every v0.1 workflow; assert functional completeness, content-free attempted-egress ledger, and no cloud fallback.
  - [ ] **Sub-task 10.1.3.5 - Product security evidence:** Map `SR-NET-001` through `SR-NET-007`, `SR-PLT-005`, `SR-AI-004`, `SR-OPS-009`, and `RV-06`/`RV-07`; retain packet captures, syscall/socket reports, process attribution, path classifications, and offline workflow results.

##### Story Acceptance Criteria

- [ ] **Story AC 10.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the release process/socket/path inventory exactly matches observed processes, listeners, connections, and writable roots; every discrepancy blocks release.
- [ ] **Story AC 10.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then acquisition exit returns the system to a freshly proven offline state, and strict-local data roots reject every detected synchronized or remote location.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 10.AC1:** `AT-NET-001` and `AT-NET-002` pass on all reference platforms.
- [ ] **Sprint AC 10.AC2:** Only the kernel's selected local inference path is reachable by its declared client.
- [ ] **Sprint AC 10.AC3:** No normal-operation component attempts an undeclared outbound connection.
- [ ] **Sprint AC 10.AC4:** Strict-local state cannot be placed in a detected cloud-synchronized or remote path.
- [ ] **Sprint AC 10.AC5:** Disabling the network leaves every v0.1 operation functional after installation.

**Gate decision:** Sprint 10 is PASS only when Story 10.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 11 - Encrypted Canonical Operational Store

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-011`.

**Sprint goal:** Make encrypted SQLite the sole crash-safe authority for v0.1 operational state.

**Source coverage:** `AM-DAT-001`, `AM-PRV-001`; PRD Sections 12 and 13; inventory Sections 10, 10E, 21, and 23; `AT-DATA-001`, `AT-PRIV-001`, `AT-PRIV-002`.

**Dependencies:** Sprint 10; legacy dependency record: Sprint 5 (legacy S-005), Sprint 7 (legacy S-007), Sprint 10 (legacy S-010).

#### [ ] Story 11.1 - Encrypted Canonical Operational Store

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need encrypted canonical operational store so that AgentMage delivers the following bounded outcome: Make encrypted SQLite the sole crash-safe authority for v0.1 operational state.

##### Tasks and Sub-tasks

- [ ] **Task 11.1.1 - Implement the bounded story**
  - [ ] **Sub-task 11.1.1.1** (legacy `S-011-I01`): Implement normalized tables and migrations for sessions, objectives, plans, tasks, actions, evidence, decisions, grants, receipts, checkpoints, files, retention, and schema history.
  - [ ] **Sub-task 11.1.1.2** (legacy `S-011-I02`): Enforce one writer, transactions, write-ahead logging, foreign keys, atomic checkpoints, and migration rollback.
  - [ ] **Sub-task 11.1.1.3** (legacy `S-011-I03`): Obtain the database key only through the platform secret-store adapter and fail closed for private persistence when unavailable.
  - [ ] **Sub-task 11.1.1.4** (legacy `S-011-I04`): Implement pre-persistence classification, secret detection, minimization, encryption selection, retention assignment, and receipt generation.
  - [ ] **Sub-task 11.1.1.5** (legacy `S-011-I05`): Keep raw attachments, full tool output, environment variables, prompts, and model responses ephemeral by default.
  - [ ] **Sub-task 11.1.1.6** (legacy `S-011-I06`): Implement expiration, legal or user holds, cryptographic erasure, export boundaries, backup, restore, and corruption recovery.
  - [ ] **Sub-task 11.1.1.7** (legacy `S-011-I07`): Keep JSON Lines derived and export-only; prevent dual-write or startup authority from exports.

- [ ] **Task 11.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 11.1.2.1:** Versioned database schema and migrations.
  - [ ] **Sub-task 11.1.2.2:** Data-classification and retention decision table.
  - [ ] **Sub-task 11.1.2.3:** Secret-store integration and encrypted-backup format.
  - [ ] **Sub-task 11.1.2.4:** Crash-point and secret-canary results.

- [ ] **Task 11.1.3 - Verify and close the story**
  - [ ] **Sub-task 11.1.3.1:** `S-011-UT01` exercises schema constraints, foreign keys, one-writer behavior, retention transitions, migration versions, and invalid records; assert no partial or orphaned state.
  - [ ] **Sub-task 11.1.3.2:** `S-011-ST01` injects unique secrets into every input field and inspects database pages, WAL, temporary files, logs, backups, exports, model context, and crash output; assert only policy-authorized encrypted persistence.
  - [ ] **Sub-task 11.1.3.3:** `S-011-RT01` crashes before and after each transaction, checkpoint, migration, key retrieval, backup, restore, expiry, and deletion transition across at least 100 seeded runs; assert no repeated completed effect.
  - [ ] **Sub-task 11.1.3.4:** `S-011-IT01` disables or substitutes the secret store and cryptographic provider; assert protected persistence fails closed, keys never enter process arguments/environment, and ephemeral behavior follows policy.
  - [ ] **Sub-task 11.1.3.5 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-012`, `SR-OPS-001` through `SR-OPS-005`, `SR-TST-005`, and `RV-08` through `RV-10`; retain schema/migration results, cryptographic inventory, provider evidence, canary scans, crash traces, and sanitization report.

##### Story Acceptance Criteria

- [ ] **Story AC 11.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then sQLite is the only v0.1 operational authority, all durable records satisfy classification/minimization/retention policy before commit, and recovery selects one valid state.
- [ ] **Story AC 11.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then backup, restore, expiry, hold, deletion, and sanitization update every linked table/index/artifact consistently and produce tamper-evident receipts.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 11.AC1:** `AT-DATA-001`, `AT-PRIV-001`, and `AT-PRIV-002` pass.
- [ ] **Sprint AC 11.AC2:** Crash injection creates no orphan grant, receipt, action, evidence, or checkpoint.
- [ ] **Sprint AC 11.AC3:** Private and restricted records never fall back to plaintext.
- [ ] **Sprint AC 11.AC4:** Secret canaries do not appear in SQLite, logs, exports, backups, errors, or unauthorized model context.
- [ ] **Sprint AC 11.AC5:** Deleting JSON Lines exports or regenerable indexes cannot change canonical operational state.

**Gate decision:** Sprint 11 is PASS only when Story 11.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 12 - Agent Runtime, Planning, and Session Behavior

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-012`.

**Sprint goal:** Implement the bounded single-agent control loop and truthful user-visible work behavior.

**Source coverage:** inventory Sections 2, 2A, 2B, 3, and 11; `AM-KRN-001`, `AM-SES-001` foundations.

**Dependencies:** Sprint 11; legacy dependency record: Sprint 4 (legacy S-004), Sprint 5 (legacy S-005), Sprint 11 (legacy S-011).

#### [ ] Story 12.1 - Agent Runtime, Planning, and Session Behavior

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need agent runtime, planning, and session behavior so that AgentMage delivers the following bounded outcome: Implement the bounded single-agent control loop and truthful user-visible work behavior.

##### Tasks and Sub-tasks

- [ ] **Task 12.1.1 - Implement the bounded story**
  - [ ] **Sub-task 12.1.1.1** (legacy `S-012-I01`): Implement `AgentRuntime`, agent configuration, bounded observe-plan-act-review, task plans, run budgets, and stop conditions.
  - [ ] **Sub-task 12.1.1.2** (legacy `S-012-I02`): Implement task-intent, complexity, and risk classification without granting authority.
  - [ ] **Sub-task 12.1.1.3** (legacy `S-012-I03`): Implement one-active-step planning, plan history, progress events, status responses, user-message interruption, cancellation, and final-response contracts.
  - [ ] **Sub-task 12.1.1.4** (legacy `S-012-I04`): Implement problem frames, assumption registers, hypothesis ledgers, contradiction checks, clarification gates, and independent verification passes.
  - [ ] **Sub-task 12.1.1.5** (legacy `S-012-I05`): Implement evidence-required completion rules that reject unsupported claims of reading, changing, testing, committing, pushing, publishing, or completion.
  - [ ] **Sub-task 12.1.1.6** (legacy `S-012-I06`): Capture session environment, active workspace, repository, branch, permission profile, model profile, and attached-file provenance.
  - [ ] **Sub-task 12.1.1.7** (legacy `S-012-I07`): Implement bounded attachment metadata and path resolution while deferring unsupported content parsers to their release packs.

- [ ] **Task 12.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 12.1.2.1:** Single-agent state machine and event schemas.
  - [ ] **Sub-task 12.1.2.2:** Planning, reasoning-mode, and completion-evidence fixtures.
  - [ ] **Sub-task 12.1.2.3:** Interruption, cancellation, status, and final-response transcripts.
  - [ ] **Sub-task 12.1.2.4:** Session environment capture schema.

- [ ] **Task 12.1.3 - Verify and close the story**
  - [ ] **Sub-task 12.1.3.1:** `S-012-UT01` covers every legal and illegal state-machine transition, empty/oversized objective, plan revision, budget boundary, stop condition, and completion claim; assert deterministic typed outcomes.
  - [ ] **Sub-task 12.1.3.2:** `S-012-UT02` supplies false, malformed, contradictory, or authority-seeking model plans and tool calls; assert validation, bounded repair, explicit unknown state, or safe stop without execution.
  - [ ] **Sub-task 12.1.3.3:** `S-012-RT01` interrupts model calls, tools, planning, status rendering, and finalization; assert cancellation reaches descendants, cleanup completes, and the transcript never claims unfinished work succeeded.
  - [ ] **Sub-task 12.1.3.4:** `S-012-IT01` runs fixed tasks through concise/deep modes using fake clock/model/tools; assert identical authority, evidence standards, budgets, status cadence, and reproducible receipts.
  - [ ] **Sub-task 12.1.3.5 - Product security evidence:** Map `SR-ACC-001`, `SR-ACC-007`, `SR-AI-003` through `SR-AI-011`, `SR-OPS-001`, `SR-TST-004` through `SR-TST-006`; retain transition coverage, model-repair traces, cancellation proof, and truthful-completion transcripts.

##### Story Acceptance Criteria

- [ ] **Story AC 12.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the single-agent loop remains within declared turns, tools, resources, context, and stop conditions, and no plan or natural-language text can broaden capability.
- [ ] **Story AC 12.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every final response distinguishes completed, failed, blocked, unknown, and not-run work and resolves material claims to current evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 12.AC1:** Only one plan step can be in progress.
- [ ] **Sprint AC 12.AC2:** Every stop reason is explicit, typed, receipted, and resumable where allowed.
- [ ] **Sprint AC 12.AC3:** New user messages correctly replace, extend, or query status without corrupting the active task.
- [ ] **Sprint AC 12.AC4:** Adversarial prompts cannot make the runtime claim unperformed work.
- [ ] **Sprint AC 12.AC5:** Bounded loops terminate on completion, denial, failure, exhaustion, clarification, or cancellation.

**Gate decision:** Sprint 12 is PASS only when Story 12.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 13 - Model Manifest and Runtime Contract

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-013`.

**Sprint goal:** Load the one approved Gemma 4 E4B profile through interchangeable, verified local runtime adapters.

**Source coverage:** `AM-MDL-001`, `CR-P0-MDL`; PRD Section 8; inventory Sections 1 and 1A; `AT-MODEL-001`.

**Dependencies:** Sprint 12; legacy dependency record: Sprint 7 (legacy S-007), Sprint 8 (legacy S-008), Sprint 9 (legacy S-009), Sprint 10 (legacy S-010).

#### [ ] Story 13.1 - Model Manifest and Runtime Contract

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need model manifest and runtime contract so that AgentMage delivers the following bounded outcome: Load the one approved Gemma 4 E4B profile through interchangeable, verified local runtime adapters.

##### Tasks and Sub-tasks

- [ ] **Task 13.1.1 - Implement the bounded story**
  - [ ] **Sub-task 13.1.1.1** (legacy `S-013-I01`): Implement `LocalModelRuntime` load, unload, health, token count, streaming, cancellation, resource reporting, manifest verification, and zero-network contracts.
  - [ ] **Sub-task 13.1.1.2** (legacy `S-013-I02`): Define the approved Gemma 4 E4B manifest under `MODEL-PROVENANCE-POLICY.md` with identity, publisher/control, lineage, Apache-2.0 disposition, quantization, conversion, tokenizer, GGUF hashes, immutable OCI digests where applicable, runtime compatibility, context ceiling, and resource expectations.
  - [ ] **Sub-task 13.1.1.3** (legacy `S-013-I03`): Implement the signed native `llama.cpp` Metal adapter for the Mac reference path.
  - [ ] **Sub-task 13.1.1.4** (legacy `S-013-I04`): Implement native `llama.cpp` as the approved Linux reference adapter and Docker Model Runner as a separately gated compatibility adapter behind the same contract.
  - [ ] **Sub-task 13.1.1.5** (legacy `S-013-I05`): Implement provider-neutral model client, response, message, tool-call, tool-result, capability, and role schemas.
  - [ ] **Sub-task 13.1.1.6** (legacy `S-013-I06`): Implement model health, structured-output validation, plain-text fallback, bounded retry, cancellation, and malformed-response reporting.
  - [ ] **Sub-task 13.1.1.7** (legacy `S-013-I07`): Prohibit unapproved model families, changed manifests, cloud fallback, arbitrary endpoints, and automatic model switching.

- [ ] **Task 13.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 13.1.2.1:** Approved Gemma 4 E4B manifest and artifact catalog entry.
  - [ ] **Sub-task 13.1.2.2:** macOS and Linux runtime adapter implementations.
  - [ ] **Sub-task 13.1.2.3:** Runtime conformance and offline response reports.
  - [ ] **Sub-task 13.1.2.4:** Model capability and known-limitation record.

- [ ] **Task 13.1.3 - Verify and close the story**
  - [ ] **Sub-task 13.1.3.1:** `S-013-UT01` validates manifest fields, model/tokenizer/template/runtime hashes, architecture, quantization, license, conversion recipe, limits, and platform compatibility; assert any mismatch quarantines the profile.
  - [ ] **Sub-task 13.1.3.2:** `S-013-UT02` runs identical protocol vectors against fake, macOS, and Linux runtime adapters for valid, malformed, oversized, cancelled, timed-out, and resource-exhausted calls; assert typed parity.
  - [ ] **Sub-task 13.1.3.3:** `S-013-ST01` attempts runtime access to files, tools, grants, credentials, environment, unrelated sockets, and raw workspace content; assert zero authority and bounded process termination.
  - [ ] **Sub-task 13.1.3.4:** `S-013-AT01` executes the pinned factual/coding/tool-call evaluation corpus repeatedly with fixed decoding; assert reported schema validity, grounding, uncertainty, reproducibility, and negative results meet declared thresholds.
  - [ ] **Sub-task 13.1.3.5 - Product security evidence:** Map `SR-PLT-007`, `SR-SUP-006` through `SR-SUP-009`, `SR-AI-001` through `SR-AI-014`, and `RV-13`/`RV-14`; retain Model BOM, licenses, hashes, conversion provenance, adapter traces, evaluations, and independent parser review.

##### Story Acceptance Criteria

- [ ] **Story AC 13.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then only the exact approved Gemma 4 E4B artifact set loads; silent changes to any model/runtime component trigger a new manifest and security-impact review.
- [ ] **Story AC 13.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then runtime adapters are interchangeable at the kernel contract while platform, performance, quality, and limitation evidence remains separately attributable.

#### [ ] Story 13.2 - Cross-Adapter Model Parity and Fallback Gate

**User-facing value:** As a user, I need the selected Gemma profile to behave predictably across native macOS, native Linux, and Docker Linux paths, with a visible stop instead of a silent model or runtime substitution.

##### Tasks and Sub-tasks

- [ ] **Task 13.2.1 - Establish parity and admission gates**
  - [ ] **Sub-task 13.2.1.1:** Use the versioned Story 0.3 corpus and explicit context/output/resource settings for every enabled adapter; record adapter-specific prompt template or tool-call transformations without changing the shared contract.
  - [ ] **Sub-task 13.2.1.2:** Compare schema validity, tool-call recovery, grounding, citations, uncertainty, cancellation latency, context behavior, output limits, memory, throughput, and repeated-run variance against one published threshold set.
  - [ ] **Sub-task 13.2.1.3:** Quarantine an adapter that fails identity, isolation, contract, quality, or resource thresholds while leaving other approved adapters and the user's selected profile unchanged.
  - [ ] **Sub-task 13.2.1.4:** Keep Gemma 4 12B Unified disabled unless E4B is formally rejected and the fallback independently passes the full admission, runtime, hardware-fit, and evaluation gate through a recorded decision.

- [ ] **Task 13.2.2 - Verify and close the story**
  - [ ] **Sub-task 13.2.2.1:** Run matched native/Docker corpus trials with one changed model hash, image digest, template, context setting, decoding setting, and runtime build at a time; assert incomparable or unapproved results cannot be merged or enabled.
  - [ ] **Sub-task 13.2.2.2:** Force each adapter below every threshold and make E4B unavailable; assert a visible blocked result with no automatic adapter, model, frontier, or cloud fallback.
  - [ ] **Sub-task 13.2.2.3 - Product security evidence:** Complete `RV-13` and the first full `RV-14`; map `SR-SUP-006` through `SR-SUP-008`, `SR-AI-006`/`SR-AI-010` through `SR-AI-014`, and `SR-TST-006`; retain matched manifests, raw corpus results, parity calculations, quarantine receipts, negative results, and fallback decision state.

##### Story Acceptance Criteria

- [ ] **Story AC 13.2.AC1:** Given the same approved model profile and fixed corpus, when each adapter runs, then all meet the same blocking contract and quality thresholds while measurable platform differences remain separately attributable.
- [ ] **Story AC 13.2.AC2:** Given any adapter, model, manifest, or threshold failure, when selection is attempted, then AgentMage stops visibly and preserves the current task without automatically selecting another local or remote model.
- [ ] **Story AC 13.2.AC3:** Given a proposed fallback enablement, when reviewers inspect it, then a separate complete admission record, platform results, hardware-fit evidence, and accepted decision exist before the profile is selectable.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 13.AC1:** `AT-MODEL-001` passes on all reference platforms.
- [ ] **Sprint AC 13.AC2:** Every response identifies the selected local model and verified runtime.
- [ ] **Sprint AC 13.AC3:** Manifest, tokenizer, artifact, runtime, lineage, or license mismatch prevents load.
- [ ] **Sprint AC 13.AC4:** Cancellation unloads or stops work cleanly without corrupting session state.
- [ ] **Sprint AC 13.AC5:** No adapter gives the model tools, grants, workspace access, credentials, or network authority.

**Gate decision:** Sprint 13 is PASS only when Stories 13.1 and 13.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 14 - Separate Model Installer and Importer

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-014`.

**Sprint goal:** Acquire or import approved model artifacts safely without giving acquisition code session or workspace authority.

**Source coverage:** `AM-MDL-003`; inventory Sections 1, 1A, 5A, and 32; `AT-MODEL-002`.

**Dependencies:** Sprint 13; legacy dependency record: Sprint 10 (legacy S-010), Sprint 13 (legacy S-013).

#### [ ] Story 14.1 - Separate Model Installer and Importer

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need separate model installer and importer so that AgentMage delivers the following bounded outcome: Acquire or import approved model artifacts safely without giving acquisition code session or workspace authority.

##### Tasks and Sub-tasks

- [ ] **Task 14.1.1 - Implement the bounded story**
  - [ ] **Sub-task 14.1.1.1** (legacy `S-014-I01`): Implement hardware, architecture, acceleration, memory, disk, context, artifact-size, and expected-working-set preflight.
  - [ ] **Sub-task 14.1.1.2** (legacy `S-014-I02`): Display exact model identity, publisher, lineage, license, quantization, runtime requirements, and hashes before acquisition.
  - [ ] **Sub-task 14.1.1.3** (legacy `S-014-I03`): Implement user-selected local import and separately authorized bounded download.
  - [ ] **Sub-task 14.1.1.4** (legacy `S-014-I04`): Implement resumable staging, bounded retry, partial cleanup, cancellation, hash-failure quarantine, and incompatible-artifact refusal.
  - [ ] **Sub-task 14.1.1.5** (legacy `S-014-I05`): Implement atomic activation, install self-test, safe load and unload, previous-version preservation, rollback, and orphan cleanup.
  - [ ] **Sub-task 14.1.1.6** (legacy `S-014-I06`): Ensure the installer receives no workspace handle, session database, inference grant, tool authority, or unrelated secret.
  - [ ] **Sub-task 14.1.1.7** (legacy `S-014-I07`): Prove the installer exits before strict-local normal operation begins.

- [ ] **Task 14.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 14.1.2.1:** Signed installer/importer packages.
  - [ ] **Sub-task 14.1.2.2:** Preflight and license-review screens.
  - [ ] **Sub-task 14.1.2.3:** Staging, quarantine, activation, rollback, and cleanup receipts.
  - [ ] **Sub-task 14.1.2.4:** Complete lifecycle fixture results.

- [ ] **Task 14.1.3 - Verify and close the story**
  - [ ] **Sub-task 14.1.3.1:** `S-014-UT01` verifies preflight decisions for source host, expected identity/size/license/hash/destination, disk, architecture, and compatibility; assert ambiguous or unapproved input cannot begin acquisition.
  - [ ] **Sub-task 14.1.3.2:** `S-014-ST01` supplies redirected, truncated, oversized, substituted, malicious, wrong-license, wrong-hash, wrong-model, and executable-bearing artifacts; assert quarantine and zero activation.
  - [ ] **Sub-task 14.1.3.3:** `S-014-RT01` cancels or crashes at every download/import/stage/verify/activate/rollback/cleanup transition; assert closed sockets, bounded residue, and either prior or fully verified active model.
  - [ ] **Sub-task 14.1.3.4:** `S-014-IT01` runs installer and operational host concurrently and probes installer for workspace/session/tool/grant/inference authority; assert mutual exclusion and zero cross-boundary data access.
  - [ ] **Sub-task 14.1.3.5 - Product security evidence:** Map `SR-PLT-008`, `SR-NET-005` through `SR-NET-007`, `SR-SUP-003`, `SR-SUP-006` through `SR-SUP-008`, `SR-TST-007`, and `RV-07`; retain preflight views, network capture, quarantine results, lifecycle traces, and cleanup scan.

##### Story Acceptance Criteria

- [ ] **Story AC 14.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no artifact becomes selectable before license acceptance, complete hash/manifest verification, malware policy checks, compatibility checks, and atomic activation.
- [ ] **Story AC 14.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every successful, cancelled, failed, and corrupt acquisition ends with an inspectable receipt and a fresh offline preflight before normal operation.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 14.AC1:** `AT-MODEL-002` passes on all reference platforms.
- [ ] **Sprint AC 14.AC2:** Every corrupt, incomplete, cancelled, mismatched, incompatible, or under-resourced artifact remains unrunnable.
- [ ] **Sprint AC 14.AC3:** Every valid artifact activates atomically and passes self-test.
- [ ] **Sprint AC 14.AC4:** Failed acquisition leaves the previous active model intact.
- [ ] **Sprint AC 14.AC5:** Offline startup proves no installer process or acquisition network authority remains.

**Gate decision:** Sprint 14 is PASS only when Story 14.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 15 - Diagnostics, Manual Model Selection, and Resource Control

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-015`.

**Sprint goal:** Make the active local boundary inspectable and keep model selection explicit and resource-bounded.

**Source coverage:** `AM-MDL-002`, `AM-DIA-001`, `CR-P0-DIA`; inventory Sections 1A, 1B, 23, and 31A; `AT-DIA-001`, `AT-ROUTE-001`.

**Dependencies:** Sprint 14; legacy dependency record: Sprint 11 (legacy S-011), Sprint 13 (legacy S-013), Sprint 14 (legacy S-014).

#### [ ] Story 15.1 - Diagnostics, Manual Model Selection, and Resource Control

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need diagnostics, manual model selection, and resource control so that AgentMage delivers the following bounded outcome: Make the active local boundary inspectable and keep model selection explicit and resource-bounded.

##### Tasks and Sub-tasks

- [ ] **Task 15.1.1 - Implement the bounded story**
  - [ ] **Sub-task 15.1.1.1** (legacy `S-015-I01`): Implement one typed redacted doctor-data provider for model, runtime, hardware fit, offline state, sandbox, helper, workspace grant, capability versions, repository-map health, encrypted store, receipt sequence, and recovery.
  - [ ] **Sub-task 15.1.1.2** (legacy `S-015-I02`): Implement deterministic-first task dispatch and explicit user model selection.
  - [ ] **Sub-task 15.1.1.3** (legacy `S-015-I03`): Record task class, deterministic operations, model choice, validation outcome, latency, resources, and acceptance result without changing routing.
  - [ ] **Sub-task 15.1.1.4** (legacy `S-015-I04`): Implement visible model identity, digest, runtime, context, tool limits, vision limits, and resource status in every session.
  - [ ] **Sub-task 15.1.1.5** (legacy `S-015-I05`): Implement safe load and unload with one large active model by default and bounded resource-pressure handling.
  - [ ] **Sub-task 15.1.1.6** (legacy `S-015-I06`): Redact secrets, prompts, private excerpts, environment values, and unrelated absolute paths from diagnostics and exports.
  - [ ] **Sub-task 15.1.1.7** (legacy `S-015-I07`): Record later-profile and router data without enabling automatic fallback, ensembles, or frontier transfer.

- [ ] **Task 15.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 15.1.2.1:** Local and exportable diagnostics report.
  - [ ] **Sub-task 15.1.2.2:** Manual selection and deterministic-first routing receipts.
  - [ ] **Sub-task 15.1.2.3:** Resource-monitor and remediation fixtures.
  - [ ] **Sub-task 15.1.2.4:** Benchmark-record schema.

- [ ] **Task 15.1.3 - Verify and close the story**
  - [ ] **Sub-task 15.1.3.1:** `S-015-UT01` renders diagnostics from complete, partial, missing, corrupt, stale, and unsupported manifests; assert precise available/degraded/blocked states with no secret or private-content disclosure.
  - [ ] **Sub-task 15.1.3.2:** `S-015-UT02` tests manual model selection, deterministic-first routing, and attempted automatic substitution; assert only the selected verified profile runs and every change is receipted.
  - [ ] **Sub-task 15.1.3.3:** `S-015-ST01` exceeds memory, CPU/GPU time, context, output, concurrency, disk, and process limits independently and together; assert cancellation, child cleanup, responsive UI, and no authority change.
  - [ ] **Sub-task 15.1.3.4:** `S-015-IT01` reruns pinned benchmarks with identical and changed hardware/runtime/configuration identities; assert comparable runs are reproducible and incomparable runs are labeled rather than merged.
  - [ ] **Sub-task 15.1.3.5 - Product security evidence:** Map `SR-GOV-004`, `SR-PLT-010`, `SR-AI-006`, `SR-AI-009`, `SR-AI-013`, `SR-OPS-009`, `SR-TST-006`, and `RV-13`/`RV-16`; retain diagnostics, selection receipts, limit traces, benchmark manifests, and redaction scan.

##### Story Acceptance Criteria

- [ ] **Story AC 15.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a reviewer can determine exact package, platform, model, runtime, policy, capability, sandbox, storage, and offline status from one redacted local report.
- [ ] **Story AC 15.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then resource pressure degrades or stops only the affected operation; it cannot trigger cloud fallback, model switching, unbounded retries, or hidden capability changes.

#### [ ] Story 15.2 - Native Chat Doctor and Safe Diagnostic Export

**User-facing value:** As a user, I need to diagnose AgentMage from the same native Visual Studio Code Chat surface I already use, while producing a deliberately reviewed export only when support evidence is needed.

##### Tasks and Sub-tasks

- [ ] **Task 15.2.1 - Implement the diagnostic surfaces**
  - [ ] **Sub-task 15.2.1.1:** Add a native Chat request that renders the typed doctor result with healthy, degraded, blocked, unavailable, quarantined, and unsupported states plus local remediation.
  - [ ] **Sub-task 15.2.1.2:** Keep the command-line diagnostic harness non-user-facing in v0.1 and constrain it to automated/reviewer testing through the same typed provider; do not create a second chat interface.
  - [ ] **Sub-task 15.2.1.3:** Add an explicit local preview before writing a diagnostic export, showing included fields, redactions, sensitivity, destination, hash, and retention; require a one-use grant for the export write.
  - [ ] **Sub-task 15.2.1.4:** Ensure diagnostics never test health by contacting the internet and never include raw prompts, file contents, credentials, keys, environment values, unrelated paths, hostnames, usernames, or stable device identifiers.

- [ ] **Task 15.2.2 - Verify and close the story**
  - [ ] **Sub-task 15.2.2.1:** Render every diagnostic state in native Chat using keyboard-only and screen-reader navigation; assert stable status names, actionable remediation, cancellation, and no layout-dependent meaning.
  - [ ] **Sub-task 15.2.2.2:** Inject unique canaries into every prohibited source and compare Chat, harness, logs, receipts, and exported diagnostics; assert zero canary disclosure and identical non-sensitive status semantics.
  - [ ] **Sub-task 15.2.2.3:** Attempt export without approval, to an ungranted/cloud-synchronized path, with stale preview, after cancellation, and after a crash; assert no unauthorized or partial artifact remains.
  - [ ] **Sub-task 15.2.2.4 - Product security evidence:** Extend `RV-08`, `RV-16`, `RV-18`, and `RV-20`; map `SR-GOV-001`, `SR-DAT-002`/`SR-DAT-003`, `SR-OPS-003`, `SR-CIV-006` through `SR-CIV-009`; retain state fixtures, Chat snapshots/accessibility output, canary scans, export previews, and cleanup results.

##### Story Acceptance Criteria

- [ ] **Story AC 15.2.AC1:** Given any supported or failed local state, when the user asks for diagnostics in native Chat, then the response accurately names the state and remediation without requiring a terminal or network connection.
- [ ] **Story AC 15.2.AC2:** Given prohibited data in every potential source, when Chat, internal harness, logs, and export paths are exercised, then none of that data appears and equivalent non-sensitive results reconcile.
- [ ] **Story AC 15.2.AC3:** Given an export request, when preview, grant, destination, or lifecycle validation fails, then no diagnostic file is written or retained.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 15.AC1:** `AT-DIA-001` and `AT-ROUTE-001` pass.
- [ ] **Sprint AC 15.AC2:** Diagnostics accurately distinguish healthy, degraded, missing, mismatched, quarantined, offline, and unrecoverable states.
- [ ] **Sprint AC 15.AC3:** No diagnostic output contains a secret or unrelated private path.
- [ ] **Sprint AC 15.AC4:** Applicable deterministic operations always precede model inference.
- [ ] **Sprint AC 15.AC5:** No task causes an automatic model switch, external call, or frontier transfer.

**Gate decision:** Sprint 15 is PASS only when Stories 15.1 and 15.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 16 - Sandboxed Read-Only Tool Protocol

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-016`.

**Sprint goal:** Execute bounded deterministic file operations through one receipted kernel tool path inside each platform sandbox.

**Source coverage:** `AM-SEC-002`, `AM-TOL-001`; inventory Sections 4, 5, 7, and 12 read-only subset; `AT-TOOL-001`, `AT-SBX-001`.

**Dependencies:** Sprint 15; legacy dependency record: Sprint 5 (legacy S-005), Sprint 6 (legacy S-006), Sprint 8 (legacy S-008), Sprint 9 (legacy S-009), Sprint 11 (legacy S-011).

#### [ ] Story 16.1 - Sandboxed Read-Only Tool Protocol

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need sandboxed read-only tool protocol so that AgentMage delivers the following bounded outcome: Execute bounded deterministic file operations through one receipted kernel tool path inside each platform sandbox.

##### Tasks and Sub-tasks

- [ ] **Task 16.1.1 - Implement the bounded story**
  - [ ] **Sub-task 16.1.1.1** (legacy `S-016-I01`): Implement tool definition, registry, discovery, schema validation, grant consumption, bounded execution, result envelope, redaction, cancellation, and receipt emission.
  - [ ] **Sub-task 16.1.1.2** (legacy `S-016-I02`): Implement bounded directory listing, directory tree, text read, multi-file read, filename search, text search, metadata, file hash, tree hash, and binary metadata tools.
  - [ ] **Sub-task 16.1.1.3** (legacy `S-016-I03`): Enforce file-count, byte-count, depth, match-count, encoding, output, and call-depth limits.
  - [ ] **Sub-task 16.1.1.4** (legacy `S-016-I04`): Implement repeated-call protection and explicit no-result, denied, partial, truncated, malformed, cancelled, and failed states.
  - [ ] **Sub-task 16.1.1.5** (legacy `S-016-I05`): Run every production operation in the declared macOS XPC or Linux Bubblewrap worker.
  - [ ] **Sub-task 16.1.1.6** (legacy `S-016-I06`): Preserve workspace and metadata and prohibit every write-capable operation in the v0.1 tool registry.
  - [ ] **Sub-task 16.1.1.7** (legacy `S-016-I07`): Produce one receipt per attempt and redact tool output before model context or logging.

- [ ] **Task 16.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 16.1.2.1:** Core Read-Only tool pack.
  - [ ] **Sub-task 16.1.2.2:** Tool schema catalog and limit table.
  - [ ] **Sub-task 16.1.2.3:** Sandbox worker execution receipts.
  - [ ] **Sub-task 16.1.2.4:** Golden and adversarial tool results.

- [ ] **Task 16.1.3 - Verify and close the story**
  - [ ] **Sub-task 16.1.3.1:** `S-016-UT01` validates every tool schema with valid, missing, extra, malformed, oversized, duplicate, unsupported-version, and out-of-budget arguments; assert no dispatch on invalid input.
  - [ ] **Sub-task 16.1.3.2:** `S-016-UT02` checks byte/range/hash/encoding results for list, metadata, read, exact search, and hash tools against golden fixtures; assert deterministic ordering, truncation, and evidence ranges.
  - [ ] **Sub-task 16.1.3.3:** `S-016-ST01` runs path escape, symlink/race, special file, archive bomb, device, socket, environment, network, process, write, and secret-canary attacks inside each platform worker; assert zero prohibited effect.
  - [ ] **Sub-task 16.1.3.4:** `S-016-RT01` cancels, times out, kills, and crashes workers before/during/after result production; assert descendant cleanup, bounded scratch deletion, one terminal receipt, and no false completion.
  - [ ] **Sub-task 16.1.3.5 - Product security evidence:** Map `SR-PLT-003`/`SR-PLT-004`, `SR-ACC-001` through `SR-ACC-006`, `SR-AI-005`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`, and `RV-03`/`RV-04`; retain schemas, golden diffs, sandbox traces, attack corpus, cleanup proof, and independent worker review.

##### Story Acceptance Criteria

- [ ] **Story AC 16.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then each call receives exactly one consumed operation grant, one bounded input scope, one isolated worker, one typed result, and one schema-valid receipt.
- [ ] **Story AC 16.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the complete tool pack remains demonstrably read-only: fixture trees, metadata, Git state, external canaries, network state, and durable operational data are invariant except authorized receipts.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 16.AC1:** `AT-TOOL-001` and tool portions of `AT-SBX-001` pass.
- [ ] **Sprint AC 16.AC2:** Golden list, read, search, metadata, and hash results are exact.
- [ ] **Sprint AC 16.AC3:** Every configured limit fails closed with one receipt.
- [ ] **Sprint AC 16.AC4:** Workspace tree and metadata remain unchanged after every operation.
- [ ] **Sprint AC 16.AC5:** No tool can escape the current grant, sandbox, workspace, or offline boundary.

**Gate decision:** Sprint 16 is PASS only when Story 16.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 17 - Read-Only Git and Untrusted Instructions

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-017`.

**Sprint goal:** Add deterministic Git evidence while ensuring all repository text remains non-authoritative.

**Source coverage:** `AM-GIT-001`, `AM-INS-001`, `CR-P0-TRUST`; inventory Sections 6 and 13; `AT-GIT-001`, `AT-INJ-001`, `AT-INS-001`.

**Dependencies:** Sprint 16; legacy dependency record: Sprint 16 (legacy S-016).

#### [ ] Story 17.1 - Read-Only Git and Untrusted Instructions

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need read-only git and untrusted instructions so that AgentMage delivers the following bounded outcome: Add deterministic Git evidence while ensuring all repository text remains non-authoritative.

##### Tasks and Sub-tasks

- [ ] **Task 17.1.1 - Implement the bounded story**
  - [ ] **Sub-task 17.1.1.1** (legacy `S-017-I01`): Implement read-only status, branch, upstream, branch list, log, diff, staged diff, show, worktree list, object, ref, dirty-tree, and untracked-file inspection.
  - [ ] **Sub-task 17.1.1.2** (legacy `S-017-I02`): Enforce bounded counts, bytes, object types, and parser errors through the sandboxed tool protocol.
  - [ ] **Sub-task 17.1.1.3** (legacy `S-017-I03`): Implement workspace, repository, project-document, and hierarchical instruction discovery.
  - [ ] **Sub-task 17.1.1.4** (legacy `S-017-I04`): Treat every repository instruction, comment, issue, generated file, tool result, and document as untrusted data by default.
  - [ ] **Sub-task 17.1.1.5** (legacy `S-017-I05`): Implement optional non-authority guidance trust with source, hash, scope, precedence, conflict, and user decision.
  - [ ] **Sub-task 17.1.1.6** (legacy `S-017-I06`): Record files discovered versus files actually read and warn when evidence or instructions become stale.
  - [ ] **Sub-task 17.1.1.7** (legacy `S-017-I07`): Prohibit Git mutation, hooks, arbitrary repository commands, executable configuration, and automatic package installation.

- [ ] **Task 17.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 17.1.2.1:** Read-only Git tool pack.
  - [ ] **Sub-task 17.1.2.2:** Instruction provenance and trust-decision records.
  - [ ] **Sub-task 17.1.2.3:** Git invariance and injection test corpus.
  - [ ] **Sub-task 17.1.2.4:** Workspace manifest and evidence ledger.

- [ ] **Task 17.1.3 - Verify and close the story**
  - [ ] **Sub-task 17.1.3.1:** `S-017-UT01` compares status, diff, log, show, branch, tag, ignore, and metadata results with pinned Git fixture expectations; assert bounded output and no repository mutation.
  - [ ] **Sub-task 17.1.3.2:** `S-017-ST01` places instructions in filenames, source, comments, docs, diffs, commits, branches, tags, submodules, hooks, attributes, config, and model output; assert all remain cited untrusted data.
  - [ ] **Sub-task 17.1.3.3:** `S-017-ST02` seeds hooks, filters, pagers, aliases, credential helpers, unsafe directories, replacement objects, and remote URLs; assert no execution, credential access, or network contact.
  - [ ] **Sub-task 17.1.3.4:** `S-017-IT01` snapshots every byte and relevant metadata before and after full Git inspection; assert invariance except separately authorized operational receipts.
  - [ ] **Sub-task 17.1.3.5 - Product security evidence:** Map `SR-ACC-006` through `SR-ACC-008`, `SR-AI-005`, `SR-AI-008`, `SR-NET-001`, `SR-TST-002`/`SR-TST-004`; retain injection matrix, environment hardening results, before/after hashes, and evidence ledger.

##### Story Acceptance Criteria

- [ ] **Story AC 17.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then hosted or local repository content cannot change policy, tools, grants, trusted instructions, completion criteria, or evidence state.
- [ ] **Story AC 17.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every Git fact identifies repository, worktree, revision, command-equivalent operation, path/range where applicable, truncation, and freshness.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 17.AC1:** `AT-GIT-001`, `AT-INJ-001`, and `AT-INS-001` pass.
- [ ] **Sprint AC 17.AC2:** Clean, dirty, detached, untracked, renamed, and malformed fixture repositories return exact expected evidence.
- [ ] **Sprint AC 17.AC3:** Workspace tree, index, refs, and object set are identical before and after inspection.
- [ ] **Sprint AC 17.AC4:** No workspace content can alter policy, grant authority, root scope, tool availability, current user intent, transfer policy, or completion status.
- [ ] **Sprint AC 17.AC5:** Trusted guidance can narrow behavior but can never grant authority.

**Gate decision:** Sprint 17 is PASS only when Story 17.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 18 - Pinned Repository Structure

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-018`, part 1 of 2.

**Sprint goal:** Deliver pinned repository structure as a bounded part of the legacy goal: Build a bounded, Git-aware structural map with exact parser evidence and visible coverage limits.

**Source coverage:** `AM-REP-001`, `CR-P0-REP`; inventory Section 7A; `AT-REP-001`.

**Dependencies:** Sprint 17; legacy dependency record: Sprint 11 (legacy S-011), Sprint 16 (legacy S-016), Sprint 17 (legacy S-017).

#### [ ] Story 18.1 - Pinned Repository Structure

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need pinned repository structure so that AgentMage delivers the following bounded outcome: Build a bounded, Git-aware structural map with exact parser evidence and visible coverage limits.

##### Tasks and Sub-tasks

- [ ] **Task 18.1.1 - Implement the bounded story**
  - [ ] **Sub-task 18.1.1.1** (legacy `S-018-I01`): Declare and pin the v0.1 Tree-sitter language and grammar set.
  - [ ] **Sub-task 18.1.1.2** (legacy `S-018-I02`): Build policy-aware and `.gitignore`-aware inventory with tracked state, language, type, size, content hash, branch, and commit identity.
  - [ ] **Sub-task 18.1.1.3** (legacy `S-018-I03`): Extract parser-backed modules, symbols, definitions, imports, and only reliable relationship edges.
  - [ ] **Sub-task 18.1.1.4** (legacy `S-018-I04`): Store an incremental cache keyed by workspace, path, content hash, Git identity, grammar, parser, and policy versions.
  - [ ] **Sub-task 18.1.1.5** (legacy `S-018-I05`): Invalidate changed records before retrieval or citation.

- [ ] **Task 18.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 18.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 18.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 18.1.3 - Verify and close the story**
  - [ ] **Sub-task 18.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 18.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 18.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 18.1.3.4 - Product security evidence:** Map `SR-ACC-008`, `SR-AI-003`, `SR-AI-007`, `SR-AI-010`, `SR-TST-002`, `SR-TST-004`, and `SR-OPS-001`; retain parser BOM, map hashes, invalidation traces, coverage ledger, and source-resolution results.

##### Story Acceptance Criteria

- [ ] **Story AC 18.1.AC1:** Given the approved dependencies and source requirements for `S-018-I01`, `S-018-I02`, `S-018-I03`, `S-018-I04`, and `S-018-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 18.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-018-I01`, `S-018-I02`, `S-018-I03`, `S-018-I04`, and `S-018-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 18.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 18.AC1:** Every numbered implementation sub-task in Story 18.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 18.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 18.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 18.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 18.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 18 is PASS only when Story 18.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 19 - Repository Map Coverage and Source Resolution

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-018`, part 2 of 2.

**Sprint goal:** Deliver repository map coverage and source resolution as a bounded part of the legacy goal: Build a bounded, Git-aware structural map with exact parser evidence and visible coverage limits.

**Source coverage:** `AM-REP-001`, `CR-P0-REP`; inventory Section 7A; `AT-REP-001`.

**Dependencies:** Sprint 18; legacy dependency record: Sprint 11 (legacy S-011), Sprint 16 (legacy S-016), Sprint 17 (legacy S-017).

#### [ ] Story 19.1 - Repository Map Coverage and Source Resolution

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need repository map coverage and source resolution so that AgentMage delivers the following bounded outcome: Build a bounded, Git-aware structural map with exact parser evidence and visible coverage limits.

##### Tasks and Sub-tasks

- [ ] **Task 19.1.1 - Implement the bounded story**
  - [ ] **Sub-task 19.1.1.1** (legacy `S-018-I06`): Produce coverage counts for discovered, parsed, searched, skipped, excluded, unsupported, failed, and truncated files and budgets.
  - [ ] **Sub-task 19.1.1.2** (legacy `S-018-I07`): Render token-bounded maps that prioritize named targets, entry points, direct neighborhoods, tests, and configuration.
  - [ ] **Sub-task 19.1.1.3** (legacy `S-018-I08`): Resolve every structural record to workspace-relative path, content hash, source range, parser identity, and Git identity.
  - [ ] **Sub-task 19.1.1.4** (legacy `S-018-I09`): Fall back to inventory and lexical search for unsupported languages with an explicit Unknown/Blocked limitation.

- [ ] **Task 19.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 19.1.2.1:** Repository-map schema, cache, renderer, and coverage report.
  - [ ] **Sub-task 19.1.2.2:** Pinned parser and grammar manifest.
  - [ ] **Sub-task 19.1.2.3:** Golden map hashes and source-resolution fixtures.
  - [ ] **Sub-task 19.1.2.4:** Read-only invariance report.

- [ ] **Task 19.1.3 - Verify and close the story**
  - [ ] **Sub-task 19.1.3.1:** `S-018-UT01` maps empty, small, nested, ignored, generated, vendored, binary, unsupported-language, malformed, and limit-exceeding repositories; assert deterministic nodes, edges, ordering, and visible omissions.
  - [ ] **Sub-task 19.1.3.2:** `S-018-UT02` changes one file, parser version, grammar hash, ignore rule, revision, and configuration at a time; assert only correctly dependent cache entries invalidate.
  - [ ] **Sub-task 19.1.3.3:** `S-018-ST01` supplies parser crashes, hostile encodings, enormous files, recursive links, name collisions, and source injections; assert bounded fallback, no execution, and `Unknown/Blocked` rather than invented structure.
  - [ ] **Sub-task 19.1.3.4:** `S-018-IT01` resolves every rendered architecture claim back to exact revision/file/range/parser evidence and compares pre/post workspace hashes; assert complete citations and read-only invariance.
  - [ ] **Sub-task 19.1.3.5 - Product security evidence:** Map `SR-ACC-008`, `SR-AI-003`, `SR-AI-007`, `SR-AI-010`, `SR-TST-002`, `SR-TST-004`, and `SR-OPS-001`; retain parser BOM, map hashes, invalidation traces, coverage ledger, and source-resolution results.

##### Story Acceptance Criteria

- [ ] **Story AC 19.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then two runs over identical repository identity produce byte-identical maps, while any relevant source or parser change invalidates stale evidence.
- [ ] **Story AC 19.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then coverage reports quantify scanned, parsed, unsupported, ignored, truncated, failed, and uncertain content; no unsupported area is silently presented as understood.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 19.AC1:** `AT-REP-001` passes exactly.
- [ ] **Sprint AC 19.AC2:** Repeated unchanged runs are byte-identical.
- [ ] **Sprint AC 19.AC3:** Changed files, Git identity, parser, grammar, or policy invalidate only affected records before citation.
- [ ] **Sprint AC 19.AC4:** Unsupported or failed relationships are omitted or visibly Unknown/Blocked, never fabricated.
- [ ] **Sprint AC 19.AC5:** Mapping leaves workspace files, metadata, Git index, refs, objects, and instructions unchanged.

**Gate decision:** Sprint 19 is PASS only when Story 19.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 20 - Evidence-State Assignment

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-019`, part 1 of 2.

**Sprint goal:** Deliver evidence-state assignment as a bounded part of the legacy goal: Ground every material claim in an explicit evidence state and resolvable provenance.

**Source coverage:** `AM-EVD-001`, `AM-EVD-002`, `CR-P0-EVD`; PRD Section 16; inventory Sections 9, 22, and 23; `AT-EVD-001`, `AT-EVD-002`, `AT-EVD-003`.

**Dependencies:** Sprint 19; legacy dependency record: Sprint 11 (legacy S-011), Sprint 16 (legacy S-016), Sprint 19 (legacy S-018).

#### [ ] Story 20.1 - Evidence-State Assignment

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need evidence-state assignment so that AgentMage delivers the following bounded outcome: Ground every material claim in an explicit evidence state and resolvable provenance.

##### Tasks and Sub-tasks

- [ ] **Task 20.1.1 - Implement the bounded story**
  - [ ] **Sub-task 20.1.1.1** (legacy `S-019-I01`): Implement exactly four material-claim states: Observed, Derived, Inferred, and Unknown/Blocked.
  - [ ] **Sub-task 20.1.1.2** (legacy `S-019-I02`): Bind Observed claims to authorized deterministic tool receipts and exact source identities.
  - [ ] **Sub-task 20.1.1.3** (legacy `S-019-I03`): Bind Derived claims to a versioned deterministic method and observed input identifiers.
  - [ ] **Sub-task 20.1.1.4** (legacy `S-019-I04`): Bind Inferred claims to supporting citations and the exact model/runtime manifest without implying proof.
  - [ ] **Sub-task 20.1.1.5** (legacy `S-019-I05`): Encode denial, failure, conflict, stale evidence, unsupported parsing, unverifiable data, and scope exclusion as reasoned Unknown/Blocked states.

- [ ] **Task 20.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 20.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 20.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 20.1.3 - Verify and close the story**
  - [ ] **Sub-task 20.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 20.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 20.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 20.1.3.4 - Product security evidence:** Map `SR-AI-003`, `SR-AI-007`, `SR-AI-010`, `SR-AI-011`, `SR-OPS-001` through `SR-OPS-005`, `SR-TST-010`; retain labeled classification results, citation resolver output, receipt-chain verification, and recomputation report.

##### Story Acceptance Criteria

- [ ] **Story AC 20.1.AC1:** Given the approved dependencies and source requirements for `S-019-I01`, `S-019-I02`, `S-019-I03`, `S-019-I04`, and `S-019-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 20.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-019-I01`, `S-019-I02`, `S-019-I03`, `S-019-I04`, and `S-019-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 20.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 20.AC1:** Every numbered implementation sub-task in Story 20.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 20.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 20.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 20.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 20.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 20 is PASS only when Story 20.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 21 - Citation Freshness and Tamper-Evident Receipts

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-019`, part 2 of 2.

**Sprint goal:** Deliver citation freshness and tamper-evident receipts as a bounded part of the legacy goal: Ground every material claim in an explicit evidence state and resolvable provenance.

**Source coverage:** `AM-EVD-001`, `AM-EVD-002`, `CR-P0-EVD`; PRD Section 16; inventory Sections 9, 22, and 23; `AT-EVD-001`, `AT-EVD-002`, `AT-EVD-003`.

**Dependencies:** Sprint 20; legacy dependency record: Sprint 11 (legacy S-011), Sprint 16 (legacy S-016), Sprint 19 (legacy S-018).

#### [ ] Story 21.1 - Citation Freshness and Tamper-Evident Receipts

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need citation freshness and tamper-evident receipts so that AgentMage delivers the following bounded outcome: Ground every material claim in an explicit evidence state and resolvable provenance.

##### Tasks and Sub-tasks

- [ ] **Task 21.1.1 - Implement the bounded story**
  - [ ] **Sub-task 21.1.1.1** (legacy `S-019-I06`): Implement source resolution against path, hash, range or structured identity, observation point, and current file identity.
  - [ ] **Sub-task 21.1.1.2** (legacy `S-019-I07`): Mark changed or missing evidence stale before reuse.
  - [ ] **Sub-task 21.1.1.3** (legacy `S-019-I08`): Implement answer-claim ledgers, reason-code dictionaries, safe evidence projection, conflict handling, and claim-level audit rendering.
  - [ ] **Sub-task 21.1.1.4** (legacy `S-019-I09`): Implement append-only receipting with chained hashes and a keyed integrity anchor outside the ledger.

- [ ] **Task 21.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 21.1.2.1:** Evidence and citation schemas.
  - [ ] **Sub-task 21.1.2.2:** Deterministic method registry.
  - [ ] **Sub-task 21.1.2.3:** Answer-claim ledger and audit renderer.
  - [ ] **Sub-task 21.1.2.4:** Stale, conflicting, denied, and false-completion fixtures.

- [ ] **Task 21.1.3 - Verify and close the story**
  - [ ] **Sub-task 21.1.3.1:** `S-019-UT01` classifies labeled observed, derived, inferred, unknown, denied, stale, and conflicting claims; assert exact state, method, source, freshness, and limitation fields.
  - [ ] **Sub-task 21.1.3.2:** `S-019-UT02` resolves citations across rename, revision change, Unicode, truncation, stale cache, unavailable source, and changed preimage; assert current targets resolve and stale targets remain visibly stale.
  - [ ] **Sub-task 21.1.3.3:** `S-019-ST01` injects unsupported model claims, fabricated citations, tampered/reordered/removed receipts, and false success results; assert rejection or explicit inference/unknown state and tamper detection.
  - [ ] **Sub-task 21.1.3.4:** `S-019-IT01` recomputes every deterministic result and answer ledger from cited fixture bytes; assert equal values, complete material-claim coverage, and no uncited factual promotion.
  - [ ] **Sub-task 21.1.3.5 - Product security evidence:** Map `SR-AI-003`, `SR-AI-007`, `SR-AI-010`, `SR-AI-011`, `SR-OPS-001` through `SR-OPS-005`, `SR-TST-010`; retain labeled classification results, citation resolver output, receipt-chain verification, and recomputation report.

##### Story Acceptance Criteria

- [ ] **Story AC 21.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every material answer statement has exactly one explicit evidence state and enough provenance for an independent reviewer to reproduce or challenge it.
- [ ] **Story AC 21.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then denial, uncertainty, conflict, stale evidence, limits, and failed validation remain user-visible and cannot be summarized away as success.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 21.AC1:** `AT-EVD-001`, `AT-EVD-002`, and `AT-EVD-003` pass.
- [ ] **Sprint AC 21.AC2:** Every tool attempt has exactly one receipt.
- [ ] **Sprint AC 21.AC3:** Every file-grounded claim resolves to the observed source identity.
- [ ] **Sprint AC 21.AC4:** Every derivation identifies its method and observed inputs.
- [ ] **Sprint AC 21.AC5:** Changed evidence becomes stale and cannot be silently resolved to replacement content.

**Gate decision:** Sprint 21 is PASS only when Story 21.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 22 - Context Management and Crash-Safe Resume

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-020`.

**Sprint goal:** Persist and resume one read-only session without losing current intent or repeating completed work.

**Source coverage:** `AM-SES-001`; inventory Sections 10, 10A, and v0.1 subset of 10C; `AT-CRASH-001`, `AT-RESUME-001`.

**Dependencies:** Sprint 21; legacy dependency record: Sprint 11 (legacy S-011), Sprint 12 (legacy S-012), Sprint 21 (legacy S-019).

#### [ ] Story 22.1 - Context Management and Crash-Safe Resume

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need context management and crash-safe resume so that AgentMage delivers the following bounded outcome: Persist and resume one read-only session without losing current intent or repeating completed work.

##### Tasks and Sub-tasks

- [ ] **Task 22.1.1 - Implement the bounded story**
  - [ ] **Sub-task 22.1.1.1** (legacy `S-020-I01`): Implement context budgeting for instructions, newest request, active objective, plan step, corrections, approvals, blockers, evidence, memory, and expected output.
  - [ ] **Sub-task 22.1.1.2** (legacy `S-020-I02`): Deduplicate low-value context before removing authoritative evidence.
  - [ ] **Sub-task 22.1.1.3** (legacy `S-020-I03`): Implement checked summaries that preserve paths, errors, identifiers, commands, decisions, unresolved questions, citation IDs, and receipt IDs.
  - [ ] **Sub-task 22.1.1.4** (legacy `S-020-I04`): Keep summaries separate from source evidence and reopen original sources when stale, disputed, or insufficient.
  - [ ] **Sub-task 22.1.1.5** (legacy `S-020-I05`): Create safe-boundary checkpoints with objective, plan, next action, workspace, repository, branch, permission, instructions, model, evidence, and blockers.
  - [ ] **Sub-task 22.1.1.6** (legacy `S-020-I06`): Commit action state, consumed grant, receipt, evidence pointer, and next checkpoint atomically.
  - [ ] **Sub-task 22.1.1.7** (legacy `S-020-I07`): Revalidate workspace, file, instruction, branch, map, citation, model, permission, and policy drift before resuming.
  - [ ] **Sub-task 22.1.1.8** (legacy `S-020-I08`): Provide explicit continue, restart, or cancel when material drift is detected.

- [ ] **Task 22.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 22.1.2.1:** Context manager and summary schema.
  - [ ] **Sub-task 22.1.2.2:** Atomic checkpoint and resume state machine.
  - [ ] **Sub-task 22.1.2.3:** Context-debug view with sensitivity-labeled inputs.
  - [ ] **Sub-task 22.1.2.4:** Crash-point, maximum-context, and drift test results.

- [ ] **Task 22.1.3 - Verify and close the story**
  - [ ] **Sub-task 22.1.3.1:** `S-020-UT01` composes context at empty, nominal, maximum, and over-limit sizes with conflicting/stale/denied evidence; assert deterministic priority, bounded excerpts, visible omissions, and no secret canary.
  - [ ] **Sub-task 22.1.3.2:** `S-020-UT02` validates checkpoint schemas and legal resume transitions with missing, stale, corrupt, future-version, mismatched-workspace, and mismatched-policy state; assert safe refusal or explicit recovery.
  - [ ] **Sub-task 22.1.3.3:** `S-020-RT01` injects crashes before and after each checkpoint and tool terminal state across at least 100 resumes; assert no completed operation repeats and no pending operation is claimed complete.
  - [ ] **Sub-task 22.1.3.4:** `S-020-IT01` resumes long fixture sessions after model/runtime/configuration/repository changes; assert impact is surfaced, evidence is invalidated where needed, and original intent/revision history remains inspectable.
  - [ ] **Sub-task 22.1.3.5 - Product security evidence:** Map `SR-DAT-002`/`SR-DAT-003`, `SR-AI-008` through `SR-AI-010`, `SR-OPS-003`, `SR-TST-005`/`SR-TST-006`; retain checkpoint hashes, crash matrix, context manifests, canary scans, and resume comparisons.

##### Story Acceptance Criteria

- [ ] **Story AC 22.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then resume restores one canonical task/session state or blocks visibly; it never merges incompatible state, silently drops current intent, or repeats a side effect.
- [ ] **Story AC 22.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then context-debug output accounts for every included/excluded item, classification, token/size budget, source identity, and redaction without exposing prohibited content.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 22.AC1:** `AT-CRASH-001` and `AT-RESUME-001` pass.
- [ ] **Sprint AC 22.AC2:** Forced termination never repeats a completed operation.
- [ ] **Sprint AC 22.AC3:** Material drift is detected before another action.
- [ ] **Sprint AC 22.AC4:** Context condensation preserves the active request, correction, evidence references, and next safe action.
- [ ] **Sprint AC 22.AC5:** Ephemeral mode leaves no persisted session record.

**Gate decision:** Sprint 22 is PASS only when Story 22.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 23 - Native Visual Studio Code Chat Experience

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-021`, part 1 of 3.

**Sprint goal:** Deliver native visual studio code chat experience as a bounded part of the legacy goal: Deliver the complete v0.1 workflow in native Visual Studio Code Chat and close every blocking release threshold.

**Source coverage:** `AM-HOF-001`, `AM-VSC-001`, `AM-VSC-002`, `AM-TST-001`, `AM-TST-002`, `AM-DOC-001`, `CR-P0-EVAL`; inventory Sections 27, 31A-34; `AT-HOF-001`, `AT-VSC-001`, `AT-VSC-002`, `AT-QUAL-001`, `AT-PERF-001`, `AT-SPEC-001`, `AT-DOC-001`.

**Dependencies:** Sprint 22; legacy dependency record: Sprint 8 (legacy S-008) through Sprint 22 (legacy S-020).

#### [ ] Story 23.1 - Native Visual Studio Code Chat Experience

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need native visual studio code chat experience so that AgentMage delivers the following bounded outcome: Deliver the complete v0.1 workflow in native Visual Studio Code Chat and close every blocking release threshold.

##### Tasks and Sub-tasks

- [ ] **Task 23.1.1 - Implement the bounded story**
  - [ ] **Sub-task 23.1.1.1** (legacy `S-021-I01`): Register the AgentMage language-model provider through the pinned stable Visual Studio Code API.
  - [ ] **Sub-task 23.1.1.2** (legacy `S-021-I02`): Expose **AgentMage - Gemma 4 E4B (Local, Read Only)** in the native model picker with exact capabilities and limits.
  - [ ] **Sub-task 23.1.1.3** (legacy `S-021-I03`): Route every request through the authenticated bridge, kernel runtime, selected model adapter, grants, and tool dispatcher.
  - [ ] **Sub-task 23.1.1.4** (legacy `S-021-I04`): Stream text, evidence states, citations, progress, diagnostics, tool results, denials, cancellation, and failures.
  - [ ] **Sub-task 23.1.1.5** (legacy `S-021-I05`): Render validated clickable display links and session, workspace, model, permission, tool, and offline indicators.

- [ ] **Task 23.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 23.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 23.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 23.1.3 - Verify and close the story**
  - [ ] **Sub-task 23.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 23.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 23.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 23.1.3.4 - Product security evidence:** Map `SR-PLT-005`/`SR-PLT-006`, `SR-ACC-007`, `SR-DAT-003`, `SR-OPS-001`/`SR-OPS-003`, `SR-TST-004`, and `SR-CIV-006` through `SR-CIV-009`; extend `RV-05`, `RV-08`, and `RV-18`; retain authenticated message traces, raw-interface denial tests, redaction scans, native Chat workflow output, and reviewer disposition.

##### Story Acceptance Criteria

- [ ] **Story AC 23.1.AC1:** Given the approved dependencies and source requirements for `S-021-I01`, `S-021-I02`, `S-021-I03`, `S-021-I04`, and `S-021-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 23.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-021-I01`, `S-021-I02`, `S-021-I03`, `S-021-I04`, and `S-021-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 23.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### [ ] Story 23.2 - Accessible Native Chat Workflow

**User-facing value:** As a user with varied visual, motor, or assistive-technology needs, I need every v0.1 action and status to remain operable and understandable in native Visual Studio Code Chat.

##### Tasks and Sub-tasks

- [ ] **Task 23.2.1 - Implement accessible interaction and output**
  - [ ] **Sub-task 23.2.1.1:** Define accessible names, roles, states, descriptions, live-status behavior, error associations, focus order, focus restoration, and keyboard operation for model selection, workspace selection, chat, citations, diagnostics, cancellation, and handoff preview.
  - [ ] **Sub-task 23.2.1.2:** Ensure status and evidence meaning never depends only on color, icon, animation, position, hover, pointer precision, or timing; preserve content at supported zoom and reflow settings.
  - [ ] **Sub-task 23.2.1.3:** Make generated Markdown, citations, diagnostics, limitations, receipts, and error guidance structurally navigable and understandable by screen readers.
  - [ ] **Sub-task 23.2.1.4:** Publish a versioned accessibility conformance report that distinguishes automated passes, manual passes, failures, not-tested items, platform differences, and remediation.

- [ ] **Task 23.2.2 - Verify and close the story**
  - [ ] **Sub-task 23.2.2.1:** Complete each core workflow by keyboard alone at supported zoom/reflow levels; assert visible focus, no trap, no clipped control, no pointer-only action, and successful cancellation/recovery.
  - [ ] **Sub-task 23.2.2.2:** Perform manual VoiceOver testing on macOS and declared Linux screen-reader testing, plus automated accessibility checks where supported; retain exact OS, VS Code, extension, and assistive-technology versions.
  - [ ] **Sub-task 23.2.2.3:** Seed missing names, bad focus order, color-only meaning, inaccessible live updates, timeout pressure, and malformed generated structure; assert each blocks the core-workflow gate.
  - [ ] **Sub-task 23.2.2.4 - Product security evidence:** Complete the first `RV-20`; map `SR-CIV-006` through `SR-CIV-009` and `SR-TST-008`; retain keyboard transcripts, accessibility-tree output, contrast/reflow results, assistive-technology notes, seeded-failure results, and the conformance report.

##### Story Acceptance Criteria

- [ ] **Story AC 23.2.AC1:** Given a keyboard-only user, when each v0.1 core workflow is performed, then every action, status, citation, error, cancellation, and recovery path is reachable with visible logical focus and no trap.
- [ ] **Story AC 23.2.AC2:** Given supported screen readers and zoom/reflow settings, when Chat streams text, tools, evidence states, diagnostics, and failures, then updates are announced in order without lost content, color-only meaning, overlap, or forced timing.
- [ ] **Story AC 23.2.AC3:** Given raw automated and manual results, when the conformance report is generated, then every pass, failure, not-tested item, platform difference, and remediation reconciles to current evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 23.AC1:** Every numbered implementation sub-task in Stories 23.1 and 23.2 is complete and linked to its source requirement or issue identity.
- [ ] **Sprint AC 23.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 23.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 23.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 23.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 23 is PASS only when Stories 23.1 and 23.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 24 - Manual Codex Handoff Boundary

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-021`, part 2 of 3.

**Sprint goal:** Deliver manual codex handoff boundary as a bounded part of the legacy goal: Deliver the complete v0.1 workflow in native Visual Studio Code Chat and close every blocking release threshold.

**Source coverage:** `AM-HOF-001`, `AM-VSC-001`, `AM-VSC-002`, `AM-TST-001`, `AM-TST-002`, `AM-DOC-001`, `CR-P0-EVAL`; inventory Sections 27, 31A-34; `AT-HOF-001`, `AT-VSC-001`, `AT-VSC-002`, `AT-QUAL-001`, `AT-PERF-001`, `AT-SPEC-001`, `AT-DOC-001`.

**Dependencies:** Sprint 23; legacy dependency record: Sprint 8 (legacy S-008) through Sprint 22 (legacy S-020).

#### [ ] Story 24.1 - Manual Codex Handoff Boundary

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need manual codex handoff boundary so that AgentMage delivers the following bounded outcome: Deliver the complete v0.1 workflow in native Visual Studio Code Chat and close every blocking release threshold.

##### Tasks and Sub-tasks

- [ ] **Task 24.1.1 - Implement the bounded story**
  - [ ] **Sub-task 24.1.1.1** (legacy `S-021-I06`): Build the local Codex handoff packet with objective, acceptance criteria, cited evidence, constraints, disclosure list, and unresolved questions.
  - [ ] **Sub-task 24.1.1.2** (legacy `S-021-I07`): Enforce zero Codex invocation, tab activation or population, clipboard write, endpoint call, packet transmission, or autonomous delivery.

- [ ] **Task 24.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 24.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 24.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 24.1.3 - Verify and close the story**
  - [ ] **Sub-task 24.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 24.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 24.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 24.1.3.4 - Product security evidence:** Map `SR-ACC-007`/`SR-ACC-008`, `SR-DAT-002`/`SR-DAT-003`, `SR-NET-002`, `SR-AI-004`/`SR-AI-008`, `SR-OPS-001`/`SR-OPS-003`; extend `RV-08`, `RV-11`, and `RV-18`; retain packet hashes, disclosure/redaction results, prohibited-transfer traces, approval receipts, and independent boundary review.

##### Story Acceptance Criteria

- [ ] **Story AC 24.1.AC1:** Given the approved dependencies and source requirements for `S-021-I06`, and `S-021-I07`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 24.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-021-I06`, and `S-021-I07`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 24.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### [ ] Story 24.2 - Handoff Disclosure and Staleness Warnings

**User-facing value:** As a user, I need to know exactly what a Codex handoff packet contains, what it excludes, whether it is stale, and that I alone decide whether to submit it outside AgentMage.

##### Tasks and Sub-tasks

- [ ] **Task 24.2.1 - Implement handoff safety communication**
  - [ ] **Sub-task 24.2.1.1:** Show a mandatory pre-handoff review with objective, exact included files/ranges/excerpts, citations/hashes, inferred content, exclusions, unresolved questions, sensitivity labels, redactions, destination class, and estimated size.
  - [ ] **Sub-task 24.2.1.2:** State clearly that the packet remains local, AgentMage has not contacted Codex or any external service, and external handling begins only if the user manually transfers selected content.
  - [ ] **Sub-task 24.2.1.3:** Revalidate workspace, source hashes, citations, policy, and redaction immediately before final rendering; mark changed evidence stale and require regeneration rather than silently carrying it forward.
  - [ ] **Sub-task 24.2.1.4:** Require explicit acknowledgment when a packet contains any permitted non-public or user-provided content, while continuing to block credentials, keys, prohibited data, hidden files, and unrelated context.

- [ ] **Task 24.2.2 - Verify and close the story**
  - [ ] **Sub-task 24.2.2.1:** Inject secret canaries, hidden files, stale citations, inferred claims, conflicting classifications, oversized excerpts, and prompt-injection requests to conceal disclosure; assert blocking or accurate visible treatment.
  - [ ] **Sub-task 24.2.2.2:** Attempt tab control, Chat population, clipboard writes, URI launches, local/raw-runtime delivery, network calls, and automatic submission through every handoff state; assert zero effect and one denial receipt per attempt.
  - [ ] **Sub-task 24.2.2.3:** Compare disclosure preview, rendered packet, and packet manifest byte-for-byte for included content and hashes; assert no unpreviewed field or excerpt appears.
  - [ ] **Sub-task 24.2.2.4 - Product security evidence:** Extend `RV-08`, `RV-11`, and `RV-18`; map `SR-ACC-007`/`SR-ACC-008`, `SR-DAT-002`/`SR-DAT-003`, `SR-AI-004`/`SR-AI-008`/`SR-AI-010`, `SR-CIV-003`/`SR-CIV-004`/`SR-CIV-009`; retain previews, packet manifests, canary scans, staleness results, prohibited-action traces, and acknowledgments.

##### Story Acceptance Criteria

- [ ] **Story AC 24.2.AC1:** Given a proposed handoff, when the review is rendered, then every included source, excerpt, inference, sensitivity label, redaction, exclusion, unresolved question, and destination implication is visible before the user acts.
- [ ] **Story AC 24.2.AC2:** Given changed, prohibited, hidden, or unpreviewed content, when final rendering is attempted, then the handoff blocks or requires regeneration and no packet is transmitted, copied, or injected into another interface.
- [ ] **Story AC 24.2.AC3:** Given an approved current preview, when the packet is rendered, then its content and manifest match exactly and AgentMage records only a local receipt, never an external-delivery claim.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 24.AC1:** Every numbered implementation sub-task in Stories 24.1 and 24.2 is complete and linked to its source requirement or issue identity.
- [ ] **Sprint AC 24.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 24.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 24.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 24.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 24 is PASS only when Stories 24.1 and 24.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 25 - v0.1 Cross-Platform Release

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-021`, part 3 of 3.

**Sprint goal:** Deliver v0.1 cross-platform release as a bounded part of the legacy goal: Deliver the complete v0.1 workflow in native Visual Studio Code Chat and close every blocking release threshold.

**Source coverage:** `AM-HOF-001`, `AM-VSC-001`, `AM-VSC-002`, `AM-TST-001`, `AM-TST-002`, `AM-DOC-001`, `CR-P0-EVAL`; inventory Sections 27, 31A-34; `AT-HOF-001`, `AT-VSC-001`, `AT-VSC-002`, `AT-QUAL-001`, `AT-PERF-001`, `AT-SPEC-001`, `AT-DOC-001`.

**Dependencies:** Sprint 24; legacy dependency record: Sprint 8 (legacy S-008) through Sprint 22 (legacy S-020).

#### [ ] Story 25.1 - v0.1 Cross-Platform Release

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need v0.1 cross-platform release so that AgentMage delivers the following bounded outcome: Deliver the complete v0.1 workflow in native Visual Studio Code Chat and close every blocking release threshold.

##### Tasks and Sub-tasks

- [ ] **Task 25.1.1 - Implement the bounded story**
  - [ ] **Sub-task 25.1.1.1** (legacy `S-021-I08`): Rerun the complete security, privacy, injection, path, network, model, repository-map, evidence, recovery, quality, performance, accessibility, support, and documentation suites after their owning sprints have completed first execution.
  - [ ] **Sub-task 25.1.1.2** (legacy `S-021-I09`): Publish install, first-run, model installation, diagnostics, permissions, evidence, repository-map, privacy, offline proof, recovery, troubleshooting, limitation, and maintainer release guides.
  - [ ] **Sub-task 25.1.1.3** (legacy `S-021-I10`): Run clean installations and the identical supported workflow on the recorded MacBook Pro M5, Fedora, and Ubuntu environments.

- [ ] **Task 25.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 25.1.2.1:** Signed and verified v0.1 packages for all reference platforms.
  - [ ] **Sub-task 25.1.2.2:** Visual Studio Code extension and provider package.
  - [ ] **Sub-task 25.1.2.3:** Complete v0.1 acceptance-result bundle and release manifest.
  - [ ] **Sub-task 25.1.2.4:** Published operating guides, capability matrix, limitations, and release notes.

- [ ] **Task 25.1.3 - Verify and close the story**
  - [ ] **Sub-task 25.1.3.1:** `S-021-UT01` validates every extension/host message, Chat response, command, cancellation, status, citation, and error schema with malformed/replayed/wrong-session inputs; assert authenticated fail-closed handling.
  - [ ] **Sub-task 25.1.3.2:** `S-021-ST01` attempts filesystem, Git, model, key, grant, raw-runtime, network, Codex-handoff, write, shell, and external-action access from the extension and native Chat surface; assert display/interaction authority only.
  - [ ] **Sub-task 25.1.3.3:** `S-021-IT01` runs the complete v0.1 task corpus through native VS Code Chat on macOS, Fedora, and Ubuntu; assert shell parity, exact citations, truthful statuses, accessibility behavior, and no alternate UI requirement.
  - [ ] **Sub-task 25.1.3.4:** `S-021-AT01` performs three independent clean standard-user installs per supported platform, imports the pinned model, goes offline, completes every workflow, exports evidence, and uninstalls using published instructions.
  - [ ] **Sub-task 25.1.3.5:** `S-021-AT02` forces each blocking threshold, exclusion, dependency, and security control to fail independently; assert no signed production release or closed `G-V0.1` is produced.
  - [ ] **Sub-task 25.1.3.6 - Product security evidence:** Map all applicable v0.1 `SR-GOV-*`, `SR-PLT-*`, `SR-ACC-*`, `SR-DAT-*`, `SR-NET-*`, `SR-SUP-*`, `SR-AI-*`, `SR-OPS-*`, `SR-TST-*`, and `SR-CIV-*`; execute `RV-01` through `RV-22` as applicable and retain the complete reviewer evidence bundle defined by `SECURITY-REVIEW.md`.

##### Story Acceptance Criteria

- [ ] **Story AC 25.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every v0.1 backlog row and quantitative threshold has current raw evidence on its declared platform; no Linux result substitutes for Mac deployment evidence and no skipped check is green.
- [ ] **Story AC 25.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the release package is read-only, local-only after acquisition, single-model, single-agent, native-Chat-first, standard-user installable, accessible, reproducible, recoverable, and independently verifiable.

#### [ ] Story 25.2 - Incident Tabletop and Release Support Readiness

**User-facing value:** As a user or reviewer, I need evidence that maintainers can contain and repair a security failure without improvising access, collecting private content, or relying on hidden cloud control.

##### Tasks and Sub-tasks

- [ ] **Task 25.2.1 - Prepare incident and support operations**
  - [ ] **Sub-task 25.2.1.1:** Publish versioned local-first runbooks for suspected egress, compromised dependency/package, prompt-injection disclosure, model/runtime revocation, key-store or cryptographic failure, and corrupted operational state.
  - [ ] **Sub-task 25.2.1.2:** Define detection, local suspension, containment, bounded evidence preservation, severity, ownership, communication, remediation, signed manual patch, verification, recovery, and lessons-record transitions for each scenario.
  - [ ] **Sub-task 25.2.1.3:** Define what diagnostics may be requested and prohibit raw prompts, workspace files, credentials, private keys, full environment dumps, unrelated paths, and unreviewed archives from support evidence.
  - [ ] **Sub-task 25.2.1.4:** Prepare signed emergency-disable and manual patch fixtures that require explicit local user installation and never create a remote kill switch, telemetry channel, or silent update check.

- [ ] **Task 25.2.2 - Execute tabletop and patch exercises**
  - [ ] **Sub-task 25.2.2.1:** Run the four required `RV-21` scenarios with named participants independent of the component under test; inject ambiguous, late, duplicate, and false-positive signals and record decisions.
  - [ ] **Sub-task 25.2.2.2:** Run `RV-22` against valid, wrong-signer, downgrade, interrupted, corrupt, manifest-mismatched, migration-failed, rollback, revoked-component, and end-of-support manual patch states.
  - [ ] **Sub-task 25.2.2.3:** Search every tabletop, diagnostic, support, and patch artifact for synthetic canaries and prohibited host identity; assert redaction and retention policy before evidence is retained.
  - [ ] **Sub-task 25.2.2.4 - Product security evidence:** Complete `RV-21` and v0.1 `RV-22`; map `SR-SUP-010`, `SR-OPS-004` through `SR-OPS-007`, `SR-TST-010`, and `SR-CIV-005`/`SR-CIV-009`; retain timelines, decisions, communications, redacted evidence, patch verification, recovery results, and lessons/actions with owners.

##### Story Acceptance Criteria

- [ ] **Story AC 25.2.AC1:** Given each required incident scenario, when the tabletop runs, then participants detect, suspend, contain, preserve bounded evidence, assign ownership, communicate, remediate, verify, recover, and record lessons without acquiring undeclared authority or private user content.
- [ ] **Story AC 25.2.AC2:** Given valid and adversarial signed manual patches, when `RV-22` runs, then only the exact authorized non-downgrade patch succeeds, rollback preserves security and data integrity, and revoked or unsupported states remain visibly constrained.
- [ ] **Story AC 25.2.AC3:** Given the complete tabletop evidence, when an independent reviewer reconstructs each timeline, then every action, decision, failure, notification, open risk, and follow-up owner is present and no synthetic secret canary is disclosed.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 25.AC1:** Every `AM-*` v0.1 backlog row is complete with its required `AT-*` receipts.
- [ ] **Sprint AC 25.AC2:** Every Section 31B threshold passes without waiver on the declared platforms.
- [ ] **Sprint AC 25.AC3:** `AT-HOF-001`, `AT-VSC-001`, `AT-VSC-002`, `AT-QUAL-001`, `AT-PERF-001`, `AT-SPEC-001`, `AT-DOC-001`, `RV-21`, and v0.1 `RV-22` pass.
- [ ] **Sprint AC 25.AC4:** Release notes list every v0.1 exclusion, including writes, semantic indexing, Obsidian, full CLI, desktop, GitHub, browser, connectors, schedules, child agents, and Codex transfer.
- [ ] **Sprint AC 25.AC5:** `G-V0.1` closes only after the signed artifacts, documentation, tests, and offline proof agree exactly.

**Gate decision:** Sprint 25 is PASS only when Stories 25.1 and 25.2, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 2 - v0.2 - Knowledge, Obsidian, and Memory

### [ ] Sprint 26 - Canonical Human Knowledge Domain

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-022`.

**Sprint goal:** Add user-owned Markdown as the sole authority for human knowledge while keeping operational state in encrypted SQLite.

**Source coverage:** inventory Sections 8A, 10, 10B, 10D, and 10E; `CR-P1-SES` foundation.

**Dependencies:** Sprint 25; legacy dependency record: `G-V0.1`.

#### [ ] Story 26.1 - Canonical Human Knowledge Domain

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need canonical human knowledge domain so that AgentMage delivers the following bounded outcome: Add user-owned Markdown as the sole authority for human knowledge while keeping operational state in encrypted SQLite.

##### Tasks and Sub-tasks

- [ ] **Task 26.1.1 - Implement the bounded story**
  - [ ] **Sub-task 26.1.1.1** (legacy `S-022-I01`): Define the `KnowledgeStore` interface and canonical schemas for people, organizations, projects, meetings, tasks, decisions, commitments, documents, correspondence, deadlines, approvals, risks, questions, and handoffs.
  - [ ] **Sub-task 26.1.1.2** (legacy `S-022-I02`): Implement a plain-folder Markdown adapter with configurable folder, filename, frontmatter, tag, link, identifier, privacy, and retention templates.
  - [ ] **Sub-task 26.1.1.3** (legacy `S-022-I03`): Preserve stable identifiers across note rename, move, index rebuild, export, restore, and conversation resume.
  - [ ] **Sub-task 26.1.1.4** (legacy `S-022-I04`): Implement atomic Markdown create and update previews without making writes available until the v0.3 grant path exists.
  - [ ] **Sub-task 26.1.1.5** (legacy `S-022-I05`): Build a disposable SQLite knowledge index that can be deleted and rebuilt without changing canonical Markdown.
  - [ ] **Sub-task 26.1.1.6** (legacy `S-022-I06`): Implement relationship links, duplicate detection, import validation, workspace dashboard generation, and explicit JSON Lines export.
  - [ ] **Sub-task 26.1.1.7** (legacy `S-022-I07`): Implement plain-folder backup, integrity, restore, and migration dry runs.
  - [ ] **Sub-task 26.1.1.8** (legacy `S-022-I08`): Prove operational records never depend on Markdown and knowledge records never become co-authoritative in operational SQLite.

- [ ] **Task 26.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 26.1.2.1:** Versioned knowledge-domain schemas.
  - [ ] **Sub-task 26.1.2.2:** Plain-folder adapter and regenerable index.
  - [ ] **Sub-task 26.1.2.3:** Canonical-authority boundary tests.
  - [ ] **Sub-task 26.1.2.4:** Knowledge backup, restore, and migration report.

- [ ] **Task 26.1.3 - Verify and close the story**
  - [ ] **Sub-task 26.1.3.1:** `S-022-UT01` classifies canonical Markdown, derived index, operational state, cache, export, and temporary data; assert exactly one owner, storage rule, retention rule, and rebuild path per field.
  - [ ] **Sub-task 26.1.3.2:** `S-022-UT02` mutates or deletes derived indexes and operational records; assert canonical user files remain unchanged and every derived view rebuilds from approved sources.
  - [ ] **Sub-task 26.1.3.3:** `S-022-ST01` introduces symlinks, cloud-synced roots, adjacent folders, hidden files, secrets, conflicting identities, and malicious note content; assert bounded scope, classification, and non-authority.
  - [ ] **Sub-task 26.1.3.4:** `S-022-RT01` backs up, migrates, restores, and deletes synthetic knowledge across schema versions; assert canonical identity, links, provenance, retention, and index regeneration.
  - [ ] **Sub-task 26.1.3.5 - Product security evidence:** Map `SR-GOV-006`, `SR-DAT-001` through `SR-DAT-004`, `SR-DAT-010` through `SR-DAT-012`, `SR-CIV-001` through `SR-CIV-005`; retain data dictionary, authority tests, rebuild hashes, lifecycle results, and privacy/records decision placeholders.

##### Story Acceptance Criteria

- [ ] **Story AC 26.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then human knowledge authority is inspectable user-owned Markdown; SQLite and indexes never silently become the source of truth for a human fact.
- [ ] **Story AC 26.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every stored or derived knowledge field has purpose, classification, minimization, encryption, retention, correction, export, deletion, and recovery behavior.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 26.AC1:** Deleting the knowledge index changes no canonical record.
- [ ] **Sprint AC 26.AC2:** Rebuilding the index reproduces all expected identifiers, links, metadata, and hashes.
- [ ] **Sprint AC 26.AC3:** Operational startup succeeds without Markdown knowledge and knowledge browsing succeeds without operational export files.
- [ ] **Sprint AC 26.AC4:** Duplicate, malformed, conflicting, private, and restricted records are detected before import.
- [ ] **Sprint AC 26.AC5:** The capability remains read-only against user files until `G-V0.3`.

**Gate decision:** Sprint 26 is PASS only when Story 26.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 27 - Obsidian Note Parsing

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-023`, part 1 of 2.

**Sprint goal:** Deliver obsidian note parsing as a bounded part of the legacy goal: Read and index an approved local Obsidian vault directly without requiring or automating Obsidian.

**Source coverage:** inventory Section 8 and Obsidian portions of Section 8A.

**Dependencies:** Sprint 26; legacy dependency record: Sprint 26 (legacy S-022).

#### [ ] Story 27.1 - Obsidian Note Parsing

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need obsidian note parsing so that AgentMage delivers the following bounded outcome: Read and index an approved local Obsidian vault directly without requiring or automating Obsidian.

##### Tasks and Sub-tasks

- [ ] **Task 27.1.1 - Implement the bounded story**
  - [ ] **Sub-task 27.1.1.1** (legacy `S-023-I01`): Implement deterministic Markdown discovery with ignored folders, stable ordering, Unicode paths, spaces, and symlink boundaries.
  - [ ] **Sub-task 27.1.1.2** (legacy `S-023-I02`): Parse frontmatter, headings, tasks, wiki links, aliases, backlinks, timestamps, and source line numbers while excluding code fences.
  - [ ] **Sub-task 27.1.1.3** (legacy `S-023-I03`): Fail closed on malformed frontmatter and report ambiguous or unresolved links.
  - [ ] **Sub-task 27.1.1.4** (legacy `S-023-I04`): Require an explicitly selected local vault root through the same workspace and path protections as every file tool.
  - [ ] **Sub-task 27.1.1.5** (legacy `S-023-I05`): Refuse strict-local vault roots in detected cloud-synchronized or remote locations.

- [ ] **Task 27.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 27.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 27.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 27.1.3 - Verify and close the story**
  - [ ] **Sub-task 27.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 27.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 27.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 27.1.3.4 - Product security evidence:** Map `SR-ACC-004` through `SR-ACC-008`, `SR-AI-005`, `SR-DAT-002`, `SR-TST-002`/`SR-TST-004`; retain parser corpus, link graph expectations, injection results, index transaction traces, and no-Obsidian proof.

##### Story Acceptance Criteria

- [ ] **Story AC 27.1.AC1:** Given the approved dependencies and source requirements for `S-023-I01`, `S-023-I02`, `S-023-I03`, `S-023-I04`, and `S-023-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 27.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-023-I01`, `S-023-I02`, `S-023-I03`, `S-023-I04`, and `S-023-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 27.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 27.AC1:** Every numbered implementation sub-task in Story 27.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 27.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 27.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 27.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 27.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 27 is PASS only when Story 27.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 28 - Vault Indexing, Links, and Recovery

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-023`, part 2 of 2.

**Sprint goal:** Deliver vault indexing, links, and recovery as a bounded part of the legacy goal: Read and index an approved local Obsidian vault directly without requiring or automating Obsidian.

**Source coverage:** inventory Section 8 and Obsidian portions of Section 8A.

**Dependencies:** Sprint 27; legacy dependency record: Sprint 26 (legacy S-022).

#### [ ] Story 28.1 - Vault Indexing, Links, and Recovery

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need vault indexing, links, and recovery so that AgentMage delivers the following bounded outcome: Read and index an approved local Obsidian vault directly without requiring or automating Obsidian.

##### Tasks and Sub-tasks

- [ ] **Task 28.1.1 - Implement the bounded story**
  - [ ] **Sub-task 28.1.1.1** (legacy `S-023-I06`): Keep index, memory, conversation state, temporary text, and hidden agent records outside the vault.
  - [ ] **Sub-task 28.1.1.2** (legacy `S-023-I07`): Implement current-versus-historical classification, special current-note readers, bounded queries, relationship traversal, and stale-index detection.
  - [ ] **Sub-task 28.1.1.3** (legacy `S-023-I08`): Preserve raw meeting-note sections and produce per-file previews for all later note changes.
  - [ ] **Sub-task 28.1.1.4** (legacy `S-023-I09`): Implement a local watcher that updates only the disposable index and emits access receipts.

- [ ] **Task 28.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 28.1.2.1:** Obsidian adapter and index schema.
  - [ ] **Sub-task 28.1.2.2:** Synthetic vault corpus with links, aliases, tasks, conflicts, and malformed notes.
  - [ ] **Sub-task 28.1.2.3:** Vault access and no-Obsidian proof receipts.
  - [ ] **Sub-task 28.1.2.4:** Index rebuild and stale-update results.

- [ ] **Task 28.1.3 - Verify and close the story**
  - [ ] **Sub-task 28.1.3.1:** `S-023-UT01` parses frontmatter, headings, blocks, links, embeds, aliases, tags, tasks, properties, callouts, attachments, and malformed variants; assert exact source ranges and graceful unsupported syntax.
  - [ ] **Sub-task 28.1.3.2:** `S-023-UT02` resolves valid, missing, ambiguous, renamed, aliased, case-colliding, Unicode-colliding, and cyclic links; assert deterministic graph state and visible conflicts.
  - [ ] **Sub-task 28.1.3.3:** `S-023-ST01` seeds plugins/config/workspace files, scripts, URI schemes, remote embeds, prompt injections, secrets, and out-of-vault links; assert no execution, Obsidian automation, network use, or authority change.
  - [ ] **Sub-task 28.1.3.4:** `S-023-RT01` interrupts full rebuild and incremental update, then changes/deletes notes during indexing; assert atomic index publication, stale detection, and source-file invariance.
  - [ ] **Sub-task 28.1.3.5 - Product security evidence:** Map `SR-ACC-004` through `SR-ACC-008`, `SR-AI-005`, `SR-DAT-002`, `SR-TST-002`/`SR-TST-004`; retain parser corpus, link graph expectations, injection results, index transaction traces, and no-Obsidian proof.

##### Story Acceptance Criteria

- [ ] **Story AC 28.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then agentMage reads only the explicitly approved vault through its own bounded parser/indexer; Obsidian need not be installed, launched, configured, or automated.
- [ ] **Story AC 28.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every indexed element resolves to vault identity, relative path, source range, content hash, parser version, and freshness; unsupported material remains visible in coverage.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 28.AC1:** The synthetic mini-vault parses to exact expected records and source lines.
- [ ] **Sprint AC 28.AC2:** Code-fence content never creates false tasks or links.
- [ ] **Sprint AC 28.AC3:** Obsidian absent from the machine does not remove any supported capability.
- [ ] **Sprint AC 28.AC4:** Vault reads produce no user-file mutation and no external process or network access.
- [ ] **Sprint AC 28.AC5:** Plain-folder and Obsidian adapters pass the same knowledge-domain contract tests.

**Gate decision:** Sprint 28 is PASS only when Story 28.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 29 - Deterministic Knowledge Retrieval

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-024`.

**Sprint goal:** Answer bounded knowledge questions through exact search, metadata, ranking, freshness, conflicts, and citations before semantic retrieval is introduced.

**Source coverage:** inventory Section 9 deterministic items; Sections 22 and 31A.

**Dependencies:** Sprint 28; legacy dependency record: Sprint 26 (legacy S-022), Sprint 28 (legacy S-023).

#### [ ] Story 29.1 - Deterministic Knowledge Retrieval

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need deterministic knowledge retrieval so that AgentMage delivers the following bounded outcome: Answer bounded knowledge questions through exact search, metadata, ranking, freshness, conflicts, and citations before semantic retrieval is introduced.

##### Tasks and Sub-tasks

- [ ] **Task 29.1.1 - Implement the bounded story**
  - [ ] **Sub-task 29.1.1.1** (legacy `S-024-I01`): Implement source-document and context-query contracts with roots, dates, file types, authority, result budgets, hashes, and source ranges.
  - [ ] **Sub-task 29.1.1.2** (legacy `S-024-I02`): Implement exact keyword and phrase search, metadata filtering, and deterministic ranking.
  - [ ] **Sub-task 29.1.1.3** (legacy `S-024-I03`): Prefer current handoffs, canonical records, direct evidence, and recently verified sources under explicit rules.
  - [ ] **Sub-task 29.1.1.4** (legacy `S-024-I04`): Implement bounded context assembly with deduplication and citation preservation.
  - [ ] **Sub-task 29.1.1.5** (legacy `S-024-I05`): Implement conflict detection, source freshness checks, and no-evidence responses.
  - [ ] **Sub-task 29.1.1.6** (legacy `S-024-I06`): Preserve evidence states through retrieval, synthesis, and final rendering.
  - [ ] **Sub-task 29.1.1.7** (legacy `S-024-I07`): Build representative knowledge questions with known answers, conflicts, stale sources, and absent evidence.

- [ ] **Task 29.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 29.1.2.1:** Deterministic knowledge retrieval pipeline.
  - [ ] **Sub-task 29.1.2.2:** Ranking and freshness decision table.
  - [ ] **Sub-task 29.1.2.3:** Labeled retrieval corpus and expected citation set.
  - [ ] **Sub-task 29.1.2.4:** Retrieval coverage and blind-spot report.

- [ ] **Task 29.1.3 - Verify and close the story**
  - [ ] **Sub-task 29.1.3.1:** `S-024-UT01` scores exact terms, fields, tags, links, dates, tasks, headings, and metadata using a fixed corpus; assert deterministic tie-breaking and expected ranked citations.
  - [ ] **Sub-task 29.1.3.2:** `S-024-UT02` varies freshness, conflicting notes, supersession, missing targets, stale indexes, empty query, limits, and normalization; assert documented ranking and evidence states.
  - [ ] **Sub-task 29.1.3.3:** `S-024-ST01` introduces query injection, regex/path abuse where applicable, oversized tokens, adversarial Unicode, hidden secrets, and unrelated-workspace canaries; assert bounded search and zero leakage.
  - [ ] **Sub-task 29.1.3.4:** `S-024-IT01` answers labeled knowledge questions from raw sources and rebuilt indexes; assert material claims match expected citation sets and blind spots are reported.
  - [ ] **Sub-task 29.1.3.5 - Product security evidence:** Map `SR-AI-003`, `SR-AI-005`, `SR-AI-007` through `SR-AI-011`, `SR-TST-004`; retain corpus labels, ranking traces, metric calculation, expected/actual citation diffs, and coverage report.

##### Story Acceptance Criteria

- [ ] **Story AC 29.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then deterministic retrieval meets the declared precision/recall/top-k/freshness thresholds on a versioned corpus before semantic retrieval can be recommended.
- [ ] **Story AC 29.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then empty, missing, stale, contradictory, denied, and ambiguous results produce explicit states and citations rather than confident synthesis.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 29.AC1:** Exact and metadata queries return the expected bounded source set in stable order.
- [ ] **Sprint AC 29.AC2:** Conflicting and stale sources are visible and never silently reconciled.
- [ ] **Sprint AC 29.AC3:** Missing evidence yields Unknown/Blocked rather than invention.
- [ ] **Sprint AC 29.AC4:** Every answer citation resolves to the canonical Markdown source identity.
- [ ] **Sprint AC 29.AC5:** Retrieval remains functional with all semantic components absent.

**Gate decision:** Sprint 29 is PASS only when Story 29.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 30 - Optional Local Semantic Retrieval

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-025`.

**Sprint goal:** Add opt-in local embeddings and reranking as a measured complement to structural and lexical retrieval.

**Source coverage:** `CR-P1-RET`; inventory Section 9 semantic and hybrid requirements; Section 35B v0.2 additions.

**Dependencies:** Sprint 29; legacy dependency record: Sprint 29 (legacy S-024).

#### [ ] Story 30.1 - Optional Local Semantic Retrieval

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need optional local semantic retrieval so that AgentMage delivers the following bounded outcome: Add opt-in local embeddings and reranking as a measured complement to structural and lexical retrieval.

##### Tasks and Sub-tasks

- [ ] **Task 30.1.1 - Implement the bounded story**
  - [ ] **Sub-task 30.1.1.1** (legacy `S-025-I01`): Select an approved, manifest-pinned, non-prohibited-origin local embedding and reranking profile through the full model policy.
  - [ ] **Sub-task 30.1.1.2** (legacy `S-025-I02`): Require per-workspace opt-in with an exact included-root and file preview.
  - [ ] **Sub-task 30.1.1.3** (legacy `S-025-I03`): Keep embedding, reranking, index, and excerpt data on the workstation and prohibit source upload or remote services.
  - [ ] **Sub-task 30.1.1.4** (legacy `S-025-I04`): Key index records by content hash, source range, branch, model manifest, tokenizer, chunker, index schema, and policy.
  - [ ] **Sub-task 30.1.1.5** (legacy `S-025-I05`): Preserve source ranges from chunking through retrieval and answer assembly.
  - [ ] **Sub-task 30.1.1.6** (legacy `S-025-I06`): Implement deterministic invalidation, lexical fallback, storage classification, encryption or explicit storage tradeoff, and orphan cleanup.
  - [ ] **Sub-task 30.1.1.7** (legacy `S-025-I07`): Implement inspect, delete, and complete rebuild controls that prove deleted content is no longer retrievable.
  - [ ] **Sub-task 30.1.1.8** (legacy `S-025-I08`): Compare structural-only, lexical, semantic, and hybrid retrieval on the same corpus and budgets.

- [ ] **Task 30.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 30.1.2.1:** Approved embedding and reranking manifests.
  - [ ] **Sub-task 30.1.2.2:** Opt-in scope and storage disclosures.
  - [ ] **Sub-task 30.1.2.3:** Semantic index lifecycle implementation.
  - [ ] **Sub-task 30.1.2.4:** Comparative retrieval benchmark report.

- [ ] **Task 30.1.3 - Verify and close the story**
  - [ ] **Sub-task 30.1.3.1:** `S-025-UT01` validates embedding/reranker manifests, scope, hashes, dimensions, tokenizer, index schema, and compatibility; assert unapproved or changed components remain unavailable.
  - [ ] **Sub-task 30.1.3.2:** `S-025-UT02` embeds only approved bounded text and tests add/change/delete/rebuild/migration paths; assert no unrelated fields, secrets, or stale vectors remain.
  - [ ] **Sub-task 30.1.3.3:** `S-025-ST01` attacks embeddings with instruction content, adversarial tokens, poisoned neighbors, oversized notes, resource exhaustion, and cross-workspace canaries; assert no authority or data-boundary change.
  - [ ] **Sub-task 30.1.3.4:** `S-025-AT01` compares lexical-only, semantic-only, and combined retrieval on the same labeled corpus, hardware, limits, and metric code; assert all gains, regressions, latency, memory, and uncertainty failures are reported.
  - [ ] **Sub-task 30.1.3.5 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-004`, `SR-SUP-007`/`SR-SUP-008`, `SR-AI-006` through `SR-AI-013`, `SR-TST-006`; retain model manifests, opt-in receipt, vector lifecycle scans, benchmark code/raw scores, and decision record.

##### Story Acceptance Criteria

- [ ] **Story AC 30.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then semantic retrieval is disabled by default and can be enabled only after exact disclosure of local model, indexed fields, storage, resources, retention, deletion, and measured benefit.
- [ ] **Story AC 30.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then combined retrieval must exceed the approved improvement threshold without violating grounding/privacy/resource thresholds; otherwise deterministic retrieval remains the release behavior.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 30.AC1:** Semantic retrieval improves a declared concept-or-prose task set without reducing citation correctness.
- [ ] **Sprint AC 30.AC2:** Structural and lexical retrieval remain primary for code symbols and remain a complete fallback.
- [ ] **Sprint AC 30.AC3:** Model, tokenizer, chunker, source, branch, or policy changes invalidate affected records.
- [ ] **Sprint AC 30.AC4:** Index deletion and rebuild are complete, inspectable, and local.
- [ ] **Sprint AC 30.AC5:** Remote embedding, reranking, indexing, and upload attempts are rejected and receipted.

**Gate decision:** Sprint 30 is PASS only when Story 30.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 31 - Rolling Memory and Human-Readable Memory Files

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-026`.

**Sprint goal:** Add source-backed, inspectable, portable memory without mixing temporary context, durable facts, preferences, procedures, and operational state.

**Source coverage:** inventory Sections 10B, 10D, and 10E.

**Dependencies:** Sprint 30; legacy dependency record: Sprint 26 (legacy S-022), Sprint 29 (legacy S-024).

#### [ ] Story 31.1 - Rolling Memory and Human-Readable Memory Files

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need rolling memory and human-readable memory files so that AgentMage delivers the following bounded outcome: Add source-backed, inspectable, portable memory without mixing temporary context, durable facts, preferences, procedures, and operational state.

##### Tasks and Sub-tasks

- [ ] **Task 31.1.1 - Implement the bounded story**
  - [ ] **Sub-task 31.1.1.1** (legacy `S-026-I01`): Implement working, episodic, semantic, procedural, and preference memory types with workspace, project, conversation, source, sensitivity, confidence, and expiry namespaces.
  - [ ] **Sub-task 31.1.1.2** (legacy `S-026-I02`): Implement the memory-candidate pipeline for evidence checks, secret detection, confidence, retention, user policy, and promotion.
  - [ ] **Sub-task 31.1.1.3** (legacy `S-026-I03`): Create `MEMORY.md` as a compact linked index and per-topic Markdown memory records with shared tags and wiki links.
  - [ ] **Sub-task 31.1.1.4** (legacy `S-026-I04`): Create bounded `WORKING.md` with complete-load limits and end-of-task compaction into reviewed durable candidates.
  - [ ] **Sub-task 31.1.1.5** (legacy `S-026-I05`): Implement selective long-term loading by tag, link, source, relevance, and context budget.
  - [ ] **Sub-task 31.1.1.6** (legacy `S-026-I06`): Implement contradiction preservation, supersession, correction, decay, inspection, deletion, and last-verification state.
  - [ ] **Sub-task 31.1.1.7** (legacy `S-026-I07`): Implement encrypted versioned export and import without machine-specific paths or secrets.
  - [ ] **Sub-task 31.1.1.8** (legacy `S-026-I08`): Test project isolation, stale summaries, interrupted writes, corrupt indexes, backup restore, and migration between machines.

- [ ] **Task 31.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 31.1.2.1:** Memory schemas and candidate-policy engine.
  - [ ] **Sub-task 31.1.2.2:** Human-readable memory templates and lint.
  - [ ] **Sub-task 31.1.2.3:** Memory inspection, correction, supersession, export, and deletion interfaces.
  - [ ] **Sub-task 31.1.2.4:** Recovery and isolation test bundle.

- [ ] **Task 31.1.3 - Verify and close the story**
  - [ ] **Sub-task 31.1.3.1:** `S-026-UT01` classifies candidate facts as temporary context, durable fact, preference, procedure, unresolved claim, contradiction, or prohibited content; assert source requirements and confidence/state rules.
  - [ ] **Sub-task 31.1.3.2:** `S-026-UT02` exercises approve, reject, edit, supersede, correct, export, expire, hold, and delete operations; assert linked indexes/views update and history remains bounded and attributable.
  - [ ] **Sub-task 31.1.3.3:** `S-026-ST01` attempts secret capture, inferred-sensitive memory, cross-person/project leakage, prompt-based self-promotion, and source-free durable claims; assert no automatic durable memory.
  - [ ] **Sub-task 31.1.3.4:** `S-026-RT01` interrupts memory-file and index updates, restores backups, and resolves simultaneous edits; assert valid Markdown, no lost user text, exact conflict preservation, and deterministic rebuild.
  - [ ] **Sub-task 31.1.3.5 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-004`, `SR-DAT-010`, `SR-AI-003`/`SR-AI-007`/`SR-AI-008`, `SR-CIV-001` through `SR-CIV-005`; retain candidate decisions, canary scans, lifecycle receipts, conflict files, and recovery hashes.

##### Story Acceptance Criteria

- [ ] **Story AC 31.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no durable memory exists without a visible source-backed candidate and the required user/policy decision; the model cannot remember, correct, or delete facts on its own authority.
- [ ] **Story AC 31.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a user can inspect, search, correct, supersede, export, and delete every memory item and trace it to source, decision, date, scope, and current status.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 31.AC1:** No unsupported model assertion can become durable memory.
- [ ] **Sprint AC 31.AC2:** Secrets and restricted content cannot enter Markdown memory.
- [ ] **Sprint AC 31.AC3:** Memory retrieval explains source, reason, last verification, sensitivity, and supersession.
- [ ] **Sprint AC 31.AC4:** Unrelated workspace or project memory never enters a task.
- [ ] **Sprint AC 31.AC5:** Export, import, backup, and restore preserve evidence identities without credentials or machine-specific authority.

**Gate decision:** Sprint 31 is PASS only when Story 31.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 32 - Conversation Search and Branching

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-027`, part 1 of 2.

**Sprint goal:** Deliver conversation search and branching as a bounded part of the legacy goal: Provide complete local conversation search, exact-point branching, encrypted archives, and privacy-controlled evidence export.

**Source coverage:** inventory Section 10C; `CR-P1-SES`.

**Dependencies:** Sprint 31; legacy dependency record: Sprint 22 (legacy S-020), Sprint 31 (legacy S-026).

#### [ ] Story 32.1 - Conversation Search and Branching

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need conversation search and branching so that AgentMage delivers the following bounded outcome: Provide complete local conversation search, exact-point branching, encrypted archives, and privacy-controlled evidence export.

##### Tasks and Sub-tasks

- [ ] **Task 32.1.1 - Implement the bounded story**
  - [ ] **Sub-task 32.1.1.1** (legacy `S-027-I01`): Store sensitivity-labeled conversation identity, title, local date fields, workspace, project, model, status, parent, current turn, attachments by reference, grants, receipts, and checkpoints in canonical SQLite.
  - [ ] **Sub-task 32.1.1.2** (legacy `S-027-I02`): Implement filter, full-text search, timeline, read-only history, relationship view, rename, pin, archive, tag, retention, and approval-gated deletion.
  - [ ] **Sub-task 32.1.1.3** (legacy `S-027-I03`): Resume the latest checkpoint in place and branch from an exact historical turn without rewriting the original.
  - [ ] **Sub-task 32.1.1.4** (legacy `S-027-I04`): Compare current and recorded files, instructions, repository, model, and permissions before resume.
  - [ ] **Sub-task 32.1.1.5** (legacy `S-027-I05`): Preserve citation IDs, receipt IDs, source hashes, and original evidence through compaction and branching.

- [ ] **Task 32.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 32.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 32.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 32.1.3 - Verify and close the story**
  - [ ] **Sub-task 32.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 32.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 32.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 32.1.3.4 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-004`, `SR-DAT-007`, `SR-DAT-010` through `SR-DAT-012`, `SR-CIV-003` through `SR-CIV-005`, `SR-OPS-003`; retain archive integrity tests, branch graphs, disclosure previews, canary reports, and deletion/recovery evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 32.1.AC1:** Given the approved dependencies and source requirements for `S-027-I01`, `S-027-I02`, `S-027-I03`, `S-027-I04`, and `S-027-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 32.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-027-I01`, `S-027-I02`, `S-027-I03`, `S-027-I04`, and `S-027-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 32.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 32.AC1:** Every numbered implementation sub-task in Story 32.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 32.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 32.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 32.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 32.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 32 is PASS only when Story 32.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 33 - Private Archives and Evidence Bundles

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-027`, part 2 of 2.

**Sprint goal:** Deliver private archives and evidence bundles as a bounded part of the legacy goal: Provide complete local conversation search, exact-point branching, encrypted archives, and privacy-controlled evidence export.

**Source coverage:** inventory Section 10C; `CR-P1-SES`.

**Dependencies:** Sprint 32; legacy dependency record: Sprint 22 (legacy S-020), Sprint 31 (legacy S-026).

#### [ ] Story 33.1 - Private Archives and Evidence Bundles

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need private archives and evidence bundles so that AgentMage delivers the following bounded outcome: Provide complete local conversation search, exact-point branching, encrypted archives, and privacy-controlled evidence export.

##### Tasks and Sub-tasks

- [ ] **Task 33.1.1 - Implement the bounded story**
  - [ ] **Sub-task 33.1.1.1** (legacy `S-027-I06`): Implement encrypted local archives with inspect, restore, export, retention, and delete controls.
  - [ ] **Sub-task 33.1.1.2** (legacy `S-027-I07`): Implement redacted evidence bundles with claims, states, citations, methods, receipts, manifests, constraints, and user-selected excerpts.
  - [ ] **Sub-task 33.1.1.3** (legacy `S-027-I08`): Show an exact disclosure preview and exclude secrets, unrelated private text, hidden prompts, and unapproved content.

- [ ] **Task 33.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 33.1.2.1:** Conversation search and branching APIs.
  - [ ] **Sub-task 33.1.2.2:** Encrypted archive and evidence-bundle formats.
  - [ ] **Sub-task 33.1.2.3:** Disclosure preview and redaction report.
  - [ ] **Sub-task 33.1.2.4:** Resume, corruption, simultaneous-access, and deletion test results.

- [ ] **Task 33.1.3 - Verify and close the story**
  - [ ] **Sub-task 33.1.3.1:** `S-027-UT01` searches exact text, metadata, date, project, evidence state, and branch ancestry over empty/large/corrupt archives; assert deterministic bounded results and resolvable message identities.
  - [ ] **Sub-task 33.1.3.2:** `S-027-UT02` branches at every event type and compares parent/child histories; assert immutable shared prefix, independent continuation, and no duplicated side effects or grants.
  - [ ] **Sub-task 33.1.3.3:** `S-027-ST01` exports bundles containing secret, private, stale, denied, copyrighted, and unrelated canaries; assert disclosure preview, policy redaction, explicit omissions, and no hidden metadata leakage.
  - [ ] **Sub-task 33.1.3.4:** `S-027-RT01` crashes during archive, branch, simultaneous access, export, deletion, and restore; assert encryption, canonical ordering, atomic state, retention consistency, and no orphaned content.
  - [ ] **Sub-task 33.1.3.5 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-004`, `SR-DAT-007`, `SR-DAT-010` through `SR-DAT-012`, `SR-CIV-003` through `SR-CIV-005`, `SR-OPS-003`; retain archive integrity tests, branch graphs, disclosure previews, canary reports, and deletion/recovery evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 33.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every conversation event and branch has stable identity, provenance, retention, encryption, and deletion behavior; searches and exports never become operational authority.
- [ ] **Story AC 33.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then evidence bundles are reproducible from selected content, disclose exactly what leaves the local boundary, and exclude all unapproved data after independent canary scanning.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 33.AC1:** Exact-point resume and branch fixtures preserve the original and continue from the selected evidence state.
- [ ] **Sprint AC 33.AC2:** Every summary and branch can reopen its original citations and receipts.
- [ ] **Sprint AC 33.AC3:** Deletion removes eligible records and indexes without breaking retained evidence obligations.
- [ ] **Sprint AC 33.AC4:** Exports contain only previewed fields and no secrets or unrelated workspace content.
- [ ] **Sprint AC 33.AC5:** Every shell reads and writes conversation state only through the kernel.

**Gate decision:** Sprint 33 is PASS only when Story 33.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 34 - Knowledge Tasks, Declarative Skills, and v0.2 Release

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-028`.

**Sprint goal:** Complete the read-only knowledge release with task workflows and trust-gated declarative skills.

**Source coverage:** inventory Sections 11, 28 declarative foundation, 31, and 32 knowledge guides; `CR-P1-SKL`; `G-V0.2`.

**Dependencies:** Sprint 33; legacy dependency record: Sprint 26 (legacy S-022) through Sprint 33 (legacy S-027).

#### [ ] Story 34.1 - Knowledge Tasks, Declarative Skills, and v0.2 Release

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need knowledge tasks, declarative skills, and v0.2 release so that AgentMage delivers the following bounded outcome: Complete the read-only knowledge release with task workflows and trust-gated declarative skills.

##### Tasks and Sub-tasks

- [ ] **Task 34.1.1 - Implement the bounded story**
  - [ ] **Sub-task 34.1.1.1** (legacy `S-028-I01`): Implement task identity, owner, project, source, evidence, dependencies, status, blocker, next action, priority, duplicate detection, and deferred views.
  - [ ] **Sub-task 34.1.1.2** (legacy `S-028-I02`): Implement evidence-required task transitions and source links to canonical records.
  - [ ] **Sub-task 34.1.1.3** (legacy `S-028-I03`): Define declarative skill records with identity, source, hash, signer or provenance, license, version, compatibility, purpose, files, requested scope, and trust state.
  - [ ] **Sub-task 34.1.1.4** (legacy `S-028-I04`): Load only prompts, schemas, examples, and templates; give declarative skills no filesystem, shell, secret, network, connector, or approval authority.
  - [ ] **Sub-task 34.1.1.5** (legacy `S-028-I05`): Implement visible instruction precedence, conflict reporting, bounded context contribution, and skill-influence receipts.
  - [ ] **Sub-task 34.1.1.6** (legacy `S-028-I06`): Implement read-only Daily Setup, Daily Briefing, Issue Intake, Handoff, Meeting Cleanup, Repository Learning, Plain-Workspace Steward, and Obsidian Vault Steward skills.
  - [ ] **Sub-task 34.1.1.7** (legacy `S-028-I07`): Run malicious instruction, hidden-tool, path-expansion, injection, excessive-context, and conflicting-policy skill fixtures.
  - [ ] **Sub-task 34.1.1.8** (legacy `S-028-I08`): Publish knowledge, vault, memory, conversation, retrieval, privacy, task, and skill guides.

- [ ] **Task 34.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 34.1.2.1:** Knowledge task store and views.
  - [ ] **Sub-task 34.1.2.2:** Declarative skill registry and initial skill pack.
  - [ ] **Sub-task 34.1.2.3:** v0.2 acceptance and migration bundle.
  - [ ] **Sub-task 34.1.2.4:** v0.2 capability matrix, limitations, and release notes.

- [ ] **Task 34.1.3 - Verify and close the story**
  - [ ] **Sub-task 34.1.3.1:** `S-028-UT01` validates task and declarative-skill schemas with unknown capabilities, hidden instructions, executable content, unbounded inputs, vague completion, and unsupported versions; assert disabled status.
  - [ ] **Sub-task 34.1.3.2:** `S-028-ST01` attempts skill-based grant creation, tool registration, network access, code execution, workspace expansion, memory promotion, and write behavior; assert declarative skills remain authority-free.
  - [ ] **Sub-task 34.1.3.3:** `S-028-IT01` completes every promoted knowledge workflow through native Chat and CLI-compatible kernel contracts using lexical and approved optional semantic paths; assert evidence parity and source-file invariance.
  - [ ] **Sub-task 34.1.3.4:** `S-028-AT01` upgrades v0.1 data, runs clean/offline/privacy/recovery/accessibility suites on each platform, then downgrades or rolls back; assert no canonical knowledge loss or hidden capability.
  - [ ] **Sub-task 34.1.3.5 - Product security evidence:** Map applicable `SR-ACC-*`, `SR-DAT-*`, `SR-AI-*`, `SR-OPS-*`, `SR-TST-*`, and `SR-CIV-*`; retain skill manifests, workflow traces, cross-interface comparisons, migration/rollback results, and signed release decision.

##### Story Acceptance Criteria

- [ ] **Story AC 34.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every v0.2 task and skill is source-backed, bounded, inspectable, removable, non-executable, and incapable of extending grants or trusted instruction channels.
- [ ] **Story AC 34.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then `G-V0.2` closes only when knowledge, vault, retrieval, memory, conversation, migration, privacy, and read-only invariance evidence is current on all supported platforms.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 34.AC1:** Plain-folder and Obsidian workflows pass identical canonical-record and task acceptance suites.
- [ ] **Sprint AC 34.AC2:** Optional semantic retrieval remains local, opt-in, deletable, and non-authoritative.
- [ ] **Sprint AC 34.AC3:** Skills cannot grant tools, relax policy, expand roots, or execute code.
- [ ] **Sprint AC 34.AC4:** Every v0.2 user-file operation remains read-only.
- [ ] **Sprint AC 34.AC5:** `G-V0.2` closes only when knowledge, retrieval, memory, task, privacy, backup, recovery, and documentation gates pass.

**Gate decision:** Sprint 34 is PASS only when Story 34.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 3 - v0.3 - Controlled Writes

### [ ] Sprint 35 - Exact-Preimage Write Approval

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-029`, part 1 of 2.

**Sprint goal:** Deliver exact-preimage write approval as a bounded part of the legacy goal: Define and implement the exact transaction every state-changing file operation must follow.

**Source coverage:** `CR-P1-WRT`; inventory Sections 5 and 14B; Section 35B v0.3 additions.

**Dependencies:** Sprint 34; legacy dependency record: `G-V0.2`, Sprint 5 (legacy S-005), Sprint 6 (legacy S-006), Sprint 21 (legacy S-019).

#### [ ] Story 35.1 - Exact-Preimage Write Approval

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need exact-preimage write approval so that AgentMage delivers the following bounded outcome: Define and implement the exact transaction every state-changing file operation must follow.

##### Tasks and Sub-tasks

- [ ] **Task 35.1.1 - Implement the bounded story**
  - [ ] **Sub-task 35.1.1.1** (legacy `S-029-I01`): Observe and hash every exact source preimage under the current read grant.
  - [ ] **Sub-task 35.1.1.2** (legacy `S-029-I02`): Generate proposed operations into a shadow change set outside user-owned files.
  - [ ] **Sub-task 35.1.1.3** (legacy `S-029-I03`): Validate patch applicability, syntax, paths, encoding, line endings, duplicate operations, generated-file policy, and expected postimages.
  - [ ] **Sub-task 35.1.1.4** (legacy `S-029-I04`): Render the complete diff, rationale, files, behavior, verification plan, risks, rollback, and unverified assumptions.
  - [ ] **Sub-task 35.1.1.5** (legacy `S-029-I05`): Issue a short-lived single-use grant bound to the exact change-set identity, files, operations, workspace, expiry, and permitted verification.
  - [ ] **Sub-task 35.1.1.6** (legacy `S-029-I06`): Re-read every preimage immediately before apply and invalidate the grant on any mismatch.

- [ ] **Task 35.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 35.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 35.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 35.1.3 - Verify and close the story**
  - [ ] **Sub-task 35.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 35.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 35.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 35.1.3.4 - Product security evidence:** Map `SR-ACC-001` through `SR-ACC-007`, `SR-DAT-002`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-005`/`SR-TST-011`/`SR-TST-012`; retain transition/property results, attack traces, pre/post hashes, restoration proof, and independent transaction review.

##### Story Acceptance Criteria

- [ ] **Story AC 35.1.AC1:** Given the approved dependencies and source requirements for `S-029-I01`, `S-029-I02`, `S-029-I03`, `S-029-I04`, `S-029-I05`, and `S-029-I06`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 35.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-029-I01`, `S-029-I02`, `S-029-I03`, `S-029-I04`, `S-029-I05`, and `S-029-I06`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 35.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 35.AC1:** Every numbered implementation sub-task in Story 35.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 35.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 35.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 35.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 35.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 35 is PASS only when Story 35.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 36 - Atomic Write Application and Rollback

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-029`, part 2 of 2.

**Sprint goal:** Deliver atomic write application and rollback as a bounded part of the legacy goal: Define and implement the exact transaction every state-changing file operation must follow.

**Source coverage:** `CR-P1-WRT`; inventory Sections 5 and 14B; Section 35B v0.3 additions.

**Dependencies:** Sprint 35; legacy dependency record: `G-V0.2`, Sprint 5 (legacy S-005), Sprint 6 (legacy S-006), Sprint 21 (legacy S-019).

#### [ ] Story 36.1 - Atomic Write Application and Rollback

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need atomic write application and rollback so that AgentMage delivers the following bounded outcome: Define and implement the exact transaction every state-changing file operation must follow.

##### Tasks and Sub-tasks

- [ ] **Task 36.1.1 - Implement the bounded story**
  - [ ] **Sub-task 36.1.1.1** (legacy `S-029-I07`): Apply atomically where supported and restore all already-applied preimages if an operation fails.
  - [ ] **Sub-task 36.1.1.2** (legacy `S-029-I08`): Emit per-operation proposed, approved, applied, verified, failed, rolled-back, or superseded receipts with hashes.
  - [ ] **Sub-task 36.1.1.3** (legacy `S-029-I09`): Require separately bounded grants for formatters, tests, builds, migrations, or other post-write commands.
  - [ ] **Sub-task 36.1.1.4** (legacy `S-029-I10`): Implement rollback as a fresh reviewed transaction that refuses to overwrite later user changes.

- [ ] **Task 36.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 36.1.2.1:** Write-transaction state machine and schemas.
  - [ ] **Sub-task 36.1.2.2:** Shadow change-set and preview format.
  - [ ] **Sub-task 36.1.2.3:** Atomic application and restoration engine.
  - [ ] **Sub-task 36.1.2.4:** Stale, partial-failure, collision, uncertain-result, and rollback fixtures.

- [ ] **Task 36.1.3 - Verify and close the story**
  - [ ] **Sub-task 36.1.3.1:** `S-029-UT01` exercises every legal and illegal write-transaction transition from request through validate/stage/preview/approve/revalidate/apply/verify/commit-or-restore; assert deterministic state and receipt.
  - [ ] **Sub-task 36.1.3.2:** `S-029-UT02` mutates target, arguments, bytes, preimage, metadata, preview, policy, grant, workspace, and expected side effects after preview; assert stale approval and zero target change.
  - [ ] **Sub-task 36.1.3.3:** `S-029-ST01` races file replacement, symlink/alias swap, rename, concurrent writer, mount change, grant replay, and approval replay at each boundary; assert descriptor identity and atomic consumption prevent unintended write.
  - [ ] **Sub-task 36.1.3.4:** `S-029-RT01` crashes before and after every staging, application, verification, restoration, and durable-state transition; assert prior bytes or exact approved bytes, never an unexplained partial state.
  - [ ] **Sub-task 36.1.3.5 - Product security evidence:** Map `SR-ACC-001` through `SR-ACC-007`, `SR-DAT-002`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-005`/`SR-TST-011`/`SR-TST-012`; retain transition/property results, attack traces, pre/post hashes, restoration proof, and independent transaction review.

##### Story Acceptance Criteria

- [ ] **Story AC 36.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no state-changing file operation can bypass exact preview, current preimage, single-use grant, immediate revalidation, atomic apply, postcondition verification, and terminal receipt.
- [ ] **Story AC 36.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then any stale, uncertain, colliding, partial, cancelled, or failed outcome stops dependent work and preserves enough evidence to restore or reconcile without guessing.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 36.AC1:** No write reaches a user path before exact preview and grant consumption.
- [ ] **Sprint AC 36.AC2:** Any preimage, operation, preview, policy, path, or scope change invalidates approval.
- [ ] **Sprint AC 36.AC3:** Partial failure leaves either the complete approved postimage or restored preimages, never an unreported mixed state.
- [ ] **Sprint AC 36.AC4:** Rollback preserves concurrent user work and requires new approval.
- [ ] **Sprint AC 36.AC5:** Every operation status and hash is reconstructable from receipts.

**Gate decision:** Sprint 36 is PASS only when Story 36.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 37 - File Creation, Patch, Copy, Move, and Delete Controls

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-030`.

**Sprint goal:** Implement the bounded filesystem mutation primitives on top of the controlled write transaction.

**Source coverage:** inventory Section 7 write operations and Section 5 protected-file rules.

**Dependencies:** Sprint 36; legacy dependency record: Sprint 36 (legacy S-029).

#### [ ] Story 37.1 - File Creation, Patch, Copy, Move, and Delete Controls

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need file creation, patch, copy, move, and delete controls so that AgentMage delivers the following bounded outcome: Implement the bounded filesystem mutation primitives on top of the controlled write transaction.

##### Tasks and Sub-tasks

- [ ] **Task 37.1.1 - Implement the bounded story**
  - [ ] **Sub-task 37.1.1.1** (legacy `S-030-I01`): Implement new-file creation in approved roots with collision detection, staging, content preview, classification, and postimage verification.
  - [ ] **Sub-task 37.1.1.2** (legacy `S-030-I02`): Implement exact-preimage patching with structured patch parsing and no ad hoc target replacement.
  - [ ] **Sub-task 37.1.1.3** (legacy `S-030-I03`): Implement copy with source and destination hashes and overwrite refusal by default.
  - [ ] **Sub-task 37.1.1.4** (legacy `S-030-I04`): Implement move with source identity, destination collision checks, atomic behavior where supported, and rollback record.
  - [ ] **Sub-task 37.1.1.5** (legacy `S-030-I05`): Implement trash-first delete only after a separate high-risk grant and explicit target preview.
  - [ ] **Sub-task 37.1.1.6** (legacy `S-030-I06`): Implement atomic temporary-file writes, permission preservation, post-write diff, metadata, and hash reporting.
  - [ ] **Sub-task 37.1.1.7** (legacy `S-030-I07`): Protect instructions, handoffs, source records, secrets, Git metadata, canonical stores, and unrelated uncommitted work.
  - [ ] **Sub-task 37.1.1.8** (legacy `S-030-I08`): Enforce file-count, byte, depth, operation-count, and output limits for each transaction.

- [ ] **Task 37.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 37.1.2.1:** Controlled filesystem write pack.
  - [ ] **Sub-task 37.1.2.2:** Operation-specific preview and receipt schemas.
  - [ ] **Sub-task 37.1.2.3:** Cross-platform atomicity and metadata report.
  - [ ] **Sub-task 37.1.2.4:** Protected-file and collision attack corpus.

- [ ] **Task 37.1.3 - Verify and close the story**
  - [ ] **Sub-task 37.1.3.1:** `S-030-UT01` covers create, exact patch, copy, move, and delete with empty, nominal, maximum, existing, missing, wrong-type, case/Unicode collision, and metadata variants; assert documented bytes and metadata only.
  - [ ] **Sub-task 37.1.3.2:** `S-030-UT02` verifies operation-specific previews against canonical serialized actions; alter one source/destination/hunk/delete target/metadata field and assert approval invalidation.
  - [ ] **Sub-task 37.1.3.3:** `S-030-ST01` targets repository control files, application state, secrets, sockets/devices, out-of-root paths, links, aliases, hard links, and files changed concurrently; assert protected-path denial and zero collateral effect.
  - [ ] **Sub-task 37.1.3.4:** `S-030-RT01` injects disk-full, permission, interruption, process death, verification mismatch, and restoration failure on each operation; assert atomic outcome or visible blocked recovery state.
  - [ ] **Sub-task 37.1.3.5 - Product security evidence:** Map `SR-PLT-004`, `SR-ACC-002` through `SR-ACC-006`, `SR-OPS-001`, `SR-TST-004`/`SR-TST-005`; retain operation matrix, preview digests, filesystem snapshots, collision corpus, recovery traces, and platform comparison.

##### Story Acceptance Criteria

- [ ] **Story AC 37.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then each primitive changes exactly the approved source/target bytes and declared metadata, with no implicit parent creation, overwrite, recursive deletion, wildcard expansion, or neighboring-file change.
- [ ] **Story AC 37.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then cross-platform behavior is equivalent where promised and explicitly documented where filesystem semantics prevent parity; no platform silently weakens safety.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 37.AC1:** Every primitive passes success, denial, collision, stale-source, partial-failure, cancellation, and rollback tests.
- [ ] **Sprint AC 37.AC2:** Unapproved overwrite and delete remain impossible.
- [ ] **Sprint AC 37.AC3:** Atomic writes preserve unrelated content, expected permissions, encoding, and line endings.
- [ ] **Sprint AC 37.AC4:** Source and destination hashes match every accepted operation receipt.
- [ ] **Sprint AC 37.AC5:** Write workers remain bounded by operating-system isolation and exact granted paths.

**Gate decision:** Sprint 37 is PASS only when Story 37.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 38 - Controlled Markdown and Knowledge Writes

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-031`.

**Sprint goal:** Enable safe updates to canonical Markdown knowledge without hidden state, bulk reorganization, or lost source structure.

**Source coverage:** inventory Sections 8, 8A, 10D, 11, and 16 write items.

**Dependencies:** Sprint 37; legacy dependency record: Sprint 26 (legacy S-022), Sprint 28 (legacy S-023), Sprint 36 (legacy S-029), Sprint 37 (legacy S-030).

#### [ ] Story 38.1 - Controlled Markdown and Knowledge Writes

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need controlled markdown and knowledge writes so that AgentMage delivers the following bounded outcome: Enable safe updates to canonical Markdown knowledge without hidden state, bulk reorganization, or lost source structure.

##### Tasks and Sub-tasks

- [ ] **Task 38.1.1 - Implement the bounded story**
  - [ ] **Sub-task 38.1.1.1** (legacy `S-031-I01`): Implement a structure-preserving Markdown parser and writer for headings, lists, tables, code fences, links, frontmatter, raw notes, and line endings.
  - [ ] **Sub-task 38.1.1.2** (legacy `S-031-I02`): Implement per-file knowledge create and update previews with stable identifiers and source hashes.
  - [ ] **Sub-task 38.1.1.3** (legacy `S-031-I03`): Preserve unrelated sections and every meeting note's raw source section.
  - [ ] **Sub-task 38.1.1.4** (legacy `S-031-I04`): Implement wiki-link and frontmatter-safe note creation without launching Obsidian or requiring a plugin.
  - [ ] **Sub-task 38.1.1.5** (legacy `S-031-I05`): Update derived knowledge indexes only after canonical Markdown commits successfully.
  - [ ] **Sub-task 38.1.1.6** (legacy `S-031-I06`): Implement note-write collision detection, stale-index invalidation, broken-link checks, and duplicate-record checks.
  - [ ] **Sub-task 38.1.1.7** (legacy `S-031-I07`): Require explicit approval for moves, renames, reorganizations, supersession, and deletion.
  - [ ] **Sub-task 38.1.1.8** (legacy `S-031-I08`): Add task, decision, commitment, correspondence, meeting, handoff, and memory write workflows through the same transaction.

- [ ] **Task 38.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 38.1.2.1:** Controlled Markdown writer.
  - [ ] **Sub-task 38.1.2.2:** Knowledge-record write adapters.
  - [ ] **Sub-task 38.1.2.3:** Structure-preservation and raw-notes fixtures.
  - [ ] **Sub-task 38.1.2.4:** Index consistency and collision reports.

- [ ] **Task 38.1.3 - Verify and close the story**
  - [ ] **Sub-task 38.1.3.1:** `S-031-UT01` updates frontmatter, heading, block, list, task, table, link, and bounded text regions across formatting variants; assert exact requested semantic change and byte preservation elsewhere.
  - [ ] **Sub-task 38.1.3.2:** `S-031-UT02` processes malformed Markdown, duplicate headings/keys, aliases, comments, raw notes, line endings, encodings, case/Unicode collisions, and unsupported constructs; assert safe refusal or explicit fidelity warning.
  - [ ] **Sub-task 38.1.3.3:** `S-031-ST01` attempts bulk reorganization, hidden metadata insertion, source erasure, link expansion outside scope, prompt-driven memory promotion, and automatic Obsidian action; assert denial or exact additional approval.
  - [ ] **Sub-task 38.1.3.4:** `S-031-RT01` crashes during source write/index update and races external note edits; assert source remains canonical, conflicts are preserved, and indexes rebuild to current bytes.
  - [ ] **Sub-task 38.1.3.5 - Product security evidence:** Map `SR-ACC-004` through `SR-ACC-008`, `SR-DAT-001` through `SR-DAT-003`, `SR-CIV-003`/`SR-CIV-004`, `SR-TST-004`/`SR-TST-005`; retain parser/writer round trips, scoped diffs, collision results, index hashes, and recovery evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 38.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a before/after structural and byte diff proves no unrelated note content, ordering, links, formatting, metadata, or raw-note material changed.
- [ ] **Story AC 38.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every derived index and memory/task view either matches the committed canonical Markdown revision or is visibly stale and queued for deterministic rebuild.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 38.AC1:** Approved changes alter only the previewed records and lines.
- [ ] **Sprint AC 38.AC2:** Raw notes, code fences, links, frontmatter, identifiers, and unrelated formatting survive round trips.
- [ ] **Sprint AC 38.AC3:** Failed canonical writes never update the derived index.
- [ ] **Sprint AC 38.AC4:** Plain-folder and Obsidian adapters produce equivalent domain behavior.
- [ ] **Sprint AC 38.AC5:** No agent-initiated bulk reorganization, silent supersession, or unapproved deletion is possible.

**Gate decision:** Sprint 38 is PASS only when Story 38.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 39 - Write Privacy, Recovery, and Audit

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-032`.

**Sprint goal:** Prove that state changes preserve privacy, recoverability, and truthful completion under failure and concurrency.

**Source coverage:** inventory Sections 5, 10, 10E, 22, 23, and 31 write fixtures.

**Dependencies:** Sprint 38; legacy dependency record: Sprint 36 (legacy S-029), Sprint 37 (legacy S-030), Sprint 38 (legacy S-031).

#### [ ] Story 39.1 - Write Privacy, Recovery, and Audit

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need write privacy, recovery, and audit so that AgentMage delivers the following bounded outcome: Prove that state changes preserve privacy, recoverability, and truthful completion under failure and concurrency.

##### Tasks and Sub-tasks

- [ ] **Task 39.1.1 - Implement the bounded story**
  - [ ] **Sub-task 39.1.1.1** (legacy `S-032-I01`): Add write-aware checkpoints before and after each state-changing transaction.
  - [ ] **Sub-task 39.1.1.2** (legacy `S-032-I02`): Bind canonical action state, consumed grant, file receipts, evidence, index updates, and next checkpoint atomically where their stores permit.
  - [ ] **Sub-task 39.1.1.3** (legacy `S-032-I03`): Add secret scanning and classification before previews, staging, receipts, model context, and persistence.
  - [ ] **Sub-task 39.1.1.4** (legacy `S-032-I04`): Redact sensitive values without storing removed content.
  - [ ] **Sub-task 39.1.1.5** (legacy `S-032-I05`): Implement crash recovery for staging, application, index update, receipt persistence, and rollback boundaries.
  - [ ] **Sub-task 39.1.1.6** (legacy `S-032-I06`): Implement concurrent-user-edit, concurrent-session, disk-full, permission-change, moved-root, and lost-secret-store fixtures.
  - [ ] **Sub-task 39.1.1.7** (legacy `S-032-I07`): Generate a human-readable state-change audit summary with exact files, operations, validation, failures, and rollback status.

- [ ] **Task 39.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 39.1.2.1:** Write-aware checkpoint schema.
  - [ ] **Sub-task 39.1.2.2:** Recovery and concurrency test suite.
  - [ ] **Sub-task 39.1.2.3:** Redacted state-change audit report.
  - [ ] **Sub-task 39.1.2.4:** Orphan staging and cleanup diagnostics.

- [ ] **Task 39.1.3 - Verify and close the story**
  - [ ] **Sub-task 39.1.3.1:** `S-032-UT01` validates write-aware checkpoint, receipt, rollback, retention, and cleanup schemas for every terminal/intermediate state; assert correlation and no ambiguous completion.
  - [ ] **Sub-task 39.1.3.2:** `S-032-ST01` injects secrets/private excerpts into targets, diffs, previews, errors, staging, logs, checkpoints, backups, diagnostics, and exports; assert typed redaction and policy-bounded storage.
  - [ ] **Sub-task 39.1.3.3:** `S-032-RT01` combines concurrent edits, cancellation, timeout, disk full, crash, stale grant, uncertain result, rollback failure, and restart; assert no repeated write and a deterministic recovery instruction.
  - [ ] **Sub-task 39.1.3.4:** `S-032-IT01` scans all durable/temporary roots after every outcome and retention transition; assert no orphan staging, expired content, undeclared copy, or inaccessible rollback material.
  - [ ] **Sub-task 39.1.3.5 - Product security evidence:** Map `SR-DAT-002` through `SR-DAT-004`, `SR-DAT-010` through `SR-DAT-012`, `SR-OPS-001` through `SR-OPS-007`, `SR-TST-005`; retain canary scans, checkpoint/recovery matrix, cleanup inventory, retention results, and audit-chain verification.

##### Story Acceptance Criteria

- [ ] **Story AC 39.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every write can be attributed, reconstructed, verified, and where promised restored without logging or exporting unapproved file content.
- [ ] **Story AC 39.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then recovery and cleanup are idempotent, bounded, separately receipted, and never broaden authority or silently discard a user conflict.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 39.AC1:** Crash injection never causes an unreceipted write or repeated completed write.
- [ ] **Sprint AC 39.AC2:** Concurrent changes stop for review and are never silently overwritten or merged.
- [ ] **Sprint AC 39.AC3:** Secret canaries do not enter previews, logs, exports, or unauthorized model context.
- [ ] **Sprint AC 39.AC4:** Orphan staging is detectable, attributable, and safely removable.
- [ ] **Sprint AC 39.AC5:** Completion is reported only after postimage and receipt verification.

**Gate decision:** Sprint 39 is PASS only when Story 39.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 40 - v0.3 Write Release Gate

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-033`.

**Sprint goal:** Release controlled file writes without enabling generic shell, commits, publication, or unattended changes.

**Source coverage:** inventory release sequence and Section 35B v0.3; controlled-write documentation in Section 32.

**Dependencies:** Sprint 39; legacy dependency record: Sprint 36 (legacy S-029) through Sprint 39 (legacy S-032).

#### [ ] Story 40.1 - v0.3 Write Release Gate

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need v0.3 write release gate so that AgentMage delivers the following bounded outcome: Release controlled file writes without enabling generic shell, commits, publication, or unattended changes.

##### Tasks and Sub-tasks

- [ ] **Task 40.1.1 - Implement the bounded story**
  - [ ] **Sub-task 40.1.1.1** (legacy `S-033-I01`): Run the complete write, path, grant, sandbox, privacy, crash, concurrency, stale-preimage, rollback, and evidence suites on all reference platforms.
  - [ ] **Sub-task 40.1.1.2** (legacy `S-033-I02`): Verify that v0.1 and v0.2 read-only behavior is unchanged when the write pack is disabled.
  - [ ] **Sub-task 40.1.1.3** (legacy `S-033-I03`): Publish write preview, approval, staging, rollback, recovery, protected-file, and limitation guides.
  - [ ] **Sub-task 40.1.1.4** (legacy `S-033-I04`): Publish the exact capability delta and prove generic shell, Git publication, connectors, schedules, and unattended writes remain disabled.
  - [ ] **Sub-task 40.1.1.5** (legacy `S-033-I05`): Perform clean upgrade, downgrade, backup, restore, and uninstall checks.

- [ ] **Task 40.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 40.1.2.1:** v0.3 packages and write-pack manifest.
  - [ ] **Sub-task 40.1.2.2:** Cross-platform write acceptance bundle.
  - [ ] **Sub-task 40.1.2.3:** Upgrade, downgrade, and recovery report.
  - [ ] **Sub-task 40.1.2.4:** v0.3 release notes and capability matrix.

- [ ] **Task 40.1.3 - Verify and close the story**
  - [ ] **Sub-task 40.1.3.1:** `S-033-IT01` executes create/patch/copy/move/delete and Markdown/knowledge updates through every supported interface and platform; assert shared grants, previews, receipts, atomicity, and exact diffs.
  - [ ] **Sub-task 40.1.3.2:** `S-033-ST01` attempts generic shell, Git commit/push, network publication, unattended write, wildcard approval, bulk reorganization, and extension/model bypass; assert all remain absent or denied.
  - [ ] **Sub-task 40.1.3.3:** `S-033-RT01` upgrades v0.2 state, exercises writes, rolls back/downgrades, restores backups, and resumes interrupted operations; assert canonical files and evidence remain valid.
  - [ ] **Sub-task 40.1.3.4:** `S-033-AT01` forces each write-security, privacy, collision, recovery, and clean-platform threshold to fail; assert package signing and `G-V0.3` closure are blocked.
  - [ ] **Sub-task 40.1.3.5 - Product security evidence:** Map applicable `SR-ACC-*`, `SR-DAT-*`, `SR-OPS-*`, `SR-TST-*`, and `SR-CIV-*`; retain cross-platform write bundle, prohibited-capability results, migration/rollback evidence, release manifest, and independent gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 40.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every promoted write primitive passes exact-preimage, preview, approval, revalidation, atomicity, postcondition, rollback, privacy, and audit tests on each reference platform.
- [ ] **Story AC 40.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the release adds only declared controlled file writes; generic command execution, commits, remote actions, schedules, and autonomous mutation remain tested exclusions.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 40.AC1:** Every controlled-write transaction stage is covered by reproducible success and failure tests.
- [ ] **Sprint AC 40.AC2:** Disabled write capability restores a provably read-only product.
- [ ] **Sprint AC 40.AC3:** No generic shell, commit, push, publish, connector, schedule, or unattended write path is present.
- [ ] **Sprint AC 40.AC4:** Documentation clean-runs complete without undocumented authority or recovery steps.
- [ ] **Sprint AC 40.AC5:** `G-V0.3` closes only after every write safety threshold passes.

**Gate decision:** Sprint 40 is PASS only when Story 40.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 4 - v0.4 - Coding and Complete Local CLI

### [ ] Sprint 41 - Bounded Command Runner

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-034`.

**Sprint goal:** Add separately granted, literal command execution without exposing unrestricted shell authority.

**Source coverage:** inventory Section 12; command portions of Sections 5 and 29.

**Dependencies:** Sprint 40; legacy dependency record: `G-V0.3`.

#### [ ] Story 41.1 - Bounded Command Runner

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need bounded command runner so that AgentMage delivers the following bounded outcome: Add separately granted, literal command execution without exposing unrestricted shell authority.

##### Tasks and Sub-tasks

- [ ] **Task 41.1.1 - Implement the bounded story**
  - [ ] **Sub-task 41.1.1.1** (legacy `S-034-I01`): Implement `CommandSpec` with executable, literal arguments, working directory, environment allowlist, risk, grant requirement, cancellation, and bounds.
  - [ ] **Sub-task 41.1.1.2** (legacy `S-034-I02`): Execute direct programs without a shell whenever possible.
  - [ ] **Sub-task 41.1.1.3** (legacy `S-034-I03`): Implement exact executable, path, argument, working-directory, environment, and capability validation.
  - [ ] **Sub-task 41.1.1.4** (legacy `S-034-I04`): Filter inherited environment and credentials and reject interactive processes without a dedicated adapter.
  - [ ] **Sub-task 41.1.1.5** (legacy `S-034-I05`): Capture bounded standard output, standard error, exit code, cancellation, timeout, resource use, and child cleanup.
  - [ ] **Sub-task 41.1.1.6** (legacy `S-034-I06`): Render exact command previews and issue command receipts after every attempt.
  - [ ] **Sub-task 41.1.1.7** (legacy `S-034-I07`): Start with trusted deterministic read-only commands and promote repository commands only through separate fixtures.
  - [ ] **Sub-task 41.1.1.8** (legacy `S-034-I08`): Keep unrestricted shells, shell expansion, arbitrary repository setup, and hidden hook execution prohibited.

- [ ] **Task 41.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 41.1.2.1:** Command runner and command registry.
  - [ ] **Sub-task 41.1.2.2:** Command preview and receipt schemas.
  - [ ] **Sub-task 41.1.2.3:** Environment and child-process isolation report.
  - [ ] **Sub-task 41.1.2.4:** Command injection and cancellation corpus.

- [ ] **Task 41.1.3 - Verify and close the story**
  - [ ] **Sub-task 41.1.3.1:** `S-034-UT01` validates allowlisted executable identity, literal argument vectors, working directory, environment, limits, expected outputs, and grant binding; assert unknown commands/flags/paths are rejected before spawn.
  - [ ] **Sub-task 41.1.3.2:** `S-034-ST01` attempts shell metacharacters, substitution, globbing, response files, config discovery, aliases, pagers, hooks, loaders, inherited descriptors, proxy/credential environment, and PATH substitution; assert no interpretation or ambient authority.
  - [ ] **Sub-task 41.1.3.3:** `S-034-RT01` cancels, times out, kills, and crashes parent/child/grandchild process trees; assert complete descendant termination, descriptor closure, scratch cleanup, and one truthful terminal receipt.
  - [ ] **Sub-task 41.1.3.4:** `S-034-IT01` runs every approved template at minimum/maximum limits on each platform sandbox; assert exact command identity, output truncation, resource accounting, filesystem/network effects, and exit classification.
  - [ ] **Sub-task 41.1.3.5 - Product security evidence:** Map `SR-ACC-001` through `SR-ACC-007`, `SR-PLT-003`, `SR-AI-005`/`SR-AI-009`, `SR-TST-004`/`SR-TST-006`; retain template registry, argv/env traces, injection corpus, process-tree cleanup, resource results, and independent runner review.

##### Story Acceptance Criteria

- [ ] **Story AC 41.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the runner exposes no generic shell or arbitrary executable path; each invocation matches one versioned template and one exact current grant.
- [ ] **Story AC 41.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then success is based on verified exit/result/postconditions, not model interpretation, and every attempted process and side effect is attributable in evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 41.AC1:** Literal approved commands run with exact expected arguments and working directory.
- [ ] **Sprint AC 41.AC2:** Shell metacharacters, argument-prefix tricks, path substitution, environment leakage, and unapproved executables fail closed.
- [ ] **Sprint AC 41.AC3:** Cancellation and limits terminate all descendants and report actual status.
- [ ] **Sprint AC 41.AC4:** Commands cannot escape sandbox, workspace, grant, network, or credential scope.
- [ ] **Sprint AC 41.AC5:** No model narration is accepted as evidence that a command ran.

**Gate decision:** Sprint 41 is PASS only when Story 41.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 42 - Git Worktrees and Remote Repository Safety

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-035`.

**Sprint goal:** Isolate coding tasks from the user's active checkout and add bounded remote repository synchronization.

**Source coverage:** `CR-P1-WKT`; inventory Section 13A and read-only remote portions of Section 13.

**Dependencies:** Sprint 41; legacy dependency record: Sprint 41 (legacy S-034).

#### [ ] Story 42.1 - Git Worktrees and Remote Repository Safety

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need git worktrees and remote repository safety so that AgentMage delivers the following bounded outcome: Isolate coding tasks from the user's active checkout and add bounded remote repository synchronization.

##### Tasks and Sub-tasks

- [ ] **Task 42.1.1 - Implement the bounded story**
  - [ ] **Sub-task 42.1.1.1** (legacy `S-035-I01`): Implement remote URL, default branch, upstream, fetch state, and local-versus-remote divergence discovery.
  - [ ] **Sub-task 42.1.1.2** (legacy `S-035-I02`): Implement approval-gated clone into an empty selected directory and bounded read-only fetch without merge, rebase, branch switch, or working-tree mutation.
  - [ ] **Sub-task 42.1.1.3** (legacy `S-035-I03`): Implement currentness reports for ahead, behind, diverged, stale, dirty, and untracked states.
  - [ ] **Sub-task 42.1.1.4** (legacy `S-035-I04`): Implement temporary one-branch-per-task worktrees separate from the active checkout.
  - [ ] **Sub-task 42.1.1.5** (legacy `S-035-I05`): Record task, source commit, branch, owner, grants, file ownership, processes, resource budgets, retention, cleanup, and disposition per worktree.
  - [ ] **Sub-task 42.1.1.6** (legacy `S-035-I06`): Exclude ignored secrets and detect overlapping edits, renamed paths, changed preimages, and concurrent user changes before transfer.
  - [ ] **Sub-task 42.1.1.7** (legacy `S-035-I07`): Implement snapshot, restoration, merge-back or patch-transfer preview, cleanup, and remote-operation receipts.
  - [ ] **Sub-task 42.1.1.8** (legacy `S-035-I08`): Describe worktrees only as change and concurrency isolation, never as the operating-system security sandbox.

- [ ] **Task 42.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 42.1.2.1:** Worktree manager and ownership registry.
  - [ ] **Sub-task 42.1.2.2:** Remote discovery, fetch, currentness, and operation receipts.
  - [ ] **Sub-task 42.1.2.3:** Collision and active-checkout preservation fixtures.
  - [ ] **Sub-task 42.1.2.4:** Worktree handoff and recovery format.

- [ ] **Task 42.1.3 - Verify and close the story**
  - [ ] **Sub-task 42.1.3.1:** `S-035-UT01` creates, identifies, lists, hands off, and removes owned worktrees across clean/dirty/detached/missing/renamed states; assert stable ownership and no active-checkout mutation.
  - [ ] **Sub-task 42.1.3.2:** `S-035-ST01` seeds hooks, filters, submodules, alternates, malicious refs, case collisions, unsafe directories, credential helpers, and hostile remote URLs; assert no execution, secret access, or unapproved network.
  - [ ] **Sub-task 42.1.3.3:** `S-035-IT01` performs visible approved remote discovery/fetch with exact host/repository/ref/byte budgets, then verifies currentness and returns offline; assert zero push, publication, or implicit credential reuse.
  - [ ] **Sub-task 42.1.3.4:** `S-035-RT01` interrupts fetch, worktree creation/removal, branch movement, and cleanup while the user changes the active checkout; assert user changes survive and recovery identifies every owned artifact.
  - [ ] **Sub-task 42.1.3.5 - Product security evidence:** Map `SR-ACC-006` through `SR-ACC-008`, `SR-NET-005` through `SR-NET-007` where enabled, `SR-OPS-001`, `SR-TST-004`/`SR-TST-005`; retain before/after repository snapshots, ownership records, network traces, collision results, and recovery bundles.

##### Story Acceptance Criteria

- [ ] **Story AC 42.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then coding work occurs only in an AgentMage-owned isolated worktree with exact base identity; the user's active checkout and unrelated changes remain byte-for-byte preserved.
- [ ] **Story AC 42.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every network Git read is separately visible, bounded, receipted, currentness-checked, and incapable of mutating a remote repository.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 42.AC1:** Creating, using, transferring, and deleting a task worktree leaves the active checkout's branch, index, untracked files, and unfinished changes intact.
- [ ] **Sprint AC 42.AC2:** Fetch and currentness operations do not merge, rebase, reset, stash, discard, or run hooks.
- [ ] **Sprint AC 42.AC3:** Collision and stale-preimage conditions stop for review.
- [ ] **Sprint AC 42.AC4:** Repository credentials never enter model context, configuration, memory, command output, or audit logs.
- [ ] **Sprint AC 42.AC5:** Worktree operations remain inside the operating-system sandbox and exact grants.

**Gate decision:** Sprint 42 is PASS only when Story 42.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 43 - Deep Repository Comprehension

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-036`.

**Sprint goal:** Expand deterministic maps into cited architecture, feature, data, permission, schema, test, history, and dependency understanding.

**Source coverage:** inventory Sections 14 and 14A.

**Dependencies:** Sprint 42; legacy dependency record: Sprint 19 (legacy S-018), Sprint 30 (legacy S-025), Sprint 42 (legacy S-035).

#### [ ] Story 43.1 - Deep Repository Comprehension

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need deep repository comprehension so that AgentMage delivers the following bounded outcome: Expand deterministic maps into cited architecture, feature, data, permission, schema, test, history, and dependency understanding.

##### Tasks and Sub-tasks

- [ ] **Task 43.1.1 - Implement the bounded story**
  - [ ] **Sub-task 43.1.1.1** (legacy `S-036-I01`): Build repository profiles for languages, frameworks, package managers, entry points, tests, builds, linters, types, and instructions.
  - [ ] **Sub-task 43.1.1.2** (legacy `S-036-I02`): Add separately confined parser, language-native, and language-server read adapters for definitions, references, calls, types, diagnostics, and symbols.
  - [ ] **Sub-task 43.1.1.3** (legacy `S-036-I03`): Build package, application, service, library, entry-point, configuration, script, test, generated-code, and external-dependency maps.
  - [ ] **Sub-task 43.1.1.4** (legacy `S-036-I04`): Build cited architecture, feature, data-flow, authentication, authorization, schema, migration, interface, test, continuous-integration, and dependency traces.
  - [ ] **Sub-task 43.1.1.5** (legacy `S-036-I05`): Add Git-history, documentation-drift, glossary, branch-aware, cross-repository, and stale-index views.
  - [ ] **Sub-task 43.1.1.6** (legacy `S-036-I06`): Build large-repository slicing by package, entry point, dependency neighborhood, history, and user-selected scope.
  - [ ] **Sub-task 43.1.1.7** (legacy `S-036-I07`): Produce exact coverage and blind-spot reports and prohibit whole-repository claims without complete agreed coverage.
  - [ ] **Sub-task 43.1.1.8** (legacy `S-036-I08`): Export onboarding, architecture, dependency, feature, test, build, operations, and open-question guides.

- [ ] **Task 43.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 43.1.2.1:** Deep repository index and analysis adapters.
  - [ ] **Sub-task 43.1.2.2:** Cited map and trace formats.
  - [ ] **Sub-task 43.1.2.3:** Large-repository coverage strategy.
  - [ ] **Sub-task 43.1.2.4:** Repository-learning export suite.

- [ ] **Task 43.1.3 - Verify and close the story**
  - [ ] **Sub-task 43.1.3.1:** `S-036-UT01` derives architecture, feature, data flow, permission, schema, test, history, ownership, and dependency facts from labeled repositories; assert expected nodes/edges and exact source/revision citations.
  - [ ] **Sub-task 43.1.3.2:** `S-036-UT02` varies parser support, generated/vendor boundaries, history depth, repository size, stale revision, and missing dependencies; assert bounded degradation and quantified coverage.
  - [ ] **Sub-task 43.1.3.3:** `S-036-ST01` injects misleading docs/comments/names, contradictory implementations, secret canaries, malicious metadata, parser failures, and fabricated model explanations; assert deterministic evidence outranks prose/model claims.
  - [ ] **Sub-task 43.1.3.4:** `S-036-IT01` exports repository-learning maps and independently resolves/recomputes a sampled set; assert no private content beyond approved excerpts and no write/network side effects.
  - [ ] **Sub-task 43.1.3.5 - Product security evidence:** Map `SR-ACC-008`, `SR-AI-003`, `SR-AI-007`/`SR-AI-010`/`SR-AI-011`, `SR-DAT-003`, `SR-TST-004`/`SR-TST-006`; retain labeled corpus results, coverage metrics, citation sampling, canary scan, and export manifest.

##### Story Acceptance Criteria

- [ ] **Story AC 43.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every reported relationship is observed/derived with reproducible source evidence or explicitly inferred/unknown; confidence never substitutes for missing coverage.
- [ ] **Story AC 43.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then large-repository limits expose what was scanned, omitted, sampled, failed, stale, and unsupported, with stable rerun behavior.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 43.AC1:** Every structural claim resolves to exact file, range, symbol, parser or method, branch, and commit evidence.
- [ ] **Sprint AC 43.AC2:** Branches, worktrees, generated outputs, and commits are never combined silently.
- [ ] **Sprint AC 43.AC3:** Unsupported relationships remain Inferred or Unknown/Blocked.
- [ ] **Sprint AC 43.AC4:** Index invalidation detects file, branch, submodule, lockfile, generated-code, and instruction changes.
- [ ] **Sprint AC 43.AC5:** Coverage language accurately distinguishes discovered, read, indexed, skipped, and unknown material.

**Gate decision:** Sprint 43 is PASS only when Story 43.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 44 - Coding Intent, Reproduction, and Change Planning

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-037`.

**Sprint goal:** Produce the smallest defensible change plan before any code is modified.

**Source coverage:** inventory Sections 2B, 14, and 14B planning and debugging requirements.

**Dependencies:** Sprint 43; legacy dependency record: Sprint 43 (legacy S-036).

#### [ ] Story 44.1 - Coding Intent, Reproduction, and Change Planning

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need coding intent, reproduction, and change planning so that AgentMage delivers the following bounded outcome: Produce the smallest defensible change plan before any code is modified.

##### Tasks and Sub-tasks

- [ ] **Task 44.1.1 - Implement the bounded story**
  - [ ] **Sub-task 44.1.1.1** (legacy `S-037-I01`): Implement change-intent records with requested and current behavior, evidence, users, acceptance checks, exclusions, risks, and rollback.
  - [ ] **Sub-task 44.1.1.2** (legacy `S-037-I02`): Identify minimal affected files, symbols, tests, configuration, migrations, documentation, interfaces, dependencies, and data.
  - [ ] **Sub-task 44.1.1.3** (legacy `S-037-I03`): Implement reproducibility records for environment, inputs, steps, observed result, expected result, logs, and outcome.
  - [ ] **Sub-task 44.1.1.4** (legacy `S-037-I04`): Implement competing hypotheses, discriminating checks, state tracing, log correlation, and rejected explanations.
  - [ ] **Sub-task 44.1.1.5** (legacy `S-037-I05`): Require a failing regression test before a reproducible defect fix when safe and feasible.
  - [ ] **Sub-task 44.1.1.6** (legacy `S-037-I06`): Generate alternatives for architecture, dependency, access, cost, and irreversible choices.
  - [ ] **Sub-task 44.1.1.7** (legacy `S-037-I07`): Produce decision records and clarification stops for material unknowns.
  - [ ] **Sub-task 44.1.1.8** (legacy `S-037-I08`): Select security, privacy, data, accessibility, performance, migration, and rollback reviews based on evidence.

- [ ] **Task 44.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 44.1.2.1:** Change-intent and reproduction schemas.
  - [ ] **Sub-task 44.1.2.2:** Minimal-change and impact report.
  - [ ] **Sub-task 44.1.2.3:** Hypothesis and decision records.
  - [ ] **Sub-task 44.1.2.4:** Regression-test planning fixtures.

- [ ] **Task 44.1.3 - Verify and close the story**
  - [ ] **Sub-task 44.1.3.1:** `S-037-UT01` normalizes valid, ambiguous, contradictory, overbroad, and missing-context requests into intent/scope/exclusions/success fields; assert unresolved decisions block mutation.
  - [ ] **Sub-task 44.1.3.2:** `S-037-UT02` reproduces labeled failures with pinned inputs and compares observed output to proposed hypotheses; assert evidence, non-reproduction, and uncertainty are recorded distinctly.
  - [ ] **Sub-task 44.1.3.3:** `S-037-ST01` supplies repository instructions to broaden scope, suppress tests, expose secrets, edit unrelated files, or claim success; assert they remain untrusted evidence and appear as rejected risks.
  - [ ] **Sub-task 44.1.3.4:** `S-037-IT01` generates minimal change/impact/regression plans for the fictional corpus and compares touched files/contracts/tests with goldens; assert no unexplained scope.
  - [ ] **Sub-task 44.1.3.5 - Product security evidence:** Map `SR-GOV-005`/`SR-GOV-010`, `SR-ACC-007`/`SR-ACC-008`, `SR-AI-003`/`SR-AI-007`/`SR-AI-011`, `SR-TST-001`; retain intent records, reproduction logs, hypothesis decisions, golden plan diffs, and scope approval.

##### Story Acceptance Criteria

- [ ] **Story AC 44.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no code change begins until objective, exact base, reproduction, affected behavior, exclusions, risks, proposed files, validation, and rollback are reviewable and internally consistent.
- [ ] **Story AC 44.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the selected plan is the smallest evidence-supported change; broader alternatives and unresolved hypotheses remain documented rather than silently implemented.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 44.AC1:** No change proposal proceeds without cited current behavior and explicit acceptance checks.
- [ ] **Sprint AC 44.AC2:** Unreproduced or uncertain defects remain labeled and are not presented as proven root causes.
- [ ] **Sprint AC 44.AC3:** Minimal-change reports include callers, data, permissions, tests, docs, and rollback where relevant.
- [ ] **Sprint AC 44.AC4:** Material ambiguity stops for user clarification.
- [ ] **Sprint AC 44.AC5:** Proposed validation is separately grantable and does not inherit write authority.

**Gate decision:** Sprint 44 is PASS only when Story 44.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 45 - Structured Code Changes and Language Services

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-038`.

**Sprint goal:** Apply bounded, reviewable code changes using exact preimages, syntax-aware operations, and isolated services.

**Source coverage:** inventory Sections 14 and 14B controlled coding requirements.

**Dependencies:** Sprint 44; legacy dependency record: Sprint 36 (legacy S-029), Sprint 42 (legacy S-035), Sprint 44 (legacy S-037).

#### [ ] Story 45.1 - Structured Code Changes and Language Services

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need structured code changes and language services so that AgentMage delivers the following bounded outcome: Apply bounded, reviewable code changes using exact preimages, syntax-aware operations, and isolated services.

##### Tasks and Sub-tasks

- [ ] **Task 45.1.1 - Implement the bounded story**
  - [ ] **Sub-task 45.1.1.1** (legacy `S-038-I01`): Extend shadow change sets to code, configuration, tests, documentation, migrations, and generated outputs with explicit classifications.
  - [ ] **Sub-task 45.1.1.2** (legacy `S-038-I02`): Implement syntax-tree-aware edits for supported languages and deterministic text fallback only where structurally safe.
  - [ ] **Sub-task 45.1.1.3** (legacy `S-038-I03`): Implement language-server definitions, references, rename previews, diagnostics, and code actions without direct write authority.
  - [ ] **Sub-task 45.1.1.4** (legacy `S-038-I04`): Implement ordered atomic multi-file changes with preimages, postimages, rollback, and collision checks.
  - [ ] **Sub-task 45.1.1.5** (legacy `S-038-I05`): Add interface, dependency, migration, security, performance, accessibility, and compatibility review hooks.
  - [ ] **Sub-task 45.1.1.6** (legacy `S-038-I06`): Generate tests in repository style for boundaries, edge cases, failures, permissions, data changes, and rollback.
  - [ ] **Sub-task 45.1.1.7** (legacy `S-038-I07`): Preserve unrelated user changes and refuse broad unreviewed refactors, dependency upgrades, or migrations.
  - [ ] **Sub-task 45.1.1.8** (legacy `S-038-I08`): Scaffold new packages only from approved language conventions, license, tests, formatting, docs, and local commands.

- [ ] **Task 45.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 45.1.2.1:** Language-aware change adapters.
  - [ ] **Sub-task 45.1.2.2:** Multi-file transaction and rollback implementation.
  - [ ] **Sub-task 45.1.2.3:** Language-server confinement report.
  - [ ] **Sub-task 45.1.2.4:** Fictional multi-language coding corpus.

- [ ] **Task 45.1.3 - Verify and close the story**
  - [ ] **Sub-task 45.1.3.1:** `S-038-UT01` applies syntax-aware rename/edit/import/format operations to supported-language goldens and malformed/unsupported inputs; assert exact AST/text changes and preserved unrelated bytes.
  - [ ] **Sub-task 45.1.3.2:** `S-038-UT02` changes source preimages, language-server version, edit list, file identity, or plan after preview; assert stale transaction and zero workspace mutation.
  - [ ] **Sub-task 45.1.3.3:** `S-038-ST01` attacks language services through config/plugins, network, workspace expansion, executable discovery, environment secrets, generated code, and hostile responses; assert confinement and untrusted-output validation.
  - [ ] **Sub-task 45.1.3.4:** `S-038-RT01` crashes before/after each multi-file apply and rollback transition and introduces concurrent edits; assert all approved files commit together or prior state/conflicts are preserved.
  - [ ] **Sub-task 45.1.3.5 - Product security evidence:** Map `SR-ACC-002` through `SR-ACC-008`, `SR-SUP-003`/`SR-SUP-009`, `SR-AI-005`, `SR-TST-002`/`SR-TST-005`/`SR-TST-011`; retain AST/text goldens, service sandbox traces, transaction snapshots, FFI/unsafe inventory, and independent boundary review.

##### Story Acceptance Criteria

- [ ] **Story AC 45.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every code change is traceable to approved intent and exact preimages, remains inside the owned worktree, and produces a reviewable semantic/text diff with no undeclared file change.
- [ ] **Story AC 45.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then unsupported language/construct/service states use bounded textual proposals or block explicitly; they never invent structure or weaken transaction controls.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 45.AC1:** Supported fictional Python, TypeScript, JavaScript, Go, shell, SQL, and mixed-language tasks produce exact approved diffs.
- [ ] **Sprint AC 45.AC2:** Language servers cannot write files, run arbitrary commands, install packages, access secrets, or escape granted roots.
- [ ] **Sprint AC 45.AC3:** Multi-file failure restores the approved preimages without overwriting later user work.
- [ ] **Sprint AC 45.AC4:** Unrelated files and user changes remain byte-identical.
- [ ] **Sprint AC 45.AC5:** Broad refactors and dependency changes require their own explicit review and grant.

**Gate decision:** Sprint 45 is PASS only when Story 45.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 46 - Trusted Test and Validation Runner

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-039`.

**Sprint goal:** Run repository checks as separately approved evidence and never infer success from model output.

**Source coverage:** `CR-P1-VAL`; inventory Section 15 and validation portions of Sections 12 and 14B.

**Dependencies:** Sprint 45; legacy dependency record: Sprint 41 (legacy S-034), Sprint 45 (legacy S-038).

#### [ ] Story 46.1 - Trusted Test and Validation Runner

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need trusted test and validation runner so that AgentMage delivers the following bounded outcome: Run repository checks as separately approved evidence and never infer success from model output.

##### Tasks and Sub-tasks

- [ ] **Task 46.1.1 - Implement the bounded story**
  - [ ] **Sub-task 46.1.1.1** (legacy `S-039-I01`): Discover candidate test, lint, format, type, build, package, and security commands only from trusted project configuration or user input.
  - [ ] **Sub-task 46.1.1.2** (legacy `S-039-I02`): Require separate approval for each command template and effective execution scope.
  - [ ] **Sub-task 46.1.1.3** (legacy `S-039-I03`): Implement focused-test selection, fail-fast behavior, failed-test rerun, bounded waiting, cancellation, and cleanup.
  - [ ] **Sub-task 46.1.1.4** (legacy `S-039-I04`): Parse passed, failed, skipped, duration, failed names, artifacts, and affected files from process evidence.
  - [ ] **Sub-task 46.1.1.5** (legacy `S-039-I05`): Separate unit, integration, end-to-end, lint, types, build, packaging, and security results.
  - [ ] **Sub-task 46.1.1.6** (legacy `S-039-I06`): Capture exact command, arguments, working directory, bounded environment classification, process status, exit code, output, and artifacts in validation receipts.
  - [ ] **Sub-task 46.1.1.7** (legacy `S-039-I07`): Mark partial validation distinctly and every unrun check as unverified.
  - [ ] **Sub-task 46.1.1.8** (legacy `S-039-I08`): Classify failures as change-caused, baseline, flaky, dependency, environment, permission, or unrelated.

- [ ] **Task 46.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 46.1.2.1:** Trusted command-template registry.
  - [ ] **Sub-task 46.1.2.2:** Validation runner and result parsers.
  - [ ] **Sub-task 46.1.2.3:** Detailed validation receipt schema.
  - [ ] **Sub-task 46.1.2.4:** Failure-classification corpus.

- [ ] **Task 46.1.3 - Verify and close the story**
  - [ ] **Sub-task 46.1.3.1:** `S-039-UT01` validates trusted test templates, executable hashes, argv, environment, directory, timeout, resource, parser, and expected-output fields; assert substitutions and unknown flags are rejected.
  - [ ] **Sub-task 46.1.3.2:** `S-039-UT02` parses pass, assertion failure, compile failure, infrastructure failure, timeout, cancellation, crash, flaky, skipped, malformed, truncated, and zero-tests-run outputs; assert exact non-conflated states.
  - [ ] **Sub-task 46.1.3.3:** `S-039-ST01` uses forged green text, ANSI/control sequences, injected result files, test hooks/plugins, network fetches, secret output, and parser bombs; assert no false pass, execution escape, or leakage.
  - [ ] **Sub-task 46.1.3.4:** `S-039-RT01` kills process trees and runner/parser/storage at every stage; assert bounded cleanup, preserved raw output, uncertain state when warranted, and no automatic retry of non-idempotent setup.
  - [ ] **Sub-task 46.1.3.5 - Product security evidence:** Map `SR-SUP-003`, `SR-TST-001` through `SR-TST-006`, `SR-TST-010`, `SR-OPS-003`; retain template manifests, parser corpus, raw/normalized comparisons, injection results, resource traces, and cleanup evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 46.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then only a manifest-declared command with verified identity runs, and its pass status is computed from trusted process/result semantics plus required test-count/postcondition checks.
- [ ] **Story AC 46.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then raw and normalized results expose all failures, skips, retries, flakes, truncation, environment identity, duration, limits, and parser version; summaries are recomputable.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 46.AC1:** No repository command runs from untrusted text or model narration.
- [ ] **Sprint AC 46.AC2:** A passed status requires matching process and artifact evidence.
- [ ] **Sprint AC 46.AC3:** Partial, cancelled, timed-out, malformed, and unrun checks are never reported as a full pass.
- [ ] **Sprint AC 46.AC4:** Cancellation terminates descendants and leaves the worktree inspectable.
- [ ] **Sprint AC 46.AC5:** Validation receipts reproduce the exact execution boundary without leaking secrets.

**Gate decision:** Sprint 46 is PASS only when Story 46.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 47 - Review Packets, Commit Planning, and Local Source Control

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-040`.

**Sprint goal:** Turn validated changes into complete local review packages while keeping commits and publication explicitly user-controlled.

**Source coverage:** inventory Sections 13, 14, 14B, and 28 coding review skills.

**Dependencies:** Sprint 46; legacy dependency record: Sprint 42 (legacy S-035), Sprint 45 (legacy S-038), Sprint 46 (legacy S-039).

#### [ ] Story 47.1 - Review Packets, Commit Planning, and Local Source Control

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need review packets, commit planning, and local source control so that AgentMage delivers the following bounded outcome: Turn validated changes into complete local review packages while keeping commits and publication explicitly user-controlled.

##### Tasks and Sub-tasks

- [ ] **Task 47.1.1 - Implement the bounded story**
  - [ ] **Sub-task 47.1.1.1** (legacy `S-040-I01`): Generate review packets with objective, behavior delta, changed files, full diff, tests, checks not run, risks, rollback, screenshots or outputs, and proposed commit groups.
  - [ ] **Sub-task 47.1.1.2** (legacy `S-040-I02`): Implement code-review modes for correctness, simplicity, maintainability, security, data integrity, accessibility, performance, tests, and documentation.
  - [ ] **Sub-task 47.1.1.3** (legacy `S-040-I03`): Suppress duplicate and low-confidence findings while preserving evidence and severity.
  - [ ] **Sub-task 47.1.1.4** (legacy `S-040-I04`): Classify generated output, formatting, tests, docs, behavior, migrations, and unrelated user changes into logical commit plans.
  - [ ] **Sub-task 47.1.1.5** (legacy `S-040-I05`): Draft commit messages only from the approved change set.
  - [ ] **Sub-task 47.1.1.6** (legacy `S-040-I06`): Inspect hardware-backed or OpenPGP signing configuration and verify signed commits.
  - [ ] **Sub-task 47.1.1.7** (legacy `S-040-I07`): Require an exact diff and message preview plus a manual approval before any local commit.
  - [ ] **Sub-task 47.1.1.8** (legacy `S-040-I08`): Keep automatic commit, push, merge, release, reset, discard, and force operations prohibited.

- [ ] **Task 47.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 47.1.2.1:** Local review-packet format.
  - [ ] **Sub-task 47.1.2.2:** Review-mode result schemas.
  - [ ] **Sub-task 47.1.2.3:** Logical commit plan and signing report.
  - [ ] **Sub-task 47.1.2.4:** Manual commit approval receipts.

- [ ] **Task 47.1.3 - Verify and close the story**
  - [ ] **Sub-task 47.1.3.1:** `S-040-UT01` builds review packets from exact base/head identities, diffs, tests, risks, receipts, unresolved issues, and rollback; assert stable hashes and rejection of missing/stale components.
  - [ ] **Sub-task 47.1.3.2:** `S-040-UT02` exercises correctness/security/privacy/evidence/accessibility/performance review modes on labeled defects; assert expected findings, severity rationale, source anchors, and no invented defect.
  - [ ] **Sub-task 47.1.3.3:** `S-040-ST01` changes staged diff, commit message, identity, signature configuration, test evidence, or branch after preview; assert commit approval invalidation and zero commit.
  - [ ] **Sub-task 47.1.3.4:** `S-040-IT01` creates approved local signed commits from logical plans, verifies signatures and exact staged bytes, and snapshots remotes; assert no push, PR, review, merge, release, or publication.
  - [ ] **Sub-task 47.1.3.5 - Product security evidence:** Map `SR-GOV-005`/`SR-GOV-010`, `SR-ACC-002`/`SR-ACC-007`, `SR-SUP-002`/`SR-SUP-005`, `SR-TST-010`/`SR-TST-011`; retain packet hashes, finding corpus, staged/preimage comparisons, signature verification, remote snapshots, and approval receipt.

##### Story Acceptance Criteria

- [ ] **Story AC 47.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a reviewer can reconstruct intent, source base, every changed byte, validation, residual risk, provenance, and rollback from one integrity-protected packet.
- [ ] **Story AC 47.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then commit creation is separate from testing and publication, requires current exact approval, and leaves all unrelated user changes and remote state untouched.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 47.AC1:** Review packets account for every changed file and every unrun check.
- [ ] **Sprint AC 47.AC2:** Commit plans exclude unrelated user work.
- [ ] **Sprint AC 47.AC3:** No commit occurs without exact manual approval and required signature verification.
- [ ] **Sprint AC 47.AC4:** Commit failure or changed staged diff invalidates approval.
- [ ] **Sprint AC 47.AC5:** Push and every hosted mutation remain absent from v0.4.

**Gate decision:** Sprint 47 is PASS only when Story 47.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 48 - Complete Local CLI and Headless Contracts

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-041`.

**Sprint goal:** Expose the stable kernel through an interactive CLI and fail-closed machine-readable clients without creating alternate authority paths.

**Source coverage:** `CR-P2-CLI`; inventory Section 27A.

**Dependencies:** Sprint 47; legacy dependency record: `G-V0.1`, Sprint 33 (legacy S-027), Sprint 41 (legacy S-034), Sprint 46 (legacy S-039).

#### [ ] Story 48.1 - Complete Local CLI and Headless Contracts

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need complete local cli and headless contracts so that AgentMage delivers the following bounded outcome: Expose the stable kernel through an interactive CLI and fail-closed machine-readable clients without creating alternate authority paths.

##### Tasks and Sub-tasks

- [ ] **Task 48.1.1 - Implement the bounded story**
  - [ ] **Sub-task 48.1.1.1** (legacy `S-041-I01`): Implement interactive chat through the same runtime, grants, tools, memory, conversations, and receipts as Visual Studio Code.
  - [ ] **Sub-task 48.1.1.2** (legacy `S-041-I02`): Implement conversation list, search, show, open, resume, and exact-turn branch commands.
  - [ ] **Sub-task 48.1.1.3** (legacy `S-041-I03`): Implement vault search, note show, links, backlinks, tasks, checkpoint, handoff, audit, memory inspect, memory correct, export, import, and diagnostics commands.
  - [ ] **Sub-task 48.1.1.4** (legacy `S-041-I04`): Render active workspace, model, permission, conversation, plan, writable roots, offline state, previews, diffs, citations, errors, and receipts.
  - [ ] **Sub-task 48.1.1.5** (legacy `S-041-I05`): Implement stable exit codes, versioned input and event schemas, bounded output, deterministic cancellation, and shell completion.
  - [ ] **Sub-task 48.1.1.6** (legacy `S-041-I06`): Require predeclared bounded expiring grants for noninteractive operations and fail closed when approval is unavailable or stale.
  - [ ] **Sub-task 48.1.1.7** (legacy `S-041-I07`): Implement JSON, software-development-kit, and Agent Client Protocol clients only as thin kernel clients with no direct storage, tool, model, connector, or secret access.
  - [ ] **Sub-task 48.1.1.8** (legacy `S-041-I08`): Test malformed events, broken pipes, client termination, cancellation races, partial output, and policy-version changes.

- [ ] **Task 48.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 48.1.2.1:** Interactive CLI package and command reference.
  - [ ] **Sub-task 48.1.2.2:** Versioned JSON, event, exit-code, and cancellation contracts.
  - [ ] **Sub-task 48.1.2.3:** Thin client adapters and authority-boundary report.
  - [ ] **Sub-task 48.1.2.4:** Headless adversarial test results.

- [ ] **Task 48.1.3 - Verify and close the story**
  - [ ] **Sub-task 48.1.3.1:** `S-041-UT01` validates CLI arguments, JSON requests/responses/events, protocol versions, exit codes, streaming order, cancellation, and bounded output with malformed/oversized/replayed inputs.
  - [ ] **Sub-task 48.1.3.2:** `S-041-UT02` runs identical work packets through native Chat, interactive CLI, JSON, SDK, and ACP-compatible thin clients; assert equal policy decisions, grants, receipts, evidence, and final states.
  - [ ] **Sub-task 48.1.3.3:** `S-041-ST01` attempts client-side tool dispatch, filesystem/model/key access, grant minting, hidden approval, policy override, prompt injection, and raw-host connection; assert clients remain display/transport only.
  - [ ] **Sub-task 48.1.3.4:** `S-041-RT01` disconnects/reconnects clients and cancels during every event phase; assert one canonical kernel operation, no duplicate effect, resumable event position, and descendant cleanup.
  - [ ] **Sub-task 48.1.3.5 - Product security evidence:** Map `SR-PLT-005`/`SR-PLT-006`, `SR-ACC-001`/`SR-ACC-007`, `SR-OPS-001`, `SR-TST-001`/`SR-TST-004`; retain conformance vectors, cross-interface diff, adversarial traces, disconnect/cancellation results, and client boundary report.

##### Story Acceptance Criteria

- [ ] **Story AC 48.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then headless clients fail closed without an interactive approval channel and cannot obtain broader authority than native Chat for the same authenticated task.
- [ ] **Story AC 48.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then published protocol fixtures are sufficient for an independent client to reproduce success/error/cancellation behavior without internal imports or hidden state.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 48.AC1:** Every CLI operation produces the same policy, evidence, receipt, and state transition as native Chat.
- [ ] **Sprint AC 48.AC2:** Missing, stale, ambiguous, or interactive-only authority fails closed without hidden prompting or fallback.
- [ ] **Sprint AC 48.AC3:** No client reads or writes the canonical database directly.
- [ ] **Sprint AC 48.AC4:** Network-disabled execution never launches a browser, Obsidian, cloud login, or unrelated application.
- [ ] **Sprint AC 48.AC5:** Agent Client Protocol and JSON paths cannot bypass the kernel.

**Gate decision:** Sprint 48 is PASS only when Story 48.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 49 - Later Model Profiles and Measured Local Routing

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-042`.

**Sprint goal:** Evaluate additional approved local profiles and enable routing only where measured evidence justifies it.

**Source coverage:** inventory Sections 1, 1A, 1B, 2B, and 31A; `CR-P1-ROL`.

**Dependencies:** Sprint 48; legacy dependency record: Sprint 15 (legacy S-015), Sprint 43 (legacy S-036), Sprint 46 (legacy S-039).

#### [ ] Story 49.1 - Later Model Profiles and Measured Local Routing

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need later model profiles and measured local routing so that AgentMage delivers the following bounded outcome: Evaluate additional approved local profiles and enable routing only where measured evidence justifies it.

##### Tasks and Sub-tasks

- [ ] **Task 49.1.1 - Implement the bounded story**
  - [ ] **Sub-task 49.1.1.1** (legacy `S-042-I01`): Verify Gemma 4 26B and Devstral Small 2 identities, publishers, lineage, licenses, origin policy, artifacts, tokenizers, runtimes, resources, and platform support before configuration.
  - [ ] **Sub-task 49.1.1.2** (legacy `S-042-I02`): Benchmark dialogue, tool selection, summarization, repository maps, embeddings, reranking, patch generation, citation verification, planning, coding, retrieval, and document roles independently.
  - [ ] **Sub-task 49.1.1.3** (legacy `S-042-I03`): Implement role-to-profile allowlists and refuse unmeasured role assignments.
  - [ ] **Sub-task 49.1.1.4** (legacy `S-042-I04`): Implement visible fast, standard, deep, and verify budgets with explicit model, context, tools, and review boundaries.
  - [ ] **Sub-task 49.1.1.5** (legacy `S-042-I05`): Implement a measured local router that uses task class, risk, and capability results rather than model self-confidence.
  - [ ] **Sub-task 49.1.1.6** (legacy `S-042-I06`): Preserve manual selection and expose every routing choice and disagreement.
  - [ ] **Sub-task 49.1.1.7** (legacy `S-042-I07`): Add optional second-model verification only where it improves measured high-risk results.
  - [ ] **Sub-task 49.1.1.8** (legacy `S-042-I08`): Keep invisible fallback, broad provider marketplace, automatic frontier routing, and unmeasured ensembles disabled.

- [ ] **Task 49.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 49.1.2.1:** Approved later-profile manifests.
  - [ ] **Sub-task 49.1.2.2:** Role-specific capability matrix and benchmark corpus.
  - [ ] **Sub-task 49.1.2.3:** Measured routing decision table.
  - [ ] **Sub-task 49.1.2.4:** Routing and disagreement audit views.

- [ ] **Task 49.1.3 - Verify and close the story**
  - [ ] **Sub-task 49.1.3.1:** `S-042-UT01` verifies each later profile's model/runtime/license/lineage/quantization/template/hash/resource/platform manifest; assert unapproved, Chinese, Chinese-derived, incompatible, or silently changed profiles cannot register.
  - [ ] **Sub-task 49.1.3.2:** `S-042-UT02` evaluates deterministic-first and measured routing rules at every threshold/tie/degraded state; assert stable chosen profile, visible rationale, and manual override within approved choices.
  - [ ] **Sub-task 49.1.3.3:** `S-042-ST01` attempts model self-selection, profile escalation, cloud fallback, automatic install, authority transfer, and disagreement suppression; assert zero hidden switch or expanded capability.
  - [ ] **Sub-task 49.1.3.4:** `S-042-AT01` runs role-specific quality, grounding, reliability, latency, memory, energy where measured, and failure benchmarks repeatedly; assert routing activates only for statistically supported declared benefit.
  - [ ] **Sub-task 49.1.3.5 - Product security evidence:** Map `SR-SUP-006` through `SR-SUP-008`, `SR-AI-001`/`SR-AI-006`/`SR-AI-010` through `SR-AI-014`, `SR-TST-006`; retain manifests, supplier decisions, benchmark code/raw results, routing traces, disagreement cases, and impact reviews.

##### Story Acceptance Criteria

- [ ] **Story AC 49.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every enabled profile satisfies security, license, provenance, platform, resource, and quality policy independently; routing evidence cannot waive a failed boundary.
- [ ] **Story AC 49.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then users and reviewers can see which deterministic method/model/runtime handled each step, why it was selected, what disagreed, and how evidence was validated.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 49.AC1:** No profile or role is enabled without passing its license, lineage, origin, security, quality, hardware, and platform gates.
- [ ] **Sprint AC 49.AC2:** Routing improves declared acceptance metrics over manual E4B selection for the promoted task classes.
- [ ] **Sprint AC 49.AC3:** Unmeasured, unavailable, degraded, or failing tiers are never selected.
- [ ] **Sprint AC 49.AC4:** User selection and visible stop behavior remain available.
- [ ] **Sprint AC 49.AC5:** No local routing decision can invoke or transfer content to a frontier service.

**Gate decision:** Sprint 49 is PASS only when Story 49.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 50 - Coding Skills, Documentation, and v0.4 Release

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-043`.

**Sprint goal:** Close the coding release with complete workflows, CLI parity, and no autonomous publication.

**Source coverage:** inventory Section 28 coding skills; Section 32 coding, CLI, repository, patch, signing, and troubleshooting guides; `G-V0.4`.

**Dependencies:** Sprint 49; legacy dependency record: Sprint 41 (legacy S-034) through Sprint 49 (legacy S-042).

#### [ ] Story 50.1 - Coding Skills, Documentation, and v0.4 Release

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need coding skills, documentation, and v0.4 release so that AgentMage delivers the following bounded outcome: Close the coding release with complete workflows, CLI parity, and no autonomous publication.

##### Tasks and Sub-tasks

- [ ] **Task 50.1.1 - Implement the bounded story**
  - [ ] **Sub-task 50.1.1.1** (legacy `S-043-I01`): Implement declarative Repository Cartographer, Feature Trace, Change Impact, Debugging, Test and Verification, Repository Documentation, Bug Reproduction, Git History Analysis, and bounded review skills.
  - [ ] **Sub-task 50.1.1.2** (legacy `S-043-I02`): Verify every skill against the declarative trust and no-authority contract.
  - [ ] **Sub-task 50.1.1.3** (legacy `S-043-I03`): Run fictional repository tasks for learning, diagnosis, bug fixing, feature work, tests, review, rollback, and recovery across supported languages.
  - [ ] **Sub-task 50.1.1.4** (legacy `S-043-I04`): Run worktree collision, stale preimage, command injection, malicious project configuration, failed validation, and unrelated-change suites.
  - [ ] **Sub-task 50.1.1.5** (legacy `S-043-I05`): Verify native Chat and CLI produce equivalent kernel decisions and receipts.
  - [ ] **Sub-task 50.1.1.6** (legacy `S-043-I06`): Publish coding, repository comprehension, command approval, validation, worktree, review, commit, rollback, CLI, and troubleshooting guides.
  - [ ] **Sub-task 50.1.1.7** (legacy `S-043-I07`): Prove automatic commit, push, pull-request publication, merge, release, deployment, dependency upgrade, broad refactor, and migration remain disabled.

- [ ] **Task 50.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 50.1.2.1:** Coding skill pack and evaluation results.
  - [ ] **Sub-task 50.1.2.2:** Cross-interface parity report.
  - [ ] **Sub-task 50.1.2.3:** v0.4 coding acceptance and recovery bundle.
  - [ ] **Sub-task 50.1.2.4:** v0.4 release notes and capability matrix.

- [ ] **Task 50.1.3 - Verify and close the story**
  - [ ] **Sub-task 50.1.3.1:** `S-043-UT01` validates coding-skill definitions against vague completion, hidden authority, unsupported tools/languages, missing validation, and overbroad scope; assert disabled or corrected status.
  - [ ] **Sub-task 50.1.3.2:** `S-043-IT01` completes comprehension-to-plan-to-edit-to-test-to-review-to-local-commit workflows through Chat and CLI on the fictional corpus; assert interface parity and exact worktree isolation.
  - [ ] **Sub-task 50.1.3.3:** `S-043-ST01` attempts autonomous commit, push, PR, review, merge, release, deploy, arbitrary shell, or frontier handoff through every coding path; assert tested exclusions remain absent/denied.
  - [ ] **Sub-task 50.1.3.4:** `S-043-AT01` performs clean install/upgrade/offline/recovery/accessibility and forced-gate-failure runs on all reference platforms; assert no package or `G-V0.4` closure on failure.
  - [ ] **Sub-task 50.1.3.5 - Product security evidence:** Map applicable `SR-ACC-*`, `SR-SUP-*`, `SR-AI-*`, `SR-OPS-*`, `SR-TST-*`, and `SR-CIV-006` through `SR-CIV-009`; retain skill evaluations, full workflow packets, exclusion attempts, platform results, and signed gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 50.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every promoted coding workflow is source-grounded, minimally scoped, isolated, validated by trusted tools, reviewable, recoverable, and manually controlled through local commit.
- [ ] **Story AC 50.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then all interfaces expose the same kernel authority and evidence, and remote publication remains outside the release boundary.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 50.AC1:** Every promoted language workflow passes exact change, validation, preservation, and rollback fixtures.
- [ ] **Sprint AC 50.AC2:** Worktrees, pending changes, and path checks are never represented as the operating-system sandbox.
- [ ] **Sprint AC 50.AC3:** CLI and other clients cannot bypass grants, storage, retention, receipts, or offline policy.
- [ ] **Sprint AC 50.AC4:** Hosted mutation and automatic publication remain impossible.
- [ ] **Sprint AC 50.AC5:** `G-V0.4` closes only after coding, shell, model, security, recovery, and documentation suites pass.

**Gate decision:** Sprint 50 is PASS only when Story 50.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 5 - v0.5 - Manual Frontier Consultation

### [ ] Sprint 51 - Frontier Recommendation and Disclosure Packet

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-044`.

**Sprint goal:** Recommend frontier consultation only after measured local failure and prepare a bounded local packet without delivering it.

**Source coverage:** inventory Section 1B, Section 10D frontier roadmap, and Section 35B v0.5.

**Dependencies:** Sprint 50; legacy dependency record: `G-V0.4`, Sprint 21 (legacy S-019), Sprint 33 (legacy S-027), Sprint 49 (legacy S-042).

#### [ ] Story 51.1 - Frontier Recommendation and Disclosure Packet

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need frontier recommendation and disclosure packet so that AgentMage delivers the following bounded outcome: Recommend frontier consultation only after measured local failure and prepare a bounded local packet without delivering it.

##### Tasks and Sub-tasks

- [ ] **Task 51.1.1 - Implement the bounded story**
  - [ ] **Sub-task 51.1.1.1** (legacy `S-044-I01`): Implement task tiers for deterministic script, local model, ask user, and frontier recommended.
  - [ ] **Sub-task 51.1.1.2** (legacy `S-044-I02`): Trigger recommendations only from measured capability failure, repeated validation failure, contradiction, rejected verification, exhausted budget, or material clarification need.
  - [ ] **Sub-task 51.1.1.3** (legacy `S-044-I03`): Keep human clarification separate from frontier recommendation.
  - [ ] **Sub-task 51.1.1.4** (legacy `S-044-I04`): Build packets deterministically from objective, current state, citations, receipts, constraints, authority boundary, acceptance checks, disclosure list, unresolved questions, and required output contract.
  - [ ] **Sub-task 51.1.1.5** (legacy `S-044-I05`): Minimize packet content, redact secrets, enforce workspace scope, and show exact included excerpts and metadata.
  - [ ] **Sub-task 51.1.1.6** (legacy `S-044-I06`): Render the packet locally with a stable hash and user review controls.
  - [ ] **Sub-task 51.1.1.7** (legacy `S-044-I07`): Prohibit Codex or external-model invocation, tab control, prompt population, clipboard writes, launch commands, endpoint calls, and transmission.
  - [ ] **Sub-task 51.1.1.8** (legacy `S-044-I08`): Record recommendation reason, packet hash, redaction result, and later user-supplied destination only when the user chooses to record it.

- [ ] **Task 51.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 51.1.2.1:** Tier decision and frontier-recommendation schema.
  - [ ] **Sub-task 51.1.2.2:** Deterministic packet builder and disclosure preview.
  - [ ] **Sub-task 51.1.2.3:** Redaction, scope, and prohibited-delivery fixtures.
  - [ ] **Sub-task 51.1.2.4:** Frontier recommendation receipt format.

- [ ] **Task 51.1.3 - Verify and close the story**
  - [ ] **Sub-task 51.1.3.1:** `S-044-UT01` evaluates local-success, local-failure, uncertainty, unsupported-capability, and threshold-boundary cases; assert frontier consultation is recommended only by the approved measured rule.
  - [ ] **Sub-task 51.1.3.2:** `S-044-UT02` builds identical packets twice from exact selected evidence; assert byte-stable manifest, hashes, bounded excerpts, exclusions, task, expected return schema, and disclosure inventory.
  - [ ] **Sub-task 51.1.3.3:** `S-044-ST01` seeds credentials, private files, unrelated context, hidden metadata, absolute paths, excessive excerpts, authority objects, and prompt injections; assert redaction/blocking before packet approval.
  - [ ] **Sub-task 51.1.3.4:** `S-044-IT01` attempts automatic send/upload/API/browser/clipboard/Codex transfer and post-approval packet mutation; assert zero delivery and invalidated disclosure approval.
  - [ ] **Sub-task 51.1.3.5 - Product security evidence:** Map `SR-ACC-007`, `SR-DAT-002`/`SR-DAT-003`, `SR-AI-004`/`SR-AI-008`/`SR-AI-010`, `SR-OPS-001`/`SR-OPS-003`; retain tier decisions, packet hashes, disclosure previews, canary scans, prohibited-delivery traces, and approval receipt.

##### Story Acceptance Criteria

- [ ] **Story AC 51.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then agentMage may recommend and prepare a local packet but cannot choose a service, authenticate, transmit, upload, paste, or invoke Codex/frontier tooling.
- [ ] **Story AC 51.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the user sees and approves the exact complete disclosure, including files/excerpts/metadata/risks, and any byte change requires a new preview.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 51.AC1:** Every recommendation cites a failed or unmeasured local acceptance check.
- [ ] **Sprint AC 51.AC2:** Valid packets contain all required context and only previewed content.
- [ ] **Sprint AC 51.AC3:** Secret, unrelated-workspace, restricted, hidden-prompt, and unapproved excerpts are absent.
- [ ] **Sprint AC 51.AC4:** Direct, injected, scheduled, routed, and standing-consent delivery attempts all fail.
- [ ] **Sprint AC 51.AC5:** Only the user can move the reviewed packet to another interface.

**Gate decision:** Sprint 51 is PASS only when Story 51.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 52 - Frontier Result Import and Local Revalidation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-045`.

**Sprint goal:** Import user-provided frontier results as untrusted proposals and return every action to local policy and verification.

**Source coverage:** inventory Sections 1B, 10D, 14B, 22, and 23.

**Dependencies:** Sprint 51; legacy dependency record: Sprint 51 (legacy S-044).

#### [ ] Story 52.1 - Frontier Result Import and Local Revalidation

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need frontier result import and local revalidation so that AgentMage delivers the following bounded outcome: Import user-provided frontier results as untrusted proposals and return every action to local policy and verification.

##### Tasks and Sub-tasks

- [ ] **Task 52.1.1 - Implement the bounded story**
  - [ ] **Sub-task 52.1.1.1** (legacy `S-045-I01`): Define a return manifest containing decision or artifact, rationale, remaining steps, tier, inputs, acceptance checks, and approval requirements.
  - [ ] **Sub-task 52.1.1.2** (legacy `S-045-I02`): Parse and validate imported packets without executing embedded instructions, code, tools, links, or commands.
  - [ ] **Sub-task 52.1.1.3** (legacy `S-045-I03`): Treat all returned claims and files as untrusted and classify them before storage or context use.
  - [ ] **Sub-task 52.1.1.4** (legacy `S-045-I04`): Re-resolve citations and re-check local source preimages, workspace state, model state, policy, and permissions.
  - [ ] **Sub-task 52.1.1.5** (legacy `S-045-I05`): Re-run each returned step through local task classification, grant, tool, write, validation, and evidence contracts.
  - [ ] **Sub-task 52.1.1.6** (legacy `S-045-I06`): Fail loudly on mislabeled, unsupported, stale, unsafe, or unverified returned steps.
  - [ ] **Sub-task 52.1.1.7** (legacy `S-045-I07`): Preserve frontier and local disagreements and resolve through evidence or user direction.
  - [ ] **Sub-task 52.1.1.8** (legacy `S-045-I08`): Record outcome, local decisions, validation, re-escalation reason, and capability-matrix feedback.

- [ ] **Task 52.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 52.1.2.1:** Return-manifest schema and untrusted importer.
  - [ ] **Sub-task 52.1.2.2:** Local revalidation pipeline.
  - [ ] **Sub-task 52.1.2.3:** Malicious, stale, overbroad, and malformed import corpus.
  - [ ] **Sub-task 52.1.2.4:** Round-trip outcome receipts.

- [ ] **Task 52.1.3 - Verify and close the story**
  - [ ] **Sub-task 52.1.3.1:** `S-045-UT01` validates return manifests with missing/extra/malformed/oversized/version-mismatched fields, wrong request hash, stale base, and unsupported artifacts; assert quarantine or explicit rejection.
  - [ ] **Sub-task 52.1.3.2:** `S-045-ST01` imports prompt injection, malicious code, path escapes, hidden binaries, secrets, fabricated tests/citations, overbroad changes, and authority requests; assert all content remains untrusted and non-executing.
  - [ ] **Sub-task 52.1.3.3:** `S-045-IT01` converts accepted proposals into normal local intent/write/test/review flows; assert fresh grants, preimages, sandboxes, trusted validation, citations, and user approvals are required.
  - [ ] **Sub-task 52.1.3.4:** `S-045-RT01` interrupts import/quarantine/validation/application and changes repository/model/policy state mid-round-trip; assert no partial trust, duplicate effect, or stale acceptance.
  - [ ] **Sub-task 52.1.3.5 - Product security evidence:** Map `SR-ACC-002`/`SR-ACC-007`/`SR-ACC-008`, `SR-AI-003` through `SR-AI-005`, `SR-AI-010`/`SR-AI-011`, `SR-TST-002`/`SR-TST-004`; retain import corpus, quarantine report, local-validation traces, state-change tests, and outcome ledger.

##### Story Acceptance Criteria

- [ ] **Story AC 52.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then imported results carry no authority, evidence status, trust, or completion credit merely because they came from a stronger model or match the requested schema.
- [ ] **Story AC 52.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every accepted byte and claim is revalidated locally under current state; rejected/unknown portions remain visible in the round-trip receipt.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 52.AC1:** Imported content cannot alter canonical state, issue grants, call tools, or write files directly.
- [ ] **Sprint AC 52.AC2:** Every accepted returned claim has local evidence or remains explicitly Inferred or Unknown/Blocked.
- [ ] **Sprint AC 52.AC3:** Every state-changing returned step requires the same fresh local preview and grant as native work.
- [ ] **Sprint AC 52.AC4:** Repeated escalation becomes benchmark evidence rather than automatic recursion.
- [ ] **Sprint AC 52.AC5:** Import works without granting AgentMage any outbound network capability.

**Gate decision:** Sprint 52 is PASS only when Story 52.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 53 - v0.5 Frontier Release Gate

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-046`.

**Sprint goal:** Release manual frontier consultation without creating an autonomous cloud bridge.

**Source coverage:** PRD Section 15; inventory release sequence, Sections 1B and 35A; Codex boundary documentation.

**Dependencies:** Sprint 52; legacy dependency record: Sprint 51 (legacy S-044), Sprint 52 (legacy S-045).

#### [ ] Story 53.1 - v0.5 Frontier Release Gate

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need v0.5 frontier release gate so that AgentMage delivers the following bounded outcome: Release manual frontier consultation without creating an autonomous cloud bridge.

##### Tasks and Sub-tasks

- [ ] **Task 53.1.1 - Implement the bounded story**
  - [ ] **Sub-task 53.1.1.1** (legacy `S-046-I01`): Run recommendation, packet, disclosure, redaction, import, revalidation, and round-trip fixtures.
  - [ ] **Sub-task 53.1.1.2** (legacy `S-046-I02`): Re-run Codex delivery attacks through Chat, CLI, model tools, skills, schedules, injected content, clipboard APIs, Visual Studio Code commands, and network paths.
  - [ ] **Sub-task 53.1.1.3** (legacy `S-046-I03`): Verify packet export is a user-initiated local file operation under the controlled write contract.
  - [ ] **Sub-task 53.1.1.4** (legacy `S-046-I04`): Publish frontier recommendation, disclosure review, manual transfer, import, validation, privacy, and limitation guides.
  - [ ] **Sub-task 53.1.1.5** (legacy `S-046-I05`): Publish capability-matrix rules for local task tiers and frontier recommendations.

- [ ] **Task 53.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 53.1.2.1:** v0.5 acceptance bundle.
  - [ ] **Sub-task 53.1.2.2:** Frontier threat-model and boundary report.
  - [ ] **Sub-task 53.1.2.3:** User-controlled consultation guides.
  - [ ] **Sub-task 53.1.2.4:** v0.5 release notes and capability matrix.

- [ ] **Task 53.1.3 - Verify and close the story**
  - [ ] **Sub-task 53.1.3.1:** `S-046-IT01` performs measured recommendation, packet preview/export, user-mediated external consultation simulation, import, local revalidation, and optional controlled application; assert complete round-trip receipts.
  - [ ] **Sub-task 53.1.3.2:** `S-046-ST01` probes every interface/process for autonomous cloud client, credential access, delivery, browser automation, direct Codex handoff, hidden telemetry, and imported-authority paths; assert zero capability.
  - [ ] **Sub-task 53.1.3.3:** `S-046-RT01` cancels or corrupts every round-trip phase and repeats import/export; assert idempotent local artifacts, no duplicate application, bounded retention, and safe cleanup.
  - [ ] **Sub-task 53.1.3.4:** `S-046-AT01` forces redaction, disclosure, freshness, local validation, network-exclusion, and privacy thresholds to fail; assert no release or gate closure.
  - [ ] **Sub-task 53.1.3.5 - Product security evidence:** Map `SR-ACC-007`/`SR-ACC-008`, `SR-DAT-002`/`SR-DAT-003`, `SR-AI-003` through `SR-AI-005`, `SR-AI-008` through `SR-AI-011`, `SR-TST-004`; retain threat model, end-to-end bundle, prohibited-capability scan, failure injection, guides, and signed gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 53.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the complete frontier workflow remains manually transported, exactly disclosed, locally auditable, authority-free on return, and optional/removable without affecting local capability.
- [ ] **Story AC 53.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then `G-V0.5` closes only with current privacy, injection, disclosure, import, revalidation, recovery, and no-autonomous-network evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 53.AC1:** Zero autonomous or unattended external-model delivery paths exist.
- [ ] **Sprint AC 53.AC2:** Every export requires an exact local disclosure preview and user-initiated write.
- [ ] **Sprint AC 53.AC3:** Every import remains untrusted until locally revalidated.
- [ ] **Sprint AC 53.AC4:** Strict-local mode remains fully usable with no external account or service.
- [ ] **Sprint AC 53.AC5:** `G-V0.5` closes only after boundary, privacy, evidence, import, and documentation gates pass.

**Gate decision:** Sprint 53 is PASS only when Story 53.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 6 - v0.6 - Administrative and Document Work

### [ ] Sprint 54 - Executive Assistant and Task Portfolio

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-047`.

**Sprint goal:** Turn canonical local knowledge into evidence-backed priorities, briefings, commitments, decisions, approvals, and follow-up views.

**Source coverage:** inventory Sections 11 and 11A; executive skills in Section 28.

**Dependencies:** Sprint 53; legacy dependency record: `G-V0.5`, Sprint 26 (legacy S-022), Sprint 31 (legacy S-026), Sprint 38 (legacy S-031).

#### [ ] Story 54.1 - Executive Assistant and Task Portfolio

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need executive assistant and task portfolio so that AgentMage delivers the following bounded outcome: Turn canonical local knowledge into evidence-backed priorities, briefings, commitments, decisions, approvals, and follow-up views.

##### Tasks and Sub-tasks

- [ ] **Task 54.1.1 - Implement the bounded story**
  - [ ] **Sub-task 54.1.1.1** (legacy `S-047-I01`): Implement start-of-cycle briefing, closeout, recurring review, portfolio, and status views from approved local records.
  - [ ] **Sub-task 54.1.1.2** (legacy `S-047-I02`): Implement commitment, decision, waiting, approval, deadline, and reminder trackers with exact source evidence.
  - [ ] **Sub-task 54.1.1.3** (legacy `S-047-I03`): Implement priority ranking across urgency, importance, dependencies, schedule records, effort, user preference, and consequences with visible reasons.
  - [ ] **Sub-task 54.1.1.4** (legacy `S-047-I04`): Implement meeting, decision, person, and organization briefs with confirmed, inferred, historical, disputed, and unknown distinctions.
  - [ ] **Sub-task 54.1.1.5** (legacy `S-047-I05`): Implement correspondence drafts and response checks for unanswered questions, accidental commitments, unclear dates, missing attachments, unsupported claims, sensitive content, and uncertain names.
  - [ ] **Sub-task 54.1.1.6** (legacy `S-047-I06`): Implement local message-export triage without connecting to or sending through an inbox.
  - [ ] **Sub-task 54.1.1.7** (legacy `S-047-I07`): Implement workload conflict, preparation, briefing-pack, change-since, and forgotten-item reviews.
  - [ ] **Sub-task 54.1.1.8** (legacy `S-047-I08`): Enforce ordinary, private, confidential, and highly restricted retrieval, retention, index, and export rules.

- [ ] **Task 54.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 54.1.2.1:** Executive-assistant views and trackers.
  - [ ] **Sub-task 54.1.2.2:** Ranking and recommendation explanation schemas.
  - [ ] **Sub-task 54.1.2.3:** Evidence-backed briefing and correspondence templates.
  - [ ] **Sub-task 54.1.2.4:** Executive-assistant skill pack and audit view.

- [ ] **Task 54.1.3 - Verify and close the story**
  - [ ] **Sub-task 54.1.3.1:** `S-047-UT01` ranks labeled commitments/priorities across urgency, importance, dependency, owner, due date, confidence, conflict, stale evidence, and missing fields; assert deterministic explanation and tie handling.
  - [ ] **Sub-task 54.1.3.2:** `S-047-UT02` generates briefings, decision logs, follow-ups, and correspondence drafts from fixed evidence; assert every material fact/citation/unknown and no invented commitment, recipient, date, or decision.
  - [ ] **Sub-task 54.1.3.3:** `S-047-ST01` attempts hidden prioritization, inferred sensitive traits, unauthorized memory, automatic assignment, notification, send, schedule, or file mutation; assert proposal-only behavior and explicit approval boundaries.
  - [ ] **Sub-task 54.1.3.4:** `S-047-IT01` updates the portfolio after source correction, supersession, task completion, and conflict; assert all views reconcile to canonical sources with preserved history.
  - [ ] **Sub-task 54.1.3.5 - Product security evidence:** Map `SR-AI-003`/`SR-AI-004`/`SR-AI-007`/`SR-AI-010`, `SR-DAT-002`, `SR-CIV-001` through `SR-CIV-005`; retain ranking goldens, claim ledgers, prohibited-action traces, source reconciliation, and audit views.

##### Story Acceptance Criteria

- [ ] **Story AC 54.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every recommendation exposes source, method, assumptions, uncertainty, conflicts, and user-editable rationale; deterministic business rules take precedence where defined.
- [ ] **Story AC 54.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the assistant drafts and organizes locally but never commits another person, changes external state, sends, schedules, or promotes inferred facts without the required controlled workflow.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 54.AC1:** Every briefing, priority, reminder, and recommendation resolves to approved local sources.
- [ ] **Sprint AC 54.AC2:** Proposals, assumptions, preferences, and reported statements are never rendered as confirmed decisions.
- [ ] **Sprint AC 54.AC3:** Private classes remain isolated under their retrieval and export policies.
- [ ] **Sprint AC 54.AC4:** No invitation, message, commitment, calendar change, or contact action occurs automatically.
- [ ] **Sprint AC 54.AC5:** The same acceptance corpus passes against plain-folder and Obsidian knowledge stores.

**Gate decision:** Sprint 54 is PASS only when Story 54.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 55 - Meeting Records and Continuity

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-048`, part 1 of 2.

**Sprint goal:** Deliver meeting records and continuity as a bounded part of the legacy goal: Provide reliable local meeting, correspondence, deadline, document-control, and filing workflows without sending or moving anything automatically.

**Source coverage:** inventory Section 11B; secretary and document-control skills in Section 28.

**Dependencies:** Sprint 54; legacy dependency record: Sprint 54 (legacy S-047).

#### [ ] Story 55.1 - Meeting Records and Continuity

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need meeting records and continuity so that AgentMage delivers the following bounded outcome: Provide reliable local meeting, correspondence, deadline, document-control, and filing workflows without sending or moving anything automatically.

##### Tasks and Sub-tasks

- [ ] **Task 55.1.1 - Implement the bounded story**
  - [ ] **Sub-task 55.1.1.1** (legacy `S-048-I01`): Implement agenda and meeting-request drafts with purpose, participants, topics, decision needs, preparation, and outputs.
  - [ ] **Sub-task 55.1.1.2** (legacy `S-048-I02`): Implement attendee states without inferring acceptance or attendance.
  - [ ] **Sub-task 55.1.1.3** (legacy `S-048-I03`): Implement live-note and transcript cleanup that preserves verbatim source, timestamps, attribution confidence, and unclear-language markers.
  - [ ] **Sub-task 55.1.1.4** (legacy `S-048-I04`): Implement minutes with confirmed versus proposed decisions, actions, owners, dates, questions, risks, and next meeting.
  - [ ] **Sub-task 55.1.1.5** (legacy `S-048-I05`): Mark missing owner or date as unknown rather than guessing.
  - [ ] **Sub-task 55.1.1.6** (legacy `S-048-I06`): Implement closeout, recurring continuity, follow-up draft, and source-linked action carry-forward.

- [ ] **Task 55.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 55.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 55.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 55.1.3 - Verify and close the story**
  - [ ] **Sub-task 55.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 55.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 55.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 55.1.3.4 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-003`, `SR-AI-003`/`SR-AI-007`/`SR-AI-010`, `SR-CIV-003` through `SR-CIV-009`; retain labeled workflow results, attribution checks, disclosure/filing previews, accessibility output, and records-review fields.

##### Story Acceptance Criteria

- [ ] **Story AC 55.1.AC1:** Given the approved dependencies and source requirements for `S-048-I01`, `S-048-I02`, `S-048-I03`, `S-048-I04`, `S-048-I05`, and `S-048-I06`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 55.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-048-I01`, `S-048-I02`, `S-048-I03`, `S-048-I04`, `S-048-I05`, and `S-048-I06`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 55.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 55.AC1:** Every numbered implementation sub-task in Story 55.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 55.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 55.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 55.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 55.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 55 is PASS only when Story 55.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 56 - Document, Correspondence, and Filing Control

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-048`, part 2 of 2.

**Sprint goal:** Deliver document, correspondence, and filing control as a bounded part of the legacy goal: Provide reliable local meeting, correspondence, deadline, document-control, and filing workflows without sending or moving anything automatically.

**Source coverage:** inventory Section 11B; secretary and document-control skills in Section 28.

**Dependencies:** Sprint 55; legacy dependency record: Sprint 54 (legacy S-047).

#### [ ] Story 56.1 - Document, Correspondence, and Filing Control

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need document, correspondence, and filing control so that AgentMage delivers the following bounded outcome: Provide reliable local meeting, correspondence, deadline, document-control, and filing workflows without sending or moving anything automatically.

##### Tasks and Sub-tasks

- [ ] **Task 56.1.1 - Implement the bounded story**
  - [ ] **Sub-task 56.1.1.1** (legacy `S-048-I07`): Implement document and correspondence registers with version, approval, attachments, commitments, deadlines, hashes, and source paths.
  - [ ] **Sub-task 56.1.1.2** (legacy `S-048-I08`): Implement naming, duplicate, superseded, final-copy, quality, deadline, routing-slip, mail-merge preview, calendar-file draft, and filing-suggestion workflows.
  - [ ] **Sub-task 56.1.1.3** (legacy `S-048-I09`): Require exact preview and approval for every saved draft, rename, move, or filing action.

- [ ] **Task 56.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 56.1.2.1:** Meeting and secretary workflow schemas.
  - [ ] **Sub-task 56.1.2.2:** Document and correspondence registers.
  - [ ] **Sub-task 56.1.2.3:** Template pack for agendas, minutes, letters, memoranda, logs, and routing slips.
  - [ ] **Sub-task 56.1.2.4:** Records quality and filing-preview reports.

- [ ] **Task 56.1.3 - Verify and close the story**
  - [ ] **Sub-task 56.1.3.1:** `S-048-UT01` converts labeled meeting/correspondence inputs into agendas, minutes, action items, decisions, deadlines, registers, and routing slips; assert exact attribution and unresolved ambiguity.
  - [ ] **Sub-task 56.1.3.2:** `S-048-UT02` validates names, dates, owners, quorum/status, attachments, versions, record category, filing destination, and retention fields at empty/boundary/conflicting values.
  - [ ] **Sub-task 56.1.3.3:** `S-048-ST01` seeds unsupported identity claims, hidden recipients, malicious attachments, prompt instructions, sensitive content, and record-disposition requests; assert no invented attribution, send, move, delete, or disposition.
  - [ ] **Sub-task 56.1.3.4:** `S-048-IT01` performs draft/review/correct/version/finalize/file-preview workflows; assert source links, approvals, accessibility checks, naming rules, and no actual external filing.
  - [ ] **Sub-task 56.1.3.5 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-003`, `SR-AI-003`/`SR-AI-007`/`SR-AI-010`, `SR-CIV-003` through `SR-CIV-009`; retain labeled workflow results, attribution checks, disclosure/filing previews, accessibility output, and records-review fields.

##### Story Acceptance Criteria

- [ ] **Story AC 56.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then meeting and correspondence records distinguish verbatim source, observed fact, derived action, inferred summary, unresolved conflict, and user-approved final language.
- [ ] **Story AC 56.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then filing remains a preview until exact destination/content/metadata approval, and records schedule/disposition decisions remain with the designated records owner.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 56.AC1:** Raw notes and approved prior minutes remain unchanged.
- [ ] **Sprint AC 56.AC2:** Unknown owners, dates, attendance, and decisions remain explicitly unknown.
- [ ] **Sprint AC 56.AC3:** Follow-up drafts quote and link the approved record accurately.
- [ ] **Sprint AC 56.AC4:** Mail merge and calendar generation create local previews only.
- [ ] **Sprint AC 56.AC5:** No automatic sending, scheduling, recipient selection, records disposition, or silent final-document change is possible.

**Gate decision:** Sprint 56 is PASS only when Story 56.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 57 - Markdown and Plain-Text Artifacts

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-049`.

**Sprint goal:** Produce and edit high-fidelity plain-text work products with exact structural preservation and citations.

**Source coverage:** inventory Section 16 and applicable Section 28 skills.

**Dependencies:** Sprint 56; legacy dependency record: Sprint 36 (legacy S-029), Sprint 38 (legacy S-031).

#### [ ] Story 57.1 - Markdown and Plain-Text Artifacts

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need markdown and plain-text artifacts so that AgentMage delivers the following bounded outcome: Produce and edit high-fidelity plain-text work products with exact structural preservation and citations.

##### Tasks and Sub-tasks

- [ ] **Task 57.1.1 - Implement the bounded story**
  - [ ] **Sub-task 57.1.1.1** (legacy `S-049-I01`): Complete parsing and writing for headings, paragraphs, lists, tables, code fences, links, frontmatter, line endings, and source ranges.
  - [ ] **Sub-task 57.1.1.2** (legacy `S-049-I02`): Implement CommonMark spacing, broken-link, duplicate-heading, malformed-table, and structure checks.
  - [ ] **Sub-task 57.1.1.3** (legacy `S-049-I03`): Implement configurable plain-language review and acronym handling without guessing expansions.
  - [ ] **Sub-task 57.1.1.4** (legacy `S-049-I04`): Implement meeting cleanup, status report, standup script, task document, handoff, decision, and evidence-report generation.
  - [ ] **Sub-task 57.1.1.5** (legacy `S-049-I05`): Preserve unrelated content and apply only exact previewed changes.
  - [ ] **Sub-task 57.1.1.6** (legacy `S-049-I06`): Generate non-authoritative local file-and-line display links from validated workspace paths.
  - [ ] **Sub-task 57.1.1.7** (legacy `S-049-I07`): Implement deterministic round-trip and visual rendering checks where applicable.

- [ ] **Task 57.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 57.1.2.1:** Plain-text and Markdown artifact pack.
  - [ ] **Sub-task 57.1.2.2:** Structure-preservation and language-quality fixtures.
  - [ ] **Sub-task 57.1.2.3:** Report and handoff templates.
  - [ ] **Sub-task 57.1.2.4:** Round-trip result bundle.

- [ ] **Task 57.1.3 - Verify and close the story**
  - [ ] **Sub-task 57.1.3.1:** `S-049-UT01` parses/renders headings, lists, tables, links, code fences, frontmatter, comments, whitespace, encodings, and line endings; assert structural identity for unchanged content.
  - [ ] **Sub-task 57.1.3.2:** `S-049-UT02` applies scoped edits and language-quality checks to malformed, mixed-format, long-word, narrow-width, and unsupported syntax fixtures; assert exact requested changes and visible limitations.
  - [ ] **Sub-task 57.1.3.3:** `S-049-ST01` seeds executable HTML/script, remote assets, dangerous URI schemes, hidden text, secret canaries, and instruction content; assert inert treatment, redaction policy, and no network/execution.
  - [ ] **Sub-task 57.1.3.4:** `S-049-IT01` generates reports/handoffs/templates, reopens them, resolves citations, and compares semantic/byte/rendered structure; assert accessibility and no unrelated source modification.
  - [ ] **Sub-task 57.1.3.5 - Product security evidence:** Map `SR-DAT-002`/`SR-DAT-003`, `SR-AI-010`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain round-trip goldens, scoped diffs, inert-content results, citation checks, and accessibility report.

##### Story Acceptance Criteria

- [ ] **Story AC 57.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then generated artifacts are valid, readable, accessible, source-grounded, and round-trip without silent loss of supported structure.
- [ ] **Story AC 57.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then unsupported or potentially executable content is preserved inert or blocked with a precise limitation; it never executes or disappears silently.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 57.AC1:** Unchanged Markdown round-trips byte-identically where no normalization was approved.
- [ ] **Sprint AC 57.AC2:** Edited documents change only previewed structures.
- [ ] **Sprint AC 57.AC3:** Code fences and raw-note regions remain intact.
- [ ] **Sprint AC 57.AC4:** Unknown acronyms remain marked rather than expanded incorrectly.
- [ ] **Sprint AC 57.AC5:** Every generated factual statement carries its required evidence state and citation.

**Gate decision:** Sprint 57 is PASS only when Story 57.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 58 - Word Extraction and Structural Preservation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-050`, part 1 of 2.

**Sprint goal:** Deliver word extraction and structural preservation as a bounded part of the legacy goal: Read, generate, edit, and visually verify Word documents without silently damaging originals or unsupported structure.

**Source coverage:** inventory Sections 17 and 26B Word requirements.

**Dependencies:** Sprint 57; legacy dependency record: Sprint 36 (legacy S-029), Sprint 57 (legacy S-049).

#### [ ] Story 58.1 - Word Extraction and Structural Preservation

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need word extraction and structural preservation so that AgentMage delivers the following bounded outcome: Read, generate, edit, and visually verify Word documents without silently damaging originals or unsupported structure.

##### Tasks and Sub-tasks

- [ ] **Task 58.1.1 - Implement the bounded story**
  - [ ] **Sub-task 58.1.1.1** (legacy `S-050-I01`): Select and approve cross-platform Word extraction, generation, rendering, and inspection dependencies with pinned versions, hashes, licenses, and isolated environments.
  - [ ] **Sub-task 58.1.1.2** (legacy `S-050-I02`): Implement deterministic source-to-sidecar text extraction with source hash and conversion identity caching.
  - [ ] **Sub-task 58.1.1.3** (legacy `S-050-I03`): Preserve originals and report tables, comments, tracked changes, headers, footers, numbering, and layout that extraction may lose.
  - [ ] **Sub-task 58.1.1.4** (legacy `S-050-I04`): Implement bounded document inspection and structured Markdown-to-Word generation.

- [ ] **Task 58.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 58.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 58.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 58.1.3 - Verify and close the story**
  - [ ] **Sub-task 58.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 58.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 58.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 58.1.3.4 - Product security evidence:** Map `SR-SUP-003`/`SR-SUP-006`/`SR-SUP-008`/`SR-SUP-009`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain dependency manifest, package diffs, parser corpus, render comparisons, accessibility results, and independent native-boundary review.

##### Story Acceptance Criteria

- [ ] **Story AC 58.1.AC1:** Given the approved dependencies and source requirements for `S-050-I01`, `S-050-I02`, `S-050-I03`, and `S-050-I04`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 58.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-050-I01`, `S-050-I02`, `S-050-I03`, and `S-050-I04`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 58.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 58.AC1:** Every numbered implementation sub-task in Story 58.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 58.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 58.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 58.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 58.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 58 is PASS only when Story 58.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 59 - Word Generation and Visual Verification

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-050`, part 2 of 2.

**Sprint goal:** Deliver word generation and visual verification as a bounded part of the legacy goal: Read, generate, edit, and visually verify Word documents without silently damaging originals or unsupported structure.

**Source coverage:** inventory Sections 17 and 26B Word requirements.

**Dependencies:** Sprint 58; legacy dependency record: Sprint 36 (legacy S-029), Sprint 57 (legacy S-049).

#### [ ] Story 59.1 - Word Generation and Visual Verification

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need word generation and visual verification so that AgentMage delivers the following bounded outcome: Read, generate, edit, and visually verify Word documents without silently damaging originals or unsupported structure.

##### Tasks and Sub-tasks

- [ ] **Task 59.1.1 - Implement the bounded story**
  - [ ] **Sub-task 59.1.1.1** (legacy `S-050-I05`): Implement styles, page setup, headers, footers, metadata, warnings, tables, decision cards, hyperlinks, borders, margins, numbering, paragraph, cloning, and section-replacement helpers.
  - [ ] **Sub-task 59.1.1.2** (legacy `S-050-I06`): Implement controlled edit, redline, comment, and exact change preview.
  - [ ] **Sub-task 59.1.1.3** (legacy `S-050-I07`): Render on every supported platform and compare page images for layout regressions.
  - [ ] **Sub-task 59.1.1.4** (legacy `S-050-I08`): Emit artifact receipts with inputs, generator version, changes, checks, render outputs, and known fidelity limits.

- [ ] **Task 59.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 59.1.2.1:** Approved Word dependency manifest.
  - [ ] **Sub-task 59.1.2.2:** Word extraction, generation, edit, and render adapters.
  - [ ] **Sub-task 59.1.2.3:** Complex Word fixture corpus.
  - [ ] **Sub-task 59.1.2.4:** Round-trip and visual comparison reports.

- [ ] **Task 59.1.3 - Verify and close the story**
  - [ ] **Sub-task 59.1.3.1:** `S-050-UT01` extracts paragraphs, headings, lists, tables, headers/footers, notes, links, images, fields, comments, revisions, styles, properties, and unsupported constructs with exact part/range provenance.
  - [ ] **Sub-task 59.1.3.2:** `S-050-UT02` generates and edits representative documents, then reopens package XML; assert valid relationships/content types, intended semantics/styles, no macros/external links, and preserved untouched parts.
  - [ ] **Sub-task 59.1.3.3:** `S-050-ST01` supplies macro-enabled, encrypted, malformed, zip-bomb, path-traversal, external-template, active-link, hidden-content, and parser-crash fixtures; assert quarantine/bounded failure and no execution/network.
  - [ ] **Sub-task 59.1.3.4:** `S-050-IT01` renders before/after pages at pinned settings and runs structural plus visual comparison; assert declared thresholds for pagination, clipping, overlap, font fallback, tables, and images with human-review flags.
  - [ ] **Sub-task 59.1.3.5 - Product security evidence:** Map `SR-SUP-003`/`SR-SUP-006`/`SR-SUP-008`/`SR-SUP-009`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain dependency manifest, package diffs, parser corpus, render comparisons, accessibility results, and independent native-boundary review.

##### Story Acceptance Criteria

- [ ] **Story AC 59.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no original Word file is overwritten; generated/edited copies preserve supported content and disclose every unsupported, removed, substituted, or visually changed feature.
- [ ] **Story AC 59.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then package validation, semantic checks, visual checks, accessibility checks, malware policy, and canary scans all pass before an artifact can be marked complete.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 59.AC1:** Extraction never changes the original.
- [ ] **Sprint AC 59.AC2:** Lossy or unsupported structures are reported before use as evidence.
- [ ] **Sprint AC 59.AC3:** Generated files pass structural inspection and visual review on every reference platform.
- [ ] **Sprint AC 59.AC4:** Edits preserve unapproved sections, comments, links, numbering, and metadata where supported.
- [ ] **Sprint AC 59.AC5:** Fidelity failures block completion and are never hidden by text-only success.

**Gate decision:** Sprint 59 is PASS only when Story 59.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 60 - PDF Extraction and Page Citations

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-051`, part 1 of 2.

**Sprint goal:** Deliver pdf extraction and page citations as a bounded part of the legacy goal: Extract, cite, generate, redact, render, and verify Portable Document Format artifacts locally.

**Source coverage:** inventory Sections 18 and 26B Portable Document Format requirements.

**Dependencies:** Sprint 59; legacy dependency record: Sprint 36 (legacy S-029), Sprint 57 (legacy S-049).

#### [ ] Story 60.1 - PDF Extraction and Page Citations

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need pdf extraction and page citations so that AgentMage delivers the following bounded outcome: Extract, cite, generate, redact, render, and verify Portable Document Format artifacts locally.

##### Tasks and Sub-tasks

- [ ] **Task 60.1.1 - Implement the bounded story**
  - [ ] **Sub-task 60.1.1.1** (legacy `S-051-I01`): Select and approve cross-platform extraction, metadata, rendering, generation, and optional optical-character-recognition dependencies.
  - [ ] **Sub-task 60.1.1.2** (legacy `S-051-I02`): Implement bounded text extraction with page identities and exact-page citations.
  - [ ] **Sub-task 60.1.1.3** (legacy `S-051-I03`): Detect scanned, encrypted, malformed, truncated, or extraction-limited pages.
  - [ ] **Sub-task 60.1.1.4** (legacy `S-051-I04`): Run optical character recognition only through an approved local package and preserve uncertainty and source-page identity.

- [ ] **Task 60.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 60.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 60.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 60.1.3 - Verify and close the story**
  - [ ] **Sub-task 60.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 60.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 60.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 60.1.3.4 - Product security evidence:** Map `SR-DAT-002`/`SR-DAT-003`, `SR-SUP-008`/`SR-SUP-009`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain PDF manifests, extraction goldens, redaction residue scans, render diffs, accessibility and independent review results.

##### Story Acceptance Criteria

- [ ] **Story AC 60.1.AC1:** Given the approved dependencies and source requirements for `S-051-I01`, `S-051-I02`, `S-051-I03`, and `S-051-I04`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 60.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-051-I01`, `S-051-I02`, `S-051-I03`, and `S-051-I04`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 60.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 60.AC1:** Every numbered implementation sub-task in Story 60.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 60.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 60.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 60.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 60.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 60 is PASS only when Story 60.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 61 - PDF Generation, Redaction, and Visual Verification

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-051`, part 2 of 2.

**Sprint goal:** Deliver pdf generation, redaction, and visual verification as a bounded part of the legacy goal: Extract, cite, generate, redact, render, and verify Portable Document Format artifacts locally.

**Source coverage:** inventory Sections 18 and 26B Portable Document Format requirements.

**Dependencies:** Sprint 60; legacy dependency record: Sprint 36 (legacy S-029), Sprint 57 (legacy S-049).

#### [ ] Story 61.1 - PDF Generation, Redaction, and Visual Verification

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need pdf generation, redaction, and visual verification so that AgentMage delivers the following bounded outcome: Extract, cite, generate, redact, render, and verify Portable Document Format artifacts locally.

##### Tasks and Sub-tasks

- [ ] **Task 61.1.1 - Implement the bounded story**
  - [ ] **Sub-task 61.1.1.1** (legacy `S-051-I05`): Extract title, author, page count, creation metadata, links, forms, and encryption state.
  - [ ] **Sub-task 61.1.1.2** (legacy `S-051-I06`): Implement Markdown-to-HTML and Portable Document Format report export with safe names, escaped content, tables, links, and local-only assets.
  - [ ] **Sub-task 61.1.1.3** (legacy `S-051-I07`): Replace remote Mermaid dependencies with an offline renderer.
  - [ ] **Sub-task 61.1.1.4** (legacy `S-051-I08`): Implement creation, redaction, fillable-form, page-image, metadata, and visual verification workflows.

- [ ] **Task 61.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 61.1.2.1:** Approved Portable Document Format dependency manifest.
  - [ ] **Sub-task 61.1.2.2:** Extractor, citation, renderer, exporter, and verifier adapters.
  - [ ] **Sub-task 61.1.2.3:** Scanned, encrypted, malformed, form, link, and layout fixtures.
  - [ ] **Sub-task 61.1.2.4:** Page-level evidence and round-trip reports.

- [ ] **Task 61.1.3 - Verify and close the story**
  - [ ] **Sub-task 61.1.3.1:** `S-051-UT01` extracts text, coordinates, pages, metadata, links, forms, images, OCR layers, and encryption/permission state from labeled PDFs; assert exact page/object provenance and visible extraction limits.
  - [ ] **Sub-task 61.1.3.2:** `S-051-UT02` generates/merges/splits/redacts supported fixtures and reparses them; assert valid structure, expected pages, removed target content, no hidden residual text/metadata, and stable citations.
  - [ ] **Sub-task 61.1.3.3:** `S-051-ST01` supplies encrypted, malformed, oversized, recursive object, JavaScript, launch action, embedded file, remote-link, font, image bomb, and parser-crash fixtures; assert inert bounded processing.
  - [ ] **Sub-task 61.1.3.4:** `S-051-IT01` rasterizes every output and compares page geometry, clipping, overlap, order, fonts, redactions, and accessibility; assert thresholds plus manual review for ambiguous visual differences.
  - [ ] **Sub-task 61.1.3.5 - Product security evidence:** Map `SR-DAT-002`/`SR-DAT-003`, `SR-SUP-008`/`SR-SUP-009`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain PDF manifests, extraction goldens, redaction residue scans, render diffs, accessibility and independent review results.

##### Story Acceptance Criteria

- [ ] **Story AC 61.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every cited fact resolves to document hash, page, bounding/source region where available, extraction method, and confidence/limitation.
- [ ] **Story AC 61.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then redaction removes target information from visible, text, object, metadata, attachment, incremental-update, and searchable layers as defined by the approved redaction profile.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 61.AC1:** Every extracted claim resolves to an exact source page and file hash.
- [ ] **Sprint AC 61.AC2:** Scanned and failed extraction remains Unknown/Blocked unless approved local optical recognition succeeds.
- [ ] **Sprint AC 61.AC3:** Report export uses no content-delivery network, remote font, or web dependency.
- [ ] **Sprint AC 61.AC4:** Redaction tests prove removed content is absent from text, metadata, objects, and rendered output.
- [ ] **Sprint AC 61.AC5:** Structural and visual verification pass before an artifact is complete.

**Gate decision:** Sprint 61 is PASS only when Story 61.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 62 - Spreadsheet, CSV, and JSON Parsing

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-052`, part 1 of 2.

**Sprint goal:** Deliver spreadsheet, csv, and json parsing as a bounded part of the legacy goal: Inspect, compare, reconcile, generate, and validate structured office data without altering sources or introducing executable formulas.

**Source coverage:** inventory Section 19 and Section 26B spreadsheet requirements.

**Dependencies:** Sprint 61; legacy dependency record: Sprint 36 (legacy S-029).

#### [ ] Story 62.1 - Spreadsheet, CSV, and JSON Parsing

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need spreadsheet, csv, and json parsing so that AgentMage delivers the following bounded outcome: Inspect, compare, reconcile, generate, and validate structured office data without altering sources or introducing executable formulas.

##### Tasks and Sub-tasks

- [ ] **Task 62.1.1 - Implement the bounded story**
  - [ ] **Sub-task 62.1.1.1** (legacy `S-052-I01`): Select approved isolated spreadsheet dependencies and retain a direct Open XML read path for deterministic inspection.
  - [ ] **Sub-task 62.1.1.2** (legacy `S-052-I02`): Implement workbook, worksheet, cell, formula, displayed value, style, date, hyperlink, hidden row, hidden column, and source-hash records.
  - [ ] **Sub-task 62.1.1.3** (legacy `S-052-I03`): Implement bounded read-only summaries, filtering, sorting, matching, duplicates, missing values, overlap, differing fields, and reason codes.
  - [ ] **Sub-task 62.1.1.4** (legacy `S-052-I04`): Implement safe CSV parsing and writing with formula-injection prevention and stable normalization helpers.

- [ ] **Task 62.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 62.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 62.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 62.1.3 - Verify and close the story**
  - [ ] **Sub-task 62.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 62.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 62.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 62.1.3.4 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-003`, `SR-SUP-008`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`, `SR-CIV-008`; retain normalization goldens, reconciliation calculations, injection corpus, independent reopen/recalc output, and visual/accessibility checks.

##### Story Acceptance Criteria

- [ ] **Story AC 62.1.AC1:** Given the approved dependencies and source requirements for `S-052-I01`, `S-052-I02`, `S-052-I03`, and `S-052-I04`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 62.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-052-I01`, `S-052-I02`, `S-052-I03`, and `S-052-I04`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 62.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 62.AC1:** Every numbered implementation sub-task in Story 62.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 62.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 62.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 62.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 62.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 62 is PASS only when Story 62.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 63 - Reconciliation, Safe Output, and Verification

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-052`, part 2 of 2.

**Sprint goal:** Deliver reconciliation, safe output, and verification as a bounded part of the legacy goal: Inspect, compare, reconcile, generate, and validate structured office data without altering sources or introducing executable formulas.

**Source coverage:** inventory Section 19 and Section 26B spreadsheet requirements.

**Dependencies:** Sprint 62; legacy dependency record: Sprint 36 (legacy S-029).

#### [ ] Story 63.1 - Reconciliation, Safe Output, and Verification

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need reconciliation, safe output, and verification so that AgentMage delivers the following bounded outcome: Inspect, compare, reconcile, generate, and validate structured office data without altering sources or introducing executable formulas.

##### Tasks and Sub-tasks

- [ ] **Task 63.1.1 - Implement the bounded story**
  - [ ] **Sub-task 63.1.1.1** (legacy `S-052-I05`): Implement JSON parsing, schema validation, stable key ordering, size limits, and redaction.
  - [ ] **Sub-task 63.1.1.2** (legacy `S-052-I06`): Implement deterministic comparisons with source hashes and match reasons.
  - [ ] **Sub-task 63.1.1.3** (legacy `S-052-I07`): Generate reconciliation workbooks with escaped text, formulas, styles, tables, charts, validation, and formula-error scanning.
  - [ ] **Sub-task 63.1.1.4** (legacy `S-052-I08`): Recalculate and visually verify generated workbooks while preserving originals.

- [ ] **Task 63.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 63.1.2.1:** Spreadsheet, CSV, and JSON adapters.
  - [ ] **Sub-task 63.1.2.2:** Normalization and reconciliation method registry.
  - [ ] **Sub-task 63.1.2.3:** Formula-injection and malformed-data corpus.
  - [ ] **Sub-task 63.1.2.4:** Structural, recalculation, and visual verification reports.

- [ ] **Task 63.1.3 - Verify and close the story**
  - [ ] **Sub-task 63.1.3.1:** `S-052-UT01` parses types, nulls, dates/time zones, decimals, formulas, errors, names, sheets, merged cells, hidden data, JSON nesting, CSV dialects, and malformed/large inputs; assert declared normalization.
  - [ ] **Sub-task 63.1.3.2:** `S-052-UT02` runs reconciliation methods with duplicates, rounding, missing keys, many-to-many joins, conflicts, tolerance boundaries, and stale data; assert deterministic totals, unmatched rows, and method provenance.
  - [ ] **Sub-task 63.1.3.3:** `S-052-ST01` seeds formula/CSV injection, external links, macros, DDE, hidden sheets/rows, unsafe numbers, prototype-like keys, zip bombs, and secrets; assert inert output, no recalculation side effect, and disclosure.
  - [ ] **Sub-task 63.1.3.4:** `S-052-IT01` writes outputs, reopens them with independent parsers/calculation where approved, and compares structure/values/formulas/rendering; assert no source overwrite or silent precision/type loss.
  - [ ] **Sub-task 63.1.3.5 - Product security evidence:** Map `SR-DAT-001` through `SR-DAT-003`, `SR-SUP-008`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`, `SR-CIV-008`; retain normalization goldens, reconciliation calculations, injection corpus, independent reopen/recalc output, and visual/accessibility checks.

##### Story Acceptance Criteria

- [ ] **Story AC 63.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every transformation/reconciliation records source hashes, schema/type decisions, formula policy, join keys, tolerances, unmatched/conflicting rows, totals, and reproducible method version.
- [ ] **Story AC 63.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then produced cells/CSV fields cannot execute when opened under the approved threat model; risky content is escaped, removed, or blocked and reported.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 63.AC1:** Source workbooks and CSV files remain unchanged during inspection and comparison.
- [ ] **Sprint AC 63.AC2:** Every match and discrepancy includes deterministic reason codes and source identities.
- [ ] **Sprint AC 63.AC3:** Generated CSV values cannot execute as formulas when opened in common spreadsheet software.
- [ ] **Sprint AC 63.AC4:** Generated workbooks contain no formula errors and pass rendered visual checks.
- [ ] **Sprint AC 63.AC5:** Malformed, oversized, encrypted, unsupported, and ambiguous input fails explicitly.

**Gate decision:** Sprint 63 is PASS only when Story 63.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 64 - Presentation Workflows

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-053`, part 1 of 2.

**Sprint goal:** Deliver presentation workflows as a bounded part of the legacy goal: Add presentation and image workflows with provenance, redaction, round-trip checks, and visual evidence.

**Source coverage:** inventory Sections 19A, 20, and 26B presentation, diagram, image, and visualization requirements.

**Dependencies:** Sprint 63; legacy dependency record: Sprint 36 (legacy S-029), Sprint 57 (legacy S-049).

#### [ ] Story 64.1 - Presentation Workflows

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need presentation workflows so that AgentMage delivers the following bounded outcome: Add presentation and image workflows with provenance, redaction, round-trip checks, and visual evidence.

##### Tasks and Sub-tasks

- [ ] **Task 64.1.1 - Implement the bounded story**
  - [ ] **Sub-task 64.1.1.1** (legacy `S-053-I01`): Implement presentation extraction for slides, speaker notes, links, images, captions, layouts, and source hashes without executing embedded content.
  - [ ] **Sub-task 64.1.1.2** (legacy `S-053-I02`): Implement controlled presentation creation and editing with exact slide previews.
  - [ ] **Sub-task 64.1.1.3** (legacy `S-053-I06`): Implement diagrams, plots, charts, comparison tables, and small local visualizations with deterministic data sources.

- [ ] **Task 64.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 64.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 64.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 64.1.3 - Verify and close the story**
  - [ ] **Sub-task 64.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 64.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 64.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 64.1.3.4 - Product security evidence:** Map `SR-DAT-002`/`SR-DAT-003`, `SR-SUP-008`/`SR-SUP-009`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain package/pixel goldens, hostile corpus, metadata/redaction scans, render diffs, and accessibility review.

##### Story Acceptance Criteria

- [ ] **Story AC 64.1.AC1:** Given the approved dependencies and source requirements for `S-053-I01`, `S-053-I02`, and `S-053-I06`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 64.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-053-I01`, `S-053-I02`, and `S-053-I06`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 64.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 64.AC1:** Every numbered implementation sub-task in Story 64.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 64.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 64.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 64.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 64.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 64 is PASS only when Story 64.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 65 - Images, Redaction, and Visual Verification

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-053`, part 2 of 2.

**Sprint goal:** Deliver images, redaction, and visual verification as a bounded part of the legacy goal: Add presentation and image workflows with provenance, redaction, round-trip checks, and visual evidence.

**Source coverage:** inventory Sections 19A, 20, and 26B presentation, diagram, image, and visualization requirements.

**Dependencies:** Sprint 64; legacy dependency record: Sprint 36 (legacy S-029), Sprint 57 (legacy S-049).

#### [ ] Story 65.1 - Images, Redaction, and Visual Verification

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need images, redaction, and visual verification so that AgentMage delivers the following bounded outcome: Add presentation and image workflows with provenance, redaction, round-trip checks, and visual evidence.

##### Tasks and Sub-tasks

- [ ] **Task 65.1.1 - Implement the bounded story**
  - [ ] **Sub-task 65.1.1.1** (legacy `S-053-I03`): Implement image metadata for dimensions, type, color space, size, and provenance.
  - [ ] **Sub-task 65.1.1.2** (legacy `S-053-I04`): Implement bounded local screenshot and image viewing for approved vision-capable profiles.
  - [ ] **Sub-task 65.1.1.3** (legacy `S-053-I05`): Implement sensitive-region review and redaction before model use or export.
  - [ ] **Sub-task 65.1.1.4** (legacy `S-053-I07`): Implement before-and-after visual diff for documents, slides, images, and user interfaces.
  - [ ] **Sub-task 65.1.1.5** (legacy `S-053-I08`): Add optional image generation or editing only through an approved local or explicitly approved provider-backed profile with a disclosure preview and receipt.

- [ ] **Task 65.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 65.1.2.1:** Presentation and image adapters.
  - [ ] **Sub-task 65.1.2.2:** Visual artifact and provenance schema.
  - [ ] **Sub-task 65.1.2.3:** Redaction and before-after comparison corpus.
  - [ ] **Sub-task 65.1.2.4:** Rendered presentation and image verification bundle.

- [ ] **Task 65.1.3 - Verify and close the story**
  - [ ] **Sub-task 65.1.3.1:** `S-053-UT01` extracts slide/object/order/text/note/media/theme/link/alt-text and image dimensions/color/metadata from labeled fixtures; assert stable object provenance and unsupported-feature flags.
  - [ ] **Sub-task 65.1.3.2:** `S-053-UT02` generates/edits presentations and raster images, then reparses; assert valid packages/pixels, exact intended changes, preserved untouched content, and provenance metadata.
  - [ ] **Sub-task 65.1.3.3:** `S-053-ST01` tests external links/media, embedded executables/OLE, active actions, malformed packages, decompression bombs, steganographic/metadata canaries, and parser exploits; assert quarantine/inert handling.
  - [ ] **Sub-task 65.1.3.4:** `S-053-IT01` renders all slides/images across target dimensions and compares clipping, overlap, order, contrast, fonts, cropping, redaction, and alt text; assert declared visual/accessibility thresholds.
  - [ ] **Sub-task 65.1.3.5 - Product security evidence:** Map `SR-DAT-002`/`SR-DAT-003`, `SR-SUP-008`/`SR-SUP-009`, `SR-TST-002`/`SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`; retain package/pixel goldens, hostile corpus, metadata/redaction scans, render diffs, and accessibility review.

##### Story Acceptance Criteria

- [ ] **Story AC 65.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every generated visual artifact has source/provenance, content/structure validation, rendered comparison, accessibility results, and disclosure of substituted or unsupported features.
- [ ] **Story AC 65.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then redaction and metadata removal are verified on decoded pixels, package parts, notes, thumbnails, relationships, and exported files, not accepted from visual appearance alone.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 65.AC1:** No macros, scripts, formulas, attachments, or embedded executables run during inspection.
- [ ] **Sprint AC 65.AC2:** Extracted notes, links, captions, and images preserve slide identity.
- [ ] **Sprint AC 65.AC3:** Sensitive screenshots are blocked or redacted before model context or export.
- [ ] **Sprint AC 65.AC4:** Generated and edited artifacts pass structural and visual review.
- [ ] **Sprint AC 65.AC5:** Provider-backed generation cannot occur in strict-local mode or without exact disclosure approval.

**Gate decision:** Sprint 65 is PASS only when Story 65.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 66 - Safe Additional File Parsers

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-054`, part 1 of 2.

**Sprint goal:** Deliver safe additional file parsers as a bounded part of the legacy goal: Complete the promoted file-type roadmap using bounded non-executing parsers and explicit fidelity limits.

**Source coverage:** inventory Section 19A; Section 26B audio and rich-artifact requirements.

**Dependencies:** Sprint 65; legacy dependency record: Sprint 57 (legacy S-049) through Sprint 65 (legacy S-053).

#### [ ] Story 66.1 - Safe Additional File Parsers

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need safe additional file parsers so that AgentMage delivers the following bounded outcome: Complete the promoted file-type roadmap using bounded non-executing parsers and explicit fidelity limits.

##### Tasks and Sub-tasks

- [ ] **Task 66.1.1 - Implement the bounded story**
  - [ ] **Sub-task 66.1.1.1** (legacy `S-054-I01`): Implement local parsing for saved HTML, XML, YAML, notebooks, and structured logs with source ranges and size limits.
  - [ ] **Sub-task 66.1.1.2** (legacy `S-054-I02`): Extract notebook Markdown, code, outputs, and execution metadata without executing cells.
  - [ ] **Sub-task 66.1.1.3** (legacy `S-054-I03`): Extract YAML configuration with secret redaction and no executable constructors.
  - [ ] **Sub-task 66.1.1.4** (legacy `S-054-I04`): Parse timestamped text, JSON Lines, stack traces, and bounded event sequences from logs.
  - [ ] **Sub-task 66.1.1.5** (legacy `S-054-I05`): Add archive inventory and quarantine without opening nested or hostile content outside bounded scratch.
  - [ ] **Sub-task 66.1.1.6** (legacy `S-054-I08`): Keep deferred proprietary formats and all embedded execution disabled until separately promoted.

- [ ] **Task 66.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 66.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 66.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 66.1.3 - Verify and close the story**
  - [ ] **Sub-task 66.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 66.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 66.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 66.1.3.4 - Product security evidence:** Map `SR-SUP-003`/`SR-SUP-006`/`SR-SUP-008`/`SR-SUP-009`, `SR-AI-007`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`; retain per-format BOM, fuzz/adversarial results, transcription metrics, round-trip comparisons, and capability decision.

##### Story Acceptance Criteria

- [ ] **Story AC 66.1.AC1:** Given the approved dependencies and source requirements for `S-054-I01`, `S-054-I02`, `S-054-I03`, `S-054-I04`, `S-054-I05`, and `S-054-I08`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 66.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-054-I01`, `S-054-I02`, `S-054-I03`, `S-054-I04`, `S-054-I05`, and `S-054-I08`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 66.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 66.AC1:** Every numbered implementation sub-task in Story 66.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 66.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 66.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 66.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 66.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 66 is PASS only when Story 66.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 67 - Local Audio Transcription and Common Receipts

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-054`, part 2 of 2.

**Sprint goal:** Deliver local audio transcription and common receipts as a bounded part of the legacy goal: Complete the promoted file-type roadmap using bounded non-executing parsers and explicit fidelity limits.

**Source coverage:** inventory Section 19A; Section 26B audio and rich-artifact requirements.

**Dependencies:** Sprint 66; legacy dependency record: Sprint 57 (legacy S-049) through Sprint 65 (legacy S-053).

#### [ ] Story 67.1 - Local Audio Transcription and Common Receipts

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need local audio transcription and common receipts so that AgentMage delivers the following bounded outcome: Complete the promoted file-type roadmap using bounded non-executing parsers and explicit fidelity limits.

##### Tasks and Sub-tasks

- [ ] **Task 67.1.1 - Implement the bounded story**
  - [ ] **Sub-task 67.1.1.1** (legacy `S-054-I06`): Add optional local audio transcription with timestamps, speaker uncertainty, source identity, and original-audio retention controls.
  - [ ] **Sub-task 67.1.1.2** (legacy `S-054-I07`): Emit a common artifact receipt for every converter, parser, generator, renderer, redactor, and verifier.

- [ ] **Task 67.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 67.1.2.1:** Additional parser and transcription adapters.
  - [ ] **Sub-task 67.1.2.2:** Common artifact receipt schema.
  - [ ] **Sub-task 67.1.2.3:** Hostile archive, parser, notebook, YAML, and log corpus.
  - [ ] **Sub-task 67.1.2.4:** Fidelity and unsupported-feature matrix.

- [ ] **Task 67.1.3 - Verify and close the story**
  - [ ] **Sub-task 67.1.3.1:** `S-054-UT01` validates each promoted parser/transcriber against normal, empty, boundary, malformed, oversized, unsupported-codec/feature, and mixed-encoding fixtures; assert stable typed output and limitations.
  - [ ] **Sub-task 67.1.3.2:** `S-054-ST01` attacks archives/notebooks/YAML/logs/audio and other formats with traversal, bombs, aliases, object tags, executable cells, terminal controls, remote references, polyglots, and parser exploits; assert non-execution and bounded resources.
  - [ ] **Sub-task 67.1.3.3:** `S-054-UT02` checks audio segmentation/timestamps/speaker labels/confidence and transcription uncertainty against labeled fixtures; assert no invented words/speakers and exact source-time citations.
  - [ ] **Sub-task 67.1.3.4:** `S-054-IT01` runs extraction/export/round-trip and common-receipt validation for every type; assert provenance, source invariance, cleanup, and an explicit fidelity/unsupported matrix.
  - [ ] **Sub-task 67.1.3.5 - Product security evidence:** Map `SR-SUP-003`/`SR-SUP-006`/`SR-SUP-008`/`SR-SUP-009`, `SR-AI-007`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`; retain per-format BOM, fuzz/adversarial results, transcription metrics, round-trip comparisons, and capability decision.

##### Story Acceptance Criteria

- [ ] **Story AC 67.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then a file type is enabled only with a pinned dependency, threat model, resource limits, hostile corpus, provenance model, fidelity threshold, accessibility impact, and removal path.
- [ ] **Story AC 67.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then executable semantics remain disabled; unsupported or ambiguous content is preserved inert, omitted with disclosure, or blocks completion.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 67.AC1:** No parser executes macros, scripts, formulas, notebook cells, YAML constructors, or attachments.
- [ ] **Sprint AC 67.AC2:** Every extracted item retains source file, hash, location, parser identity, and limitation state.
- [ ] **Sprint AC 67.AC3:** Archive traversal, decompression expansion, nested content, and malicious names remain bounded.
- [ ] **Sprint AC 67.AC4:** Transcription distinguishes observed audio positions from inferred speakers or unclear language.
- [ ] **Sprint AC 67.AC5:** Unsupported formats remain explicit exclusions rather than silent partial support.

**Gate decision:** Sprint 67 is PASS only when Story 67.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 68 - Local Database and Structured Evidence

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-055`.

**Sprint goal:** Support AgentMage-owned SQLite and approved fixture data through parameterized, bounded, auditable adapters.

**Source coverage:** inventory Sections 21 and 22.

**Dependencies:** Sprint 67; legacy dependency record: Sprint 11 (legacy S-011), Sprint 63 (legacy S-052).

#### [ ] Story 68.1 - Local Database and Structured Evidence

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need local database and structured evidence so that AgentMage delivers the following bounded outcome: Support AgentMage-owned SQLite and approved fixture data through parameterized, bounded, auditable adapters.

##### Tasks and Sub-tasks

- [ ] **Task 68.1.1 - Implement the bounded story**
  - [ ] **Sub-task 68.1.1.1** (legacy `S-055-I01`): Implement parameterized SQLite helpers with transactions, migrations, limits, and typed results.
  - [ ] **Sub-task 68.1.1.2** (legacy `S-055-I02`): Separate schema inspection, row-data access, and mutation permissions.
  - [ ] **Sub-task 68.1.1.3** (legacy `S-055-I03`): Implement read-only query policy, approved-source registry, statement classification, result limits, cancellation, redaction, and receipts.
  - [ ] **Sub-task 68.1.1.4** (legacy `S-055-I04`): Restrict database connections to AgentMage-owned stores and synthetic fixtures for this release.
  - [ ] **Sub-task 68.1.1.5** (legacy `S-055-I05`): Define a database-adapter interface that keeps SQLite, future PostgreSQL, and fixture files distinct.
  - [ ] **Sub-task 68.1.1.6** (legacy `S-055-I06`): Implement report-scope and authorization checks.
  - [ ] **Sub-task 68.1.1.7** (legacy `S-055-I07`): Integrate structured rows with evidence normalization, source identity, reason codes, uncertainty, and reviewer decisions.
  - [ ] **Sub-task 68.1.1.8** (legacy `S-055-I08`): Keep live external database access deferred.

- [ ] **Task 68.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 68.1.2.1:** Local database adapter and query policy.
  - [ ] **Sub-task 68.1.2.2:** Structured-evidence projection and receipt schemas.
  - [ ] **Sub-task 68.1.2.3:** Injection, limit, timeout, redaction, and permission fixtures.
  - [ ] **Sub-task 68.1.2.4:** Live-access exclusion test.

- [ ] **Task 68.1.3 - Verify and close the story**
  - [ ] **Sub-task 68.1.3.1:** `S-055-UT01` validates parameterized query templates, schemas, tables/columns, types, row/byte/time limits, and expected read effects; assert raw SQL, multiple statements, writes, pragmas, attachments, and extensions are rejected.
  - [ ] **Sub-task 68.1.3.2:** `S-055-ST01` runs injection, malicious identifiers, recursive/expensive queries, lock contention, corrupt database, hidden secrets, external functions, and file/network access attempts; assert bounded read-only failure.
  - [ ] **Sub-task 68.1.3.3:** `S-055-UT02` projects rows into evidence with source database hash/schema/query-template/parameters/row identity/freshness; assert deterministic ordering, redaction, truncation, and recomputation.
  - [ ] **Sub-task 68.1.3.4:** `S-055-IT01` points adapters at AgentMage-owned fixtures and simulated live/remote/credentialed databases; assert only approved local fixtures open and no live connector is registered.
  - [ ] **Sub-task 68.1.3.5 - Product security evidence:** Map `SR-ACC-002`/`SR-ACC-006`, `SR-DAT-001` through `SR-DAT-003`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`; retain query policy, injection corpus, plan/limit traces, evidence recomputation, canary scans, and live-access exclusion.

##### Story Acceptance Criteria

- [ ] **Story AC 68.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then all access is parameterized, query-plan/resource bounded, read-only at multiple layers, separately granted, and limited to declared local databases or synthetic fixtures.
- [ ] **Story AC 68.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then results preserve type/null/precision and source provenance while secrets, excessive rows, and prohibited fields are minimized or blocked before model/context persistence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 68.AC1:** Unparameterized, multi-statement, unauthorized, write, or live-source queries fail closed.
- [ ] **Sprint AC 68.AC2:** Schema access does not imply row-data access.
- [ ] **Sprint AC 68.AC3:** Query results obey row, column, byte, redaction, and scope limits.
- [ ] **Sprint AC 68.AC4:** Every result identifies source, query classification, receipt, and freshness.
- [ ] **Sprint AC 68.AC5:** No live database credential or external connection path exists.

**Gate decision:** Sprint 68 is PASS only when Story 68.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 69 - v0.6 Administrative and Document Release Gate

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-056`.

**Sprint goal:** Release office and administrative workflows only after content fidelity, privacy, evidence, and user authority are proven.

**Source coverage:** inventory Sections 11A, 11B, 16-22, 26B, 28, and related Section 32 guides.

**Dependencies:** Sprint 68; legacy dependency record: Sprint 54 (legacy S-047) through Sprint 68 (legacy S-055).

#### [ ] Story 69.1 - v0.6 Administrative and Document Release Gate

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need v0.6 administrative and document release gate so that AgentMage delivers the following bounded outcome: Release office and administrative workflows only after content fidelity, privacy, evidence, and user authority are proven.

##### Tasks and Sub-tasks

- [ ] **Task 69.1.1 - Implement the bounded story**
  - [ ] **Sub-task 69.1.1.1** (legacy `S-056-I01`): Run canonical-record, briefing, meeting, task, correspondence, deadline, document-control, and privacy suites.
  - [ ] **Sub-task 69.1.1.2** (legacy `S-056-I02`): Run extraction, generation, round-trip, redaction, recalculation, page, slide, image, link, metadata, and accessibility checks for every promoted format.
  - [ ] **Sub-task 69.1.1.3** (legacy `S-056-I03`): Run malformed, encrypted, hostile, oversized, unsupported, and embedded-execution fixtures.
  - [ ] **Sub-task 69.1.1.4** (legacy `S-056-I04`): Verify every output uses the controlled-write path and every factual claim retains evidence.
  - [ ] **Sub-task 69.1.1.5** (legacy `S-056-I05`): Publish executive-assistant, secretary, document conversion, artifact verification, privacy, recovery, and limitation guides.
  - [ ] **Sub-task 69.1.1.6** (legacy `S-056-I06`): Prove sending, live calendar changes, messaging, external database access, automatic recipient selection, and unattended disposition remain disabled.

- [ ] **Task 69.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 69.1.2.1:** v0.6 cross-format acceptance corpus and results.
  - [ ] **Sub-task 69.1.2.2:** Administrative workflow acceptance bundle.
  - [ ] **Sub-task 69.1.2.3:** Fidelity, privacy, accessibility, and limitation matrix.
  - [ ] **Sub-task 69.1.2.4:** v0.6 release notes and capability matrix.

- [ ] **Task 69.1.3 - Verify and close the story**
  - [ ] **Sub-task 69.1.3.1:** `S-056-IT01` executes every administrative workflow and promoted file type from intake through extraction/draft/review/render/verify/export using fixed cross-format cases; assert linked provenance and no source overwrite.
  - [ ] **Sub-task 69.1.3.2:** `S-056-ST01` combines hostile mixed-format attachments, prompt injection, active content, secrets, malformed structures, excessive resources, hidden metadata, and unauthorized send/file actions; assert containment and exact limitations.
  - [ ] **Sub-task 69.1.3.3:** `S-056-RT01` interrupts conversion/edit/export/index/cleanup and migrates/rolls back v0.5 state; assert canonical sources, staged outputs, indexes, receipts, and retention reconcile.
  - [ ] **Sub-task 69.1.3.4:** `S-056-AT01` independently reviews representative artifacts for semantic accuracy, structural fidelity, visual fidelity, privacy, accessibility, provenance, and safe handling; force every threshold to fail and verify release blocking.
  - [ ] **Sub-task 69.1.3.5 - Product security evidence:** Map applicable `SR-DAT-*`, `SR-SUP-*`, `SR-AI-*`, `SR-OPS-*`, `SR-TST-*`, and `SR-CIV-*`; retain cross-format raw results, independent artifact reviews, canary scans, recovery evidence, limitations matrix, and signed gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 69.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every supported format meets its declared extraction, generation, round-trip, visual, accessibility, hostile-input, resource, privacy, and evidence thresholds on all supported platforms.
- [ ] **Story AC 69.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then `G-V0.6` closes only when administrative drafts remain user-controlled and no connector send, external filing, live database, executable content, or unattended action is introduced.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 69.AC1:** Every promoted format passes structural and rendered verification on declared platforms.
- [ ] **Sprint AC 69.AC2:** Every recommendation, draft, task, decision, and record is source-backed and correctly sensitivity-labeled.
- [ ] **Sprint AC 69.AC3:** No embedded content executes and no external message or state change occurs.
- [ ] **Sprint AC 69.AC4:** Failed fidelity or privacy checks block artifact completion.
- [ ] **Sprint AC 69.AC5:** `G-V0.6` closes only after all workflow, format, security, recovery, and documentation gates pass.

**Gate decision:** Sprint 69 is PASS only when Story 69.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 7 - v0.7 - Read-Only GitHub and Connectors

### [ ] Sprint 70 - Visible Network Capability and Connector Cache

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-057`.

**Sprint goal:** Introduce temporary, user-initiated network reads without weakening the strict-local baseline.

**Source coverage:** `CR-P2-CON`; inventory Sections 5B, 13B connector foundations, 23, and 26.

**Dependencies:** Sprint 69; legacy dependency record: `G-V0.6`, Sprint 10 (legacy S-010), Sprint 21 (legacy S-019), Sprint 33 (legacy S-027).

#### [ ] Story 70.1 - Visible Network Capability and Connector Cache

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need visible network capability and connector cache so that AgentMage delivers the following bounded outcome: Introduce temporary, user-initiated network reads without weakening the strict-local baseline.

##### Tasks and Sub-tasks

- [ ] **Task 70.1.1 - Implement the bounded story**
  - [ ] **Sub-task 70.1.1.1** (legacy `S-057-I01`): Define a separately selectable connected profile while keeping strict-local as the default and independently testable.
  - [ ] **Sub-task 70.1.1.2** (legacy `S-057-I02`): Require an exact destination, purpose, operation, scope, expected data, byte bound, expiry, and cancellation preview before network activation.
  - [ ] **Sub-task 70.1.1.3** (legacy `S-057-I03`): Make activation visible, temporary, destination-scoped, receipted, and automatically returned to offline baseline after the operation.
  - [ ] **Sub-task 70.1.1.4** (legacy `S-057-I04`): Build a sensitivity-labeled local connector cache with source host, immutable object identity, freshness, retention, deletion, content hash, and import receipt.
  - [ ] **Sub-task 70.1.1.5** (legacy `S-057-I05`): Treat all connector content as untrusted data with no policy, grant, tool, instruction, or completion authority.
  - [ ] **Sub-task 70.1.1.6** (legacy `S-057-I06`): Keep credentials in Keychain or Secret Service and provide only bounded derived tokens to connector processes when required.
  - [ ] **Sub-task 70.1.1.7** (legacy `S-057-I07`): Implement rate limits, pagination, retry-after behavior, cancellation, uncertain-result handling, and network-attempt receipts.
  - [ ] **Sub-task 70.1.1.8** (legacy `S-057-I08`): Prohibit startup polling, silent refresh, hidden network, remote semantic indexing, source upload, and cloud model fallback.

- [ ] **Task 70.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 70.1.2.1:** Connected-profile capability and threat model.
  - [ ] **Sub-task 70.1.2.2:** Network grant and connector-cache schemas.
  - [ ] **Sub-task 70.1.2.3:** Credential delegation and redaction report.
  - [ ] **Sub-task 70.1.2.4:** Temporary-network and offline-return test bundle.

- [ ] **Task 70.1.3 - Verify and close the story**
  - [ ] **Sub-task 70.1.3.1:** `S-057-UT01` validates network grants for actor/session/task/connector/host/method/path/query/byte/time/credential/cache/expiry scope; mutate each field and assert no request.
  - [ ] **Sub-task 70.1.3.2:** `S-057-ST01` attempts proxy/DNS/redirect/host confusion, SSRF, local/LAN/container access, credential reuse, hidden telemetry, unapproved method, oversized response, and cross-connector cache access; assert denial.
  - [ ] **Sub-task 70.1.3.3:** `S-057-RT01` cancels or loses connectivity before/during/after request and cache commit; assert bounded sockets, uncertain-result semantics, no blind retry, and expiry/cleanup.
  - [ ] **Sub-task 70.1.3.4:** `S-057-IT01` enables a user-visible connected profile for one read, records disclosure/traffic/cache evidence, disables it, then reruns strict-local proof; assert no residual network client or credential path.
  - [ ] **Sub-task 70.1.3.5 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-002`/`SR-ACC-007`, `SR-DAT-001` through `SR-DAT-004`, `SR-NET-003` through `SR-NET-007`, `SR-OPS-001` through `SR-OPS-003`; retain threat model, grant mutation results, packet capture, cache canary scan, cancellation traces, and offline-return proof.

##### Story Acceptance Criteria

- [ ] **Story AC 70.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then network is absent in strict-local profiles and becomes available only for an explicit user-initiated bounded connector read with visible destination, scope, credential class, and retention.
- [ ] **Story AC 70.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then connector caches are sensitivity-labeled, encrypted, freshness-labeled, account/workspace isolated, retention-bounded, and removable without changing canonical local data.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 70.AC1:** No network request occurs without an exact current grant and visible status.
- [ ] **Sprint AC 70.AC2:** Ending, cancelling, expiring, or failing a sync restores the offline baseline.
- [ ] **Sprint AC 70.AC3:** Connector credentials never enter model context, logs, memory, repositories, or plaintext configuration.
- [ ] **Sprint AC 70.AC4:** Imported content cannot modify policy or authority.
- [ ] **Sprint AC 70.AC5:** Strict-local acceptance remains unchanged when connected capabilities are disabled.

**Gate decision:** Sprint 70 is PASS only when Story 70.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 71 - GitHub Authentication and Read-Only Provider Core

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-058`.

**Sprint goal:** Authenticate to approved GitHub hosts and normalize bounded read-only access without exposing credentials.

**Source coverage:** inventory Section 13B authentication, provider, cache, pagination, and audit requirements.

**Dependencies:** Sprint 70; legacy dependency record: Sprint 70 (legacy S-057).

#### [ ] Story 71.1 - GitHub Authentication and Read-Only Provider Core

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need github authentication and read-only provider core so that AgentMage delivers the following bounded outcome: Authenticate to approved GitHub hosts and normalize bounded read-only access without exposing credentials.

##### Tasks and Sub-tasks

- [ ] **Task 71.1.1 - Implement the bounded story**
  - [ ] **Sub-task 71.1.1.1** (legacy `S-058-I01`): Implement GitHub.com and user-approved Enterprise host adapters without credential crossover.
  - [ ] **Sub-task 71.1.1.2** (legacy `S-058-I02`): Support approved command-line, REST, or GraphQL transports behind one normalized kernel tool contract.
  - [ ] **Sub-task 71.1.1.3** (legacy `S-058-I03`): Integrate approved SSH agent, credential helper, fine-grained token, or GitHub App authentication through the secret-store adapter.
  - [ ] **Sub-task 71.1.1.4** (legacy `S-058-I04`): Report active host, account or app, installation, repositories, scopes, expiry, single-sign-on state, and missing permissions without secrets.
  - [ ] **Sub-task 71.1.1.5** (legacy `S-058-I05`): Implement pagination, conditional requests, cache validation, rate-limit state, retry-after, cancellation, and freshness.
  - [ ] **Sub-task 71.1.1.6** (legacy `S-058-I06`): Create complete read receipts with host, repository, actor, object, immutable identity, request type, result, and external-state-change flag.
  - [ ] **Sub-task 71.1.1.7** (legacy `S-058-I07`): Refuse credentials, hosts, scopes, or transports not included in the active network and connector grant.

- [ ] **Task 71.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 71.1.2.1:** GitHub provider and authentication adapters.
  - [ ] **Sub-task 71.1.2.2:** Normalized read-only API schemas.
  - [ ] **Sub-task 71.1.2.3:** Authentication diagnostic and permission report.
  - [ ] **Sub-task 71.1.2.4:** Rate-limit, expiry, revocation, and host-isolation fixtures.

- [ ] **Task 71.1.3 - Verify and close the story**
  - [ ] **Sub-task 71.1.3.1:** `S-058-UT01` normalizes successful, partial, paginated, empty, malformed, rate-limited, expired, revoked, unauthorized, forbidden, and unavailable API responses; assert stable typed states and freshness.
  - [ ] **Sub-task 71.1.3.2:** `S-058-UT02` derives least-privilege credentials for approved hosts/repositories/read scopes and tests expiry/revocation/account change; assert no long-lived secret enters tools, logs, config, model context, or cache.
  - [ ] **Sub-task 71.1.3.3:** `S-058-ST01` attacks OAuth/device flow or approved auth path with redirect/host/account confusion, token substitution, cross-host replay, scope inflation, credential helper/environment access, and malicious error bodies; assert fail closed.
  - [ ] **Sub-task 71.1.3.4:** `S-058-IT01` authenticates, reads a synthetic/approved fixture repository, reports effective permissions, revokes access, and verifies all further reads/cache refreshes stop without affecting local-only workflows.
  - [ ] **Sub-task 71.1.3.5 - Product security evidence:** Map `SR-ACC-002`/`SR-ACC-006`, `SR-DAT-002`/`SR-DAT-007`, `SR-NET-005`/`SR-NET-006`, `SR-OPS-001` through `SR-OPS-003`, `SR-TST-004`; retain auth-flow traces, scope reports, token canary scans, host-isolation results, revocation evidence, and provider schema inventory.

##### Story Acceptance Criteria

- [ ] **Story AC 71.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then authentication identity, host, account, scopes, repositories, expiry, and storage are visible and match observed provider permissions; excessive scope blocks enablement.
- [ ] **Story AC 71.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then the provider registers only normalized read operations; mutation methods/endpoints and reusable raw credentials are structurally absent from tool/model boundaries.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 71.AC1:** Authentication succeeds only for the approved host and repository scope.
- [ ] **Sprint AC 71.AC2:** Tokens and private credential values never reach the model or persisted output.
- [ ] **Sprint AC 71.AC3:** Expired, revoked, wrong-host, missing-single-sign-on, and insufficient-scope states fail readably.
- [ ] **Sprint AC 71.AC4:** Every request is bounded, cancellable, fresh-labeled, and receipted.
- [ ] **Sprint AC 71.AC5:** No write-capable GitHub request is registered in this release.

**Gate decision:** Sprint 71 is PASS only when Story 71.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 72 - GitHub Repository and Source Evidence

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-059`.

**Sprint goal:** Read hosted repository, branch, commit, release, rules, workflow, and security metadata as cited evidence.

**Source coverage:** inventory Section 13B organization, repository, content, commit, release, workflow, rules, and security reads.

**Dependencies:** Sprint 71; legacy dependency record: Sprint 71 (legacy S-058).

#### [ ] Story 72.1 - GitHub Repository and Source Evidence

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need github repository and source evidence so that AgentMage delivers the following bounded outcome: Read hosted repository, branch, commit, release, rules, workflow, and security metadata as cited evidence.

##### Tasks and Sub-tasks

- [ ] **Task 72.1.1 - Implement the bounded story**
  - [ ] **Sub-task 72.1.1.1** (legacy `S-059-I01`): Implement organization, user, team, repository, visibility, archive, fork, template, language, topic, license, default-branch, and update discovery.
  - [ ] **Sub-task 72.1.1.2** (legacy `S-059-I02`): Implement bounded search across repositories, code, paths, commits, branches, tags, releases, discussions, and users where permitted.
  - [ ] **Sub-task 72.1.1.3** (legacy `S-059-I03`): Read files, directories, symlinks, submodules, large-file pointers, blobs, and commit trees with immutable links.
  - [ ] **Sub-task 72.1.1.4** (legacy `S-059-I04`): Inspect branches, tags, commits, authors, parents, signatures, comparisons, ancestry, releases, assets, checksums, and provenance.
  - [ ] **Sub-task 72.1.1.5** (legacy `S-059-I05`): Inspect branch protection, rulesets, required review, required checks, signing, history, merge methods, and deletion policy.
  - [ ] **Sub-task 72.1.1.6** (legacy `S-059-I06`): Inspect workflow triggers, permissions, environments, referenced secret names, variables, concurrency, reusable workflows, and artifact retention without secret values.
  - [ ] **Sub-task 72.1.1.7** (legacy `S-059-I07`): Inspect dependency, code-scanning, secret-scanning, advisory, dependency-graph, dependency-review, and software-bill-of-materials data when authorized.
  - [ ] **Sub-task 72.1.1.8** (legacy `S-059-I08`): Preserve immutable identifiers, source links, freshness, classification, and evidence states for every object.

- [ ] **Task 72.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 72.1.2.1:** Hosted repository evidence tools.
  - [ ] **Sub-task 72.1.2.2:** Immutable-link and source-identity schema.
  - [ ] **Sub-task 72.1.2.3:** Repository rules, workflow, release, and security view formats.
  - [ ] **Sub-task 72.1.2.4:** Permission and partial-coverage fixtures.

- [ ] **Task 72.1.3 - Verify and close the story**
  - [ ] **Sub-task 72.1.3.1:** `S-059-UT01` reads repository identity, refs, commits, trees, blobs, releases, rules, workflows, security metadata, pagination, and permission-limited variants; assert normalized source identity and coverage.
  - [ ] **Sub-task 72.1.3.2:** `S-059-UT02` constructs immutable links from host/repository/commit/path/range and tests renamed/deleted/private/stale targets; assert resolvable links or explicit inaccessible/stale state.
  - [ ] **Sub-task 72.1.3.3:** `S-059-ST01` supplies hostile repository names/metadata/content, redirects, submodules, archives, LFS pointers, generated links, and oversized responses; assert no execution, path escape, credential leak, or uncited claim.
  - [ ] **Sub-task 72.1.3.4:** `S-059-IT01` compares hosted metadata/content with an exact local fetch/worktree revision; assert identity matches or differences, permissions, freshness, and missing coverage are explicit.
  - [ ] **Sub-task 72.1.3.5 - Product security evidence:** Map `SR-ACC-008`, `SR-AI-003`/`SR-AI-007`/`SR-AI-010`, `SR-NET-005`/`SR-NET-006`, `SR-OPS-001`, `SR-TST-004`; retain API fixtures, immutable-link resolver results, hostile-content traces, hosted/local identity comparison, and coverage report.

##### Story Acceptance Criteria

- [ ] **Story AC 72.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every hosted source claim resolves to approved host, repository, immutable revision/object, path/range where applicable, retrieval time, account, and permission/coverage state.
- [ ] **Story AC 72.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then missing permissions, pagination, unavailable APIs, stale local copies, and partial security/workflow views cannot be presented as complete repository state.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 72.AC1:** Every hosted fact resolves to host, repository, immutable object identity, and freshness.
- [ ] **Sprint AC 72.AC2:** Secret values are absent even when workflow or security metadata names a secret.
- [ ] **Sprint AC 72.AC3:** Missing permissions produce Unknown/Blocked rather than partial claims presented as complete.
- [ ] **Sprint AC 72.AC4:** Hosted instructions remain untrusted and cannot affect local policy.
- [ ] **Sprint AC 72.AC5:** All operations leave hosted and local repository state unchanged.

**Gate decision:** Sprint 72 is PASS only when Story 72.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 73 - GitHub Issues, Pull Requests, Checks, and Reviews

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-060`.

**Sprint goal:** Provide complete read-only issue, pull-request, review, check, notification, and triage intelligence.

**Source coverage:** inventory Section 13B issue, pull-request, review, check, notification, and triage requirements.

**Dependencies:** Sprint 72; legacy dependency record: Sprint 72 (legacy S-059).

#### [ ] Story 73.1 - GitHub Issues, Pull Requests, Checks, and Reviews

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need github issues, pull requests, checks, and reviews so that AgentMage delivers the following bounded outcome: Provide complete read-only issue, pull-request, review, check, notification, and triage intelligence.

##### Tasks and Sub-tasks

- [ ] **Task 73.1.1 - Implement the bounded story**
  - [ ] **Sub-task 73.1.1.1** (legacy `S-060-I01`): Inspect issues with authors, states, types, labels, assignees, milestones, project fields, comments, reactions, links, dependencies, timeline, and closing reason.
  - [ ] **Sub-task 73.1.1.2** (legacy `S-060-I02`): Build read-only triage for unassigned, stale, blocked, duplicate, dependency-linked, recently changed, and user-selected work.
  - [ ] **Sub-task 73.1.1.3** (legacy `S-060-I03`): Inspect pull requests with base and head, forks, commits, files, patches, comments, threads, reviews, labels, milestones, issues, draft state, mergeability, and update status.
  - [ ] **Sub-task 73.1.1.4** (legacy `S-060-I04`): Inspect checks, workflow runs, jobs, steps, annotations, summaries, logs, artifacts, attempts, cancellations, reruns, and exact tested commit.
  - [ ] **Sub-task 73.1.1.5** (legacy `S-060-I05`): Inspect notifications for reviews, assignments, mentions, failing checks, releases, and watched repositories with local deduplication and read state.
  - [ ] **Sub-task 73.1.1.6** (legacy `S-060-I06`): Ingest user-initiated event updates with signature and replay protection when a bounded webhook or polling adapter is approved.
  - [ ] **Sub-task 73.1.1.7** (legacy `S-060-I07`): Link hosted objects to local repository, branch, worktree, commit, task, decision, and evidence records.
  - [ ] **Sub-task 73.1.1.8** (legacy `S-060-I08`): Build draft issue, pull-request, and review packages locally without submitting them.

- [ ] **Task 73.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 73.1.2.1:** Issue, pull-request, review, check, and notification read tools.
  - [ ] **Sub-task 73.1.2.2:** Hosted-to-local relationship schema.
  - [ ] **Sub-task 73.1.2.3:** Read-only triage views and local draft packages.
  - [ ] **Sub-task 73.1.2.4:** Event signature and replay-protection report.

- [ ] **Task 73.1.3 - Verify and close the story**
  - [ ] **Sub-task 73.1.3.1:** `S-060-UT01` normalizes issue/PR/review/thread/check/workflow/notification states including pagination, edits, deletion, moved lines, partial permissions, and unknown provider fields; assert stable identity and freshness.
  - [ ] **Sub-task 73.1.3.2:** `S-060-UT02` correlates hosted commits/lines/checks/reviews with local repository/worktree evidence; assert exact base/head and explicit unresolved relationships.
  - [ ] **Sub-task 73.1.3.3:** `S-060-ST01` replays, reorders, duplicates, forges, delays, and mutates event fixtures and embeds prompt injections in every hosted text field; assert signature/idempotency/freshness handling and no authority change.
  - [ ] **Sub-task 73.1.3.4:** `S-060-IT01` produces triage views and local draft responses/fixes from read-only data, then inspects provider calls; assert zero comment, label, assignment, review, workflow, branch, or publication mutation.
  - [ ] **Sub-task 73.1.3.5 - Product security evidence:** Map `SR-ACC-007`/`SR-ACC-008`, `SR-AI-005`/`SR-AI-007`/`SR-AI-010`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-004`; retain normalized corpus, correlation results, replay/signature matrix, API method trace, and no-mutation snapshots.

##### Story Acceptance Criteria

- [ ] **Story AC 73.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every triage item exposes hosted identity, current state, source evidence, uncertainty, permission gaps, and local draft status without representing drafts as published.
- [ ] **Story AC 73.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then event duplication/replay and provider inconsistency cannot duplicate local tasks, erase history, or silently mark stale findings current.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 73.AC1:** Triage and draft packages reflect exact hosted state and immutable identifiers.
- [ ] **Sprint AC 73.AC2:** Changed base, moved lines, added commits, stale reviews, and missing permissions are visible.
- [ ] **Sprint AC 73.AC3:** No comment, label, assignment, review, workflow, branch, or notification state is changed.
- [ ] **Sprint AC 73.AC4:** Event replay and invalid signatures are rejected.
- [ ] **Sprint AC 73.AC5:** Connector data remains sensitivity-labeled, retained, deletable, and untrusted.

**Gate decision:** Sprint 73 is PASS only when Story 73.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 74 - Pull-Request Worktrees and Local Review Intelligence

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-061`.

**Sprint goal:** Review hosted changes locally in isolated worktrees without publishing findings or fixes.

**Source coverage:** inventory Sections 13A, 13B local review, 14A, 14B, and 28 review skills.

**Dependencies:** Sprint 73; legacy dependency record: Sprint 42 (legacy S-035), Sprint 43 (legacy S-036), Sprint 47 (legacy S-040), Sprint 73 (legacy S-060).

#### [ ] Story 74.1 - Pull-Request Worktrees and Local Review Intelligence

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need pull-request worktrees and local review intelligence so that AgentMage delivers the following bounded outcome: Review hosted changes locally in isolated worktrees without publishing findings or fixes.

##### Tasks and Sub-tasks

- [ ] **Task 74.1.1 - Implement the bounded story**
  - [ ] **Sub-task 74.1.1.1** (legacy `S-061-I01`): Fetch immutable pull-request refs under an exact network grant and check them out into isolated worktrees.
  - [ ] **Sub-task 74.1.1.2** (legacy `S-061-I02`): Discover status, dependencies, instructions, tests, base movement, merge base, and commits added since prior review.
  - [ ] **Sub-task 74.1.1.3** (legacy `S-061-I03`): Compare pull-request head to current base with branch-aware indexes and exact line links.
  - [ ] **Sub-task 74.1.1.4** (legacy `S-061-I04`): Run repository-aware correctness, security, data, accessibility, performance, dependency, and test reviews locally.
  - [ ] **Sub-task 74.1.1.5** (legacy `S-061-I05`): Suppress formatting-only, duplicate, stale, low-confidence, and deterministic-check-enforced findings.
  - [ ] **Sub-task 74.1.1.6** (legacy `S-061-I06`): Generate suggested fixes only as local shadow patches with tests, risks, rollback, and full diff.
  - [ ] **Sub-task 74.1.1.7** (legacy `S-061-I07`): Build local review packages with inline positions, severity, confidence, evidence, and stale-position handling.
  - [ ] **Sub-task 74.1.1.8** (legacy `S-061-I08`): Keep review submission, branch update, commit, push, merge, and release disabled.

- [ ] **Task 74.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 74.1.2.1:** Pull-request worktree workflow.
  - [ ] **Sub-task 74.1.2.2:** Branch-aware review and stale-finding engine.
  - [ ] **Sub-task 74.1.2.3:** Local review and suggested-fix package formats.
  - [ ] **Sub-task 74.1.2.4:** Base-movement, moved-line, added-commit, and conflict fixtures.

- [ ] **Task 74.1.3 - Verify and close the story**
  - [ ] **Sub-task 74.1.3.1:** `S-061-UT01` creates PR worktrees for exact base/head/fork combinations and validates ownership, refs, changed files, merge base, and permission state; assert no active-checkout or remote mutation.
  - [ ] **Sub-task 74.1.3.2:** `S-061-UT02` maps findings across moved lines, renamed files, rebases, added commits, force-updated fixture refs, resolved threads, and deleted content; assert current/stale/superseded/conflicting state.
  - [ ] **Sub-task 74.1.3.3:** `S-061-ST01` uses malicious patches, binary changes, submodules, symlinks, hooks, generated files, test output, and review instructions; assert sandboxing, untrusted content, bounded analysis, and no publication.
  - [ ] **Sub-task 74.1.3.4:** `S-061-IT01` runs local comprehension/tests/review/suggested-fix planning, then refreshes hosted state before final packet; assert exact currentness, locally validated findings, and draft-only outputs.
  - [ ] **Sub-task 74.1.3.5 - Product security evidence:** Map `SR-ACC-006` through `SR-ACC-008`, `SR-AI-003`/`SR-AI-005`/`SR-AI-010`, `SR-TST-004`/`SR-TST-005`; retain worktree snapshots, finding remap corpus, sandbox traces, hosted refresh comparison, and no-publication proof.

##### Story Acceptance Criteria

- [ ] **Story AC 74.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then each review packet binds to immutable base/head identities and labels every finding current, stale, moved, resolved, uncertain, or locally unverifiable after refresh.
- [ ] **Story AC 74.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then suggested fixes remain isolated controlled-write proposals; no comment, review, commit, push, or PR update occurs in this release.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 74.AC1:** Review evidence is tied to the exact base, head, merge base, file, line, and commit.
- [ ] **Sprint AC 74.AC2:** Stale findings are detected before display or later submission.
- [ ] **Sprint AC 74.AC3:** Suggested fixes remain local and cannot update the hosted branch.
- [ ] **Sprint AC 74.AC4:** Active checkout and hosted state remain unchanged.
- [ ] **Sprint AC 74.AC5:** Review quality fixtures meet declared precision and duplicate-suppression thresholds.

**Gate decision:** Sprint 74 is PASS only when Story 74.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 75 - v0.7 Read-Only Connector Release Gate

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-062`.

**Sprint goal:** Release user-initiated hosted evidence while proving that all network behavior remains bounded and read-only.

**Source coverage:** inventory release sequence, Sections 13A, 13B, 26, 31A, 32, and Section 35B v0.7.

**Dependencies:** Sprint 74; legacy dependency record: Sprint 70 (legacy S-057) through Sprint 74 (legacy S-061).

#### [ ] Story 75.1 - v0.7 Read-Only Connector Release Gate

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need v0.7 read-only connector release gate so that AgentMage delivers the following bounded outcome: Release user-initiated hosted evidence while proving that all network behavior remains bounded and read-only.

##### Tasks and Sub-tasks

- [ ] **Task 75.1.1 - Implement the bounded story**
  - [ ] **Sub-task 75.1.1.1** (legacy `S-062-I01`): Run network scope, credential, host isolation, rate-limit, cancellation, uncertain-result, cache, retention, injection, and offline-return suites.
  - [ ] **Sub-task 75.1.1.2** (legacy `S-062-I02`): Run complete GitHub repository, issue, pull-request, check, security, workflow, notification, worktree, and review fixtures.
  - [ ] **Sub-task 75.1.1.3** (legacy `S-062-I03`): Re-run strict-local v0.1-v0.6 acceptance with connected packs disabled.
  - [ ] **Sub-task 75.1.1.4** (legacy `S-062-I04`): Prove no write request type, submission command, publication hook, or ambient hosted credential is reachable.
  - [ ] **Sub-task 75.1.1.5** (legacy `S-062-I05`): Publish connected-profile, authentication, privacy, cache, read-only GitHub, local review, offline return, recovery, and limitation guides.
  - [ ] **Sub-task 75.1.1.6** (legacy `S-062-I06`): Publish exact future hosted-write exclusions and threat-model prerequisites.

- [ ] **Task 75.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 75.1.2.1:** v0.7 network and connector acceptance bundle.
  - [ ] **Sub-task 75.1.2.2:** Read-only GitHub conformance report.
  - [ ] **Sub-task 75.1.2.3:** Strict-local regression proof.
  - [ ] **Sub-task 75.1.2.4:** v0.7 release notes and capability matrix.

- [ ] **Task 75.1.3 - Verify and close the story**
  - [ ] **Sub-task 75.1.3.1:** `S-062-IT01` runs authentication, repository evidence, issue/PR/check/review triage, and local PR review across approved host/account/permission/network conditions; assert exact read scopes and evidence.
  - [ ] **Sub-task 75.1.3.2:** `S-062-ST01` attempts every provider write method, credential transfer, hidden background sync, alternate host, redirect, cache crossover, event replay, and content-based authority escalation; assert zero external mutation/leakage.
  - [ ] **Sub-task 75.1.3.3:** `S-062-RT01` tests rate limits, revocation, network loss, stale cache, provider inconsistency, cancellation, and restart, then disables/uninstalls connector; assert bounded recovery and credential/cache cleanup.
  - [ ] **Sub-task 75.1.3.4:** `S-062-AT01` reruns full strict-local v0.1-v0.6 workflows and 60-minute egress proof with the connector absent and disabled; assert original offline guarantees and no residual registration.
  - [ ] **Sub-task 75.1.3.5 - Product security evidence:** Map applicable `SR-GOV-010`, `SR-ACC-*`, `SR-DAT-*`, `SR-NET-*`, `SR-OPS-*`, and `SR-TST-*`; retain method/packet traces, mutation attempts, cleanup scan, strict-local regression, conformance report, and signed gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 75.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every network request is user-initiated, exact, least-privilege, read-only, freshness-labeled, receipted, and attributable to one connector/account/task.
- [ ] **Story AC 75.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then `G-V0.7` closes only when hosted writes are impossible, strict-local mode is unchanged, and all caches/credentials/network capabilities can be completely removed.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 75.AC1:** Every external request is user-initiated, scoped, temporary, visible, cancellable, and receipted.
- [ ] **Sprint AC 75.AC2:** Every connector result is untrusted, sensitivity-labeled, cited, freshness-labeled, retained, and deletable.
- [ ] **Sprint AC 75.AC3:** Zero hosted state changes occur across the complete adversarial suite.
- [ ] **Sprint AC 75.AC4:** Disabling connected packs restores the verified strict-local product.
- [ ] **Sprint AC 75.AC5:** `G-V0.7` closes only after network, credential, evidence, read-only, recovery, and documentation gates pass.

**Gate decision:** Sprint 75 is PASS only when Story 75.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 8 - v1+ - Desktop, Extensions, Actions, Scheduling, and Agents

### [ ] Sprint 76 - Desktop Conversation and Workspace Experience

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-063`, part 1 of 2.

**Sprint goal:** Deliver desktop conversation and workspace experience as a bounded part of the legacy goal: Add macOS and Linux desktop applications as thin, offline-capable clients of the same guarded kernel.

**Source coverage:** inventory Section 27B and desktop portions of Sections 5B and 32.

**Dependencies:** Sprint 75; legacy dependency record: `G-V0.7`, Sprint 33 (legacy S-027), Sprint 48 (legacy S-041).

#### [ ] Story 76.1 - Desktop Conversation and Workspace Experience

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need desktop conversation and workspace experience so that AgentMage delivers the following bounded outcome: Add macOS and Linux desktop applications as thin, offline-capable clients of the same guarded kernel.

##### Tasks and Sub-tasks

- [ ] **Task 76.1.1 - Implement the bounded story**
  - [ ] **Sub-task 76.1.1.1** (legacy `S-063-I01`): Select and record the desktop architecture without creating an external web service or second authority boundary.
  - [ ] **Sub-task 76.1.1.2** (legacy `S-063-I02`): Implement conversation navigation by date fields, workspace, project, model, tag, status, pinned, and archived state.
  - [ ] **Sub-task 76.1.1.3** (legacy `S-063-I03`): Implement local search, transcript, citations, tools, approvals, errors, checkpoints, branches, open, continue, branch, rename, archive, export, and approval-gated delete views.
  - [ ] **Sub-task 76.1.1.4** (legacy `S-063-I04`): Implement checkpoint comparison across files, instructions, repository, model, permissions, and next action.
  - [ ] **Sub-task 76.1.1.5** (legacy `S-063-I05`): Implement workspace and vault selection with local-only path checks and no Obsidian dependency.
  - [ ] **Sub-task 76.1.1.6** (legacy `S-063-I06`): Implement Markdown, links, tasks, backlinks, previews, exact diff, command, write, and delete approval screens through kernel contracts.

- [ ] **Task 76.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 76.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 76.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 76.1.3 - Verify and close the story**
  - [ ] **Sub-task 76.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 76.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 76.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 76.1.3.4 - Product security evidence:** Map `SR-PLT-001` through `SR-PLT-010`, `SR-ACC-001`, `SR-TST-007` through `SR-TST-009`, `SR-CIV-006` through `SR-CIV-009`; retain protocol diffs, boundary attacks, crash traces, signed packages, visual/accessibility results, and clean-install evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 76.1.AC1:** Given the approved dependencies and source requirements for `S-063-I01`, `S-063-I02`, `S-063-I03`, `S-063-I04`, `S-063-I05`, and `S-063-I06`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 76.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-063-I01`, `S-063-I02`, `S-063-I03`, `S-063-I04`, `S-063-I05`, and `S-063-I06`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 76.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 76.AC1:** Every numbered implementation sub-task in Story 76.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 76.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 76.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 76.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 76.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 76 is PASS only when Story 76.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 77 - Desktop Status, Recovery, and Packaging

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-063`, part 2 of 2.

**Sprint goal:** Deliver desktop status, recovery, and packaging as a bounded part of the legacy goal: Add macOS and Linux desktop applications as thin, offline-capable clients of the same guarded kernel.

**Source coverage:** inventory Section 27B and desktop portions of Sections 5B and 32.

**Dependencies:** Sprint 76; legacy dependency record: `G-V0.7`, Sprint 33 (legacy S-027), Sprint 48 (legacy S-041).

#### [ ] Story 77.1 - Desktop Status, Recovery, and Packaging

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need desktop status, recovery, and packaging so that AgentMage delivers the following bounded outcome: Add macOS and Linux desktop applications as thin, offline-capable clients of the same guarded kernel.

##### Tasks and Sub-tasks

- [ ] **Task 77.1.1 - Implement the bounded story**
  - [ ] **Sub-task 77.1.1.1** (legacy `S-063-I07`): Display model, runtime, context, memory, plan, tools, offline state, resources, and audit status.
  - [ ] **Sub-task 77.1.1.2** (legacy `S-063-I08`): Implement crash-safe conversation state, single-writer locks, forced-termination recovery, and read-only safe mode.
  - [ ] **Sub-task 77.1.1.3** (legacy `S-063-I09`): Package all fonts, icons, themes, help, and update metadata locally with no telemetry, advertisements, remote assets, or automatic cloud checks.

- [ ] **Task 77.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 77.1.2.1:** Signed macOS and verified Linux desktop packages.
  - [ ] **Sub-task 77.1.2.2:** Desktop-to-kernel protocol conformance report.
  - [ ] **Sub-task 77.1.2.3:** Desktop accessibility, visual, offline, and recovery test bundle.
  - [ ] **Sub-task 77.1.2.4:** Desktop operating and troubleshooting guides.

- [ ] **Task 77.1.3 - Verify and close the story**
  - [ ] **Sub-task 77.1.3.1:** `S-063-UT01` runs published kernel protocol vectors through desktop message/event/cancellation/approval/deep-link/file-picker interfaces; assert schema, identity, ordering, and authority parity.
  - [ ] **Sub-task 77.1.3.2:** `S-063-ST01` attempts direct file/model/key/tool/grant access, hidden network, unsafe URL/file open, drag/drop escape, clipboard leakage, and desktop-only approval bypass; assert thin-client confinement.
  - [ ] **Sub-task 77.1.3.3:** `S-063-RT01` crashes/restarts shell and kernel during tasks, approvals, rendering, updates, and shutdown; assert canonical state recovery, no duplicated effect, and safe stale-view handling.
  - [ ] **Sub-task 77.1.3.4:** `S-063-AT01` installs and exercises signed macOS and verified Linux packages offline with keyboard/screen-reader/zoom/contrast workflows across supported window sizes; assert no overlap, blocked control, or inaccessible state.
  - [ ] **Sub-task 77.1.3.5 - Product security evidence:** Map `SR-PLT-001` through `SR-PLT-010`, `SR-ACC-001`, `SR-TST-007` through `SR-TST-009`, `SR-CIV-006` through `SR-CIV-009`; retain protocol diffs, boundary attacks, crash traces, signed packages, visual/accessibility results, and clean-install evidence.

##### Story Acceptance Criteria

- [ ] **Story AC 77.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then desktop, native Chat, and CLI produce equivalent kernel policy/evidence for the same work packet; no desktop-only authority or canonical state exists.
- [ ] **Story AC 77.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then clean standard-user install, offline operation, accessibility, recovery, uninstall, and residue checks pass independently on each declared desktop platform.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 77.AC1:** Desktop, CLI, and Visual Studio Code produce equivalent grants, receipts, evidence, state, and denials.
- [ ] **Sprint AC 77.AC2:** The desktop never accesses canonical storage, models, tools, connectors, or secrets outside the kernel.
- [ ] **Sprint AC 77.AC3:** Network-disabled operation remains complete and uses only local assets.
- [ ] **Sprint AC 77.AC4:** Crash recovery and safe mode preserve canonical state and user files.
- [ ] **Sprint AC 77.AC5:** Windows and Intel Mac remain excluded until separately promoted and tested.

**Gate decision:** Sprint 77 is PASS only when Story 77.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 78 - Capability Package Trust and Lifecycle

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-064`, part 1 of 2.

**Sprint goal:** Deliver capability package trust and lifecycle as a bounded part of the legacy goal: Add inspectable modular capabilities without allowing packages to bypass kernel authority or install themselves.

**Source coverage:** inventory Sections 26A and 28 declarative and package requirements; `CR-P1-SKL`; Section 35A rejected defaults.

**Dependencies:** Sprint 77; legacy dependency record: Sprint 3 (legacy S-003), Sprint 5 (legacy S-005), Sprint 34 (legacy S-028), Sprint 77 (legacy S-063).

#### [ ] Story 78.1 - Capability Package Trust and Lifecycle

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need capability package trust and lifecycle so that AgentMage delivers the following bounded outcome: Add inspectable modular capabilities without allowing packages to bypass kernel authority or install themselves.

##### Tasks and Sub-tasks

- [ ] **Task 78.1.1 - Implement the bounded story**
  - [ ] **Sub-task 78.1.1.1** (legacy `S-064-I01`): Define a versioned package manifest for identity, source, signer, hashes, license, compatibility, tools, skills, hooks, permissions, network domains, dependencies, entry points, and side effects.
  - [ ] **Sub-task 78.1.1.2** (legacy `S-064-I02`): Implement exact preview and approval for install, enable, disable, update, rollback, and uninstall.
  - [ ] **Sub-task 78.1.1.3** (legacy `S-064-I03`): Verify signatures, checksums, licenses, provenance, dependency locks, and compatibility before installation or update.
  - [ ] **Sub-task 78.1.1.4** (legacy `S-064-I04`): Enforce per-package filesystem, command, network, credential, connector, publication, resource, and retention scopes through the shared policy engine.

- [ ] **Task 78.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 78.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 78.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 78.1.3 - Verify and close the story**
  - [ ] **Sub-task 78.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 78.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 78.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 78.1.3.4 - Product security evidence:** Map `SR-GOV-010`, `SR-PLT-011`, `SR-ACC-001`, `SR-SUP-002` through `SR-SUP-013`, `SR-OPS-008`/`SR-OPS-010`, `SR-TST-011`; retain package corpus, signature/provenance verification, lifecycle traces, capability inventory diffs, safe-mode proof, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 78.1.AC1:** Given the approved dependencies and source requirements for `S-064-I01`, `S-064-I02`, `S-064-I03`, and `S-064-I04`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 78.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-064-I01`, `S-064-I02`, `S-064-I03`, and `S-064-I04`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 78.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 78.AC1:** Every numbered implementation sub-task in Story 78.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 78.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 78.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 78.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 78.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 78 is PASS only when Story 78.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 79 - Hooks, Safe Mode, and Package Recovery

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-064`, part 2 of 2.

**Sprint goal:** Deliver hooks, safe mode, and package recovery as a bounded part of the legacy goal: Add inspectable modular capabilities without allowing packages to bypass kernel authority or install themselves.

**Source coverage:** inventory Sections 26A and 28 declarative and package requirements; `CR-P1-SKL`; Section 35A rejected defaults.

**Dependencies:** Sprint 78; legacy dependency record: Sprint 3 (legacy S-003), Sprint 5 (legacy S-005), Sprint 34 (legacy S-028), Sprint 77 (legacy S-063).

#### [ ] Story 79.1 - Hooks, Safe Mode, and Package Recovery

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need hooks, safe mode, and package recovery so that AgentMage delivers the following bounded outcome: Add inspectable modular capabilities without allowing packages to bypass kernel authority or install themselves.

##### Tasks and Sub-tasks

- [ ] **Task 79.1.1 - Implement the bounded story**
  - [ ] **Sub-task 79.1.1.1** (legacy `S-064-I05`): Implement bounded lifecycle hooks with ordering, timeouts, cancellation, failure isolation, receipts, and safe mode.
  - [ ] **Sub-task 79.1.1.2** (legacy `S-064-I06`): Build a local capability catalog showing which package provides each tool or skill and why it is active.
  - [ ] **Sub-task 79.1.1.3** (legacy `S-064-I07`): Test package compatibility against kernel, tool protocol, configuration, memory, storage, shell, and policy versions.
  - [ ] **Sub-task 79.1.1.4** (legacy `S-064-I08`): Prohibit public auto-discovery, automatic download, automatic enabling, unsigned execution, broader-than-task authority, and alternate endpoint bypass.

- [ ] **Task 79.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 79.1.2.1:** Package manifest and lifecycle implementation.
  - [ ] **Sub-task 79.1.2.2:** Signature, provenance, compatibility, and permission reports.
  - [ ] **Sub-task 79.1.2.3:** Safe-mode and hook-failure recovery paths.
  - [ ] **Sub-task 79.1.2.4:** Malicious and overbroad package corpus.

- [ ] **Task 79.1.3 - Verify and close the story**
  - [ ] **Sub-task 79.1.3.1:** `S-064-UT01` validates package identity, owner, version, compatibility, signature, source/provenance, dependencies, tools, roots, data, network, budgets, hooks, retention, and removal; assert missing/extra/overbroad manifests stay disabled.
  - [ ] **Sub-task 79.1.3.2:** `S-064-ST01` supplies tampered, unsigned, revoked, downgraded, dependency-confused, path-escaping, self-installing, self-updating, hidden-code, permission-escalating, and hook-recursive packages; assert no load or execution.
  - [ ] **Sub-task 79.1.3.3:** `S-064-RT01` interrupts install/enable/disable/upgrade/remove and crashes hooks; assert prior/complete valid package state, bounded cleanup, no kernel corruption, and safe mode startup.
  - [ ] **Sub-task 79.1.3.4:** `S-064-IT01` enables/removes each approved synthetic package and inventories processes/files/tools/network/storage before/after; assert only declared capability delta and complete revocation.
  - [ ] **Sub-task 79.1.3.5 - Product security evidence:** Map `SR-GOV-010`, `SR-PLT-011`, `SR-ACC-001`, `SR-SUP-002` through `SR-SUP-013`, `SR-OPS-008`/`SR-OPS-010`, `SR-TST-011`; retain package corpus, signature/provenance verification, lifecycle traces, capability inventory diffs, safe-mode proof, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 79.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then packages can describe capabilities but cannot register authority outside kernel policy, modify themselves, load undeclared executable code, or survive disable/removal through hidden state.
- [ ] **Story AC 79.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every package has a separately reviewable threat model, supply-chain record, test suite, compatibility range, permission delta, data lifecycle, failure isolation, and rollback/removal proof.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 79.AC1:** Every package starts disabled and requires exact local review.
- [ ] **Sprint AC 79.AC2:** Package code cannot inherit ambient filesystem, shell, network, secret, connector, or publication authority.
- [ ] **Sprint AC 79.AC3:** Hook failure cannot corrupt the action transaction or suppress a receipt.
- [ ] **Sprint AC 79.AC4:** Safe mode starts with all optional executable packages disabled.
- [ ] **Sprint AC 79.AC5:** Removing or rolling back a package restores the prior capability and schema state.

**Gate decision:** Sprint 79 is PASS only when Story 79.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 80 - Read-Only MCP Identity and Manifests

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-065`, part 1 of 2.

**Sprint goal:** Deliver read-only mcp identity and manifests as a bounded part of the legacy goal: Support read-only Model Context Protocol servers only through the same kernel grants, limits, classifications, cancellation, and receipts as core tools.

**Source coverage:** `CR-P2-MCP`; inventory Section 26.

**Dependencies:** Sprint 79; legacy dependency record: Sprint 70 (legacy S-057), Sprint 79 (legacy S-064).

#### [ ] Story 80.1 - Read-Only MCP Identity and Manifests

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need read-only mcp identity and manifests so that AgentMage delivers the following bounded outcome: Support read-only Model Context Protocol servers only through the same kernel grants, limits, classifications, cancellation, and receipts as core tools.

##### Tasks and Sub-tasks

- [ ] **Task 80.1.1 - Implement the bounded story**
  - [ ] **Sub-task 80.1.1.1** (legacy `S-065-I01`): Define MCP connection, discovery, tool, resource, prompt, request, response, error, cancellation, and disconnect contracts.
  - [ ] **Sub-task 80.1.1.2** (legacy `S-065-I02`): Require a declarative manifest containing identity, version, package and process hashes, transport, schemas, side effects, roots, destinations, secrets, limits, cancellation, and requested authority.
  - [ ] **Sub-task 80.1.1.3** (legacy `S-065-I03`): Verify process identity and package hash at launch and connection and invalidate grants when either changes.
  - [ ] **Sub-task 80.1.1.4** (legacy `S-065-I04`): Distinguish in-process, local process, local socket, loopback, and remote transports visibly.

- [ ] **Task 80.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 80.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 80.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 80.1.3 - Verify and close the story**
  - [ ] **Sub-task 80.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 80.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 80.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 80.1.3.4 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-008`, `SR-AI-005`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`; retain conformance vectors, malicious-server corpus, process/network traces, parity report, cleanup scan, and independent gateway review.

##### Story Acceptance Criteria

- [ ] **Story AC 80.1.AC1:** Given the approved dependencies and source requirements for `S-065-I01`, `S-065-I02`, `S-065-I03`, and `S-065-I04`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 80.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-065-I01`, `S-065-I02`, `S-065-I03`, and `S-065-I04`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 80.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 80.AC1:** Every numbered implementation sub-task in Story 80.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 80.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 80.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 80.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 80.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 80 is PASS only when Story 80.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 81 - MCP Request Mediation and Failure Isolation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-065`, part 2 of 2.

**Sprint goal:** Deliver mcp request mediation and failure isolation as a bounded part of the legacy goal: Support read-only Model Context Protocol servers only through the same kernel grants, limits, classifications, cancellation, and receipts as core tools.

**Source coverage:** `CR-P2-MCP`; inventory Section 26.

**Dependencies:** Sprint 80; legacy dependency record: Sprint 70 (legacy S-057), Sprint 79 (legacy S-064).

#### [ ] Story 81.1 - MCP Request Mediation and Failure Isolation

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need mcp request mediation and failure isolation so that AgentMage delivers the following bounded outcome: Support read-only Model Context Protocol servers only through the same kernel grants, limits, classifications, cancellation, and receipts as core tools.

##### Tasks and Sub-tasks

- [ ] **Task 81.1.1 - Implement the bounded story**
  - [ ] **Sub-task 81.1.1.1** (legacy `S-065-I05`): Scope every server independently to workspace roots, network destinations, credentials, operation classes, response classes, budgets, and expiry.
  - [ ] **Sub-task 81.1.1.2** (legacy `S-065-I06`): Validate request and response schemas, item counts, byte limits, content classification, bounded logging, timeout, cancellation, termination, and malformed output.
  - [ ] **Sub-task 81.1.1.3** (legacy `S-065-I07`): Emit receipts for discovery, connection, manifest verification, request, response classification, side effects, cancellation, failure, and disconnect.
  - [ ] **Sub-task 81.1.1.4** (legacy `S-065-I08`): Register only read-only tools and prohibit direct filesystem, shell, secret, network, connector, publication, or approval inheritance.

- [ ] **Task 81.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 81.1.2.1:** Kernel MCP gateway and manifest registry.
  - [ ] **Sub-task 81.1.2.2:** Transport and process-identity verifier.
  - [ ] **Sub-task 81.1.2.3:** Read-only MCP receipt and classification schemas.
  - [ ] **Sub-task 81.1.2.4:** Malicious server, malformed protocol, timeout, and bypass corpus.

- [ ] **Task 81.1.3 - Verify and close the story**
  - [ ] **Sub-task 81.1.3.1:** `S-065-UT01` validates MCP manifests, server identity, transport, tool/resource/prompt schemas, sizes, classifications, limits, versions, and read-only declarations; assert unsupported or write-capable registrations fail.
  - [ ] **Sub-task 81.1.3.2:** `S-065-ST01` runs malicious servers that spoof identity, mutate schemas, request roots/credentials/network, return injections/secrets/oversized streams, spawn children, write files, or bypass cancellation; assert isolation and denial.
  - [ ] **Sub-task 81.1.3.3:** `S-065-RT01` cancels/kills/disconnects/restarts server and gateway across initialize/list/call/result phases; assert process cleanup, one terminal receipt, no replay, and cache invalidation.
  - [ ] **Sub-task 81.1.3.4:** `S-065-IT01` compares equivalent core and MCP read tools; assert same grant/path/classification/budget/receipt/evidence semantics and no direct model-to-server channel.
  - [ ] **Sub-task 81.1.3.5 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-008`, `SR-AI-005`, `SR-TST-002`/`SR-TST-004`/`SR-TST-006`; retain conformance vectors, malicious-server corpus, process/network traces, parity report, cleanup scan, and independent gateway review.

##### Story Acceptance Criteria

- [ ] **Story AC 81.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then mCP is a transport adapter behind the kernel, not an alternate authority path; every call has an exact local grant and all server output remains untrusted.
- [ ] **Story AC 81.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then enabling an MCP server adds only its reviewed read-only manifest; disabling it removes process, tools, roots, network, cache, and retained state completely.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 81.AC1:** Every MCP interaction passes through the kernel tool dispatcher and effective grant.
- [ ] **Sprint AC 81.AC2:** Direct server side effects and undeclared network, path, secret, or process access fail.
- [ ] **Sprint AC 81.AC3:** Changed process or package identity invalidates connection authority.
- [ ] **Sprint AC 81.AC4:** Cancellation terminates pending server work and produces attributable receipts.
- [ ] **Sprint AC 81.AC5:** No writable MCP tool is enabled.

**Gate decision:** Sprint 81 is PASS only when Story 81.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 82 - Public Research and Citations

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-066`, part 1 of 3.

**Sprint goal:** Deliver public research and citations as a bounded part of the legacy goal: Add cited web research and bounded visible application interaction only where structured local or connector tools cannot satisfy the task.

**Source coverage:** inventory Sections 25 and 25A; Best Practices Research skill in Section 28.

**Dependencies:** Sprint 81; legacy dependency record: Sprint 70 (legacy S-057), Sprint 77 (legacy S-063).

#### [ ] Story 82.1 - Public Research and Citations

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need public research and citations so that AgentMage delivers the following bounded outcome: Add cited web research and bounded visible application interaction only where structured local or connector tools cannot satisfy the task.

##### Tasks and Sub-tasks

- [ ] **Task 82.1.1 - Implement the bounded story**
  - [ ] **Sub-task 82.1.1.1** (legacy `S-066-I01`): Implement public search with query, domain, recency, source type, result, and byte controls.
  - [ ] **Sub-task 82.1.1.2** (legacy `S-066-I02`): Prefer primary documentation, original research, and authoritative records and label every inference.
  - [ ] **Sub-task 82.1.1.3** (legacy `S-066-I03`): Capture claim-level title, direct URL, access point, publication metadata, excerpt hash, freshness, and quotation limits.

- [ ] **Task 82.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 82.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 82.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 82.1.3 - Verify and close the story**
  - [ ] **Sub-task 82.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 82.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 82.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 82.1.3.4 - Product security evidence:** Map `SR-ACC-002`/`SR-ACC-007`/`SR-ACC-008`, `SR-DAT-002`/`SR-DAT-003`, `SR-NET-003` through `SR-NET-006`, `SR-AI-004`/`SR-AI-005`; retain source/citation set, browser attack traces, action previews/screenshots, download scans, uncertain-result tests, and privacy review.

##### Story Acceptance Criteria

- [ ] **Story AC 82.1.AC1:** Given the approved dependencies and source requirements for `S-066-I01`, `S-066-I02`, and `S-066-I03`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 82.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-066-I01`, `S-066-I02`, and `S-066-I03`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 82.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 82.AC1:** Every numbered implementation sub-task in Story 82.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 82.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 82.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 82.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 82.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 82 is PASS only when Story 82.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 83 - Sandboxed Browser Inspection

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-066`, part 2 of 3.

**Sprint goal:** Deliver sandboxed browser inspection as a bounded part of the legacy goal: Add cited web research and bounded visible application interaction only where structured local or connector tools cannot satisfy the task.

**Source coverage:** inventory Sections 25 and 25A; Best Practices Research skill in Section 28.

**Dependencies:** Sprint 82; legacy dependency record: Sprint 70 (legacy S-057), Sprint 77 (legacy S-063).

#### [ ] Story 83.1 - Sandboxed Browser Inspection

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need sandboxed browser inspection so that AgentMage delivers the following bounded outcome: Add cited web research and bounded visible application interaction only where structured local or connector tools cannot satisfy the task.

##### Tasks and Sub-tasks

- [ ] **Task 83.1.1 - Implement the bounded story**
  - [ ] **Sub-task 83.1.1.1** (legacy `S-066-I04`): Implement bounded navigate, inspect, find, click, screenshot, and download with visible action trail and quarantined downloads.
  - [ ] **Sub-task 83.1.1.2** (legacy `S-066-I05`): Separate public search from an authenticated browser profile whose cookies and tokens never enter model context or logs.
  - [ ] **Sub-task 83.1.1.3** (legacy `S-066-I06`): Implement domain allowlists, session protection, download limits, redaction, and per-action grants.

- [ ] **Task 83.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 83.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 83.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 83.1.3 - Verify and close the story**
  - [ ] **Sub-task 83.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 83.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 83.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 83.1.3.4 - Product security evidence:** Map `SR-ACC-002`/`SR-ACC-007`/`SR-ACC-008`, `SR-DAT-002`/`SR-DAT-003`, `SR-NET-003` through `SR-NET-006`, `SR-AI-004`/`SR-AI-005`; retain source/citation set, browser attack traces, action previews/screenshots, download scans, uncertain-result tests, and privacy review.

##### Story Acceptance Criteria

- [ ] **Story AC 83.1.AC1:** Given the approved dependencies and source requirements for `S-066-I04`, `S-066-I05`, and `S-066-I06`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 83.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-066-I04`, `S-066-I05`, and `S-066-I06`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 83.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 83.AC1:** Every numbered implementation sub-task in Story 83.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 83.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 83.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 83.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 83.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 83 is PASS only when Story 83.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 84 - Confirmed Computer Use

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-066`, part 3 of 3.

**Sprint goal:** Deliver confirmed computer use as a bounded part of the legacy goal: Add cited web research and bounded visible application interaction only where structured local or connector tools cannot satisfy the task.

**Source coverage:** inventory Sections 25 and 25A; Best Practices Research skill in Section 28.

**Dependencies:** Sprint 83; legacy dependency record: Sprint 70 (legacy S-057), Sprint 77 (legacy S-063).

#### [ ] Story 84.1 - Confirmed Computer Use

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need confirmed computer use so that AgentMage delivers the following bounded outcome: Add cited web research and bounded visible application interaction only where structured local or connector tools cannot satisfy the task.

##### Tasks and Sub-tasks

- [ ] **Task 84.1.1 - Implement the bounded story**
  - [ ] **Sub-task 84.1.1.1** (legacy `S-066-I07`): Implement explicit front-window capture and local application computer use only after structured tools are unavailable.
  - [ ] **Sub-task 84.1.1.2** (legacy `S-066-I08`): Require confirmation before irreversible click, submit, upload, send, or external state change.
  - [ ] **Sub-task 84.1.1.3** (legacy `S-066-I09`): Capture before-and-after screenshots and exact page or application state for computer-use evidence.

- [ ] **Task 84.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 84.1.2.1:** Public research and citation tools.
  - [ ] **Sub-task 84.1.2.2:** Sandboxed browser and authenticated-profile boundary.
  - [ ] **Sub-task 84.1.2.3:** Computer-use action and screenshot receipts.
  - [ ] **Sub-task 84.1.2.4:** Download quarantine and web-injection corpus.

- [ ] **Task 84.1.3 - Verify and close the story**
  - [ ] **Sub-task 84.1.3.1:** `S-066-UT01` ranks current-source candidates, records publication/event/retrieval dates, extracts bounded claims, and resolves citations; assert changing facts require live evidence and source limitations are visible.
  - [ ] **Sub-task 84.1.3.2:** `S-066-ST01` tests malicious pages, redirects, downloads, scripts, service workers, browser extensions, prompt injection, credential phishing, local/LAN targets, hidden uploads, and cross-profile storage; assert sandbox/isolation/quarantine.
  - [ ] **Sub-task 84.1.3.3:** `S-066-UT02` validates computer-use actions against exact foreground app/window/screenshot/coordinates/control/payload/precondition/grant; alter any field and assert no action.
  - [ ] **Sub-task 84.1.3.4:** `S-066-IT01` performs a structured-tool-first research workflow and a separately approved visible fallback action, capturing before/after state; assert confirmation before every irreversible click/submit/upload/send.
  - [ ] **Sub-task 84.1.3.5:** `S-066-RT01` handles navigation change, popup, focus loss, stale screenshot, timeout, cancellation, crash, and uncertain submit; assert stop/reconcile behavior and no blind repeat.
  - [ ] **Sub-task 84.1.3.6 - Product security evidence:** Map `SR-ACC-002`/`SR-ACC-007`/`SR-ACC-008`, `SR-DAT-002`/`SR-DAT-003`, `SR-NET-003` through `SR-NET-006`, `SR-AI-004`/`SR-AI-005`; retain source/citation set, browser attack traces, action previews/screenshots, download scans, uncertain-result tests, and privacy review.

##### Story Acceptance Criteria

- [ ] **Story AC 84.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then public and authenticated browsing use separately approved profiles/storage/credentials, and web content can never change policy, grants, trusted instructions, tools, or completion state.
- [ ] **Story AC 84.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every external state change is exact, current, visible, separately approved, before/after evidenced, and idempotently reconciled; downloads remain quarantined until all checks pass.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 84.AC1:** Every changing fact is live-verified and every web claim has a resolvable citation.
- [ ] **Sprint AC 84.AC2:** Public and authenticated sessions cannot share credentials or storage silently.
- [ ] **Sprint AC 84.AC3:** Web content remains untrusted and cannot change grants, tools, policy, or completion state.
- [ ] **Sprint AC 84.AC4:** Downloads remain quarantined until type, size, hash, malware, path, and user approval checks pass.
- [ ] **Sprint AC 84.AC5:** Silent submit, upload, send, publication, and irreversible clicks remain impossible.

**Gate decision:** Sprint 84 is PASS only when Story 84.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 85 - GitHub Mutation Preview and Authority

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-067`, part 1 of 2.

**Sprint goal:** Deliver github mutation preview and authority as a bounded part of the legacy goal: Add narrowly scoped hosted GitHub mutations with exact previews, idempotency, recovery, and separate publication approvals.

**Source coverage:** inventory Section 13B hosted-write requirements.

**Dependencies:** Sprint 84; legacy dependency record: `G-V0.7`, Sprint 47 (legacy S-040), Sprint 70 (legacy S-057), Sprint 73 (legacy S-060), Sprint 74 (legacy S-061).

#### [ ] Story 85.1 - GitHub Mutation Preview and Authority

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need github mutation preview and authority so that AgentMage delivers the following bounded outcome: Add narrowly scoped hosted GitHub mutations with exact previews, idempotency, recovery, and separate publication approvals.

##### Tasks and Sub-tasks

- [ ] **Task 85.1.1 - Implement the bounded story**
  - [ ] **Sub-task 85.1.1.1** (legacy `S-067-I01`): Implement exact local draft packages for issue, pull request, review, comment, label, assignment, milestone, project field, workflow action, branch update, merge preparation, and release.
  - [ ] **Sub-task 85.1.1.2** (legacy `S-067-I02`): Re-read hosted objects and local branches immediately before approval and again before submission.
  - [ ] **Sub-task 85.1.1.3** (legacy `S-067-I03`): Bind each network write grant to host, repository, actor, object identity, exact payload, expected hosted effect, preview hash, expiry, and idempotency key.
  - [ ] **Sub-task 85.1.1.4** (legacy `S-067-I04`): Implement approval-gated issue, pull-request, review, thread, workflow, commit, push, merge, and release operations as separate capability classes.
  - [ ] **Sub-task 85.1.1.5** (legacy `S-067-I05`): Require signed commits, exact staged diff and message approval, signature verification, and a second approval before remote branch update.

- [ ] **Task 85.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 85.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 85.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 85.1.3 - Verify and close the story**
  - [ ] **Sub-task 85.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 85.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 85.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 85.1.3.4 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-007`, `SR-DAT-002`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-005`/`SR-TST-012`; retain mutation vectors, idempotency/reconciliation traces, API audit, signature checks, remote pre/post snapshots, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 85.1.AC1:** Given the approved dependencies and source requirements for `S-067-I01`, `S-067-I02`, `S-067-I03`, `S-067-I04`, and `S-067-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 85.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-067-I01`, `S-067-I02`, `S-067-I03`, `S-067-I04`, and `S-067-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 85.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 85.AC1:** Every numbered implementation sub-task in Story 85.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 85.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 85.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 85.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 85.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 85 is PASS only when Story 85.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 86 - GitHub Idempotency, Recovery, and Prohibited Operations

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-067`, part 2 of 2.

**Sprint goal:** Deliver github idempotency, recovery, and prohibited operations as a bounded part of the legacy goal: Add narrowly scoped hosted GitHub mutations with exact previews, idempotency, recovery, and separate publication approvals.

**Source coverage:** inventory Section 13B hosted-write requirements.

**Dependencies:** Sprint 85; legacy dependency record: `G-V0.7`, Sprint 47 (legacy S-040), Sprint 70 (legacy S-057), Sprint 73 (legacy S-060), Sprint 74 (legacy S-061).

#### [ ] Story 86.1 - GitHub Idempotency, Recovery, and Prohibited Operations

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need github idempotency, recovery, and prohibited operations so that AgentMage delivers the following bounded outcome: Add narrowly scoped hosted GitHub mutations with exact previews, idempotency, recovery, and separate publication approvals.

##### Tasks and Sub-tasks

- [ ] **Task 86.1.1 - Implement the bounded story**
  - [ ] **Sub-task 86.1.1.1** (legacy `S-067-I06`): Implement duplicate protection and uncertain-result reconciliation for comments, labels, issues, pull requests, workflow dispatches, releases, and retries.
  - [ ] **Sub-task 86.1.1.2** (legacy `S-067-I07`): Implement recovery for credential expiry, permission change, renamed repository, deleted branch, moved line, base movement, and partial publication.
  - [ ] **Sub-task 86.1.1.3** (legacy `S-067-I08`): Emit complete read and write receipts identifying whether external state changed.
  - [ ] **Sub-task 86.1.1.4** (legacy `S-067-I09`): Keep force push, automatic review, automatic fixes, automatic merge, automatic release, repository administration, secret changes, and ruleset changes prohibited.

- [ ] **Task 86.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 86.1.2.1:** Hosted mutation schemas and preview interfaces.
  - [ ] **Sub-task 86.1.2.2:** Per-action grant and idempotency implementation.
  - [ ] **Sub-task 86.1.2.3:** Uncertain-result and partial-publication recovery tools.
  - [ ] **Sub-task 86.1.2.4:** GitHub write audit and rollback guidance.

- [ ] **Task 86.1.3 - Verify and close the story**
  - [ ] **Sub-task 86.1.3.1:** `S-067-UT01` canonicalizes previews/grants for each issue/PR/review/comment/label/assignment/milestone/project/workflow/branch/merge/release action; mutate every field and assert no request.
  - [ ] **Sub-task 86.1.3.2:** `S-067-UT02` simulates success, provider conflict, validation error, timeout-before/after effect, duplicate response, rate limit, and unknown result with idempotency keys; assert exact reconciliation and no duplicate.
  - [ ] **Sub-task 86.1.3.3:** `S-067-ST01` attempts stale-state writes, cross-repository/account action, hidden recipient/visibility/attachment, force push, admin/ruleset/secret changes, automatic review/fix/merge/release, and grant replay; assert denial.
  - [ ] **Sub-task 86.1.3.4:** `S-067-IT01` signs an exact local commit after approval, verifies it, requests separate push approval, refreshes remote state, updates a fixture branch, and verifies hosted postconditions; assert distinct receipts and scopes.
  - [ ] **Sub-task 86.1.3.5:** `S-067-RT01` revokes credentials, moves base/branch/line, and crashes around submission; assert uncertain state blocks retry until current remote state proves effect or non-effect.
  - [ ] **Sub-task 86.1.3.6 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-007`, `SR-DAT-002`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-005`/`SR-TST-012`; retain mutation vectors, idempotency/reconciliation traces, API audit, signature checks, remote pre/post snapshots, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 86.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every hosted mutation changes exactly one previewed object/effect under current preconditions and one single-use grant; any state/payload/identity change invalidates approval.
- [ ] **Story AC 86.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then commit, push, PR/review, workflow, merge, and release remain distinct capability/approval classes, and prohibited administrative/autonomous operations remain absent.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 86.AC1:** Every mutation changes exactly the previewed hosted object and fields.
- [ ] **Sprint AC 86.AC2:** Changed hosted state invalidates stale approval before submission.
- [ ] **Sprint AC 86.AC3:** Duplicate retries cannot create duplicate external effects.
- [ ] **Sprint AC 86.AC4:** Commit and push require distinct approvals and verified signatures.
- [ ] **Sprint AC 86.AC5:** Prohibited automatic and administrative operations remain absent.

**Gate decision:** Sprint 86 is PASS only when Story 86.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 87 - Connector Governance and Isolation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-068`, part 1 of 2.

**Sprint goal:** Deliver connector governance and isolation as a bounded part of the legacy goal: Add separately gated email, messaging, calendar, document-repository, archive, database, and cloud actions without creating broad connector authority.

**Source coverage:** inventory Section 26 connector interface; deferred external scopes in Sections 5, 11A, 11B, 21, and 25.

**Dependencies:** Sprint 86; legacy dependency record: Sprint 70 (legacy S-057), Sprint 79 (legacy S-064), Sprint 81 (legacy S-065), Sprint 86 (legacy S-067).

#### [ ] Story 87.1 - Connector Governance and Isolation

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need connector governance and isolation so that AgentMage delivers the following bounded outcome: Add separately gated email, messaging, calendar, document-repository, archive, database, and cloud actions without creating broad connector authority.

##### Tasks and Sub-tasks

- [ ] **Task 87.1.1 - Implement the bounded story**
  - [ ] **Sub-task 87.1.1.1** (legacy `S-068-I01`): Create a separate threat model, capability manifest, data classification, credential scope, rate limit, publication rule, recovery path, and acceptance suite for each connector.
  - [ ] **Sub-task 87.1.1.2** (legacy `S-068-I02`): Begin each connector with read-only discovery and draft generation before adding any write operation.
  - [ ] **Sub-task 87.1.1.3** (legacy `S-068-I06`): Keep credentials in the platform secret store and provide only the narrowest short-lived derived credential to the connector process.
  - [ ] **Sub-task 87.1.1.4** (legacy `S-068-I08`): Prevent one connector, account, workspace, or task from reusing another's grants, credentials, cache, or context.

- [ ] **Task 87.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 87.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 87.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 87.1.3 - Verify and close the story**
  - [ ] **Sub-task 87.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 87.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 87.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 87.1.3.4 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-008`, `SR-DAT-*`, `SR-NET-003` through `SR-NET-006`, `SR-OPS-*`, `SR-CIV-001` through `SR-CIV-005`; retain per-connector control maps, mutation/isolation results, remote snapshots, recovery traces, lifecycle scan, and reviewer decisions.

##### Story Acceptance Criteria

- [ ] **Story AC 87.1.AC1:** Given the approved dependencies and source requirements for `S-068-I01`, `S-068-I02`, `S-068-I06`, and `S-068-I08`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 87.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-068-I01`, `S-068-I02`, `S-068-I06`, and `S-068-I08`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 87.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 87.AC1:** Every numbered implementation sub-task in Story 87.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 87.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 87.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 87.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 87.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 87 is PASS only when Story 87.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 88 - Approval-Gated Connector Writes and Recovery

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-068`, part 2 of 2.

**Sprint goal:** Deliver approval-gated connector writes and recovery as a bounded part of the legacy goal: Add separately gated email, messaging, calendar, document-repository, archive, database, and cloud actions without creating broad connector authority.

**Source coverage:** inventory Section 26 connector interface; deferred external scopes in Sections 5, 11A, 11B, 21, and 25.

**Dependencies:** Sprint 87; legacy dependency record: Sprint 70 (legacy S-057), Sprint 79 (legacy S-064), Sprint 81 (legacy S-065), Sprint 86 (legacy S-067).

#### [ ] Story 88.1 - Approval-Gated Connector Writes and Recovery

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need approval-gated connector writes and recovery so that AgentMage delivers the following bounded outcome: Add separately gated email, messaging, calendar, document-repository, archive, database, and cloud actions without creating broad connector authority.

##### Tasks and Sub-tasks

- [ ] **Task 88.1.1 - Implement the bounded story**
  - [ ] **Sub-task 88.1.1.1** (legacy `S-068-I03`): Bind every write to exact recipient or object, payload, attachments, visibility, expected side effect, disclosure preview, expiry, and idempotency key.
  - [ ] **Sub-task 88.1.1.2** (legacy `S-068-I04`): Implement user-reviewed sending, messaging, calendar changes, document updates, archive actions, and database writes as distinct grants.
  - [ ] **Sub-task 88.1.1.3** (legacy `S-068-I05`): Re-read current remote state before mutation and reconcile uncertain results before any retry.
  - [ ] **Sub-task 88.1.1.4** (legacy `S-068-I07`): Record request, response classification, external identity, freshness, changed state, failures, retries, and rollback or compensating action.

- [ ] **Task 88.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 88.1.2.1:** Per-connector threat models and manifests.
  - [ ] **Sub-task 88.1.2.2:** Read, draft, preview, write, receipt, and recovery contracts.
  - [ ] **Sub-task 88.1.2.3:** Cross-account and cross-connector isolation corpus.
  - [ ] **Sub-task 88.1.2.4:** Connector-specific user and recovery guides.

- [ ] **Task 88.1.3 - Verify and close the story**
  - [ ] **Sub-task 88.1.3.1:** `S-068-UT01` validates each connector's identity/account/scope/data/destination/payload/attachment/visibility/side-effect/expiry/idempotency schemas with field-by-field mutation; assert no write.
  - [ ] **Sub-task 88.1.3.2:** `S-068-ST01` tests cross-account/connector/workspace/task credential, cache, grant, context, recipient, and attachment confusion plus content injection; assert complete isolation and no disclosure.
  - [ ] **Sub-task 88.1.3.3:** `S-068-IT01` runs read, draft, exact preview, approve, refresh, mutate, verify, and receipt for each connector's smallest allowed fixture action; assert only declared remote fields change.
  - [ ] **Sub-task 88.1.3.4:** `S-068-RT01` simulates expiry, revocation, rate limit, rename/delete, conflict, partial effect, timeout, duplicate retry, and rollback/compensation; assert reconciliation before any retry.
  - [ ] **Sub-task 88.1.3.5 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-008`, `SR-DAT-*`, `SR-NET-003` through `SR-NET-006`, `SR-OPS-*`, `SR-CIV-001` through `SR-CIV-005`; retain per-connector control maps, mutation/isolation results, remote snapshots, recovery traces, lifecycle scan, and reviewer decisions.

##### Story Acceptance Criteria

- [ ] **Story AC 88.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no connector write registers until its read-only gate, threat model, privacy/records/accessibility review, least-privilege credential design, and dedicated adversarial/recovery suite pass.
- [ ] **Story AC 88.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then disabling a connector revokes/removes its credentials, network scope, tools, cache, schedules, background processes, and retained data according to policy without harming other connectors.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 88.AC1:** Each connector passes its own read-only gate before any write tool is registered.
- [ ] **Sprint AC 88.AC2:** Every external mutation requires an exact current user approval.
- [ ] **Sprint AC 88.AC3:** Recipient, attachment, visibility, object, or payload changes invalidate approval.
- [ ] **Sprint AC 88.AC4:** Uncertain results are reconciled before retry and cannot duplicate effects.
- [ ] **Sprint AC 88.AC5:** Disabling a connector removes its credentials, tools, network scope, cache access, and background behavior.

**Gate decision:** Sprint 88 is PASS only when Story 88.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 89 - Queue, Lease, and Retry Semantics

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-069`, part 1 of 2.

**Sprint goal:** Deliver queue, lease, and retry semantics as a bounded part of the legacy goal: Support resumable jobs, local notifications, and user-created read-only schedules without unattended authority expansion.

**Source coverage:** `CR-P2-JOB`; inventory Sections 24 and 24A read-only requirements.

**Dependencies:** Sprint 88; legacy dependency record: Sprint 11 (legacy S-011), Sprint 12 (legacy S-012), Sprint 70 (legacy S-057).

#### [ ] Story 89.1 - Queue, Lease, and Retry Semantics

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need queue, lease, and retry semantics so that AgentMage delivers the following bounded outcome: Support resumable jobs, local notifications, and user-created read-only schedules without unattended authority expansion.

##### Tasks and Sub-tasks

- [ ] **Task 89.1.1 - Implement the bounded story**
  - [ ] **Sub-task 89.1.1.1** (legacy `S-069-I01`): Implement queue, job, retry, pending, running, succeeded, failed, cancelled, waiting-approval, and dead-letter states.
  - [ ] **Sub-task 89.1.1.2** (legacy `S-069-I02`): Require unique leases with owner, acquisition, renewal, expiration, cancellation, and recovery rules.
  - [ ] **Sub-task 89.1.1.3** (legacy `S-069-I03`): Implement idempotency keys, bounded retry, recorded reasons, clean shutdown, and no-repeat resume.
  - [ ] **Sub-task 89.1.1.4** (legacy `S-069-I04`): Enforce per-job budgets for model, tools, processes, resources, network, retries, and retained output.
  - [ ] **Sub-task 89.1.1.5** (legacy `S-069-I05`): Implement explicit wake, sleep, offline, missed-run, unavailable-workspace, expiry, cancellation, and cleanup rules.

- [ ] **Task 89.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 89.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 89.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 89.1.3 - Verify and close the story**
  - [ ] **Sub-task 89.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 89.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 89.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 89.1.3.4 - Product security evidence:** Map `SR-ACC-001`/`SR-ACC-007`, `SR-DAT-010`, `SR-AI-004`/`SR-AI-009`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-005`/`SR-TST-006`; retain state/property tests, fake-clock matrix, prohibited-action attempts, multi-runner crash results, and post-run receipts.

##### Story Acceptance Criteria

- [ ] **Story AC 89.1.AC1:** Given the approved dependencies and source requirements for `S-069-I01`, `S-069-I02`, `S-069-I03`, `S-069-I04`, and `S-069-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 89.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-069-I01`, `S-069-I02`, `S-069-I03`, `S-069-I04`, and `S-069-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 89.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 89.AC1:** Every numbered implementation sub-task in Story 89.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 89.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 89.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 89.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 89.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 89 is PASS only when Story 89.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 90 - Read-Only Schedules, Notifications, and Receipts

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-069`, part 2 of 2.

**Sprint goal:** Deliver read-only schedules, notifications, and receipts as a bounded part of the legacy goal: Support resumable jobs, local notifications, and user-created read-only schedules without unattended authority expansion.

**Source coverage:** `CR-P2-JOB`; inventory Sections 24 and 24A read-only requirements.

**Dependencies:** Sprint 89; legacy dependency record: Sprint 11 (legacy S-011), Sprint 12 (legacy S-012), Sprint 70 (legacy S-057).

#### [ ] Story 90.1 - Read-Only Schedules, Notifications, and Receipts

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need read-only schedules, notifications, and receipts so that AgentMage delivers the following bounded outcome: Support resumable jobs, local notifications, and user-created read-only schedules without unattended authority expansion.

##### Tasks and Sub-tasks

- [ ] **Task 90.1.1 - Implement the bounded story**
  - [ ] **Sub-task 90.1.1.1** (legacy `S-069-I06`): Implement local notifications for completed, failed, blocked, approval-waiting, and resource-constrained work.
  - [ ] **Sub-task 90.1.1.2** (legacy `S-069-I07`): Implement inspectable schedule records, dry runs, manual test runs, pause, resume, edit, run-now, delete, and complete run history.
  - [ ] **Sub-task 90.1.1.3** (legacy `S-069-I08`): Start with read-only jobs and local notifications only; deny scheduled file writes, generic shell, connector writes, publication, or remote state changes.
  - [ ] **Sub-task 90.1.1.4** (legacy `S-069-I09`): Emit post-run receipts covering lease, idempotency, grant, operations, network, resources, outputs, failures, retries, and cleanup.

- [ ] **Task 90.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 90.1.2.1:** Queue, lease, schedule, and notification schemas.
  - [ ] **Sub-task 90.1.2.2:** Read-only scheduler and local job runner.
  - [ ] **Sub-task 90.1.2.3:** Post-run receipt and history views.
  - [ ] **Sub-task 90.1.2.4:** Duplicate, offline, expiry, cancellation, and recovery corpus.

- [ ] **Task 90.1.3 - Verify and close the story**
  - [ ] **Sub-task 90.1.3.1:** `S-069-UT01` covers every queue/job/lease/schedule state and illegal transition, duplicate idempotency key, lease race, renewal boundary, expiry, retry limit, and dead-letter path; assert one owner/effect.
  - [ ] **Sub-task 90.1.3.2:** `S-069-UT02` uses fake clock for due/missed/sleep/wake/offline/time-change/pause/resume/edit/delete cases; assert documented run selection and no catch-up storm.
  - [ ] **Sub-task 90.1.3.3:** `S-069-ST01` attempts scheduled write, shell, publication, connector mutation, credential escalation, interactive approval, self-edit, budget expansion, and cross-workspace access; assert denial.
  - [ ] **Sub-task 90.1.3.4:** `S-069-RT01` crashes/restarts multiple runners during every state and cancels/expires a parent with descendants; assert no repeated completed operation, released leases/resources, and complete history.
  - [ ] **Sub-task 90.1.3.5 - Product security evidence:** Map `SR-ACC-001`/`SR-ACC-007`, `SR-DAT-010`, `SR-AI-004`/`SR-AI-009`, `SR-OPS-001`/`SR-OPS-002`, `SR-TST-005`/`SR-TST-006`; retain state/property tests, fake-clock matrix, prohibited-action attempts, multi-runner crash results, and post-run receipts.

##### Story Acceptance Criteria

- [ ] **Story AC 90.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then initial scheduled work is read-only and locally notifying only; each job has exact scope, budgets, expiry, lease, idempotency, stop conditions, retention, and inspectable history.
- [ ] **Story AC 90.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then pause, revoke, expire, cancel, offline, unavailable workspace, and emergency cleanup operate independently of the model and terminate every descendant.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 90.AC1:** A lease prevents duplicate concurrent execution.
- [ ] **Sprint AC 90.AC2:** Restart, retry, and missed-run recovery never repeat a completed operation.
- [ ] **Sprint AC 90.AC3:** Expiry and cancellation terminate all descendant work and release resources.
- [ ] **Sprint AC 90.AC4:** Unattended jobs cannot request interactive approval or broaden authority.
- [ ] **Sprint AC 90.AC5:** Every scheduled write, shell, publication, and remote mutation attempt is denied.

**Gate decision:** Sprint 90 is PASS only when Story 90.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 91 - Separately Threat-Modeled Scheduled Actions

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-070`.

**Sprint goal:** Permit selected scheduled state changes only after unattended authority has dedicated security, approval, recovery, and cancellation proof.

**Source coverage:** inventory Sections 24A, 13A scheduled worktrees, 26 connectors, and Section 35A early-background exclusions.

**Dependencies:** Sprint 90; legacy dependency record: Sprint 36 (legacy S-029), Sprint 42 (legacy S-035), Sprint 86 (legacy S-067), Sprint 88 (legacy S-068), Sprint 90 (legacy S-069).

#### [ ] Story 91.1 - Separately Threat-Modeled Scheduled Actions

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need separately threat-modeled scheduled actions so that AgentMage delivers the following bounded outcome: Permit selected scheduled state changes only after unattended authority has dedicated security, approval, recovery, and cancellation proof.

##### Tasks and Sub-tasks

- [ ] **Task 91.1.1 - Implement the bounded story**
  - [ ] **Sub-task 91.1.1.1** (legacy `S-070-I01`): Define an unattended-authority threat model that distinguishes prior user configuration from live action approval.
  - [ ] **Sub-task 91.1.1.2** (legacy `S-070-I02`): Define which repeatable actions may use predeclared narrow grants and which always require live approval.
  - [ ] **Sub-task 91.1.1.3** (legacy `S-070-I03`): Bind schedules to exact task template, workspace, worktree, model, tools, destinations, payload constraints, budgets, expiry, and stop conditions.
  - [ ] **Sub-task 91.1.1.4** (legacy `S-070-I04`): Use dedicated worktrees and file ownership for repository schedules.
  - [ ] **Sub-task 91.1.1.5** (legacy `S-070-I05`): Require dry-run evidence and an exact activation preview before enabling a state-changing schedule.
  - [ ] **Sub-task 91.1.1.6** (legacy `S-070-I06`): Implement lease, idempotency, precondition recheck, stale-state stop, cancellation propagation, and post-run verification.
  - [ ] **Sub-task 91.1.1.7** (legacy `S-070-I07`): Implement pause, revoke, expire, inspect, and emergency-stop controls independent of the model.
  - [ ] **Sub-task 91.1.1.8** (legacy `S-070-I08`): Prohibit autonomous recipient selection, open-ended messaging bots, arbitrary shell, force operations, self-edited schedules, and self-expanded authority.

- [ ] **Task 91.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 91.1.2.1:** Unattended-authority threat model and action allowlist.
  - [ ] **Sub-task 91.1.2.2:** Predeclared schedule grant schema.
  - [ ] **Sub-task 91.1.2.3:** State-changing schedule dry-run and activation interface.
  - [ ] **Sub-task 91.1.2.4:** Adversarial unattended-action corpus.

- [ ] **Task 91.1.3 - Verify and close the story**
  - [ ] **Sub-task 91.1.3.1:** `S-070-UT01` validates predeclared grants for exact template/workspace/worktree/model/tool/destination/payload constraint/budget/expiry/stop fields; mutate each and assert no execution.
  - [ ] **Sub-task 91.1.3.2:** `S-070-ST01` attempts autonomous recipient/payload selection, arbitrary shell, force operation, self-edited schedule, recursive scheduling, authority aggregation, stale state, and prompt-driven expansion; assert denial.
  - [ ] **Sub-task 91.1.3.3:** `S-070-IT01` performs dry run, exact activation preview, approved fixture action, precondition refresh, postcondition verify, and history; assert only allowlisted repeatable effect.
  - [ ] **Sub-task 91.1.3.4:** `S-070-RT01` revokes, pauses, expires, emergency-stops, crashes, and creates uncertain external effects; assert descendant termination, no blind retry, reconciliation, and independently operable controls.
  - [ ] **Sub-task 91.1.3.5 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-007`, `SR-AI-004`/`SR-AI-005`, `SR-OPS-001`/`SR-OPS-002`/`SR-OPS-006`, `SR-TST-005`/`SR-TST-011`; retain threat model, grant mutations, dry-run/activation evidence, emergency-stop traces, and independent unattended-authority review.

##### Story Acceptance Criteria

- [ ] **Story AC 91.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then each unattended state-changing action has its own approved threat model and allowlist entry; actions requiring live judgment or variable destination/payload cannot be scheduled.
- [ ] **Story AC 91.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no schedule can create/edit/activate/renew/broaden itself, and any changed state or identity blocks execution pending a new user-reviewed activation.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 91.AC1:** Only explicitly allowlisted repeatable effects can use a predeclared schedule grant.
- [ ] **Sprint AC 91.AC2:** Changed state, payload, destination, workspace, worktree, tool, policy, or schedule invalidates execution.
- [ ] **Sprint AC 91.AC3:** Cancellation, expiry, revocation, and emergency stop terminate all descendants.
- [ ] **Sprint AC 91.AC4:** Uncertain results stop and reconcile rather than retrying blindly.
- [ ] **Sprint AC 91.AC5:** No schedule can create, edit, or broaden itself.

**Gate decision:** Sprint 91 is PASS only when Story 91.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 92 - Agent Definitions and Registry

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-071`, part 1 of 2.

**Sprint goal:** Deliver agent definitions and registry as a bounded part of the legacy goal: Let users define, test, version, and inspect bounded specialist agents before any child execution is enabled.

**Source coverage:** inventory Section 2C definition and studio requirements; Local Agent Builder skill in Section 28; `CR-P2-MAG` foundation.

**Dependencies:** Sprint 91; legacy dependency record: Sprint 34 (legacy S-028), Sprint 49 (legacy S-042), Sprint 79 (legacy S-064), Sprint 90 (legacy S-069).

#### [ ] Story 92.1 - Agent Definitions and Registry

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need agent definitions and registry so that AgentMage delivers the following bounded outcome: Let users define, test, version, and inspect bounded specialist agents before any child execution is enabled.

##### Tasks and Sub-tasks

- [ ] **Task 92.1.1 - Implement the bounded story**
  - [ ] **Sub-task 92.1.1.1** (legacy `S-071-I01`): Define versioned agent specifications for identity, purpose, allowed and prohibited tasks, model profile, tools, roots, memory, budgets, approvals, output, evidence, completion, and stop conditions.
  - [ ] **Sub-task 92.1.1.2** (legacy `S-071-I02`): Implement an agent registry with owners, versions, compatibility, status, source hashes, signatures, and package lifecycle.
  - [ ] **Sub-task 92.1.1.3** (legacy `S-071-I03`): Implement a creation wizard that exposes purpose, prohibitions, evidence, budget, permission, and stopping gaps.
  - [ ] **Sub-task 92.1.1.4** (legacy `S-071-I04`): Implement templates for research, planning, briefing, meetings, documents, repository learning, coding, testing, review, and verification.

- [ ] **Task 92.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 92.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 92.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 92.1.3 - Verify and close the story**
  - [ ] **Sub-task 92.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 92.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 92.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 92.1.3.4 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001`/`SR-ACC-007`/`SR-ACC-008`, `SR-SUP-005`/`SR-SUP-006`, `SR-AI-003` through `SR-AI-006`, `SR-TST-004`; retain schema/lint corpus, compatibility reports, hostile definitions, synthetic state snapshots, and enablement decision.

##### Story Acceptance Criteria

- [ ] **Story AC 92.1.AC1:** Given the approved dependencies and source requirements for `S-071-I01`, `S-071-I02`, `S-071-I03`, and `S-071-I04`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 92.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-071-I01`, `S-071-I02`, `S-071-I03`, and `S-071-I04`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 92.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 92.AC1:** Every numbered implementation sub-task in Story 92.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 92.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 92.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 92.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 92.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 92 is PASS only when Story 92.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 93 - Agent Validation, Dry Runs, and Enablement

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-071`, part 2 of 2.

**Sprint goal:** Deliver agent validation, dry runs, and enablement as a bounded part of the legacy goal: Let users define, test, version, and inspect bounded specialist agents before any child execution is enabled.

**Source coverage:** inventory Section 2C definition and studio requirements; Local Agent Builder skill in Section 28; `CR-P2-MAG` foundation.

**Dependencies:** Sprint 92; legacy dependency record: Sprint 34 (legacy S-028), Sprint 49 (legacy S-042), Sprint 79 (legacy S-064), Sprint 90 (legacy S-069).

#### [ ] Story 93.1 - Agent Validation, Dry Runs, and Enablement

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need agent validation, dry runs, and enablement so that AgentMage delivers the following bounded outcome: Let users define, test, version, and inspect bounded specialist agents before any child execution is enabled.

##### Tasks and Sub-tasks

- [ ] **Task 93.1.1 - Implement the bounded story**
  - [ ] **Sub-task 93.1.1.1** (legacy `S-071-I05`): Implement instruction checking for contradiction, vague completion, hidden network, missing evidence, excessive permission, and unsupported tool assumptions.
  - [ ] **Sub-task 93.1.1.2** (legacy `S-071-I06`): Implement synthetic dry runs with fake files, tools, models, connectors, and grants.
  - [ ] **Sub-task 93.1.1.3** (legacy `S-071-I07`): Produce capability reports for available, degraded, untested, denied, and incompatible requirements.
  - [ ] **Sub-task 93.1.1.4** (legacy `S-071-I08`): Keep every new agent disabled until its definition and synthetic suite pass user review.

- [ ] **Task 93.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 93.1.2.1:** Agent definition schema and registry.
  - [ ] **Sub-task 93.1.2.2:** Agent creation wizard and template pack.
  - [ ] **Sub-task 93.1.2.3:** Prompt, authority, and compatibility checker.
  - [ ] **Sub-task 93.1.2.4:** Synthetic evaluation and capability report.

- [ ] **Task 93.1.3 - Verify and close the story**
  - [ ] **Sub-task 93.1.3.1:** `S-071-UT01` validates agent identity/purpose/prohibitions/model/tools/roots/memory/budgets/approvals/output/evidence/completion/stop schemas with omitted, vague, contradictory, recursive, and overbroad values.
  - [ ] **Sub-task 93.1.3.2:** `S-071-UT02` evaluates compatibility and effective requested capabilities against installed packages/platform/policy; assert available/degraded/untested/denied/incompatible states with rationale.
  - [ ] **Sub-task 93.1.3.3:** `S-071-ST01` supplies self-modifying, self-enabling, self-spawning, hidden-network, excessive-permission, source-instruction, credential, and vague-completion definitions; assert disabled status.
  - [ ] **Sub-task 93.1.3.4:** `S-071-IT01` runs every template entirely against synthetic files/models/tools/connectors/grants and inspects real system state; assert zero real data/external effect and complete attributable evaluation.
  - [ ] **Sub-task 93.1.3.5 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001`/`SR-ACC-007`/`SR-ACC-008`, `SR-SUP-005`/`SR-SUP-006`, `SR-AI-003` through `SR-AI-006`, `SR-TST-004`; retain schema/lint corpus, compatibility reports, hostile definitions, synthetic state snapshots, and enablement decision.

##### Story Acceptance Criteria

- [ ] **Story AC 93.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then an agent package remains disabled until source/signature/compatibility, authority analysis, deterministic lint, synthetic evaluation, limitations, and user review all pass.
- [ ] **Story AC 93.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then agent definitions can restrict but never mint tools, roots, credentials, network, models, budgets, approval modes, child spawning, or completion authority.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 93.AC1:** Agent definitions cannot grant themselves tools, roots, credentials, network, models, or budgets.
- [ ] **Sprint AC 93.AC2:** Contradictory, vague, unsupported, overbroad, or untestable definitions remain disabled.
- [ ] **Sprint AC 93.AC3:** Package signatures and source hashes resolve before enablement.
- [ ] **Sprint AC 93.AC4:** Dry runs touch no real user data or external system.
- [ ] **Sprint AC 93.AC5:** Self-created, self-modifying, recursively spawning, and permission-expanding agents remain prohibited.

**Gate decision:** Sprint 93 is PASS only when Story 93.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 94 - Child Authority and Isolation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-072`, part 1 of 2.

**Sprint goal:** Deliver child authority and isolation as a bounded part of the legacy goal: Execute and direct a limited set of specialist agents with explicit child grants, isolated state, attributable evidence, and deterministic conflict handling.

**Source coverage:** inventory Section 2C execution requirements; Local Agent Director skill; `CR-P2-MAG`.

**Dependencies:** Sprint 93; legacy dependency record: Sprint 42 (legacy S-035), Sprint 79 (legacy S-064), Sprint 90 (legacy S-069), Sprint 93 (legacy S-071).

#### [ ] Story 94.1 - Child Authority and Isolation

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need child authority and isolation so that AgentMage delivers the following bounded outcome: Execute and direct a limited set of specialist agents with explicit child grants, isolated state, attributable evidence, and deterministic conflict handling.

##### Tasks and Sub-tasks

- [ ] **Task 94.1.1 - Implement the bounded story**
  - [ ] **Sub-task 94.1.1.1** (legacy `S-072-I01`): Compute effective child authority as the intersection of user, parent, task, and explicit child grants.
  - [ ] **Sub-task 94.1.1.2** (legacy `S-072-I02`): Prevent coordinators from aggregating narrow child grants into broader authority.
  - [ ] **Sub-task 94.1.1.3** (legacy `S-072-I03`): Isolate child memory, conversation, temporary files, receipts, outputs, and writable worktrees.
  - [ ] **Sub-task 94.1.1.4** (legacy `S-072-I04`): Declare per-child file ownership and prevent concurrent writes to the same path.
  - [ ] **Sub-task 94.1.1.5** (legacy `S-072-I05`): Enforce limits for agent count, nesting, turns, tools, processes, resources, model loads, network, and output.

- [ ] **Task 94.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 94.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 94.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 94.1.3 - Verify and close the story**
  - [ ] **Sub-task 94.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 94.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 94.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 94.1.3.4 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-008`, `SR-AI-003` through `SR-AI-005`, `SR-AI-009`/`SR-AI-010`, `SR-TST-005`/`SR-TST-006`/`SR-TST-011`; retain property corpus, isolation canaries, process/worktree traces, conflict packets, cancellation graph, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 94.1.AC1:** Given the approved dependencies and source requirements for `S-072-I01`, `S-072-I02`, `S-072-I03`, `S-072-I04`, and `S-072-I05`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 94.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-072-I01`, `S-072-I02`, `S-072-I03`, `S-072-I04`, and `S-072-I05`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 94.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 94.AC1:** Every numbered implementation sub-task in Story 94.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 94.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 94.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 94.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 94.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 94 is PASS only when Story 94.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 95 - Agent Coordination, Review, and Direction

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-072`, part 2 of 2.

**Sprint goal:** Deliver agent coordination, review, and direction as a bounded part of the legacy goal: Execute and direct a limited set of specialist agents with explicit child grants, isolated state, attributable evidence, and deterministic conflict handling.

**Source coverage:** inventory Section 2C execution requirements; Local Agent Director skill; `CR-P2-MAG`.

**Dependencies:** Sprint 94; legacy dependency record: Sprint 42 (legacy S-035), Sprint 79 (legacy S-064), Sprint 90 (legacy S-069), Sprint 93 (legacy S-071).

#### [ ] Story 95.1 - Agent Coordination, Review, and Direction

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need agent coordination, review, and direction so that AgentMage delivers the following bounded outcome: Execute and direct a limited set of specialist agents with explicit child grants, isolated state, attributable evidence, and deterministic conflict handling.

##### Tasks and Sub-tasks

- [ ] **Task 95.1.1 - Implement the bounded story**
  - [ ] **Sub-task 95.1.1.1** (legacy `S-072-I06`): Implement task assignments, acknowledgements, ordering, dependencies, retries, cancellation, and return contracts.
  - [ ] **Sub-task 95.1.1.2** (legacy `S-072-I07`): Propagate pause, cancellation, expiry, and termination to every descendant process, model call, tool, lease, and change set.
  - [ ] **Sub-task 95.1.1.3** (legacy `S-072-I08`): Treat child results as untrusted proposals and require parent review of sources, receipts, changes, validation, and completion.
  - [ ] **Sub-task 95.1.1.4** (legacy `S-072-I09`): Implement conflict preservation and resolution, parent accept or reject or revise, sequential pipelines, and bounded parallel read-only review.
  - [ ] **Sub-task 95.1.1.5** (legacy `S-072-I10`): Emit attributable per-child receipts and a director view for assignments, models, budgets, actions, approvals, failures, and results.

- [ ] **Task 95.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 95.1.2.1:** Child-grant and assignment protocols.
  - [ ] **Sub-task 95.1.2.2:** Agent isolation, ownership, and conflict engine.
  - [ ] **Sub-task 95.1.2.3:** Director view and attributable receipt format.
  - [ ] **Sub-task 95.1.2.4:** Delegation, cancellation, collision, aggregation, and false-completion corpus.

- [ ] **Task 95.1.3 - Verify and close the story**
  - [ ] **Sub-task 95.1.3.1:** `S-072-UT01` computes child authority for generated user/parent/task/child grant sets; assert exact intersection, deny precedence, no aggregation, and no authority from results/descriptions.
  - [ ] **Sub-task 95.1.3.2:** `S-072-UT02` exercises assignment/acknowledgement/dependency/result/conflict/accept-reject-revise state machines at agent/turn/tool/resource/nesting/output limits; assert deterministic coordination.
  - [ ] **Sub-task 95.1.3.3:** `S-072-ST01` attempts recursive spawn, coordinator grant aggregation, sibling credential/context/receipt access, shared writes, ownership collision, forged completion, hidden child, and result-as-authority; assert denial/isolation.
  - [ ] **Sub-task 95.1.3.4:** `S-072-RT01` pauses/cancels/expires/terminates parents and intermediate children during model/tool/process/network/write/lease operations; assert all descendants stop and cleanup is proven.
  - [ ] **Sub-task 95.1.3.5:** `S-072-IT01` runs bounded parallel read-only reviews and sequential writable worktree pipelines with conflicts; assert attributable evidence, separate ownership, preserved disagreements, and parent validation.
  - [ ] **Sub-task 95.1.3.6 - Product security evidence:** Map `SR-GOV-010`, `SR-ACC-001` through `SR-ACC-008`, `SR-AI-003` through `SR-AI-005`, `SR-AI-009`/`SR-AI-010`, `SR-TST-005`/`SR-TST-006`/`SR-TST-011`; retain property corpus, isolation canaries, process/worktree traces, conflict packets, cancellation graph, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 95.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then no child exceeds the full authority intersection or shares uncontrolled state; writable children have exclusive paths/worktrees and conflicts never auto-resolve destructively.
- [ ] **Story AC 95.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every child action/result is attributable and untrusted until parent review; cancellation, expiry, budgets, and evidence requirements propagate through the complete descendant graph.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 95.AC1:** No child action exceeds the complete authority intersection.
- [ ] **Sprint AC 95.AC2:** Coordinators cannot combine grants or use one child result as another child's authority.
- [ ] **Sprint AC 95.AC3:** Writable children use separate worktrees and cannot collide silently.
- [ ] **Sprint AC 95.AC4:** Cancellation and expiry terminate all descendants and prove cleanup.
- [ ] **Sprint AC 95.AC5:** Recursive spawning, shared uncontrolled writes, unsupervised swarms, and self-expansion remain impossible.

**Gate decision:** Sprint 95 is PASS only when Story 95.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 96 - Signed Updates and Supply-Chain Maintenance

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-073`, part 1 of 3.

**Sprint goal:** Deliver signed updates and supply-chain maintenance as a bounded part of the legacy goal: Make the complete product maintainable, recoverable, portable, and diagnosable without silent network or authority expansion.

**Source coverage:** inventory Sections 5A, 5B, 10B-10E, 23, 29, 30, 31A, and 32.

**Dependencies:** Sprint 95; legacy dependency record: Sprint 77 (legacy S-063) through Sprint 95 (legacy S-072).

#### [ ] Story 96.1 - Signed Updates and Supply-Chain Maintenance

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need signed updates and supply-chain maintenance so that AgentMage delivers the following bounded outcome: Make the complete product maintainable, recoverable, portable, and diagnosable without silent network or authority expansion.

##### Tasks and Sub-tasks

- [ ] **Task 96.1.1 - Implement the bounded story**
  - [ ] **Sub-task 96.1.1.1** (legacy `S-073-I01`): Implement signed versioned update packages with exact change preview, compatibility checks, integrity verification, staged activation, rollback, and no automatic remote check.
  - [ ] **Sub-task 96.1.1.2** (legacy `S-073-I06`): Generate complete dependency inventory, licenses, hashes, software bill of materials, vulnerability review, and removal plan.

- [ ] **Task 96.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 96.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 96.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 96.1.3 - Verify and close the story**
  - [ ] **Sub-task 96.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 96.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 96.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 96.1.3.4 - Product security evidence:** Map `SR-DAT-005` through `SR-DAT-012`, `SR-SUP-002` through `SR-SUP-013`, `SR-OPS-006` through `SR-OPS-010`, `SR-TST-003`/`SR-TST-005`/`SR-TST-007` through `SR-TST-011`, and `RV-19`/`RV-21`/`RV-22`; retain complete maintenance and independent-verification bundles.

##### Story Acceptance Criteria

- [ ] **Story AC 96.1.AC1:** Given the approved dependencies and source requirements for `S-073-I01`, and `S-073-I06`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 96.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-073-I01`, and `S-073-I06`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 96.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 96.AC1:** Every numbered implementation sub-task in Story 96.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 96.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 96.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 96.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 96.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 96 is PASS only when Story 96.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 97 - Backup and Migration

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-073`, part 2 of 3.

**Sprint goal:** Deliver backup and migration as a bounded part of the legacy goal: Make the complete product maintainable, recoverable, portable, and diagnosable without silent network or authority expansion.

**Source coverage:** inventory Sections 5A, 5B, 10B-10E, 23, 29, 30, 31A, and 32.

**Dependencies:** Sprint 96; legacy dependency record: Sprint 77 (legacy S-063) through Sprint 95 (legacy S-072).

#### [ ] Story 97.1 - Backup and Migration

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need backup and migration so that AgentMage delivers the following bounded outcome: Make the complete product maintainable, recoverable, portable, and diagnosable without silent network or authority expansion.

##### Tasks and Sub-tasks

- [ ] **Task 97.1.1 - Implement the bounded story**
  - [ ] **Sub-task 97.1.1.1** (legacy `S-073-I02`): Implement complete backup and restore for configuration, encrypted operational state, approved knowledge, indexes, manifests, packages, conversations, and audit anchors without exporting secrets.
  - [ ] **Sub-task 97.1.1.2** (legacy `S-073-I03`): Implement migration across supported machines, platform adapters, schemas, model manifests, and package versions.

- [ ] **Task 97.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 97.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 97.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 97.1.3 - Verify and close the story**
  - [ ] **Sub-task 97.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 97.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 97.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 97.1.3.4 - Product security evidence:** Map `SR-DAT-005` through `SR-DAT-012`, `SR-SUP-002` through `SR-SUP-013`, `SR-OPS-006` through `SR-OPS-010`, `SR-TST-003`/`SR-TST-005`/`SR-TST-007` through `SR-TST-011`, and `RV-19`/`RV-21`/`RV-22`; retain complete maintenance and independent-verification bundles.

##### Story Acceptance Criteria

- [ ] **Story AC 97.1.AC1:** Given the approved dependencies and source requirements for `S-073-I02`, and `S-073-I03`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 97.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-073-I02`, and `S-073-I03`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 97.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 97.AC1:** Every numbered implementation sub-task in Story 97.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 97.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 97.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 97.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 97.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 97 is PASS only when Story 97.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 98 - Safe Mode, Diagnostics, and Operational Recovery

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-073`, part 3 of 3.

**Sprint goal:** Deliver safe mode, diagnostics, and operational recovery as a bounded part of the legacy goal: Make the complete product maintainable, recoverable, portable, and diagnosable without silent network or authority expansion.

**Source coverage:** inventory Sections 5A, 5B, 10B-10E, 23, 29, 30, 31A, and 32.

**Dependencies:** Sprint 97; legacy dependency record: Sprint 77 (legacy S-063) through Sprint 95 (legacy S-072).

#### [ ] Story 98.1 - Safe Mode, Diagnostics, and Operational Recovery

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need safe mode, diagnostics, and operational recovery so that AgentMage delivers the following bounded outcome: Make the complete product maintainable, recoverable, portable, and diagnosable without silent network or authority expansion.

##### Tasks and Sub-tasks

- [ ] **Task 98.1.1 - Implement the bounded story**
  - [ ] **Sub-task 98.1.1.1** (legacy `S-073-I04`): Implement startup integrity checks, orphan cleanup, database and index repair, safe mode, package disablement, and last-known-good recovery.
  - [ ] **Sub-task 98.1.1.2** (legacy `S-073-I05`): Extend redacted diagnostics across desktop, connectors, MCP, schedules, packages, and child agents.
  - [ ] **Sub-task 98.1.1.3** (legacy `S-073-I07`): Test full disk, corrupt state, missing secret store, revoked credentials, missing model, broken package, lost worktree, interrupted update, and failed rollback.
  - [ ] **Sub-task 98.1.1.4** (legacy `S-073-I08`): Publish backup, migration, update, rollback, safe-mode, disaster-recovery, and operational troubleshooting guides.

- [ ] **Task 98.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 98.1.2.1:** Signed update and rollback system.
  - [ ] **Sub-task 98.1.2.2:** Encrypted backup and migration formats.
  - [ ] **Sub-task 98.1.2.3:** Extended diagnostics and safe-mode implementation.
  - [ ] **Sub-task 98.1.2.4:** Operational failure and recovery bundle.

- [ ] **Task 98.1.3 - Verify and close the story**
  - [ ] **Sub-task 98.1.3.1:** `S-073-UT01` validates update/backup/migration manifests, signatures, versions, compatibility, dependencies, crypto formats, data classes, and rollback points; assert tampered/downgrade/unsupported packages are rejected.
  - [ ] **Sub-task 98.1.3.2:** `S-073-ST01` tests supply-chain substitution, revoked component, secret inclusion, malicious migration, hidden network/update check, package persistence, backup exfiltration, and safe-mode bypass; assert block/containment.
  - [ ] **Sub-task 98.1.3.3:** `S-073-RT01` injects full disk, corrupt state, missing key store/model/package/worktree, credential revocation, and crash before/after every maintenance transition; assert prior or complete new valid state.
  - [ ] **Sub-task 98.1.3.4:** `S-073-IT01` backs up, migrates between supported platforms/versions, restores, rolls back, enters safe mode, repairs, and uninstalls populated synthetic systems; assert identity/evidence/retention and documented remnants.
  - [ ] **Sub-task 98.1.3.5:** `S-073-AT01` builds twice in clean release runners and independently verifies package signatures/hashes/SBOM/provenance/CBOM/Model BOM; assert reproducible or explicitly bounded signed differences.
  - [ ] **Sub-task 98.1.3.6 - Product security evidence:** Map `SR-DAT-005` through `SR-DAT-012`, `SR-SUP-002` through `SR-SUP-013`, `SR-OPS-006` through `SR-OPS-010`, `SR-TST-003`/`SR-TST-005`/`SR-TST-007` through `SR-TST-011`, and `RV-19`/`RV-21`/`RV-22`; retain complete maintenance and independent-verification bundles.

##### Story Acceptance Criteria

- [ ] **Story AC 98.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then maintenance is explicit, signed, previewed, integrity-checked, staged, recoverable, and offline unless a separately approved acquisition action is active; no secret enters package/backup/log/export.
- [ ] **Story AC 98.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then safe mode disables all optional executable/network/schedule/package/MCP/connector/browser/multi-agent capabilities while preserving authorized diagnostics, evidence, and recovery.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 98.AC1:** Update, rollback, backup, restore, and migration preserve canonical identity and evidence.
- [ ] **Sprint AC 98.AC2:** Secrets do not enter packages, backups, exports, logs, or diagnostics.
- [ ] **Sprint AC 98.AC3:** Interrupted maintenance leaves either the prior valid state or the complete new valid state.
- [ ] **Sprint AC 98.AC4:** Safe mode disables optional executable, network, schedule, and multi-agent capabilities.
- [ ] **Sprint AC 98.AC5:** No maintenance feature silently contacts a remote service or broadens authority.

**Gate decision:** Sprint 98 is PASS only when Story 98.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 99 - Cross-Interface Authority and Isolation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-074`, part 1 of 2.

**Sprint goal:** Deliver cross-interface authority and isolation as a bounded part of the legacy goal: Prove that desktop, packages, MCP, browser, hosted writes, connectors, schedules, and agents cannot bypass the original kernel contracts.

**Source coverage:** inventory Sections 2C, 13B, 24-28, 31A, 32, 35A, and 35B v1+.

**Dependencies:** Sprint 98; legacy dependency record: Sprint 77 (legacy S-063) through Sprint 98 (legacy S-073).

#### [ ] Story 99.1 - Cross-Interface Authority and Isolation

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need cross-interface authority and isolation so that AgentMage delivers the following bounded outcome: Prove that desktop, packages, MCP, browser, hosted writes, connectors, schedules, and agents cannot bypass the original kernel contracts.

##### Tasks and Sub-tasks

- [ ] **Task 99.1.1 - Implement the bounded story**
  - [ ] **Sub-task 99.1.1.1** (legacy `S-074-I01`): Run cross-interface authority parity across Visual Studio Code, CLI, desktop, JSON, software-development-kit, and Agent Client Protocol clients.
  - [ ] **Sub-task 99.1.1.2** (legacy `S-074-I02`): Run package, hook, MCP, connector, browser, schedule, and child-agent attempts to bypass grants, receipts, paths, retention, cancellation, sandbox, and offline policy.
  - [ ] **Sub-task 99.1.1.3** (legacy `S-074-I03`): Run cross-account, cross-workspace, cross-project, cross-agent, cross-worktree, cross-connector, and cross-transport isolation suites.

- [ ] **Task 99.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 99.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 99.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 99.1.3 - Verify and close the story**
  - [ ] **Sub-task 99.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 99.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 99.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 99.1.3.4 - Product security evidence:** Map every applicable `SR-*` requirement and execute all applicable `RV-01` through `RV-22`; retain cross-boundary raw results, authority/data-flow parity, canary scans, descendant recovery traces, strict-local/safe-mode proof, and independent gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 99.1.AC1:** Given the approved dependencies and source requirements for `S-074-I01`, `S-074-I02`, and `S-074-I03`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 99.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-074-I01`, `S-074-I02`, and `S-074-I03`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 99.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 99.AC1:** Every numbered implementation sub-task in Story 99.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 99.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 99.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 99.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 99.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 99 is PASS only when Story 99.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 100 - v1+ Privacy, Recovery, and Release Evidence

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-074`, part 2 of 2.

**Sprint goal:** Deliver v1+ privacy, recovery, and release evidence as a bounded part of the legacy goal: Prove that desktop, packages, MCP, browser, hosted writes, connectors, schedules, and agents cannot bypass the original kernel contracts.

**Source coverage:** inventory Sections 2C, 13B, 24-28, 31A, 32, 35A, and 35B v1+.

**Dependencies:** Sprint 99; legacy dependency record: Sprint 77 (legacy S-063) through Sprint 98 (legacy S-073).

#### [ ] Story 100.1 - v1+ Privacy, Recovery, and Release Evidence

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need v1+ privacy, recovery, and release evidence so that AgentMage delivers the following bounded outcome: Prove that desktop, packages, MCP, browser, hosted writes, connectors, schedules, and agents cannot bypass the original kernel contracts.

##### Tasks and Sub-tasks

- [ ] **Task 100.1.1 - Implement the bounded story**
  - [ ] **Sub-task 100.1.1.1** (legacy `S-074-I04`): Run uncertain-result, duplicate-effect, revocation, expiry, cancellation, crash, update, rollback, backup, restore, and safe-mode suites.
  - [ ] **Sub-task 100.1.1.2** (legacy `S-074-I05`): Run privacy and secret canaries through every new content, credential, export, cache, transcript, screenshot, and audit path.
  - [ ] **Sub-task 100.1.1.3** (legacy `S-074-I06`): Verify strict-local operation remains complete when all connected and executable extension packs are disabled.
  - [ ] **Sub-task 100.1.1.4** (legacy `S-074-I07`): Publish final v1+ threat models, operating guides, capability matrices, exclusions, recovery paths, and release evidence.

- [ ] **Task 100.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 100.1.2.1:** v1+ cross-capability acceptance bundle.
  - [ ] **Sub-task 100.1.2.2:** Authority and data-flow parity report.
  - [ ] **Sub-task 100.1.2.3:** Strict-local regression and safe-mode proof.
  - [ ] **Sub-task 100.1.2.4:** v1+ release notes and capability matrix.

- [ ] **Task 100.1.3 - Verify and close the story**
  - [ ] **Sub-task 100.1.3.1:** `S-074-IT01` replays identical authority/evidence scenarios through Chat, CLI, desktop, JSON, SDK, ACP, package, MCP, connector, browser, schedule, coordinator, and child boundaries; assert policy/receipt parity.
  - [ ] **Sub-task 100.1.3.2:** `S-074-ST01` executes cross-account/workspace/project/agent/worktree/connector/transport attacks and attempts bypass of grants, paths, credentials, sandbox, network, retention, cancellation, and audit; assert zero unauthorized effect.
  - [ ] **Sub-task 100.1.3.3:** `S-074-RT01` combines duplicate/uncertain effect, revocation, expiry, cancellation, crash, update, rollback, backup, restore, safe mode, and resource exhaustion across descendant graphs; assert deterministic containment/recovery.
  - [ ] **Sub-task 100.1.3.4:** `S-074-ST02` propagates unique privacy/secret canaries through every input/output/cache/transcript/screenshot/export/audit path; assert zero unauthorized persistence/disclosure and complete cleanup.
  - [ ] **Sub-task 100.1.3.5:** `S-074-AT01` disables/removes every v1+ capability pack and reruns strict-local core plus independent release verification; force each security threshold to fail and assert no `G-V1+` closure.
  - [ ] **Sub-task 100.1.3.6 - Product security evidence:** Map every applicable `SR-*` requirement and execute all applicable `RV-01` through `RV-22`; retain cross-boundary raw results, authority/data-flow parity, canary scans, descendant recovery traces, strict-local/safe-mode proof, and independent gate decision.

##### Story Acceptance Criteria

- [ ] **Story AC 100.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every side effect across every interface has one attributable actor, exact current grant/precondition, isolated execution, verified postcondition, receipt, retention rule, cancellation path, and recovery outcome.
- [ ] **Story AC 100.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then v1+ capabilities are independently removable and cannot weaken the original strict-local read-only core; any cross-capability discrepancy or failed privacy/security threshold blocks release.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 100.AC1:** No interface, package, MCP server, connector, browser, schedule, coordinator, or child can bypass the kernel.
- [ ] **Sprint AC 100.AC2:** Every side effect has an exact grant, current precondition, receipt, and attributable actor.
- [ ] **Sprint AC 100.AC3:** Every persisted or exported value passes classification, minimization, encryption, retention, and redaction.
- [ ] **Sprint AC 100.AC4:** Cancellation and expiry propagate through every descendant operation.
- [ ] **Sprint AC 100.AC5:** `G-V1+` closes only after all promoted authority paths pass their dedicated and cross-capability suites.

**Gate decision:** Sprint 100 is PASS only when Story 100.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

## [ ] Epic 9 - Inherited-Roadmap Closure Checkpoint

### [ ] Sprint 101 - Requirement and Deferred-Scope Closure

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-075`, part 1 of 2.

**Sprint goal:** Deliver requirement and deferred-scope closure as a bounded part of the legacy goal: Demonstrate complete promoted scope, preserve explicit exclusions, and create the final auditable product release decision.

**Source coverage:** entire README, PRD, inventory Sections 1-35C, and this plan.

**Dependencies:** Sprint 100; legacy dependency record: `G-FOUNDATION`, `G-V0.1`, `G-V0.2`, `G-V0.3`, `G-V0.4`, `G-V0.5`, `G-V0.6`, `G-V0.7`, `G-V1+`.

#### [ ] Story 101.1 - Requirement and Deferred-Scope Closure

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need requirement and deferred-scope closure so that AgentMage delivers the following bounded outcome: Demonstrate complete promoted scope, preserve explicit exclusions, and create the final auditable product release decision.

##### Tasks and Sub-tasks

- [ ] **Task 101.1.1 - Implement the bounded story**
  - [ ] **Sub-task 101.1.1.1** (legacy `S-075-I01`): Rebuild the requirement registry from the current canonical documents and compare it with every sprint completion record.
  - [ ] **Sub-task 101.1.1.2** (legacy `S-075-I02`): Prove every promoted `BUILD`, `VERIFY`, `CAPABILITY GATE`, and `ROADMAP` item maps to implementation, tests, documentation, release, owner, and evidence.
  - [ ] **Sub-task 101.1.1.3** (legacy `S-075-I03`): Review every `DEFER` item and either preserve it as an explicit tested exclusion or promote it through a new approved stable backlog and acceptance gate.

- [ ] **Task 101.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 101.1.2.1:** Produce implementation and contract changes for only the numbered sub-tasks in this story.
  - [ ] **Sub-task 101.1.2.2:** Produce requirement-to-code-to-test traceability and a hashed evidence index for this story.

- [ ] **Task 101.1.3 - Verify and close the story**
  - [ ] **Sub-task 101.1.3.1:** Run every issue-local positive, invalid/prohibited, boundary, dependency-failure/cancellation, and exact-side-effect case for the assigned implementation sub-tasks.
  - [ ] **Sub-task 101.1.3.2:** Run integration and adversarial checks proving the partial story cannot broaden authority, data scope, network scope, platform scope, or completion claims.
  - [ ] **Sub-task 101.1.3.3:** Recompute the result summary from raw evidence and block on every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or unreviewed check.
  - [ ] **Sub-task 101.1.3.4 - Product security evidence:** Map the full `SECURITY-REVIEW.md` control and reviewer-protocol sets; retain the complete signed evidence bundle, raw-result recomputation, clean-platform witnesses, deferred/exclusion register, risk/remediation inputs, rollback plan, and user approval.

##### Story Acceptance Criteria

- [ ] **Story AC 101.1.AC1:** Given the approved dependencies and source requirements for `S-075-I01`, `S-075-I02`, and `S-075-I03`, when the story is exercised against its approved fixtures, then every behavior stated by those issue identities is demonstrably satisfied and no undeclared capability is enabled.
- [ ] **Story AC 101.1.AC2:** Given positive, invalid/prohibited, boundary, cancellation, dependency-failure, and side-effect cases for `S-075-I01`, `S-075-I02`, and `S-075-I03`, when the story test set runs, then each assigned sub-task produces its specified value, state, and receipt while every prohibited side effect remains absent.
- [ ] **Story AC 101.1.AC3:** Given the raw test output and environment manifest, when a reviewer recomputes the story result, then failures, skips, retries, suppressions, and limitations remain visible and the summary matches the raw evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 101.AC1:** Every numbered implementation sub-task in Story 101.1 is complete and linked to its legacy requirement or issue identity.
- [ ] **Sprint AC 101.AC2:** All applicable positive, negative, boundary, error/cancellation, side-effect, integration, adversarial, and recovery checks pass with raw evidence.
- [ ] **Sprint AC 101.AC3:** No workspace, authority, privacy, network, platform, or canonical-state behavior outside this story's declared scope changes.
- [ ] **Sprint AC 101.AC4:** Required artifacts are present, hashed, source-traceable, and reproducible from the recorded environment.
- [ ] **Sprint AC 101.AC5:** The gate is recorded as PASS only when no blocking test is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, or awaiting required independent review.

**Gate decision:** Sprint 101 is PASS only when Story 101.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.
### [ ] Sprint 102 - Inherited-Scope Verification Checkpoint

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Legacy roadmap source:** `S-075`, part 2 of 2.

**Sprint goal:** Verify and close the inherited Sprints 0-100 scope while preserving its explicit exclusions; Decision 0008 supersedes this checkpoint as final product authority.

**Source coverage:** entire README, PRD, inventory Sections 1-35C, and this plan.

**Dependencies:** Sprint 101; legacy dependency record: `G-FOUNDATION`, `G-V0.1`, `G-V0.2`, `G-V0.3`, `G-V0.4`, `G-V0.5`, `G-V0.6`, `G-V0.7`, `G-V1+`.

#### [ ] Story 102.1 - Inherited-Scope Verification Checkpoint

**User-facing value:** As an AgentMage user, maintainer, or reviewer, I need the inherited roadmap reconciled before delivery expansion so that no earlier requirement or exclusion is lost and no checkpoint is mistaken for v1.0 GA.

##### Tasks and Sub-tasks

- [ ] **Task 102.1.1 - Implement the bounded story**
  - [ ] **Sub-task 102.1.1.1** (legacy `S-075-I04`): Re-run all clean-platform installations, upgrades, offline workflows, connected workflows, safe mode, backup, restore, migration, and uninstall paths.
  - [ ] **Sub-task 102.1.1.2** (legacy `S-075-I05`): Re-run complete deterministic, model, evidence, privacy, path, sandbox, network, package, connector, browser, schedule, hosted-write, and multi-agent suites.
  - [ ] **Sub-task 102.1.1.3** (legacy `S-075-I06`): Verify every shell and model remains authority-free and every capability remains removable without corrupting canonical state.
  - [ ] **Sub-task 102.1.1.4** (legacy `S-075-I07`): Validate complete documentation, examples, limitations, troubleshooting, threat models, data maps, software bill of materials, licenses, and release manifests.
  - [ ] **Sub-task 102.1.1.5** (legacy `S-075-I08`): Preserve the legacy final-release artifact as an inherited-scope checkpoint containing passed gates, unresolved risks, explicit exclusions, platforms, models, capabilities, and rollback plan; label it non-GA under Decision 0008.

- [ ] **Task 102.1.2 - Produce reviewable artifacts**
  - [ ] **Sub-task 102.1.2.1:** Complete requirement-to-code-to-test-to-document traceability report.
  - [ ] **Sub-task 102.1.2.2:** Final cross-platform acceptance and clean-install bundle.
  - [ ] **Sub-task 102.1.2.3:** Deferred and excluded capability register.
  - [ ] **Sub-task 102.1.2.4:** Signed inherited-scope manifests, software bill of materials, capability matrix, and non-GA checkpoint decision.

- [ ] **Task 102.1.3 - Verify and close the story**
  - [ ] **Sub-task 102.1.3.1:** `S-075-UT01` rebuilds the complete requirement graph and validates unique source/implementation/test/document/owner/release/evidence links; assert zero promoted orphan and exact explicit-exclusion coverage.
  - [ ] **Sub-task 102.1.3.2:** `S-075-IT01` recomputes every gate summary from raw evidence, checks staleness against current source/dependency/config/model/platform manifests, and verifies reviewer identities/findings; assert no omitted failure or stale pass.
  - [ ] **Sub-task 102.1.3.3:** `S-075-ST01` attempts all documented prohibited capabilities and seeded regressions across clean packages; assert absent/denied behavior and no security/privacy/authority threshold waiver.
  - [ ] **Sub-task 102.1.3.4:** `S-075-AT01` performs three independent clean installs per supported platform plus upgrade/offline/connected/safe-mode/backup/restore/migration/uninstall workflows using published reviewer commands only.
  - [ ] **Sub-task 102.1.3.5:** `S-075-AT02` independently verifies signatures, notarization where applicable, hashes, source/binary SBOMs, provenance, CBOM, Model BOM, licenses, component/process/path/socket closure, and residue inventory.
  - [ ] **Sub-task 102.1.3.6 - Product security evidence:** Map the full `SECURITY-REVIEW.md` control and reviewer-protocol sets; retain the complete signed evidence bundle, raw-result recomputation, clean-platform witnesses, deferred/exclusion register, risk/remediation inputs, rollback plan, and user approval.

##### Story Acceptance Criteria

- [ ] **Story AC 102.1.AC1:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then every promoted requirement has current reproducible code/test/document/evidence/owner/release linkage and no unresolved blocking dependency; every deferred item has an approved disposition and tested exclusion unless formally promoted.
- [ ] **Story AC 102.1.AC2:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then final release status remains `BLOCKED` for any failed, skipped, stale, unavailable, flaky, suppressed, unreviewed, or unreconciled blocking control regardless of feature completeness.
- [ ] **Story AC 102.1.AC3:** Given the story dependencies and approved fixtures, when the implementation and verification tasks are completed, then legacy `G-PRODUCT` may close only as an inherited-scope checkpoint after user review; it cannot authorize v1.0 GA.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 102.AC1:** Every promoted requirement has current reproducible evidence and no unresolved dependency.
- [ ] **Sprint AC 102.AC2:** Every explicit exclusion has a test proving the prohibited path is absent or denied.
- [ ] **Sprint AC 102.AC3:** All supported platforms, models, interfaces, capability packs, storage domains, and recovery paths agree across documents and packages.
- [ ] **Sprint AC 102.AC4:** No failed security, privacy, authority, evidence, recovery, or clean-install threshold is waived by feature completeness.
- [ ] **Sprint AC 102.AC5:** Legacy `G-PRODUCT` closes only after the user approves the inherited-scope checkpoint and every blocking gate in that scope is green; final release authority remains `G-GA`.

**Gate decision:** Sprint 102 is PASS only when Story 102.1, every numbered task/sub-task, every story criterion, every sprint criterion, and the Universal Story Definition of Done are complete with current evidence. Otherwise it is BLOCKED.

Decision 0008 supersedes Sprint 102 as the final product gate. Sprint 102 remains the historically stable inherited-roadmap closure checkpoint and retains `G-PRODUCT` as its legacy gate identity. It cannot authorize or describe v1.0 GA.

## [ ] Epic 10 - Provider-Neutral Delivery System and Windows 11

### [ ] Sprint 103 - Delivery Graph, Adapter SDK, and Support Matrix

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Freeze the provider-neutral delivery model and prove that adapters can be added, versioned, degraded, and removed without provider logic entering the kernel.

**Source coverage:** `AM-GA-001`, `AM-DEL-001`, `AM-ADP-001`, `AT-DEL-001`, `AT-ADP-001`; `DELIVERY-SYSTEM.md` Sections 3-6 and 13; `SR-DEL-001`, `SR-DEL-002`, `RV-23`, `RV-27`.

**Dependencies:** Sprint 102 as the inherited-scope checkpoint; Sprints 4, 5, 21, 70, 79, 81, 87, and 88 for kernel, grants, evidence, connected profiles, packages, mediation, and connector controls.

#### [ ] Story 103.1 - Provider-Neutral Delivery Foundation

**User-facing value:** As a user and reviewer, I need one truthful delivery model so that source, work, CI, artifacts, deployments, telemetry, incidents, and releases can be correlated without granting authority or hiding provider differences.

##### Tasks and Sub-tasks

- [ ] **Task 103.1.1 - Implement delivery graph contracts**
  - [ ] **Sub-task 103.1.1.1:** Define versioned types for service, work item, repository, change, commit, review, build, check, artifact, provenance, environment, deployment, telemetry, incident, finding, release, rollback, and evidence-backed edge.
  - [ ] **Sub-task 103.1.1.2:** Bind every node to provider, exact host, tenant or organization, project, immutable identity, display identity, version, freshness, sensitivity, tombstone state, and source receipt.
  - [ ] **Sub-task 103.1.1.3:** Distinguish observed, derived, inferred, conflicting, stale, deleted, inaccessible, and unknown graph relationships; prohibit inferred edges from authorizing operations.
  - [ ] **Sub-task 103.1.1.4:** Implement graph migrations, bounded indexes, cache deletion, source refresh, stale propagation, and deterministic export/import.
- [ ] **Task 103.1.2 - Implement the adapter SDK and matrix**
  - [ ] **Sub-task 103.1.2.1:** Define `describe`, `diagnose`, `discover`, `plan`, `preview`, `execute`, `reconcile`, `rollback_or_compensate`, and `remove` contracts.
  - [ ] **Sub-task 103.1.2.2:** Define signed adapter manifests and the provider/host/version/object/operation/scope/event/limit/degradation/support matrix.
  - [ ] **Sub-task 103.1.2.3:** Implement L0 manifested, L1 observable, L2 writable, L3 executable, L4 deployable, and L5 administrative registration with no level inheritance.
  - [ ] **Sub-task 103.1.2.4:** Implement namespaced provider extensions and reject unknown extensions that lack a schema, policy, and conformance identity.
  - [ ] **Sub-task 103.1.2.5:** Build fake, fault, future-version, eventual-consistency, and hostile provider adapters plus complete removal fixtures.
- [ ] **Task 103.1.3 - Verify and close the story**
  - [ ] **Sub-task 103.1.3.1:** `S-103-UT01` round-trips every delivery object and relationship through minimum, maximum, empty, malformed, extra-field, future-version, rename, transfer, delete, tombstone, and collision fixtures.
  - [ ] **Sub-task 103.1.3.2:** `S-103-UT02` mutates every manifest and support-matrix field and compares registration with conformance level; unsupported operations must remain absent.
  - [ ] **Sub-task 103.1.3.3:** `S-103-ST01` injects model and provider attempts to invent edges, capabilities, versions, support, or completion; assert no authority or support claim changes.
  - [ ] **Sub-task 103.1.3.4:** `S-103-RT01` upgrades, downgrades, corrupts, disables, and removes adapters around graph migrations and active reads; assert deterministic rollback and no orphaned authority.
  - [ ] **Sub-task 103.1.3.5:** Execute `RV-23` and `RV-27`; retain raw graph corpus, adapter conformance matrix, registration diff, removal scan, evidence hashes, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 103.1.AC1:** Given heterogeneous provider fixtures, when the graph is built and refreshed, then every authoritative node and edge resolves to immutable source evidence while inference and conflict remain visibly non-authoritative.
- [ ] **Story AC 103.1.AC2:** Given an adapter manifest and tested provider version, when registration occurs, then only operations at the proven conformance level register and every unsupported operation remains absent.
- [ ] **Story AC 103.1.AC3:** Given adapter removal or an unsupported provider version, when diagnostics and cleanup run, then AgentMage enters the declared blocked or degraded state without residual authority or a misleading support claim.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 103.AC1:** Delivery graph schemas and migrations are deterministic, bounded, versioned, and evidence-preserving.
- [ ] **Sprint AC 103.AC2:** The kernel imports only provider-neutral contracts and contains no provider-specific API branch.
- [ ] **Sprint AC 103.AC3:** Every adapter operation is traceable to a matrix tuple and conformance result.
- [ ] **Sprint AC 103.AC4:** Fake/fault/future providers and complete adapter removal pass.
- [ ] **Sprint AC 103.AC5:** `RV-23` and `RV-27` have current independently reviewed evidence.

**Gate decision:** Sprint 103 is PASS only when Story 103.1, all criteria, `AT-DEL-001`, `AT-ADP-001`, and the Universal Story Definition of Done pass with current evidence. Otherwise it is BLOCKED.

### [ ] Sprint 104 - Connected Identity, Credentials, and Capability Classes

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Prove that no provider, host, tenant, account, project, environment, credential, or capability class can be confused with another.

**Source coverage:** `AM-IDN-001`, `AT-IDN-001`; `DELIVERY-SYSTEM.md` Sections 4 and 7; `SR-DEL-002` through `SR-DEL-004`, `RV-24`.

**Dependencies:** Sprint 103; Sprints 5, 11, 21, 70, and 87.

#### [ ] Story 104.1 - Exact Connected Identity and Secret Isolation

**User-facing value:** As a user, I need AgentMage to show and use the exact account and destination so that a credential or approval can never cross into another provider domain.

##### Tasks and Sub-tasks

- [ ] **Task 104.1.1 - Implement connected identity and credential contracts**
  - [ ] **Sub-task 104.1.1.1:** Define canonical provider, host, port, TLS identity, tenant, account, project, environment, credential reference, single-sign-on state, and capability-class identities.
  - [ ] **Sub-task 104.1.1.2:** Resolve credentials only inside operation-scoped provider workers after kernel grant and worker identity validation.
  - [ ] **Sub-task 104.1.1.3:** Validate redirects, proxies, DNS results, callback targets, clone hosts, provider-supplied URLs, and cross-host API links before sending a request or credential.
  - [ ] **Sub-task 104.1.1.4:** Expose non-secret account/scope/expiry diagnostics and deny missing, ambiguous, excessive, stale, revoked, or cross-domain credentials.
- [ ] **Task 104.1.2 - Implement capability-class separation**
  - [ ] **Sub-task 104.1.2.1:** Encode `observe`, `draft`, `local-write`, `remote-write`, `execute`, `deploy`, `secrets`, and `admin` as non-inheriting grant classes.
  - [ ] **Sub-task 104.1.2.2:** Make each class use distinct tool registration, policy checks, preview fields, receipts, diagnostics, and support-matrix entries.
  - [ ] **Sub-task 104.1.2.3:** Prevent nested provider calls, workflow inputs, issue content, plugins, and model output from escalating one class into another.
  - [ ] **Sub-task 104.1.2.4:** Add secret canaries and cross-domain fixtures for every provider worker, cache, log, receipt, diagnostic, error, and model-context path.
- [ ] **Task 104.1.3 - Verify and close the story**
  - [ ] **Sub-task 104.1.3.1:** `S-104-UT01` mutates every connected identity field and credential state; assert stable denial reason, no request, no secret serialization, and one receipt.
  - [ ] **Sub-task 104.1.3.2:** `S-104-ST01` runs all pairwise capability-class escalation attempts through direct requests, provider content, nested actions, imports, and model tool calls.
  - [ ] **Sub-task 104.1.3.3:** `S-104-ST02` runs at least 2,000 host/tenant/account/project/environment/credential/redirect/proxy/DNS/callback confusion cases.
  - [ ] **Sub-task 104.1.3.4:** `S-104-IT01` authenticates multiple synthetic accounts on identical and different hosts and performs bounded reads concurrently; assert complete credential, cache, and result isolation.
  - [ ] **Sub-task 104.1.3.5:** Execute `RV-24`; retain process/network traces, secret-canary scan, request destinations, denial matrix, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 104.1.AC1:** Given multiple hosts and accounts with identical display names, when a connected operation is previewed and executed, then the exact destination and credential domain remain unambiguous and no credential crosses domains.
- [ ] **Story AC 104.1.AC2:** Given authority for one capability class, when any direct or indirect escalation is attempted, then the stronger operation is absent or denied before provider contact.
- [ ] **Story AC 104.1.AC3:** Given unavailable or excessive credentials, when diagnostics run, then AgentMage reports non-secret remediation and fails closed without persisting or exposing the credential.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 104.AC1:** All 2,000 confusion attacks produce zero wrong-domain request or disclosure.
- [ ] **Sprint AC 104.AC2:** Every pairwise capability escalation is denied.
- [ ] **Sprint AC 104.AC3:** Secret canaries are absent from all prohibited surfaces.
- [ ] **Sprint AC 104.AC4:** Concurrent provider workers cannot share credentials, caches, grants, or context.
- [ ] **Sprint AC 104.AC5:** `RV-24` passes with independently reproducible evidence.

**Gate decision:** Sprint 104 is PASS only when Story 104.1, all criteria, `AT-IDN-001`, and the Universal Story Definition of Done pass with current evidence. Otherwise it is BLOCKED.

### [ ] Sprint 105 - External Effects, Events, and Uncertain Results

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Implement one safe lifecycle for remote mutations and event ingestion before any live provider write or execution adapter is promoted.

**Source coverage:** `AM-ADP-001`, `AM-IDN-001`, `AT-ADP-001`, `AT-XTE-001`; `DELIVERY-SYSTEM.md` Sections 5 and 8; `SR-DEL-005` through `SR-DEL-009`, `RV-25`, `RV-26`.

**Dependencies:** Sprint 104; Sprints 35-40, 85-88, and 89.

#### [ ] Story 105.1 - Exact External Effect and Event Lifecycle

**User-facing value:** As a user, I need AgentMage to execute exactly one reviewed external effect and recover truthfully when the provider or network gives an uncertain result.

##### Tasks and Sub-tasks

- [ ] **Task 105.1.1 - Implement effect planning and execution**
  - [ ] **Sub-task 105.1.1.1:** Define canonical effect plans containing actor, target, object, payload, attachments, visibility, source revision, environment, expected change, cost/budget, preconditions, expiry, and recovery.
  - [ ] **Sub-task 105.1.1.2:** Re-read affected remote objects before preview and again before grant consumption; invalidate approval on any identity, policy, permission, state, or payload change.
  - [ ] **Sub-task 105.1.1.3:** Implement provider idempotency keys and deterministic operation fingerprints with effect, non-effect, duplicate, partial, and unknown reconciliation states.
  - [ ] **Sub-task 105.1.1.4:** Implement verified postconditions and receipts for denial, cancellation, validation failure, timeout, partial effect, unknown effect, duplicate effect, and success.
  - [ ] **Sub-task 105.1.1.5:** Implement rollback/compensation only as a fresh plan and grant that refuses to overwrite later independent changes.
- [ ] **Task 105.1.2 - Implement event integrity**
  - [ ] **Sub-task 105.1.2.1:** Define webhook/event identity, signature, host, tenant, timestamp, nonce, sequence, cursor, ordering, duplication, tombstone, and backfill contracts.
  - [ ] **Sub-task 105.1.2.2:** Implement replay windows, secret rotation, gap detection, bounded polling overlap, pagination-loop detection, and idempotent event application.
  - [ ] **Sub-task 105.1.2.3:** Keep event content untrusted and unable to create a grant, approval, completion claim, or follow-on operation.
- [ ] **Task 105.1.3 - Verify and close the story**
  - [ ] **Sub-task 105.1.3.1:** `S-105-UT01` field-mutates every plan, preview, grant, request, result, reconciliation, receipt, and compensation schema.
  - [ ] **Sub-task 105.1.3.2:** `S-105-FT01` injects loss/crash before send, during transport, after effect, before local commit, and during reconciliation across at least 1,000 runs; assert zero duplicate effect.
  - [ ] **Sub-task 105.1.3.3:** `S-105-ST01` forges, delays, replays, duplicates, reorders, omits, truncates, and mutates events, cursors, signatures, timestamps, and tombstones.
  - [ ] **Sub-task 105.1.3.4:** `S-105-RT01` changes remote state after effect and before rollback; assert a stale rollback cannot execute and later work is preserved.
  - [ ] **Sub-task 105.1.3.5:** Execute `RV-25` and `RV-26`; retain raw provider snapshots, operation fingerprints, event streams, fault schedules, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 105.1.AC1:** Given a current exact preview, when the user approves and the provider responds normally, then exactly the previewed effect occurs and its postcondition and receipt are verified.
- [ ] **Story AC 105.1.AC2:** Given a timeout, crash, duplicate response, or partial provider effect, when recovery runs, then no retry occurs until reconciliation proves the current effect state.
- [ ] **Story AC 105.1.AC3:** Given forged, replayed, missing, or reordered events, when ingestion runs, then local state remains deterministic, gaps are visible, and no event gains operation authority.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 105.AC1:** Field or remote-state changes invalidate approval before any provider request.
- [ ] **Sprint AC 105.AC2:** At least 1,000 fault schedules produce zero duplicate external effect.
- [ ] **Sprint AC 105.AC3:** Unknown and partial effects remain visible and block unsafe retry.
- [ ] **Sprint AC 105.AC4:** Rollback and compensation preserve later independent changes.
- [ ] **Sprint AC 105.AC5:** `RV-25` and `RV-26` pass with current independent evidence.

**Gate decision:** Sprint 105 is PASS only when Story 105.1, all criteria, the effect/event portions of `AT-XTE-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 106 - GitHub.com and GitHub Enterprise Full Conformance

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Complete the published GitHub.com and GitHub Enterprise Server matrix, including local signed commits, separate pushes, hosted writes, workflows, reviews, and releases.

**Source coverage:** `AM-GHE-001`, `AT-GHE-001`; inventory Sections 13, 13A, 13B, and 36B; `SR-DEL-001` through `SR-DEL-009`, `RV-23` through `RV-26`.

**Dependencies:** Sprint 105; Sprints 41-47, 70-75, and 85-86.

#### [ ] Story 106.1 - Bounded Complete GitHub Capability

**User-facing value:** As a developer, I need one chat surface to understand and safely update personal or enterprise GitHub repositories without hiding the exact account, branch, review, workflow, or publication effect.

##### Tasks and Sub-tasks

- [ ] **Task 106.1.1 - Complete observable GitHub behavior**
  - [ ] **Sub-task 106.1.1.1:** Normalize GitHub.com and version-bounded GitHub Enterprise Server host discovery, authentication, organizations, repositories, refs, commits, trees, blobs, releases, rulesets, security, and Actions objects.
  - [ ] **Sub-task 106.1.1.2:** Normalize issues, discussions where supported, pull requests, reviews, threads, checks, workflows, environments, artifacts, notifications, pagination, rate limits, and permission gaps.
  - [ ] **Sub-task 106.1.1.3:** Preserve REST/GraphQL/provider-only identities and unknown fields without flattening unsupported GHES differences.
- [ ] **Task 106.1.2 - Complete effectful GitHub behavior**
  - [ ] **Sub-task 106.1.2.1:** Implement exact drafts and approval-gated issue, comment, label, assignment, milestone, project-field, pull-request, review, thread, workflow, environment, and release operations.
  - [ ] **Sub-task 106.1.2.2:** Implement exact local staged diff/message approval, required commit signing, signature verification, and a distinct approval for remote push.
  - [ ] **Sub-task 106.1.2.3:** Implement approval-gated workflow dispatch/rerun/cancel and environment approval as `execute`, never generic `remote-write`.
  - [ ] **Sub-task 106.1.2.4:** Keep force push, repository/organization administration, secret changes, ruleset changes, automatic merge, automatic release, and automatic review absent unless separately promoted to L5.
- [ ] **Task 106.1.3 - Verify and close the story**
  - [ ] **Sub-task 106.1.3.1:** `S-106-CT01` runs every published GitHub object and operation across GitHub.com and the supported GHES version matrix, permission levels, pagination, rate limits, and unavailable features.
  - [ ] **Sub-task 106.1.3.2:** `S-106-ST01` runs cross-host credentials, redirects, stale refs, moved lines, branch protection changes, injected content, hidden fields, and every prohibited operation.
  - [ ] **Sub-task 106.1.3.3:** `S-106-IT01` reads an issue, creates an isolated change, tests it, signs a local commit, separately pushes, opens a pull request, submits a review, dispatches CI, verifies checks, drafts a release, and reconciles every effect.
  - [ ] **Sub-task 106.1.3.4:** `S-106-RT01` injects GHES version skew, revocation, single-sign-on changes, rate limits, branch movement, timeout, duplicate response, partial publication, crash, and cancellation.
  - [ ] **Sub-task 106.1.3.5:** Execute applicable `RV-23` through `RV-26`; retain API traces, pre/post snapshots, signatures, support matrix, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 106.1.AC1:** Given a supported GitHub.com or GHES tuple, when a published read or write operation runs, then its provider semantics, permissions, immutable identities, exact effects, and receipts match the support matrix.
- [ ] **Story AC 106.1.AC2:** Given a local commit and remote push, when approvals occur, then staged diff/message/signature and remote destination/ref are reviewed and granted separately.
- [ ] **Story AC 106.1.AC3:** Given an unsupported GHES feature or administrative operation, when requested directly or indirectly, then it is absent or denied without fallback to a different host or method.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 106.AC1:** The complete published GitHub.com/GHES matrix passes at its declared level.
- [ ] **Sprint AC 106.AC2:** No credential crosses GitHub hosts or accounts.
- [ ] **Sprint AC 106.AC3:** Every hosted effect has exact pre/post snapshots, reconciliation, and one receipt.
- [ ] **Sprint AC 106.AC4:** Commit, push, review, workflow execution, environment approval, merge preparation, and release remain separate capability classes.
- [ ] **Sprint AC 106.AC5:** All prohibited and unsupported operations pass negative tests.

**Gate decision:** Sprint 106 is PASS only when Story 106.1, all criteria, `AT-GHE-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 107 - Work Management Across GitHub, Jira, and Azure Boards

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Deliver truthful, identity-safe work-item planning and approved updates across GitHub Issues, Jira Cloud/Data Center, and Azure Boards.

**Source coverage:** `AM-WRK-001`, `AT-WRK-001`; `DELIVERY-SYSTEM.md`; `SR-DEL-001` through `SR-DEL-013`, `RV-23` through `RV-27`.

**Dependencies:** Sprint 106.

#### [ ] Story 107.1 - Cross-Provider Work Planning

**User-facing value:** As a delivery lead or developer, I need issues and work items correlated without losing provider-specific fields or accidentally updating the wrong project.

##### Tasks and Sub-tasks

- [ ] **Task 107.1.1 - Implement work-item reads and graph links**
  - [ ] **Sub-task 107.1.1.1:** Implement GitHub Issue, Jira Cloud, Jira Data Center, and Azure Boards identities, types, fields, states, transitions, hierarchy, links, iterations, comments, attachments, history, and permissions.
  - [ ] **Sub-task 107.1.1.2:** Preserve provider-only workflows, custom fields, projects, area/iteration paths, boards, sprints, and link semantics as namespaced extensions.
  - [ ] **Sub-task 107.1.1.3:** Link work items to repositories, branches, commits, reviews, builds, releases, incidents, and evidence only through exact provider references or labeled inference.
- [ ] **Task 107.1.2 - Implement bounded drafts and writes**
  - [ ] **Sub-task 107.1.2.1:** Draft and preview create, edit, comment, assign, label/tag, link, attach, transition, close, and reopen operations with exact field-level effects.
  - [ ] **Sub-task 107.1.2.2:** Re-read workflow, field schema, permissions, object revision, and attachment identity before submission.
  - [ ] **Sub-task 107.1.2.3:** Implement provider-specific idempotency/reconciliation and prevent hidden recipients, watchers, visibility changes, project moves, or cascading transitions.
- [ ] **Task 107.1.3 - Verify and close the story**
  - [ ] **Sub-task 107.1.3.1:** `S-107-CT01` runs all published objects, custom-field types, transitions, links, attachments, pagination, permissions, and version fixtures for all four providers.
  - [ ] **Sub-task 107.1.3.2:** `S-107-ST01` attacks cross-project identity, reused issue numbers, hidden watchers, malicious attachments, injected comments, stale transitions, and provider-link confusion.
  - [ ] **Sub-task 107.1.3.3:** `S-107-IT01` creates and links synthetic work across providers, drafts updates, approves one exact transition, and verifies only declared fields change.
  - [ ] **Sub-task 107.1.3.4:** `S-107-RT01` injects schema change, permission loss, object move/delete, transition removal, rate limit, timeout, duplicate response, and crash.
  - [ ] **Sub-task 107.1.3.5:** Retain support matrices, normalized/provider-extension fixtures, pre/post snapshots, attachment scans, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 107.1.AC1:** Given work items with identical numbers or names in different domains, when AgentMage reads or links them, then immutable provider identity prevents cross-project or cross-tenant confusion.
- [ ] **Story AC 107.1.AC2:** Given provider-specific workflows and custom fields, when AgentMage previews an update, then it preserves exact semantics and exposes unsupported or unknown behavior rather than guessing.
- [ ] **Story AC 107.1.AC3:** Given a changed schema, transition, permission, attachment, or object revision, when submission begins, then stale approval is invalidated and no external effect occurs.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 107.AC1:** Every promoted work provider passes its exact object/operation/version matrix.
- [ ] **Sprint AC 107.AC2:** Cross-provider links preserve evidence and never become operation authority.
- [ ] **Sprint AC 107.AC3:** Hidden recipients, cross-project moves, and cascading effects are absent or separately previewed.
- [ ] **Sprint AC 107.AC4:** Duplicate and uncertain results reconcile without duplicate work items or comments.
- [ ] **Sprint AC 107.AC5:** `AT-WRK-001` passes with independent evidence.

**Gate decision:** Sprint 107 is PASS only when Story 107.1, all criteria, `AT-WRK-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 108 - Azure Repos and GitLab Source Adapters

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Add Azure Repos and GitLab source/review capabilities without weakening the Git or provider authority contracts proven for GitHub.

**Source coverage:** `AM-SRC-001`, `AT-SRC-001`; inventory Sections 13, 13A, 13B, and 36B; `SR-DEL-*`, `RV-23` through `RV-27`.

**Dependencies:** Sprint 107; Sprints 41-47 and 74.

#### [ ] Story 108.1 - Multi-Provider Source and Review

**User-facing value:** As a developer, I need the same bounded repository, review, commit, and push workflow on Azure Repos and GitLab while retaining each provider's policies and identities.

##### Tasks and Sub-tasks

- [ ] **Task 108.1.1 - Implement source-provider reads**
  - [ ] **Sub-task 108.1.1.1:** Implement repository, project/group, ref, commit, tree, blob, tag, release, policy/protection, pull/merge request, review, thread, status, check, and permission reads for Azure Repos and GitLab.
  - [ ] **Sub-task 108.1.1.2:** Implement immutable PR/merge-request checkout into isolated worktrees with base/head refresh, moved-line handling, instruction discovery, and local review evidence.
  - [ ] **Sub-task 108.1.1.3:** Preserve Azure and GitLab-specific review, approval, pipeline, fork, protection, and merge semantics as namespaced fields.
- [ ] **Task 108.1.2 - Implement source-provider writes**
  - [ ] **Sub-task 108.1.2.1:** Implement exact draft and approval flows for branch push, pull/merge request, comments, review/thread, labels, reviewers, draft state, closure, and merge preparation within the matrix.
  - [ ] **Sub-task 108.1.2.2:** Enforce signed local commits where the repository policy requires them and always separate commit approval from remote push approval.
  - [ ] **Sub-task 108.1.2.3:** Keep force push, protected-branch bypass, repository/project/group administration, secret changes, and automatic merge/release absent.
- [ ] **Task 108.1.3 - Verify and close the story**
  - [ ] **Sub-task 108.1.3.1:** `S-108-CT01` runs Azure Repos and GitLab matrices across cloud/self-hosted versions, permissions, forks, protected branches, merge methods, pagination, and unavailable features.
  - [ ] **Sub-task 108.1.3.2:** `S-108-ST01` tests credential crossover, malicious diffs, stale refs, line movement, hooks, submodules, large-file pointers, branch policy changes, and every prohibited operation.
  - [ ] **Sub-task 108.1.3.3:** `S-108-IT01` performs issue-linked isolated changes, tests, signed commit, separate push, review submission, refresh, and postcondition verification on both providers.
  - [ ] **Sub-task 108.1.3.4:** `S-108-RT01` injects version skew, permission loss, branch movement, partial publication, timeout, duplicate response, crash, and cancellation.
  - [ ] **Sub-task 108.1.3.5:** Retain provider matrices, API traces, worktree snapshots, signatures, pre/post state, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 108.1.AC1:** Given a supported Azure Repos or GitLab tuple, when repository or review operations run, then provider-specific policy and identity remain exact while shared Git behavior matches the common contract.
- [ ] **Story AC 108.1.AC2:** Given a changed branch, review, permission, or policy after preview, when an effect is attempted, then approval is invalidated and no stale push or hosted update occurs.
- [ ] **Story AC 108.1.AC3:** Given a force, bypass, administrative, or unsupported operation, when requested, then the operation remains absent or is denied before provider contact.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 108.AC1:** Azure Repos and GitLab pass their published read/write matrices.
- [ ] **Sprint AC 108.AC2:** Active user checkouts and unrelated changes remain untouched.
- [ ] **Sprint AC 108.AC3:** Commit and push remain distinct approvals with signature evidence.
- [ ] **Sprint AC 108.AC4:** No cross-provider credential, cache, identity, or receipt collision occurs.
- [ ] **Sprint AC 108.AC5:** `AT-SRC-001` passes with current independent evidence.

**Gate decision:** Sprint 108 is PASS only when Story 108.1, all criteria, `AT-SRC-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 109 - Multi-Provider CI Execution and Evidence

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Safely inspect and execute GitHub Actions, Azure Pipelines, GitLab CI, and Jenkins with exact source, inputs, environment, permissions, budgets, and result attribution.

**Source coverage:** `AM-CIC-001`, `AT-CIC-001`; `DELIVERY-SYSTEM.md` Section 9; `SR-DEL-005` through `SR-DEL-014`, `RV-23` through `RV-26`, `RV-29`.

**Dependencies:** Sprint 108; Sprints 47 and 89-90.

#### [ ] Story 109.1 - Bounded Continuous-Integration Control

**User-facing value:** As a developer, I need to inspect, dispatch, rerun, cancel, and approve CI from chat while knowing exactly which source, inputs, environment, and runner will execute.

##### Tasks and Sub-tasks

- [ ] **Task 109.1.1 - Implement CI observation and normalization**
  - [ ] **Sub-task 109.1.1.1:** Implement workflow/pipeline/job/step/run/attempt/annotation/log/artifact/environment/approval identities for GitHub Actions, Azure Pipelines, GitLab CI, and Jenkins.
  - [ ] **Sub-task 109.1.1.2:** Parse definitions as untrusted source and expose triggers, permissions, variables by name, secret references by name, concurrency, runners/agents, retention, dependencies, and reusable components without secret values.
  - [ ] **Sub-task 109.1.1.3:** Correlate each run with exact repository, immutable revision, actor, inputs, environment, worker identity, result, logs, artifacts, and receipt.
- [ ] **Task 109.1.2 - Implement CI execution classes**
  - [ ] **Sub-task 109.1.2.1:** Preview and grant dispatch, rerun, cancel, and environment approval separately with exact ref, inputs, environment, permissions, budget, expected artifacts, and cancellation contract.
  - [ ] **Sub-task 109.1.2.2:** Implement provider idempotency/reconciliation for dispatch and rerun, including queue identity and timeout before/after run creation.
  - [ ] **Sub-task 109.1.2.3:** Stream bounded logs and artifacts through secret scanning, archive/path defenses, size limits, cancellation, classification, and retention before model use.
  - [ ] **Sub-task 109.1.2.4:** Keep CI definition changes, runner/agent administration, secret changes, and deployment approval outside generic CI execution.
- [ ] **Task 109.1.3 - Verify and close the story**
  - [ ] **Sub-task 109.1.3.1:** `S-109-CT01` runs each provider's definition/read/dispatch/rerun/cancel/approval/log/artifact matrix across versions and permissions.
  - [ ] **Sub-task 109.1.3.2:** `S-109-ST01` injects malicious YAML/scripts/logs/artifacts, hidden inputs, secret echoes, redirect downloads, archive bombs, stale refs, runner confusion, and nested deployment attempts.
  - [ ] **Sub-task 109.1.3.3:** `S-109-FT01` executes at least 1,000 timeout/retry/crash/duplicate schedules around run creation and reconciliation; assert zero duplicate execution.
  - [ ] **Sub-task 109.1.3.4:** `S-109-RT01` exercises rate limits, queue delay, permission reduction, provider outage, cancellation races, oversized output, full disk, and restart.
  - [ ] **Sub-task 109.1.3.5:** Retain exact run graphs, provider traces, secret scans, artifact inventories, effect reconciliation, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 109.1.AC1:** Given a supported CI provider, when a run is inspected, then every result and artifact is attributable to the exact source revision, inputs, environment, actor, worker, and attempt.
- [ ] **Story AC 109.1.AC2:** Given an approved dispatch or rerun, when uncertainty or retry occurs, then no duplicate run is created and AgentMage reports unknown state until reconciliation completes.
- [ ] **Story AC 109.1.AC3:** Given hostile definitions, logs, or artifacts, when they are processed, then they remain bounded untrusted data and cannot disclose secrets or trigger a stronger capability.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 109.AC1:** All four CI providers pass the published observe and execute matrices.
- [ ] **Sprint AC 109.AC2:** At least 1,000 uncertain-result schedules create zero duplicate runs.
- [ ] **Sprint AC 109.AC3:** CI execution cannot imply deployment, secret, or administration authority.
- [ ] **Sprint AC 109.AC4:** Logs and artifacts pass bounds, archive, secret, retention, and cancellation tests.
- [ ] **Sprint AC 109.AC5:** `AT-CIC-001` passes with independent evidence.

**Gate decision:** Sprint 109 is PASS only when Story 109.1, all criteria, `AT-CIC-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 110 - Artifact Registries and Immutable Promotion

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Make immutable artifact identity the bridge between CI and deployment across OCI, GitHub, Azure, Artifactory, and Nexus repositories.

**Source coverage:** `AM-ART-001`, `AT-ART-001`; `DELIVERY-SYSTEM.md` Section 9; `SR-DEL-001` through `SR-DEL-014`, `RV-23` through `RV-27`.

**Dependencies:** Sprint 109.

#### [ ] Story 110.1 - Digest-First Artifact Lifecycle

**User-facing value:** As a release operator, I need every downloadable or promotable artifact tied to an immutable digest and provenance rather than a mutable tag or build label.

##### Tasks and Sub-tasks

- [ ] **Task 110.1.1 - Implement artifact-provider reads**
  - [ ] **Sub-task 110.1.1.1:** Implement repository, package, manifest, blob, digest, tag, version, build-info, property, retention, signature, provenance, and permission reads for OCI Distribution, GitHub Container Registry, Azure Container Registry, Artifactory, and Nexus.
  - [ ] **Sub-task 110.1.1.2:** Resolve tags and names to immutable digests and record observed mapping time, source host, repository, media type, size, platform, and deletion state.
  - [ ] **Sub-task 110.1.1.3:** Implement bounded streaming download with hash verification, archive/path defenses, cancellation, quarantine, cleanup, and no model access to raw binary content.
- [ ] **Task 110.1.2 - Implement promotion and retention**
  - [ ] **Sub-task 110.1.2.1:** Preview source digest, target repository, target labels/tags, metadata, retention, signature/provenance state, expected bytes, and overwrite/collision behavior.
  - [ ] **Sub-task 110.1.2.2:** Implement approval-gated copy/promotion by immutable digest, postcondition verification, idempotency, partial-upload recovery, and cleanup.
  - [ ] **Sub-task 110.1.2.3:** Keep delete, retention-policy change, signing-key use, repository administration, and mutable-tag replacement as separately disabled or separately approved operations.
- [ ] **Task 110.1.3 - Verify and close the story**
  - [ ] **Sub-task 110.1.3.1:** `S-110-CT01` runs repository/package/media-type/platform/digest/tag/version/permission/retention matrices for all promoted providers.
  - [ ] **Sub-task 110.1.3.2:** `S-110-ST01` tests digest mismatch, tag swap, manifest confusion, cross-repository credential use, malicious media types, traversal archives, oversized layers, decompression bombs, and signature spoofing.
  - [ ] **Sub-task 110.1.3.3:** `S-110-IT01` traces CI output to an immutable artifact, verifies it, promotes it once, refreshes both repositories, and proves exact bytes and metadata.
  - [ ] **Sub-task 110.1.3.4:** `S-110-RT01` injects partial upload, rate limit, timeout, tag race, deletion, retention conflict, full disk, cancellation, crash, and restart.
  - [ ] **Sub-task 110.1.3.5:** Retain digest maps, transfer traces, scanner output, pre/post inventories, cleanup scans, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 110.1.AC1:** Given a mutable artifact label, when AgentMage reads or promotes it, then authority binds only to the resolved immutable digest and a changed mapping invalidates approval.
- [ ] **Story AC 110.1.AC2:** Given a partial, failed, duplicated, or cancelled transfer, when reconciliation runs, then no corrupt artifact is promoted and cleanup/retry behavior is deterministic.
- [ ] **Story AC 110.1.AC3:** Given a destructive, administrative, retention, or signing-key operation, when requested through promotion authority, then it remains absent or is separately gated.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 110.AC1:** All promoted artifact providers pass their exact read and promotion matrices.
- [ ] **Sprint AC 110.AC2:** Every promoted artifact is digest-, hash-, size-, media-, source-, and receipt-bound.
- [ ] **Sprint AC 110.AC3:** Mutable labels never carry operation authority.
- [ ] **Sprint AC 110.AC4:** Malicious, partial, oversized, and mismatched artifacts remain quarantined and bounded.
- [ ] **Sprint AC 110.AC5:** `AT-ART-001` passes with independent evidence.

**Gate decision:** Sprint 110 is PASS only when Story 110.1, all criteria, `AT-ART-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 111 - Supply-Chain Evidence and Security Findings

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Tie software bills of materials, signatures, provenance, policy, and security findings to exact source, build, and artifact identities without losing conflicting evidence.

**Source coverage:** `AM-SUP-014`, `AM-SEC-003`, `AT-SUP-001`, `AT-SEC-003`; `SR-SUP-*`, `SR-DEL-004`, `SR-DEL-011`, `SR-DEL-013`, `RV-19`, `RV-23`, `RV-27`.

**Dependencies:** Sprint 110; Sprint 3 supply-chain foundation.

#### [ ] Story 111.1 - Verifiable Supply Chain and Finding Reconciliation

**User-facing value:** As a reviewer, I need every artifact and finding traced to exact evidence so that signatures, bills of materials, vulnerabilities, and policy results cannot be mixed, hidden, or overstated.

##### Tasks and Sub-tasks

- [ ] **Task 111.1.1 - Implement supply-chain evidence**
  - [ ] **Sub-task 111.1.1.1:** Parse and validate SPDX and CycloneDX documents with schema, producer, subject, component, dependency, license, hash, and generation-context preservation.
  - [ ] **Sub-task 111.1.1.2:** Verify Sigstore/Cosign signatures and attestations, signer identities, transparency evidence where applicable, certificate validity, policy, and artifact digest.
  - [ ] **Sub-task 111.1.1.3:** Validate SLSA provenance predicates and source/build/artifact links without claiming an assurance level not proven by the evidence.
  - [ ] **Sub-task 111.1.1.4:** Implement versioned OPA/Conftest-style policy inputs, bundle identity, result, explanation, and release-gate mapping.
- [ ] **Task 111.1.2 - Implement security-finding normalization**
  - [ ] **Sub-task 111.1.2.1:** Ingest CodeQL, Semgrep, SonarQube, Snyk, Trivy, Grype, and SARIF findings with original tool, rule, version, location, severity, confidence, reachability, suppression, and immutable source identity.
  - [ ] **Sub-task 111.1.2.2:** Correlate duplicates without erasing conflicting severity, location, reachability, fix, or suppression evidence.
  - [ ] **Sub-task 111.1.2.3:** Detect stale locations and changed source/artifact identities before presenting or gating a finding.
  - [ ] **Sub-task 111.1.2.4:** Keep finding text, remediation, suppressions, and policy output untrusted and unable to change release state without deterministic gate logic.
- [ ] **Task 111.1.3 - Verify and close the story**
  - [ ] **Sub-task 111.1.3.1:** `S-111-UT01` mutates each SBOM, signature, attestation, provenance, policy, and finding field, schema version, identity, and digest.
  - [ ] **Sub-task 111.1.3.2:** `S-111-ST01` injects forged signers, swapped subjects, incomplete graphs, malicious package URLs, suppression abuse, conflicting severities, stale lines, and injected remediation.
  - [ ] **Sub-task 111.1.3.3:** `S-111-IT01` reconstructs source-to-build-to-artifact-to-deployment evidence and independently verifies signatures, bills of materials, findings, and policy.
  - [ ] **Sub-task 111.1.3.4:** `S-111-RT01` changes source, rebuilds under the same label, revokes a signer, updates a finding tool, and makes prior evidence stale; assert blocked reuse.
  - [ ] **Sub-task 111.1.3.5:** Execute `RV-19` and applicable `RV-23`/`RV-27`; retain raw tool output, normalization diffs, signature/provenance verification, policy bundles, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 111.1.AC1:** Given an artifact, when supply-chain evidence is evaluated, then every component, signature, provenance statement, policy result, and finding resolves to that exact digest and recorded producer.
- [ ] **Story AC 111.1.AC2:** Given conflicting or duplicate security findings, when normalization runs, then original evidence remains visible and no blocking result disappears through deduplication or suppression.
- [ ] **Story AC 111.1.AC3:** Given changed source, artifact, signer, policy, or tool identity, when prior evidence is reused, then it becomes stale and cannot satisfy the release gate.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 111.AC1:** SPDX, CycloneDX, signatures, provenance, and policy evidence reconcile to immutable artifacts.
- [ ] **Sprint AC 111.AC2:** All promoted security tools preserve original provenance and conflicts.
- [ ] **Sprint AC 111.AC3:** Unsupported assurance claims and hidden blocking findings are rejected.
- [ ] **Sprint AC 111.AC4:** Staleness propagates from source/build/artifact/tool/policy changes.
- [ ] **Sprint AC 111.AC5:** `AT-SUP-001` and `AT-SEC-003` pass with independent evidence.

**Gate decision:** Sprint 111 is PASS only when Story 111.1, all criteria, `AT-SUP-001`, `AT-SEC-003`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 112 - Kubernetes, Helm, and Kustomize Deployment Safety

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Add inspect, plan, promote, health, drift, and rollback behavior for Kubernetes, Helm, and Kustomize with deployment authority isolated from generic writes and CI.

**Source coverage:** `AM-DEP-001`, `AT-DEP-001`; `DELIVERY-SYSTEM.md` Section 9; `SR-DEL-002`, `SR-DEL-005` through `SR-DEL-014`, `RV-25`, `RV-28`.

**Dependencies:** Sprint 111.

#### [ ] Story 112.1 - Exact Kubernetes Deployment Lifecycle

**User-facing value:** As an operator, I need to see the exact resources and artifact that will change and to retain a tested health and rollback path before any cluster effect occurs.

##### Tasks and Sub-tasks

- [ ] **Task 112.1.1 - Implement deployment observation and planning**
  - [ ] **Sub-task 112.1.1.1:** Implement cluster/context, namespace, workload, service, ingress, configuration metadata, policy, event, revision, owner, health, rollout, and drift reads without exposing secret values.
  - [ ] **Sub-task 112.1.1.2:** Implement pinned offline Helm rendering and Kustomize build with exact source revision, dependency/chart digest, values/input hashes, renderer version, and bounded output.
  - [ ] **Sub-task 112.1.1.3:** Produce deterministic resource-level create/update/delete/no-change plans and flag immutable-field, ownership, policy, secret-reference, namespace, and production boundaries.
  - [ ] **Sub-task 112.1.1.4:** Bind deploy plans to immutable artifact digests, exact cluster identity, namespace, policy, actor, health criteria, timeout, and rollback target.
- [ ] **Task 112.1.2 - Implement promotion, health, and rollback**
  - [ ] **Sub-task 112.1.2.1:** Register non-production and production deploy as separate grants and keep secret/admin changes outside deployment.
  - [ ] **Sub-task 112.1.2.2:** Verify post-deployment resource identity, rollout status, health windows, events, metrics references, drift, and artifact digest.
  - [ ] **Sub-task 112.1.2.3:** Implement cancellation, unknown-effect reconciliation, partial deployment reporting, and fresh approval for rollback or compensation.
- [ ] **Task 112.1.3 - Verify and close the story**
  - [ ] **Sub-task 112.1.3.1:** `S-112-UT01` mutates cluster, context, namespace, source, chart, values, resource, policy, artifact, health, timeout, and rollback fields.
  - [ ] **Sub-task 112.1.3.2:** `S-112-ST01` tests context confusion, namespace escape, malicious templates, resource bombs, hidden hooks, secret output, policy bypass, image-tag swap, and nested admin actions.
  - [ ] **Sub-task 112.1.3.3:** `S-112-IT01` promotes an immutable synthetic artifact to non-production, verifies health, detects drift, simulates failure, previews rollback, approves it separately, and verifies final state.
  - [ ] **Sub-task 112.1.3.4:** `S-112-FT01` crashes/cancels before apply, during apply, after partial effect, during health, and during rollback; assert exact reconciliation and no unsafe retry.
  - [ ] **Sub-task 112.1.3.5:** Execute deployment portions of `RV-25` and `RV-28`; retain plans, manifests, cluster snapshots, health evidence, drift, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 112.1.AC1:** Given a rendered deployment plan, when any source, artifact, cluster, namespace, policy, or resource precondition changes, then approval becomes stale before cluster contact.
- [ ] **Story AC 112.1.AC2:** Given an approved deployment, when it executes, then only previewed resources in the exact environment change and health/postconditions bind to the immutable artifact.
- [ ] **Story AC 112.1.AC3:** Given partial effect, health failure, or later independent change, when recovery runs, then the state remains explicit and rollback requires a fresh non-destructive review.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 112.AC1:** Helm/Kustomize rendering is pinned, offline, deterministic, bounded, and source-preserving.
- [ ] **Sprint AC 112.AC2:** Deployment, production promotion, secret change, and cluster administration are separate capabilities.
- [ ] **Sprint AC 112.AC3:** Every cluster effect has exact resource pre/post state and artifact identity.
- [ ] **Sprint AC 112.AC4:** Crash, partial-effect, drift, health, and rollback fixtures recover safely.
- [ ] **Sprint AC 112.AC5:** `AT-DEP-001` deployment subset and `RV-28` pass.

**Gate decision:** Sprint 112 is PASS only when Story 112.1, all criteria, the applicable `AT-DEP-001` cases, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 113 - Argo CD and Flux GitOps Control

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Add GitOps observation, synchronization, health, drift, suspension, and rollback without turning repository write or CI authority into deployment authority.

**Source coverage:** `AM-DEP-001`, `AT-DEP-001`; `SR-DEL-*`, `RV-25`, `RV-27`, `RV-28`.

**Dependencies:** Sprint 112.

#### [ ] Story 113.1 - Bounded GitOps Reconciliation

**User-facing value:** As an operator, I need AgentMage to distinguish changing desired state from synchronizing an environment and to preview every prune, hook, and rollback effect.

##### Tasks and Sub-tasks

- [ ] **Task 113.1.1 - Implement GitOps reads and planning**
  - [ ] **Sub-task 113.1.1.1:** Implement Argo CD and Flux application/source/revision/destination/resource/health/sync/drift/history/event/permission reads.
  - [ ] **Sub-task 113.1.1.2:** Correlate exact Git source, rendered resources, artifact digests, cluster/namespace, current live state, desired state, and prior synchronization.
  - [ ] **Sub-task 113.1.1.3:** Preview sync, prune, force, replace, hook, suspend, resume, reconcile, and rollback as distinct effects; keep force/replace/prune disabled by default.
- [ ] **Task 113.1.2 - Implement approved GitOps effects**
  - [ ] **Sub-task 113.1.2.1:** Implement ordinary sync/reconcile under exact revision, destination, resource diff, policy, health, timeout, and idempotency conditions.
  - [ ] **Sub-task 113.1.2.2:** Implement suspend/resume and rollback as distinct grants with refreshed controller and live-cluster state.
  - [ ] **Sub-task 113.1.2.3:** Detect controller-driven effects, concurrent reconciliation, changed desired state, auto-sync policy, and unknown completion before any retry.
- [ ] **Task 113.1.3 - Verify and close the story**
  - [ ] **Sub-task 113.1.3.1:** `S-113-CT01` runs application/source/destination/sync/health/drift/history matrices for supported Argo CD and Flux versions.
  - [ ] **Sub-task 113.1.3.2:** `S-113-ST01` tests repository/cluster confusion, malicious hooks, prune escalation, auto-sync races, stale desired state, controller impersonation, and hidden secret/admin effects.
  - [ ] **Sub-task 113.1.3.3:** `S-113-IT01` detects drift, previews ordinary sync, approves it, verifies health, changes desired state concurrently, and proves stale rollback/sync denial.
  - [ ] **Sub-task 113.1.3.4:** `S-113-RT01` injects controller outage, partial sync, delayed events, duplicate reconciliation, permission loss, cancellation, crash, and restart.
  - [ ] **Sub-task 113.1.3.5:** Retain source/live/desired snapshots, controller events, resource diffs, effect reconciliation, receipts, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 113.1.AC1:** Given GitOps desired and live state, when AgentMage plans a sync, then it identifies the exact source revision, controller, destination, resource effects, health conditions, and auto-sync policy.
- [ ] **Story AC 113.1.AC2:** Given ordinary sync authority, when prune, force, replace, hook, secret, or administration is embedded or inferred, then the stronger effect remains absent or separately gated.
- [ ] **Story AC 113.1.AC3:** Given controller or desired-state changes during execution, when reconciliation runs, then AgentMage reports current effect truth and blocks unsafe retry or rollback.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 113.AC1:** Argo CD and Flux pass their published observe and bounded-effect matrices.
- [ ] **Sprint AC 113.AC2:** Desired-state write and environment synchronization remain distinct authorities.
- [ ] **Sprint AC 113.AC3:** Prune, force, replace, hook, secret, and admin paths pass negative or separate-gate tests.
- [ ] **Sprint AC 113.AC4:** Controller races, delayed events, and partial sync reconcile without duplicate effect.
- [ ] **Sprint AC 113.AC5:** The complete `AT-DEP-001` gate passes with independent evidence.

**Gate decision:** Sprint 113 is PASS only when Story 113.1, all criteria, `AT-DEP-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 114 - Terraform and OpenTofu Infrastructure Safety

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Add infrastructure inspection, plan, policy, apply, and recovery while preserving exact state identity and separating destructive or production effects.

**Source coverage:** `AM-IAC-001`, `AT-IAC-001`; `DELIVERY-SYSTEM.md` Section 9; `SR-DEL-*`, `RV-25`, `RV-28`, `RV-29`.

**Dependencies:** Sprint 113.

#### [ ] Story 114.1 - Exact Infrastructure Plan and Apply

**User-facing value:** As an infrastructure developer, I need AgentMage to explain and apply only an exact reviewed plan against the current state without leaking secrets or retrying an uncertain apply.

##### Tasks and Sub-tasks

- [ ] **Task 114.1.1 - Implement infrastructure reads and plans**
  - [ ] **Sub-task 114.1.1.1:** Inspect Terraform/OpenTofu configuration, modules, providers, lock files, backends by non-secret identity, workspaces, state serial/lineage, resources, outputs by sensitivity, imports, and drift.
  - [ ] **Sub-task 114.1.1.2:** Run init/validate/plan in an isolated worker using approved pinned providers/modules and explicit network grants where acquisition is required.
  - [ ] **Sub-task 114.1.1.3:** Bind saved plans to tool/provider/module versions, source and lock hashes, backend/workspace identity, state lineage/serial, variables by redacted digest, policy result, and exact create/update/replace/delete effects.
  - [ ] **Sub-task 114.1.1.4:** Classify destructive, replacement, production, data-loss, secret, cost, and policy-sensitive changes for separate approval or denial.
- [ ] **Task 114.1.2 - Implement apply and recovery**
  - [ ] **Sub-task 114.1.2.1:** Apply only an unchanged saved plan under current state lock and preconditions; prohibit ad hoc apply and implicit auto-approve.
  - [ ] **Sub-task 114.1.2.2:** Stream bounded redacted progress, preserve provider request uncertainty, and verify state serial, resources, outputs, drift, and receipts after apply.
  - [ ] **Sub-task 114.1.2.3:** Implement lock conflict, interruption, partial effect, provider failure, state recovery, import, and compensation workflows without automatic retry or state surgery.
- [ ] **Task 114.1.3 - Verify and close the story**
  - [ ] **Sub-task 114.1.3.1:** `S-114-UT01` mutates plan/source/lock/provider/module/backend/workspace/state/variable/policy/effect fields and asserts stale-plan denial.
  - [ ] **Sub-task 114.1.3.2:** `S-114-ST01` tests malicious providers/modules, backend confusion, secret output, state injection, path traversal, plan substitution, destructive concealment, and nested cloud administration.
  - [ ] **Sub-task 114.1.3.3:** `S-114-IT01` plans and applies a non-production synthetic change, verifies state and drift, then proves the same plan cannot apply after source or state movement.
  - [ ] **Sub-task 114.1.3.4:** `S-114-FT01` injects lock loss, provider outage, rate limit, partial effect, timeout, cancellation, crash, full disk, and restart; assert unknown-state blocking.
  - [ ] **Sub-task 114.1.3.5:** Execute infrastructure portions of `RV-25`, `RV-28`, and `RV-29`; retain plans, state digests, redaction scans, policy, pre/post snapshots, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 114.1.AC1:** Given a saved infrastructure plan, when any source, dependency, backend, workspace, state, variable, policy, or provider identity changes, then apply is denied.
- [ ] **Story AC 114.1.AC2:** Given an approved non-production plan, when apply succeeds, then only the exact planned resources change and current state/postconditions are verified without secret disclosure.
- [ ] **Story AC 114.1.AC3:** Given partial or unknown effect, when recovery runs, then no automatic retry or state mutation occurs until current infrastructure and state are reconciled.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 114.AC1:** Terraform and OpenTofu pass identical common contracts and separate version matrices.
- [ ] **Sprint AC 114.AC2:** Plan/apply, non-production/production, destructive/non-destructive, secret, and admin authorities remain separate.
- [ ] **Sprint AC 114.AC3:** State, plan, provider, module, variable, and policy identities are exact and stale-aware.
- [ ] **Sprint AC 114.AC4:** Fault and resource cases never cause duplicate apply, secret leakage, or unsafe state surgery.
- [ ] **Sprint AC 114.AC5:** `AT-IAC-001` passes with independent evidence.

**Gate decision:** Sprint 114 is PASS only when Story 114.1, all criteria, `AT-IAC-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 115 - Releases, Feature Flags, Progressive Delivery, and Migrations

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Complete the release lifecycle with exact promotion, flag, progressive-delivery, migration, health, rollback, and compensation contracts.

**Source coverage:** `AM-REL-001`, `AT-REL-001`; `DELIVERY-SYSTEM.md` Sections 9 and 13; `SR-DEL-*`, `RV-25`, `RV-28`.

**Dependencies:** Sprint 114.

#### [ ] Story 115.1 - Governed Release and Change Lifecycle

**User-facing value:** As a release owner, I need source, artifacts, environments, flags, migrations, health, and rollback tied to one exact release without hiding production or data effects.

##### Tasks and Sub-tasks

- [ ] **Task 115.1.1 - Implement release identity and promotion**
  - [ ] **Sub-task 115.1.1.1:** Build semantic-version and changelog drafts from exact commits, work items, reviews, checks, artifacts, provenance, and prior releases with source citations.
  - [ ] **Sub-task 115.1.1.2:** Define immutable release manifests linking source, CI, artifact, SBOM, provenance, signatures, environments, policies, migrations, flags, health, and rollback.
  - [ ] **Sub-task 115.1.1.3:** Implement environment promotion with exact source/target, immutable digest, approval class, deployment plan, health window, and postcondition.
- [ ] **Task 115.1.2 - Implement flags, progressive delivery, and migrations**
  - [ ] **Sub-task 115.1.2.1:** Implement reference LaunchDarkly and Unleash flag reads/drafts/writes with project/environment/flag/variation/target/prerequisite identity and separate production approval.
  - [ ] **Sub-task 115.1.2.2:** Implement progressive-delivery steps, traffic or audience bounds, pause, health decision, advance, abort, and rollback without autonomous advancement.
  - [ ] **Sub-task 115.1.2.3:** Implement Flyway and Liquibase migration discovery, checksum/order/direction/compatibility/lock/backup/timeout/health contracts and separate destructive approval.
  - [ ] **Sub-task 115.1.2.4:** Make every rollback or compensation a fresh exact plan that accounts for later releases, flag changes, schema state, and user changes.
- [ ] **Task 115.1.3 - Verify and close the story**
  - [ ] **Sub-task 115.1.3.1:** `S-115-UT01` mutates every release, version, artifact, environment, flag, rollout, migration, health, rollback, and compensation field.
  - [ ] **Sub-task 115.1.3.2:** `S-115-ST01` tests tag/version reuse, artifact swap, hidden production target, audience expansion, prerequisite loops, migration checksum/order attacks, secret output, and destructive concealment.
  - [ ] **Sub-task 115.1.3.3:** `S-115-IT01` creates a synthetic release, promotes progressively, applies a compatible migration, changes a flag under separate approval, detects failed health, and executes separately approved compensation.
  - [ ] **Sub-task 115.1.3.4:** `S-115-RT01` injects concurrent release, changed flag, partial migration, lock timeout, health delay, provider outage, cancellation, crash, and later independent change.
  - [ ] **Sub-task 115.1.3.5:** Retain manifests, diffs, health windows, migration/flag identities, pre/post state, receipts, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 115.1.AC1:** Given a release manifest, when promotion begins, then source, artifact, policy, environment, flag, migration, health, and rollback identities are exact and current.
- [ ] **Story AC 115.1.AC2:** Given deployment authority, when a production flag, migration, progressive step, destructive change, or database effect is requested, then it requires its own exact capability and approval.
- [ ] **Story AC 115.1.AC3:** Given failed health or partial migration, when recovery runs, then no automatic advance or retry occurs and compensation preserves later independent state.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 115.AC1:** Release manifests reconcile source through environment and rollback.
- [ ] **Sprint AC 115.AC2:** LaunchDarkly, Unleash, Flyway, and Liquibase pass their promoted matrices.
- [ ] **Sprint AC 115.AC3:** Production, progressive, flag, migration, destructive, secret, and rollback effects remain distinct.
- [ ] **Sprint AC 115.AC4:** Concurrent and partial-change recovery is deterministic and non-destructive.
- [ ] **Sprint AC 115.AC5:** `AT-REL-001` passes with independent evidence.

**Gate decision:** Sprint 115 is PASS only when Story 115.1, all criteria, `AT-REL-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 116 - OpenTelemetry Correlation Foundation

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Establish provider-neutral metric, log, trace, error, monitor, service, environment, release, and incident correlation without converting correlation into causation.

**Source coverage:** `AM-OBS-001`, `AT-OBS-001`; `DELIVERY-SYSTEM.md` Section 10; `SR-DEL-004`, `SR-DEL-011`, `SR-DEL-014`, `RV-27`, `RV-29`.

**Dependencies:** Sprint 115.

#### [ ] Story 116.1 - Bounded Telemetry Identity and Correlation

**User-facing value:** As a developer or operator, I need telemetry linked to the exact service, release, deployment, artifact, and time window while AgentMage remains honest about uncertainty and causation.

##### Tasks and Sub-tasks

- [ ] **Task 116.1.1 - Implement telemetry contracts**
  - [ ] **Sub-task 116.1.1.1:** Define OpenTelemetry resource, service, environment, trace, span, metric, log, event, error, monitor, release, deployment, and time-window identities.
  - [ ] **Sub-task 116.1.1.2:** Implement bounded query plans, time/range/cardinality/result limits, sampling metadata, clock/skew handling, freshness, classification, retention, and cancellation.
  - [ ] **Sub-task 116.1.1.3:** Correlate telemetry with service/catalog, source, artifact, deployment, release, incident, and receipt through exact attributes or labeled inference.
  - [ ] **Sub-task 116.1.1.4:** Implement deterministic aggregation and statistical summaries with method, missingness, sampling, uncertainty, and source citations.
- [ ] **Task 116.1.2 - Enforce telemetry authority boundaries**
  - [ ] **Sub-task 116.1.2.1:** Treat attributes, messages, stack traces, links, and logs as untrusted and secret-scanned before persistence or model use.
  - [ ] **Sub-task 116.1.2.2:** Prevent telemetry, monitors, anomalies, and model interpretation from triggering rollback, deployment, notification, issue creation, or durable memory automatically.
  - [ ] **Sub-task 116.1.2.3:** Label temporal/structural correlation separately from deterministic causation and require evidence for any causal claim.
- [ ] **Task 116.1.3 - Verify and close the story**
  - [ ] **Sub-task 116.1.3.1:** `S-116-UT01` mutates telemetry identity, timestamps, resources, sampling, attributes, units, aggregation, missingness, and relationship fields.
  - [ ] **Sub-task 116.1.3.2:** `S-116-ST01` injects secrets, prompt attacks, cardinality explosions, malformed encodings, oversized payloads, clock skew, trace collisions, and false causal narratives.
  - [ ] **Sub-task 116.1.3.3:** `S-116-IT01` correlates a synthetic release/deployment with metrics/logs/traces/errors and an incident window, preserving source and uncertainty.
  - [ ] **Sub-task 116.1.3.4:** `S-116-RT01` exercises backend outage, partial data, sampling change, late arrival, duplicate spans, cancellation, full disk, and memory pressure.
  - [ ] **Sub-task 116.1.3.5:** Execute telemetry portions of `RV-27` and `RV-29`; retain query plans, raw/normalized data, canary scans, statistical methods, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 116.1.AC1:** Given telemetry from a release window, when AgentMage correlates it, then every observation retains exact service/environment/time/provider identity and every inferred relationship is labeled.
- [ ] **Story AC 116.1.AC2:** Given high-cardinality, malformed, secret-bearing, or injected telemetry, when processing runs, then limits, redaction, and untrusted-content controls prevent disclosure and unbounded use.
- [ ] **Story AC 116.1.AC3:** Given a temporal association between deployment and failure, when AgentMage explains it, then it does not claim causation without deterministic evidence.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 116.AC1:** OpenTelemetry identities and queries are deterministic, bounded, stale-aware, and source-cited.
- [ ] **Sprint AC 116.AC2:** Telemetry cannot create operation authority or durable memory automatically.
- [ ] **Sprint AC 116.AC3:** Secret, injection, malformed, clock, and cardinality attacks pass.
- [ ] **Sprint AC 116.AC4:** Statistical summaries expose method, missingness, sampling, and uncertainty.
- [ ] **Sprint AC 116.AC5:** The OpenTelemetry subset of `AT-OBS-001` passes.

**Gate decision:** Sprint 116 is PASS only when Story 116.1, all criteria, the OpenTelemetry subset of `AT-OBS-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 117 - Datadog and Multi-Vendor Observability

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Add Datadog, Prometheus/Grafana/Loki, Elastic, Splunk, and Sentry through the same bounded OpenTelemetry-centered observability contract.

**Source coverage:** `AM-OBS-001`, `AT-OBS-001`; `SR-DEL-*`, `RV-23`, `RV-24`, `RV-27`, `RV-29`.

**Dependencies:** Sprint 116.

#### [ ] Story 117.1 - Conformant Observability Adapters

**User-facing value:** As an operator, I need to compare and correlate telemetry across supported systems without leaking credentials, flattening vendor semantics, or making unsupported claims.

##### Tasks and Sub-tasks

- [ ] **Task 117.1.1 - Implement vendor adapters**
  - [ ] **Sub-task 117.1.1.1:** Implement Datadog metrics, logs, traces, monitors, dashboards, events, errors, services, releases, and incident links within the supported matrix.
  - [ ] **Sub-task 117.1.1.2:** Implement Prometheus query, Grafana dashboards/annotations, Loki logs, and exact datasource/tenant identity.
  - [ ] **Sub-task 117.1.1.3:** Implement Elastic search/observability, Splunk search, and Sentry project/issue/event/release reads with provider-specific fields retained.
  - [ ] **Sub-task 117.1.1.4:** Normalize common telemetry into OpenTelemetry identities and preserve non-equivalent vendor behavior as namespaced extensions.
- [ ] **Task 117.1.2 - Implement safe queries and diagnostics**
  - [ ] **Sub-task 117.1.2.1:** Preview provider, host, tenant/account, data scope, query, time window, cardinality/byte/cost bounds, retention, and expected result before activation.
  - [ ] **Sub-task 117.1.2.2:** Implement pagination/streaming, rate-limit, quota, cancellation, partial-result, freshness, and query-cost diagnostics without secret values.
  - [ ] **Sub-task 117.1.2.3:** Keep monitor edits, dashboard publication, incident changes, notification, and remediation outside observe authority.
- [ ] **Task 117.1.3 - Verify and close the story**
  - [ ] **Sub-task 117.1.3.1:** `S-117-CT01` runs all promoted vendor objects, versions, tenants, permissions, pagination, queries, and degradation modes.
  - [ ] **Sub-task 117.1.3.2:** `S-117-ST01` tests cross-tenant credentials, datasource confusion, query injection, secret-bearing logs, malicious links, cardinality explosion, and hidden write/remediation calls.
  - [ ] **Sub-task 117.1.3.3:** `S-117-IT01` correlates equivalent synthetic telemetry across all vendors with exact release/deployment identity and compares normalized results without erasing differences.
  - [ ] **Sub-task 117.1.3.4:** `S-117-RT01` injects version skew, outage, slow query, rate limit, quota exhaustion, partial results, late data, cancellation, and restart.
  - [ ] **Sub-task 117.1.3.5:** Retain support matrices, query traces, result comparisons, canary scans, resource metrics, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 117.1.AC1:** Given a supported observability provider, when a query runs, then exact tenant, query, time, limits, source identity, freshness, and partial-result state remain visible.
- [ ] **Story AC 117.1.AC2:** Given semantically different vendor fields, when normalization runs, then common meaning is preserved and non-equivalent behavior remains namespaced rather than guessed.
- [ ] **Story AC 117.1.AC3:** Given observe authority, when content or a model requests monitor edits, publication, incident change, notification, or remediation, then no stronger operation registers or executes.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 117.AC1:** Datadog, Prometheus/Grafana/Loki, Elastic, Splunk, and Sentry pass their published read matrices.
- [ ] **Sprint AC 117.AC2:** Cross-tenant and credential-confusion suites report zero crossover.
- [ ] **Sprint AC 117.AC3:** Query/resource bounds and cancellation remain effective under failure.
- [ ] **Sprint AC 117.AC4:** Vendor differences and degradation remain explicit.
- [ ] **Sprint AC 117.AC5:** Complete `AT-OBS-001` passes with independent evidence.

**Gate decision:** Sprint 117 is PASS only when Story 117.1, all criteria, `AT-OBS-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 118 - Incidents, Bounded Notifications, and ChatOps

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Correlate incidents and prepare or send separately approved notifications without allowing telemetry or incident content to trigger autonomous remediation.

**Source coverage:** `AM-INC-001`, `AT-INC-001`; `DELIVERY-SYSTEM.md` Section 10; `SR-DEL-*`, `SR-OPS-*`, `RV-21`, `RV-23` through `RV-29`.

**Dependencies:** Sprint 117; Sprint 107 work management and Sprint 115 rollback contracts.

#### [ ] Story 118.1 - Human-Controlled Incident Lifecycle

**User-facing value:** As an incident lead, I need evidence, drafts, notifications, work items, and rollback options correlated in one place while every external communication and remediation remains under human control.

##### Tasks and Sub-tasks

- [ ] **Task 118.1.1 - Implement incident and communication adapters**
  - [ ] **Sub-task 118.1.1.1:** Implement PagerDuty, Jira Service Management, and Datadog incident identities, services, responders, status, severity, timeline, evidence, links, notes, and permissions.
  - [ ] **Sub-task 118.1.1.2:** Implement Slack and Teams destination/channel/thread/member identities and local-only draft notifications with exact recipients, mentions, attachments, visibility, and disclosure warnings.
  - [ ] **Sub-task 118.1.1.3:** Correlate incidents with service/catalog, work, source, CI, artifact, deployment, telemetry, finding, release, rollback, and receipts through exact evidence.
  - [ ] **Sub-task 118.1.1.4:** Generate evidence-backed status, impact, hypothesis, action, owner, decision, and next-update drafts while separating fact, inference, conflict, and unknown.
- [ ] **Task 118.1.2 - Implement bounded incident effects**
  - [ ] **Sub-task 118.1.2.1:** Separately preview and grant incident state/severity/assignment, work-item creation, Slack/Teams message, deployment rollback, flag change, and closure.
  - [ ] **Sub-task 118.1.2.2:** Re-read incident, destination membership, work item, environment, release, and remediation preconditions before submission.
  - [ ] **Sub-task 118.1.2.3:** Implement duplicate-event/message/work prevention, uncertain-result reconciliation, correction/follow-up workflow, and complete audit receipts.
  - [ ] **Sub-task 118.1.2.4:** Keep auto-remediation, auto-page, auto-message, autonomous rollback, and content-triggered severity changes prohibited.
- [ ] **Task 118.1.3 - Verify and close the story**
  - [ ] **Sub-task 118.1.3.1:** `S-118-CT01` runs incident and notification provider matrices across identity, permissions, lifecycle states, recipients, threads, attachments, edits, and deletion.
  - [ ] **Sub-task 118.1.3.2:** `S-118-ST01` injects false telemetry, prompt attacks, hidden recipients, channel confusion, malicious attachments, cross-tenant incidents, urgency pressure, and nested remediation requests.
  - [ ] **Sub-task 118.1.3.3:** `S-118-IT01` correlates a synthetic failed release, drafts an incident and message, separately approves work creation and notification, then separately previews rollback.
  - [ ] **Sub-task 118.1.3.4:** `S-118-RT01` injects duplicate/delayed events, changed responders, destination membership changes, timeout, partial message, provider outage, cancellation, and crash.
  - [ ] **Sub-task 118.1.3.5:** Execute an expanded `RV-21` plus applicable `RV-24` through `RV-29`; retain timelines, evidence graphs, previews, message canary scans, effect traces, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 118.1.AC1:** Given an incident window, when AgentMage builds the record, then observed evidence, hypotheses, conflicts, decisions, and unknowns remain distinct and source-linked.
- [ ] **Story AC 118.1.AC2:** Given incident authority, when notification, work creation, rollback, flag, severity, assignment, or closure is requested, then each is previewed and approved as a separate external effect.
- [ ] **Story AC 118.1.AC3:** Given duplicate, delayed, injected, or partial incident/provider data, when recovery runs, then no autonomous or duplicate communication/remediation occurs.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 118.AC1:** Incident and notification providers pass their exact promoted matrices.
- [ ] **Sprint AC 118.AC2:** Zero hidden recipients, cross-tenant effects, secret disclosures, or autonomous remediation occur.
- [ ] **Sprint AC 118.AC3:** Duplicate and uncertain effects reconcile without duplicate pages, messages, work, or rollback.
- [ ] **Sprint AC 118.AC4:** Incident evidence distinguishes correlation, inference, conflict, and causation.
- [ ] **Sprint AC 118.AC5:** `AT-INC-001` and the expanded incident tabletop pass.

**Gate decision:** Sprint 118 is PASS only when Story 118.1, all criteria, `AT-INC-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 119 - Service Catalog and Ownership Graph

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Link Backstage services and owners to delivery evidence without treating catalog metadata as provider truth or operation authority.

**Source coverage:** `AM-CAT-001`, `AT-CAT-001`; `DELIVERY-SYSTEM.md` Sections 3 and 14; `SR-DEL-004`, `SR-DEL-011` through `SR-DEL-014`, `RV-23`, `RV-27`, `RV-29`.

**Dependencies:** Sprint 118.

#### [ ] Story 119.1 - Evidence-Backed Service Catalog

**User-facing value:** As a delivery lead, I need to navigate from a service and owner to its work, source, pipeline, artifact, environment, telemetry, incident, and release without guessing identity.

##### Tasks and Sub-tasks

- [ ] **Task 119.1.1 - Implement the Backstage reference adapter**
  - [ ] **Sub-task 119.1.1.1:** Implement catalog entity, kind, namespace, name, UID, owner, system, domain, component, API, resource, group, user, relation, annotation, location, lifecycle, and permission reads.
  - [ ] **Sub-task 119.1.1.2:** Resolve repository, CI, artifact, environment, observability, incident, documentation, and release links against exact provider identities rather than trusting catalog URLs alone.
  - [ ] **Sub-task 119.1.1.3:** Preserve conflicting owners, duplicate names, stale entities, missing targets, provider mismatch, and inferred links as explicit unresolved states.
  - [ ] **Sub-task 119.1.1.4:** Keep catalog descriptors and annotations untrusted and unable to register tools, credentials, plugins, provider hosts, or operations.
- [ ] **Task 119.1.2 - Define catalog extension conformance**
  - [ ] **Sub-task 119.1.2.1:** Define L0/L1 extension fixtures for Port, Cortex, and Compass without promoting live support.
  - [ ] **Sub-task 119.1.2.2:** Define owner/service identity mapping, namespaced extensions, support-matrix entries, and no-write removal requirements.
  - [ ] **Sub-task 119.1.2.3:** Add provider-catalog drift, rename, transfer, delete, and ownership-change diagnostics.
- [ ] **Task 119.1.3 - Verify and close the story**
  - [ ] **Sub-task 119.1.3.1:** `S-119-CT01` runs Backstage entities, relations, versions, permissions, pagination, location, rename, delete, and stale fixtures.
  - [ ] **Sub-task 119.1.3.2:** `S-119-ST01` tests duplicate names, forged URLs, malicious annotations, owner confusion, cross-tenant links, injected instructions, graph cycles, and plugin/tool registration attempts.
  - [ ] **Sub-task 119.1.3.3:** `S-119-IT01` builds a service-centered delivery graph and independently resolves each provider link and owner relation.
  - [ ] **Sub-task 119.1.3.4:** `S-119-RT01` changes ownership, transfers repositories, removes telemetry, deletes an entity, and makes catalog/provider evidence stale.
  - [ ] **Sub-task 119.1.3.5:** Retain catalog/provider snapshots, link resolutions, conflict corpus, extension manifests, removal scan, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 119.1.AC1:** Given a Backstage service, when AgentMage builds its delivery view, then every provider relationship resolves independently or remains visibly unresolved/inferred.
- [ ] **Story AC 119.1.AC2:** Given duplicate or conflicting identity and ownership, when correlation runs, then AgentMage does not guess, overwrite evidence, or grant authority.
- [ ] **Story AC 119.1.AC3:** Given a future catalog adapter, when only L0/L1 conformance exists, then no write operation or unsupported support claim registers.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 119.AC1:** Backstage passes the published read and relationship matrix.
- [ ] **Sprint AC 119.AC2:** Catalog/provider conflicts and staleness remain explicit.
- [ ] **Sprint AC 119.AC3:** Catalog content cannot register or broaden capability.
- [ ] **Sprint AC 119.AC4:** Port, Cortex, and Compass remain manifested extension candidates only.
- [ ] **Sprint AC 119.AC5:** `AT-CAT-001` passes with independent evidence.

**Gate decision:** Sprint 119 is PASS only when Story 119.1, all criteria, `AT-CAT-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 120 - Markdown and LaTeX Mathematics

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Support safe, source-preserving, accessible inline and display mathematics in Markdown across Fedora, Ubuntu, and Windows 11.

**Source coverage:** `AM-MTH-001`, `AT-MTH-001`; PRD Section 25; inventory Sections 16 and 36B; `SR-TST-002`, `SR-TST-004`, `SR-CIV-006` through `SR-CIV-009`, `RV-15`, `RV-20`.

**Dependencies:** Sprint 119; Sprints 57 and 60-65 document/visual foundations.

#### [ ] Story 120.1 - Safe Mathematical Authoring and Rendering

**User-facing value:** As a technical author, I need to write and review formulas in Markdown while preserving source, preventing executable LaTeX behavior, and producing accessible local previews.

##### Tasks and Sub-tasks

- [ ] **Task 120.1.1 - Implement mathematics parsing and preservation**
  - [ ] **Sub-task 120.1.1.1:** Define the supported inline/display delimiter, environment, command, macro, label, reference, escaping, Unicode, code-fence, and Markdown interaction subset.
  - [ ] **Sub-task 120.1.1.2:** Build a structured parser that preserves exact source spans, delimiters, whitespace where meaningful, labels, references, diagnostics, and unsupported syntax.
  - [ ] **Sub-task 120.1.1.3:** Implement round-trip edits and diffs that never reinterpret code fences, currency text, escaped delimiters, frontmatter, or ordinary Markdown as mathematics.
  - [ ] **Sub-task 120.1.1.4:** Reject shell escape, file input/output, network resources, package loading, executable extensions, unsafe links, recursive/unbounded macros, and unsupported commands.
- [ ] **Task 120.1.2 - Implement offline preview and accessibility**
  - [ ] **Sub-task 120.1.2.1:** Select, pin, admit, and package an offline renderer with no runtime download, remote font, remote asset, or executable extension.
  - [ ] **Sub-task 120.1.2.2:** Render deterministic native Chat previews with syntax errors, source links, copyable source, zoom/reflow, high contrast, keyboard navigation, and screen-reader text.
  - [ ] **Sub-task 120.1.2.3:** Implement bounded render time, macro depth, input/output size, cancellation, cache identity, cleanup, and cross-platform parity.
- [ ] **Task 120.1.3 - Verify and close the story**
  - [ ] **Sub-task 120.1.3.1:** `S-120-UT01` round-trips valid equations across supported syntax, Unicode, labels/references, lists, tables, quotes, links, code fences, frontmatter, and escaping.
  - [ ] **Sub-task 120.1.3.2:** `S-120-ST01` fuzzes malformed delimiters, nested environments, unsafe commands, path/file attempts, network URLs, package escapes, macro recursion, token bombs, and mixed Markdown attacks.
  - [ ] **Sub-task 120.1.3.3:** `S-120-VT01` renders the golden corpus on Fedora, Ubuntu, and Windows and compares structure, errors, accessibility tree, bounds, and approved visual tolerances.
  - [ ] **Sub-task 120.1.3.4:** `S-120-RT01` cancels, crashes, fills cache/disk, changes renderer identity, corrupts cache, and resumes; assert cleanup and no stale preview reuse.
  - [ ] **Sub-task 120.1.3.5:** Execute applicable `RV-15` and `RV-20`; retain parser corpus, fuzz seeds/shrinks, renderer manifest, visual/accessibility output, resource traces, and review.

##### Story Acceptance Criteria

- [ ] **Story AC 120.1.AC1:** Given supported mathematics in Markdown, when AgentMage reads, edits, and renders it, then source round-trips exactly except for explicitly previewed edits and references remain correct.
- [ ] **Story AC 120.1.AC2:** Given unsafe, malformed, recursive, oversized, or executable LaTeX content, when parsing/rendering runs, then it is rejected or bounded without file, network, process, package, or authority access.
- [ ] **Story AC 120.1.AC3:** Given the same document on Fedora, Ubuntu, and Windows, when previewed, then rendering, diagnostics, interaction, and accessibility satisfy the declared parity contract.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 120.AC1:** Supported Markdown mathematics round-trips source and exact locations.
- [ ] **Sprint AC 120.AC2:** Unsafe commands and every executable/resource escape path are denied.
- [ ] **Sprint AC 120.AC3:** Rendering is pinned, offline, deterministic, bounded, cancellable, and cache-safe.
- [ ] **Sprint AC 120.AC4:** Keyboard, screen-reader, zoom, contrast, and error workflows pass.
- [ ] **Sprint AC 120.AC5:** `AT-MTH-001` passes on all first-GA platforms.

**Gate decision:** Sprint 120 is PASS only when Story 120.1, all criteria, `AT-MTH-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 121 - Windows Package, Visual Studio Code Bridge, and IPC

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Deliver a signed per-user Windows 11 package and authenticated Visual Studio Code-to-kernel boundary without administrator or ambient authority.

**Source coverage:** `AM-WIN-001`, `AT-WIN-001`; `WINDOWS-BOUNDARIES.md` Sections 1-4 and 10-11; `SR-PLT-001`, `SR-PLT-005` through `SR-PLT-017`, `RV-01`, `RV-02`, `RV-05`.

**Dependencies:** Sprint 120; Sprints 7, 23, 96, and 98 shared platform/interface/update foundations.

#### [ ] Story 121.1 - Signed Windows Host and Authenticated IPC

**User-facing value:** As a Windows 11 user, I need AgentMage to install per user and appear in native Visual Studio Code Chat with every local peer and package identity verified.

##### Tasks and Sub-tasks

- [ ] **Task 121.1.1 - Implement Windows packaging and lifecycle**
  - [ ] **Sub-task 121.1.1.1:** Define the serviced Windows 11 x64, stable Visual Studio Code, compiler/SDK, package identity, certificate, timestamp, component hash, path, process, and dependency matrix.
  - [ ] **Sub-task 121.1.1.2:** Build reproducible per-user MSIX packaging, Authenticode signing, timestamp verification, extension installation, package diagnostics, and offline verification.
  - [ ] **Sub-task 121.1.1.3:** Implement standard-user install, launch, repair, upgrade, rollback, safe mode, uninstall, and residue inventory without services, drivers, scheduled tasks, system-wide writes, or policy weakening.
  - [ ] **Sub-task 121.1.1.4:** Publish signed process/file/path/pipe/package/hash/version indicators for endpoint reconciliation.
- [ ] **Task 121.1.2 - Implement Windows bridge and named-pipe IPC**
  - [ ] **Sub-task 121.1.2.1:** Build the minimal signed native bridge and access-controlled named pipe with exact current-user ACL and no workspace/model/credential authority.
  - [ ] **Sub-task 121.1.2.2:** Validate user SID, logon session, integrity level, executable identity, package identity, protocol version, message sequence, size, launch challenge, replay state, and cancellation.
  - [ ] **Sub-task 121.1.2.3:** Prevent handle leakage/inheritance, alternate pipe names, cross-user access, elevation confusion, binary replacement, downgrade, and undeclared listener creation.
  - [ ] **Sub-task 121.1.2.4:** Register the stable native Visual Studio Code Chat model provider and redacted diagnostics using the same kernel contracts as Linux.
- [ ] **Task 121.1.3 - Verify and close the story**
  - [ ] **Sub-task 121.1.3.1:** `S-121-AT01` performs three clean standard-user install/launch/repair/upgrade/rollback/uninstall lifecycles and reconciles package/component/residue manifests.
  - [ ] **Sub-task 121.1.3.2:** `S-121-ST01` attempts wrong-user, wrong-session, low/high-integrity, unsigned, replaced, stale, replaying, malformed, oversized, reordered, rapidly reconnecting, and inherited-handle clients.
  - [ ] **Sub-task 121.1.3.3:** `S-121-IT01` streams local Chat, tool progress, cancellation, evidence, errors, and diagnostics through authenticated IPC while tracing extension/bridge/kernel authority.
  - [ ] **Sub-task 121.1.3.4:** `S-121-RT01` interrupts install/update/rollback, crashes bridge/kernel, changes VS Code build, revokes package identity, and resumes; assert safe recovery.
  - [ ] **Sub-task 121.1.3.5:** Execute Windows portions of `RV-01`, `RV-02`, and `RV-05`; retain package/signature/timestamp output, IPC traces, process identities, lifecycle snapshots, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 121.1.AC1:** Given a clean standard-user Windows account, when AgentMage is installed and launched, then every component and location matches the signed manifest without requiring post-install administrator authority.
- [ ] **Story AC 121.1.AC2:** Given an undeclared, replaced, wrong-user, wrong-session, wrong-integrity, replaying, or malformed IPC peer, when it connects, then the connection is rejected before any protected operation or data disclosure.
- [ ] **Story AC 121.1.AC3:** Given native Visual Studio Code Chat, when the user interacts with AgentMage, then the extension and bridge remain authority-free and all work flows through authenticated kernel contracts.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 121.AC1:** Three clean Windows package lifecycles pass with exact residue accounting.
- [ ] **Sprint AC 121.AC2:** Signatures, timestamps, components, dependencies, and package identities reconcile independently.
- [ ] **Sprint AC 121.AC3:** All IPC impersonation/replay/malformed/resource attacks fail safely.
- [ ] **Sprint AC 121.AC4:** No service, driver, scheduled task, system-wide write, policy weakening, or undeclared listener exists.
- [ ] **Sprint AC 121.AC5:** Native Chat and diagnostics satisfy the shared platform contract.

**Gate decision:** Sprint 121 is PASS only when Story 121.1, all criteria, the applicable `AT-WIN-001` cases, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 122 - Windows Path, Sandbox, Keys, Model, and Connected Workers

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Complete the Windows workspace, tool, key, model, storage, and network boundaries and prove equivalence with shared AgentMage contracts.

**Source coverage:** `AM-WIN-001`, `AT-WIN-001`; `WINDOWS-BOUNDARIES.md` Sections 5-11; `SR-PLT-013` through `SR-PLT-017`, `RV-03`, `RV-04`, `RV-06`, `RV-08` through `RV-10`, `RV-16`, `RV-17`, `RV-20`, `RV-24`, `RV-30`.

**Dependencies:** Sprint 121; Sprints 6, 9-16, 22, and 104.

#### [ ] Story 122.1 - Complete Windows Runtime Boundary

**User-facing value:** As a Windows user, I need local files, secrets, model inference, and connected provider operations confined to the exact scopes I approve.

##### Tasks and Sub-tasks

- [ ] **Task 122.1.1 - Implement Windows workspace and workers**
  - [ ] **Sub-task 122.1.1.1:** Implement handle-relative NTFS path resolution and reject device, extended-length, volume, UNC, WebDAV, pipe, traversal, reserved-name, trailing-dot/space, alternate-stream, reparse, junction, symlink, mount, cloud-placeholder, case, Unicode, short-name, hard-link, rename, replace, and race escapes.
  - [ ] **Sub-task 122.1.1.2:** Implement fresh AppContainer or equivalently reviewed restricted-token tool workers with one workspace handle, one grant, bounded scratch, no network, Job Object limits, mitigations, descendant termination, and residue proof.
  - [ ] **Sub-task 122.1.1.3:** Deny ambient profile, adjacent directory, registry, environment, credential, clipboard, desktop, device, camera, microphone, process, job, and neighboring-user access.
- [ ] **Task 122.1.2 - Implement Windows keys, state, model, and network workers**
  - [ ] **Sub-task 122.1.2.1:** Protect the operational data key with DPAPI through a reviewed provider and use Credential Manager only through typed non-secret references.
  - [ ] **Sub-task 122.1.2.2:** Enforce a local fixed NTFS data root outside OneDrive, redirected profiles, remote shares, removable media, and cloud synchronization for strict-local persistence.
  - [ ] **Sub-task 122.1.2.3:** Package hash-pinned native signed `llama.cpp` with no listener, workspace, credential, grant, tool, or network authority and explicit CPU/GPU profile identity.
  - [ ] **Sub-task 122.1.2.4:** Implement separately confined provider workers with exact destination/account/capability/credential/byte/time scopes and complete removal.
- [ ] **Task 122.1.3 - Verify and close the story**
  - [ ] **Sub-task 122.1.3.1:** `S-122-ST01` runs at least 1,000 NTFS/path/race and 500 sandbox/ambient-access attacks with unique canaries and zero escape.
  - [ ] **Sub-task 122.1.3.2:** `S-122-NT01` runs a 60-minute strict-local model/tool/recovery workload with packet, DNS, socket, process, firewall, and listener observation; require zero outbound attempt/byte.
  - [ ] **Sub-task 122.1.3.3:** `S-122-ST02` injects key/credential canaries, unavailable DPAPI/Credential Manager, risky data roots, model substitution, hostile local services, redirects, proxies, and cross-adapter credentials.
  - [ ] **Sub-task 122.1.3.4:** `S-122-RT01` crashes/cancels every durable, tool, model, credential, and connected effect boundary; assert cleanup, no repeated completed operation, and accurate uncertainty.
  - [ ] **Sub-task 122.1.3.5:** `S-122-AT01` runs keyboard, screen-reader, zoom, reflow, high-contrast, focus, status, error, progress, cancellation, and generated-document accessibility.
  - [ ] **Sub-task 122.1.3.6:** Execute all applicable Windows reviewer protocols; retain attack corpora, network/process traces, canary scans, model/state manifests, accessibility output, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 122.1.AC1:** Given a selected NTFS workspace, when valid and adversarial paths race through tool execution, then valid files resolve by exact identity and no operation escapes the workspace.
- [ ] **Story AC 122.1.AC2:** Given strict-local Windows operation, when model, tools, recovery, and diagnostics run for 60 minutes, then AgentMage creates no outbound attempt or byte and no undeclared listener.
- [ ] **Story AC 122.1.AC3:** Given connected-provider authority, when a worker executes, then it can access only the exact destination, credential reference, account, capability, operation, and bounded data; removal leaves no connected authority.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 122.AC1:** At least 1,000 path/race and 500 sandbox attacks yield zero boundary escape.
- [ ] **Sprint AC 122.AC2:** DPAPI, Credential Manager, local data root, encryption, retention, and canary tests pass.
- [ ] **Sprint AC 122.AC3:** Native model and strict-local tools produce zero outbound attempt/byte and no listener.
- [ ] **Sprint AC 122.AC4:** Connected workers pass identity, credential, destination, cancellation, and removal tests.
- [ ] **Sprint AC 122.AC5:** Windows accessibility, recovery, and shared-contract parity pass.

**Gate decision:** Sprint 122 is PASS only when Story 122.1, all criteria, `AT-WIN-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 123 - Provider Version Skew, Failure, and Resource Campaign

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Break every promoted adapter under version drift, outage, hostile input, limits, concurrency, and resource pressure before release integration.

**Source coverage:** `AM-XTE-001`, `AT-XTE-001`; `DELIVERY-SYSTEM.md` Section 12; `SR-TST-*`, `SR-DEL-012` through `SR-DEL-014`, `RV-15`, `RV-16`, `RV-17`, `RV-23` through `RV-29`.

**Dependencies:** Sprint 122.

#### [ ] Story 123.1 - Extreme Adapter and Provider Resilience

**User-facing value:** As a user and reviewer, I need AgentMage to fail safely and truthfully when providers, networks, schemas, credentials, resources, or local processes behave badly.

##### Tasks and Sub-tasks

- [ ] **Task 123.1.1 - Build the extreme campaign harness**
  - [ ] **Sub-task 123.1.1.1:** Generate minimum/maximum/future/provider-changed API fixtures and compare registration, degradation, diagnostics, and support claims.
  - [ ] **Sub-task 123.1.1.2:** Compose rate limit, quota, revocation, permission reduction, outage, partition, latency, clock skew, eventual consistency, event flood, and pagination-loop schedules.
  - [ ] **Sub-task 123.1.1.3:** Compose malformed/oversized schemas, logs, archives, artifacts, telemetry cardinality, repositories, concurrent workers, disk, memory, CPU, GPU, and cancellation pressure.
  - [ ] **Sub-task 123.1.1.4:** Add deterministic seeds, shrinking, coverage, sanitizer support, raw-result schema, environment identity, and reproducible replay for every campaign.
- [ ] **Task 123.1.2 - Run cross-adapter adversarial tests**
  - [ ] **Sub-task 123.1.2.1:** Inject prompt attacks and secret canaries into every provider-controlled input class and cross-adapter handoff.
  - [ ] **Sub-task 123.1.2.2:** Execute cross-host/account/project/environment credential and object-confusion attacks while adapters run concurrently.
  - [ ] **Sub-task 123.1.2.3:** Crash/cancel before, during, and after every read, event, write, execute, deploy, reconciliation, persistence, and recovery transition.
  - [ ] **Sub-task 123.1.2.4:** Force every gate to fail, skip, stale, flake, quarantine, suppress, or lose reviewer evidence and assert the release remains blocked.
- [ ] **Task 123.1.3 - Verify and close the story**
  - [ ] **Sub-task 123.1.3.1:** `S-123-FT01` runs all provider/version/failure combinations against fake/fault and approved live synthetic environments.
  - [ ] **Sub-task 123.1.3.2:** `S-123-FZ01` fuzzes every adapter schema/parser/event/result and retains seeds, corpus, coverage, crashes, sanitizers, and shrinks.
  - [ ] **Sub-task 123.1.3.3:** `S-123-ST01` runs every hostile-content, credential-confusion, authority-escalation, hidden-effect, and false-completion fixture.
  - [ ] **Sub-task 123.1.3.4:** `S-123-RT01` runs prolonged concurrency/resource/partition/cancellation campaigns and verifies bounded cleanup and responsiveness.
  - [ ] **Sub-task 123.1.3.5:** Execute `RV-15` through `RV-17` and `RV-23` through `RV-29`; retain complete raw evidence and independent red-team review.

##### Story Acceptance Criteria

- [ ] **Story AC 123.1.AC1:** Given any unsupported or changed provider behavior, when conformance runs, then AgentMage refuses or enters only the declared degraded mode and never inherits a stale support claim.
- [ ] **Story AC 123.1.AC2:** Given provider/network/resource failure at any transition, when recovery runs, then authority remains bounded, completed effects are not repeated, uncertainty is explicit, and the product remains responsive or stops safely.
- [ ] **Story AC 123.1.AC3:** Given any failed, skipped, stale, flaky, quarantined, suppressed, unreconciled, or unreviewed blocking result, when summaries are generated, then the blocker remains visible and prevents gate closure.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 123.AC1:** Every promoted provider version passes supported, boundary, and out-of-matrix behavior.
- [ ] **Sprint AC 123.AC2:** All parsers and trust boundaries complete the declared fuzz campaign with no unresolved security failure.
- [ ] **Sprint AC 123.AC3:** Failure/resource campaigns cause no duplicate effect, leak, escalation, corruption, or false completion.
- [ ] **Sprint AC 123.AC4:** Raw results, seeds, coverage, environment, versions, failures, and review reproduce exactly.
- [ ] **Sprint AC 123.AC5:** Applicable `RV-15` through `RV-29` pass without hidden blocker.

**Gate decision:** Sprint 123 is PASS only when Story 123.1, all criteria, the resilience portions of `AT-XTE-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 124 - Adapter Removal and Strict-Local Restoration

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Prove that every connected capability can be removed independently and together while the strict-local product remains complete and unchanged.

**Source coverage:** `AM-ADP-001`, `AM-XTE-001`, `AT-ADP-001`, `AT-XTE-001`; `SR-DEL-010`, `RV-30`.

**Dependencies:** Sprint 123.

#### [ ] Story 124.1 - Complete Connected-Capability Removal

**User-facing value:** As a privacy-conscious user, I need to remove any or all delivery integrations and return to a provably strict-local product without losing my local workspace or unrelated state.

##### Tasks and Sub-tasks

- [ ] **Task 124.1.1 - Implement adapter removal and retention**
  - [ ] **Sub-task 124.1.1.1:** Define per-adapter inventory for credentials, cache records, graph nodes/edges, event cursors, webhooks, schedules, workers, processes, sockets, firewall policy, temporary files, logs, receipts, and retained evidence.
  - [ ] **Sub-task 124.1.1.2:** Preview remove, revoke, delete, retain, export, and inaccessible-remote actions without deleting user repositories, provider data, or neighboring adapters.
  - [ ] **Sub-task 124.1.1.3:** Implement ordered cancellation, webhook/event disablement, credential revocation/removal, worker/network deregistration, cache/retention cleanup, graph tombstones, and residue report.
  - [ ] **Sub-task 124.1.1.4:** Implement reinstall/reconnect with new identity and explicit import rather than silently reusing stale credentials, cache, approvals, or support state.
- [ ] **Task 124.1.2 - Restore and verify strict-local state**
  - [ ] **Sub-task 124.1.2.1:** Remove each adapter independently, remove each provider domain, and remove all connected packs together from clean and failure-interrupted states.
  - [ ] **Sub-task 124.1.2.2:** Inspect process, socket, network, credential, cache, database, file, registry, package, schedule, webhook, and temporary residue on Fedora, Ubuntu, and Windows.
  - [ ] **Sub-task 124.1.2.3:** Rerun complete strict-local tools, model, repository map, writes, coding, evidence, recovery, diagnostics, and 60-minute zero-egress suites.
- [ ] **Task 124.1.3 - Verify and close the story**
  - [ ] **Sub-task 124.1.3.1:** `S-124-UT01` validates removal plans against missing, duplicate, stale, partially removed, shared, retained, and legally held records.
  - [ ] **Sub-task 124.1.3.2:** `S-124-ST01` attempts post-removal tool registration, credential recovery, cache access, event receipt, background sync, scheduled action, socket use, and model/provider crossover.
  - [ ] **Sub-task 124.1.3.3:** `S-124-RT01` crashes/cancels every removal stage and resumes to a deterministic complete or visibly blocked state without harming unrelated data.
  - [ ] **Sub-task 124.1.3.4:** `S-124-AT01` runs strict-local acceptance on all first-GA platforms after each removal combination and complete removal.
  - [ ] **Sub-task 124.1.3.5:** Execute `RV-30`; retain before/after inventories, revocation evidence, residue scans, strict-local raw results, and independent review.

##### Story Acceptance Criteria

- [ ] **Story AC 124.1.AC1:** Given an installed adapter, when removal is approved, then its credentials, tools, events, workers, network scope, cache, schedules, and policy registrations are removed according to retention without harming user/provider or neighboring state.
- [ ] **Story AC 124.1.AC2:** Given interruption during removal, when recovery runs, then AgentMage reaches a deterministic complete removal or visible blocked state and cannot use partially removed authority.
- [ ] **Story AC 124.1.AC3:** Given all connected packs removed, when strict-local acceptance runs, then local behavior, privacy, storage, model, evidence, recovery, and zero-egress guarantees remain unchanged.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 124.AC1:** Every adapter passes independent and aggregate removal on Fedora, Ubuntu, and Windows.
- [ ] **Sprint AC 124.AC2:** Zero undeclared credential, event, process, socket, schedule, cache, or network authority remains.
- [ ] **Sprint AC 124.AC3:** Interrupted removal is recoverable and cannot damage unrelated data.
- [ ] **Sprint AC 124.AC4:** Strict-local acceptance and 60-minute zero-egress proof pass after removal.
- [ ] **Sprint AC 124.AC5:** `RV-30` passes with independent evidence.

**Gate decision:** Sprint 124 is PASS only when Story 124.1, all criteria, the removal portions of `AT-ADP-001` and `AT-XTE-001`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

### [ ] Sprint 125 - Cross-Provider Delivery and Windows Release Gates

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Prove complete work-to-release and incident-to-rollback workflows across the promoted provider matrix on Fedora, Ubuntu, and Windows before final GA assembly.

**Source coverage:** `AM-XTE-001`, `AM-WIN-001`, `AT-XTE-001`, `AT-WIN-001`; integrated behavior from all prior Section 36 owners; `SR-DEL-*`, `SR-PLT-013` through `SR-PLT-017`, `RV-01` through `RV-30`.

**Dependencies:** Sprint 124; all prior first-GA adapter and platform gates.

#### [ ] Story 125.1 - Integrated Delivery and Platform Closure

**User-facing value:** As a user and reviewer, I need the complete promoted system to work across real lifecycle boundaries and to fail closed under cross-system drift, attack, and recovery.

##### Tasks and Sub-tasks

- [ ] **Task 125.1.1 - Run complete lifecycle scenarios**
  - [ ] **Sub-task 125.1.1.1:** Run work-item-to-branch-to-change-to-review-to-CI-to-artifact-to-provenance-to-deployment-to-telemetry-to-release across each valid reference-provider path.
  - [ ] **Sub-task 125.1.1.2:** Run incident-to-evidence-to-communication-to-work-to-flag/migration/deployment rollback-to-health-to-closure with every external effect separately approved.
  - [ ] **Sub-task 125.1.1.3:** Run mixed-provider paths and verify exact identities, graph edges, credentials, authority classes, receipts, staleness, and support tuples at every handoff.
  - [ ] **Sub-task 125.1.1.4:** Repeat the complete workflows on Fedora, Ubuntu, and Windows through native Visual Studio Code Chat using only published operator steps.
- [ ] **Task 125.1.2 - Run integrated attack and recovery scenarios**
  - [ ] **Sub-task 125.1.2.1:** Inject content attacks, credential confusion, host redirect, event replay, branch movement, artifact swap, policy change, environment drift, telemetry forgery, and hidden recipient/effect across lifecycle boundaries.
  - [ ] **Sub-task 125.1.2.2:** Inject rate limits, partitions, provider outage, permission reduction, version skew, partial effect, cancellation, process crash, system restart, low resources, and interrupted rollback.
  - [ ] **Sub-task 125.1.2.3:** Compare exact effects, no-effects, unknowns, partials, duplicate prevention, rollback preservation, and false-completion outcomes across platforms/providers.
  - [ ] **Sub-task 125.1.2.4:** Remove connected packs after integrated execution and rerun strict-local plus residue verification.
- [ ] **Task 125.1.3 - Verify and close the epic gates**
  - [ ] **Sub-task 125.1.3.1:** `S-125-AT01` executes the complete lifecycle matrix and checks every provider/version/capability tuple against the signed support matrix.
  - [ ] **Sub-task 125.1.3.2:** `S-125-ST01` executes all integrated attacks with zero unauthorized access, disclosure, effect, execution, deployment, secret/admin action, duplicate, or false completion.
  - [ ] **Sub-task 125.1.3.3:** `S-125-RT01` executes all integrated failure/recovery schedules with current pre/post snapshots and no repeated completed operation.
  - [ ] **Sub-task 125.1.3.4:** `S-125-AT02` reruns the complete Windows gate and Linux parity gates against release-candidate packages.
  - [ ] **Sub-task 125.1.3.5:** Re-run applicable `RV-01` through `RV-30`; retain signed raw evidence, cross-system graph, platform manifests, support matrix, removal proof, and independent decisions for `G-DELIVERY` and `G-WINDOWS`.

##### Story Acceptance Criteria

- [ ] **Story AC 125.1.AC1:** Given any promoted reference-provider path, when a complete lifecycle runs, then every object, effect, artifact, environment, observation, incident, and release is exact, attributable, receipted, and support-matrix conformant.
- [ ] **Story AC 125.1.AC2:** Given attack, drift, failure, uncertainty, or resource pressure at any cross-system handoff, when recovery runs, then no authority crosses classes/domains, no completed effect repeats, and no false completion is reported.
- [ ] **Story AC 125.1.AC3:** Given the same supported workflow on Fedora, Ubuntu, and Windows, when release-candidate packages run, then shared contracts match and platform-specific evidence remains independent.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 125.AC1:** Every promoted work-to-release and incident-to-rollback provider path passes.
- [ ] **Sprint AC 125.AC2:** Integrated adversarial and recovery campaigns produce zero unauthorized or duplicate effect and zero hidden blocker.
- [ ] **Sprint AC 125.AC3:** Fedora, Ubuntu, and Windows release-candidate workflows pass independently.
- [ ] **Sprint AC 125.AC4:** Connected-pack removal restores strict-local behavior after complete lifecycle execution.
- [ ] **Sprint AC 125.AC5:** `G-DELIVERY` and `G-WINDOWS` close only with current signed independent evidence.

**Gate decision:** Sprint 125 is PASS only when Story 125.1, all criteria, `AT-XTE-001`, `AT-WIN-001`, `G-DELIVERY`, `G-WINDOWS`, and the Universal Story Definition of Done pass. Otherwise it is BLOCKED.

## [ ] Epic 11 - v1.0 GA Verification and Release Decision

### [ ] Sprint 126 - First Supported GA Evidence and Release Decision

**Planning unit:** Dependency-bounded sprint; no calendar estimate.

**Sprint goal:** Rebuild, independently reproduce, and sign the exact v1.0 GA release decision without hiding any unsupported capability, stale result, residual risk, or platform/provider limitation.

**Source coverage:** `AM-GA-001`, `AM-GAD-001`, `AT-GA-001`; entire README, PRD, inventory through Section 36B, implementation plan, security review, runtime/delivery/Windows boundaries, model policy, security policy, Decisions 0001-0008, and all Sprints 0-125.

**Dependencies:** Sprint 125; `G-FOUNDATION`, all internal milestone gates used by promoted scope, `G-LEGACY-CLOSURE`, `G-DELIVERY`, and `G-WINDOWS`. Apple Silicon `BLOCKED-MACOS` items are retained post-GA and are not `G-GA` dependencies under Decision 0008.

#### [ ] Story 126.1 - Truthful v1.0 GA Closure

**User-facing value:** As a user and reviewer, I need a release whose supported platforms, providers, operations, security, limitations, and evidence match the exact package I receive.

##### Tasks and Sub-tasks

- [ ] **Task 126.1.1 - Rebuild release scope and evidence**
  - [ ] **Sub-task 126.1.1.1:** Rebuild the complete requirement graph and prove every promoted requirement has current source, design, code, test, evidence, owner, support, and release linkage.
  - [ ] **Sub-task 126.1.1.2:** Generate signed Fedora, Ubuntu, Windows, model/runtime, component/process/path/socket, adapter, provider/version/capability, data-flow, retention, support, and exclusion manifests.
  - [ ] **Sub-task 126.1.1.3:** Regenerate source and binary SBOMs, cryptographic BOM, Model BOM, licenses, provenance, signatures, hashes, vulnerability dispositions, and support/end-of-support metadata.
  - [ ] **Sub-task 126.1.1.4:** Publish exact install, diagnostics, strict-local, connected-profile, credential, provider, capability, recovery, rollback, removal, limitation, accessibility, and troubleshooting documentation.
- [ ] **Task 126.1.2 - Independently rerun the complete release**
  - [ ] **Sub-task 126.1.2.1:** Perform three clean standard-user install/upgrade/rollback/uninstall lifecycles per first-GA platform using published instructions only.
  - [ ] **Sub-task 126.1.2.2:** Rerun every promoted provider conformance matrix and every `AT-*` first-GA threshold against exact release candidates and synthetic provider environments.
  - [ ] **Sub-task 126.1.2.3:** Rerun `RV-01` through `RV-30`, cross-provider lifecycles, strict-local removal, accessibility, performance, recovery, incident tabletop, and documentation checks.
  - [ ] **Sub-task 126.1.2.4:** Recompute every summary from raw evidence, validate staleness against all source/dependency/config/model/platform/provider manifests, and reconcile every failure, skip, suppression, quarantine, and reviewer finding.
- [ ] **Task 126.1.3 - Decide and sign v1.0 GA**
  - [ ] **Sub-task 126.1.3.1:** Produce the final provider/version/capability matrix with exact supported, degraded, unsupported, disabled, and post-GA states.
  - [ ] **Sub-task 126.1.3.2:** Produce the final risk, limitation, remediation, rollback, support, vulnerability, and release decision from raw evidence.
  - [ ] **Sub-task 126.1.3.3:** Force every release gate and support claim to fail in synthetic checks and prove packaging/publication cannot proceed.
  - [ ] **Sub-task 126.1.3.4:** Obtain independent reviewer signatures over the exact evidence index and user approval over the final release decision.
  - [ ] **Sub-task 126.1.3.5:** Sign and hash the release manifests, packages, evidence index, checksums, and public release notes only after every blocking gate is green.

##### Story Acceptance Criteria

- [ ] **Story AC 126.1.AC1:** Given the exact v1.0 release candidates, when independent reviewers follow published procedures, then platform, provider, strict-local, connected, security, recovery, accessibility, support, and removal results reproduce from raw evidence.
- [ ] **Story AC 126.1.AC2:** Given any failed, skipped, stale, unavailable, flaky, quarantined, suppressed, unreconciled, or unreviewed blocking result, when release status is computed, then `G-GA` remains blocked and no supported-release package is produced.
- [ ] **Story AC 126.1.AC3:** Given final release notes and support matrices, when compared with code, registrations, manifests, packages, and evidence, then every capability and limitation agrees exactly and Apple Silicon remains accurately labeled post-GA.

#### Sprint Acceptance Criteria

- [ ] **Sprint AC 126.AC1:** Every promoted requirement has current reproducible requirement-to-release traceability.
- [ ] **Sprint AC 126.AC2:** Fedora, Ubuntu, and Windows pass independent clean lifecycle, platform, accessibility, performance, recovery, and removal gates.
- [ ] **Sprint AC 126.AC3:** Every promoted provider/version/capability tuple passes its exact conformance and extreme tests; every unsupported operation passes negative tests.
- [ ] **Sprint AC 126.AC4:** Bills of materials, provenance, signatures, hashes, manifests, support matrices, documentation, and raw evidence reconcile exactly.
- [ ] **Sprint AC 126.AC5:** `G-GA` closes only after independent reproduction and explicit user approval with no hidden blocker.

**Gate decision:** Sprint 126 and `G-GA` are PASS only when Story 126.1, all criteria, `AT-GA-001`, every applicable `AT-*`, every `RV-01` through `RV-30`, and the Universal Story Definition of Done pass with current signed evidence. Otherwise they are BLOCKED.

## Legacy Traceability Appendices

The tables below retain the original `S-NNN` planning identifiers. Each is mapped in the sprint body to one or more new sequential stories and remains available for requirement provenance.

### v0.1 Stable Backlog Coverage

| Backlog ID | Primary implementation sprint | Blocking integration sprint |
|---|---|---|
| `AM-KRN-001` | `S-004` | `S-021` |
| `AM-CFG-001` | `S-003` | `S-021` |
| `AM-PLT-001` | `S-007` | `S-021` |
| `AM-SEC-001` | `S-008`, `S-009` | `S-021` |
| `AM-SEC-002` | `S-008`, `S-009`, `S-016` | `S-021` |
| `AM-NET-001` | `S-010` | `S-021` |
| `AM-PTH-001` | `S-006` | `S-021` |
| `AM-AUT-001` | `S-005` | `S-021` |
| `AM-DAT-001` | `S-011` | `S-021` |
| `AM-PRV-001` | `S-011` | `S-021` |
| `AM-MDL-001` | `S-013` | `S-021` |
| `AM-MDL-002` | `S-015` | `S-021` |
| `AM-MDL-003` | `S-014` | `S-021` |
| `AM-DIA-001` | `S-015` | `S-021` |
| `AM-TOL-001` | `S-016` | `S-021` |
| `AM-GIT-001` | `S-017` | `S-021` |
| `AM-INS-001` | `S-017` | `S-021` |
| `AM-REP-001` | `S-018` | `S-021` |
| `AM-EVD-001` | `S-019` | `S-021` |
| `AM-EVD-002` | `S-019` | `S-021` |
| `AM-HOF-001` | `S-021` | `S-021` |
| `AM-SES-001` | `S-020` | `S-021` |
| `AM-VSC-001` | `S-021` | `S-021` |
| `AM-VSC-002` | `S-021` | `S-021` |
| `AM-TST-001` | `S-002`, `S-021` | `S-021` |
| `AM-TST-002` | `S-002`, `S-021` | `S-021` |
| `AM-DOC-001` | `S-021` | `S-021` |

### Inventory Section Coverage

| Inventory area | Sprint coverage |
|---|---|
| Architecture, rules, backlog, release sequence | `S-000` through `S-004`, `S-075` |
| 1. Local Model and Runtime | `S-013` through `S-015`, `S-042`, `S-073` |
| 1A. Model Routing and Local Resource Management | `S-015`, `S-042`, `S-073` |
| 1B. Tiered Local Routing and Frontier Handoff | `S-042`, `S-044` through `S-046` |
| 2. Agent Orchestrator | `S-004`, `S-012`, `S-020` |
| 2A. Session Behavior | `S-012`, `S-020`, `S-021`, `S-041`, `S-063` |
| 2B. Reasoning and Verification | `S-012`, `S-037`, `S-042` |
| 2C. Agent Studio and Direction | `S-071`, `S-072`, `S-074` |
| 3. Work Packet Contract | `S-004`, `S-012` |
| 4. Tool Protocol and Registry | `S-004`, `S-016`, `S-034`, `S-065` |
| 5. Permissions and Safety | `S-005`, `S-006`, `S-029`, `S-032`, `S-067` through `S-074` |
| 5A. Platform Threat Model and Isolation | `S-007` through `S-010`, `S-073`, `S-074` |
| 5B. Strict Local Operation | `S-010`, `S-021`, `S-057`, `S-062`, `S-074` |
| 6. Workspace and Instructions | `S-017`, `S-018`, `S-028`, `S-064` |
| 7. Filesystem Tools | `S-016`, `S-029` through `S-032` |
| 7A. Repository Map | `S-018`, `S-021`, `S-036` |
| 8. Obsidian Vault | `S-023`, `S-028`, `S-031` |
| 8A. Human Knowledge Workspace | `S-022`, `S-023`, `S-028`, `S-031` |
| 9. Retrieval | `S-024`, `S-025`, `S-028` |
| 10. Operational State and Resume | `S-011`, `S-020`, `S-027`, `S-032` |
| 10A. Context Continuity | `S-020`, `S-027` |
| 10B. Rolling Memory | `S-026`, `S-073` |
| 10C. Conversation Library | `S-027`, `S-041`, `S-063` |
| 10D. Human-Readable Memory and Frontier | `S-026`, `S-044` through `S-046` |
| 10E. Classification and Retention | `S-011`, `S-026`, `S-027`, `S-032`, `S-073` |
| 11. Task Management | `S-028`, `S-047`, `S-048` |
| 11A. Executive Assistant | `S-047`, `S-056` |
| 11B. Secretary Operations | `S-048`, `S-056` |
| 12. Process Runner | `S-034`, `S-039`, `S-041` |
| 13. Git Tools | `S-017`, `S-035`, `S-040`, `S-067` |
| 13A. Remote Repositories and Worktrees | `S-035`, `S-061`, `S-070`, `S-072` |
| 13B. GitHub Integration | `S-057` through `S-062`, `S-067` |
| 14. Codebase Understanding | `S-036` through `S-040` |
| 14A. Deep Repository Comprehension | `S-036`, `S-061` |
| 14B. Coding Assistance | `S-029`, `S-037` through `S-040`, `S-043` |
| 15. Test and Validation | `S-039`, `S-040`, `S-043` |
| 16. Markdown and Plain Text | `S-031`, `S-049` |
| 17. Word Documents | `S-050`, `S-056` |
| 18. Portable Document Format | `S-051`, `S-056` |
| 19. Spreadsheet, CSV, and JSON | `S-052`, `S-056` |
| 19A. File-Type Priority | `S-053`, `S-054`, `S-056` |
| 20. Images and Visual Verification | `S-050` through `S-054`, `S-056`, `S-063` |
| 21. Database Tools | `S-011`, `S-055`, `S-068` |
| 22. Evidence Reconciliation | `S-019`, `S-024`, `S-055` |
| 23. Audit and Observability | `S-011`, `S-019`, `S-032`, `S-057`, `S-073` |
| 24. Queues and Jobs | `S-069`, `S-070`, `S-072` |
| 24A. Scheduled Work | `S-069`, `S-070`, `S-074` |
| 25. Browser Access | `S-066`, `S-074` |
| 25A. Public Research and Computer Use | `S-066`, `S-074` |
| 26. Connectors and MCP | `S-057`, `S-065`, `S-068`, `S-074` |
| 26A. Plugins and Hooks | `S-064`, `S-065`, `S-074` |
| 26B. Rich Artifacts | `S-050` through `S-054`, `S-056` |
| 27. Visual Studio Code | `S-021`, `S-041`, `S-074` |
| 27A. Local CLI | `S-041`, `S-043`, `S-074` |
| 27B. Desktop Application | `S-063`, `S-074` |
| 28. Workflow Skills | `S-028`, `S-043`, `S-047`, `S-048`, `S-061`, `S-064`, `S-066`, `S-071`, `S-072` |
| 29. Dependencies | `S-001`, `S-003`, `S-007`, `S-050` through `S-054`, `S-073` |
| 30. Configuration | `S-003`, `S-041`, `S-064`, `S-073` |
| 31. Fixtures and Acceptance | `S-002` and every release gate sprint |
| 31A. Capability Evaluation | `S-002`, `S-015`, `S-021`, `S-042`, `S-074` |
| 31B. Quantitative v0.1 Matrix | `S-021` |
| 31C. Design Closure | `S-000`, `S-021`, `S-075` |
| 32. Operating Guides | Every release gate sprint, `S-073`, `S-075` |
| 33. Build Order | `S-004` through `S-021` |
| 34. v0.1 Completion | `S-021` |
| 35. Competitive Register | All mapped `CR-*` sprints and `S-075` |
| 35A. Rejected Defaults | Every security gate and `S-075` |
| 35B. Competitive Release Additions | Corresponding release gate sprint |
| 35C. Additions-Only Rule | `S-000`, `S-075` |
| 36. First-GA Delivery and Windows Backlog | Sprints 103-126 |
| 36A. First-GA Quantitative Matrix | Owning Sprint 103-124 and integrated Sprints 125-126 |
| 36B. Delivery-System Checklist | Sprints 103-126 |

### First-GA Stable Backlog Coverage

| Backlog ID | Primary sprint | Integrated gate |
|---|---|---|
| `AM-GA-001` | Sprint 103 | Sprint 126 |
| `AM-DEL-001` | Sprint 103 | Sprints 125-126 |
| `AM-ADP-001` | Sprints 103 and 105 | Sprints 124-126 |
| `AM-IDN-001` | Sprint 104 | Sprints 123-126 |
| `AM-GHE-001` | Sprint 106 | Sprints 125-126 |
| `AM-WRK-001` | Sprint 107 | Sprints 125-126 |
| `AM-SRC-001` | Sprint 108 | Sprints 125-126 |
| `AM-CIC-001` | Sprint 109 | Sprints 125-126 |
| `AM-ART-001` | Sprint 110 | Sprints 125-126 |
| `AM-SUP-014` | Sprint 111 | Sprints 125-126 |
| `AM-DEP-001` | Sprints 112-113 | Sprints 125-126 |
| `AM-IAC-001` | Sprint 114 | Sprints 125-126 |
| `AM-REL-001` | Sprint 115 | Sprints 125-126 |
| `AM-OBS-001` | Sprints 116-117 | Sprints 125-126 |
| `AM-INC-001` | Sprint 118 | Sprints 125-126 |
| `AM-SEC-003` | Sprint 111 | Sprints 125-126 |
| `AM-CAT-001` | Sprint 119 | Sprints 125-126 |
| `AM-MTH-001` | Sprint 120 | Sprint 126 |
| `AM-WIN-001` | Sprints 121-122 | Sprints 125-126 |
| `AM-XTE-001` | Sprints 123-125 | Sprint 126 |
| `AM-GAD-001` | Sprint 126 | Sprint 126 |

## Sprint Completion Record Template

```markdown
# Sprint Completion - Sprint N

## Scope

- Sprint goal:
- Story IDs:
- Task and sub-task IDs:
- Legacy requirement and issue IDs:
- Security requirements (`SR-*`):
- Dependencies verified:
- Explicit exclusions:
- Threat cases:

## Delivered

- Reviewed commit and release-manifest identity:
- Code and packages:
- Schemas and migrations:
- Fixtures:
- Documentation:
- Recovery changes:

## Verification

- Story acceptance criteria passed:
- Sprint acceptance criteria passed:
- Issue-local unit cases passed:
- Named test IDs passed:
- Tests failed:
- Tests skipped, unavailable, flaky, quarantined, or suppressed:
- Platform results:
- Security, privacy, records, and accessibility results:
- Raw evidence root:
- Raw-to-summary reconciliation result:
- Evidence-bundle hash and integrity verification:
- Independent reviewer, reviewed commit, findings, and dispositions:

## Decision

- Gate status: `PASS` or `BLOCKED`
- Blocking story, task, sub-task, criterion, or dependency:
- Known limitations:
- Deferred items:
- Rollback point:
- Next eligible sprint:
- Approver:
```
