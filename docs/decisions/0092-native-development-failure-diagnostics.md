# Decision 0092: Preserve Native Development Failure Diagnostics

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061 and 0081; current owner restart |
| Scope | AMR-01 launch and tool-composition diagnostics |

## Finding

The current actual CLI/host launch refused the native Git executable before
serving IPC. The host emitted only `coding.development.platform-failed` and the
client added `coding.development.client.transport-failed`. The retained
[Linux startup observation](../verification/linux-current-startup-2026-09-27.md)
records the refusal, empty event stream and preserved disposable repository.
Doctor separately reported untrusted native paths. A failed launch-envelope
transfer does not establish whether a native prerequisite or authentication failed.

## Decision

Preserve the existing native repository, command, read-sandbox and development
boundary error categories in the development composition. Emit only their closed,
static codes. Repository setup returns its actual native error; tool composition
reports its native cause to local stderr while preserving the existing closed IPC
failure. A private-directory rejection also retains its native category. Validation
command preparation must not replace an already classified error with a generic one.

Give the existing development process boundary a fixed code for each error kind.
The CLI preserves missing/unsafe executable, failed spawn, failed envelope transfer
and failed process-control categories. Peer authentication and unexpected host exit
keep their existing transport disposition. Existing process exit classes remain
unchanged. Never parse stderr, expose raw child output as a diagnostic, or derive
codes from paths, arguments, process identities or model/workspace content.

These diagnostics confer no trust or effect authority. Keep all activation,
executable, confinement, peer, model and resource checks. A clean lifecycle record
or successful diagnostic command does not admit native execution. No IPC schema,
error-frame handshake, process owner, canonical store or release path is introduced.
Crash-safe worker ownership and the native workflow remain separate open work.

## Verification

Exercise actual native input refusals without launching tools and verify category
preservation and content-free output. Rebuild and exercise the real CLI/host startup
refusal in a fresh disposable repository; retain exact binary/source identities,
exit codes, event absence and worktree/index preservation. Test missing and invalid
sibling executables separately as startup diagnostics, with synthetic executables
clearly labelled. Native successful coding, real-model, manual, independent review
and release acceptance remain open. Regenerate evidence once after the source batch.
