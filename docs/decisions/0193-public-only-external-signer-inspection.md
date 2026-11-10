# Decision 0193: Public-only external signer inspection

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0192 |
| Scope | Public identity inspection in the dormant local-commit adapter |

Pin a public signing identity through two offline GnuPG workers using the
existing closed supervisor. Capture only the selected external keyring's
`pubring.kbx`, or `pubring.gpg` when the first name is absent. Read through a held
owner-private directory, reject symlinks, special files, unsafe ownership or
modes, empty files and files larger than four MiB, and seal the bounded snapshot.
Both listing and public export use the same immutable database. Do not mount
the live keyring directory, private keys, configuration or agent sockets.

List public keys with fixed options disabling option files, agent startup,
key retrieval and default keyrings. Require the exact fingerprint attached to
its public key or subkey record, that key's own signing capability, a current
non-revoked/non-disabled identity and a usable primary identity. A fingerprint
elsewhere in the listing cannot qualify. Export public material only after the
listing succeeds. Bind the database, listing, export and selected fingerprint
into the inspection evidence digest.

This deliberately pins **public identity only**. It does not inspect a secret
key, establish private-key possession, prove signing availability or admit a
usable signing key. GnuPG secret-key listing requires its agent; an isolated
secret-key listing without an agent may succeed with no keys. Do not connect
to a host agent or relax the worker's socket-denying policy to avoid this.
The old direct inspection helper is retained only for disposable tests, with
no production fallback. Keybox-daemon stores are unsupported by this capture.

Use held root-owned installed executables, the original thirty-second total
deadline, independent output bounds and digests, resource limits, withheld
seccomp filter and whole-unit cleanup. The supervisor has a distinct closed
inspection owner and exact one-payload projection count. Partial output,
non-successful execution, cancellation, expiry or cleanup uncertainty refuses
inspection; stderr is diagnostic and cannot supply a key identity.

Signed creation remains direct and dormant. Its confinement, a usable private
signing mechanism, native artifact admission and complete approval/preservation
composition remain prerequisites for activation. No product route, approval,
commit/push permission, model measurement, evaluation input or threshold changes.
Component fixtures and disposable GnuPG runs do not establish native workflow
success. Neither T-303 nor T-306 closes on this increment.
