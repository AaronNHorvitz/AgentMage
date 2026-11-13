# Trusted endpoint thinking control — 2026-11-13

T-303/T-306 remain open. This increment addresses the largest refusal cause in
the completed six-profile JSON-object repeat: 36 incomplete responses in 180
attempts. Independent core review and a full post-change development repeat
remain required. No task score, model qualification or row closure is claimed.

## Diagnostic observations

Two additional development tasks, dev-001/dev-002, ran with the previously frozen
API and helper at source `40cad6472560e47314a55ddb1f45e2c4c22b1792`.
The existing partial path selected source, called the credential-owning chat API
and used the unchanged strict checker. Full repository retrieval, native effects,
tests, feedback and retries remain unavailable. The original grading-control and
native executable admission blockers are preserved; no control is reconstructed.
No hidden or held-out task ran.

Bounded private upstream response captures show that both baseline attempts
returned empty answer content and separate reasoning content at their output
allowances. A second pair changed only the server template option to
`enable_thinking: false`. Prompts and context JSON are byte-identical between
pairs. Complete framing, JSON-object output, temperature 0, seed 42, 32K context,
strict checker and 300-second idle/total deadlines stayed fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`. The template option added two
rendered input tokens per task. Does not fit: zero. This diagnostic tests server
configuration, not the new API extension.

| Task / server option | Input tokens | Output allowance | Output tokens | Answer bytes | Reasoning bytes | Finish | Strict ungraded proposal |
| --- | --- | --- | --- | --- | --- | --- | --- |
| dev-001 / default | 9502 | 23234 | 23234 | 0 | 77214 | length | No |
| dev-002 / default | 9564 | 23172 | 23172 | 0 | 77046 | length | No |
| dev-001 / thinking disabled | 9504 | 23232 | 1134 | 3490 | 0 | stop | Yes |
| dev-002 / thinking disabled | 9566 | 23170 | 1759 | 5728 | 0 | stop | Yes |

Request/check time is 385.558 seconds for the baseline pair and 54.061 seconds
for the disabled pair. GPU reservations total 408.306 seconds / 6.805 minutes
and 76.891 seconds / 1.282 minutes respectively. These include verification,
load and cleanup, excluding queues and wrapper overhead; active kernel time is
unmeasured. Each pair used both resource wrappers with a 900-second GPU cap,
and both reservations were released between pairs. The first diagnostic startup
failed its health check because the chosen Unix socket pathname exceeded the
platform bound. It consumed 191.567 seconds / 3.193 GPU job minutes, ran zero
tasks and cleaned up; a shorter path corrected the launcher without changing a
limit. These are additional attempts, never added to the reviewed 180-attempt
repeat. Provider counters cover all four attempts and remain unverified reports.
The driver tuple remains unqualified. First-attempt and within-budget task
passes remain unavailable; proposals are not solved tasks.

## Implementation

[Decision 0208](../decisions/0208-trusted-endpoint-thinking-control.md) adds
`thinking_mode: provider_default | disabled` to trusted endpoint settings.
The default omits any thinking extension. Disabled sends exactly
`chat_template_kwargs: {"enable_thinking": false}`. Prepared-request hashes bind
the actual body, and the transport holds the immutable choice. The API reports
the requested mode, enforces its existing control-token boundary and settings
revision, and resets to the default when omitted on a new choice. Ordinary chat
and proposal inputs cannot select it. It works without any folder or repository.
Native development launchers retain their separate defaults.

A server may ignore or reject the optional extension. No prompt is changed,
answer repaired, reasoning promoted to answer text, retry added or fallback
attempted. Empty answers remain incomplete. All strict edit checks, approvals,
credential and destination rules, cancellation, byte/token ceilings and deadlines
remain unchanged. No model profile, frozen task, hidden test, threshold or
supply-chain output is changed.

## Source verification

The complete affected-crate gate finishes all 30 targets: **1,167 passed,
53 failed, 19 ignored**, zero measured or filtered. Every failure identity
matches the retained preceding source gate exactly. **The gate remains failing.**
Formatting and Clippy with warnings denied pass. Three new API socket tests
cover trusted selection/default/reset, invalid/duplicate settings and client-body
refusal before I/O, upstream encoding in both framings, continued empty-answer
refusal when the option is ignored, valid unapplied/untested proposals and server
rejection without fallback. The complete-response transport test now verifies
both thinking choices with exact wire-body hashes and optional usage. The
codec's exact request-size boundary test includes both thinking choices, formats,
framings and usage options. Existing synthetic confinement failures remain open.

Direct Markdown lint checks all five changed Markdown documents with zero issues.
All 507 ledger table lines remain unchanged. Source verification uses the shared
build reservation; the gate already wraps its heavy commands individually. An
initial mistakenly nested reservation was interrupted while waiting, before its
Rust checks ran, and is not counted as a passing gate. No other worker was
signalled. No supply-chain outputs are regenerated during this source batch.

The real API extension check and the full post-change development repeat remain
pending at this source checkpoint. The two-pair diagnostic above used the old
API and a server-side option. No claim about the new option's real endpoint
behavior follows from synthetic tests alone.
