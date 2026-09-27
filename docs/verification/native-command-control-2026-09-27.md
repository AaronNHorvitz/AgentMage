# Native Command Control and Evidence Freshness

Scope: bounded AMR-01/AMR-02 prerequisites, not package completion.
Status: Accepted under owner delegation, 2026-09-20. Authority: Decision 0054.
Independent acceptance is not supplied by that engineering status.

## Implementation and reproduced corrections

The existing Linux supervisor now owns read, registered-command/validation and
read-only Git attempts through a closed manifest set and one retained attempt slot.
Command/Git inspection no longer use separate blocking reader/control loops.
Original deadlines, exact executable/workspace descriptors, offline confinement,
output limits and consumed authority remain mandatory. Every cancellation poll
validates the consumed task/correlation scope. Invalid or unavailable control stops
execution; known-clean failures withhold result payloads, while uncertain cleanup
takes priority and quarantines same-process admission. This is not crash-safe
worker ownership across hosts.

The native filter-release handshake exposed an stdin regression: Bubblewrap closed
fd0 after consuming its seccomp filter. The unchanged descriptor-table test failed.
A separate held pipe now delivers the same atomic filter only after exact running
invocation/cgroup checks, preserving null stdin and the original descriptor contract.
The read end remains retained with uncertain ownership. Tests cover withheld bytes
and EOF, CLOEXEC, exact release, non-replay and the closed owner's unit-name and
projection-cardinality rules. No second process owner or reaper was introduced.

Bounded nonblocking cleanup drains observed output through EOF. Forced launcher
termination or a sticky read failure cannot establish complete output. Full observed
stream hashes remain distinct from retained prefixes; overflow remains a failure.
Resource observations use bounded reads through the already-held cgroup. Unavailable
counters do not invent zero usage. Sampling is not continuous peak certification.

An actual CLI diagnostic reproduced lost small structured cancelled-command
metadata: the coordinator routed it inline and discarded it on terminalization,
although the consumed authority receipt and raw stderr artifact survived.
The existing coordinator now retains valid non-success metadata through its existing
artifact owner, within unchanged output, disk and count budgets. These audit
artifacts never become successful completion evidence. Invalid-control and uncertain
projections still supply no result payload/artifacts. Native tools, approval and
verifier ownership are unchanged.

The structural effect audit inventories the exact crate-internal supervisor API,
three existing manifest variants, single attempt slot and enumerated consumers.
The child accounting helper cannot own processes or sockets. Negative tests reject
public exports, extra owners/callers/fields and accounting launch APIs. This is not
public raw-launch authority or an exemption from kernel permit checks.

## Verification and exact source identity

The source parent is `f38f9f85fdbc02c1f7d37b0b89e5fb2bef2ba491`.
The final Rust/native/scripted run retained tracked diff SHA-256
`9a63982138ca3a48e7ffe93790b929c8356b9268d5a1ae274c7393dfaa49c7ce`
and separate new resource-helper SHA-256
`e481beb61540b836df3e95cb4524f2c4c885d54401e7b2135f8357365eeafa52`.
Only the developer evidence schema-type guard and publication/evidence documents
changed afterward; production Rust remained unchanged. The commit containing this
record provides the complete source, including that helper. Private logs retain
per-file identities, exact commands, failed attempts and disposable workspaces.

| Check | Inspected result |
|---|---|
| Engine library | 1160 passed, 7 explicitly ignored |
| Host library | 323 passed, 8 explicitly ignored |
| Linux library | 180 passed, 50 explicitly ignored |
| Engine integration binaries | 67 passed |
| Engine documentation tests | 8 passed |
| Optional research-worker binary | 20 passed; not native research admission |
| Focused runtime-loop audit | 62 passed; overlaps the engine total |
| Final evidence freshness tests | 17 passed, including exact schema-type refusal |
| Python CLI/harness/effect/native-evidence suite | 98 passed before the final schema-type-only edit |
| Feature-enabled strict Clippy | Engine/Linux/host/read-only, all targets, passed |
| Actual-binary scripted matrix | 22 cases passed; not real-model qualification |

