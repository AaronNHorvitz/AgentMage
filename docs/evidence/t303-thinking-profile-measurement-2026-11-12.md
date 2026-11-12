# Thinking-disabled development experiment — 2026-11-12

All 30 frozen development tasks ran once on each of two controlled model
profiles at product source `42612ca9787f07e4df79402e6d92f39f578e5c67`.
The measurement checkpoint is `f619ec3cdb681bc399bb779285f837332f76d3b2`;
the intervening commits change documentation only. **Partial path: Rust source
selection, credential-owning chat and strict edit checking. First-attempt and
within-budget task passes remain unavailable, not zero.** Native Git executable
ownership and missing original grading controls still block execution and grading.
No candidate tests, feedback, retries, approvals, apply, commit, push or held-out
tasks ran. Selection receives only the statement and editable base file; full
repository analysis and retrieval are not measured. No control was bypassed or
reconstructed. A strict proposal is an ungraded edit, never a solved task.

Changed variable versus each profile's previous complete development measurement:
`chat_template_kwargs` changes from `{}` to `{"enable_thinking":false}`.
This is a candidate experiment; no product profile or model role is adopted.
All 60 contexts and request bodies match their corresponding previous run except
the output allowance computed from template-rendered input. Rendering and the
resulting output reservation may change. Model/runtime bytes, decoding
(temperature zero, seed 42), prompts, strict checker and safety remain unchanged.
Context remains 32768 tokens, source-selection ceiling 32768 bytes, and output
allowance `min(32768, 32768 - rendered_input_tokens - 32)`, including reasoning.
32K is the largest prepared context tested, not an exhaustive hardware-limit
search. The effect-free adapter uses the existing chat allowance; the product
proposal route remains capped at 4096 output tokens. Total/idle time remains
300/60 s, client/relay 310 s, and each two-task reservation 900 s. Unchanged
4 MiB stream-body, 5 MiB wire and 1 MiB text ceilings can stop generation before
the token allowance. The earlier timeout audit did not prove reasoning was the
cause; this single repeat does not establish task-solving quality or generalization.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 | 24150–29867 | unavailable / unavailable (30 tasks) | 30/30 | 1132.698 | 1462.580 | 24.376 |
| 2026-11-12 | gemma4-31b | 32768 | 22996–29577 | unavailable / unavailable (30 tasks) | 0/30 | 1814.415 | 2337.099 | 38.952 |

One attempt per task and zero retries. GPU work minutes mean reserved model-job
wall/60, including verification, loading and cleanup, excluding queues and wrapper
checks. Active GPU kernel time was not measured. Request/check time excludes
preparation and preflight. All 30 two-task reservations completed and released
both shared slots. No model was downloaded; no model or API process from this
experiment remains.
API usage was not requested; unambiguous backend counters are reported below.

Largest individual failure in these 60 observations: `strict-edit-refused:code.proposal.format` (30).
The next generation/proposal work follows this measured failure; further limits,
deadlines, cancellation and bookkeeping remain T-309 backlog unless an open
high/medium finding or the row's own completion requirement needs them.
T-303/T-306 remain open, without new rows, sub-rows, model-role selection or ticks.

Compared with the corresponding prior runs, timeouts change from 21/30 to 0/30
on 27B and from 22/30 to 0/30 on 31B. Strict ungraded proposals change from
8/30 to 30/30 and from 0/30 to 0/30, respectively. These are generation and
format observations; task correctness remains unknown. No model role or product
profile is adopted from this comparison.

## qwen35-27b observations

| Outcome / cause | Tasks |
| --- | --- |
| `ungraded-proposal` | 30 |

