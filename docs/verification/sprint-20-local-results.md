# Sprint 20 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 20 |
| Local core result | Pass |
| Sprint result | Blocked |
| Release approval | No |

## Verified Locally

- Shared contracts expose exactly Observed, Derived, Inferred, and Unknown/Blocked state kinds with
  state-specific, unknown-field-denying provenance payloads.
- Observed assignments require a successful, hash-valid, tool-call-bound Observe receipt and exact
  content-addressed sources at the claim subject and revision.
- Derived assignments require an exact registered deterministic method and distinct same-task
  Observed assignment identities.
- Inferred assignments preserve supporting citations and the exact model run, manifest, artifact,
  tokenizer, template, codec, runtime adapter, runtime build, platform, and response digest without
  treating the model output as proof.
- Eight closed Unknown/Blocked reasons preserve unavailable, denied, failed, conflicting, stale,
  unsupported, unverifiable, and excluded evidence states.
- Internally consistent denial, failure, cancellation, timeout, uncertainty, non-observe,
  missing-tool, stale-source, duplicate, malformed, and model-confidence injection fixtures fail
  closed.
- Evidence assignments are sealed descriptive records and cannot broaden authority or satisfy the
  existing deterministic completion ledger.
- The complete local product gate passed after the implementation, while its pre-existing ignored
  live, native-platform, and large-model tests remained visible.

## Open Evidence

Production response composition does not yet require one assignment per material rendered claim.
Live citation resolution, changed-source freshness checks, the durable answer-claim ledger,
append-only receipt chaining, the external keyed integrity anchor, clock-change sequencing, and
independent review remain absent. These are principally Sprint 21 dependencies rather than hidden
Sprint 20 successes.

Sprint 20 is therefore blocked even though every bounded state-assignment implementation item and
its local adversarial suite pass. The source-bound machine-readable record will be retained at
[`local-evidence-report.json`](../../artifacts/sprints/sprint-20/local-evidence-report.json).
