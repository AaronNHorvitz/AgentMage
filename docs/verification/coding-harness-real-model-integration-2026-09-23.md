# Coding Harness Real-Model Integration — 2026-09-23

Status: implementation and native-model diagnostics in progress. No coding-model,
platform, release or `M-HARNESS-DAILY` qualification is claimed by this record.
Authority: Decisions 0054/0061/0062 and the owner's explicit continuation.
The [prospective native campaign](coding-harness-native-campaign-2026-09-23.md)
declares the repeated-case criteria and preserves development versus admission scope.

## Corrected integration defects

The 2026-09-22 rejection payloads were not sufficient evidence of model incapability.
The retained Muse frame selected the registered validation tool, but the old decoder
accepted only final user output. The GPT response struggled with incomplete edit
schemas and model-authored cryptographic envelopes before emitting malformed final
output. Raw llama.cpp `/completion` uses the codec prompt, not its chat-route template.

Decisions 0067–0070 record the corrections and their authority boundaries:

- Native ATEM/Harmony calls bind to frozen tool definitions. Complete parameter
  schemas, explicit invocation rules and readable observations replace incomplete
  schemas and double-escaped byte-array presentation. Duplicate/unknown fields,
  malformed frames and unsupported operations still refuse.
- The development read boundary now launches the existing sealed native worker,
  not its former `/usr/bin/true` placeholder. Production root-owned worker trust,
  exact grants, namespaces, syscall policy and resource confinement are unchanged.
- Exact completed calls accompany results in the existing coordinator/continuation.
  Native feedback uses the actual tool recipient and family template; selected
  pairs remain atomic and ordered. Resume never re-executes historical calls.
- The two development runtime build identifiers now satisfy the unchanged strict
  answer-evidence identity contract. The executable SHA, b10423 source revision,
  model artifacts, 32768 served context and 4096 output reserve are unchanged.
  Existing 8K demo build labels and the separate Muse service remain untouched.
- Controlled creation receives source-bound initial parent observations through
  the held development workspace boundary. The actual effect still requires an
  absent destination, matching current parent/siblings and a fresh exact grant.

## Source checkpoints

Starting checkout: `b0445064edc68e3e4a0e7dca77d4b3fd9c3ebcf2` on
`demo/fedora-local-docs`. Existing work and all prior failures were preserved.

| Commit | Change |
|---|---|
| `9c530aff` | Native proposals, full schemas, readable contracts and retained-payload regressions |
| `f0986e85` | Confined development read-worker composition |
| `0c7df648` | Operation-specific invocation guidance and ATEM parameters |
| `a13fd15e` | Exact native call/result feedback and durable continuation binding |
| `eca3f405` | Development runtime identity, creation observations and run provenance |
| `cf727b39` | Lossless repeated-schema factoring and exact capacity diagnostics |
| `5574c43a` | Native template bodies and read-only evidence collector; first complete Muse repair |
| `bca18d22` | Paired-feedback rollback fixture correction and prospective campaign protocol |

No supply-chain or historical evidence regeneration has run during this source
batch. Local source checkpoints are not independently reviewed gate closures.

## Retained observations

Private raw logs are under
`~/.local/state/agentmage-codex-coding/runs/2026-09-23-*`.
The adjacent `real-model-progress-2026-09-23.md` records every attempt and command.
Model prompt/response/rejection records are copied into their durable run folders;
original failed runs are not overwritten or promoted to successes.

| Run suffix | Observed disposition |
|---|---|
| `muse-baseline` | Old final-channel rejection of a native validation proposal |
| `muse-native-1` | Malformed `={...}` arguments rejected |
| `gpt-native-1` | Harmony JSON-format header exposed; invalid native arguments retained |
| `gpt-native-2` | Parameter renderer rejected a legitimate generic-object schema before launch |
| `gpt-native-3` | Valid granted read exposed the placeholder worker |
| `gpt-native-4` | Operation-specific hash arguments invalid; no tool effect; overlapping rebuild recorded |
| `gpt-native-5` | Native hash succeeded; unsupported Python text fallback rejected |
| `muse-native-2` | Genuine failing validation succeeded as feedback; next response was reasoning-only context echo |
| `muse-native-3` | Six native tools completed, including failed test, syntax repair, passing test and diff/status; finalization failed |
| `muse-native-4` | Seven native tools completed; 32K preflight refused final request after an additional real read; exit 8 |
| `muse-native-5` | Genuine failed validation, then reasoning-only context echo; exit 8, 246.471 seconds |
| `gpt-native-6` | Native hash, then duplicate commentary headers and duplicate JSON key; exit 8, 186.214 seconds |
| `muse-native-6` | First complete real-model diagnostic: verifier `SUCCESS`, 6 turns/5 tools, 367.589 seconds |
| `campaign-muse-new-file-1` | Genuine failed validation, then multi-part reasoning framing rejected; exit 8, 260.892 seconds |

