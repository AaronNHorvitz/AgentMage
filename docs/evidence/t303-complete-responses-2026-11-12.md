# Bounded complete model responses — 2026-11-12

T-303/T-306 remain open. This source increment implements
[Decision 0206](../decisions/0206-bounded-complete-model-responses.md).
No native admission, task grade, model role, row closure or readiness is claimed.

The preceding six-model development repeat's largest aggregate refusal cause
is stream size: 48/180 attempts. The independently reviewed A4B repeat reports
29 cuts before any visible answer. Complete upstream JSON responses provide a
way to measure the cost of stream framing while retaining existing ceilings.
They do not establish that the model will stop reasoning or finish a useful edit.

Trusted endpoint settings now accept `response_mode: complete`; omission keeps
`stream`. The transport sends `stream: false` to the same approved endpoint,
then checks one JSON completion after valid HTTP EOF. The selected transport is
immutable and each prepared request binds its actual bytes. API event contracts
stay the same, with at most one text event for complete mode. Chat without a
folder and strict proposal preparation both exercise this option. Native
development launchers retain their separate default configuration.

Wire, body, text, request, token and time ceilings are unchanged. Wrong models,
tools, refusal, malformed completion fields, truncated responses, invalid
requested usage and cancellation produce no successful exchange. A narrowed
text budget is checked before any callback. Callback cancellation revokes the
completed observation. No fallback or automatic retry occurs. Strict editing,
source selection, model profiles, evaluation inputs and hidden tests are unchanged.

Complete mode provides no intermediate progress. Idle time still bounds the
wait for a body. Measurements must disclose any explicit choice of existing
timeout values alongside the changed framing; choosing the mode adjusts neither.

## Verification

The complete affected gate covers three crates and 30 targets:
**1,160 passed, 53 failed, 19 ignored**, zero measured or filtered. The 53
failure identities exactly match the retained preceding source gate. The
additional ignored identities come from the inference targets newly included
in this gate; no ignore annotation is added. This is a failing gate, not green.

All six new socket/API tests pass. They cover actual upstream request bytes and
hash bindings, trusted settings and default preservation, client and model-body
refusals, chat and strict proposals, fragmented responses, malformed envelopes,
duplicate envelope fields, tool/refusal rejection, usage validation, body/text
ceilings, truncation, cancellation, total/idle deadlines and owner stop.
Formatting and Clippy with warnings denied pass. The initial compiler failures
remain in the raw log; the corrected complete gate above finishes all targets.
All compilation and project tests use the shared build reservation. No
supply-chain or historical evidence output is regenerated mid-batch.

The source-built local API is prepared for a resumable development repeat.
The repository-map helper can be reused only after verifying its unchanged
source and bytes. First-attempt and within-budget task passes remain unavailable:
native executable trust and the original grading-controls receipt are unresolved.
No controls may be reconstructed or admission bypassed. No held-out task runs.
Independent core review is required. Further source tuning waits for the full
prepared development repeats, except fixes for high or medium review findings.

## Measurement scheduling

Product source is `7d860097edbf2e16cae279191b0fa7ac292b1283`.
The fresh local API binary and unchanged source-selection/checking helper are
frozen for the next development campaign. Its explicit changed variables are
complete upstream responses and the existing 300-second idle choice, replacing
60 seconds; total remains 300 seconds and other ceilings remain unchanged.

The first two-task reservation waited at the shared build slot and was ended
before source-review handoff. It never acquired a GPU reservation, created a
model job or executed a task. Only its verified queued process group was stopped;
no other worker was changed. The queue log and exit 143 are retained. This is
not a model failure or a scoreboard run. All 30 tasks on every prepared model
remain pending before further source tuning, except high/medium review fixes.

## Independent review and empty-answer repair

