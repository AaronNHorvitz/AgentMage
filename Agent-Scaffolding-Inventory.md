# AgentMage — Portable Local Agent Scaffolding Inventory

## What This Inventory Is and Why It Exists

AgentMage is a brand-new, from-scratch project. This inventory defines its requirements from the beginning.

AgentMage is an independent, privately developed product created by Aaron N. Horvitz on personal time, on personally controlled hardware, with independently obtained tools and services. It is not sponsored, commissioned, or developed on behalf of an employer. It is intended for public distribution. Evaluation or installation on a managed device is a separate decision by that device's owner or operator and does not change project ownership.

This inventory is interpreted with the public vulnerability/support process in [SECURITY.md](SECURITY.md), the model admission rules in [MODEL-PROVENANCE-POLICY.md](MODEL-PROVENANCE-POLICY.md), the process, privilege, socket, and data-flow specification in [RUNTIME-BOUNDARIES.md](RUNTIME-BOUNDARIES.md), and the current-state contract in [architecture/status-model.json](architecture/status-model.json) under [Decision 0012](docs/decisions/0012-stabilization-truth-and-status-model.md).

This is a design inventory for building a small, local assistant that can use approved models, read files, preserve memory, run bounded tools, and show evidence for its work. It is not an implementation and does not authorize access to private or environment-specific content, external systems, commits, uploads, messages, or other state-changing actions; the checkboxes describe possible building blocks, their status, and the order in which they should be evaluated.

## Current Implementation Truth

Current product lifecycle: `scaffolded`.

Current integrated workflow: none.

Current enabled models: none.

Current supported platforms: none.

Stabilization scope freeze: active.

This inventory preserves accepted target requirements and historical wording; it
does not promote them to current product claims. The 227 stable requirements and
their additions-only identities remain unchanged while the original roadmap is
paused for approval-gated stabilization.

The context that makes this project worth building is simple: frontier models are extraordinary but expensive, token-limited, cloud-bound, and unavailable for content that must never leave the machine — while most of a working day's actual load is not frontier work at all. Reading notes, tracking tasks, cleaning meeting records, converting documents, inspecting repositories, assembling briefings, and preserving continuity are often mechanical jobs with checkable answers. AgentMage's long-term direction is to use the least powerful measured tier that satisfies explicit acceptance checks. v0.1 is intentionally simpler: deterministic operations run first when applicable, the user selects the local model explicitly, no automatic model switch occurs, and no frontier transfer exists.

The deeper design bet is that most dependability lives in scaffolding rather than in any model. Evidence receipts, capability grants, bounded budgets, schema validation, contradiction checks, and audit records turn model output into a proposal that the kernel can verify. Encrypted SQLite is the sole authority for operational state. Beginning in v0.2, Markdown is authoritative only for human-owned knowledge and approved portable memory, while JSON Lines remains export-only. Because models remember nothing between sessions, v0.1 persists the minimum encrypted checkpoint needed to resume one read-only session without repeating completed work.

The trajectory and posture are deliberate. v0.1 remains an internal read-only target in native Visual Studio Code Chat. Under Decision 0008, v1.0 is the first supported public-release target: Fedora, Ubuntu, and Windows 11 x64 must support the complete promoted delivery workflow before that gate can pass, while Apple Silicon MacBook Pro M5 work remains preserved as a post-GA lane and Intel Mac remains deferred. Writes are deny-by-default and approval-gated, workspaces are boundaries rather than suggestions, secrets never enter memory or logs, and every claim of completed work must trace to a tool receipt or source citation. The checkboxes that follow are labeled by status, sequenced by the recommended build order, and gated by synthetic evaluation fixtures before anything touches real files, so this document should be read as a map of what could be built and the order in which to find out, not a promise that all of it will be.

## Local Model Compatibility

The agent is designed around a provider-neutral model interface rather than one model family. A model can be added when a local runtime exposes a compatible chat endpoint and the model profile records its identifier, context limits, output limits, tool support, vision support, timeout, and known limitations.

v0.1 originally named **Gemma 4 E4B** as its first candidate local profile, subject to the early feasibility and admission gate in [MODEL-PROVENANCE-POLICY.md](MODEL-PROVENANCE-POLICY.md). Its current feasibility disposition is rejected and disabled. **Gemma 4 12B Unified**, the named fallback candidate, is also rejected and disabled. Neither is an enabled model, and either requires a new revision-bound admission and evaluation decision before activation. A future admitted model manifest binds the first-party model identity, Apache-2.0 license disposition, upstream hash, conversion and quantization recipe, packaged-artifact hash, tokenizer hash, and runtime compatibility. `ai/gemma4:e4b` remains only a candidate Docker Model Runner tag; only an admitted immutable OCI digest may identify a release artifact. Native adapters may use only the same admitted, hash-pinned profile through `llama.cpp`. A model may handle routine chat, extraction, summaries, and bounded read-only tool-assisted tasks only after the fixed quality and resource corpus passes.

**Gemma 4 12B Unified** remains the named fallback candidate. Its rejected
disposition keeps it disabled; reopening it requires the complete provenance,
license, security, runtime, hardware-fit, quality, and resource admission
process. AgentMage never switches to it automatically.

Two later candidate profiles remain disabled until their release-specific capability and resource gates pass:

- **Gemma 4 26B** — later deep and verification work; its exact runtime identifier and digest must be verified before it can be configured.
- **Mistral Devstral Small 2** — later coding work, identified as `ai/devstral-small-2:24B` and digest-pinned before use.

Frontier consultation is a later, manually approved export/import workflow and is not a local model profile or a v0.1 capability. Other compatible model families may be evaluated only when a measured capability gap justifies the additional profile.

v0.1 uses explicit user model selection and never performs an automatic model switch, fallback, ensemble call, or frontier transfer. Later routing work requires repeatable benchmarks and a separate release decision.

## Inventory Rules

These rules define the safety and evidence boundaries for the inventory. They keep the project focused on a local, testable assistant and prevent the inventory from implying that every listed feature should be built at once.

- The stable-ID release backlog is executable; the larger checkbox inventory is a capability roadmap and does not define release scope.
- Build AgentMage as its own bounded local assistant and make only capability claims supported by acceptance tests.
- Keep every capability local, bounded, testable, and approval-gated according to its risk.
- Keep v0.1 fully local after model installation: no cloud-hosted model, external API, external web service, remote database, telemetry, crash reporting, or cloud storage.
- Treat Apple Silicon macOS, Fedora, and Ubuntu as first-class v0.1 targets. Platform adapters may implement isolation differently but may never weaken the shared path, authority, privacy, evidence, or offline contracts.
- Treat Codex as a separate user-controlled Visual Studio Code surface, never as an AgentMage model, tool, fallback, router destination, or authority. AgentMage may prepare a local handoff packet, but only the user may switch tabs and submit selected content.
- Permit only local in-process calls, local command-line programs, Unix domain sockets, or explicitly approved loopback connections that never bind to a non-loopback address.
- Store durable state only in a user-selected encrypted local data root outside cloud-synchronized folders.
- Preserve user files and write generated outputs to separate staging files unless an exact change is approved.
- Never persist passwords, tokens, private keys, hardware-backed signing secrets, or other credentials in prompts, memory, receipts, or logs.

## Status Labels

These labels describe roadmap disposition; they do not assign a requirement to a release.

- `BUILD` — implementation is required.
- `CAPABILITY GATE` — must pass before the capability containing the item can be enabled.
- `ROADMAP` — retained for a later release and excluded from v0.1.
- `DEFER` — intentionally outside the planned release sequence.
- `VERIFY` — confirm package approval, installed version, security boundary, or behavior before use.
- `RECOMMENDED DEFAULT` — initial operating behavior subject to measured revision.

## Product Architecture

AgentMage has three layers with one-way dependencies:

1. **Kernel** — configuration, task contract, capability grants, policy enforcement, receipts, canonical operational storage, model adapters, sandboxed tool execution, platform-security contracts, and recovery. The kernel never imports an interface or capability pack.
2. **Capability packs** — bounded tools and workflows built on the kernel. Planned packs are Core Read-Only, Knowledge and Obsidian, Controlled Writes, Coding, Administrative Work, Documents, GitHub, Frontier Consultation, and Multi-Agent Direction.
3. **Shells** — user interfaces that call the kernel without gaining additional authority. The v0.1 shell is native Visual Studio Code Chat. A diagnostic command-line harness supports development but is not a second v0.1 user interface. A complete command-line shell and desktop application arrive later.

Every capability pack declares its tools, data access, required grants, acceptance tests, and release. Shells may display and request authority, but only the kernel can issue, validate, consume, or reject a capability grant.

The kernel uses explicit platform adapters for local inference, workspace authorization, secure path resolution, tool confinement, secret storage, resource control, installation, and updates. Capability packs and shells cannot branch on the operating system or bypass those contracts.

## Executable v0.1 Backlog

The following table is the complete executable backlog for v0.1, **Read-Only Local Evidence Assistant**. A requirement enters implementation only through this table. Every row has a stable identifier, explicit dependencies, disposition, release target, and acceptance-test identifiers.

| ID | Requirement | Dependencies | Disposition | Target | Acceptance tests |
|---|---|---|---|---|---|
| `AM-KRN-001` | Implement the interface-independent kernel boundary and typed task, tool, result, and receipt contracts. | None | Build | v0.1 | `AT-ARCH-001`, `AT-EVD-001` |
| `AM-CFG-001` | Load and validate versioned configuration without silently broadening authority. | `AM-KRN-001` | Build | v0.1 | `AT-CFG-001` |
| `AM-PLT-001` | Implement and package the platform-adapter boundary for Apple Silicon macOS, Fedora, and Ubuntu, with MacBook Pro M5 as the primary launch reference. | `AM-KRN-001`, `AM-CFG-001` | Build | v0.1 | `AT-PLAT-001` |
| `AM-SEC-001` | Implement the shared threat model and documented macOS and Linux process topologies. | `AM-PLT-001` | Build | v0.1 | `AT-SEC-001` |
| `AM-SEC-002` | Run every file and Git tool inside the restricted platform tool worker. | `AM-SEC-001` | Build | v0.1 | `AT-SBX-001`, `AT-INJ-001` |
| `AM-NET-001` | Enforce offline operation and restrict the selected local inference runtime to the guarded inference path. | `AM-SEC-001` | Build | v0.1 | `AT-NET-001`, `AT-NET-002` |
| `AM-PTH-001` | Enforce canonical workspace-relative capability paths through the platform path adapter while generating absolute paths only for display links. | `AM-PLT-001` | Build | v0.1 | `AT-PATH-001` |
| `AM-AUT-001` | Define and enforce the single-use `CapabilityGrant` contract for every authority-bearing action. | `AM-CFG-001`, `AM-PTH-001` | Build | v0.1 | `AT-AUTH-001` |
| `AM-DAT-001` | Make SQLite the sole authority for sessions, tasks, checkpoints, grants, and receipts. | `AM-KRN-001` | Build | v0.1 | `AT-DATA-001`, `AT-CRASH-001` |
| `AM-PRV-001` | Classify data before persistence and enforce encryption, minimization, retention, and operating-system key storage. | `AM-DAT-001`, `AM-PLT-001` | Build | v0.1 | `AT-PRIV-001`, `AT-PRIV-002` |
| `AM-MDL-001` | Verify the same manifest-pinned Gemma 4 E4B profile through native `llama.cpp` on macOS, Fedora, and Ubuntu, and through the separately gated Docker Model Runner compatibility adapter on Linux; require contract and evaluation parity before enabling either Linux adapter. | `AM-PLT-001`, `AM-NET-001` | Verify | v0.1 | `AT-MODEL-001` |
| `AM-MDL-002` | Use deterministic operations first and explicit user model selection; record benchmark data without automatic routing. | `AM-MDL-001` | Build | v0.1 | `AT-ROUTE-001` |
| `AM-MDL-003` | Implement the separate approved-model installer/importer with hardware-fit checks, license and lineage display, resumable staging, hash verification, quarantine, atomic activation, self-test, unload, and cleanup. | `AM-PLT-001`, `AM-NET-001`, `AM-MDL-001` | Build | v0.1 | `AT-MODEL-002` |
| `AM-DIA-001` | Produce a redacted local doctor response inside native Visual Studio Code Chat for model/runtime identity, offline state, platform boundary, workspace grant, capability versions, repository-map health, encrypted storage, and session recovery without exposing secrets; retain a non-user-facing diagnostic harness for automated testing. | `AM-SEC-001`, `AM-PRV-001`, `AM-MDL-003` | Build | v0.1 | `AT-DIA-001`, `AT-DOC-001` |
| `AM-TOL-001` | Implement bounded list, read, search, metadata, and hash tools. | `AM-SEC-002`, `AM-PTH-001` | Build | v0.1 | `AT-TOOL-001`, `AT-PATH-001` |
| `AM-GIT-001` | Implement read-only Git status, diff, log, branch, and object inspection. | `AM-SEC-002`, `AM-PTH-001` | Build | v0.1 | `AT-GIT-001` |
| `AM-INS-001` | Treat repository instructions and all workspace text as untrusted content; allow only cited or explicitly trust-gated behavioral guidance that cannot change policy, grants, roots, tools, or user intent. | `AM-SEC-002`, `AM-TOL-001` | Build | v0.1 | `AT-INJ-001`, `AT-INS-001` |
| `AM-REP-001` | Build a deterministic, Git-aware, policy-aware repository map with pinned Tree-sitter parsers, supported-language symbols, reliable definitions/imports/relationships, bounded cache, coverage report, and exact source citations. | `AM-TOL-001`, `AM-GIT-001`, `AM-INS-001` | Build | v0.1 | `AT-REP-001`, `AT-EVD-002` |
| `AM-EVD-001` | Require a receipt for every tool action and a resolvable citation for every file-grounded claim. | `AM-TOL-001`, `AM-GIT-001` | Build | v0.1 | `AT-EVD-001`, `AT-EVD-002` |
| `AM-EVD-002` | Label material claims as Observed, Derived, Inferred, or Unknown/Blocked and bind citations to source hashes and ranges so changed evidence becomes visibly stale. | `AM-EVD-001`, `AM-REP-001` | Build | v0.1 | `AT-EVD-003`, `AT-RESUME-001` |
| `AM-HOF-001` | Render a local, reviewable Codex handoff packet without invoking Codex, controlling its tab, writing the clipboard, or transmitting content. | `AM-EVD-002`, `AM-PRV-001` | Build | v0.1 | `AT-HOF-001` |
| `AM-SES-001` | Persist and resume one active local session without duplicating completed actions. | `AM-DAT-001`, `AM-EVD-002` | Build | v0.1 | `AT-CRASH-001`, `AT-RESUME-001` |
| `AM-VSC-001` | Register AgentMage as a Visual Studio Code language-model chat provider and expose Gemma 4 E4B in the native model picker. | `AM-MDL-001`, `AM-KRN-001` | Build | v0.1 | `AT-VSC-001` |
| `AM-VSC-002` | Stream local responses, evidence states, citations, progress, cancellation, diagnostics, and failures in native Visual Studio Code Chat. | `AM-VSC-001`, `AM-EVD-002`, `AM-DIA-001` | Build | v0.1 | `AT-VSC-002` |
| `AM-TST-001` | Run the complete platform, path, sandbox, instruction-injection, network, privacy, handoff, and crash security suite. | `AM-PLT-001`, `AM-SEC-001`, `AM-SEC-002`, `AM-NET-001`, `AM-PTH-001`, `AM-AUT-001`, `AM-DAT-001`, `AM-PRV-001`, `AM-INS-001`, `AM-HOF-001` | Build | v0.1 | `AT-PLAT-001`, `AT-SEC-001`, `AT-SBX-001`, `AT-INJ-001`, `AT-INS-001`, `AT-NET-001`, `AT-NET-002`, `AT-PATH-001`, `AT-AUTH-001`, `AT-DATA-001`, `AT-CRASH-001`, `AT-PRIV-001`, `AT-PRIV-002`, `AT-HOF-001` |
| `AM-TST-002` | Run the repository-map, evidence-state, model lifecycle, diagnostics, quality, latency, memory, context, and model capability suite against fixed fixtures. | `AM-MDL-001`, `AM-MDL-002`, `AM-MDL-003`, `AM-DIA-001`, `AM-TOL-001`, `AM-GIT-001`, `AM-REP-001`, `AM-EVD-002`, `AM-SES-001`, `AM-VSC-001`, `AM-VSC-002` | Build | v0.1 | `AT-MODEL-001`, `AT-MODEL-002`, `AT-DIA-001`, `AT-ROUTE-001`, `AT-TOOL-001`, `AT-GIT-001`, `AT-REP-001`, `AT-EVD-001`, `AT-EVD-002`, `AT-EVD-003`, `AT-RESUME-001`, `AT-VSC-001`, `AT-VSC-002`, `AT-QUAL-001`, `AT-PERF-001` |
| `AM-DOC-001` | Publish startup, limitation, privacy, recovery, and offline-verification instructions. | `AM-TST-001`, `AM-TST-002` | Build | v0.1 | `AT-SPEC-001`, `AT-DOC-001` |

Every `Build` row is new AgentMage work. A `Verify` row establishes an environmental fact before dependent work begins.

## Release Sequence

| Release | Product increment | Explicit exclusions |
|---|---|---|
| v0.1 | MacBook Pro M5-first, Fedora-compatible, and Ubuntu-compatible read-only evidence assistant in native Visual Studio Code Chat using verified local Gemma 4 E4B, model diagnostics, deterministic repository maps, explicit evidence states, and local Codex handoff previews. | Intel Mac, Windows, Codex invocation or transfer, Obsidian, semantic/vector indexing, writes, coding changes, frontier delivery, full CLI, standalone desktop UI, GitHub, browser, connectors, schedules, child agents. |
| v0.2 | Knowledge pack: direct read-only Obsidian and Markdown knowledge, portable long-term memory, and handoffs. | Writes to user files and all external integrations. |
| v0.3 | Controlled Writes pack: new-file staging and exact-preimage patches using capability grants. | Commit, push, publication, and unattended writes. |
| v0.4 | Coding pack and complete command-line shell: repository comprehension, tests, bounded patches, and review packets. | Automatic commit, push, merge, or publication. |
| v0.5 | Frontier Consultation pack: user-reviewed local packets for manual export and validated import. | Agent-initiated, automatic, or unattended cloud transfer. |
| v0.6 | Administrative and Document packs. | Sending correspondence or changing live calendars. |
| v0.7 | Read-only GitHub and approved connector foundations. | Automatic hosted changes. |
| v1.0+ | Desktop shell, approval-gated hosted changes, and bounded multi-agent direction after separate security gates. | Agent swarms and self-expanding authority. |

Decision 0008 reclassifies every v0.x row above as an internal milestone and supersedes the prior v1.0 boundary. The first supported v1.0 GA additionally requires Section 36, Fedora, Ubuntu, Windows 11 x64, and the complete published delivery capability matrix. Historical exclusions remain accurate for their individual milestone rows.

## 1. Local Model and Runtime

This section covers how the local assistant talks to a model and how different models can be selected safely. It exists so the rest of the system remains independent of Docker, `llama.cpp`, Metal, Gemma, or any single model provider.

- [ ] `CAPABILITY GATE` Define a `LocalModelRuntime` contract for load, unload, health, token count, streaming inference, cancellation, resource reporting, model-manifest verification, and zero-network operation.
- [ ] `CAPABILITY GATE` On the MacBook Pro M5 reference path, use a pinned, signed `llama.cpp` build with Metal acceleration inside the sandboxed inference service; do not require Docker Desktop, Homebrew, Rosetta, or a hosted account.
- [ ] `CAPABILITY GATE` On Fedora and Ubuntu, make a pinned native `llama.cpp` adapter the security and performance reference, and support Docker Engine with Docker Model Runner as a separately gated compatibility adapter behind the same contract. Both must pass identical contract, quality, isolation, resource, cancellation, and zero-egress tests.
- [ ] `CAPABILITY GATE` Treat Docker Desktop with Docker Model Runner as an optional macOS adapter whose separate licensing and security checks must pass; it is not required for the reference installation.
- [ ] `CAPABILITY GATE` Keep model acquisition separate from inference. A signed model installer/importer may temporarily use the network or accept a user-selected local artifact, but it receives no workspace bookmark, session database, grant, or inference authority. It must display the model license, verify the approved manifest and all hashes in staging, install atomically, and exit before normal operation begins.
- [ ] `CAPABILITY GATE` Publish an approved-artifact catalog containing exact model identity, publisher, upstream lineage, license, quantization, tokenizer, expected download size, expected working set, context ceiling, acceleration requirements, runtime compatibility, and all hashes; do not expose an arbitrary model marketplace in v0.1.
- [ ] `CAPABILITY GATE` Before acquisition, assess operating system, architecture, available memory, free disk, supported acceleration, and expected artifact/runtime fit, and refuse an artifact that cannot satisfy the approved profile on the current machine.
- [ ] `CAPABILITY GATE` Make downloads and imports recoverable: use bounded retry, resumable staging, partial-download cleanup, hash-failure quarantine, atomic activation, installation self-test, safe load/unload, and clean cancellation.
- [ ] `CAPABILITY GATE` Provide Gemma 4 E4B as the fast default model using one verified manifest. Record the first-party artifact, conversion recipe, quantization, tokenizer, packaged hash, runtime build, and `ai/gemma4:e4b` registry digest where applicable.
- [ ] `CAPABILITY GATE` Keep Gemma 4 12B Unified as a named but disabled fallback profile. Enable it only through a recorded release decision after the complete independent model-admission and hardware-fit gate passes; never switch to it automatically.
- [ ] `VERIFY` Gemma 4 26B as a slower comparison or difficult-task model after registry verification and measurement of memory use, latency, context behavior, and task quality on the reference workstation; record the exact identifier and digest before enabling it.
- [ ] `VERIFY` Devstral Small 2 for the later Coding pack using `ai/devstral-small-2:24B`; resolve and pin its registry digest before enabling it.
- [ ] `BUILD` a `ModelConfig` class with model name, endpoint, context limit, output limit, timeout, temperature, tool-use support, and enabled state.
- [ ] `BUILD` a provider-neutral `BaseModelClient` interface so the agent is not tied to one model, server, operating system, or inference engine.
- [ ] `BUILD` provider-independent internal model roles for dialogue, tool selection, repository-map summarization, context compaction, citation verification, and later embedding, reranking, and patch generation. v0.1 enables only roles passed by the approved Gemma 4 profile and does not route automatically.
- [ ] `CAPABILITY GATE` Bind every internal role to an allowlisted, manifest-pinned profile admitted under [MODEL-PROVENANCE-POLICY.md](MODEL-PROVENANCE-POLICY.md) that separately passes license, publisher/control, lineage, security, quality, hardware-fit, and task-specific evaluation; never expose an arbitrary provider or endpoint marketplace as the v0.1 selection model.
- [ ] `BUILD` `BaseLLMClient`, `LLMResponse`, `Message`, `ToolCall`, and `ToolResult` abstractions behind the provider-neutral model interface.
- [ ] `BUILD` native `LlamaCppMetalClient` and `LlamaCppLinuxClient` adapters plus a `DockerModelRunnerClient` for approved isolated Linux use, all hidden behind `LocalModelRuntime` and all consuming the same versioned profile and conformance fixtures.
- [ ] `BUILD` model health checks that verify the endpoint, selected model, backend, and exact-response behavior before a session starts.
- [ ] `BUILD` bounded retry and exponential-backoff behavior that never retries indefinitely.
- [ ] `BUILD` timeout and cancellation support for model calls and tool loops.
- [ ] `BUILD` structured JavaScript Object Notation (JSON) response validation with a plain-text fallback.
- [ ] `BUILD` streaming-response support for the Visual Studio Code chat interface.
- [ ] `BUILD` a model-capability registry that records tool calling, vision, maximum input, maximum output, and known limitations.
- [ ] `ROADMAP` a benchmark-gated task router that can recommend or select among enabled profiles only after v0.1 records sufficient task-class evidence.
- [ ] `VERIFY` that every configured model is locally permitted and is not based on a prohibited model family.
- [ ] `VERIFY` offline behavior on the MacBook Pro M5, Fedora, and Ubuntu by disabling network access after model installation and proving that prompts and responses remain on the workstation.

## 1A. Model Routing and Local Resource Management

This section controls how the agent chooses among installed models and stays responsive on a local computer. It exists so a larger model, failed endpoint, full disk, or memory pressure cannot silently stall or destabilize the entire assistant.

- [ ] `CAPABILITY GATE` Show the active model, exact digest, runtime, configured context limit, and known tool or vision limitations in every session.
- [ ] `CAPABILITY GATE` Provide a redacted local doctor response in native Visual Studio Code Chat containing model and runtime manifests, hardware-fit result, offline state, platform sandbox and helper status, workspace grant, capability-pack versions, repository-map health, encrypted-store availability, last receipt sequence, and session recoverability without including secrets, prompts, or unrelated absolute paths. Keep any command-line diagnostic harness internal to development and verification in v0.1.
- [ ] `CAPABILITY GATE` Let the user choose the model manually and change models without losing the active task, plan, evidence, or rolling memory.
- [ ] `CAPABILITY GATE` In v0.1, run deterministic read-only operations before model inference, never switch models automatically, never contact a frontier model, and stop with a visible failure when the selected model cannot satisfy the task contract.
- [ ] `CAPABILITY GATE` Record task class, selected model, deterministic operations, validation outcomes, latency, and acceptance result as benchmark data without changing routing behavior.
- [ ] `ROADMAP` a measured fallback chain that activates only after benchmark gates pass and the user enables automatic routing.
- [ ] `BUILD` model-specific prompt and tool-call adapters so one model's formatting quirks do not leak into the shared agent contract.
- [ ] `BUILD` local resource monitoring for available memory, disk space, model load time, inference latency, and thermal pressure.
- [ ] `BUILD` safe model loading and unloading with one large active model by default and a visible warning before concurrent models are loaded.
- [ ] `BUILD` per-model benchmark results for coding, retrieval, planning, document work, tool use, and factual accuracy.
- [ ] `BUILD` optional high-risk answer verification by a second installed model, with both models and disagreements shown to the user.
- [ ] `DEFER` automatic model ensembles and invisible task routing until local benchmarks prove that they improve results reliably.

## 1B. Tiered Local Routing and User-Initiated Frontier Handoff

This roadmap section defines how a later release may divide local work among deterministic scripts and approved local models, then recommend an external frontier consultation when local acceptance checks fail. No release may autonomously invoke Codex or another frontier service. AgentMage stops after preparing a local packet; only the user may switch interfaces and submit selected content.

- [ ] `CAPABILITY GATE` Route local work to the least expensive measured local tier that passes its acceptance checks: deterministic scripts first, then an approved local model, with human clarification as its own destination.
- [ ] `CAPABILITY GATE` Base routing on objective signals — task-intent class, the complexity-and-risk classification, and the measured capability matrix — never on a local model's self-reported confidence, which may be one input but never the deciding vote.
- [ ] `CAPABILITY GATE` On repeated validation failures, failed tests, contradictions, rejected verification, or exhausted budgets, stop local work and present either a clarification request or a frontier-handoff recommendation; never send it automatically.
- [ ] `CAPABILITY GATE` Keep clarification and frontier recommendation separate; the user decides whether the task belongs to a human, Codex, another approved frontier surface, or no further action.
- [ ] `CAPABILITY GATE` Assemble every frontier handoff deterministically as a bounded local packet containing compressed current state, tool receipts and citations, the exact task or question, constraints and authority boundary, acceptance checks, a disclosure list, and a required output contract.
- [ ] `CAPABILITY GATE` Render the packet locally for review. AgentMage must not invoke Codex, activate or populate the Codex tab, write the packet to the clipboard, call an external endpoint, or otherwise deliver it.
- [ ] `CAPABILITY GATE` Require secret redaction, workspace-scope checks, and an explicit disclosure preview before the user may manually copy or transfer any packet.
- [ ] `BUILD` a `tier` field on task records with values script, local-model, frontier-recommended, and ask-user, so recommendations are recorded, auditable, and available across sessions.
- [ ] `BUILD` a local router that consults measured per-task-class success rates and refuses to assign work to an unmeasured or failing local tier; frontier is a recommendation, never an assignment destination.
- [ ] `BUILD` an escalation-packet builder that uses the handoff machinery: current objective, evidence, read order, and an exact frontier prompt.
- [ ] `BUILD` a required return-manifest schema in the frontier output contract: the decision or artifact with rationale, plus remaining steps each labeled with tier, inputs, acceptance check, and approval requirement, so de-escalation back to scripts or the local model is structural rather than conversational.
- [ ] `BUILD` return-manifest enforcement that runs every returned step under the same validation and acceptance checks as native work; a mislabeled step fails loudly and re-escalates with its failure evidence attached rather than being silently accepted.
- [ ] `BUILD` a handoff receipt recording the trigger, packet hash, redaction result, user-recorded destination when provided, response summary, and resulting local decisions in the audit log.
- [ ] `BUILD` round-trip limits and a rule that repeated escalation of the same task class is a benchmarking signal to remeasure, not a routine cost to keep paying.
- [ ] `RECOMMENDED DEFAULT` Treat frontier engagement as memo-shaped: one dense packet in, one structured response out, like a consultant writing an opinion rather than a pair programmer on retainer.
- [ ] `RECOMMENDED DEFAULT` Record every escalation outcome in the capability matrix so routing thresholds improve from evidence instead of staying guesses.
- [ ] `CAPABILITY GATE` Prohibit agent-initiated, automatic, or unattended delivery of content to Codex or any other external model in every release; standing consent cannot override this rule.

## 2. Agent Orchestrator

The orchestrator is the control layer that turns a user request into bounded steps, tool calls, evidence, and a final result. It exists to keep the model from deciding on its own how far it may go or claiming work without proof.

- [ ] `BUILD` an `AgentRuntime` class that owns the model client, instructions, tools, permissions, working directory, task state, and session log.
- [ ] `BUILD` an `AgentConfig` data class or schema for model, tools, permissions, budgets, memory, output directories, and approval policy.
- [ ] `BUILD` `BaseAgent`, `ChatAgent`, `ScopeAgent`, `QueryAgent`, `KnowledgeAgent`, `NarrativeAgent`, and `DisclosureAgent` interfaces for bounded agent roles.
- [ ] `BUILD` a bounded observe-plan-act-review loop with a maximum number of model turns and tool calls.
- [ ] `BUILD` a `TaskPlan` object with objective, steps, current step, status, evidence, blockers, and completion checks.
- [ ] `BUILD` a `RunBudget` object with turn, tool, time, file, output, and retry limits.
- [ ] `BUILD` explicit stop conditions for completion, missing authority, repeated failure, budget exhaustion, unsafe request, and user interruption.
- [ ] `BUILD` a fact-versus-inference response contract.
- [ ] `BUILD` a completion-evidence rule that prevents the agent from claiming it read, changed, tested, committed, or published something without corresponding tool evidence.
- [ ] `BUILD` a task-intent classifier for answer, explain, review, diagnose, plan, change, monitor, and wait requests.
- [ ] `BUILD` a scope guard that rejects actions outside the active workspace or user-approved task.
- [ ] `BUILD` a resumable session state that records the exact objective and next safe action.
- [ ] `DEFER` autonomous self-improvement, self-modifying prompts, unlimited loops, and unsupervised background work.

## 2A. AgentMage Session Behavior

This section covers the visible working behavior around planning, progress updates, interruption, attachments, and handoff. It exists because a useful assistant needs reliable session behavior in addition to a capable model.

- [ ] `CAPABILITY GATE` Build an active-plan manager with pending, in-progress, completed, and blocked steps and no more than one in-progress step.
- [ ] `CAPABILITY GATE` Send short progress updates before tool use and during long operations so the user can see what is happening.
- [ ] `CAPABILITY GATE` Accept new user messages while work is running and decide whether they replace the task, add to it, or ask only for status.
- [ ] `CAPABILITY GATE` Support cancellation at safe tool and file boundaries.
- [ ] `CAPABILITY GATE` Produce a self-contained final response containing the outcome, changed files, checks performed, remaining work, and blockers.
- [ ] `CAPABILITY GATE` Create clickable non-authoritative display links with optional line numbers from validated `WorkspacePath` values.
- [ ] `CAPABILITY GATE` Capture the session environment: date, timezone, current directory, workspace roots, active repository, branch, and permission profile.
- [ ] `CAPABILITY GATE` Resolve attached file paths and inspect the actual file before answering questions about an attachment.
- [ ] `BUILD` checkpoints before and after state-changing actions, with a recoverable record of the pre-change state.
- [ ] `BUILD` a plan-update history so the user can see which steps were added, completed, or changed and why.
- [ ] `BUILD` a bounded wait mechanism for long commands that returns new output without busy polling.
- [ ] `BUILD` status responses that report actual progress without ending the active task.
- [ ] `BUILD` a local skill router that selects skills from the request, reads the complete selected instructions, resolves referenced files, and records which skill affected the work.
- [ ] `BUILD` tool discovery by name, purpose, side effects, and required permission instead of exposing every tool in every prompt.
- [ ] `BUILD` local attachment handlers for text, code, Word, Portable Document Format, Excel, comma-separated value, PowerPoint, image, notebook, archive, and log files.
- [ ] `BUILD` bounded delegation for one independent subtask only after the single-agent runtime is reliable; require a clear task, file boundary, result contract, and parent review.
- [ ] `DEFER` automatic multi-agent swarms, uncontrolled delegation, and shared writes by multiple agents.

## 2B. Reasoning, Verification, and Decision Quality

This section improves how the agent handles ambiguous, multistep, or high-risk work. It exists because local-model quality improves more from disciplined problem definition, evidence gathering, testing, and independent review than from asking the model to produce a longer hidden chain of thought.

- [ ] `CAPABILITY GATE` Create visible reasoning modes named fast, standard, deep, and verify, each with explicit model, context, tool, time, and review budgets.
- [ ] `CAPABILITY GATE` Begin substantial work with a compact problem frame containing the objective, known facts, constraints, unknowns, acceptance checks, risks, and authority boundary.
- [ ] `CAPABILITY GATE` Separate planning, execution, and verification so the same unsupported assumption cannot silently pass through every phase.
- [ ] `CAPABILITY GATE` Prefer tool evidence, tests, source inspection, and small experiments over model speculation whenever the claim can be checked.
- [ ] `BUILD` a complexity-and-risk classifier that recommends a reasoning mode while leaving the final model and budget visible to the user.
- [ ] `BUILD` bounded task decomposition that turns a large request into ordered subproblems with dependencies, evidence needs, and stopping conditions.
- [ ] `BUILD` an assumption register that records each material assumption, why it was needed, its risk, how it can be checked, and whether it was later confirmed or rejected.
- [ ] `BUILD` a hypothesis ledger containing possible explanations, evidence for and against each one, discriminating tests, and current status.
- [ ] `BUILD` calibrated uncertainty labels that distinguish known, directly observed, inferred, disputed, unknown, and unverified claims.
- [ ] `BUILD` contradiction and consistency checks across the request, instructions, retrieved sources, memory, tool results, proposed action, and final answer.
- [ ] `BUILD` counterexample, edge-case, failure-mode, security, data-loss, and rollback review for changes whose failure would be costly.
- [ ] `BUILD` alternative generation that compares at least two viable approaches when architecture, access, cost, or irreversible choices are involved.
- [ ] `BUILD` decision records containing the selected option, rejected alternatives, evidence, assumptions, tradeoffs, owner, and conditions that would reopen the decision.
- [ ] `BUILD` an independent verification pass that receives the task, evidence, proposed result, and acceptance checks but does not inherit the first pass's unsupported conclusions.
- [ ] `BUILD` optional cross-model review for difficult work, with disagreements preserved and resolved through evidence rather than majority vote.
- [ ] `BUILD` a clarification gate that stops and asks the user when an unanswered question would materially change scope, authority, cost, safety, or the result.
- [ ] `BUILD` a reasoning budget controller that stops when acceptance checks pass, evidence cannot improve, authority is missing, or the configured budget is exhausted.
- [ ] `BUILD` concise reasoning summaries containing the decision, key evidence, assumptions, uncertainty, and tests without requesting, storing, or exposing private chain-of-thought text.
- [ ] `BUILD` reasoning evaluations that score correctness, evidence use, calibration, contradiction detection, unnecessary tool use, recovery, and false completion claims.
- [ ] `DEFER` unbounded tree search, repeated self-debate, large model councils, and autonomous recursive planning until controlled evaluations show a reliable benefit.

### Recommended Default Reasoning Profile

