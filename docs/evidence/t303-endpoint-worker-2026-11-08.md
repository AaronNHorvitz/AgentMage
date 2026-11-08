# T-303/T-306: explicit sealed sibling development worker

Date: 2026-11-08. Tier: core. Immutable predecessor:
`7aacd4261581a5d61fc1508af9dfe0beac22e255`.
Its independent launch review remains in flight at session start.
That review arrived during verification with **Verdict: PASS** and no high or
medium finding; its report is retained unchanged. Low L1/L2 stay in the same
rows; low L3's heading spacing is corrected here. No parent closes on that
predecessor's acceptance.
This is a non-closing increment in the existing parent rows. No row is added.

## Behavior and boundaries

The private endpoint launch document now accepts an explicit
`worker_trust: "sealed_sibling_development"`. Omission and `root_owned` retain
the original worker admission. There is no automatic fallback. The development
selection accepts only the exact read-only worker sibling of the canonical
running local API binary, through the original development snapshot mechanism.
The existing host selection shares the same closed launcher mapping.

The worker retains its original name, guest path, owner/mode/ancestry checks,
bounded copying, drift detection and immutable sealed descriptor. Production
manifest verification, system executables/libraries, runner and supervision are
unchanged. Both selections precede endpoint-key reads and authority creation.
The startup consent flag, per-advance model-disclosure consent and exact native
read approvals retain their existing meanings. No new execution authority,
model profile, permission mechanism, context or verifier is introduced.
Worker selection must be a JSON string; the underlying unit-enum object form
is explicitly refused. Startup progress and the selected address go to stderr,
so a refused launch does not emit service output on stdout.

Two selector components use first-party synthetic bytes without executing them.
They cover both launcher names, exact sibling binding, wrong launchers, foreign
and relative paths, aliases, missing/unsafe/non-executable siblings, immutable
snapshot contents after pathname mutation and continued production refusal.
A configuration component checks the default, both explicit values and invalid
types/names. A real executable integration refuses an arbitrary worker path
before credential access, state creation or model contact. These are components
and executable refusal evidence, not successful worker/model execution.

Native success remains blocked by the lane's fixed system-executable ownership
admission. Edit/test/retry, durable recovery, approved Git and parent measurements
remain open. No evaluation inputs, hidden/held-out tests, scoring thresholds,
dependencies, model/GPU work, SBOM, status model, release or push changes.

## Verification

The complete affected-package baseline ran before the Rust edits: 25 targets,
1042 passed, 72 failed, 63 ignored, zero filtered. The failures remain failures;
this is not a whole-source pass or a waiver of deferred T-301.1.
The initial complete gate added three passing components and one failing new
executable assertion: the existing startup banner was written to stdout. Its
raw failure is retained. The banner now goes to stderr; the assertion remains.
Source review also identified the unit-enum object representation; a positive
typed-deserialization control and explicit string-shape refusal cover it.
The final complete affected-package gate ran all 25 targets: **1046 passed,
72 failed, 63 ignored, zero filtered**. All 1042 baseline passing identities,
all failing/ignored identities and every normalized failure diagnostic are
preserved, plus exactly the four new passing methods. Formatting, strict Clippy,
changed-document/review lint and hygiene pass. Seven commands took 134.51 seconds
including their build reservations; all five Rust input hashes stayed unchanged.
Complete raw baseline, initial and final logs and source snapshots are retained
privately. The source gate remains **FAIL on the 72 pre-existing failures**;
there is no deferred T-301.1 waiver, native workflow or model-quality claim.

Structural checks confirm that production manifest verification, shared system
admission, snapshot construction and sandbox supervision remain byte-identical.
Existing API test bodies and ledger table rows are unchanged. The only launcher
main change directs its existing endpoint startup banner to stderr. The new
positive unit-enum representation control passes while the public configuration
loader rejects that non-string form. Independent core review is required.

The guide's exact offline build command also successfully built the actual
local API and read-only worker binaries together. This is build evidence only;
no worker or model was executed. Native service success remains unobserved.