The broad job did **not** exit successfully: after the passing component suites and
four native confinement tests, an existing stale-worktree fixture failed under
umask 077. It had set mode 0700 assuming the initial mode differed; under that umask
there was no identity change, and the authorized unchanged command correctly ran.
The fixture now toggles an observed permission bit and asserts the actual change.
Original no-launch assertions remain. Both umask 022 and 077 cases subsequently
passed, as did hostile-descendant cleanup. Production implementation was unchanged.
The separate remainder job finished with numerical exit 0; no failed broad-job
status was rewritten and no ignored test is counted as executed.

Separately selected native tests passed the original descriptor contract, printf,
timeout/cancellation/overflow, ten Git inspection operations, hostile Git behavior,
read lifecycle and late borrowed-command cancellation/observer failures. Each of
the latter three cases was followed by a successful native reuse. Exact logged
units were checked absent/inactive with zero PIDs and no cgroup after cleanup;
no unrelated process was inspected or signalled.

The actual CLI/host/private authenticated IPC diagnostic cancelled both command and
validation after native body entry. It checked full stderr and structured cancelled
metadata artifacts, consumed transaction/grant/receipt identities, terminal event
ordering and two canonical-owner reopens for each case. A fresh host then completed
an ordinary native failed-test repair. The read-cancellation diagnostic and fresh
repair also passed. Proposals were scripted; these are executable/native control
checks, not a local-model campaign or crash-recovery qualification.

The corrected command/validation diagnostic used copied CLI SHA-256
`5680b8dc92e1f7d4ea3dab4a2d4cc6e2d1cd14fe8f6cf60f509642deb8e6ef0d`
and host SHA-256
`5695ac5081ddadd959e28e929d237f001d2512fe034e71b20aee72583f849067`.
The Python CLI tests subsequently rebuilt the default-feature binary pair. The
22-case matrix used CLI
`e3702dba4edcca4eccb6bddc4ee251a9d55545506b3d6d7781d92a23befb4edb`
and host `1771323b0d427aec19528870636b4a68574d9f0a61c4b25e12f850bd50e285e0`.
These are separately identified builds, not interchangeable binary hashes.

All 22 case dispositions and checks were inspected, including expected nonzero
denial, cancellation, exhaustion, malformed-protocol and integrity outcomes.
Each raw stdout/stderr digest and reported terminal state was rechecked against
the retained stream, as were the actual final CLI/host hashes. The report SHA-256
is `7182b94df4aa815e8e15be57e506766ebfef202715b19618f9df7bc72d88bbf8`.
Private transcripts and operator paths are not published.

All heavy work used the shared heavy reservation, at least 16 GiB available RAM
at command start, one Cargo job and one test thread, MemoryHigh 5 GiB, MemoryMax
6 GiB and swap maximum 512 MiB. No inference ran in this source unit.

## Retained failures and run identities

| Retained run | Numerical disposition | Log SHA-256 |
|---|---|---|
| Original native descriptor-table failure | Failed; assertion unchanged in correction | `8e09b6b47ad040ee3d62bce2225e4d6b9d091f0a10efce10b4c47a45fd7ad7b5` |
| Corrected fd/printf/timeout/overflow | 0 | `f250e2d49ba92b5d83e0e956231e36c569cf83ce329fd1d170eada870205bfd9` |
| First CLI cancelled-metadata assertion | 101; validation/reuse not reached | `3421203cb2183fd54ec403894c6bae3875b81c87e9851a2bcae84ba38676ddd4` |
| Corrected command/validation and Clippy | 0 | `1e7975fb65b35ceb858172d79b88789c14c9fa3cd8d3eed087b4c33823cb02ea` |
| First output-budget fixture | 101; per-payload/cumulative fixture mistake | `2d76a363dbb617b83d484576b1158ce74b6b26c4b624e84648170dec99030a6e` |
| Corrected runtime/Python/audit | 0 | `7e8aafecc813482eed5549a61058e14390bd6e2b3ca456c75fb6b3087166795d` |
| Broad suite, then umask fixture failure | 101 | `e60ea0ba145169d4ac769e0b5d4777d1b7630e7e51fa9362d5bb7195dc9c8dfc` |
| Native/Clippy/Python/scripted remainder | 0 | `fdbb9483740ab68358f8e6da5057f5e5e9cb1a7de26e4d1378c7ccde33e0d26a` |
| Final freshness tests and retained inspection | 0 | `e3ced9f2f273c33543c6f48932cc00d00f9860b544525c28b93186ce73d04947` |