This is the recommended starting configuration assembled from the capabilities above. It keeps routine work fast, gives difficult tasks more deliberate planning and verification, and makes every quality improvement measurable rather than assuming that more model output is automatically better.

- [ ] `RECOMMENDED DEFAULT` Use Gemma 4 E4B for quick extraction, summaries, routine file work, and straightforward tool execution.
- [ ] `RECOMMENDED DEFAULT` Use Gemma 4 26B or another locally approved, benchmarked reasoning model for architecture, diagnosis, ambiguous requirements, difficult coding, and high-consequence review.
- [ ] `RECOMMENDED DEFAULT` Separate substantial work into planner, executor, and independent verifier phases, even when one model performs all three phases sequentially.
- [ ] `RECOMMENDED DEFAULT` Record material assumptions and competing hypotheses, then update their status when evidence confirms, rejects, or leaves them unresolved.
- [ ] `RECOMMENDED DEFAULT` Prefer source inspection, tool receipts, tests, and small experiments over model confidence whenever the claim can be checked.
- [ ] `RECOMMENDED DEFAULT` Run counterexample, edge-case, failure-mode, security, data-loss, and rollback review before recommending or applying a risky change.
- [ ] `RECOMMENDED DEFAULT` Label important claims as known, directly observed, inferred, disputed, unknown, or unverified.
- [ ] `RECOMMENDED DEFAULT` Stop reasoning when the acceptance checks pass, evidence stops improving, authority is missing, a material clarification is required, or the configured budget is exhausted.
- [ ] `RECOMMENDED DEFAULT` Save concise decision summaries containing evidence, assumptions, uncertainty, tradeoffs, tests, and next actions; do not request or store private chain-of-thought text.
- [ ] `RECOMMENDED DEFAULT` Benchmark the fast and deep profiles on representative tasks and keep the slower profile only where it produces a measured improvement in correctness, evidence, or recovery.

## 2C. Local Agent Studio and Agent Direction

This section lets the user create and direct specialist agents without tying the design to one model, interface, or vendor. It exists so a parent agent can assign bounded work, monitor it, stop it, and review its evidence without giving a child more access than the user approved.

- [ ] `CAPABILITY GATE` Define every agent with a versioned local specification containing its name, purpose, allowed tasks, prohibited tasks, model profile, tools, readable roots, writable roots, memory scope, budgets, approval rules, output contract, completion checks, and stop conditions.
- [ ] `CAPABILITY GATE` Require every child agent to inherit the intersection of the parent agent's permissions and the current task's permissions; a child can never gain broader access by requesting another role or model.
- [ ] `CAPABILITY GATE` Require a separate explicit child grant for each assignment; effective child authority is the intersection of user authority, parent authority, task authority, and that child grant, with missing permissions denied.
- [ ] `CAPABILITY GATE` Prevent a coordinator from combining several narrow child grants into broader aggregate authority or using one child's result to authorize another child's action.
- [ ] `CAPABILITY GATE` Keep each agent's working memory, files, conversation, tool receipts, and temporary outputs isolated by default.
- [ ] `CAPABILITY GATE` Give every writable child its own isolated worktree, declared file ownership, conflict policy, and transfer review; no two children may write the same path concurrently.
- [ ] `CAPABILITY GATE` Treat every child-agent response as an untrusted proposal until the parent checks its sources, tool receipts, changed files, and completion evidence.
- [ ] `CAPABILITY GATE` Limit agent count, nesting depth, turns, tool calls, elapsed time, memory, disk use, and concurrent model loads.
- [ ] `CAPABILITY GATE` Provide pause, resume, cancel, terminate, and inspect controls for every active agent and queued assignment.
- [ ] `CAPABILITY GATE` Propagate cancellation and expiry from user to coordinator to every descendant process, model call, tool call, lease, and pending change set, and prove termination in attributable receipts.
- [ ] `BUILD` an `AgentDefinition` schema and local registry with stable identifiers, versions, owners, compatibility, status, and source hashes.
- [ ] `BUILD` an agent-creation wizard that asks what the agent should do, what it must never do, what evidence it must return, and when it must stop.
- [ ] `BUILD` versioned agent templates for research, planning, executive briefing, meeting support, document work, repository learning, coding, testing, review, and verification.
- [ ] `BUILD` a prompt and instruction checker that detects contradictions, missing boundaries, vague completion rules, hidden network needs, excessive permissions, and unsupported tool assumptions before an agent is enabled.
- [ ] `BUILD` a dry-run mode using synthetic files and fake tools so a new agent can be evaluated without reading or changing real work.
- [ ] `BUILD` an agent capability report showing which requested tools, file types, models, and permissions are available, degraded, untested, or denied.
- [ ] `BUILD` a local task-assignment protocol containing objective, inputs, allowed files, expected output, evidence requirements, due or time limit, dependency state, and return channel.
- [ ] `BUILD` a local message bus or job queue that preserves ordering, identifiers, acknowledgements, cancellations, retries, and complete local audit history.
- [ ] `BUILD` a director view showing the agent tree, active assignment, model, elapsed time, budget use, latest action, waiting approval, failure, and result.
- [ ] `BUILD` attributable per-child receipts containing assignment, effective grant, worktree, file ownership, resource use, cancellation state, evidence, outputs, and parent disposition.
- [ ] `BUILD` a shared-read and single-writer rule for files that several agents inspect; require an explicit merge step before any child output changes a user-owned file.
- [ ] `BUILD` conflict detection when agents return incompatible facts, plans, patches, or decisions; preserve both results and resolve the conflict through source checks or user direction.
- [ ] `BUILD` a parent review step that accepts, rejects, requests revision, or converts a child result into a new bounded assignment.
- [ ] `BUILD` sequential agent pipelines in which each stage receives only the approved output of the previous stage, such as planner, implementer, tester, and verifier.
- [ ] `BUILD` optional parallel read-only review for independent questions, followed by one controlled reconciliation step.
- [ ] `BUILD` per-agent model selection and model replacement without changing the agent's tool contract, permissions, memory namespace, or output schema.
- [ ] `BUILD` agent evaluation suites for instruction following, tool use, evidence quality, refusal, recovery, cost in local resources, and false completion claims.
- [ ] `BUILD` signed or hashed local agent packages with install, enable, disable, update, rollback, clone, and archive operations.
- [ ] `DEFER` self-created agents, recursive agent spawning, self-modifying agent definitions, automatic permission expansion, and unsupervised agent swarms.

## 3. Bounded Work Packet Contract

A work packet gives each task a structured objective, owner, evidence requirement, required capability class, and stopping rule. It describes work but carries no authority.

- [ ] `BUILD` a `ValidationIssue` contract for structured validation failures.
- [ ] `BUILD` `validate_packet()` for validating a bounded work request.
- [ ] `BUILD` `validate_transition()` for validating work-state changes.
- [ ] `BUILD` `_is_non_empty_string()`, `_is_non_empty_string_list()`, and `_is_iso_date()` validation helpers.
- [ ] `BUILD` `_contains_secret_like_value()` as one layer of secret-canary detection.
- [ ] `BUILD` a public `WorkPacket` data class for the defined contract fields.
- [ ] `BUILD` packet fields for objective, reason, owner, state, authoritative evidence, mutable files, protected files, expected output, acceptance checks, required evidence, required capability class, budgets, stop conditions, rollback, sensitivity, and last verification date.
- [ ] `BUILD` conditional fields for next action, next review, status reason, disposition, completion evidence, and superseding work.
- [ ] `BUILD` a deterministic packet serializer and parser.
- [ ] `BUILD` a packet revision history that never silently overwrites prior decisions.
- [ ] `BUILD` a packet-to-task-plan adapter.
- [ ] `BUILD` a packet completion validator that requires the specified evidence before closing work.

## 4. Tool Protocol and Registry

This section defines how tools are described, discovered, validated, called, and reported. It exists to give every file, shell, Git, and document operation the same input checks, limits, and evidence envelope.

- [ ] `BUILD` an abstract `Tool` interface for registered, schema-validated capabilities.
- [ ] `BUILD` a `ToolDefinition` class with name, purpose, JSON input schema, output schema, risk level, side effects, required grant template, and timeout.
- [ ] `BUILD` a `ToolRegistry` with `register_tool()`, `get_tool()`, `list_tools()`, `validate_arguments()`, and `execute_tool()`.
- [ ] `BUILD` a `ToolDispatcher` that validates and consumes the required `CapabilityGrant` before every authority-bearing call.
- [ ] `BUILD` a `ToolResult` envelope with success, output, error, evidence references, elapsed time, and whether state changed.
- [ ] `BUILD` a bounded tool-call loop with validation, receipts, cancellation, and explicit stopping conditions.
- [ ] `BUILD` protection against repeated identical tool calls.
- [ ] `BUILD` maximum tool-call depth and per-tool call budgets.
- [ ] `BUILD` output truncation that preserves the beginning, end, error, file path, and command exit code.
- [ ] `BUILD` tool-result redaction before results enter model context or logs.
- [ ] `BUILD` deterministic fake tools for tests.
- [ ] `BUILD` tool discovery that exposes only tools approved for the current workspace and task.
- [ ] `DEFER` automatic installation of tools or packages.

## 5. Permissions, Approval, and Safety

These controls determine what AgentMage may read, write, execute, or publish. The `CapabilityGrant` is the only authority-bearing object. Work packets, plans, tool definitions, interface state, model output, and child-agent requests may describe an action but cannot authorize it.

- [ ] `CAPABILITY GATE` Define a versioned `CapabilityGrant` containing grant identifier, actor, session, task, action kind, tool, canonical workspace-relative targets, argument hash, preimage hashes, expected side effects, rollback description, issuance time, expiration time, nonce, use limit, parent grant when applicable, confirmation digest, and status.
- [ ] `CAPABILITY GATE` Create a session-scoped read grant only after the user selects a workspace and sees the resolved root, exclusions, sensitivity class, and expiration; derive a single-use operation grant for each tool call inside that scope.
- [ ] `CAPABILITY GATE` Require a separately confirmed, single-use grant for every write, command, network, commit, push, publish, upload, send, deploy, database-write, credential, or deletion action.
- [ ] `CAPABILITY GATE` Bind confirmation to the exact preview hash; any change to arguments, file preimage, target, side effects, workspace, task, or policy invalidates the grant and requires a new preview.
- [ ] `CAPABILITY GATE` Validate and atomically consume the grant immediately before execution so it cannot be replayed, raced, delegated beyond scope, or reused after an uncertain result.
- [ ] `CAPABILITY GATE` Permit child agents only a derived intersection of the parent grant and task scope; a model, tool, plugin, shell, or child agent can never mint or broaden a grant.
- [ ] `BUILD` a `PolicyEngine` that evaluates the grant, user authority, workspace scope, file scope, command scope, network scope, current preimages, expiration, nonce, use count, and expected side effects.
- [ ] `BUILD` an `ApprovalRequest` display object that renders the exact proposed action and confirmation digest but carries no authority itself.
- [ ] `BUILD` an explicit allow/deny policy with deny taking precedence at every scope.
- [ ] `BUILD` a special source-control publication gate matching the active workspace instructions.
- [ ] `BUILD` a commit/push gate that requires the exact diff and commit message to be approved before the action.
- [ ] `BUILD` deny-by-default policies for delete, overwrite, commit, push, publish, upload, send, deploy, database write, and credential changes.
- [ ] `BUILD` protected-file rules for `AGENTS.md`, handoffs, source records, secrets files, Git metadata, and user-owned uncommitted changes.
- [ ] `BUILD` secret scanning for passwords, tokens, private keys, access keys, connection strings, certificate secrets, and high-entropy values.
- [ ] `BUILD` output redaction that records that a value was removed without retaining the value.
- [ ] `BUILD` `WorkspacePath` as the only tool path type, containing a workspace identifier and normalized relative path; reject absolute paths, empty components, `.` components, `..` components, NUL bytes, and alternate separators at the tool boundary.
- [ ] `BUILD` resolve paths beneath an already-authorized workspace handle through the platform adapter: Linux uses `openat2` beneath/no-symlink constraints when available; macOS resolves a user-selected read-only security-scoped bookmark and uses descriptor-relative `openat`/`fstatat` component walks with no-follow checks. Both fail closed on ambiguity.
- [ ] `BUILD` reject case-folding and Unicode-normalization collisions, aliases, stale bookmarks, mount changes, and file-identity changes on macOS; include the equivalent case-sensitive and symlink attacks on Linux.
- [ ] `BUILD` hold file descriptors across validation and use exact preimage hashes for later writes to prevent time-of-check/time-of-use path substitution.
- [ ] `BUILD` generate absolute `file:line` links only as non-authoritative interface output; never accept a displayed absolute path as a capability or tool argument.
- [ ] `BUILD` file-count, byte-count, directory-depth, and search-result limits.
- [ ] `BUILD` process timeouts and child-process cleanup.
- [ ] `BUILD` a network deny policy for the offline release.
- [ ] `BUILD` a tamper-evident local action ledger using append-only records, chained hashes, and a per-session keyed integrity anchor stored outside the ledger; do not describe chained hashes alone as immutable.
- [ ] `BUILD` a human-readable audit summary for every state-changing session.
- [ ] `VERIFY` that client-side permission checks are not treated as an operating-system sandbox.
- [ ] `DEFER` access to live databases, document repositories, archive services, downstream services, cloud platforms, messaging, email, calendars, and source-control hosting.

## 5A. macOS and Linux Threat Model, Process Topology, and Isolation

The protected assets are user files, conversations, operational state, credentials, grants, receipts, and computing resources. The expected threats are malicious workspace text, prompt injection, an incorrect or compromised model response, path traversal, symlink races, hostile archives, unauthorized network egress, plugin misuse, grant replay, crashes, and tampering with local records. AgentMage does not claim to protect against a compromised operating-system account, hostile root access, a compromised kernel, or physical attacks on an unlocked workstation.

The v0.1 process topology is fixed by a shared contract with platform-specific enforcement:

| Component | Apple Silicon macOS | Fedora and Ubuntu | Shared authority and network rule |
|---|---|---|---|
| Visual Studio Code extension | Thin JavaScript/TypeScript client plus a minimal signed arm64 IPC bridge | Thin JavaScript/TypeScript client | Display, user interaction, and cancellation only; authenticated local IPC to kernel host; no AgentMage workspace or model access |
| AgentMage kernel host | Developer ID-signed and notarized arm64 app bundle with Hardened Runtime and App Sandbox | Unprivileged user process | Policy, grants, receipts, orchestration, encrypted data root, and authorized workspace handles; no general network access |
| Tool worker | Signed, sandboxed, stateless XPC/read-only helper receiving one security-scoped bookmark and one consumed grant | Fresh Bubblewrap worker with read-only bind, seccomp, and user cgroup limits | One operation only, isolated scratch, bounded resources, no ambient home access, no network, and no workspace writes |
| Local inference service | Signed sandboxed `llama.cpp` arm64 service using Metal and pinned GGUF artifacts; no network entitlement | Native `llama.cpp` security-reference service, or separately gated Docker Model Runner compatibility container with immutable engine/model digests | Untrusted inference only; no tools, grants, credentials, workspace access, or general network; guarded local connection only |
| Model installer/importer | Separate signed component with temporary download authority or user-selected artifact access; no workspace, session, or inference authority | Separate installation command with the same staging and verification contract | Shows the license, verifies the manifest and hashes in staging, installs atomically, then exits before offline operation |
| Secret store | macOS Keychain item restricted to the signed AgentMage identity | Linux Secret Service item restricted to the AgentMage user session | Encryption key only; never model, tool, configuration, log, export, or backup content |
| SQLite operational store | App-group or application-support data root protected by encryption and macOS permissions | User-selected encrypted local data root | Canonical operational state; no network |

- [ ] `CAPABILITY GATE` Package the macOS kernel host, inference service, and tool helper as arm64 code signed with Developer ID, notarized, stapled, and verified by Gatekeeper; enable Hardened Runtime and minimal App Sandbox entitlements.
- [ ] `CAPABILITY GATE` Freeze a macOS release manifest before implementation that records the minimum and tested macOS builds, arm64 architecture, Apple SDK and Swift toolchain, Team ID, bundle identifiers, App Group identifier, designated requirements, entitlements, helper hashes, and package digest; refuse an unsupported architecture or operating-system build before starting the kernel.
- [ ] `CAPABILITY GATE` Build, sign, notarize, staple, and test the macOS package on an isolated Apple Silicon release runner using an Apple Developer Program identity. Keep signing and notarization credentials in the release secret store and out of the repository, build output, logs, model context, and distributed package.
- [ ] `CAPABILITY GATE` Bundle a minimal signed arm64 bridge with the Visual Studio Code extension. On macOS, connect it to the sandboxed host through a mode `0600` Unix socket inside the registered App Group container; validate the peer's audit token and designated code-signing requirement, and require a fresh in-memory launch challenge so another same-user process cannot impersonate either endpoint. On Linux, validate Unix peer credentials and use an equally short-lived authenticated session.
- [ ] `CAPABILITY GATE` Require the user to choose each macOS workspace through the native open panel, store only a read-only app-scoped security-scoped bookmark, detect stale bookmarks, and revoke access when the workspace is removed.
- [ ] `CAPABILITY GATE` Give the macOS XPC tool helper only inherited App Sandbox rights, the selected read-only bookmark, isolated scratch, and one consumed operation grant; it has no network entitlement and exits or is recycled after the operation.
- [ ] `CAPABILITY GATE` Run every Fedora and Ubuntu v0.1 file and Git tool in a fresh Bubblewrap sandbox with a read-only approved workspace bind, private temporary directory, minimal executable and environment allowlists, no home-directory bind, no device access beyond what is required, and no network namespace connectivity.
- [ ] `CAPABILITY GATE` Apply seccomp filtering and `systemd-run --user` cgroup limits on Linux, and platform process limits plus kernel-enforced cancellation and watchdog termination on macOS, for process count, memory, CPU time, output bytes, and elapsed time.
- [ ] `CAPABILITY GATE` Keep all workspace writes impossible inside the v0.1 tool worker; later write packs stage output in agent-owned scratch and let the kernel apply an approved exact-preimage operation.
- [ ] `CAPABILITY GATE` Treat the model, workspace text, retrieved evidence, tool output, plugin output, and imported frontier responses as untrusted proposals that cannot alter policy, issue grants, or call tools directly.
- [ ] `CAPABILITY GATE` Fail startup on macOS when signing, notarization, Hardened Runtime, App Sandbox, XPC identity, Keychain access, security-scoped workspace access, path protection, resource limits, or no-network state cannot be verified.
- [ ] `CAPABILITY GATE` Fail startup on Fedora or Ubuntu when Bubblewrap, seccomp policy, cgroup limits, Secret Service, workspace descriptor protection, required network controls, selected runtime identity, or adapter-specific isolation cannot be verified.

- [ ] `BUILD` trust labels for user instructions, workspace instructions, retrieved evidence, external content, generated text, and tool results.
- [ ] `BUILD` prompt-injection tests that try to override permissions, expose secrets, broaden scope, or trigger hidden tool calls through retrieved content.
- [ ] `BUILD` quarantine, type detection, size limits, hash recording, and optional malware scanning for downloaded files and archives.
- [ ] `BUILD` a secret-store adapter that uses the operating-system keychain or another approved local secret manager; tools receive only the credential they need for the shortest practical time.
- [ ] `BUILD` encrypted operational storage from v0.1 using a project-owned encrypted SQLite store and a key obtained from macOS Keychain or Linux Secret Service through the platform secret-store adapter; never fall back silently to plaintext for private or restricted records.
- [ ] `BUILD` a network egress monitor with destination allowlists, visible request previews, byte limits, and receipts.
- [ ] `BUILD` isolated dependency installation with pinned versions, lock files, hashes, licenses, and a software bill of materials.
- [ ] `BUILD` signed and versioned update packages with rollback and integrity verification.
- [ ] `BUILD` backup and restore tests that prove the agent can recover configuration and memory without restoring secrets into logs or prompts.

## 5B. Strict Local-Only Operation and Data Residency

This section makes "local" an enforceable technical rule rather than a promise in a prompt. It exists so a model, plugin, desktop screen, command-line command, vault reader, or background process cannot quietly send content away from the computer or place private files in a cloud-synchronized folder.

- [ ] `CAPABILITY GATE` Deny outbound network traffic for the Visual Studio Code extension, AgentMage kernel, tool workers, converters, indexers, and shells after model installation, except the kernel's verified local inference connection.
- [ ] `CAPABILITY GATE` Treat Docker Model Runner's unauthenticated HTTP interface as an untrusted local service: loopback binding is necessary but not sufficient. Place it behind the kernel's guarded adapter path, prevent access from unrelated same-user processes, tool containers, separate namespaces, and non-loopback interfaces, and give the model runtime no tool, file, grant, or credential authority.
- [ ] `CAPABILITY GATE` Place an AgentMage-owned mode `0600` Unix-socket service between Visual Studio Code and the kernel; the extension must never connect directly to Docker Model Runner.
- [ ] `CAPABILITY GATE` Refuse startup if Docker Model Runner or any required service is bound to `0.0.0.0`, a local-area-network address, an undeclared container-accessible address, or any non-loopback interface, or if an undeclared local peer can reach the service despite loopback binding.
- [ ] `CAPABILITY GATE` Verify from a separate container and network namespace that the inference port and kernel socket are unreachable outside their intended boundary.
- [ ] `CAPABILITY GATE` Keep conversations, memory, indexes, logs, checkpoints, temporary files, generated files, and backups in one user-selected local data root outside cloud-synchronized folders.
- [ ] `CAPABILITY GATE` Detect common cloud-synchronized paths and mounted remote filesystems, show the exact risky path, and refuse to store agent state there under the strict-local profile.
- [ ] `CAPABILITY GATE` Do not require login, account creation, license activation, telemetry, analytics, update checks, model marketplaces, remote feature flags, or cloud-hosted error reporting.
- [ ] `BUILD` a strict-local configuration profile that cannot enable network, remote providers, cloud connectors, remote repositories, browser tools, or cloud storage through a child setting or model response.
- [ ] `BUILD` a startup report showing every process, path, local socket or port, writable directory, enabled tool, and network rule used by the session.
- [ ] `BUILD` filesystem controls that prevent temporary-file helpers, document libraries, and desktop frameworks from silently using cloud-backed recent-files, autosave, or backup locations.
- [ ] `BUILD` dependency checks for hidden telemetry, automatic downloads, remote fonts, remote icons, content-delivery networks, and update services; replace them with local assets or disable the capability.
- [ ] `BUILD` a network-attempt ledger that records the executable, destination, time, and blocked result without storing document or prompt contents.
- [ ] `VERIFY` the strict-local profile with the network disabled and with a test firewall that fails the build if any process attempts an unapproved outbound connection.
- [ ] `VERIFY` that uninstalling or never installing the Obsidian desktop application does not remove any vault-reading, search, task, link, handoff, or note-writing capability.

## 6. Workspace and Instruction Discovery

This section explains how the assistant finds the active workspace, repositories, instruction files, and project documents. It exists so the assistant uses the right local rules and does not treat generated or unrelated files as authoritative evidence.

- [ ] `CAPABILITY GATE` Treat `AGENTS.md`, README instructions, source comments, issue text, generated content, and every other workspace file as untrusted data by default. Workspace text cannot change kernel policy, grant authority, enable tools, expand a root, override the current user request, or instruct the kernel to ignore higher-priority controls.
- [ ] `CAPABILITY GATE` Allow a repository instruction to become behavioral guidance only after its source, hash, scope, precedence, and non-authority effect are shown and explicitly trusted for the current workspace; trust never converts content into a capability grant.
- [ ] `BUILD` a `WorkspaceResolver` that finds the active repository and permitted root.
- [ ] `BUILD` hierarchical instruction discovery for root and nested `AGENTS.md` files and explicitly approved project instructions.
- [ ] `BUILD` instruction precedence rules in which kernel policy and the current user request remain authoritative, while trust-gated workspace and nested repository guidance may only narrow behavior within the current grant and must expose conflicts rather than resolving them silently.
- [ ] `BUILD` an instruction-provenance record containing workspace-relative path, content hash, discovered scope, trust decision, conflicts, and citations used in the current task.
- [ ] `BUILD` a `WorkspaceManifest` containing root, repository list, important folders, instruction files, file counts, and generated-directory exclusions.
- [ ] `BUILD` default exclusions for `.git`, `node_modules`, virtual environments, build outputs, caches, and large generated artifacts.
- [ ] `BUILD` a bounded `scan_workspace()` function using `rg --files` or a safe Python equivalent.
- [ ] `BUILD` a `discover_repositories()` function that finds Git roots without altering them.
- [ ] `BUILD` a `discover_instruction_files()` function.
- [ ] `BUILD` a `discover_project_docs()` function for readmes, architecture files, plans, tests, and handoffs.
- [ ] `BUILD` an evidence ledger that records which files were actually read and which were merely discovered.
- [ ] `BUILD` a stale-evidence warning when source files changed after the agent's last scan.
- [ ] `BUILD` workspace tree hashing for repeatable scans.
- [ ] `VERIFY` instruction-injection fixtures in which repository text requests hidden tools, policy changes, secret disclosure, broader roots, external transfer, or false completion; every attempt must remain inert and visible as untrusted content.

## 7. Filesystem Tools

These tools provide the basic ability to inspect and change local files with limits, hashes, and previews. They exist because nearly every useful workflow depends on reliable file operations that can be checked before and after execution.

- [ ] `BUILD` bounded `read_file`, `list_directory`, and `search_files_content` tools for the read-only pilot.
- [ ] `BUILD` `read_text_file(path, start_line, end_line)` with encoding detection, size limits, and line numbers.
- [ ] `BUILD` `read_multiple_text_files(paths)` with a strict file-count and byte budget.
- [ ] `BUILD` `list_directory(path)` with hidden-file and generated-folder controls.
- [ ] `BUILD` `directory_tree(path, max_depth, max_entries)`.
- [ ] `BUILD` `search_file_names(pattern, roots, globs)` backed by `rg --files`.
- [ ] `BUILD` `search_text(pattern, roots, globs, max_matches)` backed by `rg`.
- [ ] `BUILD` `file_metadata(path)` for size, timestamps, type, permissions, and hash.
- [ ] `BUILD` `hash_file(path)` and `hash_tree(path)` using Secure Hash Algorithm 256.
- [ ] `BUILD` `read_binary_metadata(path)` without sending binary bytes to the model.
- [ ] `BUILD` `write_new_file(path, content)` as the first approval-gated write operation.
- [ ] `BUILD` `apply_patch(path, patch)` with exact preimage checks.
- [ ] `BUILD` `move_file(source, destination)` with collision detection and rollback record.
- [ ] `BUILD` `copy_file(source, destination)` with source/destination hashes.
- [ ] `BUILD` `delete_file(path)` only in a later release with explicit approval and trash-first behavior.
- [ ] `BUILD` an atomic-write helper that writes a temporary file, verifies it, and then replaces the target.
- [ ] `BUILD` a post-write diff and hash report.

## 7A. Deterministic Read-Only Repository Map

This v0.1 capability gives the local model a compact structural view of an approved repository without granting write, shell, language-server, embedding, or network authority. It exists so repository questions start from deterministic syntax and Git evidence rather than repeatedly spending model context rediscovering file structure.

- [ ] `CAPABILITY GATE` Keep the repository mapper read-only, deterministic, offline, and inside the same restricted tool worker and workspace grant as file and Git inspection.
- [ ] `CAPABILITY GATE` Limit v0.1 to a declared set of pinned Tree-sitter grammars. Unsupported languages fall back to file inventory and text search with a visible coverage limitation; the model may not invent parser results.
- [ ] `CAPABILITY GATE` Exclude semantic/vector embeddings, remote indexing, arbitrary language-server execution, package installation, generated/vendor trees, and user-unapproved roots from v0.1.
- [ ] `BUILD` a Git-aware and policy-aware inventory honoring the approved root, explicit exclusions, `.gitignore` where applicable, tracked/untracked state, file type, language, content hash, size, and branch/commit identity.
- [ ] `BUILD` parser-backed records for supported-language modules, symbols, definitions, imports, and only those reference or relationship edges that the pinned parser can derive reliably.
- [ ] `BUILD` an incremental SQLite cache keyed by workspace, path, content hash, Git identity, grammar identity, parser version, and policy version; changed inputs invalidate affected records before they can be cited.
- [ ] `BUILD` a coverage report listing files discovered, parsed, searched, skipped, excluded, unsupported, failed, and truncated, plus byte, entry, and token budgets and all remaining blind spots.
- [ ] `BUILD` token-budgeted map rendering that prioritizes entry points, user-named files or symbols, direct dependency neighborhoods, tests, and configuration while preserving a visible partial-coverage warning.
- [ ] `BUILD` source resolution from every structural record to workspace-relative path, content hash, line range, parser identity, and Git identity when available.
- [ ] `BUILD` evidence typing that marks parser facts as Observed, deterministic graph calculations as Derived, model explanations as Inferred, and unavailable or failed parsing as Unknown/Blocked.
- [ ] `VERIFY` exact, stable output on supported-language fixture repositories across clean, dirty, detached, untracked, renamed, case-sensitive, Unicode, malformed, generated, and unsupported-language cases.
- [ ] `VERIFY` that building, reading, invalidating, and rendering the map leaves the workspace tree, Git index, refs, object set, instructions, and file metadata unchanged.

## 8. Obsidian Vault Reader and Index

This section covers reading and indexing the Obsidian vault, including tasks, links, handoffs, meetings, and current-versus-historical notes. It exists so the assistant can recover project context without rereading the entire vault blindly every time.

- [ ] `BUILD` `VaultReadError`, `discover_markdown()`, `hash_tree()`, and `read_vault()` with bounded evidence tests.
- [ ] `BUILD` deterministic discovery with ignored directories, Unicode paths, spaces, symlink boundaries, and stable ordering.
- [ ] `BUILD` Markdown parsing for YAML frontmatter, headings, tasks, wiki links, and source line numbers.
- [ ] `BUILD` code-fence exclusion so fake tasks and links inside code blocks are not treated as vault data.
- [ ] `BUILD` fail-closed behavior for malformed frontmatter.
- [ ] `CAPABILITY GATE` Open the vault through ordinary local filesystem reads; do not launch, automate, install, or communicate with the Obsidian desktop application.
- [ ] `CAPABILITY GATE` Require an explicitly selected local vault root and use the same `WorkspacePath`, descriptor-relative, and symlink protections as every other file tool.
- [ ] `CAPABILITY GATE` Run the vault reader in read-only mode by default; a request to change a note must produce an exact per-file preview and require separate approval.
- [ ] `CAPABILITY GATE` Keep the vault index, conversation database, temporary text, and agent memory outside the vault so the agent does not fill the user's notes with hidden state.
- [ ] `CAPABILITY GATE` Refuse strict-local vault access when the selected vault lives in a known cloud-synchronized folder or remote filesystem unless the user first moves or copies it to an approved local-only location.
- [ ] `BUILD` a pure-Python or equivalent local Markdown adapter that implements vault discovery, parsing, search, links, backlinks, tasks, and note creation without Obsidian plugins or APIs.
- [ ] `BUILD` an ignore policy for `.obsidian/`, plugin folders, caches, trash, attachments, and other hidden or binary content; read any excluded area only when the user names it and the policy permits it.
- [ ] `BUILD` a local filesystem watcher that updates the SQLite index from file changes without sending paths, names, hashes, metadata, or text to another process outside the approved local runtime.
- [ ] `BUILD` a vault-access receipt showing the approved root, files read, files ignored, index location, read/write mode, and proof that no Obsidian application or cloud service was used.
- [ ] `BUILD` an `ObsidianNote` class with path, title, frontmatter, headings, tasks, links, backlinks, timestamps, and content hash.
- [ ] `BUILD` an `ObsidianIndex` backed by local SQLite.
- [ ] `BUILD` exact wiki-link resolution, alias handling, unresolved-link detection, and ambiguity reporting.
- [ ] `BUILD` current-versus-historical classification using note status, superseded markers, dates, and handoff pointers.
- [ ] `BUILD` special readers for `Current Handoff.md`, `Current Work.md`, today's daily note, and the master task list.
- [ ] `BUILD` bounded vault queries by folder, date, note type, person, project, task status, and link target.
- [ ] `BUILD` backlink and relationship traversal with maximum depth.
- [ ] `BUILD` contact-card, project-card, meeting-note, communication-note, and task-list extractors.
- [ ] `BUILD` preservation rules for every meeting note's `## Raw Notes` section.
- [ ] `BUILD` an Obsidian-safe draft writer that uses wiki links and never bulk-reorganizes the vault.
- [ ] `BUILD` vault change previews and per-file approval.
- [ ] `BUILD` stale-index detection using content hashes and timestamps.
- [ ] `BUILD` synthetic mini-vault tests before access to selected real vault folders.

## 8A. Human Knowledge Workspace

This v0.2 capability provides the same knowledge workflows whether the user opens a plain folder or an Obsidian vault. User-owned Markdown is the sole authority for people, projects, meetings, tasks, decisions, correspondence, and handoffs. A separate SQLite knowledge index is disposable and fully rebuildable; JSON Lines is an explicit export format only.

- [ ] `CAPABILITY GATE` Define a `KnowledgeStore` domain interface whose operations preserve Markdown as the canonical human-readable record.
- [ ] `CAPABILITY GATE` Provide a plain-folder implementation using ordinary Markdown plus a regenerable local SQLite search index.
- [ ] `CAPABILITY GATE` Provide an Obsidian implementation that adds wiki links, backlinks, frontmatter, daily-note conventions, and vault-specific views without creating a second authority.
- [ ] `CAPABILITY GATE` Keep canonical record types for people, organizations, projects, meetings, tasks, decisions, commitments, documents, correspondence, deadlines, approvals, risks, questions, and handoffs.
- [ ] `BUILD` configurable folder and filename templates so the assistant fits the user's existing organization rather than forcing one vault structure.
- [ ] `BUILD` a schema registry for each record type with required fields, optional fields, privacy level, retention rule, links, evidence references, and last-verified time.
- [ ] `BUILD` atomic Markdown create and update-preview operations, derived SQLite indexing, and explicit JSON Lines export without dual writes or bidirectional synchronization.
- [ ] `BUILD` a migration tool that previews moves between a plain-folder workspace and an Obsidian vault without losing identifiers, links, evidence paths, timestamps, raw notes, or task state.
- [ ] `BUILD` an import checker that detects duplicate people, projects, meetings, tasks, decisions, and documents before adding them.
- [ ] `BUILD` stable local identifiers that survive note renames, folder moves, index rebuilds, and conversation resumes.
- [ ] `BUILD` relationship links between people, meetings, projects, tasks, decisions, correspondence, and supporting files without requiring a graph database.
- [ ] `BUILD` a local workspace dashboard generated from the canonical Markdown records.
- [ ] `BUILD` backup, restore, integrity, migration, and complete-index-rebuild tests for plain folders and Obsidian vaults.
- [ ] `VERIFY` that executive-assistant and secretary acceptance tests pass against both Markdown layouts and that deleting the SQLite index changes no canonical record.

## 9. Retrieval and Grounded Context

Retrieval selects relevant evidence for a question and keeps citations attached to the answer. It exists to reduce invented facts, resolve conflicting notes, and make the assistant explain which local sources support a conclusion.

