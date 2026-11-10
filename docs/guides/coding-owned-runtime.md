# Live created-worktree ownership in the coding coordinator

The Linux repository owner and existing development coding coordinator now have
an in-process ownership handoff. The manual HTTP service now composes this path
with the existing coordinator, as described below. Its separate-process launcher
remains available without ownership transfer. Positive native create/run/apply
execution remains unobserved in this lane.
See [Decision 0158](../decisions/0158-live-worktree-authority-in-the-native-coordinator.md).

After successful native worktree creation, show the owner's held worktree root
identity and obtain explicit handoff consent. Consume that same owner through
`LinuxCodingRepository::into_runtime_worktree(expected_root_sha256)`; refusal returns
the owner and its error intact. The returned
`LinuxCodingWorktreeLease` is process-only, cannot be decoded from input and
confers no read, model, command or write approval. Pending manual decisions are
discarded. Keep one lease for explicit recovery; attach it to the existing
`CodingDevelopmentRuntimeFactory` before preparing its first session with
`attach_owned_worktree`. Its independently validated activation must select the
exact created path. It still needs the ordinary private roots and explicit
[linked metadata activation](coding-linked-worktrees.md).

The factory carries a lease into each native boundary. Existing-file structured
edits, selected hunks and rollback use the unchanged native atomic writer under
the same kernel grants. Its wrapper binds successful native writes to the held
creation owner. The owner is locked through transaction reconciliation and
terminal publication. Concurrent writers refuse. A lost publication or uncertain
write removes those attempted paths' eligibility for original transfer.

After ending and releasing every coordinator and dropping the factory, call
`try_into_repository` on the retained lease. If references remain, it returns the
lease intact; do not create a new owner or adopt a path to bypass that refusal.
The recovered owner keeps its original held roots, creation receipt and
cumulative successful edit lineage. It can prepare a new original-checkout diff
after read consent, followed by a separate single-use apply approval. Previous
manual approvals cannot be reused. Restart cannot recover the lease or lineage.
Poisoning clears the created ownership and pending decisions before recovery.

Native controlled-file creation now uses the same live owner lock. Only a
successful native creation and matching verified transaction after terminal
publication establish new-file transfer eligibility. Subsequent edits preserve
that creation's absent-before provenance and native mode. Recover the owner,
then use the existing [original preview and approval](coding-original-apply.md);
no original write follows automatically. Mixed edit/creation selections refuse;
missing parents are not created. Manually added files, caller receipts and
restart cannot establish provenance. Tests use synthetic worktree-creation
provenance with real held directories, kernel grants and native file writes;
they do not prove native Git creation, coordinator execution or inference.
The complete native host still encounters the deferred executable-ownership
blocker in this lane. No new coding score or model admission is established.

Original write-history review permits a retry to repeat an earlier content
change after a rollback when each checked application is distinct. It retains
every original approval and application separately. Reusing an application
digest refuses even if a different change-set digest is supplied. A final
net-zero path still refuses review; returning temporarily to the original bytes
inside a later nonzero history is permitted. The sixty-four-transaction and
eight-MiB history limits remain in force. This supplies no new write, test,
reviewer, commit or push authority.

## HTTP handoff in the manual coding service

`serve-coding` now has an explicit in-process path into the same native coding
coordinator. First create a worktree through the existing repository preview and
single-use decision routes. Install its private linked development marker through
the existing explicit setup procedure. Keep the original metadata paths exact;
a marker never supplies creation authority. Run this service inside the existing
bounded GPU reservation whenever it can start native model work.

Using the control credential, send `POST /v1/code/runtime/attach` with these exact
JSON fields:

```json
{
  "state_root": "<separate-private-runtime-state>",
  "disposable_root": "<private-disposable-root>",
  "workspace_root": "<created-marked-worktree>",
  "expected_worktree_root_sha256": "<identity-shown-by-repository-edit-status>",
  "model": "gpt-oss",
  "scenario": "python-project",
  "approve_inspection": true,
  "approve_handoff": true
}
```

