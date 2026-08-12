#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ed25519_dalek::{Signer, SigningKey};
use rustix::fs::{FileType, Mode, OFlags, fstat, open};
use zeroize::Zeroize;

const PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"agentmage.package-manifest.v2\0";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const USAGE: &str = "usage:\n  cargo run -p agentmage-xtask -- package-candidate [--version X.Y.Z] [--output PATH]\n  cargo run -p agentmage-xtask -- package-release-bundle --version X.Y.Z --release-sequence N [--output PATH]\n  cargo run -p agentmage-xtask -- sign-package-manifest --manifest PATH --public-key PATH --signature PATH < PRIVATE-SEED";

enum ReleaseCommand {
    PackageCandidate(Vec<OsString>),
    PackageReleaseBundle(Vec<OsString>),
    SignPackageManifest {
        manifest: PathBuf,
        public_key: PathBuf,
        signature: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<(), &'static str> {
    match parse_arguments(&arguments)? {
        ReleaseCommand::PackageCandidate(forwarded) => run_builder(
            "scripts/package_candidate.py",
            forwarded,
            "release.package_candidate_failed",
        ),
        ReleaseCommand::PackageReleaseBundle(forwarded) => run_builder(
            "scripts/package_release.py",
            forwarded,
            "release.package_release_bundle_failed",
        ),
        ReleaseCommand::SignPackageManifest {
            manifest,
            public_key,
            signature,
        } => sign_package_manifest(&manifest, &public_key, &signature, &mut std::io::stdin()),
    }
}

fn run_builder(
    script: &str,
    forwarded: Vec<OsString>,
    failure: &'static str,
) -> Result<(), &'static str> {
    let root = workspace_root()?;
    let status = Command::new("python3")
        .arg(root.join(script))
        .args(forwarded)
        .current_dir(root)
        .status()
        .map_err(|_| "release.package_builder_unavailable")?;
    status.success().then_some(()).ok_or(failure)
}

fn parse_arguments(arguments: &[OsString]) -> Result<ReleaseCommand, &'static str> {
    let Some(command) = arguments.first().and_then(|value| value.to_str()) else {
        eprintln!("{USAGE}");
        return Err("release.arguments_invalid");
    };
    if command == "sign-package-manifest" {
        return parse_sign_arguments(&arguments[1..]);
    }
    let release_bundle = command == "package-release-bundle";
    if command != "package-candidate" && !release_bundle {
        eprintln!("{USAGE}");
        return Err("release.arguments_invalid");
    }
    let mut forwarded = Vec::new();
    let mut cursor = 1;
    while cursor < arguments.len() {
        let option = arguments[cursor]
            .to_str()
            .ok_or("release.arguments_invalid")?;
        if !matches!(option, "--version" | "--output" | "--release-sequence")
            || (!release_bundle && option == "--release-sequence")
            || cursor + 1 >= arguments.len()
        {
            return Err("release.arguments_invalid");
        }
        let value = arguments[cursor + 1]
            .to_str()
            .ok_or("release.arguments_invalid")?;
        if value.is_empty() || value.starts_with('-') {
            return Err("release.arguments_invalid");
        }
        forwarded.push(arguments[cursor].clone());
        forwarded.push(arguments[cursor + 1].clone());
        cursor += 2;
    }
    if release_bundle && !forwarded.iter().any(|value| value == "--release-sequence") {
        return Err("release.arguments_invalid");
    }
    Ok(if release_bundle {
        ReleaseCommand::PackageReleaseBundle(forwarded)
    } else {
        ReleaseCommand::PackageCandidate(forwarded)
    })
}

fn parse_sign_arguments(arguments: &[OsString]) -> Result<ReleaseCommand, &'static str> {
    let mut manifest = None;
    let mut public_key = None;
    let mut signature = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let option = arguments[cursor]
            .to_str()
            .ok_or("release.arguments_invalid")?;
        let value = arguments
            .get(cursor + 1)
            .ok_or("release.arguments_invalid")?;
        if value.is_empty() || value.to_string_lossy().starts_with('-') {
            return Err("release.arguments_invalid");
        }
        let slot = match option {
            "--manifest" => &mut manifest,
            "--public-key" => &mut public_key,
            "--signature" => &mut signature,
            _ => return Err("release.arguments_invalid"),
        };
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err("release.arguments_invalid");
        }
        cursor += 2;
    }
    Ok(ReleaseCommand::SignPackageManifest {
        manifest: manifest.ok_or("release.arguments_invalid")?,
        public_key: public_key.ok_or("release.arguments_invalid")?,
        signature: signature.ok_or("release.arguments_invalid")?,
    })
}

