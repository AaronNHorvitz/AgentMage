# Decision 0194: Socket-free private signing primitive

| Field | Value |
| --- | --- |
| Status | Dormant implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0193 |
| Scope | Selected-key capture and foreground signing primitive, without activation |

Prepare a private signing primitive using a foreground GnuPG agent in server
mode, fed a fixed command stream through sealed stdin. Keep the existing
socket-denying filter unchanged. Do not mount a host agent socket, live keyring,
repository, user configuration or a directory containing private keys. The
only secret projection is one sealed, selected private-key file. A separate
sealed input holds fixed reset, key selection, SHA-256 hash selection, sign and
exit commands. No shell interprets input.

Obtain the private-file keygrip only from the exact eligible fingerprint's
public listing, using `--with-keygrip` in the existing offline worker. The
inspection evidence already binds the complete listing. Missing, malformed or
duplicate selected keygrips leave public identity inspection usable but refuse
private use. Keygrips do not establish possession or usability of a private key.

Capture the selected file through held private directories with no-follow,
nonblocking opens. Refuse unsafe ownership or modes, hardlinks, symlinks,
special files, empty files and files larger than 128 KiB. Check cancellation
between bounded reads and recheck names, held metadata and directory identity
before and after sealing. Wipe the temporary heap buffer on drop. Capture is
not atomic against a concurrent same-user writer and does not promise secure
memory locking or erasure of kernel storage.

Use a distinct closed supervisor owner, exactly two payloads and held,
revalidated root-owned installed artifacts. The worker has a private temporary
home, no option files, no pinentry program, no smartcard daemon and no external
passphrase cache. Protected or externally backed keys may fail; there is no
interactive or host-agent fallback. Preserve resource limits, independent
bounded output, the withheld filter and whole-unit cleanup. Refuse cancellation,
expiry, malformed or partial responses, inquiries, unsuccessful exit and
uncertain cleanup. Diagnostic stderr never provides a signature.

The result is an **unverified signature expression**, not an OpenPGP packet,
a complete commit signature or authority to write an object. Packet/content
construction, independent cryptographic verification, native sandbox/artifact
admission and approval composition remain prerequisites for activation. The
existing direct signed-creation adapter stays dormant and is not replaced by
this primitive yet. No product route, approval or commit/push permission is
added. Disposable cryptographic fixtures are component evidence only.
T-303/T-306 remain open, without any new row or sub-row.
