// SPDX-License-Identifier: Apache-2.0
//! Dormant RSA/v4 signature assembly; no object write or approval authority.
//!
//! Original implementation of the wire fields in RFC 4880 sections 3.2,
//! 4.2.2.3, 5.2.3, 5.2.4 and 6.2-6.3. No reference code is incorporated.
#![allow(
    dead_code,
    reason = "signed creation remains dormant pending native admission and approval composition"
)]

use super::*;
use agentmage_kernel_engine::local_commit::{LocalCommitPlan, verify_local_commit_plan};

const LIMIT: usize = crate::sandbox::MAX_OUTPUT_BYTES;
const MAX_RSA_BYTES: usize = 1024;

fn refused() -> LinuxLocalCommitError {
    error(LinuxLocalCommitErrorKind::PlanDenied)
}

// The fields cannot be supplied independently or changed between hashing and
// assembly. This value is not a verified signature, receipt or launch permit.
struct Prepared {
    content: Vec<u8>,
    hashed: Vec<u8>,
    digest: [u8; 32],
    key_id: [u8; 8],
    signature_header: &'static [u8],
}

impl Prepared {
    fn new(plan: &LocalCommitPlan) -> Result<Self, LinuxLocalCommitError> {
        verify_local_commit_plan(plan).map_err(|_| refused())?;
        if plan.parent_object.len() != plan.candidate_tree.len()
            || plan.signer.key_fingerprint.len() != 40
        {
            return Err(refused());
        }
        let timestamp = u32::try_from(plan.timestamp_epoch_seconds).map_err(|_| refused())?;
        let fingerprint = &plan.signer.key_fingerprint;
        let mut key_id = [0; 8];
        for (i, byte) in key_id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&fingerprint[24 + 2 * i..26 + 2 * i], 16)
                .map_err(|_| refused())?;
        }
        let timezone = git_timezone(plan.timezone_offset_minutes);
        // Do not normalize message bytes or add a final newline. Git's object
        // framing ("commit <length>\0") is never part of the signed content.
        let content = format!(
            "tree {}\nparent {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\n{}",
            plan.candidate_tree,
            plan.parent_object,
            plan.author.name,
            plan.author.email,
            plan.timestamp_epoch_seconds,
            timezone,
            plan.committer.name,
            plan.committer.email,
            plan.timestamp_epoch_seconds,
            timezone,
            plan.message
        )
        .into_bytes();
        if content.len() > LIMIT {
            return Err(refused());
        }
        // v4, binary document, RSA, SHA-256; hashed creation-time subpacket.
        // Advisory issuer key ID goes in the unhashed region. Only independent
        // verification against the pinned full fingerprint supplies identity.
        let mut hashed = vec![4, 0, 1, 8, 0, 6, 5, 2];
        hashed.extend_from_slice(&timestamp.to_be_bytes());
        let mut context = Sha256::new();
        context.update(&content);
        context.update(&hashed);
        context.update([4, 255]);
        context.update((hashed.len() as u32).to_be_bytes());
        Ok(Self {
            content,
            hashed,
            digest: context.finalize().into(),
            key_id,
            signature_header: if plan.parent_object.len() == 40 {
                b"gpgsig "
            } else {
                b"gpgsig-sha256 "
            },
        })
    }

    fn assemble(&self, expression: &[u8]) -> Result<Vec<u8>, LinuxLocalCommitError> {
        let scalar = rsa_scalar(expression).ok_or_else(refused)?;
        let mut body = self.hashed.clone();
        body.extend_from_slice(&[0, 10, 9, 16]);
        body.extend_from_slice(&self.key_id);
        body.extend_from_slice(&self.digest[..2]);
        let bits = (scalar.len() * 8 - scalar[0].leading_zeros() as usize) as u16;
        body.extend_from_slice(&bits.to_be_bytes());
        body.extend_from_slice(scalar);
        // One signature packet, definite five-octet length. No partial bodies.
        let mut packet = vec![0xc2, 255];
        packet.extend_from_slice(&(body.len() as u32).to_be_bytes());
        packet.extend_from_slice(&body);
        let armor = armor(&packet);
        let separator = self
            .content
            .windows(2)
            .position(|w| w == b"\n\n")
            .ok_or_else(refused)?;
        let mut raw = self.content[..separator + 1].to_vec();
        raw.extend_from_slice(self.signature_header);
        for (index, line) in armor.split_inclusive(|b| *b == b'\n').enumerate() {
            if index != 0 {
                raw.push(b' ');
            }
            raw.extend_from_slice(line);
        }
        raw.extend_from_slice(&self.content[separator + 1..]);
        if raw.len() > LIMIT {
            return Err(refused());
        }
        Ok(raw)
    }
}

