# macOS Platform Adapter

This Swift module owns the retained Apple Silicon App Sandbox, XPC, Keychain,
security-scoped bookmark, code-identity, and native IPC boundaries.

The kernel-host source admission boundary now validates, in fixed order, arm64,
macOS 15 or newer, the running code signature, Hardened Runtime metadata, exact
bundle and Team identity, App Sandbox, one App Group, app-scoped bookmarks,
read-only user-selected files, one Keychain access group, and a closed entitlement
set. It returns only a content-free refusal or an opaque verified identity. The
contract entitlement file uses the repository's visibly synthetic `com.example`
and `AAAAAAAAAA` values; it is not a signing or release input.

The bridge source contract adds protocol version 1, a fixed 68-byte HMAC-SHA256
launch handshake, one-use replay rejection, closed 64 KiB request and 4 MiB
response bounds, and fixed App Group socket placement with a `0700` parent and
`0600` Unix socket. It binds the synthetic release-declared bridge, host, Team,
and App Group identities.

The peer-verification source reads `LOCAL_PEERTOKEN`, `LOCAL_PEERPID`, and
effective peer credentials from the accepted Unix socket. It resolves live code
with `kSecGuestAttributeAudit`, checks the exact release-designated requirement,
signature, bundle, Team, App Sandbox, App Group, entitlement closure, launch PID,
UID, and GID, and only then permits the one-use fresh challenge to be consumed.
The pure mutation suite is synthetic: no native socket observation, signed peer,
or security decision has executed on this Linux host.

The workspace source configures one directory-only, non-alias `NSOpenPanel`,
creates only app-scoped read-only security-scoped bookmarks, stores their bounded
bytes in the host Keychain access group, resolves without UI or volume mounting,
refreshes stale data only after exact revalidation, and balances every successful
scope start with an explicit or deinitializing stop. Root aliases, symbolic links,
resource/volume races, noncanonical names, case collisions, oversized roots, and
malformed or missing records fail with content-free classes. This is source and
synthetic mutation coverage only; no native picker, Keychain, bookmark, move,
reboot, alias, mount, or revocation lifecycle has executed here.

The tool-helper source defines a private `NSXPCListener` service that verifies
its own exact signature and two-key sandbox entitlement closure, admits only the
exact signed host identity, and decodes one closed invocation carrying an
already-consumed `WorkspaceRead` grant, one bounded read-only bookmark, and one
digest-bound request. The helper resolves no Keychain state, opens only a
read-only no-follow workspace descriptor, creates and removes an owner-only
scratch directory, applies CPU, address-space, file, process, and descriptor
ceilings, returns a digest-bound bounded result, and rejects replay. Its XPC
target deliberately does not use sandbox inheritance, an App Group, network,
user-selected read-write, Keychain, device, or temporary-exception entitlement.
This is source and synthetic mutation coverage only; no signed XPC target,
native connection, sandbox enforcement, forced lifecycle, or Rust executor
composition has executed here.

The Metal-inference source defines a separate private `NSXPCListener` service
that validates its exact signed identity, Hardened Runtime metadata, and
one-key App Sandbox entitlement closure before admitting the exact signed host.
Its closed request carries one profile identity, manifest/artifact/runtime and
codec hashes, bounded inert input bytes, deterministic sampling values, and a
wall deadline. It deliberately has no workspace path or bookmark, tool, grant,
credential, environment, listener, App Group, installer, or network field. The
host passes a model as an already-open file handle; the service duplicates it,
requires an owner-matching read-only regular single-link descriptor, verifies
its exact byte count and SHA-256, and latches one profile for the process. Work
is serialized, request identities cannot replay, cancellation and timeout are
bounded, output is digest bound, and CPU, address-space, file, process, and
descriptor ceilings are applied. The executor receives only the read-only model
descriptor, exact profile, inert request bytes, decoding limits, and a
cancellation probe. The service has only `com.apple.security.app-sandbox`; it
has no network, file-selection, bookmark, App Group, Keychain, device,
temporary-exception, JIT, or library-validation exception entitlement. This is
source and synthetic mutation coverage only: no signed inference target, native
XPC file-handle transfer, Metal device, `llama.cpp` load, GGUF inference,
sandbox attack, forced lifecycle, or shared Rust runtime composition has run.

Native compilation, signing, execution, and support remain `BLOCKED-MACOS` until
the manual Apple Silicon source check and the required physical-M5 evidence exist.
