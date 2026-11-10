# T-303/T-306: cancellable original repository reads

Status: independent core review required. T-303/T-306 remain open without new
rows or sub-rows. The complete affected gate remains FAIL.

The original-owner checker previously checked cancellation around complete Git
observations while the reads inside them used unrelated fallback tokens. Its
caller token now reaches preservation workers, linked inventory and immutable
base type/tree/blob reads. A cancelled worker result cannot produce accepted
sources or a preservation manifest, even when its exit status is successful.
No later observation starts after a stop is observed.

The existing collector gains `collect_cancellable`; the synchronous `collect`
interface delegates to the same complete implementation without a caller token.
All prior manifest fields and hashing inputs remain. The extraction into a
private preservation module adds no public observation-substitution interface.
Checkpoints surround bounded local filesystem phases and final sealing. They
do not preempt filesystem operations or scans. Existing per-worker deadlines
remain; no total collection deadline is claimed.

Candidate-tree construction and signed-commit preflight/postflight use the same
caller token. Scratch cleanup still precedes candidate postflight. Cancellation
after effects yields no rollback or preservation claim; existing uncertain
result handling remains required. The installed blocking checker itself still
needs host-managed bounds. Trusted executable admission, confinement, output
limits and descendant supervision are unchanged.

Six new methods cover cancellation at all 17 synthetic preservation-worker
returns plus precancellation; shared-scope forwarding to a synthetic running
worker; all four type/tree/two-blob return boundaries; precancellation before
untrusted executable admission; native observation-wrapper refusal before
opening displaced roots; and equal complete manifests with/without a live token,
including optional refs and actual source/index drift. Filesystem operations
and held-root checks are real; worker output, creation and validation provenance
in the review fixture remain synthetic. No native worker or real model workflow
is demonstrated.

## Validation

Initial preservation-focused run: four passes, including three new methods.
The subsequent cancellation selection: 23 passes, one inherited Git artifact
admission failure and one unchanged ignore. These are filtered diagnostic runs,
not gate PASS. The full affected gate validates final source bytes.

The final complete affected gate remains **FAIL**: **1,437 passed, 72 failed,
64 ignored**, zero measured or filtered, across **26 unfiltered targets** of
the host, local API and Linux adapter. Every failed and ignored identity equals
the exact parent baseline; six added Linux methods account for all added passes.
All six new methods pass. Among 82 direct test methods in changed sources,
67 pass and 15 retain their inherited native-admission failures. Formatting,
Clippy with warnings denied and all light checks pass. The ten Rust source
hashes remain unchanged throughout both full runs and afterward.

An initial full run retained the same Rust counts and an additional Markdown
spacing error. That guide error was fixed, the light checks passed, and the
complete affected gate was rerun. The final run takes **199.16 seconds**,
including reservations: formatting 1.35 seconds, Clippy 0.25 and Rust tests
196.14 seconds. No timeout occurs. Final raw log SHA-256:
`76fa805c020cb134e947fb6d3922be7542a590b5d5cbf588c2db81078a095fa6`.
The initial full log and its diagnostics are retained separately. The exact
parent failure/ignore comparison exits zero; this is data comparison only,
never gate PASS.

A separate comparison confirms every preservation-manifest field expression
is identical to the parent after ignoring whitespace. The live-token tests
also demonstrate actual source and index drift changes the bound manifest.
Reproduction scripts, raw logs, source hashes and exact parent comparisons are
retained privately under `/lane/state/tmp/t306-read-cancellation`. The full
runner reserves heavy commands internally; never outer-wrap it. Final light
checks cover documentation and the unchanged retained parent report after
source freeze. No source edit follows that freeze.

The exact parent commit `e8edde37790ca3853f8226b6a0593c7d0fd1b44b`
receives independent **Verdict: PASS**, with no high or medium finding. Its
report is retained byte-for-byte. Low L-1 (direct state-change/result-digest
assertions) and earlier carried lows remain same-row backlog. No parent row
closes. The retained report and final documentation follow source freeze;
final light checks cover their exact bytes.
Trusted checker/artifact integration, full original-owner approval composition,
native admission, repository/signer recovery, model measurements and approved
push remain open. No evaluator, task, hidden/held-out input, prompt, threshold,
dependency, SBOM, lifecycle, scoreboard, GPU, host key, model download, push,
release or readiness change.
