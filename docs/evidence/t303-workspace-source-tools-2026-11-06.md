# T-303/T-306: approved workspace source tools

This connected implementation batch follows the exact-commit independent
[snapshot-proposal PASS](../reviews/3aef4160e881-local-coding-snapshot-proposals.txt).
That report is retained unchanged. It approves its implementation batch only;
T-303 and T-306 remain open. No additional task or review-finding row is created.

## Connected behavior

The [workspace source API](../guides/coding-workspace.md) connects actual native
file reads to the existing bounded source selection, credential-owning transport
and strict in-memory proposal checks. Trusted control credentials select an
absolute root and exact relative source allowlist. Ordinary clients can read or
perform bounded literal search within that list. A separate explicit source
disclosure choice permits proposals through the chosen endpoint; any settings
revision invalidates the old disclosure approval. Source text and model output
cannot change the root, allowlist, endpoint, budgets or approval.

The native `LinuxCodingReadScope` reuses `WorkspacePath`,
`authorize_workspace_root` and `LinuxPathAdapter`. It holds the selected root's
descriptor and identity, resolves selected files afresh, reads exact preimages,
then revalidates/re-resolves before returning. It preserves native symlink,
hardlink, special-file and mount-crossing refusals. Case aliases, Git metadata,
ambiguous paths, binary/empty text and excess file/aggregate bytes are refused.
This scope confers no write, command, production platform or release authority.

The HTTP orchestration and its unchanged binary name move into the new crate
`agentmage-local-api` at `platforms/local-api`, which composes existing components.
The inference crate no longer depends on repository parsing or the Linux
adapter. The API crate depends on those existing workspace components and the
inference transport. There is no source duplication, new third-party package,
version, acquisition, licence or model download. The carried-forward inventory
selects reuse of native confined adapters; unrelated platform components are
not activated. Existing production activation, authority and executable trust
checks remain unchanged. The old Rust module path changes to
`agentmage_local_api`; no other repository runtime caller used the former
module. HTTP routes and the `agentmage-local-api` executable name stay stable.

The initial dependency layout put the Linux adapter inside inference. Its debug
unit executable grew to 194,350,920 bytes and correctly failed the existing
128 MiB executable identity bound in
`docker_linux_observer::tests::current_process_is_held_and_revalidated`. The
first full gate therefore has **73 failures**, with that one additional identity;
a complete inference-library repeat reproduced it. These failed logs are
retained. Separating HTTP orchestration fixes the dependency layout without
raising the bound, changing the observer test, stripping its executable or
allowlisting the failure. The separated inference library passes all 175 tests
with its four existing ignored tests, and all 27 API methods pass again.
Its observed debug unit executable is now 35,797,600 bytes.
Selection, read, search and proposal operations share the single-operation
budget. Status/cancellation remain available; scope or settings replacement is
refused until the active operation ends. Activity records workspace/settings
revisions and fixed outcomes without source, root, model answer or credentials.
Records and grants have process-only retention. Native filesystem I/O is bounded
in bytes, not by a hard interruptible deadline; a stalled filesystem may delay a
read. This batch does not establish general command confinement or durable runs.

The proposal path still returns `applied: false`, `tested: false` and
`authority: "proposal-only"`. Isolated worktrees, writes, confined test commands,
failed-test feedback, retries, diff/apply, approved branch commit/push,
durability/resume/replay, MCP and the task CLI remain open mission work. No
parent-row closure or native coding qualification is requested.

## Verification

Rust checks use the pinned offline toolchain and shared build reservation.
Reservation waits are retained in the logs and do not count as executed tests.

| Check | Final observed result |
| --- | --- |
| Native `coding_workspace` integration target | PASS: 6 methods, no ignored/filtered cases, 0.01 s; exercised again in the final full gate |
| API `local_chat_api` integration target in the new crate | PASS: 27 methods, no ignored/filtered cases, 4.37 s on the focused separated-crate run |
| Complete inference library after separation | PASS: 175 methods, 4 existing ignored, zero filtered, 1.01 s on the focused run |
| Whole-workspace formatting and all-target Clippy, warnings denied | PASS in the final ordinary gate |
| Changed Markdown, unchanged review-file format and Git whitespace | PASS, including final publication checks |
| External-process source verification | PASS: 30 actual file selections/reads/searches and 30 exact model-bound contexts, synthetic upstream, 2.857 s |
| Final full per-change gate | **FAIL**: 3,006 passed / 72 failed / 78 existing ignored / 0 filtered; test command 262.21 s including compile/reservation |

