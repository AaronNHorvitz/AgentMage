# Development model request diagnostics — verification

Date: 2026-09-28. Source: `0898b05a6c82bd531704e3f13db412f537ca4f75`,
tree `8f59dba9af7fdebc0a26d7a5bc789bdc69e632a8`.
[Decision 0098](../decisions/0098-development-model-request-diagnostics.md) governs
this development-wrapper change. The [retained record](harness-model-diagnostics-2026-09-28.json)
binds exact source, tests, executable observations and their limitations.

## Behavior and source checks

Diagnosis schema 2 names the synthetic fixture as `fixture_profile_id` and reports
model intent separately. Only one closed `--model` selection from an exactly owned
process becomes an observed request. Missing, conflicting, unrecognized or stale
observations remain unavailable. Every result keeps serving unobserved and
qualification unassessed. A running process or state-key file does not prove either.

The existing process observation supplies both lifecycle and model intent. PID,
start time, boot, UID, executable and exact root checks still bracket the argument
read. Run-record schema 2, lifecycle values, no-adoption behavior and process-descriptor
cancellation are unchanged. No native runtime, wire contract, model profile,
confinement or admission rule changed.

All **43 tests** in the diagnosis, ownership, harness and model-evidence modules
passed. These include six supervised Python children with synthetic arguments,
all three closed selections, absent and ambiguous arguments, private-value omission,
reserved/running/stale/idle diagnosis and unchanged run-record bytes. Those children
are non-model component fixtures. Existing record-contention, identity, cleanup and
process-descriptor cancellation checks remain in the suite. No test acquired GPU
resources. Strict-local source, hostile-network and effect-boundary audits passed;
the full source-batch Markdown check passed all **573 files**.

The initial private, unexecuted source plan predates shortening the new test's
scratch-directory prefix. The actual pre-execution snapshot and all seven source
hashes match the executed and committed source. The source checks passed without a
failed test or audit. Actual launch refusals are recorded below; failures from
earlier batches remain retained under their original pins.

## Actual CLI and host observation

The clean source built the actual CLI, host and read worker. Help, setup, diagnosis
and status returned success. Diagnosis schema 2 reported no model request after
the refused launch, `confinement=false`, and idle lifecycle `ready`.

An explicitly approved scripted fail/repair launch returned **exit 5** before IPC,
tools, protected prompts or model inference. Its native causes remain
`linux.repository.git_artifact.invalid` and
`linux.development.launch-envelope.failed`. An idle stop returned **exit 1** with
no owned session. This is a refusal, not active cancellation verification.

A separate disposable repository with staged, unstaged and untracked work returned
**exit 1** at wrapper preflight, before creating its launch log directory. Both
complete file/mode/HEAD/raw-index/index-entry/status/diff snapshots were unchanged.
All three binary identities remained unchanged across the observations. These
results establish preservation only in those refusal paths; they do not establish
successful editing, failed-test repair, protected denial or coexistence during work.

## Evidence and acceptance boundaries

The full supply-chain check passed without writing the SBOM. No Cargo member,
dependency, model profile or generated runtime report changed. The direct named
binding inventory found **36 newly stale bindings** across
**11 unchanged historical records**, plus **12** older or historical bindings.
Those records preserve every input and their original tested-source pins. This is
not a global, transitive or line-span freshness claim. The new record retains the
preceding record's complete public input set and adds this batch's source.

The first final-document driver omitted the evidence index's `--check` switch and
unintentionally rewrote its source revision and derived index digest. The manifest
validator rejected the resulting change in binding counts. The failed candidate,
changed index and unchanged validation reproduction are retained. Only those two
index fields had changed; its original complete bytes were restored. The correct
check mode was then run. No source test, executable observation or SBOM generation
was repeated for this correction, and no input or acceptance rule was weakened.

The required Linux edit/validate/fail/repair workflow remains blocked by trusted
native executable and user-systemd prerequisites. Native Linux and host suite
failures from the preceding batch remain failures; unaffected full Rust suites
were not rerun for this wrapper-only change. Real-model, manual-user, independent
review and release results remain absent. No task or approval gate closes.

Use the [local setup and launch commands](../LOCAL-TESTING.md). To repeat the focused
component checks through the shared reservation:

```sh
bash /tools/build-slot python3 -m unittest \
  tests.test_coding_harness_model_diagnostics tests.test_coding_harness_ownership \
  tests.test_coding_harness tests.test_coding_harness_model_evidence
```
