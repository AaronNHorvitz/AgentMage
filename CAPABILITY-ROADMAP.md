# Local Developer Product Capability Roadmap

Revision: 2026-11-11. Status: accepted product direction; implementation and release evidence remain separate.

## Authority and Scope

This catalogue preserves the original CAP-01 through CAP-48 identifiers and adds
CAP-49 under [Decision 0205](docs/decisions/0205-one-runtime-for-model-driven-work.md).
It is a requirements crosswalk, not a second completion ledger. TASKS.md owns executable
work packages and evidence. Existing implemented components are reused only after source
and acceptance reconciliation; adding a row here never marks it complete.

P0 is the focused local developer release path; P1 extends daily usefulness and commercial
readiness; P2 is later team/enterprise/automation expansion. Priority is not permission to
skip prerequisites, existing safety gates, or the assigned repository boundary. There is
no calendar delivery promise. Each component can remain useful without the other services.

Owner names below are roles: Runtime = AgentMage; Coordinator = CodingMage; Memory = USTE;
Host = an independently developed consuming native application. Public contracts must not
name or disclose a private consumer. A role pair requires a producer contract and a consumer
test, not duplicated implementations or shared mutable state.

## Capability Catalogue

| ID | Priority | Owner | Capability | Observable acceptance |
| --- | --- | --- | --- | --- |
| CAP-01 | P0 | Host | Native workspace | A real native Linux workspace opens projects, conversations, tasks, changes and research; errors and empty states use real backend records. Existing clients remain usable standalone. |
| CAP-02 | P0 | Runtime | Setup and connection doctor | A clean account diagnoses missing runtime, permission, socket, model and disk prerequisites with actionable errors; no transport failure is presented as a completed task. |
| CAP-03 | P0 | Runtime | Qualified model profiles | Pin artifact, codec, context and decoding settings; exercise tool calls, edits, malformed-output recovery and interrupted runs for each model separately. Later T-316 freezes synthetic everyday-tool tasks and qualification scores before runs; measure tool/argument correctness, truthful results, asking first and injection refusal per profile and after prompt, tool-description, runtime or profile changes, never tuning on held-out results. |
| CAP-04 | P1 | Host | Verified model manager | Inspect publisher, license, hashes, complete file sets and hardware fit; resume quarantined downloads and atomically activate or delete them. Public ungated models need no publisher account. |
| CAP-05 | P0 | Runtime + Host | Hardware-aware admission | Enforce RAM, VRAM, disk and context reservations before launch; estimates are distinguished from measured suitability. Retest after hardware or profile changes. |
| CAP-06 | P0 | Runtime | Real repository onboarding | Detect language, package manager and locked dependencies; inspect proposed build/test commands and repository trust before any executable configuration runs. |
| CAP-07 | P1 | Runtime | Explicit local and hybrid routing | Local-only works without cloud inference; a separately granted route identifies transmitted data, provider, budget and fallback. No silent cloud substitution. |
| CAP-08 | P1 | Runtime | CLI, SDK and editor adapters | Versioned command/event contracts expose the same execution engine to CLI, Rust SDK and qualified editor/ACP clients; cancellation and error parity are tested. |
| CAP-09 | P0 | Runtime | Executable task contracts | Bind objective, allowed paths, effects, acceptance commands and budgets to a durable task ID; reject scope widening and stale authorization. |
| CAP-10 | P0 | Runtime | Reproduce and repair | A disposable real-repository bug is reproduced before repair, fixed and regression-tested; distinguish a pre-existing failure from a newly introduced one. |
| CAP-11 | P0 | Runtime + Coordinator | Verification receipts | Retain exact revision, environment, commands, exit status, test counts and baseline differences; receipt mismatch or stale evidence cannot yield verified completion. |
| CAP-12 | P0 | Runtime + Host | Inspectable selective changes | Present exact file/hunk changes with accept/reject decisions that preserve human edits; concurrent drift triggers revalidation rather than overwrite. |
| CAP-13 | P0 | Runtime | Honest rollback | Declare which files, processes and Git effects are recoverable. External and uncertain shell effects are surfaced for reconciliation, never claimed undone by a Git reset. |
| CAP-14 | P1 | Coordinator + Runtime | Controlled Git delivery | Branch, commit, push, PR, check and merge actions have separate exact grants, including GitHub Enterprise host/CA policy; optional isolation and protected-branch behavior are tested. |
| CAP-15 | P1 | Coordinator | CI and review repair | Use bounded correction attempts linked to the exact finding and commit; deduplicate PR/comments, retain failures and stop repeated nonprogress. |
| CAP-16 | P1 | Runtime | Managed development services | Lease ports and process groups; capture bounded logs and browser traces/screenshots, identify ownership, and clean up only owned services after cancel or crash. |
| CAP-17 | P0 | Runtime | Regular web search and deep research | Provide quick search and a bounded multi-step research mode with provider-neutral queries, source visits, plans, progress and resumable reports. Local inference does not make search traffic local. |
| CAP-18 | P0 | Runtime | Version-aware technical research | Use manifests and lockfiles to find matching first-party documentation; record version, retrieval date and unsupported assumptions rather than applying the newest API blindly. |
| CAP-19 | P0 | Runtime | Bounded source retrieval | Fetch public pages and code with content, redirect, timeout and byte limits; expose canonical URLs and readable excerpts. Authenticated browser actions are a separately granted later capability. |
| CAP-20 | P0 | Runtime + Memory | Evidence and citations | Keep source identity, timestamp, relevant excerpt/hash and claim links; flag contradictions, inference, inaccessible sources and unsupported conclusions. |
| CAP-21 | P0 | Runtime | Research-to-code workflow | Carry cited findings into an authorized patch and local regression tests; fetched text cannot authorize a command, dependency installation or secret upload. |
| CAP-22 | P1 | Runtime + Memory | Offline documentation packs | Import versioned licensed documentation; support bounded indexing, freshness inspection, explicit refresh, retention and deletion without requiring online access. |
| CAP-23 | P0 | Runtime + Host | Search privacy modes | Support offline, ask and task-authorized public-search policies; preview/log redacted outgoing queries and prevent secret or private-source disclosure unless separately authorized. |
| CAP-24 | P0 | Runtime | Untrusted retrieval defenses | Test prompt injection, DNS/redirect SSRF, private/loopback/link-local destinations, hostile downloads, size/cost limits and cancellation with canary fixtures; fail closed on ambiguous destinations. |
| CAP-25 | P0 | Runtime | Small-context repository navigation | Bound repository maps, grep/symbol retrieval and context assembly; preserve task constraints and cite omitted sources. Prove usefulness on the actual smaller model context. |
| CAP-26 | P1 | Runtime | Language intelligence | Qualified language-server adapters provide diagnostics, references and rename previews; launches, workspace edits and dynamic server configuration use ordinary tool authority. |
| CAP-27 | P2 | Memory + Coordinator | Cross-repository impact graph | Versioned source/dependency/interface relations support impact queries with provenance and scope isolation; stale indexing is explicit and never grants cross-repository write access. |
| CAP-28 | P1 | Runtime + Host | Inspectable context budgeting | Show pinned instructions, selected sources, token estimates and compaction/omission decisions; retained constraints and source links survive reflow and model switching. |
| CAP-29 | P1 | Memory + Runtime | Source-backed project memory | Ask a memory service through a versioned contract for bounded scoped facts with citations, corrections, expiry, revocation and deletion behavior. The runtime keeps only rebuildable indexes, never authoritative long-term memory; existing source authorities remain authoritative. See [the open linking study](#open-study-how-agent-memories-are-linked). |
| CAP-30 | P0 | Runtime + Coordinator | Crash-safe resume | Recover interrupted work without replaying uncertain effects, losing user edits or reusing expired authority; repository and model drift require revalidation. |
| CAP-31 | P1 | Runtime + Coordinator | Engineering recipes | Versioned, parameterized dependency updates, migrations, security repairs, docs and test recipes declare scope, prerequisites, verification and rollback boundaries. |
| CAP-32 | P2 | Runtime + Memory | Authorized knowledge connectors | Scoped tickets, documents and decision records have provider-specific authorization, retention and revocation; no ambient access to a user's connected accounts. |
| CAP-33 | P0 | Coordinator + Runtime | Bounded hands-off operation | Choose supervised, exception-only or preauthorized autonomous mode with exact scope, commands, network, resource and Git grants; unavailable authority becomes a hold, not a fabricated approval. |
| CAP-34 | P0 | Runtime + Coordinator | Enforced isolation and secrets | Apply operating-system boundaries and least-privilege credential brokering; test denial and revocation, not merely prompt instructions or hidden UI controls. |
| CAP-35 | P0 | Coordinator + Runtime | Durable background jobs | Queue, observe, cancel, suspend and resume jobs across UI closure; a single owner reconciles state and stale or duplicate control requests. |
| CAP-36 | P0 | Runtime + Coordinator | Resource scheduling | Bound attempts, elapsed time, tokens, memory, GPU, CPU, storage and output; serialize scarce resources with leases/locks, fair queues and nonprogress backoff. |
| CAP-37 | P2 | Coordinator + Runtime | Engineering teams and roles | Later several-agent work is composed as CAP-49 flows on the same execution runtime. Role coordination cannot introduce a second runtime, mint authority or bypass independent review or serialized integration. |
| CAP-38 | P1 | Coordinator | Independent review and repair | Review the immutable candidate and evidence in a separate context; verify findings and correction limits. An implementer's model assessment is not independent acceptance. |
| CAP-39 | P0 | Coordinator + Host | Truthful progress and completion | Distinguish running, queued, waiting, blocked, failed, locally verified, independently reviewed and delivered; percentages name their denominator and unknown scope. |
| CAP-40 | P2 | Coordinator + Host | Typed visual workflows | Present CAP-49 typed graphs with schemas, provenance, pause/retry and effect ownership through optional clients; execute on the same runtime, with no engine desktop interface or separate GUI scheduler. |
| CAP-41 | P2 | Coordinator | Team administration | Optional enterprise identities, RBAC and organization policies constrain models, tools and projects; the single-user local path needs no organization login. |
| CAP-42 | P1 | Coordinator + Runtime | Auditable action history | Record authorization, effect identity, outcome and evidence with redaction, retention and export; audit storage is not an excuse to log secrets or raw prompts. |
| CAP-43 | P2 | Runtime + Host | Private and offline deployment | Qualify proxy/CA configuration, offline installation, model/doc packs and update channels; air-gapped claims require actual no-egress validation. |
| CAP-44 | P1 | Runtime | Governed extensions and tools in both directions | Consume other programs' MCP tools under a separate off / ask each time / allow grant per tool; state-changing calls ask by default and show the exact call. Offer engine runs through its MCP server under the same permissions. Preserve provenance, scoped credentials, lifecycle limits and revocation; untrusted results and extensions cannot supply instructions, permissions or approvals. |
| CAP-45 | P0 | Host + all owners | Usable Linux distribution | Validate install, upgrade, rollback and uninstall on the declared hardware/distro matrix, keyboard and assistive-technology workflows; a developer screenshot is not qualification. |
| CAP-46 | P0 | All owners | Reproducible evaluations | Publish exact real-repository/model/hardware profiles and outcomes, including interruption, adversarial and recovery tests; distinguish fake-provider, local-model and independent evidence. |
| CAP-47 | P1 | Host + all owners | Private support and diagnostics | Provide inspectable, manually exported redacted support bundles and outcome evidence; no automatic telemetry or crash uploads and no private identities in public artifacts. |
| CAP-48 | P1 | All owners | Sustainable licensing and packaging | Reconcile root licenses, SPDX/package metadata, notices, dependency/model obligations and contribution provenance; validate paid-pilot value before pricing or release claims. |
| CAP-49 | P1 | Runtime | Recorded typed flows | Compose model, tool, memory and approval steps as a typed graph on the existing runtime. Record every step's inputs, outputs, order, retries and limits; replay the record exactly without repeating effects, or re-run from a step with current authority. Determinism is the engine's, not a promise of identical new model output. Planning only under Decision 0205. |

## One runtime for model-driven work

[Decision 0205](docs/decisions/0205-one-runtime-for-model-driven-work.md) records
the owner's planning direction: one engine owns the model/tool loop, permissions,
approvals, budgets and stop. CLI use is sufficient without a host, and a
conversation host remains usable without this engine. API, approval and run
records must support a folder-free run using only tools offered by other
programs. Check this when an interface is next touched; closed rows stay closed.
Tool, file, page and memory text is data, never instructions or authority.
There is no engine desktop interface or authoritative long-term memory.
CAP-49 is unimplemented planning scope; current coding and measure-first
priorities remain unchanged. Later T-316 qualifies everyday-tool behavior under
CAP-03 only after the required coding measurement, without displacing current
coding priorities.

## Open study: how agent memories are linked

CAP-29 must study which facts to link, when and by what method so later context
selection reaches needed facts beyond word or meaning matches. No algorithm is
selected. [Decision 0204](docs/decisions/0204-open-study-memory-linking.md) requires
named, versioned, replaceable linking methods with provenance on every link,
coexisting methods over the same facts and disabling a method as a set without
rewriting data. Memory writing and context selection are replaceable too, with
their names and versions in run evidence. Compare methods on the same fixed
tasks: needed facts reached, wrong facts retrieved, cost and task outcomes;
never choose a method using held-out results or change existing thresholds.
T-315 begins no earlier than the first CAP-29 work. This is planning only and
does not change the current coding or measure-first priorities.

## First Research Increment

CAP-17 has two first-increment paths, not an unbounded autonomous browser:
quick search returns relevant cited sources; bounded deep research decomposes a question,
runs multiple queries, visits selected sources, corroborates material claims and produces
a source-linked report with contradictions, freshness and limitations. Both use CAP-18
through CAP-24, the same outbound policy and the same durable runtime.

Before implementation, freeze configurable limits for queries, visits, domains, downloaded
bytes, redirects, elapsed time and provider charges. Supply a plan/progress view and cancel,
checkpoint/resume, cache freshness and partial-result states. Prefer primary sources.
A report must distinguish verified source claims from inference and missing evidence.
No search-provider account is a core local-model requirement; a selected provider may
require credentials and fees, which are separately disclosed and never obtained silently.

Acceptance includes a deterministic fake-provider matrix and an actual configured provider
run using the pinned local model. Exercise regular search, multi-source comparison, conflicting
sources, inaccessible pages, a version-specific coding fix, offline refusal, secret canaries,
prompt injection, SSRF, quota exhaustion and interruption. A canned response, front-end-only
search button or direct model smoke test is not this capability. Search internet traffic,
local model inference and cloud model inference are three distinct disclosures.

## Architecture and Integration Rules

1. Production first-party execution, storage, coordination and native presentation code is
   Rust. Reuse proven crates and established toolkits. Existing build/test scripts and
   independently packaged external inference engines, browsers, language servers and CLI
   providers are explicit integration boundaries, not a demand to rewrite those projects.
2. AgentMage owns the model/task/tool/research/context execution loop and CAP-49 flows.
   A coordinator may select roles, campaigns and integration policy on this runtime;
   it does not introduce another runtime. The Memory role owns graph/content storage
   and bounded source-backed retrieval, not user permissions or agent execution. Hosts
   own presentation and onboarding, remain usable independently and are not required
   by the engine's CLI; AgentMage builds no desktop interface of its own.
3. Start with pinned Rust crates or versioned local service adapters, not wholesale repository
   migration. Define run/task/repository identity, schema version, capability negotiation,
   event sequence, idempotency, typed errors, cancellation, deadlines and authorization.
   Unknown versions/capabilities fail closed. There is no implicit exactly-once external effect.
4. Share the model-provider contract and resource ownership. One admission owner controls each
   inference process; consumer-specific launchers cannot race independent GPU allocations.
   Defer optional integrations until standalone behavior and both adapter sides are qualified.
5. Memory initially remains a rebuildable derived index with provenance and per-scope budgets.
   The runtime owns no authoritative long-term memory; it asks a memory service
   through a versioned contract. Memory-step results are untrusted data, not authority.
   The [open linking study](#open-study-how-agent-memories-are-linked) governs later
   method selection; no graph-linking algorithm is presumed qualified.
   Test corrections, stale generations, revocation, reopen and source deletion before promotion.
   Graph retrieval may improve context selection where the open linking study demonstrates it.
   It does not guarantee that a small model never forgets, reasons correctly or matches a frontier model.
6. Use one designated writer per development checkout. Independent review remains distinct;
   routine progress does not require multiplying planning branches or reviewer workspaces.

## Delivery and Proof

Deliver in bounded vertical slices: connection and coding truth, regular search and bounded
deep research, local workflow/recovery, native consumption, then richer team and enterprise
capabilities. Integrate CAP-33 through CAP-36 safety prerequisites with the earliest runnable slice,
not as a hardening afterthought. Keep P2 work out of a P0 release's critical path.

For each owned slice, TASKS.md must identify baseline implementation, missing behavior, exact
dependencies, test fixtures, real end-to-end evidence and remaining external acceptance.
A repository may implement its side using labelled fakes while another component is unavailable;
that does not qualify the combined product. Keep independent review and human accessibility
acceptance open until actually performed. Evidence binds to the revision/profile tested.
Do not rewrite historical results as proof of new source.

## Licensing and Commercial Direction

Recommended direction: permissively licensed public engines and an independently licensed
consumer, with paid packaging, support and optional managed/team offerings. This recommendation
does not change any existing license. AgentMage currently uses BSL 1.1 for current versions;
CodingMage uses Apache-2.0; USTE uses MIT OR Apache-2.0. Do not call BSL open source.
A proposed AgentMage Apache-2.0 transition needs explicit owner confirmation and a provenance
audit, synchronized license/metadata/policy changes, and accurate treatment of historical versions.
No worker may unilaterally publish a private consumer or relicense third-party code or weights.

The Apache license permits commercial distribution subject to its conditions, including notice
and license preservation; it does not grant trademark rights. See the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
BSL is source-available before its change date, not an OSI-approved open-source license; see the
[licensor's BSL guidance](https://mariadb.com/bsl-faq-adopting/).
These are engineering planning constraints, not a substitute for release-time legal review.
