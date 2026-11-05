# T-311.15/T-311.16: Combined Cleanup Repair

The owner refocus in [Decision 0154](../decisions/0154-local-coding-delivery-refocus.md)
ends per-finding evaluation rows. This batch fixes the existing cleanup medium
finding in the same two rows. Neither row closes on builder verification.
Independent exact-commit PASS is required. Parent T-311 stays open.

Work began 2026-11-04; final verification continued on 2026-11-05.

## Change

The [third cleanup review](../reviews/69e083f8f0c7-local-coding-t311-15-descriptor-repair.txt)
found quadratic host time and memory: each deferred sibling retained its complete
path and repair reopened that path from the root. A branching chain built cheaply
by a confined candidate could exhaust the host grader's memory and leave scratch.

Oracle v26 replaces that repair traversal with a depth-first walk. Frames retain
pending sibling names and parent device/inode identities; ascent opens `..` and
verifies its identity before using the parent. At 256 levels, a detached walk
moves directories to shallow numeric names in a private staging sibling inside
the case. Each moved inode is verified before use. Records contain one basename,
parent index and inode identity. Repair restores children before parents; removal
deletes emptied staged directories. The stage is never mounted into a worker.
This bounds bind-mount parent lookup depth as well as eliminating copied paths.
Metadata and directory operations scale linearly with the inspected tree; open
descriptors remain constant with depth.
Frames and names occupy compact primitive buffers, with a cursor into each
directory's names instead of retained Python container graphs. Pending traversal
buffers are cleared in `finally`, including on `MemoryError`,
so an exception traceback does not retain the tree walk during outer recovery.

Directory opens retain `O_NOFOLLOW`, `O_DIRECTORY` and `O_CLOEXEC`. Permission
changes address only pinned directory inodes through the host's own descriptor.
Regular files, hard-link targets and symlink targets are never chmod targets.
Postorder removal retains its parent and child inode checks. Concurrent moves
can refuse cleanup; no synchronous descendant-quiescence claim is made.

## Verification

Raw logs and machine-readable command results are retained privately. The review
request provides their location and fresh-output commands. The tests cover:

- Counted directory operations on two branching depths, checking an absolute
  linear bound and growth on doubling, independently of timing noise.
- Repair and removal at 10,000 and 20,000 levels under a 128 MiB address-space
  cap and a 32-descriptor limit, recording wall times and peak RSS. The capped
  child starts with `LC_ALL=C`, preventing a locale archive larger than the cap
  from being mapped before the test begins. Worker environments are unchanged.
- A real candidate namespace creating a 12,000-level locked branching tree:
  the original observed test result survives cleanup without residue.
- Injected allocation failure: the exception propagates, pending traversal lists
  are released and no directory descriptor leaks.
- A moved child's changed parent inode refuses repair. Existing live symlink
  swapping, pinned-inode rename, hard-link, removal and interruption tests remain.
- Direct staging tests preserve unusual and non-UTF-8 names, file bytes/modes,
  symlinks and external hard-link targets. Replacing a staged inode with a
  symlink refuses before following it, without leaking a descriptor.

The first uncommitted replacement used a linear list-based DFS. Its 11-method
focused run had one failure: 10,000-level repair took 0.804562 seconds while
20,000-level repair took 4.669896 seconds, exceeding the new test's unchanged
growth bound. Peak RSS was 35,640 KiB at both sizes. A separate diagnostic run
then failed allocation in `scandir` under the 128 MiB address-space cap, with
test-owned scratch left by its generic teardown. A compact-buffer iteration also
failed allocation. The test process had already mapped a 223 MiB locale archive;
its cap setup was invalid. The C locale reduces startup virtual memory to about
15 MiB, without changing the 128 MiB or 32-descriptor limits. Failed logs and
source snapshots are retained. The final walk also stages deep subtrees to
bound parent lookup depth; timing thresholds are unchanged. No failed run becomes
a passing result by omission.

The packed-walk profile measured 3.862716 seconds in parent opens out of
4.361475 seconds total, versus 0.038090 seconds in child opens. Deep staging
addresses that cost without relaxing the growth bound. The first staging pilot
exposed opening a parent before unlocking the current directory; recovery then
exposed directory-move permission on a child. Both permission-order bugs are
fixed in this batch. Their logs and exact source snapshots remain available.
All test-owned residue, including the interrupted private stage, was recovered.

The final full batch used one shared reservation. Results:

| Check | Observed result | Command wall seconds |
| --- | --- | --- |
| Pre-fix snapshot with three new regressions | Expected three failures | 0.194467 |
| Focused repair/staging/confinement regressions | 13 PASS | 5.682890 |
| Scoped fast gate | PASS, 97 tests, syntax/lint/review text checks | 106.451218 |
| Legacy evaluator unit suites | 59 PASS | 24.924868 |
| Independent verifier | 19 valid pairs, 25 original methods, 192 reference calls; 38 attempts: 19 pass / 19 fail | 9.948951 |
| Legacy verifier | All 30 pairs valid; 89 reference tests; no skips | 44.843547 |
| Two persistent real-namespace branching probes | Correct answer passes; wrong mode remains test_failure; both leave no residue | 3.419514 |
| Recovery of test-owned failed-run residue | All recorded targets removed | 1.085451 |
| Input/report/log audit | PASS: 100 reports, 240 log bindings, all 30 frozen tasks | Not timed |

