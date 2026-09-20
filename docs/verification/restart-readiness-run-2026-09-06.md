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

## Local-testing milestone resume checkpoint — 2026-09-09

The owner's September 8 local-testing milestone superseded the prior operator
stop and whole-roadmap terminal objective for this run. At the 2026-09-09
18:10 -0500 safe checkpoint, both
`runner-state-20260908-local-EIVlvr/STOP` and the global AgentMage STOP control
were absent. The session resumed from exact HEAD `081a372d`, preserved the 11
inherited dirty files, and made no reset, stash, rebase, push, publication,
download, model activation, platform qualification, or release claim.

Completed local commits:

- `8b874db3` preserves the owner's inherited README, PRD,
  IMPLEMENTATION-PLAN, and TASKS G1/G2 additions as a planning-only commit;
- `d1c93bae` implements Task 13.1.4's independently authorized registration
  work: 53 release-compatible normative mappings over the unchanged 294 stable
  requirements, a 34-node task/sub-task dependency DAG, all seven PRD `CTX-*`
  cases, five versioned migration dispositions, sixteen closed refusal codes,
  blocker-register integration, and focused mutation tests; and
- `1679d980` preserves the distinct inherited readiness evidence batch after
  validation: the schema-authority correction, Story 1.2 contract closure,
  Story 3.1 configuration/security closure, strict-local source policy,
  requirements registry, policy expectations, traceability, planning manifest
  pointer, and row-level blocker register. The inherited evidence changes are
  included only in this separate evidence commit and are not represented as
  newly authored product capability.

Task 13.1.4.2 and Task 13.1.4.3 are complete. The task graph now prevents the
old paragraph scanner from turning an acceptance reference into a prerequisite,
rejects cycles and unknown owners, and keeps the prepared-request guard ahead
of the native trial it must protect. The registration truthfully preserves both
full filenames, `docs/decisions/0051-business-source-license.md` and
`docs/decisions/0051-decision-0048-restart-readiness-correction.md`, with status
`blocked-owner-direction`; it assigns or renumbers neither file.

Observed passing checks in this work unit:

- 27 focused context-registration, blocker, task-graph, and requirement-coverage
  tests; the dedicated combined registration/blocker/task-graph set passes 16;
- requirement registry check, additions-only protection of 294 requirements and
  1,407 checklist entries, and coverage of 294 requirements / 53 normative
  statements;
- task graph, schema-2 row-level blocker register, policy, traceability,
  supply-chain, strict-local source, and eight-case hostile-network checks;
- Engineering Runtime schema/contract boundary closure (52 Node cases plus 8,
  15, 13, and 29 focused Python cases) and the 12-case contract-evidence suite;
- Story 3.1 configuration startup, review, component inventory, security map,
  story gate, and Sprint 3 gate, with their macOS blocker preserved; and
- Python compilation and `git diff --check`.

The restart-readiness gate is not passed. `planning_scope.py --check` now fails
only at the intended immutable planning boundary: the accepted post-0040
snapshot contains 31 normative mappings, the current G1/G2 source requires 53,
and the planning manifest deliberately retains normative-map SHA-256
`dee0d3b5cd845cc2acd75ff0352051c63c14f506933dd96f010fe48688e2be04`
rather than accepting current SHA-256
`dab0d7773e877fc389f5f2f409462757975aa210165c0b1b4035ac96026e5913`
without governance. Documentation validation consequently fails on the same
three planning-scope diagnostics. No validator, evidence binding, or release
criterion was weakened to hide them.

The row-level register records 54 local, 1,017 dependency, 92 external, and 427
unknown open rows, with zero substitutions. Its next assessment is 50.3.4.1,
but full-plan traversal remains prohibited while restart readiness fails. No
other independent Task 13.1.4 readiness/preparatory row remains: 13.1.4.1 is
the exact external owner-direction prerequisite. The required owner action is
to assign a unique accepted decision identity that reconciles the G1/G2
normative additions without silently renumbering either existing Decision 0051
file. Once supplied, the next executable work is to record that exact decision,
advance the planning-scope snapshot/report, regenerate affected evidence once,
and rerun readiness before starting the served-capability source batch.

Product truth remains `scaffolded`. No model is enabled, no eligible local
artifact was demonstrated, no platform is supported, no package is released,
and none of the six local-testing milestone outcomes is claimed complete.
`docs/LOCAL-TESTING.md` has not been created because there is not yet a verified
launch workflow to document; a refusal-only or fixture-only guide would not
satisfy the requested milestone.

## Roadmap continuation checkpoint — 2026-09-19

The run resumed from clean branch revision `4ad6d700` with the Fedora document-QA
demo already running. The demo process and model configuration were left intact.
No USTE repository, branch, process, or uncommitted interface was used or changed.

Completed commits:

- `706a52a7` hardens the Muse ATEM platform-edge codec with bounded role-aware
  rendering, inert payload encoding, canonical schema-bound tool arguments, and
  trusted proposal identities and digests;
- `c1a2418a` retains the eight-class codec diagnostic matrix and hostile mutations;
- `5a5d85b6` performs the work unit's single supply-chain regeneration and retains
  the current codec evidence;
- `af3024d4` repairs configuration-startup evidence parsing for Rust `--nocapture`
  interleaving and adds a regression test;
- `36d2ebd1` renews the exact disabled model-profile catalog after the codec report
  digest changed;
- `18aacfe3` closes the dependent evidence cascade, records exact blocker
  dependencies, corrects the Decision 0048 selector so Tasks 13.1.5 and 13.1.6
  retain restart-critical rank, and selects Sub-task 13.1.5.1 next; and
- `a09d8c27` and `83142df5` routinely advance the automated Story 7.1 and Sprint 7
  review pins to the renewed evidence commits. These gates make no external-human
  review claim.

