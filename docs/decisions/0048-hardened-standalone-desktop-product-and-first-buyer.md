# Decision 0048: Hardened Standalone Desktop Product and First Buyer

| Field | Value |
|---|---|
| Status | Accepted product-scope decision |
| Date | 2026-09-06 |
| Scope | Defines the first buyer; makes a signed standalone desktop application the first supported release surface (Windows 11 x64 first, Apple Silicon macOS second, Fedora and Ubuntu retained); defines the first public release as the Epic 1 read-only local evidence workflow delivered through that application with one qualified local model; establishes a first-release critical path across the full plan opened by Decision 0047; adds the missing product stories; narrows the Decision 0045 first-release mandatory set; records the commercial model |
| Supersedes | The "First interface target" rows of `README.md` and `PRD.md`; the clause of Decision 0045 items 1 through 3 that makes the Visual Studio Code tab the canonical surface of the first supported release; the ordering effect of Decision 0021 item 1 across the open plan (Decision 0021 itself is preserved) |
| Preserves | Every epic, sprint, story, task, and identifier; additions-only history; completed evidence; evidence binding; Decision 0046 and Decision 0047 in full, including the completed freeze, the full-plan continuation, and the blocked-row rules; the Decision 0040 execution venues and physical-hardware blockers; Decision 0045 items 4 (as narrowed below), 8, and 9; Decisions 0027 and 0044 model qualification; the Apache License 2.0 on the repository |
| Does not authorize | Deletion or renumbering; substitution of platform evidence; enabling any model without Decision 0044 qualification; any network egress, telemetry, or diagnostic upload without explicit per-event user consent; a cloud dependency of any kind in the first release; relicensing of the repository; weakening of evidence binding; release publication without the release gate in item 9 |

## Context

Measured on 2026-09-02. `README.md` names the first interface target as
"AgentMage Verified Chat in Visual Studio Code". `PRD.md` line 245 and
`IMPLEMENTATION-PLAN.md` line 237 make native Visual Studio Code Chat the sole
v0.1 interface. Across the 5,148 rows of `TASKS.md`, "installer" appears in 17
rows, "notariz" in 12, "standalone" in 5, "onboarding" in 1, and "system tray",
"auto-update", "model manager", and "model download" in none. "Non-technical" or
"end user" appears in 6. The product the plan builds is a developer tool with a
hardened core.

The same day, OpenClaw released version 2026.8.1 with hardware-detected local
model recommendation, managed `llama.cpp` setup, subscription and API-key
detection, a browser-first interface, and shared cloud sessions, built by 933
contributors across more than 16,000 pull requests on a project with roughly
247,000 GitHub stars. Its maintainers state publicly that a user who cannot
operate a command line cannot use it safely; an independent security team found
a third-party skill performing data exfiltration and prompt injection without
user awareness. The unoccupied position is therefore not easier local models. It
is an agent a non-technical person can install and trust because it is
structurally unable to act without evidence, hold a credential beyond one
operation, write without approval, or send data off the machine.

AgentMage's existing design already answers that position: deterministic effect
gating, the tool-observation boundary, the deterministic verifier, the event
journal, operation-scoped credentials, the strict-local Model Gateway profile,
and the disposable secured webview. What it lacks is a surface a non-developer
can reach and a buyer definition. Epic 1 already contains the workflow such a
buyer needs (the read-only local evidence assistant, Sprints 4 through 25), the
model installer and importer (Sprint 14), macOS packaging (Sprint 8), release
manifests (Sprint 7), and a cross-platform release sprint (Sprint 25). Epic 8
Sprints 76 and 77 already carry desktop application, status, recovery, and
packaging work. Windows packaging, process, IPC, update, rollback, and uninstall
rows sit in Epic 10. The product exists in the plan as parts; this decision
orders them and adds the connective rows.

Both model profiles in `architecture/status-model.json` carry disposition
`rejected`. A first-run flow that offers "one vetted model" therefore depends on
Decision 0044 qualification producing at least one approved catalog entry. That
dependency is part of the critical path, not a UI detail.

As of 2026-09-06 the Decision 0046 freeze is complete and Decision 0047 opened
the full preserved plan; its terminal audit bound all 1,484 remaining rows to
exact external blockers with empty substitution sets. This decision therefore
orders the open plan rather than a frozen subset, and changes no blocked
disposition.

## Decision

1. **First buyer.** The first buyer is a non-technical professional who holds
   information under a duty of confidentiality or a compliance regime and works
   alone or in a firm of one to twenty-five people: solo and small-firm lawyers,
   accountants and tax preparers, clinicians and small practices, and
   independent financial advisers. Every first-release product decision is
   resolved in favor of that person's ability to install, trust, and use the
   product without documentation, a terminal, or an administrator.
