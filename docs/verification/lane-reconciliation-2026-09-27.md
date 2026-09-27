# Restart reconciliation, 2026-09-27

## Source and retained results

The restart began on clean main at `eef257aadc077a259b676e0b24232013162ac997`.
The preceding handoff's three dirty native fixture files were already committed
at `3e6a0fdb`; they were not replayed. All nine entries in the retained connected
source manifest matched at reconciliation. Later IP-policy document edits remain
real input changes, not retroactive renewal of hash-bound evidence.

The retained native sequence logs have explicit completed checks and native
success/refusal results. Their digests are
`73de1f8252230bea0ea588ee29d52e266292a3322dfb8f65fdb88ecf2b5f1b29` and
`c2430fbdaa84c0a862fe414cfa4059c71da04dfda778a6e6d67c73ebaafe6463`.
These are historical observations of the previous source/worker profile, not
current-source transport or model qualification.

The broad log `0941403e554f8494dde187c6c81c21c7fc78710bdf19bae7916ef7efb15e36eb`
records 181 default Linux, 192 optional Linux, 20 worker, one classification and
324 host tests passing, plus 69 Python tests. Counts overlap. That driver exited
one because its namespace fixture directory was absent; its passing earlier checks
do not turn the whole driver into a pass.

The remainder log is
`0e2b7f4e03e97f09a92eaa9f35d6a184c6987a4917a2ebc6c484473da5036bc9`.
Its two DNS/connection tests and TLS matrix completed, with explicit owned-server
cleanup and transport completion markers. The later scripted CLI matrix did not
finish: 21 case result files are present; `rollback-conflict` has empty streams and
no result; `acceptance-report.json` is absent. All 44 stream identities were inspected,
including the two empty interrupted streams. The final audits, final source checks
and `CONNECTED_REMAINDER_COMPLETE` marker are absent. The recorded
`CONNECTED_REMAINDER_EXIT=0` followed by termination does not establish acceptance.
No aggregate report has been reconstructed or represented as an original receipt.

All 21 retained case results identify the scripted model and the same three initial
and final binary identities. Their implementation records agree on base
`09c41852c7f9275bdfc2177bbde9bc6599a2b758`, tracked diff, untracked fixture and
wrapper identity. Reconstructing the two tracked native-fixture changes from commit
`3e6a0fdb` exactly reproduces their recorded diff digest
`2242801820591f08fb86d153bf11c47404f4e696eff750927feb9e55277c8002`.
The subsequently committed fixture matches the recorded untracked-file digest
`e929966a4f2c0fc15468500616137c11200d947603712ca8e568445f7e1dab8e`.
The wrapper digest matches the initial restart source. This reconciles the retained
identities; it does not supply the missing last case, aggregate acceptance or a
reproducible-build claim.
The case `result.json` files are invocation receipts, not the acceptance-check
records. The acceptance driver keeps those checks in memory until it writes the
final aggregate, so 21 retained invocations must not be reported as 21 passing cases.

## Current lane observations

The pinned dependency cache initially lacked `hkdf`, so the first offline research
test command exited 101 without compiling. A locked dependency fetch completed under
the shared build reservation, with no manifest/lockfile change or model acquisition.
The unchanged-source focused Rust research run then passed 91 tests.

Disposable setup and diagnosis ran. An actual scripted CLI/host launch returned
exit 5 with `coding.development.platform-failed` and a client transport failure before
tools. The native Git trust boundary requires root-owned executable/path objects;
the lane presents those objects with an unmapped ownership identity. Separately,
the user-systemd bus is unavailable. Neither requirement was disabled or replaced.
The source fixture stayed clean. No live-model process was launched.

The rebuilt CLI/host was rechecked at `50f0dbb9728a68657c404f0f00cd53feef5e192a`
with only the reconciliation documentation changed. It again exited 5 before tools,
with an empty event stream, unchanged invocation binary hashes and a clean disposable
fixture. The [component manifest](research-report-component-2026-09-27.json) retains
its exact binary and result identities. This is a reproduced prerequisite refusal,
not a successful edit/test/repair or denial/cancellation workflow.

The developer doctor previously reported confinement available based only on four
files existing. Its corrected diagnostic checks native executable/path prerequisites
and a bounded user-manager query, with an explicit `prerequisites-only` scope.
It skips executing an untrusted control binary. The current lane reports unavailable
prerequisites. These diagnostics cannot admit a platform or replace the native checks.
Fifteen Python diagnostic/harness tests passed, including absent manager, timeout,
untrusted ownership, writable/alias paths and non-executable cases.

## Acceptance remains separate

| Evidence class | Disposition |
| --- | --- |
| Component checks | The baseline research and diagnostic results above are executed results; the subsequent report/URL batch has its own [component verification](research-report-component-2026-09-27.md). |
| Executable scripted workflow | Restart launch blocked; retained preceding matrix interrupted before its final case and aggregate report. |
| Real local model | Earlier eight-case Muse campaigns retain their exact historical pins; GPT-OSS remains separately unqualified. No new campaign ran in this lane. |
| Manual user workflow | Not performed in this restart. |
| Independent review | No exact-candidate PASS review supplied at reconciliation. |
| Platform, installation and release | Open; no installation to host locations or release performed. |

[Local testing](../LOCAL-TESTING.md) records coding setup/launch commands and the
current limitations. Private raw streams, logs, runtime data and paths remain outside
Git. This record closes no roadmap, model, human, independent-review or release row.
