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

### Sprint 21 attempt — waited, then stood down cleanly

Sub-task `21.1.3.5`'s named work was queued behind a guarded wrapper that waits for a sustained
free shared Cargo window, re-checks immediately before starting, and would run inside a capped
scope. It waited its full 60-minute budget, from 15:20 to 16:20 CDT, and stood down without
starting anything:

```text
ABORT: no free Cargo window within the wait budget
```

USTE held the window continuously across that period with a single
`cargo test --locked --offline --workspace --all-targets --all-features -- --test-threads=1`
campaign. No USTE process was interrupted, signalled, or inspected beyond reading its command
line and working directory, and no AgentMage Cargo work was started alongside it. The wrapper is
preserved and can be rerun as-is; it is the first thing to retry when the machine is quiet.

## Claude Code checkpoint — 2026-09-20 18:30 CDT

This checkpoint covers the first block of work under the standing owner delegation of
2026-09-20, recorded as **Decision 0054**. Every decision below carries the status line
"Accepted under owner delegation, 2026-09-20". The delegation also replaced the shared-machine
rule: this session no longer waits for another repository's Cargo work and starts a build
whenever `free -h` reports at least 16 GB available, keeping the scope caps and using up to four
Cargo jobs. USTE, CodingMage and AgentMagik were not read, written, or signalled at any point.

### The rustup blocker is resolved — Decision 0055

The clean build fetched `rustup-init` from an **unversioned** URL while the policy pinned a
digest, so the two drifted apart on every rustup release. The versioned immutable archive path is
now used and the unversioned `/rustup/dist/` path is gone from every build input:

- `rustup_init_version`: **1.29.1**, resolved from `release-stable.toml`
- `rustup_init_linux_x64_sha256`:
  **`dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71`**

The digest was verified against the publisher's own `rustup-init.sha256` for that exact archive
path, and independently against a locally computed hash of the 21,113,232-byte download. The
artifact was hashed only — never made executable, never run outside the container — and deleted
afterwards. Version and digest live together in `architecture/clean-build-policy.json` and are
passed as build arguments, so the Containerfile cannot silently disagree with the policy. The
in-image `sha256sum --check --strict` verification is retained unchanged; the versioned URL is an
additional control, not a replacement. `scripts/linux_vm_promoted_matrix.py` carried the same
unversioned pattern and the same stale pin, and was corrected identically.

### The five scaffold test failures are fixed — Decision 0056

With rustup resolved, the clean build advanced to the container's `test` command and failed there
on five `LicenseMismatch` panics — the long-recorded "five existing Apache scaffold fixture
failures" from `docs/DEMO-PROGRESS.md`. The cause was a test-input defect, not a licensing one.

`package_scaffold` generates **new user projects** under Apache-2.0 and pins the exact artifact it
emits. Its shared test helper fed it `include_bytes!("../../../LICENSE")` — the AgentMage
repository's own licence. That was coincidentally correct while AgentMage was itself Apache-2.0;
when Decision 0051 relicensed it to the Business Source License, the helper began feeding BSL
bytes into an Apache-2.0 scaffolder and all five tests panicked before reaching any assertion.

The exact Apache-2.0 text the constant pins was recovered from this repository's own history —
commit `b4b8f4d8` carries a `LICENSE` blob whose SHA-256 is exactly
`02f41e321c6eabad29b0f412b9aaa710dfcff9df7fa563a8db0a408e35e5ba6f` — and retained at
`fixtures/licensing/apache-2.0.txt`. The helper now reads that fixture.

**No licence choice was made.** AgentMage remains under the Business Source License 1.1;
`LICENSE` and `NOTICE` are byte-identical. The scaffolder still emits Apache-2.0 projects; the
pinned digest and all seven `Apache-2.0` output declarations are unchanged. **No test was
weakened**: the suite moved from 43 passed / 5 failed to 48 passed / 0 failed with no assertion or
threshold relaxed, because the tests now reach the assertions they were written for.

### Demo acceptance was re-run with the real model

Editing a Cargo workspace member flipped the SBOM, which the demo acceptance reports bind as
inputs, so those reports went stale. Re-binding them without re-running would have fabricated
evidence, so the full acceptance suite was re-run through `scripts/demo_smoke.py` against the
actual Muse Glimmer 30B Q4_K_M under llama.cpp b10423 Vulkan.

All cases passed: browser launch and real model ready, multi-document admission and unsupported
input reasons, real inference with checked source citation, follow-up and second-source grounding,
honest insufficient evidence, context overflow refused before generation without silent
truncation, cancellation then recovery, model-unavailable recovery, bounded six-turn conversation
with explicit new-conversation recovery, folder-boundary symlink traversal and invalid inputs,
and privileged endpoints rejecting missing token, wrong origin and DNS rebinding. Offline
isolation passed with the model inside a networkless `bwrap` namespace, and a full stop/restart
was followed by a successful fresh interaction. Report timestamps advanced from 2026-09-16 to
2026-09-20 with 15 recorded real model interactions, so these are genuine new runs.

