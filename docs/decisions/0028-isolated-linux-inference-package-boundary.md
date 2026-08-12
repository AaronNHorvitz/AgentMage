# Decision 0028: Isolated Linux Inference Package Boundary

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-12 |
| Scope | Separate Fedora/Ubuntu native-inference process, one-way compile dependencies, exact package enrollment, inactive process behavior, and pre-runtime evidence |
| Implements | Sprint 9 Sub-task `9.1.1.6` package/process portion |
| Amends | Decisions 0020, 0022, and 0023 package payload closure |
| Preserves | Decision 0027, zero enabled models, no automatic fallback, external release trust, no production signer, no workspace/tool/grant/credential authority, and no release claim |
| Does not implement | `LocalModelRuntime`, llama.cpp packaging, model profiles, codecs, inference, streaming, cancellation, runtime resources, Docker compatibility, or clean Ubuntu execution |

## Context

The Linux package previously contained the host, Visual Studio Code extension,
license, and exact payload manifest. Sprint 9 also requires the declared local
inference adapter to be packaged behind the shared boundary and isolated from
tool and workspace authority. Decision 0027 subsequently assigned the complete
candidate-neutral model runtime and first Muse vertical slice to Sprint 13 and
retained zero enabled models.

Putting model behavior into the host would collapse process authority. Treating
`llama-server` itself as the platform capability would expose a generally
listening model service before AgentMage has implemented its authenticated
runtime protocol. Packaging a model or runtime candidate now would also imply
admission that the current evidence does not support.

## Decision

1. `platforms/linux-inference` is a separate Rust package and executable named
   `agentmage-native-inference`. Its sole compile dependency is
   `agentmage-kernel-contracts`; it has no kernel-engine, platform-worker,
   capability-pack, host, connector, or credential dependency.
2. The shared boundary at this stage is the existing nonzero
   `LocalEndpointIdentity` for
   `KernelNativeInferenceAdapter` over `AuthenticatedUnixSocket`. No new model
   request, profile, codec, or output contract is introduced before Sprint 13.
3. The executable accepts exactly `--self-check`. It emits a fixed content-free
   descriptor stating that authority inputs are empty, enabled models are zero,
   inference is unavailable, and no network listener exists. Every other
   operation fails with one stable code.
4. The Fedora and Ubuntu candidate and signable payloads add the executable at
   `/usr/libexec/agentmage/agentmage-native-inference`. The package manifest
   binds its path, mode, size, and SHA-256, and the host requires that fourth
   file during both candidate and signed-root verification.
5. Linux `LocalInference` capability discovery observes the exact packaged
   adapter executable. It no longer treats an ambient or generally listening
   `llama-server` executable as the AgentMage adapter identity.
6. No llama.cpp library, server, model artifact, tokenizer, template, profile,
   credential, workspace path, tool, or grant is packaged or passed to the
   process in this increment.
7. The adapter remains inactive until Sprint 13 implements and verifies the
   candidate-neutral runtime protocol and an exact profile passes its own gates.
   Presence in a package cannot enable a model or satisfy a support gate.

## Verification

- Unit and process tests prove the exact self-check closure, fixed refusal,
  nonzero authenticated-endpoint construction, content-free descriptor, and
  zero model state.
- Architecture tests enforce the one-way dependency graph and package assembly
  edge.
- Candidate tests require the adapter in the complete payload and reject
  content, mode, manifest, symlink, class, and signature mutation.
- Real local RPM and DEB builds are extracted and verified by the packaged host;
  detached-signature lifecycle tests also pass with an external ephemeral test
  identity that is not retained.
- The SBOM and provenance inventory include the new internal package and no new
  third-party dependency.

## Consequences

- Decisions 0020 and 0022 retain their signing and lifecycle semantics, but
  references to the former three-file host/VSIX/license closure are superseded
  by the four-file host/adapter/VSIX/license closure.
- Decision 0023 still grants no product authority after package authentication;
  the verified package now also binds the inactive adapter identity.
- Sub-task `9.1.1.6` can close only for the package/process isolation boundary.
  Story 9.2 and Sprint 13 remain open for native llama.cpp, resource and socket
  topology, the complete runtime contract, model evidence, and inference.
- Clean Fedora and Ubuntu install/launch/uninstall/recovery evidence remains
  owned by Sub-task `9.1.1.7` and cannot be inferred from local format tests.
