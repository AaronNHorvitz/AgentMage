# Local agent development

Decision 0151 and the active queue at the top of TASKS.md define this workstream.
The preserved ledger describes earlier work; a checked component row does not
establish that the local application is ready for everyday use.

## Per-change gate

Use the existing pinned Rust toolchain and locked dependencies. Python 3 and the
repository's pinned Node/npm tooling are needed. Use the already installed
Markdown linter from node_modules; the gate never installs dependencies. Put the
Node binary directory on PATH if the lane does not expose it by default.
For separate review sessions, verify that this directory is in a mount shared
with the reviewer; a builder-only tool directory is insufficient. Record the
runtime version, binary checksum and exact PATH setup in the private request.

```sh
python3 scripts/local_agent_fast_gate.py --base <commit-before-this-batch>
```

The base must be a commit. The gate includes committed changes since that commit,
staged edits, unstaged edits and untracked files not ignored by Git. With no
`--base`, it compares to HEAD. A clean tree fails with an instruction to supply a
base; it cannot silently report that nothing was tested. To inspect the selected
paths and commands, add `--plan`; this does not establish a passing gate.

The checks are:

- Whitespace errors across the selected Git diff and all untracked additions,
  using Git's whitespace rules without staging files; the existing Markdown
  linter for changed Markdown files.
- Unchanged reviewer reports retained as plain `.txt` files directly under
  docs/reviews/ get UTF-8, whitespace, full commit, scope and final verdict checks.
  This supports reports with metadata before their heading without editing them
  or changing Markdown lint rules. The format check does not establish reviewer
  independence or the correctness of a verdict. Other text files outside crates
  still require their own registered coverage.
  Accepted endings match the reviewer format: `Verdict: PASS` or
  `Verdict: FINDINGS` as the last line, or `Verdict: INCONCLUSIVE` followed by
  one non-empty reason line.
- Python syntax, indentation and whitespace checks for changed Python files, plus
  their test modules. A script maps to `tests/test_<script>.py`; a changed test
  module also runs. A missing or unknown mapping fails. These checks do not claim
  full Python style formatting or type checking. Importing Python modules and
  Rust callers of Python scripts are not automatically selected; add explicit
  caller checks to the review request when changing shared helpers. Automatic
  Python importer selection remains a low-priority backlog item for T-309.
- Rust formatting, Clippy with warnings denied and full unit/integration/binary
  test targets for touched workspace crates and their transitive workspace
  dependents. Workspace manifest, lockfile, toolchain or formatter changes select
  the whole workspace. There are no test filters, failure allowlists or new skips.
  Cargo's existing explicitly ignored tests retain their existing status.

An unrecognized file type fails with a request to extend coverage, inside or
outside crates. Crate inputs accepted by the Rust checks are `.rs`, `.toml`,
`.json`, `.sql`, `.txt` and the four existing extensionless license/fixture files
listed in the gate. Markdown and Python always get their respective checks.
Add the relevant checks and tests when introducing UI assets or another language;
do not simply mark them ignored. Deleted files still influence scope. Rust
dependencies are locked and offline. The current plan uses default crate features;
add explicit feature coverage when a batch changes an optional feature.

Each command prints its elapsed time and exit code. Any nonzero exit fails the
gate even if later checks pass. Each check has a supervisor in a separate session.
A pipe held only by the gate signals its death, including SIGKILL of the gate's
process group. The supervisor terminates the check's group, waits up to five
seconds, and kills remaining group members after timeout, interruption or normal
exit. SIGINT, SIGTERM and SIGHUP stop the gate without starting later checks.
The supervisor observes exit without reaping the check leader until both group
signals have been sent; the leader reserves its process-group ID during cleanup.
`--timeout` defaults to 600 seconds per command, including any
reservation wait; a timeout is never a pass. The lane's Rust commands acquire the
shared build slot when its wrapper is present. Builds use two jobs and tests four
threads, within the four-CPU lane allocation. No GPU work belongs to this gate.
This is development process cleanup, not a sandbox: it cannot contain a check
that deliberately escapes its process group. SIGKILL directed at the supervisor
itself, or killing both gate and supervisor, defeats its cleanup; the operator
must then verify owned processes and reservation release. The ordinary gate and
its process group can be killed without killing the supervisor.

The target is minutes with the existing build cache, not a promise of cold-build
latency. The [baseline failure classification](../evidence/t301-baseline-failures-2026-10-25.md)
records an environmental blocker: the lane maps only the working user, so trusted
system executables appear owned by the overflow UID rather than root. Correct
ownership checks refuse them. The inherited 53 host and 19 Linux test failures
remain failures. A change in any crate except `agentmage-platform-windows` or
`agentmage-xtask` selects one or both failing packages through reverse dependents;
its Rust gate therefore remains red in this lane while the blocker persists.

T-301.1 records the owner action: provide a lane where trusted system executables
and their path ancestors retain root ownership, or run the unchanged checks on the
native owner host. Do not copy/re-own executables, relax trust checks, filter tests
or allowlist failures. Keep the full Rust result FAIL and report identities and
any new failures in the request. No Rust acceptance can be claimed until this
blocker is resolved. Independently useful changes can still be built and measured;
the documentation/tooling batch's green gate establishes only its selected scope.

## Evidence and acceptance

Keep row measurements and limitations under docs/evidence/. Do not copy private
paths, host details, credentials, model content or personal data into the public
repository. Reviews go under docs/reviews/ after the separate reviewer writes
them. Never edit an independent review. A row stays unchecked until an exact
commit has `Verdict: PASS` and no open high or medium finding.

Complete all source changes in a batch before any supply-chain/evidence renewal.
The fast gate does not write an SBOM, move review pins or regenerate old reports.
Historical hash-bound evidence remains historical until renewed. Do not shrink
input bindings or reinterpret stale reports as current acceptance.

At T-310, run the full product checks and the unchanged long documentation gate
once, through the shared build slot in the lane:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 npm run product:check
bash /tools/build-slot env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 npm run docs:check
```

Also run the mission's end-to-end demonstration, collect independent acceptance
review and prepare the owner test kit. An unavailable prerequisite or failing
legacy check is reported explicitly; it cannot be relabeled a pass. The full
gate has not run for this mission yet. No release is authorized by a local build,
a green fast gate or a review of an earlier component.
