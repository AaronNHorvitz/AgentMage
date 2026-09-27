# Durable Tool-Start and Terminal Publication — Bounded Verification

Date: 2026-09-26. Source parent: `20f5714ad6d7d236f062e2d74966b52c584d0a62`.
Scope: AMR-01 live coding control and AMR-02 native lifecycle prerequisites.
This record distinguishes component checks, actual executable scripted diagnostics
and real-model qualification. The containing commit will identify the final sources.

## Integration defect and correction

The existing authority transaction committed `ToolStarted` with consumed authority
before native execution, but the coordinator published its returned start/terminal
pair only after synchronous execution completed. A client waiting for the live start
could not cancel that active effect. The retained native diagnostic demonstrated
this: the read timed out after 15 seconds, then its start and terminal events arrived
together. No cancellation was sent. This was an integration defect, not model failure.

The existing transaction port now accepts an observation-only callback. Generic
effects, controlled writes and filesystem effects invoke it only after the exact
durable start commit and before native execution. The existing bounded publisher
makes that start visible once. The coordinator validates the full returned event
identities and publishes only the exact terminal successor. Missing, duplicated or
substituted start notifications fail closed, including when an invalid transaction
fixture ignores the callback error.

Failed start observation prevents native execution and poisons the authority owner.
The consumed start remains available for conservative recovery; no completion is
fabricated. No second execution loop, event store, grant, schema relaxation or budget
increase was introduced. Approvals, confinement, receipt ownership and verifier
completion checks remain unchanged. Research dispatch is not yet host-composed;
this change does not claim a connected-research live workflow.

Status: Accepted under owner delegation, 2026-09-20. Authority: Decision 0054.
Verification and independent acceptance remain separate.

Source inspection also found a terminal-publication ordering defect. The event
pump could expose a valid terminal before the separate worker-result channel
delivered its verified outcome. `refresh()` returned successfully while still
busy, then the unchanged projection guard correctly refused that unmatched pair.
A test using a valid existing coordinator fixture, an open result channel and
deliberately withheld outcome reproduced this exact assertion failure. It did not
depend on sleeps, a disconnected channel or invalid evidence.

`refresh()` now requires matching terminal/outcome presence before returning.
The existing 250 ms boundary wait and projection/receipt checks are unchanged.
Tests cover terminal-first refusal, outcome-first refusal and successful matched
presentation. The compiled regression failed before the fix and passed after it;
its retained failing log SHA-256 is
`2d4f23d45f403a8cfd855291fe87881c79186503b1f03f13997482e36e462b8b`.
This is a reproduced component integration defect, not a claim that it caused a
historical model rejection or that this test alone qualifies an executable model.

## Focused verification and actual native diagnostic

The focused driver exited 0: 23 authority, 60 runtime-loop, 43 host-runtime,
six live-control and two feature-fixture tests passed. Four host cases remained
ignored, not passed. Strict kernel, host and read-only all-target Clippy, 47 Python
tests, source/effect audits, formatting and final source-pin checks passed.

The explicit native integration test also passed (54.09 seconds). It runs the actual
CLI, host, authenticated development IPC, permission/grant owner, Linux confinement
and canonical store. Its proposal source is labelled scripted and its read worker
is a separately feature-gated fault fixture in a private copied binary bundle.
The normal product worker and admission paths are unchanged.

The diagnostic executes a genuinely failing registered validation, observes the live
read start and sends CLI cancellation. The CLI exits 6 with cancellation sent. The
canonical transaction's redacted result digest proves actual worker body entry;
timing or a start event alone is not accepted as that proof. The read has one cancelled
receipt, consumed exact authority, no false completion or output artifact, unchanged
workspace content and receipt-bound turn closure before the original cancellation
request/acknowledgement. All grant, task, session, action, approval, operation,
attempt, call and receipt identities are asserted.

Two independent canonical-owner reopens reproduce the exact events, consumed grant,
transaction and receipt. The prior failing-validation artifact is read in full
through the canonical artifact API with its exact digest and scope. No raw database
or key-export shortcut is used. A fresh host with the normal native read worker then
completes inspection, failed validation, patch, passing validation, Git diff/status
and verifier-backed success. Its CLI exits 0 and the fault bundle remains unchanged.
This is fresh-process healthy reuse, not proof of crash-safe cross-host ownership.

