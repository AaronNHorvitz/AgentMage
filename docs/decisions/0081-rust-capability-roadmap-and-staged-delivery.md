# 0081: Rust Capability Roadmap and Staged Delivery

| Field | Value |
| --- | --- |
| Status | Accepted for product direction and implementation sequencing |
| Date | 2026-09-24 |
| Authority | Repository owner's explicit development replan and restart instruction |
| License effect | None; proposed changes require the separate owner license decision |

## Decision

Adopt [the 48-capability catalogue](../../CAPABILITY-ROADMAP.md) and
[the component implementation amendment](../../IMPLEMENTATION-AMENDMENT.md).
They extend the preserved PRD and task plan; they do not claim implementation or qualify a release.

Finish and reconcile the standalone coding harness (48.2.4-48.2.6 and 50.2.4), then implement the P0 regular-search and bounded deep-research vertical slice. Continue through dependency-ready P0/P1 runtime work before P2. Existing independent acceptance stays open.

Own the task/tool/research/context runtime and its versioned client/model/memory adapters. Do not build a second campaign coordinator, native toolkit or database. Other engines and consuming applications are optional.

Older statements restricting the worker to the previous narrow slice or leaving it indefinitely
paused are superseded by this explicit restart assignment. Existing safety, independent review,
human-only acceptance, external publication and resource boundaries remain binding. Work on
later capabilities only after dependencies, with P0 before P1 before P2 and no silent deletion
of the original roadmap. Keep one actual execution/state owner for each domain.

## Consequences

Add stable work-package crosswalks without renumbering prior tasks. Reconcile and reuse existing
code before extending it. Keep public documentation consumer-neutral. Preserve current licenses
until a separate authorized transition; no private consumer publication is authorized here.
Serialize hardware qualification and use real pinned-model evidence; planning or fake adapters
never prove that combined functionality works.
