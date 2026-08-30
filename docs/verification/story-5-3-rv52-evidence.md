# Story 5.3 RV-52 Evidence

This record covers only the Story 5.3-applicable, repository-controlled contract slice of reviewer
protocol `RV-52`. It uses synthetic identities and executes no model inference, runtime effect,
network access, persistence restart, installed client, or native-platform campaign. The complete
protocol and its later-story acceptance tests remain open.

## Retained lineage

The generated `rv52-workflow-lineage.json` retains nine ordered evidence nodes:

1. admitted closed workflow graph;
2. deterministic workflow-state event fingerprint;
3. fresh attempt identity;
4. current single-use grant admission;
5. conservative effect classification;
6. fresh single-issue receipt identity;
7. current receipt-bound verification;
8. verifier-owned terminal-state resolution; and
9. replay refusal across the complete prior-use ledger.

Every node binds one exact passing Rust test marker. The lineage is an authority-free synthetic
contract record, not a runtime receipt or evidence that an effect occurred. Integrity tests reject
missing, reordered, duplicated, or widened lineage and reject any attempt to relabel the record as
a complete `RV-52` pass.

## Applicable protocol results

The focused run verifies graph/event/terminal ordering; fresh attempt, grant, receipt, and
verification identities; conservative effect handling; fail-closed missing, duplicate, reordered,
corrupt, stale, and replayed inputs; and sticky uncertain effects. These results support the Story
5.3 workflow contract but do not complete any `RV-52` acceptance test.

## Open protocol ownership

The following scenarios remain visible as `BLOCKED_LATER_STORY`:

- client loss, host restart, persistence, and current reconstruction — Story 11.3;
- large artifact-backed output, resource pressure, and exactly one terminal observation per live
  call — Story 16.4;
- retention, inspector behavior, and observability non-authority — Story 21.4; and
- the complete installed source-to-terminal workflow — Story 22.5.

Accordingly, `AT-TIO-001`, `AT-TIO-002`, `AT-RESUME-002`, and `AT-OBS-002` remain
`NOT_EXECUTED_OWNER_OPEN`. Story, Sprint, release, native-platform, and full-protocol completion are
not claimed.
