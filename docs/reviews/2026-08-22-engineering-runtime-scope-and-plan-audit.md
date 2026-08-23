# Engineering Runtime Scope and Plan Audit

| Field | Value |
|---|---|
| Audit date | 2026-08-22 |
| Audit posture | Adversarial repository-wide review performed after the primary reconciliation pass |
| Baseline branch | `claude/sprint-41-verification` |
| Baseline commit | `835049c9e943dade2f1d523708e84ee7e4a3d5d0` |
| Scope decisions | 0043 and 0044 |
| Final disposition | **PASS WITH RECORDED LIMITATION** |
| Product-status effect | None; this is planning and contract evidence, not implementation, platform, model, endpoint, workflow, package, or release qualification |

## 1. Independence, Method, and Claim Boundary

This pass re-read the reconciled repository from a reviewer posture, traced every requested
capability from normative authority through requirements, tests, controls, milestones, stories,
tasks, schemas, and registries, attacked the authority boundaries described in the instruction,
and then repaired the findings recorded below. It is independent in sequence and review posture,
not a claim that an outside reviewer performed the work.

`PASS` in this report means the planning or contract surface is internally complete and locally
validated. It never means the planned product behavior has been implemented or qualified. Live
model, endpoint, platform, package, accessibility, performance, network, credential, and external
review evidence remains open until the owning `TASKS.md` gate is actually executed.

The local instruction input at repository root was present before the assignment, remains
untracked and byte-unchanged, and is excluded by exact path from repository documentation scans.
It is not a normative AgentMage document.

## 2. Audit Pass A: Scope Completeness

Every cell below is explicit. `F3` is the added Engineering Runtime foundational epic and `F4` is
the added Model Gateway foundational epic. Every listed story has three numbered tasks; together
the complete addition contains 20 stories, 60 tasks, and 141 sub-tasks.

