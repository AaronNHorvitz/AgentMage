//! SQLCipher-backed persistence for Verified Chat sessions, events, and source artifacts.

use std::sync::{Arc, Mutex};

use agentmage_kernel_contracts::{
    EngineeringEvent, EngineeringSessionSnapshot, RuntimeArtifactId, SessionId,
};
use rusqlite::{OptionalExtension as _, params};
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;
use crate::persistent_supervisor::{EngineeringSupervisorStore, PersistentSupervisorError};
use crate::verified_artifact::{
    ArtifactUploadSpec, VerifiedArtifactError, VerifiedArtifactRecord, VerifiedArtifactStore,
    build_verified_artifact_record, verify_capture,
};

/// Shared exclusive SQLCipher authority used by the host's Engineering Runtime services.
#[derive(Clone)]
pub struct SqlCipherEngineeringStore {
    store: Arc<Mutex<OperationalStore>>,
}

impl SqlCipherEngineeringStore {
    /// Attaches Engineering Runtime persistence to one already admitted encrypted store.
    #[must_use]
    pub fn new(store: Arc<Mutex<OperationalStore>>) -> Self {
        Self { store }
    }

    /// Returns the same exclusive store authority for durable runtime composition.
    #[must_use]
    pub fn shared_store(&self) -> Arc<Mutex<OperationalStore>> {
        Arc::clone(&self.store)
    }
}

impl EngineeringSupervisorStore for SqlCipherEngineeringStore {
    fn create_session_with_event(
        &mut self,
        snapshot: &EngineeringSessionSnapshot,
        event: &EngineeringEvent,
    ) -> Result<(), PersistentSupervisorError> {
        let snapshot_json = encode(snapshot)?;
        let event_json = encode(event)?;
        let mut store = self
            .store
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let transaction = store
            .connection
            .transaction()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        transaction
            .execute(
                "INSERT INTO engineering_sessions(session_id, snapshot_sha256, record_json)
                 VALUES (?1, ?2, ?3)",
                params![
                    snapshot.session_id.as_str(),
                    snapshot.snapshot_sha256,
                    snapshot_json
                ],
            )
            .map_err(map_supervisor_write)?;
        insert_event(&transaction, event, &event_json)?;
        transaction
            .commit()
            .map_err(|_| PersistentSupervisorError::Storage)
    }

    fn append_event_and_save_session(
        &mut self,
        event: &EngineeringEvent,
        snapshot: &EngineeringSessionSnapshot,
    ) -> Result<(), PersistentSupervisorError> {
        let snapshot_json = encode(snapshot)?;
        let event_json = encode(event)?;
        let mut store = self
            .store
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let transaction = store
            .connection
            .transaction()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        insert_event(&transaction, event, &event_json)?;
        let changed = transaction
            .execute(
                "UPDATE engineering_sessions
                 SET snapshot_sha256 = ?2, record_json = ?3
                 WHERE session_id = ?1",
                params![
                    snapshot.session_id.as_str(),
                    snapshot.snapshot_sha256,
                    snapshot_json
                ],
            )
            .map_err(|_| PersistentSupervisorError::Storage)?;
        if changed != 1 {
            return Err(PersistentSupervisorError::NotFound);
        }
        transaction
            .commit()
            .map_err(|_| PersistentSupervisorError::Storage)
    }

