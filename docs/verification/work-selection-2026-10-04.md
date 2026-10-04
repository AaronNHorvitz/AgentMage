# Work selection after the coding rows — 2026-10-04

Scope: which open rows are dependency-ready in the implementation lane once the
coding rows are blocked there, under [Decision 0088](../decisions/0088-amendment-coverage-in-work-selection.md).
It was made at `06a68260940df6fc25fd02e7d32ebff3bcfa1ac7` from the committed register
[`remaining-plan-blocker-audit.json`](remaining-plan-blocker-audit.json) (SHA-256
`d177310bbdab6e55654da84ae8d2ea73f80cc0e776e6518ba75823edce90b27b`) over `TASKS.md`
(SHA-256 `ac482721aed2eedfc320e0cddf81ea901201ee2cb5e37102e1cc66c971ab7458`).
This is not a completion ledger. No checkbox, status-model entry or gate changes here.

## Register at that commit

The register classifies 1,644 open rows: 66 local, 1,065 dependency, 163 external and
350 unknown. Twenty-five local rows have no open prerequisite. In the register's own
selection order, which sorts by critical-path rank, then amendment order, then line,
the candidates are:

1. 48.2.4.1, the first coding row. It and every coding row after it need a positive run
   on a native Linux login session. This lane maps no real root, so root-owned
   `/usr/bin/git`, `bwrap` and `systemd-run` fail the executable check, and it has no
   user systemd manager. The row stays first for a native host and is not reclassified;
   [Decision 0061](../decisions/0061-standalone-coding-harness-critical-path.md) item 4
   forbids erasing a blocker to obtain a runnable queue.
2. The 42 open AMR rows, all classified unknown by Decision 0088. Each was assessed
   ([AMR assessment](amr-assessment-2026-09-29.md) and later builder sessions) as needing a native host, a
   configured provider or admitted model, a pinned counterpart contract, or independent review.
3. Story 76.2 (rank 30): 76.2.1.1, 76.2.1.2, 76.2.1.3, 76.2.2.2, 76.2.2.3 and 76.2.3.1,
   each marked `**Execution:** local`. The story is `LOCAL_IMPLEMENTATION_PENDING`; its
   text names the shell and a deterministic-fake first run as repository-local work.
4. Story 76.3 (rank 31) and Story 77.2 (rank 32), whose stories depend on Story 76.2.
5. Then 25.3.1.x (rank 50) and the 308 legacy unknown rows (rank 40 and later).

This order matches the first-release critical path of
[Decision 0048](../decisions/0048-hardened-standalone-desktop-product-and-first-buyer.md)
item 6(d) and `AGENTS.md` section 6. Decision 0061 ranks the coding additions ahead of
desktop work; it does not exclude desktop work once the coding rows are blocked.

## Legacy unknown rows

Earlier builder sessions wrote an external-only report while the 308 legacy unknown rows
had no recorded assessment. Decision 0088 does not let an unknown disposition support
that report, so the report was premature. For each such row, the nearest ancestor story,
sprint or epic was checked for an open prerequisite outside its own subtree. A task is
not counted, because its only prerequisites are its own children.

- 177 rows inherit such a prerequisite from an ancestor.
- 131 rows have none and still need a content assessment. They are mainly native,
  cross-platform, signing, provider, independent-review and model-qualification rows,
  stories and sprints whose remaining gates need those, and a few implementation rows
  in Sprints 42 to 48, 61, 92, 96, 97 and 161.

All 131 rank after Stories 76.2, 76.3 and 77.2, so they do not change the next selection.
They must be assessed before any later external-only or completion report.

## Selection

Story 76.2 is the next dependency-ready unit in this lane.
[Decision 0150](../decisions/0150-standalone-evidence-development-host.md) records its
first increment and the presentation dependency this lane cannot acquire.
