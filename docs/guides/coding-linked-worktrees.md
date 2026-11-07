# Linked worktrees in the development coding host

The existing native coding host has explicit linked-worktree activation and
read-only inventory/inspection plumbing under its private disposable root.
Positive startup through the complete native worker remains unverified in this
lane (T-301.1); the setup below is a development procedure, not evidence of a
working coding run. It does not
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

Where native trust and the worker are available, the intended next step is the
existing [native API and terminal workflow](coding-runtime-api.md) with explicit
scenario/model selection. The complete linked startup has not been observed
passing in this lane. The linked extension does not enable
another model or an arbitrary endpoint. Real-model work still uses the GPU slot.
Each consequential model tool still needs its separate exact approval.

## Read boundary and recovery

Linked startup inventory and later Git inspection route to the same confined
offline worker. Its mount plan exposes the workspace at `/work` and the common
metadata root at `/metadata/common`, both read-only. The per-worktree metadata
remains inside that common root at `worktrees/<verified-name>`; its held host
descriptor is retained for revalidation. The platform selects
`--git-dir=/metadata/common/worktrees/<verified-name>` and `--work-tree=/work`.
This preserves the metadata's `../..` pointer for shared ref resolution without
relying on `GIT_COMMON_DIR`. The host's absolute pointers are not followed inside
the guest. The verified name comes from activation, never model output.
A separate linked Git launch owner carries the binding and admits exactly two
projections; the ordinary Git owner continues to require exactly one.
Duplicated projection descriptors carry close-on-exec.
Host paths referenced by untrusted config
includes or object alternates are absent from that mount namespace. System
runtime mounts remain those of the existing worker. No original source checkout,
home directory or network is added. Each linked inventory query uses the smaller
of its original cap and the supervisor's existing 4 MiB output limit. Overflow
refuses inventory rather than returning a truncated repository map. Ordinary
inventory caps remain unchanged. Each linked worker invocation uses the existing
15-second inspection deadline,
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
The real-Git setup/activation, precancelled supervisor admission and guest-layout
component checks do not prove a complete running native worker, successful
inference or an improved coding score. The explicit native linked inventory and
inspection test requires a trusted host with a working user service manager. Preserve
that refusal; do not change executable ownership or root-trust checks.
