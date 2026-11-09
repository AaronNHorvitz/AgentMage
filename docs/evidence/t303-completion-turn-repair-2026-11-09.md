# T-303/T-306: completion-turn repair, 2026-11-09

Status: builder verification complete; scoped gate remains FAIL on retained
baseline failures. Independent core re-review required.
Both parent rows remain open; no row or sub-row is added or closed.

## Finding and repair

The retained [combined-cycle review](../reviews/bb7678ee37c5-local-coding-edit-test-cycle.txt)
has **Verdict: PASS** for exact commit
`bb7678ee37c520b5e41c12b6ec8ddd3fddc351c6`, while medium M1 blocks parent
closure. The final prompt advertised tools despite having no prepared native
effect. Following that prompt faulted the coordinator without a terminal result.

The final context now offers only the original completion schema, with empty
tools and input schemas. It still checks the complete original sealed catalog
and serializes actual history without omitting earlier failures. The existing
coordinator has an explicit process-local completion-only restriction: it records
`runtime.proposal.completion-only` for tool-shaped answers before interpreter,
permission, grant or native dispatch. The restriction takes precedence over an
attached tool interpreter, regardless of attachment order. Valid completion
candidates still reach the original independent verifier. Ordinary turns keep
their original behavior. No new executor, permission or wire field is introduced.

The [fixed-test startup core review](../reviews/b0905e920f75-local-coding-fixed-test-launch.txt)
also arrives during this batch with **Verdict: PASS** for exact commit
`b0905e920f75490a2cee9663c25456617642ad02` and no high/medium finding.
Its low L1 refusal-diagnostic clarity and L2 executable invalid-definition
ordering coverage remain same-row backlog. Both reports are retained unchanged.

## Verification

The first full scoped run exits 1: 2,827 passed, 74 failed and 71 existing
ignored across 46 unfiltered targets. Exactly 72 failure identities match the
unchanged T-301 host/Linux baseline. The other two are incorrect assertions in
the newly added fixtures: unchanged source was compared with an edited file's
bytes, and a candidate without required postcondition evidence was expected to
be accepted. The fixtures now assert the actual initial bytes and the original
verifier's required refusal. No verifier condition or existing test is weakened.
The attachment-order fixture also checks narrowing before or after attaching a
tool interpreter. Initial raw full-gate log SHA-256:
`1e43933768ec229b2f294d9131522e4b25cc2a742f5d4ef2e6c0bdb83a28fa16`.

The final complete scoped gate exits **1 / FAIL**: **2,829 passed**, exactly
**72 retained baseline failures**, **71 existing ignored**, zero measured and
zero filtered across **46 unfiltered targets**. All failure identities match
the unchanged T-301 baseline; there are **zero new failures**. All three named
completion methods pass, including the strengthened original verifier fixture.
The two focused host methods and one focused kernel method also pass, but those
filtered runs are not the full gate. Formatting exits zero in 113.64 seconds
including its reservation wait; all-target Clippy exits zero in 20.64 seconds;
all-target tests exit 101 in 345.96 seconds. Total elapsed time including shared
reservations is 481.81 seconds. All five changed Rust hashes stay frozen through
this final run and the final documentation updates.

```sh
python3 -B scripts/local_agent_fast_gate.py --base b0905e920f75490a2cee9663c25456617642ad02 --timeout 900
```

Every Cargo command reserves its own shared build slot, with two build jobs and
four test threads; the original retained Node tooling is used without a shim or
download. Final prose and both retained reports receive the same lightweight
checks separately. The comparison checker only compares results and provenance;
it does not turn the failed gate into a pass. Final raw full-gate log SHA-256:
`bf45ed737839b1b44f85b124455c3cf058978f6a91ddc2fbbf8feaebcbc4eb30`.

## Limits and remaining work

Host fixtures select the completion phase through the existing test-only seam;
they do not execute a successful native validation. Kernel fixtures use synthetic
model responses and permission boundaries. No GPU or actual model process runs.
The development verifier still refuses certification; linked-pass/current-source
verification, successful native/model workflow, durable recovery, approved Git
and parent development/held-out measurements remain open. Review low L1 host
command-owner transitions and L2 repair-context headroom remain same-row backlog.
No hidden/held-out task, scorer, pin, threshold, dependency, SBOM, status model,
scoreboard, release or owner-ready artifact changes. Supply-chain regeneration
and the long acceptance gate are not run in this repair batch.