    fn load_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<EngineeringSessionSnapshot>, PersistentSupervisorError> {
        let store = self
            .store
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let row = store
            .connection
            .query_row(
                "SELECT snapshot_sha256, record_json FROM engineering_sessions WHERE session_id = ?1",
                [session_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        row.map(|(digest, bytes)| {
            let snapshot: EngineeringSessionSnapshot = decode(&bytes)?;
            if snapshot.session_id != *session_id || snapshot.snapshot_sha256 != digest {
                return Err(PersistentSupervisorError::Integrity);
            }
            Ok(snapshot)
        })
        .transpose()
    }

    fn list_sessions(&self) -> Result<Vec<EngineeringSessionSnapshot>, PersistentSupervisorError> {
        let store = self
            .store
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let mut statement = store
            .connection
            .prepare("SELECT session_id, snapshot_sha256, record_json FROM engineering_sessions ORDER BY session_id")
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let mut snapshots = Vec::new();
        for row in rows {
            let (session_id, digest, bytes) =
                row.map_err(|_| PersistentSupervisorError::Storage)?;
            let snapshot: EngineeringSessionSnapshot = decode(&bytes)?;
            if snapshot.session_id.as_str() != session_id || snapshot.snapshot_sha256 != digest {
                return Err(PersistentSupervisorError::Integrity);
            }
            snapshots.push(snapshot);
        }
        Ok(snapshots)
    }

    fn load_events(
        &self,
        session_id: &SessionId,
        after_sequence: Option<u64>,
    ) -> Result<Vec<EngineeringEvent>, PersistentSupervisorError> {
        let after = after_sequence
            .map(i64::try_from)
            .transpose()
            .map_err(|_| PersistentSupervisorError::ResourceExceeded)?
            .unwrap_or(-1);
        let store = self
            .store
            .lock()
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let mut statement = store
            .connection
            .prepare(
                "SELECT sequence, event_sha256, previous_event_sha256, record_json
                 FROM engineering_events
                 WHERE session_id = ?1 AND sequence > ?2
                 ORDER BY sequence",
            )
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let rows = statement
            .query_map(params![session_id.as_str(), after], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(|_| PersistentSupervisorError::Storage)?;
        let mut events = Vec::new();
        for row in rows {
            let (sequence, digest, previous, bytes) =
                row.map_err(|_| PersistentSupervisorError::Storage)?;
            let event: EngineeringEvent = decode(&bytes)?;
            if event.session_id != *session_id
                || i64::try_from(event.sequence).ok() != Some(sequence)
                || event.event_sha256 != digest
                || event.previous_event_sha256 != previous
            {
                return Err(PersistentSupervisorError::Integrity);
            }
            events.push(event);
        }
        Ok(events)
    }
}

impl VerifiedArtifactStore for SqlCipherEngineeringStore {
    fn publish(
        &mut self,
        spec: &ArtifactUploadSpec,
        bytes: Vec<u8>,
        completed_at_epoch_ms: u64,
    ) -> Result<VerifiedArtifactRecord, VerifiedArtifactError> {
        let source_sha256 = sha256(&bytes);
        if source_sha256 != spec.expected_sha256 || bytes.len() as u64 != spec.total_bytes {
            return Err(VerifiedArtifactError::IntegrityMismatch);
        }
        let artifact_id = RuntimeArtifactId::from_raw(format!(
            "verified-artifact-{}",
            sha256(format!("{}:{}", spec.session_id.as_str(), spec.upload_id.as_str()).as_bytes())
        ));
        let mut store = self
            .store
            .lock()
            .map_err(|_| VerifiedArtifactError::Storage)?;
        let transaction = store
            .connection
            .transaction()
            .map_err(|_| VerifiedArtifactError::Storage)?;
        let payload_deduplicated = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM engineering_artifact_payloads WHERE source_sha256 = ?1)",
                [&source_sha256],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| VerifiedArtifactError::Storage)?;
        let record = build_verified_artifact_record(
            spec,
            artifact_id,
            bytes,
            payload_deduplicated,
            completed_at_epoch_ms,
        )?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO engineering_artifact_payloads(source_sha256, payload) VALUES (?1, ?2)",
                params![record.capture.source_sha256, record.bytes],
            )
            .map_err(|_| VerifiedArtifactError::Storage)?;
        let record_json =
            serde_json::to_vec(&record.capture).map_err(|_| VerifiedArtifactError::Storage)?;
        transaction
            .execute(
                "INSERT INTO engineering_artifacts(
                    artifact_id, upload_id, session_id, source_sha256, receipt_sha256, record_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.capture.artifact_id.as_str(),
                    record.capture.upload_id.as_str(),
                    record.capture.session_id.as_str(),
                    record.capture.source_sha256,
                    record.capture.receipt_sha256,
                    record_json,
                ],
            )
            .map_err(|error| {
                if error.sqlite_error_code().is_some() {
                    VerifiedArtifactError::Duplicate
                } else {
                    VerifiedArtifactError::Storage
                }
            })?;
        transaction
            .commit()
            .map_err(|_| VerifiedArtifactError::Storage)?;
        Ok(record)
    }

    fn load(
        &self,
        session_id: &SessionId,
        artifact_id: &RuntimeArtifactId,
    ) -> Result<VerifiedArtifactRecord, VerifiedArtifactError> {
        let store = self
            .store
            .lock()
            .map_err(|_| VerifiedArtifactError::Storage)?;
        let row = store
            .connection
            .query_row(
                "SELECT a.source_sha256, a.receipt_sha256, a.record_json, p.payload
                 FROM engineering_artifacts a
                 JOIN engineering_artifact_payloads p ON p.source_sha256 = a.source_sha256
                 WHERE a.session_id = ?1 AND a.artifact_id = ?2",
                params![session_id.as_str(), artifact_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| VerifiedArtifactError::Storage)?
            .ok_or(VerifiedArtifactError::NotFound)?;
        let capture =
            serde_json::from_slice(&row.2).map_err(|_| VerifiedArtifactError::IntegrityMismatch)?;
        let record = VerifiedArtifactRecord {
            capture,
            bytes: row.3,
        };
        if record.capture.session_id != *session_id
            || record.capture.artifact_id != *artifact_id
            || record.capture.source_sha256 != row.0
            || record.capture.receipt_sha256 != row.1
            || sha256(&record.bytes) != row.0
        {
            return Err(VerifiedArtifactError::IntegrityMismatch);
        }
        Ok(record)
    }
}

