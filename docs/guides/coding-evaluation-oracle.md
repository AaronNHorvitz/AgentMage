# Independent function oracle

T-311.7 separates candidate execution from frozen assertions; T-311.8 adds
startup discrimination and source classification. T-311.9 adds two acceptance
verification functions. T-311.10 adds three parser/argument tasks and a third
acceptance predicate, for ten existing development tasks. T-311.11 preserves
argument list/tuple distinctions, observes candidate case roots and separates
adapter policy from malformed wire data. T-311.12 carries wrong JSON values
to the original assertions and records candidate exceptions separately. T-311.13
wraps predicate observations separately from control tags and classifies bounded
serialization refusals as adapter policy. This is a
prerequisite for trustworthy scoring,
not generated-candidate admission, a model baseline or a completed T-311.
The original collection, hidden bytes, method selections and thresholds stay
unchanged. No model, harness or prompt is tuned by this adapter.

T-311.14 adds the two existing output-redaction tasks. Their original assertions
remain in the trusted process; only fixture text, closed command lists and
bounded return observations reach the candidate. It also catches memory
exhaustion during response encoding as an adapter-policy refusal.
The T-311.14 review repair uses oracle v10 and preserves the original fixture's
completed-process or mock stub kind, rather than replacing both with one kind.
T-311.15 uses oracle v11 to add file-mode and helper-call observations for
dev-017 and dev-019. Mode assertions inspect actual disposable files from the
trusted process; a candidate cannot replace those observations with mode claims.

T-311.24 adds dev-016 through oracle v23. A proxy retains each candidate model
adapter instance across its original constructor and token-count calls. The
candidate uses the original finite transport mock; no model or socket runs.
Both complete frozen methods and their assertions stay in the trusted namespace.
The proxy replays the current complete transport-call history into the original
mock after validating the entire response. This preserves the cache call count,
retained mock references and later mutations to earlier payload objects. Repeated
frames replace that mock history rather than double-counting it. A changed fixture
from the trusted test updates the candidate stub; otherwise candidate stub state
persists. Only inert typed bytes, integer frozensets and the existing finite
value types are transported. Unknown types are policy refusals; candidate
exceptions have the existing sticky fixed category, without exception text.

The selected methods grade counts, repeated-message caching and four probe
payload fields, including the exact false Boolean for `cache_prompt`. They do
not assert the other transport arguments, the implementation of HTTP status
filtering, malformed-response refusals, every statement clause or non-usage
behavior, adapter attributes or other methods.

T-311.25 uses oracle v24 to preserve the identity of each repeated message-list
root through a bounded per-instance handle. Equal but distinct roots remain
distinct; positional and keyword calls share the same handles. Trusted changes
update that root in place. Nested objects are rebuilt from the current snapshot;
nested identity and equality-overriding subclasses remain unsupported and can
change verdicts. These limits need disposition before generated admission.
Candidate-to-trusted mutation propagation is not implemented; v25 refuses net
encoded changes instead, as described below.

Replay requires an exclusively owned inert transport mock. A second adapter
sharing that mock, pre-existing unowned history, a replacement mock, side effects
or a wrapped target causes sticky adapter-policy refusal before replay or another
request. The target is checked again after validating the response. This avoids
erasing another instance's observations or executing a replay target. It does not
merge cross-instance history or propagate cross-instance post-response mutations;
shared mocks are refused even when their history is empty. This is
an evaluator control, not inference, token-usage measurement or a model score.

T-311.26 uses oracle v25 to check that the trusted mock still has its last
replayed history before sending a request and again before replay. Frozen encoded
snapshots include child calls, direct calls and the fields used by mock assertions.
A trusted reset, extra call, child call or changed call argument causes sticky
adapter-policy refusal; the current history is preserved. Pre-existing child
history also refuses ownership. Snapshots use bounded typed encodings, so mutable
argument aliases and equal-valued Boolean/integer substitutions cannot hide drift.

