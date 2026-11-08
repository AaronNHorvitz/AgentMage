# T-303/T-306: configured endpoint request binding

Date: 2026-11-07. Non-closing core batch under Decisions 0153 and 0154.
Both parents remain open. Independent exact-commit review is required.

## Change

The credential-owning local transport now prepares an immutable, single-use
request before connecting. It owns the encoded body in a zeroizing allocation
and borrows the original transport. Execution consumes it and sends those same
bytes through that transport. Changing the caller's original message cannot
change execution. No executable request can be deserialized or reconstructed
from its public binding, and no separate transport can execute it.

The serializable binding exposes the canonical endpoint, requested model,
SHA-256 and length of the actual JSON body, usage option, requested output-token
ceiling, exact timeouts and existing transport byte ceilings. It excludes
credentials, HTTP headers and conversation bytes. The identity kind explicitly
states that the selected model is unverified; verified input tokens remain null.
The fingerprint is of the body only. It is not an authorization, server-process
identity or replay token. Identical bodies have identical body digests across
endpoint or credential changes; the complete binding also identifies the selected
endpoint and bounds. No remote weights, tokenizer or hardware is verified.
The binding describes a prepared request; it does not prove transmission or
receipt when execution fails. Such a failure can send none or only part of it.

Snapshot, selected-workspace and failed-test repair proposals use this prepared
request directly. Their context and terminal events carry the same binding.
Terminal `response_complete` distinguishes valid HTTP/codec EOF from partial,
malformed or cancelled generations. Completed invalid edits and length finishes
still refuse a candidate. Optional provider usage retains its existing checks
and labelling; missing counts never become zero or verified token accounting.
Disconnects do not invent a terminal record. All existing consent, source scope,
strict-edit, cancellation, credential and native authority boundaries remain.

The legacy chat entry points delegate to the same preparation/execution path.
Their wire bytes, return types, cancellation ordering and usage opt-in remain
unchanged. The codec and transport share the same existing stream/text ceilings;
no limit is raised. No dependency or native model-profile contract changes.

## Verification

The corrected reserved source gate ran all seven selected commands in 138.89
seconds, excluding the reservation wait. It remains **FAIL**: Cargo tests return
101 on the same 53 host failures, so the gate returns 1. Formatting (0.95
seconds), Clippy with warnings denied (11.37 seconds), whitespace, changed-document
lint and retained-review format checks pass. The complete host, local-API and
Linux-inference tests take 125.07 seconds across 27 targets: **894 passed,
53 failed, 19 ignored, 0 filtered**. All three new transport methods and all
strengthened API/repair methods pass unfiltered.

The 53 failure identities and normalized diagnostics match the retained
publication/usage source gate. All nineteen ignore identities match too; no
new skip or filter is used. The audit initially expected that older witness's
26 targets. The already-reviewed MCP binary adds the twenty-seventh target;
the corrected audit requires exactly the older set plus that binary and proves
its source was already present unchanged at this batch's base. This inventory
correction does not turn the failing gate into a pass.

Before the complete source gate, the same outer reservation ran the entire
transport target (**21 passed**) and API socket target (**37 passed**), both
with zero failures, ignores or filters. The full gate also runs the repair
module and every selected binary/integration target. Logs, source hashes,
command plans, exact failure comparisons and review-input manifests remain
private under the lane state directory. Final lightweight checks verify the
same six Rust source hashes and Cargo selection, exact ten-path scope,
unchanged ledger rows, byte-identical review, whitespace and privacy after
these result paragraphs were added. They run no build or source suite.

The first reserved attempt formatted successfully, then failed compilation:
the pinned SHA-256 return type does not implement hexadecimal formatting.
The implementation and independent socket test now convert each digest byte
to two lowercase hexadecimal characters, using the existing repository pattern.
The original failed log is retained. No test or threshold was weakened.

Three new transport tests exercise real loopback sockets with synthetic peers:

- Hash the upstream's actual received JSON bytes independently and compare them
  with the binding. Change the caller's original prompt after preparation;
  execution still sends the original escaped Unicode/control-containing text.
  Cover both usage options, canonical endpoint/path, exact limits, credential
  exclusion, unavailable counts and no automatic second connection.
- Refuse wrong endpoint approval, malformed model/token bounds and oversized
  messages before connecting. Cancellation after preparation also opens no
  socket. Dropping an unused request has no network effect.
- Separate model, prompt, output ceiling and usage-option changes in the body
  digest. Endpoint-path and timeout changes affect the complete binding, while
  credential changes do not expose or fingerprint the key. Preserve exact
  sub-millisecond timeout values and null verified token counts.

Existing API socket tests now compare the context and terminal bindings with
the upstream-observed request and verify completion flags for success, invalid
strict edits, length finish, absent/invalid usage, truncation, late bytes and
cancellation. Existing settings-race, scope, authentication, redirect, byte/time,
EOF and streaming tests remain unchanged except for additive assertions.

## Limits

These are synthetic endpoint component observations, not real-model inference,
native edit/test/retry execution, a generated-task score or model admission.
No GPU work, held-out run or new scoreboard entry is claimed. The configured
endpoint still cannot supply the native coordinator's exact-profile/token
binding. Integrating an honest endpoint binding into that same coordinator
remains open; no second agent loop is introduced.

The previous runtime-lifetimes [independent report](../reviews/7a007e7aa1f4-local-coding-runtime-lifetimes.txt)
is retained byte-identically, with **Verdict: PASS** for exact commit
`7a007e7aa1f4e9b594f2eda96789ee1269de7bd8` and no open high/medium finding.
It accepts that component increment only. Its two low findings (boundary paths
not reached by failing manifest fixtures, and wrong-root test strength/naming)
remain backlog in the same parent rows; it does not accept this endpoint batch.

Native executable trust remains subject to the deferred host-ownership blocker.
No sandbox/trust boundary, hidden test, frozen suite, threshold, prompt, scorer,
SBOM, status-model or release authority changes. No durable ownership, approved
Git publication, parent closure or owner-test-ready claim follows from this batch.