fn insert_event(
    transaction: &rusqlite::Transaction<'_>,
    event: &EngineeringEvent,
    record_json: &[u8],
) -> Result<(), PersistentSupervisorError> {
    transaction
        .execute(
            "INSERT INTO engineering_events(
                session_id, sequence, event_id, event_sha256, previous_event_sha256, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.session_id.as_str(),
                i64::try_from(event.sequence)
                    .map_err(|_| PersistentSupervisorError::ResourceExceeded)?,
                event.event_id.as_str(),
                event.event_sha256,
                event.previous_event_sha256,
                record_json,
            ],
        )
        .map_err(map_supervisor_write)?;
    Ok(())
}

fn map_supervisor_write(error: rusqlite::Error) -> PersistentSupervisorError {
    if error.sqlite_error_code().is_some() {
        PersistentSupervisorError::Duplicate
    } else {
        PersistentSupervisorError::Storage
    }
}

fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, PersistentSupervisorError> {
    serde_json::to_vec(value).map_err(|_| PersistentSupervisorError::Storage)
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, PersistentSupervisorError> {
    serde_json::from_slice(bytes).map_err(|_| PersistentSupervisorError::Integrity)
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), PersistentSupervisorError> {
    let mut session_statement = store
        .connection
        .prepare("SELECT session_id, snapshot_sha256, record_json FROM engineering_sessions ORDER BY session_id")
        .map_err(|_| PersistentSupervisorError::Storage)?;
    let session_rows = session_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(|_| PersistentSupervisorError::Storage)?;
    for row in session_rows {
        let (session_id, digest, bytes) = row.map_err(|_| PersistentSupervisorError::Storage)?;
        let snapshot: EngineeringSessionSnapshot = decode(&bytes)?;
        if snapshot.session_id.as_str() != session_id || snapshot.snapshot_sha256 != digest {
            return Err(PersistentSupervisorError::Integrity);
        }
        crate::persistent_supervisor::verify_snapshot(&snapshot)?;
        let events = load_events_direct(store, &snapshot.session_id)?;
        crate::persistent_supervisor::verify_event_chain(&events)?;
        if snapshot.last_event_sequence != events.last().map(|event| event.sequence) {
            return Err(PersistentSupervisorError::Integrity);
        }
    }
    let mut artifact_statement = store
        .connection
        .prepare(
            "SELECT a.artifact_id, a.session_id, a.source_sha256, a.receipt_sha256,
                    a.record_json, p.payload
             FROM engineering_artifacts a
             JOIN engineering_artifact_payloads p ON p.source_sha256 = a.source_sha256
             ORDER BY a.artifact_id",
        )
        .map_err(|_| PersistentSupervisorError::Storage)?;
    let artifact_rows = artifact_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })
        .map_err(|_| PersistentSupervisorError::Storage)?;
    for row in artifact_rows {
        let (artifact_id, session_id, source_digest, receipt_digest, record, payload) =
            row.map_err(|_| PersistentSupervisorError::Storage)?;
        let capture =
            serde_json::from_slice::<agentmage_kernel_contracts::ArtifactCaptureResult>(&record)
                .map_err(|_| PersistentSupervisorError::Integrity)?;
        verify_capture(&capture).map_err(|_| PersistentSupervisorError::Integrity)?;
        if capture.artifact_id.as_str() != artifact_id
            || capture.session_id.as_str() != session_id
            || capture.source_sha256 != source_digest
            || capture.receipt_sha256 != receipt_digest
            || sha256(&payload) != source_digest
        {
            return Err(PersistentSupervisorError::Integrity);
        }
    }
    let orphan_payloads: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(*) FROM engineering_artifact_payloads p
             WHERE NOT EXISTS (
                 SELECT 1 FROM engineering_artifacts a WHERE a.source_sha256 = p.source_sha256
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|_| PersistentSupervisorError::Storage)?;
    if orphan_payloads != 0 {
        return Err(PersistentSupervisorError::Integrity);
    }
    Ok(())
}

