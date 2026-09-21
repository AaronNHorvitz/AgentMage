# AgentMage Coding Implementation Brief

## Owner Instruction and Scope

The owner explicitly requested a fresh Codex tmux session with authority to make
implementation decisions and finish the standalone coding work. Running
`scripts/start_coding_codex.sh` authorizes this restart and archives the previous
`STOP-CLAUDE` marker. Model preparation alone did not authorize a restart.

Implement and verify exactly Tasks **48.2.4, 48.2.5, 48.2.6 and 50.2.4**, plus
their necessary prerequisites. This is not the entire numeric range between
those tasks. Do not resume the old `start 76.2.1.1` instruction, desktop work,
other roadmap projects, or work in AgentMagik, CodingMage or USTE.

Start implementation after reading the existing code and contracts. Do not stop
after a plan, a document update, a first passing test, or a progress report.
Continue across scoped tasks while dependency-permitted work remains. Make
routine technical, architecture, dependency and sequencing decisions yourself
under Decision 0054; do not interrupt the owner for routine confirmations.
Record delegated decisions with its required authority/status wording.

Full-access Codex is the implementing tool, not a change to AgentMage's own
permission model. Never remove application approvals, model admission,
confinement, verifier ownership or evidence checks to make a demonstration pass.

## Read First

- `AGENTS.md`, especially Section 8, and Decisions 0054, 0061 and 0062.
- `docs/architecture/standalone-coding-harness.md`.
- `PRD.md` Section 40 and `IMPLEMENTATION-PLAN.md` Section 15.
- The exact task rows, prerequisites and acceptance cases in `TASKS.md`.
- `architecture/status-model.json` and
  `docs/verification/coding-harness-replan-2026-09-21.md`.
- `docs/guides/coding-model-lab.md`,
  `docs/verification/coding-model-preparation-2026-09-21.md` and
  `model-profiles/development/coding-model-lab.json`.

Inspect the actual CLI entry point, host startup, runtime factory, transport,
Linux effect/inference boundaries, family codecs, canonical stores and tests.
Use OpenCode's documented workflow lessons through the accepted architecture;
do not embed its engine or introduce a second execution loop or store.

## Starting State

The planned baseline is commit
`c365d44d73d44c8d13d6810e557882134b8251d0`, branch
`demo/fedora-local-docs`. Check the current tree rather than assuming it has not
changed. There are intentional uncommitted model-preparation and launch files:
the model-lab script/profile/tests, Decision 0062, guides, preparation evidence,
and changes to `AGENTS.md`, `docs/README.md` and the replan handoff.
Preserve and inspect them; do not reset, clean, stash, overwrite or discard them.
After appropriate verification, checkpoint these changes and subsequent work in
scoped local commits. Do not sweep unrelated user changes into a commit.

The previous worker is stopped. A preparation probe passing does not mean the
coding CLI works. The executable currently refuses operational transport, the
ordinary host lacks the completed live composition, and the inference driver
still enforces an 8,192-token profile. Confirm these findings against source.

## Implementation and Acceptance

1. Begin with 48.2.4.1: resolve exact launch, trust, state/key, confinement,
   model/runtime/codec, context and resource prerequisites. An accepted
   development activation contract may be implemented under delegated technical
   authority; do not disable production activation or claim withheld authority.
2. Connect actual CLI/host processes over authenticated private IPC, the existing
   coordinator, native tools, exact grants and verifier. Integrate live events,
   approvals, bounded output, cancellation, follow-ups and failed-test correction.
3. Validate exact profile and actual served context. Do not simply replace all
   8K constants with 32K or remove guards. Keep Muse ATEM and GPT-OSS Harmony
   handling explicit behind the common model boundary, including tools and
   reasoning. HTTP compatibility alone is insufficient.
4. Exercise actual binaries in disposable synthetic repositories with the full
   scripted acceptance matrix. Separately run both prepared models through the
   same repeated coding campaign once their exact admission prerequisites are
   satisfied. A genuine failed test followed by a bounded repair is required.
