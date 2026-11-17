# Copy original fragments: source and first measurement — 2026-11-17 UTC

The fixed coding instruction now asks the model to copy an original fragment
before deriving its replacement, then check every edit against the original
source. This follows the independently accepted exact-edit diagnosis's largest
refusal cause. The source change is `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
The first frozen pair yields **2/2 strict ungraded proposals** on the A4B
profile. **First-attempt and within-budget task passes remain unavailable**;
178 of the planned 180 attempts remain. This is not evidence of better task
completion. T-303/T-306 remain open; independent core review is required.

The [preceding diagnosis review](../reviews/9cd1a07d1128-exact-edit-refusal-diagnosis.txt)
is independent **Verdict: PASS** for exact
`9cd1a07d1128dc21ae5e1b6c7278c233e428c03c`, with no findings at any severity.
It is retained unchanged and accepts that diagnosis and its later planning
rows only. It supplies no verdict for this generation change and closes no row.

## Generation change and controls

[Decision 0212](../decisions/0212-copy-original-fragments-before-replacement.md)
records the hypothesis. The instruction describes nonempty insertion anchors,
empty replacements for deletion, combining edits that share source characters,
omitting unchanged edits and correct JSON escaping. It contains no task answer.
Task and repository text remain untrusted data.

Only the fixed instruction changes in the Rust source. Selection and checker
bytes outside that constant are identical to the parent. The final instruction
retains the original **1,072-byte overhead**. All **30 development user payloads,
outlines and excerpts match** the preceding repeat; the current Rust helper
also reproduces the original checker outputs for all **166 saved answers**.
These are parser and visibility checks, not task grades or model efficacy tests.
No matching, JSON shape, ceiling, threshold, permission or admission is weakened.
There is no normalization, fuzzy acceptance, output repair or automatic retry.

The first 1,447-byte draft changed 11 of the 30 user contexts and caused one
additional source-test failure: the protected-file test reached an edit refusal
before its expected scope refusal after visibility shrank. That draft is
rejected. Its source, helper, preflight results and complete gate log are retained
privately; **no inference used it**. The compact version restores the old source
allowance and resolves that failure without changing a test or limit.

## Actual development run

The available path is Rust context selection → credential-owning chat HTTP API
→ Rust strict checker. The effect-free helper is rebuilt against the current
repository-map crate; its source is unchanged. The generic HTTP transport binary
is the same authenticated artifact at source
`72c002a42957340de0b6fa0eb4d48385db2c7e62` used by the preceding repeat.
No complete current native-runtime execution is implied.
Native executable admission and original grading controls remain unavailable;
full repository exploration, native effects, project tests, feedback and retries
cannot run in this measurement. No control is reconstructed or bypassed.
No candidate, hidden or held-out test ran. Proposals remain unapplied and untested.

| Observed value | Result |
| --- | --- |
| Model profile / tasks | gemma4-26b-a4b / dev-001 and dev-002 |
| Served context | 32,768 tokens |
| Rendered input preflight | 9508–9570 tokens |
| Requested output budget | 23166–23228 tokens |
| Strict ungraded proposals | 2/2 |
| Does not fit / generation refusal / edit refusal | 0 / 0 / 0 |
| First-attempt / within-budget task passes | Unavailable / Unavailable |
| Request and checker time | 60.564 seconds |
| Completed-job reserved work | 97.066 seconds / 1.618 GPU-slot work minutes |
| Controller launch to exit, including queues | 98.138 seconds |
| Backend counters | 19078 input / 4030 output tokens; 2/2 coverage |

Changed variable: the fixed instruction. Output remains
`min(32768, 32768 - rendered input tokens - 32)`. Equal byte overhead does not
imply equal rendered tokens; input tokens and the derived output allowance are
measured again. Schema, model/profile, runtime, complete framing, thinking
request and 300-second total/idle deadlines remain fixed. Both captured upstream
requests contain the exact prepared messages, named inline schema with
`strict: true` and the disabled-thinking request. Both HTTP responses are 200,
with nonempty content and absent or empty reasoning fields; no conclusion about
internal reasoning follows. Both backend logs stop below token limits.

Reserved work includes model verification, loading, requests and cleanup after
binding validation. It excludes pre-entry validation, wrapper overhead, queues
and preparation. The controller interval includes queue and wrapper time after
launch; it excludes prior preparation and reporting. Actual GPU kernel time,
complete reservation duration and total end-to-end latency are unmeasured.
Both tasks also produced strict ungraded proposals in the preceding repeat;
these two observations establish neither an improvement nor a task-quality score.

## Preservation, resources and verification

The campaign freezes all **2,283 preceding repository input names** at their
current committed bytes and retains all 26 original private bindings, adding
18 for the new campaign (44 total). All 195 changed repository hashes are
recorded, including the intervening licence maintenance and this instruction;
no input name is removed or hash check skipped. A complete **9,706-file** Git
archive matches the source commit and runs read-only with Git metadata available.

The first snapshot preflight failed before GPU reservation or model launch because its
read-only tree lacked an empty Git mount-point directory. Its failure log is
retained; its wall time is unmeasured and no GPU job or task attempt occurred.
Adding that empty directory let the unchanged mount arrangement pass its
runtime-manifest, Git and device checks. Preparation uses the build slot and
releases it before the controller; each model pair independently acquires both
wrappers. No extra device, privilege or network authority is introduced.

This two-task job used low priority, nice 19, the 16 GiB/four-CPU lane cap and a
900-second GPU cap. It finished normally at the requested pair boundary.
Settings are revoked, API and model owners reaped, keys and sockets absent,
both API tokens retired and an exact-owner process scan empty. No foreign
process is signalled; no download or external service action occurs.

| Check | Result |
| --- | --- |
| Final formatting, Clippy, Markdown, privacy and licence | PASS |
| Full affected-crate unit/integration/binary tests | FAIL: 1,030 passed / 53 failed / 11 ignored; 0 filtered |
| Comparison with retained source-gate failure identities | Same 53 inherited failures; no new or missing failure |
| Development contexts and old-answer checks | 30 identical user contexts; 166 identical checker outputs |
| Frozen bindings, actual wire bodies, backend counters, new-answer rechecks and cleanup | PASS |

The inherited failures remain fail-closed native-manifest failures. The earlier
source review selected a broader crate set (1,169/53/19); that is not this run's
count. The licence batch's two inherited Python failures and supply-chain limits
remain disclosed in its [evidence](readme-and-dual-licence-2026-11-16.md).
No failed check is filtered, allowlisted or called green. No full acceptance
or supply-chain evidence regeneration runs here; historical evidence is not
presented as current qualification.

The previous scoreboard is an exact byte prefix. All 512 existing ledger table
lines, 49 historical private-path blobs, historical inventory, Decision 0051,
status model and evaluation inputs are preserved. No row closes or lifecycle,
platform, model, release or owner-readiness status is promoted.
The remaining 28 A4B and 30 attempts for each other profile must finish under
this frozen policy before further tuning or capability work. Resume only the
first absent pair; authenticate saved attempts and never replay them.
