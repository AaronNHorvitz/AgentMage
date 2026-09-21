# Decision 0063: Disposable Coding Development Activation

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-21 |
| Authority | Decision 0054 and Tasks 48.2.4.1-48.2.4.2 |
| Scope | Repository-local Linux activation for executable coding integration and qualification campaigns |
| Preserves | Production package activation, platform admission, model admission, exact grants, confinement, evidence distinctions, release gates and independent review |

## Context

The production host correctly refuses activation without an independently signed
package and platform release. The repository also needs an actual-process venue
for the disposable coding acceptance matrix before production signing or release
authority exists. Deleting the production refusal, accepting an arbitrary model
endpoint, using a test key, or treating a component fixture as an executable
workflow would violate Decisions 0054 and 0061.

The current Linux boundaries already provide exact peer observation, a private
Unix socket, a one-use challenge and launch secret, descriptor-held workspace
objects, native tool dispatch, fresh exact grants, confined workers, bounded
model execution, and the canonical journal/artifact/checkpoint owners. A separate
development activation can compose those owners without weakening them.

## Decision

1. Add a distinct `coding-development-v1` activation. It is never selected by
   the production bootstrap, never reads production signing state, never writes
   production state, and cannot qualify a package, platform, model or release.
2. Activation is explicit at both processes. The terminal launches the sibling
   host binary with the development service operation; the host authenticates
   that exact parent process through the existing kernel-observed UID, PID,
   process-start and executable-digest binding. The one-use secret is transferred
   only through inherited standard I/O and is erased after the handshake. It is
   not accepted in arguments, environment, model context, logs or state files.
3. The workspace must be an absolute, canonical, standard-user-owned Git worktree
   beneath an explicitly supplied disposable root. The root and worktree must be
   private, must not be the AgentMage source checkout, and must contain an owned
   mode-`0600` `.agentmage-development-workspace` marker whose contents bind the
   activation version and exact canonical worktree. Symlinks, ownership drift,
   public write access, nested escape and marker substitution fail before effects.
4. Development state is rooted only in the launcher's private
   `~/.local/state/agentmage-codex-coding/runtime` directory or an explicit
   private test root. State, socket, lock, key and database objects remain
   owner-only and single-link. Startup removes only stale objects whose recorded
   owner, type and activation identity match; it never recursively deletes an
   unresolved path.
5. The development operational-store key is a separately generated mode-`0600`
   local key under that private state root. It is permitted only for the
   development database and is never a production Secret Service substitute.
   The key is not exposed to the model, workers, terminal output or evidence.
6. Scripted model proposals are permitted only when the exact activation profile
   says `scripted-executable-fixture`. The terminal and reports display that
   scope. Production activation cannot select it. Scripted proposals still pass
   through the real coordinator, native registry, policy, approvals, fresh
   grants, Linux effect boundary and verifier.
7. Real-model development activation accepts only an exact source-controlled
   profile plus verified artifact/runtime inventories. It re-observes served
   context and the complete model/runtime/codec/decoding/resource tuple before
   every campaign. Direct inference preparation is not admission. Muse ATEM and
   GPT-OSS Harmony remain explicit codecs behind the common model boundary.
8. One effect-bearing run owns the worktree lease. Duplicate request identities,
   stale launch challenges, foreign peers, changed workspace/profile identities,
   and a second writer fail closed. Shutdown reaps only descendants launched by
   this activation and reports uncertain effects when cleanup cannot be proven.
9. The host retains runtime ownership. The terminal can prepare, start, follow
   up, inspect events/outcome, answer a protected approval, cancel and release;
   it cannot submit grants, policies, model endpoints or effect receipts.
10. Evidence labels remain `component`, `executable-scripted`, or
    `qualified-model`. This activation alone changes no status in
    `architecture/status-model.json` and cannot close `M-HARNESS-MVP` or
    `M-HARNESS-DAILY`.

## Prerequisite Disposition

| Boundary | Accepted development path | Production status |
|---|---|---|
| Launch trust | Exact parent executable plus one-use authenticated private IPC | Signed package/bootstrap remains mandatory |
| State/key | Separate private root and development-only local key | Secret Service production lifecycle unchanged |
| Workspace | Explicit disposable marked Git worktree with exact held objects | General user-workspace activation remains gated |
| Confinement | Existing Linux namespaces, descriptor resolution, command templates and fresh grants | No sandbox or authority relaxation |
| Model/runtime | Exact source-controlled tuple and re-observed served capabilities | No model enabled by this decision |
| Context/codec | Profile capacity, rendered token preflight and family codec must agree | No global 8K-to-32K substitution |
| Resources | One model/slot; four threads; declared memory, swap, CPU, output and time ceilings | Release resource profiles remain separate |
| Build/test | At least 16 GiB available RAM; required systemd scope; at most four Cargo jobs | No packaging or publication authority |

## Rejection and Cleanup

Invalid activation performs no workspace effect. A failure after dispatch retains
the canonical receipt or an explicit uncertain result. Cleanup is limited to the
exact socket, lock, child processes and state objects owned by the current launch.
The implementation must test foreign peers, replay, stale challenge, wrong
workspace/profile, unsafe modes, duplicate writer and unqualified model cases.
