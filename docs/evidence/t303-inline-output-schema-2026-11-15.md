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

## Real API checkpoint

Measured source: `72c002a42957340de0b6fa0eb4d48385db2c7e62`. The corrected private
preparation completed **2/30 development tasks on A4B**, yielding two strict
ungraded proposals. Both same tasks already yielded proposals before this
change; this checkpoint establishes no improvement. The remaining 28 tasks on
this profile and five complete profile repeats are pending before further tuning.

| Observation | Result |
| --- | --- |
| Request/check time | 64.846 seconds |
| Completed reserved work | 102.462 seconds / 1.708 minutes |
| Backend tokens | 19070 input / 3665 output; 2/2 counters |
| Context / output allowance | 32,768 / 23,170–23,232 tokens |
| Input / does not fit | 9,504–9,566 tokens / 0 |
| Task passes / within-budget passes | Unavailable / unavailable |
| Held-out / candidate execution | Not run / not run |

Actual upstream bodies contain the named inline schema with `strict: true`.
Messages/context match the prior repeat exactly; all other generation settings
are fixed. Both HTTP 200 captures contain completed answers with `stop`.
Reasoning metadata is absent, not reconstructed as zero; broad model support
for either optional extension is unclaimed. Strict outcomes are tallied against
bound Rust checker outputs and actual API events, not an independent re-execution
of the checker. Task quality, effects, tests, feedback and retries remain blocked
by the existing admission and original-control prerequisites.

The first preparation used an overlong private socket path: 111 bytes. The
server loaded in 28.209 seconds, but the host health client receives the exact
`AF_UNIX path too long` error. It attempted no task and never started the API.
The unchanged startup deadline expired; cleanup reaped the model with exit -15
and removed its key/socket. No sampled GPU-memory peak was retained for that failed
startup; the report retains the lane cgroup RAM peak. Its **189.866 seconds / 3.164
reserved minutes** are retained separately, not classified as a task/model failure.
The corrected private root gives 87-byte paths; model/source/schema/budgets,
permissions, deadlines, resource caps and priority remain unchanged. An initial
preparation-method assertion expected 88 rather than the actual 87 bytes after
preparing files; no model or task was launched by that method. A separate
read-only verification accepted the existing files and recorded the actual
length. Both methods and the failed startup remain retained.

Total reserved work including the failed preparation is 292.327 seconds /
4.872 minutes. Both jobs used both wrappers with a 900-second cap after
GPU admission, low queue priority and nice 19 within 16 GiB/four CPUs. Queues
and wrapper/pre-entry overhead are excluded; timers start after frozen-input
validation. Active GPU kernel time and continuous elapsed time are unmeasured.
The corrected job revokes settings, reaps the API/model owners, removes model
key/socket and retires both API tokens. Exact-argument scanning finds no matching
owner in this PID namespace; there is no global-idle claim. Wrapper exit statuses
are unavailable because the launch parent exited after recording the exact
PID/start/argv; completed worker reporting and cleanup are observed separately.

The new private freeze preserves all 2,283 source and 26 private bindings; only
the private namespace and its disclosed policy note change. No task is replayed,
control reconstructed, hidden test loaded or threshold weakened. The source
remains unchanged after the verified commit. The retained read/hash-only capsule
includes failed preparation, both private freezes, requests/responses, contexts,
events, checks, counters, cleanup and method versions. It executes no captured
binary or model output. Independent core review remains required; no row closes.