| Capability | Normative authority | Requirements and acceptance tests | Security | Milestone, epic, sprint, story, tasks | Schemas and machine records | Result |
|---|---|---|---|---|---|---|
| Canonical Engineering Runtime | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-ERT-001`; `AT-ERT-001` | `SR-ERT-001` to `SR-ERT-003`; `RV-50` | `ER-M1`; F3; Sprint 1; Story 1.3; Tasks 1.3.1 to 1.3.3 | Workflow definition/state plus reused runtime-event schemas; module, dependency, build, and change-manifest records | PASS |
| Artifact ingestion and context fidelity | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-CTX-001` to `AM-CTX-003`; `AT-CTX-001` to `AT-CTX-004` | `SR-CTX-001` to `SR-CTX-006`; `RV-51` | `ER-M1`; F3; Sprints 1, 2, 16, 22; Stories 1.3, 2.4, 16.4, 22.5 | Artifact envelope/transformation/ingestion and context manifest/delivery schemas; artifact policy and change manifest | PASS |
| Verified workflow and deterministic completion | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-WKF-001`, `AM-WKF-002`, `AM-VER-001`; `AT-WKF-001` to `AT-WKF-003`, `AT-VER-001` | `SR-ERT-002` to `SR-ERT-006`; `RV-50`, `RV-52` | `ER-M2`; F3; Sprints 5, 11, 16, 21, 22; Stories 5.3, 11.3, 16.4, 21.4, 22.5 | Workflow, checkpoint, verification, terminal-result schemas; status and dependency records | PASS |
| Persistent task and resume | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-SES-002`; `AT-RESUME-002` | `SR-ERT-004`, `SR-ERT-005`; `RV-52` | `ER-M2`; F3; Sprint 11; Story 11.3; Tasks 11.3.1 to 11.3.3 | Workflow state/checkpoint schemas; status and change manifest | PASS |
| ToolObservation and execution lineage | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-TIO-001`; `AT-TIO-001`, `AT-TIO-002` | `SR-ERT-005`, `SR-ERT-006`; `RV-52` | `ER-M2`; F3; Sprints 16, 21; Stories 16.4, 21.4; Tasks 16.4.1 to 16.4.3 and 21.4.1 to 21.4.3 | Tool-observation and terminal-result schemas plus reused tool-call/runtime-event schemas | PASS |
| Local Model Gateway | Decision 0044; `MODEL-GATEWAY.md` | `AM-GWY-001` to `AM-GWY-003`; `AT-GWY-001` to `AT-GWY-003` | `SR-GWY-001` to `SR-GWY-006`; `RV-53` | `ER-M3`; F4; Sprints 13, 49, 50; Stories 13.5, 13.6, 49.2, 50.4 | Endpoint-profile and route-decision schemas plus six reused model/runtime schemas; profile catalog and activation policy | PASS |
| Optional remote inference | Decision 0044; `MODEL-GATEWAY.md` | `AM-REM-001` to `AM-REM-003`; `AT-REM-001` to `AT-REM-003` | `SR-GWY-005` to `SR-GWY-010`; `RV-53`, `RV-54` | `ER-M6`, `ER-M7`; F4; Sprints 123, 124; Stories 123.2, 124.2 | Endpoint and route schemas; permission-bearing values, optional components, platform lanes, activation policy | PASS WITH RECORDED LIMITATION: no live endpoint qualified |
| Observability, degradation, and performance | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-OBS-002`, `AM-DEG-001`, `AM-PERF-001`; `AT-OBS-002`, `AT-DEG-001`, `AT-PERF-002`, `AT-PERF-003` | `SR-ERT-005`, `SR-ERT-006`, `SR-GWY-004`, `SR-VSC-004`; `RV-50`, `RV-52` | `ER-M2`, `ER-M9`; F3; Sprints 21, 50; Stories 21.4, 50.4 | Runtime-event, observation, workflow-state, terminal-result, capability schemas; status model | PASS |
| AgentMage Verified Chat | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-VSC-004`; `AT-VSC-004` | `SR-VSC-001`, `SR-VSC-002`; `RV-55` | `ER-M4`; F3; Sprints 23, 50, 121; Stories 23.7, 50.4, 121.2 | Runtime-event, workflow-state, terminal-result schemas; VS Code module and platform-lane records | PASS WITH RECORDED LIMITATION: target architecture only |
| Native VS Code Chat compatibility | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-VSC-005`, `AM-VSC-006`; `AT-VSC-005`, `AT-VSC-006` | `SR-VSC-002` to `SR-VSC-004`; `RV-55` | `ER-M4`; F3; Sprints 23, 121; Stories 23.8, 121.2 | Artifact/context/runtime and reused model protocol schemas; module and optional-component records | PASS WITH RECORDED LIMITATION: native version matrix not executed |
| Engineering Capability Registry | Decision 0043; `ENGINEERING-CAPABILITY-REGISTRY.md` | `AM-CAP-001`, `AM-CAP-002`; `AT-CAP-001`, `AT-CAP-002` | `SR-CAP-001` to `SR-CAP-004`; `RV-56` | `ER-M5`; F3; Sprints 95, 125; Stories 95.3, 125.3 | Capability-manifest, workflow, state, verification schemas; capability and optional-component registries | PASS |
| Bounded multi-agent execution | Decision 0043; `ENGINEERING-RUNTIME.md` | `AM-MAG-001`; `AT-MAG-001`, `AT-MAG-002` | `SR-MAG-001` to `SR-MAG-004`; `RV-57` | `ER-M8`; F3; Sprints 95, 125; Stories 95.4, 125.3 | Capability/workflow/state schemas; task graph, dependency classes, permission values | PASS WITH RECORDED LIMITATION: blocked behind single-agent reliability gates by design |
| Integrated qualification and release reconciliation | Decisions 0043, 0044; all three new normative subsystem documents | All 24 added `AM-*` and 29 added `AT-*` IDs | `RV-50` to `RV-57` and all 34 added `SR-*` controls | `ER-M9`; F3/F4; Sprints 125, 126; Stories 125.3, 126.2 | Change manifest, traceability, status, platform, activation, policy, schema, and evidence registries | PASS WITH RECORDED LIMITATION: live and external gates remain open |

## 3. Finding Register and Repairs

