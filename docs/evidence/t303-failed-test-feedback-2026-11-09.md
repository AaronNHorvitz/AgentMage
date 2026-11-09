# T-303/T-306: failed-test feedback in the original coordinator, 2026-11-09

Status: builder scoped fast gate FAIL with exact inherited failures and none
new; independent core review required.
T-303/T-306 remain open. No row or sub-row is added or closed.

## Change

The original coordinator previously terminated every failed command, including
eligible failed assertions. Trusted native command composition can now opt into
retaining an eligible validation failure for the next endpoint model turn.
Default failure handling remains terminal. The opt-in is process-local, with no
model or HTTP selector. Prepared admission can select it before transferring the
original owner to the existing authenticated service.

The original native boundary checks the entire execution against its retained
result and elapsed observation. It reuses the existing validation parser,
repair-feedback selection and three-attempt ceiling. Admission consumes that
selection once without spending another test attempt; a lost or stopped next
model turn cannot restore a parallel proposal selection. Compilation failures are
eligible without claiming tests ran. Ordinary command failures, malformed and
skipped-only results, exhausted or consumed repair selections refuse. Denial,
cancellation, timeout, uncertainty and non-command effects cannot opt in.

Failed outcomes remain `Failed`, with the original receipt, bounded output and
`ToolFailed` event. The same coordinator retains them for context and independent
verification. No approval, grant, resource budget or clock is renewed. A proposed
edit after a failed test still stops at a fresh exact approval. A model claim
cannot erase a failed check. See [Decision 0172](../decisions/0172-failed-validation-observations-in-the-endpoint-coordinator.md)
and [the guide](../guides/runtime-endpoint-runs.md#failed-test-observations-inside-the-same-coordinator).

This increment connects the failure observation, not a general repair scheduler.
The original native owner remains held by its spent command boundary. Returning
it to edit preparation inside the same run, fresh source context, repair prompts,
subsequent test scheduling, durable recovery, approved Git and parent development
and held-out measurements remain open. No real-model coding score is claimed.

## Verification

The final complete scoped fast gate exits **1 / FAIL**. Its 46 unfiltered targets
have **2,815 passed, exactly 72 inherited host/Linux failures and none new**, 71
existing ignored, zero measured and zero filtered tests. The failed identities
exactly equal the unchanged [T-301 baseline](t301-baseline-failures-2026-10-25.md).
The comparison is a data check, not a green gate or acceptance.

All seven new regression methods pass, as does the expanded native coordinator
method. Formatting exits zero in 35.29 seconds including its reservation wait,
all-target Clippy zero in 8.73 seconds, and all-target tests exit 101 in 308.19
seconds. Total elapsed time including shared reservations is 353.93 seconds.
All seven Rust input hashes stay unchanged throughout the final gate and final
documentation updates. The final evidence/TASKS prose is checked separately with
the same whitespace, Markdown, review-format and public-hygiene checks.

```sh
python3 -B scripts/local_agent_fast_gate.py --base 3f8d1f2219a6ee27a5d15ba9b7d067c0ab42a05c --timeout 600
```

Each Cargo command takes its own shared build reservation, with two build jobs
and four test threads. The original retained Node tooling is used; no shim,
download or resource-ceiling change is introduced. No long acceptance gate or
supply-chain regeneration is run. Task table rows are unchanged from the base.

Final full-gate raw log SHA-256:
`25b04d0e4cf0dc321018b7bbb6e8b6e361153339155a42e356639e84af5c125e`.

The kernel fixtures observe the unchanged failed result in later context, a
fresh edit approval, rejection of the old decision, cumulative charging,
independent refusal of a completion that still contains a failed check,
cancellation, output/model exhaustion and classifier failure reconciliation.
Native fixtures cover opt-in/default behavior, exact result and elapsed-time
substitution, one-time and already-consumed selections, the three-attempt ceiling,
compilation, skipped-only and malformed results. The expanded coordinator fixture
passes a synthetic failed process result through the original parser, durable
grant transaction and actual coordinator before deliberately refusing upstream.
Its default boundary still stops before another model preparation. These are
synthetic model/process/creation observations, not native process or real-model
workflow qualification. No successful task or coding score is asserted.

Development outcomes are retained. The first focused kernel run rejected a new
failure code under the existing closed event vocabulary; the implementation now
uses the unchanged `runtime.tool.failed` code. A first native test incorrectly
assumed an assertion failure with some skipped tests was ineligible. The existing
rules permit repairing such an assertion while never considering it a full pass.
The matrix now explicitly distinguishes that case from skipped-only tests;
the original parser, eligibility rules and thresholds are unchanged. Formatting
was applied after the initial formatting check. A first complete gate had 2,815
passed, exactly 72 inherited failures and none new, 71 existing ignored across
46 unfiltered targets. Final inspection then tightened admission to consume the
original repair selection once; the final full gate is rerun on frozen sources. No lint or test is suppressed.

## Review and limits

The [executable startup independent review](../reviews/3f8d1f2219a6-local-coding-edit-launch.txt)
arrived during this batch: **Verdict: PASS** for exact commit
`3f8d1f2219a6ee27a5d15ba9b7d067c0ab42a05c`, with no high or medium finding.
Its bytes are retained unchanged. Low L1 executable-test portability, L2 shared
terminal nonblocking robustness, L3 approval-window start and L4 committed
root-escape cases remain same-row backlog with earlier lows. Neither parent
closes; the new feedback change requires its own independent core review.

Native Git artifact trust and command-worker admission remain unchanged.
Successful native creation and real-model coding remain unobserved in this
lane. No hidden or held-out task, scorer, task pin, threshold, dependency, SBOM,
status model, scoreboard, long acceptance gate, release or owner-ready state is
changed. No GPU/model process, history rewrite or push is performed.