After each candidate call, v25 checks every retained message root against its
pre-call encoded snapshot. Net root or nested-value changes, including changes to
an earlier retained root, cause adapter-policy refusal before any result is
replayed. It does not propagate candidate mutations or preserve nested aliases.
Transient changes restored before return and asynchronous mutations after the
snapshot are not observed. Input encoding refusals also set the sticky policy
flag before raising, so catching `ValueError` cannot turn refusal into success.
The original frozen assertions, selections, control profiles and thresholds stay
unchanged; generated scoring remains disabled.

The T-311.26 review records two low findings for the suite admission backlog:
deliberate candidate self-modification can defeat the candidate-side snapshot
check, and mutation refusal is stricter than the original assertions (a mutation
after the final call can leave those assertions passing). Such a policy refusal
does not establish a frozen-task failure. Its scoring disposition remains open
before generated admission. These are backlog notes, not new work rows.

## Run

```sh
bash /tools/build-slot python3 scripts/coding_eval_oracle.py verify --output <new-private-results>
```

The verifier runs the frozen base and historical reference for these tasks.
Every selected original hidden method remains complete and unchanged.

| Task | Function | Methods | Base failures/errors | Base calls | Reference calls |
| --- | --- | --- | --- | --- | --- |
| dev-005 | `validate_raw`, `capture` | 1 | 3 / 0 | 3 | 3 |
| dev-006 | `parser`, `route_grant_arguments` | 1 | 0 / 1 | 1 | 19 |
| dev-007 | `parser`, `memory_arguments` | 1 | 0 / 1 | 1 | 52 |
| dev-009 | `subprocess_runner` | 1 | 1 / 0 | 1 | 1 |
| dev-010 | `test_only_files` | 1 | 1 / 0 | 1 | 4 |
| dev-012 | `test_only_files` | 4 | 10 / 0 | 12 | 14 |
| dev-015 | `case_passed` | 1 | 1 / 0 | 3 | 4 |
| dev-016 | `OpenAIAdapter`, `tokenize_messages` | 2 | 0 / 2 | 5 | 5 |
| dev-017 | `make_container_readable` | 1 | 0 / 1 | 1 | 1 |
| dev-018 | `validate_raw`, `capture` | 1 | 1 / 0 | 1 | 5 |
| dev-019 | `extract_deb` | 1 | 1 / 0 | 1 | 1 |
| dev-020 | `verify_case` | 1 | 1 / 0 | 2 | 3 |
| dev-021 | `verify_case` | 1 | 0 / 1 | 1 | 6 |
| dev-022 | `verify_case`, case roots | 1 | 0 / 1 | 2 | 4 |
| dev-024 | `gpu_memory`, `CandidateResourceGuard` | 2 | 0 / 2 | 2 | 6 |
| dev-026 | `run_commands`, `main`, command declarations | 2 | 1 / 0 | 3 | 3 |
| dev-027 | `parser`, `extension_arguments` | 1 | 0 / 1 | 1 | 37 |
| dev-028 | `stop`, `control`, `parser`, export and directory helpers | 1 | 0 / 1 | 13 | 18 |
| dev-029 | `suspend_resume_checks` | 1 | 0 / 1 | 1 | 6 |

Subtests explain the ten failures across four methods in dev-012. The verifier
requires these exact failure/count/call profiles, successful startup and expected
function presence, plus a passing reference with its exact call count. The three
argument-task bases lack their requested helper and reject the first parser
call with `SystemExit(2)`; those are explicit failed controls. An
arbitrary failure cannot validate a base. Dev-016's base has its callable class,
five calls and two original errors: the absent `cache_prompt` field followed by
the fixed candidate-exception category for the unsupported context response. A missing function in dev-029 is the
expected failed candidate-exception control: one call, one original test error,
candidate exit 0 and oracle exit 1. The candidate now reports the fixed exception
tag instead of crashing and leaving a broken pipe. No assertion or count changes.
The absent helper in dev-017 has the same explicit missing-function profile.
Launch/import failure, timeout or missing counts cannot validate a control.
The extra dev-022 call observes candidate case roots before its original
uniqueness assertion; its hidden method and three predicate calls are unchanged.

