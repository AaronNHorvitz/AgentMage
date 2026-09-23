# Decision 0080: Withdraw Stale Current Demo Acceptance

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054, the scoped coding continuation, and the requirement to keep current status truthful |
| Scope | Negative status correction after the single coding-batch SBOM renewal; no demo implementation or execution |
| Preserves | Every historical report, native acceptance input/hash/check, production admission, independent review, resource bound and publication prohibition |

## Observed evidence

After completing the coding source batch and real-model campaigns at `ad28b5ca`,
the one SBOM regeneration correctly changed six first-party component tree hashes.
The existing documentation gate refused nine stale bindings: the SBOM, dependency
provenance and dependency-hashes inputs in each of the browser, offline and
restarted-browser demo reports. The untouched failed log is
`~/.local/state/agentmage-codex-coding/runs/2026-09-23-batched-docs-check-1.log`.
This is a valid refusal, not a reason to rebind old observations to new inputs.

## Decision

1. Preserve the existing demo implementation, 8K profile, separate services and
   every historical native report. Do not run desktop/demo work in this coding
   scope, regenerate a native success without execution, or alter evidence inputs.
2. Withdraw only the **current** demo native-acceptance claim. Keep lifecycle
   `integrated`, set verification to the existing `contract-tested` state,
   disposition to `blocked`, and acceptance to the existing
   `pending-real-browser-offline-restart` value. Earlier native results remain
   historical evidence, not proof about the changed whole-tree inputs.
3. Use the status model's existing pending state without changing its validators.
   Promoting back to `native-tested` still requires every original current-input
   hash and successful browser/offline/restart acceptance. The separate demo
   evidence check remains failing on the stale reports. No demo gate passes by
   this correction; general documentation may accurately describe an open gate.
4. Record the exact remaining action for a separately authorized demo owner:
   re-execute the unchanged complete acceptance against the new exact source and
   SBOM tuple, preserve failures and resources, then assess promotion. The coding
   session does not infer that execution authority or broaden the roadmap.
5. Continue unblocked repository-local coding evidence renewal. Muse's exact
   eight-case development success, GPT-OSS's failed disposition, disabled
   production models and open independent-review/milestone gates are unchanged.

## Verification

Run the unchanged status-model tests, including the mutations that require actual
complete native evidence and reject stale/missing bindings. Retain the separate
demo evidence check's expected failure. No validator, threshold, historical hash
or evidence binding is removed, narrowed, bypassed or renewed by this decision.

Observed: all 20 unchanged status-model tests and status validation passed;
touched Markdown passed. The separate demo evidence check still failed on
`demo-browser-acceptance.json: stale input supply-chain/sbom.cdx.json`, as
required. Logs use `2026-09-23-stale-demo-disposition-check-2.log` in the private
runs directory. The first status edit omitted the required `blocker` evidence
kind; two tests correctly refused it. The decision is now linked with that
kind, and the failed first check remains retained. No test was changed.
