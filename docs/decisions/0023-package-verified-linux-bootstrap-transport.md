# Decision 0023: Package-Verified Linux Bootstrap Transport

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-12 |
| Scope | Fixed installed-host launch, package-first verification, inherited-pipe credential transfer, and exact Linux peer authentication |
| Advances | Linux portions of `RV-01`, `RV-05`, and Sprint 25 trusted bootstrap preparation |
| Blocks | Approved production signer and trust-root provisioning, signed platform activation, usable product workflow, clean installation, support, and release |
| Preserves | Decisions 0001 through 0022, no ambient shell, no secret in arguments/environment/files/logs, and no authority from authentication alone |

> **Amendment:** Decision 0028 adds the inactive native-inference adapter to the
> exact verified package payload. Bootstrap remains authentication-only and
> conveys no model or product authority.

## Context

Decision 0022 supplied deterministic package signing and verification mechanics,
but ordinary Visual Studio Code activation still had no trusted way to launch
the installed host, receive one-use authentication material, or bind the local
connection to the exact extension-host process. Passing an endpoint or secret
through process arguments, inherited environment, a predictable file, or a
user-editable setting would weaken that boundary.

Package verification and product authority are separate concerns. It must be
possible to prove that the installed host and VSIX belong to one externally
signed package before opening local transport, while still denying every
workspace, model, tool, grant, and effect request until the independently
signed platform manifest and all required native mechanisms activate.

## Decision

1. The production Linux host exposes one fixed `--bootstrap-linux` command.
   It accepts no package, signature, key, endpoint, secret, workspace, or
   executable path from arguments or environment.
2. The host verifies `/` against the exact version 2 package manifest, detached
   signature at `/etc/agentmage/release/package-manifest.ed25519`, and
   independently provisioned public key at
   `/etc/agentmage/trust/package-signing-ed25519.pub` before it observes a peer,
   creates a runtime directory, binds a socket, or generates launch material.
3. The expected client is the host's exact parent process. Linux procfs provides
   its PID, start time, and executable bytes; the one-use authenticator also
   binds the current UID and protocol version. Kernel socket credentials are
   observed independently when the client connects.
4. The endpoint is a new `0600` Unix socket inside
   `/run/user/<uid>/agentmage`, whose parent and child are current-user-owned
   owner-only directories. An existing path is never unlinked or reused.
5. A fresh challenge and 256-bit launch secret are emitted once as one
   versioned, length-bounded binary frame over the directly inherited
   standard-output pipe. The serializer and parser create no secret-bearing
   string, argument, environment value, file, or log record. Both host and
   extension erase retained secret byte arrays after use.
6. Authentication-only bootstrap conveys no workspace handle, capability
   grant, tool permit, model authority, or effect authority. Until complete
   signed platform activation constructs the real workflow, an authenticated
   session remains unavailable and cannot satisfy a product or release gate.
7. Package trust failure, unsafe runtime state, process-identity failure,
   malformed transfer, replay, wrong process, wrong executable, or connection
   failure is terminal. There is no unauthenticated or user-configurable
   fallback.

## Verification

- Rust tests create an ephemeral synthetic signing identity outside the source
  tree, verify an exact package fixture, transfer one bounded binary envelope,
  connect through a private Unix socket, and authenticate the kernel-observed
  exact process.
- Focused refusal tests mutate the signed package, supply a public runtime
  parent, and name a nonexistent peer. Package mutation fails before runtime
  directory or socket creation.
- Existing IPC mutation, replay, socket-mode, peer-credential, process-start,
  executable-digest, malformed-frame, and cleanup tests remain mandatory.

## Consequences

- The platform-neutral product authority remains unavailable after transport
  authentication; a later bounded increment must activate the signed platform
  release and compose the durable workflow.
- The Visual Studio Code extension supervises one fixed installed host without
  accepting a configurable executable or secret-bearing launch input. It
  validates the binary frame, exact UID and PID, timeout, output closure, and
  child lifecycle before constructing the authenticated bridge.
- No production identity, clean installation, package-manager signature,
  supported workflow, or release is claimed by this source-level evidence.
