# Trusted model timeouts and development repeat — 2026-11-11

Source: `6bb0008ae6ca5ad491fedd02c53c8f52e1781b1f`. Decision 0202 exposes the transport's existing
budgets through control-authenticated HTTP settings. Omitted values preserve
120000 total / 30000 idle milliseconds. Native construction validates positive
values, total at most 300000 and idle at most total, before settings publication.
Invalid choices preserve the prior configuration/revision without network I/O.
GET reports effective values without the key. Frozen coding request observations
retain the exact durations. Model-facing requests cannot override them.

This does not extend any transport ceiling. Cancellation, byte/output limits,
settings-revision checks and busy/revocation rules remain intact. Native
endpoint development launches retain their original defaults. T-303/T-306 remain
open; no row, permission, native admission or release gate closes here.

## Measurement scope

All 30 frozen development tasks ran once on the same existing GPT-OSS conversion,
weight SHA-256 `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`.
The audit requires each prepared context and actual chat request to be
byte-identical to its previous November 10/11 counterpart. Context is 32768
tokens; selection is 32768 bytes; output allowance is 24223–29779 tokens, using
`min(32768, 32768 - rendered_input_tokens - 32)`. Every input fits; does-not-fit
count is zero. Temperature zero, configured seed 42, medium reasoning, four
threads, Q8 K/V cache and 256/128 batch/microbatch settings remain unchanged.
The runtime manifest is `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1`.
Fixed configuration does not establish deterministic model responses.

Changed variables: trusted total/idle timeouts are 300/60 seconds instead of
120/30; private relay/client deadlines are 310 instead of 130 seconds to contain
the longer operation; each two-task reservation permits 900 instead of 600
seconds, including model verification/loading/cleanup. Fifteen cold model jobs
completed through both shared reservations, releasing them between pairs. No
model download, response-cap increase or second attempt occurred. Other five
previously measured models were not rerun; this is not the complete bake-off.

The measured path remains Rust context selection from the task and editable base
file, the credential-owning chat API and Rust strict edit checking through the
same private effect-free adapter. The product proposal route still caps output
at 4096 tokens. No native worktree, command/test, feedback, retry, approval, apply,
commit, push or durable recovery workflow ran. Native executable ownership and
missing original grading controls remain blockers. The new private policy retains
historical native-inventory notes; no new seven-model preparation matrix ran.
No candidate/hidden test or held-out task was executed or used for tuning.

**First-attempt and within-budget task passes are unavailable, not zero.**
The strict accepted count is 12/30, the same count as
its prior partial-path run. Those are ungraded proposals, not solved tasks.
This repeat demonstrates no increase in that count and cannot establish an
effect on task-solving quality. No model role or coding admission is selected.

| Request/check s | Reserved job wall s | GPU work min | Observed token coverage | Observed input / output tokens |
| --- | --- | --- | --- | --- |
| 1428.275 | 1607.238 | 26.787 | 23/30 | 138757 / 143734 |

Request/check time excludes preparation/preflight. GPU work minutes mean reserved
model-job wall time divided by 60, including verification, loading and cleanup,
excluding queue waits and wrapper guards. They are not active GPU kernel time.
API usage was not requested. Backend counters are retained only when unambiguous;
missing counters are unknown, never zero. Responses can differ despite identical
requests, so aggregate changes are not a causal quality attribution experiment.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 1 |
| `generation-refused:error:chat.response.invalid` | 1 |
| `generation-refused:error:chat.transport.limit` | 7 |
| `strict-edit-refused:code.proposal.edit` | 8 |
| `strict-edit-refused:code.proposal.format` | 1 |
| `ungraded-proposal` | 12 |

There are zero transport timeouts in this repeat; the prior GPT-OSS run had
one timeout (dev-016, now ending in `chat.response.incomplete`). The largest individual refusal
cause is strict edit checking (8),
followed by the unchanged transport size limit
(7).
The next source work should investigate strict edit refusals; these numbers do
not authorize another deadline, byte-cap or cancellation increment. The other
models' earlier timeouts still need measurement under the selected settings.

## Verification and independent reproduction

The complete affected fast gate passes: formatting, Clippy and 269 tests across
11 unfiltered API targets, zero failures or ignores. Three new methods exercise
control-only configuration, invalid/duplicate/type/range refusal without I/O,
default restoration/revocation, exact proposal bindings, refusal of model-supplied
timeouts and actual idle/total expiry with provisional text. Progress cannot renew
total time; expiry emits an error without completed text. The separate API build
passes. The broader inherited host/Linux failures remain open; no fresh full
workspace or long documentation gate is claimed.

The full measurement audit passes all 30 tasks and 15 jobs. It verifies source
and private launch hashes, policy/model/runtime identity, context/request equality
against the prior run, every source/candidate/log binding, exact API terminal,
output budgets, counters and cleanup. Independent Rust context/check
reproduction passes 30/30, with candidate execution and hidden tests false.
A separately rebuilt offline helper also reproduces all 30 contexts/checks; its
binary hash may differ with build location. No inference attempt was replayed. Generation inputs stayed frozen; only report
and verification tooling was added after inference began.

The retained prior independent measurement review is INCONCLUSIVE because its
private inputs were unavailable, with no high or medium documentary finding.
The original sealed roots are unchanged. Their new private archive was checked
byte-for-byte against all 117972 manifest-bound files. A fresh network-disabled
sandbox with empty private state restored them and verified the same complete
manifest. The original six-model artifact audit reproduced byte-for-byte and
Rust context/check reproduction passed 180/180. A sparse source archive alone
initially failed historical Git object lookup; final commands supply `GIT_DIR`
for read-only access to existing repository objects. No check was weakened.

