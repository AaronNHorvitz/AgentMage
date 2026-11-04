# T-311.15 descriptor repair for third review

Status: third independent core review has FINDINGS; T-311.15 remains open.
The measurements below record the original repair, not acceptance.
Implementation base: `7544b49c`. T-311.15 and parent T-311 remain open.

## Review and repair

[Independent review 2](../reviews/7544b49c12d1-local-coding-t311-15-cleanup-repair.txt)
reproduced two medium findings in the same row. M1: permission repair constructed
host paths and failed on candidate-created chains beyond the path-length limit,
obscuring oracle outcomes and sometimes leaving residue. M2: the builder claimed
that reaping worker leaders synchronously stopped every namespace descendant.
The reviewer observed filesystem mutations after leader reaping. Its race probes
did not demonstrate an escape, but the path-based safety assumption was unsound.
The historical evidence and guide now explicitly correct that claim.

Oracle v13 opens only directories using Linux
`O_PATH | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC`, relative to a pinned parent.
Mode-0000 directories can be opened without reading their contents. Chmod uses
only the host's own `/proc/self/fd/<descriptor>` and therefore addresses the
pinned directory inode. A candidate rename or symlink substitution cannot
redirect that chmod to another inode. Files, hard-linked files and symlink
targets are never opened as directories or chmodded. A separate read descriptor
opens the pinned directory for scanning after permissions are restored.

A chain descends directly with one current descriptor. Deferred sibling branches
reopen from the pinned root one component at a time with the same no-follow
flags. Neither operation constructs a full descendant path; descriptor use stays
constant with depth. Branch reopening can take quadratic work on deep branching
trees; aggregate time, process, memory and scratch-disk quotas remain admission
requirements. This repair adds none of those missing quotas.

Concurrent disappearance or replacement by a non-directory omits that branch;
other I/O errors still refuse publication. The walk does not promise a stable
snapshot while descendants are tearing down. Scratch removal also uses constant
descriptors and postorder traversal. An ascent verifies the recorded parent
inode before inspecting/removing its child, then checks the child's identity.
Unlink and rmdir never follow the removed name; the root's parent is private
host-owned case storage. A move or other I/O failure refuses cleanup. Existing
case removal/retry logic must still succeed before counts and reports publish.
Original interruption,
cleanup-failure recovery, observed test outcomes and failed-attempt counts retain
their existing behavior. Generated scoring stays disabled.

## Third-review correction

[Independent review 3](../reviews/69e083f8f0c7-local-coding-t311-15-descriptor-repair.txt)
found medium M1: both memory and time are quadratic on deep branching trees.
A bounded-memory reproduction raised MemoryError with no case report, an empty
recovery record and retained scratch. The cost disclosure above was incomplete:
candidate quotas and a scratch-disk quota do not themselves bound host cleanup.
T-311.15 stays open after three non-PASS reviews; T-311.16 records linear,
bounded traversal, exceptional-state release and scaling regressions before
admission. No fourth same-row review is requested in this batch.

## Verification

All suites and diagnostic scripts ran through the shared build reservation.
Times exclude reservation waits and include command overhead except the targeted
unittest timing. Source/test bytes stayed fixed after the passing fast gate.

| Check | Result | Seconds |
| --- | --- | --- |
| Final targeted regressions | PASS; five methods | 2.386 |
| Scoped fast gate | PASS; 63 oracle unit/integration tests | 62.112236 |
| Legacy unit suites | PASS; 58 tests | 19.995703 |
| Independent verifier | PASS; fourteen valid pairs, 17 methods, 159 reference calls | 7.204620 |
| Initial full legacy verifier | FAIL; dev-003 reference process-state read race | 44.493020 |
| Full legacy verifier, unchanged repeat | PASS; thirty valid pairs, 89 reference tests, no skips | 44.347981 |
| Unchanged reviewer depth/special-file probes | PASS; five diagnostics, expected outcomes, no residue | 1.465113 |
| Policy validator | PASS; 716 Markdown files and policy invariants | 148.670918 |

The initial full legacy run retained an already documented dev-003 flake:
`ProcessLookupError` while reading a disappearing process's state in the frozen
SIGKILL cleanup test. Its complete report and logs remain, and all other pairs
were valid. The entire unchanged verifier passed on repeat. This is a manual
control repeat, not an automatic retry or a model score; the dev-003 admission
policy remains unresolved before generated scoring.

New regressions exercise actual candidate namespaces with a 2500-level locked
directory chain, both after a correct effect and after a wrong 0555 answer.
They require the original pass/test-failure outcome, oracle counts, no recovery
record and no remaining case workspace. A separate process lowers its descriptor
limit to 32 and checks repair and removal of a 2500-level branching tree. A deterministic ancestor
replacement at chmod time checks pinned-inode behavior, and a live process swaps
an ancestor with a sentinel symlink throughout repeated repairs. A fifth
regression moves a parent during removal and requires a cleanup refusal without
altering the sentinel. Existing static
symlink/hard-link, interruption, wrong-mode and cleanup-refusal tests remain.

