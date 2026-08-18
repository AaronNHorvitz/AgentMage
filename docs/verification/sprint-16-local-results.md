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
- Deterministic RPM and DEB candidates include the read-only worker as a
  verifier-required executable; extraction, exact manifest comparison,
  mutation refusal, and deterministic rebuild pass locally.
- Native Fedora 44 and Ubuntu 26.04 KVM guests install those candidates and run
  `agentmage.workspace.search-text` through the root-owned mode-`0755` worker
  under strict-offline Bubblewrap/systemd isolation. Each target retains one
  receipt, preserves the workspace, removes the package, and leaves no worker,
  unit, listener, overlay, transient source, or credential residue.
- Candidate construction normalizes every archived payload directory to mode
  `0755`; a regression test proves a group-writable staging parent cannot enter
  the DEB or RPM payload.
- Existing Linux namespace, network-syscall, path, descriptor, and workspace
  invariance contract tests pass.

## Open Evidence

The production Linux manifest correctly rejects a user-owned development
worker. The installed root-owned worker now has current native evidence for one
SearchText operation on Fedora and Ubuntu. The other nine production
operations, full live attack matrix, worker cancellation/timeout/kill/crash
campaigns, independent worker review, and native macOS XPC evidence remain
open. Model-context disclosure handling is locally verified. The remaining live
campaigns and platform evidence are blockers, not waived or substituted by the
passing installed subset.

The machine-readable records are
[`local-evidence-report.json`](../../artifacts/sprints/sprint-16/local-evidence-report.json)
and
[`installed-linux-worker-matrix.json`](../../artifacts/sprints/sprint-16/installed-linux-worker-matrix.json).
