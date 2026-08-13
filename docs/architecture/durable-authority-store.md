# Durable Authority Store

## Status and Scope

This document describes the durable authority candidate governed by accepted
Decision 0016 and composed on Linux by accepted Decision 0017. Schema v1 covers
canonical grant, nonce, authority-transaction, receipt, and checkpoint state.
Schema v2 adds normalized structural tables for sessions, objectives, plans,
tasks, actions, evidence, decisions, files, and retention. Those v2 tables have
no public domain-write API yet and are not included in the authority-state
digest until their owning transaction boundary is implemented. This is not
evidence that the complete Sprint 11 data lifecycle, application host, Ubuntu
execution, macOS, Windows, or a supported product is implemented.

## Ownership

```mermaid
flowchart LR
    P["Platform key provider"] -->|"bounded 256-bit callback"| S["OperationalStore"]
    L["Strict-local storage observation"] --> S
    S --> D[("SQLCipher database")]
    R["DurableAuthorityRuntime"] --> S
    R --> G["Grant state machine cache"]
    R --> T["Transaction state machine cache"]
    T -->|"opaque one-use permit"| E["Effect driver"]
    E -->|"bounded result"| T
```

`DurableAuthorityRuntime` is the sole public effect-launch boundary. The grant
issuer and transaction coordinator hold deterministic candidate state but
cannot publicly launch an effect. The store is canonical; caches are rebuilt
only from validated rows after every process restart.

On Linux, `LinuxOperationalStoreKeyProvider` looks up one fixed
`operational-store-key-v1` Secret Service item for one profile. It never exposes
a general lookup, list, mutation, or returned-key API. The Phase 8 aggregate
opens the fixed database through the exact
`/proc/self/fd/<held-root>/authority.db` shape after final-object creation or
verification rejects symbolic links. Ordinary non-descriptor paths retain
SQLite no-follow.

The production `open_linux_authority` composition accepts that concrete
fixed-purpose provider, not an arbitrary implementation of the kernel's
testable key callback contract. Missing, malformed, locked, substituted, or
otherwise unavailable Secret Service material fails before the database
callback runs and before a primary or backup file is created. Test-only builds
retain an explicitly named synthetic-provider constructor, which cannot enter a
normal build.

Initial key provisioning is a separate explicit operation that requires the
verified aggregate, a current-user owner-only strict-local root, and a
single-writer lifecycle lock. It refuses an existing key and refuses an
existing authority database without its key, obtains 256 random bits from the
operating system, sends encoded material only through Secret Service standard
input, and verifies exact lookup before success. Normal startup never provisions,
repairs, clears, or rotates a key. Rotation is explicitly unavailable until an
interruption-safe dual-key protocol exists.

## Schema

```mermaid
erDiagram
    SCHEMA_HISTORY {
        integer version PK
        text migration_sha256
    }
    STORE_METADATA {
        integer singleton PK
        integer generation
        text state_sha256
    }
    GRANT_IDENTITIES {
        text grant_id PK
        text nonce UK
    }
    GRANT_REVISIONS {
        text grant_id FK
        integer revision PK
        text record_sha256
        blob record_json
    }
    GRANT_HEADS {
        text grant_id PK
        integer current_revision FK
    }
    TRANSACTION_IDENTITIES {
        text transaction_id PK
        text attempt_id UK
    }
    TRANSACTION_REVISIONS {
        text transaction_id FK
        integer revision PK
        text state
        text record_sha256
        blob record_json
    }
    TRANSACTION_HEADS {
        text transaction_id PK
        integer current_revision FK
    }
    RECEIPTS {
        integer sequence PK
        text receipt_id UK
        text transaction_id FK
        text receipt_sha256 UK
        text previous_receipt_sha256
        blob receipt_json
    }
    CHECKPOINTS {
        integer generation PK
        text state_sha256
    }
    SESSIONS {
        text session_id PK
        text profile_id
        text status
        integer created_at_epoch_ms
        integer updated_at_epoch_ms
        text record_sha256
        blob record_json
    }
    OBJECTIVES {
        text objective_id PK
        text session_id FK
        integer ordinal
        text status
        text record_sha256
        blob record_json
    }
    PLANS {
        text plan_id PK
        text objective_id FK
        integer revision
        text status
        text record_sha256
        blob record_json
    }
    TASKS {
        text task_id PK
        text plan_id FK
        text parent_task_id FK
        integer ordinal
        text status
        text record_sha256
        blob record_json
    }
    ACTIONS {
        text action_id PK
        text task_id FK
        text transaction_id FK
        integer ordinal
        text operation_class
        text status
        text record_sha256
        blob record_json
    }
    EVIDENCE {
        text evidence_id PK
        text task_id FK
        text action_id FK
        integer ordinal
        text evidence_kind
        text classification
        text record_sha256
        blob record_json
    }
    DECISIONS {
        text decision_id PK
        text task_id FK
        integer ordinal
        text disposition
        text record_sha256
        blob record_json
    }
    FILES {
        text file_id PK
        text session_id FK
        text workspace_id
        text object_identity_sha256
        text relative_path_sha256
        text content_sha256
        text status
        text record_sha256
        blob record_json
    }
    RETENTION {
        text retention_id PK
        text record_family
        text record_id
        text sensitivity
        text disposition
        integer expires_at_epoch_ms
        integer legal_hold
        text policy_sha256
    }

    GRANT_IDENTITIES ||--|{ GRANT_REVISIONS : retains
    GRANT_REVISIONS ||--|| GRANT_HEADS : selects
    TRANSACTION_IDENTITIES ||--|{ TRANSACTION_REVISIONS : retains
    TRANSACTION_REVISIONS ||--|| TRANSACTION_HEADS : selects
    TRANSACTION_IDENTITIES ||--o| RECEIPTS : closes
    SESSIONS ||--o{ OBJECTIVES : owns
    OBJECTIVES ||--o{ PLANS : revises
    PLANS ||--o{ TASKS : contains
    TASKS ||--o{ TASKS : parents
    TASKS ||--o{ ACTIONS : attempts
    TASKS ||--o{ EVIDENCE : grounds
    ACTIONS ||--o{ EVIDENCE : produces
    TASKS ||--o{ DECISIONS : records
    SESSIONS ||--o{ FILES : observes
    TRANSACTION_IDENTITIES ||--o{ ACTIONS : authorizes
```

