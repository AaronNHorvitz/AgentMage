# Preparation review fixes and evidence renewal — verification

Date: 2026-09-29. Source: `648267848782fcdafac93c329709c5605bc69e86`.
The [retained record](preparation-review-fixes-2026-09-29.json) binds the commits,
stage commands, exit codes, log digests, binding inventory and CLI observation.

## Scope

An independent read-only review of the Decision 0102 source commit `69cd9adc`
passed with one medium and five low findings. This batch resolves them under
[Decision 0103](../decisions/0103-preparation-cleanup-and-displaced-cancellation.md):

- F1: a failed controlled preparation that loaded the model in the same call now
  unloads it with a validated receipt or reports cleanup uncertainty. The host's
  exact 32K and one-slot check runs inside that attempt.
- F2: readiness client errors after the startup deadline keep the startup category.
- F3: legacy `load` refuses after a controlled attempt.
- F4: legacy spawn failure keeps its category; controlled calls report uncertainty.
- F5: the coordinator's own exact cancellation, displaced by another failure, is
  journaled before the failed terminal without becoming a cancellation.
- F6 was resolved by committing the earlier verification record.

The same batch corrects three evidence-tooling defects that had blocked earlier
refreshes. [Decision 0104](../decisions/0104-markdown-gate-excludes-cargo-build-output.md)
keeps the Markdown gate on repository documents.
[Decision 0105](../decisions/0105-test-owned-source-scans-and-public-trace-paths.md)
adds exact test-owned source scanning with whole-file bindings. It also keeps the
private checkout path out of the Story 13.4 public logs.

## Component checks

With only the F1 and F5 fix lines reverted, the six changed regressions failed and
18 other controlled tests passed; the files were then restored byte-for-byte. On the
fixed source, the engine passed 1,368 tests, Linux inference 166 and contracts 83.
Workspace Clippy, strict-local, effect-boundary and hostile-network audits passed.
Formatting failed first on two test layouts; after formatting it passed and the
focused tests passed again.

The host library returned 292 passed, 50 failed and 8 ignored. All 50 failure
blocks match the retained Decision 0101 baseline after process-ID normalization.
They are native-prerequisite refusals in this sandbox, so the host suite remains failed.

## Evidence pass

The batch ran one evidence pass in six committed stages with one SBOM write. That
write changed only the engine, Linux-inference and host component hashes.

| Stage | Revision | Commands | Result |
| --- | --- | ---: | --- |
| Synthetic catalog hashes | `cb84cc4c` | 4 | pass |
| Core renewals | `3ffd90b3` | 21 of 34 | 20 pass; codec producer refused a dirty tree |
| Codec onward | `75c0339b` | 14 | pass |
| Story 11.2 pin and runtime campaign | `5ac4d147` | 11 | pass |
| Coordinator boundary review | `08e5c815` | 4 | pass |
| Story 23.4 security map and index | `7258eb77` | 7 | pass |

The codec producer requires a clean tracked worktree. Its refusal is retained. It
then ran first from the clean core commit. The Sprint 13 aggregate now binds 265
source inputs, including all 253 scanned kernel files. It reports no production
model-family reference, and its sprint stays blocked on three explicit evidence
classes. The routine Story 11.2 pin kept all 25 paths and explains its two changed
inputs. The runtime campaign peaked at 51,044 KiB under its unchanged 1 GiB limit,
after an explicit prebuild. The security index still has 16 complete and one partial
mappings; story, sprint and release completion stay false.

This closes the agent-progress, Sprint 13 aggregate and dispatch-preflight
refreshes left open by the previous pass. The platform contract refresh remains
open: its Podman image inspection needs a user namespace this lane does not provide.

A direct binding inventory against `6a4359d2` finds 77 current bindings. The only
newly stale record is the 2026-09-28 verification record, which keeps its original
pins. This is not a transitive or line-span freshness claim. Other earlier committed
logs contain the private checkout path; they remain unchanged pending an owner decision.

## Actual CLI observation

The CLI, host and read worker were rebuilt at clean revision
`1a2aa2e2a59928f9e7c5c103f1fa64e7d44a36a7`. A fresh clean scripted
failed-test-repair start returned exit 5 before any tool ran:
`linux.repository.git_artifact.invalid`, then `linux.development.launch-envelope.failed`.
Setup, diagnosis and status passed, and an idle stop returned 1. A fresh fixture with
staged, unstaged and untracked work was refused with exit 1 before a log directory
existed. Both fixtures kept every file, mode, index entry and diff unchanged.

The doctor reports why: in this sandbox, real root is unmapped. The system `git`,
`bwrap`, `systemctl` and `systemd-run` therefore fail the root-owned executable check.
There is also no user systemd manager. Confinement was not weakened to proceed.

Not run: the positive edit, validation, failure and repair workflow; protected denial and
active cancellation through native tools; any real model or GPU use; manual user
acceptance; release. Independent review of the fix commit is requested and pending.
No task row or acceptance gate closes. See [local testing](../LOCAL-TESTING.md).