2. **First release surface.** The first supported release surface is a signed
   standalone desktop application. Windows 11 x64 is the first release
   platform. Apple Silicon macOS is the second, built as far as source and
   contract tests carry it on the development workstation and released when
   the Decision 0040 item 5 hardware and signing identity exist; its
   `release_lane` value is unchanged by this decision. Fedora and Ubuntu remain
   supported platforms under Decision 0040 and receive the same application.
   The application hosts the Verified Chat surface in the Decision 0045
   disposable secured webview and requires no Visual Studio Code installation.
3. **Visual Studio Code becomes a secondary surface.** The Verified Chat tab,
   `@agentmage` participant, and Language Model Chat Provider paths are
   retained in full and remain the development harness for internal milestones
   v0.1 through v0.7. They are not required for the first public release.
   Decision 0045 items 1 through 3 apply to the Visual Studio Code surface
   whenever it ships and no longer define the first supported release.
4. **First public release definition — "v1.0-preview: Hardened Local Evidence
   Assistant".** A fresh Windows 11 x64 machine with no developer tooling; one
   signed installer; first run detects hardware, lets the user choose a folder,
   and offers exactly the models in the approved catalog that fit the hardware,
   downloading and hash-verifying the chosen one or using an already-present
   local `llama-server` through read-only discovery; the user asks questions of
   the folder and receives cited answers with evidence cards; the product
   performs no write, no network egress, and no credential use; uninstall
   leaves no residue. That is the Epic 1 workflow through the item 2 surface.
   Epic 3 controlled writes, Epic 2 knowledge integration, connectors, Team
   mode, and remote routes are not part of the first release.
5. **New stories (additions only, numbered by the implementing batch;
   proposed numbers shown).**
   - Story 76.2 — Standalone Application Shell and First Run: application
     process model reusing the Rust host as the sole authority; installer
     integration with Sprint 7 release manifests; first-run hardware detection;
     folder selection with local-only path checks; zero-configuration
     `strict_local` start; accessibility from the first increment.
   - Story 76.3 — Model Manager: guided selection from the Decision 0044
     approved catalog by hardware fit; hash-pinned download with resume and
     verification; import of an existing local runtime via read-only
     discovery; removal and rollback; built over Sprint 14 and aligned with
     Sprint 164 rather than duplicating them.
   - Story 77.2 — Background Presence, Signed Update, and Consented
     Diagnostics: tray or menu-bar presence; signed update manifests with
     rollback; crash and diagnostic reports assembled locally, redacted
     locally, and sent only on explicit per-report consent; offline by default.
   - Story 25.2 — v1.0-preview Release Gate: the item 4 definition as an
     executable acceptance procedure on a clean Windows 11 guest through the
     Decision 0040 KVM lane, with the physical-only rows of
     `WINDOWS-BOUNDARIES.md` remaining blocked until a physical machine exists.
   Each story carries the Universal Story Definition of Done unchanged.
   **Correction (Decision 0051):** Story 25.2 was already assigned to Incident
   Tabletop and Release Support Readiness. The preview gate is Story 25.3; the
   proposed 25.2 identity above is retained only as historical decision text.
6. **First-release critical path.** Across the full plan opened by Decision 0047, the
   Decision 0021 "first authoritative incomplete dependency gate" rule resolves
   in this order: (a) remaining Epic 0 gates; (b) Foundational Runtime Epics F1,
   F3, and F4 to the extent the item 4 workflow requires, with F4 limited to
   the `strict_local` profile; (c) Epic 1, Sprints 4 through 25, including
   Sprint 14; (d) Stories 76.2, 76.3, and 77.2; (e) the Epic 10 Windows
   package, process, path, IPC, key, model, tool-worker, network-worker,
   accessibility, clean-install, update, rollback, and uninstall rows through
   the Decision 0040 KVM lane; (f) Story 25.2; (g) the remainder of the frozen
   plan in numbered order. Rows outside the critical path are not executed
   while an unblocked critical-path row exists. Rows already bound to exact
   blockers by the Decision 0047 terminal audit remain blocked and are not
   reopened by this decision.
   **Correction (Decision 0051):** item (f) is Story 25.3 and item (g) is the
   remainder of the complete preserved plan in numbered order. The historical
   universal external-blockage assertion is not row-level proof; genuine exact
   blockers remain binding while local, dependency, external, and unknown rows
   are classified separately.
