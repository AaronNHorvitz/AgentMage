# Trusted inline output schema — 2026-11-15

This non-closing T-303/T-306 increment implements
[Decision 0209](../decisions/0209-trusted-inline-output-schema.md). It requests
a named inline output schema through trusted endpoint settings. It adds no
tool authority and changes no prompt, strict edit check, task or threshold.
Independent core review and a fresh development repeat remain required.
No improvement in model output or task quality is claimed by this source record.

## Measured cause

The independently accepted six-profile repeat at source
`a896f055256e96baec09966c71c5d0a7f1714d96` contains 180 attempts and
117 strict ungraded proposals. Its largest refusal cause is format, 27 attempts.
Read-only diagnosis of those answers produces:

| Primary format cause | Answers |
| --- | --- |
| Duplicate fields | 11 |
| Wrong structure | 11 |
| Malformed JSON | 4 |
| More than 65,536 encoded bytes | 1 |

Classification checks JSON syntax, then duplicates, size and structure. An
answer may have additional defects; no bytes are repaired. All 180 diagnostic
records match their six prior sealed capsule manifests. The 27 answer hashes
match their recorded log bindings. A read/hash-only verifier reproduces the
counts without executing model output, importing archived code or loading any
hidden or held-out test. No prior measurement or sealed capsule is changed.

## Behavior and checks

The [API guide](../guides/local-chat-api.md) documents the generic format and a
folder-free example. Trusted configuration alone selects the schema. It is
encoded with `strict: true` in the exact bound upstream body. The endpoint may
ignore or reject it; no fallback, retry, output repair or validation claim is
introduced. Reference/resolution keywords are refused; the engine resolves no
schema resources. Existing encoded request ceilings still include schema bytes.

The unfiltered fast gate selects the host, local API and Linux inference
crates. Formatting and Clippy with warnings denied pass. All 30 test targets
finish: **1,169 passed, 53 failed, 19 ignored, zero filtered**. The 53 failure
identities exactly match the retained prior source comparison. The full gate
therefore remains **FAIL**, not green. New and expanded tests pass, including:

- Generic schema decoding, invalid names/roots/references and encoded overflow.
- Exact request-size boundaries across framing, usage, thinking and format choices.
- Actual socket request-body binding with the generic schema and no workspace.
- Trusted settings choice, unchanged settings on refusal, reset and client-injection refusal.
- Both response framings and unchanged strict rejection of fenced output,
  duplicate fields, missing edit fields, empty edits and unsafe paths.
- Unsupported structured-output requests failing without retry or fallback.

An initial builder gate was interrupted while queued before formatting ran;
its log and exact-owner interruption receipt are retained. The completed gate
uses the existing `--timeout 3600` invocation option to allow resource waits.
No product deadline, resource ceiling or queue priority changes. Heavy commands
use the shared build reservation. Direct Markdown lint checks the actual files;
all 507 ledger table lines remain unchanged. No supply-chain regeneration runs.

## Measurement boundary

Fresh measurement is prepared in a separate namespace, preserving the previous
repeats. The edit schema requires exactly `edits`, with one to 32 objects having
exactly the string fields `path`, `old` and `new`. All six existing profiles must
repeat the 30 frozen development tasks before further tuning. Prompts, helper,
source selection, checker, context/output formula, complete framing, requested
thinking control and model profiles remain fixed. Model support for the schema
is unverified until a real run records it.

Native artifact admission and the original grading-control receipt still block
the full retrieval/effects/test/feedback/retry workflow. First-attempt and
within-budget task passes remain unavailable. No candidate, hidden or held-out
test runs, original control reconstruction, row closure or qualification follows.

The exact [independent Muse review](../reviews/cbe1d31dfac8-thinking-disabled-muse-measurement.txt)
is retained unchanged: PASS for `cbe1d31dfac869d09b32fca2e38a5498cffcc833`, with
no high or medium finding. Its low method-version retention finding is T-309
backlog; this source increment has its own independent review requirement.
