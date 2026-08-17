# Decision 0015: Canonical Held Targets and Exact-Object Workers

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | Grant target representation, consumed-permit target binding, and Linux worker object exposure |
| Resolves | `RM-009`, `RM-010` |
| Amends | Decision 0014 permit payload and Linux sandbox boundary |
| Preserves | Canonical operation taxonomy, kernel authority ownership, immutable historical evidence, and accepted product scope |

## Context

The Phase 5 permit made raw effect bypass structurally unavailable, but two
authority gaps remained. `GrantTarget` retained raw strings under weaker rules
than `WorkspacePath`, and the Linux read worker received the authorized
workspace-root descriptor even when the consumed operation named one file.
Session grants also need to represent an explicitly approved root, while an
operation path must remain non-empty.

Target canonicalization and worker isolation must be one decision. A canonical
grant that is later converted back into a broad root mount would not preserve
its authority, and an exact mount whose grant can name a weaker path would not
have a trustworthy source binding.

## Decision

### Canonical Scope and Object Forms

1. `WorkspacePath` remains non-empty and is the only path accepted by a
   platform adapter.
2. `WorkspaceScopePath` may contain zero components for an explicitly approved
   workspace root. Every non-root component uses the same validation and
   normalization implementation as `WorkspacePath`.
3. `GrantTarget` has private fields and two tagged wire variants:
   `workspace_scope` and `held_object`.
4. A workspace scope binds canonical scope, workspace authorization ID,
   adapter-instance ID, and platform.
5. A held-object target binds canonical non-empty path, the same authority and
   platform identities, object kind, mount/object identity digests, and a
   required nullable preimage. Regular files require an exact bounded
   preimage; directories prohibit one.
6. Session parents contain only scope targets and scope exclusions. Operation
   grants contain only exact held-object targets and inherit only scope
   exclusions. Scope containment requires equal workspace authorization,
   adapter, and platform identity before component-prefix comparison.
7. `GrantPreimage` must equal the canonical file digest and object revision
   derived from its indexed target. Missing, duplicate, stale, or extraneous
   preimages fail issuance and approval rendering.

### Consumption and Launch

1. `EffectAuthorization` borrows the exact targets, inherited exclusions, and
   preimages from the consumed grant in addition to its existing transaction,
   attempt, operation, digest, and tool-call bindings.
2. A driver may ask the opaque permit whether one continuously held object is
   the exact authorized target. The check rejects target count, path,
   authorization, adapter, platform, kind, identity, preimage, and exclusion
   drift.
3. The Linux driver owns one `LinuxHeldObject`, not an authorized workspace
   root. It performs the permit comparison before entering the runner.
4. The runner revalidates the continuously held root and object identities and
   the exact file preimage before invoking `systemd-run` or Bubblewrap.

### Worker Object Exposure

1. For a file operation, the supervisor copies only the grant-bound preimage
   from the exact continuously held descriptor into an anonymous file, verifies
   its byte count and SHA-256, revalidates the held object, and seals the
   projection against writes and size changes. Bubblewrap copies only that
   immutable projection to `/input/object`. Neither the original file nor the
   workspace-root descriptor is passed to the user manager, Bubblewrap, or
   worker.
2. A directory worker does not receive the source directory. The supervisor
   performs bounded descriptor enumeration, excludes the first descendant
   covered by each inherited exclusion, sorts raw entry names, and creates a
   sealed NUL-delimited anonymous projection. The held directory is revalidated
   after enumeration. Bubblewrap copies only that projection to
   `/input/object`.
3. `/tmp` remains a separately bounded private filesystem. The worker receives
   no canonical writable workspace object, sibling, parent, ambient
   `/workspace`, host home, or host network.
4. Target mismatch, stale object, and projection failure use stable
   content-free errors and have no weaker fallback.

### Wire Compatibility

The live contract schema remains version 2. Phase 4 introduced version 2, but
no durable grant store, released API, or compatibility promise exists before
Phase 7. The Phase 6 correction therefore replaces the raw version-2 target
shape before persistence. Previously serialized raw version-2 `GrantTarget`
objects fail closed; they are not silently migrated or reinterpreted.

Frozen version-1 Story 4.1 and Story 5.1 fixtures and reports remain unchanged
and revision-bound. Any future change after a durable or released version-2
record exists must use an explicit schema increment and migration decision.

## Consequences

### Subsequent Coding-Runtime Extension

The interactive coding runtime later required exact command and controlled
direct-child creation authority over an AgentMage-owned worktree root. A root
cannot be represented by `WorkspacePath` without weakening this decision's
non-empty child-path invariant. The contract therefore gained a closed
`held_workspace_root` operation-target variant and a `HeldWorkspaceRoot` trait.
The variant retains an empty canonical root scope only as path-free identity,
binds the continuously held root descriptor's platform identity, carries no file
preimage, and is never accepted as a session scope or held child object. Any
inherited subtree exclusion overlaps this root target and denies the operation.
Existing `workspace_scope` and `held_object` wire records remain unchanged and
continue to deserialize exactly; unknown root targets continue to fail closed
in older readers.

This additive pre-release extension explicitly amends the wire-compatibility
paragraph above for this one target kind. Durable version-2 records using the
two earlier variants require no rewrite, reinterpretation, or store migration;
the current reader accepts them unchanged. Rolling back to an older reader
after issuing a held-root grant fails closed on the unknown variant. Any target
shape change after a public compatibility promise still requires an explicit
schema increment and migration decision.

- A target rejected by the canonical path parser cannot be represented as a
  typed operation target or admitted through target deserialization.
- A changed workspace authorization, adapter, platform identity, native object,
  or file preimage cannot match the consumed permit.
- A file changed after authorization cannot alter the bytes observed by a
  launched worker; only a sealed projection matching the approved preimage is
  exposed.
- One-file authority cannot expose a sibling by relative path, parent traversal,
  workspace-root enumeration, or `/proc/self/fd` discovery.
- Directory exclusions are absent from the worker's only input object; the
  worker cannot recover hidden names by opening the original directory.
- Directory projection is intentionally a bounded observation format, not a
  mounted directory API. A future recursive directory protocol requires its
  own closed contract and review.
- The application host still does not compose a complete user-facing workflow,
  and authority state remains in-memory. Phase 7 still owns durable encrypted
  transactions and restart recovery.

## Verification

- Canonical path and target tests cover empty, dot, traversal, wildcard,
  rooted, separator, encoded separator, colon, ambiguous suffix, invisible
  format, Unicode normalization, workspace mismatch, and wire invariants.
- Grant, approval, and policy tests cover scope/object variant separation,
  authorization affinity, exact preimages, exclusions, stale context, and
  serialization.
- Authority-transaction tests prove exact held authorization succeeds while
  changed authorization and object identity fail before the driver records a
  worker launch.
- Default Linux tests prove immutable exact-file projection, bounded directory
  projection, and stale held-object denial without requiring a worker process.
- Eleven environment-dependent Linux sandbox tests were executed locally on
  Fedora Kinoite 44 and passed exact-file, directory-projection, sibling,
  parent, descriptor-discovery, scratch, read-only, ambient-path, environment,
  network, seccomp, output, and runtime-limit checks.
- Workspace format, Clippy, tests, documentation, and repository-diff gates
  passed before acceptance.

## Approval Gate

The user approved this decision, the Phase 6 local commit, and entry into Phase
7 on 2026-08-11. That approval did not authorize a push or a Phase 7 commit.
