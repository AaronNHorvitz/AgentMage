# Story 0.3 Guarded DMR Reachability Summary

| Probe | Observed | Result |
|---|---|---:|
| `emulated_lan_peer` | `ConnectionRefusedError` | PASS |
| `guarded_kernel_adapter` | `HTTP_200` | PASS |
| `separate_user_and_network_namespace` | `PermissionError` | PASS |
| `tool_container` | `HTTP_401` | PASS |
| `unrelated_same_user_process` | `HTTP_401` | PASS |

The exact pinned Docker Model Runner binary and filesystem ran inside private user, mount, and network namespaces. Its unauthenticated raw API existed only on a private tmpfs Unix socket. A non-dumpable evaluation guard exposed a mode `0600` Unix socket and required a fresh in-memory bearer value. Only the designated guarded request returned HTTP 200.

An unrelated same-user request received HTTP 401 and could neither traverse the supervisor's `/proc` root nor find the raw socket in the stopped image root filesystem. A tool container received HTTP 401 even with its SELinux label disabled for the probe. A separately mapped namespace received a filesystem denial. A rootless network peer connecting to the host's private non-loopback address received `ConnectionRefusedError`. The private DMR namespace contained only loopback, zero routes, zero TCP listeners, and zero network-interface bytes before and after the probes.

This is a `PASS` feasibility result and a `BLOCKED` production-support result. The non-loopback peer was a separate rootless network namespace, not a second physical workstation. The exact image filesystem was used, but the DMR binary was not an OCI workload in this guarded prototype. The guard is evaluation-only Python, no rejected model was loaded, and no product support or release approval is claimed. Raw JSON remains authoritative.
