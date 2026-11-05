# T-311.17 independent Rust corpus, 2026-11-05

The expanded suite batch remains open under
[Decision 0154](../decisions/0154-local-coding-delivery-refocus.md). This adds no
row, independent closure or review checkpoint. Parent T-311 and the unchanged
harness baseline remain open; no model score is claimed.

## Change

Oracle v31 adds an independent Rust constructor adapter for the existing frozen
development task. The complete original one-method test and 640-case corpus
execute in a trusted namespace. Candidate Rust executes in a separate namespace
with no hidden test, corpus or writable result mount. The trusted test uses an
original constructor transport shim; its assertions and fixture bytes do not
change. Candidate test-summary text cannot supply assertion counts or verdicts.

Only bounded component strings, error codes and optional component indices cross
the pipe. The trusted shim does not apply baseline path validation to returned
components. Unknown fields, invalid Unicode and invalid types refuse grading.
Protocol and observation refusals remain failed attempts. The guide and report
describe the coverage boundary explicitly.

Both builds use installed dependencies offline, one compiler job per namespace
and four-CPU affinity. Each namespace has hard limits of 16 processes and 768 MiB
address space per process, with anonymous scratch capped at 768 MiB and 32,768
inodes. The shared runtime snapshot has a 1,536 MiB/32,768-inode cap; no host-backed
runtime mount is exposed to the workers. The exchange deadline is 90 seconds.
Termination precedes result publication; anonymous mount lifetime controls
task-tree cleanup. No lane resource ceiling is raised.

Reports bind the Rust adapter and actual worker bytes. Inherited limits/counters
still describe the entire lane, including setup and other processes in this lane;
they are not candidate-only resident-memory peaks. Whole-suite resource admission
and generated-candidate admission remain disabled.

## Verification

All final checks below ran in one shared build reservation. The source/test
snapshot matches at the beginning and end and still matches the final files.
Times are command seconds, excluding reservation waits. This is evaluator
validation, not agent or model qualification.

| Check | Result | Seconds |
| --- | --- | --- |
| Ordinary scoped fast gate | PASS, 122 tests | 211.200791 |
| Source adapter module | PASS, 16 tests | 9.387319 |
| Development evaluator/collection/suite modules | PASS, 73 tests | 19.139473 |
| Fixture/resource modules | PASS, 16 tests | 5.481423 |
| Historical-control module | PASS, 11 tests | 1.828025 |
| Historical Rust module | PASS, 10 tests | 12.548437 |
| Boundary module | PASS, 9 tests | 0.589777 |
| All historical development controls | PASS, 30 base/reference pairs; 89 reference methods | 53.185452 |
| All independent development controls | PASS, 28 base/reference pairs; 57 reference methods, 948 calls | 30.647782 |
| Frozen split input check | PASS, 40 definitions; zero held-out assertions | 5.477004 |

All 257 unit/boundary methods pass. The six new Rust adapter methods include
early exit, forged summary text, hidden-input/result-mount absence, wrong error
indices, wrong case spelling, generated-input refusal and closed observation
bounds. The frozen Rust base fails its original method on the first constructor;
the reference passes all 640 cases with 576 constructor calls. Assertions and
test summaries come only from the trusted binary. The unchanged cleanup-memory
regression passes its 128 MiB cap at 91,040 KiB lifetime maximum RSS.

The integrity audit passes for all 116 reports and 288 log bindings. Every report
has verified inherited-lane observations and zero observed OOM kills. All existing
control profiles, frozen inputs, ordinary worker/test source and older ledger
rows remain unchanged. Public-file hygiene passes on new content. Post-evidence
Markdown and whitespace checks pass. No row closes.

Private artifact commitments:

- Final command manifest: `bbac3ce282efd18a9abc947e7d22dcc7327fcc7c260aee4957f7ae93187a598a`.
- Matching source snapshot: `689851856ac49e8f6dbab41774907d6377f884cd3a3fdfd3870272a3548cdb0d`.
- Independent control summary: `51f7c98d98261d718b523c031ae8f26c6f72c0c61ea39e8023b0e0d43881b4bb`.

Initial failed invocations are retained privately: an absent output parent,
exclusive source creation attempted twice, a launcher address-space mismatch,
and a double unwrap of the existing remote codec's value. Infrastructure and
protocol errors remained failed attempts. Repairs change adapter infrastructure;
no frozen task, hidden test, corpus, selection, threshold or existing control
profile changes.

The first full fast gate failed two existing tests: Rust commands built only in
the holder bypassed the host launch-failure probe, and the new result-type label
did not match the existing dictionary protocol. Rust command construction now
uses the existing host-side launch hook and validated namespace commands; the
holder still owns fixed runtime/scratch setup and teardown. Dictionary
observations use the existing protocol label. Both original tests pass unchanged
in the final run. The failed 122-test log is retained. An initial private hygiene
audit also matched unchanged synthetic privacy probes in an existing test file;
the corrected audit checks added content and complete new files. That failure
is retained too. No hidden test or threshold changes.

## Limits and remaining work

The original corpus grades constructor decisions, error codes, component indices
and case-sensitive control spellings. Its 64 invalid-UTF-8 cases reject in the
trusted JSON parser before a constructor runs. Candidate deserialization,
successful workspace identity and unasserted statement clauses remain ungraded.
No generated score can infer those properties from the passing corpus.

Independent coverage now includes 28 development tasks. The two remaining
development adapters, held-out independent grading and final coverage/resource
admission work stay in the same T-311.17 batch. No held-out assertion, harness
tuning, model/GPU run, download, supply-chain cascade, long gate or release occurs.