Muse native-3 changed only `def broken_add` to `def add` in `src/calc.py`.
Its last prompt used 27836 tokens, output 583 tokens, and peak sampled total GPU
use was 17987 MiB with no guard error. Its complete final candidate exposed the
runtime-build identity incompatibility reproduced by the new evidence-state
regression. Exit 5 remains a failed end-to-end attempt despite the successful repair.

Muse native-4 ran from `eca3f405`, completed validation, hash, read, patch, passing
validation, diff and status, and preserved all three binary identities. It exited
8 after 438.820 seconds when the next exact context exceeded the unchanged input
capacity with its output reserve. The host contract repeated identical schemas
for multiple tools. Lossless digest-keyed schema factoring passed regression;
every definition and complete schema remains reconstructable and hash-bound.

Muse native-5's prompts used 13102 and 15222 input tokens. Its reasoning-only
echo was not a capacity failure. GPT native-6's malformed tool proposal remains
rejected; neither duplicate channel metadata nor duplicate fields are repaired.
Both preserved all binary identities and passed their resource guards. Negative
fixtures bind the full raw response hashes. Subsequent presentation corrections
use ordinary native message bodies, Muse's named function-schema block and
Harmony's pinned JSON call-history header. Stored packets and source/evidence
bindings are unchanged; the relocated Muse schema map must exactly equal the
native schemas before duplicate presentation is removed.

`scripts/coding_harness_model_evidence.py` cross-checks retained prompt/result
hashes, chronological native feedback, failed-test/edit/passing-test ordering,
diff/status inspection, verifier outcomes, CLI-verified artifacts, binary pins
and resource observations. Its report is diagnostic evidence only, not independent
review, production admission or campaign qualification.

Muse native-6 ran from `5574c43a`. Its exact native chain was registered failed
validation (one failed), a syntax rename from `broken_add` to `add`, fresh complete
validation (one passed), Git diff and status, then verifier-backed `SUCCESS`.
The actual diff contains only that identifier rename. All collector checks pass;
the CLI verified 21 full artifacts. Its final request used 26035 input/281 output
tokens; sampled GPU peak was 17958 MiB, the guard passed, and all three binaries
stayed unchanged. This is the first complete diagnostic success, not repeated
campaign qualification. Every earlier failure remains retained.

The first new-file campaign attempt from `bca18d22` remains unsuccessful. It
produced multiple reasoning frames followed by a directory proposal with invalid
UTF-8 encoding. [Decision 0071](../decisions/0071-native-channel-token-preservation.md)
records the missing raw-completion special-token output setting, bounded
multi-part reasoning parsing and operation-specific read-tool guidance. The
invalid encoding is preserved for native rejection, not silently repaired.

The next new-file attempt at `b7fbf289` failed exit 5 after 238.410 seconds:
registered failed validation, then a valid native read of the absent target.
The frozen read projection refused it before any read approval or effect, but
the host aborted instead of retaining useful pre-effect feedback. Raw SHA
`a9a7e9b2110117856029230a65dab7bae3a722ddbe9f09a93bf2e7c0879a1161` is a
regression; [Decision 0072](../decisions/0072-bounded-pre-effect-read-rejection.md)
records the bounded continuation correction without broadening read access.

GPT native-7 at the same source completed a native hash read then failed exit 8
on an extra Harmony channel separator before format metadata. It ran 170.858
seconds, preserved all binary identities and passed the resource guard with
13753 MiB sampled peak total GPU use. Raw SHA
`df1678399d92dc7f3c705a5da3b4ce986ca84b86627dabc702da442cf920b366` remains a
negative decoder fixture. The accepted history rendering matches the retained
pinned upstream template; no decoder relaxation or generation-limit change is
justified by this malformed header alone. This is another unsuccessful attempt,
not a claim about general model capability.

All attempts preserve the pinned b10423 Vulkan runtime, single slot, four threads,
two-core quota, low priority, 45-minute limit, memory bounds and GPU guard. No model,
generation-budget, context-capacity, permission or verifier substitution was used.

## Verification so far

The safety-margin correction passes 1045 kernel tests (seven ignored), 303 host
tests (eight ignored), 15 binary tests, strict all-target Clippy, binary build,
effect/strict-local audits and Markdown checks. Its native campaign is pending;
these component checks do not turn either failed campaign into a pass.

