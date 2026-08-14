# Task 12.1.3.5 Product Security Evidence

## Result

Pass for complete Story 12.1 security-control mapping with explicit remaining work.

Mapped requirements: **15 of 15**.

## Control Map

| Requirement | Story 12.1 contribution | Primary retained evidence | Remaining product gate |
|---|---|---|---|
| `SR-ACC-001` | Demonstrated in story scope | Agent runtime, classification, hostile candidates | Grant-mediated production execution and release composition |
| `SR-ACC-007` | Demonstrated in story scope | Runtime, progress, transcripts | Live user-decision UI and end-to-end handoff denial |
| `SR-AI-003` | Demonstrated in story scope | Classification, reasoning, material claims | Production model and rendered-UI evaluation |
| `SR-AI-004` | Demonstrated in story scope | Runtime, hostile candidates, transcripts | End-to-end consequential action matrix |
| `SR-AI-005` | Partial story evidence | Hostile candidates | Complete direct and indirect injection corpus |
| `SR-AI-006` | Partial story evidence | Runtime fixtures, hostile candidates, mode equivalence | Production model/runtime reliability thresholds |
| `SR-AI-007` | Demonstrated in story scope | Reasoning, hostile candidates, transcripts | Live UI rendering and broader semantic corpus |
| `SR-AI-008` | Partial story evidence | Session environment, attachments, material claims | End-to-end secret-canary and adjacent-workspace campaign |
| `SR-AI-009` | Partial story evidence | Runtime budgets, deterministic boundaries, cancellation | Complete OS process, memory, CPU/GPU, disk, and concurrency campaign |
| `SR-AI-010` | Partial story evidence | Session environment, material claims, runtime schemas | Production inference provenance and durable audit integration |
| `SR-AI-011` | Partial story evidence | Mode equivalence, reasoning, material claims | Versioned factual and coding evaluation at release thresholds |
| `SR-OPS-001` | Partial story evidence | Runtime schemas, progress, transcripts | Complete durable product audit-event registry and store |
| `SR-TST-004` | Partial story evidence | Hostile candidates, deterministic runtime, schemas | Every input family across production boundaries |
| `SR-TST-005` | Partial story evidence | Cancellation and truthful transcripts | At least 100 durable crash-resume trials around every transition |
| `SR-TST-006` | Partial story evidence | Deterministic budgets and cancellation | Full platform resource-governance and responsiveness campaign |

## Retained Proof

- Transition coverage is retained by [`deterministic-runtime.json`](../../artifacts/sprints/sprint-12/story-12.1/deterministic-runtime.json).
- Bounded model-repair traces are retained by [`hostile-model-candidates.json`](../../artifacts/sprints/sprint-12/story-12.1/hostile-model-candidates.json).
- Cancellation proof is retained by [`cancellation-recovery.json`](../../artifacts/sprints/sprint-12/story-12.1/cancellation-recovery.json).
- Truthful interruption and terminal transcripts are retained by [`runtime-transcripts.json`](../../artifacts/sprints/sprint-12/story-12.1/runtime-transcripts.json).
- Fixed-task mode reproducibility is retained by [`reasoning-mode-equivalence.json`](../../artifacts/sprints/sprint-12/story-12.1/reasoning-mode-equivalence.json).

## Limits

- This map closes Story 12.1 evidence organization; it does not mark any requirement complete for a product release.
- All retained records are synthetic, local, content-minimized, and network-free.
- Production model/tool execution, durable restart, live UI, cross-platform packaging, release acceptance, and manual fuzzing remain later gates.