- [ ] `BUILD` a `KnowledgeAgent` retrieval workflow with bounded queries and evidence-backed results.
- [ ] `BUILD` a `SourceDocument` class with source path, title, text span, line range, timestamp, hash, and authority level.
- [ ] `BUILD` a `ContextQuery` class with question, allowed roots, date bounds, file types, and result budget.
- [ ] `BUILD` exact keyword and phrase search using Ripgrep before semantic search.
- [ ] `BUILD` metadata filtering by project, person, issue, meeting, date, status, and source type.
- [ ] `BUILD` result ranking that prefers current handoffs, authoritative source files, and direct evidence.
- [ ] `BUILD` citation formatting for local files and line numbers.
- [ ] `BUILD` a no-evidence response that says the requested fact was not found rather than inventing an answer.
- [ ] `BUILD` conflict detection when two sources disagree.
- [ ] `BUILD` a source freshness check before relying on cached context.
- [ ] `BUILD` bounded context assembly that deduplicates repeated text and preserves citations.
- [ ] `BUILD` optional local embeddings only after exact search is reliable and the embedding model is approved.
- [ ] `CAPABILITY GATE` Keep structural repository mapping and deterministic lexical search primary for code symbols; use semantic retrieval only as an optional complement for concepts and prose after measured fixtures justify it.
- [ ] `CAPABILITY GATE` Permit only an approved, manifest-pinned local embedding and reranking profile whose publisher, lineage, license, model-origin policy, artifact hashes, runtime, and resource use pass the same registry and platform gates as every generative model.
- [ ] `CAPABILITY GATE` Make each semantic index opt-in per workspace, record the exact included roots and files, and require explicit user approval before first construction or scope expansion.
- [ ] `CAPABILITY GATE` Keep embeddings, reranking inputs, index records, and retrieved excerpts on the workstation; prohibit remote embedding, remote reranking, and source upload in local mode.
- [ ] `BUILD` content-hash, source-range, branch, model-manifest, tokenizer, chunker, and index-schema keys so stale or incompatible semantic records are invalidated deterministically.
- [ ] `BUILD` source-range preservation from chunk creation through retrieval and answer assembly so semantic results retain exact local citations.
- [ ] `BUILD` deterministic lexical fallback whenever the semantic model is unavailable, disallowed, incompatible, corrupt, or unable to fit the current hardware budget.
- [ ] `BUILD` visible storage classification and encryption for semantic metadata, or a documented plaintext tradeoff with exact locations, exposure, and deletion behavior when encryption is unavailable.
- [ ] `BUILD` complete inspect, delete, and rebuild controls for every workspace semantic index, including orphan cleanup and proof that deleted source text is no longer retrievable.
- [ ] `VERIFY` structural-only, lexical, semantic, and hybrid retrieval against the same approved corpus, questions, citation checks, latency limits, and stale-index fixtures before semantic retrieval becomes a supported default.
- [ ] `DEFER` a graph database; use Markdown links and SQLite indexes first.
- [ ] `DEFER` a large vector database until measured retrieval failures justify it.

## 10. Operational State, Memory, Handoffs, and Resume

Gemma is stateless between sessions, but persistence must not create competing authorities. AgentMage therefore assigns one canonical representation to each kind of information.

- [ ] `CAPABILITY GATE` Make the encrypted SQLite operational store the sole authority for sessions, objectives, plans, tasks, actions, evidence pointers, capability grants, receipts, checkpoints, decisions, file observations, and resume state.
- [ ] `CAPABILITY GATE` Make user-authored Markdown the sole authority for human-owned knowledge and approved portable long-term memory beginning with the v0.2 Knowledge pack; operational state never depends on Markdown.
- [ ] `CAPABILITY GATE` Treat JSON Lines only as a versioned audit or export stream generated from canonical SQLite records; never read it as a co-authoritative operational database and never dual-write it in the action transaction.
- [ ] `CAPABILITY GATE` Use one SQLite writer, write-ahead logging, foreign keys, explicit schema migrations, transactions, and crash-safe checkpoints.
- [ ] `CAPABILITY GATE` In v0.1, persist only the minimum operational state required for one resumable read-only session; long-term memory, conversation branching, and human knowledge ingestion remain outside the release.
- [ ] `CAPABILITY GATE` Start and resume from SQLite checkpoint state, then revalidate current files, workspace, model digest, instructions, and policy before taking another action.
- [ ] `CAPABILITY GATE` Commit task state, consumed grants, receipts, evidence pointers, and the next checkpoint in one transaction so a crash cannot repeat a completed action or separate a claim from its receipt.
- [ ] `CAPABILITY GATE` Keep stored facts as pointers to evidence with hashes and verification times, never as unsupported model assertions.
- [ ] `BUILD` separate normalized tables for sessions, objectives, plans, tasks, actions, evidence, decisions, grants, receipts, checkpoints, files, retention decisions, and migrations.
- [ ] `BUILD` generated `CURRENT.md`, task, decision, handoff, and JSON Lines exports only as labeled, reproducible views; user edits require an explicit import and validation operation before they affect canonical state.
- [ ] `BUILD` a `MemoryRecord` with evidence reference, content hash, created time, last verified time, status, sensitivity, retention class, and expiration.
- [ ] `BUILD` stale-memory warnings and re-verification before use.
- [ ] `BUILD` session summary, resume-card, and handoff export generation from canonical records.
- [ ] `BUILD` a handoff validator that checks required fields and referenced evidence before import or resume.
- [ ] `BUILD` explicit user controls to inspect, correct, supersede, expire, export, or delete eligible local records.

## 10A. Context Window and Conversation Continuity

This section controls what information is placed into each model request as conversations grow. It exists to preserve the newest request and authoritative evidence while removing repetition before the model reaches its context limit.

- [ ] `CAPABILITY GATE` Build a `ContextManager` that decides what Gemma receives on every model call.
- [ ] `CAPABILITY GATE` Always preserve the newest user request, active objective, active plan step, user corrections, approval state, open blockers, and recent tool results.
- [ ] `CAPABILITY GATE` Condense older conversation content into a checked summary before the model's context limit is reached.
- [ ] `CAPABILITY GATE` Preserve exact file paths, errors, identifiers, commands, decisions, and unresolved questions during condensation.
- [ ] `CAPABILITY GATE` Keep tool receipts and source links separate from conversational summaries so claims can still be checked.
- [ ] `CAPABILITY GATE` Reopen the original source when a summary is insufficient, stale, disputed, or missing required detail.
- [ ] `BUILD` a token-budget estimator for instructions, user messages, retrieved sources, tool results, memory, and expected output.
- [ ] `BUILD` priority rules that remove duplicate and low-value text before removing authoritative evidence.
- [ ] `BUILD` a summary schema covering objective, user intent, decisions, completed work, remaining work, blockers, approvals, changed files, checks, and next action.
- [ ] `BUILD` summary versioning and source hashes so the agent can tell which conversation and evidence produced a summary.
- [ ] `BUILD` a correction mechanism that replaces a false remembered claim while retaining a record that it was corrected.
- [ ] `BUILD` a context-debug view showing what instructions, memory, sources, and tool results were sent to Gemma for a turn.
- [ ] `BUILD` a maximum-context test that proves the agent can continue after condensation without repeating completed work or losing the active request.

## 10B. Tiered Rolling Memory and Portability

This section defines the memory files as one rolling-memory system. It exists so the agent retains the right information across turns and restarts while keeping temporary chat, durable facts, user preferences, procedures, and project state separate and traceable.

- [ ] `CAPABILITY GATE` Separate working memory for the active turn and plan from durable memory; working memory may be compacted or discarded when the task ends.
- [ ] `CAPABILITY GATE` Keep episodic memory as a dated record of sessions, actions, outcomes, errors, approvals, and handoffs.
- [ ] `CAPABILITY GATE` Keep semantic memory as source-backed facts, relationships, decisions, and evidence pointers rather than free-floating model summaries.
- [ ] `CAPABILITY GATE` Keep procedural memory as versioned instructions, skills, tool contracts, and safety rules that are loaded by scope and precedence.
- [ ] `CAPABILITY GATE` Keep preference memory as user-approved communication, formatting, and workflow preferences, separate from facts about projects or people.
- [ ] `CAPABILITY GATE` Namespace every memory record by workspace, project, conversation, sensitivity, and source so material cannot leak into an unrelated task.
- [ ] `BUILD` a memory-candidate pipeline that extracts a possible memory, checks its source, detects secrets, assigns confidence and expiry, and accepts it only under the configured policy.
- [ ] `BUILD` per-session controls for using existing memory, contributing new memory, or running with memory disabled.
- [ ] `BUILD` an external-context policy that can prevent web, connector, or third-party content from becoming durable memory automatically.
- [ ] `BUILD` contradiction handling that preserves both claims, identifies their sources and dates, marks the conflict, and asks for or records a resolution instead of silently choosing one.
- [ ] `BUILD` consolidation that runs at a meaningful checkpoint or after an idle period, never while a task is still changing, and records which model and sources produced the consolidated entry.
- [ ] `BUILD` memory decay and retention rules that retain decisions and evidence while expiring temporary status, duplicated summaries, and obsolete working context.
- [ ] `BUILD` an inspectable memory view showing why each record was retrieved, when it was last verified, what superseded it, and how to correct or delete it.
- [ ] `BUILD` encrypted, versioned export and import for memory, tasks, decisions, handoffs, indexes, and configuration without machine-specific paths or secrets.
- [ ] `BUILD` rolling-memory recovery tests for interrupted writes, corrupt indexes, stale summaries, conflicting facts, project isolation, backup restoration, and migration between machines.

## 10C. Local Conversation Library and Exact-Point Resume

This section preserves full local conversations and lets the user find and continue earlier work. It exists so the command-line, desktop, and editor interfaces can answer requests such as: "Show me my conversations from August 7, open this one, and continue from that exact point."

- [ ] `CAPABILITY GATE` Save persisted conversations only in the encrypted canonical SQLite store; never use Python Pickle or another executable loading format.
- [ ] `CAPABILITY GATE` Store the conversation identifier, title, created and updated times, local timezone, workspace, project, model, status, parent conversation, and current turn.
- [ ] `CAPABILITY GATE` Classify each turn before persistence; store exact text only when session persistence is enabled, store attachment references and hashes rather than duplicate contents, and refer to grants and receipts by stable identifier.
- [ ] `CAPABILITY GATE` Create a checkpoint at every safe turn boundary containing the active objective, plan, next action, working directory, repository and branch when applicable, permission profile, instruction versions, model profile, evidence pointers, and unresolved questions.
- [ ] `CAPABILITY GATE` List and filter conversations by exact date or date range, title, text, workspace, project, model, status, and tag without sending the query or results outside the computer.
- [ ] `CAPABILITY GATE` Open a selected conversation in read-only history view and show its full timeline, summaries, checkpoints, tool receipts, and changed local files.
- [ ] `CAPABILITY GATE` Resume the latest checkpoint in the same conversation and append new turns without losing or rewriting the earlier transcript.
- [ ] `ROADMAP` Resume from a selected historical turn by creating a visible local branch from that checkpoint, preserving the original conversation unchanged.
- [ ] `CAPABILITY GATE` Before an exact-point resume, check whether referenced files, instructions, repository state, model, or permissions changed; show the differences and require the user to choose whether to continue with current or recorded state.
- [ ] `CAPABILITY GATE` Require every shell to use the same kernel and canonical conversation store; interfaces never read or write the database directly.
- [ ] `BUILD` automatic local titles and user-controlled rename, pin, archive, tag, export, and delete actions; deletion must show the exact local records and require approval.
- [ ] `BUILD` local full-text search across user and assistant messages, summaries, file names, errors, decisions, and tool receipts, with bounded result previews.
- [ ] `BUILD` a natural-language local query parser for requests such as "show my conversations from August 7," with the parsed date range and filters shown before opening anything.
- [ ] `BUILD` a conversation relationship view showing original conversation, resumed branch, superseded summary, handoff, and related project without requiring a graph database.
- [ ] `BUILD` local JSON Lines export generated from canonical records using a non-executable versioned schema, hashes, and encryption; exclude file contents and secrets unless the user explicitly includes them.
- [ ] `BUILD` conversation and task forks from an exact receipt or checkpoint while preserving the original branch, evidence identifiers, citation identifiers, and source hashes unchanged.
- [ ] `BUILD` compacted summaries that preserve citation and receipt identifiers and let the user reopen the exact supporting evidence rather than replacing it with unsupported prose.
- [ ] `BUILD` an encrypted local session archive with visible retention, inspect, restore, export, and delete controls.
- [ ] `BUILD` a redacted portable evidence bundle containing claims, evidence states, citations, receipts, methods, model manifests, constraints, and approved excerpts needed to review or continue work without copying the full private session.
- [ ] `CAPABILITY GATE` Exclude secrets, credentials, unrelated private text, hidden prompts, and unapproved file contents from every session export or evidence bundle; show an exact preview before creation.
- [ ] `BUILD` conversation retention controls that preserve required evidence and decisions while letting the user expire low-value chat text under a visible policy.
- [ ] `VERIFY` exact-point resume after process termination, model change, moved workspace, stale file, corrupt index, interrupted write, and simultaneous interface access.

## 10D. Human-Readable Memory Files and Frontier Interoperability

This v0.2-and-later section stores approved human-owned knowledge and portable long-term memory as ordinary Markdown that the user can read, edit, search, and open in Obsidian. Markdown is authoritative only for that knowledge domain and never for operational sessions, grants, tasks, receipts, or checkpoints.

- [ ] `CAPABILITY GATE` Keep a long-term memory index file (`MEMORY.md`) of one-line entries that link Obsidian-style to per-topic memory files, each carrying tags so items stay searchable and generalizable.
- [ ] `CAPABILITY GATE` Append long-term memories only through the memory-candidate pipeline; supersede outdated entries instead of rewriting or deleting them silently.
- [ ] `CAPABILITY GATE` Keep a short-term working-memory file (`WORKING.md`) with wiki links and tags pointing into long-term files, governed by a hard token budget so it always loads completely into every model turn.
- [ ] `CAPABILITY GATE` Compact the short-term file at task end: promote durable facts through the candidate pipeline, then clear the remainder.
- [ ] `CAPABILITY GATE` Load long-term memory selectively by tag, wiki link, and index search under the context manager's budget — never wholesale.
- [ ] `BUILD` a separate SQLite knowledge index strictly as a regenerable search layer over Markdown; rebuilding or deleting this index must never alter canonical Markdown or the operational database.
- [ ] `BUILD` one shared tag and wiki-link grammar across memory files, the vault, tasks, and handoffs so a single search spans all of them.
- [ ] `ROADMAP` an `AGENTMAGE-FRONTIER.md` instruction file for the v0.5 Frontier Consultation pack.
- [ ] `ROADMAP` frontier edit rules that treat all returned content as untrusted input and prohibit direct changes to canonical operational or knowledge state.
- [ ] `BUILD` a memory-file lint for broken wiki links, duplicate or orphaned tags, an oversized short-term file, and stale last-verified dates.
- [ ] `RECOMMENDED DEFAULT` Review newly promoted long-term memories at session end so promotion stays a deliberate act rather than an accumulation.

## 10E. Data Classification, Encryption, and Retention

Classification happens before persistence, logging, model context reuse, export, or indexing. The strictest applicable class wins.

| Class | Default persistence | Storage | Default retention |
|---|---|---|---|
| Ephemeral | Never | Memory only | End of turn or process termination |
| Operational | Yes when session persistence is enabled | Encrypted SQLite | 30 days after session closure |
| Durable | Only after explicit user promotion | Approved Markdown or encrypted SQLite according to domain | Until superseded or deleted by user policy |
| Restricted | No | Encrypted SQLite only after explicit confirmation; never Markdown, logs, or exports | User-selected expiry, maximum 7 days by default |

- [ ] `CAPABILITY GATE` Run classification, secret detection, minimization, retention assignment, and encryption selection before the first durable write.
- [ ] `CAPABILITY GATE` Refuse to persist private or restricted data when the encryption key or encrypted store is unavailable; offer ephemeral operation instead of plaintext fallback.
- [ ] `CAPABILITY GATE` Do not persist raw attachments, full tool output, environment variables, prompts, or model responses by default; retain hashes, bounded evidence excerpts, and receipt metadata only when sufficient.
- [ ] `CAPABILITY GATE` Store encryption keys only through macOS Keychain or Linux Secret Service via the platform adapter and keep them out of configuration, environment dumps, logs, model context, exports, and backups.
- [ ] `BUILD` enforce expiration during startup and daily maintenance, with legal or user holds represented explicitly rather than silently defeating retention.
- [ ] `BUILD` use cryptographic erasure for encrypted records and document that secure physical overwrite cannot be guaranteed on solid-state or copy-on-write storage.
- [ ] `BUILD` retain content-free performance metrics for 30 days, bounded tool excerpts for 7 days, operational sessions for 30 days, and grants and receipts for 90 days unless the user selects a shorter policy.
- [ ] `BUILD` a pre-persistence receipt recording class, fields removed, encryption state, retention deadline, and policy decision without retaining removed sensitive values.

## 11. Task and Plan Management

Task management connects requests to owners, dependencies, evidence, priorities, and completion states. It exists so the assistant can maintain a useful work list and produce accurate standup, handoff, and daily-report material.

- [ ] `BUILD` a `TaskItem` class with identifier, text, status, owner, project, source, evidence, dependencies, due date, and next action.
- [ ] `BUILD` a `TaskStore` that can read and update a bounded local task file.
- [ ] `BUILD` one-in-progress-step enforcement for active plans.
- [ ] `BUILD` task dependency and blocker tracking.
- [ ] `BUILD` duplicate-task detection across daily notes and the master task list.
- [ ] `BUILD` task status transitions with evidence requirements.
- [ ] `BUILD` source links back to the exact meeting, email, issue, or note that created the task.
- [ ] `BUILD` daily priority ranking using urgency, importance, dependency, meeting time, and user selection.
- [ ] `BUILD` a concise task-summary view and a daily-status-report view.
- [ ] `BUILD` a deferred-work section that does not clutter the active list.
- [ ] `BUILD` Daily Setup, Daily Briefing, and Issue Intake skill specifications.
- [ ] `BUILD` file-based task workflows with stable identifiers, status transitions, and evidence links.

## 11A. Executive Assistant and Chief-of-Staff Functions

This section turns the local agent into a dependable executive assistant using either Obsidian or the storage-neutral workspace. It exists to help the user see priorities, prepare for decisions, remember commitments, and communicate clearly while leaving final authority with the user.

- [ ] `BUILD` a morning briefing containing today's schedule from user-provided local records, top priorities, deadlines, waiting items, unresolved decisions, required preparation, and the next useful action.
- [ ] `BUILD` an end-of-day closeout containing completed work with evidence, unfinished work, changed priorities, new commitments, tomorrow's preparation, and items needing a handoff.
- [ ] `BUILD` a weekly review containing accomplishments, missed commitments, decisions, risks, upcoming meetings, aging tasks, project movement, and recommended focus for the next week.
- [ ] `BUILD` a portfolio view that summarizes each active project by purpose, owner, status, next milestone, dependencies, decisions, risks, and next action.
- [ ] `BUILD` a commitment tracker that answers who promised what, to whom, when, from which source, by what date, and whether the commitment was completed, changed, or superseded.
- [ ] `BUILD` a decision tracker that separates a confirmed decision from a proposal, preference, question, assumption, or reported statement.
- [ ] `BUILD` a waiting-for list containing the person or source, requested item, request date, expected response, latest follow-up, effect of delay, and next follow-up date.
- [ ] `BUILD` an approval tracker containing the item, exact version, required approver, requested date, response, conditions, and proof of approval.
- [ ] `BUILD` a priority assistant that compares urgency, importance, dependencies, meeting dates, estimated effort, user preference, and consequences of delay while showing why it ranked each item.
- [ ] `BUILD` a meeting-preparation brief containing purpose, desired outcome, attendees and roles, relevant history, prior decisions, open questions, risks, proposed agenda, and files to review.
- [ ] `BUILD` a decision brief containing the question, background, options, evidence, assumptions, tradeoffs, risks, recommendation, dissent, and decision deadline.
- [ ] `BUILD` a person brief containing only approved professional context, recent interactions, open commitments, shared projects, upcoming meetings, and source links.
- [ ] `BUILD` an organization and reporting-line map that marks confirmed, inferred, historical, and unknown relationships separately.
- [ ] `BUILD` a correspondence drafter for email, chat, memorandum, briefing note, request, thank-you, follow-up, escalation, and status update using user-approved tone profiles.
- [ ] `BUILD` a response checker that identifies unanswered questions, accidental commitments, unclear deadlines, missing attachments, unsupported claims, sensitive content, and names needing confirmation.
- [ ] `BUILD` an inbox-triage view for user-provided local messages or exports that classifies action required, response required, decision required, reference only, waiting, duplicate, and uncertain without sending anything.
- [ ] `BUILD` a local reminder system tied to tasks, commitments, approvals, meetings, and waiting items with snooze, reschedule, acknowledge, and complete actions.
- [ ] `BUILD` a workload and calendar-conflict check using user-provided local schedule data, task estimates, deadlines, focus time, and travel or transition time.
- [ ] `BUILD` a briefing-pack generator that gathers approved local source files, produces a table of contents, cites every claim, and marks missing or stale information.
- [ ] `BUILD` a "what changed since" query for a date, meeting, decision, project checkpoint, or prior briefing.
- [ ] `BUILD` a "what am I forgetting" review based on overdue tasks, commitments without owners, decisions without follow-up, meetings without notes, drafts without approval, and projects without next actions.
- [ ] `BUILD` configurable privacy classes for ordinary, private, confidential, and highly restricted records, with separate indexes, retrieval limits, retention, and export rules.
- [ ] `BUILD` an executive-assistant audit view showing which sources produced each briefing, recommendation, reminder, or draft.
- [ ] `DEFER` automatic acceptance of invitations, sending correspondence, making commitments, changing calendars, contacting people, or presenting a recommendation as the user's decision.

## 11B. Secretary and Administrative Operations

This section covers the detailed recordkeeping and follow-up work normally performed by a secretary or meeting coordinator. It exists to make meetings, documents, actions, and correspondence easy to find and hard to misstate.

- [ ] `BUILD` an agenda builder with purpose, requested topics, presenter, time estimate, decision needed, pre-read, and desired outcome for each item.
- [ ] `BUILD` a meeting request draft containing title, purpose, required and optional attendees, date and timezone, duration, location or local reference, agenda, preparation, and expected output.
- [ ] `BUILD` an attendee register that separates invited, accepted, declined, tentative, attended, absent, and unknown states without guessing attendance.
- [ ] `BUILD` live-note capture that preserves raw notes and timestamps separately from cleaned minutes.
- [ ] `BUILD` transcript cleanup with speaker-attribution confidence, unclear-language markers, duplicate removal, and a preserved verbatim source.
- [ ] `BUILD` meeting minutes containing background, attendees, discussion, confirmed decisions, proposed decisions, action items, owners, dates, questions, risks, and next meeting.
- [ ] `BUILD` a rule that an action item without a confirmed owner or date is marked owner unknown or date unknown rather than silently assigned.
- [ ] `BUILD` a meeting closeout that checks for unresolved questions, missing owners, missing dates, promised documents, follow-up messages, approval needs, and filing locations.
- [ ] `BUILD` follow-up correspondence drafts that quote the agreed decision or action accurately and link to the approved meeting record.
- [ ] `BUILD` recurring-meeting continuity that carries forward only still-open actions, unresolved decisions, and requested agenda items while preserving prior minutes unchanged.
- [ ] `BUILD` a document register containing title, purpose, owner, version, status, source path, reviewers, approval state, related meeting, retention class, and content hash.
- [ ] `BUILD` document naming, version, superseded-copy, final-copy, and duplicate detection rules that work in either a plain folder or Obsidian workspace.
- [ ] `BUILD` a correspondence register containing sender, recipients, date, subject, project, requested action, commitment, deadline, response state, attachment names, and source path.
- [ ] `BUILD` template-based letters, memoranda, agendas, minutes, briefing notes, action logs, decision logs, and routing slips with exact preview before saving.
- [ ] `BUILD` a local mail-merge preview from approved contact and template records without sending, uploading, or exposing the recipient list.
- [ ] `BUILD` deadline and suspense tracking that records the source, responsible person, review chain, reminder schedule, submitted version, and completion evidence.
- [ ] `BUILD` records filing suggestions based on record type, project, date, retention rule, and source while requiring approval before any move or rename.
- [ ] `BUILD` a quality check for dates, names, titles, numbering, attachments, cross-references, broken links, missing signatures, and inconsistent versions.
- [ ] `BUILD` local calendar-file generation for proposed meetings and reminders without modifying a live calendar or sending invitations.
- [ ] `DEFER` unattended scheduling, mass correspondence, automatic recipient selection, automatic records disposition, and silent changes to approved minutes or final documents.

## 12. Shell and Process Runner

The process runner is the controlled bridge to command-line programs. It exists to validate literal commands, limit time and output, prevent accidental shell expansion, and produce a receipt for every execution.

- [ ] `BUILD` a `CommandSpec` class with executable, literal arguments, working directory, timeout, environment allowlist, risk, and approval requirement.
- [ ] `BUILD` a `CommandRunner` that never invokes a shell when a direct executable call will work.
- [ ] `BUILD` a command allowlist beginning with `pwd`, `rg`, `rg --files`, `git status`, `git branch`, and `git diff`.
- [ ] `BUILD` exact argument validation rather than prefix-only command approval.
- [ ] `BUILD` working-directory enforcement and path canonicalization.
- [ ] `BUILD` environment-variable filtering so secrets are not inherited automatically.
- [ ] `BUILD` standard-output, standard-error, exit-code, elapsed-time, and truncation capture.
- [ ] `BUILD` interactive-process refusal unless a dedicated safe adapter exists.
- [ ] `BUILD` timeout termination and child-process cleanup.
- [ ] `BUILD` a command preview shown before approval.
- [ ] `BUILD` a command receipt shown after execution.
- [ ] `BUILD` harmless synthetic command tests before repository commands are enabled.
- [ ] `DEFER` unrestricted shell access.

## 13. Git and Source-Control Tools

This section covers read-only repository inspection and later approval-gated source-control actions. It exists to preserve user changes, make diffs reviewable, and enforce signed-commit and publication rules instead of treating Git as an unrestricted command surface.

- [ ] `BUILD` `git_status()` using porcelain output without modifying the repository.
- [ ] `BUILD` `git_current_branch()` and upstream detection.
- [ ] `BUILD` `git_branch_list()` for local and remote branch discovery.
- [ ] `BUILD` `git_log()` with bounded commit count.
- [ ] `BUILD` `git_diff()` and `git_diff_staged()` with file and byte limits.
- [ ] `BUILD` `git_show(commit)` for read-only commit inspection.
- [ ] `BUILD` `git_worktree_list()` and worktree-to-branch mapping.
- [ ] `BUILD` dirty-tree and untracked-file detection before any branch operation.
- [ ] `BUILD` changed-file classification by issue, generated output, user work, and unrelated work.
- [ ] `BUILD` source-control safety packages containing status, diffs, conflict versions, hashes, and restoration instructions.
- [ ] `BUILD` patch, conflict-snapshot, and `SHA256SUMS` behavior with deterministic tests.
- [ ] `BUILD` staged-diff checking equivalent to `git diff --cached --check`.
- [ ] `BUILD` exact commit-message drafting from the approved change set.
- [ ] `BUILD` hardware-backed and OpenPGP signing-configuration inspection.
- [ ] `BUILD` signed-commit verification equivalent to `git verify-commit`.
- [ ] `BUILD` a manual approval pause before every commit.
- [ ] `BUILD` a second manual approval pause before every push that updates an open pull request.
- [ ] `BUILD` a source-control hosting publication preview with exact title/body/comment/review-state changes.
- [ ] `DEFER` automatic commit, push, pull-request creation, issue editing, review submission, merge, reset, checkout-discard, and force operations.

## 13A. Remote Repositories, Hosting, and Isolated Worktrees

This section extends local Git inspection to approved remote repositories and separate working copies. It exists so the agent can obtain current code and review hosted changes without overwriting local work, leaking credentials, or publishing anything without approval.

- [ ] `BUILD` remote discovery for repository URLs, default branch, upstream branch, fetch state, and local-versus-remote divergence.
- [ ] `BUILD` approval-gated cloning into an empty, user-selected directory using only approved local Secure Shell agent, credential-helper, or operating-system keychain access.
- [ ] `BUILD` read-only `fetch --prune` support that updates remote references without merging, rebasing, switching branches, or changing the working tree.
- [ ] `BUILD` a currentness report that shows ahead, behind, diverged, stale, and uncommitted states before any pull or branch action.
- [ ] `BUILD` an approval-gated fast-forward-only pull and refuse automatic reset, stash, rebase, conflict resolution, or discard.
- [ ] `BUILD` read-only source-control-host inspection for issues, pull requests, checks, reviews, releases, and exact commit links with freshness timestamps.
- [ ] `BUILD` isolated Git worktrees for separate tasks, with one branch per worktree, visible ownership, disk limits, and no automatic copying of ignored secret files.
- [ ] `CAPABILITY GATE` Treat a Git worktree only as change and concurrency isolation; it supplements but never replaces operating-system sandboxing, path boundaries, capability grants, secret controls, or network policy.
- [ ] `BUILD` temporary coding worktrees that remain separate from the user's active checkout and preserve its branch, index, untracked files, and unfinished changes.
- [ ] `BUILD` per-worktree records for task identity, source commit, branch, owner, grants, file ownership, active processes, resource budgets, retention, cleanup, and final disposition.
- [ ] `BUILD` deterministic collision checks before change transfer so overlapping edits, renamed paths, changed preimages, and concurrent user changes stop for review instead of being silently combined.
- [ ] `BUILD` worktree snapshots and handoff records so a deleted or moved worktree can be restored safely.
- [ ] `BUILD` an explicit merge-back or patch-transfer review that shows every change before it reaches the main checkout.
- [ ] `BUILD` credential redaction and a rule that repository credentials never enter model context, configuration, memory, command output, or audit logs.
- [ ] `BUILD` remote operation receipts containing the host, repository, action, commit identifiers, time, result, and whether the working tree changed.
- [ ] `DEFER` automatic issue or pull-request publication, review submission, merge, release, branch deletion, and force push.

## 13B. Complete GitHub Integration

This section defines the hosted GitHub capabilities that sit above ordinary local Git commands. It exists so the agent can understand repositories, issues, pull requests, reviews, checks, and security findings while keeping credentials protected and every external change reviewable and approval-gated.

- [ ] `CAPABILITY GATE` Begin GitHub and every external connector with an explicit user-initiated, read-only synchronization into a sensitivity-labeled local cache; no startup polling, silent refresh, or write capability belongs in the first connector phase.
- [ ] `CAPABILITY GATE` Make network activation visible, temporary, destination-scoped, cancellable, and receipted; return to the offline baseline when the bounded synchronization ends.
- [ ] `CAPABILITY GATE` Treat issue text, pull requests, comments, workflow output, repository instructions, and all other connector content as untrusted data that cannot modify policy, grants, tool authority, or completion status.
- [ ] `CAPABILITY GATE` Keep connector credentials in macOS Keychain or Linux Secret Service and outside model context; if a protocol requires delegation, provide only a bounded, short-lived derived token to the connector process and never the model.
- [ ] `BUILD` cache records for classification, source host, immutable object identity, freshness, retention, deletion, content hash, and the receipt that imported each object.
- [ ] `BUILD` a provider adapter supporting GitHub.com and user-approved GitHub Enterprise hosts without sending one host's credentials to another.
- [ ] `BUILD` read-only access through an approved GitHub command-line client, REST application programming interface, or GraphQL application programming interface behind one normalized tool contract.
- [ ] `BUILD` authentication through an approved Secure Shell agent, operating-system credential helper, fine-grained personal access token, or GitHub App installation; never store tokens in prompts, memory, logs, repositories, or plain-text configuration.
- [ ] `BUILD` authentication and permission diagnostics that show the active host, account or app, installation, granted repositories, scopes, expiration, single-sign-on state, and missing permission without showing secret values.
- [ ] `BUILD` organization, user, team, repository, visibility, archived-state, fork, template, language, topic, license, default-branch, and last-update discovery within the granted scope.
- [ ] `BUILD` bounded search across repository names, descriptions, code, paths, commits, branches, tags, releases, issues, pull requests, discussions, and users where the host permits it.
- [ ] `BUILD` repository-content reading for files, directories, symbolic links, submodules, large-file pointers, raw blobs, commit trees, and line-specific permanent links.
- [ ] `BUILD` branch, tag, release, commit, author, parent, signature, comparison, and ancestry inspection with exact immutable commit identifiers.
- [ ] `BUILD` issue inspection covering title, body, author, state, type, labels, assignees, milestone, project fields, comments, reactions, linked branches, linked pull requests, dependencies, timeline events, and closing reason.
- [ ] `BUILD` issue search and triage views for unassigned, stale, blocked, duplicate, dependency-linked, recently changed, and user-selected work without changing hosted state.
- [ ] `BUILD` pull-request inspection covering base and head branches, fork status, commits, changed files, patch, comments, review threads, approvals, requested changes, requested reviewers, labels, milestone, linked issues, draft state, mergeability, and update status.
- [ ] `BUILD` checks inspection covering required checks, status contexts, workflow runs, jobs, steps, annotations, test summaries, logs, artifacts, attempts, cancellations, reruns, and the exact commit tested.
- [ ] `BUILD` branch-protection, repository-ruleset, required-review, required-check, signed-commit, linear-history, merge-method, and deletion-policy inspection.
- [ ] `BUILD` `CODEOWNERS`, repository instructions, contribution guides, pull-request templates, issue templates, security policies, support files, and nested instruction discovery before proposing or reviewing changes.
- [ ] `BUILD` read-only security inspection for dependency alerts, code-scanning alerts, secret-scanning alerts, repository security advisories, dependency graph, dependency review, and software-bill-of-materials data when permission exists.
- [ ] `BUILD` release inspection covering release notes, tags, assets, checksums, provenance, pre-release state, publication time, and comparison with the previous release.
- [ ] `BUILD` GitHub Actions workflow inspection covering triggers, permissions, environments, secrets referenced by name, variables, concurrency, reusable workflows, and artifact retention without exposing secret values.
- [ ] `BUILD` notification and subscription inspection for review requests, assignments, mentions, failing checks, release events, and watched repositories, with local deduplication and read state.
- [ ] `BUILD` pagination, conditional requests, caching, rate-limit inspection, abuse-limit handling, retry-after support, and freshness timestamps for every hosted result.
- [ ] `BUILD` webhook or bounded polling ingestion for issue, pull-request, review, check, workflow, release, and security events, with signature verification and replay protection.
- [ ] `BUILD` stable links between a local repository, remote host, issue, pull request, branch, worktree, commit, task record, decision record, and local evidence receipt.
- [ ] `BUILD` pull-request checkout into an isolated worktree using immutable refs, followed by status, dependency, instruction, and test discovery before any edit.
- [ ] `BUILD` local comparison between the pull-request branch and its current base, including merge-base detection, base movement, stale review findings, and changes added after the last review.
- [ ] `BUILD` repository-aware code review using scoped instruction files, changed-code context, relevant tests, dependency impact, severity, confidence, and exact line links.
- [ ] `BUILD` an optional security-review pass using the diff, surrounding code, configured threat model, dependency changes, permissions, data boundaries, and exploitability evidence.
- [ ] `BUILD` review-noise controls that suppress formatting-only comments, duplicate findings, stale findings, low-confidence speculation, and issues already enforced deterministically by continuous integration.
- [ ] `BUILD` suggested fixes as local patches first, with affected files, tests, risk, rollback, and validation shown before any branch update.
- [ ] `BUILD` a draft issue package containing exact title, body, labels, assignees, milestone, project fields, links, and attachments for user review before publication.
- [ ] `BUILD` a draft pull-request package containing exact base, head, title, body, linked issues, reviewers, labels, draft state, commits, checks, and changed-file summary for user review before publication.
- [ ] `BUILD` a draft review package containing exact inline comments, summary, approval or change-request state, severity, evidence, and file-line positions for user review before submission.
- [ ] `BUILD` approval-gated issue creation and updates, including comments, labels, assignment, milestones, project fields, state changes, links, transfer, lock, and reopen or close actions.
- [ ] `BUILD` approval-gated pull-request creation and updates, including body edits, comments, review requests, labels, milestones, draft conversion, branch updates, and closure.
- [ ] `BUILD` approval-gated review submission, review-thread replies, thread resolution, approval, change request, and review dismissal with the exact hosted effect shown first.
- [ ] `BUILD` approval-gated workflow dispatch, failed-job rerun, cancellation, environment approval, and artifact download with the workflow, ref, inputs, and side effects shown first.
- [ ] `BUILD` approval-gated commits and pushes that enforce the repository's signing method, show the exact staged diff and message, verify the resulting signature, and require a separate approval before updating a remote branch.
- [ ] `BUILD` approval-gated fixes from review findings that re-read the current branch, refuse unrelated changes, run the applicable checks, and show the new diff before commit or push.
- [ ] `BUILD` approval-gated merge preparation showing current approvals, required checks, unresolved threads, branch protection, merge method, resulting commit, release impact, and branch-deletion choice.
- [ ] `BUILD` approval-gated release drafting with tag, target commit, generated or edited notes, assets, checksums, provenance, pre-release state, and publication preview.
- [ ] `BUILD` a complete audit receipt for every GitHub read and write containing host, repository, actor, object, immutable identifiers, request type, time, result, rate-limit state, and whether external state changed.
- [ ] `BUILD` idempotency and duplicate protection for comments, labels, issue creation, pull-request creation, workflow dispatch, releases, and retries after an uncertain network result.
- [ ] `BUILD` failure recovery for expired credentials, missing single sign-on, revoked app installation, renamed or transferred repository, deleted branch, changed permissions, moved line positions, base-branch movement, and partial publication.
- [ ] `DEFER` automatic reviews, automatic fixes, automatic issue or pull-request updates, automatic merges, automatic releases, branch deletion, repository administration, secret changes, ruleset changes, and force pushes until each action has a separately approved policy and proven rollback path.

