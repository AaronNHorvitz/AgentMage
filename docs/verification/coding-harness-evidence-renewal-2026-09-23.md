# Coding Harness Evidence Renewal — 2026-09-23

Status: applicable evidence renewal in progress; native campaigns finished.
Authority: Decision 0054 and the owner's scoped continuation. Routine renewal,
automated pin updates, the exact inventory-test correction and the bounded
collector-deadline adjustment below are
**Accepted under owner delegation, 2026-09-20**. They do not supply independent
review, model admission, supported-platform acceptance or release approval.

## Source and native result

The immutable coding source is `ad28b5ca7862a483bb4b9bf74c199d5f994ae43f`,
tree `c4a49365b09ac2ef146b102bc557989dd37f41ca`. The
[campaign results](coding-harness-campaign13-results-2026-09-23.md) bind actual
CLI/host/worker binaries, profiles, exact prompts/responses, resource samples,
commands, receipts and verified final artifacts. Muse passed eight of eight
development cases. GPT-OSS did not qualify; its failures remain separate.
No unchanged failing native case was rerun to manufacture qualification.

No Cargo workspace-member source changed after this pin. Later commits renew
documentation, evidence and automated gate references. One root Python test's
dependency-count expectations were corrected as explained below; it does not
alter either runtime binary or the native campaign's source identity.

## Single provenance batch

After all runtime changes and native campaigns, `npm run supply-chain:build`
ran once. Existing first-party tree hashing and every evidence input set remain
unchanged. Subsequent checks verified these outputs without rebuilding them:

| Output | SHA-256 |
|---|---|
| `supply-chain/sbom.cdx.json` | `3db9cd92330fb89647df17e84d6ab51c41d949956702568120a4e065ee82bd38` |
| `supply-chain/dependency-provenance.json` | `31ba9f19e339676750fb1f6ffe61ec2061bc53be89c5d37ee0af0fb83723fb1a` |
| `supply-chain/dependency-hashes.sha256` | `d9662b9441835a80d45c83b14b0be4ec1b3e85d34a2ec774cae8de584802f292` |

All builds, tests and renewals start above 16 GiB available RAM and use the
5G high / 6G maximum / 512M swap systemd scope. Cargo uses one job following
the retained earlier parallel-link OOM. No model runs during this renewal.
The existing Ubuntu contract image was present with identity
`sha256:1b8ace9c0258bcd68e7ece0cc606cb4e9f7b81d7c12f748e2571d3398937564b`;
its contract tests use the existing read-only, unprivileged, networkless,
2 GiB container configuration. No image, runtime or model was downloaded.

## Inspected failures and remedies

- The first documentation check rejected nine stale SBOM/provenance/hash
  bindings in three historical demo reports. Decision 0080 withdraws only the
  current demo-acceptance claim using existing states. Historical native results
  and strict validators remain unchanged. The separate demo-evidence check
  still correctly fails; no desktop/demo acceptance is claimed or rerun here.
- The status correction required only the existing planning-manifest hash
  refresh and model-activation report regeneration. All production activation
  profiles, routes and endpoints remain disabled/empty.
- Existing generators renewed affected policy, storage, runtime caller-parity,
  configuration, dispatcher and platform-contract evidence. Unaffected checks
  were retained. Automated pins move only across inspected legitimate input
  changes under AGENTS.md Section 7; they are not external human review.
- Configuration inventory tests exposed an old exact count of 498, although
  live cancellation had already added locked `signal-hook` 0.3.18 and
  `signal-hook-registry` 1.4.8. Their lockfile checksums and production provenance
  were inspected. Assertions now require exactly 500 components, 371 development
  and 129 production. No component was added during renewal and no validator,
  classification rule, negative test or hash coverage was relaxed. The original
  failed test and subsequent 25-test pass are retained.
- The first Sprint 48 collector attempt found the Story 1.3 automatic pin stale
  after the RV-50 renewal. Its exact two changed inputs were inspected, the pin
  was renewed and all eight gate tests passed. The failed attempt remains intact.
- The next attempt passed the complete schema gate and reached the final
  platform aggregate, but the collector's 1,800-second full-documentation deadline
  expired while tests were still progressing. No assertion failure was reported;
  partial stdout/stderr and the timeout are retained. The owned scope subsequently
  reported inactive/dead. Only the full-documentation collection deadline is now
  2,700 seconds; every other command keeps 1,800 seconds. The command inventory,
  required outcomes, hash bindings, product performance thresholds, model budgets
  and systemd resource caps are unchanged. Regression tests require this exact
  finite deadline selection and prove a timeout cannot write a passing report.

Completed checks include requirements/current applicability, contract boundaries,
planning, activation refusal, policy, storage, configuration and unaffected
schema/documentation-tail checks. All affected prerequisite aggregates now pass
with their existing blockers: Story 4.1's eight tests, Sprint 4's seven tests,
Story 7.1's five tests and Sprint 7's four tests passed after renewal. The ten
dispatcher/security tests and eleven platform-contract/security tests also pass.
Markdown lint, 138 Mermaid blocks, 522-document validation and all policy
invariants passed. The automated Sprint 50 runtime-contract review binds the
native source pin, with all six checks true and no independent-human-review claim.
The complete documentation gate and applicable Sprint 48/50 local reports remain
pending; this record does not mark them passed in advance.

## Retention and review boundary

Raw renewal logs are under the authorized private root
`~/.local/state/agentmage-codex-coding/runs/`, prefixed
`2026-09-23-batched-`. Failed stale-binding checks and the failed inventory test
are retained alongside successful retries; none is rewritten as an earlier pass.
The private `retain_evidence_generator_20260923.py` wrapper copies declared
commands' captured stdout/stderr and records command, return code, elapsed time
and digests. It invokes unchanged repository generators and returns their exact
process results; it does not bypass validation or synthesize a pass. Its identity
and the generator identity are recorded per run. Text-mode captures are explicitly
identified as UTF-8 re-encodings; byte-mode captures remain exact bytes.

The [external review package](../reviews/2026-09-23-standalone-coding-independent-review-package.md)
pins the implemented connected workflow. It remains **not independently reviewed**.
The native model results, scripted reliability checks and renewed component gates
remain distinct. Neither model is production-enabled, and neither
`M-HARNESS-MVP` nor `M-HARNESS-DAILY` is asserted.

Remaining outside actions are exact production model/runtime/codec/context/
resource/isolation/quality admission and a fresh source-bound independent review
with findings and verified dispositions. The historical 8K demo separately needs
authorized complete browser/offline/restart reacceptance before current native
promotion; it is not part of this coding execution scope. GPT-OSS's current
unsuccessful profile disposition is a retained test result, not withheld authority
or a claim of general model incapability.