T-311.17 adds dev-009 through oracle v14. Its real child runs inside the
candidate namespace; diagnostic text and one closed error category reach the
original assertions, as described below.

T-311.18 uses oracle v15 to separate standard output from the protocol, require
EOF after the final response and hide host mount sources from launcher arguments.
It addresses T-311.17 review lows L1/L2/L3; generated scoring remains disabled.

T-311.22 adds dev-028 with oracle v20. Its complete original method checks
job-control dispatch, parser options, export selection and support-directory
validation through finite observations, described below.

T-311.23 uses oracle v22 to refuse calls through signal mocks retained after
their request ends. It addresses T-311.22 review 2 low L1 before generated
admission; it adds no task or model score.

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
namespaces, empty launcher environments, read-only runtime/source mounts and temporary
scratch. The candidate has its proposed source projection and a small function
configuration. It has no hidden tests, original Git history, result directory,
host-home mount, inherited credentials, model or live repository. The oracle has only the frozen
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
Candidate exceptions are also sticky: later calls fail and later methods can
therefore report errors that would not occur in the original single-process
suite. Alternate-candidate error counts are adapter counts; only the explicitly
measured frozen control profiles are asserted here. Pass/fail still requires
every selected original method to succeed.

Before candidate imports, the worker duplicates its protocol writer to a
non-inheritable descriptor and redirects fd 1 to diagnostics. Ordinary prints,
`sys.__stdout__`, raw fd 1 writes and exec children therefore write to the
bounded candidate log. After assertions, the oracle sends the exact closed
`{"done":true}` frame, closes its request writer and requires candidate EOF.
The end frame lets the worker return even while namespace init holds the request
pipe open. Any trailing byte is a sticky protocol failure,
even if all original assertions passed. One byte is retained within the existing
transcript bound; a worker that holds the pipe open still hits the host deadline.
The handshake also runs after startup or prerequisite-observation failure.

This descriptor separation is output hygiene, not isolation from candidate
Python inside its own interpreter. Deliberate descriptor discovery or worker
patching remains possible; the candidate controls its observations. A forged
first reply followed by the real reply is rejected at EOF. The original
assertions, counts and result mount remain in the separate trusted namespace.

The host passes bubblewrap's NUL-separated arguments through an anonymous file
descriptor with `--args`. PID 1's command line contains the launcher, the flag and
descriptor number, plus the fixed namespace-local worker command. The launcher closes that descriptor before starting
the worker. No mount-source path appears in that command line and no named
argument file is created. This is not a claim that all host filesystem metadata
is hidden by the namespace.
The host closes its descriptor after launch, including failed exec. Oracle v16
also starts each launcher with an explicitly empty environment. `--clearenv`
alone clears the executed worker, while namespace PID 1 retains the launcher's
original environment in `/proc/1/environ`. Both roles now inherit no host
environment; their workers receive only the fixed PATH, HOME, LANG and PWD.
Real-namespace regressions check PID 1 and worker environments with synthetic
host credential, home and user sentinels. Mount, permission and resource options
remain unchanged. `/proc/self/mountinfo` can still disclose host bind-mount roots;
no claim of complete host-path anonymity follows. This known metadata exposure
requires an explicit disposition before generated scoring.

T-311.18 review 1 medium M1 identified this inherited-environment gap. Its repair
left the legacy grader unchanged. T-311.19 applies the same empty-launcher
environment protection to both legacy grading branches, with a real-namespace
regression followed by their frozen tests. This resolves only inherited
environments; host-path metadata and the other generated-scoring prerequisites
remain open.

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

