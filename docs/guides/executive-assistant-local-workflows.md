# Executive Assistant Local Workflows

## Input Rule

Use only canonical records already read under an exact local grant. A file path, Obsidian link,
message export, or schedule export is not evidence until its bounded source identity, revision, and
content hash are present. Do not treat a proposed, inferred, historical, disputed, or unknown item
as a confirmed decision or commitment.

## Cycle and Portfolio Views

Start-of-cycle views group schedule, priorities, deadlines, waiting items, decisions, and
preparation. Closeout views separate completed, unfinished, waiting, and blocked records. Recurring
reviews retain accomplishments, aging work, project movement, decisions, risks, and upcoming
meetings. Portfolio and status views use the same canonical snapshot, while changes-since and
forgotten-item views compare immutable revisions and preserve history.

Use the following evidence-backed skeleton:

```markdown
# Briefing

## Purpose or Cycle

## Confirmed Priorities and Schedule

## Waiting, Decisions, and Approvals

## Inferred, Historical, Disputed, or Unknown

## Risks, Conflicts, and Preparation

## Limitations and Exact Sources
```

## Correspondence

A local draft must preserve its user-reviewed recipients, tone profile, claims, truth states,
attachments, and source identities. The deterministic checker reports unanswered questions,
accidental commitments, unclear dates, missing attachments, unsupported claims, sensitive content,
and uncertain names. A finding-free review means only that the local checks are complete; it is not
send approval.

```markdown
# Correspondence Draft

**Recipients:** [user-reviewed labels]

**Subject:** [bounded subject]

[local draft]

## Review Findings

## Material Claims, Evidence States, and Sources
```

## Message Export Triage

Only user-provided local exports are admitted. Classification is closed to action required,
response required, decision required, reference only, waiting, duplicate, and uncertain. Triage
cannot connect to an inbox, select a recipient, send a response, notify anyone, or update a source.

## Audit View

For every briefing, ranking, tracker row, reminder, or draft, show the canonical record identity,
privacy class, evidence state, deterministic method, exact source identities, conflicts, and
limitations. Plain-folder and Obsidian records use the same projection contract; storage grammar
does not change ranking or truth state.

## Local Reminder Lifecycle

Create a reminder only from an exact source-backed task, commitment, approval, meeting, deadline,
or waiting item. Snooze and reschedule require a new future local instant; acknowledge and complete
remove the active due instant without changing the source record. Every action must name the last
observed reminder digest and a unique event identity. A stale digest, replayed identity, broken hash
chain, or effect-bearing record is refused. Reminder persistence may restore this state after a
restart, but it cannot notify anyone, change a calendar, or mutate the canonical source.