The demo acceptance ran in a scope sized for a 16.7 GB model (`MemoryHigh=20G`, `MemoryMax=24G`,
`MemorySwapMax=0`) rather than the 6 GB build cap. That is a delegated choice recorded here: the
build cap is retained for builds, tests and evidence regeneration, and a model-serving workload
gets containment sized to it rather than no containment. The demo is left running, as the suite
intends.

### The clean Linux build now passes — the Story 9.1 headline result

At source revision `b29056b1`, with the four pre-existing ignored cache/build directories
preserved and restored intact:

```text
clean-build evidence passed: applicability=current-reviewed-source
```

Both `fedora-x86_64` and `ubuntu-x86_64` passed **11 of 11 commands**, `linux_platforms_passed`
is 2 of 2, and `linux_scope_complete` is true. `cross_platform_task_complete` remains **false**,
which is correct: macOS stays blocked. Scope `memory.peak` was 5.0 GiB against the 6 GB cap with
**zero swap**. The report is committed in `7b9caca3`.

### Package lifecycle evidence — exact blocker, not worked around

`npm run evidence:story9.1-linux-package:build` now clears the clean-build binding that used to
stop it and fails further in, at the container install step. The precise cause was reproduced:

```text
error: Failed dependencies:
  bubblewrap is needed by agentmage-0.0.0-1.fc44.x86_64
  git-core is needed by agentmage-0.0.0-1.fc44.x86_64
  systemd is needed by agentmage-0.0.0-1.fc44.x86_64
```

A prior session added real runtime dependencies to `packaging/linux/agentmage.spec.in` and
`debian-control.in` (`Requires` moved from `glibc, openssl-libs` to include `bubblewrap`,
`git-core`, `systemd`) when the read-only worker and isolated Git inspection landed. The lifecycle
evidence was never re-run afterwards. `rpm -i` is therefore failing **correctly**: the pinned bare
base image does not provide the package's declared prerequisites, and the lifecycle container runs
`--network=none`.

This was deliberately **not** resolved by `--nodeps`, which would stop the test verifying the
dependency declarations at all. It was also not resolved by preparing a local image with the
prerequisites installed, because `validate_container_lifecycle` requires the lifecycle image to be
an **immutable published reference present in its own `repo_digests`**, and a locally built image
has no repo digest. Relaxing that control, or the `network_used: false` assertion, would weaken an
evidence binding, which the delegation explicitly does not permit.

The recommended resolution, for the next work unit, is to give the lifecycle the same accepted
**two-phase shape the clean build already uses**: a recorded bootstrap phase that installs exactly
the prerequisites `architecture/clean-build-policy.json` already documents under
`runtime_dependencies.platform_packaged`, followed by the lifecycle steps with network disabled,
with the bootstrap recorded as its own phase rather than folded into `lifecycle_network_used`.
That mirrors `bootstrap-container-build` / `container-disabled` and weakens nothing. It is a
material change to an accepted evidence artifact's meaning, so it is recorded here before being
implemented rather than slipped in.

### Sequencing note for whoever runs this next

The clean-build report records the revision it was built from, so committing it advances `HEAD`
past that revision and the package-lifecycle builder — which requires
`clean_build.source_revision == HEAD` — will refuse. The correct order is therefore: run the clean
build, run the package-lifecycle evidence at that same `HEAD` while the only worktree change is
the clean-build report, then commit both together. This run committed the clean-build report on
its own because the package step is blocked above, so the next attempt needs a fresh clean build
at the then-current `HEAD`. That costs roughly one 14-minute build.

### Honest status

Product truth remains `scaffolded`. No production model is enabled, no platform or package is
qualified, and no release or independent-review claim is made. The clean Linux build passing is
build reproducibility evidence, not platform qualification and not a release.

## Claude Code checkpoint — 2026-09-20 19:00 CDT

Continuing under the Decision 0054 delegation. Commits `20dce0e6` and the Decision 0057 record
below complete this block.

### Strict Clippy and the Sprint 13 aggregate

Strict Clippy passed with `-D warnings` and produced no warnings on all four crates named in the
recorded continuation: `agentmage-kernel-contracts`, `agentmage-kernel-engine`,
`agentmage-platform-linux-inference`, and `agentmage-host`. The Sprint 13 aggregate was renewed at
the current source and reports its own honest verdict verbatim: **"Sprint 13 local contracts pass;
sprint remains blocked on three explicit evidence classes"** (`20dce0e6`).