| ID | Severity | Finding | Repair | Verification | Residual limitation |
|---|---|---|---|---|---|
| `ER-AUD-001` | Medium | The instruction's illustrative decision and schema paths could conflict with accepted repository numbering and the canonical `schemas/` root. | Used next legal Decisions 0043/0044 and canonical `schemas/engineering-runtime/`; validators bind both. | Architecture, schema, additions-only, and documentation checks. | None. |
| `ER-AUD-002` | High | Runtime, bridge, webview, model, tool, endpoint, verifier, and capability authority could become ambiguous across separate documents. | Decisions 0043/0044 and the three normative subsystem documents assign one Rust host authority, an authority-free TypeScript bridge/webview, proposal-only models, verifier-only completion, and profile-bound routes. | Requirement mappings, dependency rules, module inventory, effect-boundary and strict-local checks. | Product implementation remains future work. |
| `ER-AUD-003` | Medium | Initial prose counts did not cover all requirements, tests, stories, tasks, sub-tasks, schemas, epics, and unchanged roadmap totals in one reproducible record. | Rebuilt registries and added an exact change manifest with 24 requirements, 29 tests, 20 stories, 60 tasks, 141 sub-tasks, 14 new schemas, 6 reused schemas, two added foundational epics, zero release epics, and zero sprints. | Manifest unit/mutation tests, planning-scope, task-graph, additions-only, coverage, and documentation validators. | None. |
| `ER-AUD-004` | Medium | The first change-manifest draft called its 14 direct requirement-owner stories simply `stories`, undercounting the six qualification and reconciliation stories. | Renamed that count to `requirement_owner_stories`, added the complete ordered 20-story set and task/sub-task totals, and added a narrowing mutation test. | `engineering-runtime:manifest:check` reports 20 stories, 60 tasks, and 141 sub-tasks. | None. |
| `ER-AUD-005` | Medium | Existing validators assumed the prior document set, reviewer-protocol ceiling, and task-graph fixtures. The local instruction input could also be mistaken for product documentation. | Extended exact required-document, `RV-01` to `RV-57`, planning-scope, task-graph, schema, and lint rules; excluded only the exact local instruction filename. | Documentation, task-graph, schema, planning-scope, and mutation checks. | The untracked instruction input remains outside repository authority. |
| `ER-AUD-006` | Medium | Current lockfile and package-metadata drift made locked-resolution, component, supply-chain, artifact-scan, dependency-injection, and generated security records stale. | Regenerated current derived records, expanded approved transitive license expressions, recorded Git as a build tool, and updated the 92-edge dependency campaign without changing historical claims. | Resolution, supply-chain, component, artifact, dependency, schema, and focused evidence checks. | Regeneration is current-working-tree evidence until committed. |
| `ER-AUD-007` | High | Adjacent synthetic secret-detector marker literals could be concatenated by source scanning and reported as a credential-shaped false positive. | Corrected the scanner expression boundary and added a regression test while retaining all genuine credential detectors. | Artifact-scanner unit and canonical scans. | Detection is heuristic and does not replace release secret review. |
| `ER-AUD-008` | High | New endpoint and route fields could appear active, widen permissions, or promote platform support merely by being planned. | Added ten `PLANNED-NOT-ACTIVE` permission records with deny defaults, expiry, and audit rules; platform records remain planning-only; activation remains zero-model and no route is enabled. | Permission mutation, platform-gate, activation, status, policy, and documentation checks. | Live qualification remains open. |
| `ER-AUD-009` | High | Rebuilding current model policy could invalidate or overwrite accepted historical Gemma evidence. | Kept historical Gemma records byte-for-byte and taught admission/disposition validators to verify their recorded historical policy revision and hash separately from current policy. | Model source, disposition, fallback, and Story 0.3 security checks. | Historical rejection remains historical; no model is enabled. |
| `ER-AUD-010` | Medium | Full historical and clean-HEAD suites contain hash-bound kernel, grant, path, fuzz, configuration, support, platform, package, runtime-index, and release evidence that cannot truthfully be renewed from an uncommitted planning tree or without a new independent/manual review. | Left the affected gates blocked and documented exact ownership rather than rewriting reviewer evidence or promoting status. | Clean-traceability and the full historical suite reproduce the broader stale-evidence closure; historical replay passes current deterministic gates through fuzz policy and stops at the preserved kernel-contract independent reference. | Requires a later authorized commit and fresh owning evidence or independent/manual re-review as applicable. |
| `ER-AUD-011` | Low | Two security-evidence mutation tests changed only the first occurrence of an expanded control ID, leaving another valid occurrence and no longer exercising the intended deletion attack. | Mutate every occurrence of the selected control ID while leaving production documents and evidence unchanged. | Eight focused Story 12.1/16.1 tests pass. | None. |

