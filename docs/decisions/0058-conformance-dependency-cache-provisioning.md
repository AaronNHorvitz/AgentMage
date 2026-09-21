# Decision 0058: Path-Conformance Dependency-Cache Provisioning

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | How `scripts/path_platform_conformance.py` provisions the Cargo dependency cache inside its container |
| Authority | Decision 0054 standing owner delegation |
| Preserves | The pinned image, `--network=none`, `--read-only`, `--cap-drop=all`, `no-new-privileges`, `--pids-limit=256`, `--memory=2g`, the 2 GB tmpfs, the `10001:10001` runtime user, and the exact test command |
| Raises no limit | Correct. No memory, tmpfs, pids or timeout value was changed |

## Problem

The conformance container copied the **entire** host Cargo registry into its 2 GB tmpfs:

```sh
cp -a /registry /tmp/cargo/registry
```

That registry is 1.7 GB on this host and grows without bound as unrelated work caches more
crates. With the registry copied, the build had roughly 300 MB left and failed:

```text
error: failed to write to `/tmp/target/debug/deps/rmetaSBKPg8/full.rmeta`:
No space left on device (os error 28)
```

The check therefore degraded as a function of unrelated host cache growth rather than anything in
this repository. The tmpfs also counts against the container's `--memory=2g`, so the previous
shape could consume the whole memory budget before compiling anything.

## Decision

The container is given only what an offline, locked build actually needs — the registry index and
the compressed crate cache — and lets Cargo extract the subset this package requires:

```sh
cp -a /registry/index /tmp/cargo/registry/index
cp -a /registry/cache /tmp/cargo/registry/cache
```

Measured on this host: index 58 MB plus cache 194 MB, against 1.4 GB of pre-extracted `src` that
the build does not need in full. Nothing else about the run changes.

## Verification

Run under the unchanged controls before committing:

```text
test result: ok. 142 passed; 0 failed; 45 ignored
tmpfs  2.0G  845M  1.2G  42% /tmp
```

The build completes with 1.2 GB of headroom instead of exhausting the filesystem, and the result
no longer depends on how many unrelated crates the host has cached.

## Why this weakens nothing

The image, network isolation, capability set, privilege flags, pids limit, memory limit, tmpfs
size, runtime user and test command are all byte-identical. The same package is compiled from the
same locked, offline dependency graph and the same tests execute. Only the provisioning of the
dependency cache changed, from "copy everything" to "copy what the lockfile resolves". No limit
was raised and no assertion was relaxed.

## Extended to the platform manifest artifact — 2026-09-20

`scripts/platform_manifest_artifact.py` carried the identical defect: the same
`cp -a /registry /tmp/cargo/registry` into the same 2 GB tmpfs under the same `--memory=2g`,
and it failed the same way once the conformance check was fixed and `docs:check` reached it.
The same provisioning is applied there, with the same controls untouched. A repository-wide
search confirms these were the only two occurrences of the pattern.
