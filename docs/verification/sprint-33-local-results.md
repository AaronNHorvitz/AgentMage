# Sprint 33 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 33 |
| Encrypted archive lifecycle | Pass |
| Exact-preview redacted evidence bundle | Pass |
| Complete `S-027-RT01` crash and simultaneous-access matrix | Incomplete |
| Upstream Sprint 32 gate | Blocked |
| Independent Sprint 33 review | Absent |
| Sprint result | Blocked |

## Verified Locally

- Archive creation requires an exact reviewed conversation inventory and writes a separately keyed
  SQLCipher snapshot to admitted strict-local private storage.
- Archive manifests bind conversation, turn, and compaction counts; source inventory; encrypted
  bytes; schema; generation; retention; and manifest revision.
- Inspection rejects wrong keys, changed ciphertext, non-private files, and nonlocal storage before
  reporting a verified result.
- Restore produces a fresh separately keyed candidate, verifies the complete canonical inventory,
  and never replaces the live store.
- Retention changes use unchanged compare-and-swap previews. Holds and future expiry block deletion;
  deletion requires explicit approval bound to an unchanged file and preview.
- Evidence bundles derive citations, receipts, and source hashes only from selected canonical turns.
- Exact UTF-8 source ranges require explicit approval and relevance. System turns, restricted
  inclusion, unsupported citations, unapproved text, unrelated text, secret signatures, stale
  history, expired review, and nonlocal destinations fail closed.
- Publication creates one private local file containing exactly the reviewed JSON bytes. It cannot
  overwrite a destination, import operational state, execute content, or deliver externally.
- The full kernel suite, clippy, contract tests, product gate, documentation gate, strict-local
  audit, dependency rules, and supply-chain checks are part of the retained evidence command set.

## Open Evidence

Sprint 32 remains blocked by its recorded upstream and independent-review conditions. Independent
Sprint 33 review is absent. In addition, `S-027-RT01` requires a dedicated crash and concurrent
access campaign at every named archive, branch, export, deletion, and restore phase. Existing
transaction, SQLCipher backup/restore, corruption, create-new publication, and deletion tests are
useful but do not substitute for that complete matrix.

Sprint 33 therefore remains blocked. Its retained report is generated at
`artifacts/sprints/sprint-33/local-evidence-report.json` from a committed revision.
