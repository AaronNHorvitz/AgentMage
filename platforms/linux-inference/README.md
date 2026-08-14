# Linux Native Inference Boundary

This module is the separately packaged Fedora and Ubuntu native inference
adapter process. It depends only on AgentMage's shared contracts and has no
kernel-engine, workspace, tool, grant, credential, connector, or model-artifact
dependency.

The package now implements the candidate-neutral runtime contract, an exact
native `llama.cpp` driver, closed family-codec boundary, streamed inference,
cancellation, resource reporting, model acquisition, and atomic activation
primitives. The packaged command remains fail closed and accepts no ambient
inference arguments; the operational path is available only through the typed,
authenticated private boundary. This implementation does not by itself enable
a model or make a release-support claim.

The exact Muse Glimmer Fedora evaluation tuple uses the separately pinned
`llama.cpp` b10423 runtime profile and remains disabled. Isolated lifecycle,
sandboxed streaming, and cancellation evidence passed, while the early quality
profile produced no valid closed proposals and is therefore rejected for
ordinary activation. The equal response hashes observed under the diagnostic
profile apply only to that recorded tuple and are not a universal determinism
claim.

The preserved native Linux package profile is
[`llama-cpp-b10333-linux-x86_64.json`](../../model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json).
It derives one deterministic CPU-library-only package from an exact upstream
archive and excludes every upstream executable, server, RPC surface, download
surface, Vulkan backend, and model artifact. The Rust contract binds that exact
package to a private standard-user model-store identity, one authenticated
kernel endpoint, one inference slot, zero swap, and bounded cgroup inputs. These
are retained preconditions from the earlier package-boundary stage, not an
inference or release claim for that profile.

The optional Linux Docker compatibility profile is
[`docker-model-runner-v1.2.6-linux-x86_64.json`](../../model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json).
It pins one plugin version, one Docker Model Runner image manifest, and one
quarantined model OCI manifest. Its Rust contract makes the rootful-daemon and
runtime-user Docker-group exclusion explicit, accepts only a fixed guarded loopback
endpoint, closes mounts and resources, and requires no acquisition, tracking,
proxy, DNS, or egress. Docker is absent on the current host and the profile is
not activated; it provides no inference or Docker support claim.

The separate
[`docker-model-runner-guard-v1-linux-x86_64.json`](../../model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json)
profile places the raw loopback API inside a private runner-and-guard network
namespace. The exact dedicated guard is the sole raw client and returns an
unforgeable permit only for a fresh authenticated kernel session. Its outer
surface is a private Unix socket; all extension, tool-worker, same-user,
container, and undeclared caller classes are denied before a raw permit exists.
This is a compiled isolation contract, not live Docker Engine evidence.

The mandatory Docker preflight in `docker_preflight.rs` compares one complete,
fresh, exact-collector observation to a separately verified baseline before
Docker mode can be represented. Daemon privilege, socket ownership, API binds,
container reachability, image manifests, mounts, cgroup limits, privilege, and
zero-egress state each have a stable terminal refusal class. Package boundary
version 5 advertises preflight contract version 1 while Docker remains
unavailable. No live Docker collector or support claim exists in this increment.
