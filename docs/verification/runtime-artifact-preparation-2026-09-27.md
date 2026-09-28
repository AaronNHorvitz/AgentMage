# Prepared Runtime Artifacts — Component Verification

Batch date: 2026-09-27. Evidence continuation: 2026-09-28.
Source commit: `68fa41577fe217043abc890e879ed562a8661673`,
tree `7cc7b0392c6d12adedf23b44482fc49f5db895dc`.
[Decision 0096](../decisions/0096-runtime-artifact-preparation.md) governs this
AMR-03 prerequisite. The [manifest](runtime-artifact-preparation-2026-09-27.json)
binds complete source inputs and retained observations. This record closes no
native, provider, model, independent-review, human-only or release gate.

## Component behavior

A result can now describe exact artifacts before its terminal hash is sealed.
The existing correctness callback supplies a borrowed preparation builder after
its durable start observation. The trusted host appends complete candidates,
including candidates that refer to earlier prepared members. The coordinator
keeps its existing ordinal sequence, classification, retention and resource ledger.
References are descriptions; preparation publishes no payload, metadata or event.

The first preparation samples one trusted terminal observation time. Prepared
manifests and the terminal event share that sample. It does not describe the later
persistence instant, predict a future time or extend an effect deadline. Every
append retains the same receipt ID and digest. Before sealing, the complete final
candidate sequence must match each prepared kind, media type, byte count and full
content digest. The existing callback/returned-result checks remain mandatory.

The complete set shares the original output, artifact, disk and event ceilings.
A failed append, clock failure, late preparation or repeated sealing stays failed,
even when the host ignores the refusal. Final output must also fit. Preparation
occurs after consumed authority: refusal cannot undo an already attempted effect
or refund its independent research reservation. Failed advancement requires
reconciliation and cannot automatically retry the tool.

After the exact terminal commit, the existing artifact port publishes prepared
members in order, then handles ordinary output. Candidates and output are charged
once. Unprepared operations retain their prior order. Payload bytes still precede
metadata and its publication event; no atomic multi-artifact transaction is claimed.
A partial bundle or missing publication event cannot become complete research
evidence. Canonical readers, receipt checks and lifecycle restrictions are unchanged.

No new store, identifier allocator, effect authorization, network client or native
admission path is introduced. Ordinary tools do not yet construct research bundles
with this API. Every ordinary mode still refuses network operations; the native
adversarial qualification required by Decision 0084 remains open.

## Executed checks

The first source snapshot passed all **76 coordinator tests**, including four new
prepared-artifact tests. Kernel and host strict Clippy passed. Seven further tests
were then added before the complete Rust check. Full changed source bytes are
retained for the first, expanded and final snapshots, with their common base revision.

The complete kernel check passed **1,317 tests**, with **seven ignored**, across
unit, integration and documentation groups. The unit suite contributed 1,237 passes
and seven ignored cases; the other groups contributed 80 passes. All **11 new
preparation tests passed**. They cover dependent references and exact terminal
hashes, one clock sample, complete resource accounting, five exhausted budget
classes, final-output capacity, the 64-candidate ceiling across appends, reversed
or failed clocks, non-success audit output, large-output ordering, 13 substitution
or ignored-refusal cases, absent artifact ownership and two partial-publication
failure windows. Failure cases preserve the advancement barrier and effect count.

Full Linux: **183 passed, 19 failed, 51 ignored**. Full host: **282 passed, 50 failed,
eight ignored**. Both suites failed. All 69 exact names and causes match the preceding
inference-cleanup batch's native Git, executable-manifest and signer prerequisite
refusals, including the related child-exit mismatch. No case was excluded.
Strict workspace and optional-worker Clippy, 27 Python source/dependency audit tests,
source/module/status/context checks, formatting, decision lint and diff checks passed.

Only evidence-binding Python and documentation changed after the complete Rust
check; all five Rust files match its retained bytes. Ten targeted Python checks
then passed, including forbidden-owner/transport mutations in the new production
module. Source/module/status/context, two Markdown files, formatting, diff and
private-identity checks passed. Complete retained-report checks require the new
committed campaign and are recorded in the evidence stage below.

