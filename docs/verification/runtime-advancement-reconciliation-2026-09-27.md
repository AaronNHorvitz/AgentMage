# Failed runtime advancement observations

Date: 2026-09-27. Source: `a13ea8a943d225a3e1ee16140a566a6d0c1de161`.
Base: `fa7955880007b1a678cec9f21d8512730710a9f4`.
The [companion record](runtime-advancement-reconciliation-2026-09-27.json) binds
checked inputs and retained logs for [Decision 0091](../decisions/0091-failed-runtime-advancement-reconciliation.md).
These are component observations, with a failed full host suite. They close no
roadmap checkbox, independent acceptance, human gate or release approval.

## Reproduced failure and correction

A deterministic synthetic tool executed once, then its terminal-publication fixture
returned `Uncertain`. Calling the same live coordinator with a valid cancellation
signal produced a cancelled outcome with no receipt IDs. The original failing
regression is retained. This was not a native-host or model observation.

The existing reconciliation flag now covers every failed advancement and preserves
the first exact error. Further advancement or cancellation cannot mutate live or
retained state. Recovery remains with the current canonical owners. The fix does
not undo an effect or decide whether publication committed. The development
checkpoint-stop probe also requires the canonical reopen it was designed to exercise.
Successfully handled tool/model outcomes and approval waits retain their semantics.

Protected approval input is checked against the current challenge and trusted clock
before advancement. Rejected input leaves that challenge available for a later valid
response; it issues no authority. Native resolution still validates its own current
policy and grant state. No public wire schema, transaction owner, model profile or
confinement requirement changed.

## Verification

The new fault matrix covers nine publication boundaries, seven typed errors and
failures before and after publication: 126 combinations. It checks the first error
and compares live and retained event histories, full artifact payloads, checkpoint,
resource ledger, result/receipt/reference sets, model/effect counts and clock across
repeated calls and a valid cancellation probe. The original cancellation regression
and five malformed approval inputs followed by a valid response are separate tests.

| Check | Actual result |
| --- | --- |
| Original uncertain-publication regression | Failed as expected: 0 passed, 1 failed |
| Corrected focused regression | 1 passed |
| Final runtime-loop suite | 72 passed, 0 failed, 0 ignored |
| Complete engine library suite | 1,226 passed, 0 failed, 7 existing ignored |
| Complete host library suite | **275 passed, 50 failed, 8 existing ignored** |
| Engine documentation and compile-fail tests | 13 passed |
| Workspace, all targets, Clippy with warnings denied | Passed |
| Source, module, status, context, format and frozen-input checks | Passed |
| Changed source documentation lint | Passed |
| Core, AC2, canary and runtime evidence builders | Passed; 200 Python tests |
| Automated boundary and Story 11.2 gate checks | Passed in bounded source scope; 14 tests; story remains blocked |
| Story 23.4 security and evidence-index checks | Passed; 8 tests |

The first expanded regression compile failed because a new test helper omitted a
fully qualified trait name. After that correction, a filename used as a test filter
selected zero tests. Its exit zero is **not verification**. The correct module filter
then ran all 72 tests. Both failed/non-verifying logs remain retained, and the earlier
callback guide's reproduction command was corrected. Earlier observation records
keep their original source pins and complete input sets; changed inputs are historical.

All 50 host failure names and causes match the previous
[callback batch](runtime-callback-consistency-2026-09-27.md): 48 native manifest
construction failures, one downstream crash-matrix assertion caused by the same
child failure, and one repository-map artifact failure. The full source command
retained exit 101 while separately running the remaining checks. Passing those
checks does not change the failed host result. No test was excluded or ignored to
obtain these results.

Reproduce the focused checks through the shared build reservation:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
  cargo test --locked --offline -p agentmage-kernel-engine --lib runtime_loop::tests::
bash /tools/build-slot env CARGO_BUILD_JOBS=1 \
  cargo clippy --locked --offline --workspace --all-targets -- -D warnings
```

The frozen source batch had one full SBOM regeneration. Campaign inputs were
committed at `bb6f6d1468653fb624ccdae91635918fe4c0ab2e`; the automated boundary and
routine Story 11.2 pin renewal were committed at
`cebd7944d0b6beb385c8cc24bf81b346c592078a`. Git-bound reports use those immutable
inputs. Their reviewed-path sets and blockers remain intact. Automated reports
do not satisfy the owner's separate independent review requirement. No global
or transitive evidence freshness is asserted.

## Separate acceptance results

The scripted real CLI/host workflow remains blocked at the native trust
prerequisites recorded in the [restart reconciliation](lane-reconciliation-2026-09-27.md).
User-systemd is intentionally unavailable in this lane. Real-model qualification
was not run; historical model results retain their original source/profile scope.
Manual user testing, independent acceptance and release approval remain open
separately. No GPU process, model download or host installation was started.
Setup, launch and diagnostic commands remain in [local testing](../LOCAL-TESTING.md).

Research provider composition remains unfinished. This prerequisite prevents an
uncertain publication from being relabelled as clean cancellation; it does not
activate a provider, attest to fetched content or complete the Linux deliverable.