For dev-015, predicate results cross as finite built-in JSON data, including
nulls, integers, strings, lists and maps, bounded to 16 container levels. For
dev-029, results cross as string-keyed maps whose values have that same bound.
Empty maps are data too. Both use a single `value` envelope, so observation keys
such as `adapter_policy`, `candidate_exception`, `value`, `tests` and `passed`
remain data and cannot collide with the adapter's control tags. The unchanged original assertions judge truthiness, selected keys
and other observed behavior; no exact Boolean type check is added by the adapter.
Both tasks explicitly label `exact-boolean-result-type` as `not_graded`. Integer
and null truthiness variants, container truthiness and reserved map keys are
checked against the original legacy grader, alongside wrong values that fail
those assertions. Unsupported types, nonfinite values and over-deep containers
produce an explicit adapter-policy refusal; raw frames with invalid shapes
remain protocol failures. No candidate observation supplies the oracle verdict.

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
Every supported task declares `bounded-encoded-response` adapter policy; predicate,
argument and verification tasks declare their additional observation bounds.
Unknown task types cannot use a policy tag. No candidate can add a policy to its coverage. A policy tag
with an extra field, wrong value or payload is always malformed protocol. Such failures remain in the denominator. Raw hostile bytes
remain confined to the private transcript and are never reconstructed as code.
No claim of full statement coverage follows from these controls.

## Synthetic GPU observations

T-311.20 (oracle v17) transports dev-024's two complete frozen methods. The
trusted side snapshots the original completed-process stub's stdout, stderr and
return code. The candidate invokes its own `gpu_memory` with that same stub.
Tagged observations preserve scalar, list, tuple, map and lexical Path values,
so a returned list fails the original tuple equality assertion. Only the fixed
trusted `HarnessError` category is reconstructed, with a local message. Other
exceptions use the existing payload-free candidate-exception failure tag.

The guard proxy sends its original constructor arguments to the candidate and
keeps only an opaque bounded instance number. Only descriptor `-1`, the finite
profile/resource fields and integer baselines are admitted by this adapter.
A sample sends the original GPU mock's tuple, invokes the same candidate-owned
guard instance and returns its result plus observed error and peak fields.
The original assertions judge those values. Falsey containers remain values;
map keys matching control tags remain data inside the tagged envelope. Objects,
unsupported containers, excessive depth or encoded size are adapter policy.
Duplicate instances, invalid operations, unexpected fields and live descriptors
are protocol refusals. No Python object, class or callback crosses the pipe.

These are synthetic GPU observations: no GPU device, model, real observation,
lock acquisition, process cancellation or complete resource admission is tested.
Subprocess invocation arguments, exception messages, unselected guard state and
transitions and unselected statement clauses are explicitly ungraded. The
candidate can inspect and alter its own adapter; passing these finite assertions
establishes only the declared observation behavior, never live device safety.
Both missing base helpers produce two original test errors and two failed calls;
the reference runs both methods with six calls. Controls-only admission stays
in force. No generated score follows from these measurements.

## Collector deadline observations

T-311.21 uses oracle v19 for dev-026. Both complete original methods stay in
the assertion namespace. A prerequisite call carries the candidate's command
declarations as bounded list/tuple/string data, so the original count and order
assertions use that candidate global. The first method's finite completed-process
stub crosses as typed arguments, return code and hexadecimal stdout/stderr bytes.
Only declared executable names have a finite resolution mapping; an undeclared
lookup is adapter policy. No callable, mock, expression or original test crosses.

The candidate runs its own `run_commands` with reconstructed local mocks and
returns its value and observed subprocess calls. A closed codec preserves tuple,
list, Path and finite built-in value distinctions. The trusted side validates the
entire reply before replaying any call into the original MagicMock; it refuses
a real function as a replay target. The unchanged original assertions check
record/call counts, tuple argument order, each timeout and capture truthiness.
They do not inspect record contents or require an exact Boolean capture flag.
Wrong deadline/container/count/capture probes fail; integer truthy capture and
inert record maps pass both the independent and original verifier.

For `main`, the trusted test's SimpleNamespace arguments and fixed TimeoutExpired
stub become finite data. The candidate reconstructs those stubs and a local output
mock, calls its own `main`, and returns its value, timeout category and write calls.
Writes are replayed into the trusted output mock even when a timeout occurs, so
writing before re-raising fails the original `assert_not_called`. A swallowed
timeout fails the original `assertRaises`. Only the standard-library TimeoutExpired
class with fixed local fields is reconstructed. Exception text, classes and
tracebacks from the candidate never execute in the trusted process.

