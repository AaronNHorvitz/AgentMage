# Decision 0017: Linux Platform Trust and Lifecycle

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | Linux aggregate composition, independent release trust, native configuration storage, IPC ownership, and process identity |
| Resolves | `RM-013`, `RM-014`, `RM-015`, `RM-016`, `RM-017` |
| Preserves | Decisions 0001 through 0016, strict-local operation, exact-object authority, and fail-closed startup |

## Context

The repository has independently tested Linux path, sandbox, Secret Service,
IPC, inventory, and strict-local mechanisms, but no production aggregate owns
them. Platform startup also accepts the expected manifest from the same adapter
that reports the observed runtime and mechanisms. That permits an adapter to
self-authorize an arbitrary mechanism digest. Configuration parsing and policy
belong in the kernel, but the current manager also reads paths, changes native
permissions, creates backups, renames files, and synchronizes directories.

The Linux IPC listener does not own identity-safe path cleanup, strict-local
root admission does not require current-user private ownership, and process
inventory combines multiple `/proc` observations without binding them to one
stable process start identity. Phase 8 must close these boundaries before a
real Visual Studio Code workflow can be composed in Phase 9.

## Decision

### Independent Release Trust

1. `PlatformAdapter` reports only observed runtime and capability evidence. It
   does not select, construct, or return its expected release manifest.
2. The kernel accepts a platform only with a `VerifiedPlatformRelease` produced
   by strict parsing and detached Ed25519 verification of exact manifest bytes.
   The trusted verification key enters through composition independently from
   the native adapter.
3. A verified release binds the adapter API, manifest digest, target runtime,
   and exactly one nonzero expected mechanism digest for every required
   capability. Missing, duplicate, unknown, reordered, or zero identities fail
   before adapter probing.
4. Startup compares observed mechanism digests with the independently verified
   expected values. Echoing the expected manifest digest is not sufficient.
5. Unsupported schema, release status, signer, platform, architecture,
   signature, or runtime identity fails closed with bounded diagnostics. An
   adapter cannot fall back to an unsigned fixture or self-reported target.

### Aggregate Linux Adapter

1. One `LinuxPlatformAdapter` owns the platform family, adapter instance,
   runtime observer, capability probes, path adapter, strict-local state root,
   configuration store, Secret Service boundary, worker confinement, IPC, and
   process inventory composition.
2. The adapter is usable for workspace authorization or state opening only
   through `VerifiedPlatformAdapter<LinuxPlatformAdapter>`. Construction alone
   conveys no authority.
3. Workspace authorization opens the user-selected root without following
   symbolic links, retains its descriptor and identity, and is the sole
   production constructor for `LinuxAuthorizedWorkspace`.
4. Missing native mechanisms report `Unavailable`; changed or unsafe mechanisms
   report `Invalid`. Neither state silently reduces the required capability set.
5. Fedora and Ubuntu share the same kernel contract while retaining distinct
   signed release identities and native evidence. A result from one family
   cannot activate the other.

### Private State and Key Lifecycle

1. The operational root is an absolute, symlink-free, locally attached,
   non-synchronized directory owned by the current user with no group or other
   permission bits. Its descriptor, device, inode, mount, owner, mode, and
   filesystem identity remain held and are revalidated around every composition
   boundary.
2. SQLCipher opens the fixed authority database through the exact Linux
   `/proc/self/fd/<held-root>/authority.db` shape. The final file is created or
   verified as a private non-symlink before open; ordinary non-descriptor paths
   retain SQLite no-follow. The platform revalidates the root before returning
   an active durable runtime.
3. The operational key keeps one fixed purpose and profile identity. Initial
   provisioning is explicit, generates 256 random bits through the operating
   system, writes only encoded key material to Secret Service standard input,
   verifies lookup, and never places it in arguments, environment, logs, or a
   repository file.
4. Provisioning requires the verified aggregate, retains an admitted state
   root, serializes through a private fixed lifecycle lock, and refuses an
   existing database/key mismatch or key overwrite. Destructive key
   removal and state removal require a distinct explicit lifecycle operation;
   normal startup never repairs either by deletion.