The initial targeted run failed: permission repair succeeded, but the generic
remover exhausted the lane's descriptor limit on the deep tree, hiding outcomes
and retaining scratch. The bounded-descriptor remover fixes that without raising
limits or reducing depth. Both exact test-owned leftovers were removed after
their available failure/recovery reports were retained. The next targeted run
passed those cases but exposed a diagnostic-order issue in the new parent-move
test: cleanup refused with a missing-name error before checking parent identity.
Parent identity now validates first. Both failed logs remain private artifacts.

## Artifact bindings

The audit passes for 153 reports and 339 report/log bindings, including the
failed initial legacy run, the full repeat and the five reviewer diagnostics.
It verifies oracle-owned counts and sticky flags when present, all thirty
frozen task/source/test/corpus pins, the unchanged six legacy evaluator files
and the unchanged worker/test bytes. The old ledger suffix and the retained
independent report are unchanged. The collection stays development-only v9,
digest `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
Every failed invocation remains recorded. These are frozen controls and
synthetic probes: model attempts and GPU minutes are zero, model score and
tokens null. Both old test-owned temporary trees were removed; other historical
scratch was untouched. Raw task data stay private. Artifact names below are
relative to the private batch directory named in the review request.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `5085037a1b3d2e89fa71bc6e9edb1b500fb5f4c7c7e2a9c3a3dd7b5e1a2f1b40` |
| `scripts/coding_eval_oracle_worker.py` | `e291fb8f6e5c68a54517134b4f4f9da8a8a8444fa29b7ac035c73f65391f7ad5` |
| `tests/test_coding_eval_oracle.py` | `d59282462047a70276a07bb250fdb46256b040563c9a0db8a811a43725eed532` |
| `tests/test_coding_eval_oracle_worker.py` | `5a86deb2e1db6d56178af365e04a75c5c505d1305c9a958bf03c8d985acc3e54` |

| Artifact | SHA-256 |
| --- | --- |
| `targeted.log` | `184ad9912f7d6a3bd2a6defddd9d4302ffcdf4e63b7bd43c632317943aa360d5` |
| `targeted-retry.log` | `b04d2ef62a1d16eb62798232eadd06b6ca76c7a35eccedde3c22bc70916f3e8e` |
| `targeted-final.log` | `8e3223516d9773932127d6e1d3a496cd1f58661b1b2f221be7e1c23362f2dbfa` |
| `owned-recovery.json` | `e67128d0c7b3df7315989d1ba4159d0c75433459de0627c20ffad2493e0b269b` |
| `source-digests.json` | `d10ac4b4f7a3917ea908bbcc9151bd2ee3e5d8389a10710156f0c21483b13f3d` |
| `verify.py` | `a14fcd1dd21eb9a4cb7e6648c6ac401674bdd139f1d6337d040a60e8c722d3aa` |
| `fast-gate.log` | `d06c90a684c108047cba22f8e854c4f17e9ecd1d4901275df27c66ad83559ff7` |
| `verification.json` | `e9e7081278888cf82d539d8841a32bd44703c6e6331565a4b51feee332c9614f` |
| `independent/summary.json` | `5b0d75c61c8a8d44c3054f38c2889f734396d23b8b00fb83a7a59829bf82064b` |
| `legacy/summary.json` | `6a4d7d51bb1eccb50c796d257154ab3e24a241f1f0877f86400314161f2e3995` |
| `legacy-repeat/summary.json` | `3a041b2bda66787e12ad6527b27670943ec17d5544100c3988ffd884b9eb6f6b` |
| `final-controls.json` | `d666406a6512d045d879e512266b61cfbed941ad7bb22eaf78764c8d06929d17` |
| `reviewer-probes.py` | `0783660b1206756da9325a11e2314d04332b05ceeb4e63194b10176885ffb4e2` |
| `reviewer-probes.json` | `564423a42f9d76727c256e253c14859c243ae5caaa729fa94effa596175abc7a` |
| `audit.py` | `506ad3875f1dddc094b478f60392fab3cc7a019a91c2ea0e86a81a5cf6e64574` |
| `audit.json` | `0ae8f18897027db103daa7068064709a29849148d64209e2ad8beb66e7e2ca13` |
| `policy-result.json` | `33884097813a3ddb63653efdecdfd369a9cc9fbc5c558fa36a287ea71154ff92` |
| `policy.log` | `5c0e744bfd30c2a7d0ae766a62511324a08abf2d0d6d458b94c58e996018b1c9` |

## Remaining scope

Review 2's medium findings were resolved by the third exact-commit review;
its new medium finding remains open as described above.
Low L1 (host-backed scratch disk quota) remains
required before generated scoring. Low L2 (Path-only helper inputs) was resolved
by review 2. Frozen source/test bytes, selected methods, collection definitions,
thresholds, controls-only admission, the legacy evaluator and worker stay
unchanged. No model run, GPU use, held-out access, prompt tuning, download,
supply-chain regeneration, acceptance gate, release or owner readiness claim.