The exact-commit review request supplies archive hashes, bootstrap instructions,
raw retained locations, commands for both audits, fresh offline helper builds
and reproductions. Raw requests, sources, candidates and credentials remain
private. Cold-test scratch was removed. The public scoreboard and evidence
require a new independent core verdict; builder verification does not accept
these rows or the prior measurement on the reviewer's behalf.

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private launch freeze | `a00cd6ee22742fc57df3f6add0631c45dbd534e4981a047703c2eb35470e2bc2` |
| API binary | `1f9c30eaabbafd1ffcaac331627dfe8f69b63f45328fb7b192bb1d656221489b` |
| Rust helper | `4da3f97367709201d1396bf0a3ccf3c6ac7a00c9ec30dd597f6c1a2a39ab5d47` |
| Audited results | `198111f81afbd782032f9f0b56adf3819295dc8cdccd2fc3a2494e53eb2e2a28` |
| Rust reproduction | `01cf50d3780f22288489c2d604a1ef9858f9d212e96e7f12e1a5326f8ceeeabd` |

The complete current retained manifest binds 19634 files, including the
actual API binary. Manifest SHA-256:
`6741929716243eb49e8290874b871ba2961dd7d9132a8f837167de4a836e8d47`.
The private shared review archive is 156449286 bytes, SHA-256
`6f7c495839347476af7eb106dfd57d24f376997fa7f186f185082550d2e898cb`.
Fresh offline reproduction report SHA-256:
`c4e5dbc0540adccbd67382f2b2b2f638d0ef163073f3ace1d030652858ade862`.
A second empty-state, network-disabled cold restore also verified all 19634
current manifest files from the shared archive. These are review inputs, not an
independent acceptance verdict.

## Per-task observations

Each task has one attempt, zero retries and no task grade. Counts below are
unambiguous backend observations, including reasoning where reported.

| Task | Preflight input | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8309 | 24427 | 91.773 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-002 | 8339 | 24397 | 51.104 | 8339 / 9841 | `strict-edit-refused:code.proposal.format` |
| dev-003 | 4683 | 28053 | 24.640 | 4683 / 3927 | `strict-edit-refused:code.proposal.edit` |
| dev-004 | 3258 | 29478 | 40.658 | 3258 / 8282 | `strict-edit-refused:code.proposal.edit` |
| dev-005 | 4222 | 28514 | 23.262 | 4222 / 3696 | `strict-edit-refused:code.proposal.edit` |
| dev-006 | 7938 | 24798 | 29.095 | 7938 / 5390 | `strict-edit-refused:code.proposal.edit` |
| dev-007 | 8346 | 24390 | 67.758 | 8346 / 12104 | `ungraded-proposal` |
| dev-008 | 8066 | 24670 | 86.339 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-009 | 2957 | 29779 | 18.912 | 2957 / 2684 | `ungraded-proposal` |
| dev-010 | 4201 | 28535 | 45.024 | 4201 / 9063 | `strict-edit-refused:code.proposal.edit` |
| dev-011 | 6446 | 26290 | 39.969 | 6446 / 6815 | `strict-edit-refused:code.proposal.edit` |
| dev-012 | 2970 | 29766 | 46.602 | 2970 / 9631 | `ungraded-proposal` |
| dev-013 | 8508 | 24228 | 39.474 | 8508 / 6416 | `ungraded-proposal` |
| dev-014 | 6299 | 26437 | 86.940 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-015 | 8513 | 24223 | 16.014 | 8513 / 1799 | `ungraded-proposal` |
| dev-016 | 8115 | 24621 | 129.132 | 8115 / 24621 | `generation-refused:error:chat.response.incomplete` |
| dev-017 | 4443 | 28293 | 15.411 | 4443 / 1957 | `ungraded-proposal` |
| dev-018 | 3005 | 29731 | 37.001 | 3005 / 7489 | `ungraded-proposal` |
| dev-019 | 8264 | 24472 | 12.182 | 8264 / 860 | `ungraded-proposal` |
| dev-020 | 7658 | 25078 | 28.906 | 7658 / 5401 | `ungraded-proposal` |
| dev-021 | 7868 | 24868 | 90.778 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-022 | 6964 | 25772 | 85.801 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-023 | 7812 | 24924 | 26.416 | 7812 / 3934 | `strict-edit-refused:code.proposal.edit` |
| dev-024 | 5480 | 27256 | 18.801 | 5480 / 3445 | `strict-edit-refused:code.proposal.edit` |
| dev-025 | 4501 | 28235 | 88.506 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-026 | 4028 | 28708 | 4.334 | 4028 / 617 | `ungraded-proposal` |
| dev-027 | 7923 | 24813 | 30.684 | 7923 / 4699 | `ungraded-proposal` |
| dev-028 | 8152 | 24584 | 89.517 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-029 | 8058 | 24678 | 19.069 | 8058 / 2416 | `generation-refused:error:chat.response.invalid` |
| dev-rust-001 | 3590 | 29146 | 44.172 | 3590 / 8647 | `ungraded-proposal` |

The later [independent review](../reviews/d31e2969c3ab-local-coding-trusted-model-timeouts.txt)
returns **PASS** for exact commit
`d31e2969c3ab48902b2426638dc62e14cb7bd7fd`, with no high or medium finding.
It also reproduces and accepts the earlier raw measurement evidence. Its low
timeout-comparison clarification is addressed above; T-303/T-306 remain open.
