# Independent function oracle

T-311.7 separates candidate execution from frozen assertions; T-311.8 adds
startup discrimination and two source-classification tasks, for four existing
development tasks. This is a prerequisite for trustworthy scoring,
not generated-candidate admission, a model baseline or a completed T-311.
The original collection, hidden bytes, method selections and thresholds stay
unchanged. No model, harness or prompt is tuned by this adapter.

## Run

```sh
bash /tools/build-slot python3 scripts/coding_eval_oracle.py verify --output <new-private-results>
```

The verifier runs the frozen base and historical reference for these tasks.
Every selected original hidden method remains complete and unchanged.

| Task | Function | Methods | Base failures/errors | Base calls | Reference calls |
| --- | --- | --- | --- | --- | --- |
| dev-010 | `test_only_files` | 1 | 1 / 0 | 1 | 4 |
| dev-012 | `test_only_files` | 4 | 10 / 0 | 12 | 14 |
| dev-015 | `case_passed` | 1 | 1 / 0 | 3 | 4 |
| dev-029 | `suspend_resume_checks` | 1 | 0 / 1 | 1 | 6 |

Subtests explain the ten failures across four methods in dev-012. The verifier
requires these exact failure/count/call profiles, successful startup and expected
function presence, plus a passing reference with its exact call count. An
arbitrary failure cannot validate a base. A missing function in dev-029 is the
expected failed protocol control. Its oracle may exit 120 when flushing a broken
pipe at shutdown, after retaining counts; exits 1 and 120 are both failures.
Launch/import failure, timeout or missing counts cannot validate a control.

The functions use JSON values and lexical paths. Other tasks, filesystem mocks,
general exception transport and Rust execution are unsupported. The two path
tasks classify Rust source text; they do not compile or execute Rust.

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

Before assertions run, the oracle consumes one exact startup frame sent after
the candidate module imports. It records readiness and whether the function is
callable. Missing/malformed startup runs zero assertions and is infrastructure
failure, even when the reference passes. Readiness is a diagnostic, not proof of
trust: candidate code still cannot author assertions, counts or a verdict.

For path tasks, a source mapping becomes string keys and source text. Results
are a unique list containing only original input paths, or the single literal
`{"ambiguous":true}`. The oracle rebuilds lexical Path values or raises the
frozen base module's own ambiguity exception with a fixed local message.
No candidate-selected exception name, message, argument, traceback or class
is transported or evaluated. An expected ambiguity does not poison the next
call. Foreign paths, duplicates, extra fields and all other exception envelopes
fail. Neither side shares fixture files or executes source text from a request.

## Lifetime, records and remaining limits

The parent enforces a 90-second execution deadline and 2 MiB for each process's stderr.
It keeps both leaders unreaped until process-group cleanup, including launch
failure and interruption. Each PID namespace also bounds escaped descendants.
Counts and wire logs are published only after process and temporary-tree cleanup;
cleanup failure cannot report success. Existing private recovery records and
removal retries come from the accepted evaluator.

Both processes retain the evaluator's per-process 512 MiB address-space,
60-second CPU, 32 MiB file and zero core-file limits. These are **not aggregate
worker quotas**. Generated scoring remains disabled: this adapter covers four of
30 tasks and does not implement separately delegated memory/process/CPU quotas,
protected held-out inputs, covered-clause labels or a current-harness baseline.
The legacy grader continues to import candidate code with tests and has its
unchanged frozen-controls-only restriction. This adapter does not silently
replace it or broaden its admission.

Every case retains `candidate.log`, `oracle.log`, bounded raw `wire.log`, a
parent-authored report and, when available after cleanup, oracle-owned counts.
The raw wire log may contain invalid bytes from a refused reply. Reports bind
task/source/hidden/adapter digests and retain exact counts, readiness, function
presence, protocol status,
elapsed wall time, failure cause and log digests. All attempts in this verifier
are controls: model attempts and GPU minutes are zero, tokens and model score
are null. Keep logs private because they contain task data and diagnostics.
