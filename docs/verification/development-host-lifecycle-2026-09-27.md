# Development Host Lifecycle — Bounded Verification

Date: 2026-09-27. Source commit:
`0d3a1cbf8f2e48ee0efe0b280611b1ce84e1675a`, tree
`262897af706c055b60a9fe7a4e535a040a986110`.
[Decision 0093](../decisions/0093-bounded-development-host-lifecycle.md)
governs this AMR-01 increment. The [manifest](development-host-lifecycle-2026-09-27.json)
binds full inputs, binary identities and retained private results. No task or
acceptance checkbox changes.

## Findings and behavior

Source inspection found unbounded blocking envelope reads and direct-child waits.
No indefinite hang was measured. The existing parser now reads through a nonblocking
adapter with one 120-second startup deadline. Progress does not renew it. Normal
shutdown allows ten seconds, followed by up to three seconds for exact direct-child
termination and reaping. Error and destructor cleanup use the same three-second
budget. Late exit cannot turn a timeout into successful shutdown.

A private process-local reservation admits one host owner. Pre-spawn refusal or
known reaping permits release when the handle is dropped. Uncertain cleanup retains
at most that one child handle and refuses replacement for the remaining process
lifetime. These are cooperative deadlines: they cannot preempt kernel-stalled
filesystem, spawn or process syscalls. Direct-host cleanup does not prove that
native/model descendants stopped or establish cross-host crash recovery.

The previous CLI installed SIGINT/SIGTERM handlers after receiving the envelope.
An actual old-CLI test with a synthetic incomplete-envelope host reproduced the
gap: the CLI exited on SIGINT in about 0.16 seconds while the pinned child remained
alive. Only then did the test close its private lifeline to clean up the fixture.
That CLI's source was `eadfbbb299f047f8c6839036b432c2626181cc57`, with SHA-256
`ca513f5dc96938e1ff4e772ec8ea6f49df9ba96e11e4bdc1169a89c32b29c4df`.

The first baseline observer failed after 130 seconds because it waited for stderr
EOF, which the surviving fixture could hold open. Its raw failure and cleanup
record remain retained. The corrected observer waited for the owned CLI's exit
separately, then inspected the fixture before closing the lifeline. The failed
observer is not counted as a passing reproduction.

The corrected CLI owns its signal registrations before host spawn, observes the
same cancellation flag during startup and retains it for the existing runtime
cancellation port. Startup cancellation returns the existing exit class 6 only
after direct-child reaping; cleanup uncertainty takes precedence. This does not
qualify cancellation during native tools or every later blocking prompt.

## Component checks and full-suite failures

The final Linux suite passed all 16 lifecycle tests. They use real private pipes,
concurrent reservation attempts and test-owned Rust subprocesses. Cases cover
partial/fragmented frames, unchanged framing refusals, original deadline expiry,
buffered-frame cancellation, delayed cancellation, normal/failing child exit,
forced timeout, sticky late success, destructor reaping and uncertain ownership.
Seven test-owned children reached readiness and were cleaned up. The subprocess
helper is separately marked ignored and explicitly invoked by its parent tests.
These are component tests, not native host/model workflows.

| Final full suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Linux platform library | 175 | 19 | 51 |
| Host library | 278 | 50 | 8 |

Both full suites **failed**. Linux failures were 14 `InvalidGitArtifact`, four
`InvalidManifest` and one signer `ArtifactDenied`. Host failures were 48 native
`InvalidManifest` rejections, one child-exit mismatch from the same rejection and
one `InvalidGitArtifact`. Exact names and causes match the preceding observations.
No test was excluded or gate relaxed. The initial source before cancellation edits
had 173 Linux passes with the same 19 failures, and the same host counts. Its hashes
and logs remain separate; both full suites were rerun after the cancellation edits.

Final strict workspace and optional research-worker Clippy, source/module/status/
context audits, formatting and diff checks passed. The commands ran with one Cargo
job, serial Rust tests, locked dependencies and offline Cargo through the configured
build reservation. Full-suite failures remain failures despite those passing checks.

## Actual executable observations

The actual CLI, host and read worker were rebuilt from the clean source pin above.
Their content identities stayed unchanged throughout the observation.

| Executable | SHA-256 |
| --- | --- |
| CLI | `f5979f8b691df51b0bd668795a183a4fb8818097e5e21900c4f2843fe9627619` |
| Host | `b0c9a61b9945bc289032f51b4b832aedf07b39a94b6b81576a6cf444ec183c17` |
| Read worker | `04e87c0925acbc25dbf1f8de82ce37fa6811d257c43a80f00844cc89a3275e7c` |

The real clean-root scripted `failed-test-repair` start returned exit 5 in about
0.61 seconds, before IPC, events or tools. Stderr retained
`linux.repository.git_artifact.invalid` followed by
`linux.development.launch-envelope.failed`. Doctor still reports four untrusted
native executable paths and `not-probed-untrusted-systemctl`; it does not establish
a working user manager. State contains only an empty repository-management
directory, with no operational key or session record. Setup, diagnosis, status
and CLI help exited 0.

A staged/unstaged/untracked fixture was refused by wrapper preflight with exit 1
before CLI launch. Four actual-CLI synthetic sibling cases also passed: missing,
non-executable and symlinked hosts returned the unsafe-executable code, and an
invalid format returned the launch-failed code. Each returned exit 5 with no
events or state files.

Three further actual-CLI cases used a synthetic host that emitted only four bytes
of an incomplete envelope and waited on a test-owned lifeline. It served no IPC
and performed no tool or model work.

| Synthetic-host startup case | CLI exit | Observed seconds | Diagnostic |
| --- | ---: | ---: | --- |
| Incomplete envelope | 5 | 120.0603 | `linux.development.launch-envelope.timed-out` |
| SIGINT while starting | 6 | 0.0404 | `linux.development.startup.cancelled` |
| SIGTERM while starting | 6 | 0.0394 | `linux.development.startup.cancelled` |

For each, the observer pinned the test child identity and observed its exit while
the lifeline was still open. No test cleanup signal was needed. Each produced no
events or state files. This is startup cancellation and direct-child evidence,
not successful native coding or active runtime cancellation qualification.

All nine final repository snapshots matched exactly: non-Git file bytes and modes,
HEAD, raw index bytes, staged entries, status and staged/unstaged diff hashes.
Preserving pre-existing work on refusal does not demonstrate a successful edit
alongside that work. Raw receipts and snapshots remain private, with public hashes.

## Evidence and open acceptance

One post-source SBOM/core evidence regeneration covers the complete deadline,
ownership and cancellation batch; 186 Python checks and 52 schema tests passed.
The unchanged AC2, canary, runtime, automated
coordinator-boundary, Story 11.2 gate and runtime security/index reports retain
their existing reviewed pins and full inputs. No binding is narrowed. Historical
observations remain tied to their original revisions; the direct binding inventory
does not establish repository-wide or transitive evidence freshness.

[Local testing](../LOCAL-TESTING.md) retains setup, launch, status and stop commands.
Successful native edit/test/repair, protected denial, active cancellation and
successful preservation of pre-existing work remain unverified in this lane.
No GPU or real-model process ran. Manual-user, independent-review, production
admission and release gates remain open. Historical Muse and separate GPT-OSS
results are unchanged. Cross-host worker ownership and research integration remain
further dependency-ready work; this batch does not complete the Linux deliverable.
