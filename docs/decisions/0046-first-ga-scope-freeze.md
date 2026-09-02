# Decision 0046: First-GA Scope Freeze

| Field | Value |
|---|---|
| Status | Accepted execution-scope decision |
| Date | 2026-09-01 |
| Scope | Reactivates the Decision 0012 stabilization scope freeze over the first-GA delivery set: Epics 0 through 8, 10, 11, and the Universal Story Definition of Done |
| Supersedes | Only the "stabilization scope freeze is inactive" clause of Decision 0021 item 1 |
| Preserves | The complete preserved roadmap under Decision 0021 item 2, all stable identifiers, additions-only history, completed evidence, the Decision 0040 execution venues, the Decision 0045 first-release mandates, evidence binding, and current status truth |
| Does not authorize | Deletion or renumbering of any epic, story, or task; substitution of platform evidence; closure of physical-only rows without physical execution; any change to evidence binding; release publication |

## Context

Measured on 2026-09-01: 1,842 of 5,148 task items complete (35.78 percent), 258,190
lines of Rust, and roughly 2,200 commits, while `lifecycle_status` has read
`scaffolded` since 2026-08-11 with zero integrated workflows. Decision 0012 item 10
states that stabilization progress "is measured by integrated verified paths, not
document volume, checklist volume, isolated scaffolds, or retained historical
evidence." By that measure, progress toward a first GA is zero.

Of the 3,306 open items, 1,007 sit in Epics 9 and 12 through 16, which add
productivity, finance, trusted-operations, and post-GA capability families that are
not part of the first-GA platform matrix of Fedora, Ubuntu, and Windows 11 x64
established by Decision 0040. Roughly 51 Windows rows in Stories 121.2, 123.2,
124.2, 125.3, and 126.2 are designated physical-only by `WINDOWS-BOUNDARIES.md`,
and macOS release rows require M5 hardware, Developer ID signing, and notarization
under Decision 0040 item 5. None of those can close on the current development
workstation.

Decision 0021 resumes execution at "the first authoritative incomplete dependency
gate" across the entire roadmap, so ordering pressure disperses across seventeen
epics instead of concentrating on the set that produces a first GA.

## Decision

1. The Decision 0012 stabilization scope freeze is **active** over Epics 0, 1, 2,
   3, 4, 5, 6, 7, 8, 10, 11, and the Universal Story Definition of Done. This is
   the first-GA delivery set.
2. The Decision 0021 execution rule resolves "first authoritative incomplete
   dependency gate" **within the frozen set only**.
3. Epics 9, 12, 13, 14, 15, and 16 remain preserved, numbered, and `active` in
   disposition, but are **outside the freeze** and are not executed until a later
   decision lifts it.
4. The physical-only Windows rows in Stories 121.2, 123.2, 124.2, 125.3, and 126.2,
   and any macOS row requiring M5 hardware, Developer ID signing, notarization, or
   Gatekeeper evidence under Decision 0040 item 5, carry disposition **`blocked`**
   with the blocker "physical platform unavailable". They remain inside the frozen
   set and reopen when the platform exists. No agent may close them by
   substitution.
5. Epic 10 executes through the Decision 0040 item 2 local Windows 11 KVM guest
   lane. Its external prerequisites, a properly licensed Windows 11 x64 image and
   host virtualization packages, are external blockers, not repository work.
6. The "Stabilization scope freeze" fields in `TASKS.md` and
   `IMPLEMENTATION-PLAN.md` read `active (Decision 0046)`, and
   `architecture/status-model.json` registers `ADR-0046` as an amendment.
7. Nothing in this decision alters evidence binding, the `AGENTS.md` batching
   rules, or any Decision 0045 release-blocking mandate.

## Verification

- `TASKS.md` and `IMPLEMENTATION-PLAN.md` "Stabilization scope freeze" fields read
  `active (Decision 0046)`.
- `architecture/status-model.json` `amendment_decision_ids` includes `ADR-0046`.
- `AGENTS.md` section 6 directs agents to select work from the frozen set only.
- Baseline for later comparison, recorded 2026-09-01 22:55: 1,842 done and 3,306
  open overall; frozen-set open count 2,248; outside the freeze 1,007; blocked by
  physical-platform unavailability approximately 55.

## Rationale for the frozen-set size

At measured batched throughput, 2,248 items is roughly 30 to 45 days to a
verifiable Fedora, Ubuntu, and Windows 11 first-GA candidate. The full 3,306 items
at the same throughput is roughly 45 to 66 days, of which about 1,007 items produce
no first-GA capability.