## 14. Codebase Understanding and Editing

These capabilities help the assistant learn a repository before proposing or applying a change. They exist to connect issues, symbols, tests, dependencies, and patches while keeping broad rewrites and autonomous refactoring out of scope.

- [ ] `BUILD` a `RepositoryProfile` containing languages, package managers, entry points, test commands, build commands, linters, type checks, and instruction files.
- [ ] `BUILD` language and framework detection from manifests and file extensions.
- [ ] `BUILD` entry-point discovery for Python, TypeScript, JavaScript, Go, Terraform, Structured Query Language, shell, and notebooks.
- [ ] `BUILD` symbol search using Ripgrep first.
- [ ] `BUILD` on the v0.1 parser-backed repository map with language-server and language-native analysis only after the Coding pack grants those separately confined processes read authority.
- [ ] `BUILD` dependency-map extraction from imports, manifests, routes, tests, and configuration.
- [ ] `BUILD` test-to-source mapping.
- [ ] `BUILD` issue-to-code evidence maps based on names, routes, services, schemas, and tests.
- [ ] `BUILD` a read-only repository-learning report covering entry points, architecture, dependencies, tests, operations, and inspection coverage.
- [ ] `BUILD` patch proposals that name files, symbols, behavior, tests, risk, and rollback before editing.
- [ ] `BUILD` exact-preimage patches so edits fail if the file changed after review.
- [ ] `BUILD` post-edit formatting, type-check, test, build, and diff verification selected from repository evidence.
- [ ] `BUILD` changed-file review using code-simplicity, architecture, security, data-integrity, accessibility, and performance checklists when relevant.
- [ ] `BUILD` a regression-test requirement for every reproduced defect before its corrective patch is accepted.
- [ ] `DEFER` broad repository rewrites and autonomous refactoring.

## 14A. Deep Repository Comprehension

This later section expands the bounded v0.1 repository map into deeper repository comprehension before the assistant proposes a change. It exists to support questions such as where a feature starts, how data moves, which permissions apply, what tests protect it, and what will break if a file changes.

- [ ] `CAPABILITY GATE` Produce a repository coverage report listing files discovered, files read, files indexed, files skipped, generated or vendor folders excluded, byte and token limits, errors, and remaining blind spots.
- [ ] `CAPABILITY GATE` Never claim to understand an entire repository unless the coverage report shows the agreed scope was read or indexed successfully.
- [ ] `BUILD` an incremental local repository index keyed by file hash, Git commit, language, symbol, and parser version so unchanged files are not reread unnecessarily.
- [ ] `BUILD` parser adapters using language-native parsers, tree-sitter, or language-server information for definitions, references, imports, calls, types, diagnostics, and symbols.
- [ ] `BUILD` a repository map containing packages, applications, services, libraries, entry points, configuration, scripts, tests, documentation, generated code, and external dependencies.
- [ ] `BUILD` an architecture view that explains runtime components, process boundaries, storage, queues, scheduled jobs, user interfaces, and external systems with exact file citations.
- [ ] `BUILD` an end-to-end feature trace from user action or command through interface, route, controller, service, domain logic, database or file access, background work, response, and tests.
- [ ] `BUILD` a data-flow trace showing where a record originates, how it is validated and transformed, where it is stored, who can read or change it, and where it leaves the repository boundary.
- [ ] `BUILD` authentication and authorization maps covering identities, roles, permissions, middleware, policy checks, protected routes, administrative exceptions, and tests.
- [ ] `BUILD` schema and migration maps for database tables, fields, relationships, indexes, migrations, seed data, fixtures, and application models.
- [ ] `BUILD` interface maps for routes, commands, events, file formats, environment-variable names, configuration keys, and public library surfaces without exposing secret values.
- [ ] `BUILD` test maps connecting source symbols and user-visible behavior to unit, integration, end-to-end, snapshot, contract, and regression tests.
- [ ] `BUILD` continuous-integration maps showing jobs, triggers, operating systems, language versions, caches, required checks, build outputs, and deployment boundaries from local configuration files.
- [ ] `BUILD` dependency maps covering direct, development, optional, transitive, local, workspace, and cross-repository dependencies with version and lockfile evidence.
- [ ] `BUILD` Git-history views for feature introduction, later changes, deleted behavior, recurring defects, ownership signals, and decision context while warning that commit frequency does not prove current expertise or authority.
- [ ] `BUILD` documentation-drift checks comparing README files, architecture notes, examples, configuration, commands, schemas, and current code.
- [ ] `BUILD` a repository glossary that links business terms, code names, routes, tables, services, jobs, configuration, and tests.
- [ ] `BUILD` question answering with exact file, line, symbol, and immutable commit references plus a visible known, inferred, or unknown label.
- [ ] `BUILD` a large-repository strategy using package boundaries, entry points, change history, dependency neighborhoods, and user-selected slices with a clear partial-coverage warning.
- [ ] `BUILD` cross-repository tracing for shared libraries, clients, schemas, events, deployment files, and version pins using only approved local clones unless remote access is separately enabled.
- [ ] `BUILD` branch-aware indexes so the assistant does not combine behavior from different branches, commits, worktrees, or generated outputs.
- [ ] `BUILD` stale-index detection when files, branches, submodules, generated code, lockfiles, or repository instructions change.
- [ ] `BUILD` repository-learning exports for onboarding guide, architecture overview, dependency diagram, feature trace, test guide, build guide, and open questions.

## 14B. Coding Assistance and Controlled Change Development

This section expands repository reading into careful coding, debugging, and review. It exists so the assistant can propose the smallest defensible change, test it, explain it, and leave the repository in a reviewable state.

- [ ] `CAPABILITY GATE` Observe and hash every target preimage before constructing a change; refuse to plan a write from stale, partial, inaccessible, or policy-excluded source state.
- [ ] `BUILD` every proposed edit first as a shadow change set outside the target files, with stable operation identifiers and exact base hashes.
- [ ] `BUILD` deterministic shadow-patch validation for syntax, path boundaries, encoding, line endings, duplicate operations, generated-file policy, and expected postimage hashes before approval is requested.
- [ ] `CAPABILITY GATE` Show the complete diff, rationale, affected files, behavior change, validation plan, risks, rollback method, and known unverified assumptions before requesting write authority.
- [ ] `CAPABILITY GATE` Require an exact single-use grant naming the approved shadow change set, files, operations, workspace, expiry, and permitted verification; approval for one set never authorizes a regenerated or broader set.
- [ ] `CAPABILITY GATE` Re-read and compare every preimage immediately before application; any mismatch invalidates the grant and returns the task to review.
- [ ] `BUILD` atomic application where supported, with ordered operations and restoration of all already-applied preimages if any operation fails.
- [ ] `BUILD` per-operation receipts with `proposed`, `approved`, `applied`, `verified`, `failed`, `rolled_back`, or `superseded` status and exact preimage and postimage hashes.
- [ ] `CAPABILITY GATE` Treat formatting, tests, lint, builds, migrations, and every other command after a write as separately bounded verification requiring its own approved command template or grant.
- [ ] `BUILD` rollback as a fresh, approval-gated transaction that checks current postimages and concurrent edits before restoring approved preimages; never overwrite later user work automatically.
- [ ] `BUILD` a change-intent record containing the requested behavior, current behavior, evidence, affected users, acceptance checks, exclusions, risks, and rollback before editing.
- [ ] `BUILD` a minimal-change path that identifies the smallest set of files, symbols, tests, configuration, migrations, and documentation likely required.
- [ ] `BUILD` a local reproduction builder that records environment, input, steps, observed result, expected result, logs, and whether the failure was reproduced.
- [ ] `BUILD` debugging support with competing hypotheses, discriminating checks, variable or state tracing, log correlation, and a record of rejected explanations.
- [ ] `BUILD` failing-regression-test creation before a bug fix when the defect can be reproduced safely.
- [ ] `BUILD` test generation based on existing repository style, behavior boundaries, edge cases, failure cases, permissions, data changes, and rollback behavior.
- [ ] `BUILD` abstract-syntax-tree-aware edits for supported languages so structural changes do not rely only on text replacement.
- [ ] `BUILD` language-server-assisted definitions, references, rename previews, diagnostics, and code actions without letting the language server write unapproved changes.
- [ ] `BUILD` atomic multi-file change sets with exact preimages, ordered application, rollback, and post-change hashes.
- [ ] `BUILD` dependency-change review covering purpose, alternatives, license, provenance, maintenance, vulnerabilities, package size, lockfile effect, and removal plan.
- [ ] `BUILD` database and file-format migration planning covering forward change, backward compatibility, existing data, rollback, verification, and partial failure.
- [ ] `BUILD` interface-change checks for callers, clients, schemas, documentation, compatibility, versioning, and contract tests.
- [ ] `BUILD` security review for authentication, authorization, input handling, output encoding, secrets, file paths, command execution, network access, and data exposure when relevant.
- [ ] `BUILD` performance review using measured baselines, profiles, query plans, memory, disk, network, and regression budgets rather than model guesses.
- [ ] `BUILD` accessibility review for user-interface changes using repository standards and deterministic checks where available.
- [ ] `BUILD` formatting, lint, type, unit, integration, end-to-end, build, packaging, and security-check selection based on repository evidence.
- [ ] `BUILD` failure triage that separates a change-caused failure from a baseline failure, flaky test, missing dependency, environment problem, permission problem, or unrelated repository defect.
- [ ] `BUILD` a local review packet containing objective, before-and-after behavior, changed files, diff, tests, checks not run, risks, rollback, screenshots or outputs, and proposed commit groups.
- [ ] `BUILD` logical commit planning that separates behavior, tests, documentation, generated output, formatting, and unrelated user changes before any commit is approved.
- [ ] `BUILD` change documentation for README files, comments, examples, configuration, migration notes, operations, troubleshooting, and handoff when the behavior requires it.
- [ ] `BUILD` repository scaffolding for a new package or application using the selected language's normal structure, tests, formatting, documentation, license, and local development commands.
- [ ] `BUILD` local code-review modes for correctness, simplicity, maintainability, security, data integrity, accessibility, performance, tests, and documentation with duplicate-finding suppression.
- [ ] `VERIFY` coding workflows on fictional repositories for Python, TypeScript, JavaScript, Go, shell, Structured Query Language, and mixed-language projects before enabling edits in a real repository.
- [ ] `DEFER` automatic broad refactors, dependency upgrades, migrations, commits, pushes, releases, deployments, or fixes applied without exact user review.

## 15. Test and Validation Runner

This section turns repository checks into explicit, bounded evidence. It exists to distinguish focused tests from full validation and to prevent the assistant from reporting checks that were not actually run.

- [ ] `BUILD` a `TestCommand` registry per repository.
- [ ] `BUILD` focused-test selection from changed files and existing scripts.
- [ ] `BUILD` fail-fast and last-failed rerun support for test execution.
- [ ] `BUILD` bounded waiting, progress reporting, cancellation, and timeout behavior for long tests.
- [ ] `BUILD` test-result parsing for passed, failed, skipped, duration, and failed test names.
- [ ] `BUILD` separate reporting for unit tests, integration tests, end-to-end tests, lint, types, build, and security scans.
- [ ] `BUILD` a validation receipt that includes exact command, working directory, exit code, and timestamp.
- [ ] `CAPABILITY GATE` Run tests, linters, formatters, type checks, builds, and security scans only from repository-discovered or user-approved trusted command templates, with a separate approval when execution authority was not already granted.
- [ ] `BUILD` validation receipts containing the exact command and arguments, working directory, bounded environment description, start and end times, duration, timeout, cancellation state, exit code, parsed result, bounded redacted output, and affected files or artifacts.
- [ ] `CAPABILITY GATE` Derive validation status only from process and artifact evidence; never treat model narration, expected output, or a planned command as proof that a check ran or passed.
- [ ] `BUILD` a rule that partial validation cannot be reported as a full repository pass.
- [ ] `BUILD` a rule that a test not run is marked unverified.
- [ ] `BUILD` synthetic fixtures for safe initial evaluation.
- [ ] `BUILD` deterministic fake model and fake tool tests.
- [ ] `BUILD` permission-denial, path-traversal, secret-redaction, stale-memory, hallucinated-completion, and approval-bypass tests.

## 16. Markdown and Plain-Text Documents

These tools support the notes, plans, issues, reports, and handoffs that make up most of the workflow. They exist to preserve structure, links, source lines, raw meeting notes, and understandable wording while editing only the intended content.

- [ ] `BUILD` a Markdown parser that preserves headings, lists, tables, code fences, links, frontmatter, and source line numbers.
- [ ] `BUILD` a Markdown writer that preserves existing line endings and unrelated content.
- [ ] `BUILD` CommonMark spacing checks for headings and lists.
- [ ] `BUILD` local-file display-link generation with optional line numbers from validated `WorkspacePath` values; display links carry no tool authority.
- [ ] `BUILD` a plain-language review pass based on configurable language rules.
- [ ] `BUILD` acronym handling that spells out an acronym on first use and does not guess unknown expansions.
- [ ] `BUILD` meeting-note cleanup that preserves `## Raw Notes` and adds summary, decisions, actions, questions, risks, owners, and links.
- [ ] `BUILD` concise daily status-report generation using completed work, next work, and blockers.
- [ ] `BUILD` standup-note and speaking-script generation.
- [ ] `BUILD` evidence-backed task and handoff document generation.

## 17. Word Document Tools

This section covers extracting and generating Word documents without silently changing originals or losing important formatting. It exists because Word files are common document artifacts, but their binary structure requires explicit conversion and visual verification.

- [ ] `VERIFY` an approved macOS-, Fedora-, and Ubuntu-compatible Word-to-text utility for deterministic sidecar conversion.
- [ ] `BUILD` `convert_docx_to_text(source, sidecar)` with timestamp and hash caching.
- [ ] `BUILD` a rule that the original Word file is never modified during extraction.
- [ ] `BUILD` warnings for lost tables, comments, tracked changes, headers, footers, and layout.
- [ ] `BUILD` a Markdown-to-Word generation utility.
- [ ] `BUILD` Word-generation functions `configure_styles()`, `configure_page()`, `configure_header_footer()`, `add_metadata_table()`, `add_warning()`, `add_table_from_markdown()`, `add_decision_cards()`, `add_hyperlink()`, and `build()`.
- [ ] `BUILD` table-layout helpers `set_cell_shading()`, `set_cell_margins()`, `set_table_width()`, `set_table_borders()`, and `set_single_cell_border()`.
- [ ] `BUILD` numbering helpers `add_numbering_definition()` and `apply_numbering()`.
- [ ] `BUILD` paragraph creation, removal, cloning, and section-replacement helpers.
- [ ] `BUILD` a bounded Word document inspector.
- [ ] `BUILD` render-and-verify steps using Quick Look or an approved office converter.
- [ ] `BUILD` page-image comparison for layout quality assurance.
- [ ] `VERIFY` which Python environment contains or may install `python-docx`; it is not available in the current global Python environment.

## 18. Portable Document Format Tools

These tools provide bounded text extraction, page citations, metadata inspection, and visual review for Portable Document Format files. They exist so the assistant can use PDFs as evidence while recognizing scanned pages, layout limits, and possible extraction loss.

- [ ] `BUILD` a local Portable Document Format text extractor with page numbers and bounded output.
- [ ] `BUILD` scanned-page detection.
- [ ] `BUILD` optional local optical character recognition only with an approved package.
- [ ] `BUILD` a Portable Document Format renderer to page images for visual inspection.
- [ ] `BUILD` metadata extraction for title, author, page count, creation date, and encryption state.
- [ ] `BUILD` exact-page citation support.
- [ ] `BUILD` a report exporter for Markdown-to-hypertext-markup-language and Portable Document Format output.
- [ ] `BUILD` exporter functions `parseArgs()`, `escapeHtml()`, `renderInline()`, `parseTable()`, `renderTable()`, `renderMarkdown()`, `makeHtml()`, `findChrome()`, `safeFileName()`, and `main()`.
- [ ] `BUILD` an offline Mermaid renderer; do not retain the current content-delivery-network dependency in the local fallback.
- [ ] `VERIFY` approved local packages because `pdftotext`, `pdfinfo`, `mutool`, `qpdf`, `pypdf`, `pdfplumber`, and `reportlab` are not available in the current global environment.

## 19. Spreadsheet, Comma-Separated Value, and JSON Tools

This section covers the structured data formats used for reconciliation, metadata, reports, and transfer packages. It exists to support read-only inspection, deterministic comparisons, safe output, and validation without overwriting the source workbook or data file.

- [ ] `ROADMAP` Support Excel workbooks (`.xlsx`) and comma-separated value files (`.csv`) as read-only inputs.
- [ ] `ROADMAP` Show workbook names, worksheet names, row and column counts, headers, formulas, displayed values, dates, hyperlinks, hidden rows or columns, and source hashes.
- [ ] `ROADMAP` Support bounded filtering, sorting, matching, duplicate detection, missing-value checks, and comparison between two workbook or comma-separated value sources.
- [ ] `ROADMAP` Preserve the original workbook or comma-separated value file and write any normalized table or comparison to a separate output file.
- [ ] `ROADMAP` Prevent spreadsheet formula injection when generating comma-separated value output.
- [ ] `BUILD` safe comma-separated-value output and formula-injection prevention.
- [ ] `BUILD` `sha256()`, `clean()`, `normalized()`, `normalized_id()`, `normalized_filename()`, `normalized_url()`, `safe_csv_value()`, and `read_csv()` helpers.
- [ ] `BUILD` deterministic matching, overlap, differing-field, and candidate-reason logic.
- [ ] `BUILD` direct Open XML spreadsheet parsing.
- [ ] `BUILD` `shared_strings()`, `style_date_flags()`, `excel_date()`, `numeric_text()`, and `extract_workbook()` helpers.
- [ ] `BUILD` a generic `SpreadsheetReader` for workbook, worksheet, cell, formula, displayed value, style, hyperlink, hidden-row, and hidden-column metadata.
- [ ] `BUILD` a read-only workbook summarizer with row and column limits.
- [ ] `BUILD` safe comma-separated-value parsing and writing.
- [ ] `BUILD` reporting helpers for safe comma-separated-value cells, row conversion, record conversion, and parsing.
- [ ] `BUILD` JSON parsing with schema validation, stable key ordering, and redaction.
- [ ] `BUILD` deterministic spreadsheet comparison with source hashes and match reasons.
- [ ] `BUILD` reconciliation workbook functions `escapeSpreadsheetText()`, `createReconciliationWorkbook()`, and `scanWorkbookFormulaErrors()`.
- [ ] `VERIFY` approved packages because `openpyxl` and `pandas` are not available in the current global Python environment.

## 19A. File-Type Priority

This later-release roadmap sets a practical order for adding file support after v0.1. It prioritizes common formats while deferring risky or rarely needed formats and all embedded code execution.

- [ ] `ROADMAP` Plain text (`.txt`), Markdown (`.md`), JSON (`.json`), Excel (`.xlsx`), comma-separated value (`.csv`), Word (`.docx`), and Portable Document Format (`.pdf`) support.
- [ ] `ROADMAP` File metadata, source hashes, bounded extraction, citations, and safe sidecar output for every supported format.
- [ ] `ROADMAP` PowerPoint (`.pptx`) slide, speaker-note, hyperlink, and image-caption extraction for meeting presentations.
- [ ] `ROADMAP` Hypertext Markup Language (`.html`) and Extensible Markup Language (`.xml`) parsing for saved web pages, exports, and structured data.
- [ ] `ROADMAP` YAML (`.yaml` or `.yml`) parsing for configuration, agent, and skill files, with secret redaction.
- [ ] `ROADMAP` Jupyter Notebook (`.ipynb`) extraction for Markdown, code, outputs, and execution metadata without executing cells.
- [ ] `ROADMAP` log-file support for timestamped text, JSON Lines, stack traces, and bounded event summaries.
- [ ] `ROADMAP` image and screenshot metadata, local visual inspection, and optional local optical character recognition after approval.
- [ ] `DEFER` database dumps, Parquet, Avro, compressed archives, email files, Outlook data files, and proprietary binary formats until a real task requires them.
- [ ] `DEFER` automatic execution of macros, spreadsheet formulas, notebook cells, embedded scripts, or document attachments.

## 20. Images, Screenshots, and Visual Verification

These tools let the assistant inspect screenshots and verify rendered documents or interfaces. They exist because visual layout, permissions errors, and dashboard behavior cannot always be confirmed from extracted text alone.

- [ ] `VERIFY` Quick Look or `open` integration on macOS and `xdg-open` integration on Fedora and Ubuntu for local previews.
- [ ] `VERIFY` approved image metadata and conversion utilities on macOS, Fedora, and Ubuntu without making Homebrew a v0.1 prerequisite.
- [ ] `BUILD` an image-metadata reader for dimensions, type, color space, and file size.
- [ ] `BUILD` a screenshot viewer that can return a bounded local image to a vision-capable model.
- [ ] `BUILD` visual-diff support for before/after document and user-interface screenshots.
- [ ] `BUILD` image redaction before model use when screenshots may contain sensitive information.
- [ ] `DEFER` image generation in the first contingency release.

## 21. Database and Structured-Data Tools

This section separates local memory databases, fixture databases, schema inspection, and any future live database access. It exists to make queries bounded and auditable and to prevent a local assistant from treating a production database as an ordinary writable file.

- [ ] `VERIFY` the embedded SQLite library and optional local command-line diagnostic utility for memory, indexes, and agent state on macOS, Fedora, and Ubuntu.
- [ ] `BUILD` parameterized SQLite read/write helpers with migrations and transactions.
- [ ] `BUILD` a read-only Structured Query Language query policy that permits only explicitly approved data sources and statements.
- [ ] `BUILD` query result limits, timeouts, column redaction, and audit receipts.
- [ ] `BUILD` schema inspection separate from row-data access.
- [ ] `BUILD` database configuration and fixture connections only for synthetic tests, not live access.
- [ ] `BUILD` report-scope and authorization controls.
- [ ] `BUILD` a database-adapter interface so SQLite, PostgreSQL, and fixture files remain separate implementations.
- [ ] `VERIFY` PostgreSQL command-line tools are not installed in the current shell.
- [ ] `DEFER` live external database access until access, policy, environment, and audit requirements are approved.

## 22. Evidence Normalization and Reconciliation

These components turn different source records into comparable evidence with identity, provenance, reason codes, and uncertainty. They exist so the assistant can explain a match or discrepancy rather than merely produce an unexplained result.

- [ ] `BUILD` normalized-evidence, evidence-context, and evidence-kind types.
- [ ] `CAPABILITY GATE` Assign every material answer claim exactly one user-visible state: Observed, Derived, Inferred, or Unknown/Blocked. Model confidence never substitutes for an evidence state.
- [ ] `BUILD` Observed claims only from authorized deterministic tool results with a valid receipt; bind each file citation to workspace-relative path, content hash, line range or structured object identity, and observation time.
- [ ] `BUILD` Derived claims only from recorded deterministic methods and observed inputs; retain the method identifier and input receipt/citation identifiers.
- [ ] `BUILD` Inferred claims as model interpretations that name their supporting citations and never imply mechanical proof.
- [ ] `BUILD` Unknown/Blocked claims for unavailable, denied, failed, stale, conflicting, unverifiable, or out-of-scope evidence with the exact reason exposed.
- [ ] `BUILD` citation resolution that compares current source identity with the observed hash and marks changed or missing evidence stale instead of silently resolving it to new content.
- [ ] `BUILD` an answer-claim ledger connecting rendered statements to evidence state, citations, deterministic method where applicable, model/runtime manifest for inferences, and final completion receipt.
- [ ] `BUILD` normalization for relational records, repository files, archive items, and version-history evidence.
- [ ] `BUILD` safe evidence projection so only approved fields enter model prompts.
- [ ] `BUILD` deterministic document-identity normalization and evidence collection.
- [ ] `BUILD` reason-code dictionaries and explanation helpers.
- [ ] `BUILD` validation and construction of evidence-backed investigation steps.
- [ ] `BUILD` archive-evidence and version-history review behavior for deterministic verification.
- [ ] `BUILD` lifecycle-review logic for explicit include, exclude, defer, and review decisions.
- [ ] `BUILD` explanation-provider interfaces and deterministic fake providers for isolation and tests.
- [ ] `BUILD` explanation-request construction, eligibility checks, response validation, caching, and bounded execution.
- [ ] `BUILD` an evidence model that is not tied to any application schema or document rules.
- [ ] `BUILD` provenance, source hash, match reason, confidence, uncertainty, and reviewer-decision fields.

## 23. Logging, Audit, and Observability

This section records what the assistant did, when it did it, and what evidence it received. It exists for debugging, truthful completion claims, review, and recovery without storing secrets or unnecessarily retaining sensitive prompts.

- [ ] `BUILD` log-level, log-entry, log-metadata, and logger types.
- [ ] `BUILD` `generateCorrelationId()`, `createRequestLogger()`, and `createServiceLogger()`.
- [ ] `BUILD` structured audit events in the canonical encrypted SQLite store with timestamps, correlation identifiers, schema versions, and retention classes.
- [ ] `BUILD` separate model-call, tool-call, approval, file-change, command, validation, and error events.
- [ ] `BUILD` content-free model-call logging by default; persisted conversation text follows the explicit session-persistence and classification policy rather than being copied into logs.
- [ ] `BUILD` secret redaction before persistence.
- [ ] `BUILD` append-only action receipts with previous-record hashes and the keyed integrity anchor defined by the security contract.
- [ ] `BUILD` session metrics for model latency, token estimates, tool time, retries, failures, and files read.
- [ ] `BUILD` a human-readable session recap generated from the audit log.
- [ ] `BUILD` retention enforcement and cryptographic deletion according to the data-classification policy.
- [ ] `BUILD` JSON Lines audit export only as an explicit, versioned, derived operation; never treat exported logs as canonical state.
- [ ] `BUILD` an error taxonomy that separates user error, missing access, policy denial, tool failure, model failure, timeout, and invalid output.

## 24. Queues, Jobs, and Controlled Retries

These capabilities support long-running or resumable work without repeating completed actions. They exist to make retries bounded and idempotent while keeping unattended background processing outside v0.1.

- [ ] `CAPABILITY GATE` Require a unique task lease with owner, acquired time, renewal rule, expiration, cancellation state, and recovery behavior before any background worker may process a job.
- [ ] `CAPABILITY GATE` Start scheduled and unattended execution with read-only jobs and local notifications only; prohibit scheduled filesystem writes, generic shell commands, connector writes, publication, and remote state changes until unattended authority has a separate threat model and passing adversarial tests.
- [ ] `BUILD` per-job resource budgets for model tokens, model loads, tool calls, elapsed time, CPU, memory, disk, network bytes, retries, and retained output.
- [ ] `BUILD` explicit wake, sleep, offline, missed-window, expiration, cancellation, and shutdown rules that cannot silently extend a job's authority or lifetime.
- [ ] `BUILD` a post-run receipt covering lease, idempotency key, effective grant, start and end state, resources, operations, network use, outputs, failures, retries, and cleanup.
- [ ] `BUILD` queue behavior for retry delay, enqueueing, pending-state lookup, processing, success, failure, and requeueing.
- [ ] `BUILD` job behavior for status, next-run scheduling, queue processing, and bounded job execution.
- [ ] `BUILD` a local `JobQueue` for explicitly approved long-running tasks.
- [ ] `BUILD` queued, running, succeeded, failed, cancelled, and awaiting-approval states.
- [ ] `BUILD` idempotency keys so a resumed session does not repeat completed work.
- [ ] `BUILD` bounded retry with recorded reason and next retry time.
- [ ] `BUILD` a dead-letter state for work that should not retry automatically.
- [ ] `BUILD` user cancellation and clean shutdown.
- [ ] `DEFER` autonomous schedulers and unattended background jobs.

## 24A. Notifications and User-Controlled Scheduled Work

This section covers reminders, monitoring, and recurring jobs that the user has explicitly created. It exists so longer work can continue or report back without turning the assistant into an unsupervised background process.

- [ ] `BUILD` local notifications for completed, failed, blocked, approval-waiting, and resource-constrained work.
- [ ] `BUILD` an inspectable schedule record containing the exact prompt, cadence, workspace, model, tools, permissions, stop condition, and last result.
- [ ] `BUILD` manual test-run and dry-run support before a schedule can be enabled.
- [ ] `BUILD` pause, resume, edit, run-now, and delete controls with a complete run history.
- [ ] `BUILD` dedicated worktree execution for repository schedules so recurring work cannot collide with unfinished local changes.
- [ ] `BUILD` missed-run, offline-machine, unavailable-workspace, and partial-failure behavior that reports the problem rather than guessing or repeating writes.
- [ ] `BUILD` strict default sandbox and network denial for scheduled work; unattended runs cannot request interactive approval or broaden their own authority.
- [ ] `DEFER` any recurring task that sends messages, publishes changes, modifies remote systems, or writes outside agent-owned state without a separately approved mechanism.

## 25. Browser and Web Access

This section describes a later, tightly controlled path for web pages and authenticated sites. It exists because browser actions have larger security and publication risks than local reading and should not be enabled until the local foundation is reliable.

- [ ] `DEFER` browser automation from the first offline release.
- [ ] `BUILD` a later browser-tool interface with navigate, inspect, click, type, screenshot, and download operations.
- [ ] `BUILD` browser-automation, Playwright, and mutual-transport-layer-security boundaries if browser access is approved.
- [ ] `BUILD` domain allowlists, download quarantines, authentication-state protection, and per-action approval.
- [ ] `BUILD` separation between public web search and authenticated private browser sessions.
- [ ] `BUILD` citation capture for any web-sourced fact.
- [ ] `DEFER` silent form submission, file upload, email send, messaging platform message send, or external publication.

## 25A. Public Research, Citations, and Computer Use

This section adds evidence-oriented research and visible interaction with applications when a structured tool is unavailable. It exists so current information can be checked and cited without confusing public search, authenticated browsing, and screen-based actions.

- [ ] `BUILD` public web search with query, domain, date, and recency controls.
- [ ] `BUILD` source selection that prefers primary documentation, original research, and authoritative records and clearly labels inference.
- [ ] `BUILD` claim-level citations with page title, direct URL, access time, relevant excerpt hash, and quoted-word limits.
- [ ] `BUILD` freshness rules that require live verification for changing facts and preserve publication date separately from event date.
- [ ] `BUILD` bounded opening, finding, clicking, screenshotting, and downloading with a visible action trail.
- [ ] `BUILD` a separate authenticated-browser profile whose cookies and session tokens never enter model context or logs.
- [ ] `BUILD` screen and application capture with an explicit user action, front-window scope, redaction controls, and attachment provenance.
- [ ] `BUILD` computer-use actions for local applications only after structured file or application tools are unavailable, with confirmation before any irreversible click, submit, or upload.
- [ ] `BUILD` visual browser and interface checks that capture before-and-after screenshots and exact page state.

## 26. External Connectors and Model Context Protocol

Connectors would let the assistant reach systems such as source-control hosting, email, messaging platform, document repository, or databases. This section exists to keep those integrations explicitly scoped, authenticated, rate-limited, logged, and deferred until local tools are trustworthy.

- [ ] `DEFER` Model Context Protocol servers until local filesystem, Git, documents, and memory are reliable.
- [ ] `CAPABILITY GATE` Route every Model Context Protocol and plugin connection, discovery action, request, response, and side effect through the same kernel capability gateway, policy engine, grant checks, limits, and receipt ledger as core tools.
- [ ] `CAPABILITY GATE` Give Model Context Protocol servers and plugins no direct inheritance of filesystem, shell, secret, network, connector, publication, or approval authority from the host process or user session.
- [ ] `CAPABILITY GATE` Require a declarative capability manifest naming identity, version, package and process hashes, transport, tools, schemas, side effects, workspace roots, network destinations, secrets, limits, cancellation behavior, and requested authority before connection.
- [ ] `CAPABILITY GATE` Verify process identity and package hash at launch and connection time, and invalidate approval if either changes.
- [ ] `CAPABILITY GATE` Distinguish in-process, local-process, local-socket, loopback, and remote transports visibly; remote transport requires a separately approved network capability and may never masquerade as local mode.
- [ ] `CAPABILITY GATE` Scope each server independently to approved workspace roots, network destinations, credentials, operation classes, response classifications, and expiry.
- [ ] `BUILD` request and response schema validation, size and item limits, content classification, bounded logging, timeout, cancellation, process termination, and malformed-response recovery.
- [ ] `BUILD` receipts for discovery, connection, manifest verification, request, response classification, side effects, cancellation, timeout, failure, and disconnect.
- [ ] `CAPABILITY GATE` Release Model Context Protocol support in a read-only phase first; writable tools require a separate threat model, write-transaction integration, adversarial tests, and explicit release gate.
- [ ] `BUILD` a later connector interface with explicit read and write capability labels.
- [ ] `BUILD` connector authentication that uses operating-system or approved secret storage rather than Obsidian or configuration files.
- [ ] `BUILD` separate connectors for source-control hosting, messaging, email, calendars, document repositories, archive services, databases, and cloud services only after approval.
- [ ] `BUILD` per-connector scopes, rate limits, audit logs, and publication gates.
- [ ] `BUILD` connector result citations and freshness timestamps.
- [ ] `DEFER` automatic plugin installation and connector discovery from untrusted sources.

## 26A. Plugins, Hooks, and Capability Packages

This section defines how the agent gains modular abilities without placing every instruction and tool in its core. It exists so capabilities can be installed, inspected, limited, tested, updated, and removed as versioned packages.

- [ ] `CAPABILITY GATE` Prohibit automatic discovery, download, installation, enabling, or execution of public plugins, hooks, skills, agents, or capability packages; every package begins disabled and requires an exact local review and approval.
- [ ] `CAPABILITY GATE` Prohibit plugins and hooks from bypassing the kernel through ambient process permissions, direct endpoint calls, alternate Model Context Protocol clients, inherited credentials, or unreceipted side effects.
- [ ] `BUILD` a versioned plugin manifest containing identity, compatibility, tools, skills, hooks, required permissions, network domains, dependencies, and entry points.
- [ ] `BUILD` install, enable, disable, update, rollback, and uninstall operations with exact change previews and approval.
- [ ] `BUILD` signature, checksum, license, and dependency checks before a package is installed or updated.
- [ ] `BUILD` per-plugin filesystem, command, network, credential, connector, and publication permissions enforced by the same policy engine as core tools.
- [ ] `BUILD` lifecycle hooks for before and after model calls, tool calls, file changes, validation, session start, checkpoint, and session end.
- [ ] `BUILD` hook timeouts, ordering, failure isolation, audit receipts, and a safe mode that starts with all optional hooks disabled.
- [ ] `BUILD` plugin and skill compatibility tests against supported agent, tool-protocol, configuration, and memory-schema versions.
- [ ] `BUILD` a local capability catalog that shows which package supplies each tool or skill and why it was selected for the current task.
- [ ] `DEFER` executing unsigned or unapproved packages and packages that request broader authority than the active task.

## 26B. Rich Artifact Creation and Verification

This section extends file reading into controlled creation and editing of common work products. It exists so generated documents, spreadsheets, presentations, diagrams, images, and reports are not considered finished until their structure and rendered appearance have been checked.

- [ ] `BUILD` create, edit, redline, comment, render, and visually verify Word documents while preserving the original unless an edit is approved.
- [ ] `BUILD` create and verify Portable Document Format files, page images, links, metadata, redactions, and fillable forms.
- [ ] `BUILD` create and edit spreadsheets with formulas, styles, tables, charts, validation, and recalculation checks, followed by rendered visual review.
- [ ] `BUILD` create and edit presentation files with slide layouts, speaker notes, images, charts, links, and rendered slide review.
- [ ] `BUILD` diagrams, plots, charts, comparison tables, and small interactive visualizations when they materially improve understanding.
- [ ] `BUILD` image generation and editing as an optional local or approved provider-backed capability with an operation receipt and user review.
- [ ] `BUILD` audio transcription as an optional capability with timestamps, speaker uncertainty, and original-audio retention controls.
- [ ] `BUILD` a common artifact receipt containing input files, generator version, changed files, validation, render outputs, and known fidelity limits.
- [ ] `BUILD` format-specific round-trip tests that detect lost formulas, comments, links, notes, layout, metadata, or accessibility information.