## 4. Audit Pass B: Authority and Security

| Adversarial path | Result | Enforced planning boundary |
|---|---|---|
| Webview gains authority or canonical state | PASS | Webview is disposable projection only; Rust host owns state, policy, effects, and recovery. |
| TypeScript becomes a second kernel | PASS | Bridge is transport/presentation only and cannot mint grants, invoke unadmitted effects, or establish completion. |
| Model reaches tools or workspace directly | PASS | Models emit closed proposals; kernel admission and exact grants mediate every tool; no workspace handle is model-visible. |
| Remote model receives credentials | PASS | Credentials are opaque references resolved only inside the exact remote worker and excluded from model-visible context and observations. |
| Endpoint changes route, fallback, or completion | PASS | Endpoint, model, runtime, codec, and route profiles are separate; routing is deterministic and receipted; verifier owns completion. |
| Silent cloud fallback | PASS | `automatic_fallback` is false, local-to-remote fallback is disabled by default, and every alternate route requires explicit policy and disclosure. |
| Parser executes hostile content | PASS | Acquisition, passive parsing, transformation, context admission, and execution are separate; unsupported or hostile content receives a terminal disposition. |
| Tool output bypasses classification | PASS | Complete stdout/stderr are artifact-backed, separately identified, classified, bounded for projection, and admitted before model visibility. |
| Multi-agent scheduler bypasses leases | PASS | Multi-agent is downstream of the single-agent spine and requires separate worktrees, branches, path/test leases, bounded capacity, serialized integration, and re-verification. |
| Capability manifest widens authority | PASS | Manifest declares requirements but cannot mint grants, add tools, alter route/data policy, remove verification, or redefine completion. |
| Observability stores secrets or widens authority | PASS | Event payloads are closed, redacted, content-addressed where needed, and authority-free. |
| Retry duplicates a mutation | PASS | Retry classes are explicit; uncertain effects reconcile first; consumed grants and completed effects are not replayed. |
| UI approval survives state drift | PASS | Approval binds exact request, preview, policy, target, route, and current-state identities and expires on drift. |
| Strict-local accidentally requires cloud | PASS | Strict-local is a complete target profile; all remote components are optional and disabled. |

## 5. Audit Pass C: Planning Executability

| Check | Result | Evidence |
|---|---|---|
| No circular or impossible forward dependency | PASS | Task-graph and dependency-class validators pass. |
| No mega-sprint or vague feature-only story | PASS | Work is inserted into 20 existing owning stories with three tasks and six to eight sub-tasks each. |
| No orphan requirement, test, control, or protocol | PASS | Registry, coverage, traceability, and change-manifest checks close all 24/29 additions and `RV-50` to `RV-57`. |
| Every task has validation and every gate has evidence | PASS | Each added story names focused, mutation, integration, security, and evidence work; milestone table names gate evidence. |
| Live qualification is separate from implementation | PASS | Local/remote model, endpoint, platform, package, accessibility, and performance qualifications remain distinct open work. |
| Critical path is visible | PASS | `ER-M0` through `ER-M9` order governance, context, reliable single-agent execution, gateway, Chat, capabilities, remote routes, multi-agent, and release reconciliation. |
| Parallel work is bounded | PASS | Module/path ownership, dependency classes, worktree/lease rules, and serialized integration prevent overlapping uncoordinated effects. |
| Existing completed work is preserved | PASS | No accepted checkbox or ID was removed, renumbered, or promoted; additions occur in incomplete owning sprints. |

## 6. Audit Pass D: Status Truth

| Dimension | Current truth after reconciliation | Result |
|---|---|---|
| Product lifecycle | `scaffolded` | PASS |
| Integrated workflows | None | PASS |
| Enabled models | None | PASS |
| Enabled endpoints/routes/fallbacks | None | PASS |
| Supported platforms | None | PASS |
| New requirements/stories | Planned and unchecked | PASS |
| Release disposition | Pre-alpha; no supported binary or signed-release claim | PASS |

## 7. Audit Pass E: Local and Remote Inference

