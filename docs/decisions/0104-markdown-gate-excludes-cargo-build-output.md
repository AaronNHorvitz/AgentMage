# Decision 0104: Markdown Gate Excludes Cargo Build Output

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0061 and 0081; current owner restart |
| Scope | Repository documentation gate (`docs:lint`) and evidence builders that run it |

## Findings

The full Markdown gate reported five findings in one file,
`target/doc/static.files/SourceSerif4-LICENSE-<hash>.md`. Rustdoc copies that
third-party font licence into Cargo's build directory when documentation is built.
The file is not a repository document: `target/` is ignored and holds no tracked
files. The gate's result therefore depended on whether a local documentation build
had run, and the agent-progress evidence refresh failed on it. All 582 tracked
Markdown documents passed.

The tool's general ignore-file option was considered and rejected. It would also
skip `artifacts/sprints/sprint-92/agent-profile-reference.md`, a tracked generated
document committed despite a broad ignore rule, and so narrow the gate.

## Decision

Add exactly `target/**` to the Markdown linter's configured ignores. Every tracked
or new repository Markdown file stays in scope, including committed generated
reports. The rule set, rule settings, the licence file and the npm script are
unchanged. No evidence artifact binds the linter configuration file.

## Verification boundary

With the change, `npm run docs:lint` linted 583 files, all tracked Markdown
documents plus one new decision record, with no findings. The same command before
the change reported the five build-output findings. This corrects a gate input
defect; it closes no task, review or release gate.
