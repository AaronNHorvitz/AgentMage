# Restart Readiness Run — 2026-09-06

Attended continuation from [`first-ga-run-2026-09-05.md`](first-ga-run-2026-09-05.md).
The starting state is branch `build/agentmage-ga` at
`b62eb543ad4f2de39da529a1ab0e24e54efbb891`, matching the September 6 audit,
with a clean worktree. The owner directed this bounded restart-readiness
implementation batch on 2026-09-06. That direction authorizes the specific
governance correction and local implementation named in the completion prompt;
it does not authorize a push, publication, purchase, external message, hosted
runner, secret access, model acquisition, platform provisioning, signing, or a
release claim.

## Executable milestone plan

1. Record an additions-only correction to Decision 0048: preserve incident
   tabletop Story 25.2, assign Story 25.3 to the preview gate, correct every
   preview-gate reference and both checkbox baselines, replace residual
   frozen-plan/wrong-decision wording, and clarify read-only state plus the
   separately consented model/update/diagnostic network phases.
2. Register Stories 25.3, 76.2, 76.3, and 77.2 with dependencies, acceptance,
   controls, current status, and preview/full-GA/retained-platform
   applicability. Extend the task graph so omission, unknown dependencies,
   applicability drift, and cycles fail closed.
3. Replace sprint-paragraph blocker inheritance with a row-level dependency
   register that distinguishes local, external, and unknown/unassessed work and
   records exact prerequisites, evidence, resolution actions, owners, venues,
   and dependency paths. Add semantic regressions for unrelated blockers,
   transitive dependencies, and newly registered local work. Do not refresh the
   old artifact under its old semantics.
4. Complete the bounded source-maintenance repairs assigned to this work unit
   (TypeScript formatting and the host Clippy fixture warning), run focused
   tests, then finalize all source and governance inputs.
5. Reconcile schema authority, rebuild boundary evidence, regenerate the
   corrected blocker register and contract index in dependency order, and make
   one consolidated evidence pass. Run the applicable planning, status,
   documentation, formatting, lint, Rust, TypeScript, and Python gates. Record
   passes, failures, ignored/native-not-run work, changed files, and the exact
   next authorized critical-path gate.

## Initial truth and known failures

- Product lifecycle remains `scaffolded`; the deterministic fake-model Story
  22.5 source-level workflow is integrated; no model is enabled, no platform is
  supported, and no package is released.
- The audited baseline at `b62eb543` is 4,041 done / 1,107 open / 5,148 detailed
  items, and 4,094 done / 1,484 open / 5,578 all checkboxes including headings.
- The retained blocker artifact is deliberately stale and semantically unsound;
  it must remain visibly failing until the row-level implementation replaces
  sprint-paragraph inheritance.
- The contract chain is stale upstream at the schema-evolution authority binding
  for `ENGINEERING-RUNTIME.md`; the boundary must be repaired before the index.
- Historical Muse evaluation remains rejected and immutable. PDF/Word cache and
  Windows counter repairs are mandatory later admission gates and are not
  activated by this batch.

## Resume checkpoint

The interrupted source and planning work was revalidated on 2026-09-07. Both
the run-specific `runner-state-20260907-resume-fCVKB8/STOP` control and the
global AgentMage STOP control were absent at the latest safe checkpoint.

Implemented but not yet evidence-regenerated:

- Accepted corrective Decision 0051 preserves Story 25.2, assigns the preview
  gate to Story 25.3, corrects both historical checkbox denominators and the
  full-plan wording, and registers preview/full-GA/retained applicability.
- Stories 25.3, 76.2, 76.3, and 77.2 are present with explicit dependencies,
  acceptance, controls, status, local work, and exact external rows. The task
  graph rejects story omission, unknown requirements, cycles, applicability
  drift, source-document writes, and network-consent drift.
