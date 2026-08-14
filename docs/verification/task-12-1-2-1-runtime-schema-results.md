# Task 12.1.2.1 Runtime Schema Results

**Status:** Pass for single-agent state-machine and progress-event schemas

**Task:** `12.1.2.1`

## Result

Two closed Draft 2020-12 JSON Schemas now describe the implemented
single-agent loop and content-free progress event. AJV runs with strict mode,
no type coercion, no unknown-property removal, no defaults, and complete error
collection. Canonical fixtures and mutations are part of the existing schema
gate.

Focused cases closed: **5 of 5**.

| Case | Boundary | Verified result |
|---|---|---|
| `RSC-01` | Canonical records | The exact state-machine and progress-event fixtures validate. |
| `RSC-02` | Closed shape | Missing schema versions and unknown authority-looking fields reject. |
| `RSC-03` | State machine | Five ordered phases, eight ordered legal edges, one terminal phase, and descriptive-only authority are exact. |
| `RSC-04` | Progress event | Sequence and revision are positive; plan events have no step; step events require one exact step; content fields reject. |
| `RSC-05` | Registry | Unknown runtime record types fail explicitly rather than selecting a fallback schema. |

## Limits

- These are formal review artifacts for the current Rust contracts and state
  machine. They do not add a second runtime implementation.
- JSON Schema validation does not execute a model, tool, platform adapter, or
  state transition.
- Status, interruption, final-response, session-environment, and later
  persisted terminal-result schemas remain their assigned artifacts and tasks.
- Canonical fixture digests are synthetic review values; this task is not a
  release-signing or package-attestation claim.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing
  remain later gates.
