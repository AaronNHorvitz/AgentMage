# T-311.17 historical-control confinement, 2026-11-05

This increment stays in the expanded suite batch under
[Decision 0154](../decisions/0154-local-coding-delivery-refocus.md). It adds no
row, closure or checkpoint review. T-311.17 remains open: independent coverage
is still 20 of 30 development tasks, held-out independent adapters and remaining
admission work are unfinished, and T-311.18 has no model baseline.

## Change

The public historical Python/Rust graders now stage inputs, drivers and results
on the anonymous 128 MiB, 32,768-inode case filesystem. A trusted supervisor
constructs the fixed worker command and owns the control connection and bounded
host logs. Python grading-only Git setup runs there without source history.
The frozen held-out control entry uses the same runner, preserving its private
input validation and row-close restriction.

Python workers have a hard eight-process limit and private 64 MiB temporary
filesystem with 8,192 inodes. Rust workers have sixteen processes, 768 MiB
address space per process and 768 MiB/32,768 inodes shared by temporary output
and Cargo home. Their installed toolchain and registry are copied from read-only
inputs onto another anonymous filesystem, capped at 1,536 MiB and 32,768 inodes.
This prevents backing host paths appearing in worker mount metadata. No model,
dependency or toolchain is downloaded or installed.

Rust compilation uses one job in the trusted runtime driver copy to fit the
process/thread budget. The original driver bytes, two test threads, four-CPU
affinity and frozen test/source/corpus inputs remain unchanged. Python retains
512 MiB address space per process. The derived address-space ceilings are
4 GiB for Python and 12 GiB for Rust; these are bounds, not measured resident
usage. Aggregate CPU and resident-memory accounting remain unavailable, and
whole-suite `aggregate_worker_quotas` remains false.

Workers drop capabilities, disable new user namespaces, retain network isolation
and receive read-only roots, devices and input mounts. The supervisor limits
each log to 2 MiB and execution to 90 seconds. It kills/reaps its worker before
replying; socket deadlines bound preparation plus execution. Scratch descriptor
closure and supervisor termination release task trees without host traversal,
including locked deep trees. Host cleanup handles fixed trusted entries only.
Counts publish afterward. Original synthetic host lifecycle diagnostics retain
their API and assertions and cannot pass public historical-input admission.

Reports bind both new infrastructure modules and the declared worker policy.
These historical graders still share assertions with candidate code; they do
not establish generated-result integrity. Generated candidates remain refused
before output creation or supervisor launch. See the
[suite guide](../guides/coding-suite.md) for limits and commands.

## Observed verification

All final commands ran sequentially in one shared build reservation. Code and
test source snapshots match at the start and end; no source edit overlapped
this pass. Private logs, reports, counts and source bindings are retained.

| Check | Observed result |
| --- | --- |
| Scoped fast gate | PASS, 89 tests, 17.825 seconds for its unit command |
| Existing Rust adapter module | PASS, 10 tests, 12.805 seconds |
| All frozen development controls | PASS, 30 failing bases and 30 passing references; 89 reference methods; 54.406374 seconds |
| Existing independent-oracle module | PASS, 100 tests, 125.128 seconds; all 20 supported control pairs and existing attacks/lifecycle probes |
| Existing boundary module | PASS, nine tests, 0.612072 seconds for its command |
| Frozen input check | PASS, 30 development/10 held-out definitions; zero held-out assertions and model attempts |
| Static checks | PASS, Python syntax/whitespace and changed-guide Markdown |

Eleven new synthetic tests cover hard process limits and fork refusal, temporary
byte/inode exhaustion, timeout, output truncation, capability sets, host metadata,
read-only mounts, forged outcomes, reused holders, locked deep trees,
interruption/reaping, startup failures and prelaunch generated-input refusal.
The original Rust CLI controls compile and execute the complete frozen corpus.

The audit checks all 60 development reports, 120 log bindings, exact selected
counts and control discrimination. Frozen task dictionaries, hidden bytes,
selections, profiles, thresholds, original test files, Rust adapter, independent
worker and generated-admission guard are unchanged. The original synthetic
diagnostic body and grade predicates are unchanged. Only T-311.17 is updated.

Private artifact commitments:

- Final command manifest: `64613876dc174e4979f7557dcb471c9a98466f364c57e477387f405172fae75a`.
- Matching source snapshot report: `b8e5deddec071d6beae9f326bc3ea000f3061988518f4a6e0b562ed099e193cd`.
- Development control summary: `5be17fb1bf16df053b45316929f676d7947fd2d15e00c56ffb84985df1440315`.

Earlier failed attempts are retained. The first focused run exposed Rust mount
ordering and a new test's incorrect diagnostic expectation. The next run and
an explicit retained diagnostic control exposed compiler thread exhaustion
with two jobs; narrowing runtime compilation to one job fixed it. A first fast
gate exposed routing of an existing synthetic output-limit probe through the
real-control runner. Routing now depends on the unchanged historical admission
result; the original probe and its assertions remain intact. The final complete
pass includes every affected original test, with no threshold weakened.

No held-out assertion, model, GPU, inference, harness tuning, supply-chain
regeneration, long acceptance gate, release or owner-ready action occurred.
