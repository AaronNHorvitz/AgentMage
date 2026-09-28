# Failed runtime start consumption — verification

Date: 2026-09-28. Source: `e703de67d11f45cdf5451c6386d317ef761f505b`,
tree `45f67481f8c17e9c12aa5317681c794caf24d7cd`.
[Decision 0099](../decisions/0099-failed-start-request-consumption.md) governs this
service lifecycle fix. The [retained record](failed-start-consumption-2026-09-28.json)
binds executed source, failures, checks and actual CLI observations.

## Corrected behavior

Both native Chat and live coding services now consume an exact prepared request
before factory composition, after verification, equality and active-run checks.
Composition or subscription failure cannot make that request available for another
attempt. Invalid, substituted and unprepared requests preserve a valid preparation.
The original startup error and each service's existing duplicate refusal remain.
The CLI preserves the startup error when best-effort release is unavailable.

The existing development factory separately refuses its own repeated composition;
no repeated real-model work was observed. This fix establishes service-level single
use. It does not prove release of every factory reservation, model or durable state.
A fresh preparation cannot clear uncertain cleanup, refund authority or authorize
replay. Synchronous model startup control and factory-internal reservation recovery
remain separate open work. Existing model owners, cleanup and wire contracts are unchanged.

## Reproduced failures and checks

The first run compiled and executed ten regressions: **five passed and five failed**.
Recording factories were entered twice after native/live composition failure and
live subscription failure. Both services retained failed preparations until a new
request hit their capacity ceiling. The original source and failed output are retained.

After both fixes, **all ten regressions passed**. They also cover valid requests
surviving malformed or substituted input, fresh preparation capacity, explicit
unstarted release, owned coordinator destruction, initial advance failure and the
CLI error path. The request helper uses an in-memory read boundary and fake model;
the recording factories never load a model. These are component tests, not a native
coding workflow or proof of cleanup after real model execution.

The full host suite returned **292 passed, 50 failed and 8 ignored**, with no filtered
tests. Every failure name and recorded cause matched the preceding retained host
run. The suite remains failed. The unchanged Linux suite was not rerun for this
host-only change; its prior 19 failures remain failures. Formatting, host Clippy with
warnings denied, source boundary/module/status audits and all **575 Markdown files**
passed at the source checkpoint.

## Actual CLI observation

The clean source rebuilt the real CLI, host and read worker. Help, setup, diagnosis
and status succeeded. An authorized scripted fail/repair launch returned **exit 5**
before IPC, tools or inference, with `linux.repository.git_artifact.invalid` followed
by `linux.development.launch-envelope.failed`. Idle stop returned **exit 1**, which
is not active cancellation evidence.

A separate disposable repository containing staged, unstaged and untracked work
returned **exit 1** at wrapper preflight, before creating its launch log directory.
Complete file, mode, HEAD, raw index, index-entry and diff snapshots were unchanged
for both repositories. The three binary identities were unchanged across these
observations. This proves preservation in those refusal paths; editing, failed-test
repair, protected denial, active cancellation and coexistence during work were not reached.

## Evidence and acceptance

The batch wrote the SBOM once; only the host component content hash changed. Its six
affected foundation builders and the bounded source runtime campaign passed, along
with **64 Python tests**. The campaign retains nine commands and 24 coverage rows.
All source fixtures retain their original limits and qualification boundaries.

The automated boundary report passed all 12 source checks. Its associated gate
checks and the security/index checks passed **31 Python tests**. The security
campaign retains eight commands and 30 source inputs. The index retains 16 complete
and one partial mapping; story, sprint and release completion remain false.
These are source automation results, separate from independent agent review.

An ad-hoc privacy audit initially refused a substring shared by a generic machine
label and established public platform identities. All 20 matching fields were
unchanged declared platform or component values; the contextual audit passed without
changing public artifacts. Git then reported an ignored-parent warning while staging
all 13 tracked foundation/campaign files. Their exact staged bytes were verified and
committed without force-add. Both reporting refusals are retained. Neither required
repeating source tests, executable observations or SBOM generation.

The new record retains the preceding record's complete public input set. Historical
observations keep all inputs and their original tested revisions. No global,
transitive or line-span freshness is claimed. Independent agent review remains
pending; source automation does not satisfy it or any human-only gate. Real-model
and manual-user runs are absent, and no task, platform, model or release gate closes.
The required positive Linux coding demo remains blocked by trusted native executable
and user-systemd prerequisites.

Use the [local launch instructions](../LOCAL-TESTING.md). The focused component check is:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 cargo test --locked --offline \
  -p agentmage-host --lib runtime_start_tests:: -- --test-threads=1
```
