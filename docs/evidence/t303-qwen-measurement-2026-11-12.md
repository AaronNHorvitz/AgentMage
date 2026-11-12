# 27B development measurement — 2026-11-12

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on
`qwen35-27b`, the retained label for the existing read-only artifact. The campaign
spans November 11–12 UTC; the date below records completion. **Partial path:
Rust source selection, credential-owning chat and strict edit checking. Task
first-attempt and within-budget passes remain unavailable, not zero.** Native
Git executable ownership and missing original grading controls still prevent
native execution and task grading. No candidate tests, test feedback, retries,
approvals, apply, commit, push or held-out tasks ran. Selection receives only the
statement and editable base file; full repository analysis and retrieval are not
measured. No admission control was bypassed or reconstructed. Strict proposals
are ungraded edits, never solved tasks.

Changed variables versus this artifact's November 10/11 run: complete source
lines and merged excerpts with the exact-edit instruction; total/idle 300/60 s
(was 120/30), client/relay 310 s (was 130), and two-task jobs 900 s (was 600).
Context remains 32768 tokens and selection 32768 bytes. Output allowance is
`min(32768, 32768 - rendered_input_tokens - 32)`, including reasoning. The private
effect-free adapter uses the existing chat allowance; the product proposal route
remains capped at 4096 output tokens. The unchanged 4 MiB stream-body, 5 MiB wire
and 1 MiB text ceilings can stop generation before its token allowance. Strict
refusals remain. Combined changes do not isolate causes. 32K is the largest
prepared context tested, not an exhaustive hardware-limit search. Runtime,
weights, temperature zero, seed 42 and template settings are unchanged. All 30
contexts and request messages match the current-source primary run byte for byte;
tokenization and output allowances differ by artifact.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 | 24114–29831 | unavailable / unavailable (30 tasks) | 8/30 | 7666.469 | 8072.425 | 134.540 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Complete answers reaching strict checking: 8/30. GPU work minutes mean
reserved model-job wall/60, including verification, loading and cleanup, excluding
queue and wrapper checks. Active GPU kernel time was not measured. Request/check
time excludes preparation and preflight. All 15 two-task jobs completed and
released both shared reservations. The first pair remains unchanged; this session
resumed at task 3 without rerunning or dropping an attempt. No model was downloaded.
No model or API process remains.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 1 |
| `generation-refused:error:chat.transport.timeout` | 21 |
| `ungraded-proposal` | 8 |

Unambiguous backend counters cover 9/30 tasks:
60348 input / 52900 output tokens.
One counter-covered attempt has an incomplete API response and never reaches
strict checking; only eight complete responses are checked.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `generation-refused:error:chat.transport.timeout` (21).
All six prepared backends now have complete current-source 30-task partial-path
measurements. This establishes neither task-solving improvement nor model roles.
T-303/T-306 remain open without new rows, sub-rows or completion ticks.
Exact bindings and per-task observations follow.

## Verification and retained inputs

The audit verifies 30 tasks and 15 completed jobs, frozen source/private input
hashes, model/runtime identity, effective settings, request/candidate/log bindings,
terminal events, observed counters and cleanup. The existing clean source-built
helper reproduces all 30 contexts and 8 strict checks. No new clean build or
candidate execution is claimed. A separate comparison verifies 30 contexts against
the primary run. The six-model summary verifies all six audited result identities.
No product source changed; the prior affected gate retains 53 inherited failures.
This batch makes no fresh Rust-test or full-gate claim.

The private package preserves every earlier sealed byte and manifest. The new
manifest excludes 28 newly retired loopback authentication files from the resumed
jobs; these are unnecessary for inert reproduction. The first pair
newly seals two retired loopback tokens that were absent from earlier manifests.
Those tokens belong to the reaped API with revoked settings and remain private.
Earlier sealed credentials and every sealed manifest stay unchanged. Future
packages must exclude all newly retired tokens, without a fixed task-index range.
Model weights are not packaged; raw prompts, answers,
candidates and host identities remain private. Shared inert inputs permit
independent reproduction without launching models or reading hidden tests.

Network-disabled restoration passes from empty state, on repetition and additively
over the prior 31B package. A conflicting file is refused without overwrite; a
planted directory symlink is refused with the outside directory unchanged.
Temporary cold state is removed. Restoration can add earlier missing files before
finding a conflict; atomic preflight remains T-309 backlog. The new manifest binds
97972 retained regular files. Documentation checks and exact commands
are retained with the review request; they do not establish runtime support.