For either project scenario, the attach body may also contain `source_paths`,
for example `"source_paths": [["calculator.py"], ["package", "core.py"]]`.
This chooses one to sixteen exact existing Python source files, including files
outside `src/`. Component arrays are relative to the held worktree. The list
is canonicalized and frozen at startup; empty, duplicate, unsafe, missing or
inadmissible paths refuse the attachment. Discovery modules whose basename
starts with `test`, package initializers named `__init__.py`, directories named
`test` or `tests`, and hidden components are excluded. Omitting the field or
setting it to null keeps the default `src/` selection with the same exclusions.
An explicit list replaces the
default; it never adds the remaining `src/` files. Other scenarios refuse it.
The runtime status exposes `selected_source_paths`; the prepared request shows
the actual mutable files. A prepare body cannot supply a new selection.
See [source selection](coding-runtime-api.md#explicit-python-source-files).

The path placeholders must be replaced with absolute selected paths. The model
selection is exactly `muse` or `gpt-oss`; the scenario is exactly
`python-project`, `python-root-project` or `failed-test-repair`. This does not use the separately configured chat endpoint.
There is no resume flag: handoff starts a new process-held ownership lifecycle.
The runtime state must be separate from the manual authority store and held
repositories. Startup may inspect the chosen activation, initialize private
runtime state and freeze the repository composition. It grants no model call,
command, source edit or original-checkout application.

The service consumes its actual live owner before activation. A path, marker,
receipt or serialized object cannot replace it. Invalid identity, absent live
creation, concurrent references, activation failure and composition failure refuse
without adopting a workspace. Normal handoff refusals return the same owner;
successful lease creation invalidates pending manual decisions even if later
composition refuses. Poisoned manual ownership remains poisoned and unavailable.

While attached, use the existing [runtime routes](coding-runtime-api.md) and
`runtime-run` client. Every model disclosure and tool effect retains its existing
approval. Chat, settings and manual repository effects refuse during this period;
status and activity remain available. Embedded job control uses one fixed scope
for the authenticated control client, and the canonical ledger still decides each
request. HTTP input cannot select a peer identity.

Release every prepared or active run with the existing `/v1/code/runtime/release`
route. Even a terminal run must be explicitly released, closing its canonical
histories. Read `GET /v1/code/runtime/owner` using the control credential. It
returns the held `worktree_root_sha256` and a `recovery_sha256` unique to this
handoff, for both attached and pending recovery. It reads no source, credential or
activation path. The identity remains readable if the internal owner is poisoned;
ordinary recovery still discards poisoned creation authority and pending approvals.
Display both digests and obtain explicit consent before sending exactly:

```json
{
  "expected_worktree_root_sha256": "<displayed-held-root-digest>",
  "expected_recovery_sha256": "<displayed-handoff-digest>",
  "approve_recovery": true
}
```

Send this body to `POST /v1/code/runtime/recover`. Both digests must be lowercase
64-character SHA-256 strings. Missing fields, unknown fields, duplicate keys and
array requests refuse. A mismatched identity returns HTTP 409
`code.runtime.recovery-stale` before extracting or dropping the current owner,
service or recovery slot. The comparison holds the same state lock as ownership
publication. Reattaching the same created root produces a fresh random handoff
identity; consent for its previous attachment cannot recover it. Clones and busy
recovery retries retain the same identity. The digests identify consent and confer
no authority or source-read permission. They cannot construct a live lease.
Busy handlers and unreleased runs refuse. Successful
recovery drops the empty coordinator service and returns the original manual
owner. Inspect `/v1/code/repository/apply`, approve reading the exact roots and
paths, review a fresh full diff, and provide a new single-use apply decision.
Recovery itself applies nothing. The separate-process runtime service cannot use
this recovery route because it holds no creation owner.

If recovery answers HTTP 409 `code.runtime.owner-held`, the service retains the
same live lease in a recovery slot. `GET /v1/status` reports
`runtime_recovery_pending:true` and `runtime_attached:false`. Manual operations
and another attach remain unavailable. Retry the same explicit recovery request
after the remaining references end; no new path, marker or receipt is accepted.
Success consumes the slot once and still requires a fresh original diff and apply
decision. Recovery of a poisoned lease clears creation authority and all pending
decisions as before. A restart cannot recover this process-only slot.

Manual operation admission checks ownership under the same state lock that
publishes attach and recovery. Settings/workspace mutations and inspection also
recheck under that lock, so passing an earlier HTTP route check cannot authorize
a concurrent manual action. Only control-authenticated ownership transitions may
acquire their operation lease while attached or awaiting recovery.

The coding service registers interrupt, termination and hangup signals through
the existing stop flag. On shutdown, the embedded service signals its passive cancellation probe,
drains results and joins its worker threads before returning. A previously
accepted cancellation stays authoritative. Shutdown waits for native bounded work
to return; it does not claim an acknowledged job-control decision or synthesize a
successful terminal record. Incomplete histories remain incomplete. Hard process
termination still loses process-only ownership; restart cannot reconstruct it.

Failed starts and continuations also drain and join their workers before
discarding sessions that never entered the active-run registry. A startup error can
therefore wait for an already-running bounded call to return. It cannot report
an empty service while that discarded session still owns a live worker. No
acknowledged cancellation or terminal record is invented on this path.

The development factory now shares ownership of its platform, immutable profile
and descriptor-bound workspace with its coordinators. Refreshing the repository
composition releases the previous binding when no run retains it. Releasing runs
and recovering the manual owner drops the embedded factory's references; the last
owner releases the foundation. These three objects no longer use permanent
factory allocations.
A pending operation retains its exact original root descriptor independently,
without reopening a path, and rechecks its held targets before use. Sharing a
binding supplies neither creation authority nor an approval. This lifetime change
does not make worktree ownership durable across process termination.

The [resource-lifetime record](../evidence/t306-runtime-lifetimes-2026-11-07.md)
distinguishes actual descriptor checks from the still-unobserved full native
attach/run/recover workflow. The native host trust blocker remains unchanged.

The new HTTP tests exercise real socket refusals and owner preservation without
synthetic creation authority. Host lifecycle tests use a scripted coordinator and
real worker threads. Neither establishes native Git creation, actual inference,
a successful full HTTP create/run/apply workflow or a coding score. Those native
observations remain unavailable under the lane's executable-ownership blocker.

## Terminal handoff and recovery

The same `agentmage-code` terminal client now covers the ownership transitions.
Create the worktree and set up its explicit linked development activation as
above first. The service must still run under the GPU reservation whenever it
can start inference. Replace the quoted placeholders with the selected absolute
private paths and the service's loopback address:

```sh
agentmage-code '<loopback-ip:port>' '<private-auth-directory>' runtime-attach \
  '<private-runtime-state>' '<private-disposable-root>' '<created-worktree>' \
  gpt-oss python-root-project calculator.py package/core.py
```

The model is exactly `muse` or `gpt-oss`. Choose `python-project` for packaged
tests, `python-root-project` for root discovery, or `failed-test-repair` for the
fixed fixture. Optional trailing relative source paths are available only to the
two project scenarios. Omit them for the default `src/` selection. They use the
same one-to-sixteen exact-file admission and exclusions as the HTTP request.
The client canonicalizes their order without reading files.

Project unittest output remains a trusted-runner observation, not independent
grading. Package initializers are excluded from both default and explicit mutable
sources in both layouts: an initializer can replace subtree discovery through
its `load_tests` hook. This conservative rule does not depend on whether the
repository map happens to include every descendant test. It also refuses an
initializer in a mixed explicit list rather than silently narrowing that list.
Existing sessions whose default scope included initializers require a fresh run
and consent; their changed profile digest refuses resume. This addresses the
narrow hook from the [source-selection review](../reviews/a8bdb80dc506-local-coding-explicit-python-sources.txt),
accepted by the [independent initializer-scope review](../reviews/4f73d93ea3ac-local-coding-python-initializer-scope.txt).
Editable source can still alter test behavior. An explicitly selected existing
top-level file named after a standard-library module can shadow a module the
runner imports lazily, even when tests do not import that file. This remains low
backlog in the same row; neither layout's project result is independent grading.
Only the unchanged frozen scorer supplies independent coding scores.
See the [repair record](../evidence/t306-python-initializer-scope-2026-11-07.md).

The client reads the held root digest from repository edit status, displays the
complete attach request and its inspection/handoff consent fields, and requires
`attach <held-root-digest>` in the controlling terminal. The displayed request
includes that held root, model, scenario and source list; the client sends those
same values after confirmation. The server independently verifies creation
ownership and activation. A path or digest
cannot adopt a worktree. This confirmation discards pending manual decisions and
allows inspection and ownership transfer; model disclosure and every tool still
need their ordinary later approvals. Use `runtime` and `runtime-run` afterward.

Release every prepared or active run using `runtime-release`, including terminal
runs. Then use:

```sh
agentmage-code '<loopback-ip:port>' '<private-auth-directory>' runtime-recover
```

The client reads service state, then the control-only owner snapshot. It shows
no recovery confirmation if the snapshots disagree on whether recovery is
pending. Inspect again for a fresh consistent pair; it does not retry or send
an ownership decision from the inconsistent pair. The two reads are not an
atomic snapshot, and a matching pair does not replace the server's later checks.
For a consistent pair, it shows
the held root and handoff digests for attached and pending recovery alike and
requires `recover <handoff-digest>` in the controlling terminal. It sends those
exact digests once and checks the echoed identity. A changed handoff refuses;
inspect again and obtain fresh consent. Busy, unavailable or separately launched
runtime states refuse. Recovery does not cancel or release runs or approve original
writes. The server rechecks lifecycle and live references. After recovery, use
`original` and a fresh original diff/apply approval as before.

Both commands reject piped approvals and make only one ownership POST after a
matching confirmation. A refused, malformed or lost reply is an error, never an
automatic retry. Inspect `status` and `runtime` before choosing another action.
The terminal contracts use synthetic HTTP peers and real controlling terminals;
they are client evidence, not a positive native workflow or model score.

Original-checkout preservation and inventory Git reads now use the same offline,
read-only inspection worker as linked-worktree reads. The exact selected metadata
below the original checkout is mapped into the guest workspace; configuration
includes and object alternates cannot expose arbitrary host files. Both paths
require the existing trusted launcher prerequisites. Ordinary observations also
use the worker's 4 MiB output ceiling: oversize, cancellation or worker failure
refuses collection, with no direct host-process fallback. This read confinement
does not enable candidate construction, commits or pushes. Native positive
execution and complete coding workflow acceptance remain unverified in this lane.
Ordinary observations also inherit the read worker's fifteen-second deadline,
256 MiB memory limit, sixteen tasks and 4 MiB output ceiling; inventory requests
up to 32 MiB are capped at 4 MiB. Large ignored build/dependency listings can
therefore refuse inventory. Representative native capacity remains unmeasured.
Because external registered worktrees are absent from the read worker's guest
filesystem, its `worktree list` cannot distinguish an existing external checkout
from a deleted one without pruned metadata. The preservation digest retains
metadata but no longer proves those external directories exist. Nested linked
repositories can also appear as individual untracked files. These reduced
observation signals remain same-row backlog; they authorize no wider mount.

The existing approved worktree mutation adapter now uses a separate closed
owner in the offline supervisor. It exposes the original checkout read-only,
the exact selected Git metadata and owned scratch root writable, and no host
ancestor. Canonical paths inside the guest preserve persistent worktree pointers.
All three held directory identities are checked before execution and afterward;
changed roots or incomplete output refuse success. The existing plan, approval,
preservation checks, thirty-second deadline and output ceiling remain required.
There is no direct-process fallback. Runtime mount collisions, linked read-only
scopes and network operations refuse. Git's installed helper symlinks resolve
through a guest `/usr/bin/git` symlink to the held executable mounted read-only
at `/app/git`; no host executable directory is exposed for them. Native positive
execution remains unverified here; this does not activate candidate or local-commit adapters.
See [Decision 0189](../decisions/0189-confine-approved-local-worktree-git.md).