- The blocker audit is now a row-scoped schema-2 register. It retains complete
  direct prerequisites and a bounded deterministic transitive witness for each
  direct edge, distinguishes local/dependency/external/unknown rows, records
  source evidence, action, owner, venue, and empty substitutions, and fails
  closed on an earlier unknown. On current `TASKS.md` it classifies 1,543 open
  rows as 28 local, 997 dependency, 91 external, and 427 unknown, with zero
  unresolved reference identities. It selects local Sub-task 13.3.4.1.
- Additions-only Task 13.3.4 registers the brief's model-facing codec,
  diagnostic, and newly versioned qualification work as the first post-
  readiness local gate. The prior Muse rejection remains immutable and no
  model is enabled.
- The Muse admission validator now separates the immutable historical policy
  binding from the current catalog-policy binding. TypeScript formatting and
  the host Clippy fixture warning are repaired. The schema-evolution authority
  digest is reconciled to current `ENGINEERING-RUNTIME.md`.
- The strict-local policy now exactly includes the previously accepted current
  Cargo manifests, VS Code compatibility contributions, data-only schema/link
  origins, and `IpAddr` parsing surface. It adds no network implementation or
  normal-operation egress and retains closed mutation checks.

Current validation results:

- PASS: 61 focused readiness tests covering Muse admission, blocker semantics,
  task graph, status, schema evolution, and contract boundary.
- PASS: 11 strict-local source-policy tests and the live strict-local audit.
- PASS: `npm run product:format-check`, Rust/TypeScript builds, strict Clippy,
  ESLint, hostile-network injection, effect-boundary validation, Markdown lint,
  task graph, status model, and schema-evolution checks.
- BLOCKED BY TOOL SANDBOX: the full `npm run product:check` reaches the host
  Rust suite, where 241 tests pass, 36 fail, and 8 remain ignored because native
  test sandbox manifests and a live Git artifact are rejected in this managed
  execution environment. The common failures are `InvalidManifest` or
  `InvalidGitArtifact`; no weaker rerun or control bypass was attempted.
- BLOCKED BY TOOL SANDBOX: Mermaid browser rendering cannot launch Chromium in
  this environment. Markdown lint passes with zero findings.
- EXPECTED STALE BEFORE THE CONSOLIDATED PASS: planning-scope source binding
  and report, schema-2 blocker artifact, contract-boundary report, contract
  security map/index, SBOM, dependency provenance, and dependency hash manifest.

Product truth remains `scaffolded`: one source-level deterministic fake-model
workflow is integrated, no model is enabled, no platform is supported, no
package is released, and release approval remains blocked. This batch changes
planning/control breadth and source maintenance only; it does not promote the
product lifecycle or verification status.

Next executable action: create the local source/governance checkpoint commit so
revision-bound evidence uses the corrected sources, then refresh the planning
manifest/report, blocker register, boundary chain, contract index, SBOM, and all
affected downstream evidence once in dependency order. After readiness passes,
implement Sub-task 13.3.4.1 in `platforms/linux-inference`.

## Latest checkpoint — 2026-09-07

This checkpoint supersedes the earlier resume paragraph above. The bounded
readiness implementation and its first consolidated evidence pass were retained
in commits `735a58a5`, `dd53b88e`, `d4d97030`, `d359af20`, and `691ac2a5`, with
follow-up evidence repairs through `e1b32e7f`. The resumed audit then retained:

- `2909e972`: close the live kernel-contract source package over all current
  contract modules while explicitly excluding the repository-bound engineering
  schema test from the self-contained crate;
- `1ccc85ae`: refresh the live schema-v2 reference from 249 exports and 33
  versioned contracts to the current 519 exports and 96 versioned contracts;
- `9613571b`, `06252ba4`, `f44a7abf`, and `f40b25f3`: regenerate the package,
  reference, fixture, architecture, security, Story 4.1, and Sprint 4 evidence,
  and routinely advance the Story 4.1 and Sprint 4 automated review pins; and
