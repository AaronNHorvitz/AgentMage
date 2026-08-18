# Citation and Receipt Integrity

## Boundary

The kernel reconciliation core remains authority-free: it does not open paths, contact services,
launch workers, or store secrets. The Linux host composes that core with the verified platform path
adapter, and the durable authority composes it with encrypted local persistence. The integrity key
enters only through a separately brokered caller boundary and is never stored with the ledger or
anchor.

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

On Linux, `resolve_linux_citation` resolves the citation's exact canonical path through the
continuously held workspace descriptor. Only `path.adapter.not_found` becomes Missing; links,
mount drift, identity races, permission failures, unsafe object kinds, and resource-limit failures
remain hard errors. `verify_held_citation_bytes` recomputes byte length and complete SHA-256 from
the same held bytes and returns true only for a Current resolution. Persisted or caller-retained
identity data therefore cannot promote a changed preimage.

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

`compose_runtime_answer_claim_ledger` revalidates the exact successful runtime output and its
kernel-created Inferred assignment, then requires one Current citation resolution for every
runtime evidence reference. The encrypted operational store persists the complete verified ledger
by its self-digest and re-verifies it during reads and every restart.

## Receipt Integrity

`TamperEvidentReceiptLedger` accepts one receipt per unique operation attempt in exact monotonic
sequence. Every receipt must have valid identities, operation digest, self-digest, and previous
receipt digest. Gaps, duplicates, removal, reorder, replacement, and mutation invalidate the chain.

`ReceiptIntegrityKey` admits one nonzero 256-bit secret, has redacted debug output, is not
serializable or cloneable, and zeroizes on drop. `seal_receipt_ledger` returns a separate
`ReceiptIntegrityAnchor` containing only count, head digest, and HMAC-SHA-256. The key and anchor
are not fields of the ledger. Verification compares the HMAC in constant time.

SQLCipher schema 9 stores the receipt sequence and external anchor history in separate normalized
tables. Each anchor record also forms an unkeyed structural hash chain, while its HMAC remains
verifiable only with the external key. The durable authority can checkpoint only its own canonical
receipt sequence. Restart verifies ledger and anchor structure before authority becomes usable;
keyed verification rejects wrong keys, and wall-clock regression behind the latest anchor fails
with `evidence.store.clock_regression` without committing another record.

This evidence does not claim that an installed Secret Service, Keychain, or DPAPI broker supplied
the distinct anchor key, nor native macOS or Windows execution, independent review, release
approval, or physical-media guarantees.

## Traceability

| Requirement | Implementation | Verification |
|---|---|---|
| `S-019-I06` | `SourceCitation`, `resolve_citation`, `resolve_linux_citation` | Current, rename, revision, content, length, Unicode, missing, held-file, and unsafe-link fixtures. |
| `S-019-I07` | `CitationFreshness`, hash-bound `CitationResolution` | Changed and missing sources remain visibly stale or missing. |
| `S-019-I08` | `AnswerClaimLedger`, runtime composer, encrypted evidence store | Four-state ledger, runtime-output binding, restart, omitted, reordered, stale, fabricated, and conflict mutations. |
| `S-019-I09` | Receipt ledger, external anchor, SQLCipher schema 9 | Authority-owned append, restart, clock regression, mutation, removal, reorder, wrong-key, and zero-key cases. |
| `AT-EVD-001` through `AT-EVD-003` | Sprint 20 state assignment plus Sprint 21 reconciliation | Focused Rust suite and source-bound local evidence report. |
