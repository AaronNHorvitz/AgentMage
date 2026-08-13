# Data Classification and Retention Decision Table

**Status:** Implemented kernel boundary with explicitly listed integration gaps  
**Contract version:** 1  
**Canonical store schema:** 3

This table consolidates the implemented pre-persistence and lifecycle decisions.
It is descriptive evidence, not configuration or authority. If this document
differs from the compiled kernel, the kernel and its tests govern and this
artifact is stale.

## Classification Decisions

| ID | Candidate condition | Field handling | Current decision | Durable bytes | Encryption | Receipt |
|---|---|---|---|---|---|---|
| `DC-01` | Public, internal, or private; valid minimized metadata; no persisted secret | `persist` | Admit | Validated UTF-8 value | SQLCipher operational store | Content-free, policy-bound |
| `DC-02` | Public, internal, or private; valid minimized metadata | `digest_only` | Admit | Byte count and SHA-256 only | SQLCipher operational store | Content-free, policy-bound |
| `DC-03` | Any sensitivity | `ephemeral` | Omit field | None, including no value digest | None for omitted field | Omission count only |
| `DC-04` | Raw attachment, full tool output, environment material, prompt, or model response | Structurally forced `ephemeral` | Omit field; raw-only candidate remains ephemeral | None, including no value digest | None for raw-only candidate | Shape only; independent of raw bytes |
| `DC-05` | Secret signature in a `persist` field | `persist` | Deny complete candidate | None | None | Finding class only; no value or value digest |
| `DC-06` | Secret signature in an omitted field | `digest_only` or `ephemeral` | Omit secret field | None, including no value digest | Depends on remaining admitted metadata | Finding class and omission count only |
| `DC-07` | Restricted sensitivity | Any | Deny complete candidate | None | None | `denied_restricted` |
| `DC-08` | Invalid identity, duplicate field, invalid UTF-8 persisted value, resource excess, or clock overflow | Any | Fail before persistence decision | None | None | Stable content-free error only |

The deterministic scanner covers credential-like field names, private-key
envelopes, bearer credentials, known provider-token prefixes, cloud access-key
identities, and URI user information. It is not an exhaustive secret-discovery
system. Restricted persistence remains unavailable even though the PRD describes
a future explicitly confirmed, maximum-seven-day path.

## Retention Assignment

| ID | Intent or family | Implemented assignment | Current bound | Initial hold | Expiration action |
|---|---|---|---|---|---|
| `RT-01` | `ephemeral` or raw-only candidate | No prepared persistence | Process/turn memory only | Not applicable | No durable record exists |
| `RT-02` | `session`, non-receipt family | `occurred_at + sessions_days` | Configurable `0..=3650` days | None | Due unheld row becomes `expired` atomically |
| `RT-03` | `retained`, non-receipt family | `occurred_at + sessions_days` | Configurable `0..=3650` days | None | Due unheld row becomes `expired` atomically |
| `RT-04` | `session` or `retained`, receipt family | `occurred_at + receipts_days` | Configurable `1..=3650` days | None | Due unheld row becomes `expired` atomically |
| `RT-05` | PRD product defaults | Grants/receipts 90 days; sessions 30 days; excerpts 7 days; metrics 30 days | Design target | None | Application-host wiring remains open |

Every admitted assignment is bound to the policy SHA-256 and trusted occurrence
time. Arithmetic overflow fails before a prepared record exists. The current
kernel supports bounded policy values; no application host yet proves that the
PRD default profile is selected in a live session.

## Lifecycle Transitions

| ID | Current state | Request | Required conditions | Next state | Failure behavior |
|---|---|---|---|---|---|
| `LC-01` | Unassigned existing canonical record | Assign | Valid family/record identity, sensitivity, policy hash, trusted time, non-past expiry | `session` or `retained`, revision 1 | No row or event |
| `LC-02` | `session` or `retained`, unheld | Apply user hold | Exact current revision and nondecreasing time | `held/user`; preserve prior disposition | No change on stale or invalid request |
| `LC-03` | `session` or `retained`, unheld | Apply legal hold | Exact current revision and nondecreasing time | `held/legal`; preserve prior disposition | No change on stale or invalid request |
| `LC-04` | `held/user` or `held/legal` | Release matching hold | Exact current revision, kind, and nondecreasing time | Restore prior `session` or `retained` state | Wrong kind or stale revision changes nothing |
| `LC-05` | Due `session` or `retained`, unheld | Expire scan | Expiry at or before trusted scan time | `expired` | Held and not-due rows remain unchanged |
| `LC-06` | Any retained lifecycle | Startup verification | Complete contiguous event chain and current-state hash | Admit current state | Gap, mutation, stale head, or inconsistent hold rejects store |

Each successful transition appends one event whose SHA-256 binds retention ID,
revision, event kind, trusted time, previous event hash, and complete resulting
state hash. Selected due rows transition in one immediate transaction.

## Export, Backup, and Erasure

| ID | Operation | Data boundary | Authority consequence | Current limitation |
|---|---|---|---|---|
| `DE-01` | Derived JSON Lines export | Versioned header plus family, hashed identity, revision, and retained hash only | Never read as startup authority; deletion or tampering changes no SQLite state | Content-free audit view, not full portability archive |
| `BK-01` | Online backup | Separately keyed, fully verified SQLCipher database | Backup is not live authority | Clean-device continuity and atomic live swap remain Sprint 161 work |
| `RS-01` | Restore | Fully verified, separately keyed fresh candidate only | Does not overwrite or swap canonical state | User-confirmed atomic selection and rollback remain Sprint 161 work |
| `ER-01` | Cryptographic erasure | Complete operational-store key scope | Store closes before key destruction and verified lookup absence | Not per-record; separately keyed backups require their own erasure |
| `ER-02` | Ciphertext cleanup | Database, WAL, and shared-memory files after key absence | No authority remains under the destroyed key | No physical-overwrite claim on SSD or copy-on-write media |

## Open Verification Work

- Typed production writes and restart reconstruction for every normalized domain
  table are not complete.
- Real input adapters and the application host do not yet connect every data
  family to this gate.
- Restricted-data confirmation and its seven-day maximum are not implemented.
- Full secret-canary, crash-point, live key-service, cross-platform, packaging,
  and release campaigns remain open under later Sprint 11 tasks and gates.
