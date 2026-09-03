# DRAFT Decision 0050: Knowledge Records Governance

**Status:** Draft — awaiting human acceptance  
**Date:** 2026-09-03  
**Scope:** Records authority for canonical human-knowledge records  
**Supersedes:** Nothing  
**Preserves:** Decisions 0012, 0021, 0042 through 0046 and all current fail-closed behavior  
**Does not authorize:** a records schedule, legal hold, disposition, deletion, publication, release
support, or any capability beyond accepted Decisions

## Context

Sprint 26 exposes lifecycle, backup, restore, export, and deletion mechanics but cannot decide which
human-knowledge records are official records, who owns them, how long they must be retained, or when
hold and disposition duties apply. Those are human governance choices, not implementation facts.

## Proposed Decision

The owner should designate the records owner; classify official, convenience, derived, temporary,
and exported copies; approve retention and disposition triggers; define legal/incident hold scope
and release; assign correction, access, export, deletion, backup, and migration responsibilities;
and state which external policies control. Unknown classifications must remain blocked rather than
receiving a generated schedule.

## Consequences

Until accepted, Sprint 26 security closure and dependent gates remain blocked. Existing lifecycle
tests prove mechanics only and establish no records schedule. Acceptance would permit an exact
policy binding and evidence campaign, not release approval by itself.
