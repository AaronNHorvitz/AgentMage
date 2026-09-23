# Standalone Coding Review Package — 2026-09-23

Status: `READY FOR EXTERNAL REVIEW — NOT INDEPENDENTLY REVIEWED`.
Prepared by the implementing session, not an independent reviewer. Task 50.2.4.7
and all dependent milestone gates remain open. This supersedes the earlier
package's connected-source target, not its historical evidence or findings.

## Immutable source target

- Integrated source/evidence review commit: `064c9b12148859f8ae01e72568021e849bd69512`.
- Review tree: `b8583b3ccb5233e504d5d4ff10c9c7c00a2619bf`.
- Native campaign commit: `ad28b5ca7862a483bb4b9bf74c199d5f994ae43f`.
- Native campaign tree: `c4a49365b09ac2ef146b102bc557989dd37f41ca`.
- Baseline: `c365d44d73d44c8d13d6810e557882134b8251d0`.
- Branch: `demo/fedora-local-docs`.
- Scope: Tasks 48.2.4, 48.2.5, 48.2.6, 50.2.4 and necessary prerequisites.
- Authority: Decisions 0054, 0061, 0062 and accepted development Decisions
  0063–0080; no production permission or independent-review waiver.
- Exact 92 local commits through the review target: the first 74
  [implementation/preparation/evidence commits](2026-09-23-coding-implementation-commits.txt)
  and the subsequent 18 [evidence-maintenance commits](2026-09-23-coding-evidence-commits.txt).

The native and current scripted campaigns used the clean `ad28b5ca` target and
the exact three binary hashes in the [results record](../verification/coding-harness-campaign13-results-2026-09-23.md).
Comparison against the integrated review target finds no changes under `kernel`,
`capabilities`, `platforms` or `shells`. Later maintenance is not new native qualification.
The [renewal record](../verification/coding-harness-evidence-renewal-2026-09-23.md)
identifies the root inventory-test correction, finite documentation-collector
deadline and tests, automatic pins, non-releasing manifest renewal and separate
negative demo-status correction under Decision 0080; none changes runtime source.
Review the immutable target, not a moving worktree. Do not rebuild binaries
mid-campaign, normalize rejected bytes or replay an effect from a checkpoint.

## Evidence and disposition

Read the [architecture](../architecture/standalone-coding-harness.md),
[September 22 implementation](../verification/coding-harness-implementation-2026-09-22.md),
[integration corrections](../verification/coding-harness-real-model-integration-2026-09-23.md),
[prospective criteria](../verification/coding-harness-native-campaign-2026-09-23.md)
and [source-bound results](../verification/coding-harness-campaign13-results-2026-09-23.md).
Muse passed eight of eight real-model development cases on one exact tuple;
GPT-OSS did not qualify. Neither result enables a production profile.
Matrix13's 22 cases, resume8, protocol-resume6 and daily-core10 passed separately.
No independent review or aggregate daily milestone is claimed.

The final Sprint 48 and Sprint 50 component reports passed all thirteen and
seventeen declared commands respectively, including separate complete
documentation runs. Every exact raw stream and source binding was checked in
the private `runs/2026-09-23-batched-coding-final-audit.json` report, digest
`c02146aeae2cfd1a43a1d87b6c1d37687f8b4bb497b6f68d1f1da0adef7635f7`.
These component renewals and gate-owned automated reviews do not close either
broader sprint, admit a model or satisfy this independent review. Failed and
deliberately interrupted collector attempts remain disclosed in the renewal record.

Raw prompts/responses, rejection records, chronological events, exact commands,
collectors, resources, full-artifact verification, final diffs and failures are
under the authorized private root `~/.local/state/agentmage-codex-coding/runs/`.
The repository retains the exact machine-readable Muse summary and its digest.
The private 74-attempt inventory and its explicit older-retention limitations
are listed in the results record. Reviewers need authorized access to those
private artifacts; do not export keys, databases or credentials with the package.
The implementing session's progress histories are diagnostic context, not review.

## Required boundary inspection

Trace the actual CLI through authenticated IPC, the ordinary host, single
coordinator and native effect boundary, ending at canonical verification and
durable evidence. Retain all findings, including these questions:

1. Does the accepted disposable development activation remain separate from
   production admission and bind workspace, profile, peer and process identity?
2. Do closed framing, source-bound requests, exact approvals, consumed fresh
   grants and confined workers refuse replay, drift, unknown scope and foreign
   workspaces without relying on model prose or client authority?
3. Are the ATEM and Harmony templates, raw special-token handling, stop/usage
   accounting, native tool schemas and exact model/runtime identities correct?
   Scrutinize the actual rejection fixtures, not just HTTP compatibility.
4. Does the existing one-correction budget retain invalid protocol bytes as
   non-authoritative data, reject duplicate metadata/JSON fields, account every
   turn and survive safe restart without a second loop or replay?
5. Does Decision 0079 limit read-parser correction to stateless `Malformed`,
   keep `Denied` paths/limits/version/operation terminal and give no new opt-in
   to write, command, validation or artifact owners? Do terminal refusals retain
   zero authority and exact envelope/digest/tool/schema checks?
6. Does Decision 0078 count mandatory context/result/tool/final records within
   unchanged absolute count/byte ceilings and close pressure before an unretained
   proposal can act, while accounting completed inference usage?
7. Are tool call/result pairs exact and atomic, repository content untrusted,
   compaction omissions visible, originals retained and stale/deleted/cross-
   project sources excluded from completion evidence?
8. Do complete stdout/stderr/diff artifacts remain verified and inspectable,
   with truthful truncation, retention and deletion through existing owners?
   Include untracked-file creation receipts, not only ordinary Git diff.
9. Do cancellation, approval races, slow clients, writer exclusion, disk/output
   pressure, descendant cleanup and uncertain effects preserve truthful state?
10. Do safe resume, source/profile/policy/worktree drift refusal, exact change
    history and conflict-aware rollback preserve concurrent human work?
11. Is registered generic-command failure still terminal, while only explicitly
    targeted validation failures are repairable? Does exact command JSON remain
    bound to the user's approved request rather than reserialized substitutes?
12. Do the campaign collector and report accurately distinguish component,
    executable-scripted, native-development, admitted-model, platform and release
    claims? No cherry-picking, favorable cross-pin aggregation or hidden retries.
13. Do the later evidence corrections preserve the full input inventories and
    negative tests? Check the exact dependency count, collection-only deadline,
    stale-demo withdrawal, unchanged release prohibitions, automatic pin scope
    and the retained failed/interrupted attempts against the final raw audit.

Key implementation locations include `shells/host/src/cli.rs`,
`shells/host/src/coding_development_runtime.rs`, `shells/host/src/runtime_tools.rs`,
`kernel/engine/src/runtime_loop.rs`, `kernel/engine/src/source_runtime_context.rs`,
the native family codecs and llama.cpp driver in `platforms/linux-inference/src/`,
Linux effect/launch boundaries, canonical operational/artifact stores and the
actual-process harness/collector scripts. Resolve exact paths from the target;
the boundary questions, not this illustrative list, define the review scope.

## Reproduction and reviewer deliverable

Read `AGENTS.md` and check both stop markers before any run. Every build/test
starts with at least 16 GiB available RAM and the required 5G/6G/512M scope;
use one Cargo job after the retained parallel-link OOM. Models remain sequential,
four threads, two-core quota, low priority, 45-minute limit, start-VRAM check and
sampled GPU guard. No downloads, global runtime substitution or other services.
The [operational guide](../guides/standalone-coding-development.md) supplies actual
CLI/host reproduction commands and the exact registered Python validation scope.
Model reruns are not necessary merely to inspect this package.

The fresh reviewer must return identity and independence statement, date, exact
commit/tree, inspected boundaries, commands and untouched evidence locations,
findings with severity/reproduction/affected invariant, and each disposition.
Fixes require verification against a new exact pin. The implementer may fix and
test findings but cannot sign their independent disposition. Automated review-
pin renewal, this package, self-review and same-session reinspection do not
satisfy Task 50.2.4.7. No `M-HARNESS-DAILY` or broader completion is approved.
