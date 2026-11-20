# T-303/T-306: exact inventory verification correction

Reporting commit: `94f1889700d2c0aadc8dec1639b2b7e952ad8b9b`.
Frozen product source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.

The completed raw-source repeat remains 180 attempts and 103 strict ungraded
proposals. The largest diagnosed refusal remains 36 absent original fragments.
Task passes and retries remain unavailable. No model input, output, strict edit
check, frozen task or hidden test changes, and no model attempt is rerun.

A builder inspection found a general inventory gap after the reporting commit,
before its independent review request. The cached verifier used a maximum
Unicode character as a prefix upper bound. A filename beginning with that
character followed by more text sorts beyond that bound. This could omit an
undeclared file from a root or nested inventory count, or reject a valid declared
file. The earlier eleven controls missed these cases. This correction supersedes
the earlier general equivalence claim for the cached inventory implementation.

The corrected check covers the entire list for the root prefix. Every other
prefix ends in an ASCII slash; its immediate successor bounds every possible
suffix. All original nested inventories, SHA-256 digests, byte lengths and
file-identity drift checks remain enforced. Sixteen controls compare the original
uncached, faulty cached and corrected implementations. The original and corrected
checks agree in every case; the new controls expose two false accepts and two
false refusals in the faulty cached implementation. A neighbouring prefix is
also covered. These are verification controls, not coding-task grades.

The corrected verifier authenticates all 1,995,948 files in the original capsule
and its complete nested bindings, including the exact original postcommit
envelope. The original capsule, commit and envelope remain unchanged. Its
pending review finalizer was deliberately held; its exact script bytes, the
hold receipt and the expected missing-script exit are retained. No independent
review was requested for that incomplete finalization.

The review supplement contains the corrected verifier, regression controls,
correction records and final public files. Verification requires both the
original sealed capsule and this supplement. Every original input remains
bound and checked; the supplement adds its own complete file inventory. A real
detached Git view supplies the original reporting commit, with read-only object
storage from the caller's repository and optional Git writes disabled. The
current review commit is separately checked against the supplement envelope.
No branch, published commit, review report or historical measurement is rewritten.

The earlier portable reproduction remains unchanged: 30 contexts, 161 prior
strict checks, 165 current strict checks and 15 generation refusals. It executes
neither candidates nor inference. The correction uses no GPU. Reporting gates,
privacy/licence, preservation, supplemental authentication and the corrected
complete original-capsule verification are required before requesting core review.
The inherited source-gate failures remain; no source suite, SBOM, long acceptance
gate or release runs. T-303/T-306 remain open. Next work investigates the 36
absent-original refusals under MEASURE FIRST.
