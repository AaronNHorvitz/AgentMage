# Decision 0098: Development Model Request Diagnostics

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-28 |
| Authority | Decisions 0054, 0061, 0081 and the current owner restart |
| Scope | AMR-01 diagnosis and Task 48.2.4.6 |

## Finding

The development wrapper reports its scripted fixture profile and qualification
label for every diagnosis. The guide describes these as fixture metadata. Launch
also supports two separate model candidates, but status exposes no requested model.
The existing run record stores process ownership, without retaining model intent.
Its lifecycle and confinement diagnosis already have distinct, bounded meanings.

## Decision

Version the development diagnosis JSON as schema 2. Name the fixture identity
explicitly and expose a separate model-request observation. Report only the closed
selection found in an exactly owned process's arguments. Missing, conflicting or
unrecognized selections remain unavailable. Every diagnostic reports serving as
unobserved and qualification as unassessed; a launch argument proves neither.

Reuse the existing process observation for lifecycle, model-request diagnosis and
the boolean ownership check. Preserve the exact recorded PID, start time, boot,
UID, executable and workspace-root checks, including observations before and after
reading arguments. Preserve all lifecycle values and cancellation revalidation.
Keep run-record schema 2 unchanged. Existing records receive no invented model
intent, migration, adoption or cleanup permission.

The observation is temporary and describes a request at that instant. It grants
no inference, tool, cancellation or publication authority. Preserve the native
model/profile/resource tuple, admission, confinement, current run-record protections
and process-descriptor cancellation. This changes the existing developer wrapper;
the Rust runtime, frozen IPC contracts and acceptance-result formats are unchanged.

## Verification boundary

Exercise every closed model selection, absent and ambiguous flags, unknown values,
reserved and stale records, and unchanged lifecycle behavior. Preserve all existing
ownership/race/cleanup tests. Observe a supervised non-model Linux child carrying
synthetic model arguments to verify process ownership and truthful request reporting;
label that observation separately from CLI, inference and model qualification.

Run the existing harness, ownership and acceptance-builder regressions. Retain an
actual CLI setup/diagnosis/start observation at the new source, with pre-existing
work preserved and native prerequisite refusals recorded as refusals. Batch the
source and guide changes before renewing applicable evidence. No task, independent,
human-only, native/model, platform or release gate closes with these diagnostics.
