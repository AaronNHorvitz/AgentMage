# Native Inference Cleanup — Bounded Component Verification

Date: 2026-09-27. Source commit: `0ce9a3d4ea12dd9eedbc1b54edad00a68cef71f3`, tree `02b3d02f6700cb39006f7440bd961bb86dc2e653`.
[Decision 0095](../decisions/0095-native-inference-cleanup-ownership.md) governs this
AMR-01/AMR-03 work unit. The [manifest](native-inference-cleanup-2026-09-27.json)
binds complete source inputs, executable identities and retained private observations.
No task, model, platform, independent-review or release gate is closed.

## Reproduced refusal defect and correction

A CPU-only regression created a regular sentinel at the configured socket path.
The preceding driver reported cleanup success for this unowned object; the test
failed. Its complete changed source bytes, base revision and raw output are retained.
This is an observed component defect, not a failed native coding validation or a
real-model repair. The corrected preservation case passed in the inference suite.

The driver now holds the original private parent, generated key and observed socket
objects. It validates their complete identities and the key contents before use and
cleanup. It checks every object before the first deletion and uses fixed leaf names
relative to the held parent. Changed contents, replacement files or parents, symlinks,
hard links and mode drift refuse cleanup while preserving those objects. A failed
key creation or spawn no longer deletes a path speculatively. An already absent
leaf is accepted only with its originally held, now-unlinked inode and no replacement.

The direct launcher is reaped through bounded polling, with one original three-second
cleanup deadline. The driver also retains process descriptors and generation/ancestry
checks for the exact runtime and its isolated namespace init. Both must report exit.
Numeric PID absence, elapsed time or a successful signal cannot replace those checks.
Unknown startup ownership, observation errors and expiry keep cleanup uncertain.
Further serving is refused once cleanup begins; a later exit cannot turn a failed
attempt into a successful unload. Destruction retains the uncertain owner handles.

Before possible model spawn, the existing fixed inference lease receives a bounded,
synchronized reservation marker. The same live owner clears it only after process
and file cleanup succeeds. Abrupt owner exit leaves the marker refusing later hosts.
Malformed, changed or partial state cannot be recovered from a PID or automatically
removed. The protocol keeps the existing inode and flock rather than introducing a
second scheduler, authority source or canonical store.

These are cooperative syscall bounds and same-user ownership checks, not hard-real-time
or hostile same-UID isolation guarantees. Socket mode publication and unlink checks
are not atomic against an arbitrary concurrent same-user writer. Runtime-directory
recreation, logout, reboot and manual deletion are not proof of cleanup. Uncertain
state requires external reconciliation; this batch adds no reset command. Operator
GPU reservations and exact model/profile/resource admission remain mandatory.

## Checks and native boundary

The first inference suite passed 131 tests with four ignored helpers/native diagnostics;
strict inference Clippy passed. Two further process-descriptor and namespace-identity
checks were then added before the final source freeze. Tests use owned CPU children,
private files and synthetic descriptor pairs. The pairs do not represent a real isolated
model namespace. The timeout case observed an independently owned fixture still alive
before test cleanup; late fixture exit did not release the driver's reservation.

Final inference: **133 passed, zero failed, four ignored**. Full Linux: **183 passed,
19 failed, 51 ignored**. Full host: **282 passed, 50 failed, eight ignored**.
Both Linux and host suites failed. All 69 failure names and causes match the previous
confirmation batch: Linux native Git, manifest and signer refusals; host native
manifest/Git refusals and the related child-exit mismatch. No case was excluded.
Strict workspace and optional-worker Clippy, 27 Python source/dependency audit tests,
source/module/status/context checks, formatting and diff checks passed. The first
ad hoc module command named a nonexistent script and failed; the correct module
inventory check was subsequently run and passed.

The inference crate explicitly enables the already-pinned `rustix` readiness feature.
The complete approved manifest digest was renewed. Both existing feature inventories
remain identical: 126 default and 168 connected contexts. No package, version or
lockfile changed, and the source audit added no exception.

The clean committed source rebuilt the real CLI, host and read worker. CLI help,
setup, diagnosis and status passed. The explicitly approved scripted fail/repair
launch returned exit 5 before IPC, events, tools or prompts, with
`linux.repository.git_artifact.invalid` followed by
`linux.development.launch-envelope.failed`. A separate staged/unstaged/untracked
fixture was refused by the wrapper before launch (exit 1). Complete file, mode,
HEAD, raw index, index-entry, status and diff snapshots were unchanged in both
cases. The manifest retains executable hashes and sizes, checked again after the
observations. These are actual executable prerequisite refusals; neither case
exercised native editing, a protected denial or active tool cancellation.

## Evidence and remaining acceptance

After the complete source batch, the full SBOM was regenerated **once**. The
foundation, dependency, architecture and security evidence checks passed: 196
Python tests and 52 schema tests. The existing runtime campaign, coordinator,
security, evidence-index and Story 4/Sprint 4 gate checks passed without changing
their source pins. Platform blockers remain unchanged. The current supply-chain
check passed; the only component content hash changed by this batch is inference.

The first full Markdown check failed on five lint findings in an ignored generated
rustdoc font-license file, before any SBOM write. The complete generated cache was
preserved outside the checkout with hashes, sizes, modes and link targets verified;
no source, license text or lint configuration changed. The same full gate then passed
567 files. Both the failure and successful retry are retained. New documentation is
checked separately after generation; a generated cache is not source qualification.

The direct-binding inventory found 22 current bindings, 18 newly stale bindings and
431 previously stale or historical bindings to tracked batch inputs. The 18 newly
stale bindings belong to the preceding confirmation observation and the three native
package reports below. These numbers exclude transitive and unnamed bindings and do
not establish global evidence freshness. Historical observations are not rewritten
to imply a new execution.

Three historical Fedora package observations retain their original complete source
pins. This batch changes their inference manifest's current binding; it does not
relabel those package executions or substitute CPU fixtures for their native checks.

| Historical report | Source revision |
| --- | --- |
| Native inference package boundary | `b1ab32328e8e2ded1c2b5d656242235139c1c2ab` |
| Docker production prerequisites | `a3266796d4efba7ce35c31f251359f7b5f147d03` |
| Native runtime package | `f74feb4443e0be16156324a7a050f92cae7db6dd` |

The [local testing guide](../LOCAL-TESTING.md) retains launch commands and the exact
native prerequisite limitations. Positive native edit/test/repair, protected denial,
active cancellation, successful coexistence with pre-existing work, actual model
namespace cleanup and current-source model qualification remain unverified here.
No GPU/model process, manual-user acceptance, independent PASS or release approval
is claimed. Historical observations keep their original bindings. The full accepted
roadmap remains in scope.
