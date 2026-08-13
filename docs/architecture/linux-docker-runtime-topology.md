# Linux Docker Runtime Topology

## Status

This record describes the packaged and KVM-verified optional Docker Model
Runner compatibility boundary. Native `llama.cpp` remains the Fedora and Ubuntu
reference adapter. No model inference, physical-host certification, release
support, or product-wide requirement completion is claimed here.

## Processes and Privileges

```mermaid
flowchart LR
    K["Held AgentMage runtime peer\nUID 10001"] -->|"peer credentials, process start, cgroup, challenge"| G["Docker guard\nUID 10002, zero capabilities"]
    C["Topology collector\nUID 0, one bounded observation"] -->|"read-only procfs and Docker API observation"| D["Rootful Docker daemon"]
    C -->|"held identities and exact topology"| P["Preflight v3"]
    G -->|"loopback only in shared private net namespace"| R["Pinned Model Runner container\nzero added capabilities"]
    P -->|"exact admission only"| K
    X["Extension, worker, same-user process, LAN, or container"] -. denied .-> R
```

The collector's effective-UID-0 requirement is explicit. It proves the current
topology but does not yet satisfy the product-wide standard-user requirement;
no hidden privileged helper or automatic elevation is admitted by this record.

## Sockets and Network Namespaces

```mermaid
flowchart LR
    K["Exact runtime peer"] -->|"Unix socket 0660 under parent 0710\nauthenticated session"| G["Dedicated guard"]
    subgraph PN["Private route-free network namespace"]
        G -->|"127.0.0.1:12434"| R["Model Runner [::]:12434"]
        L["lo only; no non-local route"]
    end
    H["Host network namespace"] -. "no :12434 listener" .-> R
    O["Ordinary container or foreign namespace"] -. "no route or listener" .-> R
```

Loopback and namespace placement provide reachability control, not
authentication. The Unix peer handshake and exact held identities provide the
authentication boundary before a request can reach the guard.

## Mounts and Writable State

```mermaid
flowchart TD
    R["Model Runner read-only root"] --> M["Docker-managed model volume\nmounted read-only at /models"]
    R --> T["Private /run tmpfs\nrw,nosuid,nodev,noexec,size=64m"]
    W["Workspace"] -. "no mount" .-> R
    S["Docker socket"] -. "no mount" .-> R
    C["Credentials or host root"] -. "no mount" .-> R
    G["Guard private mount namespace"] --> U["Guard socket and one-use startup material"]
```

The production collector refuses undeclared mounts, writable model content,
Docker-socket exposure, a writable root, privilege or capability broadening,
resource drift, or ambient network configuration. Evidence for each diagram is
retained in the Story 9.2 topology, reachability, and control-disablement
artifacts rather than inferred from this document.
