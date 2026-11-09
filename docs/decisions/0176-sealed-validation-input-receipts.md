# Decision 0176: Bind sealed validation inputs to original receipts

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decision 0175 |
| Scope | Original command receipt and bounded retained validation history |

The original native executor reports the identity of the sealed projection it
hands to the original supervisor. Use an optional typed
`sealed_workspace_input` in the original platform result and canonical command
receipt. Bind the held root, complete capture observation, ordered projection
digest, regular-file count, directory count including root and total file bytes.
The projection digest covers the version and mode, root mode, ordered directory
paths and modes, and ordered file paths, modes and exact preimages verified by
the existing file-copy-and-seal primitive. No later host read supplies this digest.
Only the indexed sealed descriptors used for these records supply source mounts.

Keep this within the original permit, native manifest, supervisor, cancellation
and cleanup boundary. Require valid digests, version and original capture bounds.
A projection belongs only to an owned-worktree command and cannot accompany a
prelaunch-failure receipt. Invalid post-launch evidence remains canonical
uncertainty with no manufactured completed command receipt. Ordinary commands,
prelaunch cancellation and legacy receipts may have no projection. Omitting this
optional extension preserves their prior canonical serialization and hashes;
the existing version-one reader accepts its absence. A new reader validates the
extension when present; older strict readers cannot consume extended receipts.

The original validation receipt already binds the entire command receipt digest.
Retain and inspect the original command receipts alongside the existing bounded
validation history, including failures. Bind the final linked pass's observation
to its exact command receipt and report whether the sealed capture's root,
observation digest and counts match that observation. A missing or mismatched
record cannot assert that match. A subsequent source change invalidates the
current observation match without changing earlier receipts or sealed input data.

These are trusted executor provenance and sampled source comparisons, not
independent grading, an atomic host snapshot or execution authority. A digest
does not authenticate a caller-created record. Both completion verifiers and
`current_workspace_verified`/`test_suite_verified` stay unchanged and false.
Native/model workflow evidence, reviewed completion acceptance, durable recovery,
approved Git and parent measurements remain open. No row or model is qualified.