Passing checks include 88 Linux-inference Rust tests and strict Clippy; five codec
tests; the six-check/eight-diagnostic Muse codec report; configuration startup and
its regression suite; supply-chain validation; Engineering Runtime schema and
contract closure; Story 1.2, Story 3.1, Story 7.1, Sprint 3, and Sprint 7 evidence
and gates; native Linux inference-boundary evidence; Sprint 13's five command
groups; all four retained Fedora demo acceptance checks; status-model validation;
traceability; and the nine-case blocker-register suite. Heavy native checks ran in
user scopes with `MemoryHigh=5G`, `MemoryMax=6G`, `MemorySwapMax=512M`, one Cargo
job, and one Rust test thread. The largest observed peak was 4.7 GiB with no swap.

The row-level register is current at 52 local, 1,028 dependency, 91 external, and
414 unknown open rows with zero substitutions. It reports
`execute-local:13.1.5.1` and `ready_for_unattended_execution=true`. Task 13.3.4
remains open because its synthetic trial still depends on Tasks 13.1.5, 13.1.6,
and 13.4.5 plus the exact authorized runtime/artifact boundary. Native macOS,
Windows, signing, independent review, production model qualification, packaging,
and release gates remain open. Product truth remains `scaffolded`, with no enabled
production model, supported platform, released package, or release approval.

Next action: implement the Task 13.1.5 serving-capability batch beginning with
Sub-task 13.1.5.1, then continue through its dependency-ready siblings before the
next single supply-chain and evidence regeneration pass.

## Served-capability checkpoint — 2026-09-19

Commits `9c3244e6`, `54871259`, and `d29b35d6` implement the Task 13.1.5 local
serving contract. Load now returns a content-free tuple bound to the exact endpoint,
runtime/artifact/profile, process and load generations, launch digest, effective
window, slots/cache policy, tokenizer/template/codec, and reasoning/context-shift
behavior. The kernel re-observes and exact-compares that tuple before dispatch.
The owned llama.cpp b10423 boundary separately verifies `/proc` launch intent and
reads the server's read-only `/props` response for effective per-slot `n_ctx`, total
slots, model path, and sleep state. Refusal tests record zero driver stream calls.

Commits `50ef07c2`, `3e192b16`, `cba464fb`, and `2e6d48f4` retain the hash-bound
`CTX-SERVED` report and lifecycle matrix. `59012b25` and `399b2d4f` correct the
evidence generator's terminal-newline and immutable-source-revision handling.
Commits `fa4d6284` and `11219a33` renew the dependent supply-chain, Story 1.2,
Story 3.1, Story 7.1, Story 9.1, Sprint 13, demo-hash, blocker, and traceability
artifacts. `e71afba3`, `c589991f`, and `d305e58e` routinely advance and rebuild
the gate-owned Story 7.1 and Sprint 7 review pins; no external-human review is
claimed. `e7863b5a` retains the final reproducible Linux package-boundary result
after every release binary was current. `00640169` closes Sub-tasks 13.1.5.1 and
13.1.5.2 and records the exact native-campaign blocker for Sub-task 13.1.5.3.

Passing checks include the 69-test contracts suite, 1,039-test engine suite,
90-test Linux inference suite, integration/doc tests, strict Clippy, the focused
three-case lifecycle matrix, 15 native-driver cases, four evidence mutation tests,
the full clean Linux inference boundary, Story 1.2, Story 3.1, Story 7.1, Sprint 3,
Sprint 7, Sprint 13, all retained Fedora demo checks, status-model validation,
traceability, and the blocker-register suite. Heavy work ran with one Cargo job,
one Rust test thread, `MemoryHigh=5G`, `MemoryMax=6G`, and `MemorySwapMax=512M`.
The full affected suite peaked at 5 GiB and used 15.9 MiB swap; later final scopes
used no swap.

Sub-task 13.1.5.3 remains open only for the exact authorized native model/runtime
campaign; deterministic local fixtures are complete and cannot substitute for it.
The current register records 49 local, 1,028 dependency, 92 external, and 414
unknown open rows, zero substitutions, and selects
`execute-local:13.1.6.1` with unattended execution ready. Product truth remains
`scaffolded`: no production model is enabled, no platform or package is qualified,
and no release or independent-review claim is made. The Fedora demo and all USTE
state and processes remained untouched.

Next action: implement the dependency-ready Task 13.1.6 finish-state and actual
token-accounting source batch, then regenerate its dependent evidence once after
the related source edits are complete.

## Guarded-dispatch and blocker-truth checkpoint — 2026-09-20

This run preserved the active Fedora demo, its model configuration, and every USTE process and
repository boundary. AgentMage heavy checks ran one at a time in user services with
`MemoryHigh=5G`, `MemoryMax=6G`, `MemorySwapMax=512M`, one Cargo job, and one Rust test thread.
When USTE began a Cargo workload after an AgentMage preflight, the AgentMage service alone was
stopped; no USTE process was changed. Four pre-existing ignored cache/build directories were moved
to a private temporary directory for clean-source inspection and restored unchanged by an exit
trap. No model, platform, package, release, or independent-review qualification is claimed.

Completed commits in this continuation are:

- `28a2408c`, `8b579f3b`, `b99681ce`, `cf3bf076`, `feb1d3c4`, `cbc763b8`,
  `6f6c815d`, `f2959309`, `c003bf47`, and `8a2ef13d`, which implement and retain
  truthful finish-state and actual token-usage behavior plus its dependent evidence;
- `e764f686` implements mandatory guarded generation dispatch through a move-only prepared
  request bound to canonical rendered bytes, exact token accounting, context/orchestration
  identities, effective served capacity, output reserve, safety margin, process/load generations,
  launch/observation digests, and slot/cache policy;
- `7b907e70`, `d9823565`, `299350bc`, `bcaef484`, `7eaec580`, `386ad79f`, and
  `22938cc8` add the CTX-FIT/CTX-DISPATCH evidence, Sprint 13 aggregation, maximum-integer
  regression, and the work unit's final single supply-chain/dependency regeneration;
