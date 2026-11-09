# Decision 0175: Sealed worktree inputs for native validation

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0173 and 0174 |
| Scope | Complete bounded read-only source projection for native project tests |

Native project validation must not read a mutable host worktree through its
sandbox mount. After the original kernel launch permit and native manifest
checks, capture the whole held tree through the existing descriptor-relative
adapter. Seal each regular file with the existing file-projection primitive.
Require two matching complete observations, including identity, metadata and
content. Never follow links or silently omit files, hidden entries, generated
entries, configuration, tests, Git pointer bytes or empty directories.

Pass only sealed source descriptors to the original supervisor and native
launcher. Create the directory tree in a private sandbox tmpfs, copy file data
into read-only mounts, restore ordinary permission bits and remount the entire
directory tree read-only. Never mount the live source root for this path.
The original command, executable trust, environment, network denial, resource
bounds, permit, cancellation and cleanup owner remain authoritative. Ordinary
commands and other executor clients keep their existing mount behavior.

Capture is bounded to 128 regular files, 256 directories including root,
4,096 entries, depth 16, 16 MiB per file and 64 MiB total file bytes, with the
smaller of two seconds and the remaining command deadline. Unsafe objects,
special permission bits, unavailable cancellation, source races and incomplete
or over-budget captures refuse without launch; there is no mutable fallback.
The original exact validation approval displays this mode and its bounds.
A failed admitted attempt stays spent under the original retry rules.

Sealed file contents remain fixed after capture; the source traversal itself is
not an atomic filesystem snapshot. Inode identities, timestamps and ownership
are not reproduced in the sandbox copy. The projection is not an independent
test oracle or task grade. This increment does not bind a projection digest into
the command receipt or change either completion verifier. Current-source
acceptance, native/model execution evidence, durable recovery, approved Git and
parent measurements remain open. No row, model or release is qualified.
