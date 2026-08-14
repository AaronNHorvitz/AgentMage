# Sprint 17 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 17 |
| Local contract result | Pass |
| Sprint result | Blocked |
| Release approval | No |

## Verified Locally

- Thirteen fixed read-only Git operations reject arbitrary commands, mutation,
  executable configuration, unsafe revision syntax, invalid object identities,
  noncanonical paths, and oversized output.
- Clean, dirty, staged, renamed, detached, untracked, and malformed disposable
  fixture states return bounded deterministic evidence.
- Complete fixture bytes, path identities, modes, refs, index, and object files
  are invariant across the full inspection pass.
- Hooks, pagers, aliases, diff drivers, credential helpers, remote URLs, and
  replacement-object behavior remain inert during inspection.
- Twenty instruction source classes remain untrusted by default and are
  recorded as discovered separately from read.
- Trust decisions bind source hash, read hash, scope, precedence, conflict, and
  authenticated-decision digest; stale or conflicting evidence disables use.
- The 200-case injection corpus creates zero policy changes, grants, root
  expansions, tool additions, user-intent changes, transfers, or completion.
- Trusted guidance can only accumulate closed authority-reducing constraints.

## Open Evidence

The production Linux worker has no verified read-only repository-tree mount or
pinned Git helper/runtime yet, so production Git execution remains disabled.
The root-owned packaged worker, native macOS worker, live network observation,
manual parser fuzz campaign, and independent review evidence are also open.
Sprint 17 is therefore blocked even though its platform-neutral contracts and
local adversarial corpus pass.

The machine-readable source-bound record is
[`local-evidence-report.json`](../../artifacts/sprints/sprint-17/local-evidence-report.json).
