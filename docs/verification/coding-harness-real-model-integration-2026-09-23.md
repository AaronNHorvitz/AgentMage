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

These are executable-scripted results, not real-model success. The final source-bound
16-case matrix, restart/drift, daily/pressure/soak and repeated native-model campaigns
remain to be rerun. Independent review requires a fresh external reviewer of a new
pinned package; the earlier package does not review these changes.
