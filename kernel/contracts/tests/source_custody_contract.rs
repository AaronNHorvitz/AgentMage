use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContextSensitivity, ConversationId, PolicyId, RuntimeArtifactId,
    RuntimeArtifactIntegrityState, RuntimeArtifactKind, RuntimeArtifactLifecycleState,
    RuntimeArtifactManifest, RuntimeArtifactRef, RuntimeEventRetention, RuntimeEventRetentionKind,
    RuntimeRunId, SOURCE_BACKING_ARTIFACT_KINDS, SOURCE_CUSTODY_STORE_ID, SessionId,
    SourceArtifactCustody, SourceArtifactId, SourceCustodyDisposition, SourceCustodyError,
    SourceCustodyHold, SourceCustodyId, SourceCustodyOwnerScope, SourceCustodyRetention, TaskId,
    bind_source_artifact_custody, from_json, source_custody_disposition, to_canonical_json,
    validate_source_artifact_custody,
};

const MANIFEST_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PAYLOAD_SHA256: &str = "84d89877f0d4041efb6bf91a16f0248f2fd573e6af05c19f96bedb9f882f7882";
const POLICY_SHA256: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CUSTODY_SHA256: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn payload_reference() -> RuntimeArtifactRef {
    RuntimeArtifactRef {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: RuntimeArtifactId::from_raw("runtime-artifact-source-1"),
        manifest_sha256: MANIFEST_SHA256.to_owned(),
        payload_sha256: PAYLOAD_SHA256.to_owned(),
        byte_size: 10,
        media_type: "text/plain".to_owned(),
    }
}

fn backing_manifest() -> RuntimeArtifactManifest {
    RuntimeArtifactManifest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: RuntimeArtifactId::from_raw("runtime-artifact-source-1"),
        kind: RuntimeArtifactKind::GeneratedFile,
        payload_sha256: PAYLOAD_SHA256.to_owned(),
        byte_size: 10,
        media_type: "text/plain".to_owned(),
        sensitivity: ContextSensitivity::Private,
        retention: RuntimeEventRetention {
            kind: RuntimeEventRetentionKind::Session,
            expires_at_epoch_ms: None,
        },
        session_id: SessionId::from_raw("session-1"),
        task_id: TaskId::from_raw("task-1"),
        producer_run_id: RuntimeRunId::from_raw("run-1"),
        producer_turn_id: None,
        producer_operation_id: None,
        receipt_id: None,
        policy_id: PolicyId::from_raw("policy-1"),
        policy_sha256: POLICY_SHA256.to_owned(),
        created_at_epoch_ms: 1_000,
        integrity: RuntimeArtifactIntegrityState::Verified,
        preview: None,
        manifest_sha256: MANIFEST_SHA256.to_owned(),
    }
}

fn custody() -> SourceArtifactCustody {
    SourceArtifactCustody {
        schema_version: CONTRACT_SCHEMA_VERSION,
        custody_id: SourceCustodyId::from_raw("custody-1"),
        source_artifact_id: SourceArtifactId::from_raw("source-artifact-1"),
        store_id: SOURCE_CUSTODY_STORE_ID.to_owned(),
        payload: payload_reference(),
        payload_kind: RuntimeArtifactKind::GeneratedFile,
        owner_scope: SourceCustodyOwnerScope::Task,
        session_id: SessionId::from_raw("session-1"),
        task_id: Some(TaskId::from_raw("task-1")),
        run_id: None,
        conversation_id: None,
        retention: SourceCustodyRetention {
            kind: RuntimeEventRetentionKind::Session,
            expires_at_epoch_ms: None,
        },
        hold: SourceCustodyHold::None,
        policy_id: PolicyId::from_raw("policy-1"),
        policy_sha256: POLICY_SHA256.to_owned(),
        admitted_at_epoch_ms: 1_000,
        lifecycle: RuntimeArtifactLifecycleState::Active,
        integrity: RuntimeArtifactIntegrityState::Verified,
        lifecycle_revision: 1,
        reason_code: "source.custody.admitted".to_owned(),
        updated_at_epoch_ms: 1_000,
        active_owner_count: 1,
        custody_sha256: CUSTODY_SHA256.to_owned(),
    }
}

