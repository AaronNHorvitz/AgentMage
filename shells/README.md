# Shells

Shells compose the kernel process or present a user interface. They do not own
policy, grants, workspace access, tools, secrets, or model-runtime authority.

All first-party surfaces share the closed
[thin-client boundary](../docs/architecture/thin-client-boundary.md). Native
Chat and interactive CLI may identify a protected live approval channel; JSON,
SDK, and ACP-compatible clients require predeclared bounded grants. A surface
changes transport and presentation only, never policy or authority.