- `09b3c1fc`: repair three expanded-closure mutation checks: repeated contract
  evidence markers, the nine-file Docker prerequisite package, and the distinct
  material-claim authority mapping.

Verified results in this resumed audit:

- the kernel-contract package, Rustdoc reference, golden fixtures, architecture
  report, security map, Story 4.1 gate, and Sprint 4 gate pass; their combined
  story/sprint mutation suite passes 15 tests;
- current requirements, planning scope, task graph, status, supply-chain,
  artifact scan, dependency, policy, traceability, product-CI contract, schema,
  Markdown, strict-local, and hostile-network checks pass in their recorded
  pre-license-migration closure;
- the strict-local audit reports zero undeclared network paths and all eight
  hostile network mutations are blocked before execution; 16 focused tests pass;
- the additional component-inventory, configuration-review, configuration-
  startup, contract-index, Docker prerequisite, and material-claim repairs pass
  their focused tests; and
- fail-fast Python traversal reached 1,628 tests after excluding only exact
  unavailable native/browser wrappers. It identified and preserved the existing
  sandbox blockers below rather than weakening their gates.

Exact unavailable prerequisites remain:

- Chromium/Mermaid cannot launch in the managed sandbox;
- IPv4 and Unix socket creation is denied, blocking the native Ubuntu listener,
  VS Code host-bridge, inference-listener, and related native checks;
- network-namespace inspection (`ip -j link show`) and the path-race namespace
  harness are denied; and
- the runtime-artifact boundary review cannot pass
  `resume-report-integrity` because its required filtered host resume campaign is
  unavailable here. Historical replay therefore stops truthfully at the Story
  6.1 path-race gate.

At 2026-09-07 17:47 -0500, an independent owner-authored commit,
`2fea871a` (`Decision 0051: Business Source License 1.1`), advanced both HEAD and
`origin/build/agentmage-ga` while this readiness traversal was running. Its
license changes are preserved. It also creates a second accepted Decision 0051
alongside `0051-decision-0048-restart-readiness-correction.md`. The new decision
expressly forbids renumbering, so the agent cannot infer whether the license
decision should become Decision 0052 or whether another identity is intended.
Current documentation validation consequently fails the unique governance and
Decision 0001 license-consistency closure. No evidence was regenerated over
that unresolved authority collision.

The worktree retains six generated evidence files from the interrupted
pre-license refresh plus `security/strict-local-source-policy.json`; these are
uncommitted and now stale relative to `2fea871a`, so they were preserved rather
than committed as current. Product truth remains `scaffolded`: no model is
enabled, no platform is supported, no package is released, and no release
approval is claimed.

Blocking owner action: assign the Business Source License decision a unique
accepted identity or explicitly authorize another collision resolution. After
that decision, the next authorized task is to reconcile the license authority
and reviewed manifest hashes, regenerate supply-chain and all dependent evidence
once, rerun the readiness gate in a native environment that permits its required
socket/namespace/browser checks, and only then begin Sub-task 13.3.4.1.

### Final safe checkpoint — operator stop

The preceding license-collision paragraph records the state observed during the
run. Before this checkpoint was committed, an independent owner-side commit
`b4b8f4d8` reverted `2fea871a`; the local branch therefore no longer contains
the BUSL migration or duplicate Decision 0051. No license-collision decision is
currently requested from the owner.

At the final safe checkpoint, the run-specific STOP control remained absent but
`/var/home/aaronnhorvitz/.local/share/agentmage-run/STOP` was present. Work
stopped immediately. The six generated Story 1.2/Story 3.1 evidence files and
the strict-local policy update remain uncommitted and preserved. No readiness
gate, model/platform qualification, release, publication, or full-plan traversal
is claimed. The next action is operator-controlled: remove the global STOP file
and explicitly resume; then revalidate the concurrent revert and retained
worktree before completing the evidence batch.
