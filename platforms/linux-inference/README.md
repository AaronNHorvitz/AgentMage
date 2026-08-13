# Linux Native Inference Boundary

This module is the separately packaged Fedora and Ubuntu native inference
adapter process. It depends only on AgentMage's shared contracts and has no
kernel-engine, workspace, tool, grant, credential, connector, or model-artifact
dependency.

The current executable implements only an exact self-check and a fail-closed
inactive state. It does not load a model, perform inference, open a listener, or
accept ambient configuration. The candidate-neutral model runtime, codecs,
profiles, streaming, cancellation, and resource protocol remain owned by Sprint
13. Consequently, packaging this boundary does not enable a model or make a
runtime-support claim.

The native Linux package profile is
[`llama-cpp-b10333-linux-x86_64.json`](../../model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json).
It derives one deterministic CPU-library-only package from an exact upstream
archive and excludes every upstream executable, server, RPC surface, download
surface, Vulkan backend, and model artifact. The Rust contract binds that exact
package to a private standard-user model-store identity, one authenticated
kernel endpoint, one inference slot, zero swap, and bounded cgroup inputs. These
are preconditions for later runtime work, not an inference or release claim.
