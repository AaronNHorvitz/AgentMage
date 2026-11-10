# Decision 0187: Verify immutable base preimages before candidate planning

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0182–0186 |
| Scope | Immutable source preimages in the original live coding owner |

An approved filesystem write proves what the person allowed to change. Its
starting bytes can still contain dirty or untracked work. Before the existing
live owner releases a candidate-tree plan, compare the first preimage of every
reviewed path with the exact commit retained from native worktree creation.
Require absence for a creation and exact content for an existing-file edit.
An existing empty file is distinct from an absent file. A mismatch refuses
planning; it never silently includes starting changes in a future commit.

Read the fixed commit and its tree through the existing confined, offline
linked-repository inspection worker, after explicit source and Git metadata
read consent. Disable replacement objects on every base read. Select paths
by exact equality in parsed tree records; never turn paths into revision
expressions or pathspecs. Only ordinary base blobs are admissible. Refuse
selected trees, symlinks, gitlinks and descendants of non-tree entries.
Verify every selected blob's Git object identity before hashing its content.

Retain the existing sixteen-path and 128 KiB per-blob limits. Bound the complete
tree transcript by the existing worker's 4 MiB output ceiling and at most
100,000 entries. Existing worker deadlines, supervision, offline confinement
and executable trust remain unchanged. Repeat the base observations with the
existing final Git observation; any change refuses release of the inert plan.
These observations provide no atomic filesystem or Git lease.

The original approval/application history, creation-publication checks, sealed
validation, held roots, fresh Git state and independent review remain required.
There is no new route, grant or candidate executor. Reviewer and artifact
provenance, candidate execution, separately approved Git effects, native/model
measurement and durable recovery remain open. T-303/T-306 remain open; no new
row or sub-row and no self-approval.