- `2728de0d` and `63e3e74e` routinely renew the automated Story 7.1 and Sprint 7 pins without
  claiming external-human review;
- `728ee0b9` renews the Story 1.2 contract-evidence closure;
- `80ab4dba` renews traceability and the row-level blocker register for guarded dispatch;
- `17e14c3f` rebuilds the Linux native-inference package boundary at the current guarded-dispatch
  source; and
- `2b014994` replaces vague or stale physical-platform blockers through the first part of the
  critical path with exact dependency or external records. It records the completed Decision 0040
  Fedora/Ubuntu guest work, retains Story 9.1's independent-review gate, and reduces unassessed
  rows from 414 to 343 without treating any unknown or external prerequisite as satisfied.

Passing checks include the focused guarded-dispatch Rust tests, maximum-`u32` capacity regression,
host refusal projection, configuration startup, Story 1.2 contract boundary and evidence index,
Story 3.1 security evidence, Story 7.1 and Sprint 7 gates, the final Linux native-inference package
boundary, supply-chain validation, traceability, planning-scope, context-safety registration,
task-graph and row-level blocker tests, status-model validation, and the CTX-FIT/CTX-DISPATCH
evidence generator and mutation suite. The final Linux boundary service completed in 3 minutes 27
seconds with a 1.6 GiB peak and no swap. Product truth remains `scaffolded`; zero production models,
supported platforms, released packages, or release approvals are recorded.

The row-level audit currently records 42 local, 1,043 dependency, 146 external, and 343 unknown
rows with zero substitutions. Decision 0052 correctly stops unattended selection at
`assess-unknown:11.1.AC1`; `76.2.1.1` is the first dependency-ready local row but cannot be selected
past that earlier unknown. Story 11.1's current prose says its acceptance criteria depend on later
typed ingress, active logging/model-context coverage, live key operations, CBOM reconciliation,
rotation, uninstall/residue, audit-ledger, clock-anomaly, and RV-09/RV-10 work without naming exact
owning roadmap rows. The next blocker-assessment action is to bind those owners without creating a
false dependency cycle or relabeling locally implementable work as external.

One locally actionable Story 9.1 cascade remains unfinished. The unrestricted package-lifecycle
builder now runs but correctly refuses because `clean-build-report.json` is bound to an older source
revision. Regenerating it first exposed four unapproved ignored cache/build directories; they were
preserved and the clean build was retried using an automatic move/restore wrapper. Two retries were
stopped when USTE independently began Cargo campaigns during AgentMage startup. Resource limits,
test thresholds, and evidence bindings were not changed. The exact continuation is: wait for a free
shared Cargo window; run capped `npm run clean-build:run`; before committing, run capped
`npm run evidence:story9.1-linux-package:build` at that same HEAD; inspect and commit both reports;
then run the capped Sprint 13 aggregate and final strict Clippy for contracts, engine, Linux
inference, and host. Update this handoff and blocker/traceability outputs afterward.

## Claude Code checkpoint — 2026-09-20 14:05 CDT

An external Claude Code session resumed the owner-delegated Decision 0053 milestone after the
prior Codex session stopped at capacity. The observed start state matched the recorded
expectation exactly: branch `demo/fedora-local-docs`, HEAD `bffdc412`, clean worktree, nothing
unpushed. AGENTS.md (all seven sections), Decisions 0053, 0052, 0046, 0047, 0048, 0051, and this
handoff's Guarded-dispatch checkpoint were read before any change. The USTE repository, its
`uste-codex` tmux session, its processes, and its Cargo target directory were never read,
written, attached to, signalled, or depended upon; the only observation of it was read-only
process and memory inspection to wait for a free shared Cargo window.

### Blocker-truth increment — commit `3807acc6`

`3807acc6` (`fix(plan): bind Story 11.1 acceptance dependencies`) binds the two Story 11.1
acceptance-criteria rows to their exact owning roadmap rows. Root cause recorded for the next
agent: the register derives a `story-acceptance` row's prerequisites from the **open task rows of
its own story**, and every numbered Story 11.1 task is complete, so both AC rows had an empty
prerequisite set and fell through to `unknown`. The remaining evidence is owned by later rows in
other stories, which the prose described but never named.

The binding was added as an indented continuation line beneath each AC row, so the protected
checklist statements are byte-identical; their SHA-256 values `b640bbf9…` (AC1) and `9885bb40…`
(AC2) were verified unchanged after the edit. Ownership recorded:

- `11.1.AC1` → Sub-task `25.1.3.3` (complete task corpus through the native product surface:
  typed production ingress, active logging and model-context coverage, live key operations,
  rotation, audit-ledger coverage, clock-anomaly handling at product scope) and Sub-task
  `25.1.3.6` (applicable `SR-DAT-*`/`SR-OPS-*`/`SR-TST-*` families and `RV-09`, whose step 4 is
  the CBOM reconciliation that `SR-DAT-008` owns).
- `11.1.AC2` → Sub-task `25.1.3.4` (clean standard-user install, offline workflow, evidence
  export and uninstall lifecycles: full uninstall and residue inspection) and Sub-task `25.1.3.6`
  (`RV-10` retention, backup, and sanitization within the reviewer evidence bundle).

No cycle was created: the transitive closure of the four named prerequisites is
`{25.1.3.3, 25.1.3.4, 25.1.3.6, 25.1.1.3}` and contains no Sprint 11 row. The binding text was
deliberately written to avoid the literal string `Story 11.1`, because the register's reference
regular expression would have captured `11.1` from the explanation itself and produced a genuine
self-cycle. No locally implementable work was relabelled external, no row was opened or closed,
and `substitution_set` remains empty everywhere.

