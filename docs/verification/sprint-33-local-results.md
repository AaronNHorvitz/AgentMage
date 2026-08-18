# Sprint 33 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 33 |
| Encrypted archive lifecycle | Pass |
| Exact-preview redacted evidence bundle | Pass |
| Complete `S-027-RT01` crash and simultaneous-access matrix | Pass |
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
- `S-027-UT01` proves bounded deterministic search over empty and 1,024-record archives, exact
  metadata/date/project/evidence/ancestry filters, resolvable matching turn identities, and
  corruption refusal.
- `S-027-UT02` branches from system, user, assistant, and tool turns while preserving the complete
  parent history and giving every child an independent evidence-free continuation.
- `S-027-ST01` proves exact inclusion, secret and copyright redaction, restricted/system denial,
  unrelated private-data omission, stale-review refusal, and hidden identity/hash exclusion.
- `S-027-RT01` executes ten before/after subprocess stop cases across archive, branch, export,
  deletion, and restore; every reopened state is complete, ordered, and free of replay or orphaned
  accepted content. A second writer is refused while canonical state is held.

## Open Evidence

Sprint 32 remains blocked by its recorded upstream and independent-review conditions. Independent
Sprint 33 review is absent. Native shell integration has not yet proved that every shell reads and
writes conversation state only through the kernel. The complete local source-level search, branch,
disclosure, crash, and concurrent-access matrices do not substitute for those two remaining gates.

Sprint 33 therefore remains blocked. Its retained report is generated at
`artifacts/sprints/sprint-33/local-evidence-report.json` from a committed revision.
