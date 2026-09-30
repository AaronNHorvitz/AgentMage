# Decision 0115: Review Fixes for Hunk Selection and the Batch 7 Evidence

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088 and 0107 through 0114; current owner restart |
| Scope | Findings V1 through V10 of the independent review of `435f3416`; amends Decision 0114 |

## Findings

An independent read-only review of `1458cef4..435f3416` passed with two medium and
eight low findings.

- V1 (medium): the host answered a narrowing before it knew whether the derived write
  could be bound. The whole-file edit it used inherited limits sized for single edits:
  64 KiB of text, no carriage return, and a syntax node spanning the whole file. A
  selection in a file over 64 KiB, a CRLF file, or a syntax file that starts with a
  blank line therefore refused the original call and then failed at binding. The run
  ended in an internal error instead of a refusal. Rollback used the same edit and had
  the same limits.
- V2 (medium): the replacement boundary check verified that the dispatch site used a
  variable named `origin`, not that the variable was the request's own field. Rebinding
  it passed the check, and no loop test covered the property.
- V3: the engine did not require a derived call to target a shell-only tool.
- V4: the Linux boundary's narrowing glue was covered only by native-only tests.
- V5: four of the five blocked-refresh causes in the batch 7 record were not shown by
  their retained logs.
- V6: a selection line over 256 bytes aborted the approval.
- V7: a model proposal of a shell-only tool with valid arguments left the run without a
  terminal event.
- V8: accepted suspend and resume requests could fill the client part of a job ledger,
  so a cancellation could no longer be recorded.
- V9: the model's notice said a derived write happened whatever its outcome.
- V10: a model could propose a call under the identity of the next derived call.

## Decision

V1. The structured planner gains a whole-file replacement edit. Only trusted in-process
code builds it, from a postimage that code derived and verified. It cannot be decoded
from any proposal, so no model can request it, and it must be the only edit in a plan.
It is bounded by the planner's file limit of 4 MiB, not by the per-edit text limit. The
postimage must still be valid UTF-8, differ from the preimage and, for a syntax
language, parse. The selected write and the rollback inverse write both use it.

The host now decodes and binds the derived write over the bytes it has just read before
it answers `Narrowed`. A selection that cannot be written, for example one whose result
does not parse, is answered with a denial with reason `runtime.coding.selection-invalid`,
so the run ends declined without a write, as Decision 0114 says. Selections in files
over 64 KiB, CRLF files and syntax files that start with blank lines now bind. The
development CLI keeps offering `select` for every patch with at least two hunks,
because the remaining refusal depends on which hunks are chosen.

V2. `request_tool` no longer binds the proposal class to a local variable. The one
dispatch site uses `requested.origin`, the unmodified field of an immutable request. The
automated Story 23.4 boundary check now requires four things in `request_tool`:

- the request parameter is not mutable;
- the request is never rebound or assigned;
- the class is named only as `requested.origin`;
- the exact dispatch line appears once.

Its unit test refuses the three rebinding mutants from the review and three more. The
statement in Decision 0114 that the earlier replacement check was at least as strict as
the literal check was wrong and is corrected there. The engine also has a loop test for
the property; see V7.

V3. `request_derived_tool` refuses a derived call unless its tool is registered and
shell-only. A pending approval records the proposal class of its call, and only a
model-proposed call can be narrowed. The engine no longer infers this from the registry.

V4. The Linux boundary derives the selected write through a function that takes the
file-reading step as an argument. A host unit test gives it changed bytes and requires
refusal, gives it the unchanged bytes and requires a write, and checks the absent
selection and non-patch cases. Another test requires that a session preauthorization
admitting a patch of a path never admits the selected write of the same path. The
native tests of the whole path remain and must pass before AMR-04.2.3 closes.

V5. From this batch on, a blocked sprint refresh runs the producer's underlying
failing commands as separate planned commands in the same stage, so their own output is
retained. The batch 8 record states which batch 7 causes were asserted rather than
shown.

V6. A general development line may be up to 4,096 bytes, enough to name 511 of 512
hunks. A longer line is read through its newline and discarded; the CLI says the
selection was not accepted and asks again. A remainder over 64 KiB fails closed. The
confirmation reader keeps its 256-byte bound and its existing failure.

V7 and V10. Before it requests a model's call, the runtime checks two things. The tool
must be one that the registry offers to models. The call identity must not start with
`selected-call:`, which is reserved for derived calls. A model proposal that fails
either check ends the run `Failed`, with the diagnostic
`runtime.proposal.tool-not-offered` or `runtime.proposal.call-identity-reserved`,
before any tool budget, event or evaluation is used. A new loop test covers a
shell-only tool, an unknown tool and a reserved identity.

V8. One client entry of each job ledger is kept for an accepted cancellation. Every
other client request, and any refused cancellation, stops one entry earlier. A person
can therefore always cancel a job that has not ended, and the owner can still record
the cancellation or the job's end.

V9. The model's notice says the runtime wrote the accepted hunks only when the derived
write succeeded. Otherwise it says the write did not succeed and that the file may not
contain the accepted hunks. The authoritative result is included either way.

## Verification boundary

Each source fix has a regression test at the engine, host-unit, planner, CLI-reader or
checker level. No model, GPU or native workflow was used. The native Linux boundary
tests and the actual-process proof of AMR-04.2.3 remain open. Independent re-review of
this batch is requested and remains open.
