# Development Confirmation Input — Bounded Verification

Date: 2026-09-27. Source commit: `a34bb62379c9350b68bf3eea47886e6c9c3c4262`, tree `3c21d29ba6012fea94612d17bed42c1c23f78ca8`.
[Decision 0094](../decisions/0094-bounded-cancellable-development-confirmations.md)
governs this AMR-01 increment. The [manifest](development-confirmation-2026-09-27.json)
binds complete source inputs, binary identities and retained private checks.
No task or acceptance checkbox changes.

## Behavior and reproduced failure

Session preauthorization and exact-operation approval now share a bounded Linux
input reader and the existing cancellation flag. A confirmation requires a complete
UTF-8 line of at most 256 bytes, including its newline. The case-sensitive words
remain `preauthorize` and `yes`, with surrounding whitespace allowed. EOF without
a newline declines, even after an approving fragment. Invalid encoding, excessive
length and descriptor failures stop input without exposing its contents.

The reader polls readiness in 50-millisecond intervals and reads one byte at a time,
so it never consumes a later prompt's line. It leaves descriptor flags and terminal
settings unchanged. Both prompt paths are the dedicated CLI's sole stdin consumers;
a held standard-input lock prevents competing buffered reads in the same process.
This assumes no external consumer steals readiness from the shared descriptor.
Kernel-stalled reads retain the cooperative-bound limitation. Waiting for human
input has no new fixed deadline; existing challenge and grant expiry still apply.

An initial test reproduced a write-only input defect: the helper returned only
after its peer was closed by test cleanup two seconds later. The test failed and
its log and original input digests remain retained. A complete pre-fix source-byte
snapshot was not retained; the final source is committed. The corrected reader inspects the access mode before
polling and refuses a write-only descriptor without changing flags. The same test
then passed before cleanup closed the peer. This is a component regression, not
a failed native coding validation or a real-model repair result.

Pending cancellation remains set. Before runtime preparation, it selects the
existing startup-cancellation and direct-host cleanup path. During operation
approval, the existing interactive driver consumes cancellation before sending
any approval response. The optional automatic approval delay retains its existing
ten-second ceiling and now polls cancellation every 25 milliseconds.

## Checks and executable observations

All eight real-pipe tests passed. They cover exact words and whitespace, EOF,
byte limits, invalid UTF-8, consecutive prompts, pending and delayed cancellation,
unchanged flags and write-only refusal. Host components exercise preauthorization
cancellation and the existing delay. The real interactive driver also passed both
interactive and delayed-preauthorization variants using the actual terminal approval
port with an explicitly scripted transport: one cancel, no approval advance and
no permission-decision event. These are component checks.

| Final full suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Linux platform library | 183 | 19 | 51 |
| Host library | 282 | 50 | 8 |

Both full suites **failed**. All 69 failure names and causes exactly match the
preceding lifecycle batch. Linux has 14 native Git refusals, four invalid native
manifests and one signer artifact refusal. Host has 48 invalid native manifests,
one related child-exit mismatch and one native Git refusal. No case was excluded.
Strict workspace and optional research-worker Clippy, 27 Python audit tests,
source/module/status/context checks, formatting and diff checks passed.

The first source audit failed because the approved whole Linux manifest hash still
described the earlier manifest. After inspecting the feature inventories, only the
existing pinned `rustix` readiness feature and that manifest digest were renewed.
Package versions and the lockfile stayed unchanged. The audit then passed with
zero undeclared network paths. No binding or allowlist was removed.

The actual CLI, host and read worker rebuilt from the clean source pin above in
19.13 seconds. Setup, diagnosis, status and CLI help succeeded. The clean scripted
`failed-test-repair` launch returned exit 5 with the native Git refusal, followed
by the launch-envelope failure. It reached no IPC events, tools or confirmations.
State held only an empty repository-management directory; no operational files
were created. A separate staged/unstaged/untracked fixture returned wrapper
preflight exit 1 before CLI launch. Both complete file/mode and Git/index snapshots
matched. Executable hashes stayed unchanged throughout these observations.

| Executable | SHA-256 |
| --- | --- |
| `agentmage` | `d2be6d69dfc6ed03f4594f225ad0e53ffd37f7fae64ddd4f0c460dc89d75805c` |
| `agentmage-host` | `b72c75c5d5c8b9c3ea4bf585b8d2f4df4418fb86219d360f4b86163aa3be92c6` |
| `agentmage-read-only-worker` | `04e87c0925acbc25dbf1f8de82ce37fa6811d257c43a80f00844cc89a3275e7c` |

## Evidence and remaining acceptance

One full SBOM regeneration covers the completed source batch. The core evidence
pass completed 202 Python and 52 schema tests. The runtime source campaign
passed 30 Rust cases with its dependency-direction and effect-mediation checks;
these source fixtures do not qualify the blocked native workflow. Dependency architecture and its
security map were renewed, along with the runtime campaign. Full input sets,
thresholds and blocker statuses remain intact. AC2, canary and Story 11.2 reports
retain their existing validated pins.

The first boundary renewal stopped before any builder because its committed-input
guard had no campaign pin yet. The raw failure remains retained. The retry passed
the automated boundary check and six tests, then the old Story 4.1 gate refused
`propagation.rs` drift. Inspection found exactly one changed reviewed path, wholly
explained by commit `e26180125aad3b08f7b0a87e1d76d4c39734ac38`: cancellation ancestry,
passive observation, scope checks and regressions. AGENTS.md section 7 authorizes
renewing these automated pins. Story 4.1 and Sprint 4 now target the committed
campaign without changing their reviewed path sets or approval semantics.

Both renewed gates passed. Story 4.1 passed eight unit tests and Sprint 4 passed
seven. Their macOS blocker, full reviewed paths and absence of an external-human
review claim are unchanged. Source code and the full SBOM remained unchanged
during this routine pin renewal.

Dependent runtime security and evidence-index renewal passed, including eight
unit tests. Their full source bindings, checks and partial-evidence status remain
unchanged. The direct binding inventory contains 43 current bindings, 38 newly
stale bindings only in historical observation records, and 445 previously stale
or historical bindings. This inventory is limited to named whole-file inputs
changed in this batch; it does not establish global or transitive freshness.

The [local testing guide](../LOCAL-TESTING.md) retains launch and diagnosis commands.
Successful native edit/test/repair, protected denial and active cancellation still
need the unavailable native prerequisites. Preserving work on startup refusal is
not successful editing alongside pre-existing changes. No real-model or GPU run,
manual-user acceptance, independent review or release approval is claimed.
Historical qualification and observations remain bound to their original sources.
The full accepted roadmap remains in scope.
