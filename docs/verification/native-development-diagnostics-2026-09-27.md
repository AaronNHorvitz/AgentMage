# Native Development Diagnostics — Bounded Verification

Date: 2026-09-27. Source commit:
`eadfbbb299f047f8c6839036b432c2626181cc57`, tree
`539fed2740e6957a7ab9ebbb749dd20a7af84661`.
[Decision 0092](../decisions/0092-native-development-failure-diagnostics.md)
governs this AMR-01 increment. Exact inputs, binary identities and retained private
result hashes are in the [observation manifest](native-development-diagnostics-2026-09-27.json).
No task or acceptance checkbox changes.

## Behavior and reproduced defect

The [preceding actual CLI/host observation](linux-current-startup-2026-09-27.md)
rejected native prerequisites before tools but exposed only generic platform and
transport failures. A new missing-sibling test of that same old CLI binary also
returned only `coding.development.client.transport-failed`, exit 5, no events and
no state files. Its clean Git status was unchanged; this baseline did not measure
a complete file/index preservation snapshot. No host, tool or inference ran.

The development host now retains native repository and private-directory failure
categories. Tool composition emits its closed native cause locally and keeps the
existing IPC failure. The CLI distinguishes unsafe/missing sibling executables,
spawn failure, launch-envelope transfer and process control. Codes are static;
paths, process identities, caller content and raw child output cannot become codes.
Activation, native trust, confinement, authentication, exit classes and authority
are unchanged. The client does not parse host stderr or invent a startup frame.

## Source checks and retained failures

The final focused host command passed six tests, including three new native-input
refusal cases. Three existing Linux development-boundary tests also passed. These
exercise real validation functions with invalid synthetic inputs before effects;
they are component evidence, not successful native tool or model qualification.

```sh
cargo test --locked --offline -p agentmage-host --lib coding_development_ -- --test-threads=1
cargo test --locked --offline -p agentmage-platform-linux --lib development_boundary::tests::
cargo clippy --locked --offline --workspace --all-targets -- -D warnings
```

The full host suite **failed**: 278 passed, 50 failed and eight existing ignored
cases. All 50 failure names and causes match the preceding batch: 48 native
`InvalidManifest` rejections, one child-exit mismatch caused by that same rejection,
and one `InvalidGitArtifact`. No test was excluded or gate relaxed. Strict Clippy
then rejected five new `err().expect()` assertions. Replacing those with `expect_err`
changed only test assertions; the focused tests, strict workspace Clippy, source,
module, status, context, formatting and diff checks then passed. The failed full
host/Clippy log and its original source hashes remain retained. The full suite was
not rerun after those five assertion changes, and is not reported as passing.

One initial missing-sibling baseline preparation failed before CLI execution because
a hard link crossed filesystems. The retained retry used a digest-verified private
copy. The error and original result were preserved. No admission check was changed.

## Actual executable startup observations

The actual CLI, host and read worker were rebuilt from the clean source pin above
through the configured build reservation with one Cargo job, locked dependencies
and offline Cargo. Binary content identities stayed unchanged throughout the run.

| Executable | SHA-256 |
| --- | --- |
| CLI | `ca513f5dc96938e1ff4e772ec8ea6f49df9ba96e11e4bdc1169a89c32b29c4df` |
| Host | `98bfbdfe8719afcc8d5f63217fc3916ff6e3094ac71534151abf310276994a0f` |
| Read worker | `04e87c0925acbc25dbf1f8de82ce37fa6811d257c43a80f00844cc89a3275e7c` |

The real clean-root `failed-test-repair` invocation selected `scripted`, with explicit
per-run approval. It returned exit 5 in approximately 0.58 seconds, before any runtime
event or tool. Its exact stderr was:

```text
Error: linux.repository.git_artifact.invalid
linux.development.launch-envelope.failed
```

Setup, diagnosis, status and CLI help exited 0. Doctor still reports all four native
executables as `untrusted-path` and the manager as `not-probed-untrusted-systemctl`.
No successful user-manager probe is claimed. The state root contains only an empty
repository-management directory; no operational key or session record was created.
A separate staged/unstaged/untracked fixture was refused by wrapper preflight with
exit 1, no CLI launch and no state files. This is preservation on refusal, not a
successful edit alongside pre-existing work.

Four additional actual-CLI tests used synthetic sibling-host fixtures. Missing,
non-executable and symlinked siblings returned
`linux.development.host-executable.unsafe`; an invalid executable format returned
`linux.development.host-launch.failed`. Each returned exit 5 with no events or state
files. These are startup diagnostics; they do not run a real host coding workflow.
The failed-envelope category was observed with the real host refusal above.
Process-control failure was not induced in this executable matrix.

For all six final cases, exact pre/post snapshots matched: non-Git file bytes and
modes, Git HEAD, raw index bytes, staged index entries, status and staged/unstaged
diff hashes. Raw receipts, streams, snapshots, source/binary identities and the
observation driver remain private, with content hashes in the public manifest.

## Evidence and remaining acceptance

After all source changes, one SBOM/core structural regeneration completed, with
186 Python tests and 52 schema tests passing. The unchanged AC2, canary, runtime,
automated coordinator-boundary, Story 11.2 gate and runtime security/index inputs
validated at their existing pins. No automated review pin was advanced and no
input binding was narrowed. Historical startup and advancement records keep their
original source identities; changed inputs make those observations historical,
not acceptance of this revision. The direct binding inventory is bounded and
makes no transitive or repository-wide freshness claim.

The [local testing guide](../LOCAL-TESTING.md) contains setup, launch, status and
stop commands plus the new diagnostics. Native successful edit/test/repair,
protected denial, active cancellation and successful preservation of pre-existing
work remain unverified in this lane. No real model or GPU process ran. Manual-user,
independent reviewer, production admission and release gates remain open. Existing
historical Muse evidence and the separate GPT-OSS failure are unchanged. Bounded
development-host startup/cleanup and crash-safe worker ownership remain further
local lifecycle work; this diagnostic change does not complete those contracts.