5. Key rotation is not represented as complete until old/new key recovery can
   survive every interruption. If that protocol is not implemented in this
   phase, rotation remains unavailable rather than using a lossy rekey sequence.

### Configuration Boundary

1. The kernel retains closed-schema parsing, migration calculation, canonical
   serialization, authority comparison, diffing, and result binding. Production
   kernel code contains no native path, ownership, permission, rename, or sync
   operation.
2. Linux owns the held configuration directory and fixed target identity.
   Reads use descriptor-relative no-follow opens and require a current-user,
   private, single-link regular file.
3. Apply, migration, and rollback compare the continuously held preimage, create
   a private same-directory candidate, synchronize it, publish atomically, and
   synchronize the parent. A changed preimage, backup, directory, owner, mode,
   link count, mount, or target identity refuses publication.
4. Content-addressed backups are immutable and byte-compared on retry. A
   conflicting retained object is an integrity failure, not an overwrite.
5. Native configuration mutation is available only through a Linux effect
   driver that consumes a kernel-issued authorization for the exact
   administration operation.

### IPC and Process Lifecycle

1. Every filesystem-backed listener owns its socket path identity. Orderly drop
   removes only the unchanged socket created by that listener. A replacement,
   foreign owner, wrong mode, non-socket, or unavailable identity is retained
   and reported rather than unlinked.
2. Startup never unlinks an arbitrary pre-existing path. The Phase 8 candidate
   does not automate stale recovery: an unknown existing object is left
   untouched and startup fails closed. A later fresh-endpoint or stale-recovery
   protocol must be separately identity-bound and tested.
3. Peer identity binds UID, PID, executable digest, and process start time. The
   authentication frame and kernel peer credentials must describe the same
   stable process.
4. Inventory opens a pidfd where supported, reads start time before collection,
   captures status, executable, descriptors, and sockets, then reads start time
   again. Disappearance, PID reuse, partial state, or changed start identity is
   rejected as uncertain and never reported as authoritative.
5. Listener, worker, and inventory errors remain bounded and content-free. Raw
   paths, process arguments, environment, socket names, and file contents are
   not retained in diagnostics.

## Verification

Phase 8 verification must include:

- signature, signer, manifest, schema, status, platform, and runtime mutation;
- adapter self-report and mechanism-digest substitution attacks;
- every required capability as verified, unavailable, invalid, foreign, and
  duplicated evidence;
- production-constructor denial before verified activation;
- state-root symlink, filesystem, synchronization, owner, and mode mutations;
- missing, malformed, wrong, and explicitly provisioned operational keys;
- configuration symlink, hard-link, owner, mode, rename, preimage, backup,
  interruption, concurrent-writer, and directory-identity attacks;
- orderly IPC cleanup, socket replacement, unknown stale socket, replay, wrong
  process start time, and malformed frame cases;
- PID reuse, process disappearance, malformed `/proc`, and unsupported pidfd
  behavior; and
- Fedora-native execution for mechanisms available on the development host,
  with Ubuntu execution remaining unclaimed unless run on Ubuntu.

## Deliberate Limits

- The current repository contains contract fixtures, not signed release
  manifests or supported packages. Production discovery must therefore fail
  closed until supplied an independently signed release artifact that matches
  the installed runtime.
- A capability whose implementation or release artifact does not yet exist is
  `Unavailable`; tests may exercise the exact production verification path with
  synthetic public fixtures but cannot turn fixture evidence into a release
  claim.
- Key rotation remains unavailable unless the interruption-safe dual-key
  protocol and recovery tests are completed in this phase.
- Unknown stale-socket recovery and fresh-endpoint selection are not
  implemented; the candidate retains the object and refuses startup.
- Phase 8 does not add the Visual Studio Code read workflow, model execution,
  package publication, Ubuntu-native evidence, macOS, or Windows support.

## Approval Record

The user approved Decision 0017, the Phase 8 local commit, and entry into Phase
9 on 2026-08-11. That approval did not authorize a push or a Phase 9 commit.