fn sign_package_manifest(
    manifest_path: &Path,
    public_key_path: &Path,
    signature_path: &Path,
    private_seed: &mut impl Read,
) -> Result<(), &'static str> {
    let manifest = read_bounded_regular(manifest_path, MAX_MANIFEST_BYTES)
        .map_err(|_| "release.manifest_unavailable")?;
    let public_key =
        read_bounded_regular(public_key_path, 32).map_err(|_| "release.public_key_unavailable")?;
    if public_key.len() != 32 {
        return Err("release.public_key_invalid");
    }
    let mut seed = [0_u8; 32];
    private_seed
        .read_exact(&mut seed)
        .map_err(|_| "release.private_key_invalid")?;
    let mut trailing = [0_u8; 1];
    if private_seed
        .read(&mut trailing)
        .map_err(|_| "release.private_key_invalid")?
        != 0
    {
        seed.zeroize();
        return Err("release.private_key_invalid");
    }
    let signing_key = SigningKey::from_bytes(&seed);
    seed.zeroize();
    if signing_key.verifying_key().as_bytes() != public_key.as_slice() {
        return Err("release.signer_mismatch");
    }
    let mut signed = Vec::with_capacity(PACKAGE_SIGNATURE_DOMAIN.len() + manifest.len());
    signed.extend_from_slice(PACKAGE_SIGNATURE_DOMAIN);
    signed.extend_from_slice(&manifest);
    let signature = signing_key.sign(&signed).to_bytes();
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(signature_path)
        .map_err(|_| "release.signature_output_denied")?;
    output
        .write_all(&signature)
        .and_then(|()| output.sync_all())
        .map_err(|_| "release.signature_output_failed")
}

fn read_bounded_regular(path: &Path, maximum: u64) -> Result<Vec<u8>, std::io::Error> {
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::other)?;
    let before = fstat(&descriptor).map_err(std::io::Error::other)?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_size <= 0
        || before.st_size as u64 > maximum
        || before.st_nlink != 1
    {
        return Err(std::io::Error::other("bounded regular file required"));
    }
    let mut bytes = Vec::with_capacity(before.st_size as usize);
    let file = fs::File::from(descriptor);
    (&file).take(maximum + 1).read_to_end(&mut bytes)?;
    let after = fstat(&file).map_err(std::io::Error::other)?;
    if bytes.len() as i64 != before.st_size
        || before.st_dev != after.st_dev
        || before.st_ino != after.st_ino
        || before.st_mode != after.st_mode
        || before.st_size != after.st_size
        || before.st_nlink != after.st_nlink
    {
        return Err(std::io::Error::other("input changed during read"));
    }
    Ok(bytes)
}

fn workspace_root() -> Result<std::path::PathBuf, &'static str> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .ok_or("release.workspace_root_unavailable")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    use ed25519_dalek::{Signature, SigningKey};

    use super::{PACKAGE_SIGNATURE_DOMAIN, ReleaseCommand, parse_arguments, sign_package_manifest};

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn package_candidate_forwards_only_closed_options() {
        let arguments = [
            "package-candidate",
            "--version",
            "0.0.1",
            "--output",
            "release-output",
        ]
        .map(OsString::from);
        let ReleaseCommand::PackageCandidate(forwarded) =
            parse_arguments(&arguments).expect("valid command")
        else {
            panic!("candidate command");
        };
        assert_eq!(forwarded, arguments[1..]);
    }

    #[test]
    fn unknown_missing_and_option_like_values_fail_closed() {
        for arguments in [
            vec!["unknown"],
            vec!["package-candidate", "--version"],
            vec!["package-candidate", "--unknown", "value"],
            vec!["package-candidate", "--output", "--version"],
            vec!["package-release-bundle", "--version", "1.0.0"],
            vec!["sign-package-manifest", "--manifest", "one"],
        ] {
            let arguments: Vec<OsString> = arguments.into_iter().map(OsString::from).collect();
            assert!(parse_arguments(&arguments).is_err());
        }
    }

    #[test]
    fn signer_uses_exact_stdin_seed_and_refuses_overwrite() {
        let directory = std::env::temp_dir().join(format!(
            "agentmage-release-signer-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).expect("temporary directory");
        let manifest = directory.join("manifest.json");
        let public_key = directory.join("release.pub");
        let signature = directory.join("manifest.sig");
        let manifest_bytes = b"{\"record_type\":\"fixture\"}\n";
        let seed = [17_u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        fs::write(&manifest, manifest_bytes).expect("manifest");
        fs::write(&public_key, signing_key.verifying_key().as_bytes()).expect("public key");
        sign_package_manifest(&manifest, &public_key, &signature, &mut Cursor::new(seed))
            .expect("signature");
        let signature_bytes: [u8; 64] = fs::read(&signature)
            .expect("signature bytes")
            .try_into()
            .expect("exact signature");
        let mut signed = PACKAGE_SIGNATURE_DOMAIN.to_vec();
        signed.extend_from_slice(manifest_bytes);
        signing_key
            .verifying_key()
            .verify_strict(&signed, &Signature::from_bytes(&signature_bytes))
            .expect("signature verifies");
        assert_eq!(
            sign_package_manifest(&manifest, &public_key, &signature, &mut Cursor::new(seed),),
            Err("release.signature_output_denied")
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn signer_rejects_wrong_public_key_and_nonexact_seed() {
        let directory = std::env::temp_dir().join(format!(
            "agentmage-release-signer-negative-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).expect("temporary directory");
        let manifest = directory.join("manifest.json");
        let public_key = directory.join("release.pub");
        let signature = directory.join("manifest.sig");
        fs::write(&manifest, b"{}\n").expect("manifest");
        fs::write(
            &public_key,
            SigningKey::from_bytes(&[19; 32]).verifying_key().as_bytes(),
        )
        .expect("public key");
        assert_eq!(
            sign_package_manifest(
                &manifest,
                &public_key,
                &signature,
                &mut Cursor::new([18; 32]),
            ),
            Err("release.signer_mismatch")
        );
        assert_eq!(
            sign_package_manifest(
                &manifest,
                &public_key,
                &signature,
                &mut Cursor::new([19_u8; 33]),
            ),
            Err("release.private_key_invalid")
        );
        assert!(!signature.exists());
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
