# T-303/T-306: explicit endpoint launch observation sharing

Status: scoped fast gate PASS; independent core review required.
T-303/T-306 remain open without new rows or sub-rows. Preceding commit
`6e75e7fc1f6ab3ea2eb910357804d1ffa91b63f8` remains immutable and under independent
review at batch start. It returned independent PASS during this batch, without
high or medium findings, and its report is retained unchanged. Its low findings
on native forwarding coverage, real-time test robustness, final inventory-check
precision and cancellation/expiry error-order consistency join carried same-row
backlog. Neither row closes.

The private endpoint development launch configuration now accepts
`share_run_observations: true`. The read-only, strict-edit and combined edit/test
commands, including original-diff review variants, pass this selection to the
existing observation policy. Omission or false keeps control-only behavior.
Non-booleans and duplicate fields refuse. Listener and stop checks precede
configuration reads and repeat before admission. Admission consumes the same
single parsed document that selected sharing; there is no startup reload.

The sharing notice describes all ordinary client-credential holders' access
to the complete sealed request and canonical events. These can contain task,
source and test data. The existing MCP launch still requires its exact
run/request pin and explicit sharing flag. It opens only the client credential.
No model disclosure, effect approval, cancellation, release, settings or
original-diff authority is delegated. Released records remain unavailable.

## Validation

The complete affected fast gate **PASSES**: **261 passed, zero failed, ignored,
measured or filtered**, across all **11 unfiltered local-API targets**. The scope
selects only `agentmage-local-api` for these changes.
All five new regression methods pass. Formatting, Clippy with warnings denied,
Markdown lint and whitespace checks pass. Eight changed Rust source hashes stay
fixed through testing and documentation finalization. No test is skipped or
allowlisted, and no timeout occurs.

The run takes **55.41 seconds** including build reservations: formatting 0.30
seconds, Clippy 9.62 seconds and Rust tests 44.07 seconds. Raw log SHA-256:
`6294faf8d08afc4d6bc89aa9857d24ef31a491da6650821e246e8873c8a81731`.
A preceding reserved crate test run also passes all 261 tests and its zero-test
doc target. The full fast gate above is the final source verification. Separate
final light checks cover documentation-only result recording and the newly
retained parent review, which arrived after the full run. The private review
request names the reproduction bundle; its runner reserves heavy commands
internally and checks unchanged source hashes.

The broader host/Linux gate from the preceding batch remains FAIL with its
inherited 72 failures; those crates' full targets are outside this batch's
selection and were not rerun. A green local-API gate establishes only its stated
scope, not repair of that environmental blocker or native workflow acceptance.

The new configuration regressions exercise omission, false, true, invalid types
and duplicate fields, retaining original request, instruction and validation
identities. Actual executable tests exercise the sharing notice and original
native refusal across every development launcher, with no model connection or
repository-source mutation. Existing listener/stop-before-configuration tests
remain in the complete targets.

A new actual MCP subprocess/HTTP service test observes a run through a synthetic
endpoint response, original coordinator approval, one synthetic tool observation
and terminal rejection. An ordinary client cannot supply that approval, and MCP
cannot advance the run. Every exported page equals the control client's original
page and the complete kernel event sequence verifies. Observation does not turn
the rejecting verifier into task success; release makes the record unavailable.
The existing tests also cover default refusal, stale/foreign pins, absent sharing
consent and forbidden controls.

These are synthetic protocol/component observations, not native worktree creation,
command confinement, real inference or independent task grading. Positive native
launcher execution still encounters the unchanged executable-ownership blocker.

## Remaining work

Actual independent candidate checker/artifact integration, complete original-owner
commit approval, repository/signer recovery, positive native/model coding
measurements, approved push and durable execution recovery remain open. This batch
changes neither coordinator prompts nor evaluation inputs, hidden tests, held-out
sets, thresholds or scoreboard entries. It adds no model or GPU work, dependency,
SBOM renewal, release, public push or owner-readiness action.
