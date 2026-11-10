# T-306: Confine approved local worktree Git effects

This same-row coding prerequisite implements Decision 0189 on immutable base
`d5452baf777b7f6cdc6aaca06883da5cf4298fbb`, which is under independent core review
when work begins. T-303/T-306 remain open. No row or sub-row is added or closed.
The base review arrives during verification with **Verdict: PASS**, no high or
medium finding; it is retained byte-identically. Its low L1 reduced external
worktree-existence signal and L2 tighter ordinary read-worker resource limits
are disclosed in the guide and remain same-row backlog for improvement and
measurement. Earlier unrelated lows also remain backlog.

The original repository executor now dispatches its existing approved local
plans through the offline worker supervisor. Its direct Git spawn and direct-child
waiting helpers are removed. Source files are read-only; the exact selected Git
metadata and owned scratch root are writable. Held descriptors and repeated
identity checks bind all three projections. Canonical guest paths preserve Git's
persistent worktree pointers without mounting a host ancestor. Runtime collisions,
linked read-only scopes and network operations refuse. Existing kernel plans,
approvals and preservation reconciliation stay required. There is no fallback,
new route or newly admitted operation.

Six new regression methods cover inert launch configuration, synthetic result
classification, changed directory identities and aliases, malformed or cancelled
requests, valid linked read-only refusal, supervisor projection cardinality and
nonce recognition. Precancellation exercises real supervisor admission with
inert artifacts. It never starts those artifacts. The existing concurrency
regression's observation seam moves to the admitted worker before seccomp-filter
delivery; its original preservation assertions remain. None of these establish
positive native execution, an end-to-end model workflow or a coding score.

A separate synthetic mount-layout experiment first fails worktree creation:
Git cannot start its built-in `branch` helper, whose installed symlink resolves
to the absent `/usr/bin/git`. An attempted correction binds the same
Git descriptor at that path and `/app/git`. Its layout experiment also fails:
bubblewrap consumes a bind-fd descriptor and refuses its reuse for another mount.
The final correction mounts the held executable once at `/app/git` and creates
a guest `/usr/bin/git` symlink to it. It adds no host executable directory. The
existing inert-plan regression requires the single bind and exact guest symlink
and rejects a host executable-directory mount.
This is separate from the unchanged native executable-ownership admission.

Before the helper correction, a complete affected fast-gate run reports 1,327
Rust passes, the exact same 72 inherited failure identities and 64 existing
ignores across 26 unfiltered targets. All six new methods, three affected
controls, formatting and Clippy pass; four source hashes remain fixed for that
run. It takes 272.76 seconds including reservations. Its raw log SHA-256 is
`623c228576d633b3ea96adaed1a85c3f8804290000a797d58c3d3918bc9c5443`.
Those structural tests did not detect the helper failure; the final-source run
follows the mount correction and strengthened regression. The gate remains
**FAIL**; an exact comparison is data only, never a passing gate.

The earlier complete-gate attempt times out waiting for a Rust-test reservation
after formatting and Clippy pass, before any Rust test executes: 1,864.34 seconds
including queues, with raw log SHA-256
`5042ee50aa365b86408470362a8359f82981a920e60f4eedf82a53f75ff29eef`.
A second attempt is interrupted while waiting for formatting's reservation;
no Rust tests run in that attempt. Neither incomplete attempt is counted as a
passing check. Resource reservations and timeouts are unchanged.

The double-bind source also completes the affected tests with the same counts
and Clippy passing, but formatting fails on an assertion layout. Its data check
correctly refuses that formatting exit; no passing gate is claimed. That run
takes 229.02 seconds, with raw log SHA-256
`861de9122076b154742aa34e7429734e23dde7eaa8af504c118776ca01bf03b4`.
Another formatting-queue attempt from the preceding session is incomplete. The
final source is formatted before its full affected rerun.

The final synthetic mount-layout experiment passes worktree add/remove,
persistent host/guest pointers, ancestry and local compare-and-swap. Attempts to
read an external host file through a scratch symlink or write the read-only
checkout through Git configuration both refuse; original sentinels remain
unchanged. This directly runs bubblewrap with disposable fixtures: it does not
exercise product artifact admission, systemd supervision or seccomp, and cannot
establish native workflow acceptance. The experiment command log SHA-256 is
`a469d6c718709958b14a590a4f68f794fd3cea02963b52b848bf4c1b930cacf4`.

The final-source complete affected fast gate remains **FAIL**: 1,327 Rust
passes, the exact same 72 inherited failure identities and 64 existing ignores
across 26 unfiltered targets, with zero measured or filtered tests. All six new
methods and three affected controls pass. Formatting, Clippy and documentary
checks pass. All four Rust source hashes remain unchanged during verification.
The run takes 250.23 seconds including reservations: formatting 15.33 seconds,
Clippy 9.17 seconds and Rust tests 224.06 seconds. No command times out. Its raw
log SHA-256 is
`e35b5f901d6e533c4e95f49cc8ed3c98266688f0fccd05e3a0bc3410bdf03aea`.
The exact per-target comparison returns zero as data only; it cannot turn the
full gate into a pass. Final prose lint, privacy, byte-identical retained review
and unchanged task-table checks pass after this verification.

The existing executable-ownership refusal remains unchanged. Dormant candidate
and local-commit adapters are not activated. Candidate execution, reviewer/artifact
provenance, separately approved local commit/push effects, durable recovery and
native/model measurement remain open. No model or GPU process starts. Frozen,
hidden and held-out tasks, scorers, prompts, thresholds, dependencies, supply-chain
outputs, lifecycle and scoreboard are unchanged. No release or owner readiness
is claimed. Independent core review is required.