| Profile class | Target use | Planned adapters | Default state | Result |
|---|---|---|---|---|
| `strict_local` | Complete offline target | Native `llama.cpp`; optional separately qualified local adapters | Disabled until an exact tuple qualifies; no remote dependency | PASS |
| `local_network_private` | User-controlled private network inference | Qualified OpenAI-compatible or native protocol adapter | Disabled; explicit host/TLS/disclosure policy required | PASS WITH RECORDED LIMITATION |
| `remote_private` | Private remote worker or service | Qualified SGLang, TGI, Ray Serve LLM, KServe, or compatible adapter | Disabled; exact operator, credential reference, region, retention, quota, and route required | PASS WITH RECORDED LIMITATION |
| `remote_managed` | Managed external inference | Provider-neutral qualified adapter | Disabled; explicit operator/account/region/logging/training/cost disclosure required | PASS WITH RECORDED LIMITATION |

Technical compatibility is never treated as approval. Open-weight status is recorded separately from
license status. Publisher and endpoint operator are separate identities. HTTPS is mandatory for
non-loopback routes; TLS identity, optional mTLS, exact hosts, deny-first redirects, DNS/SSRF/proxy
controls, quotas, cost ceilings, emergency disablement, retention, logging, and training-use policy
are planned. No live endpoint or model tuple was exercised by this assignment.

## 8. Audit Pass F: VS Code Resilience

| Check | Result | Evidence or limitation |
|---|---|---|
| Stable production APIs only | PASS | Proposed APIs are confined to research; production target uses supported Chat, language-model-provider, webview, and native editor surfaces. |
| Rust state survives view disposal/reconnect | PASS | Host owns journals, checkpoints, artifacts, task state, and replay cursor; the webview reconstructs from events. |
| Verified Chat is canonical | PASS | AgentMage controls context, workflow, tools, persistence, verification, and terminal state. |
| Native Chat is compatibility only | PASS | `@agentmage` and Language Model Chat Provider paths disclose weaker original-reference guarantees and fail visibly on unresolved required content. |
| Normalized parts are preserved | PASS | Provider contract retains supported normalized message parts and rejects obvious unresolved attachment tokens. |
| Native surfaces are reused | PASS | Editor, diff, terminal, Problems, Testing, Source Control, notebook, and Explorer remain VS Code-owned. |
| Version and platform qualification | PASS WITH RECORDED LIMITATION | Stable-previous/current/Insiders, known-bad disablement, remote workspace placement, Windows, and retained macOS evidence are planned but not executed here. |
| Future Agent Host adapter is decoupled | PASS | Future registration work is an optional adapter point and cannot become a release dependency before a stable API exists. |

## 9. Audit Pass G: Diff and Generated-Artifact Review

The audit reviewed the complete changed/new path list, generated records, JSON formatting, schema
closure, Markdown links, Mermaid blocks, requirement counts, task identities, public reference
allowlist, lockfile-derived supply-chain state, model-policy history, permission defaults, platform
states, and exact local-instruction exclusion. Repository scans found no committed raw credential,
token, private key, private endpoint value, or private URL. The assignment introduced no dependency,
model artifact, endpoint, route, package, release, external effect, commit, or push.

Result: **PASS WITH RECORDED LIMITATION**. The working tree is intentionally uncommitted, so a
clean-HEAD archive cannot yet include these changes. Historical independent-review records were not
rewritten to manufacture freshness.

## 10. Appendix E Disposition