The focused deep-tree subprocess used the unchanged 128 MiB address-space cap,
32-descriptor limit and growth bound on the actual host-backed fixture:

| Branching depth | Repair seconds | Removal seconds | Reported peak RSS, KiB |
| --- | --- | --- | --- |
| 10,000 | 0.567812 | 0.443832 | 35,852 |
| 20,000 | 1.152365 | 0.908312 | 35,852 |

The fast gate independently repeats that case: repair 0.562791 / 1.133470 seconds,
removal 0.432084 / 0.883683 seconds, reported peak RSS 89,916 KiB at both depths.
These are control/cleanup measurements, not coding-model results. The two retained
12,000-level sandbox probes took 1.598582 and 1.597930 seconds respectively; the
wrong answer retains one original failure. Their host runs also stay under the
128 MiB soft address-space cap, with unchanged worker limits.

Full suites, namespace probes and deep-tree runs used the shared reservation.
Two lightweight host-only spot checks (two and three methods, 0.002 / 0.048 seconds)
also passed separately. The input/report audit is a lightweight read-only check.
The old long documentation acceptance gate remains deferred to T-310. No full
repository policy scan is claimed for this batch; the scoped fast gate and final
changed-file checks are the development gate.

## Integrity and scope

The frozen collection, source/hidden-test/fixture pins, selected original methods,
control profiles and thresholds are unchanged. The worker, legacy grader,
sandbox/launcher, production runtime, model transport and agent prompts are
unchanged. No hidden test is edited and no held-out task or model is run.
No supply-chain cascade or long acceptance gate is regenerated mid-batch.

The previous commit's independent exact-commit PASS is retained byte-identically
and closes only its original T-311.26 scope. Its two low findings are backlog
notes in the oracle guide. No new per-task or per-finding row is created.

The suite still has 30 development tasks, 19 independently graded historical
control pairs, no held-out set and no real current-harness coding baseline.
Generated candidates remain disabled. Constant descriptors and linear traversal
do not provide a disk quota or a host cleanup deadline; aggregate quotas and
admission remain work for the single suite batch. No score, release readiness,
full sandbox qualification or parent-row completion is claimed here.

## Input and artifact bindings

Private artifact names below are relative to the retained results directory
identified in the independent review request. The manifest binds 540 raw files,
including reports, logs, failed-run snapshots and verification scripts.

| Input | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `7b897700121017ceb6347f34e57d1bdf145fd2851671a22a1dfe0c53273a8b61` |
| `scripts/coding_eval_oracle_worker.py` | `88b66a80c768cc5cd3e90e77e7b4f0f2c0226e65b5ff0a92bb31db8ca9da0b90` |
| `tests/test_coding_eval_oracle.py` | `29fe617df0e937d4c02a980284d090ff6fa440b364b7f308dc3198fff7f3b205` |
| `complete/artifact-manifest.json` | `07dfeeef36796234fc0a43a70e0a7a7003c8f214e06241a7fb0c063b506b8125` |
| `complete/source-digests.json` | `63a23eef93c534fec82105b5c32752381e173e7e3369f904420098671286b8f0` |
| `complete/commands.json` | `94f896bb07a2684a7d653622f8cd120c3700593840efd01f58ead1d90fd22224` |
| `complete/focused-result.json` | `b258f926e2bda6005f47787a532b78ec9d8a44e1527d92d9b5dcb9e31ab3933e` |
| `complete/fast-result.json` | `8972e6091dd9952aaaae0d179450916537a387545195f81535fc325056c7adac` |
| `complete/legacy-unit-result.json` | `1afe32197308f2a771b12ac100168227bbcf2ccadb3bae008c388f4da93be578` |
| `complete/independent/summary.json` | `b281d42158f770b65d5b58f3aeb16d99375e105f8ee8fe77151d8e6fc0fea9bb` |
| `complete/legacy/summary.json` | `03a88eaf339c58bf66718c13b68344e388d4f30fc96d25021105a531a67d90c4` |
| `complete/branch-probes.json` | `2427469185224587018c7b7865193a45af9be123a8d8f4da12bf5c02c2de33d5` |
| `complete/owned-recovery.json` | `568c42a4e7d0bd7088f408e1d22ed6a9cc718fd996629dc5f0c9c92fc5696d6c` |
| `complete/audit.json` | `618d581a5039a95bae9dedb8756dbda5bae937bff1a6f0920c7562b98a214e06` |
| `profile-packed.json` | `0d8971318b7dc6b4b02da0343a626ce2365938f839640ad262cf3f7e4f8593c2` |
| `locale-pilot-result.json` | `9acaf155e7693e43b7f167f96065c4b4546da50bb6218127a6027a3fe2ab94b9` |
| `verified/recover-owned-result.json` | `1a151394066839a3d4b2cdf0e5c2f6d10f07cbb758ee3881bd635196cb931a9b` |
