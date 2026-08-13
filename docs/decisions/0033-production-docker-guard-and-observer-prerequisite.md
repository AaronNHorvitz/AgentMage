# Decision 0033: Production Docker Guard and Observer Prerequisite

| Field | Value |
|---|---|
| Status | Accepted planning correction |
| Date | 2026-08-13 |
| Scope | Missing executable ownership and exact Docker Model Runner listener behavior before live Linux verification |
| Implements | Adds Sprint 9 Sub-task `9.2.1.5` before Sub-task `9.2.2.1` |
| Amends | Decisions 0030 through 0032 and the Sprint 9 dependency order |
| Preserves | Every completed contract and evidence result, native `llama.cpp` as the Linux security reference, Docker unavailable by default, separate Docker administration, exact identities, zero egress, no automatic fallback, and no support claim without live evidence |
| Does not implement | Docker installation on a user workstation, model admission, inference quality, release approval, or completion of Story 9.2 |

## Context

The completed Sprint 9 Docker increments deliberately stopped at compile-time
contracts. Decision 0031 excluded a production guard executable, and Decision
0032 excluded a privileged topology collector. Sub-task `9.2.2.1` nevertheless
requires live process, identity, capability, namespace, socket, mount, cgroup,
image, and firewall inspection on clean Fedora and Ubuntu systems. No existing
task owned the two executables needed to create and inspect that topology.

Inspection of the exact pinned Docker Model Runner image identified a second
gap. The image accepts `MODEL_RUNNER_PORT` and `MODEL_RUNNER_SOCK`, but exposes
no supported host-bind setting. Its executable contains and uses the wildcard
IPv4 bind. Treating the live listener as a configurable `127.0.0.1` bind would
make the approved profile impossible to reproduce.

Advancing directly to live evidence would therefore either test code that does
not exist or silently weaken the accepted topology. Both outcomes are refused.

## Decision

1. Add Sub-task `9.2.1.5` as an additive prerequisite. Keep Sub-tasks
   `9.2.1.1` through `9.2.1.4` complete, but reopen their parent Task `9.2.1`
   until the missing executable boundary is implemented and verified.
2. Implement two separately packaged Linux executables:
   `agentmage-docker-guard` and `agentmage-docker-topology-collector`. Neither
   executable may install Docker, pull an image or model, change daemon state,
   choose a fallback runtime, or hold workspace, tool, grant, credential, Git,
   shell, package-manager, or release authority.
3. Run the exact pinned Model Runner image with Docker networking disabled. Its
   observed wildcard `0.0.0.0:12434` listener is admissible only inside the
   exact private runner-and-guard network namespace when loopback is the sole
   interface, the namespace has no non-local route, bridge peer, Domain Name
   System path, proxy, or egress interface, and the host has zero Model Runner
   listeners. The guard connects to `127.0.0.1:12434` from inside that namespace.
   A wildcard bind on the host, a bridge, or any namespace with another
   interface is always refused.
4. The guard runs as the exact dedicated non-root guard identity and shares
   only the runner's network namespace. It receives no Docker socket and no
   runner mount namespace. Its own mount, process, user, IPC, cgroup, and
   filesystem restrictions remain independently inspectable.
5. The guard creates one fixed Unix socket in an administrator-created runtime
   directory. The socket is owned by the guard, uses the invoking user's exact
   primary group and mode `0660`, and is reachable only through the parent
   directory's non-writable execute permission. Kernel peer credentials, exact
   process start identity, executable digest, cgroup identity, one fresh
   challenge, one inherited one-use secret, bounded framing, request-path
   allowlisting, replay rejection, cancellation, and teardown are mandatory.
   Group membership alone never authorizes a request.
6. The topology collector runs only as the separately authorized administrator
   identity. It accepts explicit process and session identities, rejects
   ambient discovery and partial observations, and independently records every
   field required by Docker preflight. Its output contains bounded identities,
   counts, booleans, digests, and stable refusal classes rather than source,
   prompt, response, credential, workspace, or unrestricted path content.
7. The collector executable identity, collection protocol version, observation
   nonce, source revision, operating-system image, Docker package and daemon
   identity, runner image digest, and complete raw observation are retained in
   evidence. A synthetic fixture cannot satisfy a live-evidence field.
8. Sub-task `9.2.2.1` may start only after both executables have exact unit,
   integration, mutation, malformed-input, replay, resource-bound, privilege,
   and teardown tests and an explicit source-bound no-live-support report.

## Verification

- Profile and Rust contract tests distinguish the runner's private wildcard
  bind from the guard's loopback connection target and reject every host or
  non-loopback exposure.
- Guard tests cover peer identity, executable and cgroup drift, challenge and
  secret mutation, replay, frame and request limits, forbidden HTTP paths,
  upstream failure, cancellation, socket metadata, and residue-free teardown.
- Collector tests cover every required observation field, root/admin identity,
  explicit target identity, stale and replayed sessions, partial `/proc` and
  Docker state, malformed structured output, command identity drift, and
  content-free failure behavior.
- Packaging tests prove that the two executables and their exact manifests are
  present without enabling Docker mode or changing the native default.
- Retained evidence states that executable verification is not live Docker,
  model, inference, hostile-peer, clean-image, or release evidence.

## Consequences

- Live Fedora and Ubuntu inspection gains an honest producing implementation.
- The approved topology matches the immutable upstream runner instead of
  relying on an unavailable bind option.
- The guard and collector become narrow reviewable privilege boundaries rather
  than logic hidden inside an evidence harness.
- Story 9.2 remains blocked until the later clean-image, hostile-reachability,
  independent-control-disablement, security-evidence, and independent-review
  gates pass.