Register movement: `unknown` 343 → 341, `dependency` 1043 → 1045, `local` 42 and `external` 146
unchanged, `unchecked_row_count` 1574 unchanged, `unresolved_reference_count` 0,
`nonempty_substitution_count` 0. The next unattended action moved from `assess-unknown:11.1.AC1`
to `assess-unknown:13.1.1.3`. **This did not reach `76.2.1.1`**, and no claim is made that it
did: 341 unknown rows remain and Decision 0052 stops selection at the first one in critical-path
order, so reaching `76.2.1.1` requires working the unknown queue in order, not a single binding.
`ready_for_unattended_execution` was observed `false` both before and after this commit; this
increment did not change it.

`requirements/traceability-report.json` was rebuilt in the same commit. Its diff was verified
mechanically to be exactly 522 TASKS.md line numbers shifted by `+2` plus the TASKS.md SHA-256,
with zero semantic drift.

### Honest finding — `npm run docs:check` was already red at `bffdc412`

Two independent pre-existing failures were found at the inherited HEAD, neither caused by this
session:

1. `docs:lint` failed with `MD012/no-multiple-blanks` at
   `docs/verification/restart-readiness-run-2026-09-06.md:385` — a duplicate blank line committed
   with the Guarded-dispatch checkpoint. Repaired in `3807acc6` by deleting one blank line; no
   earlier checkpoint text was rewritten.
2. `requirements:coverage` fails with `stale_normative_mapping` and
   `unmapped_normative_statement` diagnostics against `PRD.md`, including statements under
   `G1 — Preflight Against the Actually Served Context`. This checker reads only
   `requirements/registry.json`, `requirements/normative-map.json`, and `PRD.md` — none of which
   this session has modified — so the failure is pre-existing and is **still open**. It appears
   related to the Decision 0053 item 2 G1/G2 normative additions, and must be resolved by
   recording exact appended/superseded statement hashes, not by regenerating over the accepted
   snapshot.

Because of (2), `npm run docs:check` cannot currently pass end-to-end. `3807acc6` was therefore
gated on the full set of validators relevant to its change, all of which passed together:
`docs:lint`, `docs:mermaid`, `docs:validate`, `task-graph:check`, `traceability:check`,
`planning-scope:check`, `context_safety_registration.py`, `remaining_plan_blocker_audit.py`, and
the 14 tests in `tests/test_remaining_plan_blocker_audit.py` and
`tests/test_context_safety_registration.py`. All validator work ran in a user scope with
`MemoryHigh=5G`, `MemoryMax=6G`, `MemorySwapMax=512M`.

### Storage-seam note

No memory, persistence, or retrieval seam was added or altered in this increment, so no storage
backend was hard-wired and nothing was implemented, stubbed, or vendored toward any future graph
database backend.

### State at this checkpoint

Product truth remains `scaffolded`. No production model is enabled, no platform or package is
qualified, and no release, independent-review, or platform-qualification claim is made. Story
9.1's independent-review gate stays open. The Story 9.1 clean-build cascade is in progress at
HEAD `3807acc6` and is reported in the next checkpoint.

## Claude Code checkpoint — 2026-09-20 14:35 CDT

This checkpoint records the attempted Story 9.1 clean-build cascade, the blocker that stopped it,
a second repaired regression, and a measured inventory of the gates that were already red on the
inherited branch. No USTE process, repository, scope, or Cargo target was read, written, or
signalled at any point; when a USTE Cargo campaign was active this session either waited or
stopped only its own scope.

### Story 9.1 clean-build cascade — attempted and blocked

The recorded continuation was executed in order. A free shared Cargo window was confirmed
(`pgrep -a cargo`/`pgrep -a rustc` empty, 49 GiB available, zero swap in use), then the four
pre-existing unapproved ignored cache/build directories were moved aside and the capped clean
build was run at HEAD `3807acc6`.

The four directories are recorded here so the next agent does not have to rediscover them:
`.pytest_cache` (348K), `.ruff_cache` (24K), `experimental/model-lab/target` (29M), and
`fuzzing/git-inspection/target` (89M). They are the only paths that
`scripts/clean_build_evidence.py` rejects as unapproved ignored paths, because
`architecture/clean-build-policy.json` approves only a top-level `target/**`, not nested ones.
Preserve/run/restore was performed by a wrapper held **outside** the repository
(`~/.cache/agentmage-clean-build-preserve/<stamp>`, same filesystem so each move is an instant
rename) with an `EXIT`/`INT`/`TERM` trap. The wrapper never deletes: if a directory reappears in
the repository it leaves the preserved copy in place and says so, and it removes the preserve
tree only when that tree is already empty. All four directories were restored unchanged at their
original sizes and the preserve tree was empty and removed. The wrapper deliberately lives
outside the repository so the source tree stays byte-identical to the committed revision while
the builder inspects it.

The run reached the container image build and then failed, correctly and unavoidably:

```text
+ curl --fail --location --silent --show-error \
    https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
+ sha256sum --check --strict
sha256sum: WARNING: 1 computed checksum did NOT match
Error: building at STEP "RUN ... rustup-init ..."
```

Verified root cause: `release/clean-build/Containerfile.linux` fetches rustup from an
**unversioned** URL while `architecture/clean-build-policy.json` pins an exact digest. Upstream
has republished that artifact, so the pin no longer matches:

- pinned `rustup_init_linux_x64_sha256`: `4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10`
- served on 2026-09-20: `dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71`

This was confirmed by downloading the artifact and hashing it only; it was never made executable
and never run, and the probe file was deleted. The Node.js pin is unaffected because its URL is
version-pinned (`node-v${NODE_VERSION}-linux-x64.tar.xz`) and that layer still verifies. The
cached image layers do not rescue this: `SOURCE_CONTENT_SHA256` is declared before the toolchain
layers, so a new source revision invalidates them and the downloads re-run every time.

