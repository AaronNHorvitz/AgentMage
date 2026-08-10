# Requirement Registry

`registry.json` is the deterministic machine-readable inventory of every canonical `AM-*`, `AT-*`, and `CR-*` definition in `Agent-Scaffolding-Inventory.md`. Do not edit it manually.

Generate and verify it with:

```bash
npm run requirements:build
npm run requirements:check
```

## Record Contract

Each requirement record contains:

| Field | Meaning |
|---|---|
| `id` | Stable canonical identifier. |
| `kind` | `product_requirement`, `acceptance_test`, or `competitive_requirement`. |
| `title` | Exact requirement, test-method, or competitive-recommendation text from the defining table. Markdown formatting is preserved. |
| `source` | Canonical document, heading, one-based line, and SHA-256 of the exact definition row including its newline. |
| `release` | Exact target from the defining table; v0.1 acceptance tests inherit `v0.1`. |
| `dependencies` | Ordered unique `AM-*` dependencies explicitly named by the source row. An empty array means the defining table names none. |
| `disposition` | Normalized source disposition: `build`, `verify`, `required`, or `integrated`. |
| `acceptance_tests` | Ordered unique `AT-*` identifiers explicitly named by the source row. An empty array means the defining table names none. |
| `status` | Current implementation status. All records begin as `planned`; documentation integration alone is not implementation completion. |

The top-level source hash covers the complete canonical inventory. Any inventory edit makes check mode fail until the registry is regenerated and reviewed. Schema consumers must reject unsupported `schema_version` values rather than guessing.

## Conflict Policy

`conflict-policy.json` defines the fail-closed provisional rule used when two structured requirements disagree:

- Allowed actions, capabilities, data classes, platforms, releases, roots, and transports are intersected.
- Prohibitions and required controls are combined.
- Numeric resource ceilings use the lower declared value.
- Authority, canonical-store, model, and runtime identities must match exactly or the result blocks.
- An empty allowed intersection, unknown field, malformed value, or unstructured disagreement blocks.
- A mechanically narrowed result remains provisional until an accepted requirement-supersession decision preserves the original requirements and records the replacement.

The resolver in `scripts/requirement_conflicts.py` is pure: it returns an `unchanged`, `provisional`, or `blocked` result and never edits its inputs or project files.

## Coverage Audit

`normative-map.json` maps explicit normative statements in PRD Sections 5 through 17 to the v0.1 `AM-*` backlog. A normative source line is any non-table, non-code line in that scope containing `must`, `cannot`, `never`, `required`, or fail-closed wording. Each mapping pins the current heading, line, and SHA-256 of the exact source line and names one or more product requirements.

The side-effect-free audit in `scripts/requirement_coverage.py` reports and blocks:

- duplicate stable identifiers;
- unresolved product dependencies;
- missing, unresolved, or unassigned acceptance tests;
- incompatible dependency, acceptance-test, or normative-mapping releases;
- unmapped normative PRD statements;
- stale or duplicate statement mappings; and
- mappings to missing or non-product requirement records.

Run the audit alone or as part of the full requirement gate:

```bash
npm run requirements:coverage
npm run requirements:check
```

Use `python3 scripts/requirement_coverage.py --json` for a deterministic machine-readable report. A PRD edit that changes a mapped normative line requires a reviewed mapping update; the checker never silently relocates or guesses a mapping.

## Policy Expectations

`policy-expectations.json` is the generated, machine-readable policy register for all structured exclusions and deferrals in the canonical PRD and inventory:

- every v0.1 non-goal in PRD Section 4;
- every inventory `DEFER` item;
- every inventory `ROADMAP` item excluded until its target release; and
- every rejected default in inventory Section 35A.

Every record has a content-derived stable ID, exact source hash and location, enforcement scope, expected result, promotion rule, status, and `PX-TEST-001` test contract. The shared contract permits only absence, disabled state, or denial before effect and requires evidence that authority, activation, silent policy broadening, and state or external side effects did not occur.

Generate or verify the register with:

```bash
npm run policy:build
npm run policy:check
```

Canonical category counts are deliberate review boundaries. Adding, removing, or reclassifying a non-goal, `DEFER`, `ROADMAP`, or rejected-default row blocks generation until the category-count change and regenerated expectations are explicitly reviewed.

## Additions-Only Baseline

`additions-only-baseline.json` preserves the original semantic content of every stable `AM-*`, `AT-*`, and `CR-*` record and every tagged checklist entry in the canonical inventory. Checkbox completion is operational status and does not change an entry's semantic identity.

The checker allows new requirements, new checklist entries, and added dependency or acceptance-test coverage. It blocks:

- removal of a baseline requirement;
- changes to a requirement's kind, title, release, or disposition;
- removal of a baseline dependency or acceptance test;
- movement of a requirement to another canonical document or heading; and
- removal, wording changes, movement, or policy-label changes for baseline checklist entries.

Run the side-effect-free check with:

```bash
npm run additions:check
```

After an additions-only review, `npm run additions:update` appends newly discovered requirements and checklist entries to the protected baseline. It refuses to write if any prior entry was removed or changed. The baseline must not be regenerated or edited to hide a failure; supersession preserves the original requirement and adds its approved replacement.

## Planning And Release Schemas

The versioned JSON Schemas in `schemas/planning/` define closed contracts for:

- decision records;
- risk registers;
- change logs;
- release manifests; and
- requirement supersessions.

Each contract rejects unknown fields, malformed stable identities, absolute or parent-traversing repository paths, invalid dates, and unsupported schema versions. Accepted decisions, accepted risks, accepted supersessions, and released or revoked manifests require their applicable approval or signing evidence.

The validator also enforces deterministic relationships that are clearer outside JSON Schema: risk scores equal likelihood times impact, logical record identifiers are unique, change timestamps do not move backward, release IDs match versions, component and model-profile identities are unique, and original and replacement requirement sets do not overlap. Validation never fills defaults, coerces values, removes fields, or edits its inputs.

Run the schema checks with:

```bash
npm run schemas:check
```

The valid examples under `schemas/planning/examples/` are executable fixtures, not production approvals or release evidence.
