# Decision 0030: Closed Linux Docker Model Runner Compatibility Profile

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-13 |
| Scope | Fedora/Ubuntu x86_64 Docker Model Runner package, daemon privilege, immutable engine/model identities, mounts, resources, loopback endpoint, and offline compatibility contract |
| Implements | Sprint 9 Sub-task `9.2.1.2` |
| Amends | Decisions 0001, 0028, and 0029 by defining the optional Docker compatibility input beside the native reference runtime |
| Preserves | Native llama.cpp as the Linux security reference, zero enabled models, no automatic fallback, acquisition separation, exact runtime identity, guarded kernel mediation, and no workspace/tool/grant/credential authority |
| Does not implement | Docker installation, daemon startup, direct Docker Engine verification, raw-endpoint isolation, reachability enforcement, model activation, inference, streaming, cancellation, or release approval |

## Context

Docker Model Runner is an optional Linux compatibility runtime, not AgentMage's security reference. Docker documents that Docker Engine installs Model Runner through the `docker-model-plugin` package and ordinarily exposes its API through local port `12434`. Inspection of the pinned runner image shows an immutable wildcard bind rather than a supported host-bind option. Decision 0033 therefore admits it only inside a loopback-only, route-free private namespace. Local reachability does not authenticate callers. Docker Engine administration through its socket also carries host-equivalent authority, while Model Runner may perform registry `HEAD` requests unless egress is independently prevented.

Earlier quarantined evaluation retained exact Docker Model Runner and Gemma 4 E4B OCI identities and ran the runner image under rootless Podman. That evidence was explicitly not a Docker Engine support claim, and the evaluated model failed required quality thresholds. This decision may reuse immutable identities but cannot promote the model, substitute Podman for Docker Engine, or borrow old execution as evidence for this topology.

## Decision

1. [`docker-model-runner-v1.2.6-linux-x86_64.json`](../../model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json) is the sole Docker compatibility profile in this increment. Its exact SHA-256 is compiled into the isolated inference adapter.
2. The admitted plugin package tuple is `docker-model-plugin` version `1.2.6`. Installation, upgrade, rollback, and daemon lifecycle remain separate administrator operations; AgentMage receives no package-manager or Docker-daemon control authority.
3. The sole runner input is `docker.io/docker/model-runner:v1.2.6-cuda` at OCI manifest digest `sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9`, runtime source revision `72874f559c598b8f89fbb24864868337cf5afb4c`, and runtime version `b9879`. A mutable tag never establishes identity.
4. The sole model compatibility fixture is `docker.io/ai/gemma4:e4b` at manifest digest `sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444`, with exact config, model-layer, and projector-layer digests retained in the profile. This model remains blocked, unloaded, and unavailable to user data.
5. The accepted Docker prerequisite is a separately administered rootful daemon owned by UID `0`, an exact observed daemon executable, and an exact observed Docker socket object. The non-root AgentMage launch user must not belong to the socket-owning group because that group conveys host-equivalent Docker authority. Rootless Docker is not silently treated as equivalent and requires a later explicit profile.
6. Neither the AgentMage inference adapter nor the Model Runner container receives the Docker socket. The adapter receives no workspace, host-root, credential, tool, grant, shell, package-manager, or daemon-control handle.
7. The mount closure is a private runtime tmpfs and Docker-managed access to the exact immutable OCI model artifact. Workspace, credential, host-root, and Docker-socket mounts are zero. Runtime writes to the model content store are prohibited.
8. The only declared raw Model Runner bind is IPv4 `0.0.0.0:12434` inside the exact private namespace, and the guard's only connection target is `127.0.0.1:12434` in that namespace. Only `/engines/llama.cpp/v1/chat/completions` is in the future inference contract. A wildcard host or bridged bind and every model-management endpoint are prohibited. The extension and tool workers never receive either endpoint.
9. Runtime mode requires acquisition disabled, registry access disabled, `--do-not-track`, no ambient proxy or Domain Name System path, zero outbound bytes, one inference slot, zero swap, and explicit memory, task, CPU, lifetime, and output ceilings.
10. The adapter remains self-check-only and reports Docker compatibility unavailable. Sub-task `9.2.1.3` owns the guard contract; `9.2.1.4` owns preflight and drift-refusal semantics; `9.2.1.5` owns their production executables; `9.2.2.1` owns clean native Docker Engine inspection on Fedora and Ubuntu.

## Verification

- Rust tests mutate package version, profile identity, runner and model OCI digests, daemon and socket identity, daemon and launch user, prohibited runtime-user Docker-group membership, rootless mode, every mount class, every resource class, loopback address and port, tracking, acquisition, registry access, proxy, Domain Name System, egress, and endpoint topology.
- The packaged adapter self-check binds both native and Docker compatibility profile identities while retaining zero enabled models, no listener, and no inference operation.
- A retained source-bound report verifies the profile hash, immutable OCI identities, exact authority closure, prerequisites, mounts, network/resource declarations, prior-evidence limitations, and the absence of Docker on the current host.
- Existing architecture, dependency, component-inventory, artifact, strict-local, documentation, and clean-build gates must pass without treating Docker as an end-user ambient dependency or supported runtime.

## Consequences

- AgentMage can develop against a precise Docker compatibility contract without installing a privileged daemon or enabling a blocked model on the maintainer host.
- Docker Engine absence is a visible `not-run` state, not a degraded success path.
- The Docker profile may fail or remain blocked independently while native Linux work continues, unless a failure reveals a shared-contract defect.
- Live Docker Engine support, endpoint isolation, clean Fedora/Ubuntu lifecycle evidence, model quality, and release approval remain open and independently reviewable.