| Appendix | Result | Evidence and recorded limitation |
|---|---|---|
| E.1 Requirements and traceability | PASS | 24 unique requirements, 29 owned acceptance tests, controls, milestones, stories, tasks, schemas, and evidence targets validate; totals agree at 294 requirements and 31 normative mappings. |
| E.2 Architecture boundaries | PASS | Rust host, bridge, model, tool, gateway, endpoint, capability, observability, and credential boundaries are closed and deny-first in normative contracts. |
| E.3 Context fidelity | PASS WITH RECORDED LIMITATION | Capture, sentinel/range identity, provenance, warnings, dispositions, admission, omission, reconstruction, and required-unseen blocking are specified and schema-bound; full adapters are not implemented by this pass. |
| E.4 Workflow and persistence | PASS WITH RECORDED LIMITATION | Closed transitions, retry/reconciliation, grants, checkpoints, restart, no-progress ceilings, and evidence-current completion are planned; the complete supervisor remains future implementation. |
| E.5 Tool observations | PASS WITH RECORDED LIMITATION | One terminal observation, separate stream identities, artifacts, truncation disclosure, descendants, correlation, and uncertainty are specified; broad live-tool evidence is future work. |
| E.6 Local and remote gateway | PASS WITH RECORDED LIMITATION | Profiles are separated, routes are receipted, capabilities are tested, and fallback is explicit; no local or remote tuple was qualified here. |
| E.7 Remote inference security | PASS WITH RECORDED LIMITATION | Network, TLS, mTLS, redirects, DNS, SSRF, proxy, host, credential, region, retention, logging, training, quota, cost, failure, and disablement controls are planned; no private or managed route was exercised. |
| E.8 VS Code integration | PASS WITH RECORDED LIMITATION | Verified/native boundaries, attachment protocol, stable APIs, native surfaces, and version lanes are planned; complete accessibility/version/native-platform evidence remains open. |
| E.9 Capability Registry | PASS WITH RECORDED LIMITATION | Versioned closed executable manifests, authority, workflow, verification, degradation, budgets, fixtures, migration, and retirement are specified; no new capability is enabled. |
| E.10 Multi-agent execution | PASS WITH RECORDED LIMITATION | Dependency gate, worktrees, leases, capacity, isolation, review return, serialized integration, re-verification, deterministic campaign completion, and human promotion are planned; execution remains blocked behind single-agent gates. |
| E.11 Planning quality | PASS | Tasks are bounded, module/file owners are named where knowable, validation/evidence is explicit, critical path and parallel ownership are visible, and additions precede dependent gates. |
| E.12 Status truth | PASS | Planning changes enable nothing and do not promote lifecycle, model, endpoint, platform, workflow, package, signing, or release status; README and status model agree. |

## 11. Validation Results

| Command or group | Exact result |
|---|---|
| Engineering Runtime schema and manifest checks | PASS: 14 new schemas, 6 reused schemas, 24 requirements, 29 tests, 20 stories, 60 tasks, 141 sub-tasks; all schema and mutation tests pass. |
| Additions, architecture, task graph, planning scope, requirements, coverage, traceability | PASS: 294 requirements, 31 normative mappings, four foundational epics, 17 unchanged release epics, and 169 unchanged sprints. |
| Module, dependency, build, product-CI, artifact, component, policy, reference, activation, and platform checks | PASS for current local planning/contracts; 498 inventoried packages and zero enabled model/endpoint/route. |
| Documentation, links, Mermaid, identifiers, and schema suite | PASS: 344 governed Markdown files and 127 Mermaid blocks; local instruction input excluded by exact path. |
| Product format, build, strict lint, security boundary checks, and tests | PASS: `npm run product:test` exited 0; all executed Rust tests passed and VS Code shell tests passed 57/57. |
| Historical replay | BLOCKED after all preceding current deterministic checks pass through the fuzz-policy gate: the kernel-contract reference correctly refuses source that differs from its independently reviewed revision. |
| Full historical/clean-HEAD suite | BLOCKED: the diagnostic run executed 1,859 tests and reported 35 failures and 110 errors across stale hash-bound evidence and review chains; two ordinary mutation-test failures were repaired and their focused eight-test suite passes. A final diagnostic rerun is recorded in the completion report. |
| Live qualification | BLOCKED: native Fedora/Ubuntu/Windows/macOS package lanes, real model artifacts, local/private/managed endpoints, credentials, network routes, accessibility/performance matrices, signing, and external review were not executed. |

## 12. Final Disposition and Remaining Decisions

**PASS WITH RECORDED LIMITATION** for the governance, architecture, requirements, security,
implementation-planning, task-roadmap, schema, registry, validation, and audit assignment.

No additional architecture decision is required before implementation resumes. Future decisions
remain necessary only when selecting an exact remote provider/operator, changing a data destination
or retention/training policy, enabling a model or route, admitting a proposed VS Code API, changing
the support matrix, or widening authority. Those choices must not be inferred from this plan.

Strict-local remains a complete target design. Remote inference remains optional and cannot activate
or become fallback silently. No accepted requirement, test, control, reviewer protocol, story,
task, evidence claim, or completed status was deleted or renumbered. No raw credential or private
endpoint value was added. The status model remains truthful and every new implementation checkbox
remains open.
