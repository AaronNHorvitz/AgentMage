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

New-file transfer is not supported by this bridge. Tests use synthetic creation
provenance with real held directories, kernel grants and native atomic writes;
they do not prove native Git creation, coordinator execution or inference.
The complete native host still encounters the deferred executable-ownership
blocker in this lane. No new coding score or model admission is established.

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

The path placeholders must be replaced with absolute selected paths. The model
selection is exactly `muse` or `gpt-oss`; the scenario is exactly `python-project`
or `failed-test-repair`. This does not use the separately configured chat endpoint.
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
histories. Then send `POST /v1/code/runtime/recover` with exactly
`{"approve_recovery":true}`. Busy handlers and unreleased runs refuse. Successful
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

Repeated attach/recover cycles still retain leaked platform/profile/workspace
allocations and a held workspace descriptor in the existing factory composition.
Those allocations are unreachable after recovery; this remains a resource-lifetime
backlog item from the independent HTTP review, not evidence of durable ownership.

The new HTTP tests exercise real socket refusals and owner preservation without
synthetic creation authority. Host lifecycle tests use a scripted coordinator and
real worker threads. Neither establishes native Git creation, actual inference,
a successful full HTTP create/run/apply workflow or a coding score. Those native
observations remain unavailable under the lane's executable-ownership blocker.
