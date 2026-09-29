# Research disclosure binding — verification

Date: 2026-09-29. Source: `85d437907c1cae7ffbf3cd4831a8ec2882bb3bdf`.
[Decision 0106](../decisions/0106-search-disclosure-bound-to-requests.md) governs this
AMR-02.2 component change. The [retained record](research-disclosure-binding-2026-09-29.json)
binds the commits, stage commands, exit codes, log digests, inventory and CLI observation.

## Change

The research owner no longer accepts a caller-supplied query or visit label. A schema 2
plan discloses one exact search endpoint, and the scope snapshot and policy digest bind
it. For each prepared request, the owner derives the accounting class. Only the exact
endpoint shape carrying a disclosed query counts as a search. Any other request to the
provider domain is refused. Requests to other allowed domains are visits. Schema 1 plans
keep their exact bytes and policy preimage and stay readable, but cannot reserve. A
refused shape spends nothing and still retains the trusted clock observation.

The defect was established by source inspection: the public entry point took the label
as an argument. Only test fixtures reserved research requests, and no old-source
regression was executed.

## Component checks

The engine library passed 1,294 tests, including six new plan, scope and journal tests.
They cover exact queries, undisclosed and altered queries, missing, extra and changed
fields, other provider paths, visits elsewhere, schema 1 refusal and compatibility, and
reopen. The Linux library returned 183 passed, 19 failed and 51 ignored. The host library
returned 292 passed, 50 failed and 8 ignored. Every failure in both matches its retained
sandbox baseline exactly after process-ID normalization. Clippy first failed on a test
type-complexity lint; after a type alias it passed, as did formatting and the
strict-local, effect-boundary and hostile-network audits.

## Evidence pass

One SBOM write changed only the engine and Linux platform component hashes. The pass
renewed the six foundation reports and the Story 11.1, 11.2 and 11.3 storage reports.
It also renewed their security, acceptance and gate aggregates, the Story 22.1 canary,
and the Sprint 13, codec and Story 13.4 context-profile reports.

Two failures are retained. The Sprint 11 security map first failed its S-011-RT01 check.
That report had not been regenerated since its producer grew to 16 boundary families on
2026-09-07. It was then refreshed from its real tests: 224 seeds across 16 families. The
first Story 11.2 gate rebuild refused an AC1 report made stale by a later storage-security
regeneration. The uncommitted pin edit was undone, AC1 and AC3 were regenerated and
committed, and only then did the routine pin advance. It moved to `2e2b3c7f`, kept all 25
paths and explains 16 changed inputs.

A direct binding inventory against `9bb3a187` finds 131 current bindings. The only newly
stale records are historical verification records, which keep their original pins. This
is not a transitive or line-span freshness claim.

## Actual CLI observation and limits

The CLI, host and read worker were rebuilt at clean revision
`09478e3d77b1d2fed15a62cd8e07ca406cc514fb`. The clean scripted start again exited 5 at
native Git trust. The fixture with staged, unstaged and untracked work was refused with
exit 1. Both kept every byte and Git state unchanged.

Not run: the native research worker and its adversarial DNS and TLS matrix; any search
provider, real model or GPU; the positive native coding workflow; manual user
acceptance; release. The host does not enable research execution, and ask versus
task-authorized approval remains a later composition requirement. No task row or
acceptance gate closes.
