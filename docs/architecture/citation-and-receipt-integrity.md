# Citation and Receipt Integrity

## Boundary

Sprint 21 reconciles already-held source identities and receipts. It does not open paths, read
files, contact services, launch workers, or store secrets. Platform and host layers must supply the
current file identity, authority receipts, and a separately brokered integrity key.

## Citation Resolution

A `SourceCitation` binds a stable citation ID, content-addressed `EvidenceReference`, canonical
workspace-relative path, complete content hash, byte length, platform object-identity hash,
revision, observation point, and either a half-open byte range or structured identity.

`resolve_citation` compares every observed field with one caller-supplied current identity:

| Result | Condition |
|---|---|
| `current` | Path, length, content, object, and revision identities are exactly equal. |
| `stale` | A current object exists but any identity differs, including rename or changed preimage. |
| `missing` | No current object exists at the exact observed source. |

The resolver never substitutes a renamed or replacement source. Invalid ranges, malformed hashes,
noncanonical paths, zero observation points, and mismatched evidence revisions fail before a
resolution is created.

## Answer Ledger

`EvidenceStateAssigner::finalize` produces an opaque validated assignment set. The answer-ledger
builder requires an exact one-to-one match between rendered material claim IDs and validated
assignments:

- Observed and Inferred entries require their exact citations to remain current.
- Derived entries require exact same-ledger Observed assignment IDs and a registered method.
- stale Unknown/Blocked entries require exact noncurrent citations;
- conflicting Unknown/Blocked entries require at least two exact declared evidence identities;
- omitted, duplicate, foreign, reordered-after-signing, fabricated, or unrelated evidence fails.

The safe projection contains only citation and evidence IDs, content hashes, freshness, and stable
reason codes. Claim-level audit rendering contains claim IDs, state labels, counts, and limitation
codes, never source bytes, claim prose, model output, credentials, or absolute paths. The complete
ledger is hash-bound and sealed as a non-authoritative Claim Record.

## Receipt Integrity

`TamperEvidentReceiptLedger` accepts one receipt per unique operation attempt in exact monotonic
sequence. Every receipt must have valid identities, operation digest, self-digest, and previous
receipt digest. Gaps, duplicates, removal, reorder, replacement, and mutation invalidate the chain.

`ReceiptIntegrityKey` admits one nonzero 256-bit secret, has redacted debug output, is not
serializable or cloneable, and zeroizes on drop. `seal_receipt_ledger` returns a separate
`ReceiptIntegrityAnchor` containing only count, head digest, and HMAC-SHA-256. The key and anchor
are not fields of the ledger. Verification compares the HMAC in constant time.

Production persistence of the ledger and separately keyed anchor remains a host/platform
integration requirement. This pure core does not claim encrypted-store, Secret Service, Keychain,
DPAPI, crash-recovery, or clock-anomaly evidence.

## Traceability

| Requirement | Implementation | Verification |
|---|---|---|
| `S-019-I06` | `SourceCitation`, `CitationFileIdentity`, `CitationSelector`, `resolve_citation` | Current, rename, revision, content, length, Unicode, missing, and invalid-range fixtures. |
| `S-019-I07` | `CitationFreshness`, hash-bound `CitationResolution` | Changed and missing sources remain visibly stale or missing. |
| `S-019-I08` | `AnswerClaimLedger`, safe projection, audit renderer, conflict checks | Four-state complete ledger and omitted, reordered, stale, fabricated, and conflict mutations. |
| `S-019-I09` | `TamperEvidentReceiptLedger`, external `ReceiptIntegrityAnchor` | Append, duplicate attempt, mutation, removal, reorder, wrong-key, and zero-key cases. |
| `AT-EVD-001` through `AT-EVD-003` | Sprint 20 state assignment plus Sprint 21 reconciliation | Focused Rust suite and source-bound local evidence report. |
