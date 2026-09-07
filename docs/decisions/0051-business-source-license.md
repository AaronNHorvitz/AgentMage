# Decision 0051: Business Source License 1.1 for the Repository

| Field | Value |
|---|---|
| Status | Accepted product-scope decision |
| Date | 2026-09-07 |
| Scope | Changes the repository license from Apache License 2.0 to Business Source License 1.1 for all versions published from 2026-09-07 onward |
| Supersedes | Decision 0048 item 8, first sentence only ("The repository remains Apache License 2.0"); the remainder of item 8 (the sold artifact is the signed, supported build) is preserved |
| Preserves | Every epic, sprint, story, task, and identifier; additions-only history; completed evidence; evidence binding; Decisions 0046, 0047, and 0048 in all other respects; the Apache License 2.0 grant on every version published before 2026-09-07 |
| Does not authorize | Retroactive withdrawal of any right granted under Apache License 2.0; deletion or renumbering; any change to evidence binding or status truth |

## Context

The repository became public on 2026-09-07 while the product is pre-alpha. An open license on a pre-release codebase permits derivative products before the first release exists. Business Source License 1.1 keeps the source visible for review while reserving production and commercial use to the licensor until a Change Date, after which the work converts to an open license. Apache License 2.0 rights already granted on earlier versions cannot be and are not revoked.

## Decision

1. `LICENSE` is Business Source License 1.1 with these parameters: Licensor Aaron N. Horvitz; Licensed Work AgentMage; Additional Use Grant none; Change Date 2030-09-07 (four years from first public distribution; applies per version); Change License Apache License, Version 2.0.
2. `NOTICE` records the parameters and the continued availability of pre-2026-09-07 versions under Apache License 2.0.
3. Package manifests declare the SPDX identifier `BUSL-1.1`.
4. Documents that name the repository license are updated; `TASKS.md` rows are not edited (additions-only history).
5. Decision 0048 item 8 is superseded only as to the license name; the commercial model is unchanged.

## Consequences

Third parties may read and evaluate the source; they may not use it in production or commercially before the Change Date without a separate grant. On the Change Date the work becomes Apache 2.0 automatically. Contributions, if any are ever accepted, require a contributor agreement compatible with relicensing.

## Verification

`LICENSE` begins with the BUSL 1.1 parameter block; `NOTICE` exists; `grep -r "Apache-2.0" Cargo.toml package.json` returns nothing; `docs:validate`, `planning-scope:check`, `architecture:check`, `task-graph:check`, and `tests/test_status_model.py` pass on the commit.
