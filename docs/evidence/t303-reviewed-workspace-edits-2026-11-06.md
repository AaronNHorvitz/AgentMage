# T-303/T-306: reviewed workspace edits

This is a non-closing implementation batch on top of the independently accepted
[workspace source tools](../reviews/b8ed4625ce71-local-coding-workspace-sources.txt),
exact commit `b8ed4625ce71bd78a54c6036c348211851cbf907`. That separate core PASS
is retained byte for byte. T-303/T-306 remain open; no task or finding sub-row is
added and no acceptance, model qualification or release is claimed.

## Connected capability and authority

The [reviewed edit API](../guides/coding-writes.md) connects strict source edits
to complete kernel previews and actual separately approved native transactions.
Ordinary clients can prepare a preview but cannot apply it. Preparation requires
the exact complete source digests from the original context, recomputes the
unchanged deterministic source selection, and reuses the unchanged strict edit
checker. Invalid/out-of-scope answers and stale snapshots are refused. Source,
model output and approval bodies cannot choose a new root or execute a command.

`LinuxCodingPendingWrite` builds an inert kernel shadow change. Each read-parent
scope is an exact selected leaf, and its policy admits only the exact held write
targets; network, credentials and publication remain denied. The plan binds the
task and root identity. No write grant exists until the trusted control route
accepts an explicit decision with the exact workspace revision, change-set hash
and preview hash. Wrong bindings are refused; denial discards the preview without
effects. A valid approval consumes the pending object before dispatch, derives
and consumes one exact kernel grant, and uses the existing
`LinuxAtomicWriteDriver`. Its descriptor-relative identity/preimage checks and
partial-failure reconciliation remain unchanged. No alternate filesystem writer
or executable-trust exception is introduced.

Scope selection/revocation and endpoint settings changes revoke pending previews.
Only one preview exists; a newer valid preparation supersedes it. Lifetime is
two minutes, with epoch and monotonic expiry checks. Native limits are 16 existing
files and 256 KiB combined original/proposed bytes, within the source API's
stricter read and proposal bounds. A kernel transaction verifies exact resulting
bytes and retains file modes; it does not compile or test code. Outcomes preserve
committed/no-change/restored/uncertain distinctions. Transaction errors report
unknown effects conservatively. Repeated approvals cannot redispatch a consumed
preview, including after a native refusal or ambiguous disconnect.

An admitted write transaction completes and reconciles as one unit. Cancellation
is accepted before dispatch, while status and cancel explicitly report that a
dispatched transaction cannot be interrupted midway. Native filesystem I/O is
byte-bounded, not hard timed. Preparation and apply release their operation lease
and record terminal outcomes before replying, avoiding the earlier lease race
on these new routes. The selection install now checks cancellation under the
state lock before changing its grant. Existing read/proposal terminal lease
behavior and snapshot-route validation order remain low backlog.

## Verification

All Rust commands use the pinned offline toolchain and shared build reservation,
two compiler jobs and four test threads. The first native command waited for
another lane's reservation, then compiled in 2.64 seconds. No
other worker or reservation was interrupted and no build bypass was used.

| Check | Observed result |
| --- | --- |
| New native `coding_write` integration target | PASS: 5 methods, no ignored/filtered cases, 0.01 s |
| Complete API `local_chat_api` integration target | PASS: 32 methods, no ignored/filtered cases, 4.33 s |
| Actual API process, all development source contexts | PASS: 30/30 exact matches against both historical model campaigns |
| Actual API process, accepted historical development replies | PASS: 3/3 exact native writes and receipts; other workspace files unchanged |
| External-process driver total | 3.395 s; synthetic upstream and recorded replies, no inference/grading |
| Final changed Markdown, review-file format and whitespace | PASS; five Markdown files, unchanged review retained |
| Documentation policy validator | PASS: 752 Markdown files and policy invariants |
| Builder publication audit | PASS: baseline identities, 211 retained hashes, source/binary bindings, receipt chains, privacy and links |
| Touched crates and dependents: formatting / all-target Clippy, warnings denied | PASS |
| Complete ordinary per-change gate | **FAIL**: 762 passed / 72 failed / 62 existing ignored / 0 filtered; test command 121.18 s |

