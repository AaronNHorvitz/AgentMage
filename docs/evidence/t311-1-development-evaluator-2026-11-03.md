# T-311.1 development evaluator pilot — 2026-11-03

Status: builder implementation and scoped gate pass; reserved task validation
and current-harness readiness attempt queued. T-311.1 and parent T-311 remain
open. No independent review, full-suite completion or model score is claimed.
Tier: core, because the evaluator reads candidate files and launches workers.

## Scope

The [development evaluator guide](../guides/coding-evaluation.md) describes three
real tasks from this repository's own history. Each pins its base commit, statement,
source projection, editable path, historical reference source, hidden regression
module and exact test count. Only development tasks exist here; no held-out set
has been selected, read or scored. This does not complete T-311's 30-task suite.
No agent harness, model prompt, hidden test or existing threshold was changed.

One command runs base/reference discrimination. Separate prepare and score commands
withhold tests/history/answers from the editing projection, refuse edits outside
one source file and snapshot the candidate before grading. The worker has isolated
network/PID namespaces, read-only runtime/code/tests and temporary scratch. The
parent keeps bounded raw stdout/stderr outside the worker's writable result mount.
No product permission, admission, executable trust or sandbox check changes.

## Observed checks

Source base before this implementation: `25c580f871653171543932ca1aa14dae384dc138`.
The per-change gate selected Markdown, Python syntax/indentation/whitespace and
`tests.test_coding_eval`; there is no touched Rust crate. Its PASS does not
establish a green full Rust gate or resolve deferred T-301.1.

| Check | Observed result |
| --- | --- |
| Initial fast gate | FAIL: one actual boundary test; bubblewrap required an explicit user-namespace flag |
| Corrected fast gate | PASS, 16 tests, no failures/skips, 0.412 s test body; namespace flag added without relaxing confinement |
| Actual worker boundary test | PASS: candidate/test inputs read-only, no lane mount, no connection to a host listener |
| Candidate checks | PASS: sole editable file, immutable other files, no additions/deletions/extra directories, aliases, hard links, FIFOs or oversize inputs |
| Result and failure checks | PASS: missing/malformed/zero/wrong test counts and skips cannot pass; bounded output overflow and timeout fail |
| Frozen task definitions | PASS: pinned Git blobs/projections and separate task-definition fingerprint |
| Reserved base/reference validation | QUEUED, not observed yet |
| Reserved current-harness readiness attempt | QUEUED, no inference or coding result observed |

The per-file scratch allowance permits the unchanged historical oversize-input
regression; separate parent capture still enforces 2 MiB per stream. Both the
initial failed launcher check and corrected runs remain in private raw logs.
The unit boundary fixture is synthetic and is not counted as a real coding task.

## Source and retained records

| Artifact | SHA-256 |
| --- | --- |
| `scripts/coding_eval.py` | `ada163201eddc8c08624304aea1d614b0dbbe6c619f3e8dc6fb091cdedb36f66` |
| `tests/test_coding_eval.py` | `b62293ac17a1eae00219f5f69978d200a48ef5513bd4a50544e46d131f8c3352` |
| Corrected source gate raw log (private) | `cbf8b6ce98af3c8fea6d14501f3f25bb8dfbe97664c18c38885c26cfe1d9f912` |

Task-definition SHA-256:
`f950e413f8dc4bf7ff64e323a5b78c2ba018091b2429e8999eb5850de6b1c00a`.
Raw records are private because paths and test diagnostics are host-specific.
No model weights, credentials, private host paths or task transcripts are copied
into this evidence. No new dependency, acquisition, licence rewrite, SBOM renewal
or long acceptance gate occurred.

## Remaining work and limits

Reserved checks are pending; the shared reservation has been unavailable during
this source increment. A current-harness startup refusal must remain an unavailable
baseline with no model score, not be converted to task success or model failure.
Full T-311 still needs at least 30 varied tasks, sealed held-out inputs and a
bounded/resumable current-harness adapter with actual local-model attempts,
first-attempt/retry metrics, wall/GPU time, tokens and failure causes. Harness
and prompt tuning stay deferred until that baseline is recorded.

The grader imports candidate Python into the historical test interpreter. This
withholds tests while editing but does not prove resistance to deliberate test
framework/result tampering; independent result checking is still needed for a
trusted generated-candidate scoreboard. This increment runs no model candidate.
The frozen base and reference checks use unmodified historical
test bytes; reference patches will never count as agent attempts or scores.