At `8d625866`, Muse repair repetition 1 passed all 13 diagnostic checks: six
turns/five effects, genuine failed validation, exact rename, complete passing
validation, diff/status and verifier `SUCCESS`; 368.798 seconds, 21 full verified
artifacts, unchanged binaries and 17958 MiB GPU peak with no guard error.
Repetition 2 completed seven effects but failed at final preflight after 411.578
seconds. The initial reflow fix missed the controller's existing 256-token safety
margin: 28645 input exceeds the true usable input 28416. Decision 0073's follow-up
shares the controller-owned reservation calculation between selection and dispatch;
the guard was not lowered. This campaign remains unsuccessful, one pass/one failure.
Both raw chains and the exact failed-count/fit/one-over regression are retained.

The same source's matrix-4 all 16 cases, resume-3 safe/drift and daily-core-4
checks passed independently. Daily-core retained 26 distinct one-session runs,
156 verified artifacts and a clean worktree in 384.558 seconds, contention refusal
in 0.002466 seconds and cancellation in 0.870317 seconds. Report SHA-256 values:

- Matrix: `ee8ec5354bcba87b6c7f158bc3d7f974965fcfea300c4f9f87731a58ef3a70c7`.
- Resume: `154c724c6152a2c969d74dfff31472a4676b5cf0b8b4f4cd064ddf50ee96fc4a`.
- Daily core: `2fd3c2fc8f2d1475d6ce780bf4433c69ef13d0fbdcc3ddbc4e948cecfe7f7455`.

Decision 0073 verification passes: 1044 kernel tests (seven ignored), 303 host
library tests (eight ignored), 15 host-binary tests, strict all-target host/kernel
Clippy, binary build, nine wrapper/collector tests and effect/strict-local audits.
Actual scripted regression `context-reflow-scripted` completes `SUCCESS` through
seven turns/six tools with the exact expected identifier diff, full failed/read/
patch/passing/diff/status chain and unchanged binaries. These are implementation
checks, not a substitute for the next native run. No SBOM renewal ran mid-batch.

At `2b54674a`, two fresh Muse new-file repetitions passed every diagnostic check:
actual failed validation, exact create, complete passing validation, status/diff
and verifier `SUCCESS`. Repetition 1 took 419.472 seconds (seven turns, six
proposals, one pre-effect rejected read, five completed effects); repetition 2
took 382.953 seconds (six turns, five effects). GPU peaks were 17958 and 17807 MiB,
guards passed and binaries stayed fixed. Both raw chains remain retained.
The new file is untracked, so native Git diff is empty; its exact created content
and receipt-bound postimage are retained, with a separately labeled post-run
unified diff for the first repetition. No post-run artifact is mislabeled native.

The same tuple's repair repetition 1 then failed after 389.899 seconds and seven
native effects, despite a correct edit and passing rerun: exact final context
28792 plus 4096 reserved output exceeds 32768. This campaign is unsuccessful
(two passes, one failure), not a qualified tuple. The preceding raw prompt and
an extracted observation are pinned by
[Decision 0073](../decisions/0073-exact-coding-context-reflow.md).
Its bounded exact-measurement/re-selection fix uses the existing context owner;
the native correction and a new complete campaign still require verification.

Current-source scripted results at `2b54674a` were inspected separately:

- `matrix-3`: all 16 cases and every check pass; report SHA-256
  `a634b5b36c12af4e426a6a5e95342001cbb7a0c0c07e2ff3a7aa1a93cdeb4351`.
- `resume-2`: safe same-run/session resume, five new effects without redispatch;
  worktree drift refuses before effects and retains the human comment. Report
  `d4b611618fbe623358ebf0331995b71bb2c0bcca9f83ef0bdd4b221c446fff31`.
- `daily-core-3`: 3/3 setup, disk/output pressure, contention refusal in 0.002032
  seconds, cancellation in 0.829875 seconds, 26 distinct runs in one session,
  156 verified artifacts and clean worktree in 388.636 seconds. Report
  `7cd962980b6e6b81d327facafb8eb25b4c68df1c40dc4cb650c8c63d4d59e2f3`.
  This explicitly scoped core report invokes unchanged local daily checks;
  it does not recycle historical first-call-only model failure assumptions or
  claim the aggregate daily/model/independent-review gate.

At the feedback checkpoint, 17 codec tests, 37 runtime-loop tests, four continuation
tests, 297 host library tests (8 environment-dependent ignores), 15 host-binary tests
and strict Clippy passed. Subsequent identity/creation changes passed the exact
answer-identity regression, five catalog tests, six wrapper tests, seven coding-change
tests, all-target checking and strict Clippy.

Actual-process scripted regressions passed:

- `native-read-corrected` and `native-feedback-regression`: genuine failed validation,
  native read, approved patch, passing validation, diff/status and verifier success.