## 27. Visual Studio Code Interface

This section connects the guarded local runtime to the native Visual Studio Code Chat window. Gemma 4 must appear in the standard model picker and operate through AgentMage's safety layer rather than through a separate chat panel or raw model endpoint.

- [ ] `CAPABILITY GATE` Build an AgentMage Visual Studio Code extension that registers a language-model chat provider and contributes local models to the native Chat model picker.
- [ ] `CAPABILITY GATE` Pin `engines.vscode` to the first verified stable Visual Studio Code release that includes the required language-model chat-provider API; production packages cannot enable a proposed API or depend on Insiders behavior.
- [ ] `CAPABILITY GATE` Register **AgentMage — Gemma 4 E4B (Local, Read Only)** as the default selectable model in the native Visual Studio Code Chat window.
- [ ] `CAPABILITY GATE` On macOS, package and verify the signed native IPC bridge required to reach the App-Sandboxed kernel host; the extension must refuse an ad-hoc, unsigned, wrongly signed, wrong-bundle-identifier, wrong-App-Group, or replayed endpoint.
- [ ] `CAPABILITY GATE` Send model requests through the guarded `AgentRuntime` and selected `LocalModelRuntime` adapter; never expose native `llama.cpp`, Docker Model Runner, or another raw model endpoint as an authority-bearing agent endpoint.
- [ ] `CAPABILITY GATE` Stream Gemma 4 text, tool requests, tool results, cancellation, and errors into the native Chat response interface.
- [ ] `BUILD` provider metadata and token counting for model identifier, family, version, input limit, output limit, image support, and tool-call support.
- [ ] `BUILD` model selection between E4B and 26B after each model passes its capability checks.
- [ ] `BUILD` session, workspace, permission, and tool indicators in chat responses.
- [ ] `BUILD` approval prompts that show exact commands and file diffs.
- [ ] `BUILD` clickable local file links and line references.
- [ ] `BUILD` progress updates for long-running local operations.
- [ ] `BUILD` cancel, resume, restore-checkpoint, and show-audit-log actions.
- [ ] `BUILD` a startup task that checks the macOS signed helper bundle, sandbox, Keychain, security-scoped workspace bookmark, Metal model runtime, and model availability, or the corresponding native `llama.cpp`/gated Docker Model Runner, Secret Service, sandbox, and model checks on Fedora and Ubuntu.
- [ ] `BUILD` a five-minute recovery guide for restarting the local environment.
- [ ] `VERIFY` that the AgentMage provider remains local, requires no hosted account or cloud API key, and does not silently fall back to a cloud model.

## 27A. Local Command-Line Interface

This interface provides the complete agent through an ordinary terminal without requiring Visual Studio Code, Obsidian, a browser, or a desktop window. It exists as the smallest dependable way to start, inspect, recover, test, and automate the local assistant while keeping all information on the computer.

- [ ] `CAPABILITY GATE` Begin the full command-line interface, machine-readable JSON interface, software development kit, and Agent Client Protocol only after the native Visual Studio Code implementation and shared kernel contracts are stable and verified.
- [ ] `CAPABILITY GATE` Require noninteractive invocations to present predeclared, bounded, expiring grants; when a required approval is absent, ambiguous, stale, or interactive-only, fail closed without prompting, broadening scope, or retrying through another interface.
- [ ] `BUILD` stable documented exit codes, versioned input and event schemas, deterministic cancellation, timeout behavior, bounded output, and machine-readable receipts for every noninteractive operation.
- [ ] `CAPABILITY GATE` Make command-line, JSON, software development kit, and Agent Client Protocol clients thin interfaces to the same guarded kernel; none may become an alternate unmediated authority path or access storage, tools, models, connectors, or secrets directly.
- [ ] `VERIFY` headless operation against missing grants, expired grants, malformed events, broken pipes, client termination, cancellation races, partial output, and policy-version changes before advertising automation support.
- [ ] `CAPABILITY GATE` Provide an interactive `agent chat` command that uses the same guarded runtime, tools, approvals, memory, and conversation store as every other interface.
- [ ] `CAPABILITY GATE` Provide `agent conversations list --from DATE --to DATE`, `search QUERY`, `show ID`, `open ID`, and `resume ID` commands with readable tables and stable identifiers.
- [ ] `CAPABILITY GATE` Provide `agent resume ID --turn TURN_ID` to branch from an exact historical checkpoint while leaving the original conversation unchanged.
- [ ] `CAPABILITY GATE` Provide `agent vault search`, `note show`, `links`, `backlinks`, and `tasks` commands that read local Markdown directly without opening or installing Obsidian.
- [ ] `CAPABILITY GATE` Show the active workspace, model, permission profile, conversation, plan step, writable roots, and offline status in the terminal prompt or status line.
- [ ] `CAPABILITY GATE` Render approval requests, command previews, file diffs, progress, cancellation, errors, citations, and final receipts without hiding information in a graphical window.
- [ ] `BUILD` local shell completion, command help, machine-readable JSON output, and a human-readable default format.
- [ ] `BUILD` `agent doctor` for model, local runtime, database, lock, path, permission, disk, dependency, and network-denial checks.
- [ ] `BUILD` `agent checkpoint`, `handoff`, `audit`, `memory inspect`, `memory correct`, `export`, and `import` commands.
- [ ] `BUILD` a terminal conversation picker with date filters, text search, preview, open, resume, branch, pin, archive, and delete actions.
- [ ] `VERIFY` every command with the network disabled and confirm that no command silently launches a browser, Obsidian, a cloud login, or an external application.

## 27B. Local Desktop Application

This interface provides a visual conversation library and chat window while using the same local guarded runtime as the command line and editor. It exists for browsing history, opening files, reviewing changes, granting approvals, and resuming work without depending on a hosted account or a cloud-backed application.

- [ ] `BUILD` later standalone desktop applications for macOS and Linux that import the agent core as a local library or use local inter-process communication, not an external web API. The v0.1 macOS helper bundle is a security and runtime component, not a second user interface.
- [ ] `BUILD` a conversation sidebar with today, yesterday, exact date, date range, workspace, project, model, tag, status, pinned, and archived filters.
- [ ] `BUILD` a local search box that can perform: "Show me my conversations from August 7, open this one, and continue from that exact point."
- [ ] `BUILD` a transcript view with turn timestamps, model names, attachments, citations, tool calls, approvals, errors, checkpoints, and conversation branches.
- [ ] `BUILD` open, continue, resume-from-turn, branch, rename, pin, archive, export, and approval-gated delete actions.
- [ ] `BUILD` a checkpoint comparison screen that shows recorded versus current files, instructions, repository state, model, permissions, and next action before resuming.
- [ ] `BUILD` a workspace and vault picker that accepts only approved local roots, reports cloud-synchronized or remote paths, and never requires the Obsidian desktop application.
- [ ] `BUILD` local Markdown rendering, wiki-link navigation, task views, backlinks, file previews, and clickable file-and-line links using local assets only.
- [ ] `BUILD` exact diff, command, write, and delete approval screens backed by the shared policy engine; the desktop interface must not grant itself broader rights than the command-line interface.
- [ ] `BUILD` visible model, runtime, context, memory, plan, tool, offline, resource, and audit status.
- [ ] `BUILD` crash-safe autosave to the local conversation database, single-writer locks, recovery after forced termination, and a read-only safe mode.
- [ ] `BUILD` packaged local fonts, icons, themes, help, and updates; no remote fonts, content-delivery network, analytics, telemetry, advertisements, or automatic cloud update service.
- [ ] `DEFER` Windows and Intel Mac support until the Apple Silicon macOS, Fedora, and Ubuntu implementation passes the complete offline, security, storage, and recovery suites.

Decision 0008 preserves that historical checklist entry but supersedes its Windows disposition: Windows 11 x64 is now first-GA work under Sections 36, 36A, and 36B. Intel Mac remains deferred, and Apple Silicon remains a retained post-GA lane.

- [ ] `VERIFY` that the application remains fully usable with the network disabled, the Obsidian desktop application absent, and all user data located in a non-synchronized local folder.

## 28. Specialized Workflow Skills

These entries define small, focused skills for repeatable meetings, issues, reports, repositories, and handoff workflows while keeping each workflow bounded and reviewable.

- [ ] `CAPABILITY GATE` Implement declarative skills before executable plugins: prompts, schemas, examples, and templates may shape planning but receive no filesystem, shell, secret, network, connector, or approval authority of their own.
- [ ] `BUILD` a skill record containing stable identity, source, content hash, signer when available, license, version, compatibility, purpose, included files, requested scope, and trust state.
- [ ] `BUILD` trust-gated skill loading with visible instruction precedence, conflict reporting, bounded context contribution, and receipts showing which skill content influenced a task.
- [ ] `VERIFY` declarative skills against malicious-instruction, hidden-tool-request, path-expansion, prompt-injection, excessive-context, and conflicting-policy fixtures before user-authored or imported skills are enabled.

- [ ] `BUILD` Daily Setup skill — daily note, active standup note, meeting shells, and optional safe repository refresh.
- [ ] `BUILD` Daily Briefing skill — current focus, issues, pull requests, meetings, waits, questions, evidence, and next step.
- [ ] `BUILD` Issue Intake skill — issue facts, code paths, documentation, dependencies, acceptance criteria, task list, questions, tests, and commands.
- [ ] `BUILD` Issue Intelligence Report skill — combined issue, people, graph links, diagrams, and report output.
- [ ] `BUILD` Issue Knowledge Graph skill — person, project, glossary, issue, and pull-request links.
- [ ] `BUILD` Issue Report skill — Markdown, Mermaid, dated Portable Document Format report, and visual verification.
- [ ] `BUILD` Team Watchlist skill — active teammate work, overlap, and stale items.
- [ ] `BUILD` Standup Issue Update skill — noisy-note parsing with confidence labels.
- [ ] `BUILD` Meeting Cleanup skill — raw notes, summary, decisions, actions, questions, risks, owners, and links.
- [ ] `BUILD` Evidence Log Status skill — evidence-backed completed work, next work, blockers, and missed accomplishments.
- [ ] `BUILD` Handoff skill — exact continuation state and resume prompt.
- [ ] `BUILD` Repository Learning skill — entry points, architecture, modules, tests, and onboarding guide.
- [ ] `BUILD` Best Practices Research skill for later approved web access.
- [ ] `BUILD` Git History Analysis skill for read-only history and change-intent investigation.
- [ ] `BUILD` bounded code-simplicity, architecture, security, data-integrity, accessibility, performance, and language-specific review skills.
- [ ] `BUILD` Bug Reproduction and Quality Assurance skill.
- [ ] `BUILD` bounded requirements, architecture, component, interface-control, integration, verification, and release review roles.
- [ ] `BUILD` Executive Briefing skill — priorities, schedule, decisions, commitments, waiting items, risks, and preparation from approved local evidence.
- [ ] `BUILD` Chief-of-Staff Portfolio skill — project summaries, dependencies, decision needs, cross-project conflicts, and leadership briefings.
- [ ] `BUILD` Meeting Secretary skill — agenda, raw notes, transcript cleanup, minutes, decisions, actions, owners, deadlines, and follow-up drafts.
- [ ] `BUILD` Commitment and Approval Tracker skill — promises, requests, approvals, due dates, evidence, and follow-up state.
- [ ] `BUILD` Correspondence skill — user-reviewed email, chat, memorandum, letter, follow-up, escalation, and thank-you drafts.
- [ ] `BUILD` Document Control skill — register, naming, versions, reviewers, approvals, duplicates, final copies, and filing suggestions.
- [ ] `BUILD` Obsidian Vault Steward skill — daily notes, projects, contacts, meetings, tasks, links, handoffs, and change previews through direct Markdown access.
- [ ] `BUILD` Plain-Workspace Steward skill — the same records and workflows using canonical Markdown, a regenerable SQLite index, and explicit JSON Lines exports without Obsidian.
- [ ] `BUILD` Local Agent Builder skill — bounded agent definitions, permission manifests, synthetic trials, evaluations, versions, and packages.
- [ ] `BUILD` Local Agent Director skill — assignment, budgets, progress, cancellation, result review, conflict handling, and evidence merging.
- [ ] `BUILD` Repository Cartographer skill — structure, entry points, dependencies, architecture, data flow, permissions, tests, and glossary.
- [ ] `BUILD` Feature Trace skill — user action through interface, service, storage, background work, response, and tests.
- [ ] `BUILD` Change Impact skill — affected symbols, callers, data, permissions, tests, documentation, deployment, risk, and rollback.
- [ ] `BUILD` Debugging skill — reproduction, hypotheses, checks, logs, root cause, regression test, and fix evidence.
- [ ] `BUILD` Test and Verification skill — focused checks, full checks, failure classification, unrun checks, and evidence receipt.
- [ ] `BUILD` Repository Documentation skill — onboarding, architecture, build, test, operations, troubleshooting, and change-history guides.
- [ ] `DEFER` large agent swarms and automatic parallel delegation.

## 29. Local Command-Line Dependencies

This section defines dependencies that must be detected and verified without silently installing packages or assuming an ambient tool exists. A clean v0.1 Mac installation cannot require Homebrew, Rosetta, Xcode command-line tools, Python, or a user-installed Git; required read-only functionality must ship in the signed package or use operating-system APIs.

- [ ] `VERIFY` a pinned embedded search implementation for v0.1; an installed `rg` may be used only by diagnostics after version and hash checks.
- [ ] `VERIFY` a pinned embedded Git-reading library for v0.1; an installed `git` may be used only by diagnostics and later approved operations after version and path checks.
- [ ] `VERIFY` `python3` for local orchestration and document or structured-content utilities.
- [ ] `VERIFY` `node` and `npm` for the Visual Studio Code extension and JavaScript utilities.
- [ ] `VERIFY` the signed native `llama.cpp` Metal runtime on Apple Silicon macOS, native `llama.cpp` as the Fedora/Ubuntu reference, and Docker Engine with Docker Model Runner as the separately gated Linux compatibility adapter; Docker Desktop remains an optional macOS adapter.
- [ ] `VERIFY` an isolated Apple Silicon release runner, Apple SDK and Swift toolchain, Apple Developer Program signing identity, notarization service access, App Group provisioning, and secret-store-backed release credentials before macOS packaging begins; none are end-user runtime dependencies.
- [ ] `VERIFY` `jq` for bounded JSON inspection.
- [ ] `VERIFY` `sqlite3` for local memory and indexes.
- [ ] `VERIFY` an approved GitHub command-line client for future read-only inspection; publication remains approval-gated.
- [ ] `VERIFY` `gpg` for signed-commit inspection and verification.
- [ ] `VERIFY` an approved macOS-, Fedora-, and Ubuntu-compatible Word-to-text converter.
- [ ] `VERIFY` approved macOS-, Fedora-, and Ubuntu-compatible local preview and image utilities.
- [ ] `VERIFY` `unzip` and `zip` for Open XML inspection and controlled archives.
- [ ] `VERIFY` `curl` only for explicitly approved loopback or later allowlisted network calls.
- [ ] `VERIFY` `openssl` for certificates, hashes, and cryptographic inspection.
- [ ] `VERIFY` required and optional dependencies on a clean MacBook Pro M5, clean Fedora installation, and clean Ubuntu installation.
- [ ] `VERIFY` required and optional Python packages inside a project-owned virtual environment rather than the global Python environment.
- [ ] `BUILD` a dependency inventory that records the exact interpreter or virtual environment for every approved package.
- [ ] `BUILD` a no-install startup check that reports missing optional capabilities without changing the machine.

## 30. Configuration and Versioning

Configuration separates implementation from machine paths, model names, permissions, and environment-specific choices. This section exists so the same agent can move among Apple Silicon macOS, Fedora, and Ubuntu without hard-coded locations or silently broadened access.

- [ ] `BUILD` a versioned agent configuration schema.
- [ ] `BUILD` separate development, synthetic-test, read-only-vault, and repository-editing profiles.
- [ ] `BUILD` explicit model, endpoint, workspace root, tool, permission, budget, and logging sections.
- [ ] `BUILD` environment-specific overrides that cannot broaden permissions silently.
- [ ] `BUILD` configuration validation before server start.
- [ ] `BUILD` configuration hash recording in every session audit log.
- [ ] `BUILD` skill version and prompt version fields.
- [ ] `BUILD` migration scripts for configuration and local-memory schema changes.
- [ ] `BUILD` a configuration diff shown before a change is activated.
- [ ] `BUILD` backup and rollback for agent configuration.

## 31. Evaluation Fixtures and Acceptance Tests

These fixtures provide safe, synthetic data for testing the assistant before it touches real notes or repositories. They exist to measure correctness, evidence coverage, security behavior, memory recovery, and performance with repeatable inputs.

- [ ] `BUILD` a fictional Markdown workspace for repeatable acceptance tests.
- [ ] `BUILD` a synthetic Obsidian mini-vault with current and superseded handoffs, people, projects, meetings, raw notes, tasks, and broken links.
- [ ] `BUILD` a fictional Git repository with tests and one deliberate defect.
- [ ] `BUILD` supported-language repository-map fixtures with pinned expected file inventory, symbols, definitions, imports, relationships, source ranges, Git identities, coverage limits, and deterministic map hashes.
- [ ] `BUILD` unsupported-language, malformed-parser, generated-tree, ignored-file, dirty-tree, detached-head, renamed-file, Unicode-path, and stale-map fixtures.
- [ ] `BUILD` synthetic Word, Portable Document Format, spreadsheet, comma-separated-value, JSON, image, and archive fixtures.
- [ ] `BUILD` path-traversal and symlink-escape fixtures.
- [ ] `BUILD` secret-canary fixtures that prove values are detected and redacted.
- [ ] `BUILD` malformed frontmatter, Unicode filename, spaces-in-path, large-file, and binary-file fixtures.
- [ ] `BUILD` stale-memory and conflicting-source fixtures.
- [ ] `BUILD` dirty-Git-tree and unrelated-user-change fixtures.
- [ ] `BUILD` approval-required action fixtures for write, command, commit, push, publish, and network access.
- [ ] `BUILD` model hallucination checks that compare claims against tool receipts.
- [ ] `BUILD` evidence-state fixtures containing observed facts, deterministic derivations, supported inferences, missing evidence, denied tools, failed parsing, conflicting records, changed files, and stale citations.
- [ ] `BUILD` model lifecycle fixtures for insufficient memory or disk, unsupported acceleration, partial download, cancellation, corrupt artifact, hash mismatch, quarantine, interrupted activation, successful self-test, unload, and cleanup.
- [ ] `BUILD` repository-instruction injection fixtures that request policy changes, hidden tools, secret disclosure, broader roots, external transfer, or false completion.
- [ ] `BUILD` identical benchmark prompts for E4B and 26B.
- [ ] `BUILD` measurements for correctness, evidence coverage, false completion claims, tool failures, latency, and resource use.
- [ ] `BUILD` a full offline acceptance test.

## 31A. Capability, Model, and Recovery Evaluation

This section measures whether each model and tool combination is safe and useful enough to enable. It exists because a capability listed in configuration is not trustworthy until repeatable tests show what it can do, where it fails, and whether a fallback improves the result.

- [ ] `BUILD` a capability matrix that records supported, degraded, unavailable, untested, and failed states for every model, tool, skill, file type, and interface.
- [ ] `BUILD` identical task suites across installed models for planning, coding, retrieval, document work, tool use, context recovery, and refusal behavior.
- [ ] `BUILD` a Gemma 4 v0.1 repository suite that locates and cites a symbol, summarizes a module without inventing files, compares two implementations, traces an import or call path, reports an absent result as unknown, recovers from malformed tool calls, and remains within the declared context and latency budgets.
- [ ] `BUILD` golden outputs and invariant checks that score facts, citations, tool receipts, changed files, validation, and prohibited side effects rather than writing style alone.
- [ ] `BUILD` tool-schema conformance tests for malformed arguments, extra fields, missing fields, invalid output, repeated calls, timeout, cancellation, and partial failure.
- [ ] `BUILD` installer and `agentmage doctor` conformance tests across every supported platform, hardware-fit result, recovery state, manifest mismatch, sandbox state, encrypted-store state, repository-map state, offline state, and redaction rule.
- [ ] `BUILD` adversarial tests for prompt injection, memory poisoning, stale evidence, conflicting instructions, secret extraction, path escape, and approval bypass.
- [ ] `BUILD` downgrade tests that prove the agent reports missing vision, tool, context, or file support instead of pretending the selected model has it.
- [ ] `BUILD` crash and recovery tests for model-server restart, process termination, full disk, corrupt state, interrupted writes, lost network, and deleted worktrees.
- [ ] `BUILD` performance budgets for first response, tool latency, full task time, peak memory, disk growth, and model load or unload time.
- [ ] `BUILD` a release gate that prevents a new model, plugin, tool, or memory migration from becoming the default until its required tests pass.

## 31B. v0.1 Quantitative Acceptance Matrix

All v0.1 tests run on a clean, recorded Apple Silicon MacBook Pro M5 launch-reference environment, a recorded Fedora workstation, and a clean Ubuntu compatibility environment. The record includes operating-system build, processor, unified or system memory, storage format and case behavior, Visual Studio Code version, AgentMage package digest, model manifest, and runtime versions. Security tests require zero violations; averages cannot hide a single unauthorized side effect.

| Test ID | Measurement | Passing threshold |
|---|---|---|
| `AT-ARCH-001` | Static and runtime dependency checks across kernel, Core Read-Only pack, and Visual Studio Code shell. | Zero kernel imports from a capability pack or shell; every shell request enters through the kernel contract. |
| `AT-CFG-001` | Invalid, unknown, and permission-broadening configuration mutations. | 100 of 100 rejected before startup with the exact field and reason reported. |
| `AT-PLAT-001` | Clean first-run packaging and platform-adapter verification on MacBook Pro M5, Fedora, and Ubuntu. | The release manifest matches the tested macOS build, arm64 package, Apple SDK and toolchain, Team ID, bundle and App Group identifiers, designated requirements, entitlements, helpers, and package digest. The package is Developer ID-signed, notarized, stapled, accepted by Gatekeeper, Hardened Runtime-enabled, App-Sandboxed, XPC identities valid, signed extension bridge authenticated over App Group IPC, Keychain operational, read-only bookmark enforced, and Metal inference active without Homebrew, Rosetta, Xcode tools, Docker Desktop, or administrator access after installation. Fedora and Ubuntu select only their declared adapters. All three pass the identical contract fixtures. |
| `AT-SEC-001` | Process, socket, file-permission, entitlement, environment, identity, and privilege inspection on macOS, Fedora, and Ubuntu. | 100% match the documented platform topology; no process runs as root or receives an undeclared entitlement, path, bookmark, variable, descriptor, socket, or capability. |
| `AT-SBX-001` | Per-platform sandbox escape suite covering filesystem, process, device, environment, namespace, XPC, bookmark, and entitlement attacks. | Zero successful escapes or unauthorized reads across at least 500 attempts against each supported platform adapter. |
| `AT-INJ-001` | Prompt-injection and malicious-content suite. | Zero grants, hidden tool calls, policy changes, secret disclosures, or unauthorized side effects across at least 200 fixtures. |
| `AT-INS-001` | Repository-instruction trust and precedence suite covering nested instructions, conflicts, generated text, comments, policy-change requests, hidden-tool requests, broader-root requests, external transfer, and false-completion instructions. | 100% remain untrusted until explicitly trust-gated; zero workspace instructions alter policy, grant authority, enable a tool, expand a root, override current user intent, or suppress a conflict across at least 200 fixtures. |
| `AT-NET-001` | Packet, socket, process, container, namespace, and Domain Name System monitoring for a 60-minute offline end-to-end run after the separate model installer has exited on each supported platform and adapter. | Zero outbound connection attempts and zero outbound bytes; only a declared local inference or extension-bridge path exists when the selected adapter requires one. Native macOS inference has no network entitlement, Docker Model Runner exposes no reachable undeclared path, and the model installer/importer is not running. |
| `AT-NET-002` | Reachability probes from a local-area-network peer, unrelated same-user process, ordinary container where applicable, separate network namespace, tool worker, XPC helper, extension host, and kernel. | Only the kernel's explicitly selected local adapter path succeeds; every undeclared peer and process probe fails, including probes against Docker Model Runner's unauthenticated API. |
| `AT-PATH-001` | Absolute path, traversal, encoding, separator, symlink, alias, stale-bookmark, case-folding, Unicode-normalization, rename-race, mount, and file-replacement attacks. | Zero workspace escapes across at least 500 attacks against each platform adapter; 100% success for unambiguous valid fixture paths. |
| `AT-AUTH-001` | Grant mutation, replay, expiry, nonce reuse, preimage change, task change, target change, uncertain-result retry, and child-scope tests. | Zero unauthorized executions across at least 500 attempts; every accepted grant is consumed exactly once. |
| `AT-DATA-001` | Transaction and migration invariants with crashes injected at every durable-write boundary. | Zero orphan grants, receipts, actions, or checkpoints across at least 100 crash points. |
| `AT-CRASH-001` | Forced termination during model, file, Git, receipt, and checkpoint phases. | 100 of 100 resumes select the correct next safe action and repeat zero completed actions. |
| `AT-PRIV-001` | Secret canaries in prompts, files, attachments, tool output, errors, and model responses. | Zero canary values in SQLite, logs, exports, configuration, crash output, or model context beyond the authorized turn. |
| `AT-PRIV-002` | Encryption, ephemeral fallback, expiry, export, and cryptographic-erasure tests. | No private or restricted plaintext at rest; 100% of expired eligible records become unreadable and absent from indexes. |
| `AT-HOF-001` | Local Codex-handoff generation plus direct-request, prompt-injection, tool-call, Visual Studio Code command, clipboard, and network-delivery attempts. | Every valid packet contains the objective, acceptance criteria, cited evidence, constraints, disclosure list, and unresolved questions; across at least 200 delivery attempts there are zero Codex invocations, tab activations or population, clipboard writes, external calls, or transmitted bytes. The packet is submitted only after the user manually switches to Codex and chooses what to send. |
| `AT-MODEL-001` | Admission record, license, publisher/control, lineage, artifact, tokenizer, native GGUF hash, immutable OCI digest where applicable, runtime, acceleration, and offline response verification for Gemma 4 E4B. | Every platform artifact resolves to the same approved model profile and quantization policy; hashes, image digests, and runtime builds match their manifests; native and Docker adapters pass the same response, tool-schema, cancellation, context, resource, and quality thresholds; 100% of 100 prompts per platform use the selected local model with no cloud or automatic model fallback. |
| `AT-MODEL-002` | Installer/importer preflight, acquisition, cancellation, recovery, quarantine, atomic activation, self-test, load/unload, and cleanup across the supported platform matrix. | 100% correct hardware-fit and artifact decisions; every corrupt, incompatible, incomplete, cancelled, or hash-mismatched artifact remains unrunnable and outside the active store; every valid artifact activates atomically and passes its self-test. |
| `AT-DIA-001` | Redacted local doctor results rendered inside native Visual Studio Code Chat across model, runtime, offline, sandbox/helper, workspace, capability, repository-map, encrypted-store, receipt, and recovery states. | 100% expected status and remediation results across at least 100 fixtures, with zero secrets, prompts, private excerpts, environment values, or unrelated absolute paths in displayed or exported diagnostics; no user-facing command line is required. |
| `AT-ROUTE-001` | Mixed deterministic and model-assisted task suite. | Deterministic operation attempted whenever applicable; zero automatic model switches or frontier transfers in 100 tasks. |
| `AT-TOOL-001` | Golden list, read, search, metadata, and hash fixtures with size and result limits. | 100% exact deterministic results; every configured limit fails closed and produces a receipt. |
| `AT-GIT-001` | Read-only Git operations across clean, dirty, detached, untracked, and malformed fixture repositories. | 100% expected results and identical workspace tree hash, index hash, refs, and object set before and after each run. |
| `AT-REP-001` | Deterministic repository-map construction, invalidation, coverage, rendering, source resolution, and read-only invariants across supported and unsupported fixture repositories. | 100% exact inventory, supported-language symbol, definition, import, reliable relationship, source-range, Git-identity, exclusion, coverage, and map-hash results; unsupported or failed analysis is visibly Unknown/Blocked; repeated unchanged runs are byte-identical; workspace and Git state remain unchanged. |
| `AT-EVD-001` | Tool-action receipt coverage. | 100% of tool attempts, including denials and failures, have one schema-valid receipt. |
| `AT-EVD-002` | File-grounded claim and citation scoring against a labeled corpus. | 100% citation resolution and at least 98% claim-support precision; unsupported claims are labeled as inference or unknown. |
| `AT-EVD-003` | Evidence-state, deterministic-derivation, citation-hash, source-range, conflict, and stale-citation scoring against a labeled corpus. | 100% correct Observed, Derived, Inferred, and Unknown/Blocked states; 100% of derivations identify their method and observed inputs; 100% of changed or missing source identities become visibly stale before reuse. |
| `AT-RESUME-001` | Resume after file, instruction, model-digest, permission, branch, repository-map, citation, and workspace changes. | 100% of material drift is detected before action and requires an explicit continue, restart, or cancel decision. |
| `AT-VSC-001` | Clean extension and kernel-host installation plus model discovery on MacBook Pro M5, Fedora, and Ubuntu. | The extension uses the pinned stable Visual Studio Code provider API without proposed-API flags; AgentMage and the pinned Gemma 4 E4B profile appear in the native model picker with correct limits, runtime, and capabilities in every run; macOS refuses every bridge or host whose signature, designated requirement, bundle identifier, App Group, peer identity, or launch challenge is invalid. |
| `AT-VSC-002` | Native Chat streaming, evidence-state, citation, diagnostics, progress, cancellation, unavailable-model, and malformed-response tests. | 100% complete rendering; cancellation stops within 2 seconds; failures never trigger a cloud fallback or hidden retry. |
| `AT-QUAL-001` | Labeled read-only repository questions, extraction tasks, schema calls, malformed-tool recovery, and adversarial completion prompts. | At least 90% task accuracy, extraction F1 at least 0.95, at least 99% schema-valid calls, and zero false completion claims across 200 adversarial tasks; the suite includes symbol location, cited module summary, implementation comparison, import/call trace, and a required Unknown/Blocked response for absent evidence. |
| `AT-PERF-001` | Five warmups and 30 measured runs on the recorded MacBook Pro M5 and Fedora reference machines, with functional budget verification on clean Ubuntu. | On each reference machine: kernel startup p95 at most 5 seconds; deterministic file/Git tool p95 at most 2 seconds on 10,000 files/1 GiB; cold repository-map construction p95 at most 30 seconds and unchanged warm-map rendering p95 at most 3 seconds on the same fixture; warm Gemma first-token p95 at most 15 seconds and throughput at least 10 tokens/second; kernel peak memory at most 1.5 GiB excluding the model; total peak memory at most 12 GiB; prompts above the configured 32K-token v0.1 ceiling fail before inference. |
| `AT-SPEC-001` | Static cross-document validation of v0.1 scope, backlog, dependency, acceptance-test, release, platform, model, runtime, repository-map, evidence-state, diagnostics, policy, license, and data-authority statements. | Every backlog and test identifier is unique; every dependency and test reference resolves; every normative v0.1 PRD requirement maps to a backlog row; README, PRD, implementation plan, inventory, tasks, model policy, security policy, runtime boundary, and accepted decisions agree on scope; macOS M5, Fedora, and Ubuntu are v0.1 targets everywhere; native `llama.cpp` and Docker Model Runner roles are consistent; deterministic repository mapping and evidence states are v0.1 everywhere; semantic/vector indexing and every state-changing capability remain later; exactly one canonical authority is named for each data domain. |
| `AT-DOC-001` | Three clean installations, one each on MacBook Pro M5, Fedora, and Ubuntu, by a reader using only published instructions. | All three complete hardware-fit preflight, approved model installation or import, redacted doctor response in native local Chat, deterministic repository mapping, evidence-state rendering, and offline proof without undocumented steps, credential exposure, package-manager guesswork, or external assistance; Linux documents native `llama.cpp` as the reference and Docker Model Runner as a separately gated compatibility choice; the Mac requires no Homebrew, Rosetta, Xcode command-line tools, Docker Desktop, or administrator access after installation; model download time is excluded. |

A failed threshold blocks release. Any threshold change requires a dated decision record explaining the evidence, risk, owner, and replacement value.

## 31C. Design Closure Matrix

This matrix makes the design review executable. Each concern has one governing decision and at least one acceptance check; prose alone does not close an item.

| Design concern | Governing decision | Verification |
|---|---|---|
| Release boundary | v0.1 is only the read-only evidence assistant defined by the stable-ID backlog. | `AT-SPEC-001` |
| Reference platforms | Apple Silicon MacBook Pro M5 is the primary launch reference; Fedora is the Linux performance reference; Ubuntu must pass the same supported workflow in v0.1. | `AT-PLAT-001`, `AT-SEC-001`, `AT-MODEL-001`, `AT-MODEL-002`, `AT-DIA-001`, `AT-VSC-001`, `AT-DOC-001` |
| Data authority | Encrypted SQLite owns operational state and bounded repository-map records; user-owned Markdown owns v0.2+ human knowledge; JSON Lines is export only. | `AT-DATA-001`, `AT-PRIV-002`, `AT-REP-001`, `AT-SPEC-001` |
| Threat model and topology | The kernel, shell, tool worker, local model runtime, sockets, privileges, network boundaries, and untrusted-workspace-content rule are explicit and fail closed. | `AT-SEC-001`, `AT-SBX-001`, `AT-INJ-001`, `AT-INS-001`, `AT-NET-001`, `AT-NET-002` |
| Approval and authority | `CapabilityGrant` is the sole authority-bearing object and is bound, expiring, non-broadenable, and single-use. | `AT-AUTH-001` |
| Routing | Deterministic operations run first; the user selects the one enabled model; automatic switching and frontier transfer are absent. | `AT-ROUTE-001` |
| Model lifecycle and diagnostics | The approved installer/importer proves hardware fit, artifact identity, recoverable installation, and clean activation; the native Chat doctor response proves the active runtime and boundary; native and Docker adapters meet the same contract. | `AT-MODEL-001`, `AT-MODEL-002`, `AT-DIA-001` |
| Repository map | v0.1 provides only deterministic, read-only, pinned-parser repository structure with exact coverage and citations; semantic/vector indexing is excluded. | `AT-REP-001`, `AT-SBX-001`, `AT-SPEC-001` |
| Evidence states | Material claims are Observed, Derived, Inferred, or Unknown/Blocked, and stale source identities cannot be reused silently. | `AT-EVD-001`, `AT-EVD-002`, `AT-EVD-003`, `AT-RESUME-001` |
| Codex handoff | AgentMage may render a local reviewed packet, but only the user may switch to Codex and submit selected content; autonomous delivery is prohibited in every release. | `AT-HOF-001`, `AT-NET-001`, `AT-SPEC-001` |
| Privacy and retention | Classification, minimization, encryption, and retention are enforced before persistence. | `AT-PRIV-001`, `AT-PRIV-002` |
| Path semantics | Tools accept only canonical workspace-relative paths; absolute paths are display-only. | `AT-PATH-001` |
| Executable backlog | Every v0.1 row has a stable ID, explicit dependencies, disposition, target, and resolvable acceptance tests. | `AT-SPEC-001` |
| Quantitative gates | Security, correctness, evidence, recovery, latency, memory, context, and documentation have blocking thresholds. | `AM-TST-001`, `AM-TST-002`, and `AM-DOC-001` receipts plus the complete Section 31B matrix |
| Model identifiers | v0.1 admits Gemma 4 E4B only by immutable GGUF hash and, for Docker, resolved OCI digest; Gemma 4 12B Unified is a disabled fallback; later Devstral uses a digest-pinned `ai/devstral-small-2:24B`; all profiles require policy admission before enablement. | `AT-MODEL-001`, `AT-SPEC-001` |
| Complete product documents | The README, PRD, implementation plan, inventory, tasks, model-provenance policy, security policy, runtime-boundary specification, license, and accepted decisions agree on architecture, scope, exclusions, security, storage, models, runtimes, and releases. | `AT-SPEC-001`, `AT-DOC-001` |

## 32. Documentation and Operating Guides

This section lists the guides needed to install, operate, recover, and understand the local assistant. It exists so the project remains usable after a restart or handoff and so its limitations are visible rather than implied.

