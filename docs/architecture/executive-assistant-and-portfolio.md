# Executive Assistant and Task Portfolio

## Boundary

The Sprint 54 source candidate projects already-approved local records into rankings, trackers,
briefs, drafts, triage rows, portfolio history, and audit views. It does not read a vault, folder,
calendar, or inbox directly. Existing authorized adapters must first produce exact local records.

```mermaid
flowchart LR
    P["Approved plain-folder records"] --> C["Closed executive record grammar"]
    O["Approved Obsidian records"] --> C
    M["User-provided message export"] --> C
    S["User-provided schedule export"] --> C
    C --> R["Deterministic reconciliation and ranking"]
    R --> V["Trackers, briefs, drafts, triage, and audit views"]
    V -. "proposal only" .-> U["User-controlled workflow"]
    V -. "no send, schedule, notify, or write" .-> X["External state"]
```

Every record and source field distinguishes `confirmed`, `inferred`, `historical`, `disputed`,
and `unknown`. Active or completed decisions and commitments require confirmed evidence. Person and
organization briefs reject inferred sensitive-trait fields. Source correction and supersession
create a new snapshot and preserve earlier records in history.

## Priority Method

The fixed v1 method uses integer contributions for urgency, importance, explicit user preference,
consequence of delay, closed due window, schedule conflict, unresolved dependencies, bounded effort,
and evidence uncertainty. Every component and limitation is visible. Equal scores are ordered by
stable record identity. A ranking is a recommendation only and cannot assign an owner or change a
task, notification, reminder, or schedule.

## Privacy Classes

Ordinary records may use the general local index. Private and confidential records require
separate indexes; confidential retrieval and export require exact-source handling and explicit
review. Highly restricted records cannot enter a general or separate index and cannot be exported;
exact approved retrieval and retention of no more than seven days are the only representable
proposals. Policy decisions perform no operation.

## Product Composition and Open Boundaries

The registered host coordinator binds one immutable canonical snapshot to the ranking, tracker,
view, privacy decisions, and all eight admitted authority-free skills. It rejects partial
cross-privacy composition and verifies that every output remains proposal-only, source-preserving,
and unable to notify, send, schedule, write, or mutate.

Native Chat and CLI presentation, durable reminder lifecycle, live calendar or message connectors,
installed-platform acceptance, accessibility, lifecycle, independent review, signing, and manual
fuzzing remain separate later evidence.