- `native-create-observation-regression`: controlled new-file creation and verifier
  success; five turns/four tools, 36.168 seconds, all three binary identities unchanged.
- `matrix-2` at `bca18d22`: all 16 actual-process cases passed, including normal
  rollback and concurrent-human-edit conflict refusal. `matrix-1` failed because
  its scripted rollback consumer still read the older observation shape; that
  failure and the paired-shape regression are retained.
- `resume-1` at `b7fbf289`: same canonical run/session resumed to verifier success
  with five new effects and no replayed effect; worktree drift refused before
  effects and preserved the synthetic human comment. The original observer
  mistakenly counted identical canonical event replay as redispatch. Its failed
  report is preserved (SHA `4fe8d2f7e7405ec8290790976b49c0d9113c0a2fc07a43dd61a96d3378f9a851`)
  alongside the passing read-only reassessment and both exact assessor scripts.
- `rejected-read-create-1`: actual CLI/host emitted `tool_rejected` at sequence 8
  with no approval/effect, then created the file with a fresh grant, validated,
  inspected diff/status and reached verifier `SUCCESS` (6 turns, 5 proposed calls,
  4 completed effects). All binaries unchanged; 29.592 seconds. A first invocation
  omitted mandatory `--objective` and exited 2 before host launch; its corrected
  invocation used the same still-clean private setup. This is scripted regression
  evidence only. Decision 0072 checks passed: kernel 1043/7 ignored, host 300/8
  ignored plus 15 binary, inference 105, strict Clippy, binary build, five catalog
  tests, schema fixtures and changed-document Markdown lint. Initial exact graph
  count assertions and the resume fixture's reused call ID were corrected; no
  production guard was relaxed to pass those tests.

These are executable-scripted results, not real-model success. Independent review
requires a fresh external reviewer of a new pinned package; the earlier package
does not review these changes.

## Current-source diagnostics and bounded correction

At `bc2d53dbe3beddf646105f2dff28eb43dc6dc4fb`, the exact controller safety
reservation was shared by selection and dispatch. The actual matrix5 passed all
16 cases (report `a524ca485b454e81eb8860c13faf1a50d4c05c8a64dcde7846b90c8ca8dbcac5`).
Resume4 passed safe restart without redispatch and human-edit drift refusal
(`5f97033dfba66b2c0f223006ae983b7a01385707aa301c0acf4972f7434162aa`).
Daily-core5 passed every declared local check, including 26 no-op runs in one
session, 156 verified artifacts and a 383.976-second soak
(`3341432e32c98bca588dfe65fc987020943ad4ce2bc54e70fff248b5d81a7326`).
The separate core report does not claim the aggregate daily/model/review gate.

Muse campaign5 repair1 completed genuine failed validation, native syntax repair,
complete passing validation, diff/status and verifier `SUCCESS`: 6 turns, 5 effects,
402.561 seconds, 21 verified artifacts, all 13 diagnostic checks passed. Its only
diff is `broken_add` to `add`; the initial verified repository projection supplied
inspection and preimage evidence. Peak sampled GPU was 17958 MiB; guard passed.

Muse multi-file1 failed before effects because its otherwise correct ATEM call
omitted the opening wrapper; 249.530 seconds, peak 17722 MiB. The exact pinned
9992-byte upstream template hash `cfc67e5f...e678` was re-observed and requires
that wrapper. Stable1 failed on flat Git pathspecs, rejected by the unchanged
registered schema before permission; 237.198 seconds, peak 17706 MiB. GPT's
distinct new-file1 recovered from an unavailable read and created the authorized
file, then emitted the invalid duplicate Harmony channel delimiter; 190.238 seconds,
peak 13765 MiB. It also failed to run validation before creation, independently
missing the campaign's failed-test ordering requirement. All binaries remained
unchanged, all resource guards passed, raw failures and failed reports remain
retained. Neither candidate passed this campaign; favorable runs are not combined.

[Decision 0074](../decisions/0074-bounded-model-proposal-correction.md) addresses
the pre-effect feedback gap without accepting malformed output. Complete verified
native syntax rejections and explicitly opted-in argument-parser refusals can
consume the existing one-parser-failure allowance and return a labelled observation
to the next coordinator turn. Only the stateless Git parser opts in initially;
scope-checking write validators, identities, permission and effect failures remain
terminal. Canonical continuation, exact event binding and resource accounting
survive restart. A second parser rejection exhausts; no budgets increase.

The exact failing frames and Git arguments are negative regressions. Verification
passes: 1050 kernel tests (7 existing ignores), 305 host library tests (8 ignores)
and 15 binary tests, 105 inference and 26 read-only tests, plus focused CLI/Linux
launch regressions, 12 Python collector/wrapper tests and five catalog tests.
Strict Clippy, effect/strict-local audits, formatting and bounded binary build pass.
A combined four-package test build was OOM-killed under the unchanged scope;
the failed log is retained, and sequential one-job verification replaces it.

