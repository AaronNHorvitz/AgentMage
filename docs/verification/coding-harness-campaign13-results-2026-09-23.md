# Source-Bound Native Coding Results — 2026-09-23

Status: Muse bounded development campaign passed; GPT-OSS not qualified.
Authority: owner continuation and Decisions 0054, 0061, 0062 and 0063–0079.
Scope: Tasks 48.2.4–48.2.6 and 50.2.4, with their necessary prerequisites.
Production admission, independent review, gated milestones, supported platforms
and release remain open. No earlier failure is reclassified.

## Exact implementation and evidence

All campaign13 runs used clean commit
`ad28b5ca7862a483bb4b9bf74c199d5f994ae43f`, tree
`c4a49365b09ac2ef146b102bc557989dd37f41ca`, branch `demo/fedora-local-docs`.
The actual CLI, ordinary host, authenticated private IPC, existing coordinator,
native confined tools, exact grants and verifier were exercised. No scripted
response or direct inference probe counts toward the native results.

| Binary | SHA-256 |
|---|---|
| `agentmage` | `7e1185bf2197a30cbf7665c18c8335c6d07a87ad5260c22581fb5115f672e42a` |
| `agentmage-host` | `935c0404504a3e946a3a2ebe9d15ea7b85ef43a7ee12e2635a60732dcc367799` |
| `agentmage-read-only-worker` | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |

The [unchanged prospective protocol](coding-harness-native-campaign-2026-09-23.md)
requires eight of eight cases on the same source/binary/profile tuple. The
[retained Muse summary](../../artifacts/coding-harness/2026-09-23/muse-campaign13-summary.json)
is an exact copy of the read-only recomputed report, SHA-256
`62a943cb551be20995a1add1fdbdbaa44d0365883d466a200f57ea891d33a1fd`.
Each of its eight collectors was recomputed from the durable raw copies and
compared exactly with the original report. All thirteen checks per case and
all five cross-case checks passed. The summary binds each collector digest,
exact profile, binary identity and wrapper identity; it is not admission.

Private evidence root: `~/.local/state/agentmage-codex-coding/`.
Per-case streams, results, collectors, prompts and responses are in
`runs/2026-09-23-campaign13-<model>-<case>-<repetition>/`.
Private prospective drivers are `native_campaign_pass_20260923.py` and
`native_campaign_case_20260923.py`; their identities and exact objectives are
bound in `driver.json`. No owner intervention changed a case in progress.

## Muse: eight real-model passes

| Case | Repetition | Verifier | Seconds | Turns / native effects / verified artifacts |
|---|---:|---|---:|---|
| Repair | 1 | `SUCCESS` | 410.616087 | 7 / 6 / 24 |
| New file | 1 | `SUCCESS` | 412.197970 | 7 / 5 / 23 |
| Multi-file | 1 | `SUCCESS` | 408.564595 | 7 / 6 / 25 |
| Stable | 1 | `NO_OP` | 308.529451 | 4 / 3 / 13 |
| Repair | 2 | `SUCCESS` | 408.340338 | 7 / 6 / 24 |
| New file | 2 | `SUCCESS` | 403.181695 | 7 / 5 / 23 |
| Multi-file | 2 | `SUCCESS` | 539.829721 | 11 / 10 / 37 |
| Stable | 2 | `NO_OP` | 318.156959 | 4 / 3 / 13 |

Every mutation first observed a genuine failed registered validation, then
performed bounded native edits, complete passing validation and current Git
diff/status inspection before verifier success. Repairs changed only the
intended `broken_add`/`broken_subtract` identifiers. Tests were untouched.
The second multi-file case additionally hashed and read both files and retained
37 verified artifacts, exercising the corrected mandatory-record allowance.
Stable cases passed complete validation with no workspace change.

Both new-file cases correctly encountered one unavailable absent-directory read;
there were no protocol rejections. Each then created only `src/calc.py` with
`add(a, b)` returning `a + b`. Its postimage SHA-256
`ba1a531f581d2e6094e978ed6f7aca7a8d92eeb62c6e7ad73ee692f7f18bc772`
matches the native creation receipt. Ordinary Git diff omits untracked files;
the receipt, file hash and supplemental read-only no-index diff are retained as
`final-created-file-evidence.md`. No staging was used to manufacture a diff.

The unchanged profile is Muse Q4_K_M artifact
`4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`,
manifest `8a8ee8efc1738f30d143f94f753e6079808e05dc3127df28a965676d7edfd24b`.
The summary preserves complete tokenizer/template/codec/runtime identities.
The largest inspected input was 28393 tokens and output 1585 tokens; all
responses ended at EOS. Served context 32768, output reserve 4096 and safety
margin 256 were unchanged. Peak sampled total GPU memory was 18051 MiB,
below the 22528 MiB guard. Cumulative scope memory peaks were 620453888 and
664961024 bytes for the two four-case passes, not isolated per-case peaks.
All guards passed and binaries stayed unchanged. Some non-GPU scripted checks
overlapped the first pass; timings include startup integrity checks and are
not uncontended inference benchmarks.

## GPT-OSS: separate unsuccessful disposition