Revision rows are immutable. A duplicate insert must match the retained digest
and canonical bytes exactly. Head rows may advance only in the same transaction
as their newly inserted revisions. Receipt sequence and previous-receipt digest
form a validated hash chain.

Migration v2 is an additive transaction over an admitted v1 database. The
immutable v1 migration text and digest remain unchanged. A v2 conflict rolls
back its history row, `user_version`, and every table created by that
migration; reopening never treats a partial schema as current. The v2 domain
tables use closed status values, fixed SHA-256 shapes, unique per-parent
ordinals or revisions, and foreign keys that reject orphaned or cross-task
records. Retention uses a closed record-family, sensitivity, disposition, hold,
and expiration shape, but lifecycle transitions and cryptographic erasure are
later tasks.

## Pre-Persistence Gate

The kernel classifies every candidate into one closed record family,
sensitivity, and retention intent before it can produce a sealed
`PreparedPersistence`. Each bounded, uniquely named field must declare
`persist`, `digest_only`, or `ephemeral` handling. Persisted values must be
UTF-8; digest-only values retain only byte count and SHA-256; ephemeral values
retain neither content nor digest. Known credential field names, private-key
envelopes, bearer values, provider-token prefixes, cloud access-key identities,
and URI user information are detected deterministically. A finding in a
persist-marked field denies the record; a finding in an omitted field removes
both value and digest.

Every valid candidate produces a content-free, policy-bound receipt. Only an
admitted receipt selects the SQLCipher operational store and carries a bounded
expiration assignment. Ephemeral candidates select no storage. Restricted
persistence fails closed until a separately reviewed restricted-data policy is
implemented. The detector covers declared signatures and is not represented as
a complete secret-discovery system; later canary and integration gates remain
required before the story closes.

## Publication Boundaries

| Boundary | Atomic publication before continuing | Recovery result |
|---|---|---|
| Prepared | Initial transaction identity and revision | Failed; grant remains issued |
| Grant consumed | Consumed grant revision and `grant_consumed` transaction revision | Failed; grant remains consumed |
| Attempt recorded | Non-replayable attempt revision | Failed; grant remains consumed |
| Launch committed | Launch commitment before driver invocation | Uncertain; no driver retry |
| Worker returned | No new state until bounded reconciliation is complete | Prior launch commitment becomes uncertain |
| Result reconciled | Result digest, outcome, and uncertainty classification | Publish one terminal receipt without retry |
| Terminal | Terminal revision, grant uncertainty when applicable, and receipt | Return the retained receipt idempotently |

Each publication uses an immediate SQLite transaction, expected-generation
compare-and-swap, deterministic full-authority digest, and checkpoint. Any
publication failure poisons the process-local runtime. Reopen and recovery are
required before another effect can be considered.

## Database Configuration

- bundled SQLCipher through pinned `rusqlite` 0.40.2, linked on Linux to the
  distribution OpenSSL 3 `libcrypto` selected by the support matrix;
- exact 32-byte key and no plaintext fallback;
- SQLCipher logging disabled before key validation;
- cipher, quick, and foreign-key integrity checks at open;
- exclusive single writer with zero busy timeout;
- WAL, full synchronization, secure deletion, trusted schema disabled, and
  memory-only temporary storage;
- mode `0600` regular database file with symlinks rejected; and
- separate encryption key and full verification for every online backup.

The canonical connection acquires and proves its exclusive writer lock and WAL
mode before inspecting or applying migrations. Startup then re-reads the exact
foreign-key, trusted-schema, secure-delete, memory-temporary-store, full-sync,
WAL-autocheckpoint, zero-busy-timeout, locking, and journal settings before any
canonical row is admitted. A second process can therefore fail during keyed
open or explicit lock acquisition, but it cannot read, migrate, or publish
state. Publication uses an immediate transaction; metadata generation update
and checkpoint insertion either commit together or both roll back.

## Recovery Invariants

1. A consumed nonce, grant, or attempt remains non-replayable after restart.
2. Recovery never invokes an effect driver.
3. A possible effect without a verified result becomes `uncertain`.
4. A complete verified result without a receipt produces exactly one receipt.
5. Terminal recovery is idempotent.
6. Missing key, wrong key, future schema, record conflict, page corruption,
   foreign-key failure, concurrent writer, or state-digest mismatch fails
   closed with a content-free error.

## Evidence Boundary

Current tests inspect encrypted database, WAL, shared-memory, and backup
artifacts for plaintext canaries; force a rollback; corrupt an encrypted page;
exercise wrong and missing keys; reject ineligible storage; deny a second
writer; migrate a real encrypted v1 database to v2; reject a partial v2 schema;
exercise normalized relationship constraints; and restart from every authority
transition. Linux tests additionally
exercise private-root owner/mode drift, unsafe authority-state objects, and
exclusive lifecycle locking. Live Secret Service tests remain explicitly
environment-dependent. Retained historical story artifacts are not regenerated
or represented as current Phase 8 evidence.
