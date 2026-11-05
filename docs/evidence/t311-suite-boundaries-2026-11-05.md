# T-311.17 cleanup, identity adapter and coverage, 2026-11-05

This is another verified increment in the same expanded suite batch, not row
closure. No row is added or ticked. The earlier non-closing freeze review and
the original diagnostic-oracle milestone remain unchanged. T-311.18 still has
no model baseline. [Decision 0154](../decisions/0154-local-coding-delivery-refocus.md)
continues to govern order and granularity.

## Behavior

Independent grading now repairs and removes its writable scratch in a separate
trusted process with a five-second parent deadline, five CPU seconds, 256 MiB
address space, 64 descriptors and no core files. It drops inherited descriptors,
owns its process group and requests termination on parent death. The parent
kills/reaps it even on interruption. No candidate code is imported by this
helper. The original descriptor-relative, no-follow traversal stays intact.

The enclosing case also uses bounded cleanup. Its private container owns any
detached deep-tree fragments. No unbounded recursive-removal fallback runs after
a failed helper. Cleanup failure blocks result/count publication and retains a
private recovery record when writable; interruption stays the primary outcome
even if that record cannot be written. The maximum cleanup wait is five seconds
per tree, with at most two trees for a shared mode-fixture case. This does not
establish aggregate worker limits or descendant quiescence.

Namespace-private temporary filesystems have a 64 MiB byte cap. Read-only
synthetic root and device mounts prevent ordinary-file scratch bypass there.
Shared host-backed mode fixtures and the trusted result mount are outside this
cap. Process memory, inode counts, processes and CPU still lack delegated
aggregate quotas. The lane exposes its cgroup filesystem read-only; no limit or
host configuration is changed. Mount-metadata privacy and legacy grader cleanup
remain open.

Row-closing verification now binds `development-coverage.json` in its summary.
It names every unchanged development task, original hidden digest, full method
selection and count, independent observation coverage when present, and missing
independent adapters otherwise. It states unasserted statement clauses and real
model coding as unestablished. The known disappearing-process race in dev-003
has no retry/pass exemption. All failed controls, including infrastructure
failures, remain retained in their denominator. A synthetic failed-reference
probe verifies no automatic retry and retained failure logs.

The same batch adds the independent adapter for frozen dev-023, using its whole
original identity regression. Both sides have private grading-only Git snapshots.
A candidate entry imports the original candidate module and applies the frozen
test's closed binary substitution. Only inert dictionary values reach the
original assertions. The base retains missing-function startup and an original
test error; the reference passes. Wrong wrapper paths, short commit identifiers
and missing binary entries fail even after patching candidate-side assertions.
Generated input is refused before Git setup or namespace launch.

This raises independent development coverage to 20 tasks. Its identity method
checks field lengths/counts and wrapper path, not digest correctness, Git flags,
concurrent mutation or run recording. Those limits are disclosed in the
definition-bound coverage report and [oracle guide](../guides/coding-evaluation-oracle.md).

## Observations

All suites below use the shared build reservation. No held-out assertion, model,
GPU, inference, download, harness tuning, supply-chain regeneration or long
acceptance gate runs in this increment.

- Initial eight new boundary probes: PASS, 0.493 seconds.
- First selected fast gate: PASS, 129 tests, 107.071 seconds for its unit command.
  Existing 19 independent base/reference pairs run inside that complete module.
- Final nine boundary probes after additional root/device and recovery-write
  checks: PASS, 0.500 seconds. Both real namespace roles exhaust multiple small
  files at the private byte cap and refuse scratch writes through root/devices.
  Cleanup timeout and interruption probes verify the child is dead and reaped;
  locked-tree removal preserves the outside symlink target and its mode.
- The pre-identity combined fast gate: PASS, 130 tests, 107.026 seconds for its
  unit command. The frozen-input check passed for 30 development and ten
  held-out definitions, with zero held-out methods or model attempts executed.
- The three identity adapter probes: PASS, 2.681 seconds, covering both original
  controls, three assertion-patching contrasts and pre-launch generated refusal.

The existing cleanup-failure tests now inject failure at the bounded cleanup
entry and require the fixed content-minimized refusal. Their assertions that
failed cleanup cannot publish a success, counts or wire results are preserved.
No frozen test, original selection, statement, threshold or task pin changes.

The final selected fast gate passed all 133 tests in 112.870 seconds for its
unit command, including all 20 independent base/reference pairs. Python,
whitespace and four-file Markdown checks also passed. The static source audit
confirms unchanged frozen definitions, hidden files, original control profiles,
traversals, protocol and admission guard. Only the existing expanded T-311.17
row changes; the old ledger and non-closing independent freeze report are
preserved. Its initial audit incorrectly treated a changed nested test helper
as another changed test method; correcting that audit filter required no source
or test change. The failed audit log remains retained.

Expanded T-311.17 remains open: remaining development and held-out independent
adapters, aggregate limits, shared-fixture quota, legacy cleanup,
mount-metadata privacy and remaining coverage admission still need completion.
Generated-candidate admission remains disabled. No new independent checkpoint
review is requested.

## Retained artifact bindings

Artifacts remain private. Their basenames and digests identify observations,
not public downloads, independent acceptance or model scores.

| Artifact | SHA-256 |
| --- | --- |
| identity-final-fast.log | `8f30685fee83233b0f7f6f89c8a2cebd865c7f4f531f984801123575a60dac4f` |
| identity-probes.log | `eada2504029041dff3435d027658de8943365cebed1595a3787cd561f4135a51` |
| final-fast-v2.log | `5144625d53449a524ae310671f3de32a8f25bfab4ecc5b02f59a1cd030ad9a43` |
| final-boundary.log | `3b7aa7d144961797bcb160fc89d715985bdf8d596d25d8997116b8b2d778be5f` |
| input-check.log | `d0e13236267595d4818473d3c62bd78441c3892798b74833546c65896b3745bd` |
| development-coverage.json | `a066f816a857bee22a432947e087d40dd257f8c86093502d0215d8841390e779` |
| resource-observation.json | `23ded0ad411a5f8401accb94b42d36449abffc7ffd2ad2e1365161329b31d1c8` |
| initial-source-audit.log | `6f7556b8703342cefba643316a041f400aab607460c34be79e80d7ed02ddca53` |
