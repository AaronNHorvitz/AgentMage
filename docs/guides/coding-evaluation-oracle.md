# Independent function oracle

T-311.7 separates candidate execution from frozen assertions; T-311.8 adds
startup discrimination and source classification. T-311.9 adds two acceptance
verification functions. T-311.10 adds three parser/argument tasks and a third
acceptance predicate, for ten existing development tasks. T-311.11 preserves
argument list/tuple distinctions, observes candidate case roots and separates
adapter policy from malformed wire data. T-311.12 carries wrong JSON values
to the original assertions and records candidate exceptions separately. This is a
prerequisite for trustworthy scoring,
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
| dev-006 | `parser`, `route_grant_arguments` | 1 | 0 / 1 | 1 | 19 |
| dev-007 | `parser`, `memory_arguments` | 1 | 0 / 1 | 1 | 52 |
| dev-010 | `test_only_files` | 1 | 1 / 0 | 1 | 4 |
| dev-012 | `test_only_files` | 4 | 10 / 0 | 12 | 14 |
| dev-015 | `case_passed` | 1 | 1 / 0 | 3 | 4 |
| dev-020 | `verify_case` | 1 | 1 / 0 | 2 | 3 |
| dev-021 | `verify_case` | 1 | 0 / 1 | 1 | 6 |
| dev-022 | `verify_case`, case roots | 1 | 0 / 1 | 2 | 4 |
| dev-027 | `parser`, `extension_arguments` | 1 | 0 / 1 | 1 | 37 |
| dev-029 | `suspend_resume_checks` | 1 | 0 / 1 | 1 | 6 |

Subtests explain the ten failures across four methods in dev-012. The verifier
requires these exact failure/count/call profiles, successful startup and expected
function presence, plus a passing reference with its exact call count. The three
argument-task bases lack their requested helper and reject the first parser
call with `SystemExit(2)`; those are explicit failed controls. An
arbitrary failure cannot validate a base. A missing function in dev-029 is the
expected failed candidate-exception control: one call, one original test error,
candidate exit 0 and oracle exit 1. The candidate now reports the fixed exception
tag instead of crashing and leaving a broken pipe. No assertion or count changes.
Launch/import failure, timeout or missing counts cannot validate a control.
The extra dev-022 call observes candidate case roots before its original
uniqueness assertion; its hidden method and three predicate calls are unchanged.

The functions use JSON values and lexical paths. Three verification tasks also
use finite text fixtures and stub observations, described below. Other tasks,
general filesystem mocks, general exception transport and Rust execution are
unsupported. The two path
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
Replies must have exactly the expected fields and values for the selected task:
Predicate scalars or maps, lexical paths, verification checks or argument observations.
Duplicate keys, invalid UTF-8, truncated or
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

The path adapter grades returned membership and ambiguity behavior. It does
**not** grade the candidate's return container or element types: candidate-side
serialization erases those types and the oracle reconstructs Path values.
For example, a list of path strings can satisfy this adapter even though the
original hidden helper would reject strings. The original methods are unchanged,
but their inputs are adapted. Reports explicitly label these two interface
clauses `not_graded`; no full-interface or generated score follows from a control.

For dev-015, predicate results cross as finite JSON scalars (including nulls,
integers and strings). For dev-029, results cross as string-keyed maps whose
values are finite built-in JSON data, bounded to 16 container levels. Empty maps
are data too. The unchanged original assertions judge truthiness, selected keys
and other observed behavior; no exact Boolean type check is added by the adapter.
Both tasks explicitly label `exact-boolean-result-type` as `not_graded`. Integer
and null truthiness variants are checked against the original legacy grader,
alongside wrong values that fail those assertions. Reserved policy/exception tags
retain their fixed semantics; an undeclared policy tag still fails as protocol.

## Verification fixtures

For dev-020, dev-021 and dev-022, the trusted proxy snapshots the existing frozen method's
three log texts, optional `src/calc.py` text, observed rows, Git status and the
stubbed validation return code. Only fixed file labels cross the pipe. It sends
no host fixture path, test source, mock object, assertion, callable or executable
expression. The multi-file method already stubs the validation subprocess; the
proxy observes that stub, without running validation. Each function call takes a
fresh snapshot, so deletions and mutations in the unchanged method reach the
next call without sharing files or Python objects across namespaces.