The base fails one deadline assertion; its timeout method passes. Both controls
run two original methods and three protocol calls, including the prerequisite.
These fixtures run no collector commands, compiler, documentation gate or GPU
work. Report contents/digests, missing-executable handling, a minimum command
count, timeout fields, unselected main paths and other statement clauses remain
ungraded. Unsupported containers/subclasses and undeclared resolutions are adapter
policy, even where an original assertion might accept them. Candidate-controlled
call observations establish only the reported behavior, not actual command effects.
Generated scoring stays disabled and all earlier admission blockers remain.

## Failed-command diagnostic observations

For dev-009, the proxy replaces only `subprocess_runner` before the complete
original method imports it. The request contains that method's command strings
and lexical working directory. The candidate reconstructs a tuple and Path and
calls its own function. The frozen reference actually runs the small synthetic
Python child in the same confined, networkless namespace; this is no model run.
The child produces one diagnostic and exits 3. No additional host command
permission or shared writable scratch is introduced.

A candidate-local StringIO captures the function's stderr. A normal return
produces the closed `return` category, without a return value. A RuntimeError
(including subclasses, as the original assertRaises accepts) produces the closed
`runtime-error` category and its string message. The bounded reply contains only
`stderr` and `outcome`. The trusted proxy writes the text into the original
method's capture and raises its own built-in RuntimeError with that message.
Messages are plain data, never evaluated, imported or interpreted as a class,
traceback, callback or executable expression. This narrow observation preserves
the frozen assertions on diagnostic and exception-message substrings. It is
separate from the generic fixed candidate-exception failure tag. Other unexpected
exceptions retain that sticky failure; their messages never cross the pipe.

Unknown categories, extra fields, non-string text or malformed frames fail as
protocol before text is replayed. A response too large for the existing frame
bound fails as adapter policy. The 64 KiB bound includes the complete JSON frame,
newline and escaping: non-ASCII characters are escaped, so a 16,384-character
tail can exceed it. A frame-bounded diagnostic means encoded bytes, not characters.
Every refusal remains counted. The original
assertions judge the observed text; a candidate cannot replace those assertions
or publish the oracle's counts. Candidate-side unittest patches that fool the
legacy grader fail here. RuntimeError strings containing code-like text remain
inert in codec tests. General exception transport is unsupported.

The method does not assert the 16,384-character tail, terminating newline,
successful return value, subprocess invocation arguments or exact exception
class. Reports declare those omissions explicitly. A subclass and an oversized
but frame-bounded diagnostic can pass both original and independent assertions;
those controls establish no full-statement coverage. The namespace diagnostic
also proves the candidate lacks the hidden-test and output mounts. It fails the
legacy contrast because the legacy candidate has an output mount; that is an
intentional boundary contrast, not a different claim about function behavior.

## Captured output and private-path checks

For dev-005 and dev-018, proxies replace `validate_raw` and `capture` before the
unchanged original test module imports them. `validate_raw` receives the exact
synthetic text assembled by the trusted test. For `capture`, the proxy snapshots
the test's stub stdout, return code, closed stub kind and current command list.
For dev-005, it accepts the exact completed-process type only with empty list
arguments and null stderr, recreating those fixed attributes. For dev-018, it
recreates a local mock with stdout and return code set, preserving the original
mock's default attribute behavior. Other stub types, nonempty completed-process
arguments or non-null stderr are refused as trusted protocol; no mock object crosses the pipe.
The candidate uses that local subprocess stub and command list to call its own `capture`.
No fixture command runs. Both namespaces use the same lexical workspace and
home paths, so fixture path text is preserved without rewriting assertions.

Results carry bounded finite built-in JSON observations, with a fixed tag for
a top-level tuple. The trusted side reconstructs only that built-in tuple; all
other supported results remain data for the original assertions. Reserved map
keys stay inside the value envelope. Objects, nonfinite values and unsupported
containers fail as adapter policy; malformed envelopes fail as protocol. No
mock, callback, exception payload or executable source crosses the pipe.

