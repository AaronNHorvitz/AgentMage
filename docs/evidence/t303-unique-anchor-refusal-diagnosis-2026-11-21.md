# T-303/T-306: diagnose the completed unique-anchor repeat

Reporting base: `6254a671eee35a0c4521e7e53d38fef828cf6202`.
Frozen measured source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`.

The [completed repeat](t303-unique-anchor-checkpoint180-2026-11-21.md) records
180 development attempts across six profiles. Its largest broad refusal category
is `code.proposal.edit` (43 attempts). This read-only diagnosis separates those
refusals before further tuning. No new inference, source, prompt, schema, task,
profile, context, budget or acceptance-rule change occurs. T-303 and T-306 stay
open. The preceding checkpoint review was in flight at preparation. Its subsequently
received exact-commit PASS is retained below. This increment requires its own
independent core review.

## First failing condition per attempt

The frozen native helper reproduces all **158 complete-reply checks**, including
one format refusal. The diagnostic classifier follows the Rust check order on
157 typed replies: resolve the complete batch against the original file, then
check individual preimages, visibility, overlap, size and net change. Its
acceptance/refusal agrees with the native checker on all 157. These finer labels
are diagnostic classifications, rather than native error codes or task grades.

| Diagnostic condition or retained outcome | Attempts |
| --- | ---: |
| Invalid line order | 17 |
| Repeated original text | 15 |
| Incomplete response terminal | 10 |
| Transport timeout | 6 |
| Original not fully visible in one excerpt | 6 |
| Length termination | 5 |
| Overlapping edits | 3 |
| Line range outside the source | 1 |
| Resource guard interruption | 1 |
| Format refusal | 1 |
| Unchanged edit | 1 |
| Strict ungraded proposal | 114 |

The six edit-condition categories sum to the original 43 edit refusals.
There is also one format refusal and 22 generation interruptions or refusals.
The original 114 strict ungraded proposals remain ungraded. Terminal-event
labels above do not infer output-cap attribution: the preceding checkpoint
records backend counters separately. The resource interruption is kept separate
using its authenticated failed batch report and original audit identity.
The original resource gate remains **FAIL**: the sampled 22,682 MiB peak exceeded
the unchanged 22,528 MiB ceiling. No failed attempt is rerun or relabelled.

**Invalid line order is the largest individual refusal condition: 17/180.**
All 17 have start greater than end; none starts at zero. Seven have start exactly
one greater than end. Nine starts exceed the complete file's line count. These
two groups do not overlap here; the remaining reversed range stays within the
file and has a gap of two. They are observations about submitted numbers, not
claims about intended edits. Whole-batch resolution checks order before bounds;
a reversed out-of-bounds range is therefore classified as invalid order.

Repeated original text accounts for 15 attempts. All first-failing fragments
use line ranges; 13 select one line and six select only whitespace. Fourteen
addressed ranges are fully visible in one excerpt. These groups overlap.
Whole-file uniqueness still applies even when an addressed occurrence is visible.
No range is swapped, widened, normalized, repaired or resubmitted.

| Profile | Repeated original | Invalid line order | Strict ungraded proposals |
| --- | ---: | ---: | ---: |
| gemma4-26b-a4b | 0 | 2 | 23/30 |
| gemma4-31b | 1 | 2 | 25/30 |
| gemma4-e4b | 5 | 10 | 12/30 |
| gpt-oss | 5 | 1 | 10/30 |
| muse | 2 | 0 | 20/30 |
| qwen35-27b | 2 | 2 | 24/30 |

These are first-failure counts over attempts, not an inventory of every defect
or independent task outcomes. They establish neither the correctness of accepted
changes nor how many tasks an improvement would solve. No model qualification
or role selection follows.

## Reproduction and limits

The bundle authenticates **982 consumed files** against the complete unchanged
originating manifest (119,294 entries), with byte lengths and SHA-256 values.
It consumes all records, available answers/checks, contexts, terminal events,
batch indexes, 30 original source payloads, the development preflight, frozen
input index, original audit and failed resource report. The helper's identity
is checked against the frozen campaign, rather than assumed from its filename.
Every source payload also matches its exact base Git blob. All 30 contexts
are reproduced by the helper; excerpts match exact original UTF-8 byte ranges.
Record identities, available log digests, zero retries and absence of grading
are checked. The complete predecessor inventory is retained as a manifest,
**not reverified in full** by this diagnosis. Its review remains separate.

The unchanged 38 synthetic controls cover LF-only line counting, CRLF, Unicode,
no phantom final line, invalid/out-of-bounds ranges, mixed edit forms, complete
batch resolution, repeated/overlapping originals, visibility, unchanged edits,
replacement NUL, edit overlap, size and net change. Four controls cover repeated
range observations. Custom-excerpt controls and the oversized-answer size branch
are classifier-only; the other edit-condition controls also compare with the
frozen native helper. The initial and final analyzer, exact adaptation diff,
initial/final results and actual execution logs are retained. The final extension
adds arithmetic observations over invalid-order rows and reads answer bytes
without newline conversion; all initial classifications and attempt rows remain
identical. Reproduction requires byte-identical final analysis JSON.

No task loader, candidate tree, project test, hidden test or held-out input is
read or executed. Proposed code remains text. Only the authenticated effect-free
helper runs; no model, API server or GPU process starts. The analyzer emits
counts, identifiers, line numbers, lengths and digests, not replacement source.
Native executable admission and original execution/grading controls remain
unavailable and are not reconstructed. Task passes, feedback and retries remain
unavailable. No row closes.

Originating manifest SHA-256:
`22529e3cb8b09a396c29d95733e0f7adca00970cc64945a73c5b3d4c52e79869`.
Analysis SHA-256:
`0945c1fb262560416b88d745839c6a073912c0f6767aaf821a54e97d8555b435`.
Analyzer SHA-256:
`1073937b91c7e21b06223adc775a07ca0b4073479e605ad5ffece9022e5f8c46`.
Consumed-input origin index SHA-256:
`40bcd154dab058f1d9cec6473050bf693f9293419ef9c25df3acdb8d1763115d`.

Reporting fast gate, direct Markdown lint/render, privacy, licence, preservation
and portable diagnosis reproduction must pass before requesting review.
All 512 ledger table lines, 49 historical path blobs, README, historical inventory,
Decision 0051, status model and supply-chain outputs remain unchanged. Earlier
scoreboard bytes remain an exact prefix. The unchanged full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, 31 targets**. These are
retained predecessor results, not a new source-suite run. No SBOM regeneration,
long gate, acceptance, release, owner-test-ready or Git push occurs.

## Next work selected by MEASURE FIRST

Improve line-address selection, with explicit guidance for insertion anchors
and nonempty selections. The first experiment should make original line
locations and the requirement for an inclusive selection easier to use.
This is a direction selected by observed failures, not a measured improvement.
Keep exact original identity, whole-file uniqueness, visibility, all-or-nothing
refusal, permissions and every existing bound. Never reinterpret a reversed
range as an insertion or repair it in the checker. Any model-visible change
needs tests, frozen contexts and a complete six-profile development repeat
before further tuning or capability work.

Repeated anchors remain the next-largest individual refusal. The data supplies
no reason for another cap, deadline, cancellation or bookkeeping increment.
Existing low findings remain T-309 backlog; new high or medium review findings
come first. Both parent rows remain open.

The [preceding independent core review](../reviews/6254a671eee3-unique-anchor-development-checkpoint180.txt)
returns **Verdict: PASS** for exact
`6254a671eee35a0c4521e7e53d38fef828cf6202`; it is retained unchanged. It has
no open high or medium finding. Its reporting-templating low remains T-309
backlog with the existing lows. That PASS accepts the preceding measurement;
this diagnosis still needs independent review and closes no row.
