# Coding Activation Inventory: 2026-09-21

## Disposition

Decision 0063 is **Accepted under owner delegation, 2026-09-20** under Decision
0054. It resolves the design and authority choice required by Sub-task 48.2.4.1;
it does not claim that the executable path or model admission has passed.

## Observed Starting State

- `shells/host/src/bin/agentmage.rs` and `bin/agent.rs` return
  `client.transport.failed` for every operational invocation without attempting
  a connection.
- `shells/host/src/main.rs` verifies the production package bootstrap and model
  picker, authenticates one peer, then returns
  `agentmage.bootstrap.platform_activation_required`.
- `NativeChatRuntimeService` and `drive_interactive_cli_runtime` implement the
  common transport and client semantics, but the ordinary host installs no
  non-replay `NativeChatRuntimeFactory`.
- `platforms/linux/src/ipc.rs` already owns private mode-`0600` Unix sockets,
  kernel peer credentials, process-start and executable identity, a fresh
  one-use challenge, bounded frames and replay refusal.
- `LinuxCodingRuntimeBoundary`, the reusable coordinator, native coding catalog,
  exact approval/grant flow, verifier, journal, artifact and checkpoint owners
  exist and have component tests. They are not yet an installed workflow.
- `LlamaServerDriver` launches a pinned private llama.cpp process and re-observes
  served capabilities, but currently rejects every profile whose context is not
  exactly 8,192 tokens. It exposes only the Muse ATEM codec; there is no GPT-OSS
  Harmony codec.
- The prepared Muse and GPT-OSS artifacts and llama.cpp b10423 Vulkan runtime are
  present and hash-verified. Their 32,768-token direct probes passed, but neither
  model is coding-qualified or enabled.
- Complete journal, artifact and checkpoint prerequisite tasks named by Task
  50.2.4 are checked complete. The connected coding integration, native campaign,
  pressure/soak evidence and independent review remain open.

## Accepted Implementation Order

1. Make model context capacity a validated exact-profile input, add the Harmony
   family codec, and bind reasoning/resource launch settings without changing the
   existing 8K profiles.
2. Implement the isolated development activation, authenticated client connector,
   host lifecycle and versioned control/event frames.
3. Install one real runtime factory over the existing coding composition and a
   separately labeled scripted executable fixture.
4. Complete live progress, approval, cancellation, follow-up and terminal report
   behavior, then execute the external-process scripted matrix.
5. Run Muse and GPT-OSS sequentially through the same binaries and retain their
   independent admission dispositions.
6. Integrate durable resume, complete artifacts, context continuity, bounded
   session preauthorization and conflict-aware rollback through existing owners.
7. Run recovery/pressure/soak acceptance, prepare the pinned review package and
   stop at the independent-review boundary if no external reviewer result exists.

## Evidence Boundary

This inventory is source diagnosis and an accepted activation contract only.
No task checkbox, status-model entry, enabled-model list, supported-platform
claim, milestone, package or release status changes from this record.
