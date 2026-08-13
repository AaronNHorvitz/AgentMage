# S-011-IT01 Provider-Substitution Results

**Status:** Pass for the current Fedora and Ubuntu implementation boundary  
**Result version:** 1  
**Task:** `11.1.3.4`  
**Fixture data:** Synthetic only

This integration gate disables or substitutes each currently representable
Secret Service and encrypted-store dependency. Protected persistence must fail
before creating or accepting authority, while policy-declared ephemeral data
must remain storage-free and independent of provider availability.

## Integration Matrix

| ID | Dependency change | Executed assertion | Result |
|---|---|---|---|
| `IT-01` | Missing operational key | Open through an unavailable key provider | Refused; database callback is not reached and no file is created |
| `IT-02` | Malformed operational key | Decode wrong length and non-hex material | Refused; database callback count remains zero |
| `IT-03` | Arbitrary Linux key provider | Compile the normal Linux authority-open API | Not representable; production accepts only `LinuxOperationalStoreKeyProvider` |
| `IT-04` | Substituted Secret Service client | Mutate the verified `/usr/bin/secret-tool` digest | Refused as `InvalidManifest` before client process or service contact |
| `IT-05` | Secret in process metadata | Inspect the exact constructed store command | Secret absent from executable, arguments, and the two-variable environment; store input remains standard input only |
| `IT-06` | Plaintext database or crypto fallback | Present a private file with a plaintext SQLite header | Refused without modifying the candidate or creating WAL/SHM sidecars |
| `IT-07` | Secret Service security control unavailable or invalid | Mutate each Linux startup-control status | Every dependent capability becomes non-verified; no reduced mode is admitted |
| `IT-08` | Provider unavailable for ephemeral content | Evaluate every structural raw-content class and explicit ephemeral retention | `Ephemeral` with encryption `None` and no prepared storage bytes |

## Cryptographic Provider Boundary

The workspace pins `rusqlite` 0.40.2 with default features disabled and only
`backup` plus `bundled-sqlcipher` enabled. The operational store has no runtime
cryptographic-provider trait or plaintext adapter. Every connection requires a
nonempty `PRAGMA cipher_version` before schema, migration, or authority work,
then enables cipher memory security and disables cipher logging. A substituted
plaintext SQLite database cannot become authority.

This is a closed build/runtime boundary, not proof against malicious compiler,
linker, operating-system, or hardware substitution. Supply-chain provenance and
release signing remain their own gates.

## Secret Channels

The verified Secret Service client is rehashed immediately before every
execution. Commands clear the inherited environment and reconstruct only
`XDG_RUNTIME_DIR` and `DBUS_SESSION_BUS_ADDRESS`. Profile and purpose are
bounded non-secret attributes. Secret values used for store operations enter
only through a pipe connected to standard input; lookup bytes return only
through a bounded pipe into a zeroizing value. Keys never enter process
arguments, environment variables, public errors, debug output, or receipts.

## Result

- Integration cases closed: **8 of 8**.
- Missing or malformed key callbacks reached: **0**.
- Substituted client process launches: **0**.
- Plaintext fallback paths accepted: **0**.
- Secret argument or environment occurrences: **0**.
- Ephemeral decisions selecting storage: **0**.
- Private user records or credentials used: **0**.
- External network operations used: **0**.
- Manual fuzzing operations used: **0**.

## Invalidation Rule

This result becomes stale if the production Linux authority constructor, key
provider, client manifest, process command, key encoding, SQLCipher features,
connection initialization, persistence policy, or startup-control mapping
changes. A new secret store, cryptographic provider, environment variable,
argument, storage backend, or ephemeral-content class must extend this matrix.

## Deliberate Limits

- No real user credential is read, written, listed, changed, or removed.
- Live Secret Service probe, round trip, operational-key provisioning, and key
  destruction tests remain environment-dependent and ignored in this gate.
- The client-digest substitution test uses the installed root-owned
  `/usr/bin/secret-tool` identity but refuses before D-Bus contact.
- The build pins bundled SQLCipher and tests runtime presence plus plaintext
  refusal; it does not emulate a malicious toolchain or compromised binary.
- macOS Keychain, Windows Credential Manager, packaging, release acceptance,
  support, and manually deferred fuzzing remain separate gates.
