# T-306: Original native creation through candidate review

Scope: T-303/T-306, core, non-closing increment atop immutable commit
`bac3866b9c48511fef1d51e3a2c8e86784faf718`. That commit is under independent
review when work begins. Both parents remain open; no row or sub-row is added
or closed. Independent core review is required before acceptance.

The kernel's opaque filesystem apply authorization now borrows the original
approval after grant consumption and its required checkpoint. The existing
native Linux creation driver retains that approval and its exact immutable
creation plan. It takes preceding write history out of the owner before native
dispatch. Only matching checked committed completion after publication may
return that history with the original creation proof.

Checks correlate the transaction, original issued grant identity and revision,
consumed grant identity and next revision, approval binding, exact plan and
preview, verification labels, full receipt chain and every original creation
destination, postimage and mode. Repeated completion, missing or substituted
proof, failed or uncertain execution and unpublished dispatch cannot restore
earlier history. Other already-supported native creation shapes retain their
transfer-lineage behavior but cannot supply source-review proof or return
preceding review history.

The existing version 2 review contract now accepts a projection of nonempty
UTF-8 source-code creation. Zero preimage means absence, distinct from the hash
of an existing empty file. The complete escaped diff shows every created byte;
the original plan and operation digests bind its mode and classification.
Projection creates no grant, application approval or Git authority. Non-source
classifications, binary content, NUL and other filesystem operations refuse
this projection without changing the underlying filesystem transaction contract.

The same bounded flat history composes existing edits, creations and retries
without synthesizing an aggregate approval. Every original approval/application
identity remains separate. The same live owner requires complete first/final
lineage, current source bytes, both held roots and unchanged creation modes.
Losing an original creation proof cannot be repaired by a later ordinary edit.
The candidate planner compares the whole original projection and retains its
sealed-validation and fresh two-sided Git-observation requirements.

Two new kernel methods check exact absent/content projection, mode-bound
identities, incompatible classifications, binary/NUL/empty content, invalid
purpose coverage and other operation refusal. Five new Linux methods check
actual original native approval/receipt correlations, successful unsupported
creations without source proof, existing edit plus two creations and retries,
missing/unpublished history, mode drift and the complete mixed candidate plan.
The tests preserve the original repository and assert no candidate index is
created. The candidate fixture performs every edit before its first synthetic
validation; no production validation series is reset.

Native files, grants, creation/write transactions, receipt chains and sealed
input capture are actual component operations. Fixture repository creation,
Git observations, process results and the publication choice remain explicitly
synthetic. These tests demonstrate neither native Git nor actual terminal
publication, a real model workflow, a task grade or a coding score.

The complete affected gate remains **FAIL**: **2,939 passed, 72 failed,
72 ignored**, zero measured or filtered, across **46 unfiltered Rust targets**.
All forty-six targets match the parent's exact failure/ignore identities and
counts, with precisely two new kernel passes and five new Linux passes. All
seven new methods pass; no failure or ignore is added. This comparison does
not pass the gate or close either parent.

Formatting, Clippy with warnings denied and all lightweight checks pass. The
complete gate takes 478.33 seconds including reservations: formatting 15.33,
Clippy 21.43 and Rust compilation/tests 440.14 seconds. Its raw log SHA-256 is
`0a06322e4bd3d52a9e89907143cae8a89f83e9488d5516cba6f6316cbc43365c`.
All twelve changed Rust source hashes remain fixed throughout full and final
verification; no source edit follows the full gate's start. Final lightweight
checks cover the completed prose and byte-identical incoming review.

The reserved focused runs also pass all seven new methods and eight existing
new-file methods, without a failure or ignore. The mixed original-owner packet
passes the current version 2 schema and semantic consumer. Commands take
501.21, 21.10 and 0.40 seconds including reservations; these queue-inclusive
numbers are not runtime performance measurements. The complete unchanged Node
schema suite passes all 80 tests through the materialized shared binary, taking
223.87 seconds including reservation and 1.23 seconds in the test runner. Its
raw log SHA-256 is
`9287932d099db287e2a3c7051d1c682078707bd626bcd66dffbf1892875a33b8`.
No Node conformance claim authenticates review provenance or native authority.
Exact source records, target comparisons and raw logs are retained privately
for independent reproduction. The focused new-test bodies take 0.01 seconds for the kernel and 0.56 seconds
for Linux; the existing eight-method bodies take 3.71 seconds. The mixed native
edit/create/retry component measures 0.373426 seconds, including fixture setup,
projection and mode-drift checks. No model/task score is inferred from these runs.

Reviewer/artifact provenance, candidate execution, separately approved Git,
native/model measurement and durable recovery remain open. No new route,
terminal command, model request, grant, trust exception or sandbox relaxation
is introduced. Frozen/hidden/held-out tasks, scorers, thresholds, prompts,
dependencies, supply-chain outputs, status and the scoreboard stay unchanged.
No model download, GPU process, long acceptance gate, release or owner-ready
claim is involved.

During verification, the [base independent core review](../reviews/bac3866b9c48-local-coding-write-history.txt)
returned **Verdict: INCONCLUSIVE** for that exact base, with no high or medium
code finding. It reproduced the Rust/Python results and exact inherited failure
identities, but could not access the retained Node installation in its sandbox.
The report is retained byte-for-byte; its verdict is not upgraded. Its low L1
semantic-validator coverage and L2 repeated-content-history refusal findings
remain same-row backlog with earlier lows.

The existing licensed Node runtime, its license and original provenance are now
materialized inside the shared private review bundle, with exact byte hashes.
Private runners use that copy rather than an installation outside the shared
bundle. No runtime is downloaded and no dependency or source changes. Both
copies have identical bytes. Required Node schema and packet checks must be
reproduced there for the current independent review; no prior verdict is changed.
The initial lightweight guide lint's extra blank line was fixed before the
full gate, with no assertion or threshold change.