The final 72 failure identities equal both the earlier snapshot-batch result
and the committed T-301 baseline: 53 host and 19 Linux cases, no added or missing
identity. The final inference target has no failures. Deferred owner-action
T-301.1 remains open. These are failures, not an allowlist or a green full gate;
no skip, executable ownership change or weakened check was introduced. Initial
failed-layout results remain distinct from this final result.

Private logs: `<lane-state>/tmp/t303-workspace-native-v1.log`,
`t303-workspace-separated-api.log`, `t303-workspace-separated-inference-lib.log`
and `t303-workspace-fast-v2.log`. The publication audit verifies exact baseline
failure identities, all retained raw-file hashes, candidate source input hashes,
the tested binary, unchanged third-party lock entries, untouched frozen
inputs/strict-edit code, Markdown links, review-byte preservation and added-text
privacy. Its receipt is `t303-workspace-publication-audit.json`. The initial
73-failure gate is retained as `t303-workspace-fast-v1.log`; the initial
inference-library repeat also remains retained. No legacy evidence, SBOM or
review pin is regenerated, weakened or relabeled as current acceptance. The
unchanged long gate is reserved for T-310.

## Evaluation integrity and measurement scope

No frozen evaluation definition, task statement, hidden assertion, grading rule,
source pin or threshold changes. The existing strict proposal parser, source
selection and application instruction also remain unchanged. No held-out input
is loaded or run in this non-closing batch. The existing two-model scoreboard
remains historical evidence of its measured commit, not a fresh current score.

The final external-process check uses the actual API executable and all 30
frozen editable sources in temporary selected workspaces with a **synthetic**
upstream. It verifies actual whole-file reads, bounded literal search with
correct byte offsets, unchanged workspace files, and exact serialized model
messages/context against both earlier local-model campaigns. All 30 sources
match all 60 retained historical contexts. Its 30 synthetic upstream requests
complete in 2.857 seconds for the whole driver, including API setup. The stub
returns an invalid empty edit list, which is refused every time. No proposed
source, hidden assertion or grader is executed, and no GPU/model process starts.

This is source-tool verification, not inference or a model score. The full
current development model run and row-closing held-out measurement remain
required when the combined coding workflow closes. No score is inferred from
context equivalence and no new quality scoreboard line is added.

The first driver stopped during task six because it requested the next source
operation before the previous response's cleanup released the lease. Its failed
log and partial outputs remain retained. Polling `/v1/status` for `busy: false`
between operations fixed the driver; the guide now describes that client rule.
An intermediate successful 30-source run used the initial dependency layout and
is superseded by the final run after crate separation. No hidden test or
threshold was changed to obtain these source-tool results.

Final private records are under
`<lane-state>/tmp/t303-workspace/development-context-v3/`. They retain all 30
read/search results, upstream JSON requests, NDJSON streams, source files,
per-case timings, input hashes and the exact tested executable. The summary
binds the driver and source files; the manifest binds 184 retained files,
excluding API credentials. These artifacts remain private and are not published:

| Artifact | SHA-256 |
| --- | --- |
| `summary.json` | `d2fe8fd3903de40cce9ac475c4c5cfa3e4b23c936768efd353c04fba35aa49fe` |
| `retained-manifest.json` | `c98958614ef5e26b20334f1f2255f84cdcfaa5262ac1eddbbeee336887327abc` |
| Tested `agentmage-local-api` | `076761d8e92597f29539d60fd79d267675e4186e11e87f2d5c43c09fd1a113de` |
| `check-development-context-v3.py` | `a8d80816d335e0fe99b5dadcd2950f1f5c503108ec395ce0920f9ea199eb46b4` |

## Review findings carried forward

Prior low L1 has new regressions for the route's 64 KiB answer cap, terminal
refusal/activity outcome and settings PUT/DELETE refusal during an active
proposal. The workspace proposal cancellation case also exercises scope and
settings replacement refusal.

Prior low L2 remains backlog: the row-closing campaign audit must bind each
grade's candidate/task digests to the actual collected bytes. The independent
review verified all 60 historical bindings, so that earlier published score is
unchanged. Its raw audit and immutable manifest are preserved.

Prior low L3 remains a future **write** boundary requirement: every path must be
revalidated through worktree/kernel authority before any apply. This new read
scope adds stricter canonical path checks, but neither its grant nor the
snapshot display-path validator authorizes a write.
