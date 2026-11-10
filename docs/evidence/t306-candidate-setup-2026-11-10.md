# T-306: Bind candidate setup to a held directory and roll back empty failures

This is a same-row T-303/T-306 repair of low L1 and L2 in the retained
[candidate-confinement review](../reviews/de4f3fd311de-local-coding-confined-candidate.txt).
The immutable base is `530aec2c97ab745211251566433938f22231ec9e`, which is
under independent review at session start. Its **Verdict: PASS** with no high
or medium finding arrived during validation and is
[retained unchanged](../reviews/530aec2c97ab-local-coding-confined-verification.txt).
Low L1 native verification/tamper coverage and L2 dedicated machine-status stream
remain same-row backlog with the other signer findings. No ancestry is rewritten. Neither parent row closes,
and no new row or sub-row is created.

After exclusive directory creation through the held parent, setup now opens
the child with directory, no-follow and close-on-exec flags through that same
descriptor. It holds this child through worker capture, requires the captured
worker descriptor to have the same device and inode, and revalidates both roots
and the child before returning a usable session. No worker can launch from a
failed constructor. The production path still verifies and captures the scope;
the private injected capture callback is a deterministic component-test seam.

A setup guard attempts nonrecursive empty-directory removal on failure only
while the original roots still validate and the named child matches the held
device/inode, current owner, directory type and private permissions. It deletes
no files. Unknown contents, replaced roots or child, changed permissions, failed
identity checks or a failed initial descriptor open retain scratch. Successful
setup disarms the guard and leaves cleanup to the existing session owner.
This does not provide an atomic filesystem transaction against same-user host
processes: exclusive creation and initial open, or final identity check and
removal, remain separate system calls. Such interference is outside the guest
boundary; it does not grant a replacement worker scope authority.

Four new component methods exercise failure before/after scope verification,
fresh construction after empty rollback, empty directory replacement, symlink
replacement, parent replacement, nonempty claims of several types, changed
permissions, and a foreign captured descriptor. Replacements are empty so the
identity assertions cannot pass merely because a sentinel prevents removal.
The foreign descriptor test also proves that rollback does not touch that scope.
Original source and user index preservation are checked throughout. Existing
ordered-worker, exact-byte, cancellation, deadline and cleanup tests remain.
These tests are component filesystem/constructor evidence, not native sandbox,
model, approval or executable workflow admission.

The focused component run passes all four new methods with 498 filtered,
no failures or ignores (6.01 seconds compile, 0.00 seconds tests). Formatting
passes. Initial whitespace and documentary checks pass. The focused run is not
an unfiltered full gate.

The complete affected gate remains **FAIL**: 1,368 Rust passes, the exact same
72 inherited failure identities and 64 existing ignores across 26 unfiltered
targets, with zero measured or filtered tests. All four new methods, eight
candidate controls and the socket-policy control pass. Formatting and Clippy
pass, and both source hashes stay fixed. The run takes 364.58 seconds including
reservations: formatting 129.71 seconds (including waiting), Clippy 12.78 seconds,
and Rust 220.55 seconds. No command times out. Raw log SHA-256:
`b792ab5c284eafcfba97e3eb2da53255baca129bc73f57f7e3a3cf0f8d51d84e`.
Exact per-target comparison returns zero as data only, never a passing full gate.

The base review arrived after the full gate plan was captured. Final light
checks cover its retained report and the completed evidence; the affected Cargo
commands and both source hashes remain unchanged. The report is byte-identical,
the task-table rows are unchanged, and public-content checks pass. No source
edits follow the complete gate.

Signer inspection and signed creation confinement, candidate/reviewer/artifact
authority, exact approval composition, approved commit/push, durable recovery,
native workflow admission and model measurements remain open. No evaluation,
frozen or hidden task, held-out data, scorer, prompt, threshold, dependency,
supply-chain output, lifecycle, scoreboard, release or owner-ready change.
No GPU, model download or push. Independent core review is required.
