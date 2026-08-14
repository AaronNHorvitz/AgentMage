# Sprint 32 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 32 |
| Canonical conversation, search, resume, branch, compaction, and control contracts | Pass |
| Upstream Sprint 31 gate | Blocked |
| Independent Sprint 32 review | Absent |
| Sprint result | Blocked |

## Verified Locally

- Version-five SQLCipher schema stores sensitivity-labeled conversation metadata, immutable turns,
  attachment references, grants, receipts, checkpoints, citations, source hashes, tags, retention,
  and checked compactions through the kernel boundary.
- Exact text is rejected when persistence is disabled; ciphertext does not expose the plaintext
  canary or a normal SQLite header.
- Canonical JSON and normalized projections are hash-checked before search or history use.
- Local bounded date/filter/text search, archived exclusion, read-only timeline, and direct branch
  relationships are deterministic and network-free.
- Rename, pin, archive, tag, and retention changes use unchanged compare-and-swap previews.
- Leaf deletion displays exact counts, requires separately bound explicit approval, rechecks state,
  blocks child branches, and commits atomically.
- Latest and historical resume use the existing workspace, file, instruction, repository, citation,
  model, permission, and policy drift engine. Any material drift blocks branch creation.
- Exact-point branches retain an immutable parent-turn relationship and source-history digest while
  leaving the original transcript unchanged.
- Checked compaction rejects omitted citations, receipts, or source hashes and retains original
  turns unchanged.
- Encrypted backup and fresh-candidate restore preserve turns and checked compactions.
- The strict-local source audit finds no undeclared network path.

## Open Evidence

Sprint 31 remains blocked, so Sprint 32 cannot satisfy its dependency gate. Independent Sprint 32
review is also absent. Interface-specific rendering and private portable archive/evidence bundles
remain assigned to later sprints.

Sprint 32 therefore remains blocked despite passing its locally executable conversation-library
scope. Its retained report is generated at
`artifacts/sprints/sprint-32/local-evidence-report.json` from a committed revision.