The candidate adapter constructs its own temporary fixture from those labels,
stubs only `rows`, `git_status` and the validation subprocess, and invokes the
candidate's `verify_case`. Cleanup removes this fixture even after an exception.
The reply contains the returned `passed` and `checks` values as finite built-in
JSON data, bounded to 16 container levels and the existing 64 KiB frame limit.
Objects, nonfinite floats, tuples and deeper containers are adapter policy.
Nulls, strings, integers, empty maps and other representable wrong values remain
data for the unchanged assertions. Missing required fields raise a candidate
exception in the adapter instead of being labelled malformed wire data.
The oracle runs the original assertions on these returned values;
they remain observations under test, never the oracle's verdict or counts.
An always-true result fails the original negative assertions. A literal
`missing_case` tag reconstructs one fixed local KeyError, with no candidate
message or exception class. This is the expected failing dev-021 base control.
The dev-022 base likewise lacks the requested case. Its reference must reject
both a failure artifact associated with the wrong command turn and a wrong
payload digest.
Before loading that method, one observation carries the candidate's `CASE_ROOTS`
string mapping into the trusted module. The original preamble now checks
uniqueness of those candidate values. Duplicate values are valid wire data and
fail the original assertion. At most 128 entries and 256 characters per key/value
are transported; these strings are never interpreted as file access. Unsupported
values produce an adapter-policy failure; malformed wire data is a protocol
failure. Either stops before assertions, retains zero tests and one observation
call, and counts as a failed control. No hidden method or threshold is changed.

The original verification assertions grade truthiness, not exact Boolean types.
The adapter now preserves that behavior: reference results converted to 0/1 can
pass those same assertions. Reports label exact Boolean result types `not_graded`
and label bounded JSON observation types as adapter policy. This replaces the
older strict Boolean adapter policy without changing any hidden test, task pin,
selection or threshold. It does not establish full interface compliance.

These tasks grade the selected acceptance predicates on frozen synthetic rows
and fixture variations. They do not run a model, runtime, tool or validation
command; grade `run_case` integration; or check other report metadata fields,
unselected cases or statement clauses. Reports label those omissions explicitly.

## Parser and argument observations

For dev-006, dev-007 and dev-027 the oracle replaces only `parser()` and the
named argument helper. Original hidden methods pass their original argument
lists to a parser proxy. The candidate runs its own parser and returns scalar
namespace values, with an explicit lexical Path tag. The proxy rebuilds an
ordinary namespace, which the original method passes to the helper proxy.
The candidate then runs its own helper and returns JSON argument values with a fixed
`list` or `tuple` container tag. The oracle reconstructs that exact built-in
container, so the original equality assertion rejects a tuple. Other bounded
built-in JSON helper results cross as a fixed `value` field and return as data;
the original equality assertions decide. Wrong scalar, mapping or nested list
entries likewise remain data. Objects and unsupported container types are
adapter policy; these helpers use the same 16-level JSON and frame bounds. No
parser action, callback, function, class or pickled object crosses the pipe.

Namespace field names are bounded lowercase identifiers, at most 128 fields;
values are only strings, integers, Booleans, nulls or a single string Path tag.
Paths are lexical values: these methods do not open files or start the catalog.
The existing frame, transcript and call limits cover both parser and helper
calls. Fixed literal tags reconstruct only the trusted module's `HarnessError`,
`SystemExit(2)` or a fixed local `AttributeError` for an absent helper. Candidate
exception messages and exit statuses are never reconstructed. Unexpected exceptions
produce the fixed candidate-exception tag described below. No catalog, grant,
memory or extension effect runs here.