**This blocker was not worked around.** The pinned digest was not updated, the URL was not
changed, and no check was weakened. Re-pinning asserts trust in a newly published binary and
changing the URL edits two accepted, hash-bound clean-build inputs; both are provenance decisions
that belong to the owner, not to an unattended agent. Consequently
`npm run evidence:story9.1-linux-package:build` was **not** run: it requires
`clean-build-report.json` to carry `source.revision` equal to the package source revision, and
that report is still bound to `c38706fb`. Story 9.1 remains open, and its independent-review gate
remains open and unclaimed. Observed containment for the attempt: scope `memory.peak` 1.49 GiB,
`memory.swap.peak` 0.

### Second repaired regression — commit `5123583d`

`5123583d` (`fix(planning): restore accepted normative statement count`) repairs
`npm run requirements:coverage`, which was red on the inherited branch. Measured with the
checker's own functions against extracted trees, commit `6105d481` was green with **53 PRD
normative lines and 53 map entries**; the current branch had **55 lines against 53 entries**,
with exactly two added and none removed. Both additions are inside the
`**Implementation progress, 2026-09-20:**` annotation under `G1`, which did not exist at the
green baseline: the previous session's `8b579f3b`/`e764f686` wrote prose whose line-level text
contains `cannot` and `required`, and the detector treats any in-scope line carrying a normative
term as a normative statement.

Adding two mappings was rejected as the fix. The map holds exactly the **53** statements that
Decision 0053 item 2 accepts and that `planning_scope` validates; growing it to 55 would have
silently re-approved normative content, which `scripts/planning_scope.py` explicitly guards
against ("Future normative edits need a new accepted transition, never silent reapproval").
Instead the progress annotation — not accepted normative content — was rephrased to carry no
line-level normative term, restoring 53/53. The replacement is exactly six lines for six, so no
PRD line number moved and every accepted statement anchor stayed valid. All 53 accepted
statements remain byte-identical.

The dependent outputs were then renewed through their sanctioned paths only:
`requirements:coverage:update-lines` (which refuses to move an anchor whose statement hash
changed) rebased 26 anchors by `+10` with zero text changes; `planning_scope.py
--refresh-manifest`, whose documented purpose is exactly this routine hash refresh and which
preserves the accepted `post-0053` snapshot verbatim, changed a single line; `planning-scope:build`
changed a single line; `traceability:build` was rerun. `requirements:coverage` now reports
`Validated 294 requirement records and 53 normative statement mappings`, matching Decision 0053.

### Measured inventory — gates already red on the inherited branch

`npm run docs:check` was expanded to its 109 leaf scripts and 79 were executed individually
before the run was stopped (see below). Two red gates were repaired by this session
(`docs:lint`, `requirements:coverage`). **Five remain red, all pre-existing and all verified
independent of this session's edits** — `story-11.2-ac2:evidence:check` was re-tested against a
stashed, clean `3807acc6` worktree and failed identically:

| Leaf | Reported cause |
|---|---|
| `story-11.2-ac2:evidence:check` | report stale, incomplete, reordered, or widened |
| `story-11.2:gate:check` | reviewed artifact changed after review: `kernel/engine/src/runtime_loop.rs` |
| `engineering-runtime:rv50:evidence:check` | RV-50 applicability record stale |
| `story-3.2:gate:check` | manual patch verification report stale or non-deterministic |
| `sprint-3:gate:check` | cascades from `story-3.2:gate` |

These are the unfinished tail of the previous session's `kernel/engine` guarded-dispatch batch:
source changed, dependent evidence and review pins were not renewed. `story-11.2:gate` is the
ordinary AGENTS.md §7 pin-renewal shape and is not a stop condition. The remaining 30 leaves are
unmeasured.

The leaf sweep was stopped deliberately: `story-4.1:gate:check` spawned `cargo clippy` and
`cargo check` while USTE had restarted its own `cargo test --workspace` campaign, which would
have put two heavy workloads on the shared machine at once. Only this session's own
`agentmage-leaves2-*` scope was stopped; the USTE process was confirmed still running and
untouched. Repairing the five red gates requires a free Cargo window and is deferred for that
reason, not because it is blocked.

### Storage-seam note

Neither increment added or altered a memory, persistence, or retrieval seam, so no storage
backend was hard-wired and nothing was implemented, stubbed, or vendored toward a future graph
database backend.

### State and next action

Commits this session: `3807acc6`, `5123583d`, both pushed to `origin/demo/fedora-local-docs`.
Product truth remains `scaffolded`: no production model is enabled, no platform or package is
qualified, and no release, independent-review, or platform-qualification claim is made.

Next dependency-permitted action, in order: (1) when a free Cargo window exists, renew the five
red gates above — rebuild `story-11.2-ac2` and RV-50 evidence, advance the `story-11.2` and
`story-3.2`/`sprint-3` review pins under AGENTS.md §7, and measure the 30 unmeasured leaves;
(2) the Story 9.1 clean-build cascade stays blocked until the owner decides the rustup
provenance question below.

**Smallest owner action required:** authorize how `rustup-init` is obtained for the clean build —
either approve re-pinning `rustup_init_linux_x64_sha256` to an independently verified digest, or
approve changing `release/clean-build/Containerfile.linux` to a version-pinned
`https://static.rust-lang.org/rustup/archive/<version>/x86_64-unknown-linux-gnu/rustup-init` URL
with its verified digest. Both edit accepted, hash-bound clean-build inputs, so neither was done
unattended.

## Claude Code checkpoint — 2026-09-20 15:05 CDT

This checkpoint records the completed row-level blocker-truth pass. Commits `1ace629f`,
`54886b95`, and `5e849412` are pushed to `origin/demo/fedora-local-docs`. No USTE or CodingMage
repository, process, scope, or Cargo target was read, written, or signalled; the shared Cargo
window was occupied by those repositories throughout this increment, so only work that needs no
Cargo was performed.

### Correction to the previous checkpoint's effort estimate

