# T-303/T-306: diagnose the completed line-range repeat

Reporting base: `7d42cc089972cf65cddf238faf3c0c98b9255271`.
Frozen measured source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.

The [completed repeat](t303-line-range-checkpoint180-2026-11-20.md) records
180 development attempts across six profiles. Its largest broad refusal category
is `code.proposal.edit` (51 attempts). This read-only diagnosis separates those
refusals before any further tuning. No inference is rerun, and no product source,
prompt, task, profile, schema, budget or strict-check rule changes.
T-303 and T-306 remain open; independent core review is required.

## First failing condition per attempt

The frozen native helper reproduces all **162 complete-reply checks**, including
the two format refusals. A diagnostic classifier follows the Rust check order
on the 160 replies that passed typed format checks: resolve the complete batch
against the original file, then check individual preimages, visibility, overlap,
size and net change. Its acceptance/refusal agrees with the native checker on
all 160. The native checker emits a broad error code; the finer labels below
are diagnostic classifications, not additional native error codes or task grades.

| Diagnostic condition or retained outcome | Attempts |
| --- | ---: |
| Repeated original text | 21 |
| Invalid line order | 16 |
| Line range outside the source | 3 |
| Original not fully visible in one excerpt | 4 |
| Overlapping edits | 4 |
| NUL in replacement | 2 |
| Unknown source path | 1 |
| Format refusal | 2 |
| Incomplete response at output cap | 9 |
| Length termination at output cap | 3 |
| Transport timeout | 6 |
| Strict ungraded proposal | 109 |

The seven edit-condition categories sum to the original 51 edit refusals.
There are also two format refusals and 18 generation refusals. The original
109 strict ungraded proposals remain ungraded. Generation classifications are
derived from retained terminal events; the transport is not replayed. All native
results and original recorded causes are compared without changing either.

**Repeated original text is the largest individual refusal condition: 21/180.**
All 21 first-failing fragments are selected by line range. Of them,
19 select one line and 10 select only whitespace;
21 addressed ranges are fully visible in one supplied excerpt.
The one-line and whitespace groups overlap. Repeated occurrences are counted
in the complete original file, including overlapping matches. Visibility of the
addressed range does not waive whole-file uniqueness. No range is widened,
normalized, repaired or submitted as a changed answer.

| Profile | Repeated original | Invalid line order | Strict ungraded proposals |
| --- | ---: | ---: | ---: |
| gemma4-26b-a4b | 1 | 5 | 19/30 |
| gemma4-31b | 2 | 1 | 23/30 |
| gemma4-e4b | 4 | 7 | 15/30 |
| gpt-oss | 5 | 1 | 13/30 |
| muse | 5 | 0 | 15/30 |
| qwen35-27b | 4 | 2 | 24/30 |

These are first-failure counts, rather than an inventory of every defect in an
answer. They do not establish the model's intent, the correctness of any proposed
change, or how many tasks an improvement would solve. Counts are over attempts,
not independent tasks. There is no model qualification or role selection.

## Reproduction and limits

The bundle authenticates **987 consumed files** against the complete,
unchanged originating manifest (1,015,833 entries), with full byte lengths and
SHA-256 values. These include all records, available answers/checks, contexts,
terminal events, batch indexes, all 30 original source payloads, the development
preflight, original audit and frozen helper. Original source text is also compared
with its exact Git blob. Excerpts are verified as exact original UTF-8 byte ranges;
record log digests, identities, zero retries and absence of grading are checked.
The full predecessor inventory is retained as data, **not rerun or claimed
verified in full** by this diagnosis.

The 38 synthetic diagnostic controls cover LF-only line counting, CRLF,
Unicode, no phantom final line, invalid/out-of-bounds ranges, mixed edit forms,
whole-batch resolution order, repeated/overlapping originals, visibility,
unchanged edits, replacement NUL, edit overlap, size and net change. Four controls
check repeated-range observations, including an invisible addressed occurrence.
Custom excerpt tests and the oversized-answer size-branch test are classifier-only;
the other edit-condition controls also compare with the frozen native helper.
The initial analysis, exact diagnostic extension and final execution logs are
retained. Reproduction must yield byte-identical complete analysis JSON.

No task loader, candidate tree, project test, hidden test or held-out input is
read or executed. Proposed code remains text. Only the authenticated original
effect-free helper runs; no model, API server or GPU process starts. The analyzer
emits counts, identifiers, offsets, lengths and digests, not replacement source.

Original manifest SHA-256:
`5e21a4ea827d38b9b014fe4bd200e6565f5d7e1771bdd71ae8df6ec277dd94be`.
Analysis SHA-256:
`e2838f4d65f1d81210bc9d27e5a6ab54e18b8936be5aae1d25a998c4185df02c`.
Analyzer SHA-256:
`932838c54afcc160bcd659e0bdfe03fd64c5b10dd53d388ae2c197336a33d5f6`.
Consumed-input origin index SHA-256:
`341c214d148b29629a434e8535b4951220a7c5b64b69871247d118b3cccdaecc`.

The reporting fast gate, Markdown lint/render, privacy, licence, preservation
and portable diagnosis reproduction must pass before review is requested.
All 512 ledger table lines, 49 historical path blobs, README, historical inventory,
Decision 0051, status model and supply-chain outputs remain unchanged. Earlier
scoreboard bytes remain an exact prefix. The unchanged full source gate remains
**FAIL: 1,043 passed, 53 inherited failures, 11 ignored, 31 targets**. This is
retained predecessor evidence, not a new source-suite run. No SBOM, long gate,
acceptance, release, owner-test-ready or push occurs.

## Next work selected by MEASURE FIRST

Improve how the model selects a unique original anchor, especially for insertions
beside blank or repeated lines. This is an experimental direction, not a measured
improvement or a waiver of uniqueness. Keep exact original identity, visibility,
all-or-nothing refusal, permissions and every existing bound. A change to what
the model sees or how its edits are interpreted needs its own tests, frozen inputs
and a complete six-profile development repeat before further tuning.

The 16 invalid-order refusals remain the next-largest individual condition;
this analysis supplies no reason for another cap, deadline or bookkeeping feature.
Existing low findings stay T-309 backlog. Any new high or medium review finding
comes first. The preceding measurement review remains in flight at preparation;
no independent PASS is inferred.

Native executable admission and original execution/grading controls remain
unavailable and are not reconstructed. First-attempt and within-budget task passes,
feedback and retries remain unavailable. No row closes.