7. **Decision 0045 first-release mandatory set, narrowed.** For the first
   public release, item 4 requires the Rust host, durable supervisor, artifact
   service, context service, workflow engine, tool-observation boundary,
   deterministic verifier, event journal, capability registry, and the
   `strict_local` Model Gateway. The Team coordinator (items 6 and 7), the
   qualified `remote_private` and `remote_managed` routes (item 5), and the
   Visual Studio Code surfaces (items 1 through 3) remain mandatory for the
   first supported Engineering Runtime release and are not required for
   v1.0-preview. Items 8 and 9 apply unchanged.
8. **Commercial model.** The repository remains Apache License 2.0. The
   product sold is the signed, notarized, supported, auto-updating build with
   its update service and support terms, under a product license and a
   trademark policy to be recorded in a later decision. Nothing in the first
   release depends on a cloud service. Any later cloud connection is a separate
   decision and remains optional at runtime.
9. **Release gate.** No artifact is published as v1.0-preview until Story 25.2
   passes on a clean Windows 11 guest, at least one model carries an approved
   disposition under Decision 0044, and the independent review required by
   Decision 0021 for signed release has occurred.
   **Correction (Decision 0051):** the release gate identity is Story 25.3.
10. **Markers and registration.** `README.md`, `PRD.md`, and
    `IMPLEMENTATION-PLAN.md` state the item 1 buyer and the item 2 surface in
    their product tables; every `documentation_contract` document carries the
    marker `First-release surface: standalone hardened desktop application
    (Decision 0048).`; `architecture/status-model.json` registers `ADR-0048`
    as an amendment and its `required_markers` includes the new marker;
    `AGENTS.md` section 6 directs agents to select work by the item 6
    critical path across the open plan.
11. Nothing in this decision alters evidence binding, the `AGENTS.md` batching
    rules, blocked-row rules, or the Decision 0047 completion record and its
    terminal blocker audit.

## Verification

- `README.md` and `PRD.md` "First interface target" rows read: "Standalone
  hardened desktop application (Windows 11 x64 first, Apple Silicon macOS
  second); Visual Studio Code Verified Chat as a secondary surface (Decision
  0047)". A "First buyer" row follows with the item 1 text.
  **Correction (Decision 0051):** the decision identity in that interface row is
  Decision 0048.
- `IMPLEMENTATION-PLAN.md` lines describing native Visual Studio Code Chat as
  the sole v0.1 interface are amended to "sole internal v0.1 harness; the first
  public release surface is defined by Decision 0048".
- All eight `documentation_contract` documents contain the item 10 marker;
  `architecture/status-model.json` `amendment_decision_ids` reads
  `["ADR-0043", "ADR-0044", "ADR-0045", "ADR-0046", "ADR-0047", "ADR-0048"]` and
  `scripts/status_model.py` line 388 expects the same list.
- `TASKS.md` contains Stories 76.2, 76.3, 77.2, and 25.2 (or the numbers the
  implementing batch assigns) with the Universal Story Definition of Done, and
  the Epic 1 and Epic 8 headers carry a "Decision 0048 critical path" note.
  **Correction (Decision 0051):** the assigned preview-gate story is 25.3.
- `AGENTS.md` section 6 names the critical path.
- `docs:validate`, `planning-scope:check`, `architecture:check`,
  `task-graph:check`, and `tests/test_status_model.py` pass on the merge commit.
- Baseline recorded 2026-09-02 23:40: 1,845 done, 3,303 open; both model
  profiles `rejected`; macos-arm64 `retained-post-ga`. Restated at the
  2026-09-06 merge: 4,041 done, 1,484 open, all open rows blocker-bound by the
  Decision 0047 terminal audit; lifecycle 1 integrated, 1 implemented,
  15 scaffolded; product `scaffolded`.
  **Correction (Decision 0051):** at audited commit `b62eb543`, detailed items
  are 4,041 done / 1,107 open / 5,148 total; all checkboxes including headings
  are 4,094 done / 1,484 open / 5,578 total. The preceding mixed denominator is
  retained only as historical text and must not be reused.

## Rationale

The plan's hardened core is its only position the market does not already
occupy. Ordering the frozen set by a buyer-facing critical path converts that
core into an artifact a non-developer can install, in the same number of rows,
by moving roughly two hundred rows earlier and adding four stories. The first
release deliberately excludes writes, connectors, Team mode, and remote routes:
a product that can only read, only locally, with evidence for every answer, is
the strongest possible first trust claim to the item 1 buyer and the smallest
surface to harden.

**Correction (Decision 0051):** the rationale's "frozen set" phrase refers to
the complete preserved plan opened by Decision 0047; no scope freeze remains
active.
