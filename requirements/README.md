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
