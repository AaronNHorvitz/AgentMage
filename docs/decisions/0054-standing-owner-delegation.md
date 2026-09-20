# Decision 0054: Standing Owner Delegation of Repository Decisions

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | Standing delegation of repository decision authority to the implementing agent, and the resource rule change that accompanies it |
| Authority | Repository owner's explicit standing delegation message of 2026-09-20 |
| Preserves | Decisions 0021 and 0040 through 0053, AGENTS.md, the Business Source License, every stable identifier, evidence binding, test and threshold, and every prohibition restated below |
| Supersedes | Nothing. This decision adds delegated authority; it revises no earlier decision's substance |

## Owner direction

The owner instructed on 2026-09-20:

> Standing owner delegation, 2026-09-20. I need to focus elsewhere. From now on
> you make every decision in this repository yourself and never end a turn to ask
> me anything. That covers technical and architecture choices, sequencing,
> tooling, dependencies, how build inputs are obtained, how blockers are
> resolved, and whether to write new decisions. Record this delegation as a
> decision in the repository's own format, quoting this message, and mark
> anything you decide under it "Accepted under owner delegation, 2026-09-20". If
> something is genuinely impossible without me, record the exact blocker and move
> to other work. Do not stop.
>
> First, resolve the rustup-init question yourself using your own recommendation:
> the versioned archive URL pinned to the current stable rustup, with the SHA-256
> verified against the published .sha256 file. Record the version and digest, and
> never use the unversioned URL again. Then continue the Story 9.1 continuation
> and the demo checklist.
>
> One rule changes: do not wait for other agents' Cargo work. Start a build
> whenever free -h shows at least 16 GB available. Keep the systemd scope caps,
> and use up to 4 Cargo jobs.
>
> Everything else in your original instructions still binds you, delegation or
> not. No spending, accounts, credentials, publishing, releases or merges to a
> default branch. No force-push. No weakening of tests, thresholds or evidence
> bindings. No licence or trademark choices. Nothing outside this repository.
> USTE, CodingMage and AgentMagik stay off limits. Never treat your own work as
> the independent review.

## Decision

1. The implementing agent holds standing authority to decide technical and
   architecture choices, sequencing, tooling, dependencies, how build inputs are
   obtained, how blockers are resolved, and whether to write new decision
   records. The agent does not stop to ask the owner and does not end a turn to
   request confirmation.
2. Every decision taken under this delegation carries the status line
   **"Accepted under owner delegation, 2026-09-20"** and cites this decision as
   its authority, so the delegated set is separable from owner-authored decisions
   in later audit.
3. Work genuinely impossible without the owner is recorded as an exact blocker in
   the live run log, with the precise action the owner would have to take, and
   the agent then proceeds to other dependency-permitted work rather than idling.
4. The shared-machine rule is replaced. The agent no longer waits for another
   repository's Cargo work. A build may start whenever `free -h` reports at least
   **16 GB available**. Every build, test and evidence regeneration continues to
   run inside a `systemd-run --user --scope` unit with `MemoryHigh=5G`,
   `MemoryMax=6G` and `MemorySwapMax=512M`, and Cargo may use up to **4 jobs**.
   The prior instruction to wait for a free Cargo window no longer applies.
5. This delegation grants no authority the owner withheld. The prohibitions in
   the next section bind exactly as before.

## Retained prohibitions

The following remain forbidden under this delegation and may not be decided away
by any record accepted under it:

- spending, accounts, credentials, publishing, releases, and merges to a default
  branch;
- force-push;
- weakening any test, threshold, or evidence binding;
- licence or trademark choices;
- any action outside this repository, including the USTE, CodingMage, and
  AgentMagik repositories, their processes, and their scopes;
- treating the agent's own work as the independent review required by a story
  gate.

Product truth reporting is unchanged: `lifecycle_status`, `verification_status`,
and the current implementation truth in `TASKS.md` stay accurate, and no
platform, model, package, release, or independent-review claim is made that has
not been demonstrated.

## Approval record

The owner supplied the standing delegation quoted above on 2026-09-20 and
directed that it be recorded in the repository's own decision format. This record
is that registration. It carries none of the external authorities listed under
retained prohibitions.
