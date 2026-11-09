# T-303/T-306: Repair sealed-command supervisor admission

Date: 2026-11-09. Base: `a5594787b4ec4f25553332c63a91d14cad91e4ee`.
Tier: core. Both parents remain open; no row or sub-row is added or closed.

## Finding and repair

The [independent sealed-validation review](../reviews/dc5cb4532d97-local-coding-sealed-validation.txt)
has **Verdict: FINDINGS** for exact commit
`dc5cb4532d97431fef5f58fe7f8a12c266c1ab19`. H1 correctly identified that
supervisor command admission allowed at most one projection. Every sealed
capture containing two or more regular files failed before spawning, consuming
an admitted validation attempt without running the tests. Earlier statements
about native sealed retention and execution were therefore unsupported.

The closed command owner now carries an explicit source projection policy.
Ordinary commands retain the existing zero-or-one limit. Sealed commands require
exact equality with the complete capture count and enforce the original
128-file maximum, including zero for an empty tree. The executor derives this
count from the same capture input record it retains for the original command
receipt, before transferring that capture's descriptors. Unit names retain the
original exact prefix and nonce validation. Other launch owners, the command
manifest, approval, permit, original supervisor, withheld seccomp filter,
resource bounds, cancellation and cleanup remain unchanged.

A non-ignored admission regression uses actual sealed captures containing zero,
two and 128 regular files, passes their exact counts through the production
command admission policy, and rejects wrong counts, wrong unit names, 129 and
unbounded counts. It also preserves ordinary command admission. The existing
host-dependent closed-owner matrix now covers sealed commands separately.
A new host-dependent native regression runs a two-file worktree through the
original executor and supervisor: reads both files, refuses a write to `/work`,
checks known descendant cleanup and the exact guest descriptor table. It also
checks the sealed count on each original result. It has the same supported-host
requirement as the existing native launch tests; it is not replaced by a fake
launcher or weaker native manifest.

## Receipt contract repair

The [receipt-connection review](../reviews/a5594787b4ec-local-coding-sealed-input-receipts.txt)
has **Verdict: FINDINGS** for exact commit
`a5594787b4ec4f25553332c63a91d14cad91e4ee`. Its H1 is the same admission
defect. M1 correctly identifies that the published closed command-receipt schema
did not admit `sealed_workspace_input`, although the native tools bind that
schema as their output contract. This batch adds the optional closed property,
its required nested fields, version, digest shapes and original numeric bounds.
Zero files require zero bytes; launch failures cannot carry a sealed input.
Legacy receipts keep the absent field and their canonical bytes. The published
schema and its embedded host/native-tool digests now include the extension;
external clients retaining the old closed schema still need updating.

The schema tests accept the legacy example, an extended example and exact empty
and maximum-count boundaries. They reject unknown nested keys, missing fields,
invalid digests/version, non-integer and excessive counts, invalid directory
counts, invalid bytes, a zero-file/nonzero-byte record, null and launch-failure
records. The fast gate explicitly registers this schema, runs its consumers,
checks host dependants and reproduces producer fixtures. Its test now checks
both receipt schemas. Producer fixtures are regenerated once after these source
edits to retain the current command-tool definition digest; historical versions
stay unchanged. The native regression also compares the entire returned input
record with a separate complete unchanged-tree capture.

## Verification

The complete final scoped gate remains **FAIL**: 1,214 passed, exactly the
72 inherited failures and none new, 64 ignored (63 inherited and the new
supported-host native regression), zero measured or filtered across 26
unfiltered Rust targets. All nine named component/admission/workspace methods
pass, including the new real-capture admission regression. The failure identities
match the unchanged T-301 baseline exactly. A passing comparison is not a
passing full gate.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Schema tests, all 79 | 0 | 1.80 |
| Gate/producer unit tests, all 36 | 0 | 2.71 |
| Read-only producer reproduction, nine files | 0 | 121.91 |
| Formatting | 0 | 1.20 |
| Clippy, all affected targets | 0 | 9.12 |
| Rust tests, all affected targets, no fail-fast | 101 | 238.26 |
| Complete scoped gate | 1 | 377.03 |

Fourteen source/contract/fixture input hashes stayed fixed throughout the final
run: three Rust files, the receipt schema, three gate/consumer test inputs and
seven changed current producer fixtures. The producer regeneration ran once
after source edits, writing nine current files; seven differ only in derived
hashes. No literal copy of the old schema hash remains in current repository
files. Both independent reports are retained byte-for-byte. The private final
runtime namespace retains exactly eleven inherited-failure fixture directories.
The raw final gate log SHA-256 is
`681c01046d4207febe72e9257ee39170ef681abf2451da444287afb884134564`.

Both explicit supported-host test attempts exit **101** at trusted-manifest
verification before a sandbox launch. The new native launch test reports
`LinuxCommandRunnerError { kind: InvalidManifest }`; the updated closed-owner
matrix reports `LinuxSandboxError { kind: InvalidManifest }`. Each has zero
passes, one failure and 392 filtered tests. Their individual elapsed times,
including reservations, are 225.78 and 0.06 seconds. Neither reached its native
assertions. The private collection helper's success means only that it retained
both exit statuses with unchanged source; it is not a passing native check.
Native sealed execution, read-only behavior, full returned-record agreement and
descriptor cleanup remain unobserved on this lane. The original executable
trust and supported-host requirements are unchanged; no substitute launcher,
trust relaxation, GPU/model process or download was used.

The earlier admission-only full run is retained as preliminary: 1,214 passed,
72 inherited failures, 64 ignored, no new failures, 278.55 seconds. The second
independent review arrived during that run, so the batch then added M1's schema,
tests and derived producer-fixture update and reran the complete final gate.
The first focused schema run failed during strict schema compilation because
its launch-failure exclusion used a `required` keyword without a local property
declaration. It was rewritten as the equivalent false property constraint;
strict validation stays enabled. The next focused run passed all 79 tests.
The private bundle retains the initial diagnostic and both complete gate logs.
No test condition or threshold was weakened. All Cargo commands used the shared
build slot, two build jobs and at most four test threads; waiting reservations
were honored. Final prose lint and comparison pass separately after this report
is updated; the full gate remains FAIL.

Sealed-validation low L1 displayed-bound duplication and L2 route-selection
test coverage, and receipt-review low L1 absent-input/non-worktree sealing test
coverage remain same-row backlog. Earlier low findings remain backlog. Neither completion
verifier changes. Both verification flags stay false. No native/model workflow,
model qualification, independent task grade or row acceptance is claimed.
The preceding in-flight commit and all earlier history remain immutable.
No hidden/held-out task, scorer, threshold, dependency, SBOM, status, scoreboard,
release or owner-test-ready change is made. Independent core re-review is required.
