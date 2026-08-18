# Story 23.4 Local Verification Results

| Field | Result |
|---|---|
| Scope | Interface-independent coordinator, native-read matrix, client parity, and source-security evidence |
| Local focused result | Pass |
| Complete Story 23.4 result | Blocked |
| Sprint 23 result | Blocked |
| Release approval | No |

## Verified Locally

- `RuntimeRunRequest`, `RuntimeApprovalChallenge`, approval response, and
  `RuntimeOutcome` are closed, versioned, canonical digest-bound contracts.
- The reusable coordinator uses existing state, model, codec, context,
  registry, dispatcher, attempt guard, policy/grant boundary, cancellation,
  verifier, event, journal, artifact, and checkpoint owners.
- Ephemeral read-only mode rejects state-changing and command catalogs. Durable
  mode cannot attach accidentally without its required ports.
- `ASK` pauses before effect, exact `ALLOW` resumes once, `DENY` starts no
  worker, cancellation wins over a pending allow, and expired approval fails
  before launch.
- Model prose or classifier claims cannot mint success; malformed model results
  close with a truthful non-success outcome.
- All existing native read-only definitions share the common registry and
  retain their closed argument validators. Duplicate registration fails without
  replacing the existing catalog.
- A deterministic fake model calls the existing native `read_text` provider,
  receives exact evidence and a receipt in the next context, and reaches one
  verifier-backed terminal outcome with a legal ordered event stream.
- A second deterministic session composes `read_multiple`, text search, and
  fixed read-only Git inspection through the same coordinator, returning three
  unique receipts and evidence records before verified completion.
- Direct answer, approval, denial, malformed proposal, repeat, no-progress,
  budget, cancellation, model dependency, terminal worker result, maximum-load,
  over-limit, and event-canary fixtures all close with canonical streams.
- The authenticated host protocol prepares, starts, advances, cancels, replays,
  and releases one exact shared-runtime run. Replay accepts only the existing
  `RuntimeEventCursor` identity, sequence, and digest.
- `NativeChatRuntimeService` admits only a host-framed ephemeral read-only
  request, retains at most four live/prepared runs, rejects substitution, and
  permits release only before start or after a terminal outcome.
- The VS Code provider revalidates the selected profile, binds the host-framed
  request to that profile, the selected local workspace, and the exact prompt,
  then independently validates ordered event, approval, cancellation, and
  outcome relationships before display.
- Native Chat tests cover successful output, protected denial, exact
  cancellation, extension deactivation, release, replay, hidden fields,
  reordering, digest substitution, and unsafe local-link neutralization without
  adding filesystem, process, model, tool, policy, grant, or storage authority
  to the extension.
- Native Chat, interactive CLI, and workflow callers produce identical requests,
  events, artifacts, evidence, receipts, checkpoints, and outcomes for the same
  read-only and controlled-write packets.
- The retained source campaign passes 17 engine tests, six native-read tests,
  three native-transport tests, parity and bypass tests, dependency direction,
  and effect mediation. Peak command RSS remains below 47 MiB on the recorded
  host; all five in-memory coordinator scenarios remain below 250 ms.
- A sealed automated boundary review passes 12 checks over 19 committed inputs.
  Six representative shortcut mutations are rejected by its tests.
- The source-security map covers 40 controls and retains raw command output,
  canary, performance, parity, safe-stop, and bypass evidence while fixing all
  installed, independent-review, fuzz, and release claims to false.

## Focused Commands

```text
cargo test -p agentmage-kernel-engine story_23_4 --lib --locked
cargo test -p agentmage-host story_23_4 --lib --locked
cargo clippy -p agentmage-kernel-engine --lib --locked -- -D warnings
cargo clippy -p agentmage-host --lib --locked -- -D warnings
cd shells/vscode && npm test
cd shells/vscode && npm run lint
cd shells/vscode && npm run format:check
python3 scripts/story_23_4_runtime_evidence.py
python3 scripts/runtime_coordinator_boundary_review.py
python3 scripts/story_23_4_security_evidence.py
python3 scripts/story_23_4_evidence_index.py --check
npm run docs:check
```

## Open Evidence

- The source-level native Chat transport is implemented, but the installed host
  does not yet install a production `NativeChatRuntimeFactory`, admitted model
  profile, context composition, or platform-accepted runtime route. The actual
  packaged provider therefore still fails closed as unavailable.
- Injection at every post-receipt, event-publication, terminal-publication, and
  installed-process disconnect boundary remains incomplete.
- Independent coordinator and installed assistive-technology review evidence is
  absent.
- Persistent crash/restart and pressure completion remain owned jointly by
  Stories 21.2, 22.1, 22.2, and 50.2.
- Manual fuzzing remains deliberately deferred to the final campaign.

These absences keep Story 23.4, Sprint 23, and dependent release gates blocked.
The passing source-level transport and fixtures do not enable a model, complete
an installed native Chat workflow, establish a supported platform, or approve a
release.
