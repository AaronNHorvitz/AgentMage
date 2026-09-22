# Standalone Coding Harness Independent Review Package — 2026-09-22

## Review status

`READY FOR EXTERNAL REVIEW — NOT INDEPENDENTLY REVIEWED`

This package was prepared by the implementing session. It is not a review, finding disposition or
substitute for Task 50.2.4.7. The reviewer must be a fresh external reviewer and must record their
identity, review date, exact target commit, methods, findings and final disposition.

## Immutable review target

- Target commit: `5ccc77e3f420cc833ed1e51268a39e3fa3ee08e5`.
- Baseline commit: `c365d44d73d44c8d13d6810e557882134b8251d0`.
- Branch at preparation: `demo/fedora-local-docs`.
- Scope authority: Decisions 0054, 0061, 0062, 0063, 0064, 0065 and 0066.
- Implementation commits, oldest first:
  `ab76d781`, `452a789a`, `f8dd4e79`, `ea16322f`, `86823491`, `fe898a96`,
  `0dcd13d0`, `4fc23603`, `d6863f7d`, `690e5e43`, `e30e9ffb`, `03c01ded`,
  `fabe3897` and `5ccc77e3`.

Review the exact target, not a moving worktree. The implementation record is
[`coding-harness-implementation-2026-09-22.md`](../verification/coding-harness-implementation-2026-09-22.md).

## Required review boundaries

The external reviewer should trace at least these questions from CLI input through terminal state:

1. Does development activation remain distinct from signed production activation, bind the exact
   workspace/profile/peer/process and keep the scripted provider unavailable to production?
2. Is there exactly one coordinator, native tool dispatcher, authority owner, journal, artifact
   store and execution loop, with no embedded OpenCode engine or MCP substitution for native tools?
3. Do private IPC framing, peer authentication, cursor rules, duplicate submission handling and
   shutdown prevent replay, confused deputies and foreign-workspace effects?
4. Are Muse ATEM and GPT-OSS Harmony explicit family codecs behind the common model boundary, with
   exact artifact/runtime/context/decoding identities and fail-closed parser behavior?
5. Do approval presentation, fresh grants and direct session preauthorization preserve exact
   targets, arguments, preimages, effects, expiry, budgets, revocation and unknown-scope Ask/deny?
6. Can cancellation reach model, approval, tool, validation and rendering phases while preserving
   uncertain-effect truth, descendant cleanup and reserved control capacity under slow consumers?
7. Are stdout, stderr, diffs, request/continuation records and model exchanges retained, paged,
   verified, released and deleted only through the existing artifact and continuity owners?
8. Does resume accept only a safe exact checkpoint, reconcile the durable event/artifact projection
   and refuse source, worktree, model, profile, policy, key or journal-tail drift without replay?
9. Does source-backed retrieval keep repository content untrusted, disclose summaries/omissions,
   verify originals and reject stale, deleted, cross-session and cross-project material?
10. Do change history and rollback bind exact immutable preimages, current postimages, session/task
    ownership and fresh high-risk grants while preserving concurrent human edits?
11. Do pressure, overflow, false-completion, hostile instruction, unavailable model, process failure
    and candidate-codec failures remain non-successful without fallback or hidden broadening?
12. Are all scope labels truthful: component versus executable-scripted versus candidate failure,
    with no qualified-model, platform, package, release or daily milestone overclaim?

## Reproduction entry points

Start only with at least 16 GiB available RAM. Use at most four Cargo jobs and the required scope:

```bash
systemd-run --user --scope --quiet \
  -p MemoryHigh=5G -p MemoryMax=6G -p MemorySwapMax=512M \
  env CARGO_BUILD_JOBS=4 cargo test -p agentmage-host --all-targets
```

The actual-process and daily entry points are:

```text
python3 scripts/coding_harness_acceptance.py --work-root <new-short-root> --log-root <new-log-root>
python3 scripts/coding_harness_daily_acceptance.py --help
python3 -m unittest tests.test_coding_harness
```

Candidate reruns are not required merely to inspect this package. If authorized, they must use the
exact Decision 0062 profiles and packaged llama.cpp runtime, one slot at a time, and must retain all
failures without choosing a favorable repetition.

## Evidence available on the review machine

Owner-only raw records are under:

`~/.local/state/agentmage-codex-coding/runs/2026-09-22-daily-1/`

The directory contains the 16-case matrix streams, daily report and streams, both candidate logs,
and both exact retained candidate rejection payloads. The source-bound implementation report lists
their material hashes and measured thresholds. Current-target matrix and daily results are retained
under `2026-09-22-effect-boundary-matrix` and `2026-09-22-daily-current-short`; the rejected
long-path daily attempt is retained under `2026-09-22-daily-current`. Earlier genuine failed
diagnostics remain named in
`~/.local/state/agentmage-codex-coding/progress.md`; they must not be hidden when assessing the
repair sequence.

## Required reviewer deliverable

Create a separate findings record that includes:

- reviewer identity and independence statement;
- exact target commit and tree hash;
- files and runtime boundaries inspected;
- commands and untouched result locations;
- each finding with severity, affected authority/invariant, reproduction and required correction;
- each disposition as fixed, accepted risk by an authorized owner, or still open;
- verification of fixes against a new pinned commit; and
- an explicit conclusion that does not broaden model, platform, package or release status.

Until that record exists and every required finding is resolved and reverified, Task 50.2.4.7 is
open and Task 50.2.4.8 cannot record `M-HARNESS-DAILY`.
