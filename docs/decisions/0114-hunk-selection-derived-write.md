# Decision 0114: Hunk Selection Through a Runtime-Proposed Derived Write

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0075, 0081, 0107, 0111 and 0112; the implementation amendment; current owner restart |
| Scope | AMR-04.2.2: bind an exact hunk selection into the approval and grant path; amends Decision 0111 |

## Findings

Decision 0111 keeps exact-call binding: a selection refuses the original call and
yields a separately approved derived write. Four facts shaped the implementation.

- Every denial ended the run. The coordinator had no path that refused one call and
  continued.
- The event contract already allows a new tool request in the same turn after a
  denial, so no new event kind is needed.
- The development CLI already computes hunks from a challenge's exact arguments and
  digest-verified workspace bytes (Decision 0112). A presentation extension is
  therefore not needed to show hunks.
- A creation is a single hunk, so only a structured patch can be narrowed.

## Decision

1. Contracts. `RuntimeApprovalDisposition` gains `Narrow`. `RuntimeApprovalResponse`
   gains an optional `selection` field with two digests: the preimage and the
   proposal. It also lists the accepted hunk identities, sorted and unique. The
   field is omitted when absent, so every allow and deny response keeps its prior
   bytes. A peer that predates the extension refuses to decode `narrow` or the
   field, so it fails closed. The coordinator accepts a selection only with
   `Narrow` and no grant, and it refuses `Narrow` without one.
2. Engine. A trusted boundary answers a narrowing with a `Narrowed` evaluation that
   carries one derived call. The runtime records the original call as denied, with
   `PermissionDecided` and `Deny`, and never executes it. That refusal does not use
   the denial ceiling. The runtime then proposes the derived call in the same turn,
   through the shell's proposal class. The call passes the same steps as a model's
   call:
   - the tool-call ceiling and the event reserve;
   - registry validation;
   - evaluation;
   - its own challenge, approval and single-use grant;
   - execution and verification.

   The derived tool must be registered as shell-only. A model cannot propose it, and
   the runtime refuses to narrow a derived call again, so one proposal yields at most
   one derived write. If the boundary cannot honor a selection, for example because
   the file changed, it answers with a denial and the run ends declined without a
   write.
3. Host. The derived call goes to `agentmage.code.apply-hunk-selection`, a shell-only
   tool. Its arguments carry nine things:
   - the original call identity;
   - the digest of the original arguments;
   - the complete original patch arguments;
   - the preimage and proposal digests;
   - the accepted and rejected hunk identities;
   - the selection digest;
   - the postimage digest.

   The host derives the call only after it reads the held file again. It refuses a
   stale digest, an unknown hunk, a changed file, and an empty or complete selection.
   To write, it re-plans the original patch over the exact current bytes and
   recomputes the hunks and the selection. It then writes only the selected postimage
   as a whole-file structured replacement of the exact preimage, through the existing
   preview, syntax, scope, approval, grant and write path, as rollback does. A session
   preauthorization never admits a derived write, because the write exists only
   because a person decided. The model never sees the derived tool. After the write,
   the model receives a notice that names its refused call, the accepted and rejected
   hunks, and the write it did not make.
4. CLI. When a patch review has at least two hunks, the development CLI lists them
   by number. It accepts `yes`, or `select` followed by hunk numbers. A selection
   sends `Narrow` with the exact hunk identities. Any other answer refuses the call.
   The derived write's review is computed from its own arguments. It shows only the
   selected hunks and says how many of the original's hunks they are.

This amends Decision 0111 in three places. There is no presentation extension. The
runtime proposes the derived call on the person's behalf instead of the host alone.
Drift at selection time refuses the original call instead of asking for a fresh
proposal inside the same run.

## Verification boundary

Tests cover the following:

- Contract tests for prior encodings, a narrowing round trip, and refusal of unknown
  fields and values.
- Coordinator tests for every refused selection shape.
- Engine loop tests for the derived request, its approval, denial and cancellation;
  replayed and unhonored selections; the tool ceiling; and the shell-only
  restriction.
- Host unit tests for deriving, binding, drift, tampered arguments and registration.
- CLI tests for parsing and for the derived review.

The Linux host boundary glue runs only on a native host, like the rest of that
module. The actual-process proof that human edits and rejected hunks survive is
AMR-04.2.3 and stays open.