Three misspelled filters in the correction driver matched zero tests and count as
no verification. The later correctly filtered 62-test runtime audit covers those
cases. Earlier host cancelled-receipt test-assumption and freshness-tool import
failures also remain retained; none is reclassified as a successful attempt.

## Evidence freshness and remaining gates

Current local Sprint 16 validation previously accepted a missing installed artifact;
a retained regression reproduced that defect. It now requires the actual matrix,
exact artifact summary/hash and compatible committed sources before checks/write.
The original fourteen whole-file bindings remain unchanged. New schema 2 adds every
committed blob's path, mode, size and SHA-256 to a bounded complete-tree closure.
Identical full trees at different commits can match; even documentation-only tree
changes conservatively invalidate reuse. Legacy schema 1 is historically inspectable
but refused as current source proof. No installed matrix was regenerated/reaccepted.

Three native Linux evidence producers additionally bind the whole new resource-helper
file, with missing-child negative tests. Original bindings remain. Old installed,
native and cross-distribution artifacts are historical, not evidence for this change.
The initial direct-binding inspection found 14 newly stale and 97 already historical
bindings to the changed inputs. This scan excludes transitive, unnamed, untracked
and line-span bindings and makes no global currentness claim. Sprint 16/17 installed
evidence and Sprint 41/42/47/50 source/review records need their own new qualifying
execution/review; they are not silently renewed by this implementation record.

The single applicable SBOM/evidence renewal finished with numerical exit 0.
Supply-chain, structural source/module checks, contract boundary and the six
applicable dependency/configuration/security-map builders and tests passed.
The twelve generated files contain only JSON digest changes, dependency-hash
updates and raw-check timing changes; no dependency, license, binding inventory
or acceptance claim was altered. The three changed first-party SBOM components
are the engine, Linux platform and host. Retained renewal log SHA-256:
`271530c3879f118125574e8b2ae72712416e9a310f9a8b6a99b7fc2cba8a914d`.
An earlier driver exited 2 on a misspelled script path before any SBOM write;
its log `e1158331624c5fb47a9372aa1db65c087e64e92a7423ba276d5c5851715e0ca5`
is retained. Only its remaining checks ran after correcting the private driver.

The expanded post-renewal direct-binding scan covers 540 bindings: 23 current,
17 newly stale, 500 previously stale/historical. The additional three newly stale
bindings belong to Sprint 82's committed-blob report. Its bounded `BLOCKED`
disposition needs renewal against this committed SBOM as a remainder of the same
pass, without another SBOM write. The fourteen source/native/review bindings
identified above retain their explicit historical disposition. This is not
blanket historical evidence acceptance or a claim that all G-DOD-11 gates pass.

The [eight-case Muse evidence](coding-harness-campaign15-results-2026-09-27.md)
remains bounded historical development success at `b4417f47`, not fresh model proof
for this source unit. GPT-OSS remains separately unqualified; no unchanged failed
campaign was rerun. Actual-CLI native Git body-entry cancellation, crash-safe
cross-host worker ownership, connected research/provider/model qualification,
independent review, human acceptance, Task 50.2.4.7, M-HARNESS-DAILY, supported
platform and release gates remain open. No AMR checkbox or license state changes.

## Committed-source renewal remainder

Checkpoint `c16884163206ebacfa8885d3f7416e684c44eb7c` contains the inspected
source and single SBOM renewal. Sprint 82's seven registered checks subsequently
ran against that clean committed source, each with exit 0 and zero focused
blocking skips. Its three unit tests, report and supply-chain validators passed;
the outer remainder returned 0. Retained log SHA-256:
`9b7b5b3c787a4787a5439e42607dd431e48e1424559c44af06fa3a2ca6d14b26`.
The [renewed local report](../../artifacts/sprints/sprint-82/local-evidence-report.json)
has SHA-256 `b715f0872a2cb7524ba2069194030dc562a32834c902cb6274dc2e0f5e71f568`.
Only source revision, three supply-chain hashes and three command-output digests
changed. `BLOCKED`, no live public search, no privacy review and no release
approval remain unchanged. This finishes the scoped committed-blob remainder,
not the fourteen historical source/native/review bindings or independent review.
No second SBOM write or historical native/platform reacceptance occurred.
