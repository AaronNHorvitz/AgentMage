# Sprint 11 Crash-Point and Secret-Canary Results

**Status:** Current deterministic partial result  
**Result version:** 1  
**Scope:** Sub-task `11.1.2.4` review artifact  
**Not sufficient for:** `S-011-ST01`, `S-011-RT01`, or Sprint 11 closure

This record consolidates the deterministic crash and canary checks already
implemented for the encrypted authority boundary. It does not replace the
later every-field canary campaign or the at-least-100-seed crash campaign.

## Crash-Point Matrix

Each row runs from a fresh encrypted schema-v3 store, injects one simulated
process stop after the named durable checkpoint, drops the original store, and
opens a new `DurableAuthorityRuntime` under the same key. Recovery must reach
one terminal transaction, retain exactly one receipt, and reject replay without
launching another driver.

| Point | Durable state before stop | Recovered outcome | Grant state | Driver launches before restart | Driver launches during recovery/replay |
|---|---|---|---|---:|---:|
| `CP-01` | Prepared stored | Failed | Issued | 0 | 0 |
| `CP-02` | Grant consumed | Failed | Consumed | 0 | 0 |
| `CP-03` | Attempt recorded | Failed | Consumed | 0 | 0 |
| `CP-04` | Launch boundary committed | Uncertain | Uncertain | 0 | 0 |
| `CP-05` | Worker returned | Uncertain | Uncertain | 1 | 0 |
| `CP-06` | Result reconciled | Succeeded | Consumed | 1 | 0 |

The test then creates a separately keyed encrypted backup, scans available
database, WAL, shared-memory, and backup bytes for the synthetic transaction
identity, reopens the backup, and verifies the same single terminal receipt and
outcome. These are deterministic in-process stop injections at six authority
checkpoints. They are not operating-system process kills and do not cover
migration, key retrieval, backup, restore, expiry, deletion, or every storage
transaction boundary.

## Secret-Canary Matrix

One synthetic canary is inserted directly into the session identifier, profile,
and canonical-record bytes of a schema-v3 test store. Direct insertion is
intentional: it tests encryption and derivative-output containment even when
the database legitimately contains the plaintext before encryption. It does
not prove that every production ingress passed the classification gate.

| Surface | Current check | Result |
|---|---|---|
| Main SQLCipher file | Scan bytes while live and after close | Canary absent |
| WAL sidecar | Scan when present while live and after close | Canary absent |
| Shared-memory sidecar | Scan when present | Canary absent |
| Separately keyed SQLCipher backup | Scan closed file | Canary absent |
| Derived JSON Lines export | Scan complete derivative | Canary absent |
| Backup and export receipts | Scan `Debug` representation | Canary absent |
| Store debug and stable error codes | Scan bounded strings | Canary absent |
| Backup reopen | Open under backup key and verify generation | Pass |
| JSON Lines authority | Never loaded or treated as startup authority | Pass |

The test retains no raw canary in an evidence artifact. Its result reports only
surface names, counts, booleans, and source identities.

## Related Deterministic Coverage

- Raw attachment, full tool output, environment material, prompt, and model
  response constructors are structurally ephemeral before persistence.
- Secret-like values in persist-marked metadata deny the complete candidate;
  values in omitted fields retain neither content nor content digest.
- Invalid operational-store key material never invokes the database callback.
- Missing backup keys create no destination, and corrupt or wrongly keyed
  backups leave no restore candidate.
- SQLCipher logging is disabled before keyed validation, while public store and
  Secret Service errors remain content-free.

## Explicitly Open Coverage

Sub-task `11.1.3.2` still must place distinct synthetic canaries in every field
of every real input family and inspect database pages, WAL, temporary files,
logs, backups, exports, model context, and crash output. That campaign must
distinguish policy-authorized encrypted persistence from every unauthorized
surface and retain a complete sanitization report.

Sub-task `11.1.3.3` still must run at least 100 seeded crash resumes before and
after each transaction, checkpoint, migration, key retrieval, backup, restore,
expiry, and deletion transition. It must include actual process interruption
where required and prove no repeated completed effect.

This artifact also makes no live Secret Service, cross-platform, packaging, or
release claim. Manual fuzzing remains separately deferred and is not executed
by this result workflow.
