# T-306: Confine dormant candidate-tree construction

This same-row coding increment implements Decision 0190 on immutable base
`a0acdf0e978d80c07c540da18c1d4d679910c0e2`, which is under independent core review
when work begins. Its review arrives during verification with **Verdict: PASS**,
no high or medium finding; it is retained byte-identically. Its low L1 is
clarified in the prior evidence: the moved concurrency seam remains unexercised
behind native admission. Low L2 is addressed by documenting the existing
explicit work-tree override and asserting it in the inert worker-plan test.
T-303/T-306 remain open. No row or sub-row is added or closed.

The dormant candidate builder now uses the existing offline closed Git worker
instead of its direct process runner. Original source is read-only, exact
selected metadata remains writable, and only an exclusively created private
candidate directory is the third writable projection. The enclosing owned root
and its siblings are not projected. Both original and candidate directory
identities are held and checked. Exact candidate plans bind the new index path;
occupied or stale flat-index paths refuse. Duplicate postimage identities refuse.

A bounded session orders parent initialization, raw postimage hashing, exact
path/mode updates and final tree construction. Postimages use sealed, rewound
memory stdin, separate from the seccomp filter. Each returned blob must match
an independently computed object ID for its exact bytes and object format.
Sparse/split indexes, filters, background work and network remain disabled.
Any failure ends the session. Known scratch files are removed through held
roots only after certain supervision; unknown contents or changed roots are
retained. One thirty-second deadline covers all candidate steps.

Eight new methods exercise synthetic worker results, sealed real memory input,
private directory creation, inert launch configuration and refusal behavior.
They do not establish positive native worker admission. The unchanged native
executable-ownership and user-bus boundaries remain. The signed local-commit
runner remains dormant and direct, requiring confinement before activation.

An early focused diagnostic run on the six-method source snapshot
reports five passes and one fixture failure: after replacing the parent
scratch root, the test incorrectly expects the old child pathname to exist.
The corrected test checks the retained child and the replacement sentinel.
No preservation assertion is dropped. That diagnostic run is not final-source
verification.

The first standalone synthetic mount experiment fails its user-index comparison:
its host-side status observation follows the saved index snapshot. The rerun
removes that intervening refresh and passes for both SHA-1 and SHA-256. Each
builds one edited and one new executable file with exact raw blob identities
and verifies the complete selected contents/modes. Original source, user index,
refs and sentinels remain unchanged. A sibling scratch file, an outside-file
symlink and a source write are refused. Hostile filter and hook configuration
does not transform the approved bytes or overwrite the sentinel. This bypasses
product artifact admission and uses bubblewrap directly; it does not exercise
systemd, seccomp, the complete product supervisor or native workflow admission.
Both experiment attempts and raw logs are retained privately.

The first complete affected gate remains **FAIL**: 1,333 Rust passes, the same
72 inherited failures and 64 ignores across 26 unfiltered targets, with all six
then-present new methods passing. Formatting and Clippy pass. This run is not
final-source verification: the arriving review's work-tree assertion changes a
source hash during verification. It also reveals that overlapping formatting
and editing had dropped two added tests. Both tests are restored, formatting
is completed sequentially, and a fresh seven-file source freeze is required
for final verification. No source-change or missing-test check is waived.
That diagnostic gate takes 279.58 seconds including reservations; its raw
log SHA-256 is
`c0bdfb6814c3499b8a26e721fb53c16b2eece3832a6bdbe24269ea24ea45c4c8`.

The final fixed-source complete affected gate remains **FAIL**: 1,335 Rust
passes, the exact same 72 inherited failure identities and 64 existing ignores
across 26 unfiltered targets, with zero measured or filtered tests. All eight
new methods and seven affected controls pass. Formatting, Clippy and documentary
checks pass. All seven Rust source hashes remain unchanged during verification.
The run takes 278.19 seconds including reservations:
formatting 52.62 seconds, Clippy 2.56 seconds and Rust tests 221.29 seconds.
No command times out. The preceding preparation formatter's separate reservation
wait is outside that timed gate. Its raw log SHA-256 is
`04227c2af78c0e42b257f30e7c0250a51a2e3a3c8178e850bac4ef2f4550d165`.
The exact per-target comparison returns zero as **data only**; it cannot turn
the complete gate into a pass. The final-source check also requires both
restored methods and all seven source hashes. No failed or ignored test changes.
Final prose lint, public-content checks, unchanged task-table comparison and
byte-identical retained-review checks pass after the test run.

The successful standalone experiment command-log SHA-256 is
`9db3bb0b7542b38eb432551748072775d50720315eef0bf15403e582eb26d36d`.
No model or GPU process starts. Frozen, hidden and held-out tasks, scorers,
prompts, thresholds, dependencies, supply-chain outputs, lifecycle and scoreboard
are unchanged. No candidate API/coordinator activation, independent reviewer
provenance, approved commit/push execution, durable recovery, release or owner
readiness is claimed. Independent core review is required.
