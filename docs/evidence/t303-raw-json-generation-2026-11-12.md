# Raw JSON proposal generation — 2026-11-12

Product source: `a99fe467c72e2625022b639d8d38b72e845f612a`.

The preceding controlled development repeat's largest individual failure was
`strict-edit-refused:code.proposal.format` (30). Every retained 31B answer has
Markdown fences. A manifest-bound diagnosis of all 30 finds one JSON object
inside 26; four also contain malformed JSON or additional objects. This is
saved-answer inspection, with no new inference, candidate execution or accepted
proposal. Stripping fences is diagnostic only and never changes acceptance.

The fixed application instruction now includes a small original JSON syntax
example, explicit object boundaries, escaped string contents and refusal of
extra objects or trailing commas. The example is synthetic and does not come
from an evaluation task. Source selection and strict checking implementations
are byte-identical. The longer instruction can change excerpt coverage under
the unchanged serialized-message ceiling; the complete comparison reports that
rather than assuming identical source delivery. The checker still refuses
wrapped, malformed, ambiguous, overlapping, invisible, ineffective and stale
edits atomically. No fuzzy matching, output normalization, hidden-test edit,
threshold change, new authority or boundary relaxation is introduced.

## Verification

The full affected test command completes all 29 targets: **1,021 passed,
53 failed, 11 ignored**, zero measured or filtered. Every failure and ignore
identity matches the retained independently reviewed source baseline. This is
an inherited failing gate, not a green full gate. All nine strict-proposal
tests pass, including the new synthetic wrapping/refusal regression. Clippy
with warnings denied passes. The initial formatter failure remains retained;
the exact corrected formatter command passes. Formatting is bounded read-only
work, without compilation or a full suite. All heavy builds and tests use the
required shared reservation. Two redundant queue-only requests were stopped
only after checking their exact owned processes and confirming that no check
had started. No other worker was stopped or limit raised.

The measurement helper was freshly built offline against the committed
repository-map source. The reviewed chat-only API binary is reused unchanged;
its binary/source distinction is retained explicitly in the policy. The product
proposal API's output ceiling remains 4096. This measurement's effect-free
adapter uses the existing chat allowance, never a native execution path.

The remaining artifact-only audit runs at nice 10: 10.96 seconds wall,
9.15 CPU seconds and 432,156 KiB maximum resident memory. It hashes retained
files, reads frozen Git objects and calls the known effect-free context/edit
transformer. It runs no build, model, candidate or project test. Its original
reservation-only waiter was stopped before execution after checking the exact
owned process and its sleep-only children. Archive and cold artifact checks use
the same low-priority, bounded file-I/O approach within unchanged lane limits.

## Development measurement

Only the available partial path is measured: Rust source selection, the
credential-owning local chat API, and the unchanged strict checker. Selection
receives the statement and editable frozen base file; full repository analysis
and retrieval are not measured. The retained native executable-ownership refusal remains unresolved. A read-only
availability check confirms that the expected original grading-controls receipt
is absent; this measurement does not retry native admission or load hidden tests. First-attempt and
within-budget task passes remain **unavailable**, not zero. No candidate tests,
feedback, retries, approvals, apply, commit, push or held-out task ran. No control
was bypassed or reconstructed. A strict proposal is an ungraded edit, never a
solved task.

