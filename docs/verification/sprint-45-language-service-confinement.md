# Sprint 45 Language-Service Confinement Report

## Scope

This report describes the source-level descriptor, request, response, and host
composition controls implemented for Sprint 45. It is not evidence that a production
language server has been launched in an operating-system sandbox.

## Implemented Contract Controls

| Threat | Deterministic control | Current evidence |
|---|---|---|
| Executable or version substitution | Exact service ID, implementation version, executable SHA-256, descriptor digest | Unit mutation matrix |
| Parser/grammar drift | Exact compiled grammar descriptor SHA-256 | Descriptor validation and mutation rejection |
| Workspace expansion | Exact workspace-root digest and sorted file snapshots with path, source digest, and byte length | Outside-workspace and altered-path rejection |
| Network use | `network_allowed` must be false | Descriptor hostile matrix |
| Direct writes | Read-only true; workspace-write false; response mutation false; item write authority false | Descriptor, item, and terminal-state matrix |
| Command execution | Command-execution false and response-command-executed false | Descriptor and response mutation matrix |
| Package or plugin installation | Package-installation and plugin-loading false | Descriptor hostile matrix |
| Ambient executable discovery | Executable-discovery false | Descriptor hostile matrix |
| Secret-bearing environment | Inheritance false; allowlist exactly `LANG`, `LC_ALL`, `NO_COLOR` | Secret-name rejection |
| Oversized or unbounded output | Fixed response-item, timeout, memory, and CPU limits | Descriptor bounds |
| Hostile response range | Item path must be granted; source digest must match; range must fit exact source length | Observation validation |
| Response claims success after timeout or rejection | Closed terminal-state semantics with content-free code | False-terminal rejection |
| Model treats response as authority | Every item is untrusted and observations carry no write authority | Exact resealing and host composition boundary |

## Verification Surface

The focused unit suite covers all five read capabilities, seven forbidden descriptor
powers, secret environment injection, workspace escape, hostile item authority, false
terminal states, and descriptor/request/item/digest mutation. The structured coding
corpus separately checks parser-backed and safe-fallback proposals. The host bridge
accepts only verified source plans and exact held target preimages before constructing
a kernel shadow change set.

## Missing Production Evidence

The following remain blocking:

- a registered production launcher with exact installed executable identity;
- native process, network, filesystem, environment, plugin, resource, timeout,
  cancellation, and cleanup traces on each required platform;
- hostile real-server fixtures for configuration, initialization options, plugins,
  workspace folders, generated code, commands, URI schemes, and oversized responses;
- proof that canonical and neighboring filesystem, Git, process, socket, credential,
  and secret state remain unchanged;
- trusted package execution, independent boundary review, and deferred manual fuzzing.

Until those artifacts exist, language-service support is an implemented and locally
contract-tested source boundary, not an enabled product capability.