### Package lifecycle — direction fixed by Decision 0057

The install failure diagnosed in the previous checkpoint was taken further rather than patched.
Both pinned base images were probed directly and provide none of the three declared
prerequisites:

```text
docker.io/library/fedora@sha256:89f61a12…   bwrap MISSING  git MISSING  systemctl MISSING
docker.io/library/ubuntu@sha256:7b202b0e…   bwrap MISSING  git MISSING  systemctl MISSING
```

Two resolutions were considered and **rejected in writing**: `--nodeps`, which would stop the
evidence checking dependency declarations at all; and running the lifecycle in a locally built
image carrying the prerequisites, which would retire the tested control that the lifecycle runs on
an immutable, publicly verifiable image (`test_mutable_image_reference_is_refused_before_inspection`
pins that behaviour). Both are weakenings and neither was taken.

Decision 0057 fixes the direction instead: resolve the prerequisite package closure on the host
where network use is already an accepted recorded phase, record every file with its SHA-256, mount
them read-only into the container exactly as the AgentMage packages already are, and install them
in a recorded offline bootstrap step before the lifecycle begins. The base image stays immutable
and published, no lifecycle step gains network, and `rpm`/`dpkg` still verify dependencies. That
is the next work unit.

`artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json` is left **stale at its
last passing revision `36e2d4d2`**. It was deliberately not re-bound to current source without a
passing run. Story 9.1 remains open and its independent-review gate remains open and unclaimed.

### Honest status

Product truth remains `scaffolded`. The clean Linux build passing on both platforms is build
reproducibility evidence at one exact revision; it is not platform qualification, not a package
lifecycle result, and not a release.

## Claude Code checkpoint — 2026-09-20 20:15 CDT

Continuing under Decision 0054. Commits `68a83c08` and `297c6e66` complete this block, and the
full `npm run docs:check` was driven end to end for the first time in this run.

### The Apache fixture was relocated, not excepted

`docs:check` reached the RV-51 fixture security scan, which failed with one prohibited item:

```text
ScanFinding(category='remote-reference', path='fixtures/licensing/apache-2.0.txt',
            reason='non-reserved URL')
```

`scripts/fixture_security_scan.py` prohibits non-reserved URLs anywhere under `fixtures/`, and the
Apache-2.0 text legitimately contains `http://www.apache.org/licenses/`. The scan is right. Rather
than adding an exception to a security prohibition, the fixture was moved beside the crate whose
tests consume it, at `capabilities/repository-map/tests/apache-2.0-license.txt`; the scanner covers
`fixtures/**` and `artifacts/sprints/sprint-2/**` only. Its SHA-256 still matches the pinned
`APACHE_2_LICENSE_SHA256` exactly and the 48 repository-map tests still pass. Decision 0056 was
corrected to record the final location and why it is not under `fixtures/`.

That relocation flipped the crate tree hash again, so the SBOM, the real-model demo acceptance,
and the dependent cascade were all renewed a second time. The demo suite passed again end to end.

### The last two long-standing red gates are now green

`story-3.2:gate` and `sprint-3:gate` — open since the host's OpenSSL moved 3.5.7 to 3.5.8 — are
resolved. Under the delegation's authority over how blockers are resolved, the
`patch-verification-results` binding in `fixtures/support/vulnerability-workflow/workflow.valid.json`
was advanced from `520f7f49…` to the regenerated report's actual digest `fa2304f8…`. That keeps the
binding exact and is the same class of routine maintenance as an AGENTS.md §7 pin advance; it is
not a weakening, which would mean removing the binding or making it inexact. The Story 3.2 security
map and vulnerability workflow were rebuilt, and both gates' review pins were then advanced from
`a2221e5ef8` to `68a83c08` under §7, with no external human review claimed.

Story 2.4's gate was also renewed after the RV-51 scan report changed.

### `docs:check` end-to-end status

The chain now runs from `docs:lint` through the Sprint 3 gates and stops at exactly one remaining
item, which is an **environment condition rather than a repository defect**:

```text
Path platform conformance failed: conformance command failed: podman
```

`scripts/path_platform_conformance.py` bind-mounts the host's `~/.cargo/registry` read-only into a
container running as `--user=10001:10001` and copies it into a tmpfs. Two crates extracted onto
this host today — `fnv-1.0.7` and `pin-utils-0.1.0` — carry upstream `0640` file modes, which cargo
preserves, so uid 10001 inside the user namespace cannot read them:

```text
cp: cannot open '/registry/src/index.crates.io-.../fnv-1.0.7/lib.rs' for reading: Permission denied
```

