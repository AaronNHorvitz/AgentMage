# Native Supervision and Read Cancellation — Bounded Verification

Date: 2026-09-26. Source parent: `5c48f51eea4f1a1812177959f5ef4d4d712f7770`.
Scope: AMR-01 cancellation/evidence reconciliation and AMR-02 native lifecycle
prerequisites. This is a bounded development increment, not production model,
supported-platform, research, independent-review or release qualification.
The containing commit pins the final sources; commands ran on the inspected tree.

## Implementation boundary

The existing Linux sandbox owner now supervises its one retained attempt with
bounded nonblocking data and control pipes. It withholds the bounded atomic seccomp
filter until the exact transient invocation and held cgroup have been witnessed.
Cleanup uses held ownership, not a global process inventory or signalling a unit
by name. Unknown cleanup retains the attempt and prevents same-process reuse.

On the tested systemd package, transient JobTimeoutUSec was refused. The launcher
uses supported JobRunningTimeoutSec with the existing TimeoutStartSec, original
parent deadline and runtime/resource ceilings. A running-job timeout is NOT a bound
on a pending manager queue; this change does not establish crash-safe restart.
An otherwise valid control query exhausting the original deadline reports timeout;
actual control/read errors remain failures and uncertain cleanup takes precedence.

The offline seccomp policy adds an explicit native x86-64 syscall-number guard after
the pinned compiler architecture check. Native aliases at or above bit 30 receive
EPERM; the foreign-architecture guard remains intact. The policy has a new identity,
and affected evidence builders require the supervisor source and native ABI case.
Other architectures are unchanged, not newly qualified.

Cancellation borrows the existing host control owner. Every read-driver poll checks
the exact consumed task/correlation scope; invalid or unavailable observations fail
closed. A live cancellation token retains its ancestry even after an intermediate
handle is dropped. Downward weak references prevent a reference cycle; regressions
also check that finished branches and the last live ancestry are released.

A valid read cancellation caught at host preflight now stays sticky through exact
grant consumption to the driver, which records cancellation without spawning.
It cannot leave that exact grant reusable. Target revalidation, approvals and grant
checks remain required. Other effect kinds retain their existing early refusal.

Canonical cancelled receipts and receipt-bound TurnCompleted events precede
acknowledgements bearing the original cancellation identity. Event capacity is
reserved before dispatch without increasing limits. Missing or invalid original
signals preserve the receipt but terminate failed, never fabricate acknowledgement.
Uncertain cleanup and already completed effects are not rewritten as clean cancellation.

Read-only modes accept an uncertain allowed read only without output, evidence or
artifact candidates; they still cannot write, execute commands or access the network.
Command, validation and Git projections likewise withhold completion/artifacts when
cleanup is unknown or the executor result is malformed. These receipt corrections
do not complete those executors' native cancellation or lifecycle work.

Engineering status: **Accepted under owner delegation, 2026-09-20**.
Authority: Decision 0054, within the accepted roadmap. Verification is separate.

## Inspected focused and native evidence

The completed focused driver exited 0 with 16 propagation, 58 runtime-loop,
35 sandbox, three live-control, 43 host-runtime and two doctor tests passing.
Twenty explicitly native/campaign cases were ignored in those default stages.
Strict three-crate all-target Clippy with the optional research worker, 80 Python
tests, effect/module audits, formatting and diff checks passed.

Three selected native tests then passed on Linux x86-64 with systemd
259.9-1.fc44, Bubblewrap 0.12.0 and rustc 1.95.0 (59807616e):

- Native read, pre-cancel, timeout, live cancellation, confirmed child entry before
  leader exit, output pressure and reuse: 3.34 seconds.
- Sealed native/x32/native syscall fixtures: native success, x32 EPERM refusal and
  native reuse: 0.27 seconds.
- Borrowed control cancellation and late foreign-scope/unavailable observation,
  each followed by reuse: 3.70 seconds.

All 19 launched native worker attempts have matching exact owned cleanup records.
The valid borrowed-cancellation case requires actual worker output before accepting
cancellation. Error cases withhold stdout: their elapsed time is not body-entry proof.
Raw diagnostics and exact machine-local observations remain private.

The complete successful log SHA-256 is
`1fb0b20b976c464a9022432429962c4d1f55aa84039ad89b08d562b5a791ae62`.
The full tested 19-file source snapshot, including the new supervisor module, is
`f7f13328cd7f20df65d6ffc79de31777513fbaea553031fb1b5a4554b31a5d74`.

Canonical host tests separately prove consumed-grant and receipt identities, replay
refusal on the current and reopened host, original acknowledgement ordering and
SQLCipher reopen. They use a labelled scripted proposal and deterministic timing
probe. The native projection fixture has synthetic scope identities, not consumed
production authority. Combining these separate tests is not a claim of continuous
actual-CLI/native-worker/consumed-grant cancellation, or crash recovery between
each acknowledgement commit. Counts overlap with broader suites.

## Retained failures and corrections

The first cancellation verifier passed 14 propagation tests and failed the dropped
intermediate-token case. This exposed a real ownership defect, corrected by retaining
ancestry without strong downward edges. Its unchanged failing construction and a
release regression now pass. Failed log:
`02c64cb1b061f886f3f85936b25405f295eb18621a3946d4d0650d8e6e38f711`.

