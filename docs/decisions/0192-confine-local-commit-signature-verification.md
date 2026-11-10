# Decision 0192: Confine local-commit signature verification

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0191 |
| Scope | Signature verification in the existing dormant local-commit adapter |

Verify the already checked raw commit with a standalone offline GnuPG worker,
through the existing closed supervisor. Do not invoke Git's direct verification
subprocess. Retain the inspected binary public-key export, bound to the pinned
signer's public-key digest. Reconstruct the exact detached signature and signed
content for the selected object format, preserving approved message bytes.
Only sealed public key, signature and content descriptors enter the worker;
neither the repository nor the external private keyring is mounted.

Hold and revalidate the installed executable artifacts, including root-owned
ancestors, before launch and supervisor control. Use a distinct closed unit
prefix and exactly three payload projections. Preserve the original commit
session deadline, output limits, resource bounds, withheld socket-denying
seccomp policy and whole-unit cleanup. The worker gets a private temporary home,
no option files, no default keyring, no agent startup and no key retrieval.
There is no direct verification fallback.

Success requires complete output, successful supervision and process exit,
a dedicated complete machine-status stdout stream, consistent retained-stream
lengths and digests, diagnostic-only stderr, and exactly one
valid status record for the complete pinned signing fingerprint. Cancellation,
expiry, changed public material, malformed raw bytes, partial output or failed
verification stops before the task-branch update. Cleanup uncertainty remains
a failed effect, never a successful commit receipt.

Signed creation remains direct and dormant; its confinement and descendant
cleanup are still prerequisites for activation. Decision 0193 moves public
identity inspection to a separate offline worker without proving private-key
availability.
This does not introduce a product route, candidate authority, reviewer identity,
approval, commit publication or push permission. Disposable cryptographic tests
and synthetic supervisor results remain component evidence. Native artifact
admission, the complete workflow, durable recovery and model measurements remain
open. Neither T-303 nor T-306 closes on this increment.