// Accept one bounded canonical RSA expression, with exactly one scalar and no
// extension fields. The installed foreground agent emits unsigned, big-endian
// scalars (including a high first bit), without a sign-padding zero. Refuse
// zero, redundant leading zeros and trailing data; no integer reinterpretation.
fn rsa_scalar(expression: &[u8]) -> Option<&[u8]> {
    if expression.len() > MAX_RSA_BYTES + 64 {
        return None;
    }
    let value = expression.strip_prefix(b"(7:sig-val(3:rsa(1:s")?;
    let colon = value.iter().position(|b| *b == b':')?;
    let digits = value.get(..colon)?;
    if digits.is_empty()
        || digits.len() > 4
        || digits[0] == b'0'
        || !digits.iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let length = std::str::from_utf8(digits).ok()?.parse::<usize>().ok()?;
    if length == 0 || length > MAX_RSA_BYTES {
        return None;
    }
    let remainder = value.get(colon + 1..)?;
    let scalar = remainder.get(..length)?;
    if remainder.get(length..)? != b")))" {
        return None;
    }
    (scalar[0] != 0).then_some(scalar)
}

fn radix64(bytes: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let word = ((group[0] as u32) << 16)
            | ((group.get(1).copied().unwrap_or(0) as u32) << 8)
            | group.get(2).copied().unwrap_or(0) as u32;
        for shift in [18, 12, 6, 0] {
            output.push(if shift / 6 < 3 - group.len() {
                b'='
            } else {
                ALPHABET[((word >> shift) & 63) as usize]
            });
        }
    }
    output
}

fn armor(packet: &[u8]) -> Vec<u8> {
    let mut out = b"-----BEGIN PGP SIGNATURE-----\n\n".to_vec();
    for line in radix64(packet).chunks(64) {
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    // CRC-24 of packet bytes, independently expressed as a 24-bit recurrence.
    let mut crc = 0xb704ce_u32;
    for byte in packet {
        crc ^= (*byte as u32) << 16;
        for _ in 0..8 {
            let carry = crc & 0x800000 != 0;
            crc = (crc << 1) & 0xffffff;
            if carry {
                crc ^= 0x864cfb;
            }
        }
    }
    out.push(b'=');
    out.extend_from_slice(&radix64(&crc.to_be_bytes()[1..]));
    out.extend_from_slice(b"\n-----END PGP SIGNATURE-----\n");
    out
}

pub(super) fn sign_commit(
    signer: &LinuxOpenPgpSigner,
    plan: &LocalCommitPlan,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    let deadline = deadline.min(Instant::now() + Duration::from_secs(30));
    sign_commit_with(
        signer,
        plan,
        cancellation,
        deadline,
        |digest| signing::sign_hash(signer, digest, cancellation, deadline),
        |raw| verification::verify(signer, raw, plan, cancellation, deadline),
    )
}

fn sign_commit_with(
    signer: &LinuxOpenPgpSigner,
    plan: &LocalCommitPlan,
    cancellation: &CancellationToken,
    deadline: Instant,
    sign: impl FnOnce(&[u8; 32]) -> Result<Vec<u8>, LinuxLocalCommitError>,
    verify: impl FnOnce(&[u8]) -> Result<(), LinuxLocalCommitError>,
) -> Result<Vec<u8>, LinuxLocalCommitError> {
    inspection::check_control(cancellation, deadline)?;
    let prepared = Prepared::new(plan)?;
    if plan.signer != signer.pinned
        || signer.public_key.is_empty()
        || signer.public_key.len() > LIMIT
        || hash(&signer.public_key) != signer.pinned.public_key_sha256
    {
        return Err(refused());
    }
    let expression = sign(&prepared.digest)?;
    inspection::check_control(cancellation, deadline)?;
    let raw = prepared.assemble(&expression)?;
    let (_, content) = verification::split_signed_commit(&raw, plan)?;
    if content != prepared.content {
        return Err(refused());
    }
    verify(&raw)?;
    inspection::check_control(cancellation, deadline)?;
    Ok(raw)
}

#[cfg(test)]
#[path = "local_commit_packet_tests.rs"]
mod tests;