Native tests demonstrate inert preparation, complete diffs, actual verified
writes with mode retention, stale contents, same-byte inode substitution, wrong
root, wrong approval hashes, expiry, dropped previews, symlink/hardlink refusal
and whole-batch preflight with another person's changed file preserved. API
tests add model-to-preview-to-approved-write sockets, control/client separation,
forged/denied/superseded/revoked approvals, stale whole-context hashes, malicious
paths/fields/JSON shapes, consumed-failure replay refusal and concurrent identical
approvals dispatching exactly one transaction. All 27 previous API methods also
pass. The binary initializes protected credentials and serves actual sockets in
the retained process-level driver.

The ordinary gate selects the two changed crates and their host dependent:
`agentmage-platform-linux`, `agentmage-local-api` and `agentmage-host`. It runs
all their default-feature targets without test filters or new skips. The exact
72 failure identities equal the committed T-301 baseline (53 host and 19 Linux),
with no added or missing identity. Deferred owner-action T-301.1 remains open;
this comparison is not an allowlist or a green full gate. Native trust and all
existing ignored-test declarations remain unchanged. The selected-crate counts
are not the prior batch's whole-workspace counts.

Private logs are under `<lane-state>/tmp/t303-approved-writes/`:
`native-v1.log`, `api-v1.log`, `development-v1.log` and `fast-v1.log`. The private development
driver, requests, source snapshots, events, approvals, exact tested binary,
results, receipt chains and file digests are retained under `development-v1/`.
The builder publication audit is `publication-audit.json`; its script also
recomputes every exported receipt hash chain in Python and binds each replay
to the earlier immutable development campaign manifest. This is a builder check,
not the separate reviewer verdict. Credentials are excluded from the retained
manifest. All 211 retained file
hashes have been rechecked after the process ended.

| Retained artifact | SHA-256 |
| --- | --- |
| `summary.json` | `aae44fd4672c099f662f97938eca3232f2b18925e9996a57c0cf75ea1083c8ed` |
| `retained-manifest.json` | `45810ddb468563967fcfa54d0ee9dc5f87572553ced13786099e0231de89c4d2` |
| Exact tested `agentmage-local-api` | `da9ceb30d60a88beda94bdead3ef96aae8b70c6cf11dd2518d21019c86286eb8` |
| Private development driver | `e39dbadff727b5b7f50b0f66c30fb23be4eb6628faa10e0d3de2c45a824cfcbc` |

## Measurement integrity

The development driver reads only the frozen development definitions and editable
projections. All 30 contexts match the earlier two-model campaign byte for byte
apart from the newly assigned workspace revision. It supplies a synthetic empty
edit answer, which is refused each time, then replays only the three historically
completed checked development replies through the new prepare/approval/write
routes. Each result must equal the original proposal event's candidate bytes and digest;
an unselected sentinel and the complete file set must remain unchanged. No
candidate, grader or hidden assertion is executed.

This is context and write propagation, **not a fresh model score**. No model/GPU
process starts and no model is downloaded. The scoreboard remains historical;
the current model development campaign and held-out run remain required for the
combined row-closing measurement. No frozen suite definition, task statement,
hidden test, source pin, grading rule, prompt, threshold, dependency manifest,
lockfile or model profile changes. Legacy evidence, SBOMs and review pins are not
regenerated or relabeled as current. The long gate stays reserved for T-310.

## Explicit remaining scope and review backlog

These are writes in an explicitly selected workspace, **not a verified isolated
Git worktree**. The preview and response say `isolation_verified: false` and
`tested: false`; their review narrative includes those risks. Native Git trust
checks remain unchanged. Agent-owned worktree creation, project tests, retry
feedback, repository apply, approved branch commits/pushes, durable records,
resume/replay, MCP and task CLI remain open. No production profile is activated.

Previews/grants and the last eight write records exist only in process memory.
A dispatch record precedes effects and terminal receipts precede the reply;
sequence and truncation metadata disclose history eviction. A crash can lose
these records while file writes remain. Recovery requires manual inspection;
there is no automatic replay, rollback or claim of crash-safe audit. Durable
journal integration remains required for parent acceptance. Multi-file writes
are reconciled, not a filesystem-wide atomic transaction.

Prior low campaign grade/candidate binding remains backlog for the row-closing
audit. Future worktree/path authority remains open despite reuse of the native
write driver here. The independent review's remaining low lease/validation
findings stay backlog rather than creating new task rows. This batch requires
its own separate exact-commit core verdict and cannot close either parent.
