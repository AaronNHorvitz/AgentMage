# Linked worktrees in the development coding host

The existing native coding host accepts an explicitly marked linked Git worktree
under its private disposable root. This connects the linked layout to the
existing activation, inventory and offline Git inspection owners. It does not
adopt the manual API's creation receipt or edit lineage and does not qualify a
model or a complete coding workflow. T-303 and T-306 remain open. See
[Decision 0157](../decisions/0157-linked-worktree-development-activation.md).

## Explicit setup

Use a disposable linked worktree whose `.git` file names its absolute canonical
metadata directory. Its metadata must have the ordinary `worktrees/<name>`
layout, `commondir` containing exactly `../..` and a backlink naming this
worktree's `.git` file. Relative gitdir pointers, symlinked ancestors, unsafe
modes, hardlinked pointer files and metadata outside that layout are refused.
The workspace itself and the separate state/disposable roots still need their
existing owner-only permissions and separation. No active original checkout is
used as the coding workspace.
The native profile still requires a branch under `refs/heads/agentmage/tasks/`
and the selected scenario's project/test layout. Detached worktrees can validate
their metadata but do not satisfy that profile. This extension creates no branch.

Build the existing host under the shared reservation, then display the proposed
marker locally:

```sh
bash /tools/build-slot cargo build --locked --offline -p agentmage-host --bin agentmage-host
target/debug/agentmage-host --coding-development-linked-marker <absolute-disposable-worktree> <absolute-git-metadata> <absolute-git-common>
```

This command reads the selected workspace pointer and only the two explicitly
named metadata roots, then prints exact setup bytes. A mismatched pointer refuses
before opening external metadata. The ordinary `--coding-development-marker
<workspace>` command refuses linked layouts. It starts no Git command, model or service and creates no file.
Its output contains local paths; do not commit or publish it. Review the complete
output before installing those exact bytes as
`<absolute-disposable-worktree>/.agentmage-development-workspace` with mode `0600`.
Use an explicit local ignore rule for this private marker so ordinary Git status
does not report it as a source change. The operator configures that rule; marker
generation changes no Git configuration. Linked inventory admits only this exact
independently validated marker as untracked or ignored metadata, while the map
continues to deny it as a source or edit target. Other untracked, ignored, staged
or conflicted entries retain their existing refusal. Do not overwrite an existing
marker to repair an unexplained mismatch.

For a linked worktree the marker includes `git-metadata`, `git-common` and
`metadata-sha256`. Starting the native development host explicitly authorizes
read-only access to those two metadata roots as well as the selected workspace.
The shared root contains the original repository's objects, references and local
configuration; this is additional read scope. A workspace-only marker refuses
linked activation. Ordinary-worktree marker bytes stay unchanged.

After setup use the existing [native API and terminal workflow](coding-runtime-api.md)
and its explicit scenario/model selection. The linked extension does not enable
another model or an arbitrary endpoint. Real-model work still uses the GPU slot.
Each consequential model tool still needs its separate exact approval.

## Read boundary and recovery

Both linked startup inventory and later Git inspection use the same confined
offline worker, with the workspace and both metadata roots mounted read-only.
The worker uses explicit guest Git/common paths; the host's absolute pointer
targets are not followed inside it.
The platform adds fixed `--git-dir=/metadata/worktree` and `--work-tree=/work`
arguments and sets `GIT_COMMON_DIR=/metadata/common`. These namespace mappings
come from the activated metadata owner, with no path supplied by model output.
Host paths referenced by untrusted config
includes or object alternates are absent from that mount namespace. System
runtime mounts remain those of the existing worker. No original source checkout,
home directory or network is added. Inventory retains its per-query byte caps;
each linked worker invocation uses the existing 15-second inspection deadline,
resource bounds and cleanup owner. A slow or oversized inventory refuses startup.

The activation holds directory and pointer-file descriptors and checks their
current identities. Replacing a root or a pointer file with identical bytes
cannot inherit that process's activation. Metadata pointer drift refuses even
when the selected source files are unchanged. Ordinary HEAD content changes are
allowed; a linked, unsafe or oversized HEAD file refuses. After a legitimate Git
repair that replaces pointer files, stop the service, inspect the full layout
and explicitly prepare/review a fresh marker. Never automatically regenerate a
marker after a refusal.

This read-only metadata binding is not manual worktree ownership. Restart cannot
use it to recover a manual creation receipt, transferable edit provenance or a
pending write approval. Linked inventory scopes refuse preservation-manifest
collection and mutating Git execution. Approved original application, branch
commit/push and coordinator/manual-owner composition remain separate work.

In this lane native executable trust still refuses admission (T-301.1, deferred).
The real-Git setup/activation test and synthetic mount-plan controls do not prove
a running sandbox, successful inference or an improved coding score. Preserve
that refusal; do not change executable ownership or root-trust checks.
