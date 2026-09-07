# Decision 0051: Business Source License 1.1 for the Repository

| Field | Value |
|---|---|
| Status | Accepted product-scope decision |
| Date | 2026-09-07 |
| Scope | Changes the repository license from Apache License 2.0 to Business Source License 1.1 for all versions published from 2026-09-07 onward |
| Supersedes | Decision 0001 item 1 ("AgentMage is distributed under the Apache License 2.0") and the first sentence of Decision 0048 item 8 ("The repository remains Apache License 2.0"); nothing else in either decision |
| Preserves | Every epic, sprint, story, task, and identifier; additions-only history; completed evidence; evidence binding; Decisions 0001, 0046, 0047, and 0048 in all other respects; the Apache License 2.0 grant on every version published before 2026-09-07 |
| Does not authorize | Retroactive withdrawal of any right granted under Apache License 2.0; deletion or renumbering; any change to evidence binding or status truth; textual modification of protected checklist statements that record the historical Apache-2.0 obligations |

## Context

The repository became public on 2026-09-07 while the product is pre-alpha. An open license on a pre-release codebase permits derivative products before the first release exists. Business Source License 1.1 keeps the source visible for review while reserving production and commercial use to the licensor until a Change Date, after which the work converts to an open license. Rights already granted under Apache License 2.0 on earlier versions cannot be and are not revoked.

## Decision

1. `LICENSE` is Business Source License 1.1 with these parameters: Licensor Aaron N. Horvitz; Licensed Work AgentMage; Additional Use Grant none; Change Date 2030-09-07 (four years from first public distribution; applies per version); Change License Apache License, Version 2.0. The LICENSE file is authoritative.
2. `NOTICE` records the parameters and the continued availability of pre-2026-09-07 versions under Apache License 2.0.
3. Every `documentation_contract` document carries the marker `Repository license: Business Source License 1.1 (Decision 0051); versions published before 2026-09-07 remain under Apache License 2.0.`; the documentation validator requires it in every canonical document and requires the LICENSE file to be the Business Source License text with parameters.
4. **Deferred to the next evidence batch, in this order and in one pass:** package manifests (`Cargo.toml` workspace license and inherited crates, `package.json`, `shells/vscode/package.json`, `experimental/model-lab/Cargo.toml`) to the SPDX identifier `BUSL-1.1`; the RPM spec templates; `scripts/build_contract.py`, `scripts/kernel_contract_package.py`, and `scripts/supply_chain.py` expectations; and regeneration of the supply-chain, build-contract, and kernel-package evidence they bind. Until that batch lands, those artifacts still read `Apache-2.0` and grant no rights beyond the LICENSE file.
5. `TASKS.md` rows and protected checklist statements are not edited (additions-only history); the historical Apache-2.0 obligations they record are satisfied by NOTICE.
6. Decision 0001 item 1 and Decision 0048 item 8 are superseded only as to the license name; the security baseline and the commercial model are unchanged.

## Consequences

Third parties may read and evaluate the source; they may not use it in production or commercially before the Change Date without a separate grant. On the Change Date each version becomes Apache 2.0 automatically. Contributions, if ever accepted, require a contributor agreement compatible with relicensing.

## Verification

`LICENSE` begins with the BUSL 1.1 parameter block; `NOTICE` exists; every canonical document carries the item 3 marker; `docs:validate`, `planning-scope:check`, `architecture:check`, `task-graph:check`, the remaining-plan blocker audit, and `tests/test_public_policy_baseline.py`, `tests/test_story_0_2_acceptance.py`, `tests/test_status_model.py` pass on the commit. Item 4 is verified by the batch that performs it.
