use agentmage_kernel_engine::write_recovery::{
    StagingInventoryItem, StagingObjectState, WriteAwareCheckpoint, WriteAwareCheckpointInput,
    WriteBoundaryField, WriteCheckpointPhase, WriteCleanupState, WriteFieldSensitivity,
    WritePrivacyBoundary, WriteRecoveryInstruction, WriteRecoveryObservation,
    build_write_checkpoint, diagnose_staging_inventory, plan_write_recovery,
    sanitize_write_boundary, verify_write_checkpoint_chain,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn hash(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn checkpoint_input(phase: WriteCheckpointPhase, sequence: usize) -> WriteAwareCheckpointInput {
    let consumed = phase != WriteCheckpointPhase::BeforeTransaction;
    let canonical = matches!(
        phase,
        WriteCheckpointPhase::CanonicalVerified
            | WriteCheckpointPhase::IndexUpdating
            | WriteCheckpointPhase::IndexVerified
            | WriteCheckpointPhase::ReceiptPersisting
            | WriteCheckpointPhase::ReceiptPersisted
            | WriteCheckpointPhase::CleanupPending
            | WriteCheckpointPhase::Complete
    );
    let receipt = matches!(
        phase,
        WriteCheckpointPhase::ReceiptPersisted
            | WriteCheckpointPhase::FailedNoChange
            | WriteCheckpointPhase::Restored
            | WriteCheckpointPhase::CleanupPending
            | WriteCheckpointPhase::Complete
    );
    let has_receipt = canonical || receipt;
    let has_index = matches!(
        phase,
        WriteCheckpointPhase::IndexUpdating
            | WriteCheckpointPhase::IndexVerified
            | WriteCheckpointPhase::ReceiptPersisting
            | WriteCheckpointPhase::ReceiptPersisted
            | WriteCheckpointPhase::CleanupPending
            | WriteCheckpointPhase::Complete
    );
    let index_verified = matches!(
        phase,
        WriteCheckpointPhase::IndexVerified
            | WriteCheckpointPhase::ReceiptPersisting
            | WriteCheckpointPhase::ReceiptPersisted
            | WriteCheckpointPhase::CleanupPending
            | WriteCheckpointPhase::Complete
    );
    let cleanup_pending = phase == WriteCheckpointPhase::CleanupPending;
    WriteAwareCheckpointInput {
        checkpoint_id: format!("checkpoint-{sequence}"),
        transaction_id: "transaction-recovery-matrix".to_owned(),
        action_id: "action-recovery-matrix".to_owned(),
        phase,
        consumed_grant_id: consumed.then(|| "grant-recovery-matrix".to_owned()),
        file_receipt_head_sha256: has_receipt.then(|| hash('a')),
        file_receipt_count: u32::from(has_receipt),
        evidence_set_sha256: hash('b'),
        index_update_sha256: has_index.then(|| hash('c')),
        next_session_checkpoint_sha256: hash('d'),
        secret_scan_receipt_sha256: hash('e'),
        staging_inventory_sha256: hash('f'),
        staging_item_count: u32::from(cleanup_pending),
        retention_expires_at_epoch_ms: if cleanup_pending { 20_000 } else { 0 },
        canonical_postimages_verified: canonical,
        receipt_chain_verified: receipt,
        index_verified,
        rollback_verified: phase == WriteCheckpointPhase::Restored,
        cleanup_state: if cleanup_pending {
            WriteCleanupState::Pending
        } else if phase == WriteCheckpointPhase::Complete {
            WriteCleanupState::Clean
        } else {
            WriteCleanupState::NotRequired
        },
        failure_code: matches!(
            phase,
            WriteCheckpointPhase::FailedNoChange
                | WriteCheckpointPhase::Restored
                | WriteCheckpointPhase::Uncertain
        )
        .then(|| "write.fixture.failure".to_owned()),
        occurred_at_epoch_ms: 1_000 + sequence as u64,
    }
}

fn build_chain(phases: &[WriteCheckpointPhase]) -> Vec<WriteAwareCheckpoint> {
    build_chain_named(
        phases,
        "transaction-recovery-matrix",
        "action-recovery-matrix",
    )
}

fn build_chain_named(
    phases: &[WriteCheckpointPhase],
    transaction_id: &str,
    action_id: &str,
) -> Vec<WriteAwareCheckpoint> {
    let mut chain = Vec::new();
    for (sequence, phase) in phases.iter().copied().enumerate() {
        let mut input = checkpoint_input(phase, sequence);
        input.transaction_id = transaction_id.to_owned();
        input.action_id = action_id.to_owned();
        let checkpoint =
            build_write_checkpoint(input, chain.last()).expect("fixture transition must be valid");
        chain.push(checkpoint);
    }
    verify_write_checkpoint_chain(&chain).expect("fixture chain must verify");
    chain
}

fn complete_checkpoint() -> WriteAwareCheckpoint {
    build_chain(&[
        WriteCheckpointPhase::BeforeTransaction,
        WriteCheckpointPhase::GrantConsumed,
        WriteCheckpointPhase::Staging,
        WriteCheckpointPhase::Applying,
        WriteCheckpointPhase::CanonicalVerified,
        WriteCheckpointPhase::IndexUpdating,
        WriteCheckpointPhase::IndexVerified,
        WriteCheckpointPhase::ReceiptPersisting,
        WriteCheckpointPhase::ReceiptPersisted,
        WriteCheckpointPhase::Complete,
    ])
    .pop()
    .expect("complete checkpoint")
}

fn observation() -> WriteRecoveryObservation {
    WriteRecoveryObservation {
        root_identity_matches: true,
        concurrent_change_detected: false,
        secret_store_available: true,
        canonical_postimages_match: true,
        receipt_chain_verified: true,
        index_verified: true,
        staging_items_present: 0,
        staging_retention_expired: false,
    }
}

#[test]
fn crash_and_concurrency_matrix_preserves_conflicts_and_never_replays() {
    let complete = complete_checkpoint();
    let cases = [
        (
            "concurrent-user-edit",
            WriteRecoveryObservation {
                concurrent_change_detected: true,
                ..observation()
            },
            WriteRecoveryInstruction::PreserveConflictForReview,
        ),
        (
            "concurrent-session-edit",
            WriteRecoveryObservation {
                concurrent_change_detected: true,
                ..observation()
            },
            WriteRecoveryInstruction::PreserveConflictForReview,
        ),
        (
            "permission-change",
            WriteRecoveryObservation {
                root_identity_matches: false,
                ..observation()
            },
            WriteRecoveryInstruction::RestoreWorkspaceIdentity,
        ),
        (
            "moved-root",
            WriteRecoveryObservation {
                root_identity_matches: false,
                ..observation()
            },
            WriteRecoveryInstruction::RestoreWorkspaceIdentity,
        ),
        (
            "lost-secret-store",
            WriteRecoveryObservation {
                secret_store_available: false,
                ..observation()
            },
            WriteRecoveryInstruction::RestoreSecretStore,
        ),
        (
            "disk-full-after-canonical-write",
            WriteRecoveryObservation {
                receipt_chain_verified: false,
                ..observation()
            },
            WriteRecoveryInstruction::PersistTerminalReceipt,
        ),
    ];
    for (name, observed, expected) in cases {
        let decision = plan_write_recovery(&complete, &observed).expect(name);
        assert_eq!(decision.instruction, expected, "{name}");
        assert!(!decision.repeat_completed_write, "{name}");
    }
}

#[test]
fn cancellation_timeout_crash_and_rollback_failure_have_closed_recovery() {
    let before = build_chain(&[WriteCheckpointPhase::BeforeTransaction])
        .pop()
        .expect("before");
    let cancelled = plan_write_recovery(&before, &observation()).expect("cancelled before consume");
    assert_eq!(
        cancelled.instruction,
        WriteRecoveryInstruction::BuildFreshProposal
    );

    for failure in ["timeout", "crash", "stale-grant", "rollback-failure"] {
        let uncertain = build_chain(&[
            WriteCheckpointPhase::BeforeTransaction,
            WriteCheckpointPhase::GrantConsumed,
            WriteCheckpointPhase::Uncertain,
        ])
        .pop()
        .expect("uncertain");
        let decision = plan_write_recovery(&uncertain, &observation()).expect(failure);
        assert_eq!(
            decision.instruction,
            WriteRecoveryInstruction::BlockUncertainState,
            "{failure}"
        );
        assert!(!decision.repeat_completed_write, "{failure}");
    }
}

#[test]
fn secret_canaries_are_absent_from_every_write_boundary_output() {
    let canaries: [(&str, &[u8]); 6] = [
        ("password", b"correct horse battery staple"),
        (
            "private_key",
            concat!("-----BEGIN TEST ", "PRIVATE KEY-----").as_bytes(),
        ),
        ("authorization", b"Bearer abcdefghijklmnopqrstuvwxyz"),
        (
            "provider_token",
            concat!("gh", "p_abcdefghijklmnopqrstuvwxyz1234567890").as_bytes(),
        ),
        ("cloud_key", concat!("AK", "IA1234567890ABCDEF").as_bytes()),
        (
            "database_uri",
            b"https://user:password@example.invalid/path",
        ),
    ];
    for boundary in WritePrivacyBoundary::ALL {
        for (name, canary) in canaries {
            let output = sanitize_write_boundary(
                boundary,
                &[WriteBoundaryField {
                    name,
                    value: canary,
                    sensitivity: WriteFieldSensitivity::Credential,
                }],
            )
            .expect("privacy gate");
            let encoded = serde_json::to_vec(&(output.fields, output.receipt)).expect("json");
            assert!(!encoded.windows(canary.len()).any(|window| window == canary));
        }
    }
}

#[test]
fn durable_and_temporary_inventory_exposes_no_inaccessible_orphan() {
    let complete = complete_checkpoint();
    let active = build_chain_named(
        &[
            WriteCheckpointPhase::BeforeTransaction,
            WriteCheckpointPhase::GrantConsumed,
            WriteCheckpointPhase::Staging,
        ],
        "transaction-active",
        "action-active",
    )
    .pop()
    .expect("active");
    let inventory = [
        StagingInventoryItem {
            staging_id: "staging-terminal".to_owned(),
            transaction_id: complete.transaction_id.clone(),
            action_id: complete.action_id.clone(),
            content_sha256: hash('a'),
            created_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 2_000,
            state: StagingObjectState::Orphaned,
        },
        StagingInventoryItem {
            staging_id: "staging-active".to_owned(),
            transaction_id: active.transaction_id.clone(),
            action_id: active.action_id.clone(),
            content_sha256: hash('b'),
            created_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 5_000,
            state: StagingObjectState::Active,
        },
        StagingInventoryItem {
            staging_id: "staging-unknown".to_owned(),
            transaction_id: "transaction-unknown".to_owned(),
            action_id: "action-unknown".to_owned(),
            content_sha256: hash('c'),
            created_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 2_000,
            state: StagingObjectState::Quarantined,
        },
    ];
    let diagnostics = diagnose_staging_inventory(3_000, &[complete, active], &inventory)
        .expect("inventory diagnostics");
    assert_eq!(diagnostics.len(), inventory.len());
    assert!(diagnostics[0].cleanup_eligible);
    assert!(!diagnostics[1].orphaned);
    assert!(!diagnostics[1].cleanup_eligible);
    assert!(!diagnostics[2].attributable);
    assert!(!diagnostics[2].cleanup_eligible);
    assert_ne!(inventory[0].content_sha256, ZERO_SHA256);
}