Does-not-fit tasks: 0. Unambiguous counters cover 30/30 tasks: 188110 input / 35780 output tokens; API usage was not requested.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8185 | 24551 | 44.909 | 8185 / 1071 | `ungraded-proposal` |
| dev-002 | 8338 | 24398 | 30.906 | 8338 / 1035 | `ungraded-proposal` |
| dev-003 | 4227 | 28509 | 30.066 | 4227 / 723 | `ungraded-proposal` |
| dev-004 | 2869 | 29867 | 52.216 | 2869 / 2161 | `ungraded-proposal` |
| dev-005 | 7091 | 25645 | 23.289 | 7091 / 350 | `ungraded-proposal` |
| dev-006 | 8039 | 24697 | 28.107 | 8039 / 936 | `ungraded-proposal` |
| dev-007 | 8422 | 24314 | 51.630 | 8422 / 1537 | `ungraded-proposal` |
| dev-008 | 8092 | 24644 | 77.996 | 8092 / 3013 | `ungraded-proposal` |
| dev-009 | 2876 | 29860 | 18.583 | 2876 / 239 | `ungraded-proposal` |
| dev-010 | 3850 | 28886 | 12.863 | 3850 / 397 | `ungraded-proposal` |
| dev-011 | 5999 | 26737 | 109.564 | 5999 / 4010 | `ungraded-proposal` |
| dev-012 | 2875 | 29861 | 30.641 | 2875 / 1201 | `ungraded-proposal` |
| dev-013 | 8431 | 24305 | 63.384 | 8431 / 1917 | `ungraded-proposal` |
| dev-014 | 6012 | 26724 | 47.157 | 6012 / 1791 | `ungraded-proposal` |
| dev-015 | 8586 | 24150 | 42.338 | 8586 / 1135 | `ungraded-proposal` |
| dev-016 | 8127 | 24609 | 47.190 | 8127 / 1716 | `ungraded-proposal` |
| dev-017 | 3997 | 28739 | 23.673 | 3997 / 493 | `ungraded-proposal` |
| dev-018 | 3006 | 29730 | 21.331 | 3006 / 813 | `ungraded-proposal` |
| dev-019 | 8245 | 24491 | 21.249 | 8245 / 193 | `ungraded-proposal` |
| dev-020 | 7398 | 25338 | 14.939 | 7398 / 383 | `ungraded-proposal` |
| dev-021 | 7661 | 25075 | 58.065 | 7661 / 1851 | `ungraded-proposal` |
| dev-022 | 6778 | 25958 | 46.506 | 6778 / 1808 | `ungraded-proposal` |
| dev-023 | 7567 | 25169 | 34.196 | 7567 / 829 | `ungraded-proposal` |
| dev-024 | 5075 | 27661 | 15.437 | 5075 / 492 | `ungraded-proposal` |
| dev-025 | 4090 | 28646 | 63.054 | 4090 / 2251 | `ungraded-proposal` |
| dev-026 | 4758 | 27978 | 5.797 | 4758 / 78 | `ungraded-proposal` |
| dev-027 | 8040 | 24696 | 35.391 | 8040 / 853 | `ungraded-proposal` |
| dev-028 | 8165 | 24571 | 20.389 | 8165 / 595 | `ungraded-proposal` |
| dev-029 | 7960 | 24776 | 34.285 | 7960 / 803 | `ungraded-proposal` |
| dev-rust-001 | 3351 | 29385 | 27.546 | 3351 / 1106 | `ungraded-proposal` |

