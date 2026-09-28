# Runtime phase deadlines — verification

Date: 2026-09-28. Source: `51812c6297369613578b262b8783eadd66617ad2`,
tree `17588ea3c62cb17f2cc933d3a02727dfb284ca11`.
[Decision 0101](../decisions/0101-runtime-phase-deadline-admission.md)
governs this coordinator prerequisite. The [retained record](runtime-phase-deadlines-2026-09-28.json)
binds exact source phases, failures, checks and actual CLI observations.

## Behavior and limits

The coordinator checks the original run budget before admitting later phases.
Model requests bind positive remaining time at construction. Context construction
and token-binding callbacks, model dispatch and returned proposals, pending
approval resolution, permission evaluation and verifier success observe time and
cancellation before further work. Cancellation retains its original identity;
foreign or failed probes and observed clock regression fail closed.

An expired single requested tool can end as exhausted only while it has never
started. Its request and optional approval history remain; no denial, cancellation,
receipt or completed turn is invented. Started operations and incomplete effect
observations cannot use that terminal path. Actual completed-tool receipts remain
in the outcome when the run expires before a later turn.

The model request's canonical event records its construction clock sample.
Recovery reconstructs the exact remaining-time request and checks its full digest.
Historical journals can instead match only the exact original full-timeout request.
Neither form permits an arbitrary timeout or resets the original run start.

Checks are cooperative. Pending expiry is recognized on the next coordinator
advancement or valid response, including a coordinator poll. Client status
projection alone does not advance the pending run; no idle timer was added.
Blocking ports and persistence are not preempted. Native manifest/load/token
binding and aggregate preflight still need shared in-flight control through their
existing owners; factory model loading remains synchronous. These changes do not
prove native process or namespace cleanup, model admission or a hard time ceiling.

## Executed checks and retained failures

The original nine deterministic clock regressions used unchanged production and
prior test bytes: **one passed and eight failed**. They exposed renewed model
budgets, later work after context/model/permission delays, approval after run
expiry, pending polls without termination and late verification becoming success.

The first fixed run returned **18 passed and one failed**: a model-failure event
used a run-level cancellation diagnostic. The producer was corrected to the
existing model cancellation code; the diagnostic allowlist stayed unchanged.
The next 19 focused tests passed, but the full engine library returned
**1,266 passed, one failed and seven ignored**. Rejected-proposal recovery still
reconstructed the original full timeout and refused the new digest. That existing
test was preserved and the request/recovery binding corrected. A subsequent
attempt failed compilation with `E0425` from a missing test-helper import; no
tests ran. All failed logs and exact source snapshots remain retained.

After correction, **21 deadline tests and the original durable-recovery regression
passed**. Coverage includes pending ephemeral/durable paths, exact one-shot
cancellation, failed/foreign probes, preserved malformed-response challenges,
clock rollback, actual receipts after late tool completion, narrow terminal
admission, request-time digests and recovery without renewed time or effects.
These are synthetic component fixtures, with no model or native tool process.

The full engine package passed **1,349 tests, with seven ignored and none failed**.
Strict engine/host Clippy, formatting and strict-local, dependency, effect, module
and status audits passed. Full host tests returned **292 passed, 50 failed and
eight ignored**; all 50 names and complete panic/cause sections match the prior
result after numeric process-ID normalization only. The full host suite remains
failed. The unchanged inference and Linux suites were not rerun; their original
records and the prior Linux failures remain separate.

Full Markdown lint reported **five findings in a generated font-license file under
`target/doc`**. The license and configured gate were unchanged. A separate check
excluding generated output passed **579 maintained Markdown files**. It does not
replace the failed full invocation.

## Actual CLI and evidence

At the clean source revision, the actual CLI, host and read worker rebuilt.
Help, setup, diagnosis and status passed in fresh disposable repositories. The
scripted fail/repair launch returned **exit 5** before IPC, tools or inference:
`linux.repository.git_artifact.invalid`, then `linux.development.launch-envelope.failed`.
Idle stop returned **exit 1**; active cancellation was not reached. A separate
repository containing staged, unstaged and untracked work returned **exit 1** at
preflight before creating its launch log directory. File bytes, modes, HEAD,
raw index, index entries and both Git diffs were preserved. All three binary
identities remained stable. Preservation during admitted effects is unverified.

The SBOM was regenerated once; only the engine component content hash changed.
Six foundation builders, source-invalidation acceptance and the current canary
campaign passed, with **66 Python tests**. All named input sets and existing
qualification limits were preserved in 15 generated files at
`a26e55ae8bb549f1eb1e0307b335c827b4b956f6`.

The automated recovery-review pin advanced for exactly two changed inputs,
retaining all 25 reviewed paths. Its gate and the coordinator campaign passed;
the latter retained nine commands, 42 Rust cases and 24 coverage rows. Boundary,
security and index checks then passed against successive committed inputs. Their
direct Python checks returned **30 passes** (14, 7, 9
for campaign/gate, boundary and security/index). The index still has 16 complete
and one partial mapping, with story, sprint and release completion false. The
last evidence revision is `f98527d19cc14079666d663ec4d923a0cfc328e0`.

The new record preserves the previous record's complete public input set and
retains historical verification records at their original source pins. Direct
binding inventory is separate from global, transitive or line-span freshness.

Independent review remains pending. Real-model, manual-user and release runs are
absent. No task or acceptance gate closes. The positive Linux edit/validate/fail/
repair workflow, protected denial and active cancellation remain blocked by native
trust and user-systemd prerequisites. The full roadmap remains in scope.

Use the [local launch instructions](../LOCAL-TESTING.md). The focused check is:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 cargo test --locked --offline \
  -p agentmage-kernel-engine --lib run_deadline_ -- --test-threads=1
```
