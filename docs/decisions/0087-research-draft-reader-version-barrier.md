# Decision 0087: Research Draft Reader Version Barrier

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0042, 0054, 0081 and 0086; current owner restart |
| Scope | Retained research draft compatibility before integration |

## Reason

Decision 0086 introduced a reserved retained-draft format whose content requires
fresh canonical source checks. Earlier binaries accept arbitrary Report media
through generic artifact reads. A new media name alone therefore cannot prevent
an older reader from exposing a draft without those checks. The component remains
unactivated while this prerequisite is open.

## Decision

Advance the existing canonical store's compatibility version from 19 to 20 before
it can publish restricted drafts. Reuse its exclusive writer, migration transaction,
hash-bound migration history and early unsupported-version refusal. This epoch
records changed read semantics even though the relational tables do not change.
It creates no second store, schema family, permission, export or reverse migration.

Verify the complete version-19 migration history before advancing the epoch.
Publish the version-20 history entry and store version in the same transaction.
Preserve every earlier migration byte and fixture, all source identities and data.
A failed transition leaves version 19 and its records intact. A reader supporting
only version 19 must refuse version 20 before claiming a writer or interpreting
any retained record. Never rewrite a version marker to permit a downgrade.

Retain the current-reader draft restrictions and their source/event/lifecycle
checks. Successful version migration does not qualify native transport, activate
host/provider research, establish installed update or rollback support, or close
independent, human-only, model or release gates. This work exercises disposable
synthetic stores; it does not authorize migrating host user data.

## Verification

Exercise an actual encrypted version-19 fixture, exact table and data preservation,
history-prefix corruption, migration failure rollback, current reopen and older
reader refusal with encrypted byte preservation. Keep the version-18 upgrade and
failed version-19 migration cases. Update current-version fixture assertions and
evidence inputs additively; preserve earlier fixtures and migration hashes. Recheck
backup, restore, crash recovery and retained-report behavior through their existing
owners before any compatibility acceptance claim. Exact-current restore must refuse
a version-19 backup without changing it or creating a destination candidate;
automatic conversion of old backups is outside this change.