The original methods check selected private-path refusals, redaction and return
values. They do not check subprocess invocation arguments, real command
execution, other module globals or all statement clauses. Those omissions and
the finite-stub policy are explicit in report coverage. These are controls of
evidence functions, not actual command collection or a real-model run.
The dev-018 home-alias variant is also explicitly ungraded: both sandboxes use
the same `/tmp` home, so that branch of the unchanged fixture does not execute.
Malformed adapter requests remain protocol failures instead of being labelled
candidate exceptions; this also applies to argument and verification requests.
The mock-kind transport supports only the pinned fixture's stdout and return-code
configuration. Additional mock configuration is ungraded; it is not validated by
the kind check. Adding another capture task requires checking that fixture first.

## File modes and extraction helper calls

The complete dev-017 method creates a four-entry text tree and asserts its four
permission modes. Its trusted proxy snapshots the original paths, file text and
bits, with at most 128 regular file/directory entries. The candidate recreates
that input at the same lexical path in its own disposable `/tmp`. Only a fixed
completion acknowledgement crosses back. The oracle has a read-only view of
that disposable filesystem at `/candidate-tmp`; it independently stats the known
entries and replays those observed bits onto its original fixture before the
unchanged assertions run. Returned mode maps are malformed protocol, never data
used for chmod. A forged completion without a tree fails inspection.

Descriptor-relative inspection refuses symlinks at every component, including
ancestors, without following candidate aliases into trusted files. Missing,
uninspectable or non-regular entries fail as sticky adapter policy and count as
failed attempts. Only the initial trusted names are inspected or changed; reply
paths are never opened. The writable mount contains only disposable candidate
data. After signalling and reaping the two worker leaders, the host restores
owner read/write/search permissions on disposable real directories. Descendant
teardown is asynchronous; repair must remain safe during concurrent mutation.
Linux no-follow directory descriptors pin each inode, including mode-0000
directories; chmod resolves only the host's own `/proc/self/fd` descriptor.
Candidate names never enter a chmod path. Scanning and opening children are
descriptor-relative. Oracle v26 retains pending sibling names and parent inode
identities in compact buffers. Ascent opens `..` and checks the recorded identity
before using that parent. There is no full-path copying or reopening from the root.
At 256 levels it moves a subtree into a private host-only staging sibling of the
fixture root, checks every moved inode and scans its directories through shallow
numeric names. Records contain one basename and parent index, never a full path.
Repair restores children before parents; removal deletes emptied staged directories.
This also bounds deep bind-mount parent lookups. Constant descriptors and linear
traversal state and directory operations cover deep branching trees. Pending
buffers and staging records are cleared on exceptional exit before outer recovery.
Staging remains inside the private case and is never mounted into a worker.
Directory renames cannot redirect chmod to a
symlink target. File modes and hard-link targets are never changed. Concurrent
replacement can omit a branch; repair does not claim a complete stable snapshot.
Descriptor-relative postorder removal then deletes scratch with constant open
descriptors, rather than retaining one descriptor per ancestor as the generic
remover does. Deep removal uses the same staging walk. Each shallow ascent must
match the recorded parent inode before inspecting or removing its child; a
concurrent move refuses cleanup. Unlink and rmdir never
follow the removed name. The root's parent is private host-owned case storage.
Existing case cleanup removes the remaining case tree before reports publish.
This also runs on interruption or failed result reading;
permission repair preserves the already observed oracle result and cause. Actual
cleanup failures still refuse count publication and keep a private recovery
record. The initial T-311.15 implementation lacked this permission repair;
independent review M1 exposed leftover trees from natural wrong mode answers.
The first repair still used paths and wrongly claimed synchronous descendant
shutdown. Its second review exposed depth failures and that false timing
assumption; v13 uses descriptor-based repair and makes no quiescence claim.
Third review of T-311.15 found a remaining medium issue: the deferred full-path
records and branch reopening use quadratic host memory and time on deep
branching trees. A capped reproduction raised MemoryError without a case report,
left an empty recovery record and retained scratch. Candidate quotas alone do
not bound this host-side repair. The combined T-311.15/T-311.16 repair in v26
addresses this finding under the owner's explicit restart. Operation-count,
10,000/20,000-level memory/descriptor-cap and in-sandbox branching regressions
check the replacement. Both rows remain open until independent exact-commit
review approves the combined repair. Constant descriptor use and linear scaling
do not impose an aggregate scratch quota or a host cleanup deadline.
Do not infer safe generated scoring from the passing frozen controls.
The candidate mount is host-disk-backed, without an aggregate disk quota. A
size-limited or quota-backed mount is required before generated scoring.
Content preservation, changed tree shape and special-file refusal behavior are
ungraded by the original method; regular inspectable observations are explicit
adapter policy. This does not test archive extraction or container integration.