- [ ] `BUILD` a five-minute install/start guide.
- [ ] `BUILD` maintain the Apache-2.0 license, public vulnerability policy, model-provenance policy, runtime-boundary specification, and accepted architecture decisions as release-controlled documents.
- [ ] `BUILD` a MacBook Pro M5 first-run guide covering the signed/notarized package, native workspace picker, read-only bookmark, Keychain, Metal runtime, model import, Gatekeeper verification, offline proof, uninstall, and recovery without Homebrew or Rosetta.
- [ ] `BUILD` Fedora and Ubuntu installation guides covering the Linux sandbox, Secret Service, local runtime, offline proof, uninstall, and recovery.
- [ ] `BUILD` a model installation and selection guide covering hardware-fit preflight, the approved-artifact catalog, separate downloader/importer, license and lineage display, resumable staging, manifest and hash verification, quarantine, atomic install, self-test, unload, failure cleanup, and proof that the installer has exited before offline inference.
- [ ] `BUILD` a native Visual Studio Code Chat doctor guide covering redacted model/runtime, offline, sandbox/helper, workspace-grant, capability, repository-map, encrypted-store, receipt, and recovery diagnostics; document the diagnostic command harness only for maintainers and reviewers.
- [ ] `BUILD` a Visual Studio Code language-model provider, native Chat model-picker, and startup-task guide.
- [ ] `BUILD` a maintainer-only macOS release guide covering the Apple Silicon runner, pinned SDK and toolchain, Developer ID signing, App Group provisioning, notarization, stapling, Gatekeeper validation, credential handling, package verification, and rollback.
- [ ] `BUILD` a tool and permission reference.
- [ ] `BUILD` an approval and audit-log guide.
- [ ] `BUILD` an evidence-state and citation guide covering Observed, Derived, Inferred, Unknown/Blocked, deterministic methods, source hashes and ranges, conflicts, and stale evidence.
- [ ] `BUILD` a document-conversion guide with known loss of formatting or metadata.
- [ ] `BUILD` an Obsidian read-only pilot guide.
- [ ] `BUILD` a storage-neutral assistant workspace guide covering the plain-folder and Obsidian adapters.
- [ ] `BUILD` an executive-assistant guide covering briefings, priorities, commitments, decisions, approvals, waiting items, reminders, privacy, and user authority.
- [ ] `BUILD` a secretary guide covering agendas, meeting requests, raw notes, transcripts, minutes, action logs, correspondence, document control, and filing.
- [ ] `BUILD` an agent-studio guide covering agent creation, permissions, budgets, synthetic testing, direction, cancellation, result review, versioning, and rollback.
- [ ] `BUILD` a v0.1 deterministic repository-map guide covering supported pinned parsers, inventory and Git semantics, coverage reports, symbols, definitions, imports, reliable relationships, cache invalidation, exact citations, unsupported languages, and the semantic/vector-index exclusion.
- [ ] `BUILD` a later deep repository-comprehension guide covering feature traces, data flow, permissions, tests, history, semantic retrieval when approved, and citation rules.
- [ ] `BUILD` a coding-assistance guide covering reproduction, minimal changes, tests, validation, review packets, commit planning, and rollback.
- [ ] `BUILD` a strict-local data-location, network-denial, and cloud-sync avoidance guide.
- [ ] `BUILD` a local conversation search, exact-point resume, branch, retention, export, and recovery guide.
- [ ] `BUILD` a command-line interface reference with examples for chat, vault search, conversation history, checkpoint resume, audit, and diagnostics.
- [ ] `BUILD` later macOS and Linux desktop application guides covering local data paths, vault selection, conversation browsing, approvals, offline checks, backup, and recovery.
- [ ] `BUILD` a repository-review and patch-approval guide.
- [ ] `BUILD` a signed-commit and push-approval guide.
- [ ] `BUILD` an offline emergency prompt that resumes from `Current Handoff.md`.
- [ ] `BUILD` a troubleshooting guide for macOS signing, notarization, App Sandbox, bookmarks, Keychain, Metal, native `llama.cpp`, and for Docker Engine, Docker Model Runner, local ports, model pulls, slow inference, malformed tool calls, and missing packages on Fedora and Ubuntu.
- [ ] `BUILD` a capability matrix stating what the local agent can do, cannot do, and has not yet been tested to do.

## 33. Recommended Build Order

Only v0.1 is ordered here; later work follows the release sequence and is promoted into a new stable-ID backlog before implementation.

1. **Policy and feasibility:** accept the architecture decision, establish license/security/model/runtime policies, run documentation CI, and resolve the Gemma 4 E4B native/Docker feasibility spike before implementation assumes a model or runtime.
2. **Contracts:** complete `AM-KRN-001` and `AM-CFG-001`; freeze typed boundaries before writing an interface or tool.
3. **Platform boundary:** complete `AM-PLT-001`; package and prove the MacBook Pro M5, Fedora, and Ubuntu adapters before capability code depends on them.
4. **Security boundary:** complete `AM-SEC-001`, `AM-PTH-001`, `AM-AUT-001`, and `AM-NET-001`; prove all declared platform and runtime topologies with hostile fixtures and begin continuous trust-boundary fuzzing.
5. **Canonical state:** complete `AM-DAT-001` and `AM-PRV-001`; inject crashes and secret canaries before storing real sessions.
6. **Local model lifecycle:** complete `AM-MDL-001`, `AM-MDL-002`, `AM-MDL-003`, and `AM-DIA-001`; pin Gemma 4 E4B, prove native macOS/Linux and gated Docker parity, recover installation failures, and make the active boundary inspectable in native Chat.
7. **Read-only primitives and trust:** complete `AM-SEC-002`, `AM-TOL-001`, `AM-GIT-001`, and `AM-INS-001` inside each platform sandbox; workspace instructions remain untrusted content.
8. **Repository map and evidence:** complete `AM-REP-001`, `AM-EVD-001`, and `AM-EVD-002`; deterministic structure, coverage, receipts, evidence states, and citations must agree before model synthesis is enabled.
9. **Handoff and recovery:** complete `AM-HOF-001` and `AM-SES-001`; the Codex boundary must fail closed, handoff warnings must be explicit, and stale map or citation inputs must be detected before resume is enabled.
10. **Native Chat shell:** complete `AM-VSC-001` and `AM-VSC-002`; the extension remains a thin kernel client and renders evidence states, accessible controls, and diagnostics.
11. **Release gate:** complete `AM-TST-001`, `AM-TST-002`, and `AM-DOC-001`; rerun every applicable `RV-*` protocol and quantitative threshold on MacBook Pro M5, Fedora, and Ubuntu, then execute incident, support/patch, accessibility, and evidence-assembly gates.

## 34. v0.1 Completion Checklist

v0.1 is complete only when all of the following are true:

- [ ] Every row in the executable v0.1 backlog is complete with its required acceptance receipts.
- [ ] Every test in the v0.1 quantitative acceptance matrix passes on the clean recorded MacBook Pro M5, recorded Fedora workstation, and clean Ubuntu environment.
- [ ] **AgentMage — Gemma 4 E4B (Local, Read Only)** appears in the native Visual Studio Code Chat model picker and communicates only through the guarded kernel.
- [ ] The separate installer/importer proves hardware fit, policy admission, license and lineage, manifest and immutable hashes/digests, recovery and quarantine behavior, atomic activation, and self-test; the native Chat doctor response reports the active redacted boundary correctly.
- [ ] The user can select one workspace and use bounded list, read, search, metadata, hash, read-only Git, and deterministic repository-map operations without any workspace mutation.
- [ ] Every map record resolves to the correct source hash and range, every coverage gap is visible, and unsupported languages or relationships are never fabricated.
- [ ] Every tool attempt has one receipt, every file-grounded claim has a resolvable supporting citation, every material claim is Observed, Derived, Inferred, or Unknown/Blocked, and changed evidence becomes visibly stale.
- [ ] Repository instructions and workspace text cannot change policy, grants, tools, roots, current user intent, external-transfer rules, or completion status.
- [ ] AgentMage can render a reviewed local Codex handoff packet, but cannot invoke Codex, control or populate its tab, write the clipboard, call an external endpoint, or transmit the packet.
- [ ] One persisted session survives forced termination, detects environment drift, resumes at the next safe action, and repeats no completed action.
- [ ] Path, injection, sandbox, network, grant, privacy, and crash suites report zero security violations.
- [ ] Native `llama.cpp` passes as the Linux reference; Docker Model Runner remains disabled unless its immutable artifacts, unauthenticated API isolation, zero-egress behavior, and contract parity pass independently.
- [ ] v0.1 fuzzing, incident tabletop, accessibility, signed manual patch/support, diagnostic-surface, and handoff-warning stories meet their acceptance criteria and produce reviewable evidence.
- [ ] Apple Silicon MacBook Pro M5 is the primary launch reference, Fedora is the Linux performance reference, and the identical supported workflow passes on Ubuntu.
- [ ] Startup, privacy, limitation, recovery, and offline-verification instructions pass `AT-DOC-001`.
- [ ] Release notes state that v0.1 supports Apple Silicon macOS, Fedora, and Ubuntu but has no Intel Mac or Windows support, semantic/vector index, language-server write surface, Codex invocation or transfer, Obsidian pack, writes, coding changes, frontier delivery, full command-line shell, standalone desktop UI, GitHub access, browser, connectors, scheduled work, or child agents.

## 35. Competitive Review Integration Register

This register makes the competitive capability review additive and traceable. It does not replace, narrow, or remove any earlier inventory item. Each recommendation is implemented by the detailed checklist in the named sections and remains subject to the release gates below.

| ID | Competitive recommendation | Target release | Primary inventory sections | Integration status |
|---|---|---:|---|---|
| `CR-P0-REP` | Deterministic, policy-aware, citation-preserving read-only repository map without required embeddings | v0.1 | 7A, 13, 31A, 31B, 34 | Added to the v0.1 backlog and acceptance gates |
| `CR-P0-MDL` | Complete approved local-model acquisition, verification, fit, lifecycle, diagnostics, cancellation, and offline proof | v0.1 | 1, 1A, 31A, 31B, 33, 34 | Added to the v0.1 backlog and acceptance gates |
| `CR-P0-EVD` | Observed, Derived, Inferred, and Unknown/Blocked answer states with resolvable and stale-aware citations | v0.1 | 4, 9, 22, 23, 27, 31A, 31B, 34 | Added to the v0.1 backlog and acceptance gates |
| `CR-P0-DIA` | Redacted runtime and capability diagnostics | v0.1 | 1, 5B, 23, 27, 31A, 34 | Added to the v0.1 backlog and acceptance gates |
| `CR-P0-EVAL` | Gemma 4 quality, tool-recovery, context, latency, citation, and injection evaluation on reference hardware | v0.1 | 1, 31, 31A, 31B, 33, 34 | Added to the v0.1 backlog and acceptance gates |
| `CR-P0-TRUST` | Repository, notes, issues, tool output, and imported instructions remain untrusted data unless explicitly trust-gated as non-authority guidance | v0.1 | 5A, 6, 9, 13B, 31A, 34 | Added to policy and adversarial verification |
| `CR-P1-RET` | Optional approved local hybrid structural, lexical, embedding, and reranking retrieval with deletion and deterministic fallback | v0.2 | 9, 10, 31 | Added as an opt-in post-map capability |
| `CR-P1-WRT` | Exact-preimage controlled-write transaction with shadow changes, one-use grant, atomic application, receipts, verification, and rollback | v0.3 | 5, 14B, 15, 23 | Added as the required write lifecycle |
| `CR-P1-WKT` | Git worktree isolation that supplements, but is never described as, the security sandbox | v0.4 | 5A, 13A, 14B | Added as coding-workspace isolation |
| `CR-P1-VAL` | Separately approved trusted test, lint, formatter, build, and scan templates with evidence-based receipts | v0.4 | 12, 15, 23 | Added as the command-verification contract |
| `CR-P1-ROL` | Provider-independent allowlisted internal roles for dialogue, tools, summaries, repository maps, embeddings, reranking, patches, and claim verification | Design now; staged by release | 1, 1A, 31A | Added without a v0.1 provider marketplace |
| `CR-P1-SES` | Session fork, citation-preserving compaction, encrypted archive, controlled deletion, and redacted evidence bundle | v0.2-v0.5 | 10C, 10D, 10E, 23 | Added with privacy and preview gates |
| `CR-P1-SKL` | Declarative, provenance-recorded, trust-gated skills before executable plugins | v0.2-v0.4 | 6, 26A, 28, 31 | Added without independent skill authority |
| `CR-P2-MCP` | Manifested MCP and plugins fully mediated by the kernel, with identity, scopes, limits, cancellation, receipts, and a read-only first phase | v1+ | 4, 5, 26, 26A | Added as a post-write security gate |
| `CR-P2-CLI` | CLI, JSON, software development kit, and Agent Client Protocol only after stable VS Code, with predeclared grants and fail-closed headless behavior | v0.4+ | 4, 10C, 27A, 31 | Added as thin clients to the same kernel |
| `CR-P2-CON` | User-initiated read-only GitHub and connector synchronization into a sensitivity-labeled local cache | v0.7+ | 13A, 13B, 26 | Added with temporary visible network and secret-store boundaries |
| `CR-P2-JOB` | Leased, idempotent, budgeted, cancellable, expiring scheduled work with read-only first release and post-run receipts | v1+ | 23, 24, 24A | Added behind a separate unattended-authority threat model |
| `CR-P2-MAG` | Child grants, separate writable worktrees, ownership, conflicts, budgets, cancellation propagation, and attributable receipts | v1+ | 2C, 13A, 24 | Added without ambient or aggregate authority |

### 35A. Explicitly Rejected Defaults

- [ ] `CAPABILITY GATE` Reject unrestricted, "yolo," blanket, wildcard, or approve-everything tool execution in every interface and operating mode.
- [ ] `CAPABILITY GATE` Treat repository configuration as untrusted data and reject arbitrary executable repository configuration, setup scripts, hooks, tasks, or commands unless separately inspected and granted.
- [ ] `CAPABILITY GATE` Reject automatic download, installation, enabling, updating, or execution of public plugins, skills, hooks, agent packages, models, or capability bundles.
- [ ] `CAPABILITY GATE` Reject every Model Context Protocol server, plugin, hook, client, or alternate endpoint that bypasses kernel policy, grants, limits, cancellation, classification, or receipts.
- [ ] `CAPABILITY GATE` Reject remote semantic indexing, remote embedding, remote reranking, or source upload while the product or session is represented as local mode.
- [ ] `CAPABILITY GATE` Reject silent fallback from a local model or runtime to any cloud model, hosted provider, remote endpoint, or external service.
- [ ] `CAPABILITY GATE` Reject automatic Codex invocation, tab control, prompt population, clipboard transfer, endpoint transfer, or submission; only the user may move a reviewed handoff between the separate interfaces.
- [ ] `CAPABILITY GATE` Reject messaging bots, schedules, background writes, shell execution, publication, and remote state changes in early releases; later support requires its own threat model, gates, and receipts.
- [ ] `CAPABILITY GATE` Reject a broad provider or model marketplace as a first-release objective; expose only approved, manifest-pinned profiles that passed the complete review and evaluation process.
- [ ] `CAPABILITY GATE` Reject any model, embedding, reranker, tokenizer, conversion, runtime, or derived artifact that violates the project's non-Chinese and non-Chinese-derived model-origin policy.
- [ ] `CAPABILITY GATE` Never describe path checks, data directories, pending changes, shadow change sets, Git branches, or Git worktrees as the security sandbox; only the documented operating-system-enforced isolation boundary may carry that name.

### 35B. Competitive Release Gate Additions

| Release | Additive competitive scope | Required boundary |
|---|---|---|
| v0.1 | Native Visual Studio Code Chat, verified Gemma 4 lifecycle, deterministic repository map, read-only tools and Git, evidence states and citations, diagnostics, resumable session, malformed-tool and injection tests | No writes, generic shell, web, MCP, plugins, schedules, connectors, child agents, or automatic Codex action |
| v0.2 | Plain Markdown and Obsidian knowledge, optional approved local embeddings and reranking, index deletion and rebuild, session fork and private evidence export | Remains read-only; semantic indexing is workspace opt-in and local only |
| v0.3 | New-file and exact-preimage file writes through shadow change sets, complete preview, stale rejection, atomic apply, receipts, and rollback | No generic shell; every write has an exact single-use grant |
| v0.4 | Coding worktrees, broader syntax and language-server reading, separately granted test/lint/formatter commands, checkpoints, rollback, and the full CLI after headless policy verification | Worktrees do not replace the operating-system sandbox or grants |
| v0.5 | Redacted evidence and change bundles for user-controlled frontier handoff and untrusted result import | No autonomous tab switching, clipboard operation, cloud submission, or Codex invocation |
| v0.6-v0.7 | Rich administrative artifacts and narrowly scoped, user-initiated, read-only connectors beginning with GitHub | Visible temporary network use; credentials remain outside model context |
| v1+ | Kernel-mediated MCP/plugins, separately threat-modeled schedules, writable connectors, and bounded multi-agent execution | No capability may bypass grants, receipts, retention, cancellation, or offline policy |

### 35C. Additions-Only Maintenance Rule

- [ ] `DOCUMENT` Keep every competitive requirement mapped to a stable inventory section, release, acceptance test, and decision record when it becomes executable.
- [ ] `CAPABILITY GATE` Do not delete, weaken, silently merge away, or move a competitive requirement outside the canonical inventory; any future supersession requires an explicit user-approved decision record that preserves the original text and explains the replacement.
- [ ] `VERIFY` For every competitive integration pass, compare against the prior canonical inventory and prove that all prior checklist entries remain present before accepting the updated file.

## 36. First-GA Delivery and Windows Backlog

Decision 0008 preserves the entire earlier inventory as internal milestone work and adds the following first-GA requirements. Nothing in this section weakens or removes a prior checklist entry. v1.0 is the first supported public release; “full” provider support is bounded to the exact provider/version/object/operation matrix shipped with that release. [`DELIVERY-SYSTEM.md`](./DELIVERY-SYSTEM.md) governs the provider-neutral adapter boundary and [`WINDOWS-BOUNDARIES.md`](./WINDOWS-BOUNDARIES.md) governs Windows 11.

| ID | Requirement | Dependencies | Disposition | Target | Acceptance tests |
|---|---|---|---|---|---|
| `AM-GA-001` | Define v1.0 GA as the first supported public release, freeze its Fedora, Ubuntu, and Windows 11 x64 platform matrix, retain Apple Silicon macOS as post-GA work, and publish exact promoted and unsupported capabilities. | None | Build | v1.0 | `AT-GA-001`, `AT-WIN-001` |
| `AM-DEL-001` | Implement a provider-neutral delivery graph linking services, work, source, reviews, builds, artifacts, provenance, deployments, environments, telemetry, incidents, findings, releases, and rollback with immutable source identities and freshness. | `AM-GA-001` | Build | v1.0 | `AT-DEL-001` |
| `AM-ADP-001` | Implement a versioned provider-adapter SDK, lifecycle, capability-level model, support-matrix schema, event contract, removal contract, and conformance harness. | `AM-DEL-001` | Build | v1.0 | `AT-ADP-001`, `AT-XTE-001` |
| `AM-IDN-001` | Bind every connected credential and operation to exact provider, host, tenant, account, project, environment, capability class, purpose, scope, expiry, and secret-store reference without exposing secret values. | `AM-ADP-001` | Build | v1.0 | `AT-IDN-001`, `AT-XTE-001` |
| `AM-GHE-001` | Support GitHub.com and user-approved GitHub Enterprise Server repositories, commits, issues, pull requests, reviews, checks, workflows, releases, local signed commits, approval-gated pushes, and hosted mutations within a published matrix. | `AM-IDN-001` | Build | v1.0 | `AT-GHE-001`, `AT-XTE-001` |
| `AM-WRK-001` | Implement conformance-gated work-management adapters for GitHub Issues, Jira Cloud, Jira Data Center, and Azure Boards with stable identity, links, drafts, bounded writes, and complete audit receipts. | `AM-IDN-001` | Build | v1.0 | `AT-WRK-001`, `AT-XTE-001` |
| `AM-SRC-001` | Implement conformance-gated source adapters for Azure Repos and GitLab, while keeping Bitbucket and Gerrit as disabled extension candidates until promoted. | `AM-IDN-001` | Build | v1.0 | `AT-SRC-001`, `AT-XTE-001` |
| `AM-CIC-001` | Implement inspect, correlate, dispatch, rerun, cancel, approve, log, and artifact behavior for GitHub Actions, Azure Pipelines, GitLab CI, and Jenkins as separate read and execute capabilities. | `AM-GHE-001`, `AM-SRC-001` | Build | v1.0 | `AT-CIC-001`, `AT-XTE-001` |
| `AM-ART-001` | Implement immutable-digest artifact discovery, verification, download, promotion, retention, and provenance links for OCI Distribution, GitHub Container Registry, Azure Container Registry, JFrog Artifactory, and Sonatype Nexus. | `AM-CIC-001` | Build | v1.0 | `AT-ART-001`, `AT-SUP-001` |
| `AM-SUP-014` | Normalize and verify SPDX, CycloneDX, SARIF, Sigstore/Cosign signatures, SLSA provenance, vulnerability results, and policy decisions against exact source, build, and artifact identities. | `AM-ART-001` | Build | v1.0 | `AT-SUP-001`, `AT-SEC-003` |
| `AM-DEP-001` | Implement plan, diff, promote, health, drift, synchronize, and rollback support for Kubernetes, Helm, Kustomize, Argo CD, and Flux with immutable artifacts and environment-specific deployment grants. | `AM-SUP-014` | Build | v1.0 | `AT-DEP-001`, `AT-XTE-001` |
| `AM-IAC-001` | Implement inspect, plan, policy-check, apply, reconcile, and recover support for Terraform and OpenTofu while keeping destructive and production actions separately approved. | `AM-IDN-001` | Build | v1.0 | `AT-IAC-001`, `AT-XTE-001` |
| `AM-REL-001` | Implement semantic-version, changelog, release-manifest, promotion, progressive-delivery, feature-flag, database-migration, health, rollback, and compensating-action contracts. | `AM-DEP-001`, `AM-IAC-001` | Build | v1.0 | `AT-REL-001`, `AT-XTE-001` |
| `AM-OBS-001` | Use OpenTelemetry as the provider-neutral correlation foundation and implement reference adapters for Datadog, Prometheus/Grafana/Loki, Elastic, Splunk, and Sentry without converting correlation into unsupported causation. | `AM-DEL-001` | Build | v1.0 | `AT-OBS-001`, `AT-XTE-001` |
| `AM-INC-001` | Implement incident correlation, evidence, draft communication, bounded issue creation, escalation, rollback recommendation, and notification adapters with PagerDuty and compatible incident systems as independently gated capabilities. | `AM-OBS-001`, `AM-WRK-001` | Build | v1.0 | `AT-INC-001`, `AT-XTE-001` |
| `AM-SEC-003` | Normalize CodeQL, Semgrep, SonarQube, Snyk, Trivy, Grype, and SARIF findings without erasing provider provenance, conflicting severity, suppression, reachability, or source identity. | `AM-SUP-014` | Build | v1.0 | `AT-SEC-003`, `AT-XTE-001` |
| `AM-CAT-001` | Implement a Backstage reference catalog adapter linking services, owners, repositories, pipelines, artifacts, environments, telemetry, incidents, and releases; retain Port, Cortex, and Compass as conformance-gated extensions. | `AM-DEL-001`, `AM-OBS-001` | Build | v1.0 | `AT-CAT-001`, `AT-XTE-001` |
| `AM-WIN-001` | Implement and package the complete Windows 11 x64 platform boundary, including signed per-user installation, authenticated IPC, restricted workers, NTFS path safety, DPAPI key protection, native model inference, connected workers, accessibility, recovery, and removal. | `AM-GA-001` | Build | v1.0 | `AT-WIN-001`, `AT-XTE-001` |
| `AM-MTH-001` | Author, preserve, review, and render inline and display LaTeX-style mathematics in Markdown through a pinned offline safe subset with round-trip and accessibility support. | `AM-GA-001` | Build | v1.0 | `AT-MTH-001`, `AT-XTE-001` |
| `AM-XTE-001` | Run cross-provider, cross-platform, adversarial, mutation, fuzz, replay, confusion, partition, rate-limit, version-skew, crash, partial-effect, rollback, resource, removal, and end-to-end delivery tests with no hidden blocking result. | `AM-GHE-001`, `AM-WRK-001`, `AM-SRC-001`, `AM-CIC-001`, `AM-ART-001`, `AM-DEP-001`, `AM-IAC-001`, `AM-REL-001`, `AM-OBS-001`, `AM-INC-001`, `AM-SEC-003`, `AM-CAT-001`, `AM-WIN-001`, `AM-MTH-001` | Verify | v1.0 | `AT-XTE-001` |
| `AM-GAD-001` | Produce the v1.0 GA release decision from current Fedora, Ubuntu, Windows, strict-local, connected, provider-conformance, support-matrix, supply-chain, recovery, accessibility, and independent-review evidence. | `AM-XTE-001` | Build | v1.0 | `AT-GA-001` |

## 36A. First-GA Quantitative Acceptance Matrix

| ID | Test | Passing threshold |
|---|---|---|
| `AT-DEL-001` | Build and mutate delivery graphs spanning work item, source, review, CI, artifact, provenance, deployment, telemetry, incident, release, and rollback fixtures. | 100% of observed nodes and evidence-backed edges retain exact provider/host/tenant/object/version/freshness/receipt identity; inferred edges remain non-authoritative; deletion, rename, transfer, and stale observations never resolve to the wrong object. |
| `AT-ADP-001` | Run the adapter lifecycle and capability-level suite against fake, fault, reference, future-version, and malicious providers. | 100% of manifested operations match the support matrix; unsupported operations are absent; malformed or future fields fail safely; removal leaves zero credentials, workers, webhooks, schedules, caches, or network authority. |
| `AT-IDN-001` | Cross product provider, host, tenant, account, project, environment, credential, redirect, proxy, callback, and operation fixtures. | Zero credential disclosure, cross-domain request, authority reuse, or wrong-target effect across at least 2,000 confusion attempts; every denial and attempt is receipted without secret values. |
| `AT-GHE-001` | Exercise the complete published GitHub.com and GitHub Enterprise Server matrix across version, permission, object, branch, issue, pull request, review, workflow, release, local commit, signed push, and failure fixtures. | 100% supported-operation conformance, zero undeclared or duplicate mutation, zero cross-host credential use, exact pre/post state and receipts, and passing negative tests for every unsupported or administrative operation. |
| `AT-WRK-001` | Exercise GitHub Issues, Jira Cloud, Jira Data Center, and Azure Boards reads, links, drafts, field updates, comments, transitions, attachments, permissions, pagination, and recovery. | 100% published field/operation conformance; exact identity and transition semantics; zero duplicate, cross-project, hidden-recipient, or stale-state mutation. |
| `AT-SRC-001` | Exercise Azure Repos and GitLab repository, branch, commit, review, policy, local-worktree, signed-commit, and push fixtures. | 100% published operation conformance; exact immutable revisions; zero active-worktree loss, credential crossover, force operation, or stale push. |
| `AT-CIC-001` | Inspect and execute GitHub Actions, Azure Pipelines, GitLab CI, and Jenkins fixtures across refs, inputs, environments, permissions, logs, artifacts, cancellation, rerun, and uncertain results. | Every execution binds exact source, workflow, inputs, environment, budget, and actor; zero duplicate dispatch across 1,000 timeout/retry cases; logs and artifacts remain bounded, untrusted, secret-scanned, and attributable. |
| `AT-ART-001` | Discover, verify, download, compare, promote, retain, and remove artifacts across OCI, GitHub Container Registry, Azure Container Registry, Artifactory, and Nexus fixtures. | 100% artifact actions resolve immutable digests and provenance; mutable tags carry no authority; zero unsigned, mismatched, cross-repository, traversal, archive, or retention-policy bypass. |
| `AT-SUP-001` | Validate SPDX, CycloneDX, Sigstore/Cosign, SLSA, policy, vulnerability, source, build, and artifact fixture chains. | Every promoted artifact has complete source-to-build-to-artifact identity, valid required signatures and provenance, reconcilable bills of materials, and no hidden critical finding or unsupported assurance claim. |
| `AT-DEP-001` | Plan, preview, promote, health-check, detect drift, synchronize, fail, cancel, and roll back Kubernetes, Helm, Kustomize, Argo CD, and Flux fixtures. | Zero unpreviewed resource or environment change; immutable artifact binding; production approval isolation; deterministic health/timeout behavior; rollback never overwrites later independent changes. |
| `AT-IAC-001` | Inspect, plan, policy-check, apply, crash, reconcile, and recover Terraform and OpenTofu fixtures including destructive, stale-state, lock, provider, and secret cases. | Plan identity and state preconditions match every apply; destructive and production actions remain separate; zero duplicate apply, secret disclosure, stale-state write, or unsafe retry after unknown effect. |
| `AT-REL-001` | Run version, changelog, manifest, promotion, flag, progressive-delivery, migration, health, rollback, and compensation fixtures. | Every release maps exact source, artifact, environment, policy, migration, flag, health, and rollback state; no hidden production, destructive, or database effect; every compensation requires a new approval. |
| `AT-OBS-001` | Correlate OpenTelemetry, Datadog, Prometheus/Grafana/Loki, Elastic, Splunk, and Sentry metrics, logs, traces, errors, monitors, releases, and time windows. | 100% identity and time-window preservation; bounded cardinality and result size; zero secret leakage; correlations are labeled and never promoted to causation without deterministic evidence. |
| `AT-INC-001` | Detect, correlate, draft, notify, create bounded work, recommend rollback, recover, and close incident fixtures across duplicate, delayed, partial, conflicting, and injected data. | Zero autonomous mitigation or notification; exact incident/service/release identity; duplicate events do not duplicate external effects; all communications and remediation actions remain separately previewed and approved. |
| `AT-SEC-003` | Ingest and reconcile CodeQL, Semgrep, SonarQube, Snyk, Trivy, Grype, and SARIF findings with conflicting severity, locations, suppressions, reachability, and tool versions. | 100% original provenance preserved; no conflicting finding silently erased; stale locations are visible; zero finding text changes policy or authority; release gates cannot omit blocking findings. |
| `AT-CAT-001` | Build Backstage service-catalog relationships and run extension-contract fixtures for Port, Cortex, and Compass. | Every catalog relationship cites exact provider evidence; owner and service identity collisions remain unresolved rather than guessed; catalog data cannot mint authority or override provider truth. |
| `AT-WIN-001` | Run clean install, package identity, IPC, AppContainer, NTFS path/race, DPAPI, native model, strict-local network, delivery adapter, accessibility, update, rollback, uninstall, and residue suites on Windows 11 x64. | Three clean standard-user lifecycles pass; zero boundary escape across at least 1,000 path and 500 sandbox attacks; zero outbound bytes in a 60-minute strict-local run; no secret, credential, process, task, pipe, package, or data residue beyond approved retention. |
| `AT-MTH-001` | Round-trip and render Markdown containing inline/display mathematics, labels, references, escaping, code fences, malformed input, hostile commands, deep macros, accessibility labels, and cross-platform fixtures. | 100% source-preserving round trip for the supported subset; deterministic offline rendering on Fedora, Ubuntu, and Windows; zero code execution, file read, network load, package escape, or unbounded expansion; accessible output passes the declared checks. |
| `AT-XTE-001` | Run cross-provider and cross-platform contract mutation, fuzzing, hostile content, credential confusion, event replay, version skew, rate limit, partition, crash, cancellation, partial effect, rollback, resource, removal, and end-to-end lifecycle campaigns. | Zero unauthorized access, disclosure, execution, deployment, administration, duplicate effect, false completion, or hidden blocker; all seeds, corpus, coverage, environment, versions, raw results, shrinks, failures, and reviewer dispositions remain reproducible. |
| `AT-GA-001` | Rebuild all requirements and run the exact v1.0 GA platform, strict-local, connected, delivery, support-matrix, supply-chain, recovery, accessibility, documentation, and independent-review gates. | Every promoted requirement has current code/test/document/evidence/owner/release linkage; every unsupported path has a passing denial/absence test; all blocking checks pass on Fedora, Ubuntu, and Windows; the signed release decision matches raw evidence. |

## 36B. Delivery-System Construction Checklist

### Delivery Kernel and Adapter SDK

- [ ] `CAPABILITY GATE` Keep `observe`, `draft`, `local-write`, `remote-write`, `execute`, `deploy`, `secrets`, and `admin` as independent non-inheriting capability classes.
- [ ] `BUILD` typed delivery objects for service, work item, repository, change, commit, review, build, check, artifact, provenance, environment, deployment, telemetry, incident, finding, release, and rollback.
- [ ] `BUILD` stable provider identities containing exact host, tenant or organization, project, immutable object identifier, mutable display identifier, version, freshness, and source receipt.
- [ ] `BUILD` provider-neutral evidence-backed edges and keep inferred relationships visibly non-authoritative.
- [ ] `BUILD` the adapter lifecycle for describe, diagnose, discover, plan, preview, execute, reconcile, rollback or compensate, and remove.
- [ ] `BUILD` adapter manifests containing tested provider products/versions, host types, objects, operations, scopes, event modes, limits, degradation, support state, and evidence digests.
- [ ] `BUILD` L0 manifested, L1 observable, L2 writable, L3 executable, L4 deployable, and L5 administrative conformance levels.
- [ ] `CAPABILITY GATE` Register only operations that passed their exact provider/version/capability conformance level; keep every unsupported operation absent.
- [ ] `BUILD` namespaced provider extensions for behavior that cannot be normalized truthfully.
- [ ] `BUILD` fake, fault, replay, eventual-consistency, future-version, and malicious provider adapters before live-provider execution.
- [ ] `BUILD` webhook signature, timestamp, nonce, replay, duplicate, ordering, gap, backfill, and tombstone handling plus bounded polling fallback.
- [ ] `VERIFY` complete adapter removal including credentials, caches, events, webhooks, schedules, workers, registrations, and network scopes.

### Identity, Secrets, and External Effects

- [ ] `CAPABILITY GATE` Bind every credential to one exact provider, host, tenant, account, project scope, and permitted capability class.
- [ ] `BUILD` secret-store references and operation-scoped credential delivery without exposing secret values to the model, command line, environment, logs, receipts, diagnostics, or repositories.
- [ ] `BUILD` host identity, Transport Layer Security, redirect, proxy, Domain Name System, callback, and provider-supplied download-target validation.
- [ ] `BUILD` exact effect previews containing actor, target, object, payload, attachment, visibility, source revision, environment, expected state change, cost or budget, and recovery path where applicable.
- [ ] `BUILD` fresh remote precondition reads immediately before approval and immediately before submission.
- [ ] `BUILD` provider-native idempotency where available and deterministic operation fingerprints plus reconciliation where it is not.
- [ ] `CAPABILITY GATE` Treat timeout and transport failure as unknown effect until reconciliation proves effect, non-effect, duplicate effect, or partial effect.
- [ ] `BUILD` verified postconditions and immutable operation receipts for every external attempt, including denial, cancellation, failure, timeout, partial effect, unknown effect, and success.
- [ ] `CAPABILITY GATE` Make rollback and compensation new previewed, approval-gated operations that preserve later independent changes.
- [ ] `VERIFY` at least 2,000 cross-host, cross-tenant, cross-account, cross-project, cross-environment, redirect, and credential-confusion attempts with zero wrong-domain request or disclosure.

### Source, Work, and CI

- [ ] `BUILD` complete GitHub.com and GitHub Enterprise Server conformance within the published repository, branch, commit, issue, pull-request, review, check, workflow, release, and security matrix.
- [ ] `BUILD` exact staged-diff and commit-message approval, required signing, signature verification, and a separate approval for every remote push.
- [ ] `CAPABILITY GATE` Keep force push, secret changes, repository administration, organization administration, and ruleset administration absent unless separately promoted to L5.
- [ ] `BUILD` Jira Cloud, Jira Data Center, GitHub Issues, and Azure Boards field, transition, comment, attachment, link, hierarchy, iteration, and project semantics without flattening provider-only behavior incorrectly.
- [ ] `BUILD` Azure Repos and GitLab repository, branch, commit, merge-request or pull-request, policy, and signed-push adapters.
- [ ] `BUILD` GitHub Actions, Azure Pipelines, GitLab CI, and Jenkins definition, run, job, step, annotation, log, artifact, rerun, cancel, approval, and dispatch adapters.
- [ ] `CAPABILITY GATE` Bind every CI execution to exact workflow or pipeline identity, immutable source revision, inputs, environment, permissions, runner constraints, budget, cancellation, and expected artifacts.
- [ ] `BUILD` log and artifact size limits, streaming cancellation, archive defenses, secret scanning, untrusted-content labeling, and retention.
- [ ] `VERIFY` at least 1,000 CI timeout and retry combinations with zero duplicate dispatch or false completion.

