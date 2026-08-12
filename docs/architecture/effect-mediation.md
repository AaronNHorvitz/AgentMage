# Effect Mediation Boundary

This document describes the live mediation boundary through the Phase 6
candidate. Decision 0014 governs opaque effect permits; proposed Decision 0015
governs their exact held-target payload and Linux object exposure. The machine
boundary check is `python3 scripts/effect_boundary.py`.

## Authority Flow

```mermaid
sequenceDiagram
    participant S as Shell
    participant K as Kernel coordinator
    participant G as Grant issuer and policy
    participant D as Effect driver
    participant O as Operating system or service

    S->>K: Exact transaction request and inert driver
    K->>G: Validate current policy and exact grant
    G-->>K: Atomically consumed grant, exact targets, exclusions, and preimages
    K->>K: Record attempt and launch commitment
    K->>D: Consume opaque one-attempt permit
    D->>O: Perform exact configured effect
    O-->>D: Bounded result
    D-->>K: Outcome, digest, and state-change class
    K->>K: Reconcile and append terminal receipt
    K-->>S: Receipt with typed bounded output retained by driver
```

The shell supplies a request and an inert driver. It never receives the permit.
The driver cannot be invoked through its effect method without the permit, and
the permit cannot be constructed, copied, serialized, or reused outside the
kernel transaction. A held-object driver also must match its path,
authorization, adapter, platform, identity, and preimage to the consumed target.

## Process and Socket Boundary

The Linux sandbox driver is the only public façade over worker process launch.
It owns one continuously held object and starts the already verified
`systemd-run` and Bubblewrap chain only after it receives a matching exact
`workspace_read` permit. A file worker receives only a sealed projection that
exactly matches the grant-bound preimage; a directory worker receives only a
sealed bounded exclusion-safe projection. The original held objects and
workspace root never enter either worker. The Linux
Secret Service driver is
the only public façade over `secret-tool` process launch and requires an exact
`credential_access` permit. Both retain bounded outputs and return only a
content-free result identity to transaction reconciliation.

The Unix listener remains module-private and has no product driver. Host and
capability crates may not launch processes, create listeners, or mutate files
directly; the repository guard rejects those source patterns. A future network
or command driver must add a closed operation mapping and consume the same
permit type before the effect becomes available.

## Data Boundary

```mermaid
flowchart LR
    R["Private request fields"] --> K["Kernel transaction"]
    G["Consumed grant digest and exact target binding"] --> K
    K --> P["Opaque borrowed permit"]
    P --> D["Admitted effect driver"]
    D --> E["Exact configured effect"]
    E --> B["Bounded typed output"]
    B --> H["Content-free result digest"]
    H --> K
    K --> T["Terminal receipt"]

    O["Observation APIs"] --> V["Observed values"]
    V -. "no conversion" .-> P
```

Secret bytes are never placed in the permit, transaction record, result digest,
or receipt. Sandbox diagnostics remain hashed and bounded. Configuration-driver
debug output omits paths and candidate content. Observation values remain
non-authoritative and have no conversion into a permit.

## Public Surface Rules

- The coordinator owns permit construction and grant consumption.
- Drivers consume one permit by value and cannot retain its borrowed lifetime.
- Raw sandbox, secret, configuration-mutation, and socket entry points are not
  public.
- New permit consumers must be added to the explicit source inventory.
- Kernel code cannot import platform, capability, or shell crates.
- Read-only capability packs import contracts only.
- Shell and capability source cannot directly launch a process, create a socket,
  or call common filesystem mutation functions.

## Current Limits

This boundary is not complete product integration. Canonical target parity and
exact-object Linux exposure are implemented in the Phase 6 candidate. Durable
atomic transaction storage and restart recovery remain Phase 7 work. The
application host does not yet compose the real driver workflow. macOS is a
declared but unmaterialized mediation edge and remains `blocked-macos`.
