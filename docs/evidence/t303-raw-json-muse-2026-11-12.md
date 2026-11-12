# Raw JSON instruction Muse development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `muse`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the fourth prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains `reasoning_strength:medium` in its template kwargs; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | muse | 32768 | 24395–29826 | unavailable / unavailable (30 tasks) | 19/30 | 4711.068 | 5284.175 | 88.070 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 4 |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 4 |
| `ungraded-proposal` | 19 |

Does-not-fit tasks: 0. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 26/30 tasks: 166355 input / 150086 output tokens. Other counters remain unknown; API usage was not requested.

Strict ungraded proposals change from the preceding published full Muse measurement's 18/30 to 19/30 in this repeat. This new instruction repeat reports current-source observations, without isolating the instruction from changes to excerpt delivery or rendered output reservation. It establishes neither task correctness, generalization nor a model role. No new private comparison receipt is claimed.

Largest individual refusal count in this repeat: 4; tied causes: generation-refused:error:chat.transport.timeout, strict-edit-refused:code.proposal.format. Across the four current-source model repeats, the 31B profile's 25 format refusals remain the largest individual refusal cause. Two prepared current-source repeats (gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

## Retained reproduction

The additive Muse auditor requires its observed, unchanged `reasoning_strength:medium` template setting; the original immutable auditor incorrectly expected empty kwargs for this profile. No request, strict check or acceptance rule changes. The retained audit verifies every completed reservation, task, frozen source/private hash, context and request, candidate/log binding, terminal event, observed counter and cleanup receipt. The unchanged source-built helper reproduces all 30 contexts and available strict checks without executing candidates. All previously sealed GPT-OSS inputs are checked unchanged before packaging; earlier archives and manifests remain untouched. The additive archive excludes all 30 retired client/control token files from this repeat, without a numeric range filter, and contains no model weight. Raw model text, source projections and host data remain private.

Cold verification starts from an empty temporary directory, checks every member before extraction, disables networking, reproduces the audit, all 30 contexts and available strict checks, and removes temporary state. The optional helper branch uses the same source-built inference helper to test that branch; it is not a second independent build. The reviewer bootstrap prepares hash-verified own-source inputs for a fresh offline helper build. Artifact audit, packaging and cold checks are bounded file I/O and effect-free transformations at nice 10, without compiling code, running a project/candidate test or using a model. Every inference job uses both shared reservation wrappers.

No product source changed in this batch. The source batch's whole affected Rust gate remains 1,021 passed, 53 inherited failed and 11 ignored over 29 targets; it is not green. This repeat changes no failure, threshold or independent-review boundary. The documentation fast gate and direct Markdown checks cover this documentation increment. No native workflow, task grade, model qualification, row closure, release or owner readiness is claimed. Independent core review is required.

## Per-task observations

