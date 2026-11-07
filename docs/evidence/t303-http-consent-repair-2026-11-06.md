# T-303 and T-306: terminal consent repair

Builder evidence for a non-closing core repair based on
`3de442f11ce0030bd72c685f74116dcda65f53d8`. No row or sub-row closes.
The [independent driver review](../reviews/3de442f11ce0-local-coding-http-driver.txt)
is retained byte-for-byte with **Verdict: FINDINGS**. Medium M1 is repaired in
these same rows; only a separate exact-commit re-review can accept the repair.

## Change

The nonblocking terminal descriptor now retains partial prompt-write progress,
retries terminal backpressure and checks the monotonic deadline and stop flag
between writes. It reads an answer only after the complete review has been
written. File writes are unbuffered; the client does not wait for a terminal
output-drain acknowledgement that could block indefinitely.

Confirmation distinguishes a complete typed allow or denial, a latched stop and
terminal failure. Opening, writing or reading the terminal unsuccessfully, EOF
and exceeding the answer bound return an approval error. The driver sends no
tool decision on that error and reports that canonical run state must be
inspected. The existing shared driver still polls cancellation after a stopped
prompt and before dispatching any decision, using canonical revision-bound job
control. The inspection/start paths refuse failed or stopped consent.

No host loop, grant, model instruction, endpoint, execution sandbox, run record
or permission policy changes. This repair covers the connected `runtime-run`
terminal path. Other explicit terminal commands retain their existing behavior.
Low L1 typed-ahead input and L2 HTTP refusal classification from this review
remain backlog, alongside prior resume, closed-channel and shutdown findings.
An overlong answer now ends this client instead of declining and continuing;
ordinary typed-ahead input remains queued.

## Verification

Executable terminal contracts use the actual client against synthetic HTTP
peers with newly kernel-sealed records. They do not execute native tools or
models. New coverage includes:

- An inspection review exceeding 64 KiB and a slowly draining terminal, followed
  by a complete typed refusal with no prepare request.
- Start and tool reviews each exceeding 64 KiB, drained 1 KiB at a time with
  pauses, followed by separately checked exact allow and denial responses.
- Terminal EOF during inspection, start and tool approval, and an overlong tool
  answer: nonzero exit and no tool decision dispatch; tool failures report
  `cli.runtime.approval_unavailable`.
- Output deliberately left undrained during a large review: signal and deadline
  terminate the blocked prompt without a prepare request.

The existing contracts continue to check canonical signal cancellation, typed
denial, event/artifact tampering, exact preparation binding, pipe refusal and
no approval resend after a dropped reply.

After the last Rust edit and formatting, the complete scoped gate passes:

```sh
python3 -B scripts/local_agent_fast_gate.py --base 3de442f11ce0030bd72c685f74116dcda65f53d8 --timeout 3600
```

It selects only the local API crate, all its targets, three Markdown files and
the retained-review format check. Formatting takes 0.20 seconds, Clippy with
warnings denied 0.95 seconds, and complete source tests 16.21 seconds:
**91 passed, zero failed, zero ignored, zero filtered**. Fifteen executable
runtime-terminal contracts pass, including four new repair contracts. The
separate initial terminal run also passes all 15 tests in 3.28 seconds after
compilation. Both Rust commands honor the shared build reservation; their initial
reservation wait is retained in the raw logs. No failing result is suppressed.

The two Rust source hashes remain fixed from the final source checks through
final evidence and commit audits. Raw initial/final logs, source freeze and
builder audit are retained privately under `tmp/t303-http-consent-repair/`.
The audit checks the six-path scope, exact retained review, unchanged ledger
statuses and implementation truth, frozen/dependency inputs and public hygiene.
This is builder verification, not independent approval.

Final Markdown lint and retained-review format checks pass. The unchanged policy
checker validates 775 Markdown files and all policy invariants on a complete
tracked/current-source mirror, including its existing ignored legacy link target.
Ignored build caches are excluded; this is source policy verification, not the
long T-310 acceptance gate. The scoped pass does not rerun or clear the known
host/Linux ownership failures. No supply-chain output is regenerated.

## Limits

No new model score, hidden assertion, held-out task or GPU process is involved.
The development source/model request path is unchanged. The previously observed
native Git trust refusal is not retested or cleared by these terminal tests.
Native positive failure-to-repair execution, configured-endpoint composition,
durable HTTP recovery, original-checkout apply, approved commit/push, MCP and
measured parent acceptance remain open. No model download, public release or
owner-ready declaration is made.
