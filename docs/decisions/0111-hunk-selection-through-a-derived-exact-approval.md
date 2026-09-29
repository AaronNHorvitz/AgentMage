# Decision 0111: Hunk Selection Through a Derived Exact Approval

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29; design for AMR-04.2, not yet implemented |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0075, 0081, 0107 and 0109; the implementation amendment; current owner restart |
| Scope | AMR-04.2: bind an exact hunk selection into the write preview, approval and grant path |

## Findings

The protected approval challenge presents the complete schema-bound arguments of one
proposed call. Its preview digest binds what the client must show. An allowed
response lets the runtime execute exactly that call (Decision 0075). A structured
patch's arguments are edits over an exact preimage, not the resulting text, so a
person sees edit operations rather than hunks. If the runtime executed a narrowed
version of an allowed call, exact-call binding would break. So would a client that
changed the arguments.

## Decision

Selecting hunks never changes an approved call. It creates a new, separately approved
call.

1. The trusted host boundary holds the target and its exact preimage. It computes the
   proposal's complete postimage with the existing planner and divides it into hunks
   under Decision 0107. It adds a versioned review extension to the approval
   presentation: per file, the preimage and proposal digests, each hunk identity and
   range, and the bounded escaped display. The preview digest covers the extension.
   The extension is offered only to a client that declared the hunk-review capability
   for the run. Any other client keeps whole-call review, and an unknown extension
   version fails closed.
2. A new response disposition lets the person accept a subset of the presented hunk
   identities for that exact challenge. The runtime treats the response as a refusal
   of the original call, so the original arguments are never executed.
3. The runtime then reads the current bytes. If they differ from the reviewed
   preimage, it refuses the selection and requires a fresh proposal and preview. If
   they match, the host builds the selected postimage and proposes a call to a
   separate registered tool that writes one exact selected postimage. The call's
   arguments bind six values: the original call and argument digest, the preimage
   and proposal digests, the accepted and rejected hunk identities, the selection
   digest and the postimage digest. That call receives its own exact challenge showing
   only the selected change. It needs a new allow and its own single-use grant. The
   existing syntax, scope and verification checks still apply.
4. The model receives a result for its original call. The result states that the
   person narrowed the call and names the derived operation and the rejected hunks.
   It is not reported as a success of the original arguments.

Human edits are preserved in two ways: rejected hunks keep their preimage lines, and
drift refuses the selection. Nothing about a selection grants authority by itself.

## Verification boundary

Implementation needs contract fixtures for the new disposition and presentation
extension, including old-client refusal. It needs coordinator tests for derived
proposals, drift, stale and replayed selections, denial and cancellation, host tests
for the selected-write tool, and CLI rendering tests. The positive proof that human
edits and rejected hunks survive needs actual processes on a native host. Until then
AMR-04.2 stays open.