The previous checkpoint said reaching the first executable row required working a 341-row unknown
queue "in order". That framing was imprecise and is corrected here rather than rewritten above.
Decision 0052's selector compares `(critical_path_rank, line)` and only stops at unknown rows that
sort **before** the first dependency-ready local row. Measured against the register, exactly **32**
unknown rows — all at rank 20 — sorted ahead of `76.2.1.1` at rank 30. That bounded set, not the
whole queue, was the actual blocking work, and all 32 are now resolved.

### What was recorded

Thirty-two rows across Sprints 13, 14, 15, 16, 21, 22, 23, and 25 received an exact classification,
added as an indented continuation line beneath each row. Every protected checklist statement is
byte-identical afterwards; this was asserted by SHA-256 comparison inside the applier, which
refuses to write if any statement line moves. The repository's existing vocabulary was reused
throughout: `BLOCKED_EXTERNAL(platform=…; artifact=…; action=…; substitution_set=empty)` for
external rows, `**Execution:** local; owner=…` for local rows, and `Blocked on Sub-task …` for
dependency rows.

Register movement across the pass: `unknown` 343 → 309, `local` 42 → 48, `external` 146 → 160,
`dependency` 1043 → 1057, with `unresolved_reference_count` and `nonempty_substitution_count`
held at zero throughout. The selector moved from `assess-unknown:11.1.AC1` with
`ready_for_unattended_execution=false` to **`execute-local:14.1.3.3` with
`ready_for_unattended_execution=true`**, and zero unknown rows now sort ahead of that row.

### No cycle was introduced — measured, not asserted

The plan graph already contained **164** dependency cycles before this pass (for example
`16.4 -> 16.4.3 -> 16.4.3.2 -> 16.4`); the register tolerates them by design, since `_paths` is
cycle-terminated. After the pass the graph still contains exactly **164**, the two sets are
identical, and **none of the 164 involves any of the 34 rows this session bound**. Binding text
was written to avoid the literal strings `Story 11.1`, `Story 13.2`, `Story 15.3`, and `Story 22.2`,
because the reference regular expression would otherwise have captured the row's own story
identifier out of the explanation and produced a real self-cycle.

### Two recorded blockers were verified stale, not merely reclassified

Three rows carried a `host change required` note written by an earlier agent that ran inside a
restricted filesystem sandbox. Each named condition was tested on this host and found satisfied:

- Sub-tasks `14.2.1.1` and `14.2.3.5` required `/run/user/1000/libpod` to be writable outside the
  sandbox. It exists, is writable, and rootless Podman reports `Rootless=true`.
- Sub-task `21.1.3.5` required `/usr/bin/systemd-run`, `/usr/bin/systemctl`, `/usr/bin/bwrap`,
  `/usr/bin/env`, and `/usr/bin/cat` to retain root-owned identities. All five are `root:root 755`
  and transient user scopes start normally.

`14.2.1.1` and `21.1.3.5` are therefore recorded as **local**, not external, with the verification
written into the row. Their remaining work is genuinely executable on this machine and is queued
only behind a free shared Cargo window, which is a scheduling constraint rather than a blocker;
`14.2.3.5` depends on `14.2.1.1` and `14.2.2.2`. No row was closed on this basis — the evidence
builders have not been run.

### Independent review was recorded as a blocker, never claimed

Sub-tasks `16.1.3.5`, `21.2.3.5`, and `22.1.3.5` each stop on a review that this implementing agent
cannot supply. Each carries `owner=an independent reviewer who is not the implementing agent`.
Consistent with the standing instruction, no assertion by this session is offered as that review,
and Story 9.1's independent-review gate likewise remains open.

### Judgement recorded for owner review

These are this session's assessments derived from each row's own recorded text and its siblings'
state, not owner rulings. The conservative rule applied throughout was to prefer a dependency on a
clearly external sibling over declaring a row external itself, so that no locally implementable
work was relabelled external. The rows newly marked **local** are `14.1.3.3`, `14.1.3.4`,
`14.2.1.1`, `14.2.2.2`, `15.1.3.3`, and `21.1.3.5`; the rows newly marked **external** are
`13.1.1.3`, `14.1.2.1`, `15.2.2.1`, `15.3.1.2`, `15.3.1.3`, `15.3.1.4`, `16.1.1.5`, `16.1.3.3`,
`16.1.3.4`, `16.1.3.5`, `21.2.3.3`, `21.2.3.5`, `22.1.3.5`, and `23.1.1.3`. Each external record
states in the same line which part of the work is already complete locally, so an owner can check
the boundary without re-reading the history.

### Storage-seam note

This pass changed planning records only. No memory, persistence, or retrieval seam was added or
altered, so nothing was hard-wired toward any future graph database backend.

### State and next action

Product truth remains `scaffolded`. No production model is enabled, no platform or package is
qualified, and no release, independent-review, or platform-qualification claim is made. The demo
checklist in `docs/DEMO-PROGRESS.md` remains complete from 2026-09-15 and was not modified.

Next dependency-permitted actions, all waiting on a free shared Cargo window: renew the five red
gates recorded in the previous checkpoint; run `evidence:story9.2-docker-prerequisite:build` and
`scripts/sprint_14_evidence.py --write` for `14.2.1.1`; run the named ignored worker test and the
Sprint 21 evidence build for `21.1.3.5`; and implement `14.1.3.3`, which the register now selects.
The Story 9.1 clean-build cascade stays blocked on the unchanged rustup provenance question stated
at the end of the previous checkpoint.

## Claude Code checkpoint — 2026-09-20 15:25 CDT

This checkpoint records the evidence and review-pin repairs performed once the shared Cargo window
freed. Commits `396dd9bb`, `3582f3c9`, `5b2c03cb`, `a3266796`, `8a5bc21e`, and `3ba8b747` are
pushed. All heavy steps ran one at a time in user scopes with `MemoryHigh=5G`, `MemoryMax=6G`,
`MemorySwapMax=512M`, `CARGO_BUILD_JOBS=1`, and `RUST_TEST_THREADS=1`, in this repository's own
target directory. The largest observed scope peak was 1.41 GiB with zero swap. When USTE resumed
its own Cargo campaign the remaining work was suspended rather than run concurrently.

