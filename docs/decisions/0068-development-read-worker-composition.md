# Decision 0068: Development Read Worker Composition

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054, Decision 0063 and the owner's real-model integration assignment |
| Scope | Native read-worker prerequisite for the disposable Linux coding harness |
| Preserves | Production root-owned worker trust, exact read grants, held-object projection, syscall/network confinement and resource limits |

## Evidence

The first successfully decoded real GPT-OSS read proposal reached its exact grant
and then failed. The development factory still supplied `/usr/bin/true` as its
sandbox worker. Scripted repair had used only validation, patch and Git tools,
so it had not exercised this missing composition. The existing
`agentmage-read-only-worker` implements the required sealed-input protocol.

## Decision

1. Build the existing worker with the CLI and host. The development factory uses
   the exact `agentmage-read-only-worker` sibling of the running `agentmage-host`.
   Missing or unsafe files refuse; there is no stub or production fallback.
2. A separate platform constructor requires the development-adapter type. It
   checks the canonical sibling path, owner, executable mode, bounded size and
   safe ancestry, verifies a stable read and seals an immutable executable
   snapshot. A later owner edit cannot mutate the launched worker. It accepts no
   arbitrary executable/path argument.
   Bubblewrap receives this sealed executable through a read-only data mount
   with exact executable permissions. Its
   [0.12.0 source](https://github.com/containers/bubblewrap/blob/v0.12.0/bubblewrap.c)
   resolves bind-fd source paths using `realpath`; the memfd's deleted pseudo-path
   is not a usable filesystem source. The failed bind-fd attempt is retained.
3. The production manifest constructor still requires root-owned worker files
   and ancestry. Root-owned launch tools and the exact Fedora x86-64 runtime
   library files retain existing verification. No system installation, service
   change or permission-model relaxation is needed or authorized.
4. The existing sandbox consumes the same exact read grant and sealed workspace
   inputs, with its existing cgroup, namespace, seccomp and output boundaries.
   The scripted repair regression now includes a real native read after its
   failing validation, before the patch. This verifies composition only, not
   real-model qualification.

This development-only composition makes no supported-platform or release claim.