The complete focused/native log SHA-256 is
`7772fcb71f9454963ba3dc6e9b7299aa80016a52608cb6bdd8a8a440f5146db4`.
The exact eight-file source patch, including the new integration test, is
`862fda8d123c82e5748e01cc294b77d406f68fd11c838cef7d6a8e2e7e2cf188`.

| Actual copied diagnostic executable | SHA-256 |
| --- | --- |
| CLI | `2526904e16110e301964a774448f061c1648573e61afb73331f620c552ceda75` |
| Host | `ab7072c2686c807eed8d44d43dcc726ad7ab4df49bb62f32a8d383659f0075a8` |
| Normal read worker | `0d9752b938da7a55e362236faefebaf71febc3359257c4b69da9d27b0e62b169` |
| Feature-gated fault worker | `9042ecfeb96be160a032a701df76cc3e32a62e286bf8e72cb62e7d097bbb8b73` |

These are the copies actually executed, not preliminary binaries from a different
Cargo feature selection. Raw logs and synthetic repositories remain retained
privately; no private operator paths or transcripts are published.

## Retained failures

The first native-test build failed because its test-only digest helper assumed the
pinned SHA array implemented hexadecimal formatting. The helper now uses the existing
per-byte encoding idiom. No native diagnostic started in that failed attempt.
Its retained log SHA-256 is
`a10b67501031f4f3281fb04b900797ea8548cd5f2a4577fbb32f8d00865b3594`.

The next attempt compiled and passed Clippy, fixture tests, 47 Python tests and
structural audits, then failed because the CLI never received the start while the
effect was active. Native cleanup was known, the workspace was unchanged, and the
result was timeout/exhaustion, not cancellation success. The callback correction
addresses this exact observed ordering defect; the live assertion was not replaced
with a blind timer. Retained failed log SHA-256:
`a1adce5a318a1a75fd8cc079543f138b6d20a130b26d36e56f2ccc392a7899a3`.

## Broader checks and freshness

Before the terminal-pair fix, the original seven-file snapshot completed broader
checks: 1,648 library tests (63 ignored), 67 integration tests, eight documentation
tests, 20 optional-worker tests, six environment diagnostics and 134 Python tests.
All 22 executable scripted cases passed; inspection verified all 44 raw-log hashes
and exact source/binary identities before any further source change. That broader
log is retained as
`8a0c87c2abd3dcde7b9b93137c1ed7ef74a166dc027b5cfa30616913360e6594`;
its matrix report is
`9fe48cd5c21e50b567fcc7c8ce54d5a25f860cd37460bc52a7a84a5acf73860a`.
Those results remain bound to their earlier snapshot, not silently promoted to the
subsequent terminal-pair correction. The earlier focused/native log is retained as
`40e34f94ec9d2c92b6d6f6a9d1889487aca6face7c983cfc1d2bf5320b1f629e`.

The corrected eight-file snapshot then completed the same broader driver with
exit 0: kernel 1,157, host 322 and Linux 172 library tests passed, with 63 ignored
in total. All 67 integration, eight documentation, 20 optional-worker and 134 Python
tests passed. Six confined worker environment diagnostics, source/effect/dependency/
status audits, builds, formatting and final source-pin checks passed. The environment
diagnostics supply no network request and deny networking; they do not prove TLS,
native research admission or retrieval. Corrected broader log SHA-256:
`8e33aeba228c2d85bbd31bdc5faf6bdf2ea913a57f51049c84bd09f657870bd9`.

All 22 actual CLI/host scripted cases passed again, including genuine failed-test
repair, new-file and multi-file work, no-op, rollback, bounded protocol/argument
correction, native command failure, denial, cancellation, approval races, invalid
activation, stale/replayed approvals, cursor expiry, overflow, false completion,
artifact integrity and rollback conflict. Every report assertion, all 44 raw-log
hashes and the exact source/binary identities were inspected before documentation
or staging changes. Corrected matrix report SHA-256:
`ebef3110ecdf1730f1dc7dc4b3df265da2b085c44fbce594b4fc63dac05b5f7d`.
Its qualification remains `executable-scripted-only` on
`scripted-executable-fixture-32k-v1`, not real-model qualification.

