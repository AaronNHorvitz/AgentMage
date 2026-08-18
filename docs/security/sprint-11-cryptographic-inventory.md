# Sprint 11 Cryptographic Inventory

**Status:** Current implementation inventory, not a certification claim  
**Scope:** Encrypted operational store, runtime artifact payloads, and Linux key boundary
**Inventory version:** 2

| Purpose | Component and version | Primitive or mode | Key boundary | Current verification |
|---|---|---|---|---|
| Operational database and backup encryption | `rusqlite` 0.40.2 with `bundled-sqlcipher` through `libsqlite3-sys` 0.38.2 | SQLCipher database encryption; exact low-level cipher suite remains provider-owned | 32-byte operational or separate backup key supplied only inside a zeroizing callback | Nonempty runtime `cipher_version`, encrypted header, keyed integrity, exact runtime PRAGMAs, wrong-key/plaintext refusal |
| Linux runtime artifact payload encryption | `chacha20poly1305` 0.11.0 and `hkdf` 0.13.0 from RustCrypto | HKDF-SHA-256 domain separation and chunked XChaCha20-Poly1305 with authenticated terminal size and SHA-256 | Store key derived from the operational key inside its one callback; fresh 256-bit salt and derived key per file; derived buffers zeroize | Round trip, range, wrong-key, header/ciphertext/tag mutation, truncation, append, randomized-equal-input, raw-disk canary, deduplication, and restart inventory tests |
| SQLCipher cryptographic backend on supported Linux builds | Distribution OpenSSL 3 `libcrypto` selected by the support matrix | Provider-owned SQLCipher primitives | Process-local SQLCipher key setup; no configuration, argument, environment, or receipt copy | Clean-build dependency closure and package/runtime identity evidence; no cryptographic-compliance certification claim |
| Operational-key generation | Linux `getrandom` through `rustix` 1.1.4 | 256 random bits | Generated in a zeroizing 32-byte buffer, encoded only for Secret Service storage | Exact byte-count requirement and constant-time provisioning verification; live provisioning remains environment-dependent |
| Operational-key storage | Freedesktop Secret Service through reverified `/usr/bin/secret-tool` | Platform service storage; backend algorithms are outside AgentMage's claim | Exact profile plus `operational-store-key-v1`; value enters the client only over standard input | Root-owned immutable client identity, per-call SHA-256 recheck, bounded pipes, cleared environment, missing/malformed/substituted-client refusal |
| Content, state, receipt, migration, export, and artifact identity | `sha2` 0.11.0 | SHA-256 | No secret key | Canonical input construction, fixed 64-hex shapes, mutation checks, and hash-chain verification |
| Constant-time operational-key comparison | Local fixed-length comparison | Bytewise XOR accumulation over 32 bytes | Decoded generated and retrieved key buffers only | Provisioning refuses mismatches; buffers are zeroizing |

## Configuration

The workspace disables `rusqlite` default features and enables only `backup`
and `bundled-sqlcipher`. Each store connection disables cipher logging before
keyed validation, requires a nonempty `cipher_version`, then enables cipher
memory security, foreign keys, secure deletion, memory-only temporary storage,
full synchronization, one-page WAL auto-checkpointing, exclusive locking, and
zero busy timeout.

The normal Linux authority path accepts only
`LinuxOperationalStoreKeyProvider`. The platform-neutral provider trait exists
inside the kernel and in explicitly test-only Linux composition. It does not
make the current SQLCipher backend dynamically replaceable.

## Key Uses

| Key class | Creation | Storage | Use | Destruction |
|---|---|---|---|---|
| Live operational-store key | Linux `getrandom` during explicit first-install provisioning | One exact Secret Service item | Primary SQLCipher authority and HKDF root for the separately labeled Linux runtime artifact payload key | Clear exact item, verify lookup absence, then remove known SQLCipher files; retained artifact ciphertext becomes cryptographically unavailable |
| Runtime artifact file key | HKDF-SHA-256 from the domain-separated payload-store key and fresh 256-bit file salt | Never stored independently; salt is public authenticated format input | One XChaCha20-Poly1305 payload file | Drops with the cipher; whole-store operational-key destruction prevents rederivation, while individual collection unlinks verified ciphertext |
| Backup key | Caller-provided platform key callback | Production identity not yet provisioned | One separately keyed SQLCipher backup | Separate key lifecycle required; live-store erasure does not erase backup keys |
| Test keys | Deterministic synthetic fixtures | Test memory or a synthetic process-shared file | Unit, integration, crash, and erasure evidence only | Test cleanup; never represented as production key handling |

## Precise Claims and Gaps

- This inventory identifies current source dependencies, provider boundaries,
  configuration, key purposes, and verified failure behavior.
- It is not a complete product CBOM, cryptographic-compliance assertion, post-quantum claim,
  malicious-toolchain proof, hardware-security claim, or physical-overwrite
  guarantee.
- SQLCipher's exact low-level algorithms and OpenSSL operating mode must be
  extracted from the signed release candidate and runtime environment before a
  release claim.
- Independent review of the artifact file format and key hierarchy,
  interruption-safe rotation, a production backup-key identity, macOS Keychain,
  Windows Credential Manager, clean-device continuity, and product-wide CBOM
  reconciliation remain later work.
- Manual fuzzing remains deferred and is not part of this inventory.
