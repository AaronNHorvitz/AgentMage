# T-303/T-306: confined signed-commit storage

Status: complete affected gate remains **FAIL**; independent core re-review required. Neither parent closes;
no row or sub-row is added. The signature-assembly base is immutable. Its retained
[review](../reviews/5d2184140fa2-local-coding-commit-packet.txt) has
**Verdict: FINDINGS** for exact commit
`5d2184140fa2b378584a2e327e4311129ce709ae`. This same-row batch addresses
medium M-1 and low L-3; independent re-review is required. Earlier same-row low
findings remain backlog. The report is retained byte for byte.

M-1 repairs a wrong assumption: unsigned RSA values from the agent can have
leading zero octets. The bounded parser now strips that padding before minimal
MPI encoding, while empty/all-zero values and ambiguous expressions still
refuse. Deterministic padded forms must produce identical packets; an injected
zero-led form of each real captured signature must independently verify.
L-3 refuses angle brackets, leading/trailing name whitespace and whitespace in
email identities before signing. Accepted identity and message bytes remain
exact. Actual installed-Git storage/readback tests cover those accepted bytes.
The earlier evidence's minimal-scalar claim is explicitly corrected, preserving
its historical run counts and failures.

[Decision 0196](../decisions/0196-confined-signed-commit-storage.md) replaces
direct signing in the dormant executor with the socket-free signer, RSA
assembler and public-only verifier. The old direct adapter is test-only.
The assembler returns an opaque process-local value bound to the complete
plan, with the exact independently computed commit object identifier. It has
no public constructor or deserialization and grants no approval authority.

The closed Git session observes the expected parent, writes only sealed
verified bytes, requires the exact object identifier, reads the object back
byte for byte and then admits the existing task-branch compare-and-swap.
Another public-only verification precedes the ref update. Every stage checks
control, held roots, cleanup, bounded complete output, process status and stream
digests. Failures end the session without retry. The existing kernel launch
permit, preflight preservation check and final reconciliation remain in place.
No keyring, credential, network or host socket is projected to the Git worker.
The direct immediate-child capture plumbing is also compiled only for tests.
Its controls keep the existing refusal and cleanup behavior.
A partial storage failure may leave an unreachable object; its effect is
uncertain, not rolled back or asserted absent.

Deterministic tests use synthetic signer/verifier and supervisor outcomes to
exercise the closed state machine. They are not native execution, original
approval or independent-review proof. Disposable RSA primary/subkey signatures
are independently verified, stored with installed Git in both object formats,
and read back exactly; no ref moves in those component tests. Native artifact
admission, complete product approval/reviewer/artifact provenance, durable
recovery, measured coding acceptance and approved push remain open.

No evaluation, frozen/hidden/held-out input, scorer, prompt, threshold,
dependency, SBOM, lifecycle, scoreboard, release or owner-readiness change.
No host keys, GPU, download or push is used.

## Validation

The final complete affected fast gate remains **FAIL**: **1,405 passed,
72 failed, 64 ignored**, zero measured or filtered, across 26 unfiltered targets.
Every per-target failure and ignore identity matches the frozen reviewed base;
only six passes are added to the Linux library. All six new methods and 45
named controls pass. Formatting and Clippy pass for all three affected crates.
All seven changed source hashes remain fixed; no source edit follows this run.

Final validation 3 takes 223.33 seconds including reservations:
formatting 27.42, Clippy 2.46 and Rust tests 191.83 seconds. No timeout occurs.
The raw log SHA-256 is
`2f895843734ef264eb4b42672d11cac2161e68b3673c43d9d185f6e4e6d81dec`.
The private bundle is `/lane/state/tmp/t306-confined-storage`. Its `run.py`
reserves heavy commands internally and must not be outer-wrapped. The
`check.py` exit zero is an exact per-target **data comparison only**, never
a passing gate. Final documentation, public-content, unchanged-row and staged
blob checks follow without further source edits.

The final focused run has eight packet passes with 531 filtered tests, and ten
I/O passes with 529 filtered tests; both have zero failures or ignores.
Compilation takes 3.23 plus 0.03 seconds, and tests take 0.36 plus 0.03 seconds.
The isolated capture-control diagnostic also passes (one pass, 538 filtered,
2.27 seconds compilation and 3.08 seconds tests). These filtered runs do not
replace the complete final unfiltered validation.

Superseded full validation 1 records 1,405 passes, the exact inherited 72 failures
and 64 ignores, but also seven new Clippy dead-code errors after direct creation
became test-only. Those helpers are now excluded from production with
`cfg(test)`; no warning suppression or control body change is used.
Superseded full validation 2 has formatting and Clippy passes but 1,404 tests
passed, 73 failed and 64 ignored. The additional failure is the existing direct
capture caller control. The public-inspection fixture also uses that process-wide
single-child capture owner but lacked its shared test mutex. It now takes the
same existing fixture lock as the other direct-capture users. Production
refusal, parallel test thread count, timing limits, assertions and thresholds
are unchanged. Both failed runs and the diagnostic recheck are retained; no
result is discarded or relabelled success.