Measured: exactly **19 of 58,067** files in that registry lack world-read, all belonging to those
two crates, with 2020 upstream mtimes inside directories created today. Nothing in this repository
causes it and nothing in this repository can fix it without changing container controls, which
would be a weakening.

**It was deliberately not fixed**, because the only remedy is a permission change to files outside
this repository, and the standing delegation retains "nothing outside this repository" as a
prohibition. The exact remedy, for whoever holds that authority, is:

```sh
find ~/.cargo/registry/src -type f ! -perm -o=r -exec chmod o+r {} +
```

That touches 19 public open-source source files in a local build cache and changes no repository
content. After it, `evidence:story6.1-platform-conformance:check` should run and `docs:check`
should complete.

### Honest status

Product truth remains `scaffolded`. The clean Linux build passing on both platforms is build
reproducibility evidence at one exact revision; it is not platform qualification, not a package
lifecycle result, and not a release. Story 9.1 remains open with its independent-review gate
unclaimed, and the package-lifecycle work is directed by Decision 0057.

## Claude Code checkpoint — 2026-09-20 22:30 CDT

Continuing under Decision 0054. This block released the GPU, drove `docs:check` deeper, and
disproved Decision 0057's design by testing it.

### GPU released, and the practice made durable

`scripts/demo_smoke.py` deliberately leaves the demo running, and the resident Muse Glimmer 30B
held **16,735 MiB** of the 24,564 MiB GPU. The demo was stopped and the GPU fell to **1,047 MiB**,
the desktop baseline, with no `llama-server` process remaining. `docs/LOCAL-TESTING.md` now carries
the stop-and-verify step so the release is part of the documented acceptance procedure rather than
something a reader has to remember:

```sh
python3 scripts/demo.py stop
nvidia-smi --query-gpu=memory.used --format=csv
```

From here, every acceptance run in this repository ends with that release.

### The registry fix was applied, and unmasked a second cause — Decision 0058

The host permission fix landed: zero files under `~/.cargo/registry/src` now lack other-read.
The conformance check still failed, for a different reason the permission error had been hiding:

```text
error: failed to write to `/tmp/target/debug/deps/.../full.rmeta`: No space left on device
```

The container copied the **entire** host registry — 1.7 GB on this host — into a 2 GB tmpfs that
also counts against its 2 GB memory limit, leaving roughly 300 MB to compile in. The check was
degrading as a function of unrelated host cache growth.

Decision 0058 gives the container only what an offline locked build needs: the registry index
(58 MB) and the crate cache (194 MB), letting Cargo extract the subset the lockfile resolves,
instead of 1.4 GB of pre-extracted sources it does not need. Measured under the unchanged
controls: **142 tests pass** with tmpfs at 845 MB of 2 GB — 1.2 GB of headroom. No memory, tmpfs,
pids or timeout limit was raised; the image, network isolation, capability set, user and test
command are byte-identical.

### Decision 0057 was implemented far enough to test, and is disproven — Decision 0059

The prerequisite-mounting design was taken to the point of real experiments against the pinned
images, and it does not work:

1. Resolution succeeds — a 48-package, 38 MB closure for `bubblewrap git-core systemd`.
2. `rpm -i` of that closure fails: `systemd-libs < 259.9 conflicts with systemd-shared-259.9`.
3. `rpm -U` fails with exit 29 — `erase skipped` for `glibc`, `openssl-libs` and others — because
   upgrading needs capabilities that `--cap-drop=all` denies. `bwrap`, `git` and `systemctl` were
   all still absent afterwards.
4. Narrowing to `bubblewrap git-core` alone does not help: it still pulls `util-linux-core-2.41.5`,
   which file-conflicts with the image's installed `2.41.4`.

The root cause is structural: the base image is **deliberately immutable and therefore older than
the repositories** any current closure resolves from, so every closure is an upgrade, and upgrades
are impossible without capabilities. This also explains the history — the lifecycle last passed
when the package required only `glibc, openssl-libs`, both already in the image.

Decision 0059 supersedes 0057's resolution, refuses `--nodeps`, refuses granting the container
package-management capabilities, refuses substituting a locally built image for the immutable
published one, and records that dependency-satisfied installation is **already demonstrated** by
the Decision 0040 installed-platform lane —
`artifacts/sprints/sprint-16/installed-linux-worker-matrix.json` passes in strict-offline native
Fedora 44 and Ubuntu 26.04 KVM guests under QEMU 10.2.2, where a real operating system provides
both the prerequisites and the privileges a package manager needs. Decision 0057 keeps its finding
and its two rejected options, with a superseded note appended rather than a rewrite.

`artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json` stays **stale at
`36e2d4d2`**. It was not re-bound, not regenerated, and not claimed, and Story 9.1's
container-lifecycle element stays open and explicitly blocked.

