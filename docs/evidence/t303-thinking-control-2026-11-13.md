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

## Real API check and resumable repeat

The real endpoint check at source
`a896f055256e96baec09966c71c5d0a7f1714d96` includes **2/30 development tasks**.
The remaining 28 are queued under one existing resumable controller. Unlike the
server-configuration diagnostic, the server retains default template arguments
and both captured upstream requests carry the new API extension. Effective
trusted settings report `thinking_mode: disabled`. Preflight passes the same
option to template rendering; its counts agree with the backend counters.

Both tasks produce the same answer bytes as the server-configuration diagnostic
and pass the unchanged strict checker: 2/2 ungraded proposals, normal stops,
no task grades. They use 19,070 input / 2,893 output tokens, request/check
55.335 seconds and reserved GPU job wall 92.043 seconds / 1.534 minutes.
Output allowances are 23,170–23,232; does not fit is zero. Context and prompts
are byte-identical across all three pairs. No candidate test, feedback, retry,
hidden or held-out execution occurred. This establishes the option's behavior
for these two attempts only. The other five profiles have no fresh result here.

The built API SHA-256 is
`ba623b925414e0b390533b18c59b060ad7058c8996d44741420a29b433d05ac1`.
The source freeze SHA-256 is
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd`,
binding 2,283 repository and 20 private inputs. The unchanged helper remains
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
The original helper reproduces all six diagnostic source selections and strict
checks without candidate execution. Each completed pair's API and model exited,
settings were revoked, model keys/sockets are absent, and six retired API token
files were removed. The exact-argument process scan covers only those completed
owners; it explicitly records the continuing controller. It makes no claim that
the lane is idle. Later jobs and their mutable logs are outside this checkpoint.

The controller skips the completed pair, refuses silent replay of partial jobs,
uses separate two-task GPU jobs through both resource wrappers with 900-second
caps, and pauses 30 seconds outside both reservations. Shared queues are waited
normally. Do not start another controller. Finish the repeat before further
tuning, except required high/medium review fixes. Source review runs alongside
that measurement under the owner's review workflow. No source, prompt, task,
profile, threshold, permission or supply-chain change follows this source commit.

## Review retention

The private review capsule retains all six diagnostic attempts, the failed
zero-task startup, both 2,283-file source snapshots, all private input bindings,
the source gate and reproducible methods. It excludes weights, credentials,
complete candidate trees and later jobs' mutable data. Only the selected
candidate source files are retained for hash comparison; none is executed.

- `thinking-control-20261113-inputs.tar` SHA-256:
  `77465ea8ae4a0dd3261dccd42ca010fbf714945192c4d0afea1d4c3ceb17799c`.
- `thinking-control-20261113-manifest.json` SHA-256:
  `29b0860a26ffb5df282a1115587ba630f3fe7d5a9fbfb52d0d65978bb9c22267`.

The read/hash-only verifier passes all **4,720 files**. It streams the archive,
extracts nothing, executes no archived code, checks every hash and size, compares
the six changed Rust sources with their exact committed bytes, and independently
recomputes response counters, candidate source hashes and the unchanged source
gate failure identities. The capsule is an uncompressed streamed copy; no build
or GPU reservation is used for file retention. The archive's earlier queued
packaging attempt was retired before execution. Measurement still uses both
required resource wrappers. No prior sealed artifact is changed.

Independent core review covers the source increment and this fixed checkpoint;
it cannot close T-303/T-306. The remaining 28 development tasks continue under
the existing controller while review runs. No further tuning is authorized by
this checkpoint; finish the repeat and publish its full results first.
