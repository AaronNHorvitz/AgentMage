//! Authenticated streaming encryption for private runtime artifact payloads.

use std::fmt;
use std::io::{ErrorKind, Read, Write};

use agentmage_kernel_engine::runtime_artifact::{
    MAX_RUNTIME_ARTIFACT_BYTES, RuntimeArtifactPayloadError, RuntimeArtifactPayloadObservation,
};
use chacha20poly1305::{
    Key, KeyInit, Tag, XChaCha20Poly1305, XNonce,
    aead::{AeadInOut, inout::InOutBuf},
};
use hkdf::Hkdf;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const FORMAT_MAGIC: &[u8; 8] = b"AMRTAE01";
const FORMAT_VERSION: u32 = 1;
const HEADER_BYTES: usize = 48;
const FILE_SALT_BYTES: usize = 32;
const CHUNK_BYTES: usize = 64 * 1024;
const RECORD_PREFIX_BYTES: usize = 13;
const TAG_BYTES: usize = 16;
const FINAL_PLAINTEXT_BYTES: usize = 40;
const RECORD_DATA: u8 = 1;
const RECORD_FINAL: u8 = 2;
const NONCE_DOMAIN: &[u8; 16] = b"AgentMagePayload";
const ROOT_KEY_SALT: &[u8] = b"agentmage.runtime-artifact.root-key.v1";
const ROOT_KEY_INFO: &[u8] = b"agentmage.linux.payload-store.xchacha20poly1305.v1";
const FILE_KEY_INFO: &[u8] = b"agentmage.runtime-artifact.payload-file.v1";
const RECORD_AAD_DOMAIN: &[u8] = b"agentmage.runtime-artifact.record.v1";
const MAX_DATA_RECORDS: u64 = MAX_RUNTIME_ARTIFACT_BYTES.div_ceil(CHUNK_BYTES as u64);
pub(crate) const MAX_ENCRYPTED_PAYLOAD_BYTES: u64 = HEADER_BYTES as u64
    + MAX_RUNTIME_ARTIFACT_BYTES
    + MAX_DATA_RECORDS * (RECORD_PREFIX_BYTES + TAG_BYTES) as u64
    + (RECORD_PREFIX_BYTES + FINAL_PLAINTEXT_BYTES + TAG_BYTES) as u64;

/// Domain-separated key retained only by the Linux private payload adapter.
pub(crate) struct ArtifactPayloadEncryptionKey(Zeroizing<[u8; 32]>);

impl fmt::Debug for ArtifactPayloadEncryptionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArtifactPayloadEncryptionKey")
            .finish_non_exhaustive()
    }
}