Executable probes caught missing diagnostic scenario names in the CLI and Linux
launch boundary's separate closed lists. Both are now explicit and tested; failed
`protocol-resume-1/2` and `matrix-6/7` remain retained. The acceptance assessor now
records empty-output prelaunch failures instead of crashing. Matrix8 is running
the expanded 19 cases; its protocol correction already reached `SUCCESS` with
8 turns and 6 native effects. Full matrix and new native campaign remain pending.

`protocol-resume-3` recovered the canonical rejected turn/checkpoint before the
next model request, then executed the six fresh repair effects and reached
`SUCCESS` with verified full artifacts and unchanged binaries. Its original
assessor incorrectly required the fast rejection in the first client's polled
output. That failed report (`6f687e44...ab69`) is retained; a read-only reassessment
of the exact canonical replay passed all 15 checks, including no authority in the
rejected turn, no redispatch and same run/session (report
`c6359e408f2b73743d20e16aa7277a410d71988a89240f70c8754e8cdc967b58`).
This is scripted restart evidence, not model qualification or independent review.
Native collection additionally binds each prompt/response to its exact requested
model run and checks correction feedback occurs after the rejection event.
No SBOM or historical evidence renewal has run during this source batch.

## Decision 0074 executable and native checkpoint

At source `f75e8fcf4f1424c74b86ea570432dc42e3b18cb4`, tree
`bf84308eac70ccd4ea074441a6d16797d6901f23`, matrix8 finished with all 19 cases
and checks passing (report
`f3b2a9b5501828af11145e61e97b56b0599df5f6c3e1fba06cd4484717a655c4`).
The unchanged binary identities are CLI `2452782a...52e135`, host
`955bbbf...51afed` and read worker `91b16c4b...9ecf658`; full hashes are retained
in every command result and the pinned private review draft.

Muse campaign6 multi-file1 completed a genuine two-test failure, native bounded
renames in `src/calc.py` and `src/subtract.py`, complete two-test pass, current
diff/status and verifier `SUCCESS`: 7 turns, 6 effects, 25 verified artifacts,
409.440 seconds, no protocol/tool rejections and all 13 collector checks passing.
The only changes were `broken_add` to `add` and `broken_subtract` to `subtract`.
Collector SHA-256:
`08d1f179ca88fdf50b4174aa29a6d31fbd602aac1446519160769b55190cbb7c`.
Peak sampled GPU was 17953 MiB, scope memory peak 5369495552 bytes, guards passed
and all binaries remained unchanged. Raw prompts/responses and exact profiles
were copied to its durable `candidate-records`. This is one native multi-file
success, not eight-of-eight qualification or production admission.

Resume5 passed same-run/session recovery with five new effects and no redispatch,
plus drift refusal preserving the synthetic human edit (report
`83ed3c14aaeadee369c994f440e0c664a2f6ba07e12f3901663262d16da8f94c`).
Daily-core6 passed three setups, disk/output pressure, writer contention and
cancellation (0.002461-second refusal, 0.838748-second cancellation), and 26 no-op
runs in one session with 156 verified artifacts and a clean worktree in 383.780
seconds (900-second threshold). Its report is
`4964518429967a5f9bfd569bea2f001c48c8223428eae11a852374970c7b60a3`.
Separate bounded non-GPU daily/restart checks overlapped the native run; retain
that fact when interpreting latency. These do not assert the aggregate daily gate.

A subsequent contract check found the published runtime-event JSON schema still
allowed only the older read-projection rejection. The actual matrix8
`arguments_invalid` event failed that schema; changing only its reason in memory
to the older variant passed. The schema now exactly matches both native enum
variants. Regressions reject unknown reasons, authority-bearing fields and absent
rejection hashes. All 87 planning/model/catalog tests, canonical schema fixtures,
and validation of that unchanged actual event pass. This changes no binary or
model budget, but the next native campaign uses a fresh source pin; previous
evidence is retained without mixing runs into qualification. Independent review
and final batched provenance renewal remain pending.

## Campaign7 findings and native command correction

At `ef4a05d823ea0987229e2ab389867b15bb4f8282`, Muse multi-file1 and repair1
completed the required real failed-test/correction/passing-test/diff/status chain
and verifier `SUCCESS`. All 13 collector checks passed separately: multi-file
419.515 seconds, 7 turns, 6 effects and 25 artifacts (collector
`ba0c2dc24f70deaf147751cacf56754c11c01dc81f0329668cb342ab29b95e52`);
repair 434.764 seconds, 8 turns, 7 effects and 27 artifacts (collector
`c8e794f6067a8f635c241afca1abb49846726ab82c40bb8c33ce057411eade76`).
Neither had a rejection; both resource guards passed at 17953 MiB sampled GPU.

