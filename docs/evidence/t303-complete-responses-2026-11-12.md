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
