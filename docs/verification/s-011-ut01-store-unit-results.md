# S-011-UT01 Store Unit Results

**Status:** Pass at the deterministic unit boundary  
**Result version:** 1  
**Task:** `11.1.3.1`  
**Suite:** `agentmage-kernel-engine::operational_store`

This matrix maps every `S-011-UT01` requirement to current executable tests.
The complete focused suite contains 21 tests. Passing this unit gate does not
close the later secret-canary, seeded crash, live provider, cross-platform,
packaging, or release gates.

## Acceptance Matrix

| ID | Required class | Executable coverage | Verified outcome |
|---|---|---|---|
| `UT-01` | Schema constraints | `schema_constraints_and_atomic_rollback_reject_partial_authority` | Duplicate immutable identity, invalid head references, and induced checkpoint failure reject or roll back without a partial generation |
| `UT-02` | Foreign keys and relationships | `version_eighteen_schema_matches_fixture_snapshot_and_is_relational`; `retention_assignment_and_event_tampering_fail_closed` | Missing parents, invalid relationships, orphan retention, and event-chain drift fail closed |
| `UT-03` | One writer and exact runtime policy | `exclusive_writer_and_encrypted_backup_are_verified`; `required_writer_and_database_configuration_is_verified` | A second independent SQLCipher connection cannot own the canonical writer; changed WAL, locking, foreign-key, trusted-schema, secure-delete, temporary-store, synchronization, checkpoint, or timeout policy is rejected |
| `UT-04` | Retention transitions | `retention_holds_expiration_and_stale_revisions_are_atomic`; `retention_assignment_and_event_tampering_fail_closed` | User/legal holds, matching release, due expiry, stale revision denial, wrong-kind denial, orphan denial, and event tamper preserve one valid lifecycle state |
| `UT-05` | Migration versions | `version_one_upgrades_through_eighteen_with_exact_history`; `version_two_retention_rows_upgrade_to_three_with_initial_event` | Real encrypted v1 and v2 fixtures reach the current schema through exact ordered immutable history and deterministic retention events |
| `UT-06` | Migration rollback | `failed_version_two_migration_rolls_back_without_partial_schema`; `failed_version_three_migration_rolls_back_all_alterations` | Induced conflicts retain the complete prior schema, history, indexes, columns, rows, and `user_version` |
| `UT-07` | Invalid store and record states | `encrypted_store_requires_key_and_hides_sqlite_header`; `wrong_key_and_wrong_storage_class_fail_closed`; `future_schema_and_page_corruption_are_refused` | Missing/wrong keys, unsafe storage, future schema, malformed state, and page corruption do not become admitted authority |
| `UT-08` | Backup, restore, export, and erasure boundaries | Six focused backup/restore/canary/erasure/export tests | Occupied destinations and failed operations preserve existing objects; invocation-created failed candidates are removed; derivatives never become startup authority |

## No-Partial-State Assertions

1. Initial schema and each migration publish inside one SQLite transaction.
2. Failed v2 migration retains exact v1 schema and history.
3. Failed v3 migration retains exact v2 schema, including absence of every
   attempted column, index, and event table.
4. Failed authority publication rolls back metadata generation and checkpoint
   together.
5. Foreign-key and immutable-row conflicts leave no admitted head or orphan.
6. Stale or invalid retention transitions append no event and change no head.
7. Failed backup or restore removes only an invocation-created destination and
   never modifies an occupied object or canonical source.
8. Invalid JSON Lines publication changes no canonical SQLite state.

## Focused Suite Closure

The expected test set is exactly:

1. `encrypted_store_requires_key_and_hides_sqlite_header`
2. `encrypted_store_opens_through_exact_held_linux_directory_descriptor`
3. `only_exact_numeric_proc_self_fd_parent_is_a_held_descriptor_path`
4. `wrong_key_and_wrong_storage_class_fail_closed`
5. `exclusive_writer_and_encrypted_backup_are_verified`
6. `encrypted_backup_restores_only_to_a_verified_fresh_candidate`
7. `synthetic_canary_is_absent_from_encrypted_and_derived_artifacts`
8. `corrupted_or_wrongly_keyed_backup_leaves_no_restore_candidate`
9. `whole_store_cryptographic_erasure_consumes_key_scope_without_overwrite_claim`
10. `json_lines_export_is_deterministic_content_free_and_export_only`
11. `json_lines_export_rejects_occupied_or_ineligible_destinations_without_change`
12. `schema_constraints_and_atomic_rollback_reject_partial_authority`
13. `required_writer_and_database_configuration_is_verified`
14. `version_eighteen_schema_matches_fixture_snapshot_and_is_relational`
15. `version_one_upgrades_through_eighteen_with_exact_history`
16. `version_two_retention_rows_upgrade_to_three_with_initial_event`
17. `failed_version_three_migration_rolls_back_all_alterations`
18. `failed_version_two_migration_rolls_back_without_partial_schema`
19. `future_schema_and_page_corruption_are_refused`
20. `retention_holds_expiration_and_stale_revisions_are_atomic`
21. `retention_assignment_and_event_tampering_fail_closed`

## Deliberate Limits

- Writer contention uses independent SQLCipher connections in one process; it
  is not retained multiprocess contention evidence.
- Rollback uses deterministic database failures; it is not an operating-system
  process-kill or power-loss result.
- The suite does not satisfy the every-field secret-canary or at-least-100-seed
  crash campaigns in `11.1.3.2` and `11.1.3.3`.
- Live Secret Service substitution, macOS, Windows, packaging, and release
  acceptance remain separate tasks.
- Manual fuzzing is not part of this unit result and remains deferred.