### Gate-pin cascade from the renewed conformance evidence

Renewing the conformance artifact moved four gate-owned pins, each advanced under AGENTS.md §7
with no external human review claimed: the Story 6.1 path boundary review, the Story 6.1 security
evidence map, the Story 6.1 gate (`66b9cd428a72` → `923d891802`), and the Sprint 6 gate
(`96172b2279` → `4525fc01b5`). Each was committed before the pin that references it advanced, so
no pin points at content that is not in history.

### Demo checklist

`docs/DEMO-PROGRESS.md` previously recorded the five Apache scaffold fixture failures as
outstanding. They are resolved, so the document now records that, the passing clean build, both
real-model re-acceptance runs, and the GPU release. The demo itself remains complete and
re-verified; the outstanding work named there is no longer demo work.

### The same defect existed in a second generator, and the pin cascade continued

Driving `docs:check` further surfaced `platform_manifest_artifact.py` failing the same way, with
the identical `cp -a /registry /tmp/cargo/registry` into the same 2 GB tmpfs. Decision 0058 was
extended to cover it; a repository-wide search confirms those were the only two occurrences. The
Story 7.1 platform contract then rebuilt cleanly: "Sprint 7 platform contract passed on every
available non-macOS contract target".

Renewing it moved four further gate-owned pins, each advanced under AGENTS.md §7 and each
committed after the content it references, so no pin points at content absent from history:
the Story 7.1 security evidence map, the Story 7.1 gate (`22938cc843e5` → `be1450044b50`), and the
Sprint 7 gate (`2728de0d51f0` → `66e362da51be`). No external human review is claimed by any of
them.

Counting this block, nine gate-owned pins and evidence maps were renewed as the consequence of one
generator fix. That is the documented cascade behaviour, not drift: each aggregate pins the exact
artifacts beneath it, so renewing a leaf necessarily walks upward.

### `npm run docs:check` passes end to end

After the cascade above, the full documentation and evidence gate completes:

```text
DOCS_CHECK_EXIT=0
```

272 script invocations in the chain, zero occurrences of "failed" or "error" anywhere in the log,
clean worktree at `2b14a9c9`. This is the first end-to-end pass in this run. It was inherited red
on two counts at `bffdc412` — `docs:lint` and `requirements:coverage` — and every subsequent
failure it surfaced was diagnosed to root cause and repaired without weakening a control,
relaxing a threshold, or re-binding evidence to a run that did not happen.

The GPU remains released at the desktop baseline after the acceptance runs.

### Honest status

Product truth remains `scaffolded`. A green `docs:check` means the repository's documentation,
contract, supply-chain and gate-owned evidence are internally consistent at this revision. It is
not platform qualification, not a package-lifecycle result, not an independent review, and not a
release. Story 9.1's container-lifecycle element remains open and explicitly blocked by Decision
0059, and its independent-review gate remains open and unclaimed.

## Claude Code checkpoint — 2026-09-20 23:40 CDT

### Sub-task 21.1.3.5 — host prerequisite cleared, blocker corrected to its real owner

The stale host-change note recorded against this row named three commands. All were executed on
this host and all passed:

- the previously ignored `worker_receives_only_the_fixed_environment_and_no_network` test —
  1 passed, 0 failed;
- `strict-local-source:check` — "passed with zero undeclared network paths";
- both repository-owned runtime boundary reviews;
- `evidence:sprint21:build`, now recording `local_contract_passed: true` at commit `e29dbf7f`.

The host prerequisite is therefore genuinely satisfied, not merely reclassified. But the row is
**not closed**, because the generator states its own remaining blocker:

```text
blockers = [{"code": "INDEPENDENT-SPRINT-21-REVIEW-NOT-RETAINED", "owner": "21.1.3.5"}]
```

That blocker is an independent review of the classification, citation-resolver, receipt-chain and
recomputation evidence. This agent cannot supply its own independent review and does not claim
one, so the row was corrected from `local` to **`external`** against an independent review venue,
with the evidence of what now passes recorded in the row itself. The register moved to 47 local,
1,057 dependency, 161 external, 309 unknown.

This is the outcome the earlier classification could not see: the host blocker and the real
blocker were two different things stacked on one row, and clearing the first exposed the second.

### Honest status

Product truth remains `scaffolded`. Nothing in this block closed a checklist row. `docs:check`
remains green after the TASKS.md edit and its contract-boundary cascade.

### Sub-task 14.1.3.3 — the register's selected row was not actually ready

With `21.1.3.5` corrected, the register selected `14.1.3.3` as the next dependency-ready local
row. Reading the implementation before starting it showed it is not ready, for a reason its own
text does not state: **no process performs a download or import.**