| Corrected matrix executable | SHA-256 |
| --- | --- |
| CLI | `b82db8e2a5590d56f7eb8c7ea76ab30f39fce636b60b0c1ac89358a14bedb930` |
| Host | `c9d97a9a1bca12821630a6f8c78811aa7ec72e1fc7c56d4119cdcaeee0fbd69f` |
| Read worker | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |

Evidence freshness is separate from these executable results. The complete source
and documentation batch precedes the single applicable SBOM/evidence pass. Historical
native, model, platform and aggregate reports retain their original bindings until
their own prerequisites and verification are performed; a passing component test
does not renew them. No input binding or whole-workspace source hash is weakened.

The single evidence pass completed with exit 0. Supply-chain validation,
contract/schema checks, structural configuration/kernel reports and selected
source/workflow/storage/resume reports and their tests passed. All 40 changed
outputs were inspected. JSON changes are digest and byte-length updates plus three
source-parent revisions; no acceptance or qualification claim was widened. Generated
logs retain the passing named tests, current timings and filtered-test population.
Evidence log SHA-256:
`af3962240316462e9fcc03949919ce814c1c9bde0edf0d04afb45bcf2b78e79a`.

The bounded post-pass inventory parsed 748 JSON files and found 602 named whole-file
bindings to changed tracked inputs: 57 current, 12 newly stale and 533 previously
stale or historical. It excludes 1,602 line-span bindings and makes no transitive
or overall acceptance claim. The seven newly affected reports retain these
dispositions at this checkpoint:

| Retained reports | Freshness disposition |
| --- | --- |
| Story 11.1 schema-v3 and S-011-UT01 | Renew their named bounded subsets against the containing source commit; not whole-current-store or crash acceptance. |
| Sprint 16 local and installed worker matrices | Fixture source changed; local renewal and an actual installed-native rerun are separate, neither replaced by this diagnostic. |
| Story 4.1 security map | Structural input reports changed; committed-source renewal remains separate from aggregate gates. |
| Sprint 50 runtime contract review | Renew its automated scope after the source checkpoint; it cannot approve independent review. |
| Sprint 82 local evidence | SBOM inputs changed; its committed-source builder requires the containing source checkpoint. |

Previously stale native/platform/aggregate evidence remains historical, including
the previously recorded whole-store coverage and independent acceptance gaps.
There is no second SBOM write for the committed-source remainder of this batch.

### Committed-source follow-on

The follow-on completed with exit 0 against source commit
`ac55cb59db3057f95a82d66a93d21719d81d115d`, without source edits or a second SBOM
write. The schema-v3 and store-unit builders reran their named bounded checks;
their seven and four Python tests passed. Story 4.1 security mapping and seven tests,
the automated Sprint 50 contract review and its test, and Sprint 82 local evidence
and three tests passed. Unchanged agent-progress evidence, supply-chain validation
and the eight source pins also passed. Follow-on log SHA-256:
`b618373ff38e6c033ee74a50e0e4f11b78a946471e56cdf14bbb0bacf65c4372`.

The five resulting reports were inspected: changes are source revisions, hashes
and two source byte lengths, not widened acceptance claims. Store coverage remains
the named subset, Story 4.1 retains partial/product-incomplete status, Sprint 50
explicitly records no independent human review, and Sprint 82 remains blocked with
live research, native security and privacy review incomplete. Sprint 16 local and
installed matrices remain historical; neither is renewed by these checks. The
prior inventory above remains a point-in-time result, not a transitive freshness gate.

## Remaining work and gates

Command, Git and validation native cancellation/lifecycle work, preparation/sealing
cancellation, pending-manager-job ownership, host-crash and cross-host recovery,
and inter-acknowledgement crash/replay tests remain open. Process-local quarantine
is not a crash-safe ownership scheme. No installed service or second store was added.

Connected research still needs its confined transport/adversarial matrix, exact
dispatch authority, full source artifacts, configured provider and actual-model
research-to-code qualification. The product default remains offline. No model ran
in this batch. Historical Muse eight-case success remains bounded to its original
source; GPT-OSS remains separately unqualified. Independent review, human acceptance,
supported platforms, M-HARNESS-DAILY and release gates remain open. No package or
capability row is closed by these results.
