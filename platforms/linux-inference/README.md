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
