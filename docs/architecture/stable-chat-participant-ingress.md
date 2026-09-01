# Stable Chat Participant Reference Ingress

## Status

Story 23.5 has a stable public-API implementation for request-bound prompt and reference
collection. The `@agentmage` participant accounts for every supplied `ChatRequest.reference`,
revalidates the exact selected AgentMage profile immediately before capture, resolves only values
present on that request, and sends exact bytes to the Rust-owned Engineering Runtime over the
existing authenticated IPC channel.

This is local implementation evidence. Installed VSIX, assistive-technology, remote-placement,
supported-platform, qualified-production-model, and independent-review campaigns remain open.

## Authority Boundary

```mermaid
flowchart LR
    REQ[Stable ChatRequest] --> NORM[Descriptor normalization]
    NORM --> RESOLVE[Request-only VS Code resolver]
    RESOLVE --> CHUNKS[Digest-bound bounded chunks]
    CHUNKS --> IPC[Authenticated native IPC]
    IPC --> HOST[Rust Engineering Runtime]
    HOST --> ART[Immutable source artifacts]
    ART --> TURN[Verified turn]
    HOST --> STATES[Source accounting]
    STATES --> UI[Progress and terminal rendering]
```

The TypeScript shell may read a `string`, `Uri`, or `Location` only because that exact value appears
in the current stable participant request. It has no ambient enumeration, path selection, parser,
context manager, model runtime, policy, grant, tool, store, or effect authority. URI text and
private paths are not placed in the source manifest or progress output.

## Complete Accounting

Each descriptor receives a stable request-local identity and a content-free descriptor digest.
The state family is closed over `queued`, `reading`, `extracting`, `partial`, `unsupported`,
`omitted`, `stale`, `cancelled`, `failed`, and `included`. Current text is captured directly. URI
and location values are read through `workspace.fs` only after a bounded stat, and their size,
type, and modification revision are rechecked after the read. Revision drift becomes `stale`; an
unknown value becomes `unsupported`; no state falls back to a workspace search.

Prompt and included-reference bytes use Verified Chat's existing `begin_artifact`, ordered
`upload_artifact_chunk`, and `commit_artifact` operations. Each 12 KiB chunk has sequence, offset,
length, finality, and SHA-256 bindings. The Rust host revalidates the declared total and digest
before immutable capture. Cancellation is checked before each chunk and before commit, and failure
requests exact upload cancellation.

The final content-free participant manifest binds request identity, command, prompt artifact,
every supplied descriptor, terminal source state, exact included artifact identity, byte count, and
digest. Only included artifact identities enter the verified turn's context set.

## Provider Compatibility

The language-model provider remains an optional compatibility and picker surface. It now counts
every delivered message part. If any part is not a stable text part, the provider reports the exact
unsupported count, starts no runtime work, and directs the user to `@agentmage`. A provider label,
proxy, or MCP adapter therefore cannot claim omitted bytes or a resource it did not receive.

## Local Verification and External Qualification

The deterministic extension-host suite covers 999- and 1,001-character prompts, multiple text,
file, virtual, unknown, and stale references, digest-bound chunk capture, duplicate identities,
cancellation before model work, and provider non-text accounting. Existing Engineering RPC tests
cover malformed, reordered, duplicate, oversized, interrupted, digest-mismatched, and replayed
artifact frames at the Rust boundary.

The following remain non-passes until run independently in their declared environments:

- packaged VSIX interaction and keyboard/screen-reader behavior;
- Fedora, Ubuntu, Windows, macOS, WSL, Remote SSH, and Dev Container placement and cleanup;
- execution with an independently qualified production profile; and
- independent security, accessibility, and release review.
