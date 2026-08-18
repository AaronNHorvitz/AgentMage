# Sprint 16 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 16 |
| Local contract result | Pass |
| Sprint result | Blocked |
| Release approval | No |

## Verified Locally

- The catalog contains ten exact versioned read-only tools and zero
  write-capable operations.
- Every tool passes deterministic golden-result tests and the complete closed
  schema failure matrix.
- File count, input bytes, depth, matches, output bytes, encoding, and call
  depth are bounded.
- Every terminal result state is explicit and every accepted result is bound to
  its complete content by SHA-256.
- Linux request and snapshot projections are sealed, immutable, path-fixed, and
  bound to the same exact ordered held-object set as the consumed grant.
- The authenticated host rejects duplicate, unscoped, stale, wrong-kind, and
  malformed projections before worker launch.
- Every launched read-only effect retains its authority receipt on success,
  worker failure, absent or malformed output, sensitive-output refusal, and
  output-limit failure.
- Credential fields, private keys, bearer credentials, provider tokens, cloud
  access keys, and credentials embedded in URIs are withheld before verified
  tool output can enter model context, runtime events, or artifacts.
- Existing Linux namespace, network-syscall, path, descriptor, and workspace
  invariance contract tests pass.

## Open Evidence

The production Linux manifest correctly rejected the user-owned development
worker binary during a live attempt. A packaged root-owned, non-writable,
hash-verified worker must be installed before production live execution can be
claimed. The full live attack and worker cancellation/timeout/kill/crash
campaigns, independent worker review, and native macOS XPC evidence remain
open. Model-context disclosure handling is locally verified. The remaining live
campaigns and platform evidence are blockers, not waived or substituted by the
passing local contract tests.

The machine-readable source-bound record is
[`local-evidence-report.json`](../../artifacts/sprints/sprint-16/local-evidence-report.json).