One attempt per task, zero retries. The changed variable is the fixed generation
instruction and any resulting context coverage and rendered output reservation.
The 31B profile retains the preceding experiment's thinking-disabled setting;
no product profile or model role is adopted. Runtime and weight bytes, complete
launch settings except the private state root, temperature zero, seed 42,
32768-token context, 32768-byte source selection, 300/60-second total/idle limits,
310-second relay/client, byte ceilings and strict checking remain unchanged.
Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`.
32K is the largest prepared context tested, not an exhaustive hardware maximum.

Every GPU reservation covers two tasks, lasts at most 900 seconds and uses both
required wrappers. GPU work minutes mean reserved model-job wall time divided
by 60, including verification, loading and cleanup, excluding queue and wrapper
checks. Active GPU kernel time was not measured. Request/check time excludes
preparation and preflight. API usage is not requested; observed backend counters
and missing counters are reported separately. No model was downloaded.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gemma4-31b | 32768 | 23002–29511 | unavailable / unavailable (30 tasks) | 4/30 | 1653.191 | 2154.301 | 35.905 |

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 1 |
| `strict-edit-refused:code.proposal.format` | 25 |
| `ungraded-proposal` | 4 |

Does-not-fit tasks: 0. One attempt per task, zero retries. No model or API process remains. All 15 two-task reservations completed and released both shared slots.

Unambiguous backend counters cover 30/30 tasks: 215953 input / 38865 output tokens. Other counters remain unknown; API usage was not requested.

Compared with the prior thinking-disabled 31B run, strict ungraded proposals change from 0/30 to 4/30. All 30 task statements and source outlines are unchanged; 19/30 user payloads and 19/30 excerpt selections are byte-identical. The longer instruction changes the remaining selections under the same message budget. Complete launch profiles match except the private state root. These observations do not isolate wording from changed source delivery or establish task correctness, generalization or a model role.

Largest individual remaining refusal is `strict-edit-refused:code.proposal.format` (25). Syntax-only inspection classifies 19 of those answers as JSON code fences and six as other JSON-format failures; none exceeds the existing 64 KiB answer ceiling. This does not normalize a response, execute a candidate or change acceptance. No further limit, deadline, cancellation or bookkeeping feature is justified by these observations.

Only this prepared profile has completed the current-source repeat. The other five prepared profiles (qwen35-27b, gpt-oss, muse, gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No product profile or model role is adopted. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

## Retained reproduction

The audit checks every task and completed reservation, source/private hashes, profiles, context and request bodies, source/candidate/log bindings, terminal events, observed counters and cleanup. The helper reproduces all 30 contexts and available strict checks without executing a candidate. Comparison inputs are checked against the original sealed manifest. Cold reproduction starts from empty temporary state, verifies every archive member before extraction, disables networking and reproduces the audit, comparison, helper checks, syntax observations and gate comparison; the temporary state is removed. The provided-helper branch is also checked with the same freshly source-built inference helper; it is not a second independent build. A separate reviewer bootstrap prepares exact own-source helper inputs for a fresh offline build and all 30 checks. Raw model text, source projections and private host data remain private. All 30 retired API client/control token files are excluded, without an index-range exclusion; no model weight is packaged. Previous sealed roots and manifests remain unchanged.

No new native executable admission, candidate grading, model qualification, row closure, public release or owner readiness is claimed. Independent core review is required.

## Per-task observations

| Model | Task | Outcome / cause | Input tokens | Output allowance | Observed output tokens | Request/check s |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | dev-001 | `strict-edit-refused:code.proposal.format` | 9504 | 23232 | 1097 | 59.056 |
| gemma4-31b | dev-002 | `strict-edit-refused:code.proposal.format` | 9566 | 23170 | 765 | 33.197 |
| gemma4-31b | dev-003 | `strict-edit-refused:code.proposal.format` | 4754 | 27982 | 1529 | 66.995 |
| gemma4-31b | dev-004 | `strict-edit-refused:code.proposal.format` | 3225 | 29511 | 878 | 26.645 |
| gemma4-31b | dev-005 | `strict-edit-refused:code.proposal.format` | 8462 | 24274 | 417 | 38.357 |
| gemma4-31b | dev-006 | `strict-edit-refused:code.proposal.format` | 9312 | 23424 | 833 | 32.295 |
| gemma4-31b | dev-007 | `strict-edit-refused:code.proposal.format` | 9734 | 23002 | 1158 | 61.397 |
| gemma4-31b | dev-008 | `strict-edit-refused:code.proposal.format` | 9374 | 23362 | 7861 | 277.550 |
| gemma4-31b | dev-009 | `strict-edit-refused:code.proposal.format` | 3413 | 29323 | 153 | 24.477 |
| gemma4-31b | dev-010 | `strict-edit-refused:code.proposal.format` | 4317 | 28419 | 389 | 15.594 |
| gemma4-31b | dev-011 | `strict-edit-refused:code.proposal.format` | 6686 | 26050 | 2891 | 108.236 |
| gemma4-31b | dev-012 | `strict-edit-refused:code.proposal.format` | 3247 | 29489 | 1279 | 40.348 |
| gemma4-31b | dev-013 | `strict-edit-refused:code.proposal.format` | 9734 | 23002 | 2178 | 92.607 |
| gemma4-31b | dev-014 | `strict-edit-refused:code.proposal.format` | 6997 | 25739 | 1193 | 42.100 |
| gemma4-31b | dev-015 | `strict-edit-refused:code.proposal.format` | 9526 | 23210 | 1087 | 58.459 |
| gemma4-31b | dev-016 | `strict-edit-refused:code.proposal.format` | 9074 | 23662 | 1243 | 44.734 |
| gemma4-31b | dev-017 | `strict-edit-refused:code.proposal.format` | 4550 | 28186 | 520 | 36.492 |
| gemma4-31b | dev-018 | `strict-edit-refused:code.proposal.format` | 3521 | 29215 | 894 | 28.514 |
| gemma4-31b | dev-019 | `ungraded-proposal` | 9352 | 23384 | 184 | 31.516 |
| gemma4-31b | dev-020 | `strict-edit-refused:code.proposal.format` | 8466 | 24270 | 408 | 20.331 |
| gemma4-31b | dev-021 | `strict-edit-refused:code.proposal.format` | 8743 | 23993 | 1278 | 64.802 |
| gemma4-31b | dev-022 | `strict-edit-refused:code.proposal.format` | 7756 | 24980 | 1857 | 68.761 |
| gemma4-31b | dev-023 | `ungraded-proposal` | 8868 | 23868 | 856 | 51.071 |
| gemma4-31b | dev-024 | `ungraded-proposal` | 5946 | 26790 | 607 | 22.860 |
| gemma4-31b | dev-025 | `strict-edit-refused:code.proposal.edit` | 4813 | 27923 | 2605 | 96.376 |
| gemma4-31b | dev-026 | `strict-edit-refused:code.proposal.format` | 5397 | 27339 | 151 | 9.759 |
| gemma4-31b | dev-027 | `strict-edit-refused:code.proposal.format` | 9376 | 23360 | 1165 | 60.777 |
| gemma4-31b | dev-028 | `strict-edit-refused:code.proposal.format` | 9467 | 23269 | 1390 | 48.373 |
| gemma4-31b | dev-029 | `strict-edit-refused:code.proposal.format` | 9076 | 23660 | 868 | 52.015 |
| gemma4-31b | dev-rust-001 | `ungraded-proposal` | 3697 | 29039 | 1131 | 39.495 |

## Bindings

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private-input freeze | `38497c8ec4966457213a30c86c08f3afbbb48e13269a507f2582032bb10b3086` |
| Generation instruction module | `ca63ed0a78b9799030b8c2ab5ba7c5be59635b4c8f58b26274654d98d26c7329` |
| Fresh source helper | `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba` |
| Unchanged chat API | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Prior answer diagnosis | `4d7f36165dc307e017aa89d02e3d5875668703da64ffcd6579fed65f8467f939` |
| Answer syntax observations | `5a7b358eb6456b82bd5f48bf87d06115baf4234a702b3941d7a3ca8a08f2a1d2` |
| Audited results | `50025f31e3398ce06e1f9ba2a306d1841068e68c14816a28a427f55d7d8afd8a` |
| Context/check reproduction | `a59a1704d992263651e956b3a2a1ab63eee28d627002969d8c10b0eb90a941f6` |
| Prior-profile comparison | `3693d574b9f7e3e546823adfe83c2150a76bbf2ca0695d76137f3b3f863385e1` |
| Whole affected test log | `81ec76d9576e87309dadbe58ed96a6dda3a7a6452a746dcb5f74a4cb42b99640` |
| Failure/ignore comparison | `60f0869d6a226fcd21b0107e21597a9d75d93584b207eb3a0b2332c284bdbfec` |
| Corrected formatter check | `47bef728a6cc7e5e5ffb304eeb75b6ffc89cd2586dd7e5a44afa35db71747ded` |
| Retained manifest | `76fda34c915e86fa89beae24f71a3e6f5fb0bebb2dd574f835e6d8025c8c43f2` |
| Private archive | `5955f58866ffaf506f553f73ef9e7a92ee34f4bc559420a02cb518e7f12c2937` |