The retained [independent core review](../reviews/05c53085b860-bounded-complete-model-responses.txt)
returns **Verdict: FINDINGS** for exact commit
`05c53085b86074ac461185c0027361230ec136e7`. Medium M1 finds that the initial
complete decoder accepts empty content, whereas streaming refuses it. The
repair returns the same incomplete-response error before emitting text or
publishing a completed exchange. Both `stop` and `length` are covered, with
and without requested usage and with ignored non-answer metadata. The same
socket matrix preserves successful nonempty text and its token-limit finish.
Independent re-review is required; no row closes. Low L1 (null error key),
L2 (preflight framing at the request-byte boundary) and L3 (response-buffer
reallocation hygiene) remain T-309 backlog.

Before the verdict arrived, two development attempts used the original binary
from `7d860097edbf2e16cae279191b0fa7ac292b1283`. Both ended with empty visible
text and `length`; the collector refused both. They produced no accepted edit,
candidate test, task grade, retry or held-out observation. They are retained
as pre-fix partial observations and cannot establish a framing comparison.
The remaining campaign must use a newly built and frozen repaired binary;
these attempts do not count toward that complete development repeat.

The first launch never reached a task: its private Unix socket path was 109
bytes and the client rejected it as too long. Its ordinary 180-second health
deadline expired after the server loaded. The failed launch and cleanup are
retained. An additive private adapter shortens only the state path; original
scripts remain unchanged. All 2,299 input hashes matched at the post-pair
audit. A hash-verified source snapshot preserves that state separately from
the later M1 source repair. The corrected launch
passed health in 21.298 seconds. Both starts used the required build and GPU
reservations, and removed their own model key and socket at exit.

| Pre-fix task | Input tokens | Output allowance | Observed output tokens | Terminal | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9502 | 23234 | 23234 | `complete:length` | 202.092 |
| dev-002 | 9564 | 23172 | 23172 | `complete:length` | 176.503 |

The pair reserved 410.633 seconds; the startup-only failure reserved another
190.465 seconds. Combined reserved GPU work is 10.018 minutes, including
verification, load and cleanup, excluding queues; active kernel time was not
measured. First-attempt and within-budget task passes remain unavailable.
The effect-free audit verifies both records, cleanup and the unchanged inputs;
the original helper reproduces both contexts without executing candidates.
The next pair waited only for a build slot and was stopped before any job or
task started. Its queue receipt is preserved. No model/API process remains.

The private retained archive verifies 3,759 files, including the exact 2,283-file
source snapshot and all 16 frozen private inputs. It excludes both retired API
tokens and contains no model weight. A streaming member/hash check reproduces
the two backend token counters, terminal events, record bindings and cost sum;
it extracts nothing and executes no candidate, test or model. The pre-fix
context check also reproduces both contexts against the frozen snapshot. The
archive and raw records stay private; the following digests identify them.

| Pre-fix retained binding | SHA-256 |
| --- | --- |
| `complete-empty-fix-20261112-prefix-inputs.tar.gz` | `6f80743612288ab2b32fb42566a98c1adf948ae1c7eb9d8a0befd03c6d4a3d45` |
| `complete-empty-fix-20261112-prefix-manifest.json` | `aff2e859a9e1a6b31fcedab876b2e251925d54509ff0034c80ddd41fc139c37d` |

The repaired source's full affected gate completes all 30 targets:
**1,161 passed, 53 failed, 19 ignored**, zero measured or filtered. All 53
failure identities exactly match the retained prior gate. This remains a
failing gate. All five focused complete-mode transport tests pass, including
the new 16-case socket matrix. Formatting and Clippy with warnings denied
pass. Direct Markdown lint checks all five changed Markdown files with zero
issues; the CLI lint's zero-file result is not counted as direct coverage.
All 507 ledger table lines and earlier scoreboard entries remain unchanged.
Every compilation and project test uses the shared build reservation. No
supply-chain output, hidden test, threshold or independent-review boundary is
changed. The fresh development repeat still requires a new frozen API binary;
no repaired-source model measurement or new model qualification is claimed.
