# Decision 0196: Confined signed-commit storage

| Field | Value |
| --- | --- |
| Status | Dormant implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0191, 0192 and 0195 |
| Scope | Verified object storage and composition in the existing dormant executor |

Repair medium M-1 from the independent review of the immutable Decision 0195
implementation in this same batch. The agent's unsigned RSA value may have
zero leading octets within its bounded expression length. Remove that padding
before forming the minimal MPI, still refusing an empty or all-zero value,
ambiguous length, unsupported algorithm or trailing data. Deterministic padded
forms must produce the same packet, and padded real-signature values must verify.
Do not retry probabilistically until a parser happens to accept a signature.

Also resolve that review's low L-3 at the object boundary: refuse angle brackets,
leading/trailing name whitespace and whitespace in email identities before
signing. Preserve approved bytes for accepted identities rather than normalize.

Replace direct signed creation in the dormant executor with the socket-free
signer, RSA packet assembler and public-only verifier. Keep direct signed
creation only in disposable test fixtures. Do not add a product route or relax
native artifact admission. Unsupported keys fail without a direct fallback.

Return an immutable, process-local verified commit from the assembler only
after independent cryptographic verification. Bind it to the complete verified
plan and compute the Git commit object identifier locally, including the exact
object framing, for the selected object format. No serialization, public
constructor or caller-supplied bytes can restore this internal value. It is
not a kernel approval, reviewer verdict or permission to store an object.

The closed Git session must observe the expected task-branch parent, write
only those sealed verified bytes, read the expected object back byte for byte,
and then perform the original exact task-branch compare-and-swap. Storage uses
`hash-object -w -t commit --no-filters --stdin`; require the exact independently
computed identifier and one newline. Refuse skipped stages, another plan,
altered readback and inconsistent or incomplete process results. Every failure
ends the session without retry. Preserve cancellation, the existing single
session deadline, held roots, offline sandbox, bounded output and whole-unit
cleanup. Do not add a keyring, credential, socket or network projection to Git.

The original kernel launch permit, exact preflight preservation check,
post-storage public-only verification, compare-and-swap and final preservation
reconciliation stay required. Storage may leave an unreachable object after a
failure; do not claim rollback or no effect. Report storage/read/ref failure as
uncertain and never fabricate a successful preservation receipt.

Native artifact admission and the complete approved workflow remain unproven.
Disposable real signatures and Git storage/readback are component evidence;
synthetic supervisor outcomes establish control flow only. Original reviewer
and artifact provenance, product approval composition, durable recovery, model
measurements and approved push remain open. T-303/T-306 stay open with no new
row or sub-row. No release or owner readiness is asserted.