The clean source rebuilt the real CLI, host and read worker in 11.07 seconds.
CLI help, setup, diagnosis and status passed. The explicitly approved scripted
fail/repair launch in a fresh clean repository returned exit 5 before IPC, events,
tools or prompts. Stderr retained `linux.repository.git_artifact.invalid` followed
by `linux.development.launch-envelope.failed`. A second fixture containing staged,
unstaged and untracked work was refused by wrapper preflight, exit 1, before CLI
launch. Both complete file/mode/HEAD/raw-index/index-entry/status/diff snapshots
were unchanged. Executable hashes and sizes were unchanged across the observations.
These are actual executable prerequisite refusals, not a successful coding workflow,
protected denial or active cancellation.

## Evidence and remaining acceptance

After the complete Rust batch, the full SBOM was regenerated **once**. Only the
kernel and host component content hashes changed. The original full Markdown gate
passed 569 files after the entire ignored generated rustdoc cache was preserved
outside the checkout with complete hash, size, mode and link-target checks. No
source, license, lint configuration or exclusion changed.

The foundation/schema stage passed 204 Python tests and 52 schema tests,
then stopped at the stale Story 11.2 automated review pin. Its failure is retained.
The exact reviewed drift was the changed coordinator and its regenerated AC2 report.
Under AGENTS section 7, the pin advanced to the commit containing those changes;
the extracted production helper was added to its full reviewed set. The continuation
passed eight gate tests, two targeted index tests and six runtime-report tests. No
SBOM rewrite or acceptance substitution occurred.

The runtime campaign passed **nine commands and 41 Rust cases**, including all
11 preparation cases. All prior eight commands, cases and time/RSS/performance
ceilings remain intact. Its full source set grows from 16 to 19 and coverage from
22 to 23. The coordinator report retains all 12 automated checks over both production
modules; its complete source set grows from 21 to 24. Boundary and retained
Story 4/Sprint 4 checks passed, with 22 Python tests in that stage.
Story 4/Sprint 4 retain their unchanged review pins and platform blockers.

Security and index checks passed, including 9 Python tests.
The security map preserves all prior 27 inputs and adds three; the index preserves
all 37 and adds three. The index mutation test verifies that changing any added
helper, test or adapted-host file invalidates its full-byte binding. These derived
binding additions followed the source run and do not change the tested Rust bytes.

| Evidence | Exact source revision |
| --- | --- |
| Runtime campaign and Story 11.2 automated pin | `00efcbee0a9e5410457782b62ed7d63c1a9795fc` |
| Automated coordinator boundary | `274127adc1828d99614972b6e67718709445527b` |
| Security map and evidence index | `426496fd782137e1f13d069af5c563b2474e4a5f` |

The boundary report was committed before the security map read its Git-bound inputs.
Automated source review is separate from the requested external independent review.
The index still reports 16 complete source mappings and one partial mapping, with
story, sprint and release completion false. This batch changes no task checkbox.

Before adding this new summary manifest, the direct-binding inventory found
79 current bindings, 119 newly stale bindings and
500 previously stale or historical bindings to changed tracked inputs.
The newly stale bindings are in prior point-in-time verification records; their original
executions and complete source pins remain preserved. These counts exclude transitive,
unnamed and line-span bindings and do not establish global evidence freshness.

The [local testing guide](../LOCAL-TESTING.md) retains actual build and launch
commands and the native trust/user-manager limitations. The requested positive
native edit/test/repair, protected denial, active cancellation and successful
coexistence with pre-existing work remain unverified in this lane. These synthetic
coordinator checks do not establish canonical native research, producer admission,
model qualification or a usable Linux coding demo. No model/GPU process, manual-user
acceptance, independent PASS or release approval is claimed. Historical observations
keep their exact original source pins; the full accepted roadmap remains in scope.
