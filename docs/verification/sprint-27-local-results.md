# Sprint 27 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 27 |
| Local parser contracts and tests | Pass |
| Sprint result | Blocked |
| Vault write authority | Unavailable |
| Obsidian application dependency | None |

## Verified Locally

- Vault admission accepts only an explicitly selected canonical workspace scope with a nonzero,
  local, unsynchronized, symlink-free storage observation.
- Discovery is stable across input order and supports canonical Unicode and spaced paths while
  excluding configured scopes, hidden entries, directories, and non-Markdown files.
- Foreign and adjacent paths, symlinks, special entries, synchronized inputs, hash drift,
  duplicates, malformed UTF-8/frontmatter, and fixed resource overages fail closed.
- Bounded frontmatter, headings, tasks, aliases, recognized timestamps, wiki links, backlinks, and
  source lines are parsed deterministically outside fenced code blocks.
- Exact paths resolve before basenames and aliases. Ambiguous, unresolved, rooted, and
  traversal-like targets remain visible and never silently become path authority.
- Instruction-like note content stays inert. The capability has no host filesystem, process,
  Obsidian, shell, model, operational-store, or network dependency and exposes no write method.
- The complete local product, documentation, architecture, strict-local, and supply-chain gates
  pass.

## Open Evidence

Sprint 26 remains blocked, so Sprint 27's declared dependency is not closed. Required independent
Sprint 27 review is absent. The parser intentionally creates no vault index, so the index
transaction traces named by the combined legacy security-evidence item remain deferred to Sprint
28 rather than being fabricated here.

Sprint 27 therefore remains blocked despite passing its locally executable parser scope. The
machine-readable record is retained at
[`local-evidence-report.json`](../../artifacts/sprints/sprint-27/local-evidence-report.json).