GPT repair1 ended `EXHAUSTED` after one native hash-file inspection and two
complete invalid duplicate-channel Harmony frames, including after the bounded
correction notice. No validation or patch executed. Raw hashes are
`631e274338af3a78325c3b6dc805aa94ca1ca108bc79910d57e0aad883ca665a`
and `91248e91de40704338448206847ea3ab94a288009f19820b15526bd7161f8b24`.
Elapsed 192.276 seconds, resource guard passed at 13761 MiB; collector remains
failed (`25760bd3b084af368a065450a2ed9f7adcd320a6250a2fce3229db78921de831`).
The exact retained upstream template (`a4c9919c...c8146`) confirms the existing
single-channel `commentary json` header; removing `json` is not justified.
The model's omitted required field and nested path also conflict with the full
published schema. No blanket model-incapability claim follows from this case.

Muse new-file1 then exposed the integration defect in Decision 0075: a valid
registered command was approved but the wrong Direct driver compared reserialized
internal bytes against the original approved native JSON. The run failed in
279.091 seconds with `Dependency(Uncertain)`, unchanged worktree and a passing
17954 MiB resource guard. Both valid native responses and the earlier safe
unavailable-directory rejection remain retained. The command key-order host
regression reproduces the failure before the fix. Campaign7 is unsuccessful;
subsequent source changes require a new campaign, not combining favorable runs.

The correction uses the existing exact registered-wrapper binding and preserves
the generic-command versus targeted-validation distinction. Sorted/pretty JSON
passes only when those exact bytes were approved; reordering after approval is
still denied with no launch. Raw logs for all four attempts are under private
`runs/2026-09-23-campaign7-*`; no prior report is overwritten or reclassified.

Decision 0075 focused verification passes: 308 host library tests (8 ignored),
15 host binary tests, seven kernel command-owner tests including wrong-plan
refusal, three Linux development-boundary tests, and 13 wrapper/collector tests.
Strict host/Linux Clippy, rebuilt binaries, formatting, effect mediation and
strict-local source audits, touched-document lint and diff checks pass. The first
new Python assessor fixture omitted its raw-log files and failed; that log is
retained, the fixture is corrected, and the rerun passes. The actual-process
matrix and native campaign must now run against the new committed tuple.

At `7b1953d544c6f37cd11bb2c809db8b0aaa8564b7`, matrix9 passed all 19 existing
cases. Its new generic-command case returned a real failed-command receipt and
full stdout, not the former `Uncertain`, but the assessor incorrectly expected
repair to continue. Generic failed commands are terminal under the existing
coordinator; only the validation owner turns assertion failures into repairable
observations. The failed report remains retained. The diagnostic is corrected to
`native-command-failure`, requiring exactly one failed effect, original Muse
argument digest, verified failure stdout, clean tree and no validation evidence.
No runtime failure semantics or native campaign thresholds change.

On the same source, Muse campaign8 new-file1 passed all 13 native collector checks
in 352.499 seconds: complete one-test failure, controlled absent-file creation,
complete one-test pass, status/diff and verifier `SUCCESS`, 6 turns, 5 effects,
20 artifacts and no rejections. GPU peak 17953 MiB, memory peak 687923200 bytes,
resource guard passed, all binaries unchanged. Collector SHA-256:
`19b061e6d926a4f369d25fea0da0e289202fe52b16715ccc7521972ccd7c4c8b`.
Created `src/calc.py` is `def add(a, b):` followed by `return a + b`, postimage
`ba1a531f581d2e6094e978ed6f7aca7a8d92eeb62c6e7ad73ee692f7f18bc772`.
It is untracked, so ordinary Git diff is empty: creation payload/postimage and
status provide the native evidence; the supplemental read-only no-index diff
shows the two added lines. Raw logs are retained at private
`runs/2026-09-23-campaign8-muse-new-file-1`, not combined with another tuple.

## Decision 0076: Git argument disclosure

At `22e5c44e7256f51f2f263f0bb8aebc296f54d710`, matrix10 passed all 20 cases
and every check, including the corrected generic-command terminal failure
regression using Muse's original exact argument digest. Report SHA-256:
`5c6ba4ef1a8a9cc583c080a9487804c2d25607807410ba1f7ea66bd0b3c3e818`.
Daily-core7 also passed: three setups, disk/output pressure, writer refusal in
0.002787 seconds, cancellation in 0.838288 seconds, and 26 clean no-op runs in one
session with 156 verified artifacts in 380.857 seconds (900-second ceiling).
Report `942c43feacb15732443c153aaa51d389c6a3ff484696e618946aecbec2e6078b`.
These are local scripted checks, not an aggregate daily gate. The matrix and
early daily checks overlapped the native attempt in separate bounded scopes;
source and binaries remained unchanged throughout these runs.