impl ArtifactPayloadEncryptionKey {
    fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// Derives the payload-store key without reusing the SQLCipher key directly.
pub(crate) fn derive_artifact_payload_key(
    operational_store_key: &[u8],
) -> Result<ArtifactPayloadEncryptionKey, RuntimeArtifactPayloadError> {
    if operational_store_key.len() != 32 {
        return Err(RuntimeArtifactPayloadError::Invalid);
    }
    let derivation = Hkdf::<Sha256>::new(Some(ROOT_KEY_SALT), operational_store_key);
    let mut key = Zeroizing::new([0_u8; 32]);
    derivation
        .expand(ROOT_KEY_INFO, key.as_mut_slice())
        .map_err(|_| RuntimeArtifactPayloadError::Invalid)?;
    Ok(ArtifactPayloadEncryptionKey(key))
}

/// Encrypts one bounded plaintext stream and returns its plaintext identity.
pub(crate) fn encrypt_payload(
    source: &mut dyn Read,
    destination: &mut dyn Write,
    key: &ArtifactPayloadEncryptionKey,
    maximum_bytes: u64,
) -> Result<RuntimeArtifactPayloadObservation, RuntimeArtifactPayloadError> {
    if maximum_bytes == 0 || maximum_bytes > MAX_RUNTIME_ARTIFACT_BYTES {
        return Err(RuntimeArtifactPayloadError::ResourceLimit);
    }
    let header = new_header()?;
    write_all(destination, &header)?;
    let cipher = file_cipher(key, header_salt(&header))?;
    let mut digest = Sha256::new();
    let mut byte_size = 0_u64;
    let mut record_index = 0_u64;
    let mut chunk = Zeroizing::new(vec![0_u8; CHUNK_BYTES]);

    loop {
        let count = read_plaintext_chunk(source, chunk.as_mut_slice())?;
        if count == 0 {
            break;
        }
        byte_size = byte_size
            .checked_add(count as u64)
            .filter(|size| *size <= maximum_bytes)
            .ok_or(RuntimeArtifactPayloadError::ResourceLimit)?;
        digest.update(&chunk[..count]);
        let prefix = record_prefix(RECORD_DATA, record_index, count)?;
        let nonce = record_nonce(record_index);
        let aad = record_aad(&header, &prefix);
        let tag = cipher
            .encrypt_inout_detached(&nonce, &aad, InOutBuf::from(&mut chunk[..count]))
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        write_all(destination, &prefix)?;
        write_all(destination, &chunk[..count])?;
        write_all(destination, tag.as_slice())?;
        record_index = record_index
            .checked_add(1)
            .filter(|index| *index <= MAX_DATA_RECORDS)
            .ok_or(RuntimeArtifactPayloadError::ResourceLimit)?;
    }
    if byte_size == 0 {
        return Err(RuntimeArtifactPayloadError::Invalid);
    }

    let payload_digest: [u8; 32] = digest.finalize().into();
    let mut terminal = Zeroizing::new([0_u8; FINAL_PLAINTEXT_BYTES]);
    terminal[..8].copy_from_slice(&byte_size.to_be_bytes());
    terminal[8..].copy_from_slice(&payload_digest);
    let prefix = record_prefix(RECORD_FINAL, record_index, FINAL_PLAINTEXT_BYTES)?;
    let nonce = record_nonce(record_index);
    let aad = record_aad(&header, &prefix);
    let tag = cipher
        .encrypt_inout_detached(&nonce, &aad, InOutBuf::from(terminal.as_mut_slice()))
        .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
    write_all(destination, &prefix)?;
    write_all(destination, terminal.as_slice())?;
    write_all(destination, tag.as_slice())?;

    Ok(RuntimeArtifactPayloadObservation {
        payload_sha256: hex_digest(payload_digest),
        byte_size,
    })
}

/// Verifies one encrypted stream without retaining plaintext bytes.
pub(crate) fn observe_payload(
    source: &mut dyn Read,
    key: &ArtifactPayloadEncryptionKey,
) -> Result<RuntimeArtifactPayloadObservation, RuntimeArtifactPayloadError> {
    decrypt_payload(source, key, PlaintextProjection::None).map(|(_, observation)| observation)
}

/// Verifies and returns one complete plaintext payload.
pub(crate) fn read_complete_payload(
    source: &mut dyn Read,
    key: &ArtifactPayloadEncryptionKey,
) -> Result<(Vec<u8>, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError> {
    decrypt_payload(source, key, PlaintextProjection::Complete)
}

/// Verifies a complete payload while retaining only one requested plaintext range.
pub(crate) fn read_payload_range(
    source: &mut dyn Read,
    key: &ArtifactPayloadEncryptionKey,
    offset: u64,
    maximum_bytes: u64,
) -> Result<(Vec<u8>, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError> {
    if maximum_bytes == 0 || maximum_bytes > MAX_RUNTIME_ARTIFACT_BYTES {
        return Err(RuntimeArtifactPayloadError::ResourceLimit);
    }
    decrypt_payload(
        source,
        key,
        PlaintextProjection::Range {
            offset,
            end: offset
                .checked_add(maximum_bytes)
                .ok_or(RuntimeArtifactPayloadError::ResourceLimit)?,
        },
    )
}

#[derive(Clone, Copy)]
enum PlaintextProjection {
    None,
    Complete,
    Range { offset: u64, end: u64 },
}

fn decrypt_payload(
    source: &mut dyn Read,
    key: &ArtifactPayloadEncryptionKey,
    projection: PlaintextProjection,
) -> Result<(Vec<u8>, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError> {
    let mut header = [0_u8; HEADER_BYTES];
    read_exact_ciphertext(source, &mut header)?;
    validate_header(&header)?;
    let cipher = file_cipher(key, header_salt(&header))?;
    let mut retained = Vec::new();
    let mut digest = Sha256::new();
    let mut byte_size = 0_u64;
    let mut expected_index = 0_u64;
    let mut saw_short_chunk = false;

    loop {
        let mut prefix = [0_u8; RECORD_PREFIX_BYTES];
        read_exact_ciphertext(source, &mut prefix)?;
        let (record_type, record_index, plaintext_len) = parse_record_prefix(&prefix)?;
        if record_index != expected_index {
            return Err(RuntimeArtifactPayloadError::Corrupt);
        }
        let mut ciphertext = Zeroizing::new(vec![0_u8; plaintext_len]);
        read_exact_ciphertext(source, ciphertext.as_mut_slice())?;
        let mut tag_bytes = [0_u8; TAG_BYTES];
        read_exact_ciphertext(source, &mut tag_bytes)?;
        let tag = Tag::from(tag_bytes);
        let nonce = record_nonce(record_index);
        let aad = record_aad(&header, &prefix);
        cipher
            .decrypt_inout_detached(
                &nonce,
                &aad,
                InOutBuf::from(ciphertext.as_mut_slice()),
                &tag,
            )
            .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?;

        match record_type {
            RECORD_DATA => {
                if plaintext_len == 0 || plaintext_len > CHUNK_BYTES || saw_short_chunk {
                    return Err(RuntimeArtifactPayloadError::Corrupt);
                }
                let chunk_start = byte_size;
                byte_size = byte_size
                    .checked_add(plaintext_len as u64)
                    .filter(|size| *size <= MAX_RUNTIME_ARTIFACT_BYTES)
                    .ok_or(RuntimeArtifactPayloadError::Corrupt)?;
                digest.update(ciphertext.as_slice());
                retain_projection(
                    &mut retained,
                    projection,
                    chunk_start,
                    ciphertext.as_slice(),
                )?;
                saw_short_chunk = plaintext_len < CHUNK_BYTES;
                expected_index = expected_index
                    .checked_add(1)
                    .filter(|index| *index <= MAX_DATA_RECORDS)
                    .ok_or(RuntimeArtifactPayloadError::Corrupt)?;
            }
            RECORD_FINAL => {
                if plaintext_len != FINAL_PLAINTEXT_BYTES || byte_size == 0 {
                    return Err(RuntimeArtifactPayloadError::Corrupt);
                }
                let declared_size = u64::from_be_bytes(
                    ciphertext[..8]
                        .try_into()
                        .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?,
                );
                let observed_digest: [u8; 32] = digest.finalize().into();
                if declared_size != byte_size || ciphertext[8..] != observed_digest {
                    return Err(RuntimeArtifactPayloadError::Corrupt);
                }
                let mut trailing = [0_u8; 1];
                match source.read(&mut trailing) {
                    Ok(0) => {}
                    Ok(_) => return Err(RuntimeArtifactPayloadError::Corrupt),
                    Err(_) => return Err(RuntimeArtifactPayloadError::Durability),
                }
                return Ok((
                    retained,
                    RuntimeArtifactPayloadObservation {
                        payload_sha256: hex_digest(observed_digest),
                        byte_size,
                    },
                ));
            }
            _ => return Err(RuntimeArtifactPayloadError::Corrupt),
        }
    }
}

fn retain_projection(
    retained: &mut Vec<u8>,
    projection: PlaintextProjection,
    chunk_start: u64,
    chunk: &[u8],
) -> Result<(), RuntimeArtifactPayloadError> {
    match projection {
        PlaintextProjection::None => Ok(()),
        PlaintextProjection::Complete => {
            retained.extend_from_slice(chunk);
            Ok(())
        }
        PlaintextProjection::Range { offset, end } => {
            let chunk_end = chunk_start
                .checked_add(chunk.len() as u64)
                .ok_or(RuntimeArtifactPayloadError::Corrupt)?;
            let overlap_start = offset.max(chunk_start);
            let overlap_end = end.min(chunk_end);
            if overlap_start < overlap_end {
                let start = usize::try_from(overlap_start - chunk_start)
                    .map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?;
                let finish = usize::try_from(overlap_end - chunk_start)
                    .map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?;
                retained.extend_from_slice(&chunk[start..finish]);
            }
            Ok(())
        }
    }
}

fn new_header() -> Result<[u8; HEADER_BYTES], RuntimeArtifactPayloadError> {
    let mut header = [0_u8; HEADER_BYTES];
    header[..8].copy_from_slice(FORMAT_MAGIC);
    header[8..12].copy_from_slice(&FORMAT_VERSION.to_be_bytes());
    header[12..16].copy_from_slice(&(CHUNK_BYTES as u32).to_be_bytes());
    fill_random(&mut header[16..])?;
    Ok(header)
}

fn validate_header(header: &[u8; HEADER_BYTES]) -> Result<(), RuntimeArtifactPayloadError> {
    let version = u32::from_be_bytes(
        header[8..12]
            .try_into()
            .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?,
    );
    let chunk_bytes = u32::from_be_bytes(
        header[12..16]
            .try_into()
            .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?,
    );
    if &header[..8] != FORMAT_MAGIC
        || version != FORMAT_VERSION
        || chunk_bytes != CHUNK_BYTES as u32
    {
        return Err(RuntimeArtifactPayloadError::Corrupt);
    }
    Ok(())
}

fn header_salt(header: &[u8; HEADER_BYTES]) -> &[u8] {
    &header[HEADER_BYTES - FILE_SALT_BYTES..]
}

fn file_cipher(
    key: &ArtifactPayloadEncryptionKey,
    salt: &[u8],
) -> Result<XChaCha20Poly1305, RuntimeArtifactPayloadError> {
    if salt.len() != FILE_SALT_BYTES {
        return Err(RuntimeArtifactPayloadError::Corrupt);
    }
    let derivation = Hkdf::<Sha256>::new(Some(salt), key.bytes());
    let mut file_key = Zeroizing::new([0_u8; 32]);
    derivation
        .expand(FILE_KEY_INFO, file_key.as_mut_slice())
        .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?;
    Ok(XChaCha20Poly1305::new(&Key::from(*file_key)))
}

fn record_prefix(
    record_type: u8,
    record_index: u64,
    plaintext_len: usize,
) -> Result<[u8; RECORD_PREFIX_BYTES], RuntimeArtifactPayloadError> {
    let plaintext_len =
        u32::try_from(plaintext_len).map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?;
    let mut prefix = [0_u8; RECORD_PREFIX_BYTES];
    prefix[0] = record_type;
    prefix[1..9].copy_from_slice(&record_index.to_be_bytes());
    prefix[9..].copy_from_slice(&plaintext_len.to_be_bytes());
    Ok(prefix)
}

fn parse_record_prefix(
    prefix: &[u8; RECORD_PREFIX_BYTES],
) -> Result<(u8, u64, usize), RuntimeArtifactPayloadError> {
    let record_type = prefix[0];
    let record_index = u64::from_be_bytes(
        prefix[1..9]
            .try_into()
            .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?,
    );
    let plaintext_len = usize::try_from(u32::from_be_bytes(
        prefix[9..]
            .try_into()
            .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?,
    ))
    .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?;
    match record_type {
        RECORD_DATA if plaintext_len <= CHUNK_BYTES => {}
        RECORD_FINAL if plaintext_len == FINAL_PLAINTEXT_BYTES => {}
        _ => return Err(RuntimeArtifactPayloadError::Corrupt),
    }
    Ok((record_type, record_index, plaintext_len))
}

fn record_nonce(record_index: u64) -> XNonce {
    let mut nonce = [0_u8; 24];
    nonce[..16].copy_from_slice(NONCE_DOMAIN);
    nonce[16..].copy_from_slice(&record_index.to_be_bytes());
    XNonce::from(nonce)
}

fn record_aad(header: &[u8; HEADER_BYTES], prefix: &[u8; RECORD_PREFIX_BYTES]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(RECORD_AAD_DOMAIN.len() + HEADER_BYTES + RECORD_PREFIX_BYTES);
    aad.extend_from_slice(RECORD_AAD_DOMAIN);
    aad.extend_from_slice(header);
    aad.extend_from_slice(prefix);
    aad
}

fn read_plaintext_chunk(
    source: &mut dyn Read,
    destination: &mut [u8],
) -> Result<usize, RuntimeArtifactPayloadError> {
    let mut filled = 0;
    while filled < destination.len() {
        match source.read(&mut destination[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return Err(RuntimeArtifactPayloadError::Durability),
        }
    }
    Ok(filled)
}

fn read_exact_ciphertext(
    source: &mut dyn Read,
    destination: &mut [u8],
) -> Result<(), RuntimeArtifactPayloadError> {
    source.read_exact(destination).map_err(|error| {
        if error.kind() == ErrorKind::UnexpectedEof {
            RuntimeArtifactPayloadError::Corrupt
        } else {
            RuntimeArtifactPayloadError::Durability
        }
    })
}

fn write_all(destination: &mut dyn Write, bytes: &[u8]) -> Result<(), RuntimeArtifactPayloadError> {
    destination
        .write_all(bytes)
        .map_err(|_| RuntimeArtifactPayloadError::Durability)
}

fn fill_random(bytes: &mut [u8]) -> Result<(), RuntimeArtifactPayloadError> {
    let mut filled = 0;
    while filled < bytes.len() {
        let count = getrandom(&mut bytes[filled..], GetRandomFlags::empty())
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        if count == 0 {
            return Err(RuntimeArtifactPayloadError::Durability);
        }
        filled += count;
    }
    Ok(())
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use agentmage_kernel_engine::runtime_artifact::RuntimeArtifactPayloadError;

    use super::{
        CHUNK_BYTES, derive_artifact_payload_key, encrypt_payload, observe_payload,
        read_complete_payload, read_payload_range,
    };

    fn key(byte: u8) -> super::ArtifactPayloadEncryptionKey {
        derive_artifact_payload_key(&[byte; 32]).expect("test key derives")
    }

    fn encrypt(bytes: &[u8], key: &super::ArtifactPayloadEncryptionKey) -> Vec<u8> {
        let mut encrypted = Vec::new();
        let observed = encrypt_payload(
            &mut Cursor::new(bytes),
            &mut encrypted,
            key,
            bytes.len() as u64,
        )
        .expect("payload encrypts");
        assert_eq!(observed.byte_size, bytes.len() as u64);
        encrypted
    }

    #[test]
    fn derivation_is_exact_domain_separated_and_secret_safe() {
        assert_eq!(
            derive_artifact_payload_key(&[1; 31]).expect_err("short key rejects"),
            RuntimeArtifactPayloadError::Invalid
        );
        let first = key(1);
        let repeated = key(1);
        let different = key(2);
        assert_eq!(first.0.as_slice(), repeated.0.as_slice());
        assert_ne!(first.0.as_slice(), different.0.as_slice());
        assert!(!format!("{first:?}").contains(&super::hex_digest(first.0.as_slice())));
    }

    #[test]
    fn round_trip_and_range_reads_cross_chunk_boundaries() {
        let key = key(7);
        for size in [1, CHUNK_BYTES - 1, CHUNK_BYTES, CHUNK_BYTES + 37] {
            let bytes = (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>();
            let encrypted = encrypt(&bytes, &key);
            let (complete, observed) = read_complete_payload(&mut Cursor::new(&encrypted), &key)
                .expect("complete payload decrypts");
            assert_eq!(complete, bytes);
            assert_eq!(observed.byte_size, size as u64);
            let offset = (CHUNK_BYTES - 9).min(size - 1);
            let maximum = 31_u64.min((size - offset) as u64);
            let (range, range_observation) =
                read_payload_range(&mut Cursor::new(&encrypted), &key, offset as u64, maximum)
                    .expect("range decrypts");
            assert_eq!(range, bytes[offset..offset + maximum as usize]);
            assert_eq!(range_observation, observed);
        }
    }

    #[test]
    fn ciphertext_hides_plaintext_and_randomizes_equal_payloads() {
        let key = key(9);
        let plaintext = b"AGENTMAGE-SECRET-CANARY-DO-NOT-PERSIST";
        let first = encrypt(plaintext, &key);
        let second = encrypt(plaintext, &key);
        assert_ne!(first, second);
        assert!(first.len() > plaintext.len());
        assert!(
            !first
                .windows(plaintext.len())
                .any(|window| window == plaintext)
        );
        assert!(
            !second
                .windows(plaintext.len())
                .any(|window| window == plaintext)
        );
    }

    #[test]
    fn wrong_key_tamper_truncation_and_append_fail_closed() {
        let correct = key(11);
        let wrong = key(12);
        let encrypted = encrypt(b"authenticated payload", &correct);
        assert_eq!(
            observe_payload(&mut Cursor::new(&encrypted), &wrong),
            Err(RuntimeArtifactPayloadError::Corrupt)
        );

        for position in [0, 20, encrypted.len() / 2, encrypted.len() - 1] {
            let mut tampered = encrypted.clone();
            tampered[position] ^= 0x80;
            assert_eq!(
                observe_payload(&mut Cursor::new(tampered), &correct),
                Err(RuntimeArtifactPayloadError::Corrupt)
            );
        }

        for removed in [1, 17, encrypted.len() / 2] {
            let truncated = &encrypted[..encrypted.len() - removed];
            assert_eq!(
                observe_payload(&mut Cursor::new(truncated), &correct),
                Err(RuntimeArtifactPayloadError::Corrupt)
            );
        }

        let mut appended = encrypted;
        appended.push(0);
        assert_eq!(
            observe_payload(&mut Cursor::new(appended), &correct),
            Err(RuntimeArtifactPayloadError::Corrupt)
        );
    }

    #[test]
    fn empty_and_over_limit_requests_are_rejected() {
        let key = key(13);
        assert_eq!(
            encrypt_payload(&mut Cursor::new([]), &mut Vec::new(), &key, 1),
            Err(RuntimeArtifactPayloadError::Invalid)
        );
        assert_eq!(
            encrypt_payload(&mut Cursor::new([1]), &mut Vec::new(), &key, 0),
            Err(RuntimeArtifactPayloadError::ResourceLimit)
        );
    }
}
