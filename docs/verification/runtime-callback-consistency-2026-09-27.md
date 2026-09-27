# Runtime correctness callback observations

Date: 2026-09-27. Source commit: `aef901c5d9ca37fcbdaf80c8197a2b83a9f482f8`.
Base commit: `2509cacf9c9295800437ee3123c29c436ec9ff6e`.
The [companion record](runtime-callback-consistency-2026-09-27.json) binds exact
inputs and retained log hashes for [Decision 0090](../decisions/0090-runtime-correctness-callback-consistency.md).
This is component verification, with a failed native-host aggregate. No roadmap
checkbox, independent acceptance, human gate or release approval is closed.

## Reproduction and correction

A deterministic trusted-boundary fixture called the coordinator's terminal-event
builder, then changed the returned tool result's elapsed time by one millisecond.
Both executions independently passed the existing shape checks. The old coordinator
accepted the original event with the changed result and reported success. The
regression failed as expected: zero passed, one failed. This was a component
reproduction, not an exploited native host or a model run.

The coordinator now compares each returned permission evaluation, protected
resolution and checkpoint publication with the complete typed value observed by
its event builder. For tool completion it compares the complete canonical result
digest, receipt identity and digest, output discriminator, and ordered artifact
metadata and full-content digests. Payload buffers are not duplicated. Exactly
one callback is required; an adapter cannot hide a repeated invocation by ignoring
its error. The existing unavailable-read branch requires no callback or event.

A detected inconsistency latches the live coordinator until canonical recovery
reopens the state. Repeated calls cannot advance resources, events, model calls or
effects. This cannot erase an effect or event already committed by the host.
The failure is not reported as cancellation, rollback or verified completion.
Existing wire schemas, transaction owners, authority and resource limits remain.

Seven new regression tests cover independently valid result substitutions across
success, failure, cancellation, timeout, denial and uncertainty; receipt changes;
artifact content, order and metadata changes; omitted and repeated callbacks;
changed permission decisions; an independently resealed checkpoint; and the valid
artifact/completion path. The tests inspect the repeated-call refusal and absence
of accepted substituted artifacts or continuation data.

## Observed checks

All substantial checks ran through the configured shared build reservation, with
offline Cargo, one build job and serial Rust tests.

| Check | Actual result |
| --- | --- |
| Original substitution reproduction | Failed as expected: 0 passed, 1 failed |
| Corrected focused reproduction | 1 passed |
| Final runtime-loop suite | 69 passed, 0 failed, 0 ignored |
| Complete engine library suite | 1,223 passed, 0 failed, 7 existing ignored |
| Complete host library suite | **275 passed, 50 failed, 8 existing ignored** |
| Engine documentation and compile-fail tests | 13 passed |
| Workspace, all targets, Clippy with warnings denied | Passed |
| Strict-local source audit, module inventory, formatting and source hashes | Passed |
| Core evidence builders and Python tests | Passed after one SBOM regeneration |
| Story 11.2 AC2, Story 22.1 canary and Story 23.4 runtime campaigns | Passed in their bounded source/component scope |
| Automated coordinator boundary and Story 11.2 gate checks | Passed for source scope; Story 11.2 remains blocked |

The full source command stopped with exit 101 at the host suite. Documentation,
Clippy and the remaining audits were then run separately and passed. This does
not turn the failed full command into a pass.

All 50 host failures were inspected. Forty-eight directly failed unchanged native
sandbox fixture construction with `LinuxSandboxError { kind: InvalidManifest }`.
One parent crash-matrix test observed child exit 101 instead of the intended crash
exit 93 because its child failed at the same construction boundary. The remaining
repository-map test failed with `InvalidGitArtifact`. The lane's native executable
paths do not meet the existing root-ownership trust requirements; its user-systemd
bus is unavailable. These failures precede the callback changes. No failing test
was ignored, removed or presented as successful native validation.

Reproduce the focused source checks with:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
  cargo test --locked --offline -p agentmage-kernel-engine --lib runtime_loop::tests::
bash /tools/build-slot env CARGO_BUILD_JOBS=1 \
  cargo clippy --locked --offline --workspace --all-targets -- -D warnings
```

## Evidence and acceptance boundaries

The source batch was frozen before the single SBOM write. Campaign inputs were
committed at `c7a3ec555a7dc4e2cd26d6758be5fe96bcbb8e44`; the automated coordinator
review and routine Story 11.2 pin renewal were committed at
`e5076397ec64aa92d2e28b48e63f68a3e06a7fec`. The gate's reviewed-path set and blockers
were preserved. Git-bound reports retain the exact committed inputs they read.
Automated source reviews are not the separate independent reviewer requested by
the owner. Earlier reports retain their original source pins and full input sets;
this record makes no whole-project or transitive freshness claim.

Scripted executable workflow: still blocked at native admission, as recorded in
the [restart reconciliation](lane-reconciliation-2026-09-27.md). Real-model
qualification was not run. Earlier model observations keep their original source
and profile scope. Manual user testing, independent acceptance and release remain
open separately. No GPU process, model download or host installation was started.
The setup, launch and diagnostic commands remain in [local testing](../LOCAL-TESTING.md).

Research-provider integration remains open: the reference-bearing result must be
bound before its terminal event, while artifact publication and its events stay
with the existing coordinator afterward. This callback fix does not activate that
provider or substitute component fixtures for the required Linux workflow.
