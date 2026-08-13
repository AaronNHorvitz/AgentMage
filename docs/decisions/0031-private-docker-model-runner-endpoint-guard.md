# Decision 0031: Private Docker Model Runner Endpoint Guard

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-13 |
| Scope | Linux Docker Model Runner raw-endpoint isolation and exact kernel-adapter admission contract |
| Implements | Sprint 9 Sub-task `9.2.1.3` |
| Amends | Decisions 0001, 0028, and 0030 by placing the unauthenticated raw Model Runner API behind a two-hop private topology; Decision 0033 corrects the immutable runner bind and outer-socket ownership details |
| Preserves | Native llama.cpp as the Linux security reference, ordinary runtime user without Docker authority, zero enabled models, no automatic fallback, exact identities, no workspace/tool/grant/credential authority, and no support claim without live evidence |
| Does not implement | Docker installation, container launch, a production proxy executable, live namespace or socket observation, model activation, inference, quality parity, or release approval |

## Context

Loopback binding alone cannot keep an unauthenticated API away from unrelated local processes. Giving the ordinary AgentMage user Docker-group authority would also let that user create or enter containers and bypass a namespace or proxy boundary. Decision 0030 therefore requires separate administrator ownership of Docker lifecycle and excludes the runtime user from the Docker socket group.

Sub-task `9.2.1.3` needs an enforceable shape that later platform code can instantiate without exposing the raw host, port, Docker socket, or a reusable credential to Visual Studio Code, tool workers, or the model. The guard must fail before returning any object that can represent raw-endpoint use.

## Decision

1. [`docker-model-runner-guard-v1-linux-x86_64.json`](../../model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json) is the exact guard policy profile. Its SHA-256 is compiled into the isolated inference adapter.
2. Docker Model Runner and one dedicated guard process share one private network namespace. The immutable runner binds `0.0.0.0:12434`; this wildcard listener is admissible only because `lo` is the namespace's sole active interface and there are zero non-local or container-bridge routes. The guard connects to `127.0.0.1:12434` inside that namespace. The host has zero Model Runner TCP listeners, and neither process has egress.
3. The guard runs as a dedicated non-root identity distinct from the ordinary AgentMage runtime user. Its executable and cgroup identities are exact. It receives zero Docker-socket and workspace mounts and no credential, tool, grant, shell, package-manager, or daemon-control authority.
4. The guard exposes one mode-`0660` Unix socket owned by the dedicated guard and grouped to the ordinary AgentMage runtime user's exact primary group beneath an administrator-created, mode-`0710`, group-traversable but non-writable parent. Group access permits connection only; it is never authentication. Admission requires peer credentials, a fresh session challenge, exact process identity, replay defense, bounded framing, and cancellation. Raw endpoint fields are not returned across this socket.
5. The extension, tool worker, unrelated same-user process, arbitrary container, and undeclared process classes are unconditional denials at the raw boundary. Supplying the guard's UID, executable hash, cgroup hash, or endpoint identity cannot convert a denied class into an allowed caller.
6. The guarded kernel Docker adapter is admitted only when caller class, dedicated UID, executable hash, cgroup identity, and fresh authenticated kernel session all match the verified topology. Every mismatch fails before a raw-endpoint permit exists.
7. The permit contains only a content-free endpoint identity and is not serializable, clonable, or constructible outside the module. Model output, tool arguments, configuration, environment, paths, and network addresses cannot create it.
8. Package self-check remains inactive with zero enabled models and Docker compatibility unavailable. Sub-task `9.2.1.4` owns live preflight and independent control disablement; `9.2.2.2` owns live reachability attacks from all declared positions.

## Verification

- Rust tests admit the one exact guard caller and deny all five prohibited caller classes.
- Mutation tests alter guard UID, runtime UID/GID, executable, cgroup, session authentication, namespace identity, raw bind, connect host, port, interface set, route set, host and non-loopback listeners, bridge routes, Docker/workspace mounts, outer socket identity, owner, group, parent mode, socket mode, and peer authentication.
- Type-level tests require the verified guard before a raw-endpoint permit can be returned and expose no address or raw transport handle through that permit.
- A retained source-bound report records the complete caller matrix, profile and source identities, Docker-absent host state, and explicit live-enforcement limitations.

## Consequences

- The implementation has a reviewable authority boundary before a privileged Docker environment is introduced.
- The ordinary AgentMage runtime user cannot use Docker itself to bypass the guard.
- A future live implementation must reproduce this exact process and namespace topology; host-loopback-only deployment is not an accepted fallback.
- Contract proof is not live isolation proof. Docker support remains blocked until the later native Fedora/Ubuntu and hostile reachability gates pass.
