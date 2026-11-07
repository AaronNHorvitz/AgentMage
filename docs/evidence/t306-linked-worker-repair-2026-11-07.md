# T-303/T-306: Linked worker review repair

Builder record, 2026-11-07. Base `0f560d5f8d405121260733eec52e49e90da9e8b0`.
This repairs the [independent core review's](../reviews/0f560d5f8d40-local-coding-linked-worktree.txt)
H1, H2 and M1 in the same rows, plus low L1. Both parents remain open;
independent core re-review is required. No row or sub-row is added or closed.

## Corrections

The initial linked implementation could not launch: its three projections were
refused by the ordinary Git supervisor, its inventory requested an inadmissible
32 MiB output cap, and its detached metadata mount could not resolve shared
refs. The earlier setup/argv controls did not establish runtime reachability.
The previous builder record now explicitly retains that failed disposition.

The closed supervisor owner set now has a linked Git variant carrying the
verified linked binding. It admits exactly two projections after revalidating
that binding. Ordinary Git remains restricted to one. The worker projects the
workspace and common metadata read-only, retains the held per-worktree descriptor
for revalidation, and selects the verified name beneath
`/metadata/common/worktrees/<name>`. This preserves `commondir`'s `../..` layout;
it does not rely on `GIT_COMMON_DIR` for ref resolution. The shared metadata was
already part of explicit activation consent; no additional root is exposed.

Linked inventory uses the smaller of each requested cap and the supervisor's
unchanged 4 MiB limit. Overflow refuses startup; it cannot silently publish an
incomplete repository map. Ordinary inventory retains its original caps.
Inspection preflight uses that same supervisor constant. Projection duplicates
now use close-on-exec, with an explicit flag assertion in the component controls.

The existing mount-plan assertions now run with a real verified filesystem
binding. A new non-native regression passes actual projection descriptors to
`run_owned` and observes precancellation after successful admission, before any
child or executable-trust check. It covers both inventory limit classes, the
4 MiB boundary, wrong owner/count/name, oversized/empty limits and stale binding.
Its inert artifacts exist only under `cfg(test)` and never enter a product path.
The ordinary ignored owner test also checks three-projection refusal.
A new explicit native linked inventory/inspection test creates a real linked
branch and checks shared HEAD, inventory and inspection through the complete
worker. Like the existing native worker tests it requires trusted launchers and
a user service manager. No existing test was newly ignored or relaxed.

## Verification

The complete gate selects host, local API and Linux, with all targets and no
test filter or failure allowlist:

```sh
python3 -B scripts/local_agent_fast_gate.py --base 0f560d5f8d405121260733eec52e49e90da9e8b0 --timeout 3600
```

| Check | Observed result |
| --- | --- |
| Formatting and Clippy with warnings denied | PASS |
| Complete source tests | **FAIL:** 900 passed, 72 failed, 63 ignored, zero filtered |
| Failure identity comparison | Same 53 host and 19 Linux baseline identities; no new identity |
| Focused linked boundary regressions | 2 passed, zero failed, 310 intentionally filtered; separate from the complete gate |
| Real guest-layout component probe | All nine Git queries exit zero; shared HEAD matches the original fixture |
| Explicit full native linked test | FAIL at `InvalidGitArtifact`, before worker launch; T-301.1 retained |

The 63 ignored methods are the same 62 existing methods plus the new native
linked test described above. No previously running method became ignored. The
new native test was compiled in the complete gate and separately attempted with
`--ignored --exact --nocapture`; its trust refusal is retained, not counted as a
pass. Both unignored linked boundary regressions pass in the complete gate.

The disposable guest-layout probe uses actual Git 2.56.0 and Bubblewrap 0.13.1,
with read-only workspace/common directory FD projections, the preserved nested
metadata path, cleared environment, isolated namespaces and the source's hardened
Git observation prefix. Seven inventory query shapes, `show` and `for-each-ref`
all succeed. HEAD equals the original fixture commit; tracked, untracked and
ignored paths are observed. Fixture setup plus nine queries took **44,922
microseconds**. The probe mounts the Git executable by path and omits systemd
`OpenFile`, the supervisor and seccomp. It demonstrates guest ref/layout
resolution only; it is not full native admission, coordinator execution,
inference or a coding score.

The full gate records 93.21 seconds for formatting, 8.06 for Clippy and 308.62
for complete tests, including reservation waits and compilation. An earlier
formatting/focused/probe/native group waited roughly half an hour for the shared
reservation. All waits were honored. These are development-check timings, not
model or task measurements. The five Rust source files are frozen and unchanged
across final checks.

Private raw logs retain the complete gate, focused regression, guest-layout JSON
and explicit native refusal. The final audit binds those source hashes, the exact
11 changed paths, failure identities, unchanged prior-review bytes, unchanged
ledger remainder and statuses, and privacy checks. The unchanged policy checker
uses a complete current-source mirror and retains its full input manifest;
ignored build caches are excluded. Final documentation checks cover every changed
Markdown file and the raw retained review. These checks are separate from the
long T-310 gate.

## Limits and remaining work

Successful linked startup through the complete native worker remains unverified
in this lane. T-301.1's executable-trust refusal stays intact and deferred. The
operator guide distinguishes setup and component admission from native workflow
acceptance. Native failure/repair, manual-owner/coordinator composition, exact
configured-model/token admission, durable HTTP recovery, approved branch
commit/push, MCP and parent development measurements remain open.

No GPU/model run, scoreboard, held-out input, hidden test, threshold, dependency,
lockfile, SBOM, status model, release or owner-ready record changes. This batch
corrects worker admission and layout; it supplies no model-quality measurement.
Historical evidence bindings are not narrowed or regenerated. Held-out tasks
remain for row closes.
