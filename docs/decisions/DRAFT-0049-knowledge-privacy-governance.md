# DRAFT Decision 0049: Knowledge Privacy Governance

**Status:** Draft — awaiting human acceptance  
**Date:** 2026-09-03  
**Scope:** Privacy authority for canonical human-knowledge records  
**Supersedes:** Nothing  
**Preserves:** Decisions 0012, 0021, 0042 through 0046 and all current fail-closed behavior  
**Does not authorize:** persistence, collection, sharing, telemetry, network access, release support,
or any capability beyond accepted Decisions

## Context

Sprint 26 implements an inspectable Markdown authority, disposable derived index, field-level data
dictionary, minimization, classification, export, deletion, and recovery mechanics. Its security
gate requires a human-owned privacy decision before those mechanics can represent product policy.
No coding agent may accept that policy choice or substitute implementation evidence for it.

## Proposed Decision

The owner should decide the allowed knowledge purposes and data classes, lawful/consensual basis,
default minimization and retention, correction/export/deletion expectations, private and restricted
record treatment, backup handling, and the human authority responsible for exceptions. Acceptance
must name any jurisdictional or organizational assumptions and must preserve local-only,
user-controlled operation unless a later accepted Decision explicitly changes it.

## Consequences

Until accepted, Sprint 26 security closure and every dependent gate remain blocked. Existing local
implementation and synthetic evidence remain usable only as non-release mechanics. Acceptance would
authorize policy binding and verification, not deployment or release by itself.