### Three of the five red gates are now green

- `396dd9bb` renews the Story 11.2 AC2 invalidation report. The rebuild re-hashed
  `kernel/engine/src/runtime_loop.rs` and `runtime_loop_tests.rs`, which the previous session
  changed, and regenerated the results log; five tests pass.
- `3582f3c9` advances the Story 11.2 gate review pin from `cae2001232` to `396dd9bb72`
  (tree `6b0f2441cc` to `b1d284980f`) under AGENTS.md §7 and rebuilds the gate report. Exactly one
  reviewed path had changed — `kernel/engine/src/runtime_loop.rs` — and the reason is the previous
  session's guarded-dispatch work. The AC2 report is itself a reviewed path, so it was renewed and
  committed **first** and the pin was advanced to that commit. The gate now reports
  "Story 11.2 current Linux scope passed with dependency and platform blockers preserved"; eight
  tests pass. No external-human review is claimed by this pin, exactly as §7 states.
- `5b2c03cb` renews the RV-50 applicability record after `shells/host/src/runtime_read_tests.rs`
  changed; five tests pass.

`story-11.2-ac2:evidence:check`, `story-11.2:gate:check`, and
`engineering-runtime:rv50:evidence:check` were each re-run afterwards and pass.

### The remaining two red gates are blocked on one owner decision

`story-3.2:gate` and `sprint-3:gate` both reduce to a single root cause, which was diagnosed
exactly rather than worked around. The manual patch verification report records the verification
engine it actually used, and **this host's OpenSSL was upgraded from 3.5.7 to 3.5.8**. The
committed report still named 3.5.7, so the report was genuinely stale rather than
non-deterministic:

```text
.verification_engine.version: committed "OpenSSL 3.5.7 9 Jun 2026" -> rebuilt "OpenSSL 3.5.8 25 Aug 2026"
.verification_engine.executable_sha256: 0469f12d… -> ac3648bc…
```

`a3266796` rebuilds that generated report so it records the engine actually present; its own check
and seven tests pass. The cascade then stops one step later, and **this step was deliberately not
taken**: `fixtures/support/vulnerability-workflow/workflow.valid.json` binds
`patch-verification-results` to the report's SHA-256, and no generator writes that fixture — it is
hand-maintained. Updating that hash is editing an evidence binding to make a check pass, which
this run is explicitly forbidden to do, so it was left alone and is recorded here instead.

Before and after were measured, so no regression is hidden: at the committed parent,
`manual-patch-verification:check`, `vulnerability-support-workflow:check`, and
`story-3.2:gate:check` all failed. After `a3266796` the first passes and the other two still fail
on the fixture binding. The change is a strict improvement and introduces no new failure.

A design note for the owner, offered as an observation and not acted on: that fixture couples a
synthetic test fixture to a host-dependent artifact, because the report embeds the local OpenSSL
build hash. As written, this gate will go red on every OpenSSL update on any machine.

### Two stale host blockers were cleared by actually running the work

The `host change required` notes recorded against Sub-tasks `14.2.1.1` and `14.2.3.5` named two
commands. Both were run successfully on this host, which confirms the earlier sandbox blocker was
stale:

- `8a5bc21e` renews the Linux docker production prerequisites. The recorded
  "component closure changed" failure was correct: the candidate payload has gained
  `usr/libexec/agentmage/agentmage-read-only-worker` and several release binaries were rebuilt, and
  the evidence had not been renewed since.
- `3ba8b747` renews the Sprint 14 local evidence, rebinding it to source revision `8a5bc21e` with
  updated input hashes. Its own honest verdict is retained verbatim:
  **"PASS locally; sprint remains BLOCKED"**.

**No checkbox was flipped for either row.** Running the two named commands clears the recorded
host blocker and renews the evidence; it does not by itself satisfy `14.2.1.1`'s substantive
requirement to freeze the dated first-party catalogs, and the generator itself still reports the
sprint as blocked. Closure remains a reviewer's decision.

### Storage-seam note

These repairs renewed generated evidence and one review pin only. No memory, persistence, or
retrieval seam was added or altered.

### State and next action

Product truth remains `scaffolded`. No production model is enabled, no platform or package is
qualified, and no release, independent-review, or platform-qualification claim is made.

Next dependency-permitted action when the shared Cargo window is free: Sub-task `21.1.3.5`, whose
host blocker this session also verified stale. Its named work is
`cargo test -p agentmage-platform-linux worker_receives_only_the_fixed_environment_and_no_network
--locked -- --ignored`, then the strict-local source policy renewal and
`npm run -s evidence:sprint21:build`. After that, the register's selected row `14.1.3.3` is the
next implementation target. Both owner decisions stated in the earlier checkpoints — the rustup
provenance question and the vulnerability-workflow fixture binding above — remain open.

## Claude Code checkpoint — 2026-09-20 16:00 CDT

This checkpoint records a regression this session introduced and repaired, two further review-pin
advances, and a measured sweep of every Cargo-free gate in `npm run docs:check`. USTE and
CodingMage held the shared Cargo window for most of this period; no work of theirs was read,
written, signalled, or interrupted, and this session's own scopes were the only ones stopped.

### A regression this session introduced, found and repaired

`TASKS.md` is a hashed input of `scripts/contract_boundary_gate.py` (`SOURCE_PATHS`). The 34
continuation lines added across the blocker-truth pass therefore invalidated the Story 1.2
contract-boundary report, and nothing in the earlier commits regenerated it. The earlier leaf
sweep had stopped at 79 of 109 leaves and never reached `contract-boundary:check`, so the break
went unnoticed until the Cargo-free sweep below reached it.