### Artifacts, Deployment, and Infrastructure

- [ ] `BUILD` OCI Distribution, GitHub Container Registry, Azure Container Registry, Artifactory, and Nexus adapters with digest-first identity.
- [ ] `CAPABILITY GATE` Treat tags, names, channels, versions, and build numbers as non-authoritative display identities until resolved to immutable digests.
- [ ] `BUILD` SPDX and CycloneDX bill-of-material ingestion, reconciliation, validation, and exact artifact linking.
- [ ] `BUILD` Sigstore/Cosign verification, SLSA provenance validation, signer policy, policy identity, and source/build/artifact chain evidence.
- [ ] `BUILD` Kubernetes, Helm, and Kustomize read, diff, plan, promote, health, drift, rollback, and resource-ownership operations.
- [ ] `BUILD` Argo CD and Flux application, source, synchronization, health, drift, prune, suspend, resume, and rollback operations with separate destructive grants.
- [ ] `BUILD` Terraform and OpenTofu configuration, provider, module, state, lock, plan, policy, apply, output, drift, import, and recovery operations.
- [ ] `CAPABILITY GATE` Keep plan separate from apply, non-production separate from production, synchronization separate from prune, and deploy separate from secret or administrative changes.
- [ ] `BUILD` immutable release manifests, environment promotion, health windows, rollback targets, progressive delivery, and compensating actions.
- [ ] `BUILD` feature-flag and database-migration adapter contracts with explicit compatibility, direction, backup, lock, timeout, and rollback conditions.

### Observability, Incidents, Security, and Catalogs

- [ ] `BUILD` OpenTelemetry as the canonical telemetry interchange and correlation vocabulary.
- [ ] `BUILD` Datadog, Prometheus/Grafana/Loki, Elastic, Splunk, and Sentry reference adapters for bounded metrics, logs, traces, errors, monitors, dashboards, and releases.
- [ ] `CAPABILITY GATE` Distinguish temporal and structural correlation from causation; require deterministic evidence before making a causal claim.
- [ ] `BUILD` PagerDuty-compatible incident discovery, correlation, evidence package, draft communication, bounded work creation, escalation, and closure workflows.
- [ ] `CAPABILITY GATE` Keep rollback, mitigation, notification, status change, and work creation as separate exact approvals; incident content cannot trigger them automatically.
- [ ] `BUILD` CodeQL, Semgrep, SonarQube, Snyk, Trivy, Grype, and SARIF normalization preserving tool version, rule, location, severity, confidence, reachability, suppression, and original identity.
- [ ] `BUILD` OPA/Conftest-style policy evaluation with exact policy bundle identity, input digest, result, explanation, and release-gate mapping.
- [ ] `BUILD` Backstage service, component, owner, system, repository, pipeline, artifact, environment, telemetry, incident, and release links with exact provider citations.
- [ ] `ROADMAP` Add Port, Cortex, Compass, New Relic, Dynatrace, ServiceNow, Bitbucket, Gerrit, Buildkite, CircleCI, Pulumi, LaunchDarkly, Unleash, Flyway, and Liquibase only through the same staged conformance process.

### Windows 11 and Markdown Mathematics

- [ ] `BUILD` a signed per-user MSIX package and release manifest for current serviced Windows 11 x64.
- [ ] `BUILD` authenticated named-pipe IPC with user, logon-session, integrity-level, executable, package, protocol, sequence, and launch-challenge validation.
- [ ] `BUILD` fresh AppContainer or equivalently reviewed restricted-token tool workers controlled by Job Objects and denied ambient profile, registry, credential, clipboard, device, process, and network access.
- [ ] `BUILD` handle-relative NTFS path resolution defending against reparse points, junctions, links, alternate streams, device and UNC paths, short names, case, Unicode, hard links, rename, replace, and time-of-check/time-of-use races.
- [ ] `BUILD` DPAPI-protected operational keys, reviewed Credential Manager references, local-only data-root checks, native signed `llama.cpp`, and separately confined connected workers.
- [ ] `VERIFY` three clean standard-user Windows package lifecycles and the complete path, sandbox, IPC, strict-local network, recovery, accessibility, update, rollback, uninstall, and residue suites.
- [ ] `BUILD` Markdown inline and display mathematics parsing, syntax preservation, labels, references, escaping, preview, and accessible rendering through pinned offline components.
- [ ] `CAPABILITY GATE` Reject shell escape, file read, network resource, package loading, executable extension, unbounded macro expansion, and unsupported LaTeX commands.
- [ ] `VERIFY` deterministic cross-platform mathematical round trips and rendering on Fedora, Ubuntu, and Windows 11.

### Extreme Verification and First-GA Closure

- [ ] `VERIFY` contract mutation and fuzzing for every schema, parser, IPC message, webhook, provider response, log, archive, manifest, mathematics input, and model-generated operation.
- [ ] `VERIFY` prompt injection in every provider-controlled text field, code file, comment, issue, review, log, artifact, telemetry record, incident, finding, and catalog field.
- [ ] `VERIFY` pagination loops, cursor reuse, duplicate and reordered events, replay, forgery, clock skew, eventual consistency, rate limits, quota exhaustion, permission reduction, credential expiry, provider outage, and version skew.
- [ ] `VERIFY` crash and cancellation before submission, during transport, after remote effect, during reconciliation, during local persistence, and during rollback or compensation.
- [ ] `VERIFY` concurrent user and provider changes, branch movement, object rename or deletion, moved line positions, policy change, environment drift, and artifact replacement.
- [ ] `VERIFY` resource exhaustion from repositories, logs, artifacts, archives, event floods, telemetry cardinality, model context, concurrent adapters, disk, memory, processor, and graphics load.
- [ ] `VERIFY` complete work-item-to-release and incident-to-rollback workflows across the first-GA reference adapter matrix with exact cross-system identity and no hidden authority.
- [ ] `VERIFY` remove every connected capability pack and rerun the complete strict-local suite with zero residual credential, cache, event, process, schedule, listener, or network authority.
- [ ] `DOCUMENT` publish exact provider/version/capability, platform, model, runtime, support, limitation, degradation, upgrade, rollback, and removal matrices with evidence digests.
- [ ] `CAPABILITY GATE` Keep `G-GA` blocked for every failed, skipped, stale, unavailable, flaky, quarantined, suppressed, unreconciled, or unreviewed blocking result.

## 37. First-GA Productivity, Finance, and Cloud Observer Backlog

Decision 0009 preserves every prior requirement and appends the first-GA capability packs governed
by [`PRODUCTIVITY-SYSTEM.md`](./PRODUCTIVITY-SYSTEM.md). These packs are optional at runtime but
required in the first-GA release matrix. Communications can read and write under the Autonomy
Center. Financial-institution and cloud-provider access remains read-only. Money movement and cloud
mutation are absent from v1.0 at every autonomy level.

| ID | Requirement | Dependencies | Disposition | Target | Acceptance tests |
|---|---|---|---|---|---|
| `AM-ATC-001` | Implement a kernel-enforced Autonomy Center with Disabled, Read only, Draft only, Confirm each write, Scoped autonomy, and Autonomous within policy levels plus global and narrower policy ceilings, authenticated changes, visible effective state, expiry, budgets, and emergency external-write disablement. | `AM-ADP-001`, `AM-IDN-001` | Build | v1.0 | `AT-AUT-001` |
| `AM-SYNC-001` | Implement provider-neutral delta, cursor, webhook, change-notification, bounded-polling, backfill, edit, deletion, tombstone, ordering, duplicate, gap, freshness, coverage, and reconciliation contracts for productivity adapters. | `AM-ADP-001`, `AM-IDN-001` | Build | v1.0 | `AT-SYNC-001` |
| `AM-PGR-001` | Extend the evidence-backed graph with provider-scoped people, accounts, organizations, mailboxes, workspaces, channels, meetings, tasks, documents, financial records, and cloud observations while requiring confirmed or deterministic cross-provider identity links. | `AM-DEL-001`, `AM-IDN-001`, `AM-SYNC-001` | Build | v1.0 | `AT-PGR-001` |
| `AM-UIN-001` | Implement a unified activity inbox that preserves native source identity and classifies communication, delivery, meeting, task, finance, and cloud items with citations, freshness, permission gaps, coverage gaps, uncertainty, and local action state. | `AM-PGR-001`, `AM-SYNC-001` | Build | v1.0 | `AT-UIN-001` |
| `AM-COM-001` | Implement a communications adapter contract covering mailbox, conversation, channel, thread, message, recipient, mention, reaction, attachment, formatting, visibility, event, draft, send, edit, delete, archive, recovery, and removal semantics. | `AM-ATC-001`, `AM-PGR-001`, `AM-SYNC-001` | Build | v1.0 | `AT-COM-001` |
| `AM-M365-001` | Implement a conformance-gated Outlook and Exchange Online adapter for supported mailboxes, folders, messages, threads, labels, flags, categories, drafts, sends, replies, forwards, attachments, events, and shared-mailbox behavior through Microsoft Graph. | `AM-COM-001` | Build | v1.0 | `AT-M365-001` |
| `AM-TMS-001` | Implement a conformance-gated Microsoft Teams adapter for teams, channels, chats, threads, messages, replies, mentions, reactions, files, edits, deletions, membership visibility, and supported change notifications. | `AM-COM-001` | Build | v1.0 | `AT-TMS-001` |
| `AM-GML-001` | Implement a conformance-gated Gmail adapter for mailboxes, messages, threads, labels, drafts, sends, replies, forwards, attachments, history, push notifications, bounded polling, recovery, and removal. | `AM-COM-001` | Build | v1.0 | `AT-GML-001` |
| `AM-MAIL-001` | Implement capability-detected IMAP, SMTP, and JMAP adapters plus an authenticated loopback Proton Mail Bridge profile with exact TLS, server, mailbox, folder, feature, credential, rate, and removal boundaries. | `AM-COM-001` | Build | v1.0 | `AT-MAIL-001` |
| `AM-SLK-001` | Implement a conformance-gated Slack adapter for workspaces, channels, direct and group messages, threads, replies, mentions, reactions, files, edits, deletions, events, rate limits, recovery, and removal. | `AM-COM-001` | Build | v1.0 | `AT-SLK-001` |
| `AM-LMAIL-001` | Support Thunderbird, Evolution, and KMail through provider or standard protocol adapters plus explicit read-only mbox and Maildir import without direct writes to private client-profile databases or extraction of client credentials. | `AM-MAIL-001` | Build | v1.0 | `AT-LMAIL-001` |
| `AM-COMW-001` | Register communication reads, drafts, sends, replies, forwards, edits, deletions, reactions, uploads, downloads, moves, labels, flags, and archives as distinct operations with exact recipient, visibility, payload, attachment, idempotency, uncertain-result, and postcondition controls. | `AM-ATC-001`, `AM-COM-001` | Build | v1.0 | `AT-COMW-001` |
| `AM-PIM-001` | Implement Microsoft and Google calendar, contact, and task adapters plus CalDAV and CardDAV interoperability for availability, events, recurrence, invitations, responses, reminders, contacts, tasks, assignments, time zones, conflicts, writes, recovery, and removal. | `AM-ATC-001`, `AM-PGR-001`, `AM-SYNC-001` | Build | v1.0 | `AT-PIM-001` |
| `AM-DREP-001` | Implement conformance-gated OneDrive, SharePoint, Google Drive, and Confluence adapters with separately promoted Notion, Box, and Dropbox extensions covering identity, versions, content, attachments, permissions, links, visibility, drafts, bounded writes, recovery, and removal. | `AM-ATC-001`, `AM-PGR-001`, `AM-SYNC-001` | Build | v1.0 | `AT-DREP-001` |
| `AM-WFA-001` | Compile user-authored event and scheduled productivity workflows into deterministic graphs containing exact triggers, accounts, filters, joins, branches, operations, recipients, classifications, budgets, stop conditions, expiry, approvals, dry runs, recovery, and receipts. | `AM-ATC-001`, `AM-UIN-001`, `AM-PIM-001`, `AM-DREP-001` | Build | v1.0 | `AT-WFA-001` |
| `AM-FIN-001` | Implement a fixed-point financial domain with explicit currency, scale, sign, rounding, account, institution, statement, transaction, pending and posted, split, transfer, category, payee, recurring stream, budget, goal, debt, asset, liability, correction, receipt, invoice, reimbursement, and reconciliation contracts. | `AM-ATC-001`, `AM-IDN-001` | Build | v1.0 | `AT-FIN-001` |
| `AM-FIMPORT-001` | Implement immutable CSV, OFX, and QFX financial imports with profile detection, source hashes, account and statement identity, pending and posted transitions, duplicate protection, transfer matching, corrections, and deterministic reconciliation. | `AM-FIN-001` | Build | v1.0 | `AT-FIMPORT-001` |
| `AM-ACT-001` | Implement Actual Budget as the first local budgeting reference adapter through its supported local API with exact budget-file identity, import, export, account, payee, category, transaction, rule, schedule, budget, synchronization, backup, recovery, and removal behavior. | `AM-FIMPORT-001` | Build | v1.0 | `AT-ACT-001` |
| `AM-BNK-001` | Implement independently admitted read-only financial-data adapters for user-authorized transaction, balance, liability, investment, recurring-stream, and statement data with exact institution, item, account, consent, scope, freshness, cursor, revocation, and deletion behavior. | `AM-FIN-001`, `AM-SYNC-001` | Build | v1.0 | `AT-BNK-001` |
| `AM-BUD-001` | Implement deterministic category and envelope budgets, cash-flow forecasts, savings goals, debt scenarios, net-worth views, scenario comparisons, confidence and assumption disclosure, and local user-approved budget rules. | `AM-FIMPORT-001` | Build | v1.0 | `AT-BUD-001` |
| `AM-BIL-001` | Implement recurring bill, income, fee, subscription, price-change, due-date, missing-payment, and cancellation-candidate detection with exact transaction evidence, communication links, user disposition, reminders, and no autonomous cancellation or payment. | `AM-BUD-001`, `AM-UIN-001` | Build | v1.0 | `AT-BIL-001` |
| `AM-FDOC-001` | Implement receipt, invoice, reimbursement, statement, and tax-document organization with source-preserving extraction, transaction matching, duplicate detection, classification, retention, redaction, report export, and no unsupported tax or legal claim. | `AM-FIN-001`, `AM-DREP-001` | Build | v1.0 | `AT-FDOC-001` |
| `AM-FANL-001` | Implement explainable statistical outlier and potential-fraud indicators that retain features, baseline, model or rule identity, confidence, limitations, source evidence, drift, user disposition, and no definitive fraud or automated external-action claim. | `AM-FIN-001`, `AM-FIMPORT-001` | Build | v1.0 | `AT-FANL-001` |
| `AM-ACC-001` | Implement separately gated QuickBooks Online and Xero adapters for reads, drafts, and approval-gated non-money-movement accounting records after exact precision, identity, period, tax-code, reconciliation, idempotency, recovery, and removal gates pass. | `AM-ATC-001`, `AM-FIN-001` | Build | v1.0 | `AT-ACC-001` |
| `AM-FPRV-001` | Enforce dedicated financial classification, encryption, minimization, retention, model-context, cross-pack disclosure, export, backup, deletion, and money-movement absence controls across every shell, policy, workflow, adapter, schedule, and autonomy level. | `AM-BNK-001`, `AM-BUD-001`, `AM-BIL-001`, `AM-FDOC-001`, `AM-FANL-001`, `AM-ACC-001` | Build | v1.0 | `AT-FPRV-001` |
| `AM-CLO-001` | Implement a strictly read-only Cloud Observer contract with provider, organization or tenant, account, subscription or project, region, service, resource, query, time, field, row, byte, rate, credential, cache, freshness, correlation, recovery, and removal boundaries. | `AM-ATC-001`, `AM-ADP-001`, `AM-IDN-001` | Build | v1.0 | `AT-CLO-001` |
| `AM-AWS-001` | Implement a conformance-gated AWS observer for approved organization and account inventory, resource configuration, tags, health, bounded CloudWatch metrics and logs, CloudTrail references, security observations, deployment identity, and Cost Explorer summaries. | `AM-CLO-001` | Build | v1.0 | `AT-AWS-001` |
| `AM-AZR-001` | Implement a conformance-gated Azure observer for approved tenant, subscription, resource graph, configuration, tags, health, Azure Monitor metrics and logs, activity references, security observations, deployment identity, and Cost Management summaries. | `AM-CLO-001` | Build | v1.0 | `AT-AZR-001` |
| `AM-GCP-001` | Implement a conformance-gated Google Cloud observer for approved organization, folder, project, asset, configuration, labels, health, Cloud Monitoring metrics and logs, audit references, security observations, deployment identity, and billing summaries. | `AM-CLO-001` | Build | v1.0 | `AT-GCP-001` |
| `AM-CCST-001` | Correlate bounded cloud health, configuration, metric, log, security, deployment, and cost observations with exact delivery and productivity identities without asserting unsupported causation or creating cloud or delivery authority. | `AM-AWS-001`, `AM-AZR-001`, `AM-GCP-001`, `AM-PGR-001` | Build | v1.0 | `AT-CCST-001` |
| `AM-XPR-001` | Run cross-pack autonomy, identity, recipient, synchronization, injection, attachment, financial-precision, reconciliation, money-movement-absence, cloud-mutation-absence, resource, crash, recovery, removal, accessibility, and strict-local-restoration campaigns with no hidden blocker. | `AM-WFA-001`, `AM-COMW-001`, `AM-FPRV-001`, `AM-CCST-001` | Verify | v1.0 | `AT-XPR-001` |
| `AM-GAD-002` | Produce the expanded v1.0 GA release decision from current delivery, Windows, productivity, communications, finance, cloud-observer, cross-pack, strict-local, support-matrix, recovery, accessibility, and independent-review evidence at Sprint 156. | `AM-GAD-001`, `AM-XPR-001` | Build | v1.0 | `AT-GA-002` |

## 37A. Productivity, Finance, and Cloud Quantitative Acceptance Matrix

| ID | Test | Passing threshold |
|---|---|---|
| `AT-AUT-001` | Cross product every autonomy level, parent and child ceiling, pack, connector, account, workspace, operation, destination, recipient, channel, schedule, budget, expiry, stale display, replay, race, cancellation, and emergency-disable fixture. | Zero effective-policy broadening across at least 2,000 mutations; every authenticated change matches its preview and version; emergency disablement prevents all new external writes and reconciles in-flight effects. |
| `AT-SYNC-001` | Exercise signed events, change notifications, delta cursors, polling, edits, deletions, tombstones, gaps, backfill, duplicates, reordering, clock skew, revocation, throttling, partition, restart, and removal. | Exact deterministic provider state or visible incomplete state after every case; zero duplicate action, hidden gap, stale result represented as current, event-created authority, or residual event path. |
| `AT-PGR-001` | Build and mutate identity and work graphs across people, accounts, organizations, mailboxes, channels, meetings, tasks, documents, delivery objects, financial records, and cloud observations. | Every native identity and observed edge retains exact source and freshness; unconfirmed aliases, display names, homoglyphs, transfers, renames, and inferred links never authorize a recipient, account, object, or action. |
| `AT-UIN-001` | Build unified-inbox views from mixed communication, delivery, personal-information, finance, and cloud fixtures with unread, due, blocked, duplicate, stale, incomplete, and uncertain states. | 100% source and freshness preservation; deterministic classification rules reproduce exactly; model priority remains labeled; permission and coverage gaps cannot appear as empty or complete results. |
| `AT-COM-001` | Run the common communications contract against fake, fault, future-version, malicious, Microsoft, Google, Slack, Proton Bridge, IMAP, SMTP, and JMAP fixtures. | 100% manifested object and operation conformance; unsupported operations are absent; every provider transformation is visible; removal leaves zero credential, cache, cursor, event, schedule, worker, or network authority. |
| `AT-M365-001` | Exercise the published Outlook and Exchange Online matrix across mailbox, folder, thread, message, flag, category, draft, send, reply, forward, attachment, shared-mailbox, permission, pagination, event, failure, and recovery fixtures. | Exact Graph identity and scope for every supported operation; zero wrong mailbox, hidden recipient, duplicate send, stale write, credential crossover, or unsupported shared-mailbox claim. |
| `AT-TMS-001` | Exercise the published Teams matrix across tenant, team, channel, chat, thread, message, reply, mention, reaction, file, edit, deletion, membership, event, permission, failure, and recovery fixtures. | Exact tenant, destination, thread, membership, and visibility for every operation; zero cross-chat, hidden mention, duplicate effect, stale edit, attachment disclosure, or unsupported personal-account claim. |
| `AT-GML-001` | Exercise the published Gmail matrix across mailbox, message, thread, label, draft, send, reply, forward, attachment, history, push, polling, permission, failure, and recovery fixtures. | Exact account, thread, label, recipient, content, and attachment state; zero duplicate send, history gap hidden as complete, broad-scope substitution, stale write, or cross-account token use. |
| `AT-MAIL-001` | Exercise capability-negotiated IMAP, SMTP, JMAP, and Proton Bridge fixtures across server identity, TLS, authentication reference, mailbox, folder, feature, formatting, attachment, rate, disconnect, and removal states. | Zero downgrade, wrong-server credential, loopback escape, unsupported-feature registration, duplicate delivery, profile leakage, or residue; every protocol difference remains explicit. |
| `AT-SLK-001` | Exercise the published Slack matrix across workspace, channel, direct and group message, thread, reply, mention, reaction, file, edit, deletion, event, scope, pagination, rate, failure, and recovery fixtures. | Exact workspace, destination, membership, thread, visibility, and rate behavior; zero cross-workspace token use, hidden broad mention, duplicate post, stale edit, file disclosure, or omitted rate-limit state. |
| `AT-LMAIL-001` | Run Thunderbird, Evolution, KMail, mbox, and Maildir interoperability against versioned, locked, corrupted, replaced, linked, raced, credential-seeded, and unsupported profile fixtures. | Zero private-profile write, credential extraction, path escape, corruption, or unsupported claim; provider and protocol operations remain canonical and explicit imports remain read-only. |
| `AT-COMW-001` | Mutate sender, account, recipient, external domain, channel, thread, mention, visibility, body, formatting, quote, link, attachment, classification, operation, idempotency key, provider result, and postcondition around every communication effect. | Zero unauthorized or duplicate effect across at least 2,000 mutations and 1,000 timeout or retry cases; every result is reconciled and every changed field invalidates prior approval. |
| `AT-PIM-001` | Exercise Microsoft, Google, CalDAV, and CardDAV calendar, contact, and task matrices across identity, availability, time zone, recurrence, invitations, responses, reminders, contacts, assignments, permissions, conflicts, writes, failures, and removal. | Exact event, person, task, time, recurrence, recipient, and visibility state; zero accidental invitation, contact merge, task assignment, conflict suppression, duplicate effect, or cross-account write. |
| `AT-DREP-001` | Exercise OneDrive, SharePoint, Google Drive, Confluence, and promoted extension fixtures across identity, version, content, attachment, link, permission, visibility, draft, write, conflict, malicious file, recovery, and removal. | Exact version and permission semantics; zero public-link broadening, wrong-document write, attachment swap, executable-content escape, cross-repository credential use, or hidden conflict. |
| `AT-WFA-001` | Compile, dry run, execute, cancel, crash, resume, expire, and remove productivity workflows with nested triggers, branches, joins, classifications, accounts, recipients, budgets, approvals, failures, and hostile external content. | Every executed node matches the approved deterministic graph; zero self-edit, recursion escape, hidden branch, destination substitution, authority aggregation, message-created instruction, or omitted failure. |
| `AT-FIN-001` | Exercise financial values and records across currencies, scales, signs, rounding modes, boundary magnitudes, splits, transfers, categories, recurring streams, debts, assets, liabilities, corrections, and aggregate order. | Exact deterministic decimal results for 100% of fixtures; no floating-point financial decision, currency ambiguity, silent overflow, order-dependent total, or lineage loss. |
| `AT-FIMPORT-001` | Import and reconcile CSV, OFX, and QFX fixtures across encodings, profiles, accounts, periods, opening and closing balances, pending and posted transitions, duplicates, transfers, splits, corrections, malformed data, and restarts. | Exact source hashes and account identity; zero duplicate or missing accepted transaction; no false reconciliation; every conflict, exclusion, correction, and unsupported field remains visible. |
| `AT-ACT-001` | Exercise Actual Budget files through supported local API fixtures across import, export, account, payee, category, transaction, rule, schedule, budget, sync, backup, crash, recovery, and removal. | 100% supported-operation conformance against exact budget identity; zero wrong-file write, duplicate transaction, precision loss, silent rule change, corrupt recovery, network broadening, or residue. |
| `AT-BNK-001` | Exercise read-only financial-data fixtures across provider, institution, item, consent, account, product, transaction, balance, liability, investment, recurring stream, statement, cursor, freshness, revocation, deletion, outage, and removal. | Zero money movement or administration method registration; zero cross-account data, token disclosure, stale data represented as current, duplicate accepted transaction, hidden deletion, or post-revocation read. |
| `AT-BUD-001` | Run budget, cash-flow, savings, debt, net-worth, scenario, assumption, confidence, category-rule, missing-data, and changed-history fixtures. | Exact reproducible arithmetic and source periods; every assumption and uncertainty is visible; no scenario is presented as prediction, advice, or completed action; changed inputs invalidate prior output. |
| `AT-BIL-001` | Detect bills, income, fees, subscriptions, price changes, due dates, missing payments, and cancellation candidates across recurring, irregular, duplicate, refunded, renamed, and sparse-history fixtures. | Every alert cites exact evidence and confidence; false-positive and missing-history state is visible; zero autonomous cancellation, payment, commitment, recipient selection, or external contact. |
| `AT-FDOC-001` | Match receipts, invoices, reimbursements, statements, and tax documents to transactions across duplicate, partial, multi-page, multi-currency, altered, malicious, unmatched, and redacted fixtures. | Exact source and transaction identity; zero false match represented as fact, executable-content escape, sensitive export, unsupported tax or legal claim, or retention bypass. |
| `AT-FANL-001` | Run statistical outlier and potential-fraud indicators across stable, drifting, sparse, poisoned, seasonal, duplicate-merchant, changed-category, adversarial-description, and known-anomaly fixtures. | Every indicator retains features, baseline, method, confidence, limitation, drift, and source evidence; no definitive fraud claim, hidden bias, authority change, or automated external effect. |
| `AT-ACC-001` | Exercise QuickBooks Online and Xero reads, drafts, and promoted non-money-movement writes across organization, ledger, account, contact, item, invoice, bill, journal, period, tax code, precision, duplicate, conflict, timeout, recovery, and removal. | Exact organization, period, currency, record, tax, and postcondition identity; zero payment, transfer, bank action, duplicate write, stale-period mutation, precision loss, cross-organization credential use, or residue. |
| `AT-FPRV-001` | Seed financial canaries and probe classification, encryption, memory, model context, logs, diagnostics, exports, backups, cross-pack flows, deletion, crash, removal, and every money-movement family at every autonomy level. | Zero undeclared financial disclosure and zero money-movement or financial-administration request; all retained records match explicit policy and removal leaves no unauthorized credential, cache, index, schedule, or worker. |
| `AT-CLO-001` | Run the common Cloud Observer contract against fake, fault, future-version, malicious, AWS, Azure, and Google Cloud fixtures across scope, query, result, rate, cache, correlation, failure, recovery, and removal. | 100% manifested read conformance; write, execute, deploy, secret, identity, policy, and admin operations are absent; no out-of-scope data, hidden truncation, false freshness, content-created authority, or residue. |
| `AT-AWS-001` | Exercise AWS organization, account, region, service, resource, configuration, tag, health, CloudWatch, CloudTrail reference, security observation, deployment identity, cost, role, token, throttling, failure, and removal fixtures. | Exact account and resource scope; zero assume-role escape, data-plane broadening, secret value, remote command, resource write, logging change, budget change, or cross-account credential use. |
| `AT-AZR-001` | Exercise Azure tenant, subscription, region, resource graph, configuration, tag, health, Monitor, activity reference, security observation, deployment identity, cost, role, token, throttling, failure, and removal fixtures. | Exact tenant, subscription, and resource scope; zero run-command, data-plane broadening, secret value, resource write, identity or policy change, logging change, budget change, or cross-subscription credential use. |
| `AT-GCP-001` | Exercise Google Cloud organization, folder, project, region, asset, configuration, label, health, Monitoring, audit reference, security observation, deployment identity, billing, role, token, quota, failure, and removal fixtures. | Exact organization, project, and resource scope; zero SSH or command, data-plane broadening, secret value, resource write, identity or policy change, logging change, budget change, or cross-project credential use. |
| `AT-CCST-001` | Correlate cloud configuration, health, metrics, logs, security observations, deployments, and costs with delivery and productivity fixtures across identity collision, time skew, rename, transfer, stale data, conflicting evidence, and injected causation. | 100% native identity and time-window preservation; correlations remain labeled; zero unsupported causation, remediation, delivery authority, cloud authority, cross-pack disclosure, or hidden cost assumption. |
| `AT-XPR-001` | Run cross-pack mutation, fuzz, injection, identity, recipient, attachment, synchronization, financial-precision, reconciliation, money-movement-absence, cloud-mutation-absence, resource, crash, cancellation, recovery, removal, accessibility, and end-to-end campaigns. | Zero unauthorized disclosure or effect, zero money movement, zero cloud mutation, zero hidden blocker, and zero residual authority; all corpus, raw results, environments, failures, and reviewer dispositions remain reproducible. |
| `AT-GA-002` | Rebuild all requirements and run the exact expanded v1.0 platform, delivery, productivity, communications, finance, cloud-observer, support-matrix, recovery, accessibility, removal, documentation, and independent-review gates. | Every promoted requirement has current code, test, document, evidence, owner, and release linkage; every unsupported path has a passing absence or denial test; all blocking checks pass on Fedora, Ubuntu, and Windows; the Sprint 156 signed decision matches raw evidence. |

## 37B. Productivity-System Construction Checklist

### Autonomy, Identity, and Unified Work

- [ ] `BUILD` a closed autonomy-level schema and kernel policy intersection across global, pack, connector, account, workspace, operation, destination, recipient, channel, payload, schedule, budget, and expiry ceilings.
- [ ] `BUILD` the native Visual Studio Code Autonomy control as a non-authoritative display and authenticated policy-change surface.
- [ ] `BUILD` exact autonomy-delta previews, versioned decisions, stale-state rejection, session reauthentication, expiry, cancellation, and rollback to a narrower policy.
- [ ] `BUILD` a global external-write disable action that blocks new grants, cancels queues, reconciles in-flight effects, and leaves read-only diagnostics available.
- [ ] `CAPABILITY GATE` Keep financial money movement and cloud mutation prohibited regardless of autonomy level, standing consent, schedule, workflow, or model request.
- [ ] `BUILD` provider-scoped identities for people, organizations, accounts, mailboxes, workspaces, teams, channels, calendars, task stores, document repositories, financial accounts, and cloud accounts.
- [ ] `CAPABILITY GATE` Require user-confirmed or deterministic cross-provider identity links; never authorize from display-name, alias, title, timestamp, or model similarity alone.
- [ ] `BUILD` evidence-backed work-graph edges with source, observation time, classification, confidence class, freshness, and removal tombstones.
- [ ] `BUILD` a unified activity inbox with action, response, decision, waiting, blocked, due, reference, duplicate, stale, incomplete, and uncertain states.
- [ ] `BUILD` explicit permission-gap, history-gap, freshness, cursor, synchronization-health, and coverage displays so an incomplete source never appears empty or complete.

### Synchronization and Provider Lifecycle

- [ ] `BUILD` common webhook, change-notification, delta, cursor, bounded-polling, backfill, edit, deletion, tombstone, ordering, duplicate, gap, and reconciliation contracts.
- [ ] `BUILD` one operation-scoped provider worker per exact account, capability, credential reference, destination, byte and time budget, cancellation path, and receipt.
- [ ] `BUILD` provider manifests for API or protocol version, scopes, objects, operations, events, rate limits, consistency, attachments, formatting, degradation, recovery, and removal.
- [ ] `CAPABILITY GATE` Keep unsupported provider operations absent from tool registration and expose permission or tenant-policy limitations truthfully.
- [ ] `VERIFY` event forgery, replay, duplication, omission, reordering, cursor loss, backfill, clock skew, rate limit, revocation, outage, partition, and restart.
- [ ] `VERIFY` complete connector removal including credentials, caches, cursors, webhooks, schedules, indexes, workers, sockets, network scopes, and retained data.

### Email, Chat, and Linux Mail Clients

- [ ] `BUILD` Microsoft Graph Outlook and Exchange Online support for manifested mailbox, folder, thread, message, draft, send, reply, forward, attachment, flag, category, event, and shared-mailbox operations.
- [ ] `BUILD` Microsoft Teams support for manifested team, channel, chat, thread, message, reply, mention, reaction, file, edit, deletion, membership, and event operations.
- [ ] `BUILD` Gmail support for manifested mailbox, message, thread, label, draft, send, reply, forward, attachment, history, push, and polling operations.
- [ ] `BUILD` Slack support for manifested workspace, channel, direct and group message, thread, reply, mention, reaction, file, edit, deletion, and event operations.
- [ ] `BUILD` capability-detected IMAP, SMTP, and JMAP with exact server identity, TLS, authentication reference, mailbox, folder, extension, rate, and disconnect behavior.
- [ ] `BUILD` a Proton Mail Bridge profile limited to its authenticated loopback endpoints and one exact mailbox operation.
- [ ] `BUILD` Thunderbird, Evolution, and KMail interoperability through provider or standard protocols plus explicit read-only mbox and Maildir import.
- [ ] `CAPABILITY GATE` Prohibit direct private-profile database writes, credential extraction, UI scraping with standing authority, and protocol downgrade.
- [ ] `BUILD` distinct read, draft, send, reply, forward, edit, delete, reaction, upload, download, move, label, flag, and archive tools where provider evidence supports them.
- [ ] `BUILD` outward-effect previews containing provider, tenant, account, sender, recipients, channel, thread, visibility, subject, body, formatting, mentions, quotes, links, attachments, classification, provider transformation, idempotency, and postcondition.
- [ ] `VERIFY` at least 2,000 communication-field mutations and 1,000 timeout, retry, partial-effect, duplicate, and uncertain-result cases with zero wrong or duplicate delivery.
- [ ] `VERIFY` message, signature, quote, attachment, invitation, linked-page, hidden-text, script, macro, false-completion, and recipient-substitution prompt injection.

### Calendar, Contacts, Tasks, Documents, and Workflows

- [ ] `BUILD` Outlook and Google calendar plus CalDAV event, availability, time-zone, recurrence, invitation, response, reminder, resource, conflict, and write semantics.
- [ ] `BUILD` Microsoft and Google contacts plus CardDAV identity, address, organization, merge, update, duplicate, conflict, and removal semantics.
- [ ] `BUILD` Microsoft To Do and Planner, Google Tasks, and local task identity, assignment, due date, status, dependency, reminder, write, and recovery semantics.
- [ ] `BUILD` OneDrive, SharePoint, Google Drive, and Confluence document identity, version, content, attachment, permission, link, visibility, draft, write, conflict, and removal semantics.
- [ ] `ROADMAP` Promote Notion, Box, Dropbox, and additional repositories only after their independent manifests and conformance suites pass.
- [ ] `BUILD` attachment hash, size, media-type, classification, malware, archive, macro, link, permission, visibility, and post-preview-change checks.
- [ ] `BUILD` deterministic meeting agenda, transcript-import, decision, commitment, follow-up, deadline, attachment, and cited source workflows.
- [ ] `BUILD` a workflow compiler with exact trigger, account, filter, join, branch, operation, recipient, classification, budget, stop, expiry, approval, dry-run, failure, and receipt nodes.
- [ ] `CAPABILITY GATE` Prohibit workflow self-edit, recursive expansion, hidden branches, undeclared destinations, approval aggregation, message-created instructions, and cross-pack grant reuse.

### Finance and Budgeting

