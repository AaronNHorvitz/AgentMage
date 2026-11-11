# Larger-budget development measurement — 2026-11-10/11

All 30 frozen development tasks were attempted once on each of six existing local
models at source `33ae305a3bc0bca71f3dbdee3dc6f79a4c5eddf3`.
The [scoreboard](coding-scoreboard.md#larger-budget-development-measurement-2026-11-1011)
contains the aggregate results and exact refusal causes. **There are no task pass
rates for this run.** Missing original grading controls refuse admission before
candidate execution; strict-edit acceptance is not a test pass.

## Executed scope and limits

The complete native coding workflow did not run. A rebuilt current API made 210
model-independent preparation probes: all 30 frozen bases for each of seven local
artifact labels. Every probe refused with `linux.repository.git_artifact.invalid`
before worktree or model dispatch. The lane maps system Git artifacts to a
non-root owner, contrary to native admission's existing root-ownership policy.
No trust check, executable, sandbox or permission rule was changed or bypassed.
These 210 probes are not model attempts or coding failures.

The measured partial path uses a private effect-free helper calling the existing
`prepare_coding_proposal` and `check_coding_proposal` Rust library functions and
unchanged instruction, plus the existing credential-owning `/v1/chat` endpoint.
Context selection receives only each frozen task statement and its editable base
file. It does not perform a full repository analysis. The public proposal route
still caps output at 4,096 tokens; this measurement adapter does not increase that
product limit or ship a new product capability. Requests use a fresh conversation.
Only completed `stop` answers reach the strict checker. Accepted edits are saved
as inert candidate projections; refused attempts retain explicitly labelled base
fallbacks. Candidate code is never run. Worktree editing, confined tests, test
feedback, retries, original-owner approvals, apply, commit, push and durable native
recovery are not measured by this path.

Context is 32,768 tokens; application message selection is capped at 32,768
**bytes**, not tokens. The backend's own template/tokenizer preflight sets output
allowance to `min(32768, 32768 - rendered_input_tokens - 32)`, including reasoning.
This is backend token preflight, not independently verified native admission.
All observed inputs fit; none is counted as a model failure for not fitting.
32K is the largest prepared context tested here, not an exhaustive hardware-limit
search. Transport total/idle limits remain 120/30 seconds, with all existing
stream, byte and answer caps unchanged. A `chat.transport.limit` event does not
identify which transport sublimit was hit. No failure was retried or tuned away.

Every GPU job used both shared build and GPU reservations, at most 600 seconds,
two tasks per fresh model load, releasing the reservation between pairs. Ninety
jobs completed, with sampled total VRAM capped at 22,528 MiB. Models, API children,
keys and private sockets were cleaned up in every completed job. No weights were
downloaded or modified. One queue replaced two controllers between completed
jobs to avoid occupying both build slots while waiting; no dispatched attempt
was stopped or replayed. The failed initial launcher required an optional device
to remain absent; it failed before weights or any request. Its log and original
freeze remain retained. The corrected device guard was frozen before inference.
A preliminary native diagnostic used an older API and an invalid dotted task ID;
the rebuilt 210-probe matrix supersedes it and both logs remain retained.

## Models and bindings

All models use the existing runtime, temperature zero, seed 42, four threads,
256/128 batch/microbatch, Q8 K/V cache, one parallel request, flash attention,
all GPU layers and no context shift or prompt caching. Muse and GPT-OSS retain
medium family-specific reasoning settings; the four added models use default
family settings. Backend reasoning tokens count against the output budget.
`qwen35-27b` is only a private measurement label for the observed 27B local
conversion; its exact artifact is bound below, and the label is not a model claim.
A seventh local Muse wrapper lacks verified provenance and was not used for
inference; its label appears only in the model-independent native probe matrix.
No role selection, native model admission or redistribution claim follows.

| Label | Observed weight bytes | Observed weight SHA-256 |
| --- | --- | --- |
| muse | 16756683904 | `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e` |
| gpt-oss | 12109566624 | `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901` |
| gemma4-e4b | 5405170144 | `d35a3aa7a6d47de237fd488bb6f7f3efd62908d828f85a63da0ee93acdbefe9f` |
| gemma4-26b-a4b | 17035039872 | `a07f72221e8e3f77455ab0d7f7652d01a9f63c262b954aa6932a53275a0e895a` |
| gemma4-31b | 17155893600 | `05e0fa1452e73e1b43bdfd378b82b488f57eb63db2bc5e4209f458e08d0542a7` |
| qwen35-27b | 16547400032 | `5ae89d6720ba0a27f643e6fbb0d1b557870d17aeff23156e5981e863ec16985c` |

The four additional weight files declare permissive licensing in their metadata;
the primary publisher licence pages were checked for
[Gemma 4](https://ai.google.dev/gemma/apache_2) and
[Qwen 3.8](https://huggingface.co/Qwen/Qwen3.8-27B/raw/main/LICENSE).
No third-party implementation, prompt template text or task material was copied.

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
| Pre-inference source/private-input freeze | `35f6da9ba413bf0fb054bf74fc4e139d9d8a70b015f91e10d061987eb4d09c02` |
| Current measured API binary | `6d5f23e11786908dc00bc11771c92abac3146ba18ba999d747534f527326ab70` |
| Original native helper binary | `4da3f97367709201d1396bf0a3ccf3c6ac7a00c9ec30dd597f6c1a2a39ab5d47` |
| Audited six-model results | `0c9006b3ce3d5499add3f0895086c4bfcb59211b208b552217b2f206053e01a0` |
| Full native context/check reproduction | `b835b91d612df08731840ce360828b69b1a93e03e1b453aa96ff7f5ea524a896` |
| Complete retained manifest | `db75967cb49c049cd99103971ded3f01cf0e0a976cc02508bce0cf9b7f022c46` |

The retained manifest binds 117972 whole files in both private roots.
Original requests, actual API events, backend slices/full logs, source snapshots,
candidates, launchers, policies, model/runtime identities, cleanup and admission
refusals stay private. No host identity, credential or task source is published.
The original accepted complete-controls receipt and its logs are absent from lane
retention. Actual admission probes for all six collections exit 2 with
`evaluation.io-failed` / `FileNotFoundError`, before hidden tests or candidates
execute. No synthetic receipt was substituted, no gate relaxed and no held-out
controls regenerated. First and within-budget grades remain null, not zero.

## Verification and reproduction

The full artifact audit verifies all 180 tasks, 90 jobs, whole source hashes,
policies, exact contexts/output budgets, API terminals, source/candidate/log
bindings, observed counters and cleanup. A separate reproduction regenerates all
180 contexts and every completed strict check with the original native Rust
helper, compares the actual API text to each answer and the exact saved candidate
projection. It does not execute a candidate or load hidden tests. Backend timing
classes alone cannot identify natural completion: cancellation may stop below
the token limit, so the actual API terminal is authoritative.

The independent review request supplies the sealed private roots and commands to
verify the manifest, rerun the audit from the exact frozen review checkout, build
a fresh helper against that checkout offline and repeat native context/check
reproduction. Review must not execute candidates, rerun GPU attempts, load hidden
tests or change the frozen suite. Source and model inputs were fixed before
inference; only reporting/verification scripts and documentation were added later.

The documentation-only per-change gate is recorded in the review request. It does
not cure the inherited broader Rust fast-gate failure: the independently reviewed
source still has 1,476 passes, 72 inherited failures and 64 ignores across 26
unfiltered targets. No source, hidden test, threshold, row or lifecycle status was
changed. The retained [prior independent review](../reviews/33ae305a3bc0-local-coding-candidate-build-deadline.txt)
is PASS with no high or medium finding; this measurement batch requires its own
core review. T-303/T-306 remain open. Further daily/batch development measurements
must continue while they are open; held-out measurement remains reserved for row
closure. No acceptance, public release or owner-test readiness is claimed.

## Per-task observations

All rows are development tasks. Each has one attempt and no grade. Input/output
counts are actual unambiguous backend timing counters; `unknown` is not zero.
Preflight tokens are the backend's rendered input, not a second usage count.
Durations cover chat and strict checking, with queue/load/preflight excluded.

| Model | Task | Preflight input | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- | --- |
| muse | dev-001 | 8255 | 24481 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-002 | 8333 | 24403 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-003 | 4635 | 28101 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-004 | 3203 | 29533 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-005 | 4052 | 28684 | 110.741 | 4052 / 4639 | `ungraded-proposal` |
| muse | dev-006 | 7921 | 24815 | 98.094 | 7921 / 4200 | `ungraded-proposal` |
| muse | dev-007 | 8315 | 24421 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-008 | 8049 | 24687 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-009 | 2854 | 29882 | 55.008 | 2854 / 2096 | `ungraded-proposal` |
| muse | dev-010 | 4127 | 28609 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-011 | 6358 | 26378 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-012 | 2910 | 29826 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-013 | 8458 | 24278 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-014 | 6209 | 26527 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-015 | 8320 | 24416 | 77.716 | 8320 / 2976 | `ungraded-proposal` |
| muse | dev-016 | 7932 | 24804 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-017 | 4384 | 28352 | 112.561 | 4384 / 4644 | `strict-edit-refused:code.proposal.format` |
| muse | dev-018 | 2951 | 29785 | 114.200 | 2951 / 4951 | `ungraded-proposal` |
| muse | dev-019 | 8152 | 24584 | 29.329 | 8152 / 708 | `ungraded-proposal` |
| muse | dev-020 | 7549 | 25187 | 53.612 | 7549 / 2175 | `ungraded-proposal` |
| muse | dev-021 | 7759 | 24977 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-022 | 6858 | 25878 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-023 | 7765 | 24971 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-024 | 5424 | 27312 | 54.272 | 5424 / 2275 | `strict-edit-refused:code.proposal.format` |
| muse | dev-025 | 4444 | 28292 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-026 | 3963 | 28773 | 45.096 | 3963 / 1831 | `ungraded-proposal` |
| muse | dev-027 | 7927 | 24809 | 104.828 | 7927 / 3900 | `ungraded-proposal` |
| muse | dev-028 | 8130 | 24606 | 120.025 | unknown | `generation-refused:error:chat.transport.timeout` |
| muse | dev-029 | 7948 | 24788 | 119.991 | 7948 / 4618 | `ungraded-proposal` |
| muse | dev-rust-001 | 3525 | 29211 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gpt-oss | dev-001 | 8309 | 24427 | 65.589 | 8309 / 12595 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-002 | 8339 | 24397 | 80.728 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-003 | 4683 | 28053 | 60.171 | 4683 / 11726 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-004 | 3258 | 29478 | 42.492 | 3258 / 9319 | `strict-edit-refused:code.proposal.format` |
| gpt-oss | dev-005 | 4222 | 28514 | 15.653 | 4222 / 2163 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-006 | 7938 | 24798 | 18.247 | 7938 / 3328 | `ungraded-proposal` |
| gpt-oss | dev-007 | 8346 | 24390 | 85.883 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-008 | 8066 | 24670 | 41.422 | 8066 / 8233 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-009 | 2957 | 29779 | 12.288 | 2957 / 1554 | `ungraded-proposal` |
| gpt-oss | dev-010 | 4201 | 28535 | 53.115 | 4201 / 11184 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-011 | 6446 | 26290 | 37.681 | 6446 / 6815 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-012 | 2970 | 29766 | 44.246 | 2970 / 9631 | `ungraded-proposal` |
| gpt-oss | dev-013 | 8508 | 24228 | 36.871 | 8508 / 6416 | `ungraded-proposal` |
| gpt-oss | dev-014 | 6299 | 26437 | 79.663 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-015 | 8513 | 24223 | 15.976 | 8513 / 1799 | `ungraded-proposal` |
| gpt-oss | dev-016 | 8115 | 24621 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| gpt-oss | dev-017 | 4443 | 28293 | 14.897 | 4443 / 1957 | `ungraded-proposal` |
| gpt-oss | dev-018 | 3005 | 29731 | 36.536 | 3005 / 7489 | `ungraded-proposal` |
| gpt-oss | dev-019 | 8264 | 24472 | 13.112 | 8264 / 860 | `ungraded-proposal` |
| gpt-oss | dev-020 | 7658 | 25078 | 29.244 | 7658 / 5401 | `ungraded-proposal` |
| gpt-oss | dev-021 | 7868 | 24868 | 88.451 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-022 | 6964 | 25772 | 82.698 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-023 | 7812 | 24924 | 24.861 | 7812 / 3934 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-024 | 5480 | 27256 | 17.752 | 5480 / 3445 | `strict-edit-refused:code.proposal.edit` |
| gpt-oss | dev-025 | 4501 | 28235 | 99.078 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-026 | 4028 | 28708 | 4.769 | 4028 / 617 | `ungraded-proposal` |
| gpt-oss | dev-027 | 7923 | 24813 | 29.973 | 7923 / 4699 | `ungraded-proposal` |
| gpt-oss | dev-028 | 8152 | 24584 | 90.729 | unknown | `generation-refused:error:chat.transport.limit` |
| gpt-oss | dev-029 | 8058 | 24678 | 18.730 | 8058 / 2416 | `generation-refused:error:chat.response.invalid` |
| gpt-oss | dev-rust-001 | 3590 | 29146 | 42.384 | 3590 / 8647 | `ungraded-proposal` |
| gemma4-e4b | dev-001 | 9716 | 23020 | 50.799 | 9716 / 2889 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-002 | 9727 | 23009 | 34.473 | 9727 / 4118 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-003 | 5295 | 27441 | 75.455 | 5295 / 7756 | `ungraded-proposal` |
| gemma4-e4b | dev-004 | 3655 | 29081 | 39.740 | 3655 / 5430 | `strict-edit-refused:code.proposal.edit` |
| gemma4-e4b | dev-005 | 4996 | 27740 | 45.772 | 4996 / 3534 | `ungraded-proposal` |
| gemma4-e4b | dev-006 | 9367 | 23369 | 8.921 | 9367 / 810 | `strict-edit-refused:code.proposal.edit` |
| gemma4-e4b | dev-007 | 9752 | 22984 | 75.275 | 9752 / 7349 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-008 | 9498 | 23238 | 24.773 | 9498 / 3128 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-009 | 3383 | 29353 | 45.289 | 3383 / 3679 | `ungraded-proposal` |
| gemma4-e4b | dev-010 | 4627 | 28109 | 50.196 | 4627 / 6799 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-011 | 7268 | 25468 | 79.626 | 7268 / 8235 | `strict-edit-refused:code.proposal.edit` |
| gemma4-e4b | dev-012 | 3266 | 29470 | 22.900 | 3266 / 2968 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-013 | 9969 | 22767 | 69.306 | 9969 / 6485 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-014 | 7466 | 25270 | 60.060 | 7466 / 7715 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-015 | 9630 | 23106 | 30.291 | 9630 / 1086 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-016 | 9138 | 23598 | 9.293 | 9138 / 967 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-017 | 5134 | 27602 | 39.883 | 5134 / 2275 | `ungraded-proposal` |
| gemma4-e4b | dev-018 | 3472 | 29264 | 28.736 | 3472 / 3744 | `ungraded-proposal` |
| gemma4-e4b | dev-019 | 9465 | 23271 | 28.178 | 9465 / 127 | `ungraded-proposal` |
| gemma4-e4b | dev-020 | 8823 | 23913 | 21.046 | 8823 / 2471 | `ungraded-proposal` |
| gemma4-e4b | dev-021 | 9062 | 23674 | 73.737 | 9062 / 7002 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-022 | 8000 | 24736 | 30.046 | 8000 / 3633 | `ungraded-proposal` |
| gemma4-e4b | dev-023 | 9136 | 23600 | 49.187 | 9136 / 3769 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-024 | 6387 | 26349 | 22.280 | 6387 / 2674 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-025 | 5250 | 27486 | 30.035 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-e4b | dev-026 | 4684 | 28052 | 22.212 | unknown | `ungraded-proposal` |
| gemma4-e4b | dev-027 | 9361 | 23375 | 28.769 | 9361 / 697 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-028 | 9492 | 23244 | 39.907 | 9492 / 4802 | `strict-edit-refused:code.proposal.format` |
| gemma4-e4b | dev-029 | 9288 | 23448 | 52.871 | 9288 / 4242 | `ungraded-proposal` |
| gemma4-e4b | dev-rust-001 | 4064 | 28672 | 75.885 | 4064 / 9572 | `strict-edit-refused:code.proposal.format` |
| gemma4-26b-a4b | dev-001 | 9716 | 23020 | 30.036 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-002 | 9727 | 23009 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-003 | 5295 | 27441 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-004 | 3655 | 29081 | 118.660 | unknown | `generation-refused:error:chat.transport.limit` |
| gemma4-26b-a4b | dev-005 | 4996 | 27740 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-006 | 9367 | 23369 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-007 | 9752 | 22984 | 120.016 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-008 | 9498 | 23238 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-009 | 3383 | 29353 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-010 | 4627 | 28109 | 120.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-011 | 7268 | 25468 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-012 | 3266 | 29470 | 120.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-013 | 9969 | 22767 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-014 | 7466 | 25270 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-015 | 9630 | 23106 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-016 | 9138 | 23598 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-017 | 5134 | 27602 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-018 | 3472 | 29264 | 120.006 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-019 | 9465 | 23271 | 30.031 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-020 | 8823 | 23913 | 120.016 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-021 | 9062 | 23674 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-022 | 8000 | 24736 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-023 | 9136 | 23600 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-024 | 6387 | 26349 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-025 | 5250 | 27486 | 30.031 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-026 | 4684 | 28052 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-027 | 9361 | 23375 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-028 | 9492 | 23244 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-029 | 9288 | 23448 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-26b-a4b | dev-rust-001 | 4064 | 28672 | 120.006 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-001 | 9716 | 23020 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-002 | 9727 | 23009 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-003 | 5295 | 27441 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-004 | 3655 | 29081 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-005 | 4996 | 27740 | 120.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-006 | 9367 | 23369 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-007 | 9752 | 22984 | 120.019 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-008 | 9498 | 23238 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-009 | 3383 | 29353 | 120.026 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-010 | 4627 | 28109 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-011 | 7268 | 25468 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-012 | 3266 | 29470 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-013 | 9969 | 22767 | 120.015 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-014 | 7466 | 25270 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-015 | 9630 | 23106 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-016 | 9138 | 23598 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-017 | 5134 | 27602 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-018 | 3472 | 29264 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-019 | 9465 | 23271 | 93.194 | 9465 / 2071 | `strict-edit-refused:code.proposal.format` |
| gemma4-31b | dev-020 | 8823 | 23913 | 120.023 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-021 | 9062 | 23674 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-022 | 8000 | 24736 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-023 | 9136 | 23600 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-024 | 6387 | 26349 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-025 | 5250 | 27486 | 30.032 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-026 | 4684 | 28052 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-027 | 9361 | 23375 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-028 | 9492 | 23244 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-029 | 9288 | 23448 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| gemma4-31b | dev-rust-001 | 4064 | 28672 | 120.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-001 | 8500 | 24236 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-002 | 8522 | 24214 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-003 | 4771 | 27965 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-004 | 3323 | 29413 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-005 | 4278 | 28458 | 120.023 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-006 | 8133 | 24603 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-007 | 8529 | 24207 | 120.017 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-008 | 8248 | 24488 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-009 | 3014 | 29722 | 120.007 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-010 | 4256 | 28480 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-011 | 6662 | 26074 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-012 | 3011 | 29725 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-013 | 8763 | 23973 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-014 | 6569 | 26167 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-015 | 8751 | 23985 | 111.019 | 8751 / 4013 | `ungraded-proposal` |
| qwen35-27b | dev-016 | 8298 | 24438 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-017 | 4616 | 28120 | 120.013 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-018 | 3136 | 29600 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-019 | 8423 | 24313 | 45.730 | 8423 / 1112 | `ungraded-proposal` |
| qwen35-27b | dev-020 | 7850 | 24886 | 83.984 | 7850 / 3292 | `ungraded-proposal` |
| qwen35-27b | dev-021 | 8082 | 24654 | 120.018 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-022 | 7127 | 25609 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-023 | 7983 | 24753 | 120.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-024 | 5610 | 27126 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-025 | 4628 | 28108 | 120.010 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-026 | 4250 | 28486 | 44.387 | 4250 / 1767 | `ungraded-proposal` |
| qwen35-27b | dev-027 | 8051 | 24685 | 120.014 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-028 | 8266 | 24470 | 120.009 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-029 | 8266 | 24470 | 120.011 | unknown | `generation-refused:error:chat.transport.timeout` |
| qwen35-27b | dev-rust-001 | 3769 | 28967 | 120.008 | unknown | `generation-refused:error:chat.transport.timeout` |
