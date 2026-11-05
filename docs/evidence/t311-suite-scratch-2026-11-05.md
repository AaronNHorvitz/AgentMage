# T-311.17 anonymous scratch and process limits, 2026-11-05

This is a verified increment in the same expanded suite batch under
[Decision 0154](../decisions/0154-local-coding-delivery-refocus.md). It adds no
row or closure. The original reviews and non-closing freeze checkpoint remain
unchanged. Independent development coverage remains 20 of 30 tasks; T-311.18
still has no current-harness model baseline.

## Behavior

Oracle v28 stages both input projections, adapters, shared mode fixtures and
trusted results on one anonymous 128 MiB tmpfs with 32,768 inodes. Each private
worker temporary filesystem has 64 MiB and 8,192 inodes. The inode capacity
accommodates the unchanged branching lifecycle control's 24,000 directories.
The trusted supervisor has a separate 16 MiB setup filesystem. Synthetic roots
and device mounts remain read-only in the workers.

Only the trusted supervisor retains mount-setup and nested root-ID-mapping
capabilities inside its user namespace. Its root maps to the existing
unprivileged host user. Workers drop all five capability sets and disable new
user namespaces. They receive selected anonymous subdirectory mounts, without
the supervisor's control connection, full scratch descriptor, runner directory
or host logs. Their mount metadata contains namespace-local source paths.
Grading-only Git setup runs in the supervisor's namespace, without source history.

A private Unix connection transfers the scratch directory capability to the
host and accepts one trusted exchange. Socket binding uses a pinned parent
descriptor, allowing operator temporary paths longer than a Unix socket address.
The control directory contains only fixed trusted entries; unexpected entries
refuse cleanup and remain intact. Cleanup failure preserves an original interruption.
No candidate-controlled tree is recursively removed from host disk.

Before importing candidate source, each trusted worker copy installs a hard
eight-process limit. Its inherited hard address-space limit remains 512 MiB per
process, giving a derived 4 GiB address-space bound per task namespace. Fork
refusal and inability to raise the hard limit are observed in both roles;
the host limit stays unchanged. This is not a resident-memory measurement,
delegated cgroup limit or aggregate CPU-time counter. Existing 60-second
per-process CPU and 90-second exchange limits remain.

The original worker source stays unchanged. Only its trusted runtime copy gains
the limit prefix after its future import. Reports bind both versions, the new
scratch implementation and the evaluator. The original launcher arguments,
anonymous argument transport, exchange observations, assertion separation,
control profiles and success predicate remain intact. Result publication follows
namespace teardown, mount-reference closure and bounded host-container cleanup.
The [suite guide](../guides/coding-suite.md) records the current limits.

## Verification

Final commands ran sequentially under one shared build reservation, against a
retained source snapshot that remained byte-identical through the whole run.

| Check | Observed result |
| --- | --- |
| Separate synthetic scratch module | PASS, 16 tests |
| Scoped fast gate | PASS, 116 tests in 125.710 seconds for its unit command |
| Existing boundary module | PASS, nine tests |
| Frozen input check | PASS, 30 development and ten held-out definitions; zero held-out assertions and model attempts |
| Python, whitespace and changed-guide Markdown checks | PASS |

The fast gate includes all 20 independent development control pairs and the
existing forgery, policy, protocol, environment, failure and interruption probes.
The unchanged large branching, deep locked-tree and host traversal controls pass.
New tests exhaust shared byte and inode quotas, verify private filesystem
isolation with a two-way barrier, inspect process limits and all capability sets,
inspect every visible process's mount metadata and descriptors, exercise a long
operator path and confirm generated input is refused before mount setup.

The source audit confirms unchanged frozen definitions, hidden files, selected
methods, thresholds, control profiles, original exchange body and admission/success
checks. Existing oracle tests remain byte-identical. The original traversal and
bounded cleanup APIs remain, including their direct diagnostic controls. Only
the existing T-311.17 row is updated; the old ledger and status model are preserved.

Earlier failures remain retained. Initial namespace experiments exposed descriptor
visibility and nested UID-map/setup requirements. The first full gate failed
with one failure and 14 errors from launcher compatibility and temporary-directory
fault injection. A new probe initially expected the wrong existing refusal class;
it was corrected to the unchanged exact refusal code. A later 115-test gate
failed only because the long branching fixture path exceeded the Unix socket
address limit; binding through a descriptor fixed it without changing that test.
The first validation also overlapped an edit to the new isolation probe, whose
inline fixture had an escaping error. Both source snapshots and failed probe
logs are retained; that run does not establish unchanged-source verification.
The final complete run has matching start/end source bindings.

Post-write documentation and hygiene checks are retained separately. No model,
GPU, inference, download, held-out assertion, harness tuning, supply-chain
regeneration or long acceptance gate runs in this increment. Two superseded
queued diagnostics were cancelled before execution using verified owned-process
identities; no other worker or reservation owner was signalled.

## Remaining scope

Expanded T-311.17 stays open. Remaining development and held-out independent
adapters, legacy/Rust resource bounds, legacy cleanup deadlines and mount privacy,
and remaining coverage/admission requirements still need completion. Whole-suite
`aggregate_worker_quotas` remains false. Generated admission stays disabled;
neither a baseline nor expanded-row independent acceptance is claimed.

## Retained artifact bindings

Artifacts stay private; paths below are relative labels within the retained
artifact bundle, not public downloads or independent review reports.

| Artifact | SHA-256 |
| --- | --- |
| complete-v4/fast.log | `36de2e8bdb0c575c46176abdd4b15e8f84bae394c58e452b2b5d9fa722a04786` |
| complete-v4/scratch.log | `d1a9b3def725e95532c96de7c1f6252f5721f0f5bd1cef9ea0fe471666d3b8ea` |
| complete-v4/boundary.log | `e5ab4ccc959f753f95c1c3fb0c59fc42c3d2c249cb2c08c5bb8ee87183e7c8d1` |
| complete-v4/input-check.log | `84b9c98ba317964eca6c16e2de5336673feb571ccd102de84874d985520f1720` |
| complete/fast.log (failed) | `670296be7579deef133515249aacf85e4ae3dccfcca00483a5df9e89c60289f4` |
