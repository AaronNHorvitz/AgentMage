# T-306: Separate signature-verification status from diagnostics

This repairs low L2 in the retained
[signature-verification review](../reviews/530aec2c97ab-local-coding-confined-verification.txt)
within the existing T-303/T-306 rows. No new row or sub-row is created and neither
parent closes. The immutable base is
`537d80000934ce4e5fe7ba474c94e40d54bb695f`, under independent core review at
session start. No reviewed ancestry is rewritten.

The fixed public-only GnuPG verification command reserves stdout for machine
status with `--status-fd=1`. Bounded stderr remains diagnostic only. Success
requires complete newline-terminated status records, exactly one valid signature
for the complete pinned fingerprint, no failure status, successful process exit
and supervision, and unchanged cancellation/deadline conditions. Human lines,
empty records, partial records, carriage returns, NUL and invalid UTF-8 refuse
verification. Both retained streams still require matching byte counts and
SHA-256 digests within the existing bounds. Neither valid-looking nor
failure-looking diagnostics can supply or override a machine-status verdict.

Two new component methods exercise status/diagnostic swaps, missing or forged
status, misleading diagnostics, mixed output, malformed framing and complete
ordinary status records. The existing result matrix now checks both streams'
lengths and hashes independently. The disposable installed-Git/GnuPG matrix uses
the same new fixed arguments, reads machine status from stdout and observes
separate real human diagnostics. Its existing two object formats, exact primary
and subkey selection, message cases, wrong-key rejection, tampered-message and
invalid-public-key rejection, unchanged branch and absent private-key/agent
assertions remain. These are component and cryptographic evidence, not native
supervisor, artifact, approval, model or executable workflow admission.

The focused run passes all 16 selected methods with 488 filtered, no failures
or ignores: 4.16 seconds compile and 3.77 seconds tests, after the shared build
reservation wait. The initial focused compile failed on a test-only slice-type
inference error; explicit slice conversion and collection type correct it without
changing the production parser or suppressing a check. Both private raw logs
are retained. Formatting and initial documentary checks pass. This is not an
unfiltered complete affected gate.

The complete affected gate remains **FAIL**: 1,370 Rust passes, the exact same
72 inherited failure identities and 64 existing ignores across 26 unfiltered
targets, with zero measured or filtered tests. Both new methods and eleven named
controls pass. Formatting and Clippy pass, and all three source hashes remain
fixed. The run takes 230.32 seconds including reservations: formatting 1.45,
Clippy 9.48 and Rust 217.87 seconds. No command times out. Raw log SHA-256:
`4a132b1c74f06e6fd90fe897cae0f35f4191b789b307acdabf58c89cc711a018`.
Exact per-target comparison returns zero as data only, never a passing full gate.
No source edits follow this complete gate. Final documentary and public-content
checks cover the completed evidence; no task-table row changes.

The public-only projections, offline socket-denying filter, original commit
deadline, resource limits and whole-unit cleanup are unchanged. External signer
inspection and signed creation still launch directly in dormant code; confinement
remains required before activation. Native workflow, candidate/reviewer/artifact
authority, exact approval composition, approved commit/push, durable recovery
and native/model measurements remain open. Other retained low findings stay
same-row backlog. No evaluation, frozen or hidden task, held-out data, scorer,
prompt, threshold, dependency, supply-chain output, lifecycle, scoreboard,
release or owner-ready change. No GPU, download or push. Independent core review
is required.