The complete dev-019 method installs a default command-helper mock, calls
`extract_deb` and compares its exact two calls. The candidate runs that function
with an equivalent local mock. At most 128 call records carry bounded values;
closed tags preserve built-in list/tuple distinctions, dictionaries, finite JSON
scalars and lexical local Paths. Unknown tags, malformed frames or extra fields
fail as protocol. Unsupported objects and excessive nesting/counts fail as
adapter policy. Every record validates before replay, and the trusted proxy
refuses replay unless its original helper is still a mock. No recorded command
executes. This grades helper-call order and values; real package extraction,
helper error propagation and other statement clauses remain ungraded.
Both input arguments are normalized to Path, matching the frozen method's Path
inputs. This adapter does not establish behavior for callers passing strings;
such reuse requires preserving those input types explicitly.

Both tasks keep separate code, assertions and output mounts, plus the existing
networkless boundary and controls-only admission. Their real namespace tests
exercise forgery, wrong observations and confinement. Monkeypatching unittest
can still forge the legacy grader; the same diagnostics fail this independent
oracle and remain in its failed-attempt denominator.

## Job-control observations

Dev-028's original test and assertions remain in the trusted namespace. Its
candidate parser returns bounded tagged values, preserving Paths and list-valued
defaults; only data is reconstructed into a local argument namespace. Export
selection and support-directory helpers return the same tagged values or closed
HarnessError, KeyError and argument-type refusal tags. No exception class,
message or traceback supplied by the candidate crosses into trusted execution.
The support-directory fixture is rebuilt at its exact lexical
path under the candidate's private `/tmp`; relative and missing paths stay
absent. An existing directory outside that finite fixture is adapter policy.
No shared writable host scratch mount or permission-repair traversal is added.

Candidate `stop` and `control` calls use a finite signal-helper stub. Successful
dispatch observations replay only into the original frozen test's signal mock,
after the entire reply and all calls have been validated. No replay invokes a
live signal helper. For the final no-live-run case, the trusted temporary
directory is recreated at the same lexical path in the candidate's private `/tmp`. The candidate's own
`signal_running_cli` runs without a replacement helper. The reference refuses
its empty directory before looking up or signalling any process. A candidate
that returns normally fails the original refusal assertion; an unexpected
exception is a sticky candidate-exception failure, without its private message.
Signal enum values are normalized to integers because the original assertions compare their values;
enum identity is ungraded. Order, missing calls, wrong values, unknown-control
refusal and no-live refusal still reach the original assertions. Ordinary integer
values that compare equal retain the original assertion behavior.
The frozen mock records signal numbers only: its base argument, keyword forms
and the wrappers' return values are not asserted. The finite helper has the
original `(base, number)` signature, recording only calls that reach its body;
a wrong-arity call caught by the candidate contributes no signal observation.
Wrapper return values are discarded because no frozen assertion reads them.
Each recorded request expires its signal mock in a `finally` block. Calling
that mock or its original side effect after expiration sets a sticky worker
flag. Every subsequent job operation, including parsing, export and no-live
checks, refuses with adapter policy. The flag is checked after the operation
too, so catching an exception or clearing a mock's history cannot hide the
call. Merely retaining the mock does not fail a candidate. This refuses an
observation the per-request adapter cannot faithfully replay; it does not claim
the original test raised an adapter-policy exception. Current-request calls
still reach the original assertions. Candidate Python remains able to tamper
with objects inside its own interpreter; this guard does not isolate the
worker from deliberate self-modification.
PurePath and other subclasses outside the tagged codec remain adapter policy,
even when they compare equal to a Path in the original test. This is stricter
than those original assertions, as is the resource adapter's recorded subclass
policy; it does not establish a frozen-task failure.

