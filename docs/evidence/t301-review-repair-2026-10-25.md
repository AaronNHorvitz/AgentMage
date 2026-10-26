# T-301: First review repairs, 2026-10-25

This increment repairs findings from the [independent review of
`4d7e84d669d7a8ce85926deee90336527cad5b05`](../reviews/4d7e84d669d7-local-agent-t301-baseline.txt).
That report is retained byte-for-byte as plain text with `Verdict: FINDINGS`. This is the first
repair and request for a second review, not a PASS or row closure. The review
tier remains core because process cleanup and file coverage checks changed.

## M1: Process lifetime and reservation release

Each gate check now runs under a separate-session supervisor. A pipe's write
end belongs only to the gate. EOF tells the supervisor that the gate stopped,
even when the gate or its process group receives SIGKILL. The check runs in its
own process group, without the pipe descriptors. The supervisor sends TERM,
waits up to five seconds for its leader, then sends KILL to remaining group
members on every exit, including success. It reaps its direct child.

The gate handles SIGINT, SIGTERM and SIGHUP, closes the pipe, waits for cleanup,
and starts no further check after interruption. Failed commands retain their
actual status and cannot be hidden by later passes. Timeout is still a failure.
Two build jobs, four test threads and the shared build reservation are unchanged.

The 21-test module covers the prior discovery, scope and failure rules plus:

- SIGTERM, SIGHUP and SIGINT to a running gate, with no later check started.
- SIGKILL of the gate's process group and SIGTERM to the supervisor.
- Normal check exit with a surviving child that ignores TERM.
- In each of those cleanup cases, no owned check process remains running and a
  lock inherited by the check's child can be acquired again.
- Timeout cleanup of a child ignoring TERM remains covered by the original test.

Tests use temporary files and a real advisory lock to model the inherited build
reservation, without acquiring another lane's lock or signalling its processes.
Test cleanup uses process descriptors to avoid signalling reused process IDs.
This is development cleanup, not tool confinement. A check deliberately leaving
its process group, or an uncatchable kill of the supervisor itself, is outside
this mechanism's guarantee; the [guide](../guides/local-agent-development.md)
records the operator cleanup limit. Product sandbox boundaries are unchanged.

## M2: Environmental failures and the owner action

The [failure appendix](t301-baseline-failures-2026-10-25.md) now classifies all
72 retained failing-test blocks: 56 contain manifest refusals (including one
parent assertion whose child refuses), 15 Git artifact refusals and one signer
artifact refusal. Re-observation confirms the overflow ownership of all four
system executables named by the reviewer. Existing source requires root
ownership. This is evidence of an environmental trust mismatch; it does not
trace every individual refusal or qualify native execution.

The guide records that changes in eight of ten crates select the failed host
or Linux library targets. Only the Windows component and xtask have neither
failing package in their affected set. No gate result is reinterpreted: affected
Rust gates remain FAIL. T-301.1 names the owner action to provide a root-mapped
lane or run the unchanged checks natively. No executable was replaced, re-owned
or granted weaker trust. No test was skipped, filtered, allowlisted or weakened.

## Low findings and verification scope

- L1: crate directories now accept only explicit Rust/data suffixes and the four
  existing extensionless inputs. Unknown UI assets fail pending registered UI
  checks; tests cover JavaScript, HTML, CSS and unknown extensionless inputs.
- L2: the guide explicitly states the Python importer/caller selection omission.
  Automatic importer selection remains backlog for T-309; changes to shared
  helpers need explicit caller checks in their review requests.
- L3: untracked additions receive Git's whitespace check without staging. Tests
  cover trailing spaces/tabs, names with spaces/newlines, valid text and binary
  additions, and an unchanged index.

An initial full-scope gate run failed Markdown rule MD041 on the unchanged
reviewer's metadata-first report. The report is now stored as a `.txt` artifact,
with identical bytes, and receives explicit text-format checks for UTF-8,
whitespace, full commit, scope and final verdict. The test verifies that checking
does not mutate a report and rejects malformed records. Markdown files still
receive the unchanged Markdown linter and rules. No lint threshold was relaxed.

The scoped fast gate checks whitespace, review text format, Markdown lint,
Python lint and all 21 gate tests. No Rust package is touched. The original workspace build, Clippy,
format and 2929/72/78 pass/fail/ignored measurements remain observations of their
original pin, independently reproduced by review 1. The full Rust suite is not
repeated for this tooling/documentation repair, and is not claimed green.

Observed repair verification: the full mission-scope gate, comparing against
`ecf33af35782bf2f76fbfe7332f6348d31ba4277`, exited 0. All six commands exited 0
(about four seconds in total); the 21 tests took 2.31 seconds, and Markdown lint
reported no issues in six files. The repository document/link/policy validator
also passed. The retained review's SHA-256 is
`fa7d9d1cac743f5ab5a07d37f08b8276af225d5400710f893b5264b466f9727e`.
Delta/new-artifact privacy checks and the old-ledger byte comparison passed.

No dependencies, runtime, status-model entries, supply-chain outputs or historical
evidence bindings changed. The old ledger is preserved below the active queue.
No GPU, model, provider, UI, installation, legacy long gate or public release was
exercised. T-301 remains open for exact-commit independent re-review; T-302 has
not started. No owner-test readiness marker is warranted.