- `execute_model_installer` admits exactly `self-check` and `preflight-stdin`, returning
  `ModelInstallerProcessError::OperationUnavailable` otherwise.
- Its self-check descriptor declares `"normal_operation": false`, `"network_authority": false`,
  `"activation_authority": false`, `"one_shot": true`.
- `download_model_artifact` is called only from unit tests inside `model_download.rs`.

The existing interruption coverage is in-process and simulated, which is exactly why the row says
actual process termination remains open. Producing that evidence would require first admitting
download and import operations through the installer executable — widening an authority the
product deliberately withholds and that Decision 0052 keeps behind separate consent.

Decision 0060 records this. The row is corrected from `local` to `dependency`, and that correction
explicitly supersedes **this session's own earlier classification**, which was made from the row's
prose before the process boundary was read. Nothing was granted, stubbed, or pre-implemented: no
operation was admitted, no authority flag changed, no test weakened.

Register after the correction: 46 local, 1,058 dependency, 161 external, 309 unknown, selecting
`execute-local:14.1.3.4`.

### A note on the blocker-truth method

Two rows this session had previously classified as `local` turned out, on reading the code and
running the commands, to be blocked on something their prose did not name — `21.1.3.5` on an
independent review, `14.1.3.3` on an inactive process boundary. Prose-level classification is a
starting point, not a verdict; the classification is only trustworthy once the named work has been
attempted or the implementation read. Both corrections are recorded as supersessions rather than
silent edits.

## Claude Code checkpoint — 2026-09-21 00:30 CDT

A background-task notification reported "failed with exit code 1" for a `docs:check` waiter. It
was benign and was verified rather than reported as a regression: the waiter's last command was
`grep -c`, which exits 1 when it finds **zero** matches, so the non-zero status was caused by there
being no failures. The real task recorded `DOCS_CHECK_EXIT=0`.

### Two more rows were not what their prose said, and the generator agrees

Continuing down the register, the next two selected rows were both read against the implementation
before any work started, and both turned out to be blocked on something their text never named.

- **14.1.3.3** — no process performs a download or import, so actual process termination cannot be
  observed. Recorded in Decision 0060.
- **14.1.3.4** — "runs installer and operational host concurrently" requires a host that actually
  holds workspace, session, tool, grant and inference authority. The host refuses to become
  operational: `agentmage-host --bootstrap-linux` exits `agentmage.bootstrap.package_untrusted`
  because `/etc/agentmage/release/package-manifest.ed25519` and
  `/etc/agentmage/trust/package-signing-ed25519.pub` are both absent. Those are the exact
  production signing key and independently distributed trust root already recorded as external
  under Sub-task 14.1.2.1, so the row now depends on it. With no arguments the binary only
  composes components and exits, which is not an operational host.

Both findings are **independently corroborated by the repository's own Sprint 14 generator**,
which records these blockers without any input from this session:

```text
INSTALLER-EFFECT-PROTOCOL-INACTIVE (14.1)  local import, bounded download, activation, rollback
                                           and cleanup are not yet wired to a closed end-user
                                           process protocol
PRODUCTION-SIGNING-NOT-AVAILABLE  (14.1)   no approved external production signing identity exists
```

That corroboration matters: these corrections are not this agent's opinion about the roadmap, they
are the same facts the evidence generator already publishes.

### 14.2.1.1 — named commands executed, closure deliberately not claimed

The host blocker on this row was cleared by **execution**, not reclassification:
`evidence:story9.2-docker-prerequisite:build` and `scripts/sprint_14_evidence.py --write` both ran
successfully at `8a5bc21e` and `3ba8b747`. The generator now reports
`source_population_reconciled: true`, `role_matrix_reconciled: true`, and status
`PASS-SOURCE-INVENTORY-BLOCKED-EXACT-ARTIFACT-PREFLIGHT`.

The checkbox is **left open on purpose**, and the reason is recorded in the row itself: the
generator summary attests a reconciled source inventory, but this agent did not individually verify
each of the eight artifact classes the row enumerates, and a checkbox asserts the whole row. The
remaining Sprint 14 blocker owned by this story is `EXACT-ARTIFACT-PROFILES-NOT-ADMITTED`, which is
separate artifact-admission work.

### Next genuinely local row

Story 14.2's generator reports `reference_machine_preflight_complete: false`, which is Sub-task
**14.2.2.2** — the non-acquiring architecture, runtime, format, acceleration, disk, memory,
context, modality and expected-working-set preflight against each declared reference-machine
envelope. It acquires nothing and activates no model, so unlike 14.1.3.3 and 14.1.3.4 it is
genuinely executable here. That is the next implementation target.

