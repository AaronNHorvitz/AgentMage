# T-303/T-306: explain original insertion addresses before the next repeat

Base: `0b97c678aaa9f95e1e9c1db72e6db96668d35a7d`.
[Decision 0216](../decisions/0216-original-address-insertion-guidance.md) targets
17 invalid-line-order refusals, the largest individual refusal condition in the
completed unique-anchor repeat. All 17 reverse the range; seven are adjacent
reversed ranges and nine start beyond the original source. These observations
identify submitted numbers, not the intended edits.

This is preparation with **no model inference or demonstrated improvement**.
T-303 and T-306 remain open; independent core review is required. The preceding
[exact-commit diagnosis PASS](../reviews/0b97c678aaa9-unique-anchor-refusal-diagnosis.txt)
is retained unchanged. It has no high or medium finding and adds no new low.
Existing lows remain T-309 backlog. No row closes.

## Changed variable

Only the fixed generation instruction changes in production source: **1039 to
1030 UTF-8 bytes**. It emphasizes nonempty original ranges and explains insertion
before and after a selected original line with a concrete numeric example.
It forbids reversed insertion ranges, numbering new lines and addresses beyond
the original source. Existing whole-file uniqueness, visible anchors, retained
context, replacement line endings and exact-text guidance remain.

The rest of the production file is byte-identical to the base. Parser, strict
checker, retrieval algorithm, framing, schema, permissions and all limits stay
unchanged. Reversed or ambiguous ranges are never repaired. The new regression
covers 24 valid insertions at first, middle and final original lines under LF,
CRLF and both final-line-ending states. It also checks atomic batch refusal of
48 reversed ranges in both edit orders and four nonexistent final-line anchors.
All ten line-edit tests pass; previous tests and minimum budgets are unchanged.
These are effect-free component checks, not executable coding tasks.

## Development preflight

All **30** development payloads match their frozen historical source Git blobs.
The preflight authenticates **982 consumed files** against the unchanged
originating manifest (119,294 entries) and the preceding diagnosis manifest.
It verifies the frozen helper's identity against the original campaign binding,
reproduces all 30 original contexts and all **158 complete-reply checks**,
then compares those same answer bytes under the new contexts. There are
**0 check differences**. The 114 prior strict proposals remain
ungraded. This comparison does not rerun a model, repair an answer or grade a task.
The complete predecessor inventory is not reverified in full.

Selected excerpts change on **6** tasks: dev-006, dev-007, dev-015, dev-019, dev-027, dev-028.
The shorter instruction changes budget-dependent selection without changing
the algorithm or allowances. Every delivered body matches its original UTF-8
byte range; framing, byte lengths and original line bounds are counted
independently. Message content sizes are **10,991–32,618 bytes**.
These are not rendered model tokens; actual token fit remains unmeasured.
The output schema and all six profile files are retained unchanged with their
original manifest bindings. The changed variable is the fixed wording and its
resulting budget-dependent excerpts.

Preflight SHA-256: `824c7232c2f6375aa630f71662c4c98f107e4e1c0bbeae3df4a24a0d49c1e2d4`.
Method SHA-256: `e3421ceb048eccc9284f88d897689cedd608134999fbff318059be814add23c3`.
Checker preservation SHA-256: `a2b33c7e93de142f60b31f7eb79122ad3cc9c39cb25ee875c5737dc69cbe66ed`.

## Verification and next measurement

Formatting and Clippy pass. The full affected-crate gate remains **FAIL**:
**1,045 passed, 53 inherited failures, 11 ignored, zero filtered, 31 targets**.
Every failing test identity matches the previous source gate; there are no new
or absent failures. The added insertion regression accounts for the additional
passing test. No test is skipped or allowlisted to obtain a pass.
Direct Markdown lint/render, review-format, privacy, licence, preservation and
portable preflight reproduction must pass before the review request.

All 512 ledger table lines, 49 historical path blobs, README, historical inventory,
Decision 0051, status model and supply-chain outputs remain unchanged. Earlier
scoreboard bytes remain an exact prefix. No SBOM regeneration, long acceptance
gate, release, owner-test-ready or Git push occurs.

Freeze this source and all 30 contexts, then measure all 30 development tasks on
all six existing profiles in resumable GPU reservations before any more tuning
or capability work. Keep 32K context, the per-task output formula, profiles,
thinking request and deadlines. Report actual token fit, strict ungraded
proposals, refusal causes, tokens and time. No held-out input is used for tuning.

Native executable admission and original grading controls remain unavailable;
they are not reconstructed. No proposed edit or evaluation-task test is
executed; the source gate tests AgentMage itself. Task feedback and retries do
not run. First-attempt and within-budget task passes remain unavailable.
No model is admitted, qualified or selected for a role.