The frozen base reaches its original unsupported-route export error after
13 calls; the reference passes the single complete method with 18 calls. Exact
counts and profiles remain required. Real process identity, liveness and signal
delivery, support-bundle launch propagation, privacy, ownership and export
approval are ungraded. No support bundle or live CLI process is produced. Wrong
observations and candidate assertion patching fail the independent oracle;
synthetic diagnostics do not authorize generated admission.

## Candidate exceptions and cause limits

Unexpected exceptions during a candidate call, including missing report fields,
an absent function after startup and unhandled `SystemExit`, produce exactly
`{"candidate_exception":true}`. This tag carries no exception class, message,
exit status or traceback. The dev-009 expected RuntimeError text observation is
a separate, closed result category described above. The trusted process raises one fixed local error,
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
A candidate that raises the worker's own reachable `ProtocolError` also gets
the protocol cause, rather than the candidate-exception tag. That label is not
proof of an adapter-originated error; the attempt still fails and is counted.
Policy and exception failures count in the same failed-attempt denominator.
After a candidate call, response encoding is also bounded adapter work. A
response exceeding 64 KiB or refusing JSON encoding (including the interpreter's
integer-digit bound) becomes the fixed `adapter_policy` tag with normal candidate
pipe cleanup, instead of crashing that worker. Memory exhaustion during encoding also uses
that fixed refusal; no allocation-failure message crosses the pipe. This applies to every supported
result type and changes no budget. The oracle independently validates each raw
frame: forged, truncated, nonfinite, over-deep or otherwise invalid wire data
still fails as protocol. A policy tag never excuses or drops an attempt.
Predicate container/depth scope and serialization limits are explicit adapter
policy, not proof that the candidate violated a frozen assertion. These repairs
address T-311.12 review lows L1, L2 and L3; independent review is required.
A failed case-root or command-declaration prerequisite observation may retain
zero original tests only with exactly one call, all six test counts zero, and
a sticky protocol, policy or exception flag. Its cause, flags, failed-attempt
count and published `tests.json` remain available after cleanup. No other task
may retain a ready zero-test record. All other counts and the requirement that
no failed control passes stay unchanged. Oracle v19 repairs T-311.21 review M1;
the previous host validator accepted only case-root prerequisite refusals and
misclassified command-declaration refusals as infrastructure with lost counts.

## Lifetime, records and remaining limits

The parent enforces a 90-second execution deadline and 2 MiB for each process's stderr.
It keeps both leaders unreaped until process-group cleanup, including launch
failure and interruption. Each PID namespace also bounds escaped descendants.
Counts and wire logs are published only after process and temporary-tree cleanup;
cleanup failure cannot report success. Existing private recovery records and
removal retries come from the accepted evaluator.

Both processes retain the evaluator's per-process 512 MiB address-space,
60-second CPU, 32 MiB file and zero core-file limits. These are **not aggregate
worker quotas**. Generated scoring remains disabled: this adapter covers eighteen of
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

## T-311.22 same-row review repair

Review 1 found medium M1: oracle v20 substituted a fixed no-live helper, so
candidates that returned normally or raised an unrelated exception could pass
a refusal assertion the legacy grader failed. Oracle v21 runs that helper from
the candidate against the finite private empty directory. It preserves the
original method, all pins, thresholds and control profiles. Diagnostic variants
compare both graders; none is admitted for generated scoring. Low L1's unasserted
return and mock-arity mismatches and low L2's wrong-branch unit test are repaired;
L1's PurePath restriction remains an explicit adapter-policy limit.
The independent exact-commit re-review passed T-311.22; its newly reported stale
signal-mock low is addressed by T-311.23, which requires its own independent
review. Parent T-311 remains open.
