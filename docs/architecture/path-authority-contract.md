# Path Authority Contract

## Status and Scope

This reference describes the implemented shared path contract and the locally
executed Fedora Linux adapter for Story 6.1. It is not a product-release claim.
macOS implementation and execution are `blocked-macos`; Ubuntu execution is
`not-executed`. Neither platform result is inferred from Fedora evidence.

AgentMage currently has no public constructor that turns an ambient absolute
path into an authorized workspace handle. Workspace selection and durable
authorization remain outside this increment.

## Authority Types

`WorkspacePath` is the only path value accepted by `PlatformPathAdapter`. It
contains one `WorkspaceId` and normalized relative components. Its private
fields and custom deserializer force all values through the same validation.

`WorkspaceScopePath` is the root-capable counterpart used for approved session
scope and for the path-free identity of a held workspace root. It permits zero
components for the workspace root and otherwise uses the exact `WorkspacePath`
component validator. `GrantTarget` closes the authority representation into an
authorization-bound session scope, an exact held child object, or an exact
descriptor-held workspace root; raw string components are not retained as a
weaker grant path.

`AuthorizedWorkspaceHandle` binds a workspace authorization to one
`AdapterInstanceId`, `WorkspaceAuthorizationId`, `WorkspaceId`, and
`PathPlatform`. Its native root capability remains private to the adapter.

`HeldWorkspaceRoot` extends that handle with content-free identity evidence for
the continuously held root descriptor. It exists so an owned-worktree command
can bind the repository root without making an empty `WorkspacePath` valid.
Held-root operation authority overlaps every excluded subtree in the same
workspace and is therefore denied when such an exclusion is inherited.

`HeldWorkspaceObject` retains the native object from validation through use and
reports only canonical path, authorization, adapter, intent, object kind,
object identity, and optional exact file preimage. `PlatformPathAdapter` uses
associated handle and object types, which prevents a handle from one adapter
being passed to another adapter implementation by accident.

## Resolution Interface

The shared `PathResolutionIntent` values are `Metadata`, `ReadFile`,
`ReadDirectory`, and `ContentHash`. Read and hash intents require a regular
file. Directory enumeration requires a directory. The held identity contains
domain-separated mount and object SHA-256 values; native identifiers are not
exposed. `FilePreimage` contains the exact bounded byte count and SHA-256.

`PathAdapterErrorKind` is a closed, content-free failure classification. A
denial may expose the unsafe relative component index, but not the rejected
component, ambient absolute path, or operating-system error text.

## Linux Enforcement

The automatic `LinuxPathAdapter` probes and uses Linux `openat2` with all four
mandatory resolution constraints:

- `RESOLVE_BENEATH`
- `RESOLVE_NO_SYMLINKS`
- `RESOLVE_NO_MAGICLINKS`
- `RESOLVE_NO_XDEV`

Every component is opened relative to a continuously held directory descriptor
with no-follow and close-on-exec flags. If strict `openat2` is unavailable,
filtered, or incompatible, the public automatic mode returns
`UnsupportedPrimitive`; it has no public weaker fallback.

A private test-only descriptor-walk branch is admitted only when `statx`
provides the same mount identity for the root and every component. Missing or
changing mount evidence fails closed. Regular files with more than one hard
link, symbolic links, unsupported object kinds, identity changes, and files
beyond the preimage bound are rejected.

The root and final object descriptors remain held. Exact file hashing uses the
held descriptor and pre/post metadata comparisons. Revalidation compares the
held identity and, when present, recomputes the exact preimage from that same
descriptor.

For a mediated Linux operation, the consumed grant target must exactly match
the held path, authorization event, adapter instance, platform, object kind,
object identity, and preimage. The supervisor derives either a sealed immutable
projection of the exact approved file bytes or a sealed bounded directory-name
projection. The worker receives only that projection, never the original file,
source directory, or workspace-root descriptor.

## Display-Only Links

`DisplayFileLink` is a distinct one-way display type. It is created only from a
successfully resolved and revalidated held object, stores a bounded absolute
`file:///` URI plus optional positive line number, and retains the object
identity that authorized its display. It implements no deserializer and no
conversion to `WorkspacePath`, `GrantTarget`, or a platform adapter input.

The explicit rendering accessor discloses the validated absolute URI for the
user interface. Debug and error output remain redacted. Any URI or rendered
`#L` target fed to `WorkspacePath` is rejected before filesystem observation.

## Platform Evidence

| Platform | Implementation status | Execution status | Claim |
| --- | --- | --- | --- |
| Fedora Linux | Shared contract and Linux adapter implemented | Verified locally | Strict `openat2` selected and focused tests pass |
| Ubuntu Linux | Same Linux source is intended | Not executed | No Ubuntu result claimed |
| Apple Silicon macOS | Story 6.1.1.6 remains open | Blocked | No macOS result claimed or substituted |

## Review Checklist

- Confirm authority APIs accept `WorkspacePath`, never an absolute string or
  `DisplayFileLink`.
- Confirm the public Linux strategy cannot select the descriptor-walk branch.
- Confirm all four strict `openat2` resolution flags remain present.
- Confirm held-object identity and exact preimage checks span validation and
  use.
- Confirm session scopes and operation targets share the canonical component
  parser and that root operation authority requires a distinct held-root
  descriptor rather than an empty `WorkspacePath`.
- Confirm the worker boundary receives no workspace-root descriptor.
- Confirm denial errors remain content-free and display links remain one-way.
- Confirm macOS and Ubuntu claims remain blocked until their own execution
  evidence exists.

## Limitations

This increment does not implement public workspace selection, durable workspace
authorization, Visual Studio Code link activation, Ubuntu execution evidence,
packaging, or macOS path behavior. The generated path corpus, race harness,
display-authority matrix, and independent secure-path review remain separate
Story 6.1 artifacts.