The next verifier passed all 16 propagation tests and 57 runtime-loop tests, but
one fixture expected unsorted diagnostic codes. Existing production terminalization
already sorts them. Only the expected exact order changed; validation was not relaxed.
Failed log:
`aafc385d1c494fe2c6254d82851b88912bc63d14bdac792e623e728129e6d5cd`.

Earlier retained failures cover transient-property rejection, original-deadline
misclassification, an audit mistaking path/string joins for thread joins, and a
leader-exit fixture that wrongly required timeout after verified cleanup. The latter
was strengthened to require actual child-entry output before leader exit. No failure
was deleted, resource limit raised or historical evidence silently reaccepted.

## Broader verification and freshness

The broader driver exited 0. Kernel library tests passed 1,154 with seven ignored;
host tests passed 319 with eight ignored; Linux tests passed 172 with 48 ignored.
All 67 engine integration tests, eight documentation tests, 20 optional-worker tests,
six confined environment diagnostics and 134 Python tests passed. The actual
binaries built, and source/effect/dependency/status, formatting, final source-pin
and diff checks passed. Broader log SHA-256:
`c643a4303fc1d2cb79418e05e258daef080b202a03f53743c8dc0704fbccea45`.

All 22 actual CLI/host scripted cases passed, including native inspection,
failed-test repair, creation, multi-file work, protocol/argument correction,
denial, cancellation, approval races, stale/replayed approval, cursor expiry,
overflow, false completion, native command failure, artifact tampering and rollback
conflict. Every report assertion and all 44 referenced raw-log hashes were inspected.
The complete report SHA-256 is
`8f7cc0b75b0dc8b96d1be40b76a726a811ddced10c127d3704092732fcb3274e`.
Its profile is `scripted-executable-fixture-32k-v1` and qualification is
`executable-scripted-only`. This is not a real-model or research campaign.

| Executable | SHA-256 |
| --- | --- |
| CLI | `aa9a9932ac380f0d74cd543507fe17dca588f83f49d9b8fb56a9f62b54c68b26` |
| Host | `8d2100feebc05b5424d5b6d99cb65474790ba250b9ba94d530498d91a7a2dac6` |
| Read worker | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |
| Optional research worker | `5d72bed86a7f80bff8dc0d41d979c40bcd49b8c7d9ee4db0eaa0724d82324101` |

The six worker diagnostics deny networking and supply no request. They verify
composed environment/input refusal, not native transport admission, TLS or retrieval.

The single applicable SBOM/evidence pass completed with exit 0 after the complete
source batch. Supply-chain validation, contract-boundary/schema checks, structural
configuration/kernel reports and selected source/workflow/storage/resume reports
and their tests passed. Forty generated files changed: inspected JSON differences
are digest updates and three source-parent revisions; retained test-log differences
record current execution timings and the expanded filtered-test population.
Evidence-driver log SHA-256:
`32c5d3188868a4a8f0c351ab95cfaf981d14f48c61ac00f46576995d01d0a4ea`.

Hash binding is unchanged. A bounded inventory of 748 JSON files found 594 named
whole-file bindings to changed tracked inputs: 45 current, 28 newly stale and 521
previously stale or historical. It excludes 1,602 line-span bindings and makes no
transitive or overall acceptance claim. Its newly stale findings are explicitly
dispositioned below; a passing leaf test is not a renewed aggregate gate.

| Affected retained reports | Freshness disposition |
| --- | --- |
| Story 11.2 AC2 and story gate | Runtime source changed; selected tests pass, aggregate/current-family coverage still needs reconciliation. |
| Story 12.1 agent progress | Propagation source changed; renew the scoped evidence after the source checkpoint. |
| Sprint 17 local and installed Linux Git matrices | Repository inspection source changed; local renewal and actual installed-native rerun remain distinct. |
| Story 4.1 security/story gate and Sprint 4 gate | Propagation and structural report inputs changed; scoped security renewal and aggregate prerequisites remain separate. |
| Sprint 41 local/source review and Sprint 46 source review | Command-runner source changed; regenerate only their declared local/automated review scope. |
| Sprint 50 runtime contract review | Runtime and test sources changed; automated review renewal cannot approve independent review. |
| Sprint 82 local evidence | SBOM inputs changed; its committed-source builder requires the containing source checkpoint. |
| Story 9.1 clean-image, clean-package, control, cross-distribution and security reports | Propagation or native evidence builders changed; historical platform evidence is not current native acceptance. |

Historical native, model and platform artifacts remain evidence of their original
revisions. No old policy identity is rebound to the new seccomp implementation,
and no stale aggregate is silently promoted. Previously recorded whole-store
coverage and independent acceptance gaps remain open.

## Remaining owned work and external gates

Process-local quarantine does not survive host death. Pending-manager-job and
cross-host/restart ownership, preparation/sealing cancellation, continuous live CLI
cancellation, inter-commit crash cases, and command/Git/validation native lifecycle
work remain open. No installed service, second state store or scheduler was added.

Connected research still requires the confined transport, exact dispatch authority,
hostile-network matrix, canonical full source artifacts, configured provider and
real-model research-to-code campaign. The default product remains offline.

No model ran in this batch. The historical eight-case Muse result is bounded
development evidence at its original source; GPT-OSS remains separately unqualified.
Independent review, human acceptance, supported platforms, licensing changes,
M-HARNESS-DAILY and release gates remain open. No capability row is closed here.
