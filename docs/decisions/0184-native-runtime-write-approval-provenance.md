# Decision 0184: Native runtime write approval provenance

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decision 0183 |
| Scope | Original existing-file approvals in the Linux runtime writer |

Carry a borrowed original approval in the existing opaque kernel write
authorization. It becomes available to a driver only after exact preapply checks,
one-use grant consumption and the mandatory checkpoint succeed. This record
confers no additional effect, grant or replay authority. Callers cannot construct
the authorization from an approval receipt.

The owned native runtime writer retains the approval and shadow change set from
that authorization before dispatch. After the existing terminal publication,
completion must correlate the actual transaction identity, consumed grant,
original issued revision and complete verified application chain. Only then may
it retain the same private write proof used by prepared edits. Accept actual
runtime transaction identifiers rather than assuming the prepared-edit naming
convention. Public receipt/result arguments cannot replace the original approval.

Retire previous proof before dispatch, as before. Retain runtime proof only when
this transaction covers every transferable existing-file edit from its first
preimage to current postimage. Incomplete cumulative retries and new-file lineage
remain unsupported for candidate review; no aggregate approval is invented.
Unpublished, failed, uncertain or mismatched completion retains no new proof.
Current source and held-root checks remain required at review projection.

This connects original runtime write provenance to the existing internal review
path. Reviewer and artifact provenance, candidate execution, separately approved
Git operations, native/model measurement and durable recovery remain open.
No new route, command, model request, trust exception or acceptance is added.
