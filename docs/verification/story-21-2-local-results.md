# Story 21.2 Local Verification Results

| Field | Result |
|---|---|
| Scope | Runtime-event and ordered-stream foundation |
| Local focused result | Pass |
| Complete Story 21.2 result | Blocked |
| Sprint 21 result | Blocked |
| Release approval | No |

## Verified Locally

- The versioned `RuntimeEvent` contract is closed, content minimized, canonically
  hashed, and bound to one run, session, task, correlation, and policy.
- All 21 event families have one canonical correctness, progress, or metric
  persistence class.
- The incremental sequence verifier accepts one exhaustive terminal history
  covering all 21 event families and both allow/deny and success/failure
  branches. It rejects unknown decoding, malformed digests, gaps, reorder,
  replay, unknown causation, orphan or duplicate permission transitions,
  changed run/session/task/correlation/policy bindings, and post-terminal
  events without partially mutating verifier state.
- The in-process publisher preserves canonical order, bounds subscriber count
  and queue capacity, never blocks on a subscriber, and removes lagging or
  disconnected subscribers without changing runtime authority.
- The durable journal writer enforces count, byte, batch, and age ceilings;
  synchronously commits correctness boundaries; batches progress and metrics;
  atomically rejects failed batches; flushes terminal predecessors; and rejects
  retained event or indexed-projection tampering on restart.
- The dedicated bounded journal worker owns the batch writer on one named
  thread over the sole shared SQLCipher connection. Progress admission remains
  responsive while that connection is deliberately blocked, a full reservation
  returns exact saturation without accepting the event, terminal flush and
  shutdown preserve exact replay, and storage failure becomes sticky without
  creating false history.
- The durable correctness port co-publishes permission requests with parent
  grants, allow decisions with operation grants, tool starts with consumed
  launch authority, terminal tool events with normalized receipt state, and
  checkpoint events with the checkpoint and resume binding. Direct commits
  flush accepted asynchronous predecessors and reconcile the worker sequence
  after success.
- Paired SQLCipher tests prove parent grant/event and checkpoint/binding/event
  publication survive a process reopen together. Injected event-insert failures
  preserve the prior generation and leave no candidate grant, checkpoint,
  binding, or false terminal event. Generic effect tests prove the start and
  terminal receipt chain reopens exactly and that a rejected terminal event
  requires authority recovery without inventing a completion event.
- The durable Linux coding composition emits and verifies the exact ordered
  subsequence `permission_requested`, `permission_decided`, `tool_started`,
  `tool_completed`, and `checkpoint_committed`; the terminal timestamp is
  sampled only after normalized tool execution returns.
- The retained process-stop matrix covers queue admission, batch flush,
  correctness transaction, and subscriber publication before and after each
  boundary. Its eight abrupt child exits preserve no false terminal event,
  permit loss only for two declared deferred-progress cases, replay an exact
  missing event once, and verify one identical terminal history after a second
  encrypted reopen. The report binds the repository-root-redacted command
  trace, command identity, source revision, and source files and passes four
  closed-shape and mutation tests.
- The retained source-pressure campaign rejects a one-byte-short cumulative
  reservation, accepts the exact canonical byte boundary, and replays the
  rejected event once after flush. Its durable coordinator advances 17 model
  fragment steps while the sole SQLCipher store mutex is held, observes
  cancellation in 1,054 microseconds against a 250-millisecond ceiling,
  publishes model failure to the client during the delay, and withholds the
  terminal event until correctness durability resumes. The report and raw
  checkout-root-redacted trace bind both exact tests to source commit
  `8af0e30602040a316f15eededf2ab9fe02db9f38` and pass four evidence tests.
- The immutable Fedora worker campaign binds nine command logs and 42 focused
  tests to source commit `f8d521c4dc4bb9b2053447a61aaac04028b4906b`;
  its 8,196-event workload, bounded saturation recovery, memory, disk,
  throughput, publisher, and 16-reopen measurements pass the fixed profile.
- Seven synthetic sensitive-content classes are absent from serialized
  hash-only transcript, canonical/client event, diagnostic, metric, artifact
  manifest, artifact reference, and event-reference projections. Restricted
  transcript placement is refused, valid-looking unregistered event codes are
  rejected, artifact bytes require an exact owner-bound read, and a
  cross-session artifact read is denied.
- A story-local source-closure test fixes the security-authoritative kernel's
  direct dependencies and rejects external network or telemetry APIs in the
  runtime event, journal, projection, artifact, and CLI-client implementation.
