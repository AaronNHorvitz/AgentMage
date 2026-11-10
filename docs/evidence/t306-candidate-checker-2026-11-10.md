# T-303/T-306: original-owner candidate checker connection

Status: independent core review required. T-303/T-306 remain open without new
rows or sub-rows. The preceding commit and all ancestors stay immutable.

The existing selected two-root review view now accepts an explicitly installed
trusted checker. It first performs the complete live-owner candidate-plan checks,
then provides borrowed references to the exact packet, inert candidate plan,
original native creation identity and complete original validation history.
The checker cannot replace these references with serialized receipts. The
process-only result borrows the original owner and packet and exposes an
immutable packet and plan. It has no public constructor, cloning or restoration.

A rejection or unavailable checker stops immediately without post-check reads.
Success repeats all original-owner plan checks and requires identical plans.
It also compares every full private Git observation across all four pre/post
samples: original preservation manifest, task head and branch, linked identity,
task inventory and immutable base preimages. Comparing plans alone would omit
some of these observations. Source/mode, original-write history, validation and
held-root checks remain required. Cancellation checkpoints surround each Git
observation and the checker call. They cannot interrupt a blocking checker or
individual filesystem call; the host must bound its own checker work.

This is an opt-in library connection, not an installed independent reviewer.
The trusted host must select its checker outside model/tool input; the checker
must independently check task criteria, complete findings and trusted capture/
scanner evidence for every artifact claim. There is no default accepting
implementation, and the interface cannot authenticate an installed checker or
prove that it followed its contract. The old inert planner remains explanatory.
No executor consumes the new result; no HTTP, CLI or MCP route is added.
Native admission, actual checker/artifact integration, complete exact product
approval composition, durable recovery and model measurements remain open.

## Validation

Six new component methods pass in the focused run: 6.60 seconds of tests after
7.46 seconds of compilation and the shared reservation. They demonstrate exact
borrowed packet/creation/validation references; successful pre/post correlation;
checker rejection and unavailability without further reads; packet-integrity
refusal before checker/Git; source, executable-mode and both-root drift refusal;
18 complete-observation drift cases across three sample positions; six
cancellation boundaries; preserved original bytes and retained receipt/history
records; and unchanged native artifact admission with no checker invocation.
No candidate scratch is created. Real native execution is not demonstrated.

The initial compile caught a temporary review-view lifetime tied unnecessarily
to the returned value. The result now borrows the original owner and packet;
its lifetime need not retain the temporary selected view. The focused rerun
passes all six methods. Both logs are retained; no claim is made that the first
attempt passed.

The complete affected fast gate remains **FAIL**: **1,427 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 unfiltered targets. Every
per-target failure and ignore identity exactly matches the retained baseline;
the only test-count change in those targets is six additional Linux passes.
All six new methods and 24 existing review methods pass. The preceding change
also selected twenty kernel-only targets; these are outside this batch's
Linux/host/local-API source selection, and are explicitly excluded from the
baseline comparison rather than represented as rerun.

Formatting and Clippy with warnings denied pass for all three affected crates.
All four source hashes stay fixed with no source edit after the freeze. The
full run takes 298.93 seconds including shared reservations: formatting 4.42,
Clippy 24.22 and Rust tests 268.75 seconds. No timeout occurs. The raw log
SHA-256 is
`1b5c1b35ce1834c9ad1c32df95a7ec3e20764936db6e4fb82c169b6b3c010000`.
Raw attempts and reproduction scripts are retained in the private bundle
`/lane/state/tmp/t306-candidate-checker`. Its runner reserves heavy commands
internally; never outer-wrap it. Its comparison script asserts the complete
gate disposition and compares exact per-target failure/ignore identities; exit
zero from that script is data comparison only, never a passing gate.

Final documentation, public-content, unchanged-row and staged-blob checks pass.
The preceding commit remains in independent review; no verdict is claimed for it
or this increment. Neither parent row closes.

No evaluation inputs, hidden/held-out tests, scorer, prompts, thresholds,
dependencies, SBOM, lifecycle or scoreboard change. No GPU, host keys, model
download, push, release or readiness assertion is used.
