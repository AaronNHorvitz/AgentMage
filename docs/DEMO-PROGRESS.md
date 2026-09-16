# Fedora local desktop demo progress

Owner milestone: 2026-09-15. Branch `demo/fedora-local-docs`, based on current `build/agentmage-ga`; initial worktree clean. Production qualification remains incomplete.

## Dependency checklist

- [x] Inspect current branch, rules, status, September 7 checkpoint and local hardware.
- [x] Reconcile delegated decision identities and G1/G2 registration (48 focused tests).
- [x] Verify existing Muse Q4_K_M + llama.cpp b10423 Vulkan provenance and scoped JSON-schema Q&A.
- [x] Implement bounded folder admission and cited retrieval through AgentMage host/capability runtime (5 tests).
- [x] Implement protected loopback desktop browser UI, local inference, followups and recovery.
- [x] Run real-model browser acceptance, offline isolation, restart and regressions.
- [ ] Consolidate affected evidence after source batch; document, commit and push milestones.

## Observations and decisions

- RTX 4090: 24 GiB VRAM, about 1.6 GiB initially used; system RAM 62 GiB, 38 GiB available; disk ~1007 GiB available.
- Installed llama-server v10361 (14e78ddef); Ollama installed but stopped.
- Muse historical exact-profile closed-proposal quality evaluation remains REJECTED (0/12); this demo requires new scoped document-Q&A evidence and confers no release qualification.
- Practical route: localhost browser shell controlling a Rust AgentMage host document runtime and a local model process. Production native shells retained.
- Synthetic documents only during development; source snapshots read-only; compaction disabled.

## Exact next action

Complete three specifically stale current upstream captures (Stories 22.5, 23.7,
23.8), then the final Story 50.4 core aggregate and active review-gate mutation
checks. Demo acceptance, final ready instance/credentials, current hashes, G1/G2,
review pins, contract index, planning corpora, load campaign, Story 50.3 and 50.4
degradation have passed. Commit/push the remaining evidence and final recovery
checkpoint, verify clean branch/origin equality, and keep the demo running. Do not
regenerate the SBOM again; no further runtime source edits are planned.

## Integration checkpoint

- Application built and launched; protected loopback browser connects to real Rust host snapshot/retrieval process and Muse UNIX socket.
- Fixed model process lifetime: bwrap death monitoring follows its launching thread, so the startup thread now stays alive until model exit. Initial probe/app overlap briefly exhausted VRAM; probe is stopped, only app model remains.
- Browser launch, model readiness, file admission/unsupported reasons pass. First fact question incorrectly abstained; investigating exact prompt/template/decoding before any success claim.
- Final runtime targeted tests 5/5, strict host binary Clippy, governance tests 48/48 pass.
- No production release status promoted; no weights/caches/keys intended for Git.

## Current-question integration repair

The first raw grammar implementation falsely abstained on known facts/follow-ups. A shorter prompt helped initial questions; native chat completions preserve Muse reasoning, but role-history still sometimes answered an earlier question. The demo now sends one explicit CURRENT QUESTION and complete bounded conversation background as untrusted JSON, plus current source evidence. Real browser launch/ingest, known facts/citations, rehearsal follow-up, insufficient evidence and actual 8192-token context overflow pass. The remaining browser cases, offline isolation and full restart are still running/planned; no completion claim yet.

Governance milestone `9a3abe22` is committed and pushed to origin. Relevant regressions: 1710 Rust tests pass, 15 declared ignored; VS Code 95 pass; locked workspace build, formatting and strict lint/security gates pass. Broad workspace test retains five unrelated repository-map scaffold LicenseMismatch failures caused by Apache scaffold fixtures reading preserved BSL LICENSE.

## Full browser cases checkpoint

Known facts, source inspection, rehearsal follow-up, absent fact, actual context overflow, cancellation/recovery, model unavailable/retry and six-turn limit/new conversation pass. The boundary model answered token 44321 and ignored injected SECRET ROBOT instruction; a test assertion incorrectly searched both answer and quoted source attack text. Fixed the helper to inspect answer text separately, which also strengthens all known-fact assertions. Full real browser/offline/restart smoke is next.

## Retained implementation milestone

Implementation/docs milestone `49cd3fb3` is pushed. The full smoke command is now exercising answer-only assertions (source quotes inspected separately), real local chat completions, offline backend/model isolation and full stop/restart. Demo status is implemented/contract-tested pending full acceptance; production current_product markers stay unchanged. After all source/governance inputs are finalized: one SBOM regeneration, rerun final hash-bound real-model acceptance, then renew current evidence in dependency order.

## Acceptance runner diagnostic

Full smoke passed 10 browser cases, including boundary enforcement and malicious-source handling. Its final Host-header test used Node fetch, which replaced the overridden Host and returned 200 for the actual valid host. Independent raw urllib requests correctly returned 403 for missing token, wrong Origin and wrong Host. Updated the runner to use Node HTTP requests for exact header tests; backend protection was already correct. Full smoke rerun remains required before native demo promotion.

## Scoped demo status promotion

The first complete real-model acceptance pass now verifies all four current-bound
browser/offline/restart reports (`python3 scripts/demo_evidence_check.py` passed).
Only `demo_milestones=fedora-local-document-qa` advances to integrated/native-tested;
production product/platform/model qualification, rejected historical configurations,
and every roadmap checkbox remain unchanged. The demo guard verifies all browser
cases, cited inference, honest abstention, scoped external-network denial, restart
metadata, and full source bindings including supply-chain artifacts.

Exact next action: commit this acceptance milestone, regenerate supply-chain once
after the final source batch, repeat the real acceptance smoke for its new hashes,
then regenerate the affected current evidence DAG in dependency order. Historical
Git-revision evidence bundles stay immutable and receive truthful applicability.

## Verified workflow and final evidence renewal

All 11 real browser cases passed, followed by real application inference in a scoped
networkless namespace and a fresh successful interaction after full stop/restart.
All four acceptance reports pass `scripts/demo_evidence_check.py`. Scope-specific
status is integrated/native-tested/verified-local-demo; production qualification
objects and roadmap checkboxes remain unchanged. Milestone `380c643a` retains this
verified scope and inspected acceptance evidence.

The SBOM was regenerated once after the entire source batch. Full real acceptance
is now repeating against those final carrier hashes while current evidence builders
renew in dependency order. Historical revision-bound artifacts stay immutable.
The precise results and known limits are in `docs/verification/demo-acceptance.md`.

## Final real-model acceptance retained

Final source-batch provenance and current acceptance are committed/pushed as
`6105d481`. All 11 browser cases, networkless real application inference and fresh
post-restart inference passed again after the single SBOM regeneration. The four
reports and `status_model.py` pass current validation. The documented launcher
opened the desktop browser successfully; the app/model remain ready at loopback
port 8765. Final browser generation observations: 15 interactions, 4.446–19.650 s;
offline 15.780 s; restarted 15.598 s. Focused governance97/97 and context
registration5/5 pass; task graph check passes. No external blocker remains for the
demo; current evidence renewal is the remaining internal work.

## Fresh final handoff instance

One additional full stop/start rotated the local application/model credentials.
The old application token was actually rejected with 403; the new token worked.
Fresh browser launch/admission/real cited answer all passed (17.051 s). Current
four-report checks pass, and the fifth `demo-final-handoff-acceptance.json` binds
that actual new interaction, final ready PID and browser/full-smoke records. A fresh
authorized desktop tab was opened. No runtime source changes or SBOM regeneration.