- [ ] `BUILD` fixed-point decimal money values with explicit currency, scale, sign, rounding rule, effective date, conversion source, overflow behavior, and serialization.
- [ ] `BUILD` canonical financial account, institution, statement, transaction, pending, posted, split, transfer, category, payee, recurring stream, budget, goal, debt, asset, liability, correction, receipt, invoice, reimbursement, tax label, and reconciliation records.
- [ ] `CAPABILITY GATE` Preserve imported source records immutably and express corrections through adjustment or supersession records.
- [ ] `BUILD` CSV, OFX, and QFX profile detection, parsing, source hashing, account identity, encoding, duplicate protection, pending-to-posted matching, transfer matching, and error reporting.
- [ ] `BUILD` statement reconciliation with exact period, opening and closing balances, included records, pending exclusions, corrections, tolerance, conflict, and reviewer disposition.
- [ ] `BUILD` Actual Budget support through its approved local API for budget identity, accounts, payees, categories, transactions, rules, schedules, budgets, sync, backup, recovery, and removal.
- [ ] `BUILD` independently admitted read-only transaction, balance, liability, investment, recurring-stream, and statement adapters with consent, scope, freshness, cursor, revocation, and deletion state.
- [ ] `BUILD` deterministic category and envelope budgets, cash-flow forecasts, savings goals, debt scenarios, net-worth views, scenario comparisons, assumptions, and confidence disclosures.
- [ ] `BUILD` recurring bill, income, fee, subscription, price-change, due-date, missing-payment, and cancellation-candidate detection with source evidence and user disposition.
- [ ] `BUILD` receipt, invoice, reimbursement, statement, and tax-document extraction, matching, duplicate detection, retention, redaction, and report export.
- [ ] `BUILD` explainable statistical outlier and potential-fraud indicators with feature, baseline, method, confidence, limitation, drift, source, and user-disposition records.
- [ ] `BUILD` QuickBooks Online and Xero reads, drafts, and approval-gated non-money-movement accounting writes only after precision, period, tax, reconciliation, idempotency, recovery, and removal gates pass.
- [ ] `CAPABILITY GATE` Omit transfers, payments, bill pay, trades, orders, withdrawals, deposits, credit, loans, tax filing, beneficiaries, financial-account administration, and credential recovery from every first-GA schema, manifest, policy, tool, adapter, shell, workflow, and autonomy level.
- [ ] `CAPABILITY GATE` Keep financial analysis labeled as analysis, scenario, proposal, or indicator and never as guaranteed prediction, definitive fraud, financial advice, tax advice, legal advice, or completed action.
- [ ] `VERIFY` financial precision, rounding, currency, overflow, import, duplicate, pending and posted, transfer, split, correction, statement, reconciliation, budget, forecast, anomaly, privacy, crash, recovery, and removal campaigns.

### Cloud Observer

- [ ] `BUILD` a closed Cloud Observer read schema for provider, organization or tenant, account, subscription or project, region, service, resource, query, time, field, row, byte, rate, credential, cache, freshness, and result identity.
- [ ] `BUILD` AWS organization and account inventory, resource configuration, tags, health, bounded CloudWatch metrics and logs, CloudTrail references, security observations, deployment identity, and Cost Explorer summaries.
- [ ] `BUILD` Azure tenant and subscription inventory, resource graph, configuration, tags, health, bounded Monitor metrics and logs, activity references, security observations, deployment identity, and Cost Management summaries.
- [ ] `BUILD` Google Cloud organization, folder, project, and asset inventory, configuration, labels, health, bounded Monitoring metrics and logs, audit references, security observations, deployment identity, and billing summaries.
- [ ] `CAPABILITY GATE` Omit resource writes, remote commands, shells, deploys, secret values, credential rotation, identity, policy, logging, budget, upload, delete, and administration from every Cloud Observer schema, manifest, policy, tool, adapter, shell, workflow, and autonomy level.
- [ ] `CAPABILITY GATE` Keep cloud observations and correlations non-authoritative; require separate delivery-system plans, policies, previews, and grants for every delivery or infrastructure effect.
- [ ] `VERIFY` assume-role, tenant, subscription, project, region, service, resource, data-plane, query, time, field, token, redirect, proxy, credential, quota, log-volume, cost, and content-injection attacks.
- [ ] `VERIFY` every cloud write, execute, deploy, secret, identity, policy, logging, budget, upload, delete, and administration family is absent or denied before a provider request.

### Cross-Pack and Expanded GA Closure

- [ ] `CAPABILITY GATE` Require an explicit classified and receipted data-flow rule before communication, delivery, finance, cloud, document, calendar, task, or memory content crosses pack boundaries.
- [ ] `VERIFY` cross-pack credential, cache, cursor, identity, context, classification, retention, schedule, grant, recipient, attachment, financial-record, and cloud-resource confusion attacks.
- [ ] `VERIFY` end-to-end communication-to-work, meeting-to-commitment, bill-to-budget, receipt-to-transaction, cloud-observation-to-delivery-plan, and incident-to-approved-communication workflows.
- [ ] `VERIFY` disable and remove every new pack independently and together during idle, queued, in-flight, uncertain, stale, and partially synchronized states.
- [ ] `VERIFY` rerun complete strict-local and delivery suites after removal with zero residual credential, cache, cursor, webhook, schedule, index, process, socket, listener, or network authority.
- [ ] `VERIFY` keyboard-only, screen-reader, focus, status, error, contrast, dynamic-content, and confirmation behavior for autonomy, unified inbox, synchronization health, communication previews, financial reports, cloud observations, and emergency disablement.
- [ ] `DOCUMENT` publish exact provider, protocol, version, object, operation, scope, autonomy, rate, freshness, degradation, limitation, retention, recovery, removal, and support matrices with evidence digests.
- [ ] `CAPABILITY GATE` Preserve Sprint 126 as a stable delivery-and-Windows checkpoint and keep expanded `G-GA` blocked until Sprint 156 passes every Decision 0009 requirement and acceptance test.

## 38. First-GA Trusted Operations, Research, Continuity, and Model Management Backlog

Decision 0010 promotes the existing command, browser-research, credential, backup, and model-installer
foundations into an explicit first-GA architecture. Prior requirements and checklist rows remain
unchanged. The requirements below add the authority, isolation, continuity, usability, and release
evidence needed to support those capabilities publicly.

| ID | Requirement | Dependencies | Disposition | Release | Acceptance |
|---|---|---|---|---|---|
| `AM-TRU-001` | Implement the `TRUSTED-OPERATIONS.md` architecture with separately registered and removable command, public-research, credential-broker, continuity, and model-manager capability classes, workers, data flows, network paths, limits, receipts, recovery, and platform evidence. | `AM-ATC-001`, `AM-XTE-001` | Build | v1.0 | `AT-TRU-001` |
| `AM-CRD-001` | Implement an operating-system-backed credential broker using typed non-secret references, exact provider/host/tenant/account/operation/scope binding, system-browser or device OAuth where available, operation-scoped resolution, rotation, revocation, redaction, removal, and reauthentication after restore. | `AM-TRU-001`, `AM-IDN-001` | Build | v1.0 | `AT-CRD-001` |
| `AM-AUT-002` | Extend command authority with Disabled, Inspect, Workspace Autonomous, Connected Operations, and explicit Owner / Unrestricted Session levels intersected with every global and narrower autonomy ceiling. Owner mode requires direct authenticated activation, visible risk state, bounded duration, panic stop, automatic revocation, and prohibition from schedules, inheritance, model activation, or automatic renewal. | `AM-TRU-001`, `AM-ATC-001` | Build | v1.0 | `AT-AUT-002` |
| `AM-CLI-002` | Implement a fully capable local command execution broker supporting direct executables, shell syntax, pipelines, redirection, scripting, and explicit PTY sessions with exact command, working directory, path, environment, network, credential, descendant, resource, timeout, cancellation, changed-file, risk, rollback, and receipt contracts. | `AM-AUT-002`, `AM-CRD-001` | Build | v1.0 | `AT-CLI-002` |
| `AM-WEB-001` | Implement current public Internet search and retrieval with query, recency, domain, redirect, transport, item, byte, media, download-quarantine, cache, source-quality, publication/event-date, claim-level citation, excerpt-hash, inference-label, private-disclosure, cancellation, and hostile-content controls. | `AM-TRU-001`, `AM-CRD-001` | Build | v1.0 | `AT-WEB-001` |
| `AM-BKC-001` | Implement immutable client-side-encrypted local continuity snapshots with versioned manifests, integrity trees, data classification, explicit exclusions, retention, atomic completion, staging restore, migration, clean-device recovery, rollback, deletion, and no live operational state on cloud-synchronized or network storage. | `AM-TRU-001` | Build | v1.0 | `AT-BKC-001` |
| `AM-CBK-001` | Implement optional cloud continuity through a separately writable, provider-scoped backup worker with one exact encrypted destination namespace, least-privilege credential, opaque object naming, bounded transfer, resumability, versioning, retention, integrity, deletion, restore, disaster-recovery, removal, and zero broadening of Cloud Observer. | `AM-BKC-001`, `AM-CRD-001`, `AM-ADP-001`, `AM-CLO-001` | Build | v1.0 | `AT-CBK-001` |
| `AM-MCAT-001` | Implement a signed approved-model catalog with exact candidate, evaluating, approved, degraded, quarantined, rejected, and retired states plus publisher, developer, license, origin, lineage, artifact, transformation, hash, size, runtime, platform, hardware, quality, security, support, and re-review evidence. | `AM-TRU-001` | Build | v1.0 | `AT-MCAT-001` |
| `AM-MGR-001` | Implement a deterministic chat-guided model manager that can list compatible approved profiles and perform user-confirmed download, bounded resume, import, quarantine, hash and signature verification, scan, admission self-test, atomic activation, comparison, rollback, removal, and storage cleanup through the separate installer/importer. | `AM-MCAT-001`, `AM-CRD-001` | Build | v1.0 | `AT-MGR-001` |
| `AM-MUSE-001` | Add Meta Muse Glimmer to the candidate catalog and produce a truthful `PASS`, `BLOCKED`, or `REJECTED` admission disposition from verified first-party identity, open-source or open-weight classification, license, origin, lineage, artifact, transformation, runtime, resource, coding-quality, tool, security, and platform evidence without making first GA depend on a pass. | `AM-MCAT-001`, `AM-MGR-001` | Verify | v1.0 | `AT-MUSE-001` |
| `AM-XOP-001` | Run integrated command-authority, owner-session, web-injection, private-disclosure, credential-leakage, backup-integrity, cross-account, restore, model-substitution, interruption, resource, accessibility, removal, and strict-local-restoration campaigns across every first-GA platform with no hidden blocker. | `AM-CLI-002`, `AM-WEB-001`, `AM-CBK-001`, `AM-MGR-001`, `AM-MUSE-001` | Verify | v1.0 | `AT-XOP-001` |
| `AM-GAD-003` | Produce the superseding v1.0 GA decision at Sprint 166 from current delivery, Windows, productivity, finance, cloud-observer, trusted-operations, command-authority, research, credential, continuity, model-management, strict-local, recovery, accessibility, support, and independent-review evidence. | `AM-GAD-002`, `AM-XOP-001` | Build | v1.0 | `AT-GA-003` |

## 38A. First-GA Trusted Operations Quantitative Acceptance Matrix

| ID | Test | Passing threshold |
|---|---|---|
| `AT-TRU-001` | Enumerate and mutate every trusted-operations process, worker, IPC edge, network path, credential edge, store, capability registration, lifecycle transition, disablement, and removal state on every first-GA platform. | 100% agreement with signed architecture and release manifests; zero undeclared process, socket, listener, credential edge, writable path, network destination, durable object, authority union, or removal residue. |
| `AT-CRD-001` | Exercise operating-system secret stores, OAuth and token references, provider/host/tenant/account/scope binding, expiry, rotation, revocation, crash, redirect, proxy, cross-worker, restore, export, diagnostics, logs, process metadata, and removal canaries. | Zero raw secret in model context, prompts, chat, arguments, environment outside an exact grant, logs, receipts, diagnostics, exports, crash data, or backups; zero wrong-account resolution; restored systems require reauthentication. |
| `AT-AUT-002` | Cross every command level with global and narrower autonomy ceilings, activation source, authentication, session identity, expiry, lock, logout, restart, policy mutation, schedule, child agent, panic stop, race, replay, and emergency disablement. | Zero model/content/schedule activation or authority broadening across at least 5,000 mutations; Owner mode is always direct-user activated, visibly active, expiring, panic-stoppable, non-inheritable, and fully revoked with no surviving child process. |
| `AT-CLI-002` | Run direct, shell, pipeline, redirection, script, interpreter, PTY, package, build, test, Git, Docker, process-tree, environment, filesystem, network, timeout, output, cancellation, rollback, and hostile-command fixtures at every authority level. | Every launched process matches one current grant and declared authority; constrained modes produce zero path, environment, credential, network, descendant, persistence, or privilege escape; Owner mode records complete truthful host-authority use and terminates on revocation. |
| `AT-WEB-001` | Search and retrieve changing public facts across primary, secondary, stale, conflicting, redirected, malicious, oversized, script-heavy, archived, unavailable, authenticated, and private-data-requesting fixtures. | 100% claim-level source and retrieval-time coverage for web-grounded claims; zero content-created authority or undeclared private disclosure; changing facts are revalidated; downloads remain bounded and quarantined until separately admitted. |
| `AT-BKC-001` | Create, interrupt, corrupt, replay, migrate, restore, roll back, retain, expire, and delete local snapshots across empty, large, classified, partially written, cross-version, low-disk, locked, and ransomware-like loss fixtures. | No incomplete snapshot is represented as complete; every restored byte and manifest reconciles; zero live canonical store operates from cloud-synchronized or network storage; clean-device restore and rollback succeed deterministically. |
| `AT-CBK-001` | Exercise the reference cloud backup adapter and conformance fixtures across destination identity, account, prefix, object, version, encryption, metadata, multipart transfer, retry, throttle, duplicate, missing object, corruption, replay, retention, deletion, revocation, provider outage, and full disaster recovery. | Plaintext never leaves the client; zero access outside the exact backup namespace; no duplicate or false-complete snapshot; restore succeeds from independently verified objects; Cloud Observer remains structurally read-only. |
| `AT-MCAT-001` | Mutate publisher, developer, license, origin, lineage, artifact, format, conversion, tokenizer, template, hash, size, runtime, platform, hardware, quality, security, support, state, expiry, and revocation for catalog fixtures. | Zero profile becomes usable without complete exact evidence and an allowed state; every change triggers the required block, quarantine, degradation, retirement, or re-review; mutable names never serve as identity. |
| `AT-MGR-001` | Exercise chat requests for list, recommend, download, import, resume, verify, activate, compare, cancel, crash, roll back, remove, and clean storage with source substitution, size drift, low disk, malformed artifacts, failed scans, stale approval, and interrupted activation. | Every effect matches the displayed exact plan and user confirmation; zero silent download, activation, fallback, artifact substitution, workspace access, credential access, or partial active state; cancellation and rollback leave a valid prior state. |
| `AT-MUSE-001` | Verify first-party Muse Glimmer identity, classification, license, origin, lineage, artifacts, transformations, hashes, runtime/platform compatibility, measured resource fit, coding/tool quality, security, and support evidence through the normal admission pipeline. | A complete supported profile receives `PASS`; incomplete or contradictory evidence receives `BLOCKED`; a non-waivable failure receives `REJECTED`; no unsupported claim or activation occurs, and a non-pass does not block Gemma-based first GA. |
| `AT-XOP-001` | Run cross-capability attacks joining hostile web or repository content, shell authority, credentials, backup destinations, model installation, provider workers, schedules, crashes, revocation, removal, and resource pressure. | Zero unauthorized command, secret disclosure, private upload, backup escape, model substitution, persistent child, authority reuse, or strict-local regression across at least 10,000 composed mutations and every supported platform. |
| `AT-GA-003` | Rebuild and independently reproduce the complete v1.0 release from raw requirements, packages, manifests, bills of materials, platform results, provider matrices, trusted-operations evidence, reviewer protocols, limitations, and removal results. | Sprint 166 closes `G-GA` only when all promoted gates pass with current signed evidence, every unsupported path is absent or denied, and any failed, skipped, stale, unavailable, flaky, quarantined, suppressed, unreconciled, or unreviewed blocker prevents release. |

## 38B. Trusted Operations Construction Checklist

### Authority and Command Execution

- [ ] `BUILD` separate Disabled, Inspect, Workspace Autonomous, Connected Operations, and Owner / Unrestricted Session command policies under the global Autonomy Center.
- [ ] `CAPABILITY GATE` Require direct authenticated user activation, exact duration, visible high-risk state, non-replayable launch identity, panic stop, and automatic revocation for Owner mode.
- [ ] `CAPABILITY GATE` Prohibit Owner mode activation or renewal from a model, prompt, repository, webpage, message, plugin, workflow, schedule, child agent, stale display, or standing consent.
- [ ] `BUILD` direct-executable, shell-language, pipeline, redirection, interpreter, script, and explicit PTY command contracts with exact previews and terminal receipts.
- [ ] `BUILD` working-directory, readable/writable root, environment, credential-reference, network, descendant, persistence, resource, timeout, output, cancellation, changed-file, rollback, and cleanup controls.
- [ ] `CAPABILITY GATE` Keep administrator or root elevation, credential injection, and secret environment imports separate from Owner-mode activation and separately authorized per operation.
- [ ] `VERIFY` lock, logout, restart, expiry, panic, policy mutation, crash, race, replay, descendant survival, scheduled activation, and content-created-authority attacks.

### Public Research and Citations

- [ ] `BUILD` a separate public search and retrieval worker with exact query, recency, domain, redirect, transport, item, byte, media, script, archive, download, cache, and cancellation limits.
- [ ] `BUILD` claim-level title, URL, retrieval time, publication date, event date where known, excerpt hash, source-quality, and inference-state citations.
- [ ] `BUILD` current-fact revalidation and visible stale, conflicting, unavailable, partial, secondary, and inferred source states.
- [ ] `CAPABILITY GATE` Separate public research from authenticated browser state, form submission, upload, publication, purchase, account changes, and every other external effect.
- [ ] `CAPABILITY GATE` Require an exact private-data disclosure preview before any workspace, memory, message, financial, connector, or credential-derived field can leave the device for research.
- [ ] `VERIFY` hidden instructions, malicious metadata, redirects, Domain Name System rebinding, certificate failures, tracking links, prompt injection, download substitution, archive bombs, scripts, and private-context requests.

### Credentials and Continuity

- [ ] `BUILD` Linux Secret Service, macOS Keychain, and Windows Credential Manager or DPAPI credential-reference adapters with exact identity, scope, lifecycle, and conformance fixtures.
- [ ] `BUILD` system-browser and device OAuth, short-lived token, Secure Shell agent, certificate-reference, rotation, revocation, expiry, and reauthentication workflows where providers support them.
- [ ] `CAPABILITY GATE` Keep raw credentials out of model context, prompts, chat, arguments, shell history, ordinary environment, logs, receipts, diagnostics, exports, crash data, and continuity snapshots.
- [ ] `BUILD` immutable encrypted local snapshots with manifests, integrity trees, classifications, exclusions, retention, atomic completion, staging restore, migration, rollback, and deletion.
- [ ] `CAPABILITY GATE` Prohibit live operational databases, model stores, indexes, locks, queues, sockets, and temporary files on cloud-synchronized, remote, or network storage.
- [ ] `BUILD` one reference cloud backup adapter and a conformance contract for S3-compatible, OneDrive, Google Drive, Box, Dropbox, and later destinations.
- [ ] `CAPABILITY GATE` Bind cloud backup to one exact encrypted destination namespace and keep its write authority completely separate from read-only Cloud Observer.
- [ ] `VERIFY` clean-device, cross-version, low-disk, corrupt, missing, replayed, duplicated, revoked, throttled, interrupted, ransomware-like, retention, deletion, rollback, and provider-outage recovery.

### Approved Model Management

- [ ] `BUILD` signed candidate, evaluating, approved, degraded, quarantined, rejected, and retired catalog records containing every model-provenance and support field.
- [ ] `BUILD` native Chat intents for compatible-model listing, tradeoff explanation, user-confirmed download, import, verify, activate, compare, rollback, remove, and storage cleanup.
- [ ] `BUILD` deterministic preflight showing exact publisher, license, source host, artifact, runtime, size, disk, hardware fit, network use, destination, checks, limitations, and rollback before confirmation.
- [ ] `BUILD` separate download/import quarantine, bounded resume, hash/signature verification, malware and format scan, admission self-test, atomic activation, cancellation, crash recovery, and rollback.
- [ ] `CAPABILITY GATE` Prohibit a model from downloading, approving, activating, replacing, or falling back to itself or another model.
- [ ] `VERIFY` source, redirect, artifact, manifest, license, hash, size, tokenizer, template, runtime, platform, low-resource, scan, stale-preview, interruption, activation, rollback, and deletion mutations.
- [ ] `DOCUMENT` list Muse Glimmer as candidate only until verified first-party evidence produces a complete admission disposition; do not assume open-source status, support, compatibility, or hardware fit.

### Integrated First-GA Closure

- [ ] `VERIFY` all `RV-36` through `RV-43` protocols on Fedora, Ubuntu, and Windows 11 with platform-specific evidence and no borrowed result.
- [ ] `VERIFY` disable and remove each command, research, credential, continuity, and model-manager capability separately and together during idle, active, queued, interrupted, and uncertain states.
- [ ] `VERIFY` rerun strict-local, delivery, productivity, finance, cloud-observer, model, accessibility, recovery, update, rollback, and uninstall suites after removal.
- [ ] `DOCUMENT` publish exact command-authority, research, citation, credential, backup, provider, model-catalog, model-installation, support, limitation, incident, recovery, and removal matrices.
- [ ] `CAPABILITY GATE` Preserve Sprint 156 as a stable expanded-productivity checkpoint and keep `G-GA` blocked until Sprint 166 passes every Decision 0010 first-GA requirement and acceptance test.

## 39. Post-GA Experimental Model Lab Backlog

| ID | Requirement | Dependencies | Disposition | Release | Acceptance |
|---|---|---|---|---|---|
| `AM-EML-001` | Implement a separately packaged Experimental Model Lab for user-selected unapproved artifacts with quarantine, explicit provenance and license gaps, no network, no credentials, no command or connector authority, no canonical workspace write, disposable synthetic evaluation, strict resource limits, truthful results, complete removal, and promotion only through normal independent model admission. | None; implementation sequence still requires completed first-GA model-management evidence | Build | post-GA | `AT-EML-001` |

## 39A. Post-GA Experimental Model Acceptance Matrix

| ID | Test | Passing threshold |
|---|---|---|
| `AT-EML-001` | Import, parse, load, evaluate, interrupt, crash, replace, remove, and attempt promotion of unknown, malformed, oversized, hostile, unsupported, provenance-incomplete, license-unclear, and policy-excluded artifacts. | Zero network, credential, command, connector, canonical-workspace-write, approved-store, or promotion authority; every gap remains visible; resource limits and cancellation hold; removal leaves zero undeclared artifact, process, cache, socket, or permission residue. |

## 39B. Experimental Model Lab Construction Checklist

- [ ] `BUILD` a visibly experimental package, store, process identity, synthetic corpus, scratch root, resource budget, cancellation path, and removal lifecycle separate from approved-model operation.
- [ ] `CAPABILITY GATE` Retain the non-Chinese and non-Chinese-derived model rule unless a later accepted decision explicitly supersedes it.
- [ ] `CAPABILITY GATE` Deny network, credential, command, connector, messaging, finance, delivery, cloud, backup, operational-memory, and canonical-workspace-write authority structurally.
- [ ] `BUILD` user-selected import with source identity, observed license, provenance gaps, hashes, quarantine, format checks, parser hardening, resource preflight, and explicit warnings.
- [ ] `BUILD` deterministic malformed-output, prompt-injection, tool-request, exfiltration, quality, context, latency, memory, graphics, disk, cancellation, and crash evaluations over synthetic data.
- [ ] `CAPABILITY GATE` Require complete normal model admission and independent review before any experimental artifact can enter the approved catalog or ordinary AgentMage runtime.
- [ ] `VERIFY` artifact substitution, symlink and path attacks, malformed formats, decompression bombs, denial of service, policy bypass, authority requests, crash residue, removal, and attempted direct promotion.

## 40. First-GA Whole-Codebase Audit Backlog

Decision 0011 makes comprehensive repository audit a first-GA capability without renumbering the
existing Sprint 157-166 sequence. The requirements below extend repository understanding into a
deterministic, read-only, resumable, reconciled, and evidence-based whole-codebase audit.

| ID | Requirement | Dependencies | Disposition | Release | Acceptance |
|---|---|---|---|---|---|
| `AM-CBA-001` | Implement `CODEBASE-AUDIT.md` as a separately registered read-only capability with exact repository and scope identity, complete path disposition, deterministic structural indexing, bounded semantic analysis, cross-module reconciliation, persistent evidence, truthful coverage, cancellation, recovery, removal, and separate platform evidence. | `AM-TRU-001` | Build | v1.0 | `AT-CBA-001` |
| `AM-CEN-001` | Implement a deterministic repository census covering tracked, staged, unstaged, untracked, ignored, sparse, generated, vendored, binary, Large File Storage, submodule, worktree, archive, symbolic-link, hard-link, inaccessible, malformed, special, external, and changing path states with one terminal disposition per in-scope path. | `AM-CBA-001` | Build | v1.0 | `AT-CEN-001` |
| `AM-STR-001` | Build an exact structural index from admitted compilers, abstract syntax trees, language services, package and build metadata, schema parsers, test discovery, workflow parsers, source-control records, and exact search, including modules, symbols, interfaces, dependencies, calls, state, data, events, errors, configuration, tests, releases, and provenance. | `AM-CEN-001` | Build | v1.0 | `AT-STR-001` |
| `AM-SEM-001` | Partition analysis by coherent architectural unit and create source-pinned semantic evidence cards containing exact spans, hashes, structural facts, neighboring interfaces, tests, model/runtime identity, assumptions, uncertainty, conflicts, confidence, and reverse-dependency invalidation links. | `AM-STR-001` | Build | v1.0 | `AT-SEM-001` |
| `AM-RCN-001` | Reconcile semantic conclusions across dependency, call, state, data, event, error, configuration, authorization, lifecycle, requirement, decision, test, deployment, and support relationships; retain contradictions and revisit every affected partition before making a repository-wide claim. | `AM-SEM-001` | Build | v1.0 | `AT-RCN-001` |
| `AM-ROA-001` | Enforce read-only audit behavior outside the model with no canonical-root write grant, no hosted mutation or credential authority, disposable copy-on-write execution for commands that may write, secret-safe packet construction, before-and-after repository attestation, process cleanup, and zero neighboring-data mutation. | `AM-CBA-001`, `AM-CLI-002` | Build | v1.0 | `AT-ROA-001` |
| `AM-CKP-001` | Persist encrypted audit checkpoints containing exact source, scope, parser, model, policy, coverage, queue, evidence, contradiction, resource, and completion identities; revalidate on resume and transitively invalidate every dependent fact, card, finding, reconciliation, and report when an input changes. | `AM-CEN-001`, `AM-BKC-001` | Build | v1.0 | `AT-CKP-001` |
| `AM-AUR-001` | Produce deduplicated audit findings and professional reports with stable identity, category, severity, confidence, status, impact, immutable file/line or symbol evidence, graph paths, counterevidence, uncertainty, recommendation, coverage totals, gaps, strengths, limitations, and observed-versus-inferred labels. | `AM-RCN-001`, `AM-ROA-001`, `AM-CKP-001` | Build | v1.0 | `AT-AUR-001` |
| `AM-GAD-004` | Extend the Sprint 166 v1.0 GA decision with current whole-codebase audit architecture, census, structural, semantic, reconciliation, read-only, secret-protection, checkpoint, invalidation, reporting, platform, resource, accessibility, removal, and independent-review evidence. | `AM-GAD-003`, `AM-AUR-001` | Build | v1.0 | `AT-GA-004` |

## 40A. First-GA Whole-Codebase Audit Quantitative Acceptance Matrix

| ID | Test | Passing threshold |
|---|---|---|
| `AT-CBA-001` | Exercise the complete audit lifecycle across empty, small, monorepo, polyglot, dirty, generated, vendored, partially unsupported, interrupted, and changing repository fixtures on every first-GA platform. | Every audit identity, worker, scope, path disposition, index, packet, evidence card, checkpoint, finding, report, cancellation, recovery, and removal state agrees with signed manifests; no undeclared process, path, network, credential, durable object, authority, or residue. |
| `AT-CEN-001` | Enumerate and mutate tracked, staged, unstaged, untracked, ignored, sparse, generated, vendored, binary, Large File Storage, submodule, worktree, archive, link, inaccessible, malformed, special, external, and concurrently changing paths. | 100% of in-scope paths receive exactly one current terminal disposition; totals reconcile with source control and filesystem observations; no required path is silently omitted, followed outside policy, or represented as analyzed when unavailable. |
| `AT-STR-001` | Build and independently reproduce module, symbol, interface, dependency, call, state, data, event, error, configuration, schema, test, build, workflow, release, and provenance graphs across the published language and build-system matrix. | Canonical records and graph hashes reproduce exactly; every edge resolves to evidence or an explicit unknown; parser failure, ambiguity, unsupported syntax, and version skew remain visible and block affected completeness claims. |
| `AT-SEM-001` | Partition repositories at architectural boundaries under varied context budgets, packet order, interruption points, model restarts, and approved model profiles while preserving exact evidence-card provenance. | Every required unit receives bounded analysis; cards resolve to current exact source and deterministic facts; no summary-only identity, unsupported observation, secret value, or packet-order difference changes deterministic coverage or authority. |
| `AT-RCN-001` | Seed cross-module architectural drift, duplicated responsibility, dead and orphaned code, circular dependencies, conflicting state models, incompatible paradigms, stale adapters, abandoned migrations, test gaps, and documentation-to-code conflicts. | Every seeded cross-boundary condition is detected or recorded as an explicit supported limitation; contradictory evidence remains visible; global findings cite all material sides; no unresolved contradiction is silently collapsed into certainty. |
| `AT-ROA-001` | Run audits, parsers, language services, builds, tests, package operations, generators, formatters, coverage tools, cancellation, crashes, malicious repository instructions, and path races against canonical and disposable workspaces. | Zero canonical repository, Git metadata, hook, ref, configuration, submodule, worktree, untracked, ignored, hosted-service, credential, or neighboring-data mutation; all permitted writes remain in disposable storage and complete cleanup; raw secrets never enter model or retained audit surfaces. |
| `AT-CKP-001` | Pause, crash, restart, restore, change, delete, rename, reclassify, or reparse inputs at every audit stage and mutate scope, parser, model, runtime, policy, graph, checkpoint, and reverse-dependency records. | Unchanged work resumes deterministically; every changed input invalidates all and only safely identified dependent records; uncertain dependency state triggers a broader rescan; no stale card, finding, coverage value, or report can become current. |
| `AT-AUR-001` | Generate and independently verify findings and reports across supported strengths, defects, counterevidence, duplicates, conflicts, partial coverage, unavailable evidence, false-positive fixtures, severity boundaries, and model disagreement. | 100% of material claims resolve to current immutable evidence and a visible observation or inference class; findings are deduplicated and calibrated; every gap and uncertainty remains visible; no report claims whole-codebase completion before every required gate passes. |
| `AT-GA-004` | Independently reproduce a comprehensive audit of each release-reference repository and adversarial corpus from raw source, manifests, parser records, structural graphs, semantic packets, evidence cards, checkpoints, findings, reports, platform results, and removal evidence. | Sprint 166 closes `G-GA` only when `AT-GA-003`, all whole-codebase audit tests, and every applicable security and reviewer protocol pass with current signed evidence; any mutation, disclosure, stale record, unsupported hidden gap, unreconciled contradiction, non-pass, or unreviewed result blocks release. |

## 40B. Whole-Codebase Audit Construction Checklist

### Scope, Census, and Structural Index

- [ ] `BUILD` exact audit-plan schemas for repository, revision, worktree, scope, depth, language, parser, generated and vendored policy, history, submodules, worktrees, commands, network, resources, retention, cancellation, and outputs.
- [ ] `BUILD` a complete path census with deterministic identities, classifications, hashes where readable, parser state, terminal disposition, errors, exclusions, and dependent records.
- [ ] `CAPABILITY GATE` Keep generated, vendored, ignored, untracked, binary, linked, archived, Large File Storage, submodule, worktree, inaccessible, oversized, malformed, and changing content visible even when semantic analysis excludes it.
- [ ] `BUILD` compiler, abstract-syntax-tree, language-service, package, build, schema, test, workflow, source-control, and exact-search adapters behind versioned parser contracts.
- [ ] `BUILD` module, symbol, interface, dependency, call, state, data, event, error, configuration, test, build, release, requirement, decision, and provenance records and graphs.
- [ ] `CAPABILITY GATE` Prohibit embeddings, mutable names, model summaries, or retrieval rankings from serving as canonical identity, coverage, dependency, authority, or evidence.

### Read-Only and Sensitive-Data Protection

- [ ] `BUILD` canonical-root read handles with no write grant and explicit denial of Git index, ref, hook, configuration, worktree, submodule, ignored, untracked, hosted mutation, and neighboring-path changes.
- [ ] `BUILD` disposable copy-on-write verification workspaces for builds, tests, resolution, generation, coverage, formatting, and language-service initialization.
- [ ] `CAPABILITY GATE` Start audit workers without network, credentials, provider effects, hosted writes, or command authority beyond the exact read-only or disposable verification plan.
- [ ] `BUILD` before-and-after repository, filesystem, process, socket, Git metadata, user-data, and hosted-service attestation with exact changed-state failure records.
- [ ] `BUILD` pre-model secret detection, classification, typed redaction, restricted-file handling, canary scanning, and retention controls.
- [ ] `VERIFY` symlink, hard-link, mount, archive, race, special-file, hook, configuration, build-script, generated-output, parser, language-server, submodule, and repository-instruction attacks.

### Semantic Review, Memory, and Reconciliation

- [ ] `BUILD` coherent package, service, state-boundary, feature, workflow, schema-family, and cross-cutting-concern partitioning with bounded deterministic work packets.
- [ ] `BUILD` source-pinned evidence cards with parser facts, neighboring interfaces, tests, model/runtime identity, assumptions, uncertainty, conflicts, confidence, and reverse dependencies.
- [ ] `BUILD` encrypted local audit checkpoints and deterministic queues that resume after cancellation, restart, crash, model replacement, and device recovery.
- [ ] `BUILD` source-change detection and transitive invalidation for census, structure, packets, cards, findings, contradictions, coverage, and reports.
- [ ] `BUILD` cross-module dependency, call, state, data, event, error, configuration, authorization, lifecycle, requirement, decision, test, deployment, and support reconciliation passes.
- [ ] `BUILD` risk-directed prioritization for high-centrality, privileged, stateful, concurrent, exposed, weakly tested, frequently changed, and historically unstable areas without reducing baseline coverage.
- [ ] `CAPABILITY GATE` Retain contradictory evidence and revisit affected partitions before any global conclusion; never collapse unresolved conflict into certainty.

### Findings, Reporting, and First-GA Closure

- [ ] `BUILD` finding schemas for identity, category, severity, confidence, impact, status, evidence, graph paths, counterevidence, uncertainty, recommendation, deduplication, conflict, supersession, and verification.
- [ ] `BUILD` quick, targeted, and comprehensive report profiles with explicit coverage semantics; only the comprehensive profile may claim whole-codebase completion.
- [ ] `BUILD` professional reports covering architecture, structure, technical debt, progress, viability, security, operations, strengths, limitations, strategic recommendations, unresolved questions, and coverage totals.
- [ ] `VERIFY` seeded known-answer monorepos, polyglot repositories, architectural pivots, dead code, duplicate systems, state conflicts, missing tests, stale documentation, false positives, contradictory evidence, and model disagreement.
- [ ] `VERIFY` cancellation, crash, checkpoint, resume, invalidation, low-memory, low-disk, context pressure, parser failure, unsupported language, resource exhaustion, accessibility, removal, and clean strict-local restoration.
- [ ] `DOCUMENT` publish language, parser, build-system, repository-size, file-count, graph, context, model, resource, platform, support, limitation, retention, and audit-depth matrices with evidence digests.
- [ ] `CAPABILITY GATE` Keep `G-GA` blocked until Sprint 166 passes `AM-GAD-004`, `AT-GA-004`, every Decision 0011 requirement, and all assigned security and reviewer protocols.