fn load_events_direct(
    store: &OperationalStore,
    session_id: &SessionId,
) -> Result<Vec<EngineeringEvent>, PersistentSupervisorError> {
    let mut statement = store
        .connection
        .prepare(
            "SELECT record_json FROM engineering_events
             WHERE session_id = ?1 ORDER BY sequence",
        )
        .map_err(|_| PersistentSupervisorError::Storage)?;
    statement
        .query_map([session_id.as_str()], |row| row.get::<_, Vec<u8>>(0))
        .map_err(|_| PersistentSupervisorError::Storage)?
        .map(|row| {
            row.map_err(|_| PersistentSupervisorError::Storage)
                .and_then(|bytes| decode(&bytes))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        ArtifactSourceKind, ArtifactUploadChunk, ArtifactUploadId, CONTRACT_SCHEMA_VERSION,
        CloudSynchronizationMarker, CorrelationId, EngineeringSessionMode, SessionId,
        StorageFilesystemClass, StrictLocalStorageObservation,
    };

    use super::SqlCipherEngineeringStore;
    use crate::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use crate::persistent_supervisor::PersistentTaskSupervisor;
    use crate::verified_artifact::{
        ArtifactUploadSpec, VerifiedArtifactStore, VerifiedArtifactUploads,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestKey;

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&[91; 32]))
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None::<CloudSynchronizationMarker>,
            root_identity_sha256: [19; 32],
            symlink_free: true,
        }
    }

    fn temporary_path() -> (std::path::PathBuf, std::path::PathBuf) {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "agentmage-engineering-persistence-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("authority.db");
        (directory, path)
    }

    #[test]
    fn encrypted_restart_restores_session_events_and_exact_artifact() {
        let (directory, path) = temporary_path();
        let session_id = SessionId::from_raw("session-sqlcipher-restart");
        let bytes = b"first line\nsecond exact line\n".to_vec();
        let digest = super::sha256(&bytes);
        let artifact_id;
        {
            let shared = std::sync::Arc::new(std::sync::Mutex::new(
                OperationalStore::open(&path, &observation(), &mut TestKey).unwrap(),
            ));
            let adapter = SqlCipherEngineeringStore::new(shared);
            let mut supervisor = PersistentTaskSupervisor::new(adapter.clone());
            supervisor
                .create_session(
                    session_id.clone(),
                    "Durable Verified Chat".to_owned(),
                    EngineeringSessionMode::Agent,
                    CorrelationId::from_raw("correlation-create-sql"),
                    10,
                )
                .unwrap();
            supervisor
                .pause(
                    &session_id,
                    CorrelationId::from_raw("correlation-pause-sql"),
                    20,
                )
                .unwrap();
            let upload_id = ArtifactUploadId::from_raw("upload-sqlcipher-restart");
            let mut uploads = VerifiedArtifactUploads::new(adapter);
            uploads
                .begin(ArtifactUploadSpec {
                    upload_id: upload_id.clone(),
                    session_id: session_id.clone(),
                    source_kind: ArtifactSourceKind::Paste,
                    display_name: "Pasted text".to_owned(),
                    media_type: "text/plain".to_owned(),
                    total_bytes: bytes.len() as u64,
                    expected_sha256: digest.clone(),
                })
                .unwrap();
            uploads
                .append(ArtifactUploadChunk {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    upload_id: upload_id.clone(),
                    session_id: session_id.clone(),
                    sequence: 0,
                    offset: 0,
                    total_bytes: bytes.len() as u64,
                    chunk_sha256: digest.clone(),
                    final_chunk: true,
                    bytes: bytes.clone(),
                })
                .unwrap();
            artifact_id = uploads.commit(&upload_id, 30).unwrap().artifact_id;
        }
        {
            let shared = std::sync::Arc::new(std::sync::Mutex::new(
                OperationalStore::open(&path, &observation(), &mut TestKey).unwrap(),
            ));
            let adapter = SqlCipherEngineeringStore::new(shared);
            let supervisor = PersistentTaskSupervisor::new(adapter.clone());
            let snapshot = supervisor.open_session(&session_id).unwrap();
            assert_eq!(snapshot.last_event_sequence, Some(1));
            assert_eq!(supervisor.replay(&session_id, None).unwrap().len(), 2);
            let record = adapter.load(&session_id, &artifact_id).unwrap();
            assert_eq!(record.bytes, bytes);
            assert_eq!(record.capture.source_sha256, digest);
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn event_and_snapshot_advance_roll_back_as_one_transaction() {
        let (directory, path) = temporary_path();
        let shared = std::sync::Arc::new(std::sync::Mutex::new(
            OperationalStore::open(&path, &observation(), &mut TestKey).unwrap(),
        ));
        let adapter = SqlCipherEngineeringStore::new(shared.clone());
        let session_id = SessionId::from_raw("session-atomic-event");
        let mut supervisor = PersistentTaskSupervisor::new(adapter);
        supervisor
            .create_session(
                session_id.clone(),
                "Atomic transition".to_owned(),
                EngineeringSessionMode::Agent,
                CorrelationId::from_raw("correlation-atomic-create"),
                10,
            )
            .unwrap();
        shared
            .lock()
            .unwrap()
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER deny_engineering_snapshot_update
                 BEFORE UPDATE ON engineering_sessions
                 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
            )
            .unwrap();
        assert!(
            supervisor
                .pause(
                    &session_id,
                    CorrelationId::from_raw("correlation-atomic-pause"),
                    20,
                )
                .is_err()
        );
        let store = shared.lock().unwrap();
        let event_count: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM engineering_events WHERE session_id = ?1",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
        drop(store);
        assert_eq!(
            supervisor
                .open_session(&session_id)
                .unwrap()
                .last_event_sequence,
            Some(0)
        );
        drop(supervisor);
        drop(shared);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn source_payload_tampering_blocks_encrypted_store_reopen() {
        let (directory, path) = temporary_path();
        let session_id = SessionId::from_raw("session-artifact-tamper");
        {
            let shared = std::sync::Arc::new(std::sync::Mutex::new(
                OperationalStore::open(&path, &observation(), &mut TestKey).unwrap(),
            ));
            let adapter = SqlCipherEngineeringStore::new(shared.clone());
            PersistentTaskSupervisor::new(adapter.clone())
                .create_session(
                    session_id.clone(),
                    "Tamper fixture".to_owned(),
                    EngineeringSessionMode::Ask,
                    CorrelationId::from_raw("correlation-tamper-create"),
                    10,
                )
                .unwrap();
            let bytes = b"authoritative source".to_vec();
            let digest = super::sha256(&bytes);
            let upload_id = ArtifactUploadId::from_raw("upload-artifact-tamper");
            let mut uploads = VerifiedArtifactUploads::new(adapter);
            uploads
                .begin(ArtifactUploadSpec {
                    upload_id: upload_id.clone(),
                    session_id: session_id.clone(),
                    source_kind: ArtifactSourceKind::File,
                    display_name: "source.txt".to_owned(),
                    media_type: "text/plain".to_owned(),
                    total_bytes: bytes.len() as u64,
                    expected_sha256: digest.clone(),
                })
                .unwrap();
            uploads
                .append(ArtifactUploadChunk {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    upload_id: upload_id.clone(),
                    session_id,
                    sequence: 0,
                    offset: 0,
                    total_bytes: bytes.len() as u64,
                    bytes,
                    chunk_sha256: digest,
                    final_chunk: true,
                })
                .unwrap();
            uploads.commit(&upload_id, 20).unwrap();
            shared
                .lock()
                .unwrap()
                .connection
                .execute(
                    "UPDATE engineering_artifact_payloads SET payload = ?1",
                    [b"tampered".as_slice()],
                )
                .unwrap();
        }
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut TestKey)
                .expect_err("tampered source payload must fail closed"),
            crate::operational_store::OperationalStoreError::IntegrityFailure
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