- A separately implemented automated boundary reviewer recomputes 12 closed
  schema, family, persistence, queue, cancellation, network, crash, pressure,
  load, canary, evidence-index, and limitation checks over 19 hash-bound source
  and evidence inputs. It records that it is not an independent human review.
- The product-security evidence map binds all 22 applicable `SR-DAT-*`,
  `SR-AI-010`, `SR-OPS-*`, `SR-TST-*`, and `RV-18` controls to exact source and
  retained evidence at commit `613abae23a9130da457dcbd08ebb2e92bf4d041a`.
  Its repository-root-redacted trace records eight passing local-only commands
  and explicitly denies external-network use, private-user-data use, manual
  fuzzing, human review, release approval, and platform or model enablement.
- A deterministic hashed index maps every Story 21.2 sub-task to its exact
  statement, implementation files, executable tests, and retained evidence.
  It preserves 14 complete, two partial, and zero open sub-tasks and rejects
  omission, reorder, status, statement, file, test, digest, and completion
  mutations.
- The JSON schema and canonical Rust example pass the repository schema gate.
- Markdown and Mermaid validation pass for the Story 21.2 architecture record.

## Focused Commands

```text
cargo test -p agentmage-kernel-engine runtime_event --lib --locked
cargo test -p agentmage-kernel-engine runtime_journal --lib --locked
cargo test -p agentmage-kernel-engine operational_store --lib --locked
cargo test -p agentmage-kernel-engine runtime_artifact --lib --locked
cargo test -p agentmage-kernel-engine story_21_2_ --lib --locked
cargo test -p agentmage-kernel-engine --lib --locked
cargo test -p agentmage-host --lib --locked
cargo clippy -p agentmage-kernel-engine --all-targets --locked -- -D warnings
cargo clippy -p agentmage-host --all-targets --locked -- -D warnings
python3 scripts/story_21_2_evidence_index.py --check
python3 -m unittest tests.test_story_21_2_evidence_index
python3 scripts/story_21_2_crash_evidence.py
python3 -m unittest tests.test_story_21_2_crash_evidence
python3 scripts/story_21_2_pressure_evidence.py
python3 -m unittest tests.test_story_21_2_pressure_evidence
python3 scripts/runtime_journal_boundary_review.py
python3 -m unittest tests.test_runtime_journal_boundary_review
python3 scripts/story_21_2_security_evidence.py
python3 -m unittest tests.test_story_21_2_security_evidence
npm run schemas:check
npx markdownlint-cli2 README.md docs/architecture/runtime-event-journal.md docs/verification/story-21-2-local-results.md
python3 scripts/check_mermaid.py
python3 scripts/runtime_hardening_load.py --output <fresh-evidence-path>
```

## Open Evidence

- Cross-subsystem reconciliation between a physically launched effect,
  recovered authority/receipt state, and a still-open `tool_started` journal
  transition remains outside the journal-only matrix. The current source does
  not fabricate a terminal event during authority-only recovery.
- The slow-store pressure tests hold the sole connection mutex
  deterministically. Physical filesystem/device fault injection and installed
  model-runtime or native-client pressure remain open.
- Persisted user transcript and local diagnostics lifecycle implementations
  remain open. Their bounded in-memory projections do not establish durable
  retention, export, deletion, or installed-client behavior.
- Kernel `SIGKILL`, host power loss, torn-sector, controller-failure, and
  filesystem-corruption campaigns remain open; the retained matrix uses
  deterministic process exit without unwinding.
- The retained campaigns cover event count and exact byte edges, producer
  saturation, one slow consumer, bounded memory/disk, isolated slow-store
  admission, integrated source-level model progress and cancellation, and
  restart. Physical disk/device latency fault injection remains open.
- Product-wide memory-dump, crash-artifact, installed-client, and operating
  system telemetry observation remains outside this story-local projection
  campaign. The repository-wide strict-local source policy snapshot also needs
  reconciliation with broader post-baseline source and package additions; it
  is not reported as passing by this result.
- One Fedora source-host latency, throughput, memory, and disk profile is
  retained. Installed-interface and additional supported-platform profiles
  remain open.
- The separate automated boundary review passes. Independent human review of
  the hashed evidence index and critical journal boundaries remains open.
- Manual fuzzing remains deliberately deferred to the final campaign.

These absences keep Story 21.2, Sprint 21, and every dependent release gate
blocked. Passing source-level tests does not enable a model, installed client,
supported platform, or release.
