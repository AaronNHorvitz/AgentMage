# Decision 0117: Review Fixes for Run Declarations

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088, 0115 and 0116; current owner restart |
| Scope | Findings V1 through V4 of the independent review of `8fbd2bc6`; amends Decisions 0115 and 0116 |

## Findings

An independent read-only review of `435f3416..8fbd2bc6` passed with one medium and
three low findings.

- V1 (medium): a run resumed from an event cursor after a host restart is composed
  again, with a new effect record and a new list of context views. Both start empty
  at the restart and were still marked complete. The declaration could therefore
  leave out a write approved before the restart, and the view count could be short.
  This broke Decision 0116's rule that an incomplete record is never declared or
  shown as complete.
- V2: the Story 23.4 boundary check did not refuse two ways of replacing the request
  before its dispatch site. One rebinds `requested` through a pattern; the other
  passes it through a helper that assigns `origin`. Decision 0115 said more than the
  check verified.
- V3: nothing tested the host service's gating of declarations, the CLI's
  verification of received declarations, the IPC capacity refusal, the 64-view bound
  or the engine's rule that the ended boundary and context port are unreadable until
  the outcome.
- V4: the over-long selection line was tested only at the private reader. Nothing
  checked that the public reader uses the general bound and discards the rest of the
  line, or that the prompt asks again.

## Decision

V1. A resumed run declares neither part. The host service answers both parts as
unavailable for any request with an event cursor, without asking its owners. The
owners enforce the same rule:

- the effect recorder refuses to declare a resumed request;
- the context port marks its view list incomplete when it composes a context for a
  resumed request;
- the development host marks the port incomplete when it composes a resumed run.

A resumed run therefore shows its recoverability and context views as unavailable.
Rebuilding them from the durable journal is left to AMR-04.7.2, which also extends
the declaration to a durable session scope.

V2. The check now requires that `requested` appears in `request_tool` only three
ways: as the parameter, in the one destructuring, and as `requested.origin`. Any
other use fails, including a pattern binding, a tuple, a closure parameter or a
helper argument. The check also refuses any assignment to an `origin` anywhere in the
runtime loop, so no helper can reclassify a request. Its unit test refuses the
review's two probe mutants and eight more, and still accepts comparisons of the class.
Decision 0115's statement for V2 is narrowed to these forms.

V3. New tests cover each untested path:

- the live host service refuses declarations for an unknown or released run, a
  foreign request digest, and a run whose worker is busy without an outcome; it
  returns the coordinator's own declarations after the outcome;
- the CLI drops declarations that name another run or request, and drops a report
  for another run or with a stale seal while keeping the context views;
- the IPC service answers an oversized declaration with a capacity refusal at one
  byte over its bound, still answers later requests, and ends only on shutdown;
- a context port with one context past the bound shows its views as unavailable;
- the ended boundary and context port read as absent before the outcome and while
  an approval is pending.

V4. The public line reader is a thin wrapper over a descriptor reader, and the test
now drives that reader with a pipe. The selection prompt takes its line source as an
argument; its test feeds an over-long line and then a selection, and checks the
repeated prompt, the refusal notice and the narrowed selection.

## Verification boundary

Each fix has a regression test at the host, engine, Linux-reader or checker level. For
V1, V3 and V4, reverting each fixed rule by a single mutation made its new test fail;
the V2 test is itself a set of mutants of the checked source.
No model, GPU or native workflow was used. The native display of declarations
remains AMR-04.7.2 and AMR-04.10. Independent re-review of this batch is requested
and remains open.