| Model | Task | Outcome / cause | Input tokens | Output allowance | Observed output tokens | Request/check s |
| --- | --- | --- | --- | --- | --- | --- |
| muse | dev-001 | `strict-edit-refused:code.proposal.edit` | 8120 | 24616 | 8203 | 193.482 |
| muse | dev-002 | `ungraded-proposal` | 8227 | 24509 | 5864 | 136.263 |
| muse | dev-003 | `strict-edit-refused:code.proposal.edit` | 4298 | 28438 | 5527 | 129.930 |
| muse | dev-004 | `generation-refused:error:chat.transport.timeout` | 2931 | 29805 | unknown | 300.018 |
| muse | dev-005 | `ungraded-proposal` | 6622 | 26114 | 3792 | 93.092 |
| muse | dev-006 | `ungraded-proposal` | 7898 | 24838 | 5183 | 119.429 |
| muse | dev-007 | `ungraded-proposal` | 8324 | 24412 | 8073 | 189.562 |
| muse | dev-008 | `strict-edit-refused:code.proposal.format` | 7987 | 24749 | 10539 | 237.870 |
| muse | dev-009 | `ungraded-proposal` | 2910 | 29826 | 2274 | 57.441 |
| muse | dev-010 | `ungraded-proposal` | 3911 | 28825 | 5122 | 114.876 |
| muse | dev-011 | `generation-refused:error:chat.transport.timeout` | 5943 | 26793 | unknown | 300.011 |
| muse | dev-012 | `generation-refused:error:chat.transport.timeout` | 2941 | 29795 | unknown | 300.007 |
| muse | dev-013 | `strict-edit-refused:code.proposal.format` | 8333 | 24403 | 12578 | 291.475 |
| muse | dev-014 | `ungraded-proposal` | 5914 | 26822 | 7975 | 180.655 |
| muse | dev-015 | `ungraded-proposal` | 8341 | 24395 | 4413 | 110.334 |
| muse | dev-016 | `ungraded-proposal` | 7888 | 24848 | 6489 | 150.487 |
| muse | dev-017 | `ungraded-proposal` | 3987 | 28749 | 4125 | 100.076 |
| muse | dev-018 | `ungraded-proposal` | 3008 | 29728 | 5387 | 120.575 |
| muse | dev-019 | `ungraded-proposal` | 8121 | 24615 | 869 | 29.982 |
| muse | dev-020 | `ungraded-proposal` | 7308 | 25428 | 2914 | 69.449 |
| muse | dev-021 | `ungraded-proposal` | 7558 | 25178 | 8192 | 192.738 |
| muse | dev-022 | `strict-edit-refused:code.proposal.format` | 6715 | 26021 | 8371 | 191.136 |
| muse | dev-023 | `ungraded-proposal` | 7619 | 25117 | 4066 | 100.342 |
| muse | dev-024 | `ungraded-proposal` | 5103 | 27633 | 2797 | 65.121 |
| muse | dev-025 | `strict-edit-refused:code.proposal.edit` | 4109 | 28627 | 7631 | 177.592 |
| muse | dev-026 | `ungraded-proposal` | 4704 | 28032 | 1967 | 46.416 |
| muse | dev-027 | `generation-refused:error:chat.transport.timeout` | 7971 | 24765 | unknown | 300.014 |
| muse | dev-028 | `strict-edit-refused:code.proposal.format` | 8131 | 24605 | 7426 | 169.764 |
| muse | dev-029 | `ungraded-proposal` | 7852 | 24884 | 3281 | 84.418 |
| muse | dev-rust-001 | `ungraded-proposal` | 3367 | 29369 | 7028 | 158.514 |

## Bindings

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private-input freeze | `38497c8ec4966457213a30c86c08f3afbbb48e13269a507f2582032bb10b3086` |
| Generation instruction module | `ca63ed0a78b9799030b8c2ab5ba7c5be59635b4c8f58b26274654d98d26c7329` |
| Fresh source helper | `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba` |
| Unchanged chat API | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Audited results | `fadb756145d150baf52be6c185a670624ba74bc9169dea94f5c24b9cb71f89ea` |
| Context/check reproduction | `35a4df98ed29d71405fb65aac9339733b2672b10d67c6095169c3755f4f4f715` |
| Retained manifest | `92270a0c8ec148feea305c843cffb3ca82e3ab18ae8fc4f1e883be80249a0ce7` |
| Private archive | `5f27e657019db58cad064a59311c1273af341e385d6372d8333b7aeb4c67123c` |

## Independent review clarification — 2026-11-12

The [independent Muse measurement review](../reviews/fc035e714fdb-raw-json-muse-measurement.txt)
returns **Verdict: PASS** for exact commit
`fc035e714fdb8f30aba3e77f3d7d7c95e4cb4f7f`, with no high or medium finding.
Low L1 clarifies the effective output limit: the 300-second total timeout cut
four attempts at about 12,955–13,531 generated tokens, below their published
token allowances. All four backend stops report cancellation without context
truncation. At the observed rate, the time limit bounds this profile's effective
output near 13.0–13.5K tokens; the token allowance alone overstates its room.
The builder rechecked all four retained stop records and visible text sizes.
Counts and earlier entries remain unchanged. A timeout change and low L2's
private-archive hygiene remain T-309 backlog. No row closes.
