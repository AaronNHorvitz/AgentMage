# Effect Mediation Boundary

This document describes the live mediation boundary through the Phase 7
candidate. Decision 0014 governs opaque effect permits, Decision 0015 governs
their exact held-target payload and Linux object exposure, and proposed
Decision 0016 governs encrypted checkpoints and recovery. The machine boundary
check is `python3 scripts/effect_boundary.py`.

## Authority Flow

```mermaid
sequenceDiagram
    participant S as Shell
    participant K as Kernel coordinator
    participant G as Grant issuer and policy
    participant SDB as Encrypted canonical store
    participant D as Effect driver
    participant O as Operating system or service

    S->>K: Exact transaction request and inert driver
    K->>SDB: Persist prepared revision
    K->>G: Validate current policy and exact grant
    G-->>K: Candidate consumed grant and exact authority
    K->>SDB: Atomically persist consumption and transaction revision
    K->>SDB: Persist attempt and launch commitment
    K->>D: Consume opaque one-attempt permit
    D->>O: Perform exact configured effect
    O-->>D: Bounded result
    D-->>K: Outcome, digest, and state-change class
    K->>SDB: Persist result, terminal revision, and receipt
    K-->>S: Receipt with typed bounded output retained by driver
```

The shell supplies a request and an inert driver to the durable runtime. It
never receives the permit.
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
    K --> SDB["SQLCipher authority and checkpoint"]
    SDB --> T["Terminal receipt"]

    O["Observation APIs"] --> V["Observed values"]
    V -. "no conversion" .-> P
```

Secret bytes are never placed in the permit, transaction record, result digest,
or receipt. Sandbox diagnostics remain hashed and bounded. Configuration-driver
debug output omits paths and candidate content. Observation values remain
non-authoritative and have no conversion into a permit. The operational-store
key is exposed only during a platform key-provider callback and never enters a
receipt, command argument, environment variable, or plaintext fallback.

## Public Surface Rules

- The durable authority runtime is the sole public launch surface.
- The coordinator owns permit construction and grant consumption but has no
  public launch method.
- Drivers consume one permit by value and cannot retain its borrowed lifetime.
- Raw sandbox, secret, configuration-mutation, and socket entry points are not
  public.
- New permit consumers must be added to the explicit source inventory.
- Kernel code cannot import platform, capability, or shell crates.
- Read-only capability packs import contracts only.
- Shell and capability source cannot directly launch a process, create a socket,
  or call common filesystem mutation functions.

## Current Limits

This boundary is not complete product integration. Canonical target parity,
exact-object Linux exposure, encrypted authority checkpoints, and restart
recovery are implemented in the Phase 7 candidate. Full Linux state-root and
key lifecycle composition remain Phase 8 work. The application host does not
yet compose the real driver workflow. macOS is a declared but unmaterialized
mediation edge and remains `blocked-macos`.
