# Local agent development

Decision 0151 and the active queue at the top of TASKS.md define this workstream.
The preserved ledger describes earlier work; a checked component row does not
establish that the local application is ready for everyday use.

## Per-change gate

Use the existing pinned Rust toolchain and locked dependencies. Python 3 and the
repository's pinned Node/npm tooling are needed. Use the already installed
Markdown linter from node_modules; the gate never installs dependencies. Put the
Node binary directory on PATH if the lane does not expose it by default.

```sh
python3 scripts/local_agent_fast_gate.py --base <commit-before-this-batch>
```

The base must be a commit. The gate includes committed changes since that commit,
staged edits, unstaged edits and untracked files not ignored by Git. With no
`--base`, it compares to HEAD. A clean tree fails with an instruction to supply a
base; it cannot silently report that nothing was tested. To inspect the selected
paths and commands, add `--plan`; this does not establish a passing gate.

The checks are:

- Whitespace errors across the selected Git diff, and the existing Markdown linter
  for changed Markdown files.
- Python syntax, indentation and whitespace checks for changed Python files, plus
  their test modules. A script maps to `tests/test_<script>.py`; a changed test
  module also runs. A missing or unknown mapping fails. These checks do not claim
  full Python style formatting or type checking.
- Rust formatting, Clippy with warnings denied and full unit/integration/binary
  test targets for touched workspace crates and their transitive workspace
  dependents. Workspace manifest, lockfile, toolchain or formatter changes select
  the whole workspace. There are no test filters, failure allowlists or new skips.
  Cargo's existing explicitly ignored tests retain their existing status.

An unrecognized file type outside a crate fails with a request to extend coverage.
Add the relevant checks and tests when introducing UI assets or another language;
do not simply mark them ignored. Deleted files still influence scope. Rust
dependencies are locked and offline. The current plan uses default crate features;
add explicit feature coverage when a batch changes an optional feature.

Each command prints its elapsed time and exit code. Any nonzero exit fails the
gate even if later checks pass. A command timeout fails and terminates its owned
process group. `--timeout` defaults to 600 seconds per command, including any
reservation wait; a timeout is never a pass. The lane's Rust commands acquire the
shared build slot when its wrapper is present. Builds use two jobs and tests four
threads, within the four-CPU lane allocation. No GPU work belongs to this gate.

The target is minutes with the existing build cache, not a promise of cold-build
latency. A native-platform test refusal keeps the Rust gate red; record the
failure, fix it without weakening confinement, or split out independently useful
work. The documentation/tooling batch's green gate says nothing about Rust tests
that were outside that batch's scope.

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
bash /tools/build-slot npm run product:check
bash /tools/build-slot npm run docs:check
```

Also run the mission's end-to-end demonstration, collect independent acceptance
review and prepare the owner test kit. An unavailable prerequisite or failing
legacy check is reported explicitly; it cannot be relabeled a pass. The full
gate has not run for this mission yet. No release is authorized by a local build,
a green fast gate or a review of an earlier component.
