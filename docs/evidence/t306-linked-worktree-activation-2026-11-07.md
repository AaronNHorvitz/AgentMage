# T-303/T-306: Linked-worktree development activation

Builder record, 2026-11-07. Base `9008ee386cc44eea8984e64f198dae6769567892`.
Both parents remain open. Independent core review is required for this batch;
there is no new row, sub-row, completion checkbox or model score. The previous
[independent repair PASS](../reviews/9008ee386cc4-local-coding-read-consent-repair.txt)
is retained unchanged. Its low owned-edit read-root binding finding remains
backlog; it is not silently represented as fixed here.

## Running implementation

[Decision 0157](../decisions/0157-linked-worktree-development-activation.md) and
the [operator guide](../guides/coding-linked-worktrees.md) describe the extension
to the existing native coordinator. Activation, inventory composition and the
read-only Git executor now share a linked metadata binding. No second loop or
new frontend orchestration is introduced.

The binding checks exact selected paths before opening external metadata, holds
directory and pointer-file descriptors, and verifies the canonical Git layout,
ownership, modes, bounded pointer bytes and backlink. The linked marker names
both metadata roots and their complete binding digest. Activation checks its
private bounded marker before resolving external pointers. A legacy marker
refuses even when external metadata is unavailable. A replaced pointer cannot
inherit an earlier process's activation. HEAD content can change while its
regular-file safety checks remain in force.

The new host command displays linked marker bytes only after all three paths
are explicitly selected; it writes nothing. The ordinary marker command refuses
linked layouts. The existing API/terminal native service inherits the activation
through its current host launcher; its model selection, approvals and dedicated
service boundary stay unchanged.
Linked inventory independently revalidates the activation and admits only its
exact private marker as untracked/ignored metadata, retaining the map's denial
of that path as a source or edit target. Other inventory refusals remain. The
real Git fixture uses the existing task-branch namespace; detached admission to
the native profile is not added. Neither setup command creates a branch.

Linked startup inventory uses the existing offline Git worker, rather than the
ordinary unconfined collector. Bound workspace/metadata descriptors become
read-only mounts; explicit guest Git/common paths replace host pointer lookup.
Config includes and object alternates cannot introduce a host filesystem mount.
The shared worker retains executable trust, namespaces, seccomp, cancellation,
resource limits, per-query output caps and owned cleanup. Model tools still use
kernel-prepared plans and launch permits. Linked scopes refuse full preservation
manifests and mutating Git execution before dispatch.

## Verification

The complete touched-crate gate selects the host, local API and Linux platform,
with all targets and no test filters, failure allowlist or new skips:

```sh
python3 -B scripts/local_agent_fast_gate.py --base 9008ee386cc44eea8984e64f198dae6769567892 --timeout 3600
```

| Check | Observed result |
| --- | --- |
| Formatting and Clippy, warnings denied | PASS |
| Complete source tests | **FAIL:** 900 passed, 72 failed, 62 existing ignored, zero filtered |
| Failure identity comparison | Same 53 host and 19 Linux baseline failures, no new identity |
| Focused host activation tests | 2 passed, 586 intentionally filtered; separate from the complete gate |
| Marker command executable contract | PASS in the complete gate; ordinary/linked byte output, no file creation and malformed-layout refusal |

All nine new methods pass in the complete run. They cover real filesystem
pointer/root identities, unsafe modes, aliases, links, malformed and oversized
pointers, stale/other-root projections, read-only guest mappings, preservation
refusal, exact marker-only inventory admission, activation and the actual marker
command. The inert artifact and synthetic metadata/mount controls exist only in
tests and never supply a production trust bypass.

The real Git component creates a disposable repository and linked task branch,
then checks explicit marker installation, activation, legacy-marker refusal with
both available and unavailable metadata, and identical-byte pointer substitution.
Its observed elapsed time is **9,736 microseconds**. Git creates only the fixture;
this does not execute the pinned worker, coordinator or model. Earlier focused
Linux controls passed seven selected methods, including six new controls; their
platform source bytes are unchanged by the later host-only marker correction.

The first complete run had 899 passes and the same 72 failures. A subsequent
integration check found that the inventory guard would require the private marker
to be tracked. The in-row correction admits only the independently revalidated
linked activation marker and adds a negative regression; the final complete run
above includes that correction. Earlier logs and source hashes remain retained.
An initial gate launch failed before testing because PATH omitted Cargo; the
corrected launch preserves Cargo's PATH and the failed log. No threshold or
hidden input changed.

The final gate records 1,463.35 seconds for formatting, 34.11 for Clippy and
112.84 for complete tests, including shared reservation waits and compilation.
Those are development-check timings, not model/GPU/task measurements. The full
gate remains FAIL; T-301.1's native executable-trust refusal is not bypassed.

Raw logs, the eight-file Rust source freeze, exact 14-path audit, complete failure
identities, prior-review byte comparison, unchanged ledger/status checks and
privacy checks remain in private lane state. Source policy checks cover 785
Markdown files and 9,034 complete current-source inputs with the required legacy
link target; ignored build caches are excluded. The unchanged policy validator
and Markdown/review lint are separate from the deferred T-310 long gate. No
historical evidence input binding is narrowed or renewed by these checks.

## Remaining work

The manual service-created worktree owner and canonical coordinator still have
separate lifetimes. This binding cannot adopt manual creation receipts, edit
lineage or pending approvals, including after restart. Positive native
failure/repair execution remains unmeasured in the lane with deferred T-301.1
executable-trust refusal. Configured-endpoint exact model/token admission,
coordinator-connected original application, durable HTTP recovery, approved
branch commit/push, MCP and parent development measurements remain open.

No GPU/model run, held-out task, hidden test, dependency, lockfile, SBOM, status
model, release or owner-ready record changes. Historical evidence is not renewed
or represented as current by this batch. Held-out tasks remain for row closes.