The exact maintainer-converted MXFP4 artifact remained
`27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`,
manifest `25cec5c16a57a39ffea34d1d59bc0c734330b281ba8722e8154e9b7001e930a7`.
Its conversion is not represented as first-party OpenAI publication or approved
production conversion equivalence. Context, generation and resource bounds
were identical to its prepared 32K profile; no smaller model was substituted.

| Current case | Result | Seconds | Observed boundary |
|---|---|---:|---|
| Repair 1 | `EXHAUSTED`, exit 7 | 194.246270 | One hash, then two duplicate-channel refusals; no validation or edit |
| Multi-file 1 | `EXHAUSTED`, exit 7 | 218.575159 | Two hashes and one identifier patch, then two duplicate-channel refusals; no validation |
| Stable 1 | `FAILED`, exit 8 | 163.791530 | Unknown hybrid tool alias; no tool effect or edit |

The exact rejected frames and prompts were inspected. The prompt's call history
has one channel marker; the repair/multi-file responses duplicated it. The
stable response named `functions.agentmage_validation.run-template`, neither
the exact registered ID nor its published native alias. These are correct
refusals, not another demonstrated decoder gap. The existing single-correction
budget was exhausted in the first two cases; unknown-tool refusal remained
terminal. EOS outputs of at most 1618 tokens do not justify increasing 4096.
Guards passed, GPU peaks were 13759/13759/13747 MiB, scope memory peaks were
5370634240/5302513664/341053440 bytes, and all binaries stayed unchanged.

Collector SHA-256, respectively:

- `7dadd088c8fae02beea7fffeac0a7bf67a30a76bd0972a6cc324c4229c6e1d5f`
- `b620a11e1d4c264f2e6f0700b644736ed3215aea50d4db9469fb55b7dee81111`
- `795ada012c6d507c8cc1bcb046956bb7a8ab6eca41e53343929e740b5068b266`

The prior new-file case at `99cc4e16` also failed correctly after a duplicate
JSON key and duplicate Harmony channel, with one premature creation and no
validation. Its collector remains
`f1b7b36ddc59dbf47f179b888a53fd59eb94ea8aa52a15830754e291bde8c5d2`.
That unchanged failure was not rerun. No favorable cases are combined across
pins; no eight-case GPT-OSS success or repeated qualification is claimed.
These observations describe this integration/profile/campaign, not general
model incapability. Earlier actual integration defects remain documented with
their retained-payload regressions in the [repair record](coding-harness-real-model-integration-2026-09-23.md).

## Current executable reliability checks

At the same `ad28b5ca` source/binary pin:

- Matrix13: 22 actual CLI/host cases, all checks passed; report
  `9c793a4ff6fb4d3c31e5e831d1ca03ab745f8b3bef31e8d5b13087f26476b69b`.
- Resume8: safe restart without replay and drift refusal preserving the human
  comment; report `25a2413fdcf903b267d6a85c87b2fec29002d99616db875dde4fb6841d1c32b5`.
- Protocol-resume6: all fifteen checks passed, eight distinct model calls and
  six fresh native effects in one canonical run; report
  `29819c6d3e2544c1e445e981ddd28e9a6518310aeb472ecf391347eceff721d4`.
- Daily-core10: repeatable setup, disk/output pressure, contention and soak
  passed; 26 no-op runs, one session, 156 verified artifacts, clean worktree,
  400.991799 seconds under the declared 900-second threshold. Writer refusal
  took 0.002984 seconds under 5; cancellation took 0.862318 seconds under 30.
  Report `e4e72827085bd985e2a40e2978cb63e611331f8ced5449725724e92c034fdf22`.

These are executable-scripted reliability results, not native model quality or
independent review. The old daily aggregate expects the original first-call
codec failures; it is not presented as a current aggregate gate. Full unit,
negative-fixture, Clippy and boundary verification is retained in the repair
record. The one applicable source-batch SBOM/evidence renewal follows these
completed source changes; its [disposition is recorded separately](coding-harness-evidence-renewal-2026-09-23.md).

The SBOM and current requirements checks passed in the renewal. The first full
documentation attempt correctly failed on nine stale SBOM/provenance/hash
bindings in three older demo acceptance reports. Decision 0080 withdraws that
current demo claim using the existing pending status, preserving all historical
reports and unchanged native-evidence checks. No desktop/demo run is performed
or counted as passed; its complete reacceptance remains outside this scope.

## Failure retention and remaining gates

The private read-only inventory
`runs/2026-09-23-native-attempt-inventory-ad28b5ca.json` binds 74 completed native
attempt records, including failures, SHA-256
`3c1c07e1807479c5587399eb36e6bcd2a4bf74ff715d56e53f77e217ec5e70cc`.
It records missing collectors and missing standardized raw-copy directories
explicitly. Older original-only/alternate-layout raw records and pre-result
launch errors are not claimed as completely enumerated; consult the durable
progress histories and September 22 retained evidence. All current campaign13
raw records were copied durably. The inventory does not reclassify any attempt.

The exact production admission bundle is still incomplete; both catalog
profiles remain disabled. A successful development campaign does not substitute
for model/runtime/isolation/security admission or conversion evidence. A fresh
external reviewer must inspect the pinned connected boundaries and return
findings. Task 50.2.4.7 cannot be approved by this implementing session; neither
`M-HARNESS-MVP` nor `M-HARNESS-DAILY`, broader sprint, platform or release closure
is asserted here. No additional model, service, spending or publication authority
was used.