Register: 45 local, 1,059 dependency, 161 external, 309 unknown.

### Honest status

Product truth remains `scaffolded`. No checkbox was flipped in this block. Three rows this session
had been classified `local` from prose and were corrected once the code was read or the commands
were run — `21.1.3.5`, `14.1.3.3` and `14.1.3.4`. Each correction is recorded as a supersession of
this session's own earlier judgement.

## RESUME NOTE — how a fresh session picks this up (2026-09-21)

Read this section first, then AGENTS.md (all seven sections), then Decisions 0053, 0054 and 0052.

**Authority.** Decision 0054 is a standing owner delegation: decide everything yourself, never stop
to ask, never end a turn to request confirmation. Mark anything you decide "Accepted under owner
delegation, 2026-09-20" and cite 0054. Decisions 0055 through 0060 were all taken under it.

**Retained prohibitions, unchanged by the delegation.** No spending, accounts, credentials,
publishing, releases, or merges to a default branch. No force-push. No weakening of tests,
thresholds or evidence bindings. No licence or trademark choices. Nothing outside this repository;
USTE, CodingMage and AgentMagik are off limits. **Never supply an independent review and never tick
an approval that belongs to a reviewer or the owner.**

**Owner rule added 2026-09-21: do not close a row on a generator's summary alone.** Verify the
row's enumerated items individually, close only if every one holds, and record the evidence in the
row itself.

**Resources.** Start a build whenever `free -h` shows ≥16 GB available; do not wait for other
repositories' Cargo work. Keep every build, test and evidence regeneration inside
`systemd-run --user --scope -p MemoryHigh=5G -p MemoryMax=6G -p MemorySwapMax=512M`, and use up to
4 Cargo jobs. Model-serving workloads get a scope sized to the model (20G/24G/0) instead.
**Release the GPU after every acceptance run**: `python3 scripts/demo.py stop`, then confirm with
`nvidia-smi`. `scripts/demo_smoke.py` deliberately leaves the demo running.

**State at this note.** Branch `demo/fedora-local-docs`, HEAD `bbe519a7`, clean tree, nothing
unpushed. `npm run docs:check` passes end to end (272 scripts, exit 0). The clean Linux build
passes on `fedora-x86_64` and `ubuntu-x86_64`, 11/11 commands each. Register: 45 local, 1,059
dependency, 161 external, 309 unknown.

**Cascade you will hit.** `TASKS.md` is a hashed input of `contract_boundary_gate.py`, so any
TASKS.md edit requires `npm run traceability:build`, `contract-boundary:build` and
`contract-evidence:build` before the commit gate passes. Any edit inside a Cargo workspace member
flips the SBOM, which the demo acceptance reports bind, which means re-running
`scripts/demo_smoke.py` and then the Story 1.2 / 3.1 cascade. Batch source edits and regenerate
once (AGENTS.md §1 and §3).

**Open blockers, each already diagnosed — do not re-litigate them.**

- Story 9.1 container package lifecycle — Decision 0059. Immutable base image predates the repos a
  closure resolves from, so every closure is an upgrade and upgrades need capabilities
  `--cap-drop=all` denies. Dependency-satisfied install already passes in the Decision 0040 KVM lane.
- `14.1.3.3` — Decision 0060. No process performs a download or import.
- `14.1.3.4` — depends on `14.1.2.1`; the host will not bootstrap without the production package
  signature and trust root.
- `21.1.3.5` — host prerequisite cleared by execution; remaining blocker is an independent review.

**Next actions, in order.** (1) `14.2.1.1`: verify the eight enumerated artifact classes
individually and close only if all eight hold. (2) `14.2.2.2`: the Sprint 14 generator reports
`reference_machine_preflight_complete: false`; it is non-acquiring and executable here. (3) Continue
through every row that is executable on this machine, reading the implementation before trusting a
row's prose — three rows this session read as `local` and were not.

## Claude Code checkpoint — 2026-09-21 02:15 CDT

Owner rule added and now in force: **do not close a row on a generator's summary alone.** Verify
the row's enumerated items individually, close only if every one holds, and record the evidence in
the row. Never supply an independent review and never tick an approval belonging to a reviewer or
the owner.

### 14.2.1.1 closed on individually verified evidence

All eight artifact classes were checked across **all 416 frozen entries** in
`model-profiles/catalogs/2026-08-14/`, not from a summary field:

| Class | Result |
|---|---|
| Meta Muse catalog | boundary declared, 4 repositories, matches `counts.meta` |
| Google Gemma catalog | boundary declared, 412 repositories, 33 selected collections |
| Retrieval identities | 36 sources with `id`/`url`/64-hex `sha256`; 416 `immutable_source_url` pinned to revision |
| License/use-term sources | 413 resolved; 3 carry explicit `license-use-terms-unresolved` and are `BLOCKED` |
| Model cards | 416/416 `model_card_url` pinned to the immutable revision |
| Artifact listings | 416/416 listed and 416/416 digest-bound |
| Runtime documentation | 416/416 populated |
| Eligibility policy | present in both snapshot boundary and inventory policy |

