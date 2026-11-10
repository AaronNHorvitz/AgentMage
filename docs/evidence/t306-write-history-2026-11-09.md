# T-306: Original native write history through retries

Scope: T-303/T-306, core, non-closing increment atop immutable commit
`b1b4454397c73f019be9c3655f51c5b65cab15f2`. Both parents remain open; no row or
sub-row is added or closed. The base is under independent review when work begins.
During verification, the [base independent core review](../reviews/b1b4454397c7-local-coding-runtime-write-provenance.txt)
returned **Verdict: PASS** for that exact base with no high or medium finding.
The report is retained byte-for-byte. Its low L-A durable host/publication
coverage finding remains same-row backlog with the earlier lows. No row closes.
Independent core review of this increment is required before acceptance.

The same live Linux owner now retains an ordered sequence of original checked
native write proofs across prepared edits, runtime handoff, published runtime
completion and return. Each proof retains its original shadow change set,
approval binding and checked application-chain digest. A prospective dispatch
takes previous history out of the owner; only matching checked successful
completion returns it with the next original proof. Uncertain, failed or
unpublished dispatch cannot restore history. Creation, original transfer and
poisoned recovery still retire it. No externally supplied receipt constructs proof.

The kernel composes a complete net diff while preserving every original
transaction projection in order. Each path must connect consecutive preimages
and postimages, with consistent classification and review purpose. The original
Linux capture still requires complete current edit lineage, fresh source bytes
and held roots. Candidate planning compares the complete projection to that
original capture and still requires original sealed validation plus fresh Git
observations before and after planning.

A composite contains no singular write approval or application receipt. Those
original digests stay separate in `write_history`; the composite change-set
hash identifies the ordered history rather than a new approval. Review packets
and their JSON contract use version 2. Single-transaction projection fields keep
their existing serialized shape. The fast gate now selects the existing schema
consumer tests for the exact changed schema, example and validator inputs.
No schema validation or integrity hash grants native provenance or permission.

Retention is bounded to sixty-four transactions and eight MiB of complete diff
text, within the existing sixteen-path limit. A bound overflow retires proof
rather than truncating earlier history. Missing, duplicate, reordered or nested
transactions, incompatible breadth/classification, net-zero paths and new-file
lineage refuse review. Saved records and a reopened store cannot restore it.

Three new kernel methods exercise original endpoint composition, mutation and
chain refusal, the sixty-four-transaction bound and the cumulative byte bound.
Four new Linux methods exercise mixed manual/runtime/manual retries on two
files, original manual application digests, missing/unpublished history,
sixty-five actual approved native writes without truncation, and the complete
candidate planner with fresh sealed input. Native files, grants, receipt chains
and sealed input capture are actual component operations. Worktree creation,
Git observations, terminal-publication choice and command/process results in
these fixtures are explicitly synthetic. No native Git, actual terminal
publication, model execution, task grade or coding score is claimed.

Two existing runtime regressions now require retained original history after a
published retry, preserve their missing-purpose refusal and check that composites
have no singular approval. Unpublished retry refusal remains asserted. The
existing later-transaction refusal now explicitly removes original private proof
before the next actual write, preserving the missing-history refusal. No failed
test or threshold is weakened; expectations change only for the supported history.
The candidate fixture performs both edits before its first synthetic validation;
it never resets a validation series or reruns after a full pass.

A preliminary private light-check runner mistakenly selected the Node schema
suite and Python fast-gate suite without a reservation: 80 and 27 tests passed,
respectively. Their raw log is retained; this was a lane resource-rule violation,
not acceptance evidence. The private light runner now excludes suites. The
public fast gate now reserves complete Node/Python suites as well as Cargo,
with an added regression for exactly-once reservation and lightweight checks.
All complete suites are reproduced through the reserved final gate.

The first reserved Rust attempt fails to compile a new test helper because the
workspace digest type does not implement the hex-formatting shortcut used there.
Its exact error log is retained. Replacing that shortcut with the existing
kernel hash helper changes no assertion or threshold.

The corrected reserved focused run passes all seven new Rust methods: three
kernel tests (1,576 filtered) and four Linux tests (424 filtered), with no
failure or ignore. The existing four runtime-provenance methods also pass
(424 filtered). The original-owner two-transaction packet emitted by the new
candidate regression passes the version 2 JSON schema and semantic consumer.
Commands take 41.26, 22.23 and 0.40 seconds including reservations; the first
Rust command includes 39.31 seconds of compilation. Kernel and Linux new-test
bodies take 0.89 and 1.02 seconds; existing runtime bodies take 1.15 seconds.
The mixed four-write native component measures 0.623891 seconds, including
fixture/setup, writes, projection and source-drift checks. None of these numbers
is a model/task score, actual command-test execution or native Git measurement.

The complete affected gate remains **FAIL**: **2,932 passed, 72 failed,
72 ignored**, zero measured or filtered, across **46 unfiltered Rust targets**.
Every target matches the preceding full gate's exact failure/ignore identities
and counts, with precisely three new kernel passes and four new Linux passes.
All seven new methods pass. No failure or ignore is added. The changed existing
regressions also pass. This comparison does not pass the full gate or close
either parent.

Reserved schema tests (80), fast-gate tests (28), formatting and Clippy with
warnings denied pass. The complete gate takes 509.79 seconds including
reservations: schema tests 7.56 seconds, Python tests 2.50 seconds, formatting
2.46 seconds, Clippy 26.75 seconds and Rust compilation/tests 468.77 seconds.
The raw full-log SHA-256 is
`109801e6e8d3859c715d010e01cd414c216fbf317e62485aca6cd40b92d9f3a1`.
All fifteen Rust, Python, JavaScript and JSON source hashes remain fixed through
final verification. No source edit follows this full gate. Exact per-target
comparisons, original-owner packet conformance and raw command/source records
are retained privately for independent reproduction. Final lightweight checks
cover current prose, unchanged ledger rows and exact retained review bytes;
they do not repeat suites outside their reservations. The first final prose
check stops when its older temporary Node tooling entry disappears. Its failure
log is retained. The private runners now select the already retained licensed
Node installation directly; no download, dependency or source change is made.
The corrected final light check passes before committing.

Reviewer and artifact provenance, new-file review, candidate execution,
separately approved Git effects, native/model measurements and durable recovery
remain open. No new route, terminal command, model request, grant, sandbox or
trust exception is introduced. No frozen/hidden/held-out task, scorer, threshold,
prompt, dependency, supply-chain, status or scoreboard change is involved. No
model download, GPU process, long acceptance gate, release or owner-ready claim
is involved.