Muse campaign9 stable1 failed in 316.668 seconds, exit7 `EXHAUSTED`: one native
complete passing validation, followed by two pre-effect invalid Git-status
proposals. Both used `pathspecs:[["."]]`; the second changed only numeric limits.
No source changed, all full artifacts and raw records were retained, and resource
guards passed (17953 MiB sampled GPU, 657551360-byte memory peak). Collector
remains failed: `bef20bafe0def9a01de958665d0e276539039d469a6a7fe96ca300ce8e25d9d9`.
Raw response hashes:
`08f88f09b940ef2778f0f4362c92c9f3b5ebbaa55e7bc04642b33636500b4df4`
and `f417263b1654068216bb7c89c1ff1eaf8a69b338449abb52d5e62baa0968e7ec`.

The existing native owner requires empty pathspecs for status (not just canonical
components), but the published schema omitted this operation-specific rule and
accepted both rejected requests in the retained before-fix AJV check. Decision
0076 exposes the existing operation/path/revision/object constraints and supplies
native-validated status/diff examples. The native validator and one-rejection
budget stay unchanged. This is a local integration correction, not an external
model blocker. Verification and a fresh source-pinned campaign follow; no prior
result is reclassified and no SBOM renewal has yet run during this source batch.

Decision 0076 regression passes the exact two rejected argument hashes in both
the unchanged native owner and tightened published schema. All 90 planning/model/
catalog schema tests, 27 read-only capability tests, 308 host library tests
(8 ignored), 15 host binary tests and 13 wrapper/collector tests pass. Strict
Clippy, build, formatting and boundary audits pass. A mistyped nonexistent Cargo
test target failed after the schema tests; that log is retained and the correct
read-only all-target suite passes. Updated status/diff examples are checked by
the actual native validator, not only JSON Schema.

## Campaign10 and JSON-format delimiter correction

At `ba1158e824a71c29993041b21d781fc99a7b5166`, matrix11 passed all 20 cases
(report `e4c89cd637f1d5c406ada68267a5703b073a2fe6953974c10113fe5b7a66d631`).
Resume6 and protocol-resume4 passed every inspected check, including canonical
same-run recovery, no rejected-turn effects/replay and preserved drift refusal.
Their report hashes are `10aa4d4bb221dc65d2716a2ee8b2dc4022c9e21a0d2dae53083945bef1216c8a`
and `97a2c37178f1358dd5302d362d3845acf55a7997bb277048a48effe165763e17`.
Daily-core8 passed setup3, disk/output pressure, contention and 26-run soak with
156 verified artifacts, one session and a clean tree. Refusal took 0.003212 seconds,
cancellation 0.852410 seconds, and soak 395.842975 seconds, within unchanged bounds.
Report: `8e493814d4e47c820ff78c9371d5ff02c890a4cbf7d6f1b3db8e9284926e6c0f`.
These are executable-scripted checks, not aggregate daily-use qualification.

Muse campaign10 stable1 passed `NO_OP` in 308.639470 seconds with native status,
diff and complete validation, no edits/rejections and 13 full verified artifacts.
Collector: `2e45564c6f3aceceb09be9c210cf9cdc0e2c54e9149fae1fcc397703c7f9be11`.
Repair1 passed `SUCCESS` in 397.342688 seconds: genuine failed validation, native
identifier rename in `src/calc.py`, fresh complete passing validation, diff/status,
21 full verified artifacts and zero rejections. The inspected diff changes only
`broken_add` to `add`; tests remain unchanged. Collector:
`137aa82734ca501029d977dc76a49cb18c7a6cc2cf5c5b6c48af47bc01854033`.
Both pass all 13 collector checks and resource guards, peak sampled GPU 17953 MiB.
Two successful cases do not satisfy the eight-case same-tuple campaign.

GPT-OSS campaign10 new-file1 failed `EXHAUSTED` in 186.046665 seconds. It created
the file before the requested failing validation, then produced two rejected
responses and no validation. Nine full artifacts and all resource observations
were retained; peak sampled GPU 13761 MiB, no guard error, unchanged binaries.
Collector: `aee362a00abba273a18750afa2c11d3cd13d2c38d02000da7f5f293a2c7c4b2a`.
The first response's valid unspaced Harmony JSON-format delimiter exposed the
Decision 0077 decoder gap; its wrong arguments still require native rejection.
The second response's duplicate channel remains malformed. Exact raw-payload
regression reproduces the old rejection before correction. No output/context
or resource limit was implicated. The failed attempt is not reclassified.

