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
  replacement-object behavior remain inert during inspection. Clean and smudge
  filters, unsafe links, and loopback contact are also covered explicitly.
- Twenty instruction source classes remain untrusted by default and are
  recorded as discovered separately from read.
- Trust decisions bind source hash, read hash, scope, precedence, conflict, and
  authenticated-decision digest; stale or conflicting evidence disables use.
- The 200-case injection corpus creates zero policy changes, grants, root
  expansions, tool additions, user-intent changes, transfers, or completion.
- Trusted guidance can only accumulate closed authority-reducing constraints.
- Native Fedora 44 and Ubuntu 26.04 guests verify the declared package
  environment, all thirteen Git operations, seven fixture states, four
  discovery classes, nine hostile cases, strict-offline execution, repository
  invariance, and complete teardown.

## Open Evidence

Native macOS worker and attack evidence, the deliberately deferred manual Git
parser fuzz campaign, and independent review remain open. Linux evidence is not
substituted for macOS, and the installed-environment campaign does not claim a
finished user-facing product entrypoint, release approval, or product-wide
acceptance. Sprint 17 is therefore blocked even though its platform-neutral
contracts, local adversarial corpus, and native Linux campaign pass.

The machine-readable source-bound record is
[`local-evidence-report.json`](../../artifacts/sprints/sprint-17/local-evidence-report.json).
The native matrix is retained in
[`installed-linux-git-matrix.json`](../../artifacts/sprints/sprint-17/installed-linux-git-matrix.json),
and the exact control mapping is retained in
[`security-evidence-map.json`](../../artifacts/sprints/sprint-17/security-evidence-map.json).
