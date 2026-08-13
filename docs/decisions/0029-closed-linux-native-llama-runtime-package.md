# Decision 0029: Closed Linux Native llama.cpp Runtime Package

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-12 |
| Scope | Fedora/Ubuntu x86_64 native llama.cpp package identity, retained files, model-store contract, resource ceiling, and guarded IPC topology |
| Implements | Sprint 9 Sub-task `9.2.1.1` |
| Amends | Decision 0028 by defining the separate native runtime package consumed by its isolated adapter process |
| Preserves | Candidate-neutral runtime design, zero enabled models, no automatic fallback, acquisition separation, standard-user execution, exact package identity, and no workspace/tool/grant/credential authority |
| Does not implement | Model activation, `LocalModelRuntime`, a family codec, inference, streaming, cancellation, GPU/Vulkan admission, Docker compatibility, or release approval |

## Context

Decision 0028 created an inactive `agentmage-native-inference` process but intentionally packaged no upstream runtime. Sub-task `9.2.1.1` now requires a pinned native llama.cpp reference package without turning an upstream general-purpose executable or listener into an AgentMage authority boundary.

The evaluated upstream b10333 archive contains command-line tools, a web server, an RPC server and library, benchmark and conversion utilities, a Vulkan backend, and other surfaces that the first native package does not need. Shipping the archive as-is would enlarge the product process and network inventory before the guarded AgentMage runtime protocol exists.

## Decision

1. The exact Linux x86_64 source input is llama.cpp release `b10333`, source commit `08659901c43b51de735740f1cf61bb82fbe0c4e4`, archive `llama-b10333-bin-ubuntu-vulkan-x64.tar.gz`, size `32,521,550` bytes, and SHA-256 `f14e312fbee33ce60d2eed7036de5debe31c1d7f4d8f0e37920eb0a2de0854a5`.
2. [`llama-cpp-b10333-linux-x86_64.json`](../../model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json) is the source package profile. It binds every retained file, link, mode, size, hash, source revision, authority prohibition, model-store rule, resource ceiling, and IPC class.
3. The package retains only the upstream MIT notice, `libllama`, `libggml`, `libggml-base`, their exact relative SONAME links, and the b10333 CPU backend libraries. Regular payload files are read-only. No model is included.
4. The package excludes every upstream executable and entry point, including `llama-server`, `llama-cli`, `ggml-rpc-server`, benchmark, conversion, quantization, template, tokenizer, text-to-speech, and multimodal utilities. It also excludes `libggml-rpc`, `libllama-server-impl`, `libllama-common`, other utility implementation libraries, `libmtmd`, and the Vulkan backend.
5. The source archive must match its exact byte identity before parsing. Archive paths, duplicates, expanded size, member types, selected file identities, and selected link targets fail closed. Output is a deterministic tar-gzip package published with no-overwrite semantics and verified against an exact generated manifest.
6. The package extracts only beneath the private relative root `runtimes/llama-cpp-b10333-cpu-linux-x86_64`. A later installer owns atomic activation. It may not overwrite or reinterpret an existing or drifted runtime identity.
7. The adapter receives no model-store path or directory-enumeration authority. The kernel must hold and authenticate a standard-user-owned mode-`0700` store plus exact read-only model descriptors. Runtime writes are unrepresentable.
8. One launch uses one authenticated native-inference Unix-socket endpoint, one inference slot, zero swap, and explicit cgroup ceilings for memory, tasks, CPU, lifetime, and retained output. A model profile may narrow these values but cannot exceed them.
9. Package presence is not model admission, activation, inference support, release approval, or a Fedora/Ubuntu support claim. The executable remains self-check-only until Sprint 13 implements the complete candidate-neutral runtime and proposal protocol.

## Verification

- Python unit tests mutate archive identity, path shape, member type, package manifest, retained library identity, and destination existence.
- Rust tests mutate package/profile/source identity, store object identity, owner, mode, every resource class, swap, parallelism, and endpoint topology.
- A native evidence run builds twice from the pinned local archive, requires byte-identical output, verifies every package member, scans for prohibited surfaces, and binds the result to committed source.
- The existing dependency and architecture checks continue to require `platforms/linux-inference` to depend only on `agentmage-kernel-contracts`.

## Consequences

- AgentMage owns the only future runtime entry point; upstream server, CLI, RPC, and download behavior cannot be invoked from the product package.
- CPU is the initial security-reference backend for this package increment. Vulkan or another acceleration backend requires its own exact package profile, dependency and driver inventory, attack tests, resource evidence, and explicit promotion without replacing this evidence.
- Docker Model Runner remains a distinct optional topology owned by Sub-tasks `9.2.1.2` through `9.2.1.4`.
- Runtime execution, model descriptors, inference, codecs, and cancellation remain owned by Sprint 13. This decision narrows those future inputs; it does not pull their implementation forward.
