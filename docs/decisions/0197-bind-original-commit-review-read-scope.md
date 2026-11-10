# Decision 0197: Bind original commit review to selected read roots

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0183, 0186 and 0187 |
| Scope | Read selection at the existing live-owner candidate-review boundary |

Expose original write projection and candidate planning through a borrowed review
view. The trusted caller first obtains consent to inspect the complete original
repository and owned worktree, including metadata and offline Git reads. Require
both root identities shown in the existing inspection response. Compare them
with the retained owner before inspecting held roots, source or Git metadata.
A mismatch refuses even when a held root is unavailable. Path digests correlate
selection; they do not authenticate consent or supply a kernel grant.

Keep the original unscoped operations internal to the repository owner. Only its
checked constructor can issue the public view. Do not serialize, clone, restore
or adopt a view from a packet, path or receipt. Borrow the original owner so its
worktree cannot be replaced or mutated through that owner while the view is in
use. Retain all existing per-use root, source, original-write, validation, base
preimage and Git drift checks. External filesystem changes are still possible;
this is bounded observation, not an atomic lease.

The view returns only the existing authority-free change set or candidate plan.
It does not authenticate caller findings, independent review or artifact scanner
assertions. It cannot execute a candidate build, approve a commit or publish a
branch. Native admission, complete product approval composition, durable
recovery, model measurements and approved push remain open. No HTTP, CLI or MCP
route or permission relaxation is added; T-303/T-306 remain open without new rows.