| Binding | SHA-256 |
| --- | --- |
| Model weights | `5ae89d6720ba0a27f643e6fbb0d1b557870d17aeff23156e5981e863ec16985c` |
| Runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private freeze | `972c69479e9858ad0e349c020851273980a3300761eee11e23d7f42785e202e9` |
| API binary | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Original Rust helper | `0e06f7b880e0b6584a6e823249187db5e275a2c20c8a1edffa661189e1759fe0` |
| Audited results | `e4cec6c60a5ed2178ceef58d8a6098de1fefaa33fe13527c53790ff1b55aa555` |
| Source-built helper reproduction | `42f1b93cbb0c6ca6d3cbbff3047348f43d0bc3cf320fdf853971583358385697` |
| Cross-model context comparison | `02ee792b1195ab9e2ca6a004d37df47a7f21a642646200ba9ffc097698a6bd10` |
| Six-model summary | `c948a4391167af29da8d31dca22b5ac3f6045028b7d222f8ce33fa7d9fba2966` |
| Current retained manifest | `e759f7d7127700b37a68c757efa8935a5027f543025c81ea7f32154a7831269d` |
| Previous unchanged 31B manifest | `0c64857b492cf870ef8886c4059a4b45801ce753b81da274b17267dc34f24bf7` |

## Per-task observations

Each row is one attempt, zero retries and no task grade.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8221 | 24515 | 300.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-002 | 8374 | 24362 | 300.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-003 | 4263 | 28473 | 300.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-004 | 2905 | 29831 | 300.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-005 | 7127 | 25609 | 300.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-006 | 8075 | 24661 | 149.376 | 8075 / 6261 | `ungraded-proposal` |
| dev-007 | 8458 | 24278 | 300.015 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-008 | 8128 | 24608 | 300.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-009 | 2912 | 29824 | 131.007 | 2912 / 5130 | `ungraded-proposal` |
| dev-010 | 3886 | 28850 | 300.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-011 | 6035 | 26701 | 300.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-012 | 2911 | 29825 | 300.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-013 | 8467 | 24269 | 300.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-014 | 6048 | 26688 | 300.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-015 | 8622 | 24114 | 146.337 | 8622 / 5121 | `ungraded-proposal` |
| dev-016 | 8163 | 24573 | 300.015 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-017 | 4033 | 28703 | 229.524 | 4033 / 8900 | `generation-refused:error:chat.response.incomplete` |
| dev-018 | 3042 | 29694 | 300.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-019 | 8281 | 24455 | 41.280 | 8281 / 1033 | `ungraded-proposal` |
| dev-020 | 7434 | 25302 | 69.234 | 7434 / 2691 | `ungraded-proposal` |
| dev-021 | 7697 | 25039 | 300.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-022 | 6814 | 25922 | 300.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-023 | 7603 | 25133 | 300.016 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-024 | 5111 | 27625 | 300.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-025 | 4126 | 28610 | 300.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-026 | 4794 | 27942 | 49.056 | 4794 / 1866 | `ungraded-proposal` |
| dev-027 | 8076 | 24660 | 300.031 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-028 | 8201 | 24535 | 254.290 | 8201 / 10189 | `ungraded-proposal` |
| dev-029 | 7996 | 24740 | 296.126 | 7996 / 11709 | `ungraded-proposal` |
| dev-rust-001 | 3387 | 29349 | 300.006 | unknown | `generation-refused:error:chat.transport.timeout` |

### Observed causes across the completed six-model campaign

The [scoreboard](coding-scoreboard.md) retains each model’s individual entry.
The same 30 development tasks contribute 180 one-attempt observations at source
`42612ca9787f07e4df79402e6d92f39f578e5c67`. Each model's entry reports its own budgets, timing,
counters and limitations. This summary covers only the available partial path;
no task pass rate, retry result, held-out score or role selection is inferred.

| Outcome / cause | Attempts |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 2 |
| `generation-refused:error:chat.response.invalid` | 1 |
| `generation-refused:error:chat.transport.limit` | 37 |
| `generation-refused:error:chat.transport.timeout` | 45 |
| `strict-edit-refused:code.proposal.edit` | 16 |
| `strict-edit-refused:code.proposal.format` | 32 |
| `ungraded-proposal` | 47 |

Largest observed failure: `generation-refused:error:chat.transport.timeout`
(45). Under measure-first, the next
source work targets that generation failure within existing safety and resource
boundaries. Further limit, deadline, cancellation or bookkeeping work stays in
T-309 backlog unless an open high/medium finding or the row's done-when requires
it. Native execution and original grading-control admission remain blocked;
these observations do not replace end-to-end task measurements.
