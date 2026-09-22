# Standalone Coding Harness Implementation Verification — 2026-09-22

## Scope and source identity

- Source candidate: `5ccc77e3f420cc833ed1e51268a39e3fa3ee08e5`.
- Planned baseline: `c365d44d73d44c8d13d6810e557882134b8251d0`.
- Scope: Tasks 48.2.4, 48.2.5, 48.2.6 and 50.2.4 only, plus their necessary
  prerequisites.
- Implementing authority: Decision 0054 and the owner's 2026-09-22 restart instruction.
- Verification profile: pinned Fedora Linux development venue, actual `agentmage` CLI and
  `agentmage-host`, authenticated private IPC, native tools and canonical stores.

This record does not assert production activation, qualified-model, supported-platform,
independent-review, `M-HARNESS-DAILY`, package, or release completion.

## Implemented connected path

The source candidate composes a separate development activation through the ordinary CLI and host,
the existing coordinator, family-specific model codecs, native tool catalog, Linux effect boundary,
fresh exact grants, verifier, SQLCipher journal, artifact owner and checkpoints. The scripted source
is admitted only by its visibly non-production fixture profile. Muse ATEM and GPT-OSS Harmony use
separate exact 32,768-token development profiles and the packaged llama.cpp b10423 Vulkan runtime.

Live control uses a host-owned runtime worker and an independently drained bounded event stream.
Actual IPC covers progress, approvals, cursor expiry, slow subscribers, cancellation and shutdown.
Successful structured writes retain exact immutable change records. History is session/task bound;
rollback requires a fresh high-risk exact-path grant and the original postimage as its expected
current digest, so a concurrent human edit refuses without overwrite.

Consented continuity retains exact model-visible packets and model results through the existing
artifact owner. Follow-ups verify and reopen source-backed bounded projections. Safe restart resumes
the exact request and continuation without replay; source, worktree, model, profile or policy drift
refuses before another effect. Direct session preauthorization is path/template exact, expiring,
budgeted and revocable; every admitted operation still receives a fresh kernel-owned grant.

## Actual-process verification

The 16-case executable-scripted matrix passed from fresh repositories. It covers failed-test repair,
new file, bounded multi-file, exact rollback, no-op, denial, cancellation, stale and replayed
approval, event overflow, false completion, invalid activation, expired cursor, approval/cancel
race, artifact-page corruption and rollback conflict. Raw logs are retained under:

`~/.local/state/agentmage-codex-coding/runs/2026-09-22-effect-boundary-matrix`

The daily-use campaign used binaries with these exact digests:

| Binary | SHA-256 |
|---|---|
| `agentmage` | `407deef58d5bf52863436d4c0c841ca67dc72043fa177da677bce3aead275e33` |
| `agentmage-host` | `1f447b9a0abc149dabe96e5aaedabe7e8f61d30f39bd3dc2b26f50a56d5e9fdc` |

Every declared Decision 0065 local threshold passed:

| Case | Result |
|---|---|
| Repeatable setup | 3 of 3 fresh private setup/diagnose/no-op runs passed |
| Disk pressure | 1,024-byte bound refused before model/tool effect; exit 5; clean tree |
| Output pressure | complete individual outputs admitted; aggregate ended `EXHAUSTED`; exit 7; no partial output |
| Worktree contention | second owner refused in 0.0023 seconds against a 5-second ceiling |
| Cancellation | original owner ended `CANCELLED` in 0.2192 seconds against a 30-second ceiling |
| Long session | initial run plus 25 follow-ups; 26 distinct run IDs; one session; all `NO_OP` |
| Artifact verification | 156 complete artifacts independently paged and hash-verified during the soak |
| Soak duration | 488.321 seconds against a 900-second ceiling |

The recomputable JSON report and raw per-case streams are retained owner-only at:

`~/.local/state/agentmage-codex-coding/runs/2026-09-22-daily-current-short`

The first current-source daily rerun used a descriptive work root that exceeded the bounded Unix
socket path during output-pressure diagnosis. It failed before launch and remains retained at
`~/.local/state/agentmage-codex-coding/runs/2026-09-22-daily-current`; the documented short-root
retry above passed without changing a runtime threshold. An earlier platform-launch smoke also
retained the rejected Cargo hard-link assumption and its successful correction in the
`2026-09-22-effect-boundary-smoke` and `2026-09-22-effect-boundary-smoke-retry` run directories.

Safe restart `/tmp/am-resume-4` resumed the same canonical run/session after the injected
post-checkpoint stop, completed the repair and did not replay the earlier validation. Drift case
`/tmp/am-resume-5` rejected resume and retained the independently applied human comment. The slow
subscriber/cancel case is also included in the executable matrix. Supported executable scope is
Python plus the exact `fixture.python-validation@1.0.0` registered command. Structured planner
component coverage for other languages is not represented as executable daily-use qualification.

## Exact candidate dispositions

Both prepared candidates were run sequentially through the same actual CLI/host
`failed-test-repair` campaign without fallback, context reduction or threshold changes.

| Candidate | Result | Retained raw response | Peak sampled total GPU use |
|---|---|---|---:|
| Muse Glimmer | exit 8, first model call `FAILED`, no tools, `model.muse-codec.final-channel-invalid` | `388d7f34d34d2a43311177e418687e131bfcab9038754378f3570b2aa724cc09`, 1,813 bytes | 17,002 MiB |
| gpt-oss-20b | exit 8, first model call `FAILED`, no tools, `model.gpt-oss-codec.final-channel-invalid` | `0bc7606878b8242372d2844fe6b55df778f3770cb70e4c72e234d525cbf5d540`, 11,906 bytes | 13,486 MiB |

Both resource guards completed without an error and both worktrees stayed clean. Raw logs and
rejection payloads are retained under
`~/.local/state/agentmage-codex-coding/runs/2026-09-22-daily-1` in `muse-logs`,
`muse-rejections`, `gpt-oss-logs` and `gpt-oss-rejections`. These are passing
*failure-handling* results and separate
`not-qualified` model dispositions. Neither candidate reached the required genuine failed test and
bounded correction, so Tasks 48.2.4.8 and 48.2.6.2 remain open.

## Focused regression

- `cargo test -p agentmage-host --all-targets`: 292 library tests passed, 8 ignored; all 15 host
  binary tests passed; no failures.
- The current-source 16-case actual-process matrix passed after the effect-ownership correction.
- The effect boundary, build contract and dependency-class validators passed with 40 focused
  Python tests; both new Linux development-boundary unit tests passed.
- `python3 -m unittest tests.test_coding_harness`: 5 passed.
- The consolidated daily campaign passed under `MemoryHigh=5G`, `MemoryMax=6G` and
  `MemorySwapMax=512M`; the model campaigns additionally used the declared single slot, 200% CPU
  quota, four threads, low priority, VRAM guard and 45-minute ceiling.
- Markdown lint, Python compilation, Rust formatting and `git diff --check` passed.

## Remaining boundaries

Task 48.2.6.2 remains dependency-blocked by the absence of an admitted candidate that can complete
the exact live campaign. Consequently its dependent reconciliation and milestone rows cannot be
closed by substitution. Task 50.2.4.7 requires a fresh external independent reviewer of the pinned
connected source. Task 50.2.4.8 and `M-HARNESS-DAILY` remain blocked on that review and all upstream
task dependencies. Production model activation, platform support and release gates are unchanged.