5. Integrate durable sessions, drift-aware resume, verified full artifacts,
   source-backed retrieval/compaction, bounded user preauthorization and
   conflict-aware rollback using existing owners. Run the specified failure,
   restart, pressure, resource and long-session cases.
6. Retain every failure and rejection, raw logs, exact source/model/runtime/profile
   identities, interventions and measurements. Never substitute a smaller model,
   scripted reply, reduced context or direct model-server probe for live coding
   qualification. Both candidates can fail; report their separate dispositions.
7. Inspect evidence contents before updating statuses. Preserve component,
   executable-scripted, qualified-model, platform and release distinctions.
   Batch source changes before one applicable evidence/SBOM regeneration pass.
   Do not weaken bindings or continuously regenerate historical evidence after
   each individual edit.

## Prepared Models and Shared-Machine Limits

Both exact model artifacts are present and verified. Both 32,768-token
development profiles passed basic reply, native tool proposal, synthetic tool
feedback and more-than-24K-input retrieval probes. They are **not coding-qualified**.
Use the pinned packaged llama.cpp b10423 Vulkan runtime, not the unrelated global
`llama-server` symlink. Do not redownload verified models or acquire more models.
Preserve the existing 8K demo and separate Muse service.

"Light" means lightweight resource use, not low reasoning effort. Run models
sequentially, one slot at a time, using the recorded profile and resource limits.
The development lab is a synthetic diagnostic tool, not an alternate product
transport or permission to attach around native admission.

All builds, tests and evidence regeneration require at least 16 GiB available
RAM at start and the existing systemd scope limits. Use at most four Cargo jobs:

```bash
systemd-run --user --scope --quiet \
  -p MemoryHigh=5G -p MemoryMax=6G -p MemorySwapMax=512M \
  <build-or-test-command>
```

Model-lab commands already create and verify their bounded scopes. Preserve
their four threads, two-core quota, low priority, 45-minute limit, start-time
VRAM check, sampled GPU guard and cleanup. These are not hard GPU utilization
quotas. Never terminate another project's process to obtain resources. Retry
temporary contention with bounded backoff or do non-GPU work; do not spin.

## Authority, Checkpoints and Completion

Authority covers repository-local implementation and the specific AgentMage
runtime/model/evidence paths already authorized by Decisions 0054 and 0062,
plus this launcher's private state directory. Do not spend money, create accounts,
obtain or expose credentials, change licenses/trademarks, change system services,
publish, release, push, force-push, or merge to a default branch. The earlier
one-time push request is not continuing publication authority. Make local commits;
leave a clear branch and commit list for later publication.

Maintain a concise durable progress/blocker log and next-action handoff under
`~/.local/state/agentmage-codex-coding/`. Update it at meaningful checkpoints and
before context transitions. Include actual commands/results, completed and open
rows, source pins and exact blockers; never include credentials. Stop before
starting further work if either of these markers exists again:

- `~/.local/share/agentmage-run/STOP-CLAUDE`
- `~/.local/state/agentmage-codex-coding/STOP-CODEX`

Do not clear a new stop marker yourself. Marker polling is cooperative, not an
immediate kill switch; honor an interactive operator interruption immediately.

Independent review is mandatory for 50.2.4.7. Prepare a pinned-commit review
package for a fresh external reviewer, fix findings when available and verify
fixes. Self-review, another pass in this session, or automated gate-pin renewal
is not independent review. Do not claim `M-HARNESS-DAILY` before its actual gates
pass. Do not claim broader sprint, supported-platform or release completion.

When genuinely blocked, record the exact missing authority/artifact, attempted
resolution and outside owner; continue other unblocked work in this scope.
When only external blockers remain, deliver an honest ready-for-review or
blocked handoff rather than fabricate completion or expand the roadmap. Otherwise
keep implementing and testing until the requested coding work is actually done.
