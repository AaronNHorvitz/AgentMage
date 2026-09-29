# Decision 0112: Hunk Review and Run Progress in the Development CLI

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0075, 0107, 0110 and 0111; the implementation amendment; current owner restart |
| Scope | AMR-04.2.1 hunk review in write approvals and AMR-04.8.1 run progress after each CLI run |

## Findings

A write approval shows the exact tool arguments. For a structured patch those are
edit operations over a preimage digest, not the resulting text, so a person cannot
easily see what the file will become. After a run, the CLI prints the outcome, but
it does not say which parts of the result the runtime verified and which it cannot
know.

## Decision

Split AMR-04.2. AMR-04.2.1 shows hunks in write approvals. AMR-04.2.2 binds a hunk
selection into the approval and grant path under Decision 0111. Add AMR-04.8.1 for
the CLI view of the run progress projection.

The development CLI renders a hunk review after each patch or creation challenge. The
review is a pure function of two inputs: the challenge's exact arguments, and file
bytes whose digest equals the preimage those arguments bind. The CLI plans the change
with the same structured planner the host uses and divides it into hunks under
Decision 0107, so the review shows exactly what the approved call can write. The
CLI reads the file only from inside its disposable workspace. It refuses to follow a
path out of the workspace, and it never passes bytes to the host or to a write. Under
[Decision 0113](0113-review-fixes-for-inspection-and-job-control.md) (findings V9 and
V15), it opens each path component relative to its parent without following links,
and it shows a creation review only when nothing exists at the target. If the
file changed, the edits do not apply, or the change is not bounded text, the review
says so and the person reviews the whole call. The approval still covers the
complete arguments, and the write still refuses a changed preimage.

After each run, the CLI projects the verified event stream it presented. The
projection states the run's activity and terminal state, whether the runtime's
verifier accepted it, and the declared turn, model-call and tool-call ceilings as
denominators. It always reports independent review and delivery as not established
by the runtime. The CLI writes this summary to standard error, so the machine stream
on standard output keeps its outcome as the last record.

## Verification boundary

Host unit tests cover patch and creation reviews through the real planner, and the
changed, missing and refused cases, escaping, and the reader's workspace boundary for
symlinks and path escapes. They also cover both output formats and the progress
wiring. No actual-process approval with a displayed review is claimed in this lane.
The native workflow, real models and manual acceptance remain open.
