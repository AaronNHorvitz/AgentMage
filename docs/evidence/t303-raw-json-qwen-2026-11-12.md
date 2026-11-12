# Raw JSON instruction 27B development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `qwen35-27b`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the second prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains `enable_thinking:false`; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 | 24137–29804 | unavailable / unavailable (30 tasks) | 29/30 | 1385.953 | 1740.182 | 29.003 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 1 |
| `ungraded-proposal` | 29 |

Does-not-fit tasks: 0. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 29/30 tasks: 186402 input / 34920 output tokens. Other counters remain unknown; API usage was not requested.

Compared with the prior thinking-disabled 27B run, strict ungraded proposals change from 30/30 to 29/30. All 30 task statements and source outlines are unchanged; 19/30 user payloads and 19/30 excerpt selections are byte-identical. Complete launch profiles match except the private state root. The longer instruction can change selections and rendered output reservation under the same message budget. This comparison does not isolate wording from source delivery or establish task correctness, generalization or a model role.

Largest individual refusal in this repeat: generation-refused:error:chat.transport.timeout (1). Across the two current-source model repeats, the 31B profile's 25 format refusals remain the largest individual refusal cause. Four prepared current-source repeats (gpt-oss, muse, gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

## Retained reproduction

The retained audit verifies every completed reservation, task, frozen source/private hash, context and request, candidate/log binding, terminal event, observed counter and cleanup receipt. The unchanged source-built helper reproduces all 30 contexts and available strict checks without executing candidates. Comparison verifies its prior inputs against the original sealed manifest. All previously sealed 31B inputs are checked unchanged before packaging; its original archive and manifest remain untouched. The additive archive excludes all 30 retired client/control token files from this repeat, without a numeric range filter, and contains no model weight. Raw model text, source projections and host data remain private.

Cold verification starts from an empty temporary directory, checks every member before extraction, disables networking, reproduces the audit, comparison, all 30 contexts and the 29 available strict checks, and removes temporary state. The optional helper branch uses the same source-built inference helper to test that branch; it is not a second independent build. The reviewer bootstrap prepares hash-verified own-source inputs for a fresh offline helper build. Artifact audit, packaging and cold checks are bounded file I/O and effect-free transformations at nice 10, without compiling code, running a project/candidate test or using a model. Every inference job uses both shared reservation wrappers.

No product source changed in this batch. The source batch's whole affected Rust gate remains 1,021 passed, 53 inherited failed and 11 ignored over 29 targets; it is not green. This repeat changes no failure, threshold or independent-review boundary. The documentation fast gate and direct Markdown checks cover this documentation increment. No native workflow, task grade, model qualification, row closure, release or owner readiness is claimed. Independent core review is required.

## Per-task observations

| Model | Task | Outcome / cause | Input tokens | Output allowance | Observed output tokens | Request/check s |
| --- | --- | --- | --- | --- | --- | --- |
| qwen35-27b | dev-001 | `ungraded-proposal` | 8198 | 24538 | 1014 | 38.974 |
| qwen35-27b | dev-002 | `ungraded-proposal` | 8303 | 24433 | 1193 | 34.886 |
| qwen35-27b | dev-003 | `ungraded-proposal` | 4290 | 28446 | 723 | 28.689 |
| qwen35-27b | dev-004 | `generation-refused:error:chat.transport.timeout` | 2932 | 29804 | unknown | 300.008 |
| qwen35-27b | dev-005 | `ungraded-proposal` | 7154 | 25582 | 306 | 23.612 |
| qwen35-27b | dev-006 | `ungraded-proposal` | 8024 | 24712 | 958 | 28.680 |
| qwen35-27b | dev-007 | `ungraded-proposal` | 8449 | 24287 | 2777 | 79.470 |
| qwen35-27b | dev-008 | `ungraded-proposal` | 8061 | 24675 | 3104 | 76.805 |
| qwen35-27b | dev-009 | `ungraded-proposal` | 2939 | 29797 | 169 | 15.738 |
| qwen35-27b | dev-010 | `ungraded-proposal` | 3913 | 28823 | 1133 | 28.859 |
| qwen35-27b | dev-011 | `ungraded-proposal` | 6062 | 26674 | 4157 | 106.632 |
| qwen35-27b | dev-012 | `ungraded-proposal` | 2938 | 29798 | 1218 | 29.792 |
| qwen35-27b | dev-013 | `ungraded-proposal` | 8428 | 24308 | 1914 | 60.768 |
| qwen35-27b | dev-014 | `ungraded-proposal` | 6075 | 26661 | 2074 | 52.234 |
| qwen35-27b | dev-015 | `ungraded-proposal` | 8599 | 24137 | 1110 | 41.570 |
| qwen35-27b | dev-016 | `ungraded-proposal` | 8150 | 24586 | 1766 | 46.692 |
| qwen35-27b | dev-017 | `ungraded-proposal` | 4060 | 28676 | 533 | 24.204 |
| qwen35-27b | dev-018 | `ungraded-proposal` | 3069 | 29667 | 814 | 21.987 |
| qwen35-27b | dev-019 | `ungraded-proposal` | 8273 | 24463 | 168 | 19.753 |
| qwen35-27b | dev-020 | `ungraded-proposal` | 7461 | 25275 | 315 | 13.890 |
| qwen35-27b | dev-021 | `ungraded-proposal` | 7724 | 25012 | 1022 | 39.454 |
| qwen35-27b | dev-022 | `ungraded-proposal` | 6841 | 25895 | 1127 | 31.045 |
| qwen35-27b | dev-023 | `ungraded-proposal` | 7630 | 25106 | 834 | 34.331 |
| qwen35-27b | dev-024 | `ungraded-proposal` | 5138 | 27598 | 730 | 21.077 |
| qwen35-27b | dev-025 | `ungraded-proposal` | 4153 | 28583 | 2235 | 62.223 |
| qwen35-27b | dev-026 | `ungraded-proposal` | 4821 | 27915 | 132 | 7.030 |
| qwen35-27b | dev-027 | `ungraded-proposal` | 8036 | 24700 | 966 | 37.269 |
| qwen35-27b | dev-028 | `ungraded-proposal` | 8176 | 24560 | 690 | 22.955 |
| qwen35-27b | dev-029 | `ungraded-proposal` | 8023 | 24713 | 692 | 30.958 |
| qwen35-27b | dev-rust-001 | `ungraded-proposal` | 3414 | 29322 | 1046 | 26.368 |

## Bindings

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private-input freeze | `38497c8ec4966457213a30c86c08f3afbbb48e13269a507f2582032bb10b3086` |
| Generation instruction module | `ca63ed0a78b9799030b8c2ab5ba7c5be59635b4c8f58b26274654d98d26c7329` |
| Fresh source helper | `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba` |
| Unchanged chat API | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Audited results | `d1e2e1b981aafb3a048626fb553e6c02fcda9df06a964558d84b18d246e3b454` |
| Context/check reproduction | `ce8930d3ed2bbd303adce9f1452d01a8df652ac3c0c7ee10d6cc1687dc540adb` |
| Prior-profile comparison | `dc97aa33204bab9596a8cdef19451ad5550b3cce8284603cd2660f6b01b8b0e0` |
| Retained manifest | `321450f52f144d96096c9693b57028aed390ace91ba9848c4708c2a6d1c99c4c` |
| Private archive | `d35886d11bddf37577fe8ab3f72ddf87c07ea8be3150effe692904787201b79a` |
