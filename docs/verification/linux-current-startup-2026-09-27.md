# Current Linux executable startup observation

Date: 2026-09-27. Built and invoked at clean commit
`72549cf5486705dddc67357a12a639c6fb1a5968`, tree
`78fb612635051de95ca2bbc6d0911f3fd34f6a63`.
The [companion record](linux-current-startup-2026-09-27.json) retains exact binary,
input and private-result hashes. This updates the executable observation after
the reader, harness-ownership, callback and failed-advancement changes.

The current binaries build and the CLI help works. **The real coding workflow
remains blocked before tool execution.** This is an actual executable startup
attempt with the scripted development profile, not a successful coding campaign.

## Commands and actual results

The build and executable observations ran through the configured shared reservation.
No inference process or GPU work was started.

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 cargo build --locked --offline \
  -p agentmage-host -p agentmage-capability-read-only --bins
target/debug/agentmage --help
```

Setup, diagnosis, start and status used the commands in [local testing](../LOCAL-TESTING.md)
with fresh private disposable roots. The clean start selected `failed-test-repair`,
the explicit scripted model and approval for that invocation. No native admission,
tool authority, profile or confinement requirement was changed.

| Observation | Result |
| --- | --- |
| Current binary build | Exit 0 |
| Actual CLI help | Exit 0; development coding interface printed |
| Clean fixture setup, diagnosis and final status | Exit 0 |
| Actual clean scripted start | **Exit 5**, before tools |
| Clean workspace preservation | Exact file bytes, modes, Git HEAD/index/entries/status and staged/unstaged diff hashes unchanged |
| Dirty fixture setup and final status | Exit 0 |
| Dirty fixture start | **Exit 1**, `coding.harness.start-state-denied` |
| Pre-existing synthetic work preservation | Staged content, additional unstaged content and an untracked file all unchanged, including exact Git index bytes |
| CLI, host and read-worker identities | Unchanged between pre-launch and post-launch observations |

The clean start retained empty stdout and these errors:

```text
Error: coding.development.platform-failed
coding.development.client.transport-failed
```

The doctor reported `untrusted-path` for the native executables and
`user_manager=not-probed-untrusted-systemctl`. It did not execute the untrusted
manager binary. The user-systemd bus is also absent by lane configuration; this
run did not claim a successful manager probe. The final lifecycle was `ready`,
meaning no run record remained, and the operational key was not created.
Startup created a repository-management directory in the disposable state root;
it did not modify the fixture workspace or complete a coding tool.

The dirty fixture was refused by the wrapper's existing clean-start requirement.
It created no run log directory or state files. This verifies preservation on that
refusal, not successful editing of a repository containing pre-existing work.
Both disposable repositories and original streams/results remain retained privately.

## Exact binaries

| Binary | SHA-256 |
| --- | --- |
| `agentmage` | `0b3e8813f93b1f06823415b12455b93f48b8753c079b67ba8f449c7624449d48` |
| `agentmage-host` | `af56eb48271b0c01b98e8053ea8eae13df220923bff7d9a644f10040adc79b5d` |
| `agentmage-read-only-worker` | `04e87c0925acbc25dbf1f8de82ce37fa6811d257c43a80f00844cc89a3275e7c` |

## Acceptance still open

The current native failure prevents the required authorized patch, validation,
observed failed test and correction, protected denial and active cancellation
workflow. No such acceptance is inferred from setup, help, safe refusal or the
[separate component tests](runtime-advancement-reconciliation-2026-09-27.md).
No real-model run or manual user test was performed. Independent acceptance,
human-only gates, supported-platform qualification and release remain open.
No host installation, model acquisition, service change or confinement bypass occurred.