fn encode(record: &SourceArtifactCustody) -> Vec<u8> {
    serde_json::to_vec(record).expect("fixture must serialize")
}

#[test]
fn one_valid_claim_round_trips_and_retains_its_payload() {
    let record = custody();
    let encoded = to_canonical_json(&record).expect("valid claim must serialize");
    assert_eq!(from_json::<SourceArtifactCustody>(&encoded), Ok(record.clone()));
    assert_eq!(validate_source_artifact_custody(&record), Ok(()));
    let disposition = source_custody_disposition(&record, 10_000);
    assert_eq!(disposition, Ok(SourceCustodyDisposition::Retain));
    let bound = bind_source_artifact_custody(&record, &backing_manifest());
    assert_eq!(bound, Ok(()));
}

#[test]
fn unknown_missing_duplicate_and_unsupported_fields_fail_closed() {
    let valid = String::from_utf8(encode(&custody())).expect("fixture must be UTF-8");
    let unknown = valid.replace(r#""store_id""#, r#""extra":true,"store_id""#);
    let missing = valid.replace(r#""hold":"none","#, "");
    let duplicate = valid.replace(r#""hold":"none""#, r#""hold":"none","hold":"user""#);
    let unsupported = valid.replace(r#""hold":"none""#, r#""hold":"indefinite""#);
    let trailing = format!("{valid}[]");
    let cases = [
        (unknown, "contract.field.unknown"),
        (missing, "contract.field.missing"),
        (duplicate, "contract.field.duplicate"),
        (unsupported, "contract.value.unsupported"),
        (trailing, "contract.parse.syntax"),
    ];
    for (candidate, expected_code) in cases {
        let error = from_json::<SourceArtifactCustody>(candidate.as_bytes())
            .expect_err("candidate must fail closed");
        assert_eq!(error.code, expected_code);
    }

    let mut future_version = custody();
    future_version.schema_version = CONTRACT_SCHEMA_VERSION + 1;
    let encoded = encode(&future_version);
    let error = from_json::<SourceArtifactCustody>(&encoded).expect_err("version must fail");
    assert_eq!(error.code, "contract.version.unsupported");
    assert_eq!(error.field_path, ["schema_version"]);
    let rejected = validate_source_artifact_custody(&future_version);
    assert_eq!(rejected, Err(SourceCustodyError::SchemaVersionUnsupported));
}

#[test]
fn a_second_owning_identity_is_rejected_for_every_scope() {
    let scopes = [
        SourceCustodyOwnerScope::Session,
        SourceCustodyOwnerScope::Task,
        SourceCustodyOwnerScope::Run,
        SourceCustodyOwnerScope::Conversation,
    ];
    for scope in scopes {
        let mut exact = custody();
        exact.owner_scope = scope;
        exact.task_id = None;
        exact.run_id = None;
        exact.conversation_id = None;
        match scope {
            SourceCustodyOwnerScope::Session => {}
            SourceCustodyOwnerScope::Task => {
                exact.task_id = Some(TaskId::from_raw("task-1"));
            }
            SourceCustodyOwnerScope::Run => {
                exact.run_id = Some(RuntimeRunId::from_raw("run-1"));
            }
            SourceCustodyOwnerScope::Conversation => {
                exact.conversation_id = Some(ConversationId::from_raw("conversation-1"));
            }
        }
        assert_eq!(validate_source_artifact_custody(&exact), Ok(()));

        let mut duplicated = exact.clone();
        if duplicated.task_id.is_none() {
            duplicated.task_id = Some(TaskId::from_raw("task-2"));
        } else {
            duplicated.run_id = Some(RuntimeRunId::from_raw("run-2"));
        }
        let ambiguous = validate_source_artifact_custody(&duplicated);
        assert_eq!(ambiguous, Err(SourceCustodyError::OwnerAmbiguous));
    }
}

#[test]
fn a_second_store_and_a_widened_backing_family_are_rejected() {
    let mut foreign = custody();
    foreign.store_id = "source-artifact-store".to_owned();
    let rejected = validate_source_artifact_custody(&foreign);
    assert_eq!(rejected, Err(SourceCustodyError::ForeignStore));

    let outside = [
        RuntimeArtifactKind::Patch,
        RuntimeArtifactKind::StandardError,
        RuntimeArtifactKind::TestLog,
        RuntimeArtifactKind::Report,
    ];
    for kind in outside {
        assert!(!SOURCE_BACKING_ARTIFACT_KINDS.contains(&kind));
        let mut widened = custody();
        widened.payload_kind = kind;
        let rejected = validate_source_artifact_custody(&widened);
        assert_eq!(rejected, Err(SourceCustodyError::UnsupportedBackingKind));
    }
    for kind in SOURCE_BACKING_ARTIFACT_KINDS {
        let mut admitted = custody();
        admitted.payload_kind = kind;
        assert_eq!(validate_source_artifact_custody(&admitted), Ok(()));
    }
}

#[test]
fn retention_is_durable_bounded_and_ordered() {
    let invalid = Err(SourceCustodyError::RetentionInvalid);

    let mut ephemeral = custody();
    ephemeral.retention.kind = RuntimeEventRetentionKind::Ephemeral;
    assert_eq!(validate_source_artifact_custody(&ephemeral), invalid);

    let mut unbounded = custody();
    unbounded.retention.kind = RuntimeEventRetentionKind::UntilExpiration;
    assert_eq!(validate_source_artifact_custody(&unbounded), invalid);

    let mut reversed = custody();
    reversed.retention.kind = RuntimeEventRetentionKind::UntilExpiration;
    reversed.retention.expires_at_epoch_ms = Some(1_000);
    assert_eq!(validate_source_artifact_custody(&reversed), invalid);

    let mut stray = custody();
    stray.retention.expires_at_epoch_ms = Some(9_000);
    assert_eq!(validate_source_artifact_custody(&stray), invalid);

    let mut expiring = custody();
    expiring.retention.kind = RuntimeEventRetentionKind::UntilExpiration;
    expiring.retention.expires_at_epoch_ms = Some(9_000);
    assert_eq!(validate_source_artifact_custody(&expiring), Ok(()));
}

#[test]
fn lifecycle_integrity_and_hold_states_cannot_contradict_each_other() {
    let invalid = Err(SourceCustodyError::LifecycleInvalid);

    let mut corrupt_active = custody();
    corrupt_active.integrity = RuntimeArtifactIntegrityState::Corrupt;
    assert_eq!(validate_source_artifact_custody(&corrupt_active), invalid);

    let mut unowned_active = custody();
    unowned_active.active_owner_count = 0;
    assert_eq!(validate_source_artifact_custody(&unowned_active), invalid);

    let mut unreviewed = custody();
    unreviewed.lifecycle = RuntimeArtifactLifecycleState::Quarantined;
    unreviewed.integrity = RuntimeArtifactIntegrityState::Corrupt;
    assert_eq!(validate_source_artifact_custody(&unreviewed), invalid);

    let mut held_after_release = custody();
    held_after_release.lifecycle = RuntimeArtifactLifecycleState::Released;
    held_after_release.hold = SourceCustodyHold::Checkpoint;
    assert_eq!(validate_source_artifact_custody(&held_after_release), invalid);

    let mut user_hold_after_release = custody();
    user_hold_after_release.lifecycle = RuntimeArtifactLifecycleState::Released;
    user_hold_after_release.hold = SourceCustodyHold::User;
    let released_hold = validate_source_artifact_custody(&user_hold_after_release);
    assert_eq!(released_hold, invalid);

    let mut owned_deletion = custody();
    owned_deletion.lifecycle = RuntimeArtifactLifecycleState::Deleted;
    owned_deletion.integrity = RuntimeArtifactIntegrityState::Deleted;
    assert_eq!(validate_source_artifact_custody(&owned_deletion), invalid);

    let mut backdated = custody();
    backdated.updated_at_epoch_ms = 999;
    assert_eq!(validate_source_artifact_custody(&backdated), invalid);

    let malformed = Err(SourceCustodyError::FieldMalformed);

    let mut uppercase_digest = custody();
    uppercase_digest.custody_sha256 = CUSTODY_SHA256.to_uppercase();
    assert_eq!(validate_source_artifact_custody(&uppercase_digest), malformed);

    let mut short_digest = custody();
    short_digest.policy_sha256 = "not-a-digest".to_owned();
    assert_eq!(validate_source_artifact_custody(&short_digest), malformed);

    let mut empty_reason = custody();
    empty_reason.reason_code = String::new();
    assert_eq!(validate_source_artifact_custody(&empty_reason), malformed);
}

#[test]
fn every_valid_claim_resolves_to_exactly_one_disposition() {
    let expiring = |hold: SourceCustodyHold| {
        let mut record = custody();
        record.hold = hold;
        record.retention.kind = RuntimeEventRetentionKind::UntilExpiration;
        record.retention.expires_at_epoch_ms = Some(5_000);
        record
    };
    let released = |active_owner_count: u32| {
        let mut record = custody();
        record.lifecycle = RuntimeArtifactLifecycleState::Released;
        record.active_owner_count = active_owner_count;
        record
    };
    let mut quarantined = custody();
    quarantined.lifecycle = RuntimeArtifactLifecycleState::Quarantined;
    quarantined.integrity = RuntimeArtifactIntegrityState::Missing;
    quarantined.hold = SourceCustodyHold::Review;
    let mut deleted = custody();
    deleted.lifecycle = RuntimeArtifactLifecycleState::Deleted;
    deleted.integrity = RuntimeArtifactIntegrityState::Deleted;
    deleted.active_owner_count = 0;

    let cases = [
        (custody(), 10_000, SourceCustodyDisposition::Retain),
        (expiring(SourceCustodyHold::None), 4_999, SourceCustodyDisposition::Retain),
        (expiring(SourceCustodyHold::None), 5_000, SourceCustodyDisposition::ReleaseEligible),
        (expiring(SourceCustodyHold::User), 10_000, SourceCustodyDisposition::Retain),
        (expiring(SourceCustodyHold::Checkpoint), 10_000, SourceCustodyDisposition::Retain),
        (released(1), 10_000, SourceCustodyDisposition::ReleaseEligible),
        (released(0), 10_000, SourceCustodyDisposition::DeleteEligible),
        (quarantined, 10_000, SourceCustodyDisposition::Blocked),
        (deleted, 10_000, SourceCustodyDisposition::Completed),
    ];
    for (record, now_epoch_ms, expected) in cases {
        let disposition = source_custody_disposition(&record, now_epoch_ms);
        assert_eq!(disposition, Ok(expected));
    }

    let mut second_store = custody();
    second_store.store_id = "second-store".to_owned();
    let refused = source_custody_disposition(&second_store, 10_000);
    assert_eq!(refused, Err(SourceCustodyError::ForeignStore));
}

#[test]
fn a_claim_cannot_assert_a_backing_the_store_does_not_hold() {
    let mismatch = Err(SourceCustodyError::ManifestMismatch);
    let manifest = backing_manifest();

    let mut substituted_payload = custody();
    substituted_payload.payload.payload_sha256 = CUSTODY_SHA256.to_owned();
    let bound = bind_source_artifact_custody(&substituted_payload, &manifest);
    assert_eq!(bound, mismatch);

    let mut substituted_size = custody();
    substituted_size.payload.byte_size = 11;
    let bound = bind_source_artifact_custody(&substituted_size, &manifest);
    assert_eq!(bound, mismatch);

    let mut substituted_media = custody();
    substituted_media.payload.media_type = "application/json".to_owned();
    let bound = bind_source_artifact_custody(&substituted_media, &manifest);
    assert_eq!(bound, mismatch);

    let mut substituted_kind = custody();
    substituted_kind.payload_kind = RuntimeArtifactKind::StandardOutput;
    let bound = bind_source_artifact_custody(&substituted_kind, &manifest);
    assert_eq!(bound, mismatch);
}

#[test]
fn existing_artifact_reference_bytes_are_unchanged_by_source_custody() {
    let expected = br#"{"schema_version":2,"artifact_id":"runtime-artifact-source-1","manifest_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","payload_sha256":"84d89877f0d4041efb6bf91a16f0248f2fd573e6af05c19f96bedb9f882f7882","byte_size":10,"media_type":"text/plain"}"#;
    let encoded = to_canonical_json(&payload_reference()).expect("reference must serialize");
    assert_eq!(encoded, expected);
}