All logs are retained under the corresponding private `2026-09-23-*` run folders.
Non-GPU actual-process checks overlapped parts of these native diagnostics in
separate bounded scopes; timings are not uncontended benchmarks. No active run
was changed by the subsequent delimiter correction. Exact codec/catalog bindings
must be renewed before the next separately pinned campaign. The single applicable
SBOM/evidence pass remains deferred until all source changes finish.

Decision 0077 verification passes 106 Linux-inference library tests and nine
native-process tests, 308 host library tests (8 ignored), 15 host binary tests,
13 model-schema/catalog tests, strict Clippy, binary build, boundary/source audits,
formatting and touched Markdown checks. The exact native validation regression
still rejects the retained wrong body. Initial missing-helper-import and SHA
formatting mistakes in the new tests were retained, corrected and rerun; they
were not product failures. The preceding exact-payload test independently
reproduced the product's `tool-channel-invalid` defect before the correction.

## Campaign11 and recorded-artifact integration defect

At `99cc4e1630ea1e658a815e82893f650fe736c884`, Muse repair1 and new-file1
passed all 13 native collector checks in 454.238481 and 386.442055 seconds.
Both observed genuine failed validation before editing, then complete passing
validation and native diff/status. Repair changed only `broken_add` to `add`;
new-file created the authorized `src/calc.py`, with the exact creation receipt
and postimage supporting the untracked file that ordinary Git diff omits.
New-file retained one correct absent-file read refusal, then continued within
the existing bound. There were no protocol rejections in either successful run.

The next multi-file case failed after 492.799490 seconds, exit 5, despite nine
completed native effects and both repaired modules passing complete validation.
The final model completion frame was valid. The coordinator could not retain its
required record 34 because its derived count allowance was only 33. This local
accounting defect, not model capability, caused `InvalidBoundaryResult` and no
terminal outcome. The failed collector SHA-256 is
`732261e4f1323a45458e3f879feef497604bc388b3597822e8379cb401bbe35c`.
The driver stopped without running stable1 or repetition2; campaign11 is not
qualified. All raw responses, rejections, logs and exact identities are retained
in `runs/2026-09-23-campaign11-muse-{repair,new-file,multi-file}-1` under the
private coding-state directory.

GPT-OSS campaign11 new-file1 separately failed in 189.074241 seconds,
`EXHAUSTED`, one premature creation and no validation. A search proposal had a
duplicate JSON key; after the existing bounded correction, a validation proposal
had duplicate Harmony channel markers. Both refusals are correct and retained.
No additional decoder defect was found in those exact bytes. Its failed collector
SHA-256 is `f1b7b36ddc59dbf47f179b888a53fd59eb94ea8aa52a15830754e291bde8c5d2`.
This case is not repeated unchanged or converted into a success. Other distinct
cases and the eventual final source tuple require separate dispositions.

Decision 0078 corrects mandatory-record accounting, retaining the fixed
1024-record and 64 MiB ceilings and all model/resource/authority bounds. A ledger
regression with the observed sizes and the actual scripted nine-tool CLI/host
case both reproduced the prior failure; logs have the `artifact-count-before`
and `artifact-count-scripted-before` prefixes. Recording pressure now closes
canonically as `EXHAUSTED`, without acting on an unretained model proposal.
Completed inference resources are still accounted. The initial pressure-test
field-name typo and unrecognized diagnostic-code test failure are retained;
the correction uses the existing closed budget-exhausted event code, not a
weakened event validator. Neither regression nor the two earlier native
successes closes the repeated-model or independent-review gates.

After the correction, the same actual scripted inspection/repair sequence
completed `SUCCESS`: 10 turns, 9 native effects, 34 fully verified artifacts,
both tests passing and only the two intended identifier changes in the final
diff. The before/after comparison passes all seven strengthened checks only
for the corrected run; report SHA-256
`7abf665e7d75d961ee6e93185a3b722b682e49fd4d6445e8d9842108f34835a2`.
Focused count, overflow and pressure tests pass. Full verification passed
1053 kernel tests (7 ignored), 308 host library tests (8 ignored), 15 host
binary tests and 14 Python wrapper/collector tests, strict Clippy, boundary,
build/dependency-class audits, formatting and touched Markdown checks.
All ran in the required RAM-limited scopes with one Cargo job. The independent
scripted run overlapped non-GPU unit tests; timing is not a benchmark. No
SBOM or historical evidence regeneration has yet occurred in this source batch.