The frozen assertions check argument order/content and invalid combinations,
including parser mutual exclusion. They cannot check exception messages, parser
diagnostic output or the original parser object type through this adapter.
Reports label those clauses `not_graded`. Argument return container type is now
graded; only the exact built-in list and tuple types are transported.
The bounded namespace and fixed parser-exit rules are labelled adapter policy.
A callable/list namespace default, unsupported helper object or parser exit code
other than integer 2 produces a fixed `adapter_policy` tag. No exception payload
is included. The trusted side treats that tag as sticky failure with cause
`adapter_policy`, distinct from `protocol` for malformed fields or tags.
The policy tag is accepted only for argument and verification tasks, whose
coverage declares adapter policy. It is malformed protocol for Boolean, Boolean
map or path tasks. No candidate can add a policy to its coverage. A policy tag
with an extra field, wrong value or payload is always malformed protocol. Such failures remain in the denominator. Raw hostile bytes
remain confined to the private transcript and are never reconstructed as code.
No claim of full statement coverage follows from these controls.

## Candidate exceptions and cause limits

Unexpected exceptions during a candidate call, including missing report fields,
an absent function after startup and unhandled `SystemExit`, produce exactly
`{"candidate_exception":true}`. No exception class, message, exit status or
traceback crosses the pipe. The trusted process raises one fixed local error,
sets a sticky `candidate_exception_failed` flag and records cause
`candidate_exception`. Even an original test that catches that error cannot turn
the attempt into a pass. The candidate worker remains alive to finish its normal
pipe lifecycle; malformed requests or frames still fail as protocol. Import or
launch errors before readiness remain infrastructure failures. Expected ambiguity,
missing-case, helper and parser refusals retain their existing fixed tags.

The exception tag is available for every task type. Invalid or extra fields
make it malformed protocol. A candidate may forge a valid exception or an allowed
policy tag only to fail its own attempt; neither tag proves why adversarial code
failed. These labels distinguish ordinary adapter-observed failures, not a
forensic determination. A candidate that deliberately exits, corrupts its wire
or writes stderr retains the corresponding exit/protocol/output-limit cause.
Policy and exception failures count in the same failed-attempt denominator.
Candidate values exceeding the frame or JSON integer-digit encoding limit currently
fail as `protocol`, with candidate exit 1: encoding happens outside call exception
handling. This cause-label limitation is T-311.12 review low L1, retained for
backlog; these attempts still fail and remain counted.
A failed case-root prerequisite observation may retain zero original tests only
with exactly one call and a sticky protocol, policy or exception flag. All other
counts and the requirement that no failed control passes stay unchanged.

## Lifetime, records and remaining limits

The parent enforces a 90-second execution deadline and 2 MiB for each process's stderr.
It keeps both leaders unreaped until process-group cleanup, including launch
failure and interruption. Each PID namespace also bounds escaped descendants.
Counts and wire logs are published only after process and temporary-tree cleanup;
cleanup failure cannot report success. Existing private recovery records and
removal retries come from the accepted evaluator.

Both processes retain the evaluator's per-process 512 MiB address-space,
60-second CPU, 32 MiB file and zero core-file limits. These are **not aggregate
worker quotas**. Generated scoring remains disabled: this adapter covers ten of
30 tasks and does not implement separately delegated memory/process/CPU quotas,
protected held-out inputs, covered-clause labels or a current-harness baseline.
The legacy grader continues to import candidate code with tests and has its
unchanged frozen-controls-only restriction. This adapter does not silently
replace it or broaden its admission.

Every case retains `candidate.log`, `oracle.log`, bounded raw `wire.log`, a
parent-authored report and, when available after cleanup, oracle-owned counts.
The raw wire log may contain invalid bytes from a refused reply. Reports bind
task/source/hidden/adapter digests and retain exact counts, readiness, function
presence, protocol, adapter-policy and candidate-exception status, covered
observations and ungraded clauses,
elapsed wall time, failure cause and log digests. All attempts in this verifier
are controls: model attempts and GPU minutes are zero, tokens and model score
are null. Keep logs private because they contain task data and diagnostics.

Each returned report counts one control invocation, with either one pass or one
failure. The summary sums every report, including input-integrity refusal,
launch/import failure, malformed startup, timeout, output refusal and protocol
failure. None is excluded or automatically retried. These counts describe
controls, not model-attempt scores. Future generated scoring must likewise count
every unsuccessful admitted attempt in its denominator; an infrastructure label
must never silently discard or retry a candidate's syntax error or early exit.
T-311.8 review L2 remains an admission obligation until that scorer exists.
