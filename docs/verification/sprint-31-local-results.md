# Sprint 31 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 31 |
| Local candidate/lifecycle/preview/retrieval contracts | Pass |
| Durable file writes | Absent |
| Encrypted portable export/import | Pass at pure capability boundary |
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
- The strict-local source audit finds no undeclared network path.

## Open Evidence

Sprint 30 remains blocked. Protected file-adapter writes, trusted key-store and entropy adapters,
interrupted-write recovery, backup restore, simultaneous-edit handling, installed migration
evidence, and independent review are absent.

Sprint 31 therefore remains blocked despite passing its locally executable policy, lifecycle,
preview, working-memory, and selective-loading scope. Its report is generated at
`artifacts/sprints/sprint-31/local-evidence-report.json` from a committed revision.