The licence class is the one a summary would have hidden: three entries have no licence value, and
checking them individually showed each carries an explicit `license-use-terms-unresolved` blocker
because the upstream metadata genuinely has no licence tag. The freeze records the gap instead of
inventing a value, which is the correct behaviour and is why the class passes.

Both files record `frozen_on: 2026-08-14`, the digest chain holds
(`inventory.source_snapshot_sha256` equals `snapshot.snapshot_sha256`, matrix bound to
`inventory_sha256`), `model_candidate_inventory.py` reports "valid (416 exact source entries; zero
acquisition authority)", and `product_state` records 0 enabled models and 0 authorized
acquisitions. **Task 14.2.1** was then closed on the same basis: its host blocker is cleared by
execution and all four sub-tasks are complete. No reviewer or owner approval is claimed by either
closure.

### Two more rows corrected, and the local queue is now consistent

- **14.2.2.2** — all 416 entries record `hardware_preflight: BLOCKED` with reason
  `exact-artifact-size-working-set-and-runtime-envelope-required`, `acquisition_allowed` is false
  for every one, and `reference_machine_preflight_complete` is a **hardcoded `False`** in
  `sprint_14_evidence.py`. Corrected from `local` to blocked on exact artifact admission.
- **15.1.3.3** — the Sprint 15 generator records `os_worker_resource_enforcement: false` and the
  blockers `RESOURCE-STOP-NOT-WIRED-TO-OS-WORKER` and `NO-ADMITTED-PRODUCT-MODEL`. Corrected to
  blocked on those. Its purely local halves already pass and are retained.

That makes **five** rows this session that read `local` in prose and were not — `21.1.3.5`,
`14.1.3.3`, `14.1.3.4`, `14.2.2.2`, `15.1.3.3`. Each correction supersedes this session's own
earlier judgement and names the evidence.

**A completeness check was then run over the whole local queue**: for each of the 42 remaining
local rows, its own sprint's `local-evidence-report.json` was searched for a blocker whose owner is
that row or an ancestor. All 155 sprint reports were found, so the check is not vacuous, and
**no remaining local row is named by such a blocker**. The five corrected rows were exactly the
disagreements. This is a necessary check, not a sufficient one: it proves no generator contradicts
the queue, not that every row is implementable.

### Selector observation, recorded not acted on

Sub-task rows do not inherit their story's declared `**Dependencies:**` block, because
`_structural_dependencies` gives subtasks no structural edges. Story 76.2 declares dependencies on
Stories 22.5 and 76.1, yet `76.2.1.1` is offered as a dependency-free local leaf. Decision 0052
item 6 says the selector may pick only a leaf whose exact dependencies are satisfied, so this is a
gap. It was **not** patched here: propagating story dependencies to leaves would reclassify a large
number of rows at once and deserves a deliberate decision rather than a late edit.

For `76.2.1.1` specifically the dependency is substantially met: Story 22.5 is closed, and Story
76.1's implementation tasks 76.1.1 and 76.1.2 are complete with only Sub-task 76.1.3.4 open, which
is the external native-desktop security evidence that cannot close on this machine. Story 76.2 also
carves out model acquisition as non-blocking. So `76.2.1.1` — add the standalone application
process and authenticated local bridge over the Rust host — is the genuine next implementation
target, and it is a feature build rather than an evidence renewal.

### Honest status

Product truth remains `scaffolded`. Two rows were closed this session, both on individually
verified evidence, and neither claims platform qualification, independent review, or release.

## Operator Stop and Coding-Harness Replan - 2026-09-21

The owner stopped the AgentMage worker and directed a standalone coding-harness
architecture revision using OpenCode as the workflow reference. The Claude
process, four polling shells and `agentmage-claude` tmux session were stopped;
`~/.local/share/agentmage-run/STOP-CLAUDE` remains present. Other stack workers
were not interrupted.

Decision 0061 supersedes the earlier next-action instruction to start 76.2.1.1
for this workstream. The new entry is Sub-task 48.2.4.1, followed by the actual
host/transport/factory/CLI work, live control and exact-model executable proof.
The complete design and restart rules are in
[the coding-harness handoff](coding-harness-replan-2026-09-21.md).
Do not restart automatically or remove the stop marker based on this note.

All prior checkpoints remain historical evidence. This revision does not close
implementation, model, platform, independent-review or release gates.
