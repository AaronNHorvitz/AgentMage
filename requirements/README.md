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
