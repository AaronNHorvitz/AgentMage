# Sprint 31 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 31 |
| Local candidate/lifecycle/preview/retrieval contracts | Pass |
| Approved durable memory topic-bundle writes | Pass |
| `WORKING.md` installed writes | Absent |
| Encrypted portable export/import | Pass at pure capability boundary |
| Installed bundle recovery and conflict preservation | Pass |
| Digest-bound cross-root encrypted export transfer | Pass |
| Gate-owned boundary review | Pass |
| Sprint result | Blocked |

## Verified Locally

- The closed taxonomy distinguishes working, episodic, semantic, procedural, and preference memory
  under exact workspace/project/conversation, source, sensitivity, confidence, and expiry fields.
- Candidate policy covers temporary context, durable fact, preference, procedure, episode,
  unresolved claim, contradiction, and prohibited content without automatic promotion.
- Secrets, restricted content, inferred-sensitive claims, source-free claims, low confidence, and
  contradictory current facts fail closed or remain visibly unresolved.
- Approve and reject decisions bind exact candidate and decision digests; post-review mutation is
  rejected.
- Atomic catalog transitions cover insert, edit-by-replacement, supersede, correct, confidence
  decay, hold, expiry, and deletion while preserving identity and history.
- `MEMORY.md`, per-topic files, and complete bounded `WORKING.md` are deterministic portable
  previews with fixed false write markers.
- Working compaction inherits evidence and scope, cannot raise confidence or capture secrets, and
  emits review candidates without creating durable memory or clearing temporary state.
- Selective loading enforces exact workspace/project/conversation boundaries, filters by type,
  tag, link, source, relevance, state, and budgets, and explains every inclusion.
- Versioned XChaCha20-Poly1305 export/import preserves the complete catalog, revision, stable
  identities, lifecycle state, links, evidence, and full catalog digest across fresh entropy.
- Wrong keys, ciphertext changes, truncation, format-version drift, zero key/entropy, machine-path
  evidence, restricted data, and malformed portable identities fail closed without a partial
  imported catalog.
- Keys are caller-owned and zeroized on drop, debug output is redacted, and ciphertext proposals
  and import receipts have fixed false filesystem-write markers.
- The host planner recomputes every bundle identity and emits only controlled filesystem drafts for
  private publication, exact last-good copies, complete restoration, simultaneous-edit conflict
  bundles, and digest-bound opaque encrypted exports. The existing kernel/platform transaction
  path retains approval, one-use authority, atomic execution, receipt, rollback, and crash recovery.
- The gate-owned review hashes memory policy, lifecycle, portable encryption, and the installed
  filesystem owner without making a human-review or release claim.

## Open Evidence

Sprint 30 remains blocked. Trusted key-store and entropy adapters remain outside this installed
file slice. Renewal of the legacy Sprint 31 aggregate is `blocked: host change required — run the
strict-local worker and source-policy renewal outside the restricted filesystem sandbox where
/usr/bin/systemd-run, /usr/bin/systemctl, /usr/bin/bwrap, /usr/bin/env, and /usr/bin/cat retain
root-owned identities, then run python3 scripts/sprint_31_evidence.py --write --source-revision
HEAD`; `substitution_set=empty`.

Sprint 31 therefore remains blocked despite passing its locally executable policy, lifecycle,
installed recovery, preview, working-memory, and selective-loading scope. Its legacy report is at
`artifacts/sprints/sprint-31/local-evidence-report.json` from a committed revision.