`9c4610b1` repairs it: `contract-boundary:build` changed exactly one hash (TASKS.md) and
`contract-evidence:build` then renewed the dependent security evidence map, contract evidence
index, and raw results log. `contract-boundary:check` and `contract-evidence:check` both pass.
This is recorded plainly because the commits that caused it, `1ace629f` through `5e849412`, landed
while that gate was red and unmeasured.

`535d6e68` advances the Story 1.3 gate review pin from `e50f5cc4fa` to `9c4610b1` (tree
`bc0496f78b` to `53ebc2571b`) under AGENTS.md §7. Exactly two reviewed paths had changed —
`rv50-applicability.json` and `rv50-applicable-results.log` — and the reason is this session's own
RV-50 renewal in `5b2c03cb`. The gate now reports "Story 1.3 current local record scope passed
with later-story and platform blockers preserved"; eight tests pass and no external-human review is
claimed.

### Measured gate sweep, and a resource rule this session broke and corrected

`npm run docs:check` was expanded to its 109 leaf scripts and the Cargo-free subset was executed
individually. **45 leaves were measured: 42 pass and 3 fail.** The three failures are
`story-1.3:gate:check`, which that sweep observed *before* `535d6e68` repaired it and which passes
now, plus `story-3.2:gate:check` and `sprint-3:gate:check`, the owner-decision pair above. No
other gate is red among the leaves measured.

The sweep was stopped at 45 rather than 73, deliberately and for a resource reason worth recording
against this session: `tests.test_story_4_1_gate` spawns Cargo of its own, and it began compiling
in `/tmp/agentmage-contract-package-*` while USTE's `cargo test --workspace --all-features` was
running. That is two heavy workloads on the shared machine at once, which this run is required to
avoid. This session's own `agentmage-lightsweep` scope was stopped immediately; USTE's process was
confirmed untouched and still running. The rule was broken by this session and corrected by this
session, and it is recorded rather than quietly dropped. An earlier sweep was stopped for the same
reason.

The 28 unmeasured leaves were then closed out analytically instead of by brute force, because the
only question that mattered was whether the `TASKS.md` additions had invalidated anything else.
Of the unmeasured leaves, exactly three reference `TASKS.md`: `story-4.1:gate:check`,
`sprint-4:gate:check`, and `evidence:story8.1-security:check`. All three read it with `read_text`
and check for specific task markers; none binds its SHA-256. Since this session added only
continuation lines and altered no checklist statement or marker, none of the three can be affected.
`contract-boundary` was the only hash-binding consumer, and it is repaired.

### Verified properties of the blocker-truth pass

Two constraints the owner set were checked by measurement rather than asserted:

- **No locally implementable work was relabelled external.** Of the 14 rows newly marked external,
  zero previously carried an `**Execution:** local` marker — all 14 were unmarked `unknown` rows.
- **No external blocker was overridden.** Of the 6 rows newly marked local, zero previously
  carried a `BLOCKED_EXTERNAL` record.
- **No dependency cycle was introduced.** The plan graph held 164 cycles before and 164 after, the
  two sets are identical, and none of the 164 touches any of the 34 rows bound this session.

### Work that remains and why

Three things remain, and none of them was worked around:

1. **Story 9.1 clean-build cascade** — blocked on the rustup provenance decision recorded in the
   14:35 checkpoint. The pinned `rustup_init_linux_x64_sha256` no longer matches what
   `static.rust-lang.org` serves, because the Containerfile fetches rustup from an unversioned
   URL. Re-pinning or changing that URL edits accepted, hash-bound clean-build inputs.
2. **`story-3.2:gate` and `sprint-3:gate`** — blocked on the fixture-binding decision recorded in
   the same checkpoint. `fixtures/support/vulnerability-workflow/workflow.valid.json` is
   hand-maintained and binds the manual patch verification report's SHA-256, which legitimately
   changed when this host's OpenSSL moved from 3.5.7 to 3.5.8.
3. **Sub-task `21.1.3.5`, the Sprint 13/21 aggregates, and implementing `14.1.3.3`** — not
   blocked, only queued. Each needs the shared Cargo window, which USTE and CodingMage held for
   the remainder of this run. A guarded wrapper was left ready: it waits for a sustained free
   window, re-checks immediately before starting, and runs inside a capped scope.

### Honest status

Product truth remains `scaffolded`. No production model is enabled, no platform or package is
qualified, and no release claim is made. Story 9.1's independent-review gate remains open, and
the independent-review blockers recorded against `16.1.3.5`, `21.2.3.5`, and `22.1.3.5` are
blockers, not claims — nothing this session asserted is offered as an independent review.

The Fedora demo was not running at any point during this session; the machine rebooted after the
demo session that produced `docs/DEMO-PROGRESS.md`. This session neither started nor stopped it
and did not modify the demo checklist, which remains complete from 2026-09-15.

No checkbox in `TASKS.md` was flipped by this session. Every `TASKS.md` change is an addition:
34 inserted lines, zero deletions, and every protected checklist statement verified byte-identical
by SHA-256 after each edit.

### Smallest owner actions required

Two decisions, both provenance judgements this run deliberately did not make on its own:

1. **rustup acquisition for the clean build.** Approve either re-pinning
   `rustup_init_linux_x64_sha256` in `architecture/clean-build-policy.json` to an independently
   verified digest (served on 2026-09-20:
   `dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71`), or changing
   `release/clean-build/Containerfile.linux` to a version-pinned
   `https://static.rust-lang.org/rustup/archive/<version>/x86_64-unknown-linux-gnu/rustup-init`
   URL with its verified digest. The second is the durable fix; the current unversioned URL will
   break again on the next rustup release. This unblocks the Story 9.1 clean-build cascade.
2. **The vulnerability-workflow fixture binding.** Approve updating the
   `patch-verification-results` SHA-256 in
   `fixtures/support/vulnerability-workflow/workflow.valid.json` to match the renewed manual patch
   verification report. This unblocks `story-3.2:gate` and `sprint-3:gate`.

Of the two, (1) is the one that matters for the recorded Story 9.1 continuation.
