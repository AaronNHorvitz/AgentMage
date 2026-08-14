# S-012-UT02 Hostile Model Candidate Results

## Result

Pass for false, malformed, contradictory, and authority-seeking model plan and tool-call candidates.

Focused cases closed: **5 of 5**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `HMC-01` | Invalid plans | Reject malformed, false-completion, and competing-running-step plans before a controller exists | Pass |
| `HMC-02` | Bounded plan repair | Admit a valid second candidate, inspect no candidate after two failures, and produce an explicit evidence-empty `Unknown` response on exhaustion | Pass |
| `HMC-03` | Contradictory claims | Detect conflicting exact values for one claim key and expose uncertainty without selecting either value | Pass |
| `HMC-04` | Authority-seeking candidates | Keep authority-looking plan text inert and deny a model tool call that asks to authorize and execute itself | Pass |
| `HMC-05` | Tool-call repair and safe stop | Bound validation to two calls; reject malformed, unregistered, and schema-mismatched calls; keep a corrected call denied pending an exact grant | Pass |

## Evidence

- [`s012_ut02.rs`](../../kernel/engine/src/s012_ut02.rs) is a test-only adversarial harness over public plan, reasoning, authority, and tool-dispatch contracts.
- The repair harness has an exact two-attempt ceiling and never supplies authority or an executor.
- Every tool receipt reports `NotChanged`, zero elapsed execution time, no output, and no evidence.
- [`hostile_model_candidate_evidence.py`](../../scripts/hostile_model_candidate_evidence.py) binds this report, the test source, and exact command identities to an immutable Git revision.
- [`test_hostile_model_candidate_evidence.py`](../../tests/test_hostile_model_candidate_evidence.py) rejects source, case, claim, command, revision, limitation, and artifact identity drift.

## Limits

- The bounded repair controller is test-only verification logic; production model proposal codecs, streaming validation, and retry orchestration remain Sprint 13 work.
- The suite invokes no real model, model runtime, tool executor, grant issuer, worker, platform adapter, filesystem effect, or network request.
- Exact claim-key contradiction detection does not claim natural-language contradiction inference.
- The tool dispatcher proves a pre-grant safe stop; later grant-mediated execution is outside this artifact.
- Cross-platform execution, persistence, restart recovery, packaging, release acceptance, and manual fuzzing remain later gates.