## gemma4-31b observations

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.format` | 30 |

Does-not-fit tasks: 0. Unambiguous counters cover 30/30 tasks: 214782 input / 47482 output tokens; API usage was not requested.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9502 | 23234 | 59.009 | 9502 / 1100 | `strict-edit-refused:code.proposal.format` |
| dev-002 | 9623 | 23113 | 32.270 | 9623 / 815 | `strict-edit-refused:code.proposal.format` |
| dev-003 | 4688 | 28048 | 131.355 | 4688 / 3814 | `strict-edit-refused:code.proposal.format` |
| dev-004 | 3159 | 29577 | 95.484 | 3159 / 3327 | `strict-edit-refused:code.proposal.format` |
| dev-005 | 8396 | 24340 | 39.111 | 8396 / 411 | `strict-edit-refused:code.proposal.format` |
| dev-006 | 9339 | 23397 | 41.438 | 9339 / 1159 | `strict-edit-refused:code.proposal.format` |
| dev-007 | 9726 | 23010 | 80.218 | 9726 / 1910 | `strict-edit-refused:code.proposal.format` |
| dev-008 | 9426 | 23310 | 117.277 | 9426 / 3771 | `strict-edit-refused:code.proposal.format` |
| dev-009 | 3347 | 29389 | 26.089 | 3347 / 190 | `strict-edit-refused:code.proposal.format` |
| dev-010 | 4251 | 28485 | 17.472 | 4251 / 437 | `strict-edit-refused:code.proposal.format` |
| dev-011 | 6620 | 26116 | 111.886 | 6620 / 3066 | `strict-edit-refused:code.proposal.format` |
| dev-012 | 3181 | 29555 | 55.405 | 3181 / 1882 | `strict-edit-refused:code.proposal.format` |
| dev-013 | 9740 | 22996 | 89.870 | 9740 / 2186 | `strict-edit-refused:code.proposal.format` |
| dev-014 | 6931 | 25805 | 109.493 | 6931 / 3788 | `strict-edit-refused:code.proposal.format` |
| dev-015 | 9531 | 23205 | 33.225 | 9531 / 270 | `strict-edit-refused:code.proposal.format` |
| dev-016 | 9054 | 23682 | 42.669 | 9054 / 1234 | `strict-edit-refused:code.proposal.format` |
| dev-017 | 4484 | 28252 | 36.950 | 4484 / 537 | `strict-edit-refused:code.proposal.format` |
| dev-018 | 3455 | 29281 | 27.530 | 3455 / 834 | `strict-edit-refused:code.proposal.format` |
| dev-019 | 9325 | 23411 | 31.862 | 9325 / 211 | `strict-edit-refused:code.proposal.format` |
| dev-020 | 8400 | 24336 | 19.527 | 8400 / 412 | `strict-edit-refused:code.proposal.format` |
| dev-021 | 8677 | 24059 | 79.042 | 8677 / 1675 | `strict-edit-refused:code.proposal.format` |
| dev-022 | 7690 | 25046 | 49.101 | 7690 / 1297 | `strict-edit-refused:code.proposal.format` |
| dev-023 | 8802 | 23934 | 55.621 | 8802 / 1078 | `strict-edit-refused:code.proposal.format` |
| dev-024 | 5880 | 26856 | 21.432 | 5880 / 596 | `strict-edit-refused:code.proposal.format` |
| dev-025 | 4747 | 27989 | 171.553 | 4747 / 5480 | `strict-edit-refused:code.proposal.format` |
| dev-026 | 5331 | 27405 | 9.633 | 5331 / 176 | `strict-edit-refused:code.proposal.format` |
| dev-027 | 9388 | 23348 | 59.750 | 9388 / 1128 | `strict-edit-refused:code.proposal.format` |
| dev-028 | 9448 | 23288 | 78.121 | 9448 / 2556 | `strict-edit-refused:code.proposal.format` |
| dev-029 | 9010 | 23726 | 52.084 | 9010 / 946 | `strict-edit-refused:code.proposal.format` |
| dev-rust-001 | 3631 | 29105 | 39.940 | 3631 / 1196 | `strict-edit-refused:code.proposal.format` |

## Verification and bindings

The audit checks all frozen source/private inputs, task/request/candidate/log
bindings, model/runtime identity, effective settings, served context, backend
counters and cleanup. The unchanged helper independently reproduces every context
and every available strict check; no new clean build is claimed. The comparison
checks its prior inputs against the unchanged reviewed campaign manifest.
Network-disabled cold reproduction validates every packaged byte before writing
fresh temporary state, reproduces both audits, comparison and helper checks,
and removes temporary state. No candidate code executes. Retired client/control
tokens are excluded for every job, independent of task index. Model weights are
not packaged. Raw prompts, answers, candidates and host identities remain private.
The original packages and sealed manifests remain unchanged.

Independent timeout-review finding M1 is corrected in `6a8b4897`: the published
manifest hash now equals the unchanged archived manifest. Its retained review
still says FINDINGS; independent re-review is required. This batch makes no fresh
Rust-test, full-gate or supply-chain-regeneration claim. The 53 inherited
Rust-gate failures remain. Neither measurements nor documentation checks establish
native execution, functional task improvement, qualification or row closure.

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private freeze | `b2dc07a9144b948d027a002dd9de70258ec180b0e9372af22e64075c5609d7dd` |
| API binary | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Original Rust helper | `0e06f7b880e0b6584a6e823249187db5e275a2c20c8a1edffa661189e1759fe0` |
| 27B audited results | `d4c50d91258f1fc028de3709d8badca2eabdd41c2d9ffad98e3ec95597c813b2` |
| 31B audited results | `aa4ac58158954d450b28ecd648d314c6195f75387974e3181a4a3500b923d36a` |
| Context and strict-check reproduction | `8b71b0adb4d0d95b1117e51790f19ab116b1ed073a5b34d64515287dced88de0` |
| Prior comparison | `65cfdcc53609f1ac534330ea0e76221b3809f75474ff4bd281150e2e4e103018` |
| Unchanged original campaign manifest | `e759f7d7127700b37a68c757efa8935a5027f543025c81ea7f32154a7831269d` |
| Experiment retained manifest | `b8528e59584db3f12903bc787910b00cd8e0725beba2b449cef04da1c67a94e5` |
| Inert reproduction archive | `e50db1fc3218c19c46b60a6e4965fd43b4ec339d9b79be855e21da5bbf584b1a` |
| Cold-check script | `77c2abf3da7dda69bd14898fad272fae8fa15f5247c3f23174de2f13a2707d8d` |
| Cold-check log | `38e7727c09a628311f895636e56beab9af4a1f0d4906921c74677922064ac9ca` |
| qwen35-27b model weights | `5ae89d6720ba0a27f643e6fbb0d1b557870d17aeff23156e5981e863ec16985c` |
| qwen35-27b runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
| gemma4-31b model weights | `05e0fa1452e73e1b43bdfd378b82b488f57eb63db2bc5e4209f458e08d0542a7` |
| gemma4-31b runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
