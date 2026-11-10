# Decision 0188: Confine original repository observations

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0182–0187 |
| Scope | Git reads made by the existing Linux repository collector |

Use the existing offline read-only inspection worker for original-checkout
preservation and inventory observations as well as linked-worktree inventory.
Explicit read consent and hardened Git options do not confine a host process:
repository configuration and object alternates can refer to other host files.
The collector must have no direct-process observation fallback.

For ordinary scopes, translate the exact selected metadata directory below the
checkout into the worker's read-only `/work` projection. Pass explicit guest
Git-directory and worktree arguments. Never discover a different repository
through ambient paths or project another host directory. Linked scopes retain
their existing descriptor-bound common metadata projection and closed supervisor
owner. Revalidate scope and held root before accepting observations.

Keep the existing trusted executables, offline namespaces, read-only mounts,
seccomp, descendant supervision, cancellation and worker deadlines. Apply the
worker's existing 4 MiB output ceiling to ordinary observations too; larger
inventories refuse rather than truncate or raise a limit. Cancelled, incomplete,
failed and oversized worker results cannot produce accepted observations or
trigger an unsandboxed retry. Optional missing-ref exits remain distinct from
execution failures. A successful observation remains a bounded read, not an
atomic Git lease, independent review or permission for a later effect.

The historical direct-process timeout helper remains compiled only in tests,
so its existing regression is retained without leaving a product dispatch path.
The existing approved worktree mutation adapter is unchanged: its bounded
Git child still requires separate confinement work. The dormant candidate and
local-commit adapters are not activated. Their separate execution and authority
requirements remain open. No new route, grant, candidate build, commit or push
is introduced. T-303/T-306
remain open pending the executable workflow and independent acceptance.
