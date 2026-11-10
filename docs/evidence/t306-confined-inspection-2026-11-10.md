# T-303/T-306: offline public identity inspection

Status: complete affected gate remains **FAIL**; independent core review required.
Both parent rows remain open. No new row or sub-row is added.

The retained candidate-setup report has **Verdict: PASS** for exact commit
`537d80000934ce4e5fe7ba474c94e40d54bb695f`, with no high or medium finding. Its
low L1 post-capture mode regression is added to the existing test method in this
batch. The initial-open failure branch remains a disclosed test-coverage backlog.
The immutable verification-status base was under review at session start; its
report arrived during the batch with **Verdict: PASS** for exact commit
`f8236c01f04ccb92e0bae03caa546151efcb6f52`, with no high or medium finding.
Both reports are retained byte-identically. Neither parent row closes.

[Decision 0193](../decisions/0193-public-only-external-signer-inspection.md)
moves production inspection to two offline workers: public-key listing followed
by export. Only a sealed bounded public database enters either worker. The
external directory is held during capture; symlinks, special files, unsafe modes,
empty files, changed metadata and files over four MiB refuse. Neither private
keys, live directories, configuration nor host agent sockets are projected.
`pubring.kbx` has precedence; only absence permits `pubring.gpg`. Keybox-daemon
stores are unsupported. Snapshot capture is not an atomic same-user transaction.

The exact selected fingerprint must belong to its own public key/subkey record,
with its own signing capability, no revocation, disablement or expiry, and a
current primary key. A fingerprint in a user ID, prose or stderr cannot qualify.
Both workers use the same sealed database and original thirty-second deadline.
Only complete, consistently bounded and hashed successful output after supervisor
cleanup is accepted. Cleanup uncertainty keeps its redacted failure class.
The evidence digest binds the snapshot, listing, public export and fingerprint.
Root-owned artifact admission, withheld filter, socket denial, resource bounds
and whole-unit cleanup remain required. There is no production direct fallback.
The legacy synchronous constructor uses an internal cancellation token; external
cancellation control is not yet exposed through that constructor.

This pins **public identity**, not secret-key possession, signing availability
or usable private-key admission. A reserved disposable probe confirmed that
installed GnuPG secret-key listing with `--no-autostart` can exit zero with no
keys after its agent stops, while public listing succeeds. Connecting a host
agent or relaxing the worker filter is not an acceptable workaround. Signed
creation remains direct and dormant pending a confined usable signing mechanism.

Seven new inspection methods and one new closed-owner method cover sealed
capture, refusal cases, exact/current signing identities, ordered dispatch,
failures, cancellation/deadlines, output validation and fixed launch construction.
The real-GnuPG method generates disposable keys for setup, then stops the agent
and tests the fixed public-only options for exact primary/subkey selection over
both keybox and packet databases. Its supervisor results are synthetic; no native
artifact or sandbox admission is inferred. The existing installed-Git signature
matrix retains all earlier cryptographic assertions; its direct inspection helper
is now test-only. Component evidence is not an approved commit workflow.

Native artifact admission, confined signed creation, usable private-key access,
candidate/reviewer/artifact authority, approval composition, approved commit/push,
durable recovery and model measurements remain open. No evaluation task, frozen
or hidden test, held-out input, scorer, prompt, threshold, dependency, SBOM,
lifecycle, scoreboard, release or owner-ready artifact changes. No GPU, download
or push is performed.

Focused final validation: eight passes, zero failures or ignores, 504 filtered
methods; compilation 6.84 seconds and tests 0.05 seconds after waiting for the
shared reservation. This is filtered component validation only. The initial
focused compile failed on a private sandbox-error constructor referenced by the
new test. The test now exercises cancelled/timed-out results and uses the
existing dispatch seam for cleanup-uncertainty propagation, without widening
the sandbox-error constructor's visibility. The shared executable manifest
intentionally exposes its verified artifacts only to sibling local-commit modules. Both focused logs remain in the private review bundle.
The preliminary Markdown lint caught one extra blank line, which is corrected;
its successful rerun is retained. Complete validation follows with source fixed.

## Complete affected validation

The final unfiltered fast gate remains **FAIL**: **1,378 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 targets. The failure and ignore
identities match the immutable verification-status baseline exactly. No new
failure or ignore is introduced or omitted. All eight new methods and ten named
controls pass, including the strengthened existing candidate mode case and the
installed-Git cryptographic matrix. Formatting and Clippy pass. All six source
hashes remain fixed throughout the complete run. No source edits follow it.

The run took 309.98 seconds including shared reservations: formatting 1.51,
Clippy 15.35 and the complete Rust suite 291.44 seconds. There was no timeout.
The retained raw log SHA-256 is
`53d8cef0d6ac2006c0dd0c57c150b1656efa7ab95c2cdd6c8ec05b39382fbc92`.
The private bundle is `/lane/state/tmp/t306-confined-inspection`; `run.py`
reserves heavy commands internally and must not be outer-wrapped. Its `check.py`
returns zero only for an exact per-target data comparison; the complete gate
still fails. No GnuPG agent process remains in this session's namespace after
validation. Final documentation, public-content, unchanged-row and staged-blob
checks accompany the commit. The existing native artifact/user-bus boundaries
remain; this is not native workflow or private-key admission.
