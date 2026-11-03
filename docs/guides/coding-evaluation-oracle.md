# Independent function oracle

T-311.7 separates candidate execution from frozen assertions for two existing
development tasks. It is a prerequisite experiment for trustworthy scoring,
not generated-candidate admission, a model baseline or a completed T-311.
The original collection, hidden bytes, method selections and thresholds stay
unchanged. No model, harness or prompt is tuned by this adapter.

## Run

```sh
bash /tools/build-slot python3 scripts/coding_eval_oracle.py verify --output <new-private-results>
```

The verifier runs the frozen base and historical reference for `dev-015`
(`case_passed`) and `dev-029` (`suspend_resume_checks`). Each uses its complete,
unchanged original hidden method, with exactly one expected test. These methods
exercise pure functions whose arguments and results fit JSON. Other tasks,
filesystem mocks, exception transport and Rust are unsupported. A missing
function in a base is an expected failed protocol control. An infrastructure
failure, timeout or missing count cannot validate either control.

Only `verify` is exposed by the command. Its host-side `grade` entry also requires
the exact registered task and a complete frozen base/reference snapshot before
creating output or launching a worker. Generated candidates, including a comment
edit, stay refused. Tests call the diagnostic entry with builder-authored probes;
this does not authorize a generated model attempt.

## Assertion boundary

Two separate bubblewrap processes have separate PID, network, user and filesystem
namespaces, cleared environments, read-only runtime/source mounts and temporary
scratch. The candidate has its proposed source projection and a small function
configuration. It has no hidden tests, original Git history, result directory,
host home, credentials, model or live repository. The oracle has only the frozen
base projection, hidden method and writable results; no candidate source is
imported into its interpreter.

The oracle substitutes a JSON function proxy before loading the original test
module. Trusted fixture construction, imports, assertions and counting stay in
that process. Two anonymous pipes carry function arguments and return values.
No object deserialization, executable expression, callback or exception supplied
by the candidate executes in the oracle. Candidate-side changes to the test
framework, printed summaries, early successful exit and forged result objects
cannot set the oracle's count or verdict. This checks observable function
behavior; it does not prove the absence of overfitting to public development
cases or test every clause in a statement.

Requests and replies use sequence numbers and bounded newline-delimited JSON.
Replies must have exactly the expected fields and a Boolean or nonempty map of
Booleans, as selected by the task. Duplicate keys, invalid UTF-8, truncated or
oversize frames, nonfinite constants, wrong sequence/type and extra result fields
fail the exchange. A protocol failure remains sticky even if a test catches its
exception. Limits are 64 KiB per frame, 128 calls and 2 MiB retained wire data.

## Lifetime, records and remaining limits

The parent enforces a 90-second execution deadline and 2 MiB for each process's stderr.
It keeps both leaders unreaped until process-group cleanup, including launch
failure and interruption. Each PID namespace also bounds escaped descendants.
Counts and wire logs are published only after process and temporary-tree cleanup;
cleanup failure cannot report success. Existing private recovery records and
removal retries come from the accepted evaluator.

Both processes retain the evaluator's per-process 512 MiB address-space,
60-second CPU, 32 MiB file and zero core-file limits. These are **not aggregate
worker quotas**. Generated scoring remains disabled: this adapter covers two of
30 tasks and does not implement separately delegated memory/process/CPU quotas,
protected held-out inputs, covered-clause labels or a current-harness baseline.
The legacy grader continues to import candidate code with tests and has its
unchanged frozen-controls-only restriction. This adapter does not silently
replace it or broaden its admission.

Every case retains `candidate.log`, `oracle.log`, bounded raw `wire.log`, a
parent-authored report and, when available after cleanup, oracle-owned counts.
The raw wire log may contain invalid bytes from a refused reply. Reports bind
task/source/hidden/adapter digests and retain exact counts, protocol status,
elapsed wall time, failure cause and log digests. All attempts in this verifier
are controls: model attempts and GPU minutes are zero, tokens and model score
are null. Keep logs private because they contain task data and diagnostics.
