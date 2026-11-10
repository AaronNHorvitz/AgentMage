# Decision 0190: Confine dormant candidate-tree construction

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0189 |
| Scope | Existing dormant temporary-index candidate builder |

Remove the dormant candidate builder's direct Git process path. Use the same
closed offline supervisor as the original owner's approved local Git effects.
Keep exact selected metadata writable and source read-only. Restrict the third
projection to an exclusively created private candidate directory, rather than
the enclosing owned root. Its index path identity now names `index` inside
`candidate-index-<build-id>`; earlier flat-index plans refuse. No API, permission,
reviewer identity, coordinator activation or commit authority is introduced.

Hold the original roots and candidate directory identities throughout the
attempt. Claim the directory relative to the held owned-root descriptor with
exclusive creation and mode 0700. Existing directories and all symlinks refuse.
Only initialize from the exact parent, hash the next exact approved postimage,
add that verified blob with its planned path/mode, and finally write the tree.
An error ends the session. A serialized plan and its hashes do not establish
execution authority.
Check both supported object formats against independently computed blob IDs.
Duplicate postimage operation identities refuse before candidate execution.

Send postimages through a sealed, rewound memory descriptor used as stdin,
separate from the withheld seccomp filter. No host postimage file or blocking
parent pipe writer is needed. Disable filters, replacement objects, split and
sparse indexes, repository-selected programs, background work and network.
Use the existing thirty-second deadline for the whole candidate session, bounded
output and whole-unit cleanup. There is no direct-process retry or fallback.

Delete only the known temporary index and lock within the held directory after
verified worker cleanup. Unknown entries, changed roots or uncertain supervision
retain the directory for explicit recovery. Never recursively delete it or
remove a preexisting candidate directory. Reconciliation of original preservation
manifests and the user's unchanged index remains required for a successful
candidate receipt. Retained scratch after an interrupted admission is not a
successful candidate or automatic retry authority.

Synthetic worker results and standalone mount experiments do not establish
native admission, systemd supervision or a measured model workflow. The existing
executable-ownership and user-bus boundaries remain. The local signing runner is
still dormant and direct; it needs its own confinement before activation.
Reviewer/artifact provenance, candidate activation, separately approved local
commit/push execution, durable recovery and measurements remain open. Both
T-303/T-306 remain open pending the executable workflow and independent review.
